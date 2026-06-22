//! Texas vs South African Texas — the double-dummy-visible difference, distilled.
//!
//! Both conventions land a 6-card-major game-but-not-slam responder in 4-of-the-
//! major opposite a strong 1NT.  With the slam machinery held identical, the
//! *only* DD-visible difference between them is **who declares 4M**:
//!
//! - **Texas** (and the crate's current default, which transfers `2♦/2♥` then
//!   raises): the 1NT opener always declares.
//! - **South African Texas**: responder may declare directly (`1NT–4♥/4♠`,
//!   "when declaring is not bad").
//!
//! The DD/perfect-defense scorer is blind to *concealment* — the textbook reason
//! to make the strong hand declare is a single-dummy effect.  So this can only
//! measure the residual: the **positional opening-lead swing** between the two
//! declarers (DD prices `4♥ by N` and `4♥ by S` differently because the opening
//! leader differs).  Averaged over random hands of the class that swing is
//! expected to be near zero; a clearly non-zero mean would be a real, if
//! second-order, finding.
//!
//! For each sampled deal we find every (responder = 6-card major + game-no-slam
//! values, partner = 15–17 balanced 1NT opener) configuration, solve the deal
//! once, and IMP `[4M by responder] − [4M by opener]`.  Positive favors South
//! African Texas (responder declares).  We split by whether responder is short
//! (singleton/void) — a candidate "declaring is not bad" trigger — to see if any
//! hand feature predicts when responder-declares gains.
//!
//! ```text
//! cargo run --release --example ab-texas-vs-sat -- --count 200000 --vulnerability none
//! ```
//
// ponytail: measures ONLY the DD positional declarer swing — concealment (the
// conventions' real point) is invisible to double-dummy, by design of the
// scorer.  The hand-class bands (GAME_FLOOR/SLAM_CEIL) are tunable knobs, not
// laws; widen them if the sample is thin.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use clap::Parser;
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Seat, Strain, Suit, eval,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::scoring::imps;

/// Texas vs South African Texas: the positional declarer A/B
#[derive(Parser)]
struct Args {
    /// Number of random deals to sample (qualifying configs are a small %)
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
}

const SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

/// Lowest responder HCP that forces game opposite a 15–17 1NT (the crate forces
/// game with 9+), and the highest that is still clearly slamless.
const GAME_FLOOR: u8 = 9;
const SLAM_CEIL: u8 = 14;

/// High-card points of a whole hand
fn hcp(hand: Hand) -> u8 {
    SUITS.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

/// Length of a suit in a hand
fn slen(hand: Hand, suit: Suit) -> u8 {
    hand[suit].len() as u8
}

/// The seat's partner (across the table)
const fn partner(seat: Seat) -> Seat {
    Seat::ALL[(seat as usize + 2) % 4]
}

/// A 15–17 balanced hand — a textbook strong-1NT opener (4333 / 4432 / 5332)
fn strong_notrump(hand: Hand) -> bool {
    let points = hcp(hand);
    if !(15..=17).contains(&points) {
        return false;
    }
    let lens = SUITS.map(|s| slen(hand, s));
    let max = lens.iter().copied().max().unwrap_or(0);
    let min = lens.iter().copied().min().unwrap_or(0);
    let doubletons = lens.iter().filter(|&&l| l == 2).count();
    min >= 2 && max <= 5 && doubletons <= 1
}

/// Responder's 6-card major on a game-but-not-slam hand, if any (a 6-6 freak is
/// skipped as ambiguous)
fn responder_major(hand: Hand) -> Option<Suit> {
    if !(GAME_FLOOR..=SLAM_CEIL).contains(&hcp(hand)) {
        return None;
    }
    match (slen(hand, Suit::Hearts) >= 6, slen(hand, Suit::Spades) >= 6) {
        (true, false) => Some(Suit::Hearts),
        (false, true) => Some(Suit::Spades),
        _ => None,
    }
}

/// Responder holds a singleton or void — a candidate "declaring is not bad" cue
fn responder_short(hand: Hand) -> bool {
    SUITS.iter().any(|&s| slen(hand, s) <= 1)
}

fn main() {
    // Self-check the evaluators (runs in release too): a flat 20-count is neither
    // a strong-1NT (too strong) nor a 6M responder; a 16-HCP 4333 is a strong 1NT.
    assert_eq!(hcp("AKQ2.KQ2.KJ2.Q32".parse().expect("valid hand")), 20);
    assert!(strong_notrump(
        "AQJ2.KQ2.K32.J32".parse().expect("valid hand") // 16 HCP, 4333
    ));
    assert!(responder_major("AKJ842.K8.Q92.32".parse().expect("valid hand")) == Some(Suit::Spades));

    let args = Args::parse();
    let mut rng = rand::rng();

    // One entry per qualifying configuration; the deal is stored alongside so the
    // solved table lines up 1:1 after the batch solve.
    let mut deals: Vec<FullDeal> = Vec::new();
    let mut configs: Vec<(Seat, Seat, Suit, bool)> = Vec::new(); // (responder, opener, major, short)

    for index in 0..args.count {
        let deal = full_deal(&mut rng);
        for responder in Seat::ALL {
            let opener = partner(responder);
            if let Some(major) = responder_major(deal[responder])
                && strong_notrump(deal[opener])
            {
                deals.push(deal);
                configs.push((responder, opener, major, responder_short(deal[responder])));
            }
        }
        if index % 4096 == 0 {
            eprint!("\rsampled {}/{}", index + 1, args.count);
        }
    }
    eprintln!("\rsampled {}/{}        ", args.count, args.count);

    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    // (sum IMPs, board count) for all configs and the short / not-short split.
    let mut all = (0i64, 0usize);
    let mut short = (0i64, 0usize);
    let mut steady = (0i64, 0usize);
    let mut points = 0i64;

    for (&(responder, opener, major, is_short), table) in configs.iter().zip(tables.iter()) {
        let strain = Strain::from(major);
        let contract = Contract {
            bid: Bid::new(4, strain),
            penalty: Penalty::Undoubled,
        };
        let declaring_ns = matches!(responder, Seat::North | Seat::South);
        let declarer_vul = args.vulnerability.contains(if declaring_ns {
            AbsoluteVulnerability::NS
        } else {
            AbsoluteVulnerability::EW
        });
        // Tricks for the declaring side with each partner as declarer — the
        // difference is purely the opening leader (LHO of the declarer).
        let resp_tricks = u8::from(table[strain].get(responder));
        let open_tricks = u8::from(table[strain].get(opener));
        let gain = i64::from(contract.score(resp_tricks, declarer_vul))
            - i64::from(contract.score(open_tricks, declarer_vul));
        let gain_imps = imps(gain);

        points += gain;
        all.0 += gain_imps;
        all.1 += 1;
        let bucket = if is_short { &mut short } else { &mut steady };
        bucket.0 += gain_imps;
        bucket.1 += 1;
    }

    let mean = |(sum, n): (i64, usize)| sum as f64 / n.max(1) as f64;

    println!("=== Texas vs South African Texas — positional declarer A/B ===");
    println!(
        "(DD/perfect-defense scorer is concealment-blind: this is the opening-lead swing only)"
    );
    println!(
        "Sampled {} deals → {} qualifying 6M-game-no-slam-opposite-1NT configs ({:.2}%), vul {}",
        args.count,
        all.1,
        100.0 * all.1 as f64 / args.count.max(1) as f64,
        args.vulnerability,
    );
    println!("SAT − Texas  (responder declares 4M  −  opener declares 4M):");
    println!(
        "  all configs:               {:+.3} IMPs/board   ({:+} IMPs, {:+} pts, n={})",
        mean(all),
        all.0,
        points,
        all.1,
    );
    println!(
        "  responder short (1/0):     {:+.3} IMPs/board   (n={})",
        mean(short),
        short.1,
    );
    println!(
        "  responder no shortness:    {:+.3} IMPs/board   (n={})",
        mean(steady),
        steady.1,
    );
}
