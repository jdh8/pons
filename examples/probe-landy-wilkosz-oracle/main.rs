//! Oracle at **responder's** seat over their Landy: a game-forcing Wilkosz
//! `1NT (2♣) 3♦`.
//!
//! §N1s's step-0 census (docs/archive/one-notrump-competitive-landy.md).  The
//! shipped counter leaves `3♦` idle; the idea spends it on two five-card suits,
//! at least one a major, no six-card suit, `points(10..)`.  Opener names a
//! three-card major (the longer, hearts on a tie), else `3NT`; responder
//! raises the right major and retreats to `3NT` over the wrong one, so opener
//! declares every contract.  Today those hands bid `3NT` or the values `X`
//! and the major never surfaces.
//!
//! ```text
//! cargo run --release --features serde --example probe-landy-wilkosz-oracle -- \
//!     ab-results/landy-splinter-tails-r2/tails-none \
//!     --dd-cache ab-results/landy-wilkosz-oracle/dd-cache-none.json
//! ```
//!
//! Candidates are priced as the contract reached if the auction stops there,
//! with no interference over `3♦` — their double and their raise are the A/B's
//! to price.  `best` is the per-board maximum of `3NT` and each `4M` opener
//! holds three of, an upper bound no bidding scheme reaches; `fit` adds
//! opener's `4` of the other major over responder's `3NT` retreat.

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::bidding::constraint::point_count;
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use std::collections::{BTreeMap, HashMap};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Dump, mean_with_ci, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Directory of `shard-*.json` from one arm (e.g. `.../tails-none`)
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

/// The Wilkosz shape: exactly two five-card suits, at least one a major, no
/// six-card suit, and `points(10..)`.  Returns responder's two suits.
fn wilkosz(hand: Hand) -> Option<(Suit, Suit)> {
    if point_count(hand) < 10 || Suit::ASC.iter().any(|&s| hand[s].len() > 5) {
        return None;
    }
    let fives: Vec<Suit> = Suit::ASC
        .into_iter()
        .filter(|&s| hand[s].len() == 5)
        .collect();
    match fives[..] {
        [low, high] if matches!(high, Suit::Hearts | Suit::Spades) => Some((low, high)),
        _ => None,
    }
}

struct Hit {
    index: usize,
    opener: Seat,
    /// Responder's two five-card suits, lower first
    pair: (Suit, Suit),
    /// Responder's live call over `(2♣)`
    live: Call,
}

/// `[passes] 1NT (2♣)` with us North/South and a Wilkosz hand in responder's seat
fn seat_hit(board: &Board, index: usize) -> Option<Hit> {
    let auction = &board.table_a;
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let opener = seat_to_act(board.dealer, open);
    if !matches!(opener, Seat::North | Seat::South)
        || *auction.get(open)? != Call::Bid(Bid::new(1, Strain::Notrump))
        || *auction.get(open + 1)? != Call::Bid(Bid::new(2, Strain::Clubs))
    {
        return None;
    }
    let pair = wilkosz(board.deal[opener.partner()])?;
    Some(Hit {
        index,
        opener,
        pair,
        live: *auction.get(open + 2)?,
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
            if let Some(hit) = seat_hit(&board, boards.len()) {
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

/// Opener's answer to `3♦`: a three-card major, the longer first, `ties` on a
/// tie; `None` = `3NT`
fn answer(opener: Hand, ties: Suit) -> Option<Suit> {
    let (h, s) = (opener[Suit::Hearts].len(), opener[Suit::Spades].len());
    match (h >= 3, s >= 3) {
        (false, false) => None,
        (true, false) => Some(Suit::Hearts),
        (false, true) => Some(Suit::Spades),
        (true, true) if h == s => Some(ties),
        (true, true) => Some(if h > s { Suit::Hearts } else { Suit::Spades }),
    }
}

/// The scheme's contract: `4M` when opener's major is one responder holds five
/// of, else `3NT` — opener declares either way
fn scheme(pair: (Suit, Suit), major: Option<Suit>) -> Strain {
    match major {
        Some(m) if pair.0 == m || pair.1 == m => Strain::from(m),
        _ => Strain::Notrump,
    }
}

fn game(strain: Strain) -> Contract {
    let level = if strain == Strain::Notrump { 3 } else { 4 };
    Contract {
        bid: Bid::new(level, strain),
        penalty: Penalty::Undoubled,
    }
}

fn suit_name(suit: Suit) -> char {
    match suit {
        Suit::Clubs => '♣',
        Suit::Diamonds => '♦',
        Suit::Hearts => '♥',
        Suit::Spades => '♠',
    }
}

type Cell = (Vec<i64>, Vec<i64>);

#[allow(clippy::cast_precision_loss)]
fn report(title: &str, rows: &BTreeMap<(String, String), Cell>, min: usize, scanned: usize) {
    println!("\n=== {title} ===");
    println!(
        "{:<24} {:<8} {:>7} {:>19} {:>19} {:>11} {:>11}",
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
        println!("{:-<24}", "");
        for (name, cell) in cands {
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            println!(
                "{bucket:<24} {name:<8} {:>7} {:>12.3} ±{:.3} {:>12.3} ±{:.3} {:>+11.5} {:>+11.5}",
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

    println!("=== Landy Wilkosz oracle — {} ===", args.dir);
    println!(
        "vulnerability {vul:?}; {} of {scanned} boards reach the seat ({:.3}%)",
        hits.len(),
        100.0 * hits.len() as f64 / scanned.max(1) as f64,
    );

    // --- live: responder's call × final contract -----------------------------
    let mut live_rows: BTreeMap<String, usize> = BTreeMap::new();
    for hit in &hits {
        let board = &boards[hit.index];
        let reached = final_contract(&board.table_a, board.dealer).map_or_else(
            || "pass".to_owned(),
            |(c, d)| {
                let side = if d == hit.opener {
                    "O"
                } else if d == hit.opener.partner() {
                    "R"
                } else {
                    "them"
                };
                format!("{c}@{side}")
            },
        );
        *live_rows
            .entry(format!("{:<4} -> {reached}", hit.live))
            .or_default() += 1;
    }
    println!("\n=== live: responder's call -> final contract (O opener, R responder) ===");
    let mut sorted: Vec<_> = live_rows.into_iter().collect();
    sorted.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (key, n) in sorted.iter().filter(|(_, n)| *n >= args.min.min(20)) {
        println!(
            "{key:<28} {n:>7} {:>5.1}%",
            100.0 * *n as f64 / hits.len().max(1) as f64
        );
    }

    // --- candidates ----------------------------------------------------------
    let mut rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut call_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    for hit in &hits {
        let board = &boards[hit.index];
        let table = &cache[&deal_key(&board.deal)];
        let reached = final_contract(&board.table_a, board.dealer);
        let (lp, ld) = (
            ns_score_contract(reached, table, vul),
            ns_score_pd(reached, table, vul),
        );
        let opener = board.deal[hit.opener];
        let responder = hit.opener.partner();
        let score = |c: Contract, by: Seat| {
            (
                ns_score_contract(Some((c, by)), table, vul),
                ns_score_pd(Some((c, by)), table, vul),
            )
        };

        let long = scheme(hit.pair, answer(opener, Suit::Hearts));
        let up = scheme(
            hit.pair,
            if opener[Suit::Hearts].len() >= 3 {
                Some(Suit::Hearts)
            } else {
                answer(opener, Suit::Spades)
            },
        );
        let fits: Vec<Strain> = [Suit::Hearts, Suit::Spades]
            .into_iter()
            .filter(|&m| (hit.pair.0 == m || hit.pair.1 == m) && opener[m].len() >= 3)
            .map(Strain::from)
            .collect();
        // Over responder's `3NT` retreat, opener bids `4` of the other major
        // with three of it: every 5-3 fit is reached, opener declaring.
        let fit = if long == Strain::Notrump {
            fits.first().copied().unwrap_or(Strain::Notrump)
        } else {
            long
        };
        let best = fits
            .iter()
            .chain(std::iter::once(&Strain::Notrump))
            .map(|&s| score(game(s), hit.opener))
            .max_by_key(|&(p, _)| p)
            .expect("3NT is always a candidate");

        let mut cands: Vec<(&str, (i64, i64))> = vec![
            ("scheme", score(game(long), hit.opener)),
            ("up", score(game(up), hit.opener)),
            ("fit", score(game(fit), hit.opener)),
            ("3NT", score(game(Strain::Notrump), hit.opener)),
            ("best", best),
            ("par", {
                let par = par_score(table, board.dealer, vul);
                (par, par)
            }),
        ];
        if long != Strain::Notrump {
            cands.push(("4M@R", score(game(long), responder)));
            cands.push((
                "6M",
                score(
                    Contract {
                        bid: Bid::new(6, long),
                        penalty: Penalty::Undoubled,
                    },
                    hit.opener,
                ),
            ));
        }

        // responder's pair × opener's length in responder's best-held major ×
        // whether opener holds three of the other major (the guess cell)
        let majors: Vec<Suit> = [Suit::Hearts, Suit::Spades]
            .into_iter()
            .filter(|&m| hit.pair.0 == m || hit.pair.1 == m)
            .collect();
        let fit = majors
            .iter()
            .map(|&m| opener[m].len())
            .max()
            .expect("at least one major");
        let fit = match fit {
            ..=2 => "f≤2",
            3 => "f3",
            _ => "f4+",
        };
        let guess = if majors.len() == 1 {
            let other = if majors[0] == Suit::Hearts {
                Suit::Spades
            } else {
                Suit::Hearts
            };
            if opener[other].len() >= 3 { " o3+" } else { "" }
        } else {
            ""
        };
        let pair = format!("{}{}", suit_name(hit.pair.1), suit_name(hit.pair.0));
        let keys = [
            "all".to_owned(),
            pair.clone(),
            format!("{pair} {fit}{guess}"),
        ];
        let live_key = match hit.live {
            Call::Bid(b) if b.strain == Strain::Notrump => format!("live {b}"),
            other => format!("live {other}"),
        };
        for (name, (p, d)) in cands {
            for key in &keys {
                let e = rows.entry((key.clone(), name.to_owned())).or_default();
                e.0.push(imps(p - lp));
                e.1.push(imps(d - ld));
            }
            let e = call_rows
                .entry((live_key.clone(), name.to_owned()))
                .or_default();
            e.0.push(imps(p - lp));
            e.1.push(imps(d - ld));
        }
    }
    report(
        "candidates vs live, by responder's pair and opener's fit",
        &rows,
        args.min,
        scanned,
    );
    report(
        "candidates vs live, by responder's live call",
        &call_rows,
        args.min,
        scanned,
    );
}
