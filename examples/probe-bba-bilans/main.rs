//! Session 7A of the BBA floor study (`docs/ai-bidder/bba-floor.md` §7): dump
//! BBA's **internal bilans state** beside our own call, one jsonl row per
//! decision, turning the oracle from a yes/no comparator into a graded teacher.
//!
//! §5 established that BBA's off-book "calculated bid" reconstructs all four
//! hands, counts winners and losers for both sides, and picks a level by
//! expected score. Every stage of that is exported and now bound
//! ([`common::oracle::BbaOracle::probe`]), so each row carries BBA's target
//! levels and its reconstruction of all four hands at the moment it chose its
//! call. Sessions C (a trick model for our floor) and D (expected-score level
//! choice) fit against this.
//!
//! This is **reconnaissance**, not a corpus generator: one process, a few
//! thousand boards, so the row schema can be read before it is committed to.
//! `scripts/bba-gen-parallel.sh` is the sibling to copy when it is.
//!
//! ```text
//! cargo run --release --features serde --example probe-bba-bilans -- \
//!   --count 2000 --seed 42 -o bilans.jsonl
//! cargo run --features serde --example probe-bba-bilans -- --self-check
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{AbsoluteVulnerability, Bid, Hand, Level, Seat, Strain};
use pons::bidding::context::relative;
use pons::bidding::{Family, System};
use std::io::Write;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::oracle::{BbaOracle, BbaState, DEFAULT_LIB, InfoField, SYSTEM_2_OVER_1};
use common::{hand_hcp, seat_to_act, seeded_deals};

#[derive(Parser)]
#[command(about = "Dump BBA's bilans state beside our call, one jsonl row per decision")]
struct Args {
    /// Number of boards to bid out (dealer rotates per board)
    #[arg(short, long, default_value_t = 2000)]
    count: usize,

    /// Deal-stream seed; board `i` is seeded `seed + i` (the seed-hygiene
    /// invariant — pass a fresh `$(date +%s)` per experiment)
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Vulnerability the boards are bid at: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Our side's system: `american` (book + floor) or `american-floor` (floor
    /// alone — the thing sessions C/D aim to improve)
    #[arg(long, default_value = "american")]
    our_floor: String,

    /// Force EPBot's scoring form instead of keeping its default
    #[arg(long)]
    scoring: Option<i32>,

    /// Path to `libEPBot.so`
    #[arg(long, default_value = DEFAULT_LIB)]
    lib: String,

    /// Write jsonl here; default is stdout
    #[arg(short, long)]
    output: Option<String>,

    /// Assert the FFI ABI is wired correctly and exit
    ///
    /// This is the ABI self-check that would normally be a `#[test]`. EPBot's
    /// NativeAOT runtime segfaults when driven from a `cargo test` thread — the
    /// pre-existing `classify` path does too — so it lives here, on the main
    /// thread, where it actually runs.
    #[arg(long)]
    self_check: bool,
}

/// One decision: what we would call, what BBA called, and why BBA called it
#[derive(serde::Serialize)]
struct Row {
    board: usize,
    /// Index of this call within the auction
    turn: usize,
    seat: Seat,
    dealer: Seat,
    hcp: u8,
    /// The actor's hand, `Display`-rendered (`AQ4.KJ3.Q762.K85`)
    hand: String,
    /// The auction up to but excluding this call
    auction: Vec<String>,
    /// What our system would call here, regardless of whose turn it is
    our_call: String,
    /// What BBA called here
    bba_call: Option<String>,
    /// BBA's own scoring form, as it reports it
    scoring: i32,
    /// Stage 4's output; see [`BbaState::probable_level`]
    probable_level: Vec<i32>,
    /// BBA's hand model, positions 0..4 public and 4..8 reconstructed
    seats: Vec<SeatRow>,
}

/// One position of BBA's hand model, flattened for jsonl
#[derive(serde::Serialize)]
struct SeatRow {
    position: usize,
    alerting: i32,
    hcp_min: i32,
    hcp_max: i32,
    min_length: Vec<i32>,
    max_length: Vec<i32>,
    probable_length: Vec<i32>,
    strength: Vec<i32>,
    stoppers: Vec<i32>,
    honors: Vec<i32>,
    suit_power: Vec<i32>,
    /// Nonzero `feature` slots only — sparse, so complete without being huge
    features: std::collections::BTreeMap<u16, i32>,
}

fn to_rows(state: &BbaState) -> Vec<SeatRow> {
    state
        .seats
        .iter()
        .enumerate()
        .map(|(position, seat)| {
            let (hcp_min, hcp_max) = seat.hcp_range();
            SeatRow {
                position,
                alerting: seat.alerting,
                hcp_min,
                hcp_max,
                min_length: seat.min_length.to_vec(),
                max_length: seat.max_length.to_vec(),
                probable_length: seat.probable_length.to_vec(),
                strength: seat.strength.to_vec(),
                stoppers: seat.stoppers.to_vec(),
                honors: seat.honors.to_vec(),
                suit_power: seat.suit_power.to_vec(),
                features: seat.features.clone(),
            }
        })
        .collect()
}

/// The highest-logit legal call from a system, defaulting to a pass
fn call_of(
    system: &dyn System,
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    a: &Auction,
) -> Call {
    common::oracle::next_call(system, hand, seat, vul, a)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let bba = BbaOracle::load(&args.lib, SYSTEM_2_OVER_1, Vec::new())?.with_scoring(args.scoring);

    if args.self_check {
        return self_check(&bba);
    }

    let ours = match args.our_floor.as_str() {
        "american" => pons::american().against(Family::NATURAL),
        "american-floor" => pons::american_floor().against(Family::NATURAL),
        other => anyhow::bail!("--our-floor must be american|american-floor, got {other:?}"),
    };

    let mut out: Box<dyn Write> = match &args.output {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::BufWriter::new(std::io::stdout())),
    };

    for (board, deal) in seeded_deals(args.seed, args.count).into_iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        let mut auction = Auction::new();
        // Probe at EVERY turn, not just ours: BBA's assessment at its own
        // decisions is the teacher's own answer, it costs nothing extra, and
        // each row carries `seat` so downstream filtering is trivial.
        while !auction.has_ended() {
            let turn = auction.len();
            let seat = seat_to_act(dealer, turn);
            let hand = deal[seat];
            let vul = relative(args.vulnerability, seat);

            let Some(state) = bba.probe(hand, vul, &auction) else {
                anyhow::bail!("EPBot failed to allocate a bot on board {board} turn {turn}");
            };
            let our_call = call_of(&ours, hand, seat, args.vulnerability, &auction);

            serde_json::to_writer(
                &mut out,
                &Row {
                    board,
                    turn,
                    seat,
                    dealer,
                    hcp: hand_hcp(hand),
                    hand: hand.to_string(),
                    auction: auction.iter().map(ToString::to_string).collect(),
                    our_call: our_call.to_string(),
                    bba_call: state.call.map(|c| c.to_string()),
                    scoring: state.scoring,
                    probable_level: state.probable_level.to_vec(),
                    seats: to_rows(&state),
                },
            )?;
            writeln!(out)?;

            // BBA bids the whole auction, so the prefixes stay auctions BBA
            // itself produces — the state we read is state it actually reached.
            let bba_call = state
                .call
                .filter(|&call| auction.can_push(call).is_ok())
                .unwrap_or(Call::Pass);
            auction.push(bba_call);
        }
    }
    out.flush()?;
    Ok(())
}

/// Assert the widened FFI is wired correctly (see [`Args::self_check`])
fn self_check(bba: &BbaOracle) -> anyhow::Result<()> {
    let hand: Hand = "AQ4.KJ3.Q762.K85".parse()?;
    let opening = Call::Bid(Bid {
        level: Level::new(1),
        strain: Strain::Notrump,
    });

    // (a) The read path.  After a 1NT opening, BBA's public block must carry an
    // HCP band consistent with a strong notrump and sane suit lengths.  Fails
    // if the buffer size were passed in elements rather than bytes, if
    // `position` were misindexed, or if state were read before `get_bid`
    // populated it.
    let state = bba
        .probe(hand, RelativeVulnerability::empty(), &[opening])
        .ok_or_else(|| anyhow::anyhow!("EPBot failed to allocate a bot"))?;

    let (min, max) = state
        .seats
        .iter()
        .take(4)
        .map(common::oracle::SeatInfo::hcp_range)
        .find(|&(min, max)| min > 0 && max > 0 && max - min <= 6)
        .ok_or_else(|| anyhow::anyhow!("no position carries a bounded HCP band after 1NT"))?;
    anyhow::ensure!(
        (14..=18).contains(&min) && (14..=18).contains(&max),
        "1NT opener read as {min}-{max} HCP, expected a band inside 14..=18"
    );

    for (position, seat) in state.seats.iter().enumerate() {
        for suit in 0..4 {
            anyhow::ensure!(
                seat.min_length[suit] <= seat.max_length[suit],
                "position {position} suit {suit}: min {} > max {}",
                seat.min_length[suit],
                seat.max_length[suit]
            );
        }
    }

    // (b) `set_info_*`'s trailing argument is an ELEMENT count while the
    // getter's is a BYTE count — exactly the asymmetry that would silently
    // corrupt an injected hand model, so pin it with a round trip.
    let lengths = vec![2, 3, 4, 4];
    let injected = bba
        .probe_with(
            hand,
            RelativeVulnerability::empty(),
            &[],
            &[(4, InfoField::MinLength, lengths.clone())],
        )
        .ok_or_else(|| anyhow::anyhow!("EPBot failed to allocate a bot"))?;
    anyhow::ensure!(
        injected.seats[4].min_length.to_vec() == lengths,
        "set_info_min_length did not round-trip; check elements vs bytes"
    );

    // (c) `probable_levels` still returns the 9 entries §6 was decoded against.
    anyhow::ensure!(
        state.probable_level.len() == common::oracle::PROBABLE_LEVELS,
        "probable_levels changed length; re-read bba-floor.md §6"
    );

    // (d) The load-bearing indexing invariant: position `4 + turn % 4` is the
    // ACTOR's own hand, exactly — not a band.  Verified on all 21039 rows of
    // the 7A recon dump.  If this ever fails, every consumer of positions 4..8
    // is reading the wrong seat, which no other check here would catch.
    let (min, max) = state.seats[4 + 1].hcp_range();
    let expected = i32::from(hand_hcp(hand));
    anyhow::ensure!(
        min == expected && max == expected,
        "position 4+turn%4 should be the actor's exact hand ({expected} HCP), read {min}-{max}"
    );

    println!("self-check OK: read path, set_info round trip, levels width, actor-slot indexing");
    Ok(())
}
