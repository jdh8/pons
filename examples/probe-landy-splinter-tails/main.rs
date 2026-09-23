//! Oracle at the two floor tails over our passed Landy-splinter `3NT`.
//!
//! §N1r row 1 arm 1 (docs/one-notrump-competitive.md) has responder pass
//! opener's `3NT` answer to the splinter (`1NT (2♣) 3♥/3♠ - 3NT - -`), which
//! hands the opponents' last seat two floor-owned tails:
//!
//! - **`(X)`** — the pass-out double of `3NT`.  Candidates: sit (`3NTx`),
//!   redouble (`3NTxx`), run to `5m`.
//! - **`(4M)`** — a balancing four of a major.  Candidates: defend `4M`
//!   undoubled or doubled, bid `5m`.
//!
//! `m` is our best combined minor (clubs on a tie), declared by responder —
//! the first of us to name it at the five level in either seat's run.  Every
//! candidate is priced as the final contract against what the live method
//! reached (plain DD and PD), cut by responder's length in the splintered
//! major, whether their `4M` is that suit, and our combined HCP.
//!
//! ```text
//! cargo run --release --features serde --example probe-landy-splinter-tails -- \
//!     ab-results/landy-splinter-rebids/rebids-both \
//!     --dd-cache ab-results/landy-splinter-oracle/dd-cache.json
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use std::collections::{BTreeMap, HashMap};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Dump, mean_with_ci, seat_to_act};

#[derive(Parser)]
struct Args {
    /// Directory of `shard-*.json` from one arm
    dir: String,
    /// A deal-keyed DD table cache, created if absent and written back
    #[arg(long)]
    dd_cache: Option<String>,
    /// Fold rows with fewer than this many boards
    #[arg(long, default_value_t = 50)]
    min: usize,
}

fn deal_key(deal: &FullDeal) -> String {
    serde_json::to_string(deal).expect("a deal serializes")
}

fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

struct Hit {
    index: usize,
    short: Suit,
    opener: Seat,
    /// Their call in the pass-out seat: `X` or a four-level major
    tail: Call,
    /// Who made it
    by: Seat,
    /// Our live calls after it, space-joined
    rest: String,
}

/// `[passes] 1NT (2♣) 3M - 3NT - - (X|4M)` with us North/South
fn seat_hit(auction: &Auction, dealer: Seat, index: usize) -> Option<Hit> {
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let opener = seat_to_act(dealer, open);
    if !matches!(opener, Seat::North | Seat::South) {
        return None;
    }
    let calls: Vec<Call> = auction.iter().skip(open).copied().collect();
    let bid = |level, strain| Call::Bid(Bid::new(level, strain));
    if calls.len() < 8
        || calls[0] != bid(1, Strain::Notrump)
        || calls[1] != bid(2, Strain::Clubs)
        || calls[3] != Call::Pass
        || calls[4] != bid(3, Strain::Notrump)
        || calls[5] != Call::Pass
        || calls[6] != Call::Pass
    {
        return None;
    }
    let short = match calls[2] {
        Call::Bid(b) if b == Bid::new(3, Strain::Hearts) => Suit::Hearts,
        Call::Bid(b) if b == Bid::new(3, Strain::Spades) => Suit::Spades,
        _ => return None,
    };
    let tail = calls[7];
    let keep = match tail {
        Call::Double => true,
        Call::Bid(b) => b.level.get() == 4 && matches!(b.strain, Strain::Hearts | Strain::Spades),
        _ => false,
    };
    keep.then(|| Hit {
        index,
        short,
        opener,
        tail,
        by: seat_to_act(dealer, open + 7),
        rest: calls[8..]
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" "),
    })
}

fn best_minor(a: Hand, b: Hand) -> Suit {
    let fit = |s: Suit| a[s].len() + b[s].len();
    if fit(Suit::Diamonds) > fit(Suit::Clubs) {
        Suit::Diamonds
    } else {
        Suit::Clubs
    }
}

/// (plain, PD) per candidate
type Cell = (Vec<i64>, Vec<i64>);

#[allow(clippy::cast_precision_loss)]
fn report(title: &str, rows: &BTreeMap<(String, String), Cell>, min: usize) {
    println!("\n=== {title} ===");
    println!(
        "{:<34} {:<6} {:>6} {:>19} {:>19}",
        "bucket", "cand", "boards", "plain IMPs vs live", "PD IMPs vs live"
    );
    let mut buckets: BTreeMap<&str, Vec<(&str, &Cell)>> = BTreeMap::new();
    for ((bucket, cand), cell) in rows {
        buckets.entry(bucket).or_default().push((cand, cell));
    }
    for (bucket, cands) in buckets {
        if cands.first().is_some_and(|(_, c)| c.0.len() < min) {
            continue;
        }
        println!("{:-<34}", "");
        for (name, cell) in cands {
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            println!(
                "{bucket:<34} {name:<6} {:>6} {:>12.3} ±{:.3} {:>12.3} ±{:.3}",
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
    let mut shards: Vec<_> = std::fs::read_dir(&args.dir)
        .unwrap_or_else(|e| panic!("read dir {}: {e}", args.dir))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("shard-") && n.ends_with(".json"))
        })
        .collect();
    shards.sort();
    let mut vul = None;
    let mut boards = Vec::new();
    let mut hits = Vec::new();
    for shard in &shards {
        let dump: Dump = serde_json::from_reader(std::io::BufReader::new(
            std::fs::File::open(shard).expect("open shard"),
        ))
        .expect("parse shard");
        vul = Some(dump.vulnerability);
        for board in dump.boards {
            if let Some(hit) = seat_hit(&board.table_a, board.dealer, boards.len()) {
                boards.push(board);
                hits.push(hit);
            }
        }
    }
    let vul: AbsoluteVulnerability = vul.expect("at least one shard");

    let mut cache: HashMap<String, TrickCountTable> = match args.dd_cache.as_deref() {
        Some(path) if std::path::Path::new(path).exists() => serde_json::from_reader(
            std::io::BufReader::new(std::fs::File::open(path).expect("open dd cache")),
        )
        .expect("parse dd cache"),
        _ => HashMap::new(),
    };
    let missing: Vec<FullDeal> = boards
        .iter()
        .map(|b| b.deal)
        .filter(|d| !cache.contains_key(&deal_key(d)))
        .collect();
    eprintln!("{} hits, {} to solve", boards.len(), missing.len());
    for chunk in missing.chunks(4096) {
        let solved = Solver::lock(None).solve_deals(chunk, NonEmptyStrainFlags::ALL);
        for (deal, table) in chunk.iter().zip(solved) {
            cache.insert(deal_key(deal), table);
        }
    }
    if let (Some(path), false) = (args.dd_cache.as_deref(), missing.is_empty()) {
        serde_json::to_writer(
            std::io::BufWriter::new(std::fs::File::create(path).expect("create dd cache")),
            &cache,
        )
        .expect("write dd cache");
    }

    println!("=== Landy splinter tails — {} ({vul:?}) ===", args.dir);
    let mut live_rows: BTreeMap<String, Cell> = BTreeMap::new();
    let mut rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    for hit in &hits {
        let board = &boards[hit.index];
        let table = &cache[&deal_key(&board.deal)];
        let reached = final_contract(&board.table_a, board.dealer);
        let (lp, ld) = (
            ns_score_contract(reached, table, vul),
            ns_score_pd(reached, table, vul),
        );
        let responder = hit.opener.partner();
        let (o, r) = (board.deal[hit.opener], board.deal[responder]);
        let minor = best_minor(o, r);
        let hcp = hand_hcp(o) + hand_hcp(r);
        let band = match hcp {
            ..=24 => "ns..24",
            25..=27 => "ns25-27",
            _ => "ns28+",
        };
        let short_len = r[hit.short].len();
        let tail = match hit.tail {
            Call::Double => "(X)".to_owned(),
            Call::Bid(b) if Suit::try_from(b.strain).ok() == Some(hit.short) => {
                format!("({b})=short")
            }
            Call::Bid(b) => format!("({b})=other"),
            _ => unreachable!(),
        };
        let reached_str = reached.map_or("pass".to_owned(), |(c, d)| {
            let side = if matches!(d, Seat::North | Seat::South) {
                "us"
            } else {
                "them"
            };
            format!("{c}@{side}")
        });
        let cell = live_rows
            .entry(format!(
                "{tail:<13} {:<16} -> {reached_str}",
                hit.rest.chars().take(16).collect::<String>()
            ))
            .or_default();
        cell.0.push(lp);
        cell.1.push(ld);

        let contract = |level, strain, penalty| Contract {
            bid: Bid::new(level, strain),
            penalty,
        };
        let mut cands: Vec<(&str, Option<(Contract, Seat)>)> = vec![(
            "5m",
            Some((
                contract(5, Strain::from(minor), Penalty::Undoubled),
                responder,
            )),
        )];
        match hit.tail {
            Call::Double => {
                let nt = |p| Some((contract(3, Strain::Notrump, p), hit.opener));
                cands.push(("3NTx", nt(Penalty::Doubled)));
                cands.push(("3NTxx", nt(Penalty::Redoubled)));
                // Responder's own longer minor, clubs on a tie: what it can
                // actually bid without knowing opener's minors.
                let own = if r[Suit::Diamonds].len() > r[Suit::Clubs].len() {
                    Suit::Diamonds
                } else {
                    Suit::Clubs
                };
                cands.push((
                    "5r",
                    Some((
                        contract(5, Strain::from(own), Penalty::Undoubled),
                        responder,
                    )),
                ));
                cands.push((
                    "5mx",
                    Some((
                        contract(5, Strain::from(minor), Penalty::Doubled),
                        responder,
                    )),
                ));
            }
            Call::Bid(b) => {
                cands.push((
                    "4M",
                    Some((contract(4, b.strain, Penalty::Undoubled), hit.by)),
                ));
                cands.push((
                    "4Mx",
                    Some((contract(4, b.strain, Penalty::Doubled), hit.by)),
                ));
            }
            _ => unreachable!(),
        }
        let family = if hit.tail == Call::Double {
            "(X)"
        } else {
            &tail[..]
        };
        for key in [
            format!("{family} all"),
            format!("{family} short{short_len}"),
            format!("{family} {band}"),
        ] {
            for &(name, c) in &cands {
                let entry = rows.entry((key.clone(), name.to_owned())).or_default();
                entry.0.push(imps(ns_score_contract(c, table, vul) - lp));
                entry.1.push(imps(ns_score_pd(c, table, vul) - ld));
            }
        }
    }

    println!(
        "\n=== live: tail, our calls after it, final contract (rows ≥ {}) ===",
        args.min
    );
    println!(
        "{:<52} {:>6} {:>16} {:>16}",
        "tail rest -> reached", "boards", "plain score/bd", "PD score/bd"
    );
    for (key, cell) in &live_rows {
        if cell.0.len() < args.min {
            continue;
        }
        let (pm, pc) = mean_with_ci(&cell.0);
        let (dm, dc) = mean_with_ci(&cell.1);
        println!(
            "{key:<52} {:>6} {:>9.1} ±{:.1} {:>9.1} ±{:.1}",
            cell.0.len(),
            pm,
            pc,
            dm,
            dc
        );
    }
    report("candidates vs live", &rows, args.min);
}
