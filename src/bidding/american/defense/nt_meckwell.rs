//! Meckwell over their `1NT`
//!
//! `X` = a minor, or both majors; `2♣`/`2♦` = that minor plus a major.  The
//! `X` is the two-way call that makes it Meckwell rather than DONT
//! ([`set_meckwell_x_four_four`], [`set_meckwell_x_floor`]).

use super::nt_defense::{NotrumpDefense, notrump_defense};
use super::nt_dont::{
    dont_minor_major, passed_dont_2c_advance, passed_dont_2c_rebid, passed_dont_2d_advance,
    passed_dont_2d_rebid, passed_dont_2h_advance,
};
use super::nt_landy::both_majors_shape;
use super::overcall::natural_overcall_points;
use super::*;

thread_local! {
    /// Whether Meckwell's `2♣`/`2♦` (minor + a major) accept a flat 4-4 (else 5-4+);
    /// **off by default** (5-4).  A **probe** knob — the 5-4-vs-4-4 boundary is
    /// measured, not fixed by theory.  No effect unless Meckwell is on.
    static MECKWELL_MINOR_MAJOR_44: Cell<bool> = const { Cell::new(false) };
    /// Whether Meckwell's both-majors `X` accepts a flat 4-4 (else 5-4+); **on by
    /// default** (4-4, the standard weak Meckwell takeout double).  A **probe** knob.
    /// No effect unless Meckwell is on.
    static MECKWELL_X_FOUR_FOUR: Cell<bool> = const { Cell::new(true) };
    /// `points` floor for Meckwell's two-way `X`; **0 by default = inherit the natural
    /// overcall floor (8)**, byte-identical.  Raise it (e.g. 12, the Woolsey `X` floor)
    /// so only strong hands make the broad two-way double and 8-11 both-majors /
    /// single-minor hands pass — fewer sacrificial doubles over a strong 1NT.  A
    /// **probe** knob (the tournament's dominant Meckwell loss is the low-floor `X`).
    static MECKWELL_X_FLOOR: Cell<u8> = const { Cell::new(0) };
}

/// Whether the direct-seat Meckwell defense is the active system
pub(crate) fn meckwell_enabled() -> bool {
    notrump_defense() == NotrumpDefense::Meckwell
}

/// Whether Meckwell's `2♣`/`2♦` accept a flat 4-4 (default `false` = 5-4+).  A
/// **probe** knob.  See [`NotrumpDefense::Meckwell`].
pub fn set_meckwell_minor_major_44(on: bool) {
    MECKWELL_MINOR_MAJOR_44.with(|cell| cell.set(on));
}

fn meckwell_minor_major_44() -> bool {
    MECKWELL_MINOR_MAJOR_44.with(Cell::get)
}

/// Whether Meckwell's both-majors `X` accepts a flat 4-4 (default `true` = 4-4).  A
/// **probe** knob.  See [`NotrumpDefense::Meckwell`].
pub fn set_meckwell_x_four_four(on: bool) {
    MECKWELL_X_FOUR_FOUR.with(|cell| cell.set(on));
}

fn meckwell_x_four_four() -> bool {
    MECKWELL_X_FOUR_FOUR.with(Cell::get)
}

/// Set the `points` floor for Meckwell's two-way `X` (default 0 = inherit the natural
/// overcall floor of 8; set e.g. 12 for a Woolsey-strength double).  A **probe** knob.
/// See [`NotrumpDefense::Meckwell`].
pub fn set_meckwell_x_floor(floor: u8) {
    MECKWELL_X_FLOOR.with(|cell| cell.set(floor));
}

/// The configured Meckwell `X` floor, resolving the 0 sentinel to the natural
/// overcall floor.
fn meckwell_x_floor() -> u8 {
    match MECKWELL_X_FLOOR.with(Cell::get) {
        0 => natural_overcall_points().0,
        floor => floor,
    }
}

/// Meckwell two-way `X`: a single 6+ minor OR both majors,
/// `points(meckwell-x-floor..)`.  The both-majors shape is the probe knob
/// [`set_meckwell_x_four_four`], the floor is [`set_meckwell_x_floor`]; the
/// single-minor length is a fixed 6.
pub(super) fn meckwell_x() -> Rules {
    let lo = meckwell_x_floor();
    Rules::new().rule(
        Call::Double,
        190,
        meckwell_double_shape(6, meckwell_x_four_four()) & points(lo..),
    )
}

/// Meckwell `2♣`: clubs + a major, 5-4 either way (or flat 4-4 per the probe knob
/// [`set_meckwell_minor_major_44`]).  Shares [`dont_minor_major`]'s shape on the
/// Meckwell knob so the two conventions can diverge.
pub(super) fn meckwell_2c() -> Rules {
    let lo = natural_overcall_points().0;
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        200,
        dont_minor_major(Suit::Clubs, meckwell_minor_major_44()) & points(lo..),
    )
}

/// Meckwell `2♦`: diamonds + a major, 5-4 either way (or flat 4-4 per the probe knob).
pub(super) fn meckwell_2d() -> Rules {
    let lo = natural_overcall_points().0;
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        200,
        dont_minor_major(Suit::Diamonds, meckwell_minor_major_44()) & points(lo..),
    )
}

/// Meckwell two-way `X`: a single `min`+ minor (♣ or ♦, the other three suits ≤3) OR
/// both majors (5-4, or flat 4-4 when `four_four`).  The signature two-way double —
/// the two arms are disjoint (the one-suiter caps its majors ≤3, the both-majors
/// floors them ≥4), so the reading can tell a single-minor from a both-majors hand by
/// the majors alone.  `min` is a fixed 6 (the DONT one-suiter parity length).
pub(super) fn meckwell_double_shape(min: usize, four_four: bool) -> Cons<impl Constraint + Clone> {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    (len(Clubs, min..) & and([Diamonds, Hearts, Spades], ..=3))
        | (len(Diamonds, min..) & and([Clubs, Hearts, Spades], ..=3))
        | both_majors_shape(four_four)
}

/// Meckwell natural `2♥`/`2♠`: a 5+ single-suited major — the other major ≤3 (both
/// majors go through the `X`) and both minors ≤3 (a minor + this major goes through
/// `2♣`/`2♦`).  A pure one-suiter, disjoint from every Meckwell artificial call so a
/// 6-4 hand shows its two-suiter (`2♣`/`2♦`/`X`) rather than tying the natural rung.
pub(super) fn meckwell_natural_major(major: Suit) -> Cons<impl Constraint + Clone> {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    len(major, 5..) & len(other, ..=3) & and([Suit::Clubs, Suit::Diamonds], ..=3)
}

/// Advancing Meckwell's two-way `X` (`… (1NT) X -`): relay `2♣` (pass-or-correct) —
/// the doubler then names its minor or shows both majors.  A single relay resolves the
/// two-way double's ambiguity; the advancer's own suits wait for the doubler's answer.
fn meckwell_x_advance() -> Rules {
    Rules::new().rule(Bid::new(2, Strain::Clubs), 100, hcp(0..))
}

/// The Meckwell doubler naming its hand after the `2♣` relay (`… (1NT) X - 2♣ -`):
/// pass with a club one-suiter, `2♦` with a diamond one-suiter (real diamonds, short
/// majors), or `2♥` with both majors (4+ hearts — the advancer then passes or corrects
/// to `2♠` via [`passed_dont_2h_advance`]).  Names real suits throughout, so nothing
/// here is artificial (the both-majors hand under-describes as hearts, always sound).
fn meckwell_x_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, ..=3) & len(Suit::Spades, ..=3),
        )
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Direct-seat Meckwell advances: the `X` is a two-way "single 6+ minor OR
/// both majors" double
///
/// Advancer relays `2♣` (pass-or-correct); the doubler passes with clubs,
/// names `2♦` with diamonds, or bids `2♥` (4+ hearts ⇒ both majors here) and
/// the advancer passes or corrects to `2♠`.  The minor+major `2♣`/`2♦` reuse
/// the DONT pass-or-correct advances (the same "name your higher suit"
/// relay).  Every artificial leg has a doubled/redoubled escape.
pub(super) fn meckwell_advance_package() -> Package {
    Package {
        name: "meckwell-advance",
        gate: |_| meckwell_enabled(),
        entries: |_| {
            let mut entries = rows_of(Pattern::node("P* (1NT) X -"), meckwell_x_advance());
            for (key, rules) in [
                ("P* (1NT) X - 2♣ -", meckwell_x_rebid()),
                ("P* (1NT) X - 2♣ - 2♥ -", passed_dont_2h_advance()),
                // 2♣/2♦ minor+major: reuse the DONT pass-or-correct advances.
                ("P* (1NT) 2♣ -", passed_dont_2c_advance()),
                ("P* (1NT) 2♣ - 2♦ -", passed_dont_2c_rebid()),
                ("P* (1NT) 2♦ -", passed_dont_2d_advance()),
                ("P* (1NT) 2♦ - 2♥ -", passed_dont_2d_rebid()),
                // Their redouble of our X: relay 2♣ anyway (never sit 1NTxx).
                ("P* (1NT) X (XX)", meckwell_x_advance()),
                ("P* (1NT) X (XX) 2♣ -", meckwell_x_rebid()),
                // Their double of our artificial 2♣ relay: the doubler still names
                // the real suit (pass only with genuine clubs), else runs.
                ("P* (1NT) X - 2♣ (X)", meckwell_x_rebid()),
                ("P* (1NT) X (XX) 2♣ (X)", meckwell_x_rebid()),
                // Their double of the doubler's both-majors 2♥ show: advancer
                // still picks a major.
                ("P* (1NT) X - 2♣ - 2♥ (X)", passed_dont_2h_advance()),
            ] {
                entries.extend(rows_of(Pattern::node(key), rules));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
