//! Distribution of `points` and `support_points` over Dutch's real 1♠ openers.
//!
//! No double dummy. Samples random hands, classifies each as a first-seat
//! opener with an empty auction, keeps the ones Dutch actually bids 1♠ (the
//! whole book — not just the opening table's own gate, so rows shadowed by a
//! higher-priority rule are excluded too), and histograms both point scales
//! over that set.
//!
//! ```text
//! cargo run --release --example probe-dutch-1s-points -- 500000 0
//! ```
//! Args (positional, optional): hand `count` (default 500,000), `seed` (default 0).

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{Bid, Hand, Seat, Strain};
use pons::bidding::constraint::{point_count, support_point_count};
use pons::{Family, System, dutch};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn opens_one_spade(stance: &pons::Stance, hand: Hand) -> bool {
    let Some(logits) = stance.classify(hand, RelativeVulnerability::NONE, &[]) else {
        return false;
    };
    let best = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call);
    best == Some(Call::Bid(Bid::new(1, Strain::Spades)))
}

fn histogram(label: &str, values: &[u8]) {
    let lo = *values.iter().min().expect("non-empty");
    let hi = *values.iter().max().expect("non-empty");
    #[allow(clippy::cast_precision_loss)]
    let mean = values.iter().map(|&v| f64::from(v)).sum::<f64>() / values.len() as f64;
    println!(
        "\n{label} (n={}, mean={mean:.2}, range {lo}..={hi})",
        values.len()
    );
    let peak = (lo..=hi)
        .map(|v| values.iter().filter(|&&x| x == v).count())
        .max()
        .unwrap_or(1)
        .max(1);
    for v in lo..=hi {
        let count = values.iter().filter(|&&x| x == v).count();
        let bar = "#".repeat(count * 60 / peak);
        println!("{v:3} {count:6}  {bar}");
    }
}

fn main() {
    let mut argv = std::env::args().skip(1);
    let count: usize = argv.next().and_then(|s| s.parse().ok()).unwrap_or(500_000);
    let seed: u64 = argv.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    let stance = dutch().against(Family::NATURAL);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut points = Vec::new();
    let mut support = Vec::new();
    for _ in 0..count {
        let deal = full_deal(&mut rng);
        for seat in [Seat::North, Seat::East, Seat::South, Seat::West] {
            let hand = deal[seat];
            if opens_one_spade(&stance, hand) {
                points.push(point_count(hand));
                support.push(support_point_count(hand));
            }
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let rate = 100.0 * points.len() as f64 / (4 * count) as f64;
    println!(
        "{count} deals, {} 1♠ openers found ({rate:.2}% of hands)",
        points.len()
    );
    histogram("points", &points);
    histogram("support_points", &support);
}
