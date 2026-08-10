//! How dense is the kickback bit, and what raw-hand filter concentrates it?
//!
//! The configured net (`docs/ai-bidder/configured-net.md`) has to learn to read
//! one card row — slot 77, `Kickback 1430` — out of 270.  A uniform corpus
//! cannot teach it: the relocated ask decides ~0.05% of boards, so a 250k-deal
//! draw carries barely a hundred rows that depend on the bit, and Gate 2 would
//! report a null that cannot tell a worthless convention from a net that never
//! learned to look.
//!
//! The fix is a mixture corpus — uniform bulk plus an enriched slice accepted
//! on **raw hands, before the bidder**, so the filter costs nothing and never
//! reaches a verdict.  Choosing that filter needs a number the *board*
//! divergence rate does not give.  The corpus unit is a **row**, one decision,
//! and a relocated ask changes the row (4♦ where the plain ladder bids 4NT)
//! whether or not the board's score moves at all.  So the population to
//! enrich is every deal whose auction differs between the two card settings —
//! measured here by replaying each deal under both.
//!
//! Non-spade trump is the whole story: a spade ask is 4NT under either
//! setting, so only ♣/♦/♥ can relocate.  That column is reported beside the
//! divergence one to confirm the two agree about where the bit lives.
//!
//! For a grid of raw-hand filters (combined HCP × longest non-spade fit, taken
//! over both partnerships) the probe reports:
//!
//! - `accept` — what fraction of raw deals the filter lets through,
//! - `diverge` — of those, what fraction bid differently under the two cards,
//! - `lift` — `diverge` over the unfiltered rate, the enrichment factor,
//! - `bid/hit` — deals that must be *bid* per divergent board (the real cost;
//!   rejected deals never touch the bidder),
//! - `draw/hit` — deals that must be *drawn*, which is nearly free.
//!
//! ```sh
//! cargo run --release --example probe-kickback-yield -- --count 200000
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat, Suit};
use pons::american_instinct;
use pons::bidding::Partnership;
use pons::bidding::instinct::{RkcbVariant, keycard_ask_at};
use rayon::prelude::*;

fn agreements(on: bool) -> pons::bidding::agreements::Agreements {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.decision.reading.rkcb_variant = if on {
        RkcbVariant::Kickback
    } else {
        RkcbVariant::Plain
    };
    agreements
}

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seeded_deals, slam_ish};

/// Combined-HCP floors swept, weakest first
const HCP_STEPS: [u8; 4] = [24, 26, 28, 30];
/// Combined non-spade fit lengths swept, shortest first
const FIT_STEPS: [u8; 3] = [8, 9, 10];
/// One row per (HCP, fit) pair, plus the unfiltered row at index 0
const FILTERS: usize = 1 + HCP_STEPS.len() * FIT_STEPS.len();

#[derive(Parser)]
struct Args {
    /// How many random deals to bid
    #[arg(long, default_value = "200000")]
    count: usize,
    /// Seed base for the deal stream
    #[arg(long, default_value = "1")]
    seed: u64,
}

/// Whether filter `i` accepts a deal with these raw statistics; filter 0 is the
/// unfiltered baseline.
fn accepts(i: usize, (points, fit): (u8, u8)) -> bool {
    i == 0 || {
        let (h, f) = ((i - 1) / FIT_STEPS.len(), (i - 1) % FIT_STEPS.len());
        points >= HCP_STEPS[h] && fit >= FIT_STEPS[f]
    }
}

/// The label filter `i` prints under
fn label(i: usize) -> String {
    if i == 0 {
        "  (none)".into()
    } else {
        let (h, f) = ((i - 1) / FIT_STEPS.len(), (i - 1) % FIT_STEPS.len());
        format!("{:>3} hcp {}+", HCP_STEPS[h], FIT_STEPS[f])
    }
}

/// Per-filter counters: deals accepted, and of those, deals whose auction
/// contains any keycard ask / a non-spade one.
#[derive(Default, Clone, Copy)]
struct Tally {
    accepted: [u64; FILTERS],
    diverged: [u64; FILTERS],
    non_spade: [u64; FILTERS],
    /// Divergent rows summed over divergent deals, per filter
    rows: [u64; FILTERS],
    /// Non-spade asks by trump, over the whole (unfiltered) stream
    by_trump: [u64; 3],
}

impl Tally {
    fn merge(mut self, other: Self) -> Self {
        for i in 0..FILTERS {
            self.accepted[i] += other.accepted[i];
            self.diverged[i] += other.diverged[i];
            self.non_spade[i] += other.non_spade[i];
            self.rows[i] += other.rows[i];
        }
        for (a, b) in self.by_trump.iter_mut().zip(other.by_trump) {
            *a += b;
        }
        self
    }
}

/// Bid the deal out with `kickback` armed on every seat.
///
/// The knob gates rule *presence* at build time and the recognizers at
/// classification time, and both halves are captured into the partnership the caller
/// built for this arm — so the worker needs no arming of its own.
fn bid_out(
    partnership: &Partnership,
    deal: &FullDeal,
    dealer: Seat,
    vul: AbsoluteVulnerability,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = common::seat_to_act(dealer, auction.len());
        auction.push(next_call(partnership, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// How many calls the two auctions disagree on, counted over the longer of the
/// two — a divergence that shortens the auction still costs every trailing row.
fn divergent_rows(on: &Auction, off: &Auction) -> u64 {
    let (a, b): (Vec<_>, Vec<_>) = (on.iter().collect(), off.iter().collect());
    let common = a.len().min(b.len());
    let differing = (0..common).filter(|&i| a[i] != b[i]).count();
    (differing + a.len().max(b.len()) - common) as u64
}

const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];
const DEALERS: [Seat; 4] = [Seat::North, Seat::East, Seat::South, Seat::West];

fn main() {
    let args = Args::parse();
    // One partnership per arm: the knob gates rule presence, so an off-arm partnership
    // built with it on would carry alerted rungs that erase natural readings.
    let arm = |kickback| american_instinct(&agreements(kickback)).bind();
    let (on, off) = (arm(true), arm(false));
    let deals = seeded_deals(args.seed, args.count);

    let tally = deals
        .par_iter()
        .enumerate()
        .fold(Tally::default, |mut tally, (i, deal)| {
            let stats = slam_ish(deal);
            let accepted: Vec<usize> = (0..FILTERS).filter(|&f| accepts(f, stats)).collect();
            // Bidding is the expensive half, so skip it when no filter — not
            // even the baseline — would have kept the deal.  (Filter 0 always
            // accepts, so in practice this only documents the intent.)
            if accepted.is_empty() {
                return tally;
            }
            let (dealer, vul) = (DEALERS[i % 4], VULS[i % 4]);
            let auction = bid_out(&on, deal, dealer, vul);
            let plain = bid_out(&off, deal, dealer, vul);
            let rows = divergent_rows(&auction, &plain);

            let calls: Vec<_> = auction.iter().copied().collect();
            let profile = agreements(true).decision.reading;
            let non_spade = (0..calls.len())
                .find_map(|at| keycard_ask_at(profile, &calls, at))
                .map(|(_, suit, _)| suit)
                .filter(|&suit| suit != Suit::Spades);
            if let Some(suit) = non_spade {
                tally.by_trump[suit as usize] += 1;
            }
            for f in accepted {
                tally.accepted[f] += 1;
                tally.diverged[f] += u64::from(rows > 0);
                tally.rows[f] += rows;
                tally.non_spade[f] += u64::from(non_spade.is_some());
            }
            tally
        })
        .reduce(Tally::default, Tally::merge);

    let drawn = args.count as f64;
    let base = tally.diverged[0] as f64 / tally.accepted[0].max(1) as f64;

    println!(
        "{} deals bid under both cards, seed {}\n",
        args.count, args.seed
    );
    println!("   filter    accept   diverge   non-♠ ask     lift  rows/deal   bid/hit  draw/hit");
    for f in 0..FILTERS {
        let acc = tally.accepted[f] as f64;
        let hits = tally.diverged[f] as f64;
        let rate = hits / acc.max(1.0);
        // A filter that caught nothing has no cost per hit to report; printing
        // the reciprocal of zero as a number would read as a measurement.
        let per_hit = |x: f64| {
            if hits > 0.0 {
                format!("{x:9.0}")
            } else {
                format!("{:>9}", "—")
            }
        };
        println!(
            "{:>9} {:8.3}% {:8.4}% {:10.4}% {:8.1}× {:10.4} {} {}",
            label(f),
            100.0 * acc / drawn,
            100.0 * rate,
            100.0 * tally.non_spade[f] as f64 / acc.max(1.0),
            rate / base.max(f64::MIN_POSITIVE),
            tally.rows[f] as f64 / acc.max(1.0),
            per_hit(1.0 / rate),
            per_hit(drawn / hits),
        );
    }

    println!("\n  non-♠ asks by trump (unfiltered stream):");
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        println!("    {suit:?}: {}", tally.by_trump[suit as usize]);
    }
}
