//! eval-columns — per-declarer-column error of the **shipped** trick evaluator.
//!
//! Pre-A/B gates 0 and 1 of `docs/ai-bidder/competitive-accountant.md`. The
//! competitive accountant prices *their* contract as well as ours, so it reads
//! the evaluator's `Relative::Lho` / `Relative::Rho` columns — which are
//! computed on every forward pass and have **never been validated separately**.
//! Nothing else in the repo scores those columns: the trainer pools all 20
//! targets, `examples/eval-evaluator` pools all four declarers into one `Mean`,
//! reports no coverage at all, and scores against a sampler rather than the
//! deal.
//!
//! The measurement is deliberately of the *shipped* weights, not of a fresh
//! net: it bids each deal out with `american()`, reads the envelopes with
//! `Partnership::infer`, and calls
//! `trick_estimates_with_auction`
//! — the same entry point `Context::trick_estimates` serves at classify time.
//! Truth is the deal's own cached double-dummy table via
//! `gib::relativized_tricks`, so **no solver runs and no corpus file is
//! needed**; the `.pdd` bank already holds the labels.
//!
//! Three slices, each broken out by declarer column:
//!
//! - **all** — every judgement node;
//! - **contested** — `Phase::of(&auction) != Phase::Constructive`;
//! - **gate** — the accountant's own trigger: the last live call is *their*
//!   undoubled bid at level ≥ 4 and our side has already named a strain.
//!   Because the real auction is walked, this is the trigger exactly, not a
//!   feature-window approximation of it.
//!
//! **Gate 0** is the trigger rate: how often that node occurs at all. Below the
//! pre-registered 1% of boards a standard A/B cannot resolve the gate, and the
//! trigger widens to include 3NT-by-them. Caveat named in the output: both
//! sides bid `american()` here, while the A/B's opponents are BBA, so this is a
//! self-play proxy — cross-check against retained arm dumps before acting on a
//! near-miss.
//!
//! **Gate 1** passes iff, on the gate slice, the LHO/RHO columns' MAE is within
//! 0.15 tricks of the me/partner columns' and their coverage at `μ ± 0.6745σ`
//! is within 3 points of ours. Both criteria are *relative*: the shipped net
//! sits near 48.5% absolute coverage, so an absolute band would mostly grade
//! global calibration, which all four columns share. The absolute figure is
//! reported beside it. On failure the report prints the σ multiplier `k` that
//! brings their coverage up to ours — the pre-registered fallback is to inflate
//! their-side σ by that factor and keep all three gate actions.
//!
//! ```text
//! scripts/idle-run.sh cargo run --release --all-features --example eval-columns -- \
//!     --deals /nfs2/jdh8/pons/22.pdd --skip 5000000 --count 200000
//! ```
//!
//! `22.pdd` rows 0..1M are burned as evaluator training draws and 2.5M..4.2M by
//! the configured net (`docs/pdd-bank-ledger.md`), hence the default `--skip`.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat, Strain};
use ddss::TrickCountTable;
use pons::bidding::context::relative;
use pons::bidding::evaluator::trick_estimates_with_auction;
use pons::bidding::inference::Relative;
use pons::bidding::{Bidder, Partnership, Phase};
use pons::{american, gib};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use rayon::prelude::*;

/// Φ⁻¹(0.75) — the half-width of the interquartile band, in σ. Matches the
/// trainer's `Z75`, so coverage here is the same statistic it publishes.
const Z75: f64 = 0.674_490;

/// Label rows, in the GIB order `gib::relativized_tricks` emits: NT♠♥♦♣.
const STRAIN_ROWS: [Strain; 5] = [
    Strain::Notrump,
    Strain::Spades,
    Strain::Hearts,
    Strain::Diamonds,
    Strain::Clubs,
];

/// Label columns, in the order `TrickEstimates::get` indexes them.
const DECLARERS: [Relative; 4] = [
    Relative::Me,
    Relative::Lho,
    Relative::Partner,
    Relative::Rho,
];

const COLUMN_NAMES: [&str; 4] = ["me", "lho", "partner", "rho"];
const SLICE_NAMES: [&str; 3] = ["all", "contested", "gate"];

/// Buckets of `|error| / (0.6745·σ)`, capped at `RATIO_MAX`. Coverage at any
/// σ multiplier is then a prefix sum, so the fallback's inflation factor is a
/// lookup rather than a re-run.
const BINS: usize = 512;
const RATIO_MAX: f64 = 8.0;

/// The four absolute vulnerabilities, sampled uniformly per board — vulnerability
/// never reaches the evaluator, but it moves the auctions that feed it.
const VULS: [AbsoluteVulnerability; 4] = [
    AbsoluteVulnerability::NONE,
    AbsoluteVulnerability::NS,
    AbsoluteVulnerability::EW,
    AbsoluteVulnerability::ALL,
];

#[derive(Parser)]
#[command(about = "Per-declarer-column error of the shipped trick evaluator (gates 0 and 1)")]
struct Args {
    /// Pre-solved deal database: binary `.pdd` (sliceable) or GIB text
    #[arg(long, default_value = "/nfs2/jdh8/pons/22.pdd")]
    deals: String,
    /// Skip this many deals — the default clears both burned ranges of `22.pdd`
    #[arg(long, default_value_t = 5_000_000)]
    skip: u64,
    /// Number of deals to bid out
    #[arg(long, default_value_t = 200_000)]
    count: usize,
    /// RNG seed for the dealer/vulnerability stream
    #[arg(long, default_value_t = 0)]
    seed: u64,
}

/// Error moments plus the ratio histogram, for one (slice, declarer) cell
#[derive(Clone)]
struct Cell {
    n: u64,
    abs: f64,
    sq: f64,
    /// `|error| / (0.6745·σ)`, `BINS` buckets over `0..RATIO_MAX` plus overflow
    hist: Vec<u64>,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            n: 0,
            abs: 0.0,
            sq: 0.0,
            hist: vec![0; BINS + 1],
        }
    }
}

impl Cell {
    fn push(&mut self, error: f64, sd: f64) {
        self.n += 1;
        self.abs += error.abs();
        self.sq += error * error;
        // A degenerate σ lands in the overflow bucket rather than panicking;
        // `as usize` saturates, so an infinite ratio clamps to `BINS`.
        let ratio = error.abs() / (Z75 * sd);
        let bin = (ratio / RATIO_MAX * BINS as f64) as usize;
        self.hist[bin.min(BINS)] += 1;
    }

    fn merge(&mut self, other: &Self) {
        self.n += other.n;
        self.abs += other.abs;
        self.sq += other.sq;
        for (slot, count) in self.hist.iter_mut().zip(&other.hist) {
            *slot += count;
        }
    }

    fn mae(&self) -> f64 {
        self.abs / self.n.max(1) as f64
    }

    fn rmse(&self) -> f64 {
        (self.sq / self.n.max(1) as f64).sqrt()
    }

    /// Fraction inside `μ ± k·0.6745·σ`; `coverage(1.0)` is the published statistic
    fn coverage(&self, k: f64) -> f64 {
        let upto = (k / RATIO_MAX * BINS as f64)
            .round()
            .clamp(0.0, BINS as f64) as usize;
        self.hist[..upto].iter().sum::<u64>() as f64 / self.n.max(1) as f64
    }

    /// Smallest σ multiplier whose coverage reaches `target` — the pre-registered
    /// gate-1 fallback factor, read off the histogram grid
    fn multiplier_for(&self, target: f64) -> f64 {
        let need = (target * self.n as f64).ceil() as u64;
        let mut seen = 0;
        for (bin, count) in self.hist.iter().enumerate() {
            seen += count;
            if seen >= need {
                return (bin + 1) as f64 * RATIO_MAX / BINS as f64;
            }
        }
        f64::INFINITY
    }
}

#[derive(Clone, Default)]
struct Stats {
    /// `slice * 4 + declarer`
    cells: Vec<Cell>,
    nodes: u64,
    forced: u64,
    trigger_nodes: u64,
    trigger_boards: u64,
}

impl Stats {
    fn new() -> Self {
        Self {
            cells: vec![Cell::default(); SLICE_NAMES.len() * DECLARERS.len()],
            ..Self::default()
        }
    }

    fn merge(&mut self, other: &Self) {
        for (slot, cell) in self.cells.iter_mut().zip(&other.cells) {
            slot.merge(cell);
        }
        self.nodes += other.nodes;
        self.forced += other.forced;
        self.trigger_nodes += other.trigger_nodes;
        self.trigger_boards += other.trigger_boards;
    }

    /// One group's cells pooled — "ours" is me+partner, "theirs" is lho+rho
    fn pooled(&self, slice: usize, columns: &[usize]) -> Cell {
        let mut out = Cell::default();
        for &column in columns {
            out.merge(&self.cells[slice * DECLARERS.len() + column]);
        }
        out
    }
}

/// The accountant's trigger, read straight off the auction
///
/// Mirrors `instinct::their_live_bid_at_most` with the comparison inverted: the
/// last non-pass call must be a *bid* (so an undoubled contract, since X and XX
/// are not `Call::Bid`) made by an opponent (odd distance from the seat to act)
/// at level 4 or higher, and our side must already have named a strain to have
/// anything to bid on.
fn gate_node(auction: &[Call]) -> bool {
    let Some(index) = auction.iter().rposition(|&call| call != Call::Pass) else {
        return false;
    };
    if (auction.len() - index) % 2 != 1 {
        return false;
    }
    let Call::Bid(bid) = auction[index] else {
        return false;
    };
    bid.level.get() >= 4
        && auction
            .iter()
            .enumerate()
            .any(|(i, call)| matches!(call, Call::Bid(_)) && (auction.len() - i).is_multiple_of(2))
}

fn score_deal(
    partnership: &Partnership,
    deal: &FullDeal,
    table: &TrickCountTable,
    index: u64,
    seed: u64,
    stats: &mut Stats,
) {
    let mut rng = StdRng::seed_from_u64(seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let dealer = rng.random_range(0..4usize);
    let vul = VULS[rng.random_range(0..4usize)];

    let mut auction = Auction::new();
    let mut touched = false;
    while !auction.has_ended() {
        let seat = Seat::ALL[(dealer + auction.len()) % 4];
        let hand = deal[seat];
        let rel = relative(vul, seat);

        let Some(logits) = partnership.classify(hand, rel, &auction) else {
            // Forced: the deterministic shell acts, so there is no judgement to
            // price and the gate never sees this node.
            stats.forced += 1;
            auction.push(Call::Pass);
            continue;
        };
        stats.nodes += 1;

        let mut slices = vec![0usize];
        if Phase::of(&auction) != Phase::Constructive {
            slices.push(1);
        }
        if gate_node(&auction) {
            slices.push(2);
            stats.trigger_nodes += 1;
            touched = true;
        }

        let inferences = partnership.infer(rel, &auction);
        let estimates = trick_estimates_with_auction(hand, &inferences, &auction);
        let truth = gib::relativized_tricks(table, seat);
        for (row, &strain) in STRAIN_ROWS.iter().enumerate() {
            for (column, &declarer) in DECLARERS.iter().enumerate() {
                let gaussian = estimates.get(strain, declarer);
                let error = f64::from(gaussian.mean) - 13.0 * f64::from(truth[row * 4 + column]);
                let sd = f64::from(gaussian.sd);
                for &slice in &slices {
                    stats.cells[slice * DECLARERS.len() + column].push(error, sd);
                }
            }
        }

        let chosen = logits
            .iter()
            .filter(|&(call, logit)| logit.is_finite() && auction.can_push(call).is_ok())
            .max_by(|a, b| a.1.partial_cmp(b.1).expect("logits are never NaN"))
            .map_or(Call::Pass, |(call, _)| call);
        auction.push(chosen);
    }
    stats.trigger_boards += u64::from(touched);
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let agreements = pons::bidding::agreements::Agreements::default();
    let partnership = american(&agreements).bind();
    let deals = pons::pdd::load_slice(&args.deals, args.skip, args.count)?;
    eprintln!(
        "eval-columns: {} deals from {} (skip {})",
        deals.len(),
        args.deals,
        args.skip
    );

    let stats = deals
        .par_iter()
        .enumerate()
        .fold(Stats::new, |mut stats, (index, (deal, table))| {
            score_deal(
                &partnership,
                deal,
                table,
                index as u64,
                args.seed,
                &mut stats,
            );
            stats
        })
        .reduce(Stats::new, |mut a, b| {
            a.merge(&b);
            a
        });

    report(&stats, deals.len() as u64);
    Ok(())
}

fn report(stats: &Stats, boards: u64) {
    println!(
        "boards {boards}  judgement nodes {}  forced {}",
        stats.nodes, stats.forced
    );
    println!(
        "\n## gate 0 — trigger rate\n\n\
         trigger nodes    {:>9}  ({:.3}% of judgement nodes)\n\
         boards touched   {:>9}  ({:.3}% of boards; pre-registered floor 1%)\n\
         verdict          {}\n\n\
         Caveat: both sides bid `american()` here; the A/B's opponents are BBA, so this is a\n\
         self-play proxy for how often the node is reached.",
        stats.trigger_nodes,
        100.0 * stats.trigger_nodes as f64 / stats.nodes.max(1) as f64,
        stats.trigger_boards,
        100.0 * stats.trigger_boards as f64 / boards.max(1) as f64,
        if stats.trigger_boards as f64 >= 0.01 * boards as f64 {
            "PASS"
        } else {
            "FAIL — widen the trigger to 3NT-by-them"
        }
    );

    println!("\n## per-declarer-column error, shipped `evaluator_v3_dnf`\n");
    println!("| slice | column | n | MAE | RMSE | coverage |");
    println!("| --- | --- | ---: | ---: | ---: | ---: |");
    for (slice, name) in SLICE_NAMES.iter().enumerate() {
        for (column, label) in COLUMN_NAMES.iter().enumerate() {
            let cell = &stats.cells[slice * DECLARERS.len() + column];
            println!(
                "| {name} | {label} | {} | {:.4} | {:.4} | {:.2}% |",
                cell.n,
                cell.mae(),
                cell.rmse(),
                100.0 * cell.coverage(1.0)
            );
        }
    }

    println!("\n## gate 1 — their columns vs ours\n");
    println!(
        "| slice | MAE ours | MAE theirs | Δ MAE | cov ours | cov theirs | Δ cov | σ factor |"
    );
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    for (slice, name) in SLICE_NAMES.iter().enumerate() {
        let ours = stats.pooled(slice, &[0, 2]);
        let theirs = stats.pooled(slice, &[1, 3]);
        let factor = theirs.multiplier_for(ours.coverage(1.0));
        println!(
            "| {name} | {:.4} | {:.4} | {:+.4} | {:.2}% | {:.2}% | {:+.2}pp | {:.3} |",
            ours.mae(),
            theirs.mae(),
            theirs.mae() - ours.mae(),
            100.0 * ours.coverage(1.0),
            100.0 * theirs.coverage(1.0),
            100.0 * (theirs.coverage(1.0) - ours.coverage(1.0)),
            factor
        );
    }

    let ours = stats.pooled(2, &[0, 2]);
    let theirs = stats.pooled(2, &[1, 3]);
    let mae_gap = theirs.mae() - ours.mae();
    let cov_gap = 100.0 * (theirs.coverage(1.0) - ours.coverage(1.0));
    let mae_ok = mae_gap <= 0.15;
    let cov_ok = cov_gap.abs() <= 3.0;
    println!(
        "\ngate slice: Δ MAE {mae_gap:+.4} (bound 0.15) {}; Δ coverage {cov_gap:+.2}pp \
         (bound 3.00pp) {}\ngate 1 verdict: {}",
        if mae_ok { "PASS" } else { "FAIL" },
        if cov_ok { "PASS" } else { "FAIL" },
        if mae_ok && cov_ok {
            "PASS".to_string()
        } else {
            format!(
                "FAIL — inflate their-side σ by k = {:.3} and keep all three actions",
                theirs.multiplier_for(ours.coverage(1.0))
            )
        }
    );
}

#[cfg(test)]
mod tests {
    use super::{Cell, gate_node};
    use contract_bridge::Bid;
    use contract_bridge::auction::Call;

    /// `1♥ (4♠)` from the seat to act: index parity runs backwards from the
    /// actor, so the last call is RHO's and the one before it is partner's.
    fn calls(spec: &[&str]) -> Vec<Call> {
        spec.iter()
            .map(|&call| match call {
                "-" => Call::Pass,
                "X" => Call::Double,
                _ => Call::Bid(call.parse::<Bid>().expect("a legal bid")),
            })
            .collect()
    }

    #[test]
    fn trigger_wants_their_live_bid_at_four_with_a_suit_of_ours() {
        // Nothing bid, and a one-level auction: no contract to price.
        assert!(!gate_node(&calls(&[])));
        assert!(!gate_node(&calls(&["1♠"])));
        // Their 4♠ but we have named nothing — no candidate bid to veto.
        assert!(!gate_node(&calls(&["4♠"])));
        // The node the accountant exists for: partner opened, RHO leapt to 4♠.
        assert!(gate_node(&calls(&["1♥", "4♠"])));
        // Three-level: below the trigger, the book's double rules still reach it.
        assert!(!gate_node(&calls(&["1♥", "3♠"])));
        // 4♠ is now *partner's* two calls back — ours, not theirs.
        assert!(!gate_node(&calls(&["1♥", "4♠", "-"])));
        // Already doubled: the last live call is not a bid, so the contract is
        // no longer the undoubled one the EV integrals price.
        assert!(!gate_node(&calls(&["1♥", "4♠", "X"])));
    }

    /// The histogram is the σ-inflation fallback's only input, so its two
    /// readings must agree with the counts that fed it.
    #[test]
    fn coverage_and_multiplier_read_the_same_histogram() {
        let mut cell = Cell::default();
        // Four errors at exactly 1σ and one at 3σ, all with σ = 1: three
        // quarters of the mass sits inside 0.6745σ · k for k just over 1.4826.
        for _ in 0..3 {
            cell.push(0.5, 1.0);
        }
        cell.push(3.0, 1.0);
        // 0.5 / 0.6745 = 0.741, so k = 1 excludes nothing but the 3σ outlier.
        assert!((cell.coverage(1.0) - 0.75).abs() < 1e-9);
        assert!((cell.mae() - 1.125).abs() < 1e-9);
        // Reaching 75% needs a multiplier just past 0.741; reaching 100% needs
        // one past 3 / 0.6745 = 4.448.
        let three_quarters = cell.multiplier_for(0.75);
        assert!((0.741..0.76).contains(&three_quarters), "{three_quarters}");
        let all = cell.multiplier_for(1.0);
        assert!((4.448..4.47).contains(&all), "{all}");
    }
}
