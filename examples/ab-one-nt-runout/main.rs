//! Measure the doubled-1NT runout: an A/B duplicate match.
//!
//! When our 1NT is doubled, the [instinct floor][pons::bidding::instinct]
//! normally has nothing to say and responder passes — sitting for an
//! effectively-penalty double on a hand that may be broke.  The runout
//! ([`set_one_nt_runout`][pons::bidding::instinct::set_one_nt_runout]) lets a
//! weak responder escape to its longest five-plus-card suit instead.  Is that
//! worth points?
//!
//! Each board is bid twice, duplicate style: at table A the runout pair sits
//! North/South against a pair that passes (the floor's old behavior); at table
//! B the teams swap seats.  Both pairs play the very same books — the
//! per-call thread-local flip serves both from one stance.  Boards whose two
//! auctions reach different contracts are scored double dummy
//! ([`ns_score_contract`], the actual penalty as bid), and the swing is
//! credited to the runout team.
//!
//! ```text
//! cargo run --release --example ab-one-nt-runout -- --count 200000 --vulnerability both
//! cargo run --release --example ab-one-nt-runout -- --count 20000 --show 8
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::instinct::{set_one_nt_runout, set_one_nt_runout_universal, set_runout_xx_min};
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act};

/// Measure the doubled-1NT runout: an A/B duplicate match
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

    /// HCP floor for responder's XX = to-play (raise to disable XX entirely)
    #[arg(long, default_value = "7")]
    xx_min: u8,

    /// Restrict the runout to responder's direct seat (no opener escape / SOS)
    #[arg(long)]
    no_universal: bool,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,
}

/// Bid out one deal, flipping the runout per acting side
///
/// The thread-local is set just before each classification, so this is safe
/// under rayon: the worker sets and reads it on its own thread.
fn bid_out(
    stance: &Stance,
    runout_is_ns: bool,
    xx_min: u8,
    universal: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        set_one_nt_runout(seat_is_ns == runout_is_ns);
        set_runout_xx_min(xx_min);
        set_one_nt_runout_universal(universal);
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// One board: the deal and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    /// Table A: runout pair sits North/South
    table_a: Auction,
    /// Table B: runout pair sits East/West
    table_b: Auction,
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
            table_a: bid_out(
                &stance,
                true,
                args.xx_min,
                !args.no_universal,
                dealer,
                args.vulnerability,
                &deal,
            ),
            table_b: bid_out(
                &stance,
                false,
                args.xx_min,
                !args.no_universal,
                dealer,
                args.vulnerability,
                &deal,
            ),
        })
        .collect();

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy (on the main thread) and credit the swing to the runout team
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

    let mut total_points = 0i64;
    let mut total_imps = 0i64;
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        total_points += swing;
        total_imps += imps(swing);

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let calls: Vec<Call> = board.table_a.iter().copied().collect();
            println!(
                "[{shown}] dealer {:?}  A {calls:?} -> {contract_a:?}  vs  B -> {contract_b:?}  (swing {swing:+})",
                board.dealer,
            );
        }
    }

    println!(
        "=== Doubled-1NT runout A/B: {} boards, vulnerability {}, xx-min {}, universal {} ===",
        args.count, args.vulnerability, args.xx_min, !args.no_universal,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Runout team: {total_points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
}
