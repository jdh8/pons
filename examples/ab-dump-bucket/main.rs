//! Bucket a competitive-rebid A/B by the call the knob changed.
//!
//! The `set_competitive_rebid` fired-set is a mix of a DD-visible constructive
//! class (opener's sound one-suiter rebid → a making game the takeout double
//! misses) and a DD-invisible/negative obstructive class (light overcaller
//! rebids, minimum competitive pushes).  A single IMPs/board number over that
//! blend is the "obstruction wall + scope artifact" bias (docs/measurement.md);
//! the verdict has to be read per bucket.
//!
//! This pairs the ON/OFF `table_a` (our-pair-NS) contracts across every shard in
//! two dirs, finds each divergent board's **first differing call** — exactly the
//! rebid the knob added — and reports IMPs/board (±95% CI) and IMPs/fired per
//! bucket, keyed by the rebidder's role (opener vs overcaller) and the rebid
//! level, dual-scored (plain DD + perfect defense).
//!
//! ```text
//! cargo run --release --features serde --example ab-dump-bucket -- ON_DIR OFF_DIR
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::scoring::{final_contract, ns_score_contract, ns_score_pd};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, Reached, mean_with_ci, score_boards, seat_to_act};
use pons::scoring::imps;

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

/// Which of our seats made the rebid, and where in the auction it opened.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Role {
    Opener,
    Overcaller,
}

/// The bucket a divergent board lands in.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Bucket {
    role: Role,
    level: u8,
    /// The OFF call the rebid replaced: 'P' pass, 'X' double, '?' other.
    replaced: char,
}

const fn same_side(a: Seat, b: Seat) -> bool {
    matches!(a, Seat::North | Seat::South) == matches!(b, Seat::North | Seat::South)
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

/// Classify a divergent board by the first `table_a` call that differs on↔off.
fn classify(on: &Board, off: &Board) -> Option<Bucket> {
    let a = &on.table_a;
    let b = &off.table_a;
    let i = (0..a.len().min(b.len())).find(|&i| a[i] != b[i])?;
    // The knob only ever adds a suit rebid, so the ON call at the divergence is
    // that bid; the OFF call is what it displaced.
    let Call::Bid(bid) = a[i] else { return None };
    let seat = seat_to_act(on.dealer, i);
    let opener_seat = (0..a.len())
        .find(|&k| matches!(a[k], Call::Bid(_)))
        .map(|k| seat_to_act(on.dealer, k));
    let role = match opener_seat {
        Some(os) if same_side(os, seat) => Role::Opener,
        _ => Role::Overcaller,
    };
    let replaced = match b[i] {
        Call::Pass => 'P',
        Call::Double => 'X',
        _ => '?',
    };
    Some(Bucket {
        role,
        level: bid.level.get(),
        replaced,
    })
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

    let bucket_of: Vec<Option<Bucket>> = on.iter().zip(&off).map(|(a, b)| classify(a, b)).collect();

    // Solve the divergent boards ONCE (plain), then re-price the same DD tables
    // under perfect defense — one solve, both brackets.
    let scored = score_boards(&contracts, &deals, vul, ns_score_contract);
    let plain = scored.board_imps.clone();
    let mut pd = vec![0i64; contracts.len()];
    for (k, &idx) in scored.divergent.iter().enumerate() {
        let table = &scored.tables[k];
        let (con, coff) = contracts[idx];
        pd[idx] = imps(ns_score_pd(con, table, vul) - ns_score_pd(coff, table, vul));
    }

    // "fired" here = boards whose final contract diverged (the IMP-relevant set);
    // a call can differ yet land in the same contract (0 swing), so this is a
    // subset of the call-divergent boards `classify` tags.
    let contract_fired = scored.divergent.len();
    let call_fired = bucket_of.iter().filter(|b| b.is_some()).count();
    println!(
        "== competitive-rebid buckets: {} boards, vul {vul} — {} contract-divergent ({:.2}%), {} call-divergent ==",
        on.len(),
        contract_fired,
        100.0 * contract_fired as f64 / on.len().max(1) as f64,
        call_fired,
    );

    // The buckets we report, in reading order.
    let keys: Vec<(&str, Role, u8)> = vec![
        ("opener  2-lvl", Role::Opener, 2),
        ("opener  3-lvl", Role::Opener, 3),
        ("overcaller 2-lvl", Role::Overcaller, 2),
        ("overcaller 3-lvl", Role::Overcaller, 3),
    ];

    let mut divergent = vec![false; contracts.len()];
    for &i in &scored.divergent {
        divergent[i] = true;
    }

    for (label, imps) in [("PLAIN DD", &plain), ("PERFECT DEFENSE", &pd)] {
        println!("\n--- {label} ---");
        println!(
            "{:<18} {:>6}  {:>10}  {:>12}  {:>11}",
            "bucket", "fired", "IMPs/bd", "±95% CI", "IMPs/fired"
        );
        let report = |label: &str, mask: &dyn Fn(usize) -> bool| {
            let masked: Vec<i64> = imps
                .iter()
                .enumerate()
                .map(|(i, &v)| if mask(i) { v } else { 0 })
                .collect();
            // fired = contract-divergent boards in the bucket (the IMP-relevant set)
            let fired = (0..imps.len()).filter(|&i| mask(i) && divergent[i]).count();
            let total: i64 = masked.iter().sum();
            let (mean, ci) = mean_with_ci(&masked);
            let per_fired = if fired == 0 {
                0.0
            } else {
                total as f64 / fired as f64
            };
            println!(
                "{label:<18} {fired:>6}  {mean:>+10.4}  {:>12}  {per_fired:>+11.3}",
                format!("±{ci:.4}")
            );
        };
        for (label, role, level) in &keys {
            report(label, &|i| {
                bucket_of[i].is_some_and(|b| b.role == *role && b.level == *level)
            });
        }
        report("ALL fired", &|i| bucket_of[i].is_some());
    }

    // Replaced-call distribution, so an X→rebid (lost the takeout) vs P→rebid
    // (found a bid where we had none) split is visible.
    let mut px = [0usize; 3]; // P, X, other
    for b in bucket_of.iter().flatten() {
        px[match b.replaced {
            'P' => 0,
            'X' => 1,
            _ => 2,
        }] += 1;
    }
    println!(
        "\nreplaced: {} pass, {} double, {} other",
        px[0], px[1], px[2]
    );
}
