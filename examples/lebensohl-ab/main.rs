//! Lebensohl A/B, **contested**: responder's Lebensohl after our overcalled 1NT.
//!
//! When we open 1NT and an opponent overcalls, the baseline leaves responder to
//! the natural instinct floor.  The **Lebensohl** package (Section 5 of the
//! competitive book) separates weak from strong: weak hands relay through 2NT to
//! a 3♣ sign-off (or correct to a long suit), while game hands bid a forcing
//! 3-level suit or a to-play 3NT directly — so a game is never stranded in a
//! partscore.
//!
//! Both arms run the same 2/1 system; the only difference is the Lebensohl
//! [`LebensohlStyle`] each pair carries (`--ns` / `--ew`: off / plain /
//! transfer), read once at book-construction time.  Lebensohl only fires when
//! the opponents overcall our 1NT, so — unlike the constructive `*-abc`
//! harnesses — the opponents must bid.  This uses the contested seat-swap
//! duplicate match (the `nt-shape-contested` template): at table A the measured
//! (`--ns`) pair sits North/South against the baseline (`--ew`) pair East/West;
//! at table B they swap.  A board whose tables reach different contracts is
//! solved double dummy and the swing credited to the NS pair.  A positive
//! IMPs/board favors the `--ns` style.
//!
//! ```text
//! # Transfer Lebensohl (Rubensohl) vs plain Lebensohl (the incumbent):
//! cargo run --release --example lebensohl-ab -- --count 50000
//! # Transfer Lebensohl vs the bare instinct floor (the v1 baseline):
//! cargo run --release --example lebensohl-ab -- --count 50000 --ew off
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::{LebensohlStyle, set_lebensohl_style};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score};

/// Contested Lebensohl A/B
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "50000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Lebensohl style for the measured (NS) pair: off, plain, transfer
    #[arg(long, default_value = "transfer")]
    ns: String,

    /// Lebensohl style for the baseline (EW) pair: off, plain, transfer
    #[arg(long, default_value = "plain")]
    ew: String,
}

/// Parse a Lebensohl style name (off / plain / transfer)
fn style_from(name: &str) -> LebensohlStyle {
    match name {
        "off" => LebensohlStyle::Off,
        "plain" => LebensohlStyle::Plain,
        "transfer" => LebensohlStyle::Transfer,
        other => panic!("unknown lebensohl style {other:?} (use off / plain / transfer)"),
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

/// Bid one deal with the Lebensohl pair on the side picked by `lebensohl_is_ns`
fn bid_out(
    lebensohl: &Stance,
    baseline: &Stance,
    lebensohl_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == lebensohl_is_ns {
            lebensohl
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

    set_lebensohl_style(style_from(&args.ew));
    let baseline = american().against(Family::NATURAL);
    set_lebensohl_style(style_from(&args.ns));
    let lebensohl = american().against(Family::NATURAL);

    // Each board at both tables (Lebensohl NS at A, EW at B), dealer rotating.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    for index in 0..args.count {
        let dealer = Seat::ALL[index % 4];
        let deal = full_deal(&mut rng);
        let table_a = bid_out(
            &lebensohl,
            &baseline,
            true,
            dealer,
            args.vulnerability,
            &deal,
        );
        let table_b = bid_out(
            &lebensohl,
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
    // the swing to the Lebensohl team (NS at A, EW at B).
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
    eprintln!("=== Worst 15 divergent boards for Lebensohl ===");
    for &(imp, i) in worst.iter().take(15) {
        let dealer = Seat::ALL[i % 4];
        eprintln!(
            "[{imp:+} IMP] dealer {dealer:?}  A(lebensohl NS): {:?}  B(baseline NS): {:?}\n  {}",
            contracts[i].0, contracts[i].1, deals[i],
        );
    }

    println!(
        "=== Contested Lebensohl A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "(opponents overcall our 1NT — NS {} vs EW {})",
        args.ns, args.ew,
    );
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
