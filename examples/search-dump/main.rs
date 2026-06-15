//! Search-target dump (AI-bidder M3.1)
//!
//! The Phase-2 sibling of [`teacher-dump`](../teacher-dump/main.rs).  Where that
//! records the *deterministic* teacher's softmax, this bids out random boards
//! with the M2.3 live double-dummy **search** bidder
//! ([`two_over_one_search`][pons::two_over_one_search]) and records, at every
//! decision point, a training row of `(features, search_target)`:
//!
//! - **features** — the 160-float `FEATURES_V1` vector
//!   ([`features`][pons::bidding::features::features]) for the hand to act.
//! - **search_target** — the search floor's improved `Logits` at that node,
//!   masked to the *legal* calls and pushed through `softmax`, a 38-way
//!   distribution.  On-book it is the book (the search floor never fires);
//!   off-book it is the cardplay-grounded distribution the search produced.
//!
//! The output is byte-identical in layout to `teacher-dump` — a flat
//! little-endian `f32` file of `160 + 38 = 198` floats per row, a `.json`
//! sidecar pinning the feature version / system / seed / counts, and a `.tags`
//! file of one `u8` per row — so the off-crate trainer consumes it unchanged for
//! M3.2.  The only difference is the *target*: the search improves on the
//! teacher exactly where the books were silent, so this file is a
//! **trainer-compatible superset of `teacher-dump`**, identical on book nodes
//! and upgraded off-book.
//!
//! ```text
//! cargo run --release --features search --example search-dump -- --boards 3000 --seed 1 --progress
//! ```
//!
//! This run saturates every core for hours.  On a shared machine, wrap it in the
//! polite "scavenger" policy (SCHED_IDLE + idle I/O) — see
//! [`docs/shared-machine-data-gen.md`](../../docs/shared-machine-data-gen.md) and
//! `scripts/idle-run.sh`:
//!
//! ```text
//! scripts/idle-run.sh cargo run --release --features search \
//!   --example search-dump -- --boards 10000 --seed 1 --progress
//! ```
//!
//! # The M3.1 measure
//!
//! M3.1's win condition is that the targets *differ from the teacher mainly
//! off-book / contested* (where the books were silent).  So at each row this
//! also classifies the deterministic teacher ([`two_over_one`]) and the raw net
//! prior ([`two_over_one_neural`]) and accumulates, split by **off-book/on-book**
//! and **contested/constructive**:
//!
//! - the **arg-max disagreement rate** — how often the search bids a different
//!   call than the reference (the interpretable headline), and
//! - the mean **total-variation distance** `½·Σ|pᵢ − qᵢ|` between the two
//!   distributions (scale-free, unlike KL on the engineered EV-band logits).
//!
//! On-book rows are `0` on both *by construction* (the search floor never fires,
//! so all three systems return the identical book logits) — a built-in sanity
//! check; off-book rows carry the divergence, and contested should exceed
//! constructive.  The summary is printed and written into the `.json` sidecar.
//!
//! # This is slow
//!
//! Every *off-book* decision runs a full double-dummy search (128 layouts × up
//! to 8 candidates by default, ~1.4 s each), so a board costs many solves.  The
//! default board count is a smoke test, not a dataset; shrink `--layouts` /
//! `--shortlist` for a faster, noisier run, and watch `--progress` tick.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::bidding::array::Logits;
use pons::bidding::context::{Context, relative};
use pons::bidding::features::{FEATURES_LEN, FEATURES_VERSION, features};
use pons::bidding::search_floor::SearchFloor;
use pons::bidding::{Family, Phase, System};
use pons::{two_over_one, two_over_one_neural, two_over_one_search_with};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use std::collections::BTreeMap;
use std::io::{BufWriter, Write};

/// Number of calls in a `Logits` array (the softmax width).
const SOFTMAX_LEN: usize = 38;
/// Floats per training row.
const ROW_LEN: usize = FEATURES_LEN + SOFTMAX_LEN;

#[derive(Parser)]
#[command(about = "Dump (features, search_target) training rows from two_over_one_search()")]
struct Args {
    /// Number of random boards to bid out
    ///
    /// Small by default: the search is slow (~1.4 s per off-book decision).
    /// Scale up — and be patient — for a real dataset.
    #[arg(long, default_value_t = 50)]
    boards: usize,
    /// RNG seed (for reproducibility)
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Output path stem; writes `<out>.f32`, `<out>.json`, and `<out>.tags`
    #[arg(long, default_value = "target/search-data")]
    out: String,
    /// Layouts sampled and solved per off-book decision (the rollout count)
    #[arg(long, default_value_t = SearchFloor::default().layouts)]
    layouts: usize,
    /// Top-k legal calls, by the net prior, actually scored by EV
    #[arg(long, default_value_t = SearchFloor::default().shortlist)]
    shortlist: usize,
    /// EV temperature in points per nat (larger flattens the EV band)
    #[arg(long, default_value_t = SearchFloor::default().temperature)]
    temperature: f32,
    /// Print a progress line to stderr every ~10% of boards while bidding
    #[arg(long)]
    progress: bool,
}

/// The four absolute vulnerabilities, sampled uniformly per board.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

/// One divergence bucket: how the search target differs from a reference.
#[derive(Default, Clone, Copy)]
struct Bucket {
    /// Rows accumulated into this bucket
    rows: u64,
    /// Sum of total-variation distances (mean = `tv_sum / rows`)
    tv_sum: f64,
    /// Rows whose search arg-max differs from the reference's
    disagreements: u64,
}

impl Bucket {
    /// Fold one row's divergence into the bucket.
    fn push(&mut self, tv: f64, disagree: bool) {
        self.rows += 1;
        self.tv_sum += tv;
        self.disagreements += u64::from(disagree);
    }

    /// Mean total-variation distance (`0` for an empty bucket).
    fn mean_tv(self) -> f64 {
        if self.rows == 0 {
            0.0
        } else {
            self.tv_sum / self.rows as f64
        }
    }

    /// Arg-max disagreement rate as a percentage (`0` for an empty bucket).
    fn disagree_pct(self) -> f64 {
        if self.rows == 0 {
            0.0
        } else {
            100.0 * self.disagreements as f64 / self.rows as f64
        }
    }
}

/// Add two buckets (to total across the contested/constructive split).
fn merge(a: Bucket, b: Bucket) -> Bucket {
    Bucket {
        rows: a.rows + b.rows,
        tv_sum: a.tv_sum + b.tv_sum,
        disagreements: a.disagreements + b.disagreements,
    }
}

/// Divergence accumulated over the `[off_book][contested]` partition.
type Grid = [[Bucket; 2]; 2];

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // The system under measurement: the live-search floor with caller-tuned
    // knobs.  Both sides play it; a Stance routes by phase, so one suffices for
    // whichever seat is to act.
    let search = two_over_one_search_with(SearchFloor {
        layouts: args.layouts,
        shortlist: args.shortlist,
        temperature: args.temperature,
    })
    .against(Family::NATURAL);
    // The references for the M3.1 measure: the deterministic teacher and the raw
    // net prior the search starts from.  Both are cheap (no search).
    let teacher = two_over_one().against(Family::NATURAL);
    let net = two_over_one_neural().against(Family::NATURAL);
    let mut rng = StdRng::seed_from_u64(args.seed);

    let f32_path = format!("{}.f32", args.out);
    let json_path = format!("{}.json", args.out);
    let tags_path = format!("{}.tags", args.out);
    let mut writer = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut tags_writer = BufWriter::new(std::fs::File::create(&tags_path)?);

    let mut rows = 0u64;
    let mut contested = 0u64;
    let mut offbook = 0u64;
    let mut forced_pass = 0u64; // decisions the search had no logits for
    let mut call_hist: BTreeMap<String, u64> = BTreeMap::new();
    let mut vs_teacher: Grid = Grid::default();
    let mut vs_net: Grid = Grid::default();
    let mut row = [0f32; ROW_LEN];

    let step = (args.boards / 10).max(1);
    for board in 0..args.boards {
        if args.progress && board > 0 && board % step == 0 {
            eprintln!(
                "  [search-dump] {board}/{} boards, {rows} rows ({offbook} off-book)",
                args.boards,
            );
        }

        let deal = full_deal(&mut rng);
        let dealer = rng.random_range(0..4usize);
        let vul = VULS[rng.random_range(0..4usize)];

        let mut auction = Auction::new();
        while !auction.has_ended() {
            let seat = Seat::ALL[(dealer + auction.len()) % 4];
            let hand = deal[seat];
            let rel = relative(vul, seat);

            // The search floor's logits and where they came from: the root
            // `Always` fallback (`depth == 0`, `fallback` set) is the floor
            // firing — i.e. the books were silent and the search actually ran.
            let Some((logits, provenance)) = search.classify_with_provenance(hand, rel, &auction)
            else {
                forced_pass += 1;
                auction.push(Call::Pass);
                continue;
            };
            let off_book = provenance.depth == 0 && provenance.fallback.is_some();

            // Mask illegal calls; the target is over legal calls only.
            let logits = masked(logits, &auction);
            let Some(softmax) = logits.softmax() else {
                forced_pass += 1;
                auction.push(Call::Pass);
                continue;
            };

            // Record the row: features ++ search_target.
            let context = Context::new(rel, &auction);
            let feats = features(hand, &context);
            row[..FEATURES_LEN].copy_from_slice(&feats);
            row[FEATURES_LEN..].copy_from_slice(&softmax[..]);
            for value in row {
                writer.write_all(&value.to_le_bytes())?;
            }

            // Tags: bit0 = contested phase (as in teacher-dump), bit1 = off-book.
            let contested_row = Phase::of(&auction) != Phase::Constructive;
            let tag = u8::from(contested_row) | (u8::from(off_book) << 1);
            tags_writer.write_all(&[tag])?;
            rows += 1;
            contested += u64::from(contested_row);
            offbook += u64::from(off_book);

            // The M3.1 measure: how far the search target sits from each
            // reference, bucketed by off-book and contested.
            let chosen = argmax_legal(&logits);
            let cell = (usize::from(off_book), usize::from(contested_row));
            accumulate(
                &mut vs_teacher[cell.0][cell.1],
                &softmax,
                chosen,
                &teacher,
                hand,
                rel,
                &auction,
            );
            accumulate(
                &mut vs_net[cell.0][cell.1],
                &softmax,
                chosen,
                &net,
                hand,
                rel,
                &auction,
            );

            // Advance the auction by the search's legal arg-max (self-play): the
            // visited states are the ones the search bidder actually reaches.
            *call_hist.entry(format!("{chosen}")).or_insert(0) += 1;
            auction.push(chosen);
        }
    }
    writer.flush()?;
    tags_writer.flush()?;

    let git_sha = git_sha();
    let metadata = serde_json::json!({
        "feature_version": FEATURES_VERSION,
        "features_len": FEATURES_LEN,
        "softmax_len": SOFTMAX_LEN,
        "row_len": ROW_LEN,
        "row_bytes": ROW_LEN * 4,
        "dtype": "f32-le",
        "layout": "row = [160 features][38 search_target]",
        "tags": "sibling .tags file: one u8 per row, bit0 = contested phase, bit1 = off-book (search fired)",
        "system": "two_over_one_search()",
        "search": { "layouts": args.layouts, "shortlist": args.shortlist, "temperature": args.temperature },
        "measure_references": { "teacher": "two_over_one()", "net": "two_over_one_neural()" },
        "git_sha": git_sha,
        "seed": args.seed,
        "boards": args.boards,
        "rows": rows,
        "offbook_rows": offbook,
        "contested_rows": contested,
        "forced_pass_decisions": forced_pass,
        "measure": {
            "vs_teacher": grid_json(&vs_teacher),
            "vs_net": grid_json(&vs_net),
        },
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
        "search-dump: {rows} rows from {} boards → {f32_path} ({:.1} MB), \
         {offbook} off-book ({:.0}%), {contested} contested ({:.0}%), {forced_pass} forced passes.",
        args.boards,
        (rows as usize * ROW_LEN * 4) as f64 / 1e6,
        pct(offbook),
        pct(contested),
    );
    report_reference(
        "Search target vs the deterministic teacher (two_over_one):",
        &vs_teacher,
    );
    report_reference(
        "Search target vs the raw net prior (two_over_one_neural):",
        &vs_net,
    );

    eprintln!("\ntop advancing calls:");
    let mut hist: Vec<(String, u64)> = call_hist.into_iter().collect();
    hist.sort_by(|a, b| b.1.cmp(&a.1));
    for (call, count) in hist.into_iter().take(12) {
        eprintln!("  {call:>4}  {count:>8}  ({:.1}%)", pct(count));
    }
    Ok(())
}

/// Fold one decision into a reference's divergence bucket.
///
/// Classifies `reference` at the same node, masks it to the legal calls, and
/// compares its distribution against the already-computed search `target`:
/// the total-variation distance and whether the two arg-max calls differ.  A
/// reference that declines to classify (never, in practice — both references
/// carry a floor) contributes nothing.
fn accumulate(
    bucket: &mut Bucket,
    target: &pons::bidding::Array<f32>,
    chosen: Call,
    reference: &pons::bidding::Stance,
    hand: contract_bridge::Hand,
    rel: contract_bridge::auction::RelativeVulnerability,
    auction: &Auction,
) {
    let Some(logits) = reference.classify(hand, rel, auction) else {
        return;
    };
    let logits = masked(logits, auction);
    let Some(reference_softmax) = logits.softmax() else {
        return;
    };
    let tv = total_variation(&target[..], &reference_softmax[..]);
    bucket.push(tv, chosen != argmax_legal(&logits));
}

/// Total-variation distance between two distributions: `½·Σ|pᵢ − qᵢ|`.
///
/// Bounded in `[0, 1]` and scale-free, so it compares the search's engineered
/// EV-band softmax to a reference softmax honestly (unlike KL on mismatched
/// logit scales).  `0` means identical, `1` disjoint support.
fn total_variation(p: &[f32], q: &[f32]) -> f64 {
    0.5 * p
        .iter()
        .zip(q)
        .map(|(a, b)| f64::from((a - b).abs()))
        .sum::<f64>()
}

/// Mask illegal calls to `-∞`, leaving a distribution over the legal calls.
fn masked(mut logits: Logits, auction: &Auction) -> Logits {
    for (call, slot) in logits.iter_mut() {
        if auction.can_push(call).is_err() {
            *slot = f32::NEG_INFINITY;
        }
    }
    logits
}

/// The highest-logit finite (hence legal, after masking) call, defaulting to a
/// pass so the auction always terminates.
fn argmax_legal(logits: &Logits) -> Call {
    logits
        .iter()
        .filter(|(_, l)| l.is_finite())
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
        .map_or(Call::Pass, |(call, _)| call)
}

/// Print one reference's divergence table: off-book (split) vs on-book.
fn report_reference(title: &str, grid: &Grid) {
    let offbook = merge(grid[1][0], grid[1][1]);
    let onbook = merge(grid[0][0], grid[0][1]);
    eprintln!("\n{title}");
    eprintln!("                       rows   argmax≠   mean TV");
    let line = |label: &str, b: Bucket| {
        eprintln!(
            "  {label:<18} {:>7}   {:>5.1}%   {:>7.4}",
            b.rows,
            b.disagree_pct(),
            b.mean_tv(),
        );
    };
    line("off-book", offbook);
    line("  · contested", grid[1][1]);
    line("  · constructive", grid[1][0]);
    line("on-book", onbook);
    eprintln!("  (on-book = identical book logits by construction → expect 0 / 0)");
}

/// One reference's measure as JSON for the sidecar.
fn grid_json(grid: &Grid) -> serde_json::Value {
    let bucket = |b: Bucket| {
        serde_json::json!({
            "rows": b.rows,
            "argmax_disagree_pct": b.disagree_pct(),
            "mean_tv": b.mean_tv(),
        })
    };
    serde_json::json!({
        "off_book": {
            "total": bucket(merge(grid[1][0], grid[1][1])),
            "contested": bucket(grid[1][1]),
            "constructive": bucket(grid[1][0]),
        },
        "on_book": bucket(merge(grid[0][0], grid[0][1])),
    })
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
