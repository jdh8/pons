//! Source-of-tricks eight A/B: does forcing 3NT beat the transfer? (No.)
//!
//! Bridge theory says an 8-count with a running long minor should "gamble 3NT on a
//! source of tricks".  An analytic screen agreed — pricing 3NT against a *notrump*
//! invite/pass looked worth +0.2 to +0.5 IMPs/board.  But that baseline is a
//! fiction: these hands do not stop in notrump, they **transfer**, and the transfer
//! reaches the suit game.  This A/B measures the truth by bidding every qualifying
//! board with [`set_long_minor_force`] **off** (the shipped transfer routing) and
//! **on** (the 3NT force) and scoring the *real* contracts each arm reaches.
//!
//! Result (8M deals, plain DD, vul none): **−7.12 IMPs/fired.**  Club source
//! (−7.07) is a disaster — the `2♠` transfer drives to a *making 5♣* that the 3NT
//! force throws away; diamond source is a wash — the `2NT` transfer already reaches
//! 3NT.  So the force stays **off by default**; this example is why.
//!
//! The knob is read at book-construction time, so we build two systems rather than
//! toggling per board.  Opponents are silent — constructive value only.
//!
//! ```text
//! cargo run --release --example ab-long-minor-force -- --count 8000000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Rank, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::set_long_minor_force;
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, hand_hcp, mean_with_ci, seat_to_act};

/// Source-of-tricks eight A/B: force 3NT vs transfer/invite
#[derive(Parser)]
struct Args {
    /// Number of random deals to scan for the qualifying responder scenario
    #[arg(short, long, default_value = "8000000")]
    count: u64,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (defaults to a fresh random base each run)
    #[arg(short, long)]
    seed: Option<u64>,
}

/// Top honors (A/K/Q) held in one suit (0..=3).
fn top_honors(hand: Hand, suit: Suit) -> usize {
    [Rank::A, Rank::K, Rank::Q]
        .iter()
        .filter(|&&r| hand[suit].contains(r))
        .count()
}

/// A minor that runs: 7+ cards, or 6 cards headed by two of the top three honors.
fn minor_source(hand: Hand, minor: Suit) -> bool {
    let len = hand[minor].len();
    len >= 7 || (len >= 6 && top_honors(hand, minor) >= 2)
}

/// The shipped force shape: an 8-count, no four/five-card major, running long minor.
fn source_eight(hand: Hand) -> bool {
    hand_hcp(hand) == 8
        && hand[Suit::Spades].len() <= 3
        && hand[Suit::Hearts].len() <= 3
        && (minor_source(hand, Suit::Clubs) || minor_source(hand, Suit::Diamonds))
}

/// The seat of the first bidder and its opening bid, or `None` for a pass-out.
fn opening(auction: &Auction, dealer: Seat) -> Option<(Seat, Bid)> {
    auction
        .into_iter()
        .enumerate()
        .find_map(|(i, &call)| match call {
            Call::Bid(bid) => Some((seat_to_act(dealer, i), bid)),
            _ => None,
        })
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let one_nt = Bid::new(1, Strain::Notrump);
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // Two systems: the knob is read when the book is built, so build one each way.
    set_long_minor_force(false);
    let sys_off = american().against(Family::NATURAL);
    set_long_minor_force(true);
    let sys_on = american().against(Family::NATURAL);

    // Scan for a qualifying eight opposite partner's 1NT, bid both arms, keep the
    // deal plus each arm's final contract when they diverge.
    type Diverged = (FullDeal, Option<(Contract, Seat)>, Option<(Contract, Seat)>);
    let divergent: Vec<Diverged> = (0..args.count)
        .into_par_iter()
        .filter_map(|i| {
            let mut rng = StdRng::seed_from_u64(base.wrapping_add(i));
            let deal = full_deal(&mut rng);
            let dealer = Seat::ALL[(i % 4) as usize];

            let responder = [Seat::North, Seat::South]
                .into_iter()
                .find(|&s| source_eight(deal[s]))?;

            // The opening auction is arm-independent (opener acts before responder),
            // so establish the 1NT open once with either system.
            let auction_off = bid_uncontested(&sys_off, dealer, vul, &deal);
            let (opener, first) = opening(&auction_off, dealer)?;
            if first != one_nt || Seat::ALL[(opener as usize + 2) % 4] != responder {
                return None;
            }

            let off = final_contract(&auction_off, dealer);
            let on = final_contract(&bid_uncontested(&sys_on, dealer, vul, &deal), dealer);
            (off != on).then_some((deal, off, on))
        })
        .collect();

    // Solve the divergent deals once; score each arm's real contract.
    let deals: Vec<FullDeal> = divergent.iter().map(|(d, ..)| *d).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);
    let swings: Vec<i64> = divergent
        .iter()
        .zip(&tables)
        .map(|((_, off, on), table)| {
            imps(ns_score_contract(*on, table, vul) - ns_score_contract(*off, table, vul))
        })
        .collect();

    // Split the swing by which minor is the source — clubs (2♠ transfer, drives to
    // 5♣) behave very differently from diamonds (2NT transfer).
    for (label, want_clubs) in [("club-source", true), ("diamond-source", false)] {
        let sub: Vec<i64> = divergent
            .iter()
            .zip(&swings)
            .filter(|((deal, ..), _)| {
                let r = [Seat::North, Seat::South]
                    .into_iter()
                    .find(|&s| source_eight(deal[s]))
                    .expect("qualified at scan time");
                minor_source(deal[r], Suit::Clubs) == want_clubs
            })
            .map(|(_, &s)| s)
            .collect();
        let (m, c) = mean_with_ci(&sub);
        println!(
            "  {label:<14}: {:>5} fired, {m:+.4} IMPs/fired (95% CI {c:+.4})",
            sub.len()
        );
    }
    println!();

    let (mean, ci) = mean_with_ci(&swings);
    let total: i64 = swings.iter().sum();
    println!(
        "=== source-of-tricks eight: force 3NT vs transfer/invite — {} deals, vul {} ===",
        args.count, vul,
    );
    println!("(+ ⇒ FORCE wins; opponents silent, plain DD)\n");
    println!("Divergent (fired) boards: {}", swings.len());
    println!("Force - baseline: {total:+} IMPs total, {mean:+.4} IMPs/fired (95% CI {ci:+.4})",);
}
