//! Throwaway: `smoke-default`'s dump with Gladiator armed, bid **serially**
//!
//! The Gladiator port moves two guard-carrying entries (the `(2♣)` rebase and
//! the stolen-relay transplant) from `fallback_all_seats` onto the row layer.
//! `render-book` prints only a guard's label, so it cannot see a guard whose
//! *behaviour* changed — only bidding can.  Serial on purpose: an armed
//! thread-local knob does not cross into rayon workers.  Delete after the batch
//! is blessed.

use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american;
use pons::bidding::american::set_nt_overcall_gladiator;
use std::io::{BufWriter, Write};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act, seeded_deals};

fn main() {
    set_nt_overcall_gladiator(true);
    let stance = american().against();
    let mut out = BufWriter::new(std::io::stdout().lock());
    for (index, deal) in seeded_deals(1, 20000).into_iter().enumerate() {
        let dealer = Seat::ALL[index % 4];
        let vul = [
            AbsoluteVulnerability::NONE,
            AbsoluteVulnerability::NS,
            AbsoluteVulnerability::EW,
            AbsoluteVulnerability::ALL,
        ][(index / 4) % 4];
        let mut auction = Auction::new();
        while !auction.has_ended() {
            let seat = seat_to_act(dealer, auction.len());
            auction.push(next_call(&stance, deal[seat], dealer, vul, &auction));
        }
        let calls: Vec<String> = auction.iter().map(|call| format!("{call}")).collect();
        writeln!(out, "{index}\t{}", calls.join(" ")).expect("stdout");
    }
}
