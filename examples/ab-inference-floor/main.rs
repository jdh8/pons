//! Measure the inference-aware floor: an A/B duplicate match.
//!
//! The [instinct floor][pons::bidding::instinct] now reads the auction
//! interpretation ([`Inferences`][pons::bidding::Inferences]): in a forced-to-game
//! auction it bids a *known* eight-card major fit rather than a shape-blind
//! 3NT.  Is that worth points?  Each board is bid twice, duplicate style: at
//! table A the inference-aware pair sits North/South against a pair whose floor
//! ignores the interpretation (the pre-inference behavior); at table B the
//! teams swap seats.  Both pairs play the very same books — the
//! [`set_inference_aware`][pons::bidding::instinct::set_inference_aware]
//! ablation hook flips the floor's inference reading per acting side.  Boards
//! whose two auctions reach different contracts are solved double dummy once and
//! scored with **both** brackets — plain DD and perfect defense — crediting the
//! swing to the inference-aware team.
//!
//! ```text
//! cargo run --example ab-inference-floor -- --count 2000 --vulnerability ns --seed "$SEED_BASE"
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::instinct::set_inference_aware;
use pons::bidding::{Family, Stance, System};
use pons::scoring::final_contract;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{report_brackets, seat_to_act, seeded_deals};

/// Measure the inference-aware floor: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Base seed — fresh per experiment (`SEED_BASE=$(date +%s)`), shared
    /// across arms/vuls; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
}

/// The highest-logit *legal* call, defaulting to a pass
fn next_call(
    stance: &Stance,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = stance.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };

    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

/// Bid out one deal, flipping the floor's inference reading per acting side
///
/// The inference-aware flag is thread-local and set just before each
/// classification, so this stays correct whether it runs on the main thread or
/// a rayon worker (each board bids on a single thread).
fn bid_out(
    stance: &Stance,
    aware_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        set_inference_aware(seat_is_ns == aware_is_ns);
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;
    let stance = american().against(Family::NATURAL);

    // Deals are seeded per board (base + index) so every arm/vul replays the
    // identical stream.  Each bid_out sets its own thread-local per call, so
    // board bidding parallelizes; the DD solver stays on the main thread below.
    let deals = seeded_deals(base, args.count);
    let contracts: Vec<[_; 2]> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let table_a = bid_out(&stance, true, dealer, vul, deal);
            let table_b = bid_out(&stance, false, dealer, vul, deal);
            // Credit the inference-aware team: [off = table_b (aware EW),
            // on = table_a (aware NS)], matching report_brackets' on − off.
            [
                final_contract(&table_b, dealer),
                final_contract(&table_a, dealer),
            ]
        })
        .collect();

    // Only boards whose tables reach different results can swing; solve those
    // once and score both brackets (plain DD + perfect defense) from the tables.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    println!(
        "=== Inference-aware floor A/B match: {} boards, vulnerability {}, seed {} ===",
        args.count, vul, base,
    );
    println!(
        "Divergent boards: {} of {} ({:.2}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );

    report_brackets(args.count, &divergent, &tables, &contracts, vul);
}
