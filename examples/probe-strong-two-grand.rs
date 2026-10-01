//! What decides thirteen tricks after a positive to our strong `2♣`?
//!
//! The grand rung of the strong `2♣` lane (docs/next-steps.md item 2) needs a
//! gate, and king-counting was measured too strict: against BBA the lane's
//! whole deficit was missed grands.  This probe bids uncontested self-play
//! deals with the default system (`--classic` disarms
//! `rebid.strong_two_grand`, the rung it priced), keeps the auctions that start `2♣ -
//! positive`, solves them double dummy, and prints one JSON line per board —
//! the deal, the dealer, the auction, and North's and South's tricks in every
//! strain (♣ ♦ ♥ ♠ NT) — for an offline cut by keycards, kings, fit and
//! points.
//!
//! ```sh
//! cargo run --release --example probe-strong-two-grand -- -c 4000000 >lane.jsonl
//! ```

use clap::Parser;
use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, Bid, Seat, Strain};
use pons::american;
use rayon::prelude::*;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{auction_key, bid_uncontested, hand_hcp, seeded_deals};

#[derive(Parser)]
struct Args {
    /// Deals to draw (about one in 250 reaches the lane)
    #[arg(short, long, default_value = "4000000")]
    count: usize,

    /// Seed base; random when omitted
    #[arg(short, long)]
    seed: Option<u64>,

    /// Bid with `rebid.strong_two_grand` off (same seed as a default run to
    /// compare the two)
    #[arg(long)]
    classic: bool,
}

fn main() {
    let args = Args::parse();
    let base = args.seed.unwrap_or_else(rand::random);
    let vul = AbsoluteVulnerability::NONE;
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.rebid.strong_two_grand = !args.classic;
    let partnership = american(&agreements).bind();
    let two_clubs = Call::Bid(Bid::new(2, Strain::Clubs));

    let deals = seeded_deals(base, args.count);
    let lane: Vec<(usize, String)> = deals
        .par_iter()
        .enumerate()
        .filter_map(|(index, deal)| {
            // A `2♣` opening is 22+ points, and length adds at most two.
            if hand_hcp(deal[Seat::North]).max(hand_hcp(deal[Seat::South])) < 19 {
                return None;
            }
            let dealer = Seat::ALL[index % 4];
            let auction = bid_uncontested(&partnership, dealer, vul, deal);
            let calls: Vec<Call> = auction
                .iter()
                .copied()
                .skip_while(|&call| call == Call::Pass)
                .collect();
            let positive = calls.first() == Some(&two_clubs)
                && matches!(calls.get(2), Some(&Call::Bid(bid))
                    if bid > Bid::new(2, Strain::Diamonds));
            positive.then(|| {
                (
                    index,
                    auction_key(&auction.iter().copied().collect::<Vec<_>>()),
                )
            })
        })
        .collect();

    let solve: Vec<_> = lane.iter().map(|&(index, _)| deals[index]).collect();
    let tables = ddss::Solver::lock(None).solve_deals(&solve, ddss::NonEmptyStrainFlags::ALL);
    for ((index, auction), table) in lane.iter().zip(&tables) {
        let tricks = |seat: Seat| {
            Strain::ASC
                .map(|strain| u8::from(table[strain].get(seat)).to_string())
                .join(",")
        };
        println!(
            r#"{{"deal":"{}","dealer":"{:?}","auction":"{auction}","n":[{}],"s":[{}]}}"#,
            deals[*index],
            Seat::ALL[index % 4],
            tricks(Seat::North),
            tricks(Seat::South),
        );
    }
}
