//! Responsive-double A/B, **contested**, under perfect-defense scoring (ledger #100).
//!
//! Two responsive doubles, each measured against the bare instinct floor:
//!
//! - `--conv takeout` (the **shipped** node): after partner doubles their opening
//!   and an opponent raises (`(1t)–X–(2t)–?`), advancer's double shows the two
//!   unbid suits with 8+. The measured pair carries it (default on); the baseline
//!   turns it off via [`set_responsive_takeout`], dropping the auction to the floor.
//! - `--conv overcall` (a non-standard **extension**): after partner *overcalls*
//!   and an opponent raises (`(1t)–overcall–(2t)–?`), advancer doubles to show the
//!   two suits unbid by opener and partner. Off by default; the measured pair turns
//!   it on via [`set_responsive_overcall`], the baseline leaves it floored.
//!
//! Both arms run the same 2/1 system, differing only in the one measured toggle,
//! read once at book-construction time. The convention fires only when the
//! *opponents* open and our side overcalls/doubles, so — like `sohl-after-double-ab`
//! — this uses the contested seat-swap duplicate match: at table A the measured
//! pair sits North/South against the floor East/West; at table B they swap. A board
//! whose tables reach different contracts is solved double dummy and the swing
//! credited to the measured pair under **perfect-defense** [`ns_score_contract`] (a failing
//! contract is scored doubled). A positive IMPs/board favors the convention.
//!
//! ```text
//! # Shipped takeout responsive double vs the floor (perfect-defense):
//! cargo run --release --example responsive-ab -- --count 200000 --filter --conv takeout
//! # Overcall-extension responsive double vs the floor:
//! cargo run --release --example responsive-ab -- --count 200000 --filter --conv overcall
//! # Vulnerable variant:
//! cargo run --release --example responsive-ab -- --count 200000 --filter --conv takeout -v both
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::{set_responsive_overcall, set_responsive_takeout};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};

/// Contested responsive-double A/B under perfect-defense scoring
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Which responsive double to measure vs the floor: takeout or overcall
    #[arg(long, default_value = "takeout")]
    conv: String,

    /// Only count deals that can plausibly reach `(1t)–action–(2t)` (a cheap shape
    /// pre-filter), so the DD budget lands on boards that can actually diverge.
    /// `--count` is then the number of such filtered boards.
    #[arg(long, default_value = "false")]
    filter: bool,
}

/// Total HCP of a hand
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// Cheap pre-filter (no bidding): could this deal plausibly reach `(1t)–action–(2t)`
/// with a responsive-double-shaped advancer?
///
/// The four roles, in seat order from the opener: opener (`S0`, 11–21 HCP, opens a
/// 1-bid), the actioner that overcalls/doubles (`S1` = LHO, 8+ — a superset of the
/// overcall 8–16 and takeout-double 12+ bands), opener's partner who raises (`S2`,
/// 5+), and the advancer (`S3` = opener's RHO) who makes the responsive double. The
/// rare, concentrating ingredient is the advancer: a responsive double needs 8+ HCP
/// and *two* side suits of 4+, so requiring exactly that on `S3` is still a **superset**
/// of the divergence condition (every divergent board has such an advancer) but cuts the
/// kept fraction to ~the ledger's working range, keeping `IMPs/board` comparable and the
/// DD budget on boards that can actually swing.
fn could_reach_overcalled_raise(deal: &FullDeal) -> bool {
    Seat::ALL.iter().any(|&opener| {
        if !(11..=21).contains(&hand_hcp(deal[opener])) {
            return false;
        }
        let lho = Seat::ALL[(opener as usize + 1) % 4];
        let partner = Seat::ALL[(opener as usize + 2) % 4];
        let advancer = Seat::ALL[(opener as usize + 3) % 4];
        let advancer_shaped = hand_hcp(deal[advancer]) >= 8
            && Suit::ASC
                .iter()
                .filter(|&&s| deal[advancer][s].len() >= 4)
                .count()
                >= 2;
        hand_hcp(deal[lho]) >= 8 && hand_hcp(deal[partner]) >= 5 && advancer_shaped
    })
}

/// Set both responsive toggles for the next book build: `measured` carries the
/// `conv` convention, the baseline (`measured == false`) drops it to the floor while
/// keeping the other toggle at its shipped default.
fn configure(conv: &str, measured: bool) {
    // Shipped defaults: takeout on, overcall off.
    set_responsive_takeout(true);
    set_responsive_overcall(false);
    match conv {
        "takeout" => set_responsive_takeout(measured),
        "overcall" => set_responsive_overcall(measured),
        other => panic!("unknown --conv {other:?} (use takeout or overcall)"),
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

/// Bid one deal with the convention pair on the side picked by `conv_is_ns`
fn bid_out(
    conv: &Stance,
    baseline: &Stance,
    conv_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == conv_is_ns {
            conv
        } else {
            baseline
        };
        auction.push(next_call(stance, deal[seat], dealer, vul, &auction));
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();

    configure(&args.conv, false);
    let baseline = american().against(Family::NATURAL);
    configure(&args.conv, true);
    let conv = american().against(Family::NATURAL);

    // Each board at both tables (conv NS at A, EW at B), dealer rotating.
    // With `--filter`, deal until `count` boards pass the cheap shape filter.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    let mut auctions: Vec<(Auction, Auction)> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while deals.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter && !could_reach_overcalled_raise(&deal) {
            continue;
        }
        let dealer = Seat::ALL[deals.len() % 4];
        let table_a = bid_out(&conv, &baseline, true, dealer, args.vulnerability, &deal);
        let table_b = bid_out(&conv, &baseline, false, dealer, args.vulnerability, &deal);
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

    // Only boards whose tables diverge can swing; solve those once and credit
    // the swing to the convention team (NS at A, EW at B).
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
    eprintln!(
        "=== Worst 15 divergent boards for the {} responsive double ===",
        args.conv
    );
    for &(imp, i) in worst.iter().take(15) {
        let dealer = Seat::ALL[i % 4];
        eprintln!(
            "[{imp:+} IMP] dealer {dealer:?}  {}\n  A (conv NS): {} -> {:?}\n  B (conv EW): {} -> {:?}",
            deals[i], auctions[i].0, contracts[i].0, auctions[i].1, contracts[i].1,
        );
    }

    println!(
        "=== Responsive-double A/B ({} vs floor): {} boards, vulnerability {} ===",
        args.conv, args.count, args.vulnerability,
    );
    if args.filter {
        println!(
            "(pre-filtered to plausible (1t)-action-(2t): kept {} of {scanned} dealt, {:.1}%)",
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
        "{} responsive double vs floor: {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/filtered-board, {:+.3} IMPs/divergent)",
        args.conv,
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / divergent.len().max(1) as f64,
    );
    // The filter-independent real-world rate (per *raw* deal dealt): the headline
    // effect size, unlike IMPs/filtered-board, does not move with the filter's tightness.
    println!(
        "Per raw deal: {:+.4} IMPs/board ({total_imps:+} IMPs over {scanned} dealt)",
        total_imps as f64 / scanned.max(1) as f64,
    );
}
