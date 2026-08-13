//! Census: how much does each interference call over **our 1NT** cost us?
//!
//! The anchor report's ranked buckets stop at `Competitive / book / round-1` —
//! one number over every contested opening.  This splits the 1NT slice of that
//! blend by **RHO's call**, so the competitive-1NT campaign
//! ([docs/one-notrump-competitive.md](../../docs/one-notrump-competitive.md))
//! picks its next package from measured cost instead of from how wrong each
//! mismatch looks on paper.  It reads an existing anchor arm — no generation,
//! no new DD beyond the arm's own divergent set.
//!
//! Our pair sits NS at table A, so a board counts when `table_a` opens 1NT from
//! North or South and East/West act over it.  The bucket is that action: `X`,
//! `2♣`…`2NT`, or `3+`.  Both brackets are reported off one solve.
//!
//! ```text
//! cargo run --release --features serde --example probe-1nt-interference -- \
//!     ab-results/anchor/2026-08-12-ea2cde9-dirty/american-none \
//!     --dd-cache ab-results/anchor/dd-cache.json
//! ```
//!
//! Pass the anchor's `--dd-cache` and the census costs seconds instead of the
//! arm's whole DD fan-out — the tables key on the *deal*, so the anchor already
//! solved every board this reads.  Without it, every divergent board is solved
//! fresh (`bba-decompose`'s idiom, same key function).
//!
//! **What this can and cannot say.**  The swing is the *board's* IMPs, not the
//! interference decision's: the same deal may also have BBA opening 1NT at
//! table B with us defending, and every later call is in there too.  So the
//! buckets **rank**, they do not isolate — isolation is the package's own A/B
//! (`bba-gen --filter-1nt`, one knob).  The confound is broadly common to all
//! buckets, which is what leaves the ranking usable.

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use std::collections::HashMap;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, Reached, mean_with_ci, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Directory of `shard-*.json` from one anchor arm
    dir: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// The anchor's deal-keyed DD table cache; read-only here
    #[arg(long)]
    dd_cache: Option<String>,
    /// Dump this many worst plain-DD boards from `--bucket`
    #[arg(long, default_value_t = 0)]
    show: usize,
    /// Which bucket `--show` dumps, e.g. `2♣`, `X`, `3+`
    #[arg(long, default_value = "")]
    bucket: String,
}

/// A deal's cache key: its serde string, as `bba-decompose` writes it
fn deal_key(deal: &FullDeal) -> String {
    serde_json::to_string(deal).expect("a deal serializes")
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

/// What a board contributes to the census.
enum Class {
    /// We did not open 1NT at table A — an opponent opened, or we opened
    /// something else.  A 1NT *overcall* lands here too.
    Other,
    /// We opened 1NT and RHO passed.
    Uncontested,
    /// We opened 1NT and RHO acted; the label is that action.
    Contested(String),
}

/// Classify a board by RHO's action over our table-A 1NT opening.
fn classify(board: &Board) -> Class {
    let Some(open) = board.table_a.iter().position(|&c| c != Call::Pass) else {
        return Class::Other;
    };
    if !matches!(seat_to_act(board.dealer, open), Seat::North | Seat::South) {
        return Class::Other;
    }
    let Call::Bid(bid) = board.table_a[open] else {
        return Class::Other;
    };
    if bid.level.get() != 1 || bid.strain.is_suit() {
        return Class::Other;
    }
    match board.table_a.get(open + 1) {
        None | Some(&Call::Pass) => Class::Uncontested,
        Some(&Call::Redouble) => Class::Contested("XX".to_owned()),
        Some(&Call::Double) => Class::Contested("X".to_owned()),
        // ponytail: everything above 2NT is one bucket — the whole 3+ region is
        // floor-only today, so splitting it cannot change which package is next.
        Some(&Call::Bid(rho)) if rho.level.get() > 2 => Class::Contested("3+".to_owned()),
        Some(&Call::Bid(rho)) => Class::Contested(rho.to_string()),
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let (dump_vul, boards) = load_dir(&args.dir);
    let vul = args.vulnerability.unwrap_or(dump_vul);

    let contracts: Vec<(Reached, Reached)> = boards
        .iter()
        .map(|b| {
            (
                final_contract(&b.table_a, b.dealer),
                final_contract(&b.table_b, b.dealer),
            )
        })
        .collect();
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .collect();

    let mut cache: HashMap<String, TrickCountTable> = match args.dd_cache.as_deref() {
        Some(path) => serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(path).expect("open dd cache"),
        ))
        .expect("parse dd cache"),
        None => HashMap::new(),
    };
    let hits = divergent
        .iter()
        .filter(|&&i| cache.contains_key(&deal_key(&boards[i].deal)))
        .count();
    eprintln!(
        "dd cache: {hits}/{} divergent boards hit, {} to solve",
        divergent.len(),
        divergent.len() - hits
    );
    let missing: Vec<usize> = divergent
        .iter()
        .copied()
        .filter(|&i| !cache.contains_key(&deal_key(&boards[i].deal)))
        .collect();
    for chunk in missing.chunks(4096) {
        let deals: Vec<FullDeal> = chunk.iter().map(|&i| boards[i].deal).collect();
        let solved = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);
        for (&i, table) in chunk.iter().zip(solved) {
            cache.insert(deal_key(&boards[i].deal), table);
        }
    }

    let mut plain = vec![0i64; boards.len()];
    let mut pd = vec![0i64; boards.len()];
    for &i in &divergent {
        let table = &cache[&deal_key(&boards[i].deal)];
        let (a, b) = contracts[i];
        plain[i] = imps(ns_score_contract(a, table, vul) - ns_score_contract(b, table, vul));
        pd[i] = imps(ns_score_pd(a, table, vul) - ns_score_pd(b, table, vul));
    }

    let mut buckets: std::collections::BTreeMap<String, (Vec<i64>, Vec<i64>)> =
        std::collections::BTreeMap::new();
    let mut quiet: Vec<i64> = Vec::new();
    let mut shown: Vec<usize> = Vec::new();
    for (index, board) in boards.iter().enumerate() {
        match classify(board) {
            Class::Other => {}
            Class::Uncontested => quiet.push(plain[index]),
            Class::Contested(label) => {
                if label == args.bucket {
                    shown.push(index);
                }
                let entry = buckets.entry(label).or_default();
                entry.0.push(plain[index]);
                entry.1.push(pd[index]);
            }
        }
    }

    let uncontested = quiet.len();
    let contested: usize = buckets.values().map(|(p, _)| p.len()).sum();
    let opened = contested + uncontested;
    println!("=== interference over our 1NT — {} ===", args.dir);
    println!("vulnerability {vul:?}, {} boards", boards.len());
    println!(
        "we opened 1NT: {opened} ({:.2}% of boards); contested {contested} ({:.1}% of our 1NTs)",
        100.0 * opened as f64 / boards.len() as f64,
        100.0 * contested as f64 / opened.max(1) as f64,
    );
    println!(
        "\nIMPs/board are the BOARD's swing, not the node's — these RANK, they do not isolate.\n"
    );
    println!(
        "{:<6} {:>8} {:>8} {:>20} {:>20} {:>10}",
        "RHO", "boards", "share", "plain IMPs/bd", "PD IMPs/bd", "plain tot"
    );
    let mut rows: Vec<_> = buckets.iter().collect();
    rows.sort_by(|a, b| {
        let total = |v: &Vec<i64>| v.iter().sum::<i64>();
        total(&a.1.0).cmp(&total(&b.1.0))
    });
    for (label, (p, d)) in rows {
        let (pm, pc) = mean_with_ci(p);
        let (dm, dc) = mean_with_ci(d);
        println!(
            "{label:<6} {:>8} {:>7.2}% {:>12.4} ±{:.4} {:>12.4} ±{:.4} {:>10}",
            p.len(),
            100.0 * p.len() as f64 / opened.max(1) as f64,
            pm,
            pc,
            dm,
            dc,
            p.iter().sum::<i64>(),
        );
    }
    let (all_p, all_d): (Vec<i64>, Vec<i64>) = buckets
        .values()
        .flat_map(|(p, d)| p.iter().copied().zip(d.iter().copied()))
        .unzip();
    let (pm, pc) = mean_with_ci(&all_p);
    let (dm, dc) = mean_with_ci(&all_d);
    println!(
        "{:<6} {:>8} {:>7.2}% {:>12.4} ±{:.4} {:>12.4} ±{:.4} {:>10}",
        "ALL",
        all_p.len(),
        100.0,
        pm,
        pc,
        dm,
        dc,
        all_p.iter().sum::<i64>(),
    );
    let (um, uc) = mean_with_ci(&quiet);
    println!(
        "\nreference — our uncontested 1NT: {uncontested} boards, plain {um:.4} ±{uc:.4} IMPs/bd"
    );

    if args.show > 0 {
        shown.sort_by_key(|&i| plain[i]);
        println!(
            "\n--- {} worst plain boards in {} ---",
            args.show, args.bucket
        );
        for &i in shown.iter().take(args.show) {
            let b = &boards[i];
            println!(
                "[{:+} plain / {:+} PD] {}\n  us:  {}\n  bba: {}",
                plain[i], pd[i], b.deal, b.table_a, b.table_b
            );
        }
    }
}
