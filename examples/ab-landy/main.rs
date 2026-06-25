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
//! cargo run --release --example ab-landy -- --count 200000 --filter --ns-range 8
//! # Sweep the floor:
//! for lo in 6 8 9 10 11; do
//!   cargo run --release --example ab-landy -- --count 200000 --filter --ns-range $lo
//! done
//! # Vulnerable variant:
//! cargo run --release --example ab-landy -- --count 200000 --filter --ns-range 8 -v both
//! ```

use clap::Parser;
use contract_bridge::auction::{Auction, Call};
use contract_bridge::deck::full_deal;
use contract_bridge::{AbsoluteVulnerability, Bid, Contract, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver};
use pons::american;
use pons::bidding::Family;
use pons::bidding::american::{
    DoubleShape, PassedHandDefense, set_always_pass_defense, set_direct_dont,
    set_direct_landy_double, set_direct_landy_double_floor, set_direct_landy_penalty_pass,
    set_doubled_landy_escape, set_landy, set_landy_hcp, set_natural_defense,
    set_natural_double_shape, set_passed_hand_defense, set_penalty_pass,
    set_unusual_notrump_defense, set_woolsey, set_woolsey_double_floor, set_woolsey_points,
};
use pons::bidding::instinct::set_penalty_latch;
use pons::scoring::{final_contract, imps, ns_score_bid, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;

#[path = "../common/mod.rs"]
#[allow(dead_code)]
mod common;
use common::{bid_out, hand_hcp, seat_to_act};

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

    /// Shape gate of the *measured* pair's penalty double (15+ HCP fixed):
    /// `balanced` (default, the shipped 4333/4432/5332), `semibal` (also 5422/6322/
    /// 7222), or `any` (every 15+ hand). The baseline stays `balanced`. Pair with
    /// `--ns-majors ""` to isolate the double-shape change.
    #[arg(long, default_value = "balanced")]
    ns_double_shape: String,

    /// Passed-hand defense for the *measured* pair: `off` (default), `landy` (alias
    /// `on`), or `dont`. `landy` reassigns a passed hand's dead penalty double of
    /// their 1NT to both majors (≥5-4); `dont` is full DONT (X = one-suiter,
    /// 2♣/2♦/2♥ = two-suiters). Both gated on `passed_hand()` and exclude six-card
    /// (preemptable) shapes. Isolate with `--ns-majors "" --ns-minors ""` (both
    /// natural arms on): the only difference is then the passed-hand bids in the
    /// rare `[P,P,P,1NT]` seat, so read `IMPs/raw-deal`, not `IMPs/divergent`.
    /// Baseline always keeps it off.
    #[arg(long, default_value = "off")]
    ns_passed_dbl: String,

    /// Replace the *measured* pair's natural penalty-X over their 1NT with a
    /// both-majors takeout double (X = both majors, at every seat): `off` (default),
    /// `5-4` (≥5-4 in the majors), or `4-4` (a flat 4-4 accepted). Natural 2♣/♦/♥/♠
    /// overcalls are kept; the penalty double is dropped. Baseline keeps the natural
    /// penalty-X (`--ew-natural on`). Probes "replace the X with Landy" + 5-4 vs 4-4.
    #[arg(long, default_value = "off")]
    ns_landy_x: String,

    /// `points` floor for the *measured* pair's both-majors X (default 15, the
    /// shipped value — overcalls take 8–14, the X is reserved for the 15+ hands too
    /// strong to overcall). Lower it to compete lighter, raise it to compete less.
    /// Only matters with `--ns-landy-x`; the advancer's invite/game thresholds track it.
    #[arg(long, default_value = "15")]
    ns_landy_x_floor: u8,

    /// Let the *measured* pair's advancer pass the both-majors X for penalty (defend
    /// 1NTx) with no major fit and enough defense: `off` (default) or `on`. Pairs with
    /// a higher `--ns-landy-x-floor` (stronger X → the penalty pass needs less).
    #[arg(long, default_value = "off")]
    ns_landy_x_penalty: String,

    /// Replace the *measured* pair's natural 1NT defense with conventional DONT:
    /// `on` or `off` (default). One-suiter X, 2♣ = clubs + a higher major, 2♦ =
    /// diamonds + a major, 2♥ = both majors, 2♠ natural, 2NT = both minors. Forces
    /// `--ns-majors`/`--ns-minors` off (DONT owns 2♣/2NT). Pair with `--ew-natural
    /// on` for the head-to-head, or `--ew-always-pass on` for absolute worth.
    #[arg(long, default_value = "off")]
    ns_dont: String,

    /// Silly always-pass defense for the *baseline* pair: `on` or `off` (default).
    /// `on` makes the baseline never act over their 1NT — the truest "do nothing"
    /// baseline (distinct from `--ew-natural off`, which drops to the floor). Pair
    /// with `--ns-natural on --ns-majors "" --ns-minors ""` to measure the natural
    /// defense vs always-passing.
    #[arg(long, default_value = "off")]
    ew_always_pass: String,

    /// Opener's penalty-pass over a `(2♣)` overcall for the *measured* (NS) pair:
    /// `off` (default), `LEN:HCP`, or `LEN:HCP:major`. After `1NT-(2♣)-X-(P)` opener
    /// with `LEN+` clubs and `HCP+` club HCP passes to defend `2♣` doubled instead
    /// of answering the stolen Stayman; the `:major` suffix makes good clubs outrank
    /// a `2♥`/`2♠` major fit (else opener keeps the major). Run both NS and EW `on`
    /// (natural) and set this NS-only to isolate the conversion's value.
    #[arg(long, default_value = "off")]
    ns_penalty_pass: String,

    /// Opener's penalty-pass for the *baseline* (EW) pair, same format. Set this
    /// (with `--ns-natural on --ew-always-pass on`) to re-measure the value of NS's
    /// natural `2♣` overcall once the EW opener can punish it (the `2♣` row drops).
    #[arg(long, default_value = "off")]
    ew_penalty_pass: String,

    /// Doubled-Landy minor-escape gate for the *measured* (NS) pair: `MIN:MAJ`
    /// (default `6:2`). After `[1NT, 2♣, X]` the advancer runs to a long minor —
    /// `Pass` to play `2♣` doubled with clubs, `2♦` to play diamonds — with `MIN`+
    /// in that minor and ≤`MAJ` in each major. Only fires when Landy is on. Sweep to
    /// tune the escape vs. relaying/signing off into a major.
    #[arg(long, default_value = "6:2")]
    ns_doubled_escape: String,

    /// Replace the *measured* (NS) pair's 1NT defense with our Woolsey "Multi-Landy":
    /// `on` or `off` (default). X = 4-card major + longer minor, 2♣ = both majors,
    /// 2♦ = Multi (single 6+ major), 2♥/2♠ = Muiderberg. Owns every direct call, so
    /// it forces the measured natural / Landy / both-majors-X arms off. Baseline keeps
    /// its natural defense (`--ew-natural on`) for the head-to-head.
    #[arg(long, default_value = "off")]
    ns_woolsey: String,

    /// Woolsey suit-overcall (2♣/2♦/2♥/2♠) points band for the *measured* pair:
    /// `LO` (open-topped) or `LO:HI` (default `10:19`). Only matters with `--ns-woolsey`.
    /// The perfect-defense (`--score pd`) floor-sweep is monotonic: lower floors
    /// compete more and lose more, so the value is single-dummy obstruction (invisible
    /// to DD). 10 keeps a competing convention; 13 is the DD break-even.
    #[arg(long, default_value = "10:19")]
    ns_woolsey_range: String,

    /// `points` floor for the *measured* pair's Woolsey takeout X (default 12). Only
    /// matters with `--ns-woolsey`; the X advancer's game-ask threshold tracks it.
    #[arg(long, default_value = "12")]
    ns_woolsey_x_floor: u8,

    /// The penalty-double latch for the *measured* pair: `on` (default) or `off`.
    /// "Once penalty, always penalty" — after the natural penalty X of their 1NT,
    /// our later doubles read as penalty (double the runout on a stack, leave
    /// partner's double in) instead of takeout. Fires only for the side that made
    /// the penalty X (the measured pair), so it self-isolates against the baseline.
    /// Pass `off` for the A/B off arm.
    #[arg(long, default_value = "on")]
    ns_penalty_latch: String,

    /// Only count deals that can plausibly reach a Landy overcall of 1NT (a cheap
    /// shape pre-filter), so the DD budget lands on boards that can actually
    /// diverge. `--count` is then the number of such filtered boards.
    #[arg(long, default_value = "false")]
    filter: bool,

    /// RNG seed (fixed by default, so a floor sweep compares on identical boards)
    #[arg(long, default_value = "20260622")]
    seed: u64,

    /// How to score the reached contracts: `plain` (default, `ns_score_contract`,
    /// the actual penalty as bid) or `pd` (`ns_score_bid`, perfect-defense
    /// doubling — a contract that fails double-dummy is scored doubled). A weak
    /// competitive overbid that plain DD lets off is doubled under `pd`, so re-run
    /// any plain-DD positive under `pd` to catch the under-punishment artifact.
    #[arg(long, default_value = "plain")]
    score: String,
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

/// Parse a `--*-penalty-pass` spec: `off` → `None`; `"4:4"` → `Some((4, 4, false))`;
/// `"4:4:major"` → `Some((4, 4, true))` (good clubs outrank a major fit).
fn parse_penalty_pass(spec: &str, flag: &str) -> Option<(usize, u8, bool)> {
    if spec == "off" {
        return None;
    }
    let mut parts = spec.split(':');
    let len = parts.next().and_then(|s| s.parse().ok());
    let hcp = parts.next().and_then(|s| s.parse().ok());
    let over_major = match parts.next() {
        None => false,
        Some("major") => true,
        Some(other) => panic!("unknown {flag} regime {other:?} (use `major` or omit)"),
    };
    match (len, hcp) {
        (Some(len), Some(hcp)) => Some((len, hcp, over_major)),
        _ => panic!("bad {flag} {spec:?} (use off, LEN:HCP, or LEN:HCP:major)"),
    }
}

/// Parse an `on`/`off` flag into a bool, panicking on anything else.
fn parse_on_off(spec: &str, flag: &str) -> bool {
    match spec {
        "on" => true,
        "off" => false,
        other => panic!("unknown {flag} {other:?} (use on or off)"),
    }
}

/// Parse the `--ns-double-shape` flag into a [`DoubleShape`].
fn parse_double_shape(spec: &str) -> DoubleShape {
    match spec {
        "balanced" => DoubleShape::Balanced,
        "semibal" => DoubleShape::SemiBalanced,
        "any" => DoubleShape::Any,
        other => panic!("unknown --ns-double-shape {other:?} (use balanced, semibal, or any)"),
    }
}

/// The hand's shape as a sorted-descending length string, e.g. `"5422"`, `"7321"`.
fn shape_label(hand: Hand) -> String {
    let mut lengths = Suit::ASC.map(|s| hand[s].len());
    lengths.sort_unstable_by(|a, b| b.cmp(a));
    lengths.iter().map(usize::to_string).collect()
}

/// Exactly 5422/6322/7222 (one long suit, the rest doubleton-or-better, not balanced)
fn is_semibal(hand: Hand) -> bool {
    let mut lengths = Suit::ASC.map(|s| hand[s].len());
    lengths.sort_unstable();
    matches!(lengths, [2, 2, 4, 5] | [2, 2, 3, 6] | [2, 2, 2, 7])
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

/// A passed-hand DONT two-suiter shape: a 5-4 (or 5-5) two-suiter with no six-card
/// suit (the shapes the DONT `2♣`/`2♦`/`2♥` bids show — six-card suits open a
/// preempt in first seat and never reach `[P,P,P,1NT]`).  DONT one-suiters reach
/// the same suit and declarer as the natural overcall, so they never diverge; only
/// these two-suiters move the contract, so the filter need only catch them.
fn has_passed_two_suiter(hand: Hand) -> bool {
    let mut lengths = Suit::ASC.map(|s| hand[s].len());
    lengths.sort_unstable();
    let [_, _, second, longest] = lengths;
    longest == 5 && second >= 4
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

/// A defender who would double under the measured shape gate but *not* under the
/// baseline (`Balanced`) — i.e. a 15+ non-balanced hand of the widened shape. The
/// divergence source when isolating `--ns-double-shape` (majors/minors off).
fn defender_has_extended_double(hand: Hand, shape: DoubleShape) -> bool {
    // Balanced 15+ hands double in *both* arms, so they never diverge here.
    hand_hcp(hand) >= 15
        && !is_balanced_hcp(hand, 0, 40)
        && match shape {
            DoubleShape::Balanced => false,
            DoubleShape::SemiBalanced => is_semibal(hand),
            DoubleShape::Any => true,
        }
}

/// Cheap pre-filter (no bidding): could this deal plausibly reach an enabled
/// two-suiter overcall of a 1NT opening?
///
/// A superset of the divergence condition: a balanced 14–18 HCP hand (a 1NT-opener
/// candidate — generous around the 15–17 `fifths` band so no real opener is missed)
/// with a defender (its LHO or RHO) holding the shape of an *active* arm (5-4 majors
/// when `2♣` is on, 5-5 minors when `2NT` is on, or a widened-shape penalty double
/// when `extended` is set). Every divergent board has this; keying on only the active
/// arms keeps the DD budget on boards that can swing.
fn could_reach_landy(
    deal: &FullDeal,
    majors: bool,
    minors: bool,
    natural: bool,
    extended: Option<DoubleShape>,
    passed_style: Option<PassedHandDefense>,
) -> bool {
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
                || extended.is_some_and(|s| defender_has_extended_double(deal[d], s))
                // The passed-hand defense needs a two-suiter: just the majors for
                // NaturalLandyDouble (its only conventional bid), any 5-4 two-suiter
                // for DONT. ponytail: shape-only superset — low yield because
                // divergence also needs the rare [P,P,P,1NT] rotation, which a
                // seat-agnostic shape filter can't see; IMPs/raw-deal is the honest
                // effect size here.
                || match passed_style {
                    Some(PassedHandDefense::NaturalLandyDouble) => {
                        two_suiter_5_4(deal[d], Suit::Hearts, Suit::Spades)
                    }
                    Some(PassedHandDefense::Dont) => has_passed_two_suiter(deal[d]),
                    None => false,
                }
        })
    })
}

/// The natural pair's first non-pass call over the opponents' 1NT opening — the
/// defensive action (`X` / `2♣` / `2♦` / `2♥` / `2♠`) whose value over always-
/// passing we attribute the board's swing to. `None` if the natural pair opened
/// the 1NT (so never defended) or only passed. `natural_is_ns` says which side
/// carried the natural defense in this auction.
fn natural_action_over_1nt(
    auction: &[Call],
    dealer: Seat,
    natural_is_ns: bool,
) -> Option<(Seat, Call)> {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let i = auction.iter().position(|&c| c == one_nt)?;
    let opener_is_ns = matches!(seat_to_act(dealer, i), Seat::North | Seat::South);
    if opener_is_ns == natural_is_ns {
        return None; // the natural pair opened the 1NT, not defended it
    }
    auction[i + 1..].iter().enumerate().find_map(|(off, &c)| {
        let seat = seat_to_act(dealer, i + 1 + off);
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        (seat_is_ns == natural_is_ns && c != Call::Pass).then_some((seat, c))
    })
}

/// A short bucket label for an attributed defensive call (`X`, `2♣`, … or the
/// raw call for anything unexpected like a balancing jump).
fn action_label(call: Call) -> String {
    match call {
        Call::Double => "X".to_string(),
        Call::Bid(bid) => format!("{bid}"),
        other => format!("{other:?}"),
    }
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let majors = parse_range(&args.ns_majors);
    let minors = parse_range(&args.ns_minors);
    let double_shape = parse_double_shape(&args.ns_double_shape);
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
    let ns_dont = parse_on_off(&args.ns_dont, "--ns-dont");
    let ew_natural = parse_on_off(&args.ew_natural, "--ew-natural");
    let ew_always_pass = parse_on_off(&args.ew_always_pass, "--ew-always-pass");
    let passed_style = match args.ns_passed_dbl.as_str() {
        "off" => None,
        "on" | "landy" => Some(PassedHandDefense::NaturalLandyDouble),
        "dont" => Some(PassedHandDefense::Dont),
        other => panic!("unknown --ns-passed-dbl {other:?} (use off, landy, or dont)"),
    };
    let ns_landy_x = match args.ns_landy_x.as_str() {
        "off" => None,
        "5-4" => Some(false),
        "4-4" => Some(true),
        other => panic!("unknown --ns-landy-x {other:?} (use off, 5-4, or 4-4)"),
    };
    let ns_woolsey = parse_on_off(&args.ns_woolsey, "--ns-woolsey");
    let ns_penalty_latch = parse_on_off(&args.ns_penalty_latch, "--ns-penalty-latch");
    let woolsey_range = parse_range(&args.ns_woolsey_range).unwrap_or((9, 19));
    let ns_penalty_pass = parse_penalty_pass(&args.ns_penalty_pass, "--ns-penalty-pass");
    let ew_penalty_pass = parse_penalty_pass(&args.ew_penalty_pass, "--ew-penalty-pass");
    let ns_doubled_escape = {
        let (min, maj) = args
            .ns_doubled_escape
            .split_once(':')
            .expect("--ns-doubled-escape is MIN:MAJ");
        (
            min.parse::<usize>().expect("MIN is a number"),
            maj.parse::<usize>().expect("MAJ is a number"),
        )
    };
    set_landy(None);
    set_unusual_notrump_defense(None);
    set_landy_hcp(false);
    set_natural_defense(ew_natural);
    set_natural_double_shape(DoubleShape::Balanced);
    set_always_pass_defense(ew_always_pass);
    set_passed_hand_defense(None);
    set_direct_dont(false);
    set_direct_landy_double(None);
    set_direct_landy_double_floor(15);
    set_direct_landy_penalty_pass(false);
    set_woolsey(false);
    set_penalty_pass(ew_penalty_pass);
    let baseline = american().against(Family::NATURAL);
    set_landy(majors);
    set_unusual_notrump_defense(minors);
    set_landy_hcp(use_hcp);
    set_natural_defense(ns_natural);
    set_natural_double_shape(double_shape);
    set_always_pass_defense(false);
    set_passed_hand_defense(passed_style);
    set_direct_landy_double(ns_landy_x);
    set_direct_landy_double_floor(args.ns_landy_x_floor);
    set_direct_landy_penalty_pass(parse_on_off(
        &args.ns_landy_x_penalty,
        "--ns-landy-x-penalty",
    ));
    set_penalty_pass(ns_penalty_pass);
    set_doubled_landy_escape(ns_doubled_escape);
    // DONT owns 2♣ (two-suiter) and 2NT (both minors), so override the natural
    // Landy/Unusual overlays when it is on.
    set_direct_dont(ns_dont);
    if ns_dont {
        set_landy(None);
        set_unusual_notrump_defense(Some((8, 14)));
    }
    // Woolsey owns every direct call over their 1NT, so force the other measured
    // arms off — else their advance wiring would overwrite the Woolsey continuations.
    set_woolsey(ns_woolsey);
    set_woolsey_points(woolsey_range.0, woolsey_range.1);
    set_woolsey_double_floor(args.ns_woolsey_x_floor);
    if ns_woolsey {
        set_natural_defense(false);
        set_landy(None);
        set_direct_dont(false);
        set_direct_landy_double(None);
        set_passed_hand_defense(None);
    }
    let measured = american().against(Family::NATURAL);

    // Each board at both tables (Landy NS at A, EW at B), dealer rotating.
    // The baseline never acts when always-pass is on, so NS's natural action
    // diverges whenever it is enabled (no need to also differ from EW).
    // The both-majors X (--ns-landy-x) replaces the baseline's penalty-X, so every
    // hand the two arms call differently over their 1NT diverges; the broad
    // `defender_has_natural_action` superset (5+ suit, or 14+ balanced) catches them.
    // Woolsey replaces the measured pair's whole defense, so every hand it acts on
    // (all hold a 5+ suit, the `defender_has_natural_action` superset) diverges from
    // the baseline's natural defense.
    let natural_diverges = ns_landy_x.is_some()
        || ns_woolsey
        || if ew_always_pass {
            ns_natural
        } else {
            ns_natural != ew_natural
        };
    // The measured pair widens the double's shape gate above the baseline's
    // `Balanced`, so non-balanced 15+ hands are a fresh divergence source.
    let extended = (double_shape != DoubleShape::Balanced).then_some(double_shape);

    // Phase 1 (sequential, cheap): deal + the shape-only filter until `count`
    // boards pass. The RNG stays single-threaded so a seed reproduces a run.
    let mut passing: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while passing.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter
            && !could_reach_landy(
                &deal,
                majors.is_some(),
                minors.is_some(),
                natural_diverges,
                extended,
                passed_style,
            )
        {
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
            // The latch is a live-read instinct flag, so set it per worker thread
            // (Rayon workers do not inherit the main thread's thread-locals). It
            // fires only for the side that made the penalty X — the measured pair.
            set_penalty_latch(ns_penalty_latch);
            let dealer = Seat::ALL[i % 4];
            let table_a = bid_out(&measured, &baseline, true, dealer, vul, &deal);
            let table_b = bid_out(&measured, &baseline, false, dealer, vul, &deal);
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

    // Only boards whose tables diverge can swing; solve those once and credit the
    // swing to the Landy team (NS at A, EW at B).
    let divergent: Vec<usize> = (0..args.count)
        .filter(|&i| contracts[i].0 != contracts[i].1)
        .collect();
    let solve_deals: Vec<FullDeal> = divergent.iter().map(|&i| deals[i]).collect();
    let tables = Solver::lock().solve_deals(&solve_deals, NonEmptyStrainFlags::ALL);

    let mut points = 0i64;
    let mut total_imps = 0i64;
    let mut worst: Vec<(i64, usize, usize)> = Vec::new();
    // Per-defensive-action tally: label -> (boards, IMPs). The natural action that
    // drives a board's divergence appears at exactly one table (the one where the
    // natural pair are the *defenders* of the 1NT).
    let mut by_action: std::collections::BTreeMap<String, (i64, i64)> =
        std::collections::BTreeMap::new();
    // Per-doubler-shape tally: sorted-length label -> (boards, IMPs). When isolating
    // `--ns-double-shape`, every divergent board is triggered by a non-balanced 15+
    // doubler, so each row is that shape's marginal gain over the balanced baseline.
    let mut by_shape: std::collections::BTreeMap<String, (i64, i64)> =
        std::collections::BTreeMap::new();
    let pd = match args.score.as_str() {
        "plain" => false,
        "pd" => true,
        other => panic!("unknown --score {other:?} (use plain or pd)"),
    };
    // Plain DD prices the contract's actual penalty; PD doubles any contract that
    // fails double-dummy (perfect-defense), which punishes a weak overbid.
    let score = |c: Option<(_, Seat)>, table: &_, vul| {
        if pd {
            ns_score_bid(c.map(|(ct, s): (Contract, _)| (ct.bid, s)), table, vul)
        } else {
            ns_score_contract(c, table, vul)
        }
    };
    for (pos, (&i, table)) in divergent.iter().zip(tables.iter()).enumerate() {
        let (contract_a, contract_b) = contracts[i];
        let swing = score(contract_a, table, args.vulnerability)
            - score(contract_b, table, args.vulnerability);
        let board_imps = imps(swing);
        points += swing;
        total_imps += board_imps;
        worst.push((board_imps, i, pos));
        let dealer = Seat::ALL[i % 4];
        let actor = natural_action_over_1nt(&auctions[i].0, dealer, true)
            .or_else(|| natural_action_over_1nt(&auctions[i].1, dealer, false));
        let key = actor.map_or_else(|| "(other)".to_string(), |(_, call)| action_label(call));
        let entry = by_action.entry(key).or_default();
        entry.0 += 1;
        entry.1 += board_imps;
        if let Some((seat, _)) = actor {
            let shape = by_shape.entry(shape_label(deals[i][seat])).or_default();
            shape.0 += 1;
            shape.1 += board_imps;
        }
    }
    worst.sort_by_key(|w| w.0);
    eprintln!("=== Worst 15 divergent boards for Landy ===");
    for &(imp, i, pos) in worst.iter().take(15) {
        let dealer = Seat::ALL[i % 4];
        eprintln!(
            "[{imp:+} IMP] dealer {dealer:?}  {}\n  A (Landy NS): {} -> {:?}\n  B (Landy EW): {} -> {:?}",
            deals[i], auctions[i].0, contracts[i].0, auctions[i].1, contracts[i].1,
        );
        // Counterfactual for a both-majors X board: would naming the *longer major*
        // have beaten what the X-then-advance sequence reached?  Print the doubler's
        // major lengths and the DD tricks for the longer major, by the doubler and by
        // partner — compare against the contract printed above.
        let actor = natural_action_over_1nt(&auctions[i].0, dealer, true)
            .or_else(|| natural_action_over_1nt(&auctions[i].1, dealer, false));
        if let Some((seat, Call::Double)) = actor {
            let hand = deals[i][seat];
            let (h, s) = (hand[Suit::Hearts].len(), hand[Suit::Spades].len());
            let (m, lab) = if h >= s {
                (Strain::Hearts, "♥")
            } else {
                (Strain::Spades, "♠")
            };
            let partner = Seat::ALL[(seat as usize + 2) % 4];
            let table = &tables[pos];
            eprintln!(
                "    doubler {seat:?} {h}♥-{s}♠ → longer {lab}: DD {} by {seat:?} / {} by {partner:?}",
                u8::from(table[m].get(seat)),
                u8::from(table[m].get(partner)),
            );
        }
    }

    let ew_label = if ew_always_pass {
        "always-pass".to_string()
    } else if ew_natural {
        "on".to_string()
    } else {
        "off".to_string()
    };
    let pp_label = |pp: Option<(usize, u8, bool)>| match pp {
        None => "off".to_string(),
        Some((len, hcp, major)) => format!("{len}:{hcp}{}", if major { ":major" } else { "" }),
    };
    let arms = format!(
        "2♣ majors {}, 2NT minors {} [{}], natural NS {}/EW {}, X-shape {}, landy-X {}@{}+, passed-def {}, pen-pass NS {}/EW {}, pen-latch {}",
        label(majors),
        label(minors),
        args.strength,
        if ns_natural { "on" } else { "off" },
        ew_label,
        args.ns_double_shape,
        args.ns_landy_x,
        args.ns_landy_x_floor,
        match passed_style {
            None => "off",
            Some(PassedHandDefense::NaturalLandyDouble) => "landy",
            Some(PassedHandDefense::Dont) => "dont",
        },
        pp_label(ns_penalty_pass),
        pp_label(ew_penalty_pass),
        if ns_penalty_latch { "on" } else { "off" },
    );
    println!(
        "=== Landy-vs-default A/B ({arms}): {} boards, vulnerability {}, scoring {} ===",
        args.count, args.vulnerability, args.score,
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
    println!("--- IMPs won per natural defensive action ---");
    for (action, (boards, imps_won)) in &by_action {
        println!(
            "  {action:<7} {boards:>6} boards  {imps_won:+7} IMPs  ({:+.3} IMPs/action-board)",
            *imps_won as f64 / (*boards).max(1) as f64,
        );
    }
    println!("--- IMPs won per doubler shape (per-subtype marginal gain) ---");
    for (shape, (boards, imps_won)) in &by_shape {
        println!(
            "  {shape:<7} {boards:>6} boards  {imps_won:+7} IMPs  ({:+.3} IMPs/shape-board, {:+.4} IMPs/raw-deal)",
            *imps_won as f64 / (*boards).max(1) as f64,
            *imps_won as f64 / scanned.max(1) as f64,
        );
    }
    // The filter-independent real-world rate (per *raw* deal dealt): the headline
    // effect size, unlike IMPs/filtered-board, does not move with the filter's tightness.
    println!(
        "Per raw deal: {:+.4} IMPs/board ({total_imps:+} IMPs over {scanned} dealt)",
        total_imps as f64 / scanned.max(1) as f64,
    );
}
