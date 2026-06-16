//! Measure the distilled neural floors (AI-bidder M1.4, M5.1): A/B duplicate
//! matches.
//!
//! The companion to the `instinct-floor` example, for the *learned* floor.  The
//! [`NeuralFloor`][pons::bidding::neural_floor::NeuralFloor] safety shell —
//! the distilled net wrapped so the forced rails delegate to
//! [`instinct`][pons::instinct] and everything else is the legality-masked net —
//! is run as a [`Pair`][pons::Pair] floor
//! ([`two_over_one_neural`][pons::two_over_one_neural]) against two opponents,
//! each board bid twice duplicate-style with the seats swapped.  The
//! tag-augmented v2 floor
//! ([`two_over_one_neural_v2`][pons::two_over_one_neural_v2], M5.1) is measured
//! the same way, and head-to-head against v1 — the M5.1 win condition is **no
//! regression vs v1, ideally a small gain**.  Each floor is run:
//!
//! 1. **vs the deterministic floor** ([`two_over_one`]): the distillation target.
//!    A faithful clone scores ≈ 0 IMPs/board — *parity*.
//! 2. **vs bare books** ([`bare_two_over_one`], which passes off-book): the
//!    floor's worth.  Parity with the deterministic floor means ≈ +0.5
//!    IMPs/board here too — the learned floor preserves the hand-built one's gain.
//!
//! Boards whose two tables reach different contracts are scored double dummy and
//! the swing credited to the neural ("home") team.  IMPs/board is reported with
//! a 95% confidence interval over boards: the per-board swing is noisy, so a
//! headline number needs thousands of boards and sub-0.1 IMPs/board is noise
//! unless the count is large.
//!
//! ```text
//! cargo run --release --features neural-floor --example neural-floor -- --count 10000
//! ```

#![allow(clippy::cast_precision_loss)]

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::context::relative;
use pons::bidding::two_over_one::bare_two_over_one;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score, ns_score_doubling_failures};
use pons::{
    Accumulator, two_over_one, two_over_one_neural, two_over_one_neural_search,
    two_over_one_neural_v2,
};

/// Measure the distilled neural floor: A/B duplicate matches with intervals
#[derive(Parser)]
struct Args {
    /// Number of boards per match (dealer rotates per board)
    #[arg(short, long, default_value = "5000")]
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
    /// Same swings, but scored under perfect-defense doubling
    /// ([`ns_score_doubling_failures`]): a contract that fails double dummy is
    /// priced *doubled*, so phantom/failing contracts are punished as good
    /// defenders would. The realism check on the DD-optimistic `swings`.
    swings_pd: Vec<i64>,
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
    // Bid every board at both tables, dealer rotating per board.
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
    let mut swings_pd = vec![0i64; count];
    for (&index, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[index];
        let points = ns_score(contract_a, table, vul) - ns_score(contract_b, table, vul);
        swings[index] = imps(points);
        let points_pd = ns_score_doubling_failures(contract_a, table, vul)
            - ns_score_doubling_failures(contract_b, table, vul);
        swings_pd[index] = imps(points_pd);
    }

    MatchResult {
        swings,
        swings_pd,
        divergent: divergent.len(),
    }
}

/// Print one match's IMPs/board with a 95% CI and a verdict against `target`
fn report(
    label: &str,
    result: &MatchResult,
    count: usize,
    vul: AbsoluteVulnerability,
    target: f64,
) {
    let summarize = |swings: &[i64]| -> (i64, f64, f64, f64, f64) {
        let mut acc = Accumulator::new();
        for &swing in swings {
            acc.push(swing as f64);
        }
        let stats = acc.sample();
        let mean = stats.mean();
        let se = stats.sd() / (count.max(1) as f64).sqrt();
        (
            swings.iter().sum(),
            mean,
            mean - 1.96 * se,
            mean + 1.96 * se,
            se,
        )
    };
    let (total, mean, lo, hi, se) = summarize(&result.swings);
    let (total_pd, mean_pd, lo_pd, hi_pd, _) = summarize(&result.swings_pd);

    println!("\n=== {label}: {count} boards, vulnerability {vul} ===");
    println!(
        "Divergent boards: {} of {count} ({:.0}%)",
        result.divergent,
        100.0 * result.divergent as f64 / count.max(1) as f64,
    );
    println!("Home (neural) team: {total:+} IMPs, {mean:+.3} IMPs/board");
    println!("  95% CI: [{lo:+.3}, {hi:+.3}]  (SE {se:.3}, n = {count})");
    println!(
        "  perfect-defense doubling: {total_pd:+} IMPs, {mean_pd:+.3} IMPs/board, \
         CI [{lo_pd:+.3}, {hi_pd:+.3}]"
    );

    if target == 0.0 {
        let within = (lo..=hi).contains(&0.0);
        let verdict = match (within, mean.abs() < 0.1) {
            (true, true) => "parity — CI contains 0 and |mean| < 0.1 (within noise)",
            (true, false) => "CI contains 0, but |mean| ≥ 0.1 — collect more boards",
            (false, _) if mean > 0.0 => "CI excludes 0, net ahead (a pleasant surprise)",
            (false, _) => "CI excludes 0, net behind — inspect divergent boards",
        };
        println!("  Target ≈ 0 (parity): {verdict}");
    } else {
        let contains = (lo..=hi).contains(&target);
        println!(
            "  Target ≈ {target:+.1}: CI {} the {target:+.1} target",
            if contains { "contains" } else { "excludes" },
        );
    }
}

fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    let neural = two_over_one_neural().against(Family::NATURAL);
    let neural_v2 = two_over_one_neural_v2().against(Family::NATURAL);
    let neural_search = two_over_one_neural_search().against(Family::NATURAL);
    let deterministic = two_over_one().against(Family::NATURAL);
    let bare = bare_two_over_one().against(Family::NATURAL);

    println!(
        "AI-bidder M1.4 / M5.1 / M3.2: distilled neural floors (v1, tag-augmented v2, \
         and the search-target net) vs the deterministic floor and bare books\n\
         ({} boards per match; thousands needed for a tight interval)",
        args.count,
    );

    // ── version 1 (M1.4): the plain distilled floor ─────────────────────────
    let v1_vs_det = duplicate_match(
        &neural,
        &deterministic,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "v1 neural floor vs deterministic floor",
        &v1_vs_det,
        args.count,
        args.vulnerability,
        0.0,
    );

    let v1_vs_bare = duplicate_match(&neural, &bare, args.count, args.vulnerability, &mut rng);
    report(
        "v1 neural floor vs bare books",
        &v1_vs_bare,
        args.count,
        args.vulnerability,
        0.5,
    );

    // ── version 2 (M5.1): the tag-augmented floor ───────────────────────────
    let v2_vs_det = duplicate_match(
        &neural_v2,
        &deterministic,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "v2 (tag) neural floor vs deterministic floor",
        &v2_vs_det,
        args.count,
        args.vulnerability,
        0.0,
    );

    let v2_vs_bare = duplicate_match(&neural_v2, &bare, args.count, args.vulnerability, &mut rng);
    report(
        "v2 (tag) neural floor vs bare books",
        &v2_vs_bare,
        args.count,
        args.vulnerability,
        0.5,
    );

    // The headline M5.1 comparison: does seeing the recent calls' tags beat the
    // plain v1 floor head-to-head?  Target 0 = "no regression"; CI above 0 = the
    // tag block is a net gain.
    let v2_vs_v1 = duplicate_match(
        &neural_v2,
        &neural,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "v2 (tag) neural floor vs v1 neural floor — the M5.1 gain",
        &v2_vs_v1,
        args.count,
        args.vulnerability,
        0.0,
    );

    // ── M3.2 round 1: the search-target net (v1 features, distilled from the
    // live-search teacher) ──────────────────────────────────────────────────
    // The headline: does training toward the search target (M3.1) beat training
    // toward the deterministic teacher, head-to-head at the same features and
    // arch?  Target 0 = no regression; CI strictly above 0 = a real gain.
    let search_vs_v1 = duplicate_match(
        &neural_search,
        &neural,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "search-target net vs v1 neural floor — the M3.2 round-1 gain",
        &search_vs_v1,
        args.count,
        args.vulnerability,
        0.0,
    );

    // Regression guard: the search-target net must stay at least at parity with
    // the deterministic floor it ultimately derives from.
    let search_vs_det = duplicate_match(
        &neural_search,
        &deterministic,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "search-target net vs deterministic floor",
        &search_vs_det,
        args.count,
        args.vulnerability,
        0.0,
    );

    // The floor's worth is preserved: ≈ +0.5 IMPs/board vs bare books.
    let search_vs_bare = duplicate_match(
        &neural_search,
        &bare,
        args.count,
        args.vulnerability,
        &mut rng,
    );
    report(
        "search-target net vs bare books",
        &search_vs_bare,
        args.count,
        args.vulnerability,
        0.5,
    );
}
