//! Price a simple heart raise by a light responder opposite a Dutch 1H opener.

use std::collections::BTreeMap;

use clap::Parser;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Penalty, Seat, Strain, Suit,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::constraint::support_point_count;
use pons::bidding::{Family, System};
use pons::dutch;
use pons::scoring::{imps, ns_score_tricks};
use rand::SeedableRng;
use rand::rngs::StdRng;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;

#[derive(Parser)]
#[command(about = "Price light simple raises of a Dutch 1H opener")]
struct Args {
    /// Number of accepted deals to solve
    #[arg(short, long, default_value_t = 40_000)]
    count: usize,

    /// Seed for the reproducible rejection-sampling stream
    #[arg(long, default_value_t = 20_260_722)]
    seed: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum OpenerSlice {
    Minimum,
    Medium,
    Maximum,
}

impl OpenerSlice {
    const ALL: [Self; 3] = [Self::Minimum, Self::Medium, Self::Maximum];

    fn from_hcp(hcp: u8) -> Self {
        match hcp {
            10..=14 => Self::Minimum,
            15..=17 => Self::Medium,
            18..=20 => Self::Maximum,
            _ => unreachable!("a real Dutch 1H opener has 10-20 HCP"),
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Minimum => "10-14",
            Self::Medium => "15-17",
            Self::Maximum => "18-20",
        }
    }

    const fn raise_level(self) -> u8 {
        match self {
            Self::Minimum => 2,
            Self::Medium => 3,
            Self::Maximum => 4,
        }
    }
}

struct Sample {
    deal: FullDeal,
    opener_slice: OpenerSlice,
    support_points: u8,
}

#[derive(Clone, Copy, Default)]
struct Results {
    count: usize,
    imps: i64,
    raise_made: usize,
    one_heart_made: usize,
}

impl Results {
    fn record(&mut self, board_imps: i64, raise_made: bool, one_heart_made: bool) {
        self.count += 1;
        self.imps += board_imps;
        self.raise_made += usize::from(raise_made);
        self.one_heart_made += usize::from(one_heart_made);
    }

    fn mean_imps(self) -> f64 {
        self.imps as f64 / self.count as f64
    }

    fn percentage(self, made: usize) -> f64 {
        100.0 * made as f64 / self.count as f64
    }
}

fn main() {
    let args = Args::parse();
    assert!(args.count > 0, "--count must be positive");

    let stance = dutch().against(Family::NATURAL);
    let one_heart = Bid::new(1, Strain::Hearts);
    let mut rng = StdRng::seed_from_u64(args.seed);
    let mut attempts = 0usize;
    let mut samples = Vec::with_capacity(args.count);

    while samples.len() < args.count {
        attempts += 1;
        let deal = full_deal(&mut rng);
        let north = deal[Seat::North];
        let Some(logits) = stance.classify(north, RelativeVulnerability::NONE, &[]) else {
            continue;
        };
        let best = (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("opening logits contain no NaN"))
            .map(|(call, _)| call)
            .expect("opening logits are nonempty");
        if best != Call::Bid(one_heart) {
            continue;
        }

        let south = deal[Seat::South];
        if south[Suit::Hearts].len() < 3 {
            continue;
        }
        let support_points = support_point_count(south);
        if !(3..=5).contains(&support_points) {
            continue;
        }

        samples.push(Sample {
            deal,
            opener_slice: OpenerSlice::from_hcp(common::hand_hcp(north)),
            support_points,
        });
    }

    let deals: Vec<FullDeal> = samples.iter().map(|sample| sample.deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    println!(
        "Light simple-raise probe: {} accepted deals from {attempts} attempts, seed {}",
        samples.len(),
        args.seed
    );
    println!("North = real Dutch 1H opener; South = 3+ hearts and 3-5 support points");
    println!();
    println!("Sampled opener-slice masses");
    println!("{:>10} {:>9} {:>9}", "N HCP", "count", "share");
    for slice in OpenerSlice::ALL {
        let count = samples
            .iter()
            .filter(|sample| sample.opener_slice == slice)
            .count();
        println!(
            "{:>10} {:>9} {:>8.2}%",
            slice.label(),
            count,
            100.0 * count as f64 / samples.len() as f64
        );
    }

    let pass_contract = Contract {
        bid: one_heart,
        penalty: Penalty::Undoubled,
    };
    for (vul_label, vul) in [
        ("none", AbsoluteVulnerability::NONE),
        ("both", AbsoluteVulnerability::ALL),
    ] {
        let mut sliced: BTreeMap<(u8, OpenerSlice), Results> = BTreeMap::new();
        let mut marginal: BTreeMap<u8, Results> = BTreeMap::new();
        let mut all = Results::default();

        for (sample, table) in samples.iter().zip(&tables) {
            let tricks = u8::from(table[Strain::Hearts].get(Seat::North))
                .max(u8::from(table[Strain::Hearts].get(Seat::South)));
            let raise_level = sample.opener_slice.raise_level();
            let raise_contract = Contract {
                bid: Bid::new(raise_level, Strain::Hearts),
                penalty: Penalty::Undoubled,
            };
            let ns = ns_score_tricks(raise_contract, Seat::North, tricks, vul);
            let np = ns_score_tricks(pass_contract, Seat::North, tricks, vul);
            let board_imps = imps(ns - np);
            let raise_made = tricks >= raise_level + 6;
            let one_heart_made = tricks >= 7;

            sliced
                .entry((sample.support_points, sample.opener_slice))
                .or_default()
                .record(board_imps, raise_made, one_heart_made);
            marginal.entry(sample.support_points).or_default().record(
                board_imps,
                raise_made,
                one_heart_made,
            );
            all.record(board_imps, raise_made, one_heart_made);
        }

        println!();
        println!("=== Vulnerability: {vul_label} ===");
        println!("Raise minus pass, plain DD scoring");
        println!(
            "{:>11} {:>10} {:>9} {:>12} {:>12} {:>12}",
            "support pts", "N HCP", "N", "mean IMPs", "P(raise)", "P(1H)"
        );
        for support_points in 3..=5 {
            for slice in OpenerSlice::ALL {
                let results = sliced
                    .get(&(support_points, slice))
                    .copied()
                    .unwrap_or_default();
                println!(
                    "{:>11} {:>10} {:>9} {:>+12.4} {:>11.1}% {:>11.1}%",
                    support_points,
                    slice.label(),
                    results.count,
                    results.mean_imps(),
                    results.percentage(results.raise_made),
                    results.percentage(results.one_heart_made),
                );
            }
        }

        println!();
        println!("Mass-weighted EV by support points");
        println!("{:>11} {:>9} {:>12}", "support pts", "N", "mean IMPs");
        for support_points in 3..=5 {
            let results = marginal.get(&support_points).copied().unwrap_or_default();
            println!(
                "{:>11} {:>9} {:>+12.4}",
                support_points,
                results.count,
                results.mean_imps()
            );
        }

        println!();
        println!(
            "*** HEADLINE: lowering the raise floor to 3-5 support points = {:+.4} IMPs/board over {} light boards ({vul_label}) ***",
            all.mean_imps(),
            all.count
        );
    }
}
