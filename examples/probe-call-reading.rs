//! What does one call actually *read* as, on the surface the nets are handed?
//!
//! `probe-reading-census` counts how many of the five axes are ⊤; this prints
//! the ranges themselves for one auction, so a key off the census's worklist can
//! be inspected directly. Reads through [`Stance::infer`] and reports the
//! `announced()` envelope — exactly what `features::push_inference` encodes.
//!
//! Each argument is an auction; the reading shown is of its **last** call, from
//! the perspective of the seat about to act.
//!
//! ```sh
//! cargo run --release --example probe-call-reading -- "2H P" "2H X" "2H 2N"
//! ```
//!
//! The three above read `⊤ ⊤ ⊤ ⊤ ⊤`, `points 12..` (shape ⊤), and `⊤ ⊤ ⊤ ⊤ ⊤`
//! — even though the 2NT overcall is authored `hcp(15..=18) & balanced() &
//! stopper_in_their_suits()`. `project_authored` decodes **alerted calls only**,
//! and the natural walk reads length off a bid *suit*, so an unalerted notrump
//! overcall reaches the nets as nothing at all. Contrast `1N` (a book node:
//! `points 15..=18`, `♣ 2..=6 ♦ 2..=6 ♥ 2..=5 ♠ 2..=5`).

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Suit};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Envelope, Relative};

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::seat_to_act;

#[derive(Parser)]
struct Args {
    /// Auctions to read, e.g. `"2H P"`; the last call of each is the one read
    auctions: Vec<String>,

    /// Author the weak-two pass gate (`set_weak_two_pass_gate`, default off)
    #[arg(long, default_value_t = false)]
    weak_two_pass_gate: bool,

    /// Turn on the three weak-two defense candidates at once: the wider 2NT
    /// shape, the sub-3NT jump overcall, and the Michaels cue
    #[arg(long, default_value_t = false)]
    weak_two_v2: bool,
}

fn render(shown: &Envelope) -> String {
    let mut out = format!("points {:?}", shown.strength.points);
    for suit in Suit::ASC {
        out += &format!("  {suit} {:?}", shown.length(suit));
    }
    out
}

fn main() {
    let args = Args::parse();
    pons::bidding::american::set_weak_two_pass_gate(args.weak_two_pass_gate);
    pons::bidding::american::set_weak_two_notrump_shape(args.weak_two_v2);
    pons::bidding::american::set_weak_two_jump_overcall(args.weak_two_v2);
    pons::bidding::american::set_weak_two_cue(args.weak_two_v2);
    let vul = AbsoluteVulnerability::NONE;
    let stance = american().against();

    for text in &args.auctions {
        let auction: Vec<Call> = text
            .split_whitespace()
            .map(|call| call.parse().expect("a call"))
            .collect();
        // Read from the seat about to act, so the auction's last call is RHO's.
        let rel = relative(
            vul,
            seat_to_act(contract_bridge::Seat::North, auction.len()),
        );
        let read = stance.infer(rel, &auction);
        println!("{text:<12} {}", render(read.announced(Relative::Rho)));
    }
}
