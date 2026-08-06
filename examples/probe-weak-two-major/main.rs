//! Enriched A/B for responder's forcing new suit over a weak two.
//!
//! Both arms here have a trigger too narrow for a random-deal A/B: the
//! longest-first tie-break needs two qualifying five-card suits opposite a weak
//! two (~1 board in 10⁴), and the Ogust demotion needs a qualifying major plus
//! diamond support (~6 in 10⁴).  A million random boards would spend the whole
//! run bidding and solving deals that cannot diverge.
//!
//! So the accept test runs on the **raw hands, before the bidder**, and the
//! cost gradient decides the filter order: dealing is nearly free, bidding costs
//! a little, double dummy is expensive.  Deal → cheap hand predicate → bid the
//! survivors → confirm the auction really reached `2M - ?` → solve only the
//! deals whose contracts differ.
//!
//! Two modes, one per change:
//!
//! - `--mode tie` ablates [`set_weak_two_longest_first`] (shipped default-on).
//!   Accepts responder hands with **two or more** qualifying suits outside
//!   opener's, which is exactly when the tie-break can move the call.
//! - `--mode ogust` arms [`set_weak_two_major_priority`] (opt-in).  Accepts a
//!   weak 2♦ opposite a qualifying major *and* two-card diamond support, the
//!   window where the 2.0 Ogust ask outranks the 1.5 new suit.
//!
//! The deal is conditioned on North holding the opening, so the duplicate swap
//! collapses: the comparison is the same deal bid twice, our side feature vs
//! baseline, East/West on the baseline stance both times.
//!
//! Because the population is conditional, so is the headline: read the IMPs per
//! *accepted* deal against the decision table, and use the printed per-board
//! equivalent (conditional × the measured trigger density) when stacking the
//! number against the campaign ledger.
//!
//! ```text
//! cargo run --release --example probe-weak-two-major -- --mode tie   --count 20000
//! cargo run --release --example probe-weak-two-major -- --mode ogust --count 20000
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Rank, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::Accumulator;
use pons::american;
use pons::bidding::Stance;
use pons::bidding::american::{set_weak_two_longest_first, set_weak_two_major_priority};
use pons::bidding::constraint::point_count;
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::bid_out;

/// Which change this run measures
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Ablate the longest-first tie-break (shipped default-on)
    Tie,
    /// Arm the major-over-Ogust priority over 2♦ (opt-in)
    Ogust,
}

/// Enriched A/B for responder's new suit over a weak two
#[derive(Parser)]
struct Args {
    /// Which change to measure
    #[arg(long, value_enum, default_value = "ogust")]
    mode: Mode,

    /// Number of *accepted* deals (deals reaching the trigger)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed base (draw i seeded base+i; fresh per experiment)
    #[arg(long, default_value = "0")]
    seed: u64,

    /// Give up after this many draws even if `--count` is not reached
    #[arg(long, default_value = "200000000")]
    draws: u64,

    /// Print this many divergent boards (auction + contracts) for inspection
    #[arg(long, default_value = "0")]
    show: usize,
}

/// The weak-two openings, in the order the opening table considers them
const WEAK_TWOS: [Suit; 3] = [Suit::Diamonds, Suit::Hearts, Suit::Spades];

/// The one suit a weak two would open on, if any: a lone six-carder in
/// ♦/♥/♠ with the shipped `points(5..=10)` strength
fn weak_two_suit(hand: Hand) -> Option<Suit> {
    let long: Vec<Suit> = Suit::ASC
        .into_iter()
        .filter(|&suit| hand[suit].len() >= 6)
        .collect();
    match long[..] {
        [suit] if WEAK_TWOS.contains(&suit) && (5..=10).contains(&point_count(hand)) => Some(suit),
        _ => None,
    }
}

/// Two of the top three honors — the new suit's quality gate
fn good_suit(hand: Hand, suit: Suit) -> bool {
    [Rank::A, Rank::K, Rank::Q]
        .into_iter()
        .filter(|&rank| hand[suit].contains(rank))
        .count()
        >= 2
}

/// The suits responder could bid as a forcing new suit over `our`
fn qualifying_suits(hand: Hand, our: Suit) -> Vec<Suit> {
    Suit::ASC
        .into_iter()
        .filter(|&suit| suit != our && hand[suit].len() >= 5 && good_suit(hand, suit))
        .collect()
}

/// Would this North/South pair reach the mode's trigger?  Hands only — no
/// bidding, no solving.
fn accepts(mode: Mode, opener: Hand, responder: Hand) -> Option<Suit> {
    let our = weak_two_suit(opener)?;
    if point_count(responder) < 14 {
        return None;
    }
    let qualifying = qualifying_suits(responder, our);
    match mode {
        // Two qualifying suits is exactly when the tie-break has a choice to
        // make; one or none and both arms bid the same call.
        Mode::Tie => (qualifying.len() >= 2).then_some(our),
        // The Ogust collision: 2♦ only, a qualifying major, and the two-card
        // support that makes the 2.0 ask fire.
        Mode::Ogust => (our == Suit::Diamonds
            && responder[Suit::Diamonds].len() >= 2
            && qualifying
                .iter()
                .any(|&suit| Strain::from(suit) > Strain::from(our)))
        .then_some(our),
    }
}

/// One accepted board: the deal, the opening suit, and both arms' auctions
struct Board {
    deal: FullDeal,
    our: Suit,
    feature: Auction,
    baseline: Auction,
}

/// Did the auction actually open the weak two and hear a pass?  The hand
/// predicate approximates the opening table; this is the exact check.
fn opened_weak_two(auction: &Auction, our: Suit) -> bool {
    let calls: Vec<Call> = auction.iter().copied().take(2).collect();
    calls[..] == [Call::from(Bid::new(2, Strain::from(our))), Call::Pass]
}

/// Bid one accepted deal under both arms, keeping it only if the auction
/// reached the face
fn play(
    feature: &Stance,
    baseline: &Stance,
    vul: AbsoluteVulnerability,
    deal: FullDeal,
    our: Suit,
) -> Option<Board> {
    let board = Board {
        deal,
        our,
        feature: bid_out(feature, baseline, true, Seat::North, vul, &deal),
        baseline: bid_out(baseline, baseline, true, Seat::North, vul, &deal),
    };
    (opened_weak_two(&board.feature, our) && opened_weak_two(&board.baseline, our)).then_some(board)
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();

    // Both knobs gate rule *construction*, so a stance is built per arm.
    let feature = match args.mode {
        Mode::Tie => {
            set_weak_two_longest_first(true);
            american().against()
        }
        Mode::Ogust => {
            set_weak_two_major_priority(true);
            let stance = american().against();
            set_weak_two_major_priority(false);
            stance
        }
    };
    let baseline = match args.mode {
        Mode::Tie => {
            set_weak_two_longest_first(false);
            let stance = american().against();
            set_weak_two_longest_first(true);
            stance
        }
        Mode::Ogust => american().against(),
    };

    // Stage 1: deal and reject on the raw hands.  Dealing is the cheapest thing
    // in the pipeline, so this runs until `--count` deals survive.
    let mut accepted: Vec<(FullDeal, Suit)> = Vec::with_capacity(args.count);
    let mut draws = 0u64;
    while accepted.len() < args.count && draws < args.draws {
        let deal = full_deal(&mut StdRng::seed_from_u64(args.seed.wrapping_add(draws)));
        draws += 1;
        if let Some(our) = accepts(args.mode, deal[Seat::North], deal[Seat::South]) {
            accepted.push((deal, our));
        }
    }
    // Stage 2: bid the survivors, and drop any whose auction missed the face.
    let boards: Vec<Board> = accepted
        .par_iter()
        .filter_map(|&(deal, our)| play(&feature, &baseline, args.vulnerability, deal, our))
        .collect();

    // The scaling density is over the boards that *reached the face*, since
    // that is the population the per-board mean is taken over — the raw accept
    // rate would over-count by the hand predicate's slack against the real
    // opening table.
    let density = boards.len() as f64 / draws.max(1) as f64;

    // Stage 3: solve only what diverged.
    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.feature, Seat::North),
                final_contract(&board.baseline, Seat::North),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let solve: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL);

    let mut swings_pd = vec![0i64; boards.len()];
    let mut swings_dd = vec![0i64; boards.len()];
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
            let feature: Vec<Call> = board.feature.iter().copied().collect();
            let baseline: Vec<Call> = board.baseline.iter().copied().collect();
            println!(
                "[{shown}] 2{}  N {}  S {}\n      feature {feature:?} -> {contract_a:?}\n      baseline {baseline:?} -> {contract_b:?}  (PD {:+}, DD {:+})",
                board.our,
                board.deal[Seat::North],
                board.deal[Seat::South],
                imps(points_pd),
                imps(points_dd),
            );
        }
    }

    let label = match args.mode {
        Mode::Tie => "longest-first tie-break (ablation)",
        Mode::Ogust => "major over Ogust on 2♦",
    };
    println!(
        "\n=== Enriched weak-two probe: {label}, vulnerability {}, seed {} ===",
        args.vulnerability, args.seed,
    );
    println!(
        "Draws {draws}, accepted {} on hands, bid to the face {} ({:.5}% trigger density)",
        accepted.len(),
        boards.len(),
        100.0 * density,
    );
    println!(
        "Divergent boards: {} of {} accepted ({:.2}%)",
        divergent.len(),
        boards.len(),
        100.0 * divergent.len() as f64 / boards.len().max(1) as f64,
    );
    for (row, swings) in [
        ("ns_score_pd  (PD)", &swings_pd),
        ("ns_score_cnt (DD)", &swings_dd),
    ] {
        let total: i64 = swings.iter().sum();
        let mut acc = Accumulator::new();
        for &swing in swings {
            acc.push(swing as f64);
        }
        let stats = acc.sample();
        let mean = stats.mean();
        let se = stats.sd() / (boards.len().max(1) as f64).sqrt();
        let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
        let verdict = if (lo..=hi).contains(&0.0) {
            "parity"
        } else if mean > 0.0 {
            "feature ahead"
        } else {
            "feature behind"
        };
        println!(
            "{row}: {total:+} IMPs, {mean:+.5}/accepted deal  95% CI [{lo:+.5}, {hi:+.5}]  ({verdict})\n{:19} {:+.7}/board equivalent  CI [{:+.7}, {:+.7}]",
            "",
            mean * density,
            lo * density,
            hi * density,
        );
    }
}
