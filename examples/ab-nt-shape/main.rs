//! 1NT-shape A/B: the classic balanced 1NT vs the wide redesign.
//!
//! The deferred wide-1NT redesign is **shape only** — strength
//! ([`fifths`][pons::bidding::constraint::fifths] 15–17) and the inference side
//! are unchanged (a future session).  Both arms run the same 2/1 system; the
//! only difference is whether the 1NT opening also admits the modern shapely
//! hand — a 5422 whose five-card suit is a minor (the shipped default
//! [`american`] vs the balanced-only [`american_classic`]).
//!
//! Opponents are silenced (East/West always pass), so every auction is
//! constructive start to finish — this measures the *constructive* value of the
//! wider opening, not its competitive or lead-directing value (a contested A/B
//! is the follow-up).  Each board is bid twice over the same deal, once per arm;
//! boards whose arms reach different contracts are solved double dummy once and
//! scored.  A positive IMPs/board favors the redesign.
//!
//! ```text
//! cargo run --release --example ab-nt-shape -- --count 2000
//! ```

use clap::Parser;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Tag;
use pons::bidding::american::american_classic;
use pons::scoring::{final_contract, imps, ns_score_contract};
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::bid_uncontested;

/// 1NT-shape A/B: classic balanced 1NT vs the wide redesign
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    // arm 0 = baseline (classic balanced 1NT), arm 1 = redesign (wide 1NT, the
    // shipped default).
    let stances = [
        american_classic().against(Tag::NATURAL),
        american().against(Tag::NATURAL),
    ];

    // Both arms bid the same deal; the only difference is the opening table.
    // Deal sequentially (cheap), then bid in parallel — bidding is pure (the
    // books read their thread-locals at construction), so boards are independent
    // and par_iter preserves order. The DD solver stays on the main thread below.
    let deals: Vec<FullDeal> = (0..args.count).map(|_| full_deal(&mut rng)).collect();
    let contracts: Vec<[_; 2]> = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            std::array::from_fn(|arm| {
                let auction = bid_uncontested(&stances[arm], dealer, args.vulnerability, deal);
                final_contract(&auction, dealer)
            })
        })
        .collect();

    // Only boards whose arms diverge can swing; solve those once.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let base = ns_score_contract(contracts[i][0], table, args.vulnerability);
        let wide = ns_score_contract(contracts[i][1], table, args.vulnerability);
        points += wide - base;
        total_imps += imps(wide - base);
    }

    println!(
        "=== 1NT-shape A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!("(opponents silenced — constructive value only; shape change only)");
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Redesign (wide 1NT): {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board)",
        total_imps as f64 / args.count.max(1) as f64,
    );
}
