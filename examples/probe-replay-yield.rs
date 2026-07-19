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
use std::time::{Duration, Instant};

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
    let n = 50usize; // fill target; replay draws until REPLAY_DRAW_CAP (or gives
    // up early on REPLAY_DRY_LIMIT), so returned « n means the
    // auction starved even with the big budget.  A flat 0% is
    // usually *infeasible*, not starved: some authored node
    // pins the call at −∞ while keeping mass elsewhere.

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
        // Search's home is constructive reach, so the tight long auctions matter
        // most: every extra round narrows partner further (Phase 1c's target).
        (
            "p 1NT, me 2D transfer, p 2H, me?",
            vec![
                bid(1, Strain::Notrump),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
            ],
        ),
        (
            "p 1S, me 2C GF, p 2D, me 2H, p 3C, me?",
            vec![
                bid(1, Strain::Spades),
                Call::Pass,
                bid(2, Strain::Clubs),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
                bid(2, Strain::Hearts),
                Call::Pass,
                bid(3, Strain::Clubs),
                Call::Pass,
            ],
        ),
        (
            "p 2NT, me 3C Stayman, p 3S, me 4NT RKCB, p 5H, me?",
            vec![
                bid(2, Strain::Notrump),
                Call::Pass,
                bid(3, Strain::Clubs),
                Call::Pass,
                bid(3, Strain::Spades),
                Call::Pass,
                bid(4, Strain::Notrump),
                Call::Pass,
                bid(5, Strain::Hearts),
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
        let (mut range_time, mut replay_time) = (Duration::ZERO, Duration::ZERO);
        for _ in 0..seeds {
            let hand = full_deal(&mut rng)[actor];
            let start = Instant::now();
            let r = sample_layouts(hand, actor, &inferences, &mut rng, n).len();
            range_time += start.elapsed();
            let start = Instant::now();
            let p =
                sample_layouts_replay(hand, actor, &policy, vul, auction, &inferences, &mut rng, n)
                    .len();
            replay_time += start.elapsed();
            range_sum += r;
            replay_sum += p;
        }
        let range = 100.0 * range_sum as f64 / (seeds * n) as f64;
        let replay = 100.0 * replay_sum as f64 / (seeds * n) as f64;
        let ratio = if range > 0.0 { replay / range } else { 0.0 };
        // Cost per *kept* world — what a weighted dealer (Phase 1c) would cut.
        let per = |time: Duration, kept: usize| {
            if kept == 0 {
                f64::INFINITY
            } else {
                time.as_secs_f64() * 1e6 / kept as f64
            }
        };
        println!(
            "{label:40}  range fill {range:5.1}% {:8.0}µs/world   replay fill {replay:5.1}% {:9.0}µs/world   ratio {ratio:4.2}",
            per(range_time, range_sum),
            per(replay_time, replay_sum)
        );
    }
}
