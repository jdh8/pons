//! Locate the responder support-point threshold for game opposite a maximum
//! Dutch 1H opener with a heart fit.

use std::collections::BTreeMap;

use clap::Parser;
use contract_bridge::deck::full_deal;
use contract_bridge::{FullDeal, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::constraint::support_point_count;
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

#[derive(Parser)]
#[command(about = "Survey 4H make-rates opposite a maximum Dutch 1H opener")]
struct Args {
    /// Number of accepted deals to solve
    #[arg(short, long, default_value_t = 20_000)]
    count: usize,

    /// Seed for the reproducible rejection-sampling stream
    #[arg(long, default_value_t = 20_260_722)]
    seed: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum HeartLength {
    Three,
    Four,
    FivePlus,
}

impl HeartLength {
    fn from_len(len: usize) -> Self {
        match len {
            3 => Self::Three,
            4 => Self::Four,
            5.. => Self::FivePlus,
            _ => unreachable!("accepted responder has at least three hearts"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Three => "3",
            Self::Four => "4",
            Self::FivePlus => "5+",
        }
    }
}

#[derive(Default)]
struct Results {
    count: usize,
    hearts_made: usize,
    notrump_made: usize,
}

fn percentage(made: usize, count: usize) -> f64 {
    100.0 * made as f64 / count as f64
}

fn main() {
    let args = Args::parse();
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut attempts = 0usize;
    let mut samples: Vec<(FullDeal, (HeartLength, u8))> = Vec::with_capacity(args.count);

    while samples.len() < args.count {
        attempts += 1;
        let deal = full_deal(&mut rng);
        let north = deal[Seat::North];
        if !(18..=20).contains(&common::hand_hcp(north)) || north[Suit::Hearts].len() < 5 {
            continue;
        }

        let south = deal[Seat::South];
        let heart_len = south[Suit::Hearts].len();
        if heart_len < 3 {
            continue;
        }
        let support_points = support_point_count(south);
        if support_points > 11 {
            continue;
        }

        samples.push(((deal), (HeartLength::from_len(heart_len), support_points)));
    }

    let deals: Vec<FullDeal> = samples.iter().map(|&(deal, _)| deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut sliced: BTreeMap<(HeartLength, u8), Results> = BTreeMap::new();
    let mut marginal: BTreeMap<u8, Results> = BTreeMap::new();
    for ((_, (heart_len, support_points)), table) in samples.into_iter().zip(tables) {
        let hearts = u8::from(table[Strain::Hearts].get(Seat::North))
            .max(u8::from(table[Strain::Hearts].get(Seat::South)));
        let notrump = u8::from(table[Strain::Notrump].get(Seat::North))
            .max(u8::from(table[Strain::Notrump].get(Seat::South)));

        for results in [
            sliced.entry((heart_len, support_points)).or_default(),
            marginal.entry(support_points).or_default(),
        ] {
            results.count += 1;
            results.hearts_made += usize::from(hearts >= 10);
            results.notrump_made += usize::from(notrump >= 9);
        }
    }

    println!(
        "Maximum Dutch 1H opener (North), heart-fit responder (South): {} accepted deals from {attempts} attempts, seed {}",
        args.count, args.seed
    );
    println!();
    println!(
        "{:>9} {:>14} {:>8} {:>9} {:>10}",
        "S hearts", "support pts", "count", "P(4H)", "P(3NT)"
    );
    for (&(heart_len, support_points), results) in &sliced {
        println!(
            "{:>9} {:>14} {:>8} {:>8.1}% {:>9.1}%",
            heart_len.label(),
            support_points,
            results.count,
            percentage(results.hearts_made, results.count),
            percentage(results.notrump_made, results.count),
        );
    }

    println!();
    println!("P(4H) by support points, all heart lengths");
    println!("{:>14} {:>8} {:>9}", "support pts", "count", "P(4H)");
    for (&support_points, results) in &marginal {
        println!(
            "{:>14} {:>8} {:>8.1}%",
            support_points,
            results.count,
            percentage(results.hearts_made, results.count),
        );
    }

    for threshold in [0.50, 0.40] {
        let first = marginal.iter().find_map(|(&support_points, results)| {
            ((results.hearts_made as f64 / results.count as f64) >= threshold)
                .then_some(support_points)
        });
        match first {
            Some(support_points) => println!(
                "Smallest support-point count with P(4H) >= {:.0}%: {support_points}",
                100.0 * threshold
            ),
            None => println!(
                "Smallest support-point count with P(4H) >= {:.0}%: none",
                100.0 * threshold
            ),
        }
    }
}
