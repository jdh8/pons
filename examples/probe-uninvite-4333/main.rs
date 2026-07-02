//! How high should a flat (4-3-3-3) hand bid over partner's 1NT?
//!
//! A flat 4-3-3-3 has no ruff and no long suit — its tricks are exactly its high
//! cards — so it is the shape most likely to be over-bid by HCP.  This probe
//! prices, double-dummy, the responder's three choices over a 15-17 1NT opening
//! for both the invitational **eight** and the game-forcing **nine**:
//!
//!   * **pass** — play 1NT,
//!   * **invite** — the `2♠` size-ask (opener bids `2NT` with 15-16, `3NT` with 17;
//!     the invite therefore lands `2NT` opposite a minimum, `3NT` opposite a max),
//!   * **force** — jump to `3NT`.
//!
//! The system today *invites* the eight and *forces* the nine.  A positive swing
//! below means the **lower** action wins (opponents silent — constructive value).
//!
//! The `2♠` continuation book only knows the eight, so a nine cannot be routed
//! through it live; instead the invite/force/pass contracts are priced analytically
//! from opener's HCP.  That analytic invite was validated to equal the system's own
//! auction on all 21165 eights *before* the flat-4333 eight was switched to Pass
//! (0/21165 mismatch, recorded in the CHANGELOG).  Now that the switch has shipped,
//! the printed check verifies the *ship* instead: every flat-4333 eight must pass
//! 1NT (`eights not passed` should be 0).
//!
//! ```text
//! cargo run --release --example probe-uninvite-4333 -- --count 16000000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Rank, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::american;
use pons::bidding::Family;
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, hand_hcp, mean_with_ci, seat_to_act};

/// Pass / invite / force for a flat 4-3-3-3 eight or nine over partner's 1NT
#[derive(Parser)]
struct Args {
    /// Number of random deals to scan for the flat-4333 responder scenario
    #[arg(short, long, default_value = "16000000")]
    count: u64,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Deal seed (defaults to a fresh random base each run)
    #[arg(short, long)]
    seed: Option<u64>,
}

/// A qualifying board before it is solved: the deal plus the facts the slices key
/// on, and whether an eight failed to pass 1NT (the ship check).
struct Board {
    deal: FullDeal,
    opener: Seat,
    resp_hcp: u8,
    opener_hcp: u8,
    aces: usize,
    tens: usize,
    eight_not_passed: bool,
}

/// A solved board: NS double-dummy score of `k`NT by opener for k = 1, 2, 3.
struct Row {
    resp_hcp: u8,
    opener_hcp: u8,
    aces: usize,
    tens: usize,
    nt: [i64; 3],
    eight_not_passed: bool,
}

/// Exactly one four-card suit and three three-card suits.
fn is_flat_4333(hand: Hand) -> bool {
    let mut lens: [usize; 4] = [0; 4];
    for (i, &s) in Suit::ASC.iter().enumerate() {
        lens[i] = hand[s].len();
    }
    lens.sort_unstable();
    lens == [3, 3, 3, 4]
}

/// How many suits hold the given rank (0..=4).
fn count_rank(hand: Hand, rank: Rank) -> usize {
    Suit::ASC
        .iter()
        .filter(|&&s| hand[s].contains(rank))
        .count()
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

/// NS double-dummy score of `level`NT played by `opener`.
fn nt_score(level: u8, opener: Seat, table: &TrickCountTable, vul: AbsoluteVulnerability) -> i64 {
    let bid = Bid::new(level, Strain::Notrump);
    let contract = Contract {
        bid,
        penalty: Penalty::Undoubled,
    };
    ns_score_contract(Some((contract, opener)), table, vul)
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let sys = american().against(Family::NATURAL);
    let one_nt = Bid::new(1, Strain::Notrump);
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = args.vulnerability;

    // Scan (parallel, bidding only) for the flat-4333 eight/nine over a 1NT open.
    let boards: Vec<Board> = (0..args.count)
        .into_par_iter()
        .filter_map(|i| {
            let mut rng = StdRng::seed_from_u64(base.wrapping_add(i));
            let deal = full_deal(&mut rng);
            let dealer = Seat::ALL[(i % 4) as usize];

            let responder = [Seat::North, Seat::South]
                .into_iter()
                .find(|&s| is_flat_4333(deal[s]) && matches!(hand_hcp(deal[s]), 8 | 9))?;

            let auction = bid_uncontested(&sys, dealer, vul, &deal);
            let (opener, first) = opening(&auction, dealer)?;
            if first != one_nt || Seat::ALL[(opener as usize + 2) % 4] != responder {
                return None;
            }

            let rhand = deal[responder];
            let resp_hcp = hand_hcp(rhand);
            let opener_hcp = hand_hcp(deal[opener]);

            // Ship check: the flat-4333 eight must now pass 1NT (played by opener).
            let one_nt_contract = Contract {
                bid: one_nt,
                penalty: Penalty::Undoubled,
            };
            let eight_not_passed = resp_hcp == 8
                && final_contract(&auction, dealer) != Some((one_nt_contract, opener));

            Some(Board {
                deal,
                opener,
                resp_hcp,
                opener_hcp,
                aces: count_rank(rhand, Rank::A),
                tens: count_rank(rhand, Rank::T),
                eight_not_passed,
            })
        })
        .collect();

    // Batch-solve every qualifying deal once, then price the three contracts.
    let deals: Vec<FullDeal> = boards.iter().map(|b| b.deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);
    let rows: Vec<Row> = boards
        .iter()
        .zip(&tables)
        .map(|(b, t)| Row {
            resp_hcp: b.resp_hcp,
            opener_hcp: b.opener_hcp,
            aces: b.aces,
            tens: b.tens,
            nt: [
                nt_score(1, b.opener, t, vul),
                nt_score(2, b.opener, t, vul),
                nt_score(3, b.opener, t, vul),
            ],
            eight_not_passed: b.eight_not_passed,
        })
        .collect();

    // The three actions, as an NS score per row (invite = 2NT with 15-16, else 3NT).
    let pass = |r: &Row| r.nt[0];
    let invite = |r: &Row| r.nt[if r.opener_hcp >= 17 { 2 } else { 1 }];
    let force = |r: &Row| r.nt[2];

    let not_passed: usize = rows.iter().filter(|r| r.eight_not_passed).count();
    let eights = rows.iter().filter(|r| r.resp_hcp == 8).count();
    println!(
        "=== flat-4333 over 1NT: pass / invite / force — {} deals, vul {} ===",
        args.count, vul,
    );
    println!(
        "(+ ⇒ the LOWER action wins; opponents silent, plain DD.  \
         ship check — eights not passed: {not_passed}/{eights})\n",
    );

    #[allow(clippy::type_complexity)]
    let quality: [(&str, fn(&Row) -> bool); 4] = [
        ("all", |_| true),
        ("no ace", |r| r.aces == 0),
        ("no ten", |r| r.tens == 0),
        ("no ace or ten", |r| r.aces == 0 && r.tens == 0),
    ];

    let report = |title: &str, hcp: u8, lo: &dyn Fn(&Row) -> i64, hi: &dyn Fn(&Row) -> i64| {
        println!("--- {title} ---");
        println!(
            "{:<16} {:>8} {:>12} {:>12}",
            "responder cards", "boards", "IMPs/board", "95% CI"
        );
        for (label, keep) in &quality {
            let vals: Vec<i64> = rows
                .iter()
                .filter(|r| r.resp_hcp == hcp && keep(r))
                .map(|r| imps(lo(r) - hi(r)))
                .collect();
            let (mean, ci) = mean_with_ci(&vals);
            println!("{label:<16} {:>8} {mean:>+12.4} {ci:>+12.4}", vals.len());
        }
        println!();
    };

    // The eight (system default = invite): does passing beat inviting?
    report(
        "EIGHT: pass - invite (downgrade to pass)",
        8,
        &pass,
        &invite,
    );
    // The nine (system default = force): the questions the user asked.
    report(
        "NINE: invite - force (downgrade to invite)",
        9,
        &invite,
        &force,
    );
    report("NINE: pass - force (downgrade to pass)", 9, &pass, &force);
}
