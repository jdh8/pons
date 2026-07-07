//! Decompose a pons-vs-BBA duplicate match into ranked IMP-loss buckets
//!
//! The attribution half of the gap campaign
//! ([docs/bba-gap-campaign.md](../../docs/bba-gap-campaign.md)): reads
//! `bba-gen` shard dumps (one or both vulnerability arms of an anchor), finds
//! each board's first divergent call, replays it through the live books to
//! attribute it (phase × provenance × auction family × direction of loss),
//! dual-scores the divergent boards — plain DD and perfect defense from the
//! same solve — and writes a ranked-bucket `report.md` plus a
//! machine-readable `boards.jsonl`.
//!
//! Our side is deterministic, so provenance is **derived by replay**, never
//! recorded at generation time.  The printed replay-verification rate must be
//! 100% (every our-side call reproduced by the default books) for the
//! attribution to be exact; a lower rate means the dump was generated with
//! non-default knobs (check its recorded `gen_args`) or at a different git
//! revision — fix that before trusting the buckets.
//!
//! `--dd-cache` makes re-anchoring cheap: DD tables key on the *deal*, which
//! never changes under the anchor's fixed seed series, so a re-run after a
//! batch of fixes only solves newly-divergent boards.

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

use clap::Parser;
use common::{Dump, Reached, mean_with_ci, next_call, seat_to_act};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat, Strain};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::american::american;
use pons::bidding::context::relative;
use pons::bidding::{Family, Phase, Stance};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

/// Decompose `bba-gen` anchor dumps into ranked IMP-loss buckets
#[derive(Parser)]
struct Args {
    /// Shard dumps from `bba-gen`: files, or directories whose `*.json` files
    /// are the shards.  Both vulnerability arms of an anchor may be given
    /// together; they are reported per arm and bucketed jointly.
    inputs: Vec<String>,

    /// DD-table cache (JSON file), created if absent and updated with new
    /// solves — the artifact that makes a re-anchor take minutes
    #[arg(long)]
    dd_cache: Option<String>,

    /// Write the full markdown report here (a headline always prints to stdout)
    #[arg(long)]
    report: Option<String>,

    /// Write one JSON record per contract-divergent board here
    #[arg(long)]
    jsonl: Option<String>,

    /// Worst boards to detail per losing bucket
    #[arg(long, default_value = "3")]
    top: usize,

    /// How many of the worst buckets to detail with boards
    #[arg(long, default_value = "10")]
    buckets: usize,
}

/// Short label for a call (`P`, `X`, `XX`, `2♣`, …)
fn call_label(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => bid.to_string(),
    }
}

/// Render a reached contract for the report
fn contract_label(reached: Reached) -> String {
    reached.map_or("passed out".into(), |(c, s)| format!("{c} by {s:?}"))
}

/// Contract band for the direction heuristic: 0 partscore/pass-out, 1 game,
/// 2 small slam, 3 grand
fn band(reached: Reached) -> u8 {
    let Some((contract, _)) = reached else {
        return 0;
    };
    let level = contract.bid.level.get();
    let game = match level {
        3 => contract.bid.strain == Strain::Notrump,
        4 => contract.bid.strain >= Strain::Hearts,
        5.. => true,
        _ => false,
    };
    match level {
        7 => 3,
        6 => 2,
        _ if game => 1,
        _ => 0,
    }
}

/// Whether the reached contract makes double-dummy
fn makes(reached: Reached, table: &TrickCountTable, vul: AbsoluteVulnerability) -> bool {
    let Some((_, declarer)) = reached else {
        return false;
    };
    let score = ns_score_contract(reached, table, vul);
    match declarer {
        Seat::North | Seat::South => score > 0,
        Seat::East | Seat::West => score < 0,
    }
}

/// Triage label for how our line lost the board, comparing what the same
/// (table-A NS) cards did under our management (`a`) vs BBA's (`b`).
///
/// `ponytail:` a coarse heuristic for ranking, not an exhaustive taxonomy —
/// the per-board dump under each bucket is the ground truth.
fn direction(
    a: Reached,
    b: Reached,
    table: &TrickCountTable,
    vul: AbsoluteVulnerability,
    swing_plain: i64,
) -> &'static str {
    if swing_plain > 0 {
        return "gain";
    }
    if swing_plain == 0 {
        return "flat";
    }
    let ns =
        |reached: Reached| reached.is_some_and(|(_, s)| matches!(s, Seat::North | Seat::South));
    let (band_a, band_b) = (band(a), band(b));
    if ns(b) && band_b > band_a && makes(b, table, vul) {
        return match band_b {
            3 => "missed-grand",
            2 => "missed-slam",
            _ => "missed-game",
        };
    }
    if ns(a) && !makes(a, table, vul) && (band_a > band_b || !ns(b)) {
        return "overbid";
    }
    if ns(a) && ns(b) && band_a == band_b {
        let (bid_a, bid_b) = (a.expect("ns(a)").0.bid, b.expect("ns(b)").0.bid);
        if bid_a.strain != bid_b.strain {
            return "wrong-strain";
        }
        if bid_a == bid_b {
            return "doubling";
        }
    }
    if !ns(a) && ns(b) {
        return "sold-out";
    }
    "other"
}

/// Coarse auction-family tag from the divergence point.
///
/// `ponytail:` phase × round is deliberately coarse — role tags (response,
/// rebid, advance) can be added when a bucket needs the finer cut.
fn family(prefix: &[Call], index: usize) -> &'static str {
    if prefix.iter().all(|&c| c == Call::Pass) {
        return "opening";
    }
    if index >= 2 && prefix[index - 2..].iter().all(|&c| c == Call::Pass) {
        return "balancing";
    }
    match index / 4 {
        0 => "round-1",
        1 => "round-2",
        _ => "deep",
    }
}

/// One vulnerability arm of the anchor: its merged shards, with each board's
/// originating shard seed for a self-describing report
struct Arm {
    vul: AbsoluteVulnerability,
    boards: Vec<common::Board>,
    origin: Vec<(Option<u64>, usize)>,
    gen_args: Vec<String>,
}

/// Everything the report needs about one contract-divergent board
struct Row {
    arm: usize,
    board: usize,
    swing_plain: i64,
    swing_pd: i64,
    points: i64,
    div_index: usize,
    phase: Phase,
    bucket: String,
    prov: String,
    rule: String,
    family: &'static str,
    direction: &'static str,
    our_call: String,
    bba_call: String,
    hand: String,
}

fn load_arms(inputs: &[String]) -> anyhow::Result<Vec<Arm>> {
    let mut files = Vec::new();
    for input in inputs {
        let path = std::path::Path::new(input);
        if path.is_dir() {
            let mut shard: Vec<_> = std::fs::read_dir(path)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect();
            shard.sort();
            files.extend(shard);
        } else {
            files.push(path.to_path_buf());
        }
    }
    anyhow::ensure!(!files.is_empty(), "no input dumps found");

    let mut arms: Vec<Arm> = Vec::new();
    for file in files {
        let dump: Dump =
            serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&file)?))?;
        let arm = match arms.iter_mut().find(|arm| arm.vul == dump.vulnerability) {
            Some(arm) => arm,
            None => {
                arms.push(Arm {
                    vul: dump.vulnerability,
                    boards: Vec::new(),
                    origin: Vec::new(),
                    gen_args: dump.gen_args.clone(),
                });
                arms.last_mut().expect("just pushed")
            }
        };
        arm.origin
            .extend((0..dump.boards.len()).map(|i| (dump.seed, i)));
        arm.boards.extend(dump.boards);
    }
    Ok(arms)
}

/// Replay every our-side call of the arm through the default books and count
/// mismatches — the attribution-exactness guard
fn replay_verify(stance: &Stance, arm: &Arm) -> (u64, u64) {
    arm.boards
        .par_iter()
        .map(|board| {
            let mut checked = 0u64;
            let mut mismatched = 0u64;
            for (auction, ours_ns) in [(&board.table_a, true), (&board.table_b, false)] {
                let mut prefix = Auction::new();
                for (i, &call) in auction.iter().enumerate() {
                    let seat = seat_to_act(board.dealer, i);
                    let seat_ns = matches!(seat, Seat::North | Seat::South);
                    if seat_ns == ours_ns {
                        checked += 1;
                        let replayed =
                            next_call(stance, board.deal[seat], board.dealer, arm.vul, &prefix);
                        mismatched += u64::from(replayed != call);
                    }
                    prefix.push(call);
                }
            }
            (checked, mismatched)
        })
        .reduce(|| (0, 0), |x, y| (x.0 + y.0, x.1 + y.1))
}

/// A deal's cache key: its serde string, stable for a cache file
fn deal_key(deal: &FullDeal) -> String {
    serde_json::to_string(deal).expect("a deal serializes")
}

#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let arms = load_arms(&args.inputs)?;
    let stance = american().against(Family::NATURAL);

    // DD cache: deal-keyed tables survive across anchors (same seeds → same
    // deals), so only newly-divergent boards ever need a fresh solve.
    let mut cache: HashMap<String, TrickCountTable> = match args.dd_cache.as_deref() {
        Some(path) if std::path::Path::new(path).exists() => {
            serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(path)?))?
        }
        _ => HashMap::new(),
    };
    let cached_before = cache.len();

    let mut report = String::new();
    let mut rows: Vec<Row> = Vec::new();
    let mut right_siding = 0usize;

    for (arm_index, arm) in arms.iter().enumerate() {
        let count = arm.boards.len();
        let (checked, mismatched) = replay_verify(&stance, arm);
        let verified = 100.0 * (checked - mismatched) as f64 / checked.max(1) as f64;

        let contracts: Vec<(Reached, Reached)> = arm
            .boards
            .iter()
            .map(|b| {
                (
                    final_contract(&b.table_a, b.dealer),
                    final_contract(&b.table_b, b.dealer),
                )
            })
            .collect();
        let auction_divergent: Vec<Option<usize>> = arm
            .boards
            .iter()
            .map(|b| {
                (0..b.table_a.len().max(b.table_b.len()))
                    .find(|&i| b.table_a.get(i) != b.table_b.get(i))
            })
            .collect();
        let divergent: Vec<usize> = (0..count)
            .filter(|&i| contracts[i].0 != contracts[i].1)
            .collect();
        right_siding += (0..count)
            .filter(|&i| auction_divergent[i].is_some() && contracts[i].0 == contracts[i].1)
            .count();

        // Solve only the cache misses, chunked on the main thread (the solver
        // parallelizes internally; chunking just bounds one FFI batch).
        let missing: Vec<usize> = divergent
            .iter()
            .copied()
            .filter(|&i| !cache.contains_key(&deal_key(&arm.boards[i].deal)))
            .collect();
        for chunk in missing.chunks(4096) {
            let deals: Vec<FullDeal> = chunk.iter().map(|&i| arm.boards[i].deal).collect();
            let solved = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);
            for (&i, table) in chunk.iter().zip(solved) {
                cache.insert(deal_key(&arm.boards[i].deal), table);
            }
        }

        // Dual-score and attribute every contract-divergent board.
        let mut plain = vec![0i64; count];
        let mut pd = vec![0i64; count];
        for &i in &divergent {
            let board = &arm.boards[i];
            let table = &cache[&deal_key(&board.deal)];
            let (a, b) = contracts[i];
            let points =
                ns_score_contract(a, table, arm.vul) - ns_score_contract(b, table, arm.vul);
            plain[i] = imps(points);
            pd[i] = imps(ns_score_pd(a, table, arm.vul) - ns_score_pd(b, table, arm.vul));

            let div_index =
                auction_divergent[i].expect("contract-divergent implies auction-divergent");
            let prefix = &board.table_a[..div_index];
            let seat = seat_to_act(board.dealer, div_index);
            let ours_at_a = matches!(seat, Seat::North | Seat::South);
            let (our_call, bba_call) = if ours_at_a {
                (board.table_a[div_index], board.table_b[div_index])
            } else {
                (board.table_b[div_index], board.table_a[div_index])
            };
            let explained =
                stance.explain_call(board.deal[seat], relative(arm.vul, seat), prefix, our_call);
            let (prov, rule) = match &explained {
                None => ("unresolved".into(), String::new()),
                Some((p, rule)) => {
                    let base = match (p.depth, p.fallback) {
                        (_, None) => "book".to_string(),
                        (0, Some(_)) => match rule {
                            Some(r) => format!("floor#{}", r.index),
                            None => "floor".to_string(),
                        },
                        (depth, Some(_)) => format!("fallback@{depth}"),
                    };
                    let base = if p.rebases > 0 {
                        format!("{base}+rb")
                    } else {
                        base
                    };
                    (
                        base,
                        rule.as_ref()
                            .map(|r| r.description.clone())
                            .unwrap_or_default(),
                    )
                }
            };
            let phase = Phase::of(prefix);
            let fam = family(prefix, div_index);
            rows.push(Row {
                arm: arm_index,
                board: i,
                swing_plain: plain[i],
                swing_pd: pd[i],
                points,
                div_index,
                phase,
                bucket: format!("{phase:?} / {prov} / {fam}"),
                prov,
                rule,
                family: fam,
                direction: direction(a, b, table, arm.vul, plain[i]),
                our_call: call_label(our_call),
                bba_call: call_label(bba_call),
                hand: board.deal[seat].to_string(),
            });
        }

        let (mean_plain, ci_plain) = mean_with_ci(&plain);
        let (mean_pd, ci_pd) = mean_with_ci(&pd);
        let headline = format!(
            "=== arm {}: {} (us) vs {} (them), vulnerability {}, {count} boards ===\n\
             replay verification: {verified:.2}% of {checked} our-side calls ({mismatched} mismatched)\n\
             auction-divergent: {} ({:.0}%), contract-divergent: {} ({:.0}%)\n\
             plain DD: {:+.4} IMPs/board (95% CI [{:+.4}, {:+.4}]), {:+} IMPs total\n\
             perfect defense: {:+.4} IMPs/board (95% CI [{:+.4}, {:+.4}])\n\
             gen_args: {}\n",
            arm_index,
            "our american floor",
            "BBA 2/1",
            arm.vul,
            auction_divergent.iter().flatten().count(),
            100.0 * auction_divergent.iter().flatten().count() as f64 / count.max(1) as f64,
            divergent.len(),
            100.0 * divergent.len() as f64 / count.max(1) as f64,
            mean_plain,
            mean_plain - ci_plain,
            mean_plain + ci_plain,
            plain.iter().sum::<i64>(),
            mean_pd,
            mean_pd - ci_pd,
            mean_pd + ci_pd,
            arm.gen_args.join(" "),
        );
        print!("{headline}");
        let _ = writeln!(report, "{headline}");
        if mismatched > 0 {
            let warning = "WARNING: replay verification below 100% — attribution is approximate.\n\
                 The dump was generated with non-default knobs or a different revision.\n";
            print!("{warning}");
            report.push_str(warning);
        }
    }

    // IMP histogram over contract-divergent boards (plain scorer).
    let mut histogram: BTreeMap<i64, usize> = BTreeMap::new();
    for row in &rows {
        *histogram.entry(row.swing_plain).or_default() += 1;
    }
    let _ = writeln!(
        report,
        "\n## IMP histogram (plain, per contract-divergent board)\n\n\
         right-siding-only divergences (same contract, different auction): {right_siding}\n"
    );
    for (imp, n) in &histogram {
        let _ = writeln!(report, "  {imp:+3} IMPs: {n}");
    }

    // Composite buckets: phase / provenance / family, ranked worst-first.
    struct Bucket {
        n: usize,
        plain: i64,
        pd: i64,
        swings: Vec<i64>,
        rows: Vec<usize>,
    }
    let mut buckets: BTreeMap<&str, Bucket> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let bucket = buckets.entry(&row.bucket).or_insert_with(|| Bucket {
            n: 0,
            plain: 0,
            pd: 0,
            swings: Vec::new(),
            rows: Vec::new(),
        });
        bucket.n += 1;
        bucket.plain += row.swing_plain;
        bucket.pd += row.swing_pd;
        bucket.swings.push(row.swing_plain);
        bucket.rows.push(index);
    }
    let mut ranked: Vec<(&str, &Bucket)> = buckets.iter().map(|(k, v)| (*k, v)).collect();
    ranked.sort_by_key(|(_, b)| b.plain);

    let _ = writeln!(
        report,
        "\n## Ranked buckets (phase / provenance / family), losses first\n\n\
         | bucket | boards | net plain IMPs | IMPs/divergent ±CI | net PD IMPs | flag |\n\
         | --- | --- | --- | --- | --- | --- |"
    );
    for (name, bucket) in &ranked {
        // `mean_with_ci` degenerates below two samples; the mean itself is
        // always well-defined, so compute it directly and keep only the CI.
        let mean = bucket.plain as f64 / bucket.n.max(1) as f64;
        let (_, ci) = mean_with_ci(&bucket.swings);
        let noise = if mean.abs() <= ci || bucket.n < 2 {
            " ~noise"
        } else {
            ""
        };
        let artifact = if (bucket.plain < 0) != (bucket.pd < 0) {
            " plain/PD-flip"
        } else {
            ""
        };
        let _ = writeln!(
            report,
            "| {name} | {} | {:+} | {mean:+.2} ±{ci:.2} | {:+} |{noise}{artifact} |",
            bucket.n, bucket.plain, bucket.pd,
        );
    }

    // Marginal cuts for orientation.
    for (title, key) in [
        ("phase", 0usize),
        ("provenance", 1),
        ("family", 2),
        ("direction", 3),
    ] {
        let mut cut: BTreeMap<String, (usize, i64)> = BTreeMap::new();
        for row in &rows {
            let label = match key {
                0 => format!("{:?}", row.phase),
                1 => row.prov.clone(),
                2 => row.family.to_string(),
                _ => row.direction.to_string(),
            };
            let entry = cut.entry(label).or_default();
            entry.0 += 1;
            entry.1 += row.swing_plain;
        }
        let mut sorted: Vec<_> = cut.into_iter().collect();
        sorted.sort_by_key(|(_, (_, imps))| *imps);
        let _ = writeln!(report, "\n## By {title}\n");
        for (label, (n, imp)) in sorted {
            let _ = writeln!(report, "  {imp:+7} IMPs  {n:>6} boards  {label}");
        }
    }

    // Worst boards per worst bucket — the work list, deal by deal.
    let _ = writeln!(report, "\n## Worst boards per losing bucket\n");
    for (name, bucket) in ranked.iter().take(args.buckets) {
        if bucket.plain >= 0 {
            break;
        }
        let _ = writeln!(
            report,
            "### {name} ({} boards, {:+} IMPs)\n",
            bucket.n, bucket.plain
        );
        let mut worst: Vec<usize> = bucket.rows.clone();
        worst.sort_by_key(|&r| rows[r].swing_plain);
        for &r in worst.iter().take(args.top) {
            let row = &rows[r];
            let arm = &arms[row.arm];
            let board = &arm.boards[row.board];
            let (seed, shard_index) = arm.origin[row.board];
            let _ = writeln!(
                report,
                "[vul {}, seed {:?}, board {shard_index}] swing {:+} pts / {:+} IMPs (PD {:+}), \
                 diverged at call {} ({} ours vs {} BBA), {}\n  rule: {}\n  {}\n  ours NS @ A: {}  -> {}\n  ours EW @ B: {}  -> {}\n",
                arm.vul,
                seed,
                row.points,
                row.swing_plain,
                row.swing_pd,
                row.div_index,
                row.our_call,
                row.bba_call,
                row.direction,
                if row.rule.is_empty() {
                    "(none)"
                } else {
                    &row.rule
                },
                board.deal.display(Seat::West),
                board.table_a,
                contract_label(final_contract(&board.table_a, board.dealer)),
                board.table_b,
                contract_label(final_contract(&board.table_b, board.dealer)),
            );
        }
    }

    if let Some(path) = args.report.as_deref() {
        std::fs::write(path, &report)?;
        eprintln!("bba-decompose: report written to {path}");
    }
    if let Some(path) = args.jsonl.as_deref() {
        let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
        use std::io::Write as _;
        for row in &rows {
            let arm = &arms[row.arm];
            let (seed, shard_index) = arm.origin[row.board];
            serde_json::to_writer(
                &mut out,
                &serde_json::json!({
                    "vul": arm.vul.to_string(),
                    "seed": seed,
                    "board": shard_index,
                    "swing_plain": row.swing_plain,
                    "swing_pd": row.swing_pd,
                    "points": row.points,
                    "div_index": row.div_index,
                    "phase": format!("{:?}", row.phase),
                    "provenance": row.prov,
                    "rule": row.rule,
                    "family": row.family,
                    "direction": row.direction,
                    "our_call": row.our_call,
                    "bba_call": row.bba_call,
                    "hand": row.hand,
                }),
            )?;
            writeln!(out)?;
        }
        eprintln!("bba-decompose: {} rows written to {path}", rows.len());
    }
    if let Some(path) = args.dd_cache.as_deref() {
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(path)?),
            &cache,
        )?;
        eprintln!(
            "bba-decompose: DD cache {path} now holds {} tables ({} new)",
            cache.len(),
            cache.len() - cached_before,
        );
    }
    Ok(())
}
