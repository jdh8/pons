//! Teacher dump (AI-bidder M0.4)
//!
//! Bids out boards — random, or every deal in a GIB file via `--deals` — with
//! the assembled `american()` system (the *teacher*) and records, at every
//! decision point, a training row of `(features, teacher_softmax)`:
//!
//! - **features** — the feature vector for the hand to act: the 160-float
//!   v1 vector ([`features`][pons::bidding::features::features]) by default, or
//!   the tag-augmented v2 vector
//!   ([`features_v2`][pons::bidding::features::features_v2]) under
//!   `--features-version 2` (AI-bidder M5.1).
//! - **teacher_softmax** — the teacher's `Logits` at that node, masked to the
//!   *legal* calls and pushed through `softmax`, giving a 38-way distribution
//!   over calls. Matching the full distribution (not just the argmax) is what
//!   makes distillation transfer the teacher's near-misses and mixtures.
//!
//! Output is a flat little-endian `f32` file — one row of `features_len + 38`
//! floats, plus 20 more when `--deals` supplies a GIB file: that board's cached
//! double-dummy table, re-oriented to the acting seat
//! ([`gib::relativized_tricks`]), as a free regression target alongside the
//! policy. Plus a JSON sidecar pinning the feature version, teacher, seed,
//! counts, and `dd_len` (a distilled model is meaningless without its exact
//! feature extractor; they version together), and a sibling `.tags` file of one
//! `u8` per row (`1` = contested-phase decision, `0` = constructive) so the
//! trainer can report held-out agreement split by phase. The Rust/candle trainer
//! reads the `.f32` with a trivial loader.
//!
//! ```text
//! cargo run --release --example dump-teacher -- --boards 100000 --seed 1
//! ```
//!
//! The auction is advanced by the teacher's own legal argmax, so the visited
//! states are the ones the teacher actually reaches. Contested/off-book
//! oversampling beyond what random boards yield is left to M1 data prep; this
//! reports the contested fraction so we know what we have.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::TrickCountTable;
use pons::american;
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{
    FEATURES_LEN, FEATURES_LEN_V2, FEATURES_LEN_V3, FEATURES_VERSION, FEATURES_VERSION_V2,
    FEATURES_VERSION_V3, features, features_v2, features_v3,
};
use pons::bidding::{Phase, System, Tag};
use pons::gib;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

/// Number of calls in a `Logits` array (the softmax width).
const SOFTMAX_LEN: usize = 38;

#[derive(Parser)]
#[command(about = "Dump (features, teacher_softmax) training rows from american()")]
struct Args {
    /// Number of random boards to bid out (ignored when `--deals` is given)
    #[arg(long, default_value_t = 5000)]
    boards: usize,
    /// RNG seed (for reproducibility)
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Feature extractor version: 1 = the 160-float vector, 2 = + the tag block,
    /// 3 = the restrictive disclosable-only vector (88 floats)
    #[arg(long, default_value_t = 1)]
    features_version: u32,
    /// Optional GIB deal file (e.g. sol100000.txt): bid out every deal in it
    /// instead of random boards. Each line is `<PBN, West-first>:<20 hex DD>`;
    /// the cached DD table becomes a 20-float per-row regression target (random
    /// boards have no free DD, so they omit it). Dealer and vulnerability are
    /// still drawn from the seeded RNG per board.
    #[arg(long)]
    deals: Option<String>,
    /// Output path stem; writes `<out>.f32` and `<out>.json`
    #[arg(long, default_value = "target/teacher-data")]
    out: String,
}

/// The four absolute vulnerabilities, sampled uniformly per board.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let (feature_version, features_len) = match args.features_version {
        1 => (FEATURES_VERSION, FEATURES_LEN),
        2 => (FEATURES_VERSION_V2, FEATURES_LEN_V2),
        3 => (FEATURES_VERSION_V3, FEATURES_LEN_V3),
        other => {
            eprintln!("teacher-dump: unknown --features-version {other}; use 1, 2 or 3");
            std::process::exit(2);
        }
    };
    // DD label only exists when deals come from a GIB file (cached, no solving).
    let dd_len = if args.deals.is_some() { 20 } else { 0 };
    let row_len = features_len + SOFTMAX_LEN + dd_len;
    let pair = american();
    // Both sides play the same system; a Stance routes by auction phase, so one
    // suffices for whichever seat is to act (vulnerability passed in relative).
    let stance = pair.against(Tag::NATURAL);
    let mut rng = StdRng::seed_from_u64(args.seed);

    let f32_path = format!("{}.f32", args.out);
    let json_path = format!("{}.json", args.out);
    let tags_path = format!("{}.tags", args.out);
    let mut writer = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut tags_writer = BufWriter::new(std::fs::File::create(&tags_path)?);

    let mut rows = 0u64;
    let mut contested = 0u64;
    let mut forced_pass = 0u64; // decisions the teacher had no logits for
    let mut call_hist: BTreeMap<String, u64> = BTreeMap::new();
    let mut row = vec![0f32; row_len];

    // Deal source: every deal + cached DD table in `--deals` (the 100K GIB
    // file), else random boards (no DD). Dealer/vulnerability come from the
    // seeded RNG either way.
    let file_deals: Vec<(FullDeal, TrickCountTable)> = match &args.deals {
        Some(path) => load_deals(path)?,
        None => Vec::new(),
    };
    let n_boards = if args.deals.is_some() {
        file_deals.len()
    } else {
        args.boards
    };
    let mut file_iter = file_deals.iter().copied();

    for _ in 0..n_boards {
        // File deals (with their DD table) when `--deals` is set, else a fresh
        // random board with no table.
        let (deal, table) = match file_iter.next() {
            Some((deal, table)) => (deal, Some(table)),
            None => (full_deal(&mut rng), None),
        };
        let dealer = rng.random_range(0..4usize);
        let vul = VULS[rng.random_range(0..4usize)];

        let mut auction = Auction::new();
        while !auction.has_ended() {
            let seat = Seat::ALL[(dealer + auction.len()) % 4];
            let hand = deal[seat];
            let rel = relative(vul, seat);

            let Some(mut logits) = stance.classify(hand, rel, &auction) else {
                forced_pass += 1;
                auction.push(Call::Pass);
                continue;
            };

            // Mask illegal calls; the teacher target is over legal calls only.
            for (call, slot) in logits.iter_mut() {
                if auction.can_push(call).is_err() {
                    *slot = f32::NEG_INFINITY;
                }
            }
            let Some(softmax) = logits.softmax() else {
                forced_pass += 1;
                auction.push(Call::Pass);
                continue;
            };

            // Record the row: features ++ softmax (++ DD label when present).
            let context = Context::new(rel, &auction);
            let feats = match feature_version {
                FEATURES_VERSION_V2 => features_v2(hand, &context),
                FEATURES_VERSION_V3 => features_v3(hand, &context),
                _ => features(hand, &context),
            };
            row[..features_len].copy_from_slice(&feats);
            row[features_len..features_len + SOFTMAX_LEN].copy_from_slice(&softmax[..]);
            if let Some(table) = &table {
                row[features_len + SOFTMAX_LEN..]
                    .copy_from_slice(&gib::relativized_tricks(table, seat));
            }
            for value in &row {
                writer.write_all(&value.to_le_bytes())?;
            }
            let contested_row = Phase::of(&auction) != Phase::Constructive;
            tags_writer.write_all(&[u8::from(contested_row)])?;
            rows += 1;
            if contested_row {
                contested += 1;
            }

            // Advance the auction by the teacher's legal argmax.
            let next = argmax_legal(&logits);
            *call_hist.entry(format!("{next}")).or_insert(0) += 1;
            auction.push(next);
        }
    }
    writer.flush()?;
    tags_writer.flush()?;

    let git_sha = git_sha();
    let metadata = serde_json::json!({
        "feature_version": feature_version,
        "features_len": features_len,
        "softmax_len": SOFTMAX_LEN,
        "dd_len": dd_len,
        "row_len": row_len,
        "row_bytes": row_len * 4,
        "dtype": "f32-le",
        "layout": if dd_len > 0 {
            format!("row = [{features_len} features][{SOFTMAX_LEN} teacher_softmax][{dd_len} dd_tricks]")
        } else {
            format!("row = [{features_len} features][{SOFTMAX_LEN} teacher_softmax]")
        },
        "tags": "sibling .tags file: one u8 per row, 1 = contested phase, 0 = constructive",
        "teacher": "american()",
        "deals": args.deals.as_deref().unwrap_or("random"),
        "git_sha": git_sha,
        "seed": args.seed,
        "boards": n_boards,
        "rows": rows,
        "contested_rows": contested,
        "forced_pass_decisions": forced_pass,
    });
    std::fs::write(&json_path, format!("{metadata:#}\n"))?;

    let pct = |n: u64| {
        if rows == 0 {
            0.0
        } else {
            100.0 * n as f64 / rows as f64
        }
    };
    eprintln!(
        "teacher-dump: {rows} rows (feature v{feature_version}, {features_len} features) \
         from {n_boards} boards → {f32_path} ({:.1} MB), \
         {contested} contested ({:.0}%), {forced_pass} forced passes.",
        (rows as usize * row_len * 4) as f64 / 1e6,
        pct(contested),
    );
    eprintln!("top advancing calls:");
    let mut hist: Vec<(String, u64)> = call_hist.into_iter().collect();
    hist.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (call, count) in hist.into_iter().take(12) {
        eprintln!("  {call:>4}  {count:>8}  ({:.1}%)", pct(count));
    }
    Ok(())
}

/// The highest-logit finite (hence legal, after masking) call, defaulting to a
/// pass so the auction always terminates.
fn argmax_legal(logits: &pons::bidding::array::Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Load every deal and its cached double-dummy table from a GIB solution file
/// (e.g. `sol100000.txt`); see [`pons::gib::parse_line`].
fn load_deals(path: &str) -> std::io::Result<Vec<(FullDeal, TrickCountTable)>> {
    let text = std::fs::read_to_string(path)?;
    let deals: Vec<(FullDeal, TrickCountTable)> =
        text.lines().filter_map(gib::parse_line).collect();
    eprintln!("teacher-dump: loaded {} deals from {path}", deals.len());
    Ok(deals)
}

/// Best-effort current commit, for the metadata sidecar; `"unknown"` on failure.
fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}
