//! Sohl-after-a-takeout-double A/B, **contested**: the advancer's Lebensohl
//! after partner doubles their weak two (`(2X)–X–(P)–?`).
//!
//! The baseline leaves the advancer to the flat `advance_double` ladder (a weak
//! long-suit hand and a constructive hand both bid the same cheapest call, so the
//! doubler can't tell when to move). The **sohl** package separates them: weak
//! hands relay through `2NT` to a `3♣` sign-off (or correct), while constructive
//! hands bid a forcing 3-level suit (`plain`) or transfer up the line through the
//! adverse suit with the cue as Stayman (`transfer`) — so a game is found, not
//! stranded.
//!
//! Both arms run the same 2/1 system; the only difference is the
//! [`LebensohlStyle`] each pair carries for the advance of a takeout double
//! (`--ns` / `--ew`: off / plain / transfer), read once at book-construction
//! time via [`set_advance_sohl_style`]. The convention only fires when the
//! *opponents* open a weak two and our side doubles, so — like `lebensohl-ab` —
//! the opponents must bid. This uses the contested seat-swap duplicate match
//! (the `nt-shape-contested` template): at table A the measured (`--ns`) pair
//! sits North/South against the baseline (`--ew`) pair East/West; at table B
//! they swap. A board whose tables reach different contracts is solved double
//! dummy and the swing credited to the NS pair. A positive IMPs/board favors the
//! `--ns` style.
//!
//! ```text
//! # Transfer sohl vs the bare floor (the baseline):
//! cargo run --release --example ab-sohl-after-double -- --count 200000 --filter
//! # Plain sohl vs the floor:
//! cargo run --release --example ab-sohl-after-double -- --count 200000 --filter --ns plain
//! # Transfer vs plain (which sohl is best):
//! cargo run --release --example ab-sohl-after-double -- --count 200000 --filter --ns transfer --ew plain
//! ```

use clap::Parser;
use contract_bridge::auction::Auction;
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::{LebensohlStyle, set_advance_sohl_style, set_delayed_cue};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, hand_hcp};

/// Contested sohl-after-double A/B
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Sohl style for the measured (NS) pair: off, plain, transfer
    #[arg(long, default_value = "transfer")]
    ns: String,

    /// Sohl style for the baseline (EW) pair: off, plain, transfer
    #[arg(long, default_value = "off")]
    ew: String,

    /// Only count deals that can plausibly reach `(2X)–X–(P)` (a cheap shape
    /// pre-filter), so the DD budget lands on boards that can actually diverge.
    /// `--count` is then the number of such filtered boards.
    #[arg(long, default_value = "false")]
    filter: bool,

    /// Give the measured (NS) pair the stopper-split delayed cue (direct cue
    /// denies a stopper; the delayed 2NT-then-cue is Stayman with a stopper).
    /// Both pairs otherwise carry `--ns`/`--ew` Transfer, so this isolates the
    /// delayed cue. Only affects the (2♥)/(2♠) advances.
    #[arg(long, default_value = "false")]
    delayed_cue: bool,
}

/// A plausible weak-two opener: a six-card ♦/♥/♠ suit and 5–11 HCP. A loose
/// superset of the system's actual weak-two opening (the pre-filter only needs
/// to *contain* every deal that can diverge, not match the rule exactly).
fn is_weak_two_opener(hand: Hand) -> bool {
    (5..=11).contains(&hand_hcp(hand))
        && [Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .iter()
            .any(|&s| hand[s].len() == 6)
}

/// Cheap pre-filter (no bidding): could this deal plausibly reach `(2X)–X–(P)`?
///
/// Some seat is a weak-two opener whose left- or right-hand opponent holds 12+
/// HCP (takeout-double values). For an A/B that only diverges when our side
/// doubles a weak two and advances, this is a *superset* of the divergence
/// condition, so filtering on it concentrates the DD budget on relevant boards
/// without biasing the per-divergent estimate.
fn could_reach_weak_two_double(deal: &FullDeal) -> bool {
    Seat::ALL.iter().any(|&opener| {
        if !is_weak_two_opener(deal[opener]) {
            return false;
        }
        let lho = Seat::ALL[(opener as usize + 1) % 4];
        let rho = Seat::ALL[(opener as usize + 3) % 4];
        hand_hcp(deal[lho]) >= 12 || hand_hcp(deal[rho]) >= 12
    })
}

/// Parse a sohl style name (off / plain / transfer)
fn style_from(name: &str) -> LebensohlStyle {
    match name {
        "off" => LebensohlStyle::Off,
        "plain" => LebensohlStyle::Plain,
        "transfer" => LebensohlStyle::Transfer,
        other => panic!("unknown sohl style {other:?} (use off / plain / transfer)"),
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    set_delayed_cue(false);
    set_advance_sohl_style(style_from(&args.ew));
    let baseline = american().against(Family::NATURAL);
    set_delayed_cue(args.delayed_cue);
    set_advance_sohl_style(style_from(&args.ns));
    let sohl = american().against(Family::NATURAL);
    set_delayed_cue(false);

    // Phase 1 (sequential, cheap): deal + the shape-only filter until `count`
    // boards pass. The RNG stays single-threaded so a seed reproduces a run.
    let mut passing: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while passing.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter && !could_reach_weak_two_double(&deal) {
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
            let table_a = bid_out(&sohl, &baseline, true, dealer, vul, &deal);
            let table_b = bid_out(&sohl, &baseline, false, dealer, vul, &deal);
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

    // Only boards whose tables diverge can swing; solve those once and credit
    // the swing to the sohl team (NS at A, EW at B).
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    let mut worst: Vec<(i64, usize)> = Vec::new();
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[i];
        let swing = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        points += swing;
        total_imps += imps(swing);
        worst.push((imps(swing), i));
    }
    worst.sort_by_key(|w| w.0);
    eprintln!("=== Worst 15 divergent boards for the --ns style ===");
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
        "=== Contested sohl-after-double A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "(opponents open a weak two, we double and advance — NS {} vs EW {})",
        args.ns, args.ew,
    );
    if args.filter {
        println!(
            "(pre-filtered to plausible (2X)–X–(P): kept {} of {scanned} dealt, {:.1}%)",
            args.count,
            100.0 * args.count as f64 / scanned.max(1) as f64,
        );
    }
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "NS {} (vs EW {}): {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        args.ns,
        args.ew,
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
}
