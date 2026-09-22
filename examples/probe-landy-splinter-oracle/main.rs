//! Oracle at **opener's** seat over our Landy splinter: `1NT (2♣) 3♥/3♠`.
//!
//! §N1r row 1's step-0 census (docs/one-notrump-competitive.md).  Responder
//! has shown four-plus in each minor, a game force and 0-1 in the bid major;
//! opener answers `3NT`@150 on *any* stopper in it (or no four-card minor),
//! else `4m`.  Before gating that `3NT` on wastage or authoring the fit path,
//! this prices, board by board, `3NT` / `4m` / `5m` / `6m` in opener's better
//! minor against what the live method reached and against par — cut by
//! opener's holding in the short major (no stopper / one honour / wasted) ×
//! whether opener has a four-card minor, and by responder's strength.
//!
//! ```text
//! cargo run --release --features serde --example probe-landy-splinter-oracle -- \
//!     ab-results/landy-nt-vs-x/base-none \
//!     --dd-cache ab-results/landy-splinter-oracle/dd-cache.json
//! ```
//!
//! Same caveats as `probe-landy-opener-oracle`: every candidate is priced as
//! the contract reached if the auction stops there, so this orders rungs on
//! the same boards; the A/B remains the arbiter.  PD is the sharper scorer
//! here — every candidate is a declaring rung.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Holding, Penalty, Rank, Seat, Strain,
    Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use std::collections::{BTreeMap, HashMap};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, mean_with_ci, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Directory of `shard-*.json` from one arm (e.g. `.../base-none`)
    dir: String,
    /// Re-price at this vulnerability instead of the dump's
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// A deal-keyed DD table cache, created if absent and written back
    #[arg(long)]
    dd_cache: Option<String>,
    /// Stop after this many *seat* boards (0 = all) — a quick smoke cut
    #[arg(long, default_value_t = 0)]
    limit: usize,
    /// Fold bucket rows with fewer than this many boards
    #[arg(long, default_value_t = 100)]
    min: usize,
}

fn deal_key(deal: &FullDeal) -> String {
    serde_json::to_string(deal).expect("a deal serializes")
}

/// Mirrors `bidding::constraint::has_stopper` (A / Kx / Qxx / Jxxx).
const fn has_stopper(holding: Holding) -> bool {
    holding.contains(Rank::A)
        || (holding.contains(Rank::K) && holding.len() >= 2)
        || (holding.contains(Rank::Q) && holding.len() >= 3)
        || (holding.contains(Rank::J) && holding.len() >= 4)
}

/// Opener's holding in the short major, as the census cuts it
fn stopper_class(holding: Holding) -> &'static str {
    let top = [Rank::A, Rank::K, Rank::Q]
        .iter()
        .filter(|&&r| holding.contains(r))
        .count();
    if !has_stopper(holding) {
        "nost"
    } else if top >= 2 {
        "wasted"
    } else {
        "stop1"
    }
}

fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

struct Hit {
    index: usize,
    short: Suit,
    opener: Seat,
    /// Their advancer's call over the splinter
    advancer: Call,
    /// Opener's live answer (only meaningful when `advancer` is a pass)
    answer: Option<Call>,
}

/// `[passes] 1NT (2♣) 3♥/3♠` with us North/South
fn seat_hit(auction: &Auction, dealer: Seat, index: usize) -> Option<Hit> {
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let opener = seat_to_act(dealer, open);
    if !matches!(opener, Seat::North | Seat::South) {
        return None;
    }
    if *auction.get(open)? != Call::Bid(Bid::new(1, Strain::Notrump))
        || *auction.get(open + 1)? != Call::Bid(Bid::new(2, Strain::Clubs))
    {
        return None;
    }
    let short = match *auction.get(open + 2)? {
        Call::Bid(b) if b.level.get() == 3 && b.strain == Strain::Hearts => Suit::Hearts,
        Call::Bid(b) if b.level.get() == 3 && b.strain == Strain::Spades => Suit::Spades,
        _ => return None,
    };
    let advancer = *auction.get(open + 3)?;
    let answer = (advancer == Call::Pass)
        .then(|| auction.get(open + 4).copied())
        .flatten();
    Some(Hit {
        index,
        short,
        opener,
        advancer,
        answer,
    })
}

fn load_hits(dir: &str, limit: usize) -> (AbsoluteVulnerability, Vec<Board>, Vec<Hit>, usize) {
    let mut shards: Vec<std::path::PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read dir {dir}: {e}"))
        .map(|entry| entry.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("shard-") && n.ends_with(".json"))
        })
        .collect();
    assert!(!shards.is_empty(), "no shard-*.json in {dir}");
    shards.sort();
    let mut vul = None;
    let mut boards = Vec::new();
    let mut hits = Vec::new();
    let mut scanned = 0usize;
    for shard in &shards {
        let dump: Dump = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(shard).unwrap_or_else(|e| panic!("open {}: {e}", shard.display())),
        ))
        .unwrap_or_else(|e| panic!("parse {}: {e}", shard.display()));
        vul = Some(dump.vulnerability);
        scanned += dump.boards.len();
        for board in dump.boards {
            if let Some(hit) = seat_hit(&board.table_a, board.dealer, boards.len()) {
                boards.push(board);
                hits.push(hit);
            }
        }
        if limit > 0 && hits.len() >= limit {
            hits.truncate(limit);
            boards.truncate(limit);
            break;
        }
    }
    (vul.expect("at least one shard"), boards, hits, scanned)
}

fn solve_missing(cache: &mut HashMap<String, TrickCountTable>, boards: &[Board]) {
    let missing: Vec<usize> = (0..boards.len())
        .filter(|&i| !cache.contains_key(&deal_key(&boards[i].deal)))
        .collect();
    eprintln!(
        "dd cache: {}/{} hit, {} to solve",
        boards.len() - missing.len(),
        boards.len(),
        missing.len()
    );
    for (done, chunk) in missing.chunks(4096).enumerate() {
        let deals: Vec<FullDeal> = chunk.iter().map(|&i| boards[i].deal).collect();
        let solved = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);
        for (&i, table) in chunk.iter().zip(solved) {
            cache.insert(deal_key(&boards[i].deal), table);
        }
        eprintln!("  solved {} / {}", (done + 1) * 4096, missing.len());
    }
}

#[allow(clippy::cast_possible_truncation)]
fn par_score(table: &TrickCountTable, dealer: Seat, vul: AbsoluteVulnerability) -> i64 {
    pons::stats::average_ns_par(std::iter::once(*table).collect(), vul, dealer)
        .map_or(0, |par| par.score.round() as i64)
}

/// Opener's better minor: the longer, clubs on a tie.  Responder has 4+ in
/// both, so combined length follows opener's.
fn better_minor(hand: Hand) -> Suit {
    if hand[Suit::Diamonds].len() > hand[Suit::Clubs].len() {
        Suit::Diamonds
    } else {
        Suit::Clubs
    }
}

type Cell = (Vec<i64>, Vec<i64>);
/// A live cell: live plain, live PD, then `3NT`@opener minus live, plain and PD
type LiveCell = (Vec<i64>, Vec<i64>, Vec<i64>, Vec<i64>);

#[allow(clippy::cast_precision_loss)]
fn report(title: &str, note: &str, rows: &BTreeMap<(String, String), Cell>, min: usize) {
    println!("\n=== {title} ===");
    println!("{note}");
    println!(
        "{:<28} {:<8} {:>7} {:>19} {:>19}",
        "bucket", "cand", "boards", "plain IMPs vs live", "PD IMPs vs live"
    );
    let mut buckets: BTreeMap<&str, Vec<(&str, &Cell)>> = BTreeMap::new();
    for ((bucket, cand), cell) in rows {
        buckets
            .entry(bucket.as_str())
            .or_default()
            .push((cand.as_str(), cell));
    }
    for (bucket, mut cands) in buckets {
        if cands.first().is_some_and(|(_, c)| c.0.len() < min) {
            continue;
        }
        cands.sort_by(|a, b| {
            let mean = |c: &Cell| c.0.iter().sum::<i64>() as f64 / c.0.len().max(1) as f64;
            mean(b.1).partial_cmp(&mean(a.1)).expect("finite means")
        });
        println!("{:-<28}", "");
        for (name, cell) in cands {
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            println!(
                "{bucket:<28} {name:<8} {:>7} {:>12.3} ±{:.3} {:>12.3} ±{:.3}",
                cell.0.len(),
                pm,
                pc,
                dm,
                dc
            );
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() {
    let args = Args::parse();
    let (dump_vul, boards, hits, scanned) = load_hits(&args.dir, args.limit);
    let vul = args.vulnerability.unwrap_or(dump_vul);

    let mut cache: HashMap<String, TrickCountTable> = match args.dd_cache.as_deref() {
        Some(path) if std::path::Path::new(path).exists() => serde_json::from_reader(
            std::io::BufReader::new(std::fs::File::open(path).expect("open dd cache")),
        )
        .expect("parse dd cache"),
        _ => HashMap::new(),
    };
    solve_missing(&mut cache, &boards);
    if let Some(path) = args.dd_cache.as_deref() {
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(path).expect("create dd cache")),
            &cache,
        )
        .expect("write dd cache");
        eprintln!("dd cache {path} now holds {} tables", cache.len());
    }

    println!("=== Landy splinter oracle — {} ===", args.dir);
    println!(
        "vulnerability {vul:?}; {} of {scanned} boards reach the seat ({:.2}%)",
        hits.len(),
        100.0 * hits.len() as f64 / scanned.max(1) as f64,
    );
    let quiet = hits.iter().filter(|h| h.advancer == Call::Pass).count();
    println!(
        "advancer passed on {quiet} ({:.1}%); short ♥ {} / ♠ {}",
        100.0 * quiet as f64 / hits.len().max(1) as f64,
        hits.iter().filter(|h| h.short == Suit::Hearts).count(),
        hits.iter().filter(|h| h.short == Suit::Spades).count(),
    );

    let live: Vec<(i64, i64)> = hits
        .iter()
        .map(|hit| {
            let board = &boards[hit.index];
            let table = &cache[&deal_key(&board.deal)];
            let reached = final_contract(&board.table_a, board.dealer);
            (
                ns_score_contract(reached, table, vul),
                ns_score_pd(reached, table, vul),
            )
        })
        .collect();

    // --- opener's live answer × final contract, per stopper class ------------
    {
        // (live plain, live PD, `3NT`@opener − live plain, … PD)
        let mut calls: BTreeMap<String, LiveCell> = BTreeMap::new();
        for (hit, &(lp, ld)) in hits.iter().zip(&live) {
            let board = &boards[hit.index];
            let table = &cache[&deal_key(&board.deal)];
            let held = board.deal[hit.opener][hit.short];
            let nt = Contract {
                bid: Bid::new(3, Strain::Notrump),
                penalty: Penalty::Undoubled,
            };
            let nt_plain = ns_score_contract(Some((nt, hit.opener)), table, vul);
            let nt_pd = ns_score_pd(Some((nt, hit.opener)), table, vul);
            let reached = final_contract(&board.table_a, board.dealer)
                .map_or("pass".to_owned(), |(c, d)| format!("{c}@{d:?}"));
            let key = format!(
                "{:<6} {:<5} -> {}",
                stopper_class(held),
                hit.answer.map_or("(adv)".to_owned(), |c| c.to_string()),
                reached
            );
            let entry = calls.entry(key).or_default();
            entry.0.push(lp);
            entry.1.push(ld);
            entry.2.push(imps(nt_plain - lp));
            entry.3.push(imps(nt_pd - ld));
        }
        println!(
            "\n=== live: stopper class, opener's answer, final contract (rows ≥ {}) ===",
            args.min
        );
        println!(
            "{:<40} {:>7} {:>6} {:>16} {:>16} {:>15} {:>15}",
            "class answer -> reached",
            "boards",
            "share",
            "plain score/bd",
            "PD score/bd",
            "3NT-live plain",
            "3NT-live PD"
        );
        for (key, cell) in &calls {
            if cell.0.len() < args.min {
                continue;
            }
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            let (np, _) = mean_with_ci(&cell.2);
            let (nd, _) = mean_with_ci(&cell.3);
            println!(
                "{key:<40} {:>7} {:>5.1}% {:>9.1} ±{:.1} {:>9.1} ±{:.1} {:>+8.2} ({:>+7.0}) {:>+8.2} ({:>+7.0})",
                cell.0.len(),
                100.0 * cell.0.len() as f64 / hits.len().max(1) as f64,
                pm,
                pc,
                dm,
                dc,
                np,
                cell.2.iter().sum::<i64>(),
                nd,
                cell.3.iter().sum::<i64>(),
            );
        }
    }

    // --- the design cuts -----------------------------------------------------
    let mut fit_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut hcp_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    for (hit, &(lp, ld)) in hits.iter().zip(&live) {
        let board = &boards[hit.index];
        let table = &cache[&deal_key(&board.deal)];
        let hand = board.deal[hit.opener];
        let class = stopper_class(hand[hit.short]);
        let minor = better_minor(hand);
        let fit = if hand[minor].len() >= 4 {
            "fit"
        } else {
            "nofit"
        };
        let resp = hand_hcp(board.deal[hit.opener.partner()]);
        let band = match resp {
            ..=11 => "r10-11",
            12..=14 => "r12-14",
            _ => "r15+",
        };
        let cands = [
            ("3NT", 3, Strain::Notrump),
            ("4m", 4, Strain::from(minor)),
            ("5m", 5, Strain::from(minor)),
            ("6m", 6, Strain::from(minor)),
        ];
        let par = par_score(table, board.dealer, vul);
        for (rows, key) in [
            (&mut fit_rows, format!("{class} {fit}")),
            (&mut hcp_rows, format!("{class} {band}")),
        ] {
            for (name, level, strain) in cands {
                let c = Contract {
                    bid: Bid::new(level, strain),
                    penalty: Penalty::Undoubled,
                };
                let plain = ns_score_contract(Some((c, hit.opener)), table, vul);
                let pd = ns_score_pd(Some((c, hit.opener)), table, vul);
                let entry = rows.entry((key.clone(), name.to_owned())).or_default();
                entry.0.push(imps(plain - lp));
                entry.1.push(imps(pd - ld));
            }
            let entry = rows.entry((key.clone(), "par".to_owned())).or_default();
            entry.0.push(imps(par - lp));
            entry.1.push(imps(par - ld));
        }
    }
    report(
        "stopper class × opener's four-card minor",
        "Candidates declared by opener in the better minor; `live` is the reference.",
        &fit_rows,
        args.min,
    );
    report(
        "stopper class × responder's HCP",
        "Same candidates, cut by the splinter hand's strength.",
        &hcp_rows,
        args.min,
    );
}
