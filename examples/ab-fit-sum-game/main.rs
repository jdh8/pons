//! Measure the fit-length-adjusted major-game gate on the instinct floor: an A/B
//! duplicate match.
//!
//! The floor reaches a major game on a *known* eight-plus fit once the combined
//! minimum reaches the
//! [`set_fit_sum_game`][pons::bidding::instinct::set_fit_sum_game] threshold,
//! which folds the known trump length into the point total — the total-tricks
//! yardstick where a ninth trump ≈ a point, so a nine-card fit games a point
//! cheaper and a ten-card fit two cheaper.  An eight-card fit games at the shipped
//! default `31` gate (`23 + 8`); this A/B sweeps the boundary between adjacent
//! thresholds (`31` vs `32`) to isolate a one-point move.
//!
//! `--support-points` arms the opt-in `hcp_plus`
//! [`support_points`][pons::bidding::constraint::set_support_points] scale on
//! *both* sides (the ambient environment, not the treatment): once `fit_sum_game`
//! gauges `support_point_count`, its total is
//! `support_point_count + partner.min + own_len + partner_shown_len`, and the new
//! scale reads shaped hands hotter — re-tune the threshold upward for it (31→32,
//! docs/point-count-threshold-campaign.md).
//!
//! The feature side runs the armed `--threshold`; the baseline side runs
//! `--baseline` (the shipped `31` by default).  Each board is bid twice, duplicate style: at
//! table A the feature pair sits North/South against a baseline pair, at table B
//! the teams swap seats.  The per-call thread-local flip serves both stances from
//! one book.  Divergent boards are scored two ways from the same DD table:
//! [`ns_score_contract`] (plain DD) and [`ns_score_pd`] (perfect defense, which
//! prices a failing game as doubled) — a looser game gate can bid a game that
//! goes down, so the PD column is where an over-loose threshold shows its cost.
//!
//! ```text
//! cargo run --release --example ab-fit-sum-game -- --count 200000 --threshold 33
//! cargo run --release --example ab-fit-sum-game -- --count 200000 --support-points --threshold 32 --baseline 31
//! cargo run --release --example ab-fit-sum-game -- --count 20000 --threshold 33 --vulnerability both --show 8
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::constraint::set_support_points;
use pons::bidding::instinct::set_fit_sum_game;
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, next_call, seat_to_act};

/// Measure the fit-length-adjusted major-game gate: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// The armed fit-sum game threshold for the feature side (32/33/34)
    #[arg(short, long, default_value = "33")]
    threshold: u8,

    /// The reference threshold for the baseline side (the shipped default 31)
    #[arg(short, long, default_value = "31")]
    baseline: u8,

    /// Arm the opt-in `hcp_plus` support-points scale on both sides (the ambient
    /// environment the threshold is re-tuned for; the treatment stays the gate)
    #[arg(long, default_value_t = false)]
    support_points: bool,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (reproducible boards)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,
}

/// Bid out one deal, arming the fit-sum game gate only for the feature side
///
/// The thread-local is set just before each classification, so this is safe under
/// rayon: the worker sets and reads it on its own thread.
fn bid_out(
    stance: &Stance,
    args: &Args,
    feature_is_ns: bool,
    dealer: Seat,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let threshold = if seat_is_ns == feature_is_ns {
            args.threshold
        } else {
            args.baseline
        };
        set_fit_sum_game(threshold);
        // The point-count scale is the shared environment, not the treatment:
        // arm it identically for both sides so only the threshold differs.
        set_support_points(args.support_points);
        auction.push(next_call(
            stance,
            deal[seat],
            dealer,
            args.vulnerability,
            &auction,
        ));
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let stance = american().against(Family::NATURAL);

    // Deal sequentially (seeded, reproducible); bid both tables in parallel.
    let mut rng = StdRng::seed_from_u64(args.seed);
    let deals: Vec<(Seat, FullDeal)> = (0..args.count)
        .map(|index| (Seat::ALL[index % 4], full_deal(&mut rng)))
        .collect();
    let boards: Vec<Board> = deals
        .par_iter()
        .map(|&(dealer, deal)| Board {
            deal,
            dealer,
            table_a: bid_out(&stance, &args, true, dealer, &deal),
            table_b: bid_out(&stance, &args, false, dealer, &deal),
        })
        .collect();

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy (on the main thread) and credit the swing to the feature team
    // (NS at table A, EW at table B).
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
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    // Per-board IMP swing to the feature team (0 on non-divergent boards), scored
    // both ways from the same DD table: plain DD and perfect defense.
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

    println!(
        "\n=== Fit-sum game A/B: {} boards, threshold {} vs {} baseline, vulnerability {} ===",
        args.count, args.threshold, args.baseline, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    for (label, swings) in [
        ("ns_score_pd  (PD)", &swings_pd),
        ("ns_score_cnt (DD)", &swings_dd),
    ] {
        let total: i64 = swings.iter().sum();
        let mut acc = Accumulator::new();
        for &swing in swings.iter() {
            acc.push(swing as f64);
        }
        let stats = acc.sample();
        let mean = stats.mean();
        let se = stats.sd() / (args.count.max(1) as f64).sqrt();
        let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
        let verdict = if (lo..=hi).contains(&0.0) {
            "parity"
        } else if mean > 0.0 {
            "feature ahead"
        } else {
            "feature behind"
        };
        println!(
            "{label}: {total:+} IMPs, {mean:+.3}/board  95% CI [{lo:+.3}, {hi:+.3}]  ({verdict})",
        );
    }
}
