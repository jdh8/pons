//! Rule-replay vs range sampler yield (no double-dummy) — the cheap pre-check
//! before any DD A/B of [`set_rule_accept`][pons::bidding::inference::set_rule_accept].
//!
//! Fills `n` layouts per auction both ways; `replay fill` far below `range fill`
//! (ratio « 1) flags an auction where rule-replay starves and the ev.rs top-up
//! falls back to ranges.  Use it to tune the sampler's `MARGIN`.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{Bid, Level, Seat, Strain};
use pons::american;
use pons::bidding::Family;
use pons::bidding::context::Context;
use pons::bidding::inference::Inferences;
use pons::bidding::sampler::{sample_layouts, sample_layouts_replay};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid {
        level: Level::new(level),
        strain,
    })
}

fn main() {
    let policy = american().against(Family::NATURAL);
    let actor = Seat::North;
    let vul = RelativeVulnerability::NONE;
    let n = 50usize; // fill target; replay gets REPLAY_ATTEMPTS_PER_LAYOUT draws
    // each, so returned « n means it starved even with the big
    // budget (a near-infeasible auction).

    let auctions: &[(&str, Vec<Call>)] = &[
        (
            "partner 1H, RHO 2C",
            vec![bid(1, Strain::Hearts), bid(2, Strain::Clubs)],
        ),
        (
            "partner 1NT, RHO X",
            vec![bid(1, Strain::Notrump), Call::Double],
        ),
        (
            "LHO 1NT, partner 2C, RHO 2H",
            vec![
                bid(1, Strain::Notrump),
                bid(2, Strain::Clubs),
                bid(2, Strain::Hearts),
            ],
        ),
        (
            "p 1H, RHO 1S, me?, LHO 2S, p 3H, RHO P",
            vec![
                bid(1, Strain::Hearts),
                bid(1, Strain::Spades),
                Call::Pass,
                bid(2, Strain::Spades),
                bid(3, Strain::Hearts),
                Call::Pass,
            ],
        ),
    ];

    for (label, auction) in auctions {
        let context = Context::new(vul, auction);
        let inferences = Inferences::read(&context);
        let mut rng = StdRng::seed_from_u64(42);
        let seeds = 5;
        let (mut range_sum, mut replay_sum) = (0usize, 0usize);
        for _ in 0..seeds {
            let hand = full_deal(&mut rng)[actor];
            let r = sample_layouts(hand, actor, &inferences, &mut rng, n).len();
            let p = sample_layouts_replay(hand, actor, &policy, vul, auction, &mut rng, n).len();
            range_sum += r;
            replay_sum += p;
        }
        let range = 100.0 * range_sum as f64 / (seeds * n) as f64;
        let replay = 100.0 * replay_sum as f64 / (seeds * n) as f64;
        let ratio = if range > 0.0 { replay / range } else { 0.0 };
        println!(
            "{label:40}  range fill {range:5.1}%   replay fill {replay:5.1}%   ratio {ratio:4.2}"
        );
    }
}
