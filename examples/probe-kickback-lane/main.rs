//! Walk one keycard auction under the Kickback partnership, reporting at every seat what
//! the bidder chose, what else it considered, and — the question a phantom
//! contract always turns on — whether the position is **authored** or the floor.
//!
//! A book node with finite mass shadows the floor, so "which layer answered
//! this call" is not a detail: it decides whether a `-∞` elsewhere in the
//! logits means "this hand does not bid that" or merely "nobody had an
//! opinion".  `--baseline` re-walks the same deal without the relocation, so
//! the two ladders can be read side by side.

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Hand, Seat};
use pons::bidding::agreements::Agreements;
use pons::bidding::context::relative;
use pons::bidding::instinct::RkcbVariant;
use pons::bidding::{Bidder, Partnership, american};
use pons::scoring::final_contract;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act};

#[derive(Parser)]
struct Args {
    /// The four hands, clockwise from North, in PBN holding order
    #[arg(long, num_args = 4, required = true)]
    hands: Vec<String>,
    /// Dealer
    #[arg(long, default_value = "West")]
    dealer: String,
    /// Walk the un-relocated ladder instead
    #[arg(long)]
    baseline: bool,
    /// How many candidate calls to print per seat
    #[arg(long, default_value = "6")]
    top: usize,
}

const fn variant(kickback: bool) -> RkcbVariant {
    if kickback {
        RkcbVariant::Kickback
    } else {
        RkcbVariant::Plain
    }
}

fn agreements(kickback: bool) -> Agreements {
    let mut agreements = Agreements::default();
    agreements.decision.reading.rkcb_variant = variant(kickback);
    agreements.decision.instinct.keycard_minors = true;
    agreements
}

fn main() {
    let args = Args::parse();
    let kickback = !args.baseline;
    let partnership: Partnership = american(&agreements(kickback)).bind();

    let hands: Vec<Hand> = args
        .hands
        .iter()
        .map(|h| h.parse().expect("a PBN holding like AKQ3.AQ42.QT72.A"))
        .collect();
    let dealer = match args.dealer.as_str() {
        "North" => Seat::North,
        "East" => Seat::East,
        "South" => Seat::South,
        _ => Seat::West,
    };
    let vul = AbsoluteVulnerability::empty();

    println!(
        "ladder: {}",
        if kickback { "kickback" } else { "plain 4NT" }
    );
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let hand = hands[seat as usize];
        let authored = partnership.authored_at(relative(vul, seat), &auction);
        let call = next_call(&partnership, hand, dealer, vul, &auction);

        let mut top: Vec<(Call, f32)> = partnership
            .classify(hand, relative(vul, seat), &auction)
            .map(|logits| {
                logits
                    .iter()
                    .map(|(c, &l)| (c, l))
                    .filter(|&(c, l)| l.is_finite() && auction.can_push(c).is_ok())
                    .collect()
            })
            .unwrap_or_default();
        top.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
        top.truncate(args.top);
        let shown: Vec<String> = top.iter().map(|(c, l)| format!("{c:?}@{l:.2}")).collect();

        println!(
            "  {seat:?} -> {call:?}   [{}]   {}",
            if authored { "AUTHORED" } else { "floor" },
            shown.join(" ")
        );
        auction.push(call);
    }
    println!("contract: {:?}", final_contract(&auction, dealer));
}
