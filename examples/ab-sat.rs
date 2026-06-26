//! Revised South African Texas A/B — this build of `american()` vs the prior one.
//!
//! The SAT authoring is a structural change (new book nodes), not a runtime
//! toggle, so the two arms are two *binaries* (like `stayman-abc`): bid the
//! boards with the prior tree (`--phase bid`), then score the same seeded boards
//! with the new tree (`--phase score`).  The match is filtered to the only
//! configuration that can diverge — a 15–17 balanced 1NT opener opposite a
//! 6-card-major (game-or-better) responder — so a large board count stays cheap.
//! Opponents are silent (constructive value).
//!
//! ```text
//! git stash                              # revert to the prior american()
//! cargo run --release --example ab-sat -- --phase bid \
//!     --file /tmp/sat-none.txt --seed 1 --count 10000000 --vulnerability none
//! git stash pop                          # restore the gadget
//! cargo run --release --example ab-sat -- --phase score \
//!     --file /tmp/sat-none.txt --seed 1 --count 10000000 --vulnerability none
//! ```
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use clap::Parser;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat, Suit, eval};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Tag;
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::fs;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::bid_uncontested;

#[derive(Parser)]
struct Args {
    /// "bid" (emit contracts) or "score" (compare new vs the emitted baseline)
    #[arg(long)]
    phase: String,
    /// Number of random deals to sample (qualifying configs are a small %)
    #[arg(long, default_value = "10000000")]
    count: usize,
    /// Seed for the deterministic deal sequence (must match across both phases)
    #[arg(long, default_value = "1")]
    seed: u64,
    /// Vulnerability: none, ns, ew, both
    #[arg(long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
    /// Baseline contracts file (written by `bid`, read by `score`)
    #[arg(long, default_value = "/tmp/sat-ab.txt")]
    file: String,
}

const SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

fn hcp(hand: Hand) -> u8 {
    SUITS.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

fn slen(hand: Hand, suit: Suit) -> u8 {
    hand[suit].len() as u8
}

const fn partner(seat: Seat) -> Seat {
    Seat::ALL[(seat as usize + 2) % 4]
}

/// A 15–17 balanced hand — a textbook strong-1NT opener
fn strong_notrump(hand: Hand) -> bool {
    if !(15..=17).contains(&hcp(hand)) {
        return false;
    }
    let lens = SUITS.map(|s| slen(hand, s));
    let max = lens.iter().copied().max().unwrap_or(0);
    let min = lens.iter().copied().min().unwrap_or(0);
    min >= 2 && max <= 5 && lens.iter().filter(|&&l| l == 2).count() <= 1
}

/// A 6-card major with game-or-better values — the hands the new 4-level
/// structure (to-play `4♣/4♦`, slam-try `4♥/4♠`) can route differently
fn six_card_major(hand: Hand) -> bool {
    hcp(hand) >= 9 && (slen(hand, Suit::Hearts) >= 6 || slen(hand, Suit::Spades) >= 6)
}

/// The opener's seat for a qualifying board in the N/S partnership, else `None`
fn opener_seat(deal: &FullDeal) -> Option<Seat> {
    [Seat::North, Seat::South]
        .into_iter()
        .find(|&opener| strong_notrump(deal[opener]) && six_card_major(deal[partner(opener)]))
}

/// Deterministic qualifying boards `(dealer = opener, deal)` from the seed
fn boards(seed: u64, count: usize) -> Vec<(Seat, FullDeal)> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count)
        .filter_map(|_| {
            let deal = full_deal(&mut rng);
            opener_seat(&deal).map(|opener| (opener, deal))
        })
        .collect()
}

/// Serialize a final contract: `PASS`, or e.g. `4H N`
fn encode(result: Option<(Contract, Seat)>) -> String {
    match result {
        None => "PASS".to_owned(),
        Some((contract, seat)) => format!("{contract} {seat}"),
    }
}

/// Parse a line written by [`encode`]
fn decode(line: &str) -> Option<(Contract, Seat)> {
    let line = line.trim();
    if line == "PASS" {
        return None;
    }
    let (contract, seat) = line.split_once(' ').expect("contract seat");
    Some((
        contract.parse().expect("valid contract"),
        seat.parse().expect("valid seat"),
    ))
}

fn main() {
    let args = Args::parse();
    let sys = american().against(Tag::NATURAL);
    let boards = boards(args.seed, args.count);

    let contracts: Vec<Option<(Contract, Seat)>> = boards
        .iter()
        .map(|(dealer, deal)| {
            final_contract(
                &bid_uncontested(&sys, *dealer, args.vulnerability, deal),
                *dealer,
            )
        })
        .collect();

    if args.phase == "bid" {
        let body: String = contracts.iter().map(|c| encode(*c) + "\n").collect();
        fs::write(&args.file, body).expect("write baseline");
        println!(
            "wrote {} qualifying contracts ({} deals sampled) to {}",
            contracts.len(),
            args.count,
            args.file,
        );
        return;
    }

    assert_eq!(args.phase, "score", "phase must be bid or score");
    let baseline: Vec<Option<(Contract, Seat)>> = fs::read_to_string(&args.file)
        .expect("read baseline")
        .lines()
        .map(decode)
        .collect();
    assert_eq!(
        baseline.len(),
        contracts.len(),
        "qualifying-board count mismatch — same seed/count?"
    );

    let divergent: Vec<usize> = (0..contracts.len())
        .filter(|&i| baseline[i] != contracts[i])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| boards[i].1).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let base = ns_score_contract(baseline[i], table, args.vulnerability);
        let new = ns_score_contract(contracts[i], table, args.vulnerability);
        points += new - base;
        total_imps += imps(new - base);
    }

    let qualifying = contracts.len();
    println!(
        "=== Revised SAT A/B (new american vs prior): vulnerability {} ===",
        args.vulnerability,
    );
    println!(
        "Qualifying boards: {} (from {} sampled deals)",
        qualifying, args.count,
    );
    println!(
        "Divergent: {} ({:.1}% of qualifying)",
        divergent.len(),
        100.0 * divergent.len() as f64 / qualifying.max(1) as f64,
    );
    println!(
        "New SAT: {points:+} points, {total_imps:+} IMPs ({:+.4} IMPs/qualifying-board, {:+.3} IMPs/divergent)",
        total_imps as f64 / qualifying.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
}
