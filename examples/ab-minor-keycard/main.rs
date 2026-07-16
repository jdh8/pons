//! Measure the plain-4NT minor keycard (`set_minor_keycard`): an A/B duplicate match.
//!
//! The feature side keycards agreed minors (strong-2♣ minor raise asks with
//! 28+, inverted-minor responders ask over the 18–19 3NT on
//! `support_points(14..)`); the baseline side plays the pre-keycard book
//! (blind 6m jump on 27+, 3NT top-out).  The original ship A/B used a
//! worktree revert of `99da1b3` as the off arm; the knob replaces it so the
//! measure is reproducible.  Divergence is ~1 in 50k boards — run millions.
//!
//! Each board is bid twice, duplicate style (feature NS at table A, EW at
//! table B), divergent boards solved once and scored plain DD + perfect
//! defense; `--sd` adds the sd-declarer playout row (the pessimist bracket
//! for slam aggression — the misguess seam is exactly what a keycard slam
//! hinges on).
//!
//! ```text
//! cargo run --release --example ab-minor-keycard -- --count 10000000 --sd
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::set_minor_keycard;
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, bid_out, seeded_deals};

/// Measure the plain-4NT minor keycard: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "1000000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed base (board i seeded base+i; fresh per experiment)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,

    /// Add the sd-declarer playout row (blind lead + fallible declarer)
    #[arg(long, default_value_t = false)]
    sd: bool,

    /// Worlds per blind lead and per declarer decision (with --sd)
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,

    /// Seed for the sd world-sampling RNG (report it to reproduce a run)
    #[arg(long, default_value_t = 20_240_607)]
    sd_seed: u64,
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    // The knob gates node *insertion*, so it is read at book construction —
    // build one stance per arm (a per-call thread-local flip would be a
    // no-op on an already-built book).
    set_minor_keycard(true);
    let feature = american().against(Family::NATURAL);
    set_minor_keycard(false);
    let baseline = american().against(Family::NATURAL);
    set_minor_keycard(true);

    let deals: Vec<(Seat, FullDeal)> = seeded_deals(args.seed, args.count)
        .into_iter()
        .enumerate()
        .map(|(index, deal)| (Seat::ALL[index % 4], deal))
        .collect();
    let boards: Vec<Board> = deals
        .par_iter()
        .map(|&(dealer, deal)| Board {
            deal,
            dealer,
            table_a: bid_out(&feature, &baseline, true, dealer, args.vulnerability, &deal),
            table_b: bid_out(
                &feature,
                &baseline,
                false,
                dealer,
                args.vulnerability,
                &deal,
            ),
        })
        .collect();

    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let solve: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&solve, NonEmptyStrainFlags::ALL);

    let mut swings_pd = vec![0i64; args.count];
    let mut swings_dd = vec![0i64; args.count];
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points_pd = ns_score_pd(contract_a, table, args.vulnerability)
            - ns_score_pd(contract_b, table, args.vulnerability);
        let points_dd = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        swings_pd[index] = imps(points_pd);
        swings_dd[index] = imps(points_dd);

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let calls: Vec<Call> = board.table_a.iter().copied().collect();
            println!(
                "[{shown}] dealer {:?}  A {calls:?} -> {contract_a:?}  vs  B -> {contract_b:?}  (PD {:+}, DD {:+})",
                board.dealer,
                imps(points_pd),
                imps(points_dd),
            );
        }
    }

    // The sd-declarer playout row: reading uses the feature stance for both
    // tables (the knob only gates book *nodes*; range reading is shared).
    let swings_sd = args.sd.then(|| {
        let mut rng = StdRng::seed_from_u64(args.sd_seed);
        let mut swings = vec![0i64; args.count];
        for &index in &divergent {
            let board = &boards[index];
            let [a, b] = [&board.table_a, &board.table_b].map(|auction| {
                common::sd_declarer_ns_score(
                    auction,
                    board.dealer,
                    &board.deal,
                    &feature,
                    args.vulnerability,
                    &mut rng,
                    args.sd_worlds,
                    args.sd_worlds,
                )
            });
            swings[index] = imps(a - b);
        }
        swings
    });

    println!(
        "\n=== Minor-keycard A/B: {} boards, vulnerability {}, seed {} ===",
        args.count, args.vulnerability, args.seed,
    );
    println!(
        "Divergent boards: {} of {} ({:.4}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    let mut rows = vec![
        ("ns_score_pd  (PD)", &swings_pd),
        ("ns_score_cnt (DD)", &swings_dd),
    ];
    if let Some(swings) = &swings_sd {
        rows.push(("sd-declarer (SD)", swings));
    }
    for (label, swings) in rows {
        let total: i64 = swings.iter().sum();
        let mut acc = Accumulator::new();
        for &swing in swings.iter() {
            acc.push(swing as f64);
        }
        let stats = acc.sample();
        let mean = stats.mean();
        let se = stats.sd() / (args.count.max(1) as f64).sqrt();
        let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
        let per_div = total as f64 / divergent.len().max(1) as f64;
        let verdict = if (lo..=hi).contains(&0.0) {
            "parity"
        } else if mean > 0.0 {
            "feature ahead"
        } else {
            "feature behind"
        };
        println!(
            "{label}: {total:+} IMPs, {mean:+.5}/board  95% CI [{lo:+.5}, {hi:+.5}]  {per_div:+.2}/divergent  ({verdict})",
        );
    }
}
