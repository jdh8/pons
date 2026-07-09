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
//! whose two auctions reach different contracts are scored double dummy, and
//! the swing is credited to the inference-aware team in points and IMPs.
//!
//! ```text
//! cargo run --example ab-inference-floor -- --count 2000 --vulnerability ns
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::instinct::set_inference_aware;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, ns_score_contract};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, score_boards, seat_to_act};

/// Measure the inference-aware floor: an A/B duplicate match
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "2000")]
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

/// Bid out one deal, flipping the floor's inference reading per acting side
///
/// Bidding is single-threaded here, so flipping the thread-local flag just
/// before each classification cleanly serves both teams from one stance.
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
    // double dummy and credit the swing to the inference-aware team (NS at
    // table A, EW at table B).
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
        "=== Inference-aware floor A/B match: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.0}%)",
        scored.divergent.len(),
        args.count,
        100.0 * scored.divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Inference-aware team: {total_points:+} points, {total_imps:+} IMPs ({:+.2} IMPs/board)",
        total_imps as f64 / args.count.max(1) as f64,
    );
}
