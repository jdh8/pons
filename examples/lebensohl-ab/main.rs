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
//! # Transfer Lebensohl vs plain Lebensohl (the incumbent):
//! cargo run --release --example lebensohl-ab -- --count 50000
//! # Transfer Lebensohl vs the bare instinct floor (the v1 baseline):
//! cargo run --release --example lebensohl-ab -- --count 50000 --ew off
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::{LebensohlStyle, set_lebensohl_style};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Contested Lebensohl A/B
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "50000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Lebensohl style, measured (NS) pair: off/plain/transfer/rubensohl/transfersmolen
    #[arg(long, default_value = "transfer")]
    ns: String,

    /// Lebensohl style, baseline (EW) pair: off/plain/transfer/rubensohl/transfersmolen
    #[arg(long, default_value = "plain")]
    ew: String,

    /// RNG seed (fixed by default so before/after builds deal identical boards —
    /// the two-binary comparison for a structural change to the default book)
    #[arg(long, default_value = "20260620")]
    seed: u64,

    /// Only count deals that can plausibly reach `1NT–(2♦/2♥)` (a cheap shape
    /// pre-filter), so the DD budget lands on boards that can actually diverge.
    /// `--count` is then the number of such filtered boards.
    #[arg(long, default_value = "false")]
    filter_dh: bool,

    /// Restrict the totals and the worst-board list to boards whose auction
    /// reached a Transfer-Lebensohl *top-step* clubs transfer
    /// (`1NT (2♦/2♥) 3♠` or `1NT (2♠) 3♥`) — the boards the top-step fix changed.
    #[arg(long, default_value = "false")]
    only_topstep: bool,
}

/// Does this auction contain a top-step clubs transfer (`1NT (2♦/2♥) 3♠` or
/// `1NT (2♠) 3♥`) — i.e. is this a board the top-step fix can change?
fn contains_top_step(auction: &[Call]) -> bool {
    let nt = Call::Bid(Bid::new(1, Strain::Notrump));
    auction.windows(3).any(|w| {
        let (Call::Bid(over), Call::Bid(resp)) = (w[1], w[2]) else {
            return false;
        };
        if w[0] != nt {
            return false;
        }
        let top = if over == Bid::new(2, Strain::Diamonds) || over == Bid::new(2, Strain::Hearts) {
            Bid::new(3, Strain::Spades)
        } else if over == Bid::new(2, Strain::Spades) {
            Bid::new(3, Strain::Hearts)
        } else {
            return false;
        };
        resp == top
    })
}

/// Total HCP of a hand
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// A balanced 15–17 (a `1NT` opener)
fn is_1nt_opener(hand: Hand) -> bool {
    let lengths = Suit::ASC.map(|s| hand[s].len());
    let balanced =
        lengths.iter().all(|&l| l >= 2) && lengths.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (15..=17).contains(&hand_hcp(hand))
}

/// Cheap pre-filter (no bidding): could this deal plausibly reach `1NT–(2♦/2♥)`?
///
/// Some seat is a `1NT` opener whose left-hand opponent holds a five-card diamond
/// or heart suit. For an A/B that only diverges on red-suit overcalls of our 1NT,
/// this is a *superset* of the divergence condition, so filtering on it concentrates
/// the DD budget on relevant boards without biasing the per-divergent estimate.
fn could_reach_1nt_dh(deal: &FullDeal) -> bool {
    Seat::ALL.iter().any(|&opener| {
        let lho = Seat::ALL[(opener as usize + 1) % 4];
        is_1nt_opener(deal[opener])
            && (deal[lho][Suit::Diamonds].len() >= 5 || deal[lho][Suit::Hearts].len() >= 5)
    })
}

/// Parse a Lebensohl style name (off / plain / transfer)
fn style_from(name: &str) -> LebensohlStyle {
    match name {
        "off" => LebensohlStyle::Off,
        "plain" => LebensohlStyle::Plain,
        "transfer" => LebensohlStyle::Transfer,
        "rubensohl" => LebensohlStyle::Rubensohl,
        "transfersmolen" => LebensohlStyle::TransferSmolen,
        other => {
            panic!(
                "unknown lebensohl style {other:?} \
                 (use off / plain / transfer / rubensohl / transfersmolen)"
            )
        }
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
    let mut rng = StdRng::seed_from_u64(args.seed);

    set_lebensohl_style(style_from(&args.ew));
    let baseline = american().against(Family::NATURAL);
    set_lebensohl_style(style_from(&args.ns));
    let lebensohl = american().against(Family::NATURAL);

    // Each board at both tables (Lebensohl NS at A, EW at B), dealer rotating.
    // With `--filter-dh`, deal until `count` boards pass the cheap shape filter.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    let mut auctions: Vec<(Auction, Auction)> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while deals.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter_dh && !could_reach_1nt_dh(&deal) {
            continue;
        }
        let dealer = Seat::ALL[deals.len() % 4];
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
        auctions.push((table_a, table_b));
        if deals.len().is_multiple_of(1000) {
            eprint!("\rbid {}/{} (scanned {scanned})", deals.len(), args.count);
        }
    }
    eprintln!();

    // Only boards whose tables diverge can swing; solve those once and credit
    // the swing to the Lebensohl team (NS at A, EW at B).
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .filter(|&i| {
            !args.only_topstep
                || contains_top_step(&auctions[i].0)
                || contains_top_step(&auctions[i].1)
        })
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
        "=== Contested Lebensohl A/B: {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    println!(
        "(opponents overcall our 1NT — NS {} vs EW {})",
        args.ns, args.ew,
    );
    if args.filter_dh {
        println!(
            "(pre-filtered to plausible 1NT–(2♦/2♥): kept {} of {scanned} dealt, {:.1}%)",
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
