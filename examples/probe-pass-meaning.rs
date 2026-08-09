//! Stage B viability gate: what does each call **actually** hold, by traffic?
//!
//! The sampled-projection design (docs/ai-bidder/sampled-projection.md) asks
//! for the real acceptance rate before any storage machinery is built.  One
//! self-play sweep answers it for *every* position at once: bid `count` deals,
//! record the actor's hand at each decision, and group by auction prefix.
//! Traffic is the sample weight — the highest-traffic blind keys (the
//! defensive head: passes over their preempts and 1NT, fourth-seat passes) get
//! the largest samples for free, with no per-node rejection sampling at all.
//!
//! Per key the report shows the observed distribution beside the reading the
//! stance currently publishes for that seat (the `announced` envelope the nets
//! and sampler consume), so the two soundness-critical judgments are visible:
//!
//! - **separation** — where the observed mass sits well inside the published
//!   box, a probed reading has something to say (the prize);
//! - **support-edge danger** — the observed extremes are *sample* bounds, not
//!   rule bounds; p1/p99 beside min/max show whether mass dies off before the
//!   edge (safe to tighten toward) or runs right up to it (widen, never trust).
//!
//! Keys keep their **leading passes**, unlike the census's
//! [`auction_key`][common::auction_key]: passer status changes the published
//! reading — an opening pass caps at 11 points
//! ([`set_pass_reading`][pons::bidding::set_pass_reading]) — so pooling
//! `1♣ - 1♥` with `- - 1♣ - 1♥` compares a passed hand's correct 6..=11
//! against an unpassed population running to 21.  That was this example's
//! first false positive; the wider key is what keeps a row one population.
//!
//! No double-dummy, no solver.
//!
//! ```sh
//! cargo run --release --example probe-pass-meaning -- -c 100000 --filter " -"
//! ```

use clap::Parser;
use contract_bridge::auction::{Call, display_calls};
use contract_bridge::{AbsoluteVulnerability, Hand, Seat, Suit};
use pons::american;
use pons::bidding::constraint::point_count;
use pons::bidding::context::relative;
use pons::bidding::{Envelope, Range, Relative};
use rayon::prelude::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_to_act, seeded_deals};

/// The full auction prefix as a key, leading passes intact (see the module doc)
// ponytail: not `common::auction_key` — its pass stripping is dealer-invariance
// for the census worklist, and pooling passer status is unsound *here*.
fn prefix_key(prefix: &[Call]) -> String {
    display_calls(prefix).to_string()
}

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "100000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Keys to report, ranked by sample count
    #[arg(long, default_value = "40")]
    top: usize,

    /// Only report keys containing this substring (e.g. " -" for passes)
    #[arg(long)]
    filter: Option<String>,

    /// Skip keys with fewer samples than this
    #[arg(long, default_value = "100")]
    min_count: usize,
}

/// Per-key sample aggregate: point-count and per-suit length histograms
struct KeyAgg {
    count: u64,
    points: [u64; 38],
    lengths: [[u64; 14]; 4],
    /// The first full prefix seen for this key (leading passes intact), so the
    /// published reading can be recomputed for the report
    prefix: Vec<Call>,
    dealer: Seat,
}

impl KeyAgg {
    fn new(prefix: &[Call], dealer: Seat) -> Self {
        Self {
            count: 0,
            points: [0; 38],
            lengths: [[0; 14]; 4],
            prefix: prefix.to_vec(),
            dealer,
        }
    }

    fn add(&mut self, hand: Hand) {
        self.count += 1;
        self.points[usize::from(point_count(hand)).min(37)] += 1;
        for suit in Suit::ASC {
            self.lengths[suit as usize][hand[suit].len().min(13)] += 1;
        }
    }

    fn merge(&mut self, other: &Self) {
        self.count += other.count;
        for (a, b) in self.points.iter_mut().zip(other.points) {
            *a += b;
        }
        for (row, other_row) in self.lengths.iter_mut().zip(other.lengths) {
            for (a, b) in row.iter_mut().zip(other_row) {
                *a += b;
            }
        }
    }
}

/// min / p1 / mean / p99 / max of a histogram, `None` when empty
fn stats(hist: &[u64]) -> Option<(usize, usize, f64, usize, usize)> {
    let total: u64 = hist.iter().sum();
    if total == 0 {
        return None;
    }
    let min = hist.iter().position(|&n| n > 0)?;
    let max = hist.iter().rposition(|&n| n > 0)?;
    let quantile = |q: f64| {
        #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        let target = (q * total as f64).ceil() as u64;
        let mut seen = 0;
        hist.iter()
            .position(|&n| {
                seen += n;
                seen >= target.max(1)
            })
            .unwrap_or(max)
    };
    #[allow(clippy::cast_precision_loss)]
    let mean = hist
        .iter()
        .enumerate()
        .map(|(value, &n)| value as f64 * n as f64)
        .sum::<f64>()
        / total as f64;
    Some((min, quantile(0.01), mean, quantile(0.99), max))
}

/// Render a published axis range, `⊤` when it is the full (vacuous) one
fn axis(range: Range, full: Range) -> String {
    if range == full {
        "⊤".to_owned()
    } else {
        format!("{}..={}", range.min, range.max)
    }
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let stance = american(&pons::bidding::agreements::Agreements::current()).against();

    let per_board = |board: usize, deal: &contract_bridge::FullDeal| {
        let dealer = Seat::ALL[board % 4];
        let auction = bid_out(&stance, &stance, true, dealer, vul, deal);
        let mut keys: HashMap<String, KeyAgg> = HashMap::new();
        for index in 0..auction.len() {
            let prefix = &auction[..=index];
            let seat = seat_to_act(dealer, index);
            keys.entry(prefix_key(prefix))
                .or_insert_with(|| KeyAgg::new(prefix, dealer))
                .add(deal[seat]);
        }
        keys
    };

    let deals = seeded_deals(base, args.count);
    let keys: HashMap<String, KeyAgg> = deals
        .par_iter()
        .enumerate()
        .map(|(board, deal)| per_board(board, deal))
        .reduce(HashMap::new, |mut into, from| {
            for (key, agg) in from {
                into.entry(key)
                    .and_modify(|existing| existing.merge(&agg))
                    .or_insert(agg);
            }
            into
        });

    let decisions: u64 = keys.values().map(|agg| agg.count).sum();
    let covered: u64 = keys
        .values()
        .filter(|agg| agg.count >= args.min_count as u64)
        .map(|agg| agg.count)
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let coverage = 100.0 * covered as f64 / decisions.max(1) as f64;
    println!("boards {}  seed {base}", args.count);
    println!(
        "decisions {decisions}  distinct keys {}  ≥{}-sample coverage {coverage:.1}% of traffic\n",
        keys.len(),
        args.min_count,
    );

    let mut ranked: Vec<(&String, &KeyAgg)> = keys
        .iter()
        .filter(|(key, agg)| {
            agg.count >= args.min_count as u64
                && args
                    .filter
                    .as_deref()
                    .is_none_or(|needle| key.contains(needle))
        })
        .collect();
    ranked.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(b.0)));

    for (key, agg) in ranked.iter().take(args.top) {
        // The published reading of the seat that just called, seen by the next
        // actor (the census surface): the caller is that actor's RHO.
        let cut = agg.prefix.len();
        let reader = seat_to_act(agg.dealer, cut);
        let read = stance.infer(relative(vul, reader), &agg.prefix);
        let shown: &Envelope = read.announced(Relative::Rho);

        let Some((min, p1, mean, p99, max)) = stats(&agg.points) else {
            continue;
        };
        println!(
            "[{key}]  n={}  points {min}-{max} (p1 {p1}, mean {mean:.1}, p99 {p99})  \
             reading {}",
            agg.count,
            axis(shown.strength.points, Range::FULL_POINTS),
        );
        for suit in Suit::ASC {
            let Some((min, p1, mean, p99, max)) = stats(&agg.lengths[suit as usize]) else {
                continue;
            };
            print!(
                "    {suit} {min}-{max} (p1 {p1}, mean {mean:.1}, p99 {p99}) reading {}",
                axis(shown.length(suit), Range::FULL_LENGTH),
            );
        }
        println!();
    }
}
