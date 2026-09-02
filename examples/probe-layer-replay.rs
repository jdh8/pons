//! Which layer made each of our calls on a divergent board?
//!
//! `probe-divergence` says *what* diverged; the dumps carry no provenance, so
//! the forensic that asks "was that the book or the floor?" had to be run by
//! hand on `probe-decision`, one board at a time.  This replays every one of
//! our calls on the candidate arm's `table_a` (we sit North/South) through the
//! same partnership the arm was generated under and stamps each with the
//! trie's [`Provenance`][pons::bidding::trie::Provenance]: a call is the
//! **floor's** when the answering node is the root fallback (`depth == 0`,
//! `fallback.is_some()`), exactly the test helpers' `floored` bit.
//!
//! ```sh
//! cargo build --release --features serde --example probe-layer-replay
//! ./target/release/examples/probe-layer-replay ab-results/landy-lia3/lia-none \
//!     --jsonl ab-results/landy-lia3/imps-none.jsonl \
//!     --out   ab-results/landy-lia3/layers-none.jsonl --ns-landy-lia
//! ```
//!
//! One JSON record per divergent board: `index`, and `calls` — for each of
//! our calls its position, seat, the call, `floored`, the provenance triple,
//! and whether replaying reproduces it (`matches`; a `false` means the knobs
//! given here are not the arm's, and the record is worthless).  Born on the
//! lia3 forensic, where the five worst boards at every cell were auctions
//! no lia table could have produced (`2♠ (3♠) 4♠ X XX`).

use clap::Parser;
use contract_bridge::Seat;
use contract_bridge::auction::{Auction, Call};
use pons::bidding::Relative;
use pons::bidding::agreements::Agreements;
use pons::bidding::context::relative;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::io::{BufRead, Write};

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{seat_floor, seat_to_act, vs_bba_agreements};

#[derive(Parser)]
struct Args {
    /// Candidate arm: a directory of `shard-*.json` (or one dump)
    arm: String,

    /// `probe-divergence --jsonl` records; only their `index`es are replayed
    #[arg(long)]
    jsonl: String,

    /// Output jsonl
    #[arg(long)]
    out: String,

    /// Which of our floors the arm was generated under
    #[arg(long, default_value = "american")]
    floor: String,

    /// The arm ran with `--ns-landy-lia`
    #[arg(long)]
    ns_landy_lia: bool,

    /// The arm ran with `--ns-new-suit-veto`
    #[arg(long)]
    ns_new_suit_veto: bool,
}

#[derive(serde::Serialize)]
struct Made {
    i: usize,
    seat: String,
    call: String,
    floored: bool,
    depth: usize,
    fallback: Option<usize>,
    rebases: usize,
    matches: bool,
    best: String,
    /// For a suit bid: the bidder's own length in that suit, partner's
    /// announced length range in it, and whether our side had bid it before —
    /// the inputs an envelope-gated new-suit veto would read.
    suit: Option<SuitBid>,
}

#[derive(serde::Serialize)]
struct SuitBid {
    own_len: u8,
    partner_min: u8,
    partner_max: u8,
    new_suit: bool,
}

#[derive(serde::Serialize)]
struct Record {
    index: usize,
    calls: Vec<Made>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut agreements = vs_bba_agreements(Agreements::default());
    agreements.competition.defense_2c_landy_lia = args.ns_landy_lia;
    agreements.decision.instinct.new_suit_veto = args.ns_new_suit_veto;
    let partnership = seat_floor(&args.floor, &agreements)?;

    let wanted: BTreeSet<usize> = std::io::BufReader::new(std::fs::File::open(&args.jsonl)?)
        .lines()
        .map(|line| -> anyhow::Result<usize> {
            let v: serde_json::Value = serde_json::from_str(&line?)?;
            Ok(v["index"].as_u64().expect("index") as usize)
        })
        .collect::<Result<_, _>>()?;
    let dump = common::load_dump(&args.arm);
    let vul = dump.vulnerability;
    eprintln!(
        "{} boards in the arm, {} to replay",
        dump.boards.len(),
        wanted.len()
    );

    let records: Vec<Record> = wanted
        .par_iter()
        .map(|&index| {
            let board = &dump.boards[index];
            let auction = &board.table_a;
            let mut prefix = Auction::new();
            let mut calls = Vec::new();
            for (i, &call) in auction.iter().enumerate() {
                let seat = seat_to_act(board.dealer, i);
                if matches!(seat, Seat::North | Seat::South) {
                    let classified = partnership.classify_with_provenance(
                        board.deal[seat],
                        relative(vul, seat),
                        &prefix,
                    );
                    let (best, floored, depth, fallback, rebases) = match classified {
                        Some((logits, prov)) => {
                            let mut scored: Vec<(Call, f32)> = logits
                                .iter()
                                .map(|(c, &l)| (c, l))
                                .filter(|&(_, l)| l.is_finite())
                                .collect();
                            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("no NaN"));
                            let best = scored
                                .into_iter()
                                .map(|(c, _)| c)
                                .find(|&c| prefix.can_push(c).is_ok())
                                .unwrap_or(Call::Pass);
                            (
                                best,
                                prov.depth == 0 && prov.fallback.is_some(),
                                prov.depth,
                                prov.fallback,
                                prov.rebases,
                            )
                        }
                        None => (Call::Pass, true, 0, None, 0),
                    };
                    let suit = match call {
                        Call::Bid(bid) => bid.strain.suit().map(|s| {
                            let partner = partnership
                                .infer(relative(vul, seat), &prefix)
                                .announced(Relative::Partner)
                                .length(s);
                            let bid_before = |who: Seat| {
                                (0..i)
                                    .filter(|&j| seat_to_act(board.dealer, j) == who)
                                    .any(|j| match auction[j] {
                                        Call::Bid(b) => b.strain.suit() == Some(s),
                                        _ => false,
                                    })
                            };
                            SuitBid {
                                own_len: board.deal[seat][s].len() as u8,
                                partner_min: partner.min,
                                partner_max: partner.max,
                                new_suit: !bid_before(seat) && !bid_before(seat.partner()),
                            }
                        }),
                        _ => None,
                    };
                    calls.push(Made {
                        i,
                        seat: format!("{seat:?}"),
                        call: call.to_string(),
                        floored,
                        depth,
                        fallback,
                        rebases,
                        matches: best == call,
                        best: best.to_string(),
                        suit,
                    });
                }
                prefix.push(call);
            }
            Record { index, calls }
        })
        .collect();

    let mut out = std::io::BufWriter::new(std::fs::File::create(&args.out)?);
    let mut mismatched = 0usize;
    for record in &records {
        mismatched += usize::from(record.calls.iter().any(|c| !c.matches));
        serde_json::to_writer(&mut out, record)?;
        out.write_all(b"\n")?;
    }
    eprintln!(
        "{} records written to {}; {} with a call the replay did not reproduce",
        records.len(),
        args.out,
        mismatched
    );
    Ok(())
}
