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
//! Lebensohl variants only differ over a `2♦/2♥/2♠` overcall; over `2♣` every
//! variant plays *systems on* (Stayman / transfers as if uncontested), so those
//! boards are not a Lebensohl response — they are kept out of the Lebensohl
//! headline and reported in a separate `systems on` row.
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
use contract_bridge::eval::{hcp as holding_hcp, nltc};
use contract_bridge::{AbsoluteVulnerability, Bid, FullDeal, Hand, Seat, Strain, Suit};
use ddss::{NonEmptyStrainFlags, Solver, TrickCountTable};
use pons::american;
use pons::bidding::american::{
    DoubleStyle, LebensohlStyle, set_direct_3nt_stopper, set_double_override, set_double_style,
    set_lebensohl_style, set_natural_floor, set_trap_pass,
};
use pons::bidding::constraint::point_count;
use pons::bidding::context::{Context, relative};
use pons::bidding::ev::ev_all;
use pons::bidding::{Family, Stance, System};
use pons::scoring::{final_contract, imps, ns_score_contract};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::collections::HashMap;

/// Contested Lebensohl A/B
#[derive(Parser)]
struct Args {
    /// Number of boards in the match (dealer rotates per board)
    #[arg(short, long, default_value = "50000")]
    count: usize,

    /// Vulnerability: none, ns, ew, both
    #[arg(short, long, default_value = "none")]
    vulnerability: AbsoluteVulnerability,

    /// Lebensohl style, measured (NS) pair: off/plain/transfer
    #[arg(long, default_value = "transfer")]
    ns: String,

    /// Lebensohl style, baseline (EW) pair: off/plain/transfer
    #[arg(long, default_value = "plain")]
    ew: String,

    /// Responder's double meaning, measured (NS) pair. A named style
    /// (`penalty`/`penalty-light`/`takeout`/`optional`) or a parametric spec
    /// `LEN:HCP` tuning the threshold directly: `LEN` is `LO-HI` / `LO+` / `LO`
    /// (their-suit length), `HCP` the floor — e.g. `4+:9` (= penalty), `0-3:8`
    /// (= takeout, the default), `4+:8`, `2-3:8`.
    #[arg(long, default_value = "takeout")]
    ns_dbl: String,

    /// Responder's double meaning, baseline (EW) pair (see `--ns-dbl`).
    #[arg(long, default_value = "takeout")]
    ew_dbl: String,

    /// Does the measured (NS) pair's direct `3NT` over the overcall need its own
    /// stopper (`on`, the default) or may it be bid on game values alone, trusting
    /// opener's 1NT (`off`)?
    #[arg(long, default_value = "on")]
    ns_3nt_stopper: String,

    /// 3NT stopper requirement for the baseline (EW) pair (see `--ns-3nt-stopper`).
    #[arg(long, default_value = "on")]
    ew_3nt_stopper: String,

    /// Trap pass for the measured (NS) pair: with a too-good stopper (5+ HCP in
    /// their suit) responder passes instead of bidding a direct `3NT` (waiting for
    /// opener to reopen with a takeout double, then converting to penalty); the
    /// threshold is distilled from `--pd-3nt`. `on`/`off`, default `on`.
    #[arg(long, default_value = "on")]
    ns_trap: String,

    /// Trap pass for the baseline (EW) pair (see `--ns-trap`).
    #[arg(long, default_value = "on")]
    ew_trap: String,

    /// Floor the measured (NS) pair's weak natural `2♦/2♥/2♠` escape and let opener
    /// game-raise it: `off`, `Nhcp` (HCP floor) or `Npts` (total-points floor) —
    /// e.g. `6hcp` (the relay mirror), `5hcp`, `6pts`. A/B floor level/metric.
    #[arg(long, default_value = "off")]
    ns_floor: String,

    /// Weak-natural floor for the baseline (EW) pair (see `--ns-floor`).
    #[arg(long, default_value = "off")]
    ew_floor: String,

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

    /// PD-gate the measured (NS) pair's 5-card 2NT relay: at the responder's
    /// `1NT–(2X)` node, when the book would relay (`2NT`), double-dummy compare
    /// relaying vs defending (`Pass`) over sampled layouts and take the higher EV.
    /// "Relay only when our 3-level line out-scores defending their contract."
    /// Slow (one ev_all per relay decision), so pair with a small `--count`.
    #[arg(long, default_value = "false")]
    pd_relay: bool,

    /// PD-gate the measured (NS) pair's *weak natural* `2♦/2♥/2♠` escape the same
    /// way as `--pd-relay`: when the book bids a natural 2-level suit over the
    /// overcall, double-dummy compare bidding vs defending (`Pass`) and take the
    /// higher EV. With `--log-relay`, emits a `NATURAL` line per decision — the
    /// data for distilling a strength floor on the weak natural (it currently has
    /// none: `points(..=8)` caps the top, the bottom is open).
    #[arg(long, default_value = "false")]
    pd_natural: bool,

    /// PD-gate the measured (NS) pair's direct `3NT` over the overcall: when the
    /// book bids `3NT`, double-dummy compare it against trapping (`Pass`) and take
    /// the higher EV — the per-board oracle for "trap or bid game". With
    /// `--log-relay`, emits a `THREENT` line per decision (with `len_over`/
    /// `hcp_over`) — the data for distilling a smarter trap gate than blunt length.
    #[arg(long, default_value = "false")]
    pd_3nt: bool,

    /// Sampled layouts per PD-relay decision (the ev_all budget).
    #[arg(long, default_value = "64")]
    pd_layouts: usize,

    /// Log each PD-relay decision to stderr (over-suit defensive features + the
    /// relay/defend verdict + EVs) — the data for distilling a static heuristic.
    #[arg(long, default_value = "false")]
    log_relay: bool,

    /// Per-call divergence diff: bucket every divergent board by the measured
    /// (`--ns`) pair's *first* call the baseline (`--ew`) would not have made,
    /// and report IMPs per bucket. Each call is tagged `resp` (responder's action
    /// directly over our `1NT–(2X)`: the penalty double, a transfer, the relay,
    /// direct `3NT`) or `late` (a later call, e.g. opener completing a transfer).
    /// Answers "which call drives the swing" — e.g. is it the penalty double, or
    /// the transfers / 3NT? Each board lands in exactly one bucket, so the
    /// `contrib` column sums to the headline IMPs/board.
    #[arg(long, default_value = "false")]
    diverge_diff: bool,

    /// Dump every divergent board whose `--diverge-diff` bucket matches this
    /// label (e.g. `"resp 3NT"`) — the full deal, both auctions/contracts, the
    /// board swing, and the double-dummy "makes" grid for each side, so the
    /// losing/winning hands in one bucket can be inspected and the best action
    /// read off. Requires `--diverge-diff`. Empty = off.
    #[arg(long, default_value = "")]
    show_bucket: String,
}

/// The opponents' 2-level overcall suit when the auction is at the responder's
/// Lebensohl node (the last two calls are our `1NT` then a 2-level suit overcall)
fn relay_node_over(auction: &[Call]) -> Option<Suit> {
    let n = auction.len();
    if n < 2 || auction[n - 2] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .find(|&s| auction[n - 1] == Call::Bid(Bid::new(2, Strain::from(s))))
}

/// Did an opponent overcall our `1NT` with `2♣`? (systems-on, not Lebensohl)
fn overcalled_2c(auction: &Auction) -> bool {
    let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
    let two_c = Call::Bid(Bid::new(2, Strain::Clubs));
    (1..auction.len()).any(|i| auction[i] == two_c && auction[i - 1] == one_nt)
}

/// The NS pair's call, PD-gating a weak escape when `--pd-relay`/`--pd-natural` is set
///
/// Identical to [`next_call`] except: when the book's choice at the responder
/// node is a *weak escape* we are gating — the `2NT` relay (`--pd-relay`) or a
/// natural `2♦/2♥/2♠` (`--pd-natural`) — sample layouts and double-dummy compare
/// the book call against defending (`Pass`), taking the higher-EV call. This is
/// the "compete only when it beats defending" judgment a static strength gate
/// cannot make (it needs the full deal + vulnerability); `--log-relay` records
/// each decision for distilling that gate into a static floor.
#[allow(clippy::too_many_arguments)]
fn next_call_ns(
    stance: &Stance,
    hand: Hand,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    auction: &Auction,
    pd_relay: bool,
    pd_natural: bool,
    pd_3nt: bool,
    pd_layouts: usize,
    log_relay: bool,
    seed: u64,
) -> Call {
    let base = next_call(stance, hand, dealer, vul, auction);
    let Some(over) = relay_node_over(auction) else {
        return base;
    };
    let two_nt = Call::Bid(Bid::new(2, Strain::Notrump));
    let three_nt = Call::Bid(Bid::new(3, Strain::Notrump));
    let is_relay = pd_relay && base == two_nt;
    // At this node the only 2-level suit bids are the weak naturals.
    let is_natural = pd_natural
        && matches!(base, Call::Bid(b)
            if b.level.get() == 2 && b.strain.suit().is_some_and(|s| s != over));
    // Direct 3NT vs trapping (pass): the smarter trap gate — let the double-dummy
    // rollout decide per board, then distill which holdings should trap.
    let is_3nt = pd_3nt && base == three_nt;
    if !is_relay && !is_natural && !is_3nt {
        return base;
    }

    let seat = seat_to_act(dealer, auction.len());
    let context = Context::new(relative(vul, seat), auction);
    let calls = [base, Call::Pass];
    let mut rng = StdRng::seed_from_u64(seed);
    let evs = ev_all(
        hand, seat, vul, &context, &calls, stance, &mut rng, pd_layouts,
    );
    // Defend only when both lines scored and defending strictly wins; otherwise
    // keep the book call (NaN = no signal → trust the book).
    let defend = evs[0].is_finite() && evs[1].is_finite() && evs[1] > evs[0];

    if log_relay {
        let bid_suit = match base {
            Call::Bid(b) => b.strain.suit(),
            _ => None,
        };
        let kind = if is_relay {
            "RELAY"
        } else if is_natural {
            "NATURAL"
        } else {
            "THREENT"
        };
        // For the 3NT trap distill, the holding *in their suit* is the signal:
        // length plus honor strength (a running AKQxx wants 3NT; a flat stack
        // traps). Log `len_over`/`hcp_over` alongside the usual features.
        eprintln!(
            "{kind} {} over={over:?} bid={base} len_bid={} len_over={} hcp_over={} hcp_total={} pts={} nltc={:.1} ev_bid={:.0} ev_defend={:.0}",
            if defend { "DEFEND" } else { "bid" },
            bid_suit.map_or(0, |s| hand[s].len()),
            hand[over].len(),
            holding_hcp::<u8>(hand[over]),
            hand_hcp(hand),
            point_count(hand),
            Suit::ASC.iter().map(|&s| nltc(hand[s])).sum::<f64>(),
            evs[0],
            evs[1],
        );
    }
    if defend { Call::Pass } else { base }
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

/// The double-dummy "makes" grid for `side` (its two seats): the most tricks
/// either seat takes as declarer in each strain — i.e. what that side can make.
fn dd_makes(table: &TrickCountTable, side: [Seat; 2]) -> String {
    [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
        Strain::Notrump,
    ]
    .into_iter()
    .map(|strain| {
        let tricks = side
            .iter()
            .map(|&seat| u8::from(table[strain].get(seat)))
            .max()
            .unwrap_or(0);
        let sym = match strain {
            Strain::Clubs => "♣",
            Strain::Diamonds => "♦",
            Strain::Hearts => "♥",
            Strain::Spades => "♠",
            Strain::Notrump => "NT",
        };
        format!("{sym}{tricks}")
    })
    .collect::<Vec<_>>()
    .join(" ")
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

/// Parse a weak-natural floor spec into `(hcp_floor, points_floor)` for
/// [`set_natural_floor`]: `off`→`(0,0)`, `Nhcp`→`(N,0)`, `Npts`→`(0,N)`.
fn floor_from(spec: &str) -> (u8, u8) {
    if spec == "off" {
        return (0, 0);
    }
    let (num, kind) = spec.split_at(spec.len().saturating_sub(3));
    match (num.parse::<u8>(), kind) {
        (Ok(n), "hcp") => (n, 0),
        (Ok(n), "pts") => (0, n),
        _ => panic!("bad floor spec {spec:?} (use off / Nhcp / Npts, e.g. 6hcp or 6pts)"),
    }
}

/// Parse a Lebensohl style name (off / plain / transfer)
fn style_from(name: &str) -> LebensohlStyle {
    match name {
        "off" => LebensohlStyle::Off,
        "plain" => LebensohlStyle::Plain,
        "transfer" => LebensohlStyle::Transfer,
        other => {
            panic!(
                "unknown lebensohl style {other:?} \
                 (use off / plain / transfer)"
            )
        }
    }
}

/// Parse a double-meaning name (penalty / penalty-light / takeout / optional)
fn dbl_from(name: &str) -> DoubleStyle {
    match name {
        "penalty" => DoubleStyle::Penalty,
        "penalty-light" => DoubleStyle::PenaltyLight,
        "takeout" => DoubleStyle::Takeout,
        "optional" => DoubleStyle::Optional,
        other => {
            panic!(
                "unknown double style {other:?} \
                 (use penalty / penalty-light / takeout / optional)"
            )
        }
    }
}

/// Select responder's double for books built *after* this call: a named style
/// (clears any override) or a parametric `LEN:HCP` spec (sets the override). `LEN`
/// is `LO-HI`, `LO+` (open top, capped at 13), or `LO` (exact); `HCP` is the floor.
fn apply_double(spec: &str) {
    if matches!(spec, "penalty" | "penalty-light" | "takeout" | "optional") {
        set_double_override(None);
        set_double_style(dbl_from(spec));
        return;
    }
    let (len_part, hcp_part) = spec
        .split_once(':')
        .unwrap_or_else(|| panic!("bad double spec {spec:?} (use a style or LEN:HCP, e.g. 4+:9)"));
    let floor: u8 = hcp_part
        .parse()
        .unwrap_or_else(|_| panic!("bad HCP floor in {spec:?}"));
    let (lo, hi) = if let Some((a, b)) = len_part.split_once('-') {
        (parse_len(a, spec), parse_len(b, spec))
    } else if let Some(a) = len_part.strip_suffix('+') {
        (parse_len(a, spec), 13)
    } else {
        let n = parse_len(len_part, spec);
        (n, n)
    };
    set_double_override(Some((lo, hi, floor)));
}

/// Parse one suit-length component of a `--ns-dbl` spec
fn parse_len(s: &str, spec: &str) -> usize {
    s.parse()
        .unwrap_or_else(|_| panic!("bad length in double spec {spec:?}"))
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

/// Rebuild an [`Auction`] from a prefix of its calls (for replaying counterfactuals)
fn prefix_auction(calls: &[Call]) -> Auction {
    let mut auction = Auction::new();
    for &call in calls {
        auction.push(call);
    }
    auction
}

/// The measured pair's first call the baseline stance would not have made
///
/// Replays `auction` (the measured/Lebensohl pair sits NS when `is_ns`); at each
/// of that pair's turns it compares the actual call to what `baseline` would
/// choose for the same hand and prefix. Returns the call index, the diverging
/// call, and whether it is the responder's action directly over our `1NT–(2X)`
/// (so the diff can separate the responder node from later, e.g. opener
/// completing a transfer). `None` if the pair never diverged at this table.
fn first_divergent(
    auction: &Auction,
    baseline: &Stance,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    is_ns: bool,
) -> Option<(usize, Call, bool)> {
    (0..auction.len()).find_map(|i| {
        let seat = seat_to_act(dealer, i);
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        if seat_is_ns != is_ns {
            return None; // a baseline seat at this table — not a measured call
        }
        let counterfactual = next_call(
            baseline,
            deal[seat],
            dealer,
            vul,
            &prefix_auction(&auction[..i]),
        );
        (counterfactual != auction[i])
            .then(|| (i, auction[i], relay_node_over(&auction[..i]).is_some()))
    })
}

/// Bid one deal with the Lebensohl pair on the side picked by `lebensohl_is_ns`
#[allow(clippy::too_many_arguments)]
fn bid_out(
    lebensohl: &Stance,
    baseline: &Stance,
    lebensohl_is_ns: bool,
    dealer: Seat,
    vul: AbsoluteVulnerability,
    deal: &FullDeal,
    pd_relay: bool,
    pd_natural: bool,
    pd_3nt: bool,
    pd_layouts: usize,
    log_relay: bool,
    seed: u64,
) -> Auction {
    let mut auction = Auction::new();
    while !auction.has_ended() {
        let seat = seat_to_act(dealer, auction.len());
        let seat_is_ns = matches!(seat, Seat::North | Seat::South);
        let is_lebensohl = seat_is_ns == lebensohl_is_ns;
        let call = if is_lebensohl {
            next_call_ns(
                lebensohl, deal[seat], dealer, vul, &auction, pd_relay, pd_natural, pd_3nt,
                pd_layouts, log_relay, seed,
            )
        } else {
            next_call(baseline, deal[seat], dealer, vul, &auction)
        };
        auction.push(call);
    }
    auction
}

#[allow(clippy::cast_precision_loss)]
fn main() {
    let args = Args::parse();
    let mut rng = StdRng::seed_from_u64(args.seed);

    set_lebensohl_style(style_from(&args.ew));
    apply_double(&args.ew_dbl);
    set_direct_3nt_stopper(args.ew_3nt_stopper != "off");
    set_trap_pass(args.ew_trap == "on");
    let (ew_h, ew_p) = floor_from(&args.ew_floor);
    set_natural_floor(ew_h, ew_p);
    let baseline = american().against(Family::NATURAL);
    set_lebensohl_style(style_from(&args.ns));
    apply_double(&args.ns_dbl);
    set_direct_3nt_stopper(args.ns_3nt_stopper != "off");
    set_trap_pass(args.ns_trap == "on");
    let (ns_h, ns_p) = floor_from(&args.ns_floor);
    set_natural_floor(ns_h, ns_p);
    let lebensohl = american().against(Family::NATURAL);

    // Phase 1 (sequential, cheap): deal + the shape-only filter until `count`
    // boards pass. The RNG stays single-threaded so a seed reproduces a run.
    let mut passing: Vec<FullDeal> = Vec::with_capacity(args.count);
    let mut scanned = 0usize;
    while passing.len() < args.count {
        let deal = full_deal(&mut rng);
        scanned += 1;
        if args.filter_dh && !could_reach_1nt_dh(&deal) {
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
    let pd_relay = args.pd_relay;
    let pd_natural = args.pd_natural;
    let pd_3nt = args.pd_3nt;
    let pd_layouts = args.pd_layouts;
    let log_relay = args.log_relay;
    let seed = args.seed;
    let results: Vec<_> = passing
        .par_iter()
        .enumerate()
        .map(|(i, &deal)| {
            let dealer = Seat::ALL[i % 4];
            let table_a = bid_out(
                &lebensohl, &baseline, true, dealer, vul, &deal, pd_relay, pd_natural, pd_3nt,
                pd_layouts, log_relay, seed,
            );
            let table_b = bid_out(
                &lebensohl, &baseline, false, dealer, vul, &deal, pd_relay, pd_natural, pd_3nt,
                pd_layouts, log_relay, seed,
            );
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
    // 2♣ overcall: systems-on for every variant, not Lebensohl — counted apart.
    let mut systems_on = (0usize, 0i64);
    let mut worst: Vec<(i64, usize)> = Vec::new();
    let mut buckets: HashMap<String, (usize, i64)> = HashMap::new();
    // Boards to dump for `--show-bucket`: (position into `tables`, board index,
    // board IMPs, the divergent trigger).
    let mut show: Vec<(usize, usize, i64, (usize, Call, bool))> = Vec::new();
    for (pos, (&i, table)) in divergent.iter().zip(tables.iter()).enumerate() {
        let (contract_a, contract_b) = contracts[i];
        let swing = ns_score_contract(contract_a, table, args.vulnerability)
            - ns_score_contract(contract_b, table, args.vulnerability);
        let board_imps = imps(swing);

        if overcalled_2c(&auctions[i].0) || overcalled_2c(&auctions[i].1) {
            // Over (2♣) every Lebensohl variant plays *systems on* (Stayman /
            // transfers as if uncontested), not Lebensohl: keep it out of the
            // Lebensohl headline and collapse it into one `systems on` row.
            systems_on.0 += 1;
            systems_on.1 += board_imps;
            continue;
        }

        points += swing;
        total_imps += board_imps;
        worst.push((board_imps, i));

        if args.diverge_diff {
            // Credit the board to the measured pair's earliest divergent call,
            // looking at both tables (the Lebensohl node usually fires at only
            // one orientation, so this is rarely ambiguous).
            let dealer = Seat::ALL[i % 4];
            let trig_a = first_divergent(
                &auctions[i].0,
                &baseline,
                dealer,
                args.vulnerability,
                &deals[i],
                true,
            );
            let trig_b = first_divergent(
                &auctions[i].1,
                &baseline,
                dealer,
                args.vulnerability,
                &deals[i],
                false,
            );
            let trigger = match (trig_a, trig_b) {
                (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
                (a, b) => a.or(b),
            };
            let label = trigger.map_or_else(
                || "(none)".to_string(),
                |(_, call, at_node)| format!("{} {call}", if at_node { "resp" } else { "late" }),
            );
            if !args.show_bucket.is_empty() && label == args.show_bucket {
                if let Some(trig) = trigger {
                    show.push((pos, i, board_imps, trig));
                }
            }
            let entry = buckets.entry(label).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += board_imps;
        }
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

    if !args.show_bucket.is_empty() {
        println!(
            "=== bucket {:?}: {} board(s), vul {} (DD makes = most tricks either seat takes as declarer) ===",
            args.show_bucket,
            show.len(),
            args.vulnerability,
        );
        for &(pos, i, board_imps, (tidx, tcall, _)) in &show {
            let dealer = Seat::ALL[i % 4];
            let responder = seat_to_act(dealer, tidx);
            println!(
                "[{board_imps:+} IMP] dealer {dealer:?}  responder {responder:?} bid {tcall}  {}",
                deals[i],
            );
            println!(
                "  A ({} NS): {} -> {:?}",
                args.ns, auctions[i].0, contracts[i].0
            );
            println!(
                "  B ({} NS): {} -> {:?}",
                args.ew, auctions[i].1, contracts[i].1
            );
            println!(
                "  NS makes (DD): {}",
                dd_makes(&tables[pos], [Seat::North, Seat::South])
            );
            println!(
                "  EW makes (DD): {}",
                dd_makes(&tables[pos], [Seat::East, Seat::West])
            );
        }
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
    let leb_divergent = divergent.len() - systems_on.0;
    println!(
        "Divergent boards: {} of {} ({:.1}%); systems-on (1NT–2♣) {} excluded",
        leb_divergent,
        args.count,
        100.0 * leb_divergent as f64 / args.count.max(1) as f64,
        systems_on.0,
    );
    println!(
        "NS {} (vs EW {}): {points:+} points, {total_imps:+} IMPs ({:+.3} IMPs/board, {:+.3} IMPs/divergent)",
        args.ns,
        args.ew,
        total_imps as f64 / args.count.max(1) as f64,
        total_imps as f64 / leb_divergent.max(1) as f64,
    );
    println!(
        "systems on (1NT–2♣, not Lebensohl): {} boards, {:+} IMPs ({:+.3}/board) — excluded above",
        systems_on.0,
        systems_on.1,
        systems_on.1 as f64 / systems_on.0.max(1) as f64,
    );

    if args.diverge_diff {
        // Sort worst-first (most negative total) so the calls dragging the
        // --ns style down sit at the top. `contrib` = bucket IMPs / all boards,
        // so the Lebensohl rows sum to the headline IMPs/board above (the
        // trailing `systems on` row is over 2♣ and is *not* part of that sum).
        let mut rows: Vec<(String, (usize, i64))> = buckets.into_iter().collect();
        rows.sort_by_key(|(_, (_, imp))| *imp);
        println!(
            "=== Divergence diff: NS {} first divergent call vs EW {} ===",
            args.ns, args.ew,
        );
        println!(
            "{:<11} {:>7} {:>8} {:>9} {:>9}",
            "call", "boards", "IMPs", "per-bd", "contrib",
        );
        for (label, (n, imp)) in &rows {
            println!(
                "{label:<11} {n:>7} {imp:>+8} {:>+9.3} {:>+9.4}",
                *imp as f64 / *n as f64,
                *imp as f64 / args.count.max(1) as f64,
            );
        }
        println!(
            "{:<11} {:>7} {:>+8} {:>+9.3} {:>9}",
            "systems on",
            systems_on.0,
            systems_on.1,
            systems_on.1 as f64 / systems_on.0.max(1) as f64,
            "—",
        );
    }
}
