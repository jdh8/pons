//! Measure the gated live-search floor (AI-bidder M2.3): A/B duplicate matches.
//!
//! The companion to `instinct-floor` and `neural-floor`, for the *thinking*
//! floor.  The [`SearchFloor`][pons::bidding::search_floor::SearchFloor] safety
//! shell — the distilled net used only as a prior to shortlist calls, then each
//! priced by double-dummy cardplay EV ([`ev_all`][pons::bidding::ev_all]) over
//! sampled layouts before bidding the best — runs as a [`Pair`][pons::Pair] floor
//! ([`two_over_one_search`][pons::two_over_one_search]) against two opponents,
//! each board bid twice duplicate-style with the seats swapped:
//!
//! 1. **vs the deterministic floor** ([`two_over_one`]): the hand-written
//!    `instinct()` ladder.  Beating it (strictly positive IMPs/board) is the
//!    whole point of M2.3 — cardplay-grounded judgement over a static rule.
//! 2. **vs the distilled net** ([`two_over_one_neural`]): the raw one-forward-pass
//!    policy the search uses as its *prior*.  Search should beat the policy it
//!    proposes from — "net proposes, search disposes".
//!
//! Boards whose two tables reach different contracts are scored double dummy and
//! the swing credited to the search ("home") team, reported as IMPs/board with a
//! 95% confidence interval over boards.
//!
//! # This is slow
//!
//! Every non-forced decision runs a full double-dummy search (128 layouts × up to
//! 8 candidates by default, ~1.4 s each), so a board costs *thousands* of solves,
//! not one.  The default board count is therefore small — a smoke test, not a
//! verdict.  A tight interval needs thousands of boards and a long, patient run:
//!
//! ```text
//! cargo run --release --features search --example search-floor -- --count 2000
//! ```

#![allow(clippy::cast_precision_loss)]

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score};
use pons::{Accumulator, two_over_one, two_over_one_neural, two_over_one_search};

/// Measure the live-search floor: A/B duplicate matches with intervals
#[derive(Parser)]
struct Args {
    /// Number of boards per match (dealer rotates per board)
    ///
    /// Small by default: the search is slow (~1.4 s/decision).  Scale up for a
    /// real interval.
    #[arg(short, long, default_value = "50")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,
}

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
}

/// The highest-logit *legal* call, defaulting to a pass
///
/// [`Table::next_call`][pons::bidding::Table::next_call] inlined without the
/// floor-provenance tap (this example measures swing, not activation).
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

/// Bid out one deal: the `home` pair takes the NS seats when `home_is_ns`
fn bid_out(
    home: &Stance,
    away: &Stance,
    home_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();

    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == home_is_ns { home } else { away };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

/// One board: the deal and both tables' auctions
struct Board {
    deal: FullDeal,
    dealer: Seat,
    /// Table A: home pair sits North/South
    table_a: Auction,
    /// Table B: home pair sits East/West
    table_b: Auction,
}

/// The outcome of one duplicate match
struct MatchResult {
    /// Per-board IMP swing to the home team (0 on non-divergent boards)
    swings: Vec<i64>,
    /// Boards whose two tables reached different contracts
    divergent: usize,
}

/// Run a `count`-board duplicate match, crediting swings to `home`
fn duplicate_match(
    home: &Stance,
    away: &Stance,
    count: usize,
    vul: AbsoluteVulnerability,
    rng: &mut impl rand::Rng,
) -> MatchResult {
    // Bid every board at both tables, dealer rotating per board.  Each call may
    // trigger a full double-dummy search inside the floor, so this is the slow
    // part — not the divergent-board scoring below.
    let boards: Vec<Board> = (0..count)
        .map(|index| {
            let dealer = Seat::ALL[index % 4];
            let deal = full_deal(rng);
            let table_a = bid_out(home, away, true, dealer, vul, &deal);
            let table_b = bid_out(home, away, false, dealer, vul, &deal);
            Board {
                deal,
                dealer,
                table_a,
                table_b,
            }
        })
        .collect();

    // Only boards whose tables reach different results can swing; solve those
    // double dummy in one batch and credit the swing to the home team (NS at
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

    let mut swings = vec![0i64; count];
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points = ns_score(contract_a, table, vul) - ns_score(contract_b, table, vul);
        swings[index] = imps(points);
    }

    MatchResult {
        swings,
        divergent: divergent.len(),
    }
}

/// Print one match's IMPs/board with a 95% CI and a "should beat" verdict
///
/// M2.3's win condition is *strictly positive* IMPs/board: the search team
/// should be ahead, with the interval excluding zero once enough boards are in.
fn report(
    label: &str,
    opponent: &str,
    result: &MatchResult,
    count: usize,
    vul: AbsoluteVulnerability,
) {
    let mut acc = Accumulator::new();
    for &swing in &result.swings {
        acc.push(swing as f64);
    }
    let stats = acc.sample();
    let mean = stats.mean();
    let se = stats.sd() / (count.max(1) as f64).sqrt();
    let (lo, hi) = (mean - 1.96 * se, mean + 1.96 * se);
    let total: i64 = result.swings.iter().sum();

    println!("\n=== {label}: {count} boards, vulnerability {vul} ===");
    println!(
        "Divergent boards: {} of {count} ({:.0}%)",
        result.divergent,
        100.0 * result.divergent as f64 / count.max(1) as f64,
    );
    println!("Home (search) team: {total:+} IMPs, {mean:+.3} IMPs/board");
    println!("  95% CI: [{lo:+.3}, {hi:+.3}]  (SE {se:.3}, n = {count})");

    let excludes_zero = !(lo..=hi).contains(&0.0);
    let verdict = match (excludes_zero, mean > 0.0) {
        (true, true) => "search ahead, CI excludes 0 — beats it (the M2.3 goal)",
        (true, false) => "search BEHIND, CI excludes 0 — inspect divergent boards",
        (false, _) => "CI contains 0 — too few boards to call; collect more",
    };
    println!("  Should beat {opponent} (strictly positive): {verdict}");
}

fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    let search = two_over_one_search().against(Family::NATURAL);
    let deterministic = two_over_one().against(Family::NATURAL);
    let neural = two_over_one_neural().against(Family::NATURAL);

    println!(
        "AI-bidder M2.3: live double-dummy search floor vs the deterministic floor and the \
         distilled net\n({} boards per match; the search is slow — thousands of boards need a \
         long run)",
        args.count,
    );

    let vs_deterministic = duplicate_match(
        &search,
        &deterministic,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "Search floor vs deterministic floor",
        "the deterministic floor",
        &vs_deterministic,
        args.count,
        args.vulnerability,
    );

    let vs_neural = duplicate_match(&search, &neural, args.count, args.vulnerability, &mut rng);
    report(
        "Search floor vs distilled net",
        "the raw net prior",
        &vs_neural,
        args.count,
        args.vulnerability,
    );
}
