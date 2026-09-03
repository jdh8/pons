//! Book argmax vs the restricted net's argmax, at every authored node
//!
//! The audit probe of `docs/ai-bidder/logit-calibration.md` §4 ("The audit
//! probe"). The book's `i16` rungs are **precedence** — an order, not odds —
//! so where a consumer needs a *distribution* at an authored node the design
//! takes the net's softmax restricted to the calls the book admits there. That
//! makes the two orders comparable for the first time, and this probe puts them
//! side by side before any consumer depends on the hook:
//!
//! * **disagreement**, bucketed by node — where does the restricted net rank a
//!   different call first than the book does? Renormalising cannot reorder, so
//!   this count is independent of the temperature; only the mass below is not.
//! * **the epsilon fallback's price** — how often the *unrestricted* net puts
//!   essentially nothing on any admissible call, which is the case the hook
//!   must answer with the book's one-hot argmax instead.
//!
//! Self-play `american()` at neither vulnerable, so both the constructive and
//! the contested books are walked. The net is called through
//! [`classify_bba_v6`] **directly**, never through the floor shell: the shell
//! masks, gates and — at a constructive node — delegates to the rung ladder,
//! which is the very thing the odds are meant to replace.
//!
//! ```sh
//! cargo run --release --example probe-book-vs-net -- -c 20000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american;
use pons::bidding::agreements::Agreements;
use pons::bidding::array::Logits;
use pons::bidding::context::relative;
use pons::bidding::features::{CompactConfig, ConventionCard, features_v6};
use pons::bidding::neural::classify_bba_v6;
use rayon::prelude::*;
use std::collections::BTreeMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{auction_key, seat_to_act, seeded_deals};

/// Candidate thresholds for the epsilon fallback: below one of these the net
/// has put essentially nothing on any call the book admits, and the hook falls
/// back to the book's one-hot argmax. The probe's job is to price them; the
/// last is the coarsest, and the one the per-node column reports.
const EPSILONS: [f32; 5] = [1e-6, 1e-4, 1e-3, 1e-2, 1e-1];

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value_t = 20000)]
    count: usize,
    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
    /// Softmax temperature for the restricted mass. The argmax — and so every
    /// disagreement count — is scale-invariant; only the mass moves.
    #[arg(short, long, default_value_t = 1.0)]
    temperature: f32,
    /// Disagreeing nodes to list
    #[arg(long, default_value_t = 25)]
    top: usize,
}

/// What one node accumulated over the walk
#[derive(Default)]
struct Bucket {
    seen: usize,
    disagree: usize,
    /// Decisions whose admissible mass fell under each of [`EPSILONS`]
    thin: [usize; EPSILONS.len()],
    /// `book -> net` swaps, by count
    swaps: BTreeMap<String, usize>,
}

impl Bucket {
    fn merge(&mut self, other: &Self) {
        self.seen += other.seen;
        self.disagree += other.disagree;
        for (ours, theirs) in self.thin.iter_mut().zip(other.thin) {
            *ours += theirs;
        }
        for (swap, n) in &other.swaps {
            *self.swaps.entry(swap.clone()).or_default() += n;
        }
    }
}

/// First-max over the admissible calls, exactly as `select_with_legal_state`
/// breaks ties — the *first* maximum, never the last.
fn first_max(logits: &Logits, admissible: &[Call]) -> Call {
    let mut best: Option<(Call, f32)> = None;
    for (call, &logit) in logits.iter() {
        if !admissible.contains(&call) {
            continue;
        }
        if best.is_none_or(|(_, top)| logit > top) {
            best = Some((call, logit));
        }
    }
    best.map_or(Call::Pass, |(call, _)| call)
}

/// Share of the unrestricted `softmax(z / t)` sitting on the admissible calls
fn admissible_mass(logits: &Logits, admissible: &[Call], t: f32) -> f32 {
    let beta = 1.0 / t;
    let max = logits
        .iter()
        .map(|(_, &z)| z)
        .fold(f32::NEG_INFINITY, f32::max);
    let (mut inside, mut total) = (0.0, 0.0);
    for (call, &z) in logits.iter() {
        let w = (beta * (z - max)).exp();
        total += w;
        if admissible.contains(&call) {
            inside += w;
        }
    }
    inside / total
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let agreements = Agreements::default();
    let partnership = american(&agreements).bind();
    // The very cell the shipped floor is armed at (`american`): our captured
    // card on both sides.
    let compact = CompactConfig::symmetric(&ConventionCard::capture(&agreements, false));

    let deals = seeded_deals(base, args.count);
    let nodes = deals
        .par_iter()
        .fold(BTreeMap::<String, Bucket>::new, |mut acc, deal| {
            let dealer = Seat::North;
            let mut auction = Auction::new();
            while !auction.has_ended() {
                let seat = seat_to_act(dealer, auction.len());
                let hand = deal[seat];
                let rel = relative(vul, seat);
                let Some((book, provenance)) =
                    partnership.classify_with_provenance(hand, rel, &auction)
                else {
                    auction.push(Call::Pass);
                    continue;
                };
                let admissible: Vec<Call> = book
                    .iter()
                    .filter(|(call, logit)| logit.is_finite() && auction.can_push(*call).is_ok())
                    .map(|(call, _)| call)
                    .collect();
                let call = first_max(&book, &admissible);
                if provenance.is_authored() && !admissible.is_empty() {
                    let context = partnership
                        .prefixed_context(rel, &auction)
                        .with_compact(&compact);
                    let net = classify_bba_v6(&features_v6(hand, &context));
                    let pick = first_max(&net, &admissible);
                    let mass = admissible_mass(&net, &admissible, args.temperature);
                    let bucket = acc.entry(auction_key(&auction)).or_default();
                    bucket.seen += 1;
                    for (count, eps) in bucket.thin.iter_mut().zip(EPSILONS) {
                        *count += usize::from(mass < eps);
                    }
                    if pick != call {
                        bucket.disagree += 1;
                        *bucket.swaps.entry(format!("{call} -> {pick}")).or_default() += 1;
                    }
                }
                auction.push(call);
            }
            acc
        })
        .reduce(BTreeMap::new, |mut a, b| {
            for (key, bucket) in &b {
                a.entry(key.clone()).or_default().merge(bucket);
            }
            a
        });

    let seen: usize = nodes.values().map(|b| b.seen).sum();
    let disagree: usize = nodes.values().map(|b| b.disagree).sum();
    let thin = nodes.values().fold([0usize; EPSILONS.len()], |mut acc, b| {
        for (a, t) in acc.iter_mut().zip(b.thin) {
            *a += t;
        }
        acc
    });
    println!(
        "seed {base}  deals {}  authored decisions {seen}  distinct nodes {}",
        args.count,
        nodes.len()
    );
    if seen == 0 {
        return;
    }
    let pct = |n: usize| 100.0 * n as f64 / seen as f64;
    println!(
        "book argmax == restricted-net argmax: {:.2}%  ({disagree} disagreements)",
        100.0 - pct(disagree)
    );
    println!(
        "epsilon fallback at T {} — decisions whose admissible mass falls under:",
        args.temperature
    );
    for (eps, n) in EPSILONS.iter().zip(thin) {
        println!("  {eps:>8.0e}  {n:>8}  {:>7.3}%", pct(n));
    }

    let mut ranked: Vec<(&String, &Bucket)> =
        nodes.iter().filter(|(_, b)| b.disagree > 0).collect();
    ranked.sort_by_key(|(key, b)| (std::cmp::Reverse(b.disagree), key.as_str()));
    println!(
        "\n{:>7} {:>7} {:>7} {:>6}  {:<30} top swap",
        "seen", "differ", "rate", "thin", "node"
    );
    for (key, bucket) in ranked.into_iter().take(args.top) {
        let top = bucket
            .swaps
            .iter()
            .max_by_key(|(swap, n)| (*n, std::cmp::Reverse(swap.as_str())))
            .map_or(String::new(), |(swap, n)| format!("{swap} x{n}"));
        println!(
            "{:>7} {:>7} {:>6.1}% {:>6}  {:<30} {top}",
            bucket.seen,
            bucket.disagree,
            100.0 * bucket.disagree as f64 / bucket.seen as f64,
            bucket.thin[EPSILONS.len() - 1],
            key,
        );
    }
}
