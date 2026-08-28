//! Oracle at **opener's** seat in the Landy lane: `1NT (2♣) X (2♥/2♠)`.
//!
//! The seat §N1k authored a `3NT` at, lost, and gave back to the floor
//! (docs/one-notrump-competitive.md).  Before authoring it a second time this
//! prices, board by board, every contract opener could steer to from here —
//! defend their major undoubled, defend it **doubled**, declare `2NT`/`3NT`
//! from either side of our partnership, bid a natural `3m` — against what our
//! live method actually reaches, and against par.  The buckets are the design
//! question stated as a cut: opener's length in *their* major × a stopper in it
//! × opener's HCP.
//!
//! ```text
//! cargo run --release --features serde --example probe-landy-opener-oracle -- \
//!     ab-results/landy-doubler-rebids/base-none \
//!     --dd-cache ab-results/landy-doubler-rebids/dd-cache.json
//! ```
//!
//! **What this is and is not.**  Every candidate is priced as *the contract
//! opener's call leads to if the auction stops there* — the oracle prices
//! contracts, not auctions, so it cannot see partner pulling, their advancer
//! running, or the extra information a bid leaks.  It is an upper bound on
//! each rung's value and a reliable *ordering* between rungs on the same
//! boards; the A/B remains the arbiter.  The reference for every IMP column is
//! `live` — the contract the base arm actually reached — so a positive mean is
//! "this rung would have beaten today's floor on these boards", which is the
//! quantity Experiment 2 is trying to buy.
//!
//! **Read the doubling rungs on the plain column only.**  Perfect defense
//! doubles any failing undoubled contract, so it prices `2♥` and `2♥x` almost
//! identically and is structurally blind to what a penalty double buys
//! (docs/measurement.md's domain addendum).  The PD column is printed for the
//! *declaring* rungs, where it is the sharper scorer.
//!
//! Boards come from an existing arm dump: only the ~2% that reach the seat are
//! kept, so the whole 4.6M-board arm streams through in bounded memory and the
//! DD fan-out is over the seat's boards alone.

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
    /// Fold bucket rows with fewer than this many boards into `other`
    #[arg(long, default_value_t = 200)]
    min: usize,
    /// Dump this many worst-`live` boards per named candidate, e.g. `X`
    #[arg(long, default_value_t = 0)]
    show: usize,
    /// Which candidate `--show` ranks by
    #[arg(long, default_value = "2Mx")]
    candidate: String,
}

/// A deal's cache key: its serde string, as `bba-decompose` writes it
fn deal_key(deal: &FullDeal) -> String {
    serde_json::to_string(deal).expect("a deal serializes")
}

/// Whether a holding stops the suit for notrump purposes
///
/// Mirrors `bidding::constraint::has_stopper`, which is `pub(crate)`: the crisp
/// textbook definition, A / Kx / Qxx / Jxxx.  Kept in step by hand — a probe
/// that bucketed on a *different* stopper than the rule it is designing would
/// hand back a boundary the rule cannot express.
const fn has_stopper(holding: Holding) -> bool {
    holding.contains(Rank::A)
        || (holding.contains(Rank::K) && holding.len() >= 2)
        || (holding.contains(Rank::Q) && holding.len() >= 3)
        || (holding.contains(Rank::J) && holding.len() >= 4)
}

/// Total HCP of a hand
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// Which leg of the lane reached opener's seat
#[derive(Clone, Copy, PartialEq, Eq)]
enum Leg {
    /// `1NT (2♣) X (2♥/2♠)` — their advancer named the major, opener acts with
    /// partner's double still live behind them
    Direct,
    /// `1NT (2♣) X (2♦) - (2♥/2♠) - -` — the artificial relay was corrected,
    /// both partner and their advancer have passed, so opener is *balancing*
    Relay,
}

impl Leg {
    const fn name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Relay => "relay",
        }
    }
}

/// One board that reaches opener's seat
struct Hit {
    index: usize,
    leg: Leg,
    /// The major their side named
    major: Suit,
    opener: Seat,
    /// Their seat that declares the major (whoever bid it first)
    their_declarer: Seat,
    /// Opener's call at the seat, in the arm as generated
    live_call: Call,
}

/// Classify one table-A auction: does it reach opener's seat over their Landy
/// `2♣` and our values double, and on which leg?
fn seat_hit(auction: &Auction, dealer: Seat, index: usize) -> Option<Hit> {
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let opener = seat_to_act(dealer, open);
    if !matches!(opener, Seat::North | Seat::South) {
        return None;
    }
    let Call::Bid(bid) = *auction.get(open)? else {
        return None;
    };
    if bid != Bid::new(1, Strain::Notrump) {
        return None;
    }
    if *auction.get(open + 1)? != Call::Bid(Bid::new(2, Strain::Clubs))
        || *auction.get(open + 2)? != Call::Double
    {
        return None;
    }
    let major_of = |call: Option<&Call>| match call {
        Some(&Call::Bid(b)) if b.level.get() == 2 => match b.strain {
            Strain::Hearts => Some(Suit::Hearts),
            Strain::Spades => Some(Suit::Spades),
            _ => None,
        },
        _ => None,
    };
    // The direct leg: their advancer named the major and opener is next.
    let (leg, major, at) = if let Some(major) = major_of(auction.get(open + 3)) {
        (Leg::Direct, major, open + 4)
    } else if *auction.get(open + 3)? == Call::Bid(Bid::new(2, Strain::Diamonds))
        && *auction.get(open + 4)? == Call::Pass
        && auction.get(open + 6) == Some(&Call::Pass)
        && auction.get(open + 7) == Some(&Call::Pass)
        && let Some(major) = major_of(auction.get(open + 5))
    {
        // The relay leg: opener passed the artificial `2♦`, their overcaller
        // corrected to a major, and it came back round.
        (Leg::Relay, major, open + 8)
    } else {
        return None;
    };
    // Whoever of their two seats bid the major first declares it.
    let their_declarer = (open..at)
        .find(|&i| matches!(auction[i], Call::Bid(b) if b.strain == Strain::from(major)))
        .map(|i| seat_to_act(dealer, i))?;
    Some(Hit {
        index,
        leg,
        major,
        opener,
        their_declarer,
        live_call: *auction.get(at)?,
    })
}

/// Stream every `shard-*.json` in `dir`, keeping only the boards that reach the
/// seat.  The arm is 4.6M boards and ~2% of them qualify, so folding the whole
/// dump into memory (`common::load_dump`) would cost gigabytes to throw away.
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

/// Solve every deal the cache is missing, on the main thread, in chunks.
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

/// This deal's par score for NS, rounded to whole points
///
/// `average_ns_par` averages over a histogram; a single solved deal is a
/// one-entry histogram, so this is the classic per-deal par.
#[allow(clippy::cast_possible_truncation)]
fn par_score(table: &TrickCountTable, dealer: Seat, vul: AbsoluteVulnerability) -> i64 {
    pons::stats::average_ns_par(std::iter::once(*table).collect(), vul, dealer)
        .map_or(0, |par| par.score.round() as i64)
}

/// One candidate contract opener could steer to
struct Candidate {
    name: String,
    contract: Contract,
    declarer: Seat,
}

fn candidate(name: &str, level: u8, strain: Strain, penalty: Penalty, declarer: Seat) -> Candidate {
    Candidate {
        name: name.to_owned(),
        contract: Contract {
            bid: Bid::new(level, strain),
            penalty,
        },
        declarer,
    }
}

/// The candidates every seat board shares — so the columns of one bucket row
/// are all measured on the *same* boards and their means are comparable.
fn common_candidates(hit: &Hit) -> Vec<Candidate> {
    let strain = Strain::from(hit.major);
    let doubler = hit.opener.partner();
    vec![
        candidate("2M", 2, strain, Penalty::Undoubled, hit.their_declarer),
        candidate("2Mx", 2, strain, Penalty::Doubled, hit.their_declarer),
        candidate("2NT@op", 2, Strain::Notrump, Penalty::Undoubled, hit.opener),
        candidate("2NT@dbl", 2, Strain::Notrump, Penalty::Undoubled, doubler),
        candidate("3NT@op", 3, Strain::Notrump, Penalty::Undoubled, hit.opener),
        candidate("3NT@dbl", 3, Strain::Notrump, Penalty::Undoubled, doubler),
    ]
}

/// A (bucket, candidate) accumulator: the IMP swings vs `live`, plain and PD
type Cell = (Vec<i64>, Vec<i64>);

/// Print one table: rows are buckets, and within a bucket one line per
/// candidate, sorted by plain mean descending (the winner first).
#[allow(clippy::cast_precision_loss)]
fn report(title: &str, note: &str, rows: &BTreeMap<(String, String), Cell>, min: usize) {
    println!("\n=== {title} ===");
    println!("{note}");
    println!(
        "{:<26} {:<10} {:>7} {:>19} {:>19}",
        "bucket", "candidate", "boards", "plain IMPs vs live", "PD IMPs vs live"
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
        println!("{:-<26}", "");
        for (name, cell) in cands {
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            println!(
                "{bucket:<26} {name:<10} {:>7} {:>12.3} ±{:.3} {:>12.3} ±{:.3}",
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

    println!("=== Landy opener oracle — {} ===", args.dir);
    println!(
        "vulnerability {vul:?}; {} of {scanned} boards reach opener's seat ({:.2}%)",
        hits.len(),
        100.0 * hits.len() as f64 / scanned.max(1) as f64,
    );
    let legs = |leg: Leg| hits.iter().filter(|h| h.leg == leg).count();
    let majors = |suit: Suit| hits.iter().filter(|h| h.major == suit).count();
    println!(
        "legs: direct {} / relay {};  their major: ♥ {} / ♠ {}",
        legs(Leg::Direct),
        legs(Leg::Relay),
        majors(Suit::Hearts),
        majors(Suit::Spades),
    );

    // The live contract and its two prices, per seat board.
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

    // --- what opener actually calls here today, and what it is worth ---------
    {
        let mut calls: BTreeMap<String, Cell> = BTreeMap::new();
        for (hit, &(lp, ld)) in hits.iter().zip(&live) {
            let entry = calls.entry(hit.live_call.to_string()).or_default();
            entry.0.push(lp);
            entry.1.push(ld);
        }
        println!("\n=== opener's live call at the seat (the floor's, today) ===");
        println!(
            "{:<8} {:>8} {:>7} {:>18} {:>18}",
            "call", "boards", "share", "plain score/bd", "PD score/bd"
        );
        for (call, cell) in &calls {
            let (pm, pc) = mean_with_ci(&cell.0);
            let (dm, dc) = mean_with_ci(&cell.1);
            println!(
                "{call:<8} {:>8} {:>6.1}% {:>11.1} ±{:.1} {:>11.1} ±{:.1}",
                cell.0.len(),
                100.0 * cell.0.len() as f64 / hits.len().max(1) as f64,
                pm,
                pc,
                dm,
                dc
            );
        }
    }

    // --- the main cut: length in their major × stopper × HCP -----------------
    let mut main_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut leg_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut minor_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut other_major_rows: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut other_major_surface = 0usize;

    for (hit, &(lp, ld)) in hits.iter().zip(&live) {
        let board = &boards[hit.index];
        let table = &cache[&deal_key(&board.deal)];
        let hand = board.deal[hit.opener];
        let held = hand[hit.major];
        let len = held.len();
        let hcp = hand_hcp(hand);
        let bucket = format!(
            "len{} {} hcp{}",
            if len >= 4 {
                "4+".to_owned()
            } else {
                len.to_string()
            },
            if has_stopper(held) { "stop" } else { "nost" },
            if (15..=17).contains(&hcp) {
                hcp.to_string()
            } else {
                "??".to_owned()
            },
        );
        let push = |rows: &mut BTreeMap<(String, String), Cell>, key: String, c: &Candidate| {
            let plain = ns_score_contract(Some((c.contract, c.declarer)), table, vul);
            let pd = ns_score_pd(Some((c.contract, c.declarer)), table, vul);
            let entry = rows.entry((key, c.name.clone())).or_default();
            entry.0.push(imps(plain - lp));
            entry.1.push(imps(pd - ld));
        };
        for c in &common_candidates(hit) {
            push(&mut main_rows, bucket.clone(), c);
            push(&mut leg_rows, hit.leg.name().to_owned(), c);
        }
        // Par is not a rung opener can bid — it is the ceiling the rungs are
        // chasing, so it rides along as a pseudo-candidate on the two cuts
        // where every board contributes.  `average_ns_par` already prices
        // under perfect-defense doubling, so its two columns coincide.
        let par = imps(par_score(table, board.dealer, vul) - lp);
        let par_pd = imps(par_score(table, board.dealer, vul) - ld);
        for (rows, key) in [
            (&mut main_rows, bucket.clone()),
            (&mut leg_rows, hit.leg.name().to_owned()),
        ] {
            let entry = rows.entry((key, "par".to_owned())).or_default();
            entry.0.push(par);
            entry.1.push(par_pd);
        }

        // The `3m` question: a natural three-level minor is only a candidate on
        // the shapes that could bid it, so it gets its own cut where every
        // column is restricted to the same boards.
        for minor in [Suit::Clubs, Suit::Diamonds] {
            let n = hand[minor].len();
            if n < 5 {
                continue;
            }
            let key = format!("{minor} {}", if n >= 6 { "6+" } else { "5" });
            let three = candidate(
                "3m@op",
                3,
                Strain::from(minor),
                Penalty::Undoubled,
                hit.opener,
            );
            push(&mut minor_rows, key.clone(), &three);
            for c in &common_candidates(hit) {
                push(&mut minor_rows, key.clone(), c);
            }
        }

        // The `3OM` question: opener with five of the major their side did NOT
        // name.  Expected to be a sliver — they hold 4+ of it.
        let other = if hit.major == Suit::Hearts {
            Suit::Spades
        } else {
            Suit::Hearts
        };
        if hand[other].len() >= 5 {
            other_major_surface += 1;
            let key = format!("5+{other}");
            let three = candidate(
                "3OM@op",
                3,
                Strain::from(other),
                Penalty::Undoubled,
                hit.opener,
            );
            push(&mut other_major_rows, key.clone(), &three);
            for c in &common_candidates(hit) {
                push(&mut other_major_rows, key.clone(), c);
            }
        }
    }

    report(
        "leg totals",
        "Every candidate on every seat board.  `2Mx` is opener's penalty double; read it plain-only.",
        &leg_rows,
        0,
    );
    report(
        "opener's length in their major × stopper × HCP",
        "The design cut: which candidate wins the bucket, and by how much over today's floor.",
        &main_rows,
        args.min,
    );
    report(
        "the 3m rung — boards where opener holds the minor",
        "Restricted to the shapes that could bid it; the other columns are the same boards.",
        &minor_rows,
        0,
    );
    println!(
        "\n3OM surface: {other_major_surface} of {} seat boards ({:.2}%) — opener holds 5+ of the major they did not name",
        hits.len(),
        100.0 * other_major_surface as f64 / hits.len().max(1) as f64,
    );
    report(
        "the 3OM rung",
        "Opener's five-card holding in the unnamed major.",
        &other_major_rows,
        0,
    );

    if args.show > 0 {
        let mut order: Vec<usize> = (0..hits.len()).collect();
        let score_of = |i: usize| {
            let hit = &hits[i];
            let board = &boards[hit.index];
            let table = &cache[&deal_key(&board.deal)];
            common_candidates(hit)
                .iter()
                .find(|c| c.name == args.candidate)
                .map_or(0, |c| {
                    ns_score_contract(Some((c.contract, c.declarer)), table, vul) - live[i].0
                })
        };
        order.sort_by_key(|&i| -imps(score_of(i)));
        println!(
            "\n--- {} boards where `{}` gains most over live ---",
            args.show, args.candidate
        );
        for &i in order.iter().take(args.show) {
            let hit = &hits[i];
            let board = &boards[hit.index];
            println!(
                "[{:+} plain] {:?} {} ({} hcp) vs {}; live {:?}\n  {}",
                imps(score_of(i)),
                hit.opener,
                board.deal[hit.opener],
                hand_hcp(board.deal[hit.opener]),
                hit.major,
                board.table_a,
                board.deal,
            );
        }
    }
}
