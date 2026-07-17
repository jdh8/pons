//! Phase B of the BEN book extraction (docs/ben-gap-campaign.md): dump, per
//! self-play auction prefix, what **our** reading layer infers about the three
//! hidden hands — one jsonl row per (board, prefix) — so BEN's Info net can
//! annotate the same rows (`scripts/ben-info-dump.py`) and a comparer can rank
//! where our `Inferences::read` is wrong (BEN's mean outside our band) or
//! vague (our band far wider than BEN's spread), targeting inference fixes.
//!
//! Auctions are our `american()` floor in self-play at all four seats, so the
//! prefixes are auctions our own system produces and must read. Ground truth
//! (each hidden hand's actual HCP and shape) rides along: the comparer can
//! score us and BEN against *truth*, not just against each other.
//!
//! ```text
//! cargo run --features serde --example probe-ben-info -- \
//!   --count 1000 --seed 42 -o probe.jsonl
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Hand, Seat, Suit};
use pons::american;
use pons::bidding::Family;
use pons::bidding::constraint::point_count;
use pons::bidding::context::relative;
use pons::bidding::inference::{Inference, Relative};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::io::Write;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{hand_hcp, next_call, seat_to_act};

#[derive(Parser)]
#[command(about = "Dump our per-prefix hidden-hand inferences for BEN's Info net to annotate")]
struct Args {
    /// Number of boards to self-play (dealer rotates per board)
    #[arg(long, default_value_t = 1000)]
    count: usize,

    /// Deal-stream seed (single-stream `StdRng`, like ben-gen); unset = random
    #[arg(long)]
    seed: Option<u64>,

    /// Vulnerability the boards are bid at: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Enable the unshipped reading fixes (cue reading, sound length floors,
    /// table-wide alert reading, and the pass reading) to verify the probe's
    /// phantom-suit and pass-vagueness buckets drain
    #[arg(long)]
    sound_reading: bool,

    /// Write jsonl here; default is stdout
    #[arg(short, long)]
    output: Option<String>,
}

/// BEN suit letters in [`Strain`][contract_bridge::Strain] discriminant order
/// (copied from ben-gen, which owns the live-server dialect)
const STRAIN_CHARS: [char; 5] = ['C', 'D', 'H', 'S', 'N'];

/// A call as a BEN token (`P`, `X`, `XX`, `1C`…`7N`), ben-gen's `ctx` dialect
fn call_token(call: Call) -> String {
    match call {
        Call::Pass => "P".into(),
        Call::Double => "X".into(),
        Call::Redouble => "XX".into(),
        Call::Bid(bid) => format!("{}{}", bid.level.get(), STRAIN_CHARS[bid.strain as usize]),
    }
}

/// The hand in BEN's PBN order (`S.H.D.C`, ranks high-to-low, `T` for ten),
/// copied from ben-gen
fn hand_pbn(hand: Hand) -> String {
    use core::fmt::Write;
    let mut pbn = String::with_capacity(20);
    for (index, suit) in [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs]
        .into_iter()
        .enumerate()
    {
        if index > 0 {
            pbn.push('.');
        }
        write!(pbn, "{}", hand[suit]).expect("writing to a String never fails");
    }
    pbn
}

/// One hidden hand's ground truth: HCP, the upgraded [`point_count`] scale
/// our `points` bands are denominated in (raw HCP + long-suit bonus, floored;
/// fit-known `support_points` can still legitimately exceed it on raises),
/// and suit lengths in [`Suit::ASC`] order (clubs, diamonds, hearts, spades)
/// — the same order [`Inference::lengths`] uses.
#[derive(serde::Serialize)]
struct Truth {
    hcp: u8,
    points: u8,
    lengths: [usize; 4],
}

/// The three hidden seats from the actor's view, in BEN's Info-net order
#[derive(serde::Serialize)]
struct Hidden<T> {
    lho: T,
    partner: T,
    rho: T,
}

/// One decision point: the actor's hand and auction so far, our reading
/// layer's inference per hidden seat, and the truth to score against
#[derive(serde::Serialize)]
struct Row {
    board: usize,
    /// N/E/S/W of the dealer and the seat to act
    dealer: char,
    seat: char,
    vul_ns: bool,
    vul_ew: bool,
    /// The actor's 13 cards, BEN PBN (`S.H.D.C`)
    hand: String,
    /// The `prefix`-long auction so far, BEN tokens, dealer first
    auction: Vec<String>,
    prefix: usize,
    /// `points` is our upgraded scale (raw HCP only when balanced) — compare
    /// bands and ordering against BEN's HCP mean, not raw deltas
    ours: Hidden<Inference>,
    truth: Hidden<Truth>,
}

fn truth_of(hand: Hand) -> Truth {
    Truth {
        hcp: hand_hcp(hand),
        points: point_count(hand),
        lengths: Suit::ASC.map(|s| hand[s].len()),
    }
}

const SEAT_CHARS: [char; 4] = ['N', 'E', 'S', 'W'];

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    if args.sound_reading {
        pons::bidding::set_cue_reading(true);
        pons::bidding::set_length_soundness(true);
        pons::bidding::set_table_alert_reading(true);
        pons::bidding::set_pass_reading(true);
    }
    let stance = american().against(Family::NATURAL);
    let seed = args.seed.unwrap_or_else(rand::random);
    let mut rng = StdRng::seed_from_u64(seed);
    let vul = args.vulnerability;
    let mut out: Box<dyn Write> = match args.output.as_deref() {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout().lock()),
    };

    let mut rows = 0usize;
    for board in 0..args.count {
        let deal = full_deal(&mut rng);
        let dealer = Seat::ALL[board % 4];

        // Self-play: our floor at all four seats.
        let mut auction = Auction::new();
        while !auction.has_ended() {
            let seat = seat_to_act(dealer, auction.len());
            auction.push(next_call(&stance, deal[seat], dealer, vul, &auction));
        }

        // Every non-empty decision point (the empty prefix reads as all-unknown
        // on our side — no signal to compare).
        for prefix in 1..auction.len() {
            let actor = seat_to_act(dealer, prefix);
            let inferences = stance.infer(relative(vul, actor), &auction[..prefix]);
            let hidden_seat = |who: Relative| Seat::ALL[(actor as usize + who as usize) % 4];
            let row = Row {
                board,
                dealer: SEAT_CHARS[dealer as usize],
                seat: SEAT_CHARS[actor as usize],
                vul_ns: relative(vul, Seat::North).contains(RelativeVulnerability::WE),
                vul_ew: relative(vul, Seat::East).contains(RelativeVulnerability::WE),
                hand: hand_pbn(deal[actor]),
                auction: auction[..prefix].iter().map(|&c| call_token(c)).collect(),
                prefix,
                ours: Hidden {
                    lho: *inferences.lho(),
                    partner: *inferences.partner(),
                    rho: *inferences.rho(),
                },
                truth: Hidden {
                    lho: truth_of(deal[hidden_seat(Relative::Lho)]),
                    partner: truth_of(deal[hidden_seat(Relative::Partner)]),
                    rho: truth_of(deal[hidden_seat(Relative::Rho)]),
                },
            };
            serde_json::to_writer(&mut out, &row)?;
            writeln!(out)?;
            rows += 1;
        }
    }
    out.flush()?;
    eprintln!(
        "probe-ben-info: {} boards, {rows} rows, seed {seed}, vulnerability {vul}",
        args.count
    );
    Ok(())
}
