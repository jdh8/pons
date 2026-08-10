//! One-hand classification debugger: print every finite-logit call and the
//! provenance for a hand at an auction, under the default `american()` books.
//!
//! `cargo run --example probe-classify -- --hand "KQ8.A9762.A76.T6" \
//!     --auction "- 1H - 3H -"`

use clap::Parser;
use contract_bridge::Hand;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Seat};
use pons::american;
use pons::bidding::context::relative;

/// Print logits + provenance for one hand at one auction
#[derive(Parser)]
struct Args {
    /// The hand, spades first, dot-separated
    #[arg(long)]
    hand: String,

    /// Space-separated calls from the dealer (e.g. "- 1H - 3H -")
    #[arg(long)]
    auction: String,

    /// Vulnerability: none, ns, ew, both
    #[arg(long, default_value = "both")]
    vulnerability: AbsoluteVulnerability,

    /// Disable the (shipped default-on) competitive long-suit rebid floor
    #[arg(long, default_value_t = false)]
    no_competitive_rebid: bool,

    /// Classify under the legacy bounding-box hull reading
    /// (`ReadingProfile::envelope_union` off; the crate default is ON since chop F2b) — the forensic view of
    /// what the flip changes at this node
    #[arg(long, default_value_t = false)]
    no_envelope_union: bool,
}

fn main() {
    let args = Args::parse();
    let hand: Hand = args.hand.parse().expect("valid hand");
    let mut auction = Auction::new();
    for token in args.auction.split_whitespace() {
        let call: Call = token.parse().expect("valid call");
        auction.push(call);
    }

    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.instinct.competitive_rebid = !args.no_competitive_rebid;
    agreements.decision.reading.envelope_union = !args.no_envelope_union;
    let partnership = american(&agreements).bind();
    let seat = Seat::ALL[auction.len() % 4];
    let vul = relative(args.vulnerability, seat);
    // The prefixed reading — what the bidder actually sees (a bare
    // `Context::new` skips the projection overlay; see `Inferences::read`).
    println!(
        "inferences via Partnership::infer (envelope_union={}):\n{:#?}",
        !args.no_envelope_union,
        partnership.infer(vul, &auction)
    );
    match partnership.classify_with_provenance(hand, vul, &auction) {
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
