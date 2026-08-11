//! Enriched A/B for the exact weak `(1♣) 2♦` overcall.
//!
//! The raw South hand is rejected before either bidder runs. East then opens
//! with the baseline system, South uses the feature or baseline arm, and only
//! auctions whose first two calls expose the exact `(1♣) 2♦` face are retained.
//!
//! ```text
//! cargo run --release --example probe-defensive-overcalls
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Partnership;
use pons::bidding::agreements::Agreements;
use pons::bidding::constraint::point_count;
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, hand_hcp, mean_with_ci};

/// Enriched A/B for the O3 exact minor weak-jump candidate
#[derive(Parser)]
struct Args {
    /// Number of raw South hands to accept before bidding
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed base (draw i is seeded base+i)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Give up after this many raw deals
    #[arg(long, default_value = "200000000")]
    draws: u64,

    /// Print this many divergent boards
    #[arg(long, default_value = "0")]
    show: usize,
}

/// Cheap enrichment predicate: South only, before any bidding.
fn accepts(hand: Hand) -> bool {
    let hcp = hand_hcp(hand);
    hand[Suit::Diamonds].len() == 6 && point_count(hand) >= 8 && hcp <= 11
}

fn prefix(auction: &Auction) -> Option<(Bid, Call)> {
    let mut calls = auction.iter().copied();
    let Call::Bid(opening) = calls.next()? else {
        return None;
    };
    Some((opening, calls.next()?))
}

/// The feature must expose its exact direct face; the control must share the
/// same one-level suit opening and take a different action.
fn exact_face(feature: &Auction, baseline: &Auction) -> Option<Suit> {
    let ((opening, action), (base_opening, base_action)) = (prefix(feature)?, prefix(baseline)?);
    let suit = opening.strain.suit()?;
    (opening.level.get() == 1
        && opening == base_opening
        && action == Call::from(Bid::new(2, Strain::Diamonds))
        && base_action != action
        && suit == Suit::Clubs)
        .then_some(suit)
}

struct Board {
    deal: FullDeal,
    opening: Suit,
    feature: Auction,
    baseline: Auction,
}

fn play(
    feature: &Partnership,
    baseline: &Partnership,
    vul: AbsoluteVulnerability,
    deal: FullDeal,
) -> Option<Board> {
    // North/South are our side in both arms; East/West are always baseline.
    let feature_auction = bid_out(feature, baseline, true, Seat::East, vul, &deal);
    let baseline_auction = bid_out(baseline, baseline, true, Seat::East, vul, &deal);
    let opening = exact_face(&feature_auction, &baseline_auction)?;
    Some(Board {
        deal,
        opening,
        feature: feature_auction,
        baseline: baseline_auction,
    })
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut baseline_agreements = Agreements::default();
    baseline_agreements.defense.direct_minor_weak_jump_overcall = false;
    let baseline = american(&baseline_agreements).bind();
    let mut feature_agreements = Agreements::default();
    feature_agreements.defense.direct_minor_weak_jump_overcall = true;
    let feature = american(&feature_agreements).bind();

    let mut accepted = Vec::with_capacity(args.count);
    let mut draws = 0u64;
    while accepted.len() < args.count && draws < args.draws {
        let deal = full_deal(&mut StdRng::seed_from_u64(args.seed.wrapping_add(draws)));
        draws += 1;
        if accepts(deal[Seat::South]) {
            accepted.push(deal);
        }
    }

    let boards: Vec<Board> = accepted
        .par_iter()
        .filter_map(|&deal| play(&feature, &baseline, args.vulnerability, deal))
        .collect();
    let density = boards.len() as f64 / draws.max(1) as f64;

    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.feature, Seat::East),
                final_contract(&board.baseline, Seat::East),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut swings_dd = vec![0i64; boards.len()];
    let mut swings_pd = vec![0i64; boards.len()];
    let mut shown = 0;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_feature, contract_baseline) = contracts[index];
        let points_dd = ns_score_contract(contract_feature, table, args.vulnerability)
            - ns_score_contract(contract_baseline, table, args.vulnerability);
        let points_pd = ns_score_pd(contract_feature, table, args.vulnerability)
            - ns_score_pd(contract_baseline, table, args.vulnerability);
        swings_dd[index] = imps(points_dd);
        swings_pd[index] = imps(points_pd);

        if shown < args.show {
            shown += 1;
            let board = &boards[index];
            let feature: Vec<Call> = board.feature.iter().copied().collect();
            let baseline: Vec<Call> = board.baseline.iter().copied().collect();
            println!(
                "[{shown}] (1{}) South {}\n      feature {feature:?} -> {contract_feature:?}\n      baseline {baseline:?} -> {contract_baseline:?}  (DD {:+}, PD {:+})",
                board.opening,
                board.deal[Seat::South],
                imps(points_dd),
                imps(points_pd),
            );
        }
    }

    println!(
        "\n=== Enriched defensive-overcall probe: weak (1♣) 2♦, vulnerability {}, seed {} ===",
        args.vulnerability, args.seed,
    );
    println!(
        "Draws {draws}, raw South matches {}, exact direct faces {} ({:.5}% trigger density)",
        accepted.len(),
        boards.len(),
        100.0 * density,
    );
    println!(
        "Contract divergences: {} of {} faces ({:.2}%)",
        divergent.len(),
        boards.len(),
        100.0 * divergent.len() as f64 / boards.len().max(1) as f64,
    );
    for (label, swings) in [("DD", &swings_dd), ("PD", &swings_pd)] {
        let total: i64 = swings.iter().sum();
        let (mean, half_width) = mean_with_ci(swings);
        println!(
            "{label}: {total:+} IMPs, {mean:+.5}/face ± {half_width:.5}; {:+.7}/board equivalent ± {:.7}",
            mean * density,
            half_width * density,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hand(text: &str) -> Hand {
        text.parse().expect("valid test hand")
    }

    #[test]
    fn raw_filter_pins_strength_and_weak_diamond_length() {
        assert!(accepts(hand("K32.43.AJT987.42")));
        assert!(!accepts(hand("Q32.K3.AQJ987.42")));
        assert!(!accepts(hand("K32.43.AJT98.432")));
    }
}
