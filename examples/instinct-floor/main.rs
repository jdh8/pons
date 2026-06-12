//! Measure the instinct floor: an A/B duplicate match plus floor telemetry.
//!
//! Two questions about [`bidding::instinct`][pons::bidding::instinct], both
//! answered from the same random boards:
//!
//! 1. **Is the floor worth points?**  Each board is bid twice, duplicate
//!    style: at table A the floored [`two_over_one`] pair sits North/South
//!    against the bare books ([`bare_two_over_one`], which passes whenever
//!    its books run out — the pre-floor behavior); at table B the teams swap
//!    seats.  Boards whose two auctions reach different contracts are scored
//!    double dummy, and the swing is credited to the floored team in points
//!    and IMPs — the floor's regret against its own absence, sign-flipped.
//! 2. **Where does the floor fire?**  Every call the floored side classifies
//!    is checked for floor provenance
//!    ([`Stance::classify_with_provenance`]); the most-hit off-book auctions
//!    are the next nodes worth authoring properly.
//!
//! ```text
//! cargo run --example instinct-floor -- --count 200 --vulnerability ns
//! ```
//!
//! [`Stance::classify_with_provenance`]: pons::bidding::Stance::classify_with_provenance

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::context::relative;
use pons::bidding::two_over_one::bare_two_over_one;
use pons::bidding::{Family, Stance};
use pons::scoring::{final_contract, imps, ns_score};
use pons::two_over_one;
use std::collections::HashMap;

/// Measure the instinct floor: A/B duplicate match plus floor telemetry
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Number of off-book auctions to list in the telemetry report
    #[arg(short, long, default_value = "15")]
    top: usize,
}

// ---------------------------------------------------------------------------
// Telemetry
// ---------------------------------------------------------------------------

/// Floor-activation counts over the floored side's classified calls
#[derive(Default)]
struct Telemetry {
    /// Calls the floored side classified (book and floor alike)
    calls: usize,
    /// Calls answered by the instinct floor (`depth == 0`, `fallback` set)
    floor_calls: usize,
    /// Off-book auction (leading passes stripped) → (count, sample floor call)
    patterns: HashMap<String, (usize, Call)>,
}

/// One display key per off-book auction: leading passes stripped, calls joined
///
/// Leading passes only encode the seat, which the books already fan over, so
/// stripping them merges the four seats of one decision into one line.
fn auction_key(auction: &[Call]) -> String {
    auction
        .iter()
        .skip_while(|&&call| call == Call::Pass)
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------------------
// Driving the match
// ---------------------------------------------------------------------------

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// The highest-logit *legal* call, defaulting to a pass
///
/// This is [`Table::next_call`][pons::bidding::Table::next_call] with a
/// telemetry tap: when `telemetry` is given (the floored side is acting),
/// the classification's provenance is recorded.
fn next_call(
    stance: &Stance,
    hand: contract_bridge::Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
    telemetry: Option<&mut Telemetry>,
) -> Call {
    let seat = seat_to_act(dealer, auction.len());
    let Some((logits, provenance)) =
        stance.classify_with_provenance(hand, relative(vul, seat), auction)
    else {
        return Call::Pass;
    };

    let mut scored: Vec<(Call, f32)> = logits
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
    let call = scored
        .into_iter()
        .map(|(call, _)| call)
        .find(|&call| auction.can_push(call).is_ok())
        .unwrap_or(Call::Pass);

    if let Some(telemetry) = telemetry {
        telemetry.calls += 1;
        if provenance.depth == 0 && provenance.fallback.is_some() {
            telemetry.floor_calls += 1;
            telemetry
                .patterns
                .entry(auction_key(auction))
                .and_modify(|(count, _)| *count += 1)
                .or_insert((1, call));
        }
    }
    call
}

/// Bid out one deal, tapping telemetry for the floored side only
fn bid_out(
    floored: &Stance,
    bare: &Stance,
    floored_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    telemetry: &mut Telemetry,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let (stance, tap) = if seat_is_ns == floored_is_ns {
            (floored, Some(&mut *telemetry))
        } else {
            (bare, None)
        };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction, tap));
    }
    auction
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

/// One board: the deal and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    /// Table A: floored pair sits North/South
    table_a: Auction,
    /// Table B: floored pair sits East/West
    table_b: Auction,
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    let floored = two_over_one().against(Family::NATURAL);
    let bare = bare_two_over_one().against(Family::NATURAL);
    let mut telemetry = Telemetry::default();

    // Bid every board at both tables, dealer rotating per board.
    let boards: Vec<Board> = (0..args.count)
        .map(|index| {
            let dealer = Seat::ALL[index % 4];
            let deal = full_deal(&mut rng);
            let table_a = bid_out(
                &floored,
                &bare,
                true,
                dealer,
                args.vulnerability,
                &deal,
                &mut telemetry,
            );
            let table_b = bid_out(
                &floored,
                &bare,
                false,
                dealer,
                args.vulnerability,
                &deal,
                &mut telemetry,
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
    // those double dummy and credit the swing to the floored team (NS at
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
    let divergent: Vec<usize> = (0..boards.len())
        .filter(|&index| contracts[index].0 != contracts[index].1)
        .collect();
    let deals: Vec<FullDeal> = divergent.iter().map(|&index| boards[index].deal).collect();
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);

    let mut total_points = 0i64;
    let mut total_imps = 0i64;
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let swing = ns_score(contract_a, table, args.vulnerability)
            - ns_score(contract_b, table, args.vulnerability);
        total_points += swing;
        total_imps += imps(swing);
    }

    println!(
        "=== Instinct floor A/B match: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "Divergent boards: {} of {} ({:.0}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Floored team: {total_points:+} points, {total_imps:+} IMPs ({:+.2} IMPs/board)",
        total_imps as f64 / args.count.max(1) as f64,
    );

    println!(
        "\n=== Floor telemetry: {} of {} floored-side calls ({:.1}%) ===",
        telemetry.floor_calls,
        telemetry.calls,
        100.0 * telemetry.floor_calls as f64 / telemetry.calls.max(1) as f64,
    );
    let mut patterns: Vec<(&String, &(usize, Call))> = telemetry.patterns.iter().collect();
    patterns.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(b.0)));
    println!(
        "  {:>6}  {:<8}  auction (leading passes stripped)",
        "count", "floor"
    );
    for (key, &(count, sample)) in patterns.into_iter().take(args.top) {
        println!("  {count:>6}  {sample:<8}  {key}");
    }
}
