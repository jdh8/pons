//! Does the live seam-gated lookahead beat the book?  The M8 A/B
//! (`docs/ai-bidder/plan.md`), and the live twin of `probe-seam-lookahead`'s
//! replay arm.
//!
//! Self-play duplicate over random deals: at table A a
//! [`Lookahead`] over `american()` sits North/South against the bare book, at
//! table B they swap, and the swing is credited to the lookahead.  Dealer
//! rotates per deal and vulnerability alternates none/both every four — the
//! probe's layout, so its cells compare.
//!
//! A gated `classify` solves double dummy, so the lookahead's tables are bid
//! **sequentially on the main thread**.  Only the tables whose all-book
//! auction reaches a seam on the lookahead's turn are re-bid: off the gate the
//! wrapper is the book call for call, so every other table is that auction.
//!
//! ```sh
//! cargo run --release --example ab-lookahead -- -c 20000 -s 1
//! scripts/idle-run.sh cargo run --release --example ab-lookahead -- -c 600000 -s $SEED_BASE
//! ```
//!
//! Heavy: `--layouts` double-dummy solves per seam decision (≈ 6 ms each).
//!
//! `--census N` solves nothing: it walks the book and prints the `N`
//! undisturbed authored nodes where the call made is most often *not* the
//! node's commonest — the seams a wider `--seam` list could name.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::american;
use pons::bidding::agreements::Agreements;
use pons::bidding::context::relative;
use pons::bidding::lookahead::Lookahead;
use pons::bidding::{Bidder, Partnership, Table};
use pons::scoring::{final_contract, imps, ns_score_bid, ns_score_contract};
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{auction_key, mean_with_ci, seat_to_act, seeded_deals};

const BRACKETS: [&str; 2] = ["plain DD", "perfect defense"];
const VULS: [AbsoluteVulnerability; 2] = [AbsoluteVulnerability::NONE, AbsoluteVulnerability::ALL];

#[derive(Parser)]
struct Args {
    /// Deals in the match
    #[arg(short, long, default_value_t = 20_000)]
    count: usize,
    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
    /// Worlds per seam decision
    #[arg(short = 'm', long, default_value_t = 64)]
    layouts: usize,
    /// IMPs a rung's mean swing must clear on both scorers
    #[arg(long, default_value_t = 0.25)]
    margin: f64,
    /// Seam keys as `render-book` prints them (leading passes stripped)
    #[arg(long, default_values_t = ["1♠ - 2♠ -".to_owned(), "1♥ - 2♥ -".to_owned()])]
    seam: Vec<String>,
    /// Print the busiest undisturbed authored nodes instead, and stop
    #[arg(long)]
    census: Option<usize>,
}

/// Dealer rotates per deal
const fn dealer(index: usize) -> Seat {
    Seat::ALL[index % 4]
}

/// Vulnerability cell, an index into [`VULS`]: alternates every four deals
const fn vul(index: usize) -> usize {
    (index / 4) % 2
}

/// One re-bid table: the lookahead on `side` (0 = North/South) of deal `index`
struct Seated {
    index: usize,
    side: usize,
    auction: Auction,
}

fn show(values: &[i64]) -> String {
    let (mean, ci) = mean_with_ci(values);
    format!("{mean:+.4} ± {ci:.4}")
}

/// Candidate seams: every authored node our side reaches unopposed after its
/// first bid, with the calls made there, ranked by how many were not the
/// node's commonest call (reach alone ranks the forced relays first)
fn census(book: &[Auction], policy: &Partnership, top: usize) {
    let mut nodes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for (index, auction) in book.iter().enumerate() {
        for len in 1..auction.len() {
            let prefix = &auction[..len];
            // Call `i` is our side's iff it shares the actor's parity.
            let ours = |i: usize| (len - i).is_multiple_of(2);
            let mut calls = prefix.iter().enumerate();
            let quiet = calls.clone().all(|(i, &c)| ours(i) || c == Call::Pass);
            let bid = calls.any(|(i, &c)| ours(i) && c != Call::Pass);
            let rel = relative(VULS[vul(index)], seat_to_act(dealer(index), len));
            if quiet && bid && policy.authored_at(rel, prefix) {
                *nodes
                    .entry(auction_key(prefix))
                    .or_default()
                    .entry(auction[len].to_string())
                    .or_default() += 1;
            }
        }
    }
    // (off-modal count, line), so the sort needs no named row type.
    let mut ranked: Vec<(usize, String)> = nodes
        .into_iter()
        .map(|(key, calls)| {
            let mut calls: Vec<(usize, String)> = calls.into_iter().map(|(c, n)| (n, c)).collect();
            calls.sort_by(|a, b| b.cmp(a));
            let reach: usize = calls.iter().map(|&(n, _)| n).sum();
            let off = reach - calls[0].0;
            let calls: Vec<String> = calls.iter().map(|(n, c)| format!("{c} ×{n}")).collect();
            let line = format!("{off:>9}  {reach:>5}  {key}: {}", calls.join(", "));
            (off, line)
        })
        .collect();
    ranked.sort_by(|a, b| b.cmp(a));
    println!("off-modal  reach  node: calls");
    for (_, line) in ranked.into_iter().take(top) {
        println!("{line}");
    }
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let policy = american(&Agreements::default()).bind();
    let gate = |auction: &[Call]| args.seam.contains(&auction_key(auction));
    let mut lookahead = Lookahead::new(policy.clone(), gate);
    lookahead.layouts = args.layouts;
    lookahead.margin = args.margin;
    let started = std::time::Instant::now();

    let deals = seeded_deals(base, args.count);
    let book: Vec<Auction> = deals
        .par_iter()
        .enumerate()
        .map(|(i, deal)| Table::new(&policy, &policy, dealer(i), VULS[vul(i)]).bid_out(deal))
        .collect();
    let walked = started.elapsed();
    if let Some(top) = args.census {
        census(&book, &policy, top);
        return;
    }

    // The tables where the lookahead meets a seam; every other is `book[i]`.
    let mut seated: Vec<Seated> = Vec::new();
    for (index, auction) in book.iter().enumerate() {
        for side in 0..2 {
            let meets = (0..auction.len()).any(|len| {
                seat_to_act(dealer(index), len) as usize % 2 == side && gate(&auction[..len])
            });
            if meets {
                let (ours, theirs): (&dyn Bidder, &dyn Bidder) = (&lookahead, &policy);
                let (ns, ew) = if side == 0 {
                    (ours, theirs)
                } else {
                    (theirs, ours)
                };
                let table = Table::new(ns, ew, dealer(index), VULS[vul(index)]);
                seated.push(Seated {
                    index,
                    side,
                    auction: table.bid_out(&deals[index]),
                });
                if seated.len().is_multiple_of(1000) {
                    eprintln!("{} seam tables, {:.0?}", seated.len(), started.elapsed());
                }
            }
        }
    }
    let looked = started.elapsed();

    // Solve each board with a table that reached a different contract, once.
    let contract = |index: usize, auction: &Auction| final_contract(auction, dealer(index));
    let mut divergent: Vec<usize> = seated
        .iter()
        .filter(|t| contract(t.index, &t.auction) != contract(t.index, &book[t.index]))
        .map(|t| t.index)
        .collect();
    divergent.dedup();
    let solve: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables: BTreeMap<usize, TrickCountTable> = divergent
        .iter()
        .copied()
        .zip(Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL))
        .collect();

    // North/South's score at each seated table against the all-book auction,
    // signed to the lookahead: `[plain DD, perfect defense]` points.
    let points = |t: &Seated| -> [i64; 2] {
        let Some(tricks) = tables.get(&t.index) else {
            return [0; 2];
        };
        let sign = if t.side == 0 { 1 } else { -1 };
        let v = VULS[vul(t.index)];
        let (on, off) = (
            contract(t.index, &t.auction),
            contract(t.index, &book[t.index]),
        );
        [
            sign * (ns_score_contract(on, tricks, v) - ns_score_contract(off, tricks, v)),
            sign * (ns_score_bid(on, tricks, v) - ns_score_bid(off, tricks, v)),
        ]
    };

    // Per board, the duplicate swing: table A minus table B, both in points.
    let mut board = vec![[0i64; 2]; args.count];
    let mut per_table: [Vec<i64>; 2] = [Vec::new(), Vec::new()];
    let mut swaps: BTreeMap<String, [Vec<i64>; 2]> = BTreeMap::new();
    let mut fired = 0;
    for t in &seated {
        let swing = points(t);
        let control = &book[t.index];
        let at = t
            .auction
            .iter()
            .zip(control.iter())
            .position(|(a, b)| a != b);
        fired += usize::from(at.is_some());
        for bracket in 0..2 {
            board[t.index][bracket] += swing[bracket];
            per_table[bracket].push(imps(swing[bracket]));
        }
        if let Some(at) = at {
            let entry = swaps
                .entry(format!(
                    "{}  {} -> {}",
                    auction_key(&control[..at]),
                    control[at],
                    t.auction[at]
                ))
                .or_default();
            for bracket in 0..2 {
                entry[bracket].push(imps(swing[bracket]));
            }
        }
    }

    println!(
        "seed {base}, {} deals, M = {}, margin {}; walk {:.1?}, lookahead {:.1?}, solve {:.1?}",
        args.count,
        args.layouts,
        args.margin,
        walked,
        looked - walked,
        started.elapsed() - looked
    );
    println!(
        "{} seam tables ({:.2}% of deals), {fired} fired ({:.1}%), {} divergent boards",
        seated.len(),
        100.0 * seated.len() as f64 / args.count as f64,
        100.0 * fired as f64 / seated.len().max(1) as f64,
        divergent.len()
    );
    println!("\nIMPs/board for the lookahead, ± 95%:");
    for (bracket, name) in BRACKETS.iter().enumerate() {
        let cell = |keep: &dyn Fn(usize) -> bool| -> Vec<i64> {
            (0..args.count)
                .filter(|&i| keep(i))
                .map(|i| imps(board[i][bracket]))
                .collect()
        };
        println!(
            "  {name:>15}: {}   vul none {}   vul both {}",
            show(&cell(&|_| true)),
            show(&cell(&|i| vul(i) == 0)),
            show(&cell(&|i| vul(i) == 1)),
        );
    }
    println!(
        "IMPs per seam table: {} / {}",
        show(&per_table[0]),
        show(&per_table[1])
    );
    println!("\nSwaps (book -> lookahead), IMPs per swap, plain / PD:");
    for (swap, gains) in &swaps {
        println!(
            "  {swap}  ×{}  {} / {}",
            gains[0].len(),
            show(&gains[0]),
            show(&gains[1])
        );
    }
}
