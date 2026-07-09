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
//! contracts are scored double dummy, and the swing is credited to the HCP
//! team (so a positive total means HCP beats BUM-RAP).
//!
//! ```text
//! cargo run --example ab-fifths-companion -- --count 1000 --vulnerability ns
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use pons::american;
use pons::bidding::constraint::{
    FifthsCompanion, set_fifths_companion, set_fuzzy_fifths, set_fuzzy_points,
};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, ns_score_contract};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, score_boards, seat_to_act};

/// A/B the Fifths companion gauge: an HCP-vs-BUM-RAP duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
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
/// Both teams keep the shipped fuzzy gauges; only the companion differs.
/// Bidding is single-threaded, so flipping the thread-local just before each
/// classification cleanly serves both teams from one stance.
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
    let mut rng = rand::rng();
    let stance = american().against(Family::NATURAL);

    // Bid every board at both tables, dealer rotating per board.
    let boards: Vec<Board> = (0..args.count)
        .map(|index| {
            let dealer = Seat::ALL[index % 4];
            let deal = full_deal(&mut rng);
            let table_a = bid_out(&stance, true, dealer, args.vulnerability, &deal);
            let table_b = bid_out(&stance, false, dealer, args.vulnerability, &deal);
            Board {
                deal,
                dealer,
                table_a,
                table_b,
            }
        })
        .collect();

    // Only boards whose tables reach different results can swing; solve those
    // double dummy and credit the swing to the HCP team (NS at table A, EW at
    // table B).
    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let deals: Vec<FullDeal> = boards.iter().map(|board| board.deal).collect();
    let scored = score_boards(&contracts, &deals, args.vulnerability, ns_score_contract);
    let (total_points, total_imps) = (scored.total_points, scored.total_imps);

    println!(
        "=== Fifths companion A/B match (HCP vs BUM-RAP): {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.0}%)",
        scored.divergent.len(),
        args.count,
        100.0 * scored.divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "HCP team: {total_points:+} points, {total_imps:+} IMPs ({:+.2} IMPs/board)",
        total_imps as f64 / args.count.max(1) as f64,
    );
}
