//! Meckstroth-adjunct A/B: baseline opener rebids vs the `3m = INV` jumps.
//!
//! After `1M – 1NT` (or `1♥ – 1♠`) opener's medium shapely hands (5-5 / 6-5,
//! ≈15–17 points) have no descriptive rebid in the baseline — they underbid as
//! a natural two-level minor.  The **Meckstroth adjunct** adds an invitational
//! `3♣`/`3♦` jump (5+ of the minor) so responder can accept game with a fit or a
//! maximum.  Both arms run the same 2/1 system; the only difference is the
//! [`set_meckstroth_adjunct`] toggle, read once at book-construction time.
//!
//! Opponents are silenced (East/West always pass), so every auction is
//! constructive start to finish — this measures the *constructive* value of the
//! adjunct.  Each board is bid twice over the same deal, once per arm; boards
//! whose arms reach different contracts are solved double dummy once and scored.
//! A positive IMPs/board favors the adjunct.
//!
//! ```text
//! cargo run --release --example meckstroth-abc -- --count 5000
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::set_meckstroth_adjunct;
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score};

/// Meckstroth-adjunct A/B: baseline rebids vs the `3m = INV` jumps
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
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

/// Bid one deal with the opponents (East/West) forced to pass throughout
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

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = rand::rng();
    // arm 0 = baseline (no adjunct), arm 1 = adjunct (the shipped default).
    // The toggle is read at book-construction time, so build each arm under its
    // own setting; the baked tries are independent thereafter.
    set_meckstroth_adjunct(false);
    let baseline = american().against(Family::NATURAL);
    set_meckstroth_adjunct(true);
    let adjunct = american().against(Family::NATURAL);
    let stances = [baseline, adjunct];

    // Both arms bid the same deal; the only difference is opener's rebid table.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    for index in 0..args.count {
        let dealer = Seat::ALL[index % 4];
        let deal = full_deal(&mut rng);
        let board: [_; 2] = std::array::from_fn(|arm| {
            let auction = bid_uncontested(&stances[arm], dealer, args.vulnerability, &deal);
            final_contract(&auction, dealer)
        });
        deals.push(deal);
        contracts.push(board);
        eprint!("\rbid {}/{}", index + 1, args.count);
    }
    eprintln!();

    // Only boards whose arms diverge can swing; solve those once.
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i][0] != contracts[i][1])
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    for (&i, table) in divergent.iter().zip(tables.iter()) {
        let base = ns_score(contracts[i][0], table, args.vulnerability);
        let adj = ns_score(contracts[i][1], table, args.vulnerability);
        points += adj - base;
        total_imps += imps(adj - base);
    }

    println!(
        "=== Meckstroth-adjunct A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!("(opponents silenced — constructive value only)");
    println!(
        "Divergent boards: {} of {} ({:.1}%)",
        divergent.len(),
        args.count,
        100.0 * divergent.len() as f64 / args.count.max(1) as f64,
    );
    println!(
        "Adjunct (3m = INV): {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board)",
        total_imps as f64 / args.count.max(1) as f64,
    );
}
