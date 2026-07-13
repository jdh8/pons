//! Bucket a Gladiator-advance A/B by *which advance our advancer made*.
//!
//! `set_nt_overcall_gladiator` layers a whole advancing structure on top of our
//! 1NT overcall of a major (their `1M` — our `1NT` — partner `Pass` — our
//! advance).  A single IMPs/board number over that structure is the usual
//! obstruction-wall + scope blend (docs/measurement.md); the verdict has to be
//! read per advance — the constructive relay/cue/invitational advances are
//! DD-visible, the leaping/to-play sacrifices are not.
//!
//! This pairs the ON/OFF `table_a` (our-pair-NS) contracts across every shard in
//! two dirs, keys each board by the Gladiator advance found in ON's `table_a`,
//! and reports per advance: divergent count, IMPs/board (±95% CI) and
//! IMPs/fired, dual-scored (plain DD + perfect defense).  Boards without the
//! `1M`-`1NT`-`Pass`-advance shape land in `(no-gladiator)` — still a divergence
//! the convention caused downstream, just not keyed to an advance.
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-gladiator-bucket -- ON_DIR OFF_DIR
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Bid, Strain};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, Reached, mean_with_ci, score_boards};

#[derive(Parser)]
struct Args {
    /// Directory of ON-arm shard-*.json (feature armed)
    on_dir: String,
    /// Directory of OFF-arm shard-*.json (same seeds/deals)
    off_dir: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
}

/// Load every `shard-*.json` in a dir, concatenated, plus the dump vulnerability.
fn load_dir(dir: &str) -> (AbsoluteVulnerability, Vec<Board>) {
    let mut paths: Vec<_> = std::fs::read_dir(dir)
        .expect("read arm dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("shard-") && s.ends_with(".json"))
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no shard-*.json in {dir}");
    let mut vul = None;
    let mut boards = Vec::new();
    for path in paths {
        let dump: Dump = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(&path).expect("open shard"),
        ))
        .expect("parse shard");
        vul = Some(dump.vulnerability);
        boards.extend(dump.boards);
    }
    (vul.expect("at least one shard"), boards)
}

/// Compare `calls[at]` (if present) against an expected bid.
fn is_bid(calls: &[Call], at: usize, level: u8, strain: Strain) -> bool {
    calls.get(at) == Some(&Call::Bid(Bid::new(level, strain)))
}

/// The label of the Gladiator advance our advancer made in `calls` (ON's
/// `table_a`), or `(no-gladiator)` if the `1M`-`1NT`-`Pass`-advance shape is
/// absent.  `M` = their major opening, `O` = the other major.
fn gladiator_bucket(calls: &[Call]) -> &'static str {
    for i in 0..calls.len() {
        // Their major opening, our 1NT overcall, partner's pass, our advance.
        let themaj = match calls[i] {
            Call::Bid(b) if b == Bid::new(1, Strain::Hearts) => Strain::Hearts,
            Call::Bid(b) if b == Bid::new(1, Strain::Spades) => Strain::Spades,
            _ => continue,
        };
        if !is_bid(calls, i + 1, 1, Strain::Notrump) {
            continue;
        }
        let omaj = if themaj == Strain::Hearts {
            Strain::Spades
        } else {
            Strain::Hearts
        };
        match calls.get(i + 2) {
            // Uncontested: their responder passes, our advance sits at i+3.
            Some(&Call::Pass) => {
                let Some(&Call::Bid(adv)) = calls.get(i + 3) else {
                    continue;
                };
                return classify_advance(adv, themaj, omaj, calls, i);
            }
            // Contested: their responder acted over our 1NT overcall — the
            // Branch A floor runout / B stolen-relay / C Transfer-Lebensohl
            // structure. Peel these out of `(no-gladiator)` so the bucket's
            // shrinkage is visible (HANDOFF-contested-gladiator.md).
            Some(&Call::Double) | Some(&Call::Bid(_)) => {
                return classify_contested(&calls[i + 2], calls.get(i + 3), themaj);
            }
            _ => continue,
        }
    }
    "(no-gladiator)"
}

/// Label a *contested* advance: `interf` is their responder's call over our 1NT
/// overcall, `adv` is our advancer's reply (if any). Coarse by design — the
/// success signal is that these buckets are non-empty (so `(no-gladiator)`
/// shrank) and net non-negative, not their fine structure.
fn classify_contested(interf: &Call, adv: Option<&Call>, themaj: Strain) -> &'static str {
    match *interf {
        // RHO doubles our 1NT: Branch A natural runout (default-on floor, both
        // arms — so this should net ~0; a label makes that legible).
        Call::Double => match adv {
            Some(&Call::Redouble) => "vs-X-redouble",
            Some(&Call::Bid(b)) if b == Bid::new(2, Strain::Notrump) => "vs-X-2NT-minors",
            Some(&Call::Bid(_)) => "vs-X-escape",
            _ => "vs-X-pass",
        },
        // RHO bids 2♣: Branch B stolen Gladiator relay (X) vs natural systems-on.
        Call::Bid(b) if b == Bid::new(2, Strain::Clubs) => match adv {
            Some(&Call::Double) => "vs-2C-stolen-x",
            Some(&Call::Bid(_)) => "vs-2C-natural",
            _ => "vs-2C-pass",
        },
        // RHO takes the 2-level in a suit: Branch C Transfer Lebensohl.
        Call::Bid(b) if b.level.get() == 2 && b.strain.suit().is_some() => match adv {
            Some(&Call::Bid(a)) if a == Bid::new(2, Strain::Notrump) => "lebensohl-2NT",
            Some(&Call::Bid(a)) if a.strain == themaj || a.strain == b.strain => "lebensohl-cue",
            Some(&Call::Bid(_)) => "lebensohl-direct",
            Some(&Call::Double) => "lebensohl-x",
            _ => "lebensohl-pass",
        },
        // 3-level+ interference: floor territory (deferred), not a B/C bucket.
        _ => "contested-other",
    }
}

/// Classify the advance bid `adv` (their major `themaj`, other major `omaj`).
fn classify_advance(
    adv: Bid,
    themaj: Strain,
    omaj: Strain,
    calls: &[Call],
    i: usize,
) -> &'static str {
    let bid = |level, strain| adv == Bid::new(level, strain);
    if bid(2, Strain::Clubs) {
        // 2♣ relay; a follow-up 2♦ then a delayed cue of their major is the new
        // treatment.
        if calls.get(i + 4) == Some(&Call::Pass)
            && is_bid(calls, i + 5, 2, Strain::Diamonds)
            && calls.get(i + 6) == Some(&Call::Pass)
            && is_bid(calls, i + 7, 2, themaj)
        {
            "delayed-cue-3O"
        } else {
            "relay-2C"
        }
    } else if bid(2, themaj) {
        "cue-stayman-4O"
    } else if bid(2, Strain::Diamonds) {
        "2D-inv"
    } else if bid(2, omaj) {
        "2O-inv"
    } else if bid(2, Strain::Notrump) {
        "2NT-club-transfer"
    } else if bid(3, Strain::Clubs) {
        "3C-gf"
    } else if bid(3, Strain::Diamonds) {
        "3D-gf"
    } else if bid(3, omaj) {
        "3O-gf"
    } else if bid(3, themaj) {
        "3M-splinter"
    } else if bid(3, Strain::Notrump) {
        "3NT"
    } else if bid(4, Strain::Clubs) {
        "4C-leaping"
    } else if bid(4, Strain::Diamonds) {
        "4D-leaping"
    } else if bid(4, themaj) {
        "4M-leaping-both-minors"
    } else if bid(4, omaj) {
        "4O-to-play"
    } else {
        "other-advance"
    }
}

fn main() {
    let args = Args::parse();
    let (on_vul, on) = load_dir(&args.on_dir);
    let (_, off) = load_dir(&args.off_dir);
    assert_eq!(
        on.len(),
        off.len(),
        "arms must be aligned (same board count)"
    );
    let vul = args.vulnerability.unwrap_or(on_vul);

    let mut deals = Vec::with_capacity(on.len());
    let contracts: Vec<(Reached, Reached)> = on
        .iter()
        .zip(&off)
        .map(|(a, b)| {
            assert_eq!(a.deal, b.deal, "arms not seed-aligned");
            deals.push(a.deal);
            (
                final_contract(&a.table_a, a.dealer),
                final_contract(&b.table_a, b.dealer),
            )
        })
        .collect();

    // Key every board by the Gladiator advance in ON's table_a.
    let bucket_of: Vec<&'static str> = on.iter().map(|a| gladiator_bucket(&a.table_a)).collect();

    // Solve the divergent boards ONCE (plain), then re-price the same DD tables
    // under perfect defense — one solve, both brackets.
    let scored = score_boards(&contracts, &deals, vul, ns_score_contract);
    let plain = scored.board_imps.clone();
    let mut pd = vec![0i64; contracts.len()];
    let mut divergent = vec![false; contracts.len()];
    for (k, &idx) in scored.divergent.iter().enumerate() {
        divergent[idx] = true;
        let table = &scored.tables[k];
        let (con, coff) = contracts[idx];
        pd[idx] = imps(ns_score_pd(con, table, vul) - ns_score_pd(coff, table, vul));
    }

    let contract_fired = scored.divergent.len();
    println!(
        "== gladiator-advance buckets: {} boards, vul {vul} — {} contract-divergent ({:.2}%) ==",
        on.len(),
        contract_fired,
        100.0 * contract_fired as f64 / on.len().max(1) as f64,
    );

    // Per-bucket stats over a mask: IMPs/board (masked mean over ALL boards, so
    // buckets sum to the total), its 95% CI, and IMPs/fired (over the
    // contract-divergent boards in the bucket — the IMP-relevant set).
    let stats = |series: &[i64], keep: &dyn Fn(usize) -> bool| -> (usize, f64, f64, f64) {
        let masked: Vec<i64> = series
            .iter()
            .enumerate()
            .map(|(i, &v)| if keep(i) { v } else { 0 })
            .collect();
        let fired = (0..series.len())
            .filter(|&i| keep(i) && divergent[i])
            .count();
        let total: i64 = masked.iter().sum();
        let (mean, ci) = mean_with_ci(&masked);
        let per_fired = if fired == 0 {
            0.0
        } else {
            total as f64 / fired as f64
        };
        (fired, mean, ci, per_fired)
    };

    // One row per distinct advance label.
    let mut labels: Vec<&'static str> = bucket_of.clone();
    labels.sort_unstable();
    labels.dedup();

    struct Row {
        label: &'static str,
        fired: usize,
        plain_mean: f64,
        plain_ci: f64,
        plain_fired: f64,
        pd_mean: f64,
        pd_ci: f64,
        pd_fired: f64,
    }

    let mut rows: Vec<Row> = labels
        .iter()
        .filter_map(|&label| {
            let keep = |i: usize| bucket_of[i] == label;
            let (fired, plain_mean, plain_ci, plain_fired) = stats(&plain, &keep);
            let (_, pd_mean, pd_ci, pd_fired) = stats(&pd, &keep);
            (fired >= 1).then_some(Row {
                label,
                fired,
                plain_mean,
                plain_ci,
                plain_fired,
                pd_mean,
                pd_ci,
                pd_fired,
            })
        })
        .collect();
    // Biggest plain movers first.
    rows.sort_by(|a, b| {
        b.plain_mean
            .abs()
            .partial_cmp(&a.plain_mean.abs())
            .expect("means are never NaN")
    });

    println!(
        "\n{:<24} {:>6}  {:>11} {:>9}  {:>11} {:>9}  {:>11} {:>11}",
        "advance", "fired", "plain/bd", "±95%CI", "PD/bd", "±95%CI", "plain/fired", "PD/fired"
    );
    let print_row = |label: &str, r: &Row| {
        println!(
            "{label:<24} {:>6}  {:>+11.4} {:>9}  {:>+11.4} {:>9}  {:>+11.3} {:>+11.3}",
            r.fired,
            r.plain_mean,
            format!("±{:.4}", r.plain_ci),
            r.pd_mean,
            format!("±{:.4}", r.pd_ci),
            r.plain_fired,
            r.pd_fired,
        );
    };
    for r in &rows {
        print_row(r.label, r);
    }

    // TOTAL = every bucket summed (all boards, all contract-divergent).
    let (fired, plain_mean, plain_ci, plain_fired) = stats(&plain, &|_| true);
    let (_, pd_mean, pd_ci, pd_fired) = stats(&pd, &|_| true);
    print_row(
        "TOTAL",
        &Row {
            label: "TOTAL",
            fired,
            plain_mean,
            plain_ci,
            plain_fired,
            pd_mean,
            pd_ci,
            pd_fired,
        },
    );
}
