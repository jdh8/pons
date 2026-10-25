//! Paired delta of two aligned `bba-gen` dumps — the feature value on a rare auction.
//!
//! When an A/B feature fires on only a sliver of boards (e.g. the doubler's runout
//! from `(1NT) X (XX) - -`), scoring each whole dump against BBA and subtracting
//! wastes ~99% of the double-dummy budget on boards the feature never touches. If
//! the two dumps were generated with the **same seed** (so they share every deal),
//! only the boards where a contract differs can swing, and only those are solved.
//!
//! Two metrics, chosen by `--metric`, answer different questions:
//!
//! - **`direct`** (the default) — the **direct A/B** `IMP(S_on − S_off)`, from
//!   our `table_a` (North/South) contract with the feature on vs off: the two
//!   arms IMPed head to head, one table.  The **deployment** metric, and the only
//!   one that prints a `Verdict`.
//! - **`reference`** — the **reference-relative IMP gain**
//!   `IMP(S_on − R_on) − IMP(S_off − R_off)`: each arm's `table_a` IMPed against
//!   its reference, the same board's `table_b` contract (the reference pair NS,
//!   our pair EW), *then* differenced.  That is each arm's per-board anchor score
//!   (`bba-decompose`), so its mean is the anchor-window move: the **closing the
//!   gap** metric, and a tracking figure, never a ship verdict.
//!
//! A reference shared by both arms cancels in the raw points `S_on − S_off`, but
//! **not across the IMP table**, which is nonlinear: `S_on = 420`, `S_off = 400`,
//! `R = 0` is +1 IMP direct and 0 gained against the reference.  Neither metric is
//! the other's estimate.  The reference also moves whenever the feature changes our
//! East/West bidding at table B — `reference` reports how often — so its gain
//! covers both tables of the board where `direct` covers one, and runs about twice
//! the direct figure for a change that acts at both tables.
//!
//! Both brackets come off **one** solve: the divergent set is solved once and
//! each scorer only re-prices the resulting tables, so `--score both` costs one
//! DDS fan-out and one parse of the dump where two single-scorer runs paid for
//! each twice.  Under `reference` both scorers price all four contracts of a
//! board on that one table.
//!
//! `--score pdp` prices the direct A/B with **PD-P**
//! ([`ns_score_bid_honoring`][pons::scoring::ns_score_bid_honoring]): perfect
//! defense that erases no `X`/`XX` both arms saw before they split, for a lane
//! whose trigger is a double.  Direct metric only — the shared prefix is the
//! two arms' table-A auctions.
//!
//! `--jsonl` writes one record per contract-divergent board — the deal, both
//! arms' table-A auctions and each scorer's IMP swing — for a per-board
//! forensic (first divergent call, lane) off the same solve.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-diff -- on.json off.json
//! cargo run --release --features serde --example ab-dump-diff -- on/ off/ \
//!   --score both --out-plain diff.plain.txt --out-pd diff.pd.txt
//! cargo run --release --features serde --example ab-dump-diff -- on/ off/ \
//!   --metric reference --score both --out-plain gain.plain.txt --out-pd gain.pd.txt
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::AbsoluteVulnerability;
use contract_bridge::Penalty;
use pons::scoring::{
    final_contract, ns_score_bid, ns_score_bid_honoring, ns_score_contract, shared_penalty,
};
use std::io::Write;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{
    ArmVsReference, Dump, Reached, Scored, dump_density, mean_with_ci, score_reference_solved,
    score_solved, solve_divergent, verdict_line,
};

/// Which paired quantity a report carries — see the module doc
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Metric {
    /// Direct A/B: `IMP(S_on − S_off)` at table A
    Direct,
    /// Reference-relative IMP gain: `IMP(S_on − R_on) − IMP(S_off − R_off)`,
    /// each arm against its own table B
    Reference,
}

#[derive(Parser)]
struct Args {
    /// Dump bid with the feature ON (its `table_a` is the measured contract).
    /// A directory folds its `shard-*.json` into one solve.
    on: String,
    /// Dump bid with the feature OFF, same seed/deals (the baseline contract).
    /// A directory folds its `shard-*.json` into one solve.
    off: String,
    /// `direct` = the direct A/B, the deployment metric (was the only mode);
    /// `reference` = the reference-relative IMP gain, the anchor-gap metric.
    /// The two are never interchangeable: see the module doc.
    #[arg(long, value_enum, default_value = "direct")]
    metric: Metric,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// Show this many of the biggest swings (each way)
    #[arg(long, default_value_t = 8)]
    show: usize,
    /// Scorer: `plain` = honest double-dummy (was the only mode); `pd` =
    /// perfect-defense doubling, which prices a failing contract as doubled;
    /// `both` = each of those, read off **one** solve and written to
    /// `--out-plain` / `--out-pd`; `pdp` = prefix-honoring PD (direct metric
    /// only, see the module doc).
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
    /// Share of random deals the dumps' `--filter-*` accepted, for the
    /// verdict's δ / density margin; an enriched dump without it withholds
    /// the verdict.  Must be in `(0, 1]`
    #[arg(long, value_parser = common::parse_density)]
    density: Option<f64>,
    /// Also write one JSON record per contract-divergent board here: `index`,
    /// `deal`, `dealer`, `on` and `off` (table-A auctions), and one IMP swing
    /// per scorer (`plain`, `pd`, `pdp`) of the selected metric
    #[arg(long)]
    jsonl: Option<String>,
}

/// Each board's contracts, in the shape its metric scores
enum Paired {
    /// `(on, off)` at table A
    Direct(Vec<(Reached, Reached)>),
    /// `((on, on's table B), (off, off's table B))`
    Reference(Vec<(ArmVsReference, ArmVsReference)>),
}

impl Paired {
    /// Solve the divergent set once and price it under each `scorer`, in order
    fn score(
        &self,
        deals: &[contract_bridge::FullDeal],
        vul: AbsoluteVulnerability,
        scorers: &[fn(Reached, &ddss::TrickCountTable, AbsoluteVulnerability) -> i64],
    ) -> Vec<Scored> {
        match self {
            Self::Direct(contracts) => {
                let (divergent, tables) = solve_divergent(contracts, deals);
                scorers
                    .iter()
                    .map(|&scorer| {
                        score_solved(contracts, divergent.clone(), tables.clone(), vul, scorer)
                    })
                    .collect()
            }
            Self::Reference(contracts) => {
                let (divergent, tables) = solve_divergent(contracts, deals);
                scorers
                    .iter()
                    .map(|&scorer| {
                        score_reference_solved(
                            contracts,
                            divergent.clone(),
                            tables.clone(),
                            vul,
                            scorer,
                        )
                    })
                    .collect()
            }
        }
    }

    /// Boards whose reference contract differs between the arms, under `reference`
    fn moved_references(&self) -> usize {
        match self {
            Self::Direct(_) => 0,
            Self::Reference(contracts) => {
                contracts.iter().filter(|(on, off)| on.1 != off.1).count()
            }
        }
    }
}

/// What every report of one run shares: the arms, the pricing and the display
struct Experiment<'a> {
    on: &'a Dump,
    off: &'a Dump,
    vul: AbsoluteVulnerability,
    metric: Metric,
    /// [`Paired::moved_references`]
    moved: usize,
    show: usize,
    density: Option<f64>,
}

/// One scorer's report
///
/// The text is an interface, not a display detail: `scripts/ab-aggregate.sh`
/// and `scripts/ab-score.awk` parse a direct report with awk, matching
/// ` fired (` and `^Delta` and counting fields around the `boards,`, `fired`
/// and `IMPs,` tokens.  The `Verdict` line (2026-10-14) matches neither, so
/// per-shard sums are unchanged; a shard's verdict is not the pooled one.
///
/// A reference report matches neither on purpose — its lines read `changed (`
/// and `^Gain` — and both aggregators refuse a file with a `Gain` line, so a
/// gain is never summed into a direct A/B.  It prints no `Verdict`: ship
/// decisions read the direct A/B.
fn report(
    out: &mut impl Write,
    experiment: &Experiment,
    scored: &Scored,
    scorer: &str,
) -> std::io::Result<()> {
    let &Experiment {
        on,
        off,
        vul,
        metric,
        moved,
        show,
        density,
    } = experiment;
    let (mean, ci) = mean_with_ci(&scored.board_imps);
    let n = on.boards.len();
    let d = scored.divergent.len();
    let share = |k: usize| 100.0 * k as f64 / n.max(1) as f64;
    match metric {
        Metric::Direct => {
            writeln!(
                out,
                "ON {} vs OFF {} ({n} boards, vul {vul}): {d} fired ({:.2}%)",
                on.our_label,
                off.our_label,
                share(d),
            )?;
            writeln!(
                out,
                "Delta (direct A/B, IMP(on − off)): {:+} IMPs, {mean:+.4} IMPs/board \
                 [95% CI ±{ci:.4}], {:+.3} IMPs/fired",
                scored.total_imps,
                scored.total_imps as f64 / d.max(1) as f64,
            )?;
            writeln!(out, "{}", verdict_line(scorer, mean, ci, density))?;
        }
        Metric::Reference => {
            writeln!(
                out,
                "Reference-relative ON {} vs OFF {}, reference {} at table B \
                 ({n} boards, vul {vul}): {d} changed ({:.2}%), {moved} with a moved \
                 reference ({:.2}%)",
                on.our_label,
                off.our_label,
                on.their_label,
                share(d),
                share(moved),
            )?;
            writeln!(
                out,
                "Gain ({}, IMP(on − ref) − IMP(off − ref)): {:+} IMPs, {mean:+.4} IMPs/board \
                 [95% CI ±{ci:.4}], {:+.3} IMPs/changed",
                scorer,
                scored.total_imps,
                scored.total_imps as f64 / d.max(1) as f64,
            )?;
        }
    }

    let mut swings = scored.swings.clone();
    swings.sort_by_key(|&(_, _, imp)| imp);
    let show = show.min(swings.len());
    if show > 0 {
        writeln!(out, "--- {show} worst (for the feature) ---")?;
        for &(i, _, imp) in swings.iter().take(show) {
            let (a, b) = (&on.boards[i], &off.boards[i]);
            match metric {
                Metric::Direct => writeln!(
                    out,
                    "[{imp:+} IMP] {}\n  on:  {}\n  off: {}",
                    a.deal, a.table_a, b.table_a
                )?,
                Metric::Reference => writeln!(
                    out,
                    "[{imp:+} IMP] {}\n  on:  {}\n  ref: {}\n  off: {}\n  ref: {}",
                    a.deal, a.table_a, a.table_b, b.table_a, b.table_b
                )?,
            }
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let on = common::load_dump(&args.on);
    let off = common::load_dump(&args.off);
    anyhow::ensure!(on.boards.len() == off.boards.len(), "dumps must be aligned");
    // Both arms were bid at one vulnerability; `-v` only re-prices them
    anyhow::ensure!(
        on.vulnerability == off.vulnerability,
        "dumps bid at different vulnerabilities: {} vs {}",
        on.vulnerability,
        off.vulnerability,
    );
    // A reference gain is against one reference bidder
    anyhow::ensure!(
        args.metric == Metric::Direct || on.their_label == off.their_label,
        "references differ: {:?} vs {:?}",
        on.their_label,
        off.their_label,
    );
    let vul = args.vulnerability.unwrap_or(on.vulnerability);
    let density = dump_density(&on, args.density);

    // Same board, same dealer: the deals must match one to one
    for (i, (a, b)) in on.boards.iter().zip(&off.boards).enumerate() {
        anyhow::ensure!(a.deal == b.deal, "dumps not seed-aligned at board {i}");
        anyhow::ensure!(a.dealer == b.dealer, "dealers differ at board {i}");
    }
    let deals: Vec<_> = on.boards.iter().map(|board| board.deal).collect();
    let pairs = on.boards.iter().zip(&off.boards);
    // PD-P carries each arm's honored penalty in its contract's `penalty`
    // slot, which `ns_score_bid_honoring` ignores; the scorer reads it back.
    // PD-P reads only bid, declarer and honored penalty, so boards equal in
    // those cannot swing, and the solve skips them; its `fired` count can sit
    // below the plain report's (a pair differing only in a post-split double).
    let pdp = args.score == "pdp";
    anyhow::ensure!(
        !pdp || args.metric == Metric::Direct,
        "--score pdp prices the direct A/B only"
    );
    let honor = |reached: Reached, auction, other| {
        reached.map(|(contract, declarer)| {
            let penalty = shared_penalty(auction, other);
            (
                contract_bridge::Contract {
                    penalty,
                    ..contract
                },
                declarer,
            )
        })
    };
    let paired = match args.metric {
        Metric::Direct if pdp => Paired::Direct(
            pairs
                .map(|(a, b)| {
                    (
                        honor(final_contract(&a.table_a, a.dealer), &a.table_a, &b.table_a),
                        honor(final_contract(&b.table_a, b.dealer), &b.table_a, &a.table_a),
                    )
                })
                .collect(),
        ),
        Metric::Direct => Paired::Direct(
            pairs
                .map(|(a, b)| {
                    (
                        final_contract(&a.table_a, a.dealer),
                        final_contract(&b.table_a, b.dealer),
                    )
                })
                .collect(),
        ),
        Metric::Reference => Paired::Reference(
            pairs
                .map(|(a, b)| {
                    (
                        (
                            final_contract(&a.table_a, a.dealer),
                            final_contract(&a.table_b, a.dealer),
                        ),
                        (
                            final_contract(&b.table_a, b.dealer),
                            final_contract(&b.table_b, b.dealer),
                        ),
                    )
                })
                .collect(),
        ),
    };
    let moved = paired.moved_references();

    // One solve serves every scorer — a scorer only re-prices the solved tables,
    // so `--score both` costs one DDS fan-out and one parse of the dump, where
    // two single-scorer runs paid for each twice.
    type Scorer = fn(Reached, &ddss::TrickCountTable, AbsoluteVulnerability) -> i64;
    let (scorers, labels, keys): (Vec<Scorer>, Vec<&str>, Vec<&str>) = match args.score.as_str() {
        "plain" => (vec![ns_score_contract], vec!["plain DD"], vec!["plain"]),
        "pd" => (vec![ns_score_bid], vec!["PD"], vec!["pd"]),
        "pdp" => (
            vec![|reached: Reached, table: &ddss::TrickCountTable, vul| {
                let honored = reached.map_or(Penalty::Undoubled, |(c, _)| c.penalty);
                ns_score_bid_honoring(reached, honored, table, vul)
            }],
            vec!["PD-P"],
            vec!["pdp"],
        ),
        "both" => (
            vec![ns_score_contract, ns_score_bid],
            vec!["plain DD", "PD"],
            vec!["plain", "pd"],
        ),
        other => anyhow::bail!("--score must be plain|pd|pdp|both, got {other:?}"),
    };
    let scored = paired.score(&deals, vul, &scorers);
    let experiment = Experiment {
        on: &on,
        off: &off,
        vul,
        metric: args.metric,
        moved,
        show: args.show,
        density,
    };
    let write = |out: &mut dyn Write, scored: &Scored, scorer| {
        let mut out = std::io::BufWriter::new(out);
        report(&mut out, &experiment, scored, scorer)?;
        out.flush()
    };
    if args.score == "both" {
        let (plain_path, pd_path) = args
            .out_plain
            .as_deref()
            .zip(args.out_pd.as_deref())
            .expect("clap requires --out-plain and --out-pd when --score is both");
        for ((path, scored), label) in [plain_path, pd_path].into_iter().zip(&scored).zip(labels) {
            write(&mut std::fs::File::create(path)?, scored, label)?;
        }
    } else {
        write(&mut std::io::stdout().lock(), &scored[0], labels[0])?;
    }
    if let Some(path) = args.jsonl.as_deref() {
        let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
        // One solve: every scorer shares the divergent set
        for &i in &scored[0].divergent {
            let (a, b) = (&on.boards[i], &off.boards[i]);
            let mut record = serde_json::json!({
                "index": i,
                "deal": a.deal.to_string(),
                "dealer": format!("{:?}", a.dealer),
                "on": a.table_a,
                "off": b.table_a,
            });
            for (key, scored) in keys.iter().zip(&scored) {
                record[*key] = scored.board_imps[i].into();
            }
            serde_json::to_writer(&mut out, &record)?;
            writeln!(out)?;
        }
        out.flush()?;
    }
    Ok(())
}
