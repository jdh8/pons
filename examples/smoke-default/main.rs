//! Throwaway: dump the **shipped default** system's auctions on seeded deals.
//!
//! Built at two commits and diffed, this answers exactly one question — did a
//! refactor move a bid in the default system?  Nothing here is measurement; it
//! is a byte-identity check, so it takes no knobs and no arms on purpose.
//!
//! ```text
//! cargo run --release --example smoke-default -- --count 20000 --seed 1
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american_default;
use rayon::prelude::*;
use std::io::{BufWriter, Write};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{next_call, seat_to_act, seeded_deals};

#[derive(Parser)]
struct Args {
    /// Number of boards to bid out
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Deal seed base (board i seeded base+i)
    #[arg(long, default_value = "1")]
    seed: u64,
}

fn main() {
    let args = Args::parse();
    let partnership = american_default().bind();
    // Rayon is safe here because a built partnership pins the knob state it was
    // built under: the workers read the partnership, never their own thread-locals.
    // A harness that arms a *non*-default knob does the same — arm, build,
    // hand the partnership to the workers.
    //
    // Collected in board order before printing, so the dump is byte-stable
    // across runs and thread counts.  That is the whole point of the file.
    let lines: Vec<String> = seeded_deals(args.seed, args.count)
        .into_par_iter()
        .enumerate()
        .map(|(index, deal)| {
            // Rotate the dealer and the vulnerability on decorrelated periods
            // (4 × 4 = 16 boards per full cycle) so the dump reaches every
            // dealer × vulnerability cell.  In lockstep the dealer's side was
            // never vulnerable-against-not, and a default-path bid change
            // confined to unfavorable first- or third-chair decisions would
            // have escaped the byte-identity check.
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
                auction.push(next_call(&partnership, deal[seat], dealer, vul, &auction));
            }
            let calls: Vec<String> = auction.iter().map(|call| format!("{call}")).collect();
            format!("{index}\t{}", calls.join(" "))
        })
        .collect();
    let mut out = BufWriter::new(std::io::stdout().lock());
    for line in lines {
        writeln!(out, "{line}").expect("stdout");
    }
}
