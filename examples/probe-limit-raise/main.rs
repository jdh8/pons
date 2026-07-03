//! Worst-board trace for the limit-raise acceptance arm
//! (`set_limit_raise_acceptance`): replays the A/B's seeded deals, keeps the
//! divergent boards, and splits them by divergence class — a plain `4M`
//! accept the baseline declined, or a `4NT` keycard auction — with per-class
//! swing means and the worst boards printed in full (hands, both auctions,
//! plain-DD scores).  Diagnosis input for measurement step 9 ("trace the
//! worst divergent boards before declaring a loss dead").

use clap::Parser;
use contract_bridge::auction::display_calls;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::set_limit_raise_acceptance;
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, mean_with_ci};

/// Trace the divergent boards of the limit-raise acceptance A/B
#[derive(Parser)]
struct Args {
    /// Number of boards (same as the A/B run to replay its deal set)
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "both")]
    vulnerability: AbsoluteVulnerability,

    /// The A/B run's seed base
    #[arg(short, long)]
    seed: u64,

    /// How many of the worst boards to print in full
    #[arg(short, long, default_value = "12")]
    worst: usize,
}

fn main() {
    let args = Args::parse();
    let vul = args.vulnerability;

    set_limit_raise_acceptance(false);
    let baseline = american().against(Family::NATURAL);
    set_limit_raise_acceptance(true);
    let treatment = american().against(Family::NATURAL);
    set_limit_raise_acceptance(false);
    let stances = [baseline, treatment];

    let deals: Vec<FullDeal> = (0..args.count)
        .map(|i| {
            let mut rng = StdRng::seed_from_u64(args.seed.wrapping_add(i as u64));
            full_deal(&mut rng)
        })
        .collect();
    let auctions: Vec<[_; 2]> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            std::array::from_fn(|arm| bid_uncontested(&stances[arm], dealer, vul, deal))
        })
        .collect();

    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| {
            let dealer = Seat::ALL[i % 4];
            final_contract(&auctions[i][0], dealer) != final_contract(&auctions[i][1], dealer)
        })
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    // Swing per divergent board, plain DD, and the divergence class: a
    // treatment auction containing 4NT is the keycard class, else the plain
    // accept class.
    let four_nt = Bid::new(4, Strain::Notrump);
    let mut rows: Vec<(usize, i64, bool)> = divergent
        .iter()
        .zip(tables.iter())
        .map(|(&i, table)| {
            let dealer = Seat::ALL[i % 4];
            let off = ns_score_contract(final_contract(&auctions[i][0], dealer), table, vul);
            let on = ns_score_contract(final_contract(&auctions[i][1], dealer), table, vul);
            let keycard = auctions[i][1].iter().any(
                |&call| matches!(call, contract_bridge::auction::Call::Bid(bid) if bid == four_nt),
            );
            (i, imps(on - off), keycard)
        })
        .collect();

    for (label, class) in [("plain 4M accept", false), ("4NT keycard", true)] {
        let swings: Vec<i64> = rows
            .iter()
            .filter(|&&(_, _, k)| k == class)
            .map(|&(_, s, _)| s)
            .collect();
        let (mean, ci) = mean_with_ci(&swings);
        println!(
            "{label}: {} boards, {:+} IMPs total, {mean:+.3} ± {ci:.3} IMPs/divergent",
            swings.len(),
            swings.iter().sum::<i64>(),
        );
    }

    // Direction split within the accept class: does the treatment out-bid the
    // baseline (its auction is longer/higher) or under-bid it?
    let game = Bid::new(4, Strain::Hearts);
    let game_s = Bid::new(4, Strain::Spades);
    for (label, higher) in [
        ("treatment bids game", true),
        ("treatment stops low", false),
    ] {
        let swings: Vec<i64> = rows
            .iter()
            .filter(|&&(i, _, keycard)| {
                let t_game = auctions[i][1].iter().any(|&c| {
                    matches!(c, contract_bridge::auction::Call::Bid(b) if b == game || b == game_s)
                });
                !keycard && t_game == higher
            })
            .map(|&(_, s, _)| s)
            .collect();
        let (mean, ci) = mean_with_ci(&swings);
        println!(
            "  accept class, {label}: {} boards, {:+} IMPs, {mean:+.3} ± {ci:.3}/divergent",
            swings.len(),
            swings.iter().sum::<i64>(),
        );
    }

    rows.sort_by_key(|&(_, swing, _)| swing);
    println!(
        "\n=== {} worst boards (treatment − baseline, plain DD) ===",
        args.worst
    );
    for &(i, swing, keycard) in rows.iter().filter(|&&(_, _, k)| !k).take(args.worst) {
        let dealer = Seat::ALL[i % 4];
        let class = if keycard { "keycard" } else { "accept" };
        println!("\n--- board {i} (dealer {dealer:?}, {swing:+} IMPs, {class}) ---");
        for seat in Seat::ALL {
            println!("  {seat:?}: {}", deals[i][seat]);
        }
        println!("  baseline : {}", display_calls(&auctions[i][0]));
        println!("  treatment: {}", display_calls(&auctions[i][1]));
    }
}
