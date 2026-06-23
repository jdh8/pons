//! Measure the "settle" view of Pass on the instinct floor: an A/B duplicate match.
//!
//! The [instinct floor][pons::bidding::instinct] used to treat partner's live
//! takeout double as *forcing*: it must advance, climbing even to a doubled `4♣x`
//! on a bust.  The settle floor
//! ([`set_settle_floor`][pons::bidding::instinct::set_settle_floor], **now the
//! default**) recasts Pass as *playing the top bid*: a takeout double is not 100%
//! forcing, so a hand with length behind their doubled suit defends instead of
//! advancing, and a four-level advance becomes a *free bid* requiring values.  This
//! A/B is the measure that promoted it: the feature side runs the settle floor, the
//! baseline side disables it (the old always-advance behavior).
//!
//! Each board is bid twice, duplicate style: at table A the settle pair sits
//! North/South against a pair without it; at table B the teams swap seats.  Both
//! pairs play the very same books — the per-call thread-local flip serves both from
//! one stance.  Boards whose two auctions reach different contracts are scored with
//! [`ns_score_pd`] — **perfect defense, carrying the actual `X`/`XX`** — because the
//! settle floor puts real doubled contracts on the table (it passes to defend), and
//! those cannot be taken back.
//!
//! ```text
//! cargo run --release --example ab-settle-floor -- --count 200000
//! cargo run --release --example ab-settle-floor -- --count 20000 --vulnerability both --show 8
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::instinct::set_settle_floor;
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act};

/// Measure the settle floor: an A/B duplicate match
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
}

/// Bid out one deal, enabling the settle floor only for the feature side
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
        set_settle_floor(seat_is_ns == feature_is_ns);
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

/// One board: the deal and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    /// Table A: settle pair sits North/South
    table_a: Auction,
    /// Table B: settle pair sits East/West
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
            table_a: bid_out(&stance, &args, true, dealer, &deal),
            table_b: bid_out(&stance, &args, false, dealer, &deal),
        })
        .collect();

    // Only boards whose tables reach different contracts can swing; solve those
    // double dummy (on the main thread) and credit the swing to the settle team
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

    // Per-board IMP swing to the settle team (0 on non-divergent boards), scored
    // under perfect defense with the actual penalty carried (`ns_score_pd`).
    let mut swings = vec![0i64; args.count];
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points = ns_score_pd(contract_a, table, args.vulnerability)
            - ns_score_pd(contract_b, table, args.vulnerability);
        swings[index] = imps(points);

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let calls: Vec<Call> = board.table_a.iter().copied().collect();
            println!(
                "[{shown}] dealer {:?}  A {calls:?} -> {contract_a:?}  vs  B -> {contract_b:?}  (swing {:+})",
                board.dealer,
                imps(points),
            );
        }
    }

    let total: i64 = swings.iter().sum();
    let mut acc = Accumulator::new();
    for &swing in &swings {
        acc.push(swing as f64);
    }
    let stats = acc.sample();
    let mean = stats.mean();
    let se = stats.sd() / (args.count.max(1) as f64).sqrt();
    let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);

    println!(
        "\n=== Settle-floor A/B: {} boards, vulnerability {} (scored ns_score_pd) ===",
        args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Settle team: {total:+} IMPs, {mean:+.3} IMPs/board ({:+.3} IMPs/divergent)",
        total as f64 / divergent.len().max(1) as f64,
    );
    println!(
        "  95% CI: [{lo:+.3}, {hi:+.3}]  (SE {se:.3}, n = {})",
        args.count
    );
    let verdict = if (lo..=hi).contains(&0.0) {
        "parity — CI contains 0"
    } else if mean > 0.0 {
        "CI excludes 0, settle floor ahead"
    } else {
        "CI excludes 0, settle floor behind — inspect divergent boards"
    };
    println!("  {verdict}");
}
