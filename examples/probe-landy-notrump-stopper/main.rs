//! Oracle at **responder's** ungated `3NT`@168 over their Landy: `1NT (2♣) 3NT`.
//!
//! §N1r row 2's step-0 census (docs/one-notrump-competitive.md).  At `none` /
//! `both` the `3NT`@180 head needs both majors stopped, so every other direct
//! `3NT` is the ungated @168 rung — a game hand missing a stopper in a suit
//! they showed.  Before spending the idle `3♦` on "game values, stopper
//! trouble" (opener: `3NT` both stopped, `3♥`/`3♠` that major only, `4m`
//! neither), this prices, board by board, `3NT` by either hand and `5m` in the
//! best minor against what the live method reached and against par — cut by
//! responder's stoppers × opener's cover of the unstopped major(s), and by the
//! minor fit.  `scheme` is the `3♦` route itself (see [`scheme`]).
//!
//! ```text
//! cargo run --release --features serde --example probe-landy-notrump-stopper -- \
//!     ab-results/landy-nt-vs-x/base-none \
//!     --dd-cache ab-results/landy-notrump-stopper/dd-cache.json
//! ```
//!
//! Same caveats as `probe-landy-splinter-oracle`: every candidate is priced
//! as the contract reached if the auction stops there, and the `3♦` route's
//! own costs (their double of it, their room at the three level) are not
//! priced; the A/B remains the arbiter.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
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

struct Hit {
    index: usize,
    opener: Seat,
    /// Their advancer's call over our `3NT`
    advancer: Call,
}

/// `[passes] 1NT (2♣) 3NT` with us North/South
fn seat_hit(auction: &Auction, dealer: Seat, index: usize) -> Option<Hit> {
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let opener = seat_to_act(dealer, open);
    if !matches!(opener, Seat::North | Seat::South)
        || *auction.get(open)? != Call::Bid(Bid::new(1, Strain::Notrump))
        || *auction.get(open + 1)? != Call::Bid(Bid::new(2, Strain::Clubs))
        || *auction.get(open + 2)? != Call::Bid(Bid::new(3, Strain::Notrump))
    {
        return None;
    }
    Some(Hit {
        index,
        opener,
        advancer: *auction.get(open + 3)?,
    })
}

/// The `@168` boards, plus how many direct `3NT`s were the both-stopped head
#[allow(clippy::type_complexity)]
fn load_hits(
    dir: &str,
    limit: usize,
) -> (AbsoluteVulnerability, Vec<Board>, Vec<Hit>, usize, usize) {
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
    let mut head = 0usize;
    for shard in &shards {
        let dump: Dump = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(shard).unwrap_or_else(|e| panic!("open {}: {e}", shard.display())),
        ))
        .unwrap_or_else(|e| panic!("parse {}: {e}", shard.display()));
        vul = Some(dump.vulnerability);
        scanned += dump.boards.len();
        for board in dump.boards {
            if let Some(hit) = seat_hit(&board.table_a, board.dealer, boards.len()) {
                if stops(board.deal[hit.opener.partner()]) == [true, true] {
                    head += 1;
                    continue;
                }
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
    (
        vul.expect("at least one shard"),
        boards,
        hits,
        scanned,
        head,
    )
}

const MAJORS: [Suit; 2] = [Suit::Hearts, Suit::Spades];

/// Which majors `hand` stops, `[♥, ♠]`
fn stops(hand: Hand) -> [bool; 2] {
    MAJORS.map(|m| has_stopper(hand[m]))
}

/// The partnership's best minor: longer combined, clubs on a tie
fn best_minor(a: Hand, b: Hand) -> Suit {
    let len = |s: Suit| a[s].len() + b[s].len();
    if len(Suit::Diamonds) > len(Suit::Clubs) {
        Suit::Diamonds
    } else {
        Suit::Clubs
    }
}

/// The `3♦` route: opener `3NT` with both majors stopped, `3M` with that
/// major only (responder bids `3NT` holding the other, else places `5m`),
/// `4m` with neither, raised to `5m`.  Returns (notrump, opener declares);
/// every `3NT` is opener's — opener bid notrump first.
const fn scheme(opener: [bool; 2], responder: [bool; 2]) -> (bool, bool) {
    match opener {
        [true, true] => (true, true),
        [false, false] => (false, true),
        [true, false] => (responder[1], responder[1]),
        [false, true] => (responder[0], responder[0]),
    }
}

type Cell = (Vec<i64>, Vec<i64>);

#[allow(clippy::cast_precision_loss)]
fn report(title: &str, rows: &BTreeMap<(String, String), Cell>, min: usize, scanned: usize) {
    println!("\n=== {title} ===");
    println!(
        "{:<22} {:<8} {:>7} {:>19} {:>19} {:>11} {:>11}",
        "bucket", "cand", "boards", "plain IMPs vs live", "PD IMPs vs live", "plain /bd", "PD /bd"
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
        println!("{:-<22}", "");
        for (name, cell) in cands {
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            println!(
                "{bucket:<22} {name:<8} {:>7} {:>12.3} ±{:.3} {:>12.3} ±{:.3} {:>+11.5} {:>+11.5}",
                cell.0.len(),
                pm,
                pc,
                dm,
                dc,
                cell.0.iter().sum::<i64>() as f64 / scanned as f64,
                cell.1.iter().sum::<i64>() as f64 / scanned as f64,
            );
        }
    }
}

#[allow(clippy::cast_precision_loss, clippy::too_many_lines)]
fn main() {
    let args = Args::parse();
    let (dump_vul, boards, hits, scanned, head) = load_hits(&args.dir, args.limit);
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

    println!("=== Landy 3NT@168 stopper oracle — {} ===", args.dir);
    println!(
        "vulnerability {vul:?}; of {scanned} boards, {head} bid the both-stopped \
         3NT@180 head and {} the 3NT@168 ({:.2}%), advancer passing on {:.1}%",
        hits.len(),
        100.0 * hits.len() as f64 / scanned.max(1) as f64,
        100.0 * hits.iter().filter(|h| h.advancer == Call::Pass).count() as f64
            / hits.len().max(1) as f64,
    );
    println!("/bd columns are IMPs per scanned board");

    let mut cover_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut fit_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut live_rows: BTreeMap<String, usize> = BTreeMap::new();
    for hit in &hits {
        let board = &boards[hit.index];
        let (o, r) = (hit.opener, hit.opener.partner());
        let (oh, rh) = (board.deal[o], board.deal[r]);
        let (os, rs) = (stops(oh), stops(rh));
        let table = &cache[&deal_key(&board.deal)];
        let reached = final_contract(&board.table_a, board.dealer);
        let (lp, ld) = (
            ns_score_contract(reached, table, vul),
            ns_score_pd(reached, table, vul),
        );
        *live_rows
            .entry(reached.map_or("pass".to_owned(), |(c, d)| {
                format!(
                    "{c}@{}",
                    if d == r {
                        "R"
                    } else if d == o {
                        "O"
                    } else {
                        "them"
                    }
                )
            }))
            .or_default() += 1;

        let resp = match rs {
            [false, false] => "r:none",
            [true, false] => "r:h-only",
            _ => "r:s-only",
        };
        let need: Vec<usize> = (0..2).filter(|&i| !rs[i]).collect();
        let covered = need.iter().filter(|&&i| os[i]).count();
        let cover = if covered == need.len() {
            "o:cover"
        } else if covered > 0 {
            "o:part"
        } else {
            "o:bare"
        };
        let minor = best_minor(oh, rh);
        let fit_len = oh[minor].len() + rh[minor].len();
        let fit = if fit_len >= 8 { "fit8+" } else { "fit7-" };

        let price = |level: u8, strain: Strain, declarer: Seat| {
            let c = Contract {
                bid: Bid::new(level, strain),
                penalty: Penalty::Undoubled,
            };
            (
                imps(ns_score_contract(Some((c, declarer)), table, vul) - lp),
                imps(ns_score_pd(Some((c, declarer)), table, vul) - ld),
            )
        };
        let m = Strain::from(minor);
        let (s_nt, s_opener) = scheme(os, rs);
        let (s_level, s_strain) = if s_nt { (3, Strain::Notrump) } else { (5, m) };
        let par = par_score(table, board.dealer, vul);
        let cands = [
            ("3NT", price(3, Strain::Notrump, o)),
            ("5m@O", price(5, m, o)),
            ("5m@R", price(5, m, r)),
            (
                "scheme",
                price(s_level, s_strain, if s_opener { o } else { r }),
            ),
            ("par", (imps(par - lp), imps(par - ld))),
        ];
        for (rows, key) in [
            (&mut cover_rows, format!("{resp} {cover}")),
            (&mut fit_rows, format!("{resp} {fit}")),
        ] {
            for (name, (p, d)) in cands {
                let entry = rows.entry(("all".to_owned(), name.to_owned())).or_default();
                entry.0.push(p);
                entry.1.push(d);
            }
            for (name, (p, d)) in cands {
                let entry = rows.entry((key.clone(), name.to_owned())).or_default();
                entry.0.push(p);
                entry.1.push(d);
            }
        }
    }

    println!("\n=== live final contract after the @168 3NT (R responder, O opener) ===");
    let mut live: Vec<_> = live_rows.into_iter().collect();
    live.sort_by_key(|a| std::cmp::Reverse(a.1));
    for (contract, n) in live.iter().take(12) {
        println!("{contract:<12} {n:>7}");
    }
    report(
        "responder's stoppers × opener's cover of the unstopped major(s)",
        &cover_rows,
        args.min,
        scanned,
    );
    report(
        "responder's stoppers × best minor fit",
        &fit_rows,
        args.min,
        scanned,
    );
}
