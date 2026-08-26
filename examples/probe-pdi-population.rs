//! What our side **actually holds** when it passes (or doubles) after a PDI trigger
//!
//! Task 3 of the post-loss PDI redesign (docs/pdi.md).  The v2 design reads a
//! post-trigger `P` over RHO's live suit bid as the **negation of the trap** — a
//! two-term union `[their-suit ≤ ceiling] ∪ [points ≤ cap]`, which excludes
//! exactly the hands that are *long in their suit* **and** *strong enough to
//! punish*.  The two cut points are set from the population, never a priori
//! (docs/ai-bidder/sampled-projection.md: read a call off the **bidder**, not
//! its rules), and the same table decides **tag hygiene**: a lane whose passers
//! genuinely hold trap hands cannot support the claim and gets untagged instead
//! of read unsoundly.
//!
//! The population comes off existing `bba-gen` dumps rather than a fresh
//! self-play sweep: their `table_a` is our shipped pair (North/South) against
//! the real opponent at ~400k boards a vulnerability, which is the traffic the
//! A/B will be scored on.  Point the tool at the *baseline* arm of any run —
//! the knob is default-off, so `base-*` is today's bidder.
//!
//! For every one of our side's calls made over RHO's live **suit** bid, with
//! our side already latched at that point ([`Inferences::pdi_latched`]), the
//! report gives, per call class:
//!
//! - the **exclusion table** `n(their-suit ≥ L and points ≥ P)` — the count the
//!   union would refuse.  Read it down the `Pass` block for soundness (pick a
//!   corner that is empty, or as near as makes no difference) and across the
//!   `Double` block for content (a corner that excludes no doublers either is a
//!   claim that says nothing).
//! - the marginal length and point histograms, quantiles included, so a
//!   support edge is visible before a threshold is trusted.
//!
//! Lanes are keyed by the auction prefix up to and including our side's
//! **earliest double** — the trigger itself for a tagged double, and partner's
//! doubled suit for a structural conversion, which is the same lane either way.
//!
//! No double-dummy, no solver, no bidding: replay + read.
//!
//! ```sh
//! cargo run --release --features serde --example probe-pdi-population -- \
//!     /mnt/hdd-data/jdh8/pons-ab-results/pdi-latch-p2-swap-20260826-c1b3a846/base-none
//! ```

use clap::Parser;
use contract_bridge::auction::{Call, display_calls};
use contract_bridge::{Hand, Seat, Suit};
use pons::american;
use pons::bidding::constraint::point_count;
use pons::bidding::context::relative;
use pons::bidding::{Inferences, Partnership, Relative};
use rayon::prelude::*;
use std::collections::HashMap;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{load_dump, seat_to_act, vs_bba_agreements};

/// Widest grids the exclusion table sweeps; `--lengths` / `--strengths` cut into these
const LENGTH_SLOTS: usize = 6;
const STRENGTH_SLOTS: usize = 8;

#[derive(Parser)]
struct Args {
    /// One or more `bba-gen` dumps (a directory folds its `shard-*.json`)
    #[arg(required = true)]
    dumps: Vec<String>,

    /// Lanes to report, ranked by sample count
    #[arg(long, default_value_t = 12)]
    top: usize,

    /// Skip lanes with fewer samples than this
    #[arg(long, default_value_t = 40)]
    min_count: usize,

    /// Only count decisions over their bid at or below this level
    #[arg(long, default_value_t = 7)]
    max_level: u8,

    /// Their-suit floor of the trap zone the per-lane census ranks by
    #[arg(long, default_value_t = 5)]
    zone_len: u8,

    /// Strength floor of the trap zone the per-lane census ranks by
    #[arg(long, default_value_t = 8)]
    zone_str: u8,

    /// Their-suit length floors to sweep (at most six)
    #[arg(long, value_delimiter = ',', default_values_t = [3u8, 4, 5, 6, 7])]
    lengths: Vec<u8>,

    /// Strength floors to sweep (at most eight)
    #[arg(long, value_delimiter = ',', default_values_t = [8u8, 10, 11, 12, 13, 14, 16])]
    strengths: Vec<u8>,

    /// Also write one row per latched decision here, for slicing outside the tool
    ///
    /// `lane,cut,index,level,class,their_len,points,hcp,suit_hcp,conversion`
    #[arg(long)]
    csv: Option<String>,

    /// Also count decisions over RHO's **notrump** bid, which the length term cannot cut
    #[arg(long)]
    include_notrump: bool,

    /// Read the dump under bare `Agreements::default()` instead of `vs_bba_agreements`
    ///
    /// Only for a dump that really was bid without the BBA opponent disclosures.
    #[arg(long)]
    plain_default: bool,

    /// Which strength axis the trap's second term is cut on
    ///
    /// `points` is the shipped `point_count` scale [`Strength::points`] narrows;
    /// `hcp` is raw HCP; `suit-hcp` is the honour weight held **in their suit**
    /// ([`Strength::suit_hcp`]) — the sharpest available gauge of "I can punish
    /// this", since whole-hand points pay for shape a trump stack does not have.
    #[arg(long, default_value = "points")]
    strength: Axis,
}

/// The strength axis the exclusion table's second term is cut on
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Axis {
    Points,
    Hcp,
    SuitHcp,
}

impl Axis {
    fn of(self, hand: Hand, their_suit: contract_bridge::Suit) -> u8 {
        match self {
            Self::Points => point_count(hand),
            Self::Hcp => contract_bridge::Suit::ASC
                .iter()
                .map(|&suit| contract_bridge::eval::hcp::<u8>(hand[suit]))
                .sum(),
            Self::SuitHcp => contract_bridge::eval::hcp(hand[their_suit]),
        }
    }
}

/// One run's sweep grids and scope, cut from the flags
struct Grid {
    lengths: Vec<u8>,
    strengths: Vec<u8>,
    axis: Axis,
    max_level: u8,
    include_notrump: bool,
}

/// Which call our side chose over RHO's live suit bid
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Class {
    Pass,
    Double,
    Bid,
    /// Structurally identical but **not** latched — the reference class a
    /// reading's content is measured against.  A zone's share of `Control` is
    /// the prior mass the claim removes; its share of `Pass` is the mass the
    /// claim wrongly removes.
    Control,
}

impl Class {
    const ALL: [Self; 4] = [Self::Pass, Self::Double, Self::Bid, Self::Control];

    const fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS  ",
            Self::Double => "DOUBLE",
            Self::Bid => "BID   ",
            Self::Control => "UNLATCHED",
        }
    }
}

/// One call class's sample: the joint tail counts plus the two marginals
#[derive(Clone)]
struct Agg {
    count: u64,
    /// `tail[l][p]` — samples with their-suit ≥ `lengths[l]` **and** strength ≥ `strengths[p]`
    tail: [[u64; STRENGTH_SLOTS]; LENGTH_SLOTS],
    lengths: [u64; 14],
    points: [u64; 38],
    /// Decisions split by whether their bid was at the three level or below
    low: u64,
}

impl Default for Agg {
    fn default() -> Self {
        Self {
            count: 0,
            tail: [[0; STRENGTH_SLOTS]; LENGTH_SLOTS],
            lengths: [0; 14],
            points: [0; 38],
            low: 0,
        }
    }
}

impl Agg {
    /// Samples inside an arbitrary trap zone, interpolation-free (the grids are floors)
    fn zone(&self, grid: &Grid, length_floor: u8, strength_floor: u8) -> Option<u64> {
        let l = grid.lengths.iter().position(|&x| x == length_floor)?;
        let p = grid.strengths.iter().position(|&x| x == strength_floor)?;
        Some(self.tail[l][p])
    }

    fn add(&mut self, grid: &Grid, their_len: u8, strength: u8, low: bool) {
        self.count += 1;
        self.low += u64::from(low);
        self.lengths[usize::from(their_len).min(13)] += 1;
        self.points[usize::from(strength).min(37)] += 1;
        for (l, &length_floor) in grid.lengths.iter().enumerate() {
            for (p, &strength_floor) in grid.strengths.iter().enumerate() {
                if their_len >= length_floor && strength >= strength_floor {
                    self.tail[l][p] += 1;
                }
            }
        }
    }

    fn merge(&mut self, other: &Self) {
        self.count += other.count;
        self.low += other.low;
        for (row, other_row) in self.tail.iter_mut().zip(other.tail) {
            for (a, b) in row.iter_mut().zip(other_row) {
                *a += b;
            }
        }
        for (a, b) in self.lengths.iter_mut().zip(other.lengths) {
            *a += b;
        }
        for (a, b) in self.points.iter_mut().zip(other.points) {
            *a += b;
        }
    }
}

/// The three classes of one lane, plus the first prefix seen for the label
#[derive(Default, Clone)]
struct Lane {
    by_class: HashMap<Class, Agg>,
}

impl Lane {
    fn add(&mut self, grid: &Grid, class: Class, their_len: u8, strength: u8, low: bool) {
        self.by_class
            .entry(class)
            .or_default()
            .add(grid, their_len, strength, low);
    }

    fn merge(&mut self, other: &Self) {
        for (class, agg) in &other.by_class {
            self.by_class.entry(*class).or_default().merge(agg);
        }
    }

    /// Latched decisions only — the unlatched control is a reference class, not traffic
    fn count(&self) -> u64 {
        self.by_class
            .iter()
            .filter(|(class, _)| **class != Class::Control)
            .map(|(_, agg)| agg.count)
            .sum()
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

/// Our side's earliest double, as the lane label
///
/// A tagged penalty double *is* the trigger; a structural conversion pass
/// converts partner's double, which is our side's earliest double too — so one
/// cut names both kinds of lane, and names them by the doubled call.
fn lane_key(auction: &[Call], len: usize) -> String {
    let cut = (0..len)
        .find(|index| index % 2 == len % 2 && auction[*index] == Call::Double)
        .map_or(len, |index| index + 1);
    display_calls(&auction[..cut]).to_string()
}

/// One latched decision, for the `--csv` dump
struct Row {
    lane: String,
    cut: usize,
    index: usize,
    level: u8,
    class: Class,
    their_len: u8,
    points: u8,
    hcp: u8,
    suit_hcp: u8,
    /// The latest trigger of ours is a **structural conversion pass**, not a tagged double
    conversion: bool,
    /// This is our side's **very next turn** after the trigger — the latch was not
    /// yet on two calls ago.  Exact, not structural: the same predicate re-run on
    /// the shorter prefix, which carries our parity.
    fresh: bool,
    /// What our own prior calls already showed at this moment: the points floor and
    /// the their-suit length floor.  A union term dies — and the union collapses to
    /// the other term, which then narrows the **hull** the floor and the evaluator
    /// read — exactly when one of these already contradicts it.
    prior_points_min: u8,
    prior_len_min: u8,
    /// The their-suit length **ceiling** the walk already gives the passer.  This
    /// is the hull-delta denominator: a union that collapses to `len ≤ ceiling`
    /// changes nothing where this is already at or below it.
    prior_len_max: u8,
}

/// Every latched decision our side made over RHO's live suit bid, on one board
fn per_board(
    partnership: &Partnership,
    grid: &Grid,
    deal: &contract_bridge::FullDeal,
    dealer: Seat,
    auction: &[Call],
    vul: contract_bridge::AbsoluteVulnerability,
) -> (HashMap<String, Lane>, Vec<Row>) {
    let mut lanes: HashMap<String, Lane> = HashMap::new();
    let mut rows: Vec<Row> = Vec::new();
    for index in 0..auction.len() {
        let seat = seat_to_act(dealer, index);
        if !matches!(seat, Seat::North | Seat::South) {
            continue;
        }
        // RHO's live bid is the call we are acting over.  A notrump bid names no
        // suit, so the union's length term has nothing to cut on — counted only
        // under `--include-notrump`, which reports it as their-suit length 0.
        let Some(Call::Bid(their_bid)) = index.checked_sub(1).map(|i| auction[i]) else {
            continue;
        };
        let their_suit = match their_bid.strain.suit() {
            Some(suit) => suit,
            None if grid.include_notrump => Suit::Clubs,
            None => continue,
        };
        if their_bid.level.get() > grid.max_level {
            continue;
        }
        let hand: Hand = deal[seat];
        #[allow(clippy::cast_possible_truncation)]
        let their_len = hand[their_suit].len() as u8;
        // Necessary condition for a trigger, and far cheaper than reading: every
        // `.penalty()` tag in the tree sits on a `Double` rule, and a conversion
        // pass requires partner's double at the same parity — so no trigger can
        // exist without an own-side double earlier in the auction.  Everything
        // that fails it is the unlatched control, counted without a read.
        let could_latch = (0..index).any(|i| i % 2 == index % 2 && auction[i] == Call::Double);
        let read = could_latch.then(|| partnership.infer(relative(vul, seat), &auction[..index]));
        if !read.as_ref().is_some_and(Inferences::pdi_latched) {
            lanes
                .entry("(unlatched control)".to_owned())
                .or_default()
                .add(
                    grid,
                    Class::Control,
                    their_len,
                    grid.axis.of(hand, their_suit),
                    their_bid.level.get() <= 3,
                );
            rows.push(Row {
                lane: "(unlatched control)".to_owned(),
                cut: 0,
                index,
                level: their_bid.level.get(),
                class: Class::Control,
                their_len,
                points: Axis::Points.of(hand, their_suit),
                hcp: Axis::Hcp.of(hand, their_suit),
                suit_hcp: Axis::SuitHcp.of(hand, their_suit),
                conversion: false,
                fresh: false,
                prior_points_min: 0,
                prior_len_min: 0,
                prior_len_max: 13,
            });
            continue;
        }
        let read = read.expect("latched implies read");
        let prior = read.get(Relative::Me);
        let class = match auction[index] {
            Call::Pass => Class::Pass,
            Call::Double => Class::Double,
            // A redouble over their bid is impossible; it lands in `Bid` if it ever does.
            _ => Class::Bid,
        };
        let key = lane_key(auction, index);
        lanes.entry(key.clone()).or_default().add(
            grid,
            class,
            their_len,
            grid.axis.of(hand, their_suit),
            their_bid.level.get() <= 3,
        );
        rows.push(Row {
            cut: key.split_whitespace().count(),
            lane: key,
            index,
            level: their_bid.level.get(),
            class,
            their_len,
            points: Axis::Points.of(hand, their_suit),
            hcp: Axis::Hcp.of(hand, their_suit),
            suit_hcp: Axis::SuitHcp.of(hand, their_suit),
            fresh: index < 2
                || !partnership
                    .infer(relative(vul, seat), &auction[..index - 2])
                    .pdi_latched(),
            prior_points_min: prior.strength.points.min,
            prior_len_min: prior.length(their_suit).min,
            prior_len_max: prior.length(their_suit).max,
            conversion: (2..index).rev().any(|i| {
                i % 2 == index % 2
                    && auction[i] == Call::Pass
                    && auction[i - 1] == Call::Pass
                    && auction[i - 2] == Call::Double
            }),
        });
    }
    (lanes, rows)
}

/// The exclusion table plus the marginals for one class
fn report_class(grid: &Grid, class: Class, agg: &Agg, total: u64) {
    #[allow(clippy::cast_precision_loss)]
    let share = 100.0 * agg.count as f64 / total.max(1) as f64;
    #[allow(clippy::cast_precision_loss)]
    let low = 100.0 * agg.low as f64 / agg.count.max(1) as f64;
    println!(
        "  {} n={:<7} ({share:5.1}% of lane, {low:5.1}% at the three level or below)",
        class.label(),
        agg.count,
    );
    if agg.count == 0 {
        return;
    }
    print!("      excluded by [len ≤ L−1] ∪ [str ≤ S−1]:      ");
    for strength_floor in &grid.strengths {
        print!("  S≥{strength_floor:<2}   ");
    }
    println!();
    for (l, &length_floor) in grid.lengths.iter().enumerate() {
        print!("        L≥{length_floor}                                  ");
        for p in 0..grid.strengths.len() {
            let n = agg.tail[l][p];
            #[allow(clippy::cast_precision_loss)]
            let pct = 100.0 * n as f64 / agg.count as f64;
            print!(" {n:>5}{pct:5.1}%");
        }
        println!();
    }
    if let Some((min, p1, mean, p99, max)) = stats(&agg.lengths) {
        println!("      their-suit length {min}-{max} (p1 {p1}, mean {mean:.2}, p99 {p99})");
    }
    if let Some((min, p1, mean, p99, max)) = stats(&agg.points) {
        println!("      strength          {min}-{max} (p1 {p1}, mean {mean:.2}, p99 {p99})");
    }
}

fn main() {
    let args = Args::parse();
    let grid = Grid {
        lengths: args.lengths.clone(),
        strengths: args.strengths.clone(),
        axis: args.strength,
        max_level: args.max_level,
        include_notrump: args.include_notrump,
    };
    assert!(
        grid.lengths.len() <= LENGTH_SLOTS && grid.strengths.len() <= STRENGTH_SLOTS,
        "at most {LENGTH_SLOTS} length and {STRENGTH_SLOTS} strength floors"
    );
    // The agreements `bba-gen` bid these dumps under, not the bare default: our
    // reading of the opponents' Landy 2♣ and Multi 2♦ moves what the walk sees,
    // and reading a dump under a system it was not bid with drops ~3.7% of the
    // latched passes.
    let agreements = if args.plain_default {
        pons::bidding::agreements::Agreements::default()
    } else {
        vs_bba_agreements(pons::bidding::agreements::Agreements::default())
    };
    let partnership = american(&agreements).bind();

    let mut all: HashMap<String, Lane> = HashMap::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut boards = 0usize;
    for path in &args.dumps {
        let dump = load_dump(path);
        boards += dump.boards.len();
        let vul = dump.vulnerability;
        let folded = dump
            .boards
            .par_iter()
            .map(|board| {
                per_board(
                    &partnership,
                    &grid,
                    &board.deal,
                    board.dealer,
                    &board.table_a,
                    vul,
                )
            })
            .reduce(
                || (HashMap::new(), Vec::new()),
                |mut into: (HashMap<String, Lane>, Vec<Row>), from| {
                    for (key, lane) in from.0 {
                        into.0
                            .entry(key)
                            .and_modify(|existing| existing.merge(&lane))
                            .or_insert(lane);
                    }
                    into.1.extend(from.1);
                    into
                },
            );
        rows.extend(folded.1);
        for (key, lane) in folded.0 {
            all.entry(key)
                .and_modify(|existing| existing.merge(&lane))
                .or_insert(lane);
        }
    }

    if let Some(path) = &args.csv {
        use std::io::Write;
        let mut out = std::io::BufWriter::new(
            std::fs::File::create(path).unwrap_or_else(|e| panic!("create {path}: {e}")),
        );
        writeln!(
            out,
            "lane,cut,index,level,class,their_len,points,hcp,suit_hcp,conversion,\
prior_points_min,prior_len_min,prior_len_max,fresh"
        )
        .expect("csv header");
        for row in &rows {
            writeln!(
                out,
                "\"{}\",{},{},{},{},{},{},{},{},{},{},{},{},{}",
                row.lane,
                row.cut,
                row.index,
                row.level,
                row.class.label().trim(),
                row.their_len,
                row.points,
                row.hcp,
                row.suit_hcp,
                u8::from(row.conversion),
                row.prior_points_min,
                row.prior_len_min,
                row.prior_len_max,
                u8::from(row.fresh),
            )
            .expect("csv row");
        }
    }

    let total: u64 = all.values().map(Lane::count).sum();
    println!(
        "boards {boards}  lanes {}  latched decisions over RHO's live suit bid {total}\n",
        all.len()
    );

    let mut global = Lane::default();
    for lane in all.values() {
        global.merge(lane);
    }
    println!("=== ALL LANES ===");
    for class in Class::ALL {
        let agg = global.by_class.get(&class).cloned().unwrap_or_default();
        report_class(&grid, class, &agg, total);
    }

    let mut ranked: Vec<(&String, &Lane)> = all
        .iter()
        .filter(|(_, lane)| lane.count() >= args.min_count as u64)
        .collect();
    ranked.sort_by(|a, b| b.1.count().cmp(&a.1.count()).then_with(|| a.0.cmp(b.0)));

    // Tag hygiene: a lane whose *passers* live in the trap zone cannot support the
    // claim and gets untagged rather than read unsoundly (docs/pdi.md, Task 3).
    let zone = |lane: &Lane, class: Class| {
        lane.by_class
            .get(&class)
            .and_then(|agg| agg.zone(&grid, args.zone_len, args.zone_str))
            .unwrap_or(0)
    };
    let mut by_leak: Vec<(&String, &Lane)> = all.iter().collect();
    by_leak.sort_by(|a, b| {
        zone(b.1, Class::Pass)
            .cmp(&zone(a.1, Class::Pass))
            .then_with(|| b.1.count().cmp(&a.1.count()))
            .then_with(|| a.0.cmp(b.0))
    });
    println!(
        "\n=== TAG HYGIENE — passers inside the zone [their-suit ≥ {} and strength ≥ {}] ===",
        args.zone_len, args.zone_str
    );
    println!("  in-zone  P / X / bid       lane-total P / X / bid    lane");
    for (key, lane) in by_leak.iter().take(args.top.max(20)) {
        if zone(lane, Class::Pass) == 0 {
            break;
        }
        println!(
            "  {:>7} / {:>3} / {:>3}   {:>18} / {:>4} / {:>4}    [{key}]",
            zone(lane, Class::Pass),
            zone(lane, Class::Double),
            zone(lane, Class::Bid),
            lane.by_class.get(&Class::Pass).map_or(0, |a| a.count),
            lane.by_class.get(&Class::Double).map_or(0, |a| a.count),
            lane.by_class.get(&Class::Bid).map_or(0, |a| a.count),
        );
    }

    for (key, lane) in ranked.iter().take(args.top) {
        println!("\n=== [{key}] ===");
        let lane_total = lane.count();
        for class in Class::ALL {
            let agg = lane.by_class.get(&class).cloned().unwrap_or_default();
            report_class(&grid, class, &agg, lane_total);
        }
    }
}
