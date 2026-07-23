//! Measure Edit 2 — the notrump milestones valued on raw HCP — on the instinct
//! floor: an A/B duplicate match.
//!
//! The floor's 3NT/6NT/7NT milestones ([`combined_hcp`]) value both hands on the
//! length-upgraded `point_count` by default.  A long-suit bonus is a ruffing
//! value worth nothing in notrump, so it overcounts there — a 5-card suit can
//! carry a hand into 3NT the raw HCP would leave in a partscore.
//! [`set_nt_hcp_read`][pons::bidding::instinct::set_nt_hcp_read] switches those
//! milestones to raw HCP (own hand, and partner's crisp `hcp` gauge with the
//! length-scale fallback).
//!
//! The feature side arms the knob; the baseline side is the shipped default.
//! Each board is bid twice, duplicate style: at table A the feature pair sits
//! North/South, at table B the teams swap seats.  Only the notrump milestones
//! differ, so every divergent board is a notrump game/slam decision.  Divergent
//! boards are scored two ways from the same DD table: [`ns_score_contract`]
//! (plain DD) and [`ns_score_pd`] (perfect defense) — the failure mode to watch
//! is the *opposite* of a loose gate: raw HCP is **tighter**, so watch for
//! notrump games no longer reached that the length overcount used to buy.
//!
//! ```text
//! cargo run --release --example ab-nt-hcp -- --count 200000
//! cargo run --release --example ab-nt-hcp -- --count 200000 --vulnerability both
//! cargo run --release --example ab-nt-hcp -- --count 20000 --vulnerability both --show 8
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::instinct::{set_bilans_floor, set_nt_hcp_read};
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, next_call, seat_to_act};

/// Measure the raw-HCP notrump milestones: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (reproducible boards)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,

    /// Disable the bilans net floor on both sides (the ambient environment): the
    /// point-arithmetic `combined_hcp` milestone is shadowed by the net floor
    /// under the shipped default, so the raw-HCP read only bites with it off.
    #[arg(long, default_value_t = false)]
    no_bilans: bool,
}

/// Bid out one deal, arming the raw-HCP notrump read only for the feature side
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
        // Ambient environment (both sides): optionally expose the point-arithmetic
        // milestone by disabling the net floor that otherwise shadows it.
        set_bilans_floor(!args.no_bilans);
        // Edit 2 treatment: raw-HCP notrump milestones fire only for the feature side.
        set_nt_hcp_read(seat_is_ns == feature_is_ns);
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
        "\n=== Notrump raw-HCP A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
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
