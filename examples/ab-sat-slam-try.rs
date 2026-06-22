//! Revised South African Texas — the `4♥/4♠` "non-forcing slam try" gadget.
//!
//! In the swapped SAT, `4♣/4♦` are the everyday transfers (opener declares `4M`,
//! declarer-equivalent to Texas — see the `texas-vs-sat` example), and a *direct*
//! `4♥/4♠` is a non-forcing slam try: opener **passes** with a minimum, or
//! launches **RKCB** with a maximum and the partnership bids the keycard-
//! indicated slam.  So Texas and revised-SAT now agree on the to-play hands and
//! diverge only on the **slam-invitational** ones — a question of slam-bidding
//! *accuracy*, which double-dummy CAN see (reaching a making slam, or stopping
//! out of a bad one), unlike the concealment question of experiment 1.
//!
//! - **Baseline** = the crate's current bidder (`american()`, opponents silent):
//!   the status quo / Texas proxy.  It reaches slam on these hands by raw point
//!   count (the floor's `combined_points`), opener declaring.
//! - **Gadget** = modeled by hand.  Opener `< MAX` passes the non-forcing `4M`
//!   (responder declares game); opener `>= MAX` exchanges keycards (RKCB 1430,
//!   the crate's own definition: four aces + the trump king) and bids `6M` unless
//!   the partnership is missing two keycards, else signs off in `5M`.  Responder
//!   bid the major first, so **every gadget contract is responder-declared**.
//!
//! `IMP(gadget − baseline)` over double-dummy-solved deals; positive favors the
//! gadget.  Slam-reach / make-rate breakdowns are printed so the IMP number is
//! interpretable (is the gadget reaching *making* slams, or overbidding?).
//!
//! ```text
//! cargo run --release --example ab-sat-slam-try -- --count 2000000 --vulnerability none
//! ```
//
// ponytail: the gadget caps at 6M (small slam) — grand-slam / 5NT king-ask
// exploration is out of scope (rare, and not what "invites MAX to RKCB" asked
// for).  Slam-try range / MAX / keycard threshold are tunable knobs, not laws.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{
    AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Penalty, Rank, Seat, Strain, Suit, eval,
};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};

/// Revised SAT: the `4♥/4♠` non-forcing-slam-try gadget vs the current bidder
#[derive(Parser)]
struct Args {
    /// Number of random deals to sample (slam-invitational configs are rare)
    #[arg(short, long, default_value = "2000000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
}

const SUITS: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

/// Responder HCP band for the slam try: invitational to slam, not a blast (those
/// would force) and not a plain game (those take the `4♣/4♦` to-play transfer).
const SLAM_TRY_FLOOR: u8 = 15;
const SLAM_TRY_CEIL: u8 = 18;
/// Opener HCP that accepts the invitation by launching RKCB (a "maximum" 1NT).
const MAX: u8 = 17;

fn hcp(hand: Hand) -> u8 {
    SUITS.iter().map(|&s| eval::hcp::<u8>(hand[s])).sum()
}

fn slen(hand: Hand, suit: Suit) -> u8 {
    hand[suit].len() as u8
}

/// Keycards: the four aces plus the trump king (RKCB 1430, per `slam.rs`)
fn keycards(hand: Hand, trump: Suit) -> u8 {
    let aces = SUITS.iter().filter(|&&s| hand[s].contains(Rank::A)).count() as u8;
    aces + u8::from(hand[trump].contains(Rank::K))
}

/// A 15–17 balanced hand — a textbook strong-1NT opener (4333 / 4432 / 5332)
fn strong_notrump(hand: Hand) -> bool {
    if !(15..=17).contains(&hcp(hand)) {
        return false;
    }
    let lens = SUITS.map(|s| slen(hand, s));
    let max = lens.iter().copied().max().unwrap_or(0);
    let min = lens.iter().copied().min().unwrap_or(0);
    let doubletons = lens.iter().filter(|&&l| l == 2).count();
    min >= 2 && max <= 5 && doubletons <= 1
}

/// Responder's 6-card major on a slam-invitational hand (6-6 freak skipped)
fn slam_try_major(hand: Hand) -> Option<Suit> {
    if !(SLAM_TRY_FLOOR..=SLAM_TRY_CEIL).contains(&hcp(hand)) {
        return None;
    }
    match (slen(hand, Suit::Hearts) >= 6, slen(hand, Suit::Spades) >= 6) {
        (true, false) => Some(Suit::Hearts),
        (false, true) => Some(Suit::Spades),
        _ => None,
    }
}

/// `(opener, responder, major)` if the two seats form a slam-try configuration
fn configuration(deal: &FullDeal, a: Seat, b: Seat) -> Option<(Seat, Seat, Suit)> {
    if let Some(major) = slam_try_major(deal[b])
        && strong_notrump(deal[a])
    {
        Some((a, b, major))
    } else if let Some(major) = slam_try_major(deal[a])
        && strong_notrump(deal[b])
    {
        Some((b, a, major))
    } else {
        None
    }
}

/// The gadget's final contract (responder declares — see the caller)
fn gadget_contract(opener: Hand, responder: Hand, major: Suit) -> Contract {
    let level = if hcp(opener) < MAX {
        4 // minimum: pass the non-forcing 4M
    } else if keycards(opener, major) + keycards(responder, major) >= 4 {
        6 // maximum, not missing two keycards: small slam
    } else {
        5 // maximum but missing two keycards: sign off above game
    };
    Contract {
        bid: Bid::new(level, Strain::from(major)),
        penalty: Penalty::Undoubled,
    }
}

// --- baseline bidder (opponents silent), lifted from `nt-shape-abc` -----------

const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

fn next_call(
    stance: &Stance,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some(logits) = stance.classify(hand, relative(vul, seat), auction) else {
        return Call::Pass;
    };
    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass)
}

fn bid_uncontested(
    stance: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let call = if matches!(seat, Seat::East | Seat::West) {
            Call::Pass
        } else {
            next_call(stance, deal[seat], dealer, vul, &auction)
        };
        auction.push(call);
    }
    auction
}

/// Tricks the declaring side takes with `declarer` in `strain`
fn tricks(table: &ddss::TrickCountTable, strain: Strain, declarer: Seat) -> u8 {
    u8::from(table[strain].get(declarer))
}

fn main() {
    // Self-check the evaluators (runs in release): a 16-count 4432 is a strong
    // 1NT holding 3 spade keycards (♠A, ♠K, ♥A); a flat 20-count is not a 1NT.
    assert!(strong_notrump(
        "AK42.AQ32.K32.32".parse().expect("valid hand")
    ));
    assert!(!strong_notrump(
        "AKQ2.KQ2.KJ2.Q32".parse().expect("valid hand")
    ));
    assert_eq!(
        keycards(
            "AK42.AQ32.K32.32".parse().expect("valid hand"),
            Suit::Spades
        ),
        3
    );

    let args = Args::parse();
    let mut rng = rand::rng();
    let stance = american().against(Family::NATURAL);

    // Collect slam-try configurations (only the N/S partnership, so North/South
    // scoring stays sign-consistent), each with the deal for the batch solve.
    let mut deals: Vec<FullDeal> = Vec::new();
    let mut configs: Vec<(Seat, Seat, Suit)> = Vec::new(); // (opener, responder, major)

    for index in 0..args.count {
        let deal = full_deal(&mut rng);
        if let Some(config) = configuration(&deal, Seat::North, Seat::South) {
            deals.push(deal);
            configs.push(config);
        }
        if index % 8192 == 0 {
            eprint!("\rsampled {}/{}", index + 1, args.count);
        }
    }
    eprintln!("\rsampled {}/{}        ", args.count, args.count);

    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut total_imps = 0i64;
    let mut total_points = 0i64;
    // Diagnostics: slam reach and make-rate for each scheme.
    let (mut base_slam, mut base_slam_makes) = (0usize, 0usize);
    let (mut base_4major, mut base_3nt) = (0usize, 0usize); // what the baseline does otherwise
    let (mut gadget_slam, mut gadget_slam_makes, mut gadget_pass, mut gadget_five) = (0, 0, 0, 0);

    for ((opener, responder, major), (deal, table)) in
        configs.iter().zip(deals.iter().zip(tables.iter()))
    {
        let strain = Strain::from(*major);

        // Baseline: the real bidder, opener on lead-out (dealer = opener).
        let auction = bid_uncontested(&stance, *opener, args.vulnerability, deal);
        let base = final_contract(&auction, *opener);

        // Gadget: modeled, responder-declared.
        let gadget = gadget_contract(deal[*opener], deal[*responder], *major);

        let base_score = ns_score_contract(base, table, args.vulnerability);
        let gadget_score = ns_score_contract(Some((gadget, *responder)), table, args.vulnerability);
        total_points += gadget_score - base_score;
        total_imps += imps(gadget_score - base_score);

        // --- diagnostics ---
        let makes = |level: u8, declarer: Seat| tricks(table, strain, declarer) >= 6 + level;
        if let Some((contract, declarer)) = base {
            let bid = contract.bid;
            if bid.level.get() >= 6 {
                base_slam += 1;
                if makes(bid.level.get(), declarer) {
                    base_slam_makes += 1;
                }
            } else if bid.level.get() == 4 && bid.strain == strain {
                base_4major += 1;
            } else if bid.strain == Strain::Notrump && bid.level.get() == 3 {
                base_3nt += 1;
            }
        }
        match gadget.bid.level.get() {
            6 => {
                gadget_slam += 1;
                if makes(6, *responder) {
                    gadget_slam_makes += 1;
                }
            }
            5 => gadget_five += 1,
            _ => gadget_pass += 1,
        }
    }

    let n = configs.len();
    let pct = |k: usize, of: usize| 100.0 * k as f64 / of.max(1) as f64;

    println!("=== Revised SAT: 4M non-forcing-slam-try gadget vs current bidder ===");
    println!(
        "Sampled {} deals → {} slam-invitational 6M-opposite-1NT configs ({:.3}%), vul {}",
        args.count,
        n,
        pct(n, args.count),
        args.vulnerability,
    );
    println!(
        "Gadget − baseline:  {:+.3} IMPs/board   ({:+} IMPs, {:+} pts, n={})",
        total_imps as f64 / n.max(1) as f64,
        total_imps,
        total_points,
        n,
    );
    println!("--- contract reach (double-dummy make-rates) ---");
    println!(
        "  baseline:  4M {:.1}% | 3NT {:.1}% | slam {:.1}% (of which {:.1}% make) | other {:.1}%",
        pct(base_4major, n),
        pct(base_3nt, n),
        pct(base_slam, n),
        pct(base_slam_makes, base_slam),
        pct(n - base_4major - base_3nt - base_slam, n),
    );
    println!(
        "  gadget:    pass-4M {:.1}% | 5M {:.1}% | 6M {:.1}% (of which {:.1}% make)",
        pct(gadget_pass, n),
        pct(gadget_five, n),
        pct(gadget_slam, n),
        pct(gadget_slam_makes, gadget_slam),
    );
}
