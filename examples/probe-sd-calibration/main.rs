//! Calibrate the three play brackets per contract cell against Pavlicek.
//!
//! A *cell* is `(level, strain class)` — 4M, 5m, 3NT, 6X — not level alone:
//! Pavlicek's after-lead shift spans −0.075 (4m) to −0.217 (4M) within level
//! 4, and 6NT (−0.396) trails DD play by more than any suit slam.
//!
//! Plain DD, sd-lead ([`single_dummy_lead_tricks`]), and the sd-declarer
//! playout ([`single_dummy_playout`]) model progressively more of real play.
//! Pavlicek's actual-vs-DD study (rpbridge.net/8j45.htm) gives the target
//! shape: at 1NT the table *beats* DD by ≈+7pp of make-rate (the blind lead),
//! the gap tapering to zero as the level rises; at slam level the table makes
//! *fewer* contracts than DD promises (declarer misguesses).  This probe bids
//! random boards out in self-play, prices every reached contract under all
//! three brackets, and reports make-rate and mean declarer tricks per level —
//! the sd-lead column should reproduce the fading lead gap, and the
//! sd-declarer column the growing misguess haircut.
//!
//! Slam-level rows are rare in self-play, so `--min-level-count` keeps dealing
//! until every level from 1 to `--target-level` has enough contracts (bounded
//! by `--max-batches`).  The playout is sequential per board; expect minutes,
//! not seconds.
//!
//! ```text
//! cargo run --release --example probe-sd-calibration -- --count 20000
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Contract, FullDeal, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::context::relative;
use pons::scoring::final_contract;
use pons::single_dummy::{LeadQuestion, single_dummy_leads, single_dummy_playout};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, seat_to_act, seeded_deals};

#[derive(Parser)]
#[command(about = "Per-level make-rates under plain DD, sd-lead, and sd-declarer")]
struct Args {
    /// Boards to bid per batch (self-play, dealer rotates)
    #[arg(short, long, default_value_t = 20_000)]
    count: usize,
    /// Deal seed base (board i seeded base+i); fresh per experiment
    #[arg(long, default_value_t = 20_260_716)]
    seed: u64,
    /// Vulnerability the boards are bid and scored at
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
    /// Worlds per blind lead and per declarer decision
    #[arg(long, default_value_t = 16)]
    sd_worlds: usize,
    /// Cap of playouts per (level, strain-class) cell — 4M, 5m, 6NT… (the DD
    /// and sd-lead columns still see every contract; only the expensive
    /// playout is subsampled)
    #[arg(long, default_value_t = 500)]
    per_level: usize,
    /// Keep dealing batches until every level up to --target-level has this
    /// many playouts (0 = a single batch)
    #[arg(long, default_value_t = 200)]
    min_level_count: usize,
    /// Highest level the top-up loop chases (7 = grands; they are so rare
    /// that chasing them can exhaust --max-batches)
    #[arg(long, default_value_t = 6)]
    target_level: u8,
    /// Upper bound on dealt batches while topping up rare levels
    #[arg(long, default_value_t = 50)]
    max_batches: usize,
}

/// One reached contract: its level and strain class, whether the bid was made
/// under each bracket, and each bracket's declarer tricks.
struct Row {
    level: u8,
    class: usize,
    need: u8,
    dd: u8,
    sd_lead: u8,
    sd_line: Option<u8>,
}

/// The strain class λ is bucketed in, alongside level: minor, major, notrump.
/// Level alone is not enough — within level 4 the after-lead shift spans
/// −0.075 (4m) to −0.217 (4M), a factor of three, and 6NT (−0.396) is the
/// most DD-optimistic cell on the board.
const CLASSES: [&str; 3] = ["m", "M", "NT"];

fn class_of(strain: Strain) -> usize {
    match strain {
        Strain::Notrump => 2,
        s if s.is_minor() => 0,
        _ => 1,
    }
}

/// Pavlicek's **after-the-opening-lead** table (rpbridge.net/8j45.htm,
/// 77,406 expert-vs-expert vugraph contracts, 1996–2014), aggregated from his
/// per-contract table into `[level][class]` cells:
/// `(actual make %, double-dummy-after-the-actual-lead make %, n)`.
///
/// This is the exact bias position of the sd-lead endpoint — realistic lead,
/// then clairvoyant play — so `logit(actual) − logit(DD after lead)` is the
/// declarer-fallibility quantum the blend must apply on top of it.  Level 0
/// is unused padding.
///
/// The n-weighted pooled row of each level reproduces Pavlicek's own per-level
/// row exactly (67.97/68.76, 65.59/67.70, 63.85/66.91, 66.66/71.02,
/// 49.84/52.87, 66.70/73.67, 64.84/72.14), which is the transcription check.
/// Two cells are too thin to fit on: 5NT (n = 17) and 7NT (n = 95).
const PAVLICEK_AFTER_LEAD: [[(f64, f64, u32); 3]; 8] = [
    [(f64::NAN, f64::NAN, 0); 3],
    [
        (54.46, 55.36, 112),
        (70.48, 71.05, 708),
        (67.92, 68.74, 4_753),
    ],
    [
        (63.83, 64.38, 2_159),
        (67.68, 69.99, 7_837),
        (56.56, 60.00, 1_395),
    ],
    [
        (58.40, 61.20, 4_730),
        (52.39, 54.39, 5_438),
        (69.76, 73.28, 14_923),
    ],
    [
        (48.46, 50.35, 1_428),
        (67.59, 72.15, 21_792),
        (84.10, 85.93, 327),
    ],
    [
        (49.61, 53.21, 4_106),
        (50.25, 51.99, 1_964),
        (58.82, 70.59, 17),
    ],
    [
        (63.70, 70.68, 1_835),
        (67.63, 74.60, 2_669),
        (73.48, 80.45, 445),
    ],
    [(53.36, 63.60, 283), (70.00, 75.90, 390), (77.89, 82.11, 95)],
];

/// Make-rates of one cell: every contract in it (`dd`, `lead`) and the
/// playout subsample (`*_sub`), in percent.
struct Stats {
    n: usize,
    dd: f64,
    lead: f64,
    n_line: usize,
    dd_sub: f64,
    lead_sub: f64,
    line_sub: f64,
}

impl Stats {
    fn of(at: &[&Row]) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let made = |set: &[&Row], tricks: fn(&Row) -> u8| {
            100.0 * set.iter().filter(|row| tricks(row) >= row.need).count() as f64
                / set.len().max(1) as f64
        };
        let lined: Vec<&Row> = at
            .iter()
            .filter(|row| row.sd_line.is_some())
            .copied()
            .collect();
        Self {
            n: at.len(),
            dd: made(at, |row| row.dd),
            lead: made(at, |row| row.sd_lead),
            n_line: lined.len(),
            dd_sub: made(&lined, |row| row.dd),
            lead_sub: made(&lined, |row| row.sd_lead),
            line_sub: made(&lined, |row| row.sd_line.expect("filtered Some")),
        }
    }

    /// Pool cells by their share of *contracts*, not of playouts — the
    /// playout cap is per cell, so the subsample is not population-weighted.
    #[allow(clippy::cast_precision_loss)]
    fn pool<'a>(cells: impl Iterator<Item = &'a Self> + Clone) -> Self {
        let n: usize = cells.clone().map(|cell| cell.n).sum();
        let mean = |pick: fn(&Self) -> f64| {
            cells
                .clone()
                .map(|cell| pick(cell) * cell.n as f64)
                .sum::<f64>()
                / n.max(1) as f64
        };
        Self {
            n,
            dd: mean(|cell| cell.dd),
            lead: mean(|cell| cell.lead),
            n_line: cells.clone().map(|cell| cell.n_line).sum(),
            dd_sub: mean(|cell| cell.dd_sub),
            lead_sub: mean(|cell| cell.lead_sub),
            line_sub: mean(|cell| cell.line_sub),
        }
    }
}

/// `ln(p / (1 − p))` with `p` in percent
fn logit(percent: f64) -> f64 {
    let p = percent / 100.0;
    (p / (1.0 - p)).ln()
}

/// `σ(x)` back to percent
fn sigmoid_pct(x: f64) -> f64 {
    100.0 / (1.0 + (-x).exp())
}

/// Pavlicek's cell for a level and strain class, `None` = the level pooled
/// over strains (n-weighted, reproducing his own per-level row).
fn pavlicek(level: u8, class: Option<usize>) -> (f64, f64, u32) {
    let cells = PAVLICEK_AFTER_LEAD[usize::from(level)];
    let Some(class) = class else {
        let n = cells.iter().map(|&(.., n)| n).sum::<u32>();
        let mean = |pick: fn(&(f64, f64, u32)) -> f64| {
            cells
                .iter()
                .map(|cell| pick(cell) * f64::from(cell.2))
                .sum::<f64>()
                / f64::from(n)
        };
        return (mean(|cell| cell.0), mean(|cell| cell.1), n);
    };
    cells[class]
}

fn main() {
    let args = Args::parse();
    let partnership = american(&pons::bidding::agreements::Agreements::default()).bind();
    let mut rows: Vec<Row> = Vec::new();
    let mut playouts_at = [[0usize; 3]; 8];

    for batch in 0..args.max_batches.max(1) {
        // Deal and bid this batch (bidding parallelizes; the solver never
        // leaves the main thread).
        let deals = seeded_deals(args.seed + (batch * args.count) as u64, args.count);
        let boards: Vec<(Seat, FullDeal, Auction)> = deals
            .into_par_iter()
            .enumerate()
            .map(|(i, deal)| {
                let dealer = Seat::ALL[i % 4];
                let auction = bid_out(
                    &partnership,
                    &partnership,
                    true,
                    dealer,
                    args.vulnerability,
                    &deal,
                );
                (dealer, deal, auction)
            })
            .collect();

        // Plain DD for every reached contract, batched in one fan-out.
        let reached: Vec<(Seat, FullDeal, Auction, Contract, Seat)> = boards
            .into_iter()
            .filter_map(|(dealer, deal, auction)| {
                let (contract, declarer) = final_contract(&auction, dealer)?;
                Some((dealer, deal, auction, contract, declarer))
            })
            .collect();
        let solve: Vec<FullDeal> = reached.iter().map(|&(_, deal, ..)| deal).collect();
        let tables = Solver::lock(None).solve_deals(&solve, NonEmptyStrainFlags::ALL);

        // All blind leads in one pooled solve (straggler-bound otherwise),
        // then the expensive playouts only on the per-level subsample.
        let mut rng = StdRng::seed_from_u64(args.seed ^ 0x5dca_11b8 ^ batch as u64);
        let view = |auction: &Auction, dealer: Seat, seat: Seat| {
            let cut = (auction.len().saturating_sub(3)..=auction.len())
                .find(|&len| seat_to_act(dealer, len) == seat)
                .expect("one of four consecutive lengths reaches every seat");
            partnership.infer(relative(args.vulnerability, seat), &auction[..cut])
        };
        let questions: Vec<LeadQuestion> = reached
            .iter()
            .map(
                |&(dealer, deal, ref auction, contract, declarer)| LeadQuestion {
                    deal,
                    strain: contract.bid.strain,
                    declarer,
                    inferences: view(auction, dealer, declarer.lho()),
                },
            )
            .collect();
        let mut leads = Vec::with_capacity(questions.len());
        for chunk in questions.chunks(4096) {
            leads.extend(single_dummy_leads(chunk, &mut rng, args.sd_worlds));
        }

        for (((dealer, deal, auction, contract, declarer), table), (lead, lead_tricks)) in
            reached.into_iter().zip(tables).zip(leads)
        {
            let level = contract.bid.level.get();
            let class = class_of(contract.bid.strain);
            let sd_line = (playouts_at[usize::from(level)][class] < args.per_level).then(|| {
                playouts_at[usize::from(level)][class] += 1;
                u8::from(single_dummy_playout(
                    &deal,
                    contract.bid.strain,
                    declarer,
                    lead,
                    &view(&auction, dealer, declarer),
                    &mut rng,
                    args.sd_worlds,
                ))
            });
            rows.push(Row {
                level,
                class,
                need: 6 + level,
                dd: u8::from(table[contract.bid.strain].get(declarer)),
                sd_lead: u8::from(lead_tricks),
                sd_line,
            });
        }

        // The top-up loop still chases *level* totals: the rare cells (5NT,
        // any 7) are so thin in self-play that requiring them per cell would
        // exhaust --max-batches without ever filling.
        let at_level = |level: u8| playouts_at[usize::from(level)].iter().sum::<usize>();
        let filled = (1..=args.target_level).all(|level| at_level(level) >= args.min_level_count);
        if filled || args.min_level_count == 0 {
            break;
        }
        eprintln!(
            "batch {batch}: playouts per level {:?} (cells {:?}), topping up…",
            (1..=args.target_level).map(at_level).collect::<Vec<_>>(),
            &playouts_at[1..=usize::from(args.target_level)]
        );
    }

    println!(
        "=== sd calibration: {} contracts, vul {}, {} worlds, seed {} ===",
        rows.len(),
        args.vulnerability,
        args.sd_worlds,
        args.seed,
    );
    println!(
        "{:>6} {:>6} | {:>8} {:>8} {:>7} | {:>6} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "cell",
        "n",
        "DD mk%",
        "lead mk%",
        "Δlead",
        "n line",
        "DD mk%*",
        "lead%*",
        "line mk%",
        "Δguess*",
        "Δtable*",
    );
    println!(
        "(Δlead = blind lead − DD, all contracts.  Starred columns are the playout subsample: \
         Δguess = playout − sd-lead, the pure misguess haircut; Δtable = playout − DD, the \
         full table-proxy shift.)"
    );
    let mut fit: Vec<(String, u8, Option<usize>, usize, f64, f64)> = Vec::new();
    for level in 1..=7u8 {
        let mut cells: Vec<(usize, Stats)> = Vec::new();
        for class in 0..3 {
            let at: Vec<&Row> = rows
                .iter()
                .filter(|row| row.level == level && row.class == class)
                .collect();
            if at.is_empty() {
                continue;
            }
            cells.push((class, Stats::of(&at)));
        }
        // Each class, then the level pooled (`4·` — the bucket the shipped
        // per-level λ was fitted in, kept as the continuity check).  The
        // pooled row weights each class by its share of the level's
        // *contracts*: the playout cap is per cell, so pooling the starred
        // subsample raw would count 4m (12% of level 4) equally with 4M (87%)
        // and quietly report a λ for a population that never occurs.
        let pooled = Stats::pool(cells.iter().map(|(_, stats)| stats));
        for (label, class, stats) in cells
            .iter()
            .map(|&(class, ref stats)| (format!("{level}{}", CLASSES[class]), Some(class), stats))
            .chain(std::iter::once((format!("{level}·"), None, &pooled)))
        {
            println!(
                "{label:>6} {:>6} | {:>7.1}% {:>7.1}% {:>+6.1}pp | {:>6} {:>7.1}% {:>7.1}% {:>7.1}% {:>+6.1}pp {:>+6.1}pp",
                stats.n,
                stats.dd,
                stats.lead,
                stats.lead - stats.dd,
                stats.n_line,
                stats.dd_sub,
                stats.lead_sub,
                stats.line_sub,
                stats.line_sub - stats.lead_sub,
                stats.line_sub - stats.dd_sub,
            );
            fit.push((
                label,
                level,
                class,
                stats.n_line,
                stats.lead_sub,
                stats.line_sub,
            ));
        }
    }

    // The λ fit: per level, the weight of the playout endpoint in the
    // sd-blend (`common::SD_BLEND_LAMBDA`).  Both endpoints share the blind
    // lead, so the blend must apply exactly the *after-lead* declarer
    // quantum: shift the lead endpoint's make-logit by Pavlicek's
    // `logit(actual) − logit(DD after lead)`, then solve the (linear-in-
    // probability) mixture for λ.  A λ outside [0, 1] means the playout's
    // haircut is on the wrong side of the target at that level; it is
    // clamped, and the residual shows in the `blend%` column.
    println!(
        "\n-- λ fit vs Pavlicek after-lead (playout subsample; paste into common::SD_BLEND_LAMBDA) --",
    );
    println!(
        "{:>6} {:>6} {:>7} | {:>8} {:>8} {:>7} {:>7} | {:>8} {:>8} {:>8} | {:>6}",
        "cell",
        "n",
        "pav n",
        "lead mk%",
        "pav DDaL",
        "align",
        "shift",
        "target%",
        "blend%",
        "pav act",
        "λ",
    );
    println!(
        "(`align` = lead mk% − pav DDaL, the TRANSFER TEST: both are DD play after a real \
         lead, so a cell where they disagree holds different HANDS in his corpus and in \
         ours, and his shift must NOT be imported into it — inherit the pooled λ instead. \
         `blend%` vs `pav act` is the population check.  shift in log-odds.  `pav n` is \
         Pavlicek's own cell count: under ~500 the target is noise.  `·` rows are the \
         level pooled.)"
    );
    for (label, level, class, n, lead, line) in fit {
        let (actual, dd_after_lead, pav_n) = pavlicek(level, class);
        let shift = logit(actual) - logit(dd_after_lead);
        let align = lead - dd_after_lead;
        if !(0.1..=99.9).contains(&lead) || (lead - line).abs() < f64::EPSILON {
            println!(
                "{label:>6} {n:>6} {pav_n:>7} | {lead:>7.1}% {dd_after_lead:>7.2}% \
                 {align:>+6.1}pp {shift:>+7.3} | {:>8} {:>8} {actual:>7.2}% | \
                 degenerate (endpoints equal or saturated)",
                "—", "—",
            );
            continue;
        }
        let target = sigmoid_pct(logit(lead) + shift);
        let lambda = ((lead - target) / (lead - line)).clamp(0.0, 1.0);
        let blend = lead + lambda * (line - lead);
        println!(
            "{label:>6} {n:>6} {pav_n:>7} | {lead:>7.1}% {dd_after_lead:>7.2}% {align:>+6.1}pp \
             {shift:>+7.3} | {target:>7.2}% {blend:>7.2}% {actual:>7.2}% | {lambda:>6.3}",
        );
    }
}
