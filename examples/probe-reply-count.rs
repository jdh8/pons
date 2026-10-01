//! Can partner's next call be **counted**?  Phase 3a of
//! `docs/exact-posterior.md`.
//!
//! A one-ply reply lookahead needs `P(partner's reply | my cards, the
//! auction)`.  The counter offers one for free: read the auction once more
//! under each legal reply `r`, and take the exact mass `m_r` of the reading
//! `r` would leave of partner, given my thirteen cards.  Were the readings a
//! partition of what partner can hold, `m_r / Σ m` would be the forecast.
//!
//! They are not promised to be — a reading is sound, not tight, a `pred` gate
//! projects ⊤, a floor-made call may read as nothing, and a call nobody makes
//! reads as *anything* — so the replies are cut to the calls the node made on
//! some other deal, and the forecast is scored on real self-play deals
//! against the call partner actually made,
//! beside two forecasts that need no hand: the same masses with no cards seen,
//! and the node's own reply frequencies (leave-one-out).  A forecast that
//! cannot beat the node's frequencies knows less than a table lookup.
//!
//! No double-dummy, no solver.
//!
//! ```sh
//! cargo run --release --example probe-reply-count -- -c 2000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Level, Seat, Strain};
use pons::american;
use pons::bidding::Relative;
use pons::bidding::context::relative;
use rayon::prelude::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::mass::Counter;
use common::{auction_key, bid_out, seat_to_act, seeded_deals};

/// `C(52, 13)`: every hand, to a counter that has seen no cards
const ALL_HANDS: f64 = 635_013_559_600.0;

/// A forecast is charged at most `−ln FLOOR` for a call it gave no mass
const FLOOR: f64 = 1e-4;

/// The gate, fixed before the first run: a node is *countable* when it has
/// this many decisions, …
const NODE_FLOOR: usize = 30;
/// … the forecast gives the call actually made no mass at most this often, …
const MISS_CEILING: f64 = 0.02;
// … and its log-loss is no worse than the node's own leave-one-out frequencies.

/// A node has a choice to forecast when its own frequencies still lose this
/// many nats; the reader's hand helps where it saves as many
const CHOICE: f64 = 0.05;

/// Smoothing for the leave-one-out node frequencies
const EPSILON: f64 = 0.01;

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "2000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Nodes to list
    #[arg(long, default_value = "40")]
    top: usize,

    /// List only nodes whose auction starts with this text
    #[arg(long, default_value = "")]
    prefix: String,
}

/// One call of one deal, forecast by the caller's partner
struct Row {
    /// The auction before the call
    node: String,
    call: Call,
    /// The forecast's probability of `call`, with the observer's cards seen
    exact: f64,
    /// The same with no cards seen
    free: f64,
    /// `call` is the forecast's most likely reply
    top: bool,
    /// `Σ m_r` over the mass of the caller's reading before the call: 1 for a
    /// partition, above it where the replies' readings overlap
    overlap: f64,
    /// An authored node made the call, not the keyless floor
    authored: bool,
}

/// Reliability bins over every (decision, legal reply): count, Σ p, hits
type Bins = [(u64, f64, u64); 10];

fn all_calls() -> Vec<Call> {
    let bids = (1..=7).flat_map(|level| {
        Strain::ASC.map(|strain| {
            Call::Bid(Bid {
                level: Level::new(level),
                strain,
            })
        })
    });
    [Call::Pass, Call::Double, Call::Redouble]
        .into_iter()
        .chain(bids)
        .collect()
}

/// How often each node made each call, over the whole run
type Support = HashMap<(String, Call), u32>;

fn census(
    partnership: &pons::bidding::Partnership,
    prior: &Counter,
    support: &Support,
    dealer: Seat,
    deal: &contract_bridge::FullDeal,
    auction: &Auction,
) -> (Vec<Row>, Bins) {
    let vul = AbsoluteVulnerability::NONE;
    let counters = Seat::ALL.map(|seat| Counter::new(deal[seat]));
    let calls = all_calls();
    let (mut rows, mut bins) = (Vec::new(), Bins::default());
    let mut prefix = Auction::new();
    for cut in 0..auction.len() {
        let caller = seat_to_act(dealer, cut);
        let seen = &counters[(caller as usize + 2) % 4];
        // The caller's reading before the call, off the last seat to act.
        let before = cut.checked_sub(1).map_or(1.0, |last| {
            let read =
                partnership.infer(relative(vul, seat_to_act(dealer, last)), &auction[..last]);
            seen.mass(read.announced_union(Relative::Lho))
        });
        // The reading each reply leaves, off the next seat to act.
        let next = relative(vul, seat_to_act(dealer, cut + 1));
        let mut line = auction[..cut].to_vec();
        let node = auction_key(&line);
        let made = auction[cut];
        let authored =
            (partnership.classify_with_provenance(deal[caller], relative(vul, caller), &line))
                .is_some_and(|(_, provenance)| provenance.is_authored());
        // Leave this deal out: a call only it made is not a known reply.
        let known = |reply: Call| {
            (support.get(&(node.clone(), reply))).is_some_and(|&n| n > u32::from(reply == made))
        };
        let masses: Vec<(Call, f64, f64)> = (calls.iter())
            .filter(|&&reply| prefix.can_push(reply).is_ok() && known(reply))
            .map(|&reply| {
                line.push(reply);
                let read = partnership.infer(next, &line);
                let union = read.announced_union(Relative::Rho);
                line.pop();
                (
                    reply,
                    seen.mass(union),
                    prior.count(union) as f64 / ALL_HANDS,
                )
            })
            .collect();
        let (sum, sum_free): (f64, f64) = masses
            .iter()
            .fold((0.0, 0.0), |(a, b), &(_, m, f)| (a + m, b + f));
        let best = masses
            .iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|&(reply, ..)| reply);
        let of = |pick: fn(&(Call, f64, f64)) -> f64, sum: f64| {
            let mass = masses
                .iter()
                .find(|entry| entry.0 == made)
                .map_or(0.0, pick);
            if sum > 0.0 { mass / sum } else { 0.0 }
        };
        if sum > 0.0 {
            for &(reply, mass, _) in &masses {
                let p = mass / sum;
                let bin = &mut bins[((p * 10.0) as usize).min(9)];
                bin.0 += 1;
                bin.1 += p;
                bin.2 += u64::from(reply == made);
            }
        }
        rows.push(Row {
            node,
            call: made,
            exact: of(|entry| entry.1, sum),
            free: of(|entry| entry.2, sum_free),
            top: best == Some(made),
            overlap: sum / before,
            authored,
        });
        prefix.push(made);
    }
    (rows, bins)
}

fn loss(p: f64) -> f64 {
    -p.max(FLOOR).ln()
}

/// What a set of decisions scores: miss rate, top-1, mean overlap, and the
/// three log-losses (exact, hand-free, node frequencies)
struct Score {
    n: usize,
    miss: f64,
    top: f64,
    overlap: f64,
    authored: f64,
    exact: f64,
    free: f64,
    node: f64,
}

fn score(rows: &[(&Row, f64)]) -> Score {
    let n = rows.len() as f64;
    let mean = |f: &dyn Fn(&(&Row, f64)) -> f64| rows.iter().map(f).sum::<f64>() / n;
    Score {
        n: rows.len(),
        miss: mean(&|(row, _)| f64::from(u8::from(row.exact == 0.0))),
        top: mean(&|(row, _)| f64::from(u8::from(row.top))),
        overlap: mean(&|(row, _)| row.overlap),
        authored: mean(&|(row, _)| f64::from(u8::from(row.authored))),
        exact: mean(&|(row, _)| loss(row.exact)),
        free: mean(&|(row, _)| loss(row.free)),
        node: mean(&|(_, marginal)| loss(*marginal)),
    }
}

impl Score {
    fn countable(&self) -> bool {
        self.n >= NODE_FLOOR && self.miss <= MISS_CEILING && self.exact <= self.node
    }

    fn print(&self, label: &str) {
        println!(
            "{:>7} {:>6.2}% {:>6.1}% {:>7.2} {:>6.1}% {:>7.3} {:>7.3} {:>7.3}  {}{label}",
            self.n,
            100.0 * self.miss,
            100.0 * self.top,
            self.overlap,
            100.0 * self.authored,
            self.exact,
            self.free,
            self.node,
            if self.countable() { "✓ " } else { "  " },
        );
    }
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let partnership = american(&pons::bidding::agreements::Agreements::default()).bind();
    let prior = Counter::new(Hand::EMPTY);
    let deals = seeded_deals(base, args.count);
    let vul = AbsoluteVulnerability::NONE;
    let auctions: Vec<Auction> = (deals.par_iter().enumerate())
        .map(|(board, deal)| {
            bid_out(
                &partnership,
                &partnership,
                true,
                Seat::ALL[board % 4],
                vul,
                deal,
            )
        })
        .collect();
    let mut support = Support::new();
    for auction in &auctions {
        for cut in 0..auction.len() {
            *support
                .entry((auction_key(&auction[..cut]), auction[cut]))
                .or_default() += 1;
        }
    }
    let (rows, bins) = (deals.par_iter().zip(&auctions).enumerate())
        .map(|(board, (deal, auction))| {
            census(
                &partnership,
                &prior,
                &support,
                Seat::ALL[board % 4],
                deal,
                auction,
            )
        })
        .reduce(
            || (Vec::new(), Bins::default()),
            |(mut rows, mut bins), (more, other)| {
                rows.extend(more);
                for (bin, add) in bins.iter_mut().zip(other) {
                    *bin = (bin.0 + add.0, bin.1 + add.1, bin.2 + add.2);
                }
                (rows, bins)
            },
        );

    // The node's own reply frequencies, leaving the decision itself out.
    let mut replies: HashMap<(&str, Call), f64> = HashMap::new();
    let mut visits: HashMap<&str, f64> = HashMap::new();
    for row in &rows {
        *replies.entry((&row.node, row.call)).or_default() += 1.0;
        *visits.entry(&row.node).or_default() += 1.0;
    }
    let scored: Vec<(&Row, f64)> = (rows.iter())
        .map(|row| {
            let marginal = (replies[&(row.node.as_str(), row.call)] - 1.0 + EPSILON)
                / (visits[row.node.as_str()] - 1.0 + 38.0 * EPSILON);
            (row, marginal)
        })
        .collect();
    let mut nodes: HashMap<&str, Vec<(&Row, f64)>> = HashMap::new();
    for &(row, marginal) in &scored {
        nodes.entry(&row.node).or_default().push((row, marginal));
    }
    let mut nodes: Vec<(&str, Score)> = (nodes.iter())
        .map(|(node, rows)| (*node, score(rows)))
        .collect();
    nodes.sort_unstable_by(|a, b| b.1.n.cmp(&a.1.n).then_with(|| a.0.cmp(b.0)));

    println!("boards {}  seed {base}\n", args.count);
    println!(
        "{:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}  slice (✓ = countable)",
        "calls", "miss", "top-1", "overlap", "book", "exact", "free", "node"
    );
    score(&scored).print("every call");
    let rated: Vec<(&Row, f64)> = (scored.iter().copied())
        .filter(|(row, _)| visits[row.node.as_str()] >= NODE_FLOOR as f64)
        .collect();
    score(&rated).print(&format!("calls at nodes met ≥ {NODE_FLOOR} times"));
    let bids: Vec<(&Row, f64)> = (rated.iter().copied())
        .filter(|(row, _)| row.node.chars().any(|c| c.is_ascii_digit()))
        .collect();
    score(&bids).print("… of them after the opening bid");

    let countable: usize = (nodes.iter())
        .filter(|node| node.1.countable())
        .map(|node| node.1.n)
        .sum();
    println!(
        "\ncountable: {} of {} nodes met ≥ {NODE_FLOOR} times, holding {:.1}% of their calls",
        nodes.iter().filter(|node| node.1.countable()).count(),
        nodes.iter().filter(|node| node.1.n >= NODE_FLOOR).count(),
        100.0 * countable as f64 / rated.len() as f64,
    );
    // A node with one known reply is countable for free; the ones that matter
    // have a choice, read off the node's own frequencies.
    let choice = |node: &&(&str, Score)| node.1.n >= NODE_FLOOR && node.1.node > CHOICE;
    println!(
        "with a choice (node loss > {CHOICE}): {} of {}; and the reader's hand helps at {}",
        nodes
            .iter()
            .filter(choice)
            .filter(|node| node.1.countable())
            .count(),
        nodes.iter().filter(choice).count(),
        (nodes.iter().filter(choice))
            .filter(|node| node.1.countable() && node.1.exact < node.1.free - CHOICE)
            .count(),
    );

    println!("\nreliability of the exact forecast, over every legal reply\n");
    println!(
        "{:>10} {:>10} {:>9} {:>9}",
        "forecast", "replies", "mean p", "made"
    );
    for (i, (n, sum, hits)) in bins.into_iter().enumerate() {
        println!(
            "{:>4}–{:<4}% {n:>10} {:>8.2}% {:>8.2}%",
            10 * i,
            10 * i + 10,
            100.0 * sum / n as f64,
            100.0 * hits as f64 / n as f64,
        );
    }

    println!("\nnodes by visits\n");
    println!(
        "{:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7} {:>7}  auction before the call",
        "calls", "miss", "top-1", "overlap", "book", "exact", "free", "node"
    );
    for (node, score) in (nodes.iter())
        .filter(|node| node.0.starts_with(&args.prefix))
        .take(args.top)
    {
        score.print(if node.is_empty() { "(dealer)" } else { node });
    }
}
