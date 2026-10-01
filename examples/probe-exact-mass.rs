//! Pins for the exact-mass counter (`common::mass`) — Phase 0 of
//! `docs/exact-posterior.md`.  Every pin is an assertion; a clean exit is a pass.
//!
//! 1. **Parity** — the counter's per-suit keys rebuild the crate's own
//!    `point_count` on every hand, and its membership agrees with
//!    [`EnvelopeUnion::contains`] on every sampled hand of pin 3.
//! 2. **Closed forms** — the unknown reading counts `C(39, 13)`; a lone length
//!    range is hypergeometric.
//! 3. **Monte Carlo** — on real self-play readings, the exact mass sits inside
//!    binomial noise of a sampled one (the retired `probe-exact-mass`
//!    criterion).
//!
//! No double-dummy, no solver.
//!
//! ```sh
//! cargo run --release --example probe-exact-mass
//! ```

use clap::Parser;
use contract_bridge::{AbsoluteVulnerability, Card, Hand, Seat, Suit};
use pons::american;
use pons::bidding::constraint::point_count;
use pons::bidding::context::relative;
use pons::bidding::inference::{Envelope, EnvelopeUnion, Range};
use pons::bidding::{Relative, agreements::Agreements};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::time::Instant;

#[path = "common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::mass::{Counter, TOTAL, holding_key, points};
use common::{bid_out, seat_to_act, seeded_deals};

#[derive(Parser)]
struct Args {
    /// Deals to bid
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Seed base — fixed by default, so the pins are deterministic
    #[arg(short, long, default_value = "1")]
    seed: u64,

    /// Monte Carlo draws per reading
    #[arg(long, default_value = "50000")]
    draws: u32,
}

fn choose(n: u64, k: u64) -> u64 {
    (0..k).fold(1, |acc, i| acc * (n - i) / (i + 1))
}

fn main() {
    let args = Args::parse();
    let deals = seeded_deals(args.seed, args.count);

    // 1. Gauge parity.
    for hand in deals.iter().flat_map(|deal| Seat::ALL.map(|s| deal[s])) {
        let keys = Suit::ASC.map(|suit| holding_key(hand[suit]));
        let shape = keys.map(|key| key.0 as u8);
        let (hcp, wasted) = keys.iter().fold((0, 0), |a, k| (a.0 + k.1, a.1 + k.2));
        assert_eq!(
            points(shape, hcp, wasted),
            usize::from(point_count(hand)),
            "{hand}"
        );
    }
    println!("gauge parity      {} hands", 4 * deals.len());

    // 2. Closed forms.
    for deal in &deals {
        let seen = deal[Seat::North];
        let counter = Counter::new(seen);
        assert_eq!(counter.count(&EnvelopeUnion::unknown()), TOTAL);
        assert_eq!(TOTAL, choose(39, 13));
        let unseen = 13 - seen[Suit::Spades].len() as u64;
        for (min, max) in [(0, 2), (5, 13), (4, 4)] {
            let mut envelope = Envelope::unknown();
            envelope.lengths[Suit::Spades as usize] = Range::new(min, max);
            let closed: u64 = (u64::from(min)..=u64::from(max).min(unseen))
                .map(|k| choose(unseen, k) * choose(39 - unseen, 13 - k))
                .sum();
            assert_eq!(
                counter.count(&envelope.into()),
                closed,
                "{seen} ♠{min}..={max}"
            );
        }
    }
    println!("closed forms      {} hands × 4 readings", deals.len());

    // 3. Monte Carlo on real readings, timed.
    let partnership = american(&Agreements::default()).bind();
    let vul = AbsoluteVulnerability::NONE;
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut zs = Vec::new();
    let (mut readings, mut worst, mut build, mut query) = (0_u32, (0.0_f64, 0.0, 0.0), 0.0, 0.0);
    for (board, deal) in deals.iter().enumerate() {
        let dealer = Seat::ALL[board % 4];
        let auction = bid_out(&partnership, &partnership, true, dealer, vul, deal);
        // The last decision node: the tightest readings the deal produces.
        let cut = auction.len() - 1;
        let seat = seat_to_act(dealer, cut);
        let read = partnership.infer(relative(vul, seat), &auction[..cut]);
        let start = Instant::now();
        let counter = Counter::new(deal[seat]);
        build += start.elapsed().as_secs_f64();
        let mut unseen: Vec<Card> = (!deal[seat]).into_iter().collect();
        for who in [Relative::Lho, Relative::Partner, Relative::Rho] {
            let union = read.announced_union(who);
            let start = Instant::now();
            let exact = counter.mass(union);
            query += start.elapsed().as_secs_f64();
            readings += 1;
            if exact == 0.0 || exact == 1.0 {
                continue; // no binomial noise to sit inside
            }
            let hits = (0..args.draws)
                .filter(|_| {
                    let (drawn, _) = unseen.partial_shuffle(&mut rng, 13);
                    let hand: Hand = drawn.iter().copied().collect();
                    let hit = union.contains(hand);
                    assert_eq!(common::mass::contains(union, hand), hit, "{hand} {union:?}");
                    hit
                })
                .count();
            let sampled = hits as f64 / f64::from(args.draws);
            let z = (sampled - exact) / (exact * (1.0 - exact) / f64::from(args.draws)).sqrt();
            zs.push(z);
            if z.abs() > worst.0 {
                worst = (z.abs(), exact, sampled);
            }
            assert!(z.abs() < 5.0, "exact {exact} sampled {sampled} z {z:.2}");
        }
    }
    println!(
        "monte carlo       {readings} readings, worst |z| {:.2} (exact {:.3e}, sampled {:.3e})",
        worst.0, worst.1, worst.2
    );
    // Exact means z is standard normal: a bias moves the mean, a wrong class
    // of hands fattens the spread.
    let mean = zs.iter().sum::<f64>() / zs.len() as f64;
    let sd = (zs.iter().map(|z| (z - mean).powi(2)).sum::<f64>() / zs.len() as f64).sqrt();
    println!(
        "                  z over {} sampled readings: mean {mean:+.3}, sd {sd:.3}",
        zs.len()
    );
    println!(
        "cost              build {:.0} µs, query {:.1} µs",
        1e6 * build / deals.len() as f64,
        1e6 * query / f64::from(readings),
    );
}
