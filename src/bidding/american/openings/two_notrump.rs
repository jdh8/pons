//! The strong 2NT opening and its wide-minor shape agreement
//!
//! [`two_notrump_wide_shape`] is read by `src/bidding/american/defense.rs`, and
//! [`two_notrump_wide`] is read by `src/bidding/inference.rs`.

use crate::bidding::Rules;
use crate::bidding::constraint::{Cons, Constraint, balanced, fifths, length_box, shapes};
use crate::bidding::inference::Range;
use contract_bridge::{Bid, Strain};
use std::cell::Cell;

thread_local! {
    /// Whether the strong 2NT (20-21) opening admits the wide-minor shape
    /// instead of plain `balanced()`.  Default `false` (byte-identical).  See
    /// [`set_two_notrump_wide`].
    static TWO_NOTRUMP_WIDE: Cell<bool> = const { Cell::new(false) };
}

/// Open the strong 2NT (20-21) on the wide-minor shape `{M 2..=4, m 2..=6}`
/// instead of plain `balanced()` (opt-in; the default `false` is byte-identical)
///
/// This is DNF-ledger chop G0 (docs/dnf-migration.md): the 1NT `Wide6322`
/// treatment carried up to the 20-21 opening.  It **drops the 5M(332)** balanced
/// hands (a 5-card major now opens one-of-a-major and jump-rebuilds) and **adds
/// the wide minors** (5m422/6m322), mirroring [`NotrumpShape::Wide6322`][super::NotrumpShape::Wide6322] minus
/// its two major pan-handles.  The reading in
/// [`apply_opening`][crate::bidding::inference] widens opener's minors to six
/// under the same knob.
pub fn set_two_notrump_wide(on: bool) {
    TWO_NOTRUMP_WIDE.with(|cell| cell.set(on));
}

/// Whether the strong 2NT opening admits the wide-minor shape (chop G0).  Read
/// by both the opening table and the inference reading so they stay in step.
pub(crate) fn two_notrump_wide() -> bool {
    TWO_NOTRUMP_WIDE.with(Cell::get)
}

/// The wide-minor 2NT shape (chop G0): the `Wide6322` cube `{M 2..=4, m 2..=6}`
/// with no pan-handles — balanced/semi-balanced with the longest suit a minor.
///
/// The 13-card sum does the excluding: majors capped at four drop every 5-card
/// major (so 5M(332) opens one-of-a-major instead), and minors run to six for
/// the 5m(422)/6m(322) hands.  `{4432, 4333, 5m332, 5m422, 6m322}`.
pub(crate) fn two_notrump_wide_shape() -> Cons<impl Constraint + Clone> {
    shapes(
        "balanced or wide-minor 2NT shape",
        vec![length_box([
            Range::new(2, 6), // clubs
            Range::new(2, 6), // diamonds
            Range::new(2, 4), // hearts
            Range::new(2, 4), // spades
        ])],
    )
}

pub(super) fn with_two_notrump(rules: Rules) -> Rules {
    let mut rules = rules;
    // Strong 2NT.  `balanced()` by default; the G0 opt-in
    // (`set_two_notrump_wide`) swaps in the wide-minor shape.  Each arm reissues
    // `.rule()` so the differing constraint types unify to `Rules`.
    rules = if TWO_NOTRUMP_WIDE.with(Cell::get) {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            200,
            fifths(20.0..22.0) & two_notrump_wide_shape(),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            200,
            fifths(20.0..22.0) & balanced(),
        )
    };
    rules
}
