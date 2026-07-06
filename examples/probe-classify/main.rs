//! One-hand classification debugger: print every finite-logit call and the
//! provenance for a hand at an auction, under the default `american()` books.
//!
//! `cargo run --example probe-classify -- --hand "KQ8.A9762.A76.T6" \
//!     --auction "P 1H P 3H P"`

use clap::Parser;
use contract_bridge::Hand;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american;
use pons::bidding::Family;
use pons::bidding::context::relative;

/// Print logits + provenance for one hand at one auction
#[derive(Parser)]
struct Args {
    /// The hand, spades first, dot-separated
    #[arg(long)]
    hand: String,

    /// Space-separated calls from the dealer (e.g. "P 1H P 3H P")
    #[arg(long)]
    auction: String,

    /// Vulnerability: none, ns, ew, both
    #[arg(long, default_value = "both")]
    vulnerability: AbsoluteVulnerability,

    /// Disable the (shipped default-on) competitive long-suit rebid floor
    #[arg(long, default_value_t = false)]
    no_competitive_rebid: bool,
}

fn main() {
    let args = Args::parse();
    pons::bidding::instinct::set_competitive_rebid(!args.no_competitive_rebid);
    let hand: Hand = args.hand.parse().expect("valid hand");
    let mut auction = Auction::new();
    for token in args.auction.split_whitespace() {
        let call: Call = token.parse().expect("valid call");
        auction.push(call);
    }

    let stance = american().against(Family::NATURAL);
    let seat = Seat::ALL[auction.len() % 4];
    let vul = relative(args.vulnerability, seat);
    match stance.classify_with_provenance(hand, vul, &auction) {
        None => println!("no classification (auction off-book, floor rejected)"),
        Some((logits, provenance)) => {
            println!("provenance: {provenance:?}");
            let mut scored: Vec<(Call, f32)> = logits
                .iter()
                .map(|(call, &logit)| (call, logit))
                .filter(|&(_, l)| l.is_finite())
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("no NaN"));
            for (call, logit) in scored {
                println!("  {call}  {logit:+.3}");
            }
        }
    }
}
