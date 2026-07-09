//! Measure fuzzy strength: an A/B duplicate match of the upgrade policy.
//!
//! The 2/1 system gauges suit-oriented strength with upgraded
//! [`points`][pons::bidding::constraint::points] (HCP plus shape upgrades for
//! clean unbalanced hands) and notrump-defining ranges with
//! [`fifths`][pons::bidding::constraint::fifths] instead of raw HCP.  Is that
//! worth points?  Each board is bid twice, duplicate style: at table A the
//! fuzzy pair sits North/South against a pair evaluating raw HCP everywhere
//! (the pre-upgrade behavior); at table B the teams swap seats.  Both pairs
//! play the very same books — the
//! [`set_fuzzy_strength`][pons::bidding::constraint::set_fuzzy_strength]
//! ablation hook flips the strength gauge per acting side.  Boards whose two
//! auctions reach different contracts are scored double dummy, and the swing
//! is credited to the fuzzy team in points and IMPs.  `--policy` ablates the
//! halves: the fuzzy team enables only the points upgrade, only Fifths, or
//! both (the shipped system).
//!
//! ```text
//! cargo run --example ab-fuzzy-strength -- --count 1000 --vulnerability ns
//! cargo run --example ab-fuzzy-strength -- --count 1000 --policy points
//! ```

use clap::{Parser, ValueEnum};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use pons::american;
use pons::bidding::constraint::{set_fuzzy_fifths, set_fuzzy_points};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, ns_score_contract};

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{Board, score_boards, seat_to_act};

/// Which half of the fuzzy-strength policy the fuzzy team enables
#[derive(Clone, Copy, ValueEnum)]
enum Policy {
    /// Upgraded points for suit-oriented ranges only
    Points,
    /// Fifths for notrump-defining ranges only
    Fifths,
    /// Both gauges (the shipped system)
    Both,
}

impl Policy {
    fn apply(self, enabled: bool) {
        let (points, fifths) = match self {
            Self::Points => (enabled, false),
            Self::Fifths => (false, enabled),
            Self::Both => (enabled, enabled),
        };
        set_fuzzy_points(points);
        set_fuzzy_fifths(fifths);
    }
}

/// Measure fuzzy strength: an A/B duplicate match of the upgrade policy
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Which fuzzy gauges the fuzzy team enables (the baseline team always
    /// evaluates raw HCP)
    #[arg(short, long, default_value = "both")]
    policy: Policy,
}

/// The highest-logit *legal* call, defaulting to a pass
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

/// Bid out one deal, switching the strength gauge per acting side
///
/// Bidding is single-threaded here, so flipping the thread-local fuzzy flags
/// just before each classification cleanly serves both teams from one stance.
fn bid_out(
    stance: &Stance,
    policy: Policy,
    fuzzy_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        policy.apply(seat_is_ns == fuzzy_is_ns);
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    let stance = american().against(Family::NATURAL);

    // Bid every board at both tables, dealer rotating per board.
    let boards: Vec<Board> = (0..args.count)
        .map(|index| {
            let dealer = Seat::ALL[index % 4];
            let deal = full_deal(&mut rng);
            let table_a = bid_out(
                &stance,
                args.policy,
                true,
                dealer,
                args.vulnerability,
                &deal,
            );
            let table_b = bid_out(
                &stance,
                args.policy,
                false,
                dealer,
                args.vulnerability,
                &deal,
            );
            Board {
                deal,
                dealer,
                table_a,
                table_b,
            }
        })
        .collect();

    // Only boards whose tables reach different results can swing; solve
    // those double dummy and credit the swing to the fuzzy team (NS at
    // table A, EW at table B).
    let contracts: Vec<_> = boards
        .iter()
        .map(|board| {
            (
                final_contract(&board.table_a, board.dealer),
                final_contract(&board.table_b, board.dealer),
            )
        })
        .collect();
    let deals: Vec<FullDeal> = boards.iter().map(|board| board.deal).collect();
    let scored = score_boards(&contracts, &deals, args.vulnerability, ns_score_contract);
    let (total_points, total_imps) = (scored.total_points, scored.total_imps);

    println!(
        "=== Fuzzy strength A/B match: {} boards, vulnerability {}, policy {} ===",
        args.count,
        args.vulnerability,
        match args.policy {
            Policy::Points => "points",
            Policy::Fifths => "fifths",
            Policy::Both => "both",
        },
    );
    println!(
        "Divergent boards: {} of {} ({:.0}%)",
        scored.divergent.len(),
        args.count,
        100.0 * scored.divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Fuzzy team: {total_points:+} points, {total_imps:+} IMPs ({:+.2} IMPs/board)",
        total_imps as f64 / args.count.max(1) as f64,
    );
}
