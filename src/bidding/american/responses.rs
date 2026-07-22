//! Responses to one-level suit openings in the 2/1 game-forcing system

use super::super::Alert;
use super::super::Rules;
use super::super::Trie;
use super::super::constraint::{
    Cons, Constraint, balanced, described, hcp, len, points, stopper_in, support, support_points,
};
use super::notrump::flat_4333;
use crate::bidding::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether minor-opening responses pick the **longer major** (equal
    /// lengths: 4-4 up the line to `1♥`, 5-5+ higher-first to `1♠`) instead of
    /// unconditional hearts-first.  Default `true` — the established American
    /// treatment (bid the longer major on 5♠4♥), which measured a null
    /// (`ab-minor-continuations`, 2M boards: plain-DD wash, PD −0.12/−0.22 per
    /// divergent NV/vul; −0.003..−0.005 IMPs/board marginal on the shipped xyz
    /// + up-the-line package).  A push against a natural default goes to the
    /// natural method — the **naturalness tiebreak** (`docs/measurement.md`);
    /// the historic unconditional-hearts-first simplification is the opt-in
    /// (turn this knob *off*).
    static LONGER_MAJOR_RESPONSE: Cell<bool> = const { Cell::new(true) };
    /// Whether the natural minor-opening tree is completed **up the line**:
    /// the `1♣ – 1♦` response, opener's `1♠` rebid over `1m – 1♥`, and
    /// opener's natural `2♣` rebid after `1♣ – 1♦`.  Default `true`, shipped
    /// **jointly with XYZ** (`ab-minor-continuations`, 300k boards, with
    /// `set_xyz`: plain +0.0382/+0.0559 IMPs/board NV/vul, PD
    /// +0.0289/+0.0407).  Alone it is a measured **loss** (plain
    /// −0.91/−1.28 per divergent) — the 1♦ response reroutes hands into
    /// auctions only the XYZ round continues; don't enable it with XYZ off.
    static UP_THE_LINE: Cell<bool> = const { Cell::new(true) };
    /// Whether `1M – 3NT` is authored as a **choice of games**: 3-4 card
    /// support, exactly (4333), 12-15 HCP — responder offers 3NT, opener
    /// passes balanced and corrects to `4M` with shape.  Default `true` —
    /// **shipped default-on 2026-07-15** (isolated: plain +0.0006/+0.0011
    /// NV/vul, PD +0.0005/+0.0010, all CIs clear; perfectly additive atop
    /// the 2/1 fit-split, full-package numbers on that knob).
    static MAJOR_CHOICE_OF_GAMES: Cell<bool> = const { Cell::new(true) };
    /// Whether the major 2/1 game-force entry gains the **fit leg**: with
    /// exactly three-card support the 2/1 is a preparation for `4M`, so the
    /// hand is gauged in `support_points` (the fit is privately known —
    /// opener promised five).  Default `true` — **shipped default-on
    /// 2026-07-15** jointly with the `Hcp13` gate (alone a vul-only plain
    /// win; the pair plain +0.0033/+0.0048, PD +0.0070/+0.0087 NV/vul —
    /// the fit leg re-admits with support what the hcp gate demotes).
    static TWO_OVER_ONE_FIT: Cell<bool> = const { Cell::new(true) };
    /// The gauge for the **no-fit** leg of the major 2/1 game-force entry.
    /// Default [`TwoOverOneGate::Hcp13`] — **shipped 2026-07-15**; the
    /// legacy `points(13..)` is the `Points13` opt-out.
    static TWO_OVER_ONE_GATE: Cell<TwoOverOneGate> = const { Cell::new(TwoOverOneGate::Hcp13) };
    /// Whether the major 2/1 game force names **natural per-call suit lengths**
    /// instead of a uniform four: `1♠–2♥` promises five (a 2/1 into a major is
    /// a real five-card suit), `1♠–2♣` allows three (the cheapest 2/1 is the
    /// catch-all), and the rest keep four.  Default `false` (uniform four,
    /// book byte-identical); A/B pending.
    static TWO_OVER_ONE_NATURAL_LENGTHS: Cell<bool> = const { Cell::new(false) };
    /// Whether `1♠–2♥` (the five-card-major 2/1) forces game a shade light: its
    /// no-fit `Hcp*` floor drops by one — `hcp(12..)` at the default `Hcp13`
    /// gate — serving both 3NT and `4♥`.  Default `false` (book byte-identical);
    /// A/B pending.
    static TWO_OVER_ONE_MAJOR_DISCOUNT: Cell<bool> = const { Cell::new(false) };
}

/// The gauge for the no-fit leg of the major 2/1 game force
/// (`set_two_over_one_gate`)
///
/// The remnant report (docs/point-count-threshold-campaign.md) flagged the
/// 2/1 band both ways under the rule-of-N+8 scale: shaped 11s read 13+ and
/// forced game without a fit.  The shape-indifferent `Hcp13` swept best and
/// shipped; `Hcp12`'s vul plain edge came with a PD loss both vuls in the
/// paired head-to-head (the thin-game doubling signature) — an sd-lead
/// probe candidate, not the default.  `Points12` (Rule of 20) revisits the
/// same floor-lightening question on the `points` scale instead of raw HCP.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TwoOverOneGate {
    /// The legacy gate: `points(13..)` on the global scale
    Points13,
    /// `points(12..)` on the global scale — one point lighter than the
    /// legacy gate; on the shipped rule-of-N+8 scale this is exactly the
    /// Rule of 20 (raw HCP plus the two longest suits, floored at 8)
    Points12,
    /// Raw `hcp(13..)` — shape-indifferent, demotes shaped 11-12s to 1NT;
    /// the shipped default
    #[default]
    Hcp13,
    /// Raw `hcp(12..)` — one lighter, admits every 12-HCP hand
    Hcp12,
    /// Raw `hcp(14..)` — one *stricter* than the shipped default, the
    /// tightening counterpart to `Hcp12`: is 13 itself too light, or does
    /// tightening give back more than it costs?
    Hcp14,
}

impl TwoOverOneGate {
    /// The raw-HCP floor of an `Hcp*` gate (unused by the `Points*` gates,
    /// which are matched separately in [`major_responses`])
    const fn hcp_floor(self) -> u8 {
        match self {
            Self::Points13 | Self::Points12 | Self::Hcp13 => 13,
            Self::Hcp12 => 12,
            Self::Hcp14 => 14,
        }
    }
}

/// Author the longer-major response discipline for books built *after* this
/// call (default `true`; off-switch `--no-ns-longer-major-response` in
/// `bba-gen`)
///
/// On (the default): a response to `1♣`/`1♦` names the longer major — `1♠` on
/// 5♠4♥ or any 5-5+, `1♥` up the line only on 4-4 — so partner can infer
/// "spades are not longer than hearts" from `1♥`.  The M6.4 control-bid
/// classifier reads the same discipline at classify time (`classify_high_bid`
/// in `inference.rs`): the response rule, the rebid structure, and the
/// classifier move together (see `docs/bidding-theorems.md`).  Off: the
/// historic unconditional hearts-first pair — measured a null against
/// longer-major, so the naturalness tiebreak (`docs/measurement.md`) keeps the
/// established American treatment as the default.
pub fn set_longer_major_response(on: bool) {
    LONGER_MAJOR_RESPONSE.with(|cell| cell.set(on));
}

/// Whether the longer-major response discipline is active (also read by the
/// inference engine at classify time)
pub(crate) fn longer_major_response() -> bool {
    LONGER_MAJOR_RESPONSE.with(Cell::get)
}

/// Author the up-the-line completion of the natural minor tree for books
/// built *after* this call (default `true`; off-switch `--no-ns-up-the-line`
/// in `bba-gen`)
///
/// On: responder bids `1♦` over `1♣` on four-plus diamonds without a
/// four-card major (off, those hands squeeze into the notrump ladder or fall
/// to the floor), opener rebids `1♠` over `1m – 1♥` on four spades (off, the
/// 4-4 spade fit is lost to a 1NT rebid), and opener rebids a natural `2♣`
/// after `1♣ – 1♦` on six-plus clubs (off, a misdescribed 1NT catch-all).
///
/// Shipped **jointly with [`set_xyz`][super::set_xyz]**: the 1♦ response only
/// pays once responder's second round has the XYZ machinery (alone it
/// measured plain −0.91/−1.28 per divergent).
pub fn set_up_the_line(on: bool) {
    UP_THE_LINE.with(|cell| cell.set(on));
}

/// Whether the up-the-line completion is currently authored
pub(crate) fn up_the_line() -> bool {
    UP_THE_LINE.with(Cell::get)
}

/// Author the `1M – 3NT` choice-of-games response for books built after this
/// call (default `true`; off-switch `--no-ns-major-choice-of-games` in
/// `bba-gen`)
///
/// On: `3NT` over `1♥`/`1♠` shows 3-4 card support, exactly (4333) and 12-15
/// HCP (over `1♥` it also denies four spades — that hand bids `1♠` first).
/// Opener passes with a balanced hand and corrects to `4M` with shape; the
/// alerted reading pins responder's three-card support so later floor
/// decisions know the fit.  Off: the flat hand routes through its lone
/// four-card suit as a 2/1 (or Jacoby 2NT / limit raise with four trumps).
pub fn set_major_choice_of_games(on: bool) {
    MAJOR_CHOICE_OF_GAMES.with(|cell| cell.set(on));
}

/// Whether the choice-of-games 3NT is currently authored
fn major_choice_of_games() -> bool {
    MAJOR_CHOICE_OF_GAMES.with(Cell::get)
}

/// Author the fit leg of the major 2/1 game force for books built after this
/// call (default `true`; off-switch `--no-ns-two-over-one-fit` in `bba-gen`)
///
/// On: a hand with exactly three-card support and a biddable side suit enters
/// the 2/1 on `support_points(13..)` — the 2/1 is a preparation for `4M`, and
/// the fit is privately known (opener promised five), so shortness counts.
/// Off: every 2/1 is gauged by the no-fit gate alone.
pub fn set_two_over_one_fit(on: bool) {
    TWO_OVER_ONE_FIT.with(|cell| cell.set(on));
}

/// Whether the 2/1 fit leg is currently authored
fn two_over_one_fit() -> bool {
    TWO_OVER_ONE_FIT.with(Cell::get)
}

/// Set the no-fit gauge of the major 2/1 game force for books built after
/// this call (default [`TwoOverOneGate::Hcp13`];
/// `--ns-two-over-one-gate` in `bba-gen`)
pub fn set_two_over_one_gate(gate: TwoOverOneGate) {
    TWO_OVER_ONE_GATE.with(|cell| cell.set(gate));
}

/// The currently authored no-fit 2/1 gauge
fn two_over_one_gate() -> TwoOverOneGate {
    TWO_OVER_ONE_GATE.with(Cell::get)
}

/// Author natural per-call suit lengths for the major 2/1 game force for books
/// built after this call (default `false`;
/// `--ns-two-over-one-natural-lengths` in `bba-gen`)
///
/// On: `1♠–2♥` promises 5+ hearts and `1♠–2♣` allows 3+ clubs (the cheapest
/// 2/1 is the catch-all); every other 2/1 keeps its 4+ floor.  Off: a uniform
/// 4+ in every 2/1 suit.
pub fn set_two_over_one_natural_lengths(on: bool) {
    TWO_OVER_ONE_NATURAL_LENGTHS.with(|cell| cell.set(on));
}

/// Whether natural per-call 2/1 suit lengths are currently authored
fn two_over_one_natural_lengths() -> bool {
    TWO_OVER_ONE_NATURAL_LENGTHS.with(Cell::get)
}

/// Lighten the `1♠–2♥` game force by one HCP for books built after this call
/// (default `false`; `--ns-two-over-one-major-discount` in `bba-gen`)
///
/// On: the no-fit leg of `1♠–2♥` drops its `Hcp*` floor by one — `hcp(12..)`
/// at the default `Hcp13` gate — because the five-card major is worth a game
/// force a shade light.  Off: the full gate floor.  No effect on the `Points*`
/// gates or on any other 2/1.
pub fn set_two_over_one_major_discount(on: bool) {
    TWO_OVER_ONE_MAJOR_DISCOUNT.with(|cell| cell.set(on));
}

/// Whether the `1♠–2♥` HCP discount is currently authored
fn two_over_one_major_discount() -> bool {
    TWO_OVER_ONE_MAJOR_DISCOUNT.with(Cell::get)
}

/// Spades take the first response: strictly longer, or equal length five-plus
///
/// The longer-major discipline's selector — 5-5 responds `1♠` planning to
/// show hearts next; 4-4 responds `1♥` up the line.
fn spades_first() -> Cons<impl Constraint + Clone> {
    described(
        "spades longer than hearts, or equal five-plus",
        |hand: Hand, _: &Context<'_>| {
            spades_take_first(hand[Suit::Spades].len(), hand[Suit::Hearts].len())
        },
    )
}

/// The [`spades_first`] predicate on the two major lengths alone
fn spades_take_first(spades: usize, hearts: usize) -> bool {
    spades > hearts || (spades == hearts && spades >= 5)
}

/// Hearts take the first response: strictly longer, or equal length below five
///
/// The exact complement of [`spades_first`] — the 1♥ response fires precisely
/// when spades do not. Phrased positively so the book renders "hearts longer
/// than spades, or equal below five" rather than a negated `spades_first`.
fn hearts_first() -> Cons<impl Constraint + Clone> {
    described(
        "hearts longer than spades, or equal below five",
        |hand: Hand, _: &Context<'_>| {
            hearts_take_first(hand[Suit::Spades].len(), hand[Suit::Hearts].len())
        },
    )
}

/// The [`hearts_first`] predicate on the two major lengths alone
fn hearts_take_first(spades: usize, hearts: usize) -> bool {
    hearts > spades || (hearts == spades && spades < 5)
}

/// Jacoby 2NT — the game-forcing major raise with four-card support
const JACOBY_2NT: Alert = Alert("jacoby-2nt");
/// Splinter — a double jump in a new suit showing a singleton or void
const SPLINTER: Alert = Alert("splinter");
/// Weak jump shift — a single jump showing a weak six-card suit
const WEAK_JUMP_SHIFT: Alert = Alert("weak-jump-shift");
/// Inverted minor raise — forcing `2m`, preemptive `3m`
const INVERTED_MINOR: Alert = Alert("inverted-minor");
/// 2/1 game force — a new suit at the two level, game forcing
const GAME_FORCE: Alert = Alert("game-force");
/// Choice of games — `1M – 3NT` with 3-4 card support, (4333), 12-15 HCP
const CHOICE_OF_GAMES: Alert = Alert("choice-of-games-3nt");

/// Responses to our `1♥`/`1♠` opening
///
/// The 2/1 core: a new suit at the two level is game forcing
/// (`hcp(13..)`), the forcing 1NT is the catch-all below it, raises are
/// graded by strength (single / limit / Jacoby 2NT / weak jump to game), and
/// over 1♥ a four-card spade suit takes the one level.  Splinters (double jump
/// in a new suit) and weak jump shifts round out the response set.
#[must_use]
pub fn major_responses(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Jacoby 2NT: game-forcing raise with four-card support.
        .rule(
            Bid::new(2, Strain::Notrump),
            3.0,
            support(4..) & support_points(13..),
        )
        .alert(JACOBY_2NT)
        // Limit raise: four-card support, 10–12 points.
        .rule(
            Bid::new(3, trump),
            2.0,
            support(4..) & support_points(10..=12),
        )
        // Weak jump to game: lots of trumps, few points.  Left on legacy
        // `points`: this preempt's ceiling gates obstruction, and revaluing
        // shortness here would demote shapely-weak hands into a constructive
        // single raise — a DD-flattering de-preemption (see the roadmap).
        .rule(Bid::new(4, trump), 1.6, support(5..) & points(..6))
        // Single raise.
        .rule(
            Bid::new(2, trump),
            1.5,
            support(3..) & support_points(6..=9),
        )
        // Forcing 1NT: the catch-all when nothing more descriptive fits.
        // Capped one under the no-fit gate's raw-HCP floor, so the table stays
        // total: a `Points*` gate (or `Hcp13`/`Hcp12`) never needs the cap
        // above 12 (`points >= hcp` always, so hcp(13..) already clears every
        // `points` floor and wins the 2/1 rule on weight), but a gate
        // *stricter* than `Hcp13` would otherwise orphan the hands between —
        // caught by neither rule — to the floor instead of a designed 1NT.
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            hcp(6..=(two_over_one_gate().hcp_floor().max(13) - 1)),
        )
        .rule(Call::Pass, 0.0, hcp(..6));

    // 1♠ over 1♥: a new suit at the one level, preferred to a single raise.
    if major == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            1.7,
            len(Suit::Spades, 4..) & points(6..) & !support(4..),
        );
    }

    // Choice-of-games 3NT (`set_major_choice_of_games`): exactly (4333) with
    // 3-4 card support, 12-15 HCP — offer 3NT and let opener choose (the
    // curse of (4333): the flat hand often plays better in notrump).  On 4333
    // `points` reads raw HCP under the floored scale, so the band is HCP.
    // Weight 3.2 outranks Jacoby 2NT (3.0) and the limit raise (2.0) so flat
    // four-trump hands prefer it; over 1♥ the spade exclusion is load-bearing
    // — without it 3.2 would steal 4=3=3=3 from the 1♠ response (1.7).
    if major_choice_of_games() {
        let cog = support(3..=4) & flat_4333() & points(12..=15);
        rules = if major == Suit::Hearts {
            rules.rule(
                Bid::new(3, Strain::Notrump),
                3.2,
                cog & len(Suit::Spades, ..4),
            )
        } else {
            rules.rule(Bid::new(3, Strain::Notrump), 3.2, cog)
        }
        .alert(CHOICE_OF_GAMES);
    }

    // Splinters: double jump in a new suit — four-card support, 10–13 HCP,
    // singleton or void in the splinter suit.
    let splinter_suits: &[Suit] = if major == Suit::Hearts {
        &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
    } else {
        &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
    };

    for &x in splinter_suits {
        let (level, strain) = splinter_bid(major, x);
        rules = rules
            .rule(
                Bid::new(level, strain),
                2.8,
                support(4..) & support_points(10..=13) & len(x, ..=1),
            )
            .alert(SPLINTER);
    }

    // Weak jump shifts: single jump in a new suit — 6-card suit, 2–5 HCP.
    let wjs_suits: &[Suit] = if major == Suit::Hearts {
        &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
    } else {
        &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
    };

    for &x in wjs_suits {
        let (level, strain) = wjs_bid(major, x);
        rules = rules
            .rule(Bid::new(level, strain), 1.0, len(x, 6..) & points(2..=5))
            .alert(WEAK_JUMP_SHIFT);
    }

    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.  The
    // entry gate splits per the knobs: the no-fit gauge is `points` or raw
    // `hcp` (`set_two_over_one_gate`), and the fit leg
    // (`set_two_over_one_fit`) admits exactly-three-card support on
    // `support_points` — fit-known, so shortness counts.  The
    // `(off, Points13)` arm is the shipped expression, byte-identical.
    let mut weight = 1.1;
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            let bid = Bid::new(2, Strain::from(suit));
            // Suit-length floor: a 2/1 into a major promises five (2♥ over 1♠),
            // and the cheapest 2/1 (2♣ over 1♠) is the catch-all and can be
            // three; every other 2/1 stays four.  Hearts only reaches this loop
            // over 1♠ (`Strain < trump` bars it over 1♥), so it needs no guard.
            let min_len = if two_over_one_natural_lengths() {
                match suit {
                    Suit::Hearts => 5,
                    Suit::Clubs if major == Suit::Spades => 3,
                    _ => 4,
                }
            } else {
                4
            };
            // 2♥ over 1♠ (the five-card major) may force game one HCP light.
            let discount = u8::from(two_over_one_major_discount() && suit == Suit::Hearts);
            rules = match (two_over_one_fit(), two_over_one_gate()) {
                (false, TwoOverOneGate::Points13) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..) & points(13..) & !support(4..),
                ),
                (false, TwoOverOneGate::Points12) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..) & points(12..) & !support(4..),
                ),
                (false, gate) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..) & hcp((gate.hcp_floor() - discount)..) & !support(4..),
                ),
                (true, TwoOverOneGate::Points13) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..)
                        & !support(4..)
                        & (points(13..) | (support(3..) & support_points(13..))),
                ),
                (true, TwoOverOneGate::Points12) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..)
                        & !support(4..)
                        & (points(12..) | (support(3..) & support_points(13..))),
                ),
                (true, gate) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..)
                        & !support(4..)
                        & (hcp((gate.hcp_floor() - discount)..)
                            | (support(3..) & support_points(13..))),
                ),
            }
            .alert(GAME_FORCE);
            weight -= 0.05;
        }
    }
    rules
}

/// The splinter bid for major `m` with void/singleton in `x`
///
/// A splinter is the lowest double-jump bid in a new suit.
pub(super) fn splinter_bid(major: Suit, x: Suit) -> (u8, Strain) {
    // 1♥ splinters: 3♠ (one above 2♠), 4♣, 4♦
    // 1♠ splinters: 4♣, 4♦, 4♥
    let major_strain = Strain::from(major);
    let x_strain = Strain::from(x);

    if x_strain > major_strain {
        // Over 1♥, spades is a double jump at 3 level (3♠ skips 2♠)
        (3, x_strain)
    } else {
        // Below the major, level 4
        (4, x_strain)
    }
}

/// The weak jump shift bid for major `m` into suit `x`
///
/// A WJS is a single jump into a new suit below the major.
fn wjs_bid(major: Suit, x: Suit) -> (u8, Strain) {
    let major_strain = Strain::from(major);
    let x_strain = Strain::from(x);

    if x_strain > major_strain {
        // Over 1♥, 2♠ (one jump over 1♠)
        (2, x_strain)
    } else {
        // Below or equal to major: 3-level jump
        (3, x_strain)
    }
}

/// Responses to our `1♣`/`1♦` opening
///
/// Four-card majors up the line, a 2/1 game force (`1♦–2♣`), the notrump
/// ladder when no major fits, and inverted minor raises promising five-card
/// support (strong 2-of-minor forcing, weak preemptive 3-of-minor).
#[must_use]
pub fn minor_responses(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    let mut rules = Rules::new();
    // Major selection between 4+ majors, per the longer-major knob (default on).
    rules = if longer_major_response() {
        // Longer-major discipline (the default, `set_longer_major_response`): the response
        // names the longer major — 1♠ on 5♠4♥/6♠5♥ or any 5-5+, 1♥ up the
        // line only on 4-4 — so 1♥ denies longer spades and the M6.4
        // control-bid classifier can read `1♣–1♥–2♣–4♠` as a control bid.
        rules
            .rule(
                Bid::new(1, Strain::Spades),
                1.5,
                len(Suit::Spades, 4..) & points(6..) & spades_first(),
            )
            .rule(
                Bid::new(1, Strain::Hearts),
                1.4,
                len(Suit::Hearts, 4..) & points(6..) & hearts_first(),
            )
    } else {
        // Opt-in pair (`set_longer_major_response(false)`) — unconditional
        // hearts-first: any four-plus hearts responds 1♥ even with longer
        // spades (5♠4♥, 6♠5♥), so partner can only infer "1♠ denies four
        // hearts", never the converse, and the M6.4 classifier must read a
        // later jump into the suit *above* the response as natural to play (the
        // first M6.4 A/B round assumed longest-first here and lost 6 IMPs per
        // fired board).  This simplification measured a null against the
        // longer-major default and stays available as a knob; see
        // `set_longer_major_response` and `docs/bidding-theorems.md`.
        rules
            .rule(
                Bid::new(1, Strain::Hearts),
                1.5,
                len(Suit::Hearts, 4..) & points(6..),
            )
            .rule(
                Bid::new(1, Strain::Spades),
                1.4,
                len(Suit::Spades, 4..) & points(6..) & len(Suit::Hearts, ..4),
            )
    };
    // Up-the-line completion (`set_up_the_line`): a natural 1♦ over 1♣ on
    // four-plus diamonds without a four-card major.  Weight 1.2 sits below
    // the majors (1.5/1.4) and the inverted raise (1.25), above the notrump
    // ladder (1.0) — so diamond hands stop mislabeling themselves as
    // balanced notrump responses or falling to the floor.
    if minor == Suit::Clubs && up_the_line() {
        rules = rules.rule(
            Bid::new(1, Strain::Diamonds),
            1.2,
            len(Suit::Diamonds, 4..)
                & points(6..)
                & len(Suit::Hearts, ..4)
                & len(Suit::Spades, ..4),
        );
    }
    rules = rules
        // Notrump ladder without a four-card major (3NT open-ended for game-plus).
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(13..) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(11..=12) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            hcp(6..=10) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // Inverted minor raises (five-card support required since opener may hold only three).
        // Strong raise: forcing one round — no majors, 10+ points.
        .rule(
            Bid::new(2, trump),
            1.25,
            support(5..) & support_points(10..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .alert(INVERTED_MINOR)
        // Weak preemptive raise.  `support_points` here is behaviour-neutral —
        // the strong-raise floor above dominates every hand it could promote —
        // so it rides along to keep every fit-known raise gate on one scale.
        .rule(Bid::new(3, trump), 1.1, support(5..) & support_points(..=9))
        .alert(INVERTED_MINOR)
        .rule(Call::Pass, 0.0, hcp(..6));

    // Weak jump shifts: 2♥ and 2♠ over either minor.
    for x in [Suit::Hearts, Suit::Spades] {
        rules = rules
            .rule(
                Bid::new(2, Strain::from(x)),
                1.0,
                len(x, 6..) & points(2..=5),
            )
            .alert(WEAK_JUMP_SHIFT);
    }

    // 2/1 game force: 1♦–2♣ (clubs are cheaper than diamonds).
    if minor == Suit::Diamonds {
        rules = rules
            .rule(
                Bid::new(2, Strain::Clubs),
                1.3,
                len(Suit::Clubs, 4..)
                    & points(13..)
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4),
            )
            .alert(GAME_FORCE);
    }
    rules
}

/// Register the first responses and their response-level continuations
///
/// Inserts the response tables to every one-of-a-suit opening, then opener's
/// rebids after splinters and inverted raises.
pub(super) fn register(book: &mut Trie) {
    // --- First responses to every one-of-a-suit opening ---
    for major in [Suit::Hearts, Suit::Spades] {
        let m_strain = Strain::from(major);
        super::insert_uncontested(book, &[super::call(1, m_strain)], major_responses(major));

        // Opener's choice after the choice-of-games 3NT: correct to 4M with
        // an unbalanced hand (the alerted reading pins 3+ support, so the
        // 5-3 fit is known), pass balanced — including 5332, which the
        // floor's ruffing-shortness correction would wrongly pull.
        if major_choice_of_games() {
            super::insert_uncontested(
                book,
                &[super::call(1, m_strain), super::call(3, Strain::Notrump)],
                Rules::new()
                    .rule(Bid::new(4, m_strain), 1.0, !balanced())
                    .rule(Call::Pass, 0.0, hcp(0..)),
            );
        }
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        super::insert_uncontested(
            book,
            &[super::call(1, Strain::from(minor))],
            minor_responses(minor),
        );
    }

    // --- Splinter continuations (opener's rebid after a splinter) ---
    for major in [Suit::Hearts, Suit::Spades] {
        let m_strain = Strain::from(major);
        let splinter_suits: &[Suit] = if major == Suit::Hearts {
            &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
        } else {
            &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
        };

        for &x in splinter_suits {
            let (level, strain) = splinter_bid(major, x);
            let splinter = super::call(level, strain);
            let our_calls = &[super::call(1, m_strain), splinter];

            let after_splinter = Rules::new()
                .rule(Bid::new(4, Strain::Notrump), 1.0, support_points(16..))
                .alert(super::slam::RKCB)
                .rule(Bid::new(4, m_strain), 0.5, hcp(0..));

            super::insert_uncontested(book, our_calls, after_splinter);
            super::slam::install_rkcb(book, our_calls, major);
        }
    }

    // --- Inverted minor raise continuations (opener's rebid) ---
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let m_strain = Strain::from(minor);
        let our_calls = &[super::call(1, m_strain), super::call(2, m_strain)];

        // Opener's rebid after the inverted raise: no Pass (forcing).
        let after_inv_raise = Rules::new()
            .rule(Bid::new(2, Strain::Notrump), 1.0, hcp(12..=14) & balanced())
            .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(18..=19))
            .rule(
                Bid::new(2, Strain::Hearts),
                0.8,
                stopper_in(Suit::Hearts) & hcp(15..),
            )
            .rule(
                Bid::new(2, Strain::Spades),
                0.8,
                stopper_in(Suit::Spades) & hcp(15..),
            )
            .rule(Bid::new(3, m_strain), 0.5, hcp(0..));

        super::insert_uncontested(book, our_calls, after_inv_raise);

        // Responder's third call after opener bids 2NT.
        let after_2nt = Rules::new()
            .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(13..))
            .rule(Bid::new(3, m_strain), 0.5, hcp(0..));

        let our_calls_2nt = &[
            super::call(1, m_strain),
            super::call(2, m_strain),
            super::call(2, Strain::Notrump),
        ];
        super::insert_uncontested(book, our_calls_2nt, after_2nt);

        // Responder's third call after opener's 18–19 jump to 3NT: with slam
        // values (~32+ combined and 5+-card support) launch minor RKCB; else
        // play the cold 3NT.  4NT is keycard here by construction — install_rkcb
        // registers the answers below this node.  Rides the minor-keycard knob
        // (off = no authored node, the pre-keycard book where inverted raises
        // topped out at 3NT).
        if super::slam::minor_keycard() {
            let after_3nt = Rules::new()
                .rule(Bid::new(4, Strain::Notrump), 1.0, support_points(14..))
                .alert(super::slam::RKCB)
                .rule(Call::Pass, 0.5, hcp(0..));
            let our_calls_3nt = &[
                super::call(1, m_strain),
                super::call(2, m_strain),
                super::call(3, Strain::Notrump),
            ];
            super::insert_uncontested(book, our_calls_3nt, after_3nt);
            super::slam::install_rkcb(book, our_calls_3nt, minor);
        }

        // Responder's third call after opener bids 2♥ or 2♠.
        for major in [Suit::Hearts, Suit::Spades] {
            let major_strain = Strain::from(major);
            let after_major = Rules::new()
                .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(13..))
                .rule(Bid::new(2, Strain::Notrump), 0.8, hcp(10..=12))
                .rule(Bid::new(3, m_strain), 0.5, hcp(0..));

            let our_calls_major = &[
                super::call(1, m_strain),
                super::call(2, m_strain),
                super::call(2, major_strain),
            ];
            super::insert_uncontested(book, our_calls_major, after_major);

            // Fourth call: after [1m, 2m, 2M, 2NT].
            let after_2nt_4th = Rules::new().rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..));

            let our_calls_2nt_4th = &[
                super::call(1, m_strain),
                super::call(2, m_strain),
                super::call(2, major_strain),
                super::call(2, Strain::Notrump),
            ];
            super::insert_uncontested(book, our_calls_2nt_4th, after_2nt_4th);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{hearts_first, hearts_take_first, spades_take_first};
    use crate::bidding::constraint::Constraint;

    #[test]
    fn major_selectors_partition_every_holding() {
        // `hearts_first` is the exact complement of `spades_first`, so on any
        // pair of major lengths exactly one of the two selectors fires — the
        // guard that a future edit cannot let them overlap or leave a gap.
        for spades in 0..=13 {
            for hearts in 0..=13 - spades {
                assert_ne!(
                    spades_take_first(spades, hearts),
                    hearts_take_first(spades, hearts),
                    "selectors overlap or gap at {spades}♠ {hearts}♥",
                );
            }
        }
        // Direction: 4-4 up the line to hearts, 5-5 high to spades, 5♠4♥ longer.
        assert!(hearts_take_first(4, 4));
        assert!(spades_take_first(5, 5));
        assert!(spades_take_first(5, 4));
    }

    #[test]
    fn hearts_first_renders_positively() {
        // The point of the change: the 1♥ selector reads as positive prose, not
        // a negated `spades_first` ("not (…)").
        assert_eq!(
            hearts_first().describe().to_string(),
            "hearts longer than spades, or equal below five",
        );
    }
}
