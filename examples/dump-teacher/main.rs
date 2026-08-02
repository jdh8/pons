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
use pons::bidding::Stance;
use pons::bidding::card::{american_card, dutch_card};
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{
    Config, FEATURES_LEN_V3, FEATURES_LEN_V4, FEATURES_VERSION_V3, FEATURES_VERSION_V4,
    features_v3, features_v4,
};
use pons::bidding::{Phase, System};
use pons::gib;
use pons::{american, american_instinct, dutch, dutch_instinct};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::ffi::CString;
use std::io::{BufWriter, Write};
use std::os::raw::c_int;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, DEFAULT_LIB, SYSTEM_2_OVER_1, load_bbsa};

/// Number of calls in a `Logits` array (the softmax width).
const SOFTMAX_LEN: usize = 38;

#[derive(Parser)]
#[command(about = "Dump (features, teacher_softmax) training rows from american()")]
struct Args {
    /// Number of boards to bid out
    ///
    /// Without `--deals` this is how many random boards to draw (default 5000).
    /// With `--deals` it caps the window read from the file; omit it to take
    /// every deal from `--skip` to the end, which is what a bare `--deals` has
    /// always meant.
    #[arg(long)]
    boards: Option<usize>,
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
    /// Skip this many deals at the front of `--deals` before reading
    ///
    /// The banks at `/nfs2/jdh8/pons/` are read with `pdd::load_slice`, so a
    /// corpus draw never pulls a 2 GB file into memory whole.  Pair with
    /// `--boards` to bound the draw: `--skip 1000000 --boards 250000`.
    ///
    /// Training draws do **not** advance the never-replay cursor (they may
    /// overlap each other), but they must be recorded — a slice used to train a
    /// net must never later be used to score it.  See
    /// [docs/pdd-bank-ledger.md](../../docs/pdd-bank-ledger.md).
    #[arg(long, default_value_t = 0)]
    skip: u64,
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
    /// `--teacher bba` only: force one named convention on/off (repeatable),
    /// applied *after* the system's defaults and after `--card`, e.g.
    /// `--conv "Kickback 1430=1"`.  A card pins everything at once, which makes
    /// it useless for isolating a single toggle against a net trained on engine
    /// defaults; this keeps every other convention exactly where it was so the
    /// retrained twin differs from its baseline in one convention only.
    /// Recorded in the JSON sidecar beside `card`.
    #[arg(long = "conv", value_parser = parse_override, value_name = "NAME=0|1")]
    conv: Vec<(CString, c_int)>,
    /// Extract features with **our** kickback recognizer armed
    /// ([`set_kickback`]).  Pair it with `--conv "Kickback 1430=1"`: the `conv`
    /// flag makes the *teacher* play kickback, this one makes our extractor
    /// read the resulting auctions the way serving will.
    ///
    /// It matters because 40 of `features_v3`'s 88 floats come from
    /// `Inferences::read`, and the recognizer is what decides whether a
    /// relocated 4♥ reads as a diamond ask or as natural hearts.  Distilling a
    /// kickback teacher through a kickback-blind extractor trains the net on
    /// readings it will never be served — the same out-of-distribution trap the
    /// `evaluator_v3_exclusion` twin was regenerated to escape.
    #[arg(long)]
    kickback: bool,
    /// Alternate the kickback regime **per board**: even boards get the plain
    /// teacher and a kickback-blind extractor, odd boards get `--conv` and an
    /// armed one.  Requires `--conv`.
    ///
    /// One net then covers both systems instead of a knob choosing between two.
    /// It can, because the regime is *in the features*: forty of the eighty-eight
    /// come from `Inferences::read`, and the ranges a 4♥ carries differ between
    /// "six-plus hearts" and "a keycard count", so the net reads which system it
    /// is in off the auction rather than being told.
    ///
    /// Alternating **inside one dump** rather than concatenating two is what
    /// keeps the trainer's validation split honest: it takes the tail
    /// contiguously, so stitched corpora would validate entirely on whichever
    /// regime landed last, while interleaving by board leaves the tail
    /// board-disjoint *and* mixed.
    #[arg(long)]
    mix_kickback: bool,
    /// Emit the **configured** vector [`features_v4`] instead of `features_v3`
    ///
    /// 368 floats rather than 88: the v3 vector, then both partnerships'
    /// convention cards.  The point is that an A/B arm can then differ by a card
    /// row instead of by a separately trained artifact — see
    /// [docs/ai-bidder/configured-net.md](../../docs/ai-bidder/configured-net.md).
    ///
    /// Opt-in, so every existing v3 corpus recipe keeps its exact meaning.
    #[arg(long)]
    configured: bool,
    /// `--configured` only: which system *we* are declared to play
    #[arg(long, default_value = "american", value_name = "american|dutch")]
    system: String,
    /// `--configured` only: extract from a **bare** context, as v3 dumps do
    ///
    /// `--configured` otherwise builds the prefix-bearing context serving uses,
    /// because a bare one skips `project_authored` entirely — a train/serve skew
    /// measured at 3 of 40 inference floats.  This flag exists to reproduce the
    /// old behaviour for comparison, not to dump a corpus with.
    #[arg(long)]
    bare_context: bool,
    /// `--configured` only: which system the **opponents** are declared to play
    ///
    /// Defaults to ours, mirroring how `BbaOracle` treats undeclared opponents
    /// and how `Context::their_system` models them.  Naming a different one
    /// gives the cross-system cell.
    #[arg(long, value_name = "american|dutch")]
    their_system: Option<String>,
}

/// Our card for a named system, rendered off the **live** knob state
///
/// Must be called after the knobs for this cell are set: `american_card()` reads
/// them, which is precisely what keeps the card, the code and the net in sync.
fn card_for(system: &str) -> anyhow::Result<pons::bidding::card::Card> {
    Ok(match system {
        "american" => american_card(),
        "dutch" => dutch_card(),
        other => anyhow::bail!("--system must be american|dutch, got {other:?}"),
    })
}

/// `NAME=0|1`, as `bba-gen` spells it.
fn parse_override(spec: &str) -> Result<(CString, c_int), String> {
    let (name, value) = spec
        .rsplit_once('=')
        .ok_or("expected NAME=0|1 (e.g. \"Kickback 1430=1\")")?;
    let on = match value.trim() {
        "0" => 0,
        "1" => 1,
        other => return Err(format!("value must be 0 or 1, got `{other}`")),
    };
    let name = CString::new(name.trim()).map_err(|_| "name has an interior NUL".to_string())?;
    Ok((name, on))
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
    let (feature_version, features_len) = if args.configured {
        (FEATURES_VERSION_V4, FEATURES_LEN_V4)
    } else {
        (FEATURES_VERSION_V3, FEATURES_LEN_V3)
    };
    // DD label only exists when deals come from a GIB file (cached, no solving).
    let dd_len = if args.deals.is_some() { 20 } else { 0 };
    let row_len = features_len + SOFTMAX_LEN + dd_len;
    // Both sides play the same system; the classifier handles whichever seat is
    // to act (vulnerability passed in relative). `american()` routes by phase
    // through a Stance; `bba` is the vendored EPBot 2/1 oracle — a fresh
    // single-threaded FFI bot per decision.
    if args.mix_kickback && args.conv.is_empty() {
        anyhow::bail!("--mix-kickback needs --conv to say what the ON regime is");
    }
    // The regimes a board can be dumped under.  One entry normally; two when
    // mixing, selected by board parity so the corpus interleaves.
    let regimes: Vec<bool> = if args.mix_kickback {
        vec![false, true]
    } else {
        vec![args.kickback]
    };
    let build_teacher = |with_conv: bool| -> anyhow::Result<Box<dyn System>> {
        Ok(match args.teacher.as_str() {
            // `--system` selects the teacher, not merely the disclosed card.
            // Letting them drift apart writes rows labelled with a system the
            // teacher was not playing -- the mislabeling that `verify_card`
            // guards against for BBA, one level up and just as invisible.
            "american" if args.system == "dutch" => Box::new(dutch_instinct().against()),
            "american" => Box::new(american_instinct().against()),
            "bba" => {
                let path = std::env::var("BBA_LIB").unwrap_or_else(|_| DEFAULT_LIB.into());
                let card = args.card.as_deref().map(load_bbsa).transpose()?;
                let (system, mut toggles) = match card {
                    Some(card) => (card.system, card.toggles),
                    // `--configured --system dutch` means the teacher plays WJ;
                    // without this the corpus would claim WJ over 2/1 bidding.
                    None if args.configured => (card_for(&args.system)?.system, Vec::new()),
                    None => (SYSTEM_2_OVER_1, Vec::new()),
                };
                // Singles win over the card, exactly as `bba-gen` applies them.
                if with_conv {
                    toggles.extend(args.conv.iter().cloned());
                }
                Box::new(BbaOracle::load(&path, system, toggles)?)
            }
            other => anyhow::bail!("--teacher must be american|bba, got {other:?}"),
        })
    };
    // One teacher per regime: mixing needs the plain engine *and* the one
    // playing the convention, because the target has to change with the reading.
    let teachers: Vec<Box<dyn System>> = if args.mix_kickback {
        vec![build_teacher(false)?, build_teacher(true)?]
    } else {
        vec![build_teacher(!args.conv.is_empty())?]
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
        // `boards` unset means "to the end of the file", the historical meaning
        // of a bare `--deals`; set, it bounds the window so a bank draw stays
        // the size it says it is.
        Some(path) => load_deals(path, args.skip, args.boards.unwrap_or(0))?,
        None => Vec::new(),
    };
    let n_boards = if args.deals.is_some() {
        file_deals.len()
    } else {
        args.boards.unwrap_or(5000)
    };
    let mut file_iter = file_deals.iter().copied();

    for board in 0..n_boards {
        // Which regime this board is dumped under.  Constant unless mixing, in
        // which case parity alternates so the corpus interleaves and the
        // trainer's contiguous tail stays representative of both systems.
        let regime = board % regimes.len();
        // Arm the recognizer before a single row of this board is extracted:
        // `features_v3` reads `Inferences`, so this decides what the corpus
        // *says* a relocated ask is — and it must agree with the teacher that
        // produced the target.
        pons::bidding::instinct::set_kickback(regimes[regime]);
        // The card is rendered *after* the knobs are armed, which is what keeps
        // card, code and net in sync: `american_card()` reads the same knobs the
        // rules do.  Rebuilt per board only because `regime` can alternate; it is
        // 140 floats a side, not a hot cost.
        let config = args
            .configured
            .then(|| -> anyhow::Result<Config> {
                let ours = card_for(&args.system)?;
                let theirs = match &args.their_system {
                    Some(system) => card_for(system)?,
                    None => ours.clone(),
                };
                Ok(Config::new(&ours, &theirs))
            })
            .transpose()?;
        // The stance that *reads* the auction.  Built after the knobs are armed,
        // for the same reason the card is: rule presence is decided at build.
        let reader: Option<Stance> = (args.configured && !args.bare_context)
            .then(|| -> anyhow::Result<Stance> {
                Ok(match args.system.as_str() {
                    "dutch" => dutch().against(),
                    _ => american().against(),
                })
            })
            .transpose()?;
        let teacher = teachers[regime].as_ref();

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
            // Prefixed when configured: the same context serving builds, so the
            // authored-projection overlay is applied at dump time too.
            let mut context = match &reader {
                Some(stance) => stance.prefixed_context(rel, &auction),
                None => Context::new(rel, &auction),
            };
            if let Some(config) = &config {
                context = context.with_config(config);
            }
            let feats = if args.configured {
                features_v4(hand, &context)
            } else {
                features_v3(hand, &context)
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
        "teacher": &args.teacher,
        "card": args.card.as_deref().unwrap_or("engine defaults"),
        // A distilled net is identified by its extractor *and* its teacher
        // configuration, so a forced toggle belongs beside the card.
        "conv": args
            .conv
            .iter()
            .map(|(name, on)| format!("{}={on}", name.to_string_lossy()))
            .collect::<Vec<_>>(),
        // The extractor's own regime, not the teacher's — a net is identified
        // by both.  Under `--mix-kickback` the corpus carries both, alternating
        // by board, and the net learns to tell them apart from the readings.
        "our_kickback": args.kickback,
        "mix_kickback": args.mix_kickback,
        "configured": args.configured,
        "context": if args.configured && !args.bare_context { "prefixed" } else { "bare" },
        "our_system": args.configured.then(|| args.system.clone()),
        "their_system": args
            .configured
            .then(|| args.their_system.clone().unwrap_or_else(|| args.system.clone())),
        "skip": args.skip,
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

/// Load deals and their cached double-dummy tables from a solution file in
/// either format (GIB text like `sol100000.txt`, or binary `.pdd`).
///
/// `skip`/`count` read a window rather than the whole file: `24.pdd` is 2 GB and
/// the standing rule is to draw a corpus from the banks without pulling one into
/// memory entire.  `count` of 0 means "the rest of the file", which is what a
/// bare `--deals` on a small GIB file has always meant.
fn load_deals(
    path: &str,
    skip: u64,
    count: usize,
) -> std::io::Result<Vec<(FullDeal, TrickCountTable)>> {
    let deals = if skip == 0 && count == 0 {
        pons::pdd::load(path)?
    } else {
        pons::pdd::load_slice(path, skip, count)?
    };
    eprintln!(
        "teacher-dump: loaded {} deals from {path} (skip {skip}, asked {})",
        deals.len(),
        if count == 0 {
            "all".to_owned()
        } else {
            count.to_string()
        },
    );
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
