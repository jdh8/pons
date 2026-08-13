//! *What* changed between two aligned arms — the forensic sibling of `ab-dump-diff`.
//!
//! [`ab-dump-diff`](../ab-dump-diff/main.rs) answers **how much** an A/B moved:
//! IMPs, the CI, the worst boards.  It does not say what the arms actually did
//! differently, so every post-mortem re-derives the same handful of counts by
//! eye from its `--show` dump.  This reads the same two arm dirs and classifies
//! each divergent board instead: who bid differently first, whether our call
//! replaced a pass, whether a game was reached in one arm only, whether
//! declarer changed sides, whether the opponents were handed more room.
//!
//! The buckets are deliberately **package-agnostic** — a specific
//! investigation filters the per-board records (`--jsonl`) rather than growing
//! a bucket here.  The three classes the Landy-cue post-mortem needs
//! ([docs/one-notrump-competitive.md](../../docs/one-notrump-competitive.md))
//! are each one field of a record: the sub-game leak is `game_off && !game_on`,
//! the room-for-them cost is `opp_calls_on > opp_calls_off`, and the weak
//! escapes are `first_diff_ours` with `call_off` a pass.
//!
//! ```text
//! cargo run --release --features serde --example probe-divergence -- \
//!     ab-results/landy-counter-v2/landy-cues-both \
//!     ab-results/landy-counter-v2/landy-on-both --jsonl /tmp/cues.jsonl
//! ```
//!
//! Counting needs **no solver at all** — the arms already carry both auctions,
//! and a contract derives from the auction and the dealer.  `--imps`
//! additionally solves the divergent set double dummy and stamps each record
//! with its plain and perfect-defense swing, for when a bucket has to be priced
//! rather than merely counted.

use clap::Parser;
use contract_bridge::auction::{Auction, Call, display_calls};
use contract_bridge::{AbsoluteVulnerability, Contract, Seat, Strain};
use pons::scoring::{final_contract, ns_score_contract, ns_score_pd};
use std::io::Write;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, Reached, score_solved, seat_to_act, solve_divergent};

#[derive(Parser)]
struct Args {
    /// Candidate arm: a directory of `shard-*.json` (or one dump) — the `on` side
    on: String,
    /// Baseline arm, generated from the same `SEED_BASE` so the deals pair
    off: String,
    /// Re-price at this vulnerability instead of the dump's (only used by `--imps`)
    #[arg(short, long)]
    vulnerability: Option<AbsoluteVulnerability>,
    /// Write one JSON record per divergent board here
    #[arg(long)]
    jsonl: Option<String>,
    /// Also solve the divergent set and stamp each record with its IMP swings
    ///
    // ponytail: solves every divergent board fresh.  If a future arm's
    // divergent set is big enough for that to hurt, take
    // `probe-1nt-interference`'s deal-keyed `--dd-cache`.
    #[arg(long)]
    imps: bool,
    /// Print this many divergent boards (worst for the candidate first)
    #[arg(long, default_value_t = 0)]
    show: usize,
    /// **The isolation gate**: `ours` or `theirs` — exit non-zero unless every
    /// divergent board was opened by that side.
    ///
    /// A package keyed on our own opening (a counter-defense, a response
    /// structure) may only move boards *we* opened; a package keyed on theirs
    /// (an overcall, a defense) only boards *they* opened. Anything else is
    /// the convention reaching an auction it does not own — usually through
    /// the reading path, which mirrors our book onto the opponents' calls
    /// whenever no foreign book is declared (`read.rs:333-335`, `:386-389`).
    /// The Landy counter leaked that way on 38% of its divergent boards, and
    /// the shipped base counter on 21%, with nothing to catch it.
    #[arg(long)]
    gate_opener: Option<String>,
}

/// One divergent board, classified.  Every bucket in the summary is a
/// projection of these fields, so a question the summary does not answer is a
/// filter over the `--jsonl` stream rather than a change here.
#[derive(serde::Serialize)]
struct Record {
    /// Board index within the arm, shared by both dumps
    index: usize,
    deal: String,
    dealer: String,
    auction_on: String,
    auction_off: String,
    contract_on: String,
    contract_off: String,
    /// Contract level, 0 for a pass-out
    level_on: u8,
    level_off: u8,
    /// Whether the contract is game or better (3NT / 4M / 5m)
    game_on: bool,
    game_off: bool,
    doubled_on: bool,
    doubled_off: bool,
    /// Declaring pair, `null` for a pass-out
    declarer_on: Option<&'static str>,
    declarer_off: Option<&'static str>,
    /// The baseline auction's opening call, `"(pass-out)"` if nobody opened.
    /// Read off the baseline because that is what defines the board's shape;
    /// a candidate that moves the *opening* is not what this field is for.
    opening_call: String,
    /// Whether our side made that opening call — the isolation gate's subject
    opener_ours: bool,
    /// Index of the first call that differs between the two auctions
    first_diff: usize,
    /// Whether that first differing call is ours (we sit North/South at table A)
    first_diff_ours: bool,
    call_on: String,
    call_off: String,
    /// Non-pass opponent calls from `first_diff` onward — how much room each
    /// arm handed them
    opp_calls_on: usize,
    opp_calls_off: usize,
    /// `on − off`, present only under `--imps`
    imps_plain: Option<i64>,
    imps_pd: Option<i64>,
}

/// Whether a contract is game or better
const fn is_game(contract: Contract) -> bool {
    let level = contract.bid.level.get();
    match contract.bid.strain {
        Strain::Notrump => level >= 3,
        Strain::Hearts | Strain::Spades => level >= 4,
        Strain::Clubs | Strain::Diamonds => level >= 5,
    }
}

/// Which pair a seat belongs to.  We sit North/South at table A, the auction
/// both arms record.
const fn side(seat: Seat) -> &'static str {
    if matches!(seat, Seat::North | Seat::South) {
        "NS"
    } else {
        "EW"
    }
}

/// One call rendered as the auction renders it (a pass is `-`), or `(end)` past
/// the end of a shorter auction
fn call_at(auction: &Auction, index: usize) -> String {
    auction
        .get(index)
        .map_or_else(|| "(end)".to_owned(), |&c| display_calls(&[c]).to_string())
}

/// Non-pass opponent calls from `from` onward — the room this auction gave them
fn opp_calls(auction: &Auction, dealer: Seat, from: usize) -> usize {
    (from..auction.len())
        .filter(|&i| auction[i] != Call::Pass && side(seat_to_act(dealer, i)) == "EW")
        .count()
}

/// Classify one divergent board
fn classify(index: usize, on: &Board, off: &Board, contracts: (Reached, Reached)) -> Record {
    let first_diff = (0..on.table_a.len().max(off.table_a.len()))
        .find(|&i| on.table_a.get(i) != off.table_a.get(i))
        .unwrap_or_else(|| on.table_a.len());
    let level = |reached: Reached| reached.map_or(0, |(c, _)| c.bid.level.get());
    let game = |reached: Reached| reached.is_some_and(|(c, _)| is_game(c));
    let doubled = |reached: Reached| {
        reached.is_some_and(|(c, _)| c.penalty != contract_bridge::Penalty::Undoubled)
    };
    let declarer = |reached: Reached| reached.map(|(_, seat)| side(seat));
    let contract = |reached: Reached| {
        reached.map_or_else(
            || "pass-out".to_owned(),
            |(c, seat)| format!("{c} {seat:?}"),
        )
    };
    let opening = off.table_a.iter().position(|&c| c != Call::Pass);
    Record {
        index,
        deal: on.deal.to_string(),
        dealer: format!("{:?}", on.dealer),
        opening_call: opening.map_or_else(|| "(pass-out)".to_owned(), |i| call_at(&off.table_a, i)),
        opener_ours: opening.is_some_and(|i| side(seat_to_act(off.dealer, i)) == "NS"),
        auction_on: on.table_a.to_string(),
        auction_off: off.table_a.to_string(),
        contract_on: contract(contracts.0),
        contract_off: contract(contracts.1),
        level_on: level(contracts.0),
        level_off: level(contracts.1),
        game_on: game(contracts.0),
        game_off: game(contracts.1),
        doubled_on: doubled(contracts.0),
        doubled_off: doubled(contracts.1),
        declarer_on: declarer(contracts.0),
        declarer_off: declarer(contracts.1),
        first_diff,
        first_diff_ours: side(seat_to_act(on.dealer, first_diff)) == "NS",
        call_on: call_at(&on.table_a, first_diff),
        call_off: call_at(&off.table_a, first_diff),
        opp_calls_on: opp_calls(&on.table_a, on.dealer, first_diff),
        opp_calls_off: opp_calls(&off.table_a, off.dealer, first_diff),
        imps_plain: None,
        imps_pd: None,
    }
}

/// One bucket line: count, share of the divergent set, and what it means
fn line(label: &str, count: usize, divergent: usize) {
    println!(
        "  {label:<38} {count:>7} {:>7.1}%",
        100.0 * count as f64 / divergent.max(1) as f64
    );
}

#[allow(clippy::cast_precision_loss)]
fn summarize(total: usize, records: &[Record]) {
    let divergent = records.len();
    let count = |f: &dyn Fn(&Record) -> bool| records.iter().filter(|r| f(r)).count();
    println!(
        "{total} boards, {divergent} divergent ({:.2}%)\n",
        100.0 * divergent as f64 / total.max(1) as f64
    );

    println!("who opened the board (baseline arm)");
    line("ours", count(&|r| r.opener_ours), divergent);
    line("theirs", count(&|r| !r.opener_ours), divergent);

    println!("\nwho bid differently first");
    line("ours", count(&|r| r.first_diff_ours), divergent);
    line("theirs", count(&|r| !r.first_diff_ours), divergent);

    println!("\nour first differing call");
    let ours = |r: &Record, on_pass: bool, off_pass: bool| {
        r.first_diff_ours && (r.call_on == "-") == on_pass && (r.call_off == "-") == off_pass
    };
    line(
        "bid where the baseline passed",
        count(&|r| ours(r, false, true)),
        divergent,
    );
    line(
        "passed where the baseline bid",
        count(&|r| ours(r, true, false)),
        divergent,
    );
    line(
        "a different bid",
        count(&|r| ours(r, false, false)),
        divergent,
    );

    println!("\ngame reached");
    line(
        "candidate only",
        count(&|r| r.game_on && !r.game_off),
        divergent,
    );
    line(
        "baseline only",
        count(&|r| !r.game_on && r.game_off),
        divergent,
    );
    line("both", count(&|r| r.game_on && r.game_off), divergent);
    line("neither", count(&|r| !r.game_on && !r.game_off), divergent);

    println!("\ncontract");
    line(
        "declarer changed sides",
        count(&|r| {
            r.declarer_on.is_some() && r.declarer_off.is_some() && r.declarer_on != r.declarer_off
        }),
        divergent,
    );
    line(
        "doubled in exactly one arm",
        count(&|r| r.doubled_on != r.doubled_off),
        divergent,
    );
    line(
        "pass-out in exactly one arm",
        count(&|r| (r.level_on == 0) != (r.level_off == 0)),
        divergent,
    );

    println!("\nroom handed to the opponents (non-pass calls after the divergence)");
    line(
        "more in the candidate",
        count(&|r| r.opp_calls_on > r.opp_calls_off),
        divergent,
    );
    line(
        "more in the baseline",
        count(&|r| r.opp_calls_on < r.opp_calls_off),
        divergent,
    );
    line(
        "equal",
        count(&|r| r.opp_calls_on == r.opp_calls_off),
        divergent,
    );
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let on = common::load_dump(&args.on);
    let off = common::load_dump(&args.off);
    assert_eq!(on.boards.len(), off.boards.len(), "arms must be aligned");
    let vul = args.vulnerability.unwrap_or(on.vulnerability);

    let mut deals = Vec::with_capacity(on.boards.len());
    let contracts: Vec<(Reached, Reached)> = on
        .boards
        .iter()
        .zip(&off.boards)
        .map(|(a, b)| {
            assert_eq!(a.deal, b.deal, "arms not seed-aligned");
            deals.push(a.deal);
            (
                final_contract(&a.table_a, a.dealer),
                final_contract(&b.table_a, b.dealer),
            )
        })
        .collect();

    let mut records: Vec<Record> = (0..on.boards.len())
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .map(|i| classify(i, &on.boards[i], &off.boards[i], contracts[i]))
        .collect();

    if args.imps {
        // One solve serves both scorers, `ab-dump-diff`'s idiom.
        let (divergent, tables) = solve_divergent(&contracts, &deals);
        let plain = score_solved(
            &contracts,
            divergent.clone(),
            tables.clone(),
            vul,
            ns_score_contract,
        );
        let pd = score_solved(&contracts, divergent, tables, vul, ns_score_pd);
        for record in &mut records {
            record.imps_plain = Some(plain.board_imps[record.index]);
            record.imps_pd = Some(pd.board_imps[record.index]);
        }
    }

    println!("=== {} (candidate) vs {} (baseline) ===", args.on, args.off);
    summarize(on.boards.len(), &records);

    if let Some(path) = args.jsonl.as_deref() {
        let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
        for record in &records {
            serde_json::to_writer(&mut file, record)?;
            writeln!(file)?;
        }
        file.flush()?;
        println!("\n{} records written to {path}", records.len());
    }

    if args.show > 0 {
        records.sort_by_key(|r| r.imps_plain.unwrap_or(0));
        println!(
            "\n--- {} divergent boards ---",
            args.show.min(records.len())
        );
        for record in records.iter().take(args.show) {
            let imps = record.imps_plain.map_or_else(String::new, |i| {
                format!("[{i:+} plain / {:+} PD] ", record.imps_pd.unwrap_or(0))
            });
            println!(
                "{imps}{}\n  on:  {} => {}\n  off: {} => {}",
                record.deal,
                record.auction_on,
                record.contract_on,
                record.auction_off,
                record.contract_off,
            );
        }
    }

    if let Some(want) = args.gate_opener.as_deref() {
        let ours = match want {
            "ours" => true,
            "theirs" => false,
            other => anyhow::bail!("--gate-opener must be ours|theirs, got {other:?}"),
        };
        let bad: Vec<&Record> = records.iter().filter(|r| r.opener_ours != ours).collect();
        println!(
            "\nisolation gate (--gate-opener {want}): {} of {} divergent boards opened by the other side",
            bad.len(),
            records.len(),
        );
        for record in bad.iter().take(5) {
            println!(
                "  opened {} by {}\n    on:  {}\n    off: {}",
                record.opening_call,
                if record.opener_ours { "us" } else { "them" },
                record.auction_on,
                record.auction_off,
            );
        }
        if !bad.is_empty() {
            anyhow::bail!(
                "isolation gate FAILED: {} divergent boards the package does not own",
                bad.len()
            );
        }
        println!("  isolation gate PASSED");
    }
    Ok(())
}
