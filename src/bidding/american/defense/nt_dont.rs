//! DONT over their `1NT` — Disturb Opponents' NoTrump
//!
//! `X` = a one-suiter, `2♣`/`2♦`/`2♥` = that suit and a higher one, all
//! pass-or-correct.  Both the direct-seat shapes and the passed-hand twins
//! (capped below opening, so every advance is a two-level signoff) are
//! here.

use super::nt_defense::NotrumpDefense;
use super::*;

/// Whether the direct-seat DONT defense is the active system
pub(super) fn direct_dont_enabled(agreements: &Agreements) -> bool {
    agreements.decision.reading.notrump_defense() == NotrumpDefense::DirectDont
}

thread_local! {
    /// Minimum length to insist on a DONT one-suiter (the `X` for ♣/♦/♥, the
    /// natural `2♠` for spades); **5 by default**.  Set to 6 to bid only with a
    /// six-card suit, passing five-card one-suiters (the X bucket is the DD loser,
    /// so insisting only with real shape trades action for safety — toward the
    /// always-pass optimum).  An A/B knob, no effect unless DONT is on.
    static DIRECT_DONT_ONE_SUITER_MIN: Cell<u8> = const { Cell::new(5) };
    /// Whether DONT two-suiters (`2♣`/`2♦`/`2♥`) accept a flat 4-4 (else 5-4+);
    /// **on by default** — DONT is traditionally a 4-4 method (M6.2d).  Off, only
    /// 5-4+ two-suiters compete (tighter, fewer auctions).  An A/B knob, no effect
    /// unless DONT is on.
    static DIRECT_DONT_FOUR_FOUR: Cell<bool> = const { Cell::new(true) };
    /// `points` floor for the DONT one-suiter `X`; **0 by default = inherit the natural
    /// overcall floor (8)**, byte-identical.  Raise it so only strong one-suiters
    /// double and 8-11 hands pass (the `X` bucket is the DD loser — trade action for
    /// safety, as [`DIRECT_DONT_ONE_SUITER_MIN`] does for length).  A/B knob, no effect
    /// unless DONT is on.
    static DIRECT_DONT_X_FLOOR: Cell<u8> = const { Cell::new(0) };
}

/// Minimum one-suiter length for the DONT `X`/`2♠` (default 5; set 6 to pass
/// five-card one-suiters).  See [`NotrumpDefense::DirectDont`].
pub fn set_direct_dont_one_suiter_min(min: u8) {
    DIRECT_DONT_ONE_SUITER_MIN.with(|cell| cell.set(min));
}

/// The raw minimum-length knob for direct DONT's one-suiter
pub(super) fn direct_dont_one_suiter_min_raw() -> u8 {
    DIRECT_DONT_ONE_SUITER_MIN.with(Cell::get)
}

/// Minimum one-suiter length, widened to the constraint DSL's length type
pub(super) fn direct_dont_one_suiter_min(agreements: &Agreements) -> usize {
    usize::from(agreements.defense.direct_dont_one_suiter_min)
}

/// Whether DONT two-suiters accept a flat 4-4 (default true = traditional 4-4; false =
/// 5-4+).  See [`NotrumpDefense::DirectDont`].
pub fn set_direct_dont_four_four(on: bool) {
    DIRECT_DONT_FOUR_FOUR.with(|cell| cell.set(on));
}

/// Set the `points` floor for the DONT one-suiter `X` (default 0 = inherit the natural
/// overcall floor of 8; raise it to double only with strong one-suiters).  See
/// [`NotrumpDefense::DirectDont`].
pub fn set_direct_dont_x_floor(floor: u8) {
    DIRECT_DONT_X_FLOOR.with(|cell| cell.set(floor));
}

/// The raw configured DONT `X` floor before resolving the zero sentinel
pub(super) fn direct_dont_x_floor_raw() -> u8 {
    DIRECT_DONT_X_FLOOR.with(Cell::get)
}

/// The configured DONT `X` floor, resolving the zero sentinel to the natural overcall floor.
fn direct_dont_x_floor(agreements: &Agreements) -> u8 {
    match agreements.defense.direct_dont_x_floor {
        0 => agreements.decision.reading.natural_overcall_points().0,
        floor => floor,
    }
}

/// Whether the [`set_direct_dont_four_four`] knob is on
pub fn direct_dont_four_four() -> bool {
    DIRECT_DONT_FOUR_FOUR.with(Cell::get)
}

/// DONT `X`: a one-suiter (♣/♦/♥), `points(direct-dont-x-floor..)`.
pub(super) fn dont_x(agreements: &Agreements) -> Rules {
    let lo = direct_dont_x_floor(agreements);
    let one_min = direct_dont_one_suiter_min(agreements);
    Rules::new().rule(
        Call::Double,
        190,
        dont_one_suiter_direct(one_min) & points(lo..),
    )
}

/// DONT `2♣`: clubs + a higher major, 5-4 (or 4-4 when configured).
pub(super) fn dont_2c(agreements: &Agreements) -> Rules {
    let lo = agreements.decision.reading.natural_overcall_points().0;
    let ff = agreements.defense.direct_dont_four_four;
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        200,
        dont_minor_major(Suit::Clubs, ff) & points(lo..),
    )
}

/// DONT `2♦`: diamonds + a higher major, 5-4 (or 4-4 when configured).
pub(super) fn dont_2d(agreements: &Agreements) -> Rules {
    let lo = agreements.decision.reading.natural_overcall_points().0;
    let ff = agreements.defense.direct_dont_four_four;
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        200,
        dont_minor_major(Suit::Diamonds, ff) & points(lo..),
    )
}

/// DONT `2♥`: both majors, 5-4 (or 4-4 when configured).
pub(super) fn dont_2h(agreements: &Agreements) -> Rules {
    let lo = agreements.decision.reading.natural_overcall_points().0;
    let ff = agreements.defense.direct_dont_four_four;
    Rules::new().rule(
        Bid::new(2, Strain::Hearts),
        200,
        dont_both_majors(ff) & points(lo..),
    )
}

// Direct-seat DONT shapes.  Unlike the passed-hand twins these carry no six-card
// cap (an unpassed hand may hold a long suit), and they carve clubs+diamonds onto
// the `2NT` both-minors overlay so `2♣`/`2♦` mean a minor + a *major*.

/// Direct-seat DONT `X`: a one-suiter (a `min`+ suit, no second four-card suit) whose
/// long suit is a minor or hearts.  A spade one-suiter bids the natural `2♠`, so the
/// spade-long arm is omitted; each arm caps the other three suits at three, so exactly
/// one suit is long.  `min` (5 or 6) is [`set_direct_dont_one_suiter_min`].
pub(super) fn dont_one_suiter_direct(min: usize) -> Cons<impl Constraint + Clone> {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    (len(Clubs, min..) & and([Diamonds, Hearts, Spades], ..=3))
        | (len(Diamonds, min..) & and([Clubs, Hearts, Spades], ..=3))
        | (len(Hearts, min..) & and([Clubs, Diamonds, Spades], ..=3))
}

/// Direct-seat DONT `2♣`/`2♦`: a minor + a *major*, 5-4 either way (or a flat 4-4
/// when `allow_44`).  The higher suit is ♥/♠ only — a minor + the other minor is
/// shown as `2NT` (both minors), not here.  `allow_44` is
/// [`set_direct_dont_four_four`].
pub(super) fn dont_minor_major(minor: Suit, allow_44: bool) -> Cons<impl Constraint + Clone> {
    let longer = if allow_44 { 4 } else { 5 };
    // The minor (4+) plus a higher major (4+), one of the two at least `longer` — 5-4
    // either way, or a flat 4-4 when `allow_44` (then the third clause is redundant).
    len(minor, 4..)
        & or([Suit::Hearts, Suit::Spades], 4..)
        & (len(minor, longer..) | or([Suit::Hearts, Suit::Spades], longer..))
}

/// Direct-seat DONT `2♥`: both majors, 5-4 either way (or a flat 4-4 when `allow_44`).
/// A separate function from [`both_majors_shape`] (direct-Landy `X`) — identical shape
/// today, but on an independent flag, so the two conventions may diverge.
pub(super) fn dont_both_majors(allow_44: bool) -> Cons<impl Constraint + Clone> {
    let longer = if allow_44 { 4 } else { 5 };
    and([Suit::Hearts, Suit::Spades], 4..) & or([Suit::Hearts, Suit::Spades], longer..)
}

// ---------------------------------------------------------------------------
// Passed-hand DONT advances.  Both partners passed in `- - - (1NT) …`, so the
// advancer is capped below opening too: every response is a pass-or-correct
// signoff at the two level — no invite/game/ask arms (they are unreachable).
// ---------------------------------------------------------------------------

/// Advancing partner's DONT one-suiter double (`… (1NT) X -`): relay `2♣` to ask
/// which suit.  (A passed advancer is too weak to introduce its own suit, so the
/// single relay covers it.)
fn passed_dont_x_advance() -> Rules {
    Rules::new().rule(Bid::new(2, Strain::Clubs), 100, hcp(0..))
}

/// Doubler naming the one-suiter after the `2♣` relay (`… (1NT) X - 2♣ -`): pass
/// with clubs, else bid the five-or-six-card suit.
fn passed_dont_x_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Diamonds, 5..))
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 5..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing partner's DONT `2♣` (clubs + a higher suit, `… (1NT) 2♣ -`): pass
/// with club tolerance, else relay `2♦` ("name your higher suit").
pub(super) fn passed_dont_2c_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Clubs, ..=2))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler naming the higher suit after the `2♦` relay (`… (1NT) 2♣ - 2♦ -`):
/// pass with diamonds, else bid the major.
pub(super) fn passed_dont_2c_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing partner's DONT `2♦` (diamonds + a major, `… (1NT) 2♦ -`): pass with
/// diamond tolerance, else relay `2♥` ("name your major").
pub(super) fn passed_dont_2d_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Diamonds, ..=2))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler naming the major after the `2♥` relay (`… (1NT) 2♦ - 2♥ -`): pass with
/// hearts, correct to `2♠` with spades.
pub(super) fn passed_dont_2d_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing partner's DONT `2♥` (both majors, `… (1NT) 2♥ -`): pass with hearts,
/// correct to `2♠` with longer spades.
pub(super) fn passed_dont_2h_advance() -> Rules {
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Direct-seat DONT advances: the same pass-or-correct relays, keyed at
/// *every* seat (the `X`/`2♣`/`2♦`/`2♥` are direct-seat conventional calls)
///
/// Binding `(1NT) X -` is correct here — with DONT on, the direct `X` is a
/// one-suiter wanting the `2♣` relay, not a penalty.  Every artificial leg
/// carries a doubled/redoubled escape so we never sit in `1NT`-redoubled or a
/// doubled misfit `2♣`, the dominant DONT-`X` loss in the honest measure.
pub(super) fn direct_dont_advance_package() -> Package {
    Package {
        name: "direct-dont-advance",
        gate: |agreements| direct_dont_enabled(agreements),
        entries: |_| {
            let mut entries = rows_of(Pattern::node("P* (1NT) X -"), passed_dont_x_advance());
            for (key, rules) in [
                ("P* (1NT) X - 2♣ -", passed_dont_x_rebid()),
                ("P* (1NT) 2♣ -", passed_dont_2c_advance()),
                ("P* (1NT) 2♣ - 2♦ -", passed_dont_2c_rebid()),
                ("P* (1NT) 2♦ -", passed_dont_2d_advance()),
                ("P* (1NT) 2♦ - 2♥ -", passed_dont_2d_rebid()),
                ("P* (1NT) 2♥ -", passed_dont_2h_advance()),
                // Their redouble of our one-suiter X: never sit in 1NTxx — relay
                // 2♣ just as over a pass, then the doubler names the suit.
                ("P* (1NT) X (XX)", passed_dont_x_advance()),
                ("P* (1NT) X (XX) 2♣ -", passed_dont_x_rebid()),
                // Their double of our artificial 2♣ relay (after our X, passed or
                // redoubled): the relay is NOT a club fit, so the doubler must
                // still name the real one-suiter (or pass with genuine clubs).
                ("P* (1NT) X - 2♣ (X)", passed_dont_x_rebid()),
                ("P* (1NT) X (XX) 2♣ (X)", passed_dont_x_rebid()),
            ] {
                entries.extend(rows_of(Pattern::node(key), rules));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
