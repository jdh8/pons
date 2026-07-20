//! Teacher dump (AI-bidder M0.4)
//!
//! Bids out boards — random, or every deal in a GIB file via `--deals` — with
//! the *teacher* system (`american()`, or the vendored EPBot 2/1 oracle via
//! `--teacher bba`) and records, at every decision point, a training row of
//! `(features, teacher_softmax)`:
//!
//! - **features** — the restrictive *disclosable-only* v3 vector for the hand
//!   to act ([`features_v3`][pons::bidding::features::features_v3]): 88 floats
//!   of hand summary, context, inferences and vulnerability, with no
//!   card-specific values. This is the extractor the shipped floor uses.
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
use pons::american_instinct;
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{FEATURES_LEN_V3, FEATURES_VERSION_V3, features_v3};
use pons::bidding::{Family, Phase, System};
use pons::gib;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, SYSTEM_2_OVER_1, load_bbsa};

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
    /// Teacher to distil: `american` (the pure-Rust 2/1 floor, default) or `bba`
    /// (the vendored EPBot 2/1 oracle). `bba` bids through a single-threaded FFI
    /// bot per decision, so that dump is BBA-bidding-bound — tractable, not
    /// instant. Override the `.so` with `BBA_LIB`.
    #[arg(long, default_value = "american")]
    teacher: String,
    /// `--teacher bba` only: a `.bbsa` convention card (e.g.
    /// `vendor/bba/WJ.bbsa`) pinning the teacher's system *and* every one of its
    /// named conventions, engine defaults included. Without it the teacher is
    /// EPBot's 2/1 with whatever the engine defaults to — fine for the 2/1 nets,
    /// but "BBA system 2" alone does not pin `Multi` / `Polish two suiters`, so
    /// a WJ net must name its card. Recorded in the JSON sidecar: a distilled
    /// net is identified by its extractor *and* its teacher configuration.
    #[arg(long)]
    card: Option<String>,
}

/// The four absolute vulnerabilities, sampled uniformly per board.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let (feature_version, features_len) = (FEATURES_VERSION_V3, FEATURES_LEN_V3);
    // DD label only exists when deals come from a GIB file (cached, no solving).
    let dd_len = if args.deals.is_some() { 20 } else { 0 };
    let row_len = features_len + SOFTMAX_LEN + dd_len;
    // Both sides play the same system; the classifier handles whichever seat is
    // to act (vulnerability passed in relative). `american()` routes by phase
    // through a Stance; `bba` is the vendored EPBot 2/1 oracle — a fresh
    // single-threaded FFI bot per decision.
    let teacher: Box<dyn System> = match args.teacher.as_str() {
        "american" => Box::new(american_instinct().against(Family::NATURAL)),
        "bba" => {
            let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
            let card = args.card.as_deref().map(load_bbsa).transpose()?;
            let (system, toggles) = match card {
                Some(card) => (card.system, card.toggles),
                None => (SYSTEM_2_OVER_1, Vec::new()),
            };
            Box::new(BbaOracle::load(&path, system, toggles)?)
        }
        other => anyhow::bail!("--teacher must be american|bba, got {other:?}"),
    };
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

            let Some(mut logits) = teacher.classify(hand, rel, &auction) else {
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
            let feats = features_v3(hand, &context);
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
        "teacher": &args.teacher,
        "card": args.card.as_deref().unwrap_or("engine defaults"),
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

/// Load every deal and its cached double-dummy table from a solution file in
/// either format (GIB text like `sol100000.txt`, or binary `.pdd`).
fn load_deals(path: &str) -> std::io::Result<Vec<(FullDeal, TrickCountTable)>> {
    let deals = pons::pdd::load(path)?;
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
