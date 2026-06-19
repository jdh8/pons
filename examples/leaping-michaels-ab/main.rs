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
//! cargo run --release --example leaping-michaels-ab -- --count 200000 --filter
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::set_leaping_michaels;
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score};

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

    /// Use the live double-dummy **search** bidder (`american_search`) for the
    /// measured (NS) pair instead of the authored-rules floor, so the advance is
    /// chosen by cardplay EV (and can reach slam).  Requires `--features search`;
    /// slow, so pair with a small `--count`.
    #[arg(long, default_value = "false")]
    search: bool,
}

/// Total HCP of a hand
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
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

/// Build the measured pair with the live-search bidder (knobs trimmed for speed)
#[cfg(feature = "search")]
fn build_search() -> Stance {
    use pons::bidding::american::american_search_with;
    use pons::bidding::search_floor::SearchFloor;
    // Half the default layouts: ~2× faster, still enough to rank game vs slam.
    american_search_with(SearchFloor {
        layouts: 64,
        shortlist: 8,
        temperature: 100.0,
    })
    .against(Family::NATURAL)
}

/// Without the `search` feature the search bidder is unavailable.
#[cfg(not(feature = "search"))]
fn build_search() -> Stance {
    panic!("--search requires building with --features search");
}

/// Parse an on/off arm flag
fn on_from(name: &str) -> bool {
    match name {
        "on" => true,
        "off" => false,
        other => panic!("unknown arm {other:?} (use on / off)"),
    }
}

/// The seat acting after `len` calls from `dealer`
const fn seat_to_act(dealer: Seat, len: usize) -> Seat {
    Seat::ALL[(dealer as usize + len) % 4]
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

/// Bid one deal with the Leaping Michaels pair on the side picked by `lm_is_ns`
fn bid_out(
    lm: &Stance,
    baseline: &Stance,
    lm_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == lm_is_ns { lm } else { baseline };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    set_leaping_michaels(on_from(&args.ew));
    let baseline = american().against(Family::NATURAL);
    set_leaping_michaels(on_from(&args.ns));
    let lm = if args.search {
        build_search()
    } else {
        american().against(Family::NATURAL)
    };

    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    let mut auctions: Vec<(Auction, Auction)> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while deals.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter && !could_reach_leaping_michaels(&deal) {
            continue;
        }
        let dealer = Seat::ALL[deals.len() % 4];
        let table_a = bid_out(&lm, &baseline, true, dealer, args.vulnerability, &deal);
        let table_b = bid_out(&lm, &baseline, false, dealer, args.vulnerability, &deal);
        deals.push(deal);
        contracts.push((
            final_contract(&table_a, dealer),
            final_contract(&table_b, dealer),
        ));
        auctions.push((table_a, table_b));
        if deals.len().is_multiple_of(1000) {
            eprint!("\rbid {}/{} (scanned {scanned})", deals.len(), args.count);
        }
    }
    eprintln!();

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
        let swing = ns_score(contract_a, table, args.vulnerability)
            - ns_score(contract_b, table, args.vulnerability);
        points += swing;
        total_imps += imps(swing);
        worst.push((imps(swing), i));
    }
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
