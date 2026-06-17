//! 1NT-shape A/B, **contested**: classic balanced 1NT vs the wide redesign,
//! with the opponents bidding.
//!
//! The companion `nt-shape-abc` silences the opponents and so sees only the
//! *constructive* value of the wider 1NT (a 5422 with a five-card minor — the
//! shipped [`two_over_one`] vs the balanced-only [`two_over_one_classic`]).  The
//! redesign's real case is competitive: a 1NT
//! opening steals bidding space, describes the hand in one bid, and right-sides
//! the contract.  This harness measures that.
//!
//! A seat-swap duplicate match (the `instinct-floor` template): at table A the
//! redesign pair sits North/South against the baseline pair East/West; at table
//! B they swap seats.  Both pairs bid — the auctions are fully contested.  A
//! board whose tables reach different contracts is solved double dummy and the
//! swing credited to the redesign team (NS at A, EW at B).  A positive
//! IMPs/board favors the redesign.
//!
//! `--baseline` and `--redesign` each pick a shape policy — `classic`
//! (balanced only), `wide` (the shipped 5422-minor default), or `wide6322` (also
//! a 6322 with a six-card minor) — so the harness measures any pair of policies.
//! The default `classic` vs `wide` measures the shipped redesign; `--baseline
//! wide --redesign wide6322` measures the marginal value of adding 6322.
//!
//! ```text
//! cargo run --release --example nt-shape-contested -- --count 20000 --vulnerability ns
//! cargo run --release --example nt-shape-contested -- --baseline wide --redesign wide6322 --count 100000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::bidding::context::relative;
use pons::bidding::two_over_one::{two_over_one_classic, two_over_one_wide_6322};
use pons::bidding::{Family, Pair, Stance, System};
use pons::scoring::{final_contract, imps, ns_score};
use pons::two_over_one;

/// Contested 1NT-shape A/B between two opening-shape policies
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "20000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Baseline arm: classic | wide | wide6322
    #[arg(short, long, default_value = "classic")]
    baseline: String,

    /// Redesign arm (the one the swing is credited to): classic | wide | wide6322
    #[arg(short, long, default_value = "wide")]
    redesign: String,
}

/// The 2/1 pair for a shape-policy name (`classic` / `wide` / `wide6322`)
fn system(name: &str) -> Pair {
    match name {
        "classic" => two_over_one_classic(),
        "wide" => two_over_one(),
        "wide6322" => two_over_one_wide_6322(),
        other => panic!("unknown shape policy {other:?} (classic | wide | wide6322)"),
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

/// Bid one deal with the redesign pair on the side picked by `redesign_is_ns`
fn bid_out(
    redesign: &Stance,
    baseline: &Stance,
    redesign_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == redesign_is_ns {
            redesign
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
    let redesign = system(&args.redesign).against(Family::NATURAL);
    let baseline = system(&args.baseline).against(Family::NATURAL);

    // Each board at both tables (redesign NS at A, EW at B), dealer rotating.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    for index in 0..args.count {
        let dealer = Seat::ALL[index % 4];
        let deal = full_deal(&mut rng);
        let table_a = bid_out(
            &redesign,
            &baseline,
            true,
            dealer,
            args.vulnerability,
            &deal,
        );
        let table_b = bid_out(
            &redesign,
            &baseline,
            false,
            dealer,
            args.vulnerability,
            &deal,
        );
        deals.push(deal);
        contracts.push((
            final_contract(&table_a, dealer),
            final_contract(&table_b, dealer),
        ));
        eprint!("\rbid {}/{}", index + 1, args.count);
    }
    eprintln!();

    // Only boards whose tables diverge can swing; solve those once and credit
    // the swing to the redesign team (NS at A, EW at B).
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let (contract_a, contract_b) = contracts[i];
        let swing = ns_score(contract_a, table, args.vulnerability)
            - ns_score(contract_b, table, args.vulnerability);
        points += swing;
        total_imps += imps(swing);
    }

    println!(
        "=== Contested 1NT-shape A/B: {} vs {}, {} boards, vulnerability {} ===",
        args.redesign, args.baseline, args.count, args.vulnerability,
    );
    println!("(opponents bid — competitive value included; shape change only)");
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "{} (vs {}): {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board)",
        args.redesign,
        args.baseline,
        total_imps as f64 / args.count.max(1) as f64,
    );
}
