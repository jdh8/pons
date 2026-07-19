//! How often does a book node actually get *reached*?
//!
//! The ranking key for a book audit. The 2/1 game backstop was worth a
//! headline knob because it stood in for a whole subtree; a crude node at one
//! specific key is worth roughly nothing no matter how crude it is, because
//! nobody ever bids there. Crudeness is cheap to eyeball and misleading —
//! measure reach first, then argue about quality.
//!
//! Bids `-c` uncontested self-play deals and counts, per candidate key, how
//! many auctions pass *through* that node (our side's call sequence has the key
//! as a proper prefix, so somebody actually had to choose a call there).
//!
//! ```sh
//! cargo run --release --example probe-node-reach -- -c 200000
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Bid, Level, Seat, Strain};
use pons::american;
use pons::bidding::Family;
use rayon::prelude::*;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_uncontested, seat_to_act, seeded_deals};

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,
}

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let stance = american().against(Family::NATURAL);

    // The constructive book re-audit candidates (ben-gap-campaign.md), plus the
    // retired game backstop's own anchor as the calibration yardstick: it fired
    // 1.15% on the A/B harness, and anything an order of magnitude below that
    // cannot pay for a real-routing run however crude it looks.
    let keys: &[(&str, Vec<Call>)] = &[
        (
            "yardstick: 1S-2C (backstop anchor)",
            vec![bid(1, Strain::Spades), bid(2, Strain::Clubs)],
        ),
        (
            "#1 1S-2C-2D-3D opener_third_agree",
            vec![
                bid(1, Strain::Spades),
                bid(2, Strain::Clubs),
                bid(2, Strain::Diamonds),
                bid(3, Strain::Diamonds),
            ],
        ),
        (
            "#2 1S-2C-2D-3S opener_third",
            vec![
                bid(1, Strain::Spades),
                bid(2, Strain::Clubs),
                bid(2, Strain::Diamonds),
                bid(3, Strain::Spades),
            ],
        ),
        (
            "#3 1D-1S-1NT-2C-2D-2NT xyz accept_or_decline",
            vec![
                bid(1, Strain::Diamonds),
                bid(1, Strain::Spades),
                bid(1, Strain::Notrump),
                bid(2, Strain::Clubs),
                bid(2, Strain::Diamonds),
                bid(2, Strain::Notrump),
            ],
        ),
        (
            "#5 2S-2NT-3S asker_after_max_major",
            vec![
                bid(2, Strain::Spades),
                bid(2, Strain::Notrump),
                bid(3, Strain::Spades),
            ],
        ),
        (
            "#6 2C-2D-2S-3S opener_after_spades_raise",
            vec![
                bid(2, Strain::Clubs),
                bid(2, Strain::Diamonds),
                bid(2, Strain::Spades),
                bid(3, Strain::Spades),
            ],
        ),
        (
            "#7 1NT-2NT-3C-3D pass_out",
            vec![
                bid(1, Strain::Notrump),
                bid(2, Strain::Notrump),
                bid(3, Strain::Clubs),
                bid(3, Strain::Diamonds),
            ],
        ),
    ];

    let deals = seeded_deals(base, args.count);
    let counts = deals
        .par_iter()
        .enumerate()
        .map(|(index, deal)| {
            let dealer = Seat::ALL[index % 4];
            let auction = bid_uncontested(&stance, dealer, vul, deal);
            // East/West passed throughout, so our side's calls are exactly the
            // ones made from a North/South seat — the book key's alphabet.
            let ours: Vec<Call> = auction
                .iter()
                .enumerate()
                .filter(|&(i, _)| matches!(seat_to_act(dealer, i), Seat::North | Seat::South))
                .map(|(_, &c)| c)
                .collect();
            keys.iter()
                .map(|(_, key)| usize::from(ours.len() > key.len() && ours.starts_with(key)))
                .collect::<Vec<_>>()
        })
        .reduce(
            || vec![0usize; keys.len()],
            |mut a, b| {
                for (x, y) in a.iter_mut().zip(b) {
                    *x += y;
                }
                a
            },
        );

    println!("=== node reach: {} deals, seed {base} ===", args.count);
    for ((label, _), n) in keys.iter().zip(&counts) {
        #[allow(clippy::cast_precision_loss)]
        let pct = 100.0 * *n as f64 / args.count as f64;
        println!("{label:44}  {n:8}  {pct:7.4}%");
    }
}
