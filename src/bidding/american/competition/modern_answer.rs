//! Opener's answer to the Modern negative double of a one-level major
//! overcall — `1m (1♥) X -` and `1m (1♠) X -`
//!
//! Gated on `agreements.competition.modern_double_answer` and
//! [`NegativeDoubleShape::Modern`] (the other schools author their own
//! answers in [`negative_double`][super::negative_double]).  The double shows
//! the other major — exactly four spades over `(1♥)`, four-plus hearts over
//! `(1♠)` — on 6+ HCP, unlimited.
//!
//! Without the node the answer is the instinct floor's, which reads the
//! double as game values: `4M` on any four-card fit from 11 points, `3NT` on
//! 13 with a stopper, and a penalty pass on four cards in their suit (anchor
//! `46d0dc14` vs BBA, per 409,600 boards: 507 rows, −1,323 plain / −2,726 PD).
//! The table is what BBA and BEN agree on: raise by strength, complete the
//! major on three when nothing else describes the hand, notrump by range with
//! a stopper, rebid a long minor — and never pass.

use super::negative_double::NegativeDoubleShape;
use super::*;

/// Opener's answer after `1o (1v) X -`, partner showing the other major
///
/// Every rung is a fixed bid at this node, so no `min_level_is` (see
/// [`answer_cue_raise`][super::cue_raise::answer_cue_raise] for why a
/// legality-anchored rung widens readings elsewhere).
fn answer_modern_double(opening: Suit, overcall: Suit) -> Rules {
    let o = Strain::from(opening);
    let major = if overcall == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let m = Strain::from(major);
    let cheap = if major == Suit::Spades { 1 } else { 2 };
    let short = len(major, ..=3);
    let mut rules = Rules::new()
        // The fit, raised by strength.
        .rule(Bid::new(4, m), 150, len(major, 4..) & points(17..))
        .rule(Bid::new(3, m), 150, len(major, 4..) & points(15..=16));
    // The minimum raise is always `2M`, so a one-level `1♠` is exactly three.
    rules = rules.rule(Bid::new(2, m), 150, len(major, 4..) & points(..=14));
    rules = rules
        // Notrump by range with their suit stopped.
        .rule(
            Bid::new(1, Strain::Notrump),
            120,
            short.clone() & stopper_in(overcall) & hcp(12..=14),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            short.clone() & stopper_in(overcall) & hcp(18..=19),
        )
        // A long opening minor: the jump on extras, else the simple rebid.
        .rule(
            Bid::new(3, o),
            110,
            short.clone() & len(opening, 6..) & hcp(16..=18),
        )
        .rule(Bid::new(2, o), 100, short.clone() & len(opening, 6..));
    // The other minor: natural over `1♦`, a 16+ reverse over `1♣`.
    rules = if opening == Suit::Diamonds {
        rules.rule(
            Bid::new(2, Strain::Clubs),
            110,
            short.clone() & len(Suit::Clubs, 4..),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Diamonds),
            110,
            short.clone() & len(Suit::Diamonds, 4..) & hcp(16..),
        )
    };
    rules
        // Complete the major on three when nothing above describes a minimum
        // (both bots' habit; the doubler's four make a 4-3).
        .rule(Bid::new(cheap, m), 90, len(major, 3..=3) & hcp(..=14))
        // Catch-all: rebid the opening minor.  No pass — the double is not
        // for penalty, and opener's trumps sit in front of the overcaller's.
        .rule(Bid::new(2, o), 0, hcp(0..))
}

/// The package: `P* 1m (1M) X -` for both minors and both major overcalls
pub(super) fn modern_double_answer_package() -> Package {
    Package {
        name: "modern-double-answer",
        gate: |agreements| {
            agreements.competition.modern_double_answer
                && agreements.competition.negative_double_shape == NegativeDoubleShape::Modern
        },
        entries: |_| {
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds] {
                for overcall in [Suit::Hearts, Suit::Spades] {
                    entries.extend(rows_of(
                        Pattern::after(
                            &format!(
                                "P* 1{} (1{})",
                                Strain::from(opening),
                                Strain::from(overcall)
                            ),
                            "X -",
                        ),
                        answer_modern_double(opening, overcall),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
