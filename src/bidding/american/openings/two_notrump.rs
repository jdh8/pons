//! The strong 2NT opening and its wide-minor shape agreement
//!
//! [`two_notrump_wide_shape`] is read by `src/bidding/american/defense.rs`, and
//! [`two_notrump_wide`][field@crate::bidding::inference::ReadingProfile::two_notrump_wide]
//! is read by `src/bidding/inference.rs`.

use crate::bidding::Rules;
use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{Cons, Constraint, balanced, fifths, length_box, shapes};
use crate::bidding::inference::Range;
use contract_bridge::{Bid, Strain};
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

pub(super) fn with_two_notrump(rules: Rules, agreements: &Agreements) -> Rules {
    let mut rules = rules;
    // Strong 2NT.  `balanced()` by default; the G0 opt-in
    // (`ReadingProfile::two_notrump_wide`) swaps in the wide-minor shape. Each
    // arm reissues `.rule()` so the differing constraint types unify to `Rules`.
    //
    // The field is read at classify time too (`inference/readers.rs` caps the
    // 2NT opener's minor length on it), so it lives in `DecisionProfile` and
    // is read from there here rather than duplicated into `OpeningKnobs` — one
    // value, one home.
    rules = if agreements.decision.reading.two_notrump_wide {
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
