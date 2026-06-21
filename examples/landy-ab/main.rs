//! Landy defense to their 1NT A/B, **contested**, plain-DD duplicate (the A/B
//! standard `ns_score_contract`).
//!
//! Landy (`set_landy`) turns `2♣` into both majors (at least 5-4) and `2NT` into
//! both minors over an opponent's 1NT, replacing the natural `2♣` club overcall.
//! The measured pair carries Landy on the configured points range; the baseline
//! pair keeps today's default (natural overcalls + penalty double, Landy off).
//!
//! Both arms run the same 2/1 system, differing only in the Landy toggle, read
//! once at book-construction time. The convention fires only when the *opponents*
//! open 1NT and our side overcalls, so this uses the contested seat-swap duplicate
//! match: at table A the measured pair sits North/South against the baseline
//! East/West; at table B they swap. A board whose tables reach different contracts
//! is solved double dummy and the swing credited to the measured pair. A positive
//! IMPs/board favors Landy.
//!
//! `--ns-range LO[:HI]` is the strength sweep knob for the `2♣` overcall: `8`
//! means 8+ (open-topped, "unlimited"), `8:15` means 8–15. The advancer's
//! invite/game thresholds and the overcaller's min/med/max rebid track it.
//!
//! ```text
//! # Landy 8+ vs the default defense (none vul), 200k filtered boards:
//! cargo run --release --example landy-ab -- --count 200000 --filter --ns-range 8
//! # Sweep the floor:
//! for lo in 6 8 9 10 11; do
//!   cargo run --release --example landy-ab -- --count 200000 --filter --ns-range $lo
//! done
//! # Vulnerable variant:
//! cargo run --release --example landy-ab -- --count 200000 --filter --ns-range 8 -v both
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::american::{
    set_landy, set_landy_hcp, set_natural_defense, set_unusual_notrump_defense,
};
use pons::bidding::context::relative;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// Contested Landy-vs-default A/B under plain-DD duplicate scoring
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "200000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Landy `2♣` (both majors) points range: `LO` (open-topped) or `LO:HI`. Empty
    /// disables the `2♣` part (e.g. to isolate the `2NT` minors).
    #[arg(long, default_value = "8")]
    ns_majors: String,

    /// Both-minors `2NT` (5-5) points range, same format. Empty disables it. Set
    /// this alone (with `--ns-majors ""`) to measure the unusual 2NT vs the floor.
    #[arg(long, default_value = "")]
    ns_minors: String,

    /// Strength gauge for the two-suiter overcalls: `points` (default) or `hcp`
    #[arg(long, default_value = "points")]
    strength: String,

    /// Natural one-suiter defense (penalty X + natural 2♣/♦/♥/♠) for the *measured*
    /// pair: `on` (default) or `off`. Set the measured arm `on` and the baseline arm
    /// `off` (with `--ns-majors "" --ns-minors ""`) to measure the natural defense
    /// vs the bare instinct floor.
    #[arg(long, default_value = "on")]
    ns_natural: String,

    /// Natural one-suiter defense for the *baseline* pair: `on` (default) or `off`.
    /// `off` drops the baseline pair to the floor over their 1NT.
    #[arg(long, default_value = "on")]
    ew_natural: String,

    /// Only count deals that can plausibly reach a Landy overcall of 1NT (a cheap
    /// shape pre-filter), so the DD budget lands on boards that can actually
    /// diverge. `--count` is then the number of such filtered boards.
    #[arg(long, default_value = "false")]
    filter: bool,

    /// RNG seed (fixed by default, so a floor sweep compares on identical boards)
    #[arg(long, default_value = "20260622")]
    seed: u64,
}

/// Total HCP of a hand
fn hand_hcp(hand: Hand) -> u8 {
    Suit::ASC.iter().map(|&s| holding_hcp::<u8>(hand[s])).sum()
}

/// Parse a range spec: `""` → `None` (off); `"8"` → `Some((8, 37))` (open-topped);
/// `"8:15"` → `Some((8, 15))`.
fn parse_range(spec: &str) -> Option<(u8, u8)> {
    if spec.is_empty() {
        return None;
    }
    Some(match spec.split_once(':') {
        Some((lo, hi)) => (
            lo.parse().expect("range LO is a number"),
            hi.parse().expect("range HI is a number"),
        ),
        None => (spec.parse().expect("range LO is a number"), 37),
    })
}

/// Parse an `on`/`off` flag into a bool, panicking on anything else.
fn parse_on_off(spec: &str, flag: &str) -> bool {
    match spec {
        "on" => true,
        "off" => false,
        other => panic!("unknown {flag} {other:?} (use on or off)"),
    }
}

/// Render a range for the headline: `None` → `"off"`, open-top → `"8+"`, else `"8-15"`.
fn label(range: Option<(u8, u8)>) -> String {
    match range {
        None => "off".to_string(),
        Some((lo, hi)) if hi >= 37 => format!("{lo}+"),
        Some((lo, hi)) => format!("{lo}-{hi}"),
    }
}

/// Balanced shape (no singleton/void, at most one doubleton) with HCP in `lo..=hi`
fn is_balanced_hcp(hand: Hand, lo: u8, hi: u8) -> bool {
    let lengths = Suit::ASC.map(|s| hand[s].len());
    let balanced =
        lengths.iter().all(|&l| l >= 2) && lengths.iter().filter(|&&l| l == 2).count() <= 1;
    balanced && (lo..=hi).contains(&hand_hcp(hand))
}

/// At least 5-4 (or 4-5) in the two named suits — the 2♣ majors shape
fn two_suiter_5_4(hand: Hand, a: Suit, b: Suit) -> bool {
    let (x, y) = (hand[a].len(), hand[b].len());
    (x >= 5 && y >= 4) || (x >= 4 && y >= 5)
}

/// At least 5-5 in the two named suits — the 2NT minors shape
fn two_suiter_5_5(hand: Hand, a: Suit, b: Suit) -> bool {
    hand[a].len() >= 5 && hand[b].len() >= 5
}

/// Could this defender make a *natural* call over their 1NT — a one-suiter overcall
/// (a 5+ card suit with overcall-ish strength) or a penalty double (strong balanced)?
/// Generous bands around the authored `points(8..=14)` / `15+ balanced`, so the
/// divergence superset stays a superset.
fn defender_has_natural_action(hand: Hand) -> bool {
    let hcp = hand_hcp(hand);
    let longest = Suit::ASC.iter().map(|&s| hand[s].len()).max().unwrap_or(0);
    (longest >= 5 && (6..=16).contains(&hcp)) || is_balanced_hcp(hand, 14, 40)
}

/// Cheap pre-filter (no bidding): could this deal plausibly reach an enabled
/// two-suiter overcall of a 1NT opening?
///
/// A superset of the divergence condition: a balanced 14–18 HCP hand (a 1NT-opener
/// candidate — generous around the 15–17 `fifths` band so no real opener is missed)
/// with a defender (its LHO or RHO) holding the shape of an *active* arm (5-4 majors
/// when `2♣` is on, 5-5 minors when `2NT` is on). Every divergent board has this;
/// keying on only the active arms keeps the DD budget on boards that can swing.
fn could_reach_landy(deal: &FullDeal, majors: bool, minors: bool, natural: bool) -> bool {
    Seat::ALL.iter().any(|&opener| {
        if !is_balanced_hcp(deal[opener], 14, 18) {
            return false;
        }
        let lho = Seat::ALL[(opener as usize + 1) % 4];
        let rho = Seat::ALL[(opener as usize + 3) % 4];
        [lho, rho].iter().any(|&d| {
            (majors && two_suiter_5_4(deal[d], Suit::Hearts, Suit::Spades))
                || (minors && two_suiter_5_5(deal[d], Suit::Clubs, Suit::Diamonds))
                || (natural && defender_has_natural_action(deal[d]))
        })
    })
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

/// Bid one deal with the Landy (measured) pair on the side picked by `landy_is_ns`
fn bid_out(
    measured: &Stance,
    baseline: &Stance,
    landy_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let stance = if seat_is_ns == landy_is_ns {
            measured
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
    let majors = parse_range(&args.ns_majors);
    let minors = parse_range(&args.ns_minors);
    let mut rng = StdRng::seed_from_u64(args.seed);

    // Baseline = the bare floor (both two-suiters off); measured = the configured
    // majors (2♣) and/or minors (2NT). Set both toggles each build so neither leaks
    // across the two book constructions.
    let use_hcp = match args.strength.as_str() {
        "hcp" => true,
        "points" => false,
        other => panic!("unknown --strength {other:?} (use points or hcp)"),
    };
    let ns_natural = parse_on_off(&args.ns_natural, "--ns-natural");
    let ew_natural = parse_on_off(&args.ew_natural, "--ew-natural");
    set_landy(None);
    set_unusual_notrump_defense(None);
    set_landy_hcp(false);
    set_natural_defense(ew_natural);
    let baseline = american().against(Family::NATURAL);
    set_landy(majors);
    set_unusual_notrump_defense(minors);
    set_landy_hcp(use_hcp);
    set_natural_defense(ns_natural);
    let measured = american().against(Family::NATURAL);

    // Each board at both tables (Landy NS at A, EW at B), dealer rotating.
    // With `--filter`, deal until `count` boards pass the cheap shape filter.
    let mut deals: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut contracts = Vec::with_capacity(args.count);
    let mut auctions: Vec<(Auction, Auction)> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while deals.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter
            && !could_reach_landy(
                &deal,
                majors.is_some(),
                minors.is_some(),
                ns_natural != ew_natural,
            )
        {
            continue;
        }
        let dealer = Seat::ALL[deals.len() % 4];
        let table_a = bid_out(
            &measured,
            &baseline,
            true,
            dealer,
            args.vulnerability,
            &deal,
        );
        let table_b = bid_out(
            &measured,
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

    // Only boards whose tables diverge can swing; solve those once and credit the
    // swing to the Landy team (NS at A, EW at B).
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
    eprintln!("=== Worst 15 divergent boards for Landy ===");
    for &(imp, i) in worst.iter().take(15) {
        let dealer = Seat::ALL[i % 4];
        eprintln!(
            "[{imp:+} IMP] dealer {dealer:?}  {}\n  A (Landy NS): {} -> {:?}\n  B (Landy EW): {} -> {:?}",
            deals[i], auctions[i].0, contracts[i].0, auctions[i].1, contracts[i].1,
        );
    }

    let arms = format!(
        "2♣ majors {}, 2NT minors {} [{}], natural NS {}/EW {}",
        label(majors),
        label(minors),
        args.strength,
        if ns_natural { "on" } else { "off" },
        if ew_natural { "on" } else { "off" },
    );
    println!(
        "=== Landy-vs-default A/B ({arms}): {} boards, vulnerability {} ===",
        args.count, args.vulnerability,
    );
    if args.filter {
        println!(
            "(pre-filtered to plausible Landy of 1NT: kept {} of {scanned} dealt, {:.1}%)",
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
        "Measured ({arms}) vs default: {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/filtered-board, {:+.3} IMPs/divergent)",
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
