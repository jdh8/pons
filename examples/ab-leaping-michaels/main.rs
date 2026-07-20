//! Leaping Michaels A/B, **contested**: our defense to an opponent's weak two.
//!
//! Over their weak two, the baseline offers a takeout double, a 15–18 natural
//! 2NT, and natural suit overcalls.  A strong 5-5 two-suiter (a minor + a major)
//! has no clean call — it must double-then-bid and hope.  **Leaping Michaels**
//! adds a descriptive jump to `4♣`/`4♦`: over a major it shows a minor plus the
//! *other* major; over `2♦` the `4♦` cue shows both majors and `4♣` shows clubs
//! plus a major.  All are 5-5+ with game-forcing values, so partner can pick the
//! strain at once.
//!
//! Both arms run the same 2/1 system; the only difference is whether each pair
//! carries Leaping Michaels (`--ns` / `--ew`: on / off), read once at
//! book-construction time.  It only fires when an opponent opens a weak two, so —
//! unlike the constructive `*-abc` harnesses — the opponents must bid.  This uses
//! the contested seat-swap duplicate match (the `lebensohl-ab` template): at
//! table A the measured (`--ns`) pair sits North/South against the baseline
//! (`--ew`) pair East/West; at table B they swap.  A board whose tables reach
//! different contracts is solved double dummy and the swing credited to the NS
//! pair.  A positive IMPs/board favors the `--ns` arm.
//!
//! ```text
//! # Leaping Michaels (NS) vs the bare baseline (EW), filtered to plausible boards:
//! cargo run --release --example ab-leaping-michaels -- --count 200000 --filter
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat, Suit};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::set_leaping_michaels;
use pons::scoring::{final_contract, ns_score_contract};
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, hand_hcp, score_boards};

/// Contested Leaping Michaels A/B
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Leaping Michaels for the measured (NS) pair: on / off
    #[arg(long, default_value = "on")]
    ns: String,

    /// Leaping Michaels for the baseline (EW) pair: on / off
    #[arg(long, default_value = "off")]
    ew: String,

    /// Only count deals that can plausibly reach a Leaping Michaels auction (a
    /// cheap shape pre-filter), so the DD budget lands on boards that can
    /// actually diverge. `--count` is then the number of such filtered boards.
    #[arg(long, default_value = "false")]
    filter: bool,
}

/// Weak-two opening shape: a six-card D/H/S suit with 5–10 HCP (a cheap proxy for
/// the opening gate, ignoring seat)
fn is_weak_two(hand: Hand) -> bool {
    let hcp = hand_hcp(hand);
    (5..=10).contains(&hcp)
        && [Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .iter()
            .any(|&s| hand[s].len() == 6)
}

/// A plausible Leaping Michaels hand: two 5+ suits with 14+ HCP (a superset of
/// the real 5-5 minor+major / both-majors condition)
fn is_leaping_michaels_hand(hand: Hand) -> bool {
    hand_hcp(hand) >= 14 && Suit::ASC.iter().filter(|&&s| hand[s].len() >= 5).count() >= 2
}

/// Cheap pre-filter (no bidding): could this deal plausibly reach a direct
/// Leaping Michaels overcall?
///
/// Some seat has weak-two shape and its left-hand opponent (the direct
/// overcaller) holds a Leaping Michaels hand. This is a *superset* of the
/// divergence condition, so filtering on it concentrates the DD budget on
/// relevant boards without biasing the per-divergent estimate.
fn could_reach_leaping_michaels(deal: &FullDeal) -> bool {
    Seat::ALL.iter().any(|&opener| {
        let lho = Seat::ALL[(opener as usize + 1) % 4];
        is_weak_two(deal[opener]) && is_leaping_michaels_hand(deal[lho])
    })
}

/// Parse an on/off arm flag
fn on_from(name: &str) -> bool {
    match name {
        "on" => true,
        "off" => false,
        other => panic!("unknown arm {other:?} (use on / off)"),
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    set_leaping_michaels(on_from(&args.ew));
    let baseline = american().against(Family::NATURAL);
    set_leaping_michaels(on_from(&args.ns));
    let lm = american().against(Family::NATURAL);

    // Phase 1 (sequential, cheap): deal + the shape-only filter until `count`
    // boards pass. The RNG stays single-threaded so a seed reproduces a run.
    let mut passing: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while passing.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter && !could_reach_leaping_michaels(&deal) {
            continue;
        }
        passing.push(deal);
    }
    eprintln!("scanned {scanned} for {} boards; bidding...", passing.len());

    // Phase 2 (parallel): bidding is pure (the books read their thread-locals at
    // construction), so fan the two-table auctions across Rayon's work-stealing
    // pool — auction lengths vary, so dynamic balancing beats static chunks. The
    // DD solver stays on the main thread below; it parallelizes itself.
    let vul = args.vulnerability;
    let results: Vec<_> = passing
        .par_iter()
        .enumerate()
        .map(|(i, &deal)| {
            let dealer = Seat::ALL[i % 4];
            let table_a = bid_out(&lm, &baseline, true, dealer, vul, &deal);
            let table_b = bid_out(&lm, &baseline, false, dealer, vul, &deal);
            let contracts = (
                final_contract(&table_a, dealer),
                final_contract(&table_b, dealer),
            );
            (deal, contracts, (table_a, table_b))
        })
        .collect();

    let mut deals: Vec<FullDeal> = Vec::with_capacity(results.len());
    let mut contracts = Vec::with_capacity(results.len());
    let mut auctions: Vec<(Auction, Auction)> = Vec::with_capacity(results.len());
    for (deal, c, a) in results {
        deals.push(deal);
        contracts.push(c);
        auctions.push(a);
    }

    let scored = score_boards(&contracts, &deals, args.vulnerability, ns_score_contract);
    let (points, total_imps) = (scored.total_points, scored.total_imps);
    let mut worst: Vec<(i64, usize)> = scored
        .divergent
        .iter()
        .map(|&i| (scored.board_imps[i], i))
        .collect();
    worst.sort_by_key(|w| w.0);
    eprintln!("=== Worst 15 divergent boards for the --ns arm ===");
    for &(imp, i) in worst.iter().take(15) {
        let dealer = Seat::ALL[i % 4];
        eprintln!(
            "[{imp:+} IMP] dealer {dealer:?}  {}\n  A ({} NS): {} -> {:?}\n  B ({} NS): {} -> {:?}",
            deals[i],
            args.ns,
            auctions[i].0,
            contracts[i].0,
            args.ew,
            auctions[i].1,
            contracts[i].1,
        );
    }

    println!(
        "=== Contested Leaping Michaels A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "(opponent opens a weak two — NS {} vs EW {})",
        args.ns, args.ew,
    );
    if args.filter {
        println!(
            "(pre-filtered to plausible Leaping Michaels: kept {} of {scanned} dealt, {:.1}%)",
            args.count,
            100.0 * args.count as f64 / scanned.max(1) as f64,
        );
    }
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        scored.divergent.len(),
        args.count,
        100.0 * scored.divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "NS {} (vs EW {}): {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        args.ns,
        args.ew,
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / scored.divergent.len().max(1) as f64,
    );
}
