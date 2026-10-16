//! How much of an A/B's divergent set each `bba-gen --filter-*` scan would
//! keep — the bidding-only preflight before an enriched result is read
//! globally (docs/measurement.md, coverage; research-method-review §5).
//!
//! An enriched run measures `E[Δ | divergence, filter]`.  That is the
//! global `E[Δ | divergence]` only when the raw-hand scan accepts the boards
//! where the change actually moves the auction.  Bid both arms **unfiltered**
//! on one seed (bidding is cheap; no solver here), and this counts the boards
//! whose `table_a` auction differs, then asks every scan — the strict and the
//! `--wide-1nt` candidate where the flag applies — whether it would have kept
//! each one.  The rejected share is the part of the effect a filtered verdict
//! never sees; the printed examples show what it looks like.
//!
//! The scans are `common::filters`, the same code `bba-gen` draws through, so
//! a board counted here as accepted is one `bba-gen` would keep at that seed.
//!
//! ```text
//! bba-gen --count 4000 --seed 7 --ns-lebensohl-signoff-raise 24 -o on.json
//! bba-gen --count 4000 --seed 7 --ns-lebensohl-signoff-raise 23 -o off.json
//! cargo run --release --features serde --example filter-coverage -- on.json off.json
//! cargo run --release --features serde --example filter-coverage -- on/ off/ \
//!   --filter lebensohl-raise --examples 5
//! ```

use clap::Parser;
use pons::scoring::final_contract;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::filters::Filter;
use common::{Dump, load_dump};

#[cfg(test)]
mod tests;

/// Count the divergent boards of two unfiltered `bba-gen` arms that each
/// `--filter-*` scan would accept or reject
#[derive(Parser)]
struct Args {
    /// One arm: a `bba-gen` dump, or a directory of `shard-*.json`, bid unfiltered
    on: String,
    /// The other arm, on the same seed (same deals, same dealers)
    off: String,
    /// Rejected divergent boards to print per scan
    #[arg(long, default_value_t = 3)]
    examples: usize,
    /// Report only these scans, named as their `bba-gen` flag without
    /// `--filter-` (`1nt`, `landy`, `preempt`, `multi-slam`,
    /// `lebensohl-raise`); repeatable.  Default: every scan.
    #[arg(long = "filter")]
    filters: Vec<String>,
}

/// One row of the report: a scan and its 1NT candidate
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Scan {
    filter: Filter,
    /// `--wide-1nt`; always false for a scan that ignores it
    wide: bool,
}

impl Scan {
    /// Every scan, strict and wide where `--wide-1nt` applies
    fn all() -> Vec<Self> {
        Filter::ALL
            .iter()
            .flat_map(|&filter| {
                let wides: &[bool] = if filter.honours_wide() {
                    &[false, true]
                } else {
                    &[false]
                };
                wides.iter().map(move |&wide| Self { filter, wide })
            })
            .collect()
    }

    fn label(self) -> String {
        let wide = if self.wide { " --wide-1nt" } else { "" };
        format!("{}{wide}", self.filter.flag())
    }
}

/// One scan's split of the board set
#[derive(Debug, Default, PartialEq, Eq)]
struct Row {
    /// Boards of the whole draw the scan accepts: its density
    accepted: usize,
    /// Auction-divergent boards it accepts
    auction_accepted: usize,
    /// Auction-divergent boards it rejects, by index
    auction_rejected: Vec<usize>,
    /// Contract-divergent boards (the set `ab-dump-diff` solves) it accepts
    contract_accepted: usize,
    /// Contract-divergent boards it rejects
    contract_rejected: usize,
}

/// The paired arms' divergence and every scan's split of it
struct Coverage {
    boards: usize,
    /// Indices whose `table_a` auction differs
    auction_divergent: Vec<usize>,
    /// The subset whose `table_a` final contract (and declarer) differs
    contract_divergent: Vec<bool>,
    rows: Vec<(Scan, Row)>,
}

/// Refuse a dump that is itself filtered or isolated: its boards are already
/// the scan's accepted set, so coverage would read 100% by construction.
fn check_unfiltered(dump: &Dump, path: &str) -> anyhow::Result<()> {
    if let Some(flag) = dump
        .gen_args
        .iter()
        .find(|a| a.starts_with("--filter-") || a.starts_with("--isolate-"))
    {
        anyhow::bail!("{path} was generated with {flag}; coverage needs unfiltered arms");
    }
    Ok(())
}

/// Pair the arms board by board and split their divergent set by `scans`.
fn coverage(on: &Dump, off: &Dump, scans: &[Scan]) -> anyhow::Result<Coverage> {
    anyhow::ensure!(
        on.boards.len() == off.boards.len(),
        "the arms hold {} and {} boards; bid them on one seed and count",
        on.boards.len(),
        off.boards.len(),
    );
    anyhow::ensure!(
        on.vulnerability == off.vulnerability,
        "the arms were bid at different vulnerabilities"
    );
    let mut auction_divergent = Vec::new();
    let mut contract_divergent = Vec::new();
    for (i, (a, b)) in on.boards.iter().zip(&off.boards).enumerate() {
        anyhow::ensure!(
            a.deal == b.deal && a.dealer == b.dealer,
            "board {i} deals differ; the arms are not on one seed"
        );
        if a.table_a != b.table_a {
            auction_divergent.push(i);
            contract_divergent
                .push(final_contract(&a.table_a, a.dealer) != final_contract(&b.table_a, b.dealer));
        }
    }
    let rows = scans
        .iter()
        .map(|&scan| {
            let accepts = |i: usize| scan.filter.accepts(&on.boards[i].deal, scan.wide);
            let mut row = Row {
                accepted: (0..on.boards.len()).filter(|&i| accepts(i)).count(),
                ..Row::default()
            };
            for (&i, &contract) in auction_divergent.iter().zip(&contract_divergent) {
                match (accepts(i), contract) {
                    (true, true) => {
                        row.auction_accepted += 1;
                        row.contract_accepted += 1;
                    }
                    (true, false) => row.auction_accepted += 1,
                    (false, true) => {
                        row.auction_rejected.push(i);
                        row.contract_rejected += 1;
                    }
                    (false, false) => row.auction_rejected.push(i),
                }
            }
            (scan, row)
        })
        .collect();
    Ok(Coverage {
        boards: on.boards.len(),
        auction_divergent,
        contract_divergent,
        rows,
    })
}

#[allow(clippy::cast_precision_loss)]
fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        "—".into()
    } else {
        format!("{:.1}%", 100.0 * part as f64 / whole as f64)
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let scans: Vec<Scan> = Scan::all()
        .into_iter()
        .filter(|scan| {
            args.filters.is_empty()
                || args
                    .filters
                    .iter()
                    .any(|name| scan.filter.flag() == format!("--filter-{name}"))
        })
        .collect();
    anyhow::ensure!(
        !scans.is_empty(),
        "--filter names no scan (1nt, landy, preempt, multi-slam, lebensohl-raise)"
    );
    let (on, off) = (load_dump(&args.on), load_dump(&args.off));
    check_unfiltered(&on, &args.on)?;
    check_unfiltered(&off, &args.off)?;
    let report = coverage(&on, &off, &scans)?;

    let contracts = report.contract_divergent.iter().filter(|&&c| c).count();
    println!(
        "filter-coverage: {} paired boards ({}), {} auction-divergent at table_a, {} of them contract-divergent",
        report.boards,
        on.vulnerability,
        report.auction_divergent.len(),
        contracts,
    );
    println!("  on:  {}\n  off: {}", args.on, args.off);
    println!(
        "{:<38} {:>8}  {:>8} {:>8} {:>9}  {:>8} {:>8} {:>9}",
        "scan", "density", "auc acc", "auc rej", "auc rej%", "con acc", "con rej", "con rej%"
    );
    for (scan, row) in &report.rows {
        println!(
            "{:<38} {:>8}  {:>8} {:>8} {:>9}  {:>8} {:>8} {:>9}",
            scan.label(),
            percent(row.accepted, report.boards),
            row.auction_accepted,
            row.auction_rejected.len(),
            percent(row.auction_rejected.len(), report.auction_divergent.len()),
            row.contract_accepted,
            row.contract_rejected,
            percent(row.contract_rejected, contracts),
        );
    }
    if args.examples == 0 {
        return Ok(());
    }
    for (scan, row) in &report.rows {
        if row.auction_rejected.is_empty() {
            continue;
        }
        println!("--- rejected by {} ---", scan.label());
        for &i in row.auction_rejected.iter().take(args.examples) {
            let (a, b) = (&on.boards[i], &off.boards[i]);
            let contract =
                |board: &common::Board| match final_contract(&board.table_a, board.dealer) {
                    Some((contract, declarer)) => format!("{contract} by {declarer}"),
                    None => "passed out".into(),
                };
            println!(
                "[board {i}, dealer {}] {}\n  on:  {}  ({})\n  off: {}  ({})",
                a.dealer,
                a.deal,
                a.table_a,
                contract(a),
                b.table_a,
                contract(b),
            );
        }
    }
    Ok(())
}
