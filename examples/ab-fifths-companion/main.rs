//! A/B the [`fifths`][pons::bidding::constraint::fifths] companion gauge.
//!
//! Initial notrump ranges never gauge Fifths alone — Fifths is tuned for 3NT,
//! so it over-rewards aces and tens and discounts kings and queens.  The value
//! banded is the average of Fifths and an honor-weighted companion: either
//! Milton Work HCP or BUM-RAP.  Which companion bids better?  Each board is bid
//! twice, duplicate style: at table A the HCP-companion pair sits North/South
//! against the BUM-RAP-companion pair; at table B the teams swap seats.  Both
//! pairs play the very same books — only the
//! [`set_fifths_companion`][pons::bidding::constraint::set_fifths_companion]
//! hook differs per acting side.  Boards whose two auctions reach different
//! contracts are solved double dummy once and scored with **both** brackets —
//! plain DD and perfect defense — crediting the swing to the HCP team (so a
//! positive total means HCP beats BUM-RAP).
//!
//! ```text
//! cargo run --example ab-fifths-companion -- --count 1000 --vulnerability ns --seed "$SEED_BASE"
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::constraint::{
    FifthsCompanion, set_fifths_companion, set_fuzzy_fifths, set_fuzzy_points,
};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::final_contract;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{report_brackets, seat_to_act, seeded_deals};

/// A/B the Fifths companion gauge: an HCP-vs-BUM-RAP duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
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

/// Bid out one deal, switching the Fifths companion per acting side
///
/// Both teams keep the shipped fuzzy gauges; only the companion differs.  The
/// companion flag is thread-local and set just before each classification, so
/// this stays correct whether it runs on the main thread or a rayon worker
/// (each board bids on a single thread).
fn bid_out(
    stance: &Stance,
    hcp_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    set_fuzzy_points(true);
    set_fuzzy_fifths(true);

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        set_fifths_companion(if seat_is_ns == hcp_is_ns {
            FifthsCompanion::Hcp
        } else {
            FifthsCompanion::Bumrap
        });
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
            // Credit the HCP team: [off = table_b (HCP EW), on = table_a
            // (HCP NS)], matching report_brackets' on − off (positive = HCP
            // beats BUM-RAP).
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
        "=== Fifths companion A/B match (HCP vs BUM-RAP): {} boards, vulnerability {}, seed {} ===",
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
