//! The strong 1NT opening and its shape and strength agreements

use crate::bidding::Rules;
use crate::bidding::constraint::{
    Cons, Constraint, described, fifths, hcp, length_box, long_suit_box, shapes,
};
use crate::bidding::context::Context;
use crate::bidding::inference::Range;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

thread_local! {
    /// Whether our side opens a strong balanced 15-17 with 1NT.  Default `true`.
    static OPEN_ONE_NOTRUMP: Cell<bool> = const { Cell::new(true) };
}

thread_local! {
    /// Restore the fifths gauge (`fifths(14.5..17.5)`, centre-matched to plain HCP
    /// 15-17) for the 1NT opening.  Default `false` — the opening gauges plain HCP.
    static ONE_NOTRUMP_FIFTHS: Cell<bool> = const { Cell::new(false) };
}

thread_local! {
    /// Which shape policy the 1NT opening admits when `american()` rebuilds.
    /// Default [`NotrumpShape::Wide6322`] (the shipped default).
    static NOTRUMP_SHAPE: Cell<NotrumpShape> = const { Cell::new(NotrumpShape::Wide6322) };
}

thread_local! {
    /// Whether the 1NT opening admits human-style off-shape hands. Default
    /// `false` (byte-identical). See [`set_one_notrump_offshape`].
    static ONE_NOTRUMP_OFFSHAPE: Cell<bool> = const { Cell::new(false) };
}

/// Suppress (`false`) or restore (`true`, the default) our own 1NT opening.
///
/// With it off, a strong balanced 15-17 opens a minor instead of 1NT, so a
/// diagnostic can isolate our *defense* to an opponent's 1NT without our own 1NT
/// openings polluting the duplicate (see `bba-match --no-our-1nt`).
pub fn set_open_one_notrump(on: bool) {
    OPEN_ONE_NOTRUMP.with(|cell| cell.set(on));
}

/// Whether the [`set_open_one_notrump`] knob is on
pub fn open_one_notrump() -> bool {
    OPEN_ONE_NOTRUMP.with(Cell::get)
}

/// Restore the legacy fifths strength gauge for the 1NT opening (`true`); the
/// default (`false`) gauges plain HCP 15-17, which opens 1NT a touch more often.
pub fn set_one_notrump_fifths(on: bool) {
    ONE_NOTRUMP_FIFTHS.with(|cell| cell.set(on));
}

/// Select the 1NT opening [`NotrumpShape`] for the next rebuild of
/// [`american()`][crate::american()] — the web Settings shape radio.  Default
/// [`NotrumpShape::Wide6322`].  Read by
/// [`american_book`][crate::bidding::american::american_book], so every
/// constructor built on it picks up the setting.
pub fn set_notrump_shape(shape: NotrumpShape) {
    NOTRUMP_SHAPE.with(|cell| cell.set(shape));
}

/// The 1NT opening shape currently selected by [`set_notrump_shape`].
pub fn notrump_shape_setting() -> NotrumpShape {
    NOTRUMP_SHAPE.with(Cell::get)
}

/// Admit 5422 and mild singleton-honour shapes to the 15–17 1NT opening
/// (opt-in; the default `false` is byte-identical).
pub fn set_one_notrump_offshape(on: bool) {
    ONE_NOTRUMP_OFFSHAPE.with(|cell| cell.set(on));
}

/// Whether the 1NT opening currently admits the off-shape hands
/// ([`set_one_notrump_offshape`]) — read by the generated convention card as
/// well as by the rules.
pub(crate) fn one_notrump_offshape() -> bool {
    ONE_NOTRUMP_OFFSHAPE.with(Cell::get)
}

/// Which hand shapes the strong 1NT opening admits ([`super::openings_with`])
///
/// Every variant opens the balanced patterns (4333/4432/5332).  A long *major*
/// always prefers a one-of-a-major opening it can rebid, so the shapely
/// additions are minor-only.  Strength ([`fifths`] 15–17) and the inference side
/// are untouched; this is the shape-only knob for the deferred redesign (see the
/// `nt-shape-abc` and `nt-shape-contested` examples).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotrumpShape {
    /// Balanced only — the classic baseline.
    Balanced,
    /// Balanced plus a 5422 with a five-card minor — the pre-6322 baseline.
    Wide,
    /// [`Wide`][NotrumpShape::Wide] plus a 6322 with a six-card minor — the
    /// shipped default (adopted after a two-seed A/B win vs the reference
    /// opponent, +0.004…0.006 IMPs/board plain, sd-confirmed).
    Wide6322,
}

/// Shapes eligible for a 1NT opening, per the [`NotrumpShape`] policy
///
/// Authored as an exact union of length boxes ([`shapes`]).  The wide
/// variants are a bigger cube plus two pan-handles: majors capped at four and
/// minors at five (`Wide`) or six (`Wide6322`) — the 13-card sum excludes
/// 5-5, 6-4, 7222, and singletons from the cube, so it is exactly the
/// balanced-with-≤4-card-majors patterns plus 5m(422) (and 6m(322) for
/// `Wide6322`) — plus the two 5M(332) boxes restoring the balanced five-card
/// majors.  `Balanced` is the plain 4333/4432/5332 union.  Eval-equivalence
/// with the closure this replaces is pinned exhaustively by
/// `notrump_shape_boxes_match_closure`.
pub(crate) fn notrump_shape(shape: NotrumpShape) -> Cons<impl Constraint + Clone> {
    let major_cube = Range::new(2, 4);
    let (label, minor_cube) = match shape {
        NotrumpShape::Balanced => ("balanced 1NT shape", Range::new(2, 4)),
        NotrumpShape::Wide => ("balanced or 5m(422) 1NT shape", Range::new(2, 5)),
        NotrumpShape::Wide6322 => ("balanced or 5m(422)/6m(322) 1NT shape", Range::new(2, 6)),
    };
    let mut boxes = vec![length_box([minor_cube, minor_cube, major_cube, major_cube])];
    for suit in Suit::ASC {
        // The pan-handles: 5(332) for every suit under `Balanced` (the plain
        // balanced union), majors only for the wide cubes (whose minor range
        // already covers the five-card-minor patterns).
        if shape == NotrumpShape::Balanced || matches!(suit, Suit::Hearts | Suit::Spades) {
            boxes.push(long_suit_box(suit, Range::new(5, 5), Range::new(2, 3)));
        }
    }
    shapes(label, boxes)
}

/// Human-style off-shape 1NT hands: any 5422, or 4441/5431 with a singleton Q/J.
fn one_notrump_offshape_gate() -> Cons<impl Constraint + Clone> {
    let mut boxes = Vec::new();
    for suit in Suit::ASC {
        boxes.push(long_suit_box(suit, Range::new(5, 5), Range::new(2, 2)));
    }
    let shape = shapes("5422 1NT shape", boxes);
    shape
        | described(
            "4441/5431 with a singleton queen or jack",
            |hand: Hand, _: &Context<'_>| {
                let mut lengths = Suit::ASC.map(|suit| hand[suit].len());
                lengths.sort_unstable();
                matches!(lengths, [1, 3, 4, 5] | [1, 4, 4, 4])
                    && Suit::ASC.into_iter().any(|suit| {
                        hand[suit].len() == 1
                            && (hand[suit].contains(contract_bridge::Rank::Q)
                                || hand[suit].contains(contract_bridge::Rank::J))
                    })
            },
        )
}

pub(super) fn with_one_notrump(rules: Rules, shape: NotrumpShape) -> Rules {
    let mut rules = rules;
    // Strong 1NT — gated so a diagnostic can suppress our own 1NT opening
    // (`set_open_one_notrump`); the 15-17 balanced hands then open a minor.
    if OPEN_ONE_NOTRUMP.with(Cell::get) {
        // Strength gauged by plain HCP 15-17 by default; `set_one_notrump_fifths`
        // restores the legacy Andrews' fifths gauge.  Each arm reissues `.rule()`
        // so the differing constraint types unify to `Rules`.
        rules = if (ONE_NOTRUMP_FIFTHS.with(Cell::get), one_notrump_offshape()) == (true, true) {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                fifths(14.5..17.5) & (notrump_shape(shape) | one_notrump_offshape_gate()),
            )
        } else if ONE_NOTRUMP_FIFTHS.with(Cell::get) {
            // 14.5..17.5 (centre 16), not 15..18 (centre 16.5): fifths sums to 40
            // over the deck like HCP, so an unbiased "15-17 HCP" gate shares the
            // plain-HCP band's centre — the old 15..18 was half a point too high.
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                fifths(14.5..17.5) & notrump_shape(shape),
            )
        } else if one_notrump_offshape() {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                hcp(15..=17) & (notrump_shape(shape) | one_notrump_offshape_gate()),
            )
        } else {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                hcp(15..=17) & notrump_shape(shape),
            )
        };
    }
    rules
}
