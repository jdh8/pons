//! Paired delta of two aligned `bba-gen` dumps — the feature value on a rare auction.
//!
//! When an A/B feature fires on only a sliver of boards (e.g. the doubler's runout
//! from `[1NT, X, XX, P, P]`), scoring each whole dump against BBA and subtracting
//! wastes ~99% of the double-dummy budget on boards the feature never touches. But
//! if the two dumps were generated with the **same seed** (so they share every deal
//! and the BBA reference table), the only boards that differ are the ones the
//! feature fired on, and the shared BBA table cancels in the subtraction. So the
//! per-board delta is just our own score with the feature minus without it:
//! `ns_score(our `on` contract) − ns_score(our `off` contract)`.
//!
//! This reads the two dumps, pairs each board's `table_a` (our pair, North/South)
//! contracts as `(on, off)`, and hands them to [`score_boards`] — which solves only
//! the boards where the two contracts differ. Positive IMPs ⇒ the `on` feature beat
//! the `off` baseline on the boards it touched.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-diff -- on.json off.json
//! ```

use clap::Parser;
use contract_bridge::AbsoluteVulnerability;
use pons::scoring::{final_contract, ns_score_contract, ns_score_pd};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Dump, mean_with_ci, score_boards};

#[derive(Parser)]
struct Args {
    /// Dump bid with the feature ON (its `table_a` is the measured contract)
    on: String,
    /// Dump bid with the feature OFF, same seed/deals (the baseline contract)
    off: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// Show this many of the biggest swings (each way)
    #[arg(long, default_value_t = 8)]
    show: usize,
    /// Scorer: `plain` = honest double-dummy (was the only mode); `pd` =
    /// perfect-defense doubling, which prices a failing contract as doubled.
    /// For a *competitive* feature, a `plain` win that `pd` erases is the
    /// light-sacrifice / doubling artifact (see `reference_pd-vs-plain-dd-bracket`).
    #[arg(long, default_value = "plain")]
    score: String,
}

fn main() {
    let args = Args::parse();
    let on: Dump = serde_json::from_reader(std::io::BufReader::new(
        std::fs::File::open(&args.on).expect("open ON dump"),
    ))
    .expect("parse ON dump");
    let off: Dump = serde_json::from_reader(std::io::BufReader::new(
        std::fs::File::open(&args.off).expect("open OFF dump"),
    ))
    .expect("parse OFF dump");
    assert_eq!(on.boards.len(), off.boards.len(), "dumps must be aligned");
    let vul = args.vulnerability.unwrap_or(on.vulnerability);

    // Pair our (table_a) contract with the feature on vs off; the deals must match.
    let mut deals = Vec::with_capacity(on.boards.len());
    let contracts: Vec<_> = on
        .boards
        .iter()
        .zip(&off.boards)
        .map(|(a, b)| {
            assert_eq!(a.deal, b.deal, "dumps not seed-aligned");
            deals.push(a.deal);
            (
                final_contract(&a.table_a, a.dealer),
                final_contract(&b.table_a, b.dealer),
            )
        })
        .collect();

    let scorer = match args.score.as_str() {
        "plain" => ns_score_contract,
        "pd" => ns_score_pd,
        other => panic!("--score must be plain|pd, got {other:?}"),
    };
    let scored = score_boards(&contracts, &deals, vul, scorer);
    let (mean, ci) = mean_with_ci(&scored.board_imps);
    let n = on.boards.len();
    let d = scored.divergent.len();
    println!(
        "ON {} vs OFF {} ({} boards, vul {vul}): {} fired ({:.2}%)",
        on.our_label,
        off.our_label,
        n,
        d,
        100.0 * d as f64 / n.max(1) as f64,
    );
    println!(
        "Delta (run − sit): {:+} IMPs, {:+.4} IMPs/board [95% CI ±{:.4}], {:+.3} IMPs/fired",
        scored.total_imps,
        mean,
        ci,
        scored.total_imps as f64 / d.max(1) as f64,
    );

    let mut swings = scored.swings.clone();
    swings.sort_by_key(|&(_, _, imp)| imp);
    let show = args.show.min(swings.len());
    if show > 0 {
        println!("--- {show} worst (for the feature) ---");
        for &(i, _, imp) in swings.iter().take(show) {
            let b = &on.boards[i];
            println!(
                "[{imp:+} IMP] {}\n  on:  {}\n  off: {}",
                b.deal, b.table_a, off.boards[i].table_a
            );
        }
    }
}
