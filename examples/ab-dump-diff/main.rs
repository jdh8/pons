//! Paired delta of two aligned `bba-gen` dumps — the feature value on a rare auction.
//!
//! When an A/B feature fires on only a sliver of boards (e.g. the doubler's runout
//! from `(1NT) X (XX) - -`), scoring each whole dump against BBA and subtracting
//! wastes ~99% of the double-dummy budget on boards the feature never touches. But
//! if the two dumps were generated with the **same seed** (so they share every deal
//! and the BBA reference table), the only boards that differ are the ones the
//! feature fired on, and the shared BBA table cancels in the subtraction. So the
//! per-board delta is just our own score with the feature minus without it:
//! `ns_score(our `on` contract) − ns_score(our `off` contract)`.
//!
//! This reads the two dumps, pairs each board's `table_a` (our pair, North/South)
//! contracts as `(on, off)`, and hands them to [`score_boards`] — which solves only
//! the boards where the two contracts differ. Positive IMPs ⇒ the `on` feature beat
//! the `off` baseline on the boards it touched.
//!
//! Both brackets come off **one** solve: the divergent set is solved once and
//! each scorer only re-prices the resulting tables, so `--score both` costs one
//! DDS fan-out and one parse of the dump where two single-scorer runs paid for
//! each twice.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-diff -- on.json off.json
//! cargo run --release --features serde --example ab-dump-diff -- on/ off/ \
//!   --score both --out-plain diff.plain.txt --out-pd diff.pd.txt
//! ```

use clap::Parser;
use contract_bridge::AbsoluteVulnerability;
use pons::scoring::{final_contract, ns_score_contract, ns_score_pd};
use std::io::Write;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Dump, Scored, mean_with_ci, score_solved, solve_divergent};

#[derive(Parser)]
struct Args {
    /// Dump bid with the feature ON (its `table_a` is the measured contract).
    /// A directory folds its `shard-*.json` into one solve.
    on: String,
    /// Dump bid with the feature OFF, same seed/deals (the baseline contract).
    /// A directory folds its `shard-*.json` into one solve.
    off: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// Show this many of the biggest swings (each way)
    #[arg(long, default_value_t = 8)]
    show: usize,
    /// Scorer: `plain` = honest double-dummy (was the only mode); `pd` =
    /// perfect-defense doubling, which prices a failing contract as doubled;
    /// `both` = each of those, read off **one** solve and written to
    /// `--out-plain` / `--out-pd`.
    /// For a *competitive* feature, a `plain` win that `pd` erases is the
    /// light-sacrifice / doubling artifact (see `reference_pd-vs-plain-dd-bracket`).
    #[arg(long, default_value = "plain")]
    score: String,
    /// Where `--score both` writes the plain-DD report
    ///
    /// Full paths rather than a stem, because the runners do not agree on a
    /// separator — most spell `diff.…{vul}.plain.txt`, `ab-reading-knobs.sh`
    /// spells `diff-…-{vul}-plain.txt`. Both are required by `both`, and there
    /// is deliberately no way to put two scorers in one file:
    /// `scripts/ab-aggregate.sh` matches the `Delta` line without reading which
    /// bracket produced it, so it would sum plain and PD together.
    #[arg(long, required_if_eq("score", "both"))]
    out_plain: Option<String>,
    /// Where `--score both` writes the perfect-defense report
    #[arg(long, required_if_eq("score", "both"))]
    out_pd: Option<String>,
}

/// One scorer's report — byte-for-byte what the single-scorer run has always
/// printed
///
/// The text is an interface, not a display detail: `scripts/ab-aggregate.sh`
/// parses it with awk, matching ` fired (` and `^Delta` and counting fields
/// around the `boards,`, `fired` and `IMPs,` tokens.
fn report(
    out: &mut impl Write,
    on: &Dump,
    off: &Dump,
    vul: AbsoluteVulnerability,
    scored: &Scored,
    show: usize,
) -> std::io::Result<()> {
    let (mean, ci) = mean_with_ci(&scored.board_imps);
    let n = on.boards.len();
    let d = scored.divergent.len();
    writeln!(
        out,
        "ON {} vs OFF {} ({} boards, vul {vul}): {} fired ({:.2}%)",
        on.our_label,
        off.our_label,
        n,
        d,
        100.0 * d as f64 / n.max(1) as f64,
    )?;
    writeln!(
        out,
        "Delta (run − sit): {:+} IMPs, {:+.4} IMPs/board [95% CI ±{:.4}], {:+.3} IMPs/fired",
        scored.total_imps,
        mean,
        ci,
        scored.total_imps as f64 / d.max(1) as f64,
    )?;

    let mut swings = scored.swings.clone();
    swings.sort_by_key(|&(_, _, imp)| imp);
    let show = show.min(swings.len());
    if show > 0 {
        writeln!(out, "--- {show} worst (for the feature) ---")?;
        for &(i, _, imp) in swings.iter().take(show) {
            let b = &on.boards[i];
            writeln!(
                out,
                "[{imp:+} IMP] {}\n  on:  {}\n  off: {}",
                b.deal, b.table_a, off.boards[i].table_a
            )?;
        }
    }
    Ok(())
}

/// [`report`] into its own file, for `--score both`
fn write_report(
    path: &str,
    on: &Dump,
    off: &Dump,
    vul: AbsoluteVulnerability,
    scored: &Scored,
    show: usize,
) -> anyhow::Result<()> {
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    report(&mut file, on, off, vul, scored, show)?;
    file.flush()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let on = common::load_dump(&args.on);
    let off = common::load_dump(&args.off);
    assert_eq!(on.boards.len(), off.boards.len(), "dumps must be aligned");
    let vul = args.vulnerability.unwrap_or(on.vulnerability);

    // Pair our (table_a) contract with the feature on vs off; the deals must match.
    let mut deals = Vec::with_capacity(on.boards.len());
    let contracts: Vec<_> = on
        .boards
        .iter()
        .zip(&off.boards)
        .map(|(a, b)| {
            assert_eq!(a.deal, b.deal, "dumps not seed-aligned");
            deals.push(a.deal);
            (
                final_contract(&a.table_a, a.dealer),
                final_contract(&b.table_a, b.dealer),
            )
        })
        .collect();

    // One solve serves every scorer — a scorer only re-prices the solved tables,
    // so `--score both` costs one DDS fan-out and one parse of the dump, where
    // two single-scorer runs paid for each twice.
    let (divergent, tables) = solve_divergent(&contracts, &deals);

    match args.score.as_str() {
        "plain" => {
            let scored = score_solved(&contracts, divergent, tables, vul, ns_score_contract);
            report(
                &mut std::io::stdout().lock(),
                &on,
                &off,
                vul,
                &scored,
                args.show,
            )?;
        }
        "pd" => {
            let scored = score_solved(&contracts, divergent, tables, vul, ns_score_pd);
            report(
                &mut std::io::stdout().lock(),
                &on,
                &off,
                vul,
                &scored,
                args.show,
            )?;
        }
        "both" => {
            let (plain_path, pd_path) = args
                .out_plain
                .as_deref()
                .zip(args.out_pd.as_deref())
                .expect("clap requires --out-plain and --out-pd when --score is both");
            let plain = score_solved(
                &contracts,
                divergent.clone(),
                tables.clone(),
                vul,
                ns_score_contract,
            );
            write_report(plain_path, &on, &off, vul, &plain, args.show)?;
            let pd = score_solved(&contracts, divergent, tables, vul, ns_score_pd);
            write_report(pd_path, &on, &off, vul, &pd, args.show)?;
        }
        other => anyhow::bail!("--score must be plain|pd|both, got {other:?}"),
    }
    Ok(())
}
