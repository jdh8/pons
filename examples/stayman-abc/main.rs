//! Stayman (1NT–2♣) A/B: a seeded duplicate match across two builds of
//! `american()`.
//!
//! The Stayman authoring is a structural change (new book nodes), not a runtime
//! toggle, so the two arms are two *binaries*: build this example in a worktree
//! at the pre-change commit and bid the boards (`--phase bid`), then build it in
//! the new tree and score the same seeded boards against those contracts
//! (`--phase score`).  Opponents are silenced — this is the constructive value.
//!
//! ```text
//! # baseline worktree (old american):
//! cargo run --release --example stayman-abc -- \
//!     --phase bid --file /tmp/base-none.txt --count 60000 --seed 1 --vulnerability none
//! # new tree (this commit):
//! cargo run --release --example stayman-abc -- \
//!     --phase score --file /tmp/base-none.txt --count 60000 --seed 1 --vulnerability none
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::fs;

#[derive(Parser)]
struct Args {
    /// "bid" (emit contracts) or "score" (compare new vs the emitted baseline)
    #[arg(long)]
    phase: String,
    /// Number of boards (dealer rotates per board)
    #[arg(long, default_value = "60000")]
    count: usize,
    /// Seed for the deterministic deal sequence (must match across both phases)
    #[arg(long, default_value = "1")]
    seed: u64,
    /// Vulnerability: none, ns, ew, both
    #[arg(long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
    /// Baseline contracts file (written by `bid`, read by `score`)
    #[arg(long, default_value = "/tmp/stayman-baseline.txt")]
    file: String,
}

const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
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

/// Bid one deal with East/West forced to pass throughout
fn bid_uncontested(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = if matches!(seat, Seat::East | Seat::West) {
            Call::Pass
        } else {
            next_call(stance, deal[seat], dealer, vul, &auction)
        };
        auction.push(call);
    }
    auction
}

/// The same deterministic board sequence in both phases
fn boards(seed: u64, count: usize) -> Vec<(Seat, FullDeal)> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..count)
        .map(|i| (Seat::ALL[i % 4], full_deal(&mut rng)))
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

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let sys = american().against(Family::NATURAL);
    let boards = boards(args.seed, args.count);

    let contracts: Vec<Option<(Contract, Seat)>> = boards
        .par_iter()
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
        println!("wrote {} contracts to {}", contracts.len(), args.file);
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
        "board count mismatch — same seed/count?"
    );

    let divergent: Vec<usize> = (0..args.count)
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

    println!(
        "=== Stayman A/B: {} boards, vulnerability {} (opponents silenced) ===",
        args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "New Stayman authoring: {points:+} points, {total_imps:+} IMPs ({:+.4} IMPs/board, {:+.3} IMPs/divergent)",
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
}
