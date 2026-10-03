//! Opener's answer to the Modern negative double of a one-level overcall —
//! `1m (1♥) X -` and `1m (1♠) X -`, and the both-majors `1♣ (1♦) X -`
//!
//! Gated on `agreements.competition.modern_double_answer` (the major
//! overcalls), `agreements.competition.modern_double_both_majors` (`(1♦)`) and
//! [`NegativeDoubleShape::Modern`] (the other schools author their own
//! answers in [`negative_double`][super::negative_double]).  The double shows
//! the other major — exactly four spades over `(1♥)`, four-plus hearts over
//! `(1♠)` — or both majors 4-4+ over `(1♦)`, on 6+ HCP, unlimited.
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

/// Opener's answer after `1o (1v) X -`, partner showing the other major(s)
///
/// Over `(1♦)` (only after `1♣`) the double shows both majors, so the raise
/// picks spades on a tie and the completion on three is in the cheaper one.
/// Every rung is a fixed bid at this node, so no `min_level_is` (see
/// [`answer_cue_raise`][super::cue_raise::answer_cue_raise] for why a
/// legality-anchored rung widens readings elsewhere).
fn answer_modern_double(opening: Suit, overcall: Suit) -> Rules {
    let o = Strain::from(opening);
    let majors: &[Suit] = match overcall {
        Suit::Hearts => &[Suit::Spades],
        Suit::Spades => &[Suit::Hearts],
        _ => &[Suit::Spades, Suit::Hearts],
    };
    // ponytail: one shown major repeats its own term — a no-op conjunction.
    let short = len(majors[0], ..=3) & len(majors[majors.len() - 1], ..=3);
    let mut rules = Rules::new();
    // The fit, raised by strength (spades first on a 4-4 tie).  The minimum
    // raise is always `2M`, so a one-level `1♠` over `(1♥)` is exactly three.
    for (tie, &major) in (0..).zip(majors) {
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(4, m), 150 - tie, len(major, 4..) & points(17..))
            .rule(Bid::new(3, m), 150 - tie, len(major, 4..) & points(15..=16))
            .rule(Bid::new(2, m), 150 - tie, len(major, 4..) & points(..=14));
    }
    // A long opening minor: five clubs over `(1♦)` (BBA's `2♣`), else six.
    let long = if overcall == Suit::Diamonds { 5 } else { 6 };
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
        // The jump on extras, else the simple rebid.
        .rule(
            Bid::new(3, o),
            110,
            short.clone() & len(opening, 6..) & hcp(16..=18),
        )
        .rule(Bid::new(2, o), 100, short.clone() & len(opening, long..));
    // The other minor: natural over `1♦`, a 16+ reverse over `1♣` — unless
    // it is their suit.
    if opening == Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::Clubs),
            110,
            short.clone() & len(Suit::Clubs, 4..),
        );
    } else if overcall != Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::Diamonds),
            110,
            short.clone() & len(Suit::Diamonds, 4..) & hcp(16..),
        );
    }
    // Complete a major on three when nothing above describes a minimum
    // (both bots' habit; the doubler's four make a 4-3), the cheaper first.
    for (tie, &major) in (0..).zip(majors.iter().rev()) {
        let cheap = if major == Suit::Hearts && overcall == Suit::Spades {
            2
        } else {
            1
        };
        rules = rules.rule(
            Bid::new(cheap, Strain::from(major)),
            90 - tie,
            len(major, 3..=3) & hcp(..=14),
        );
    }
    rules
        // Catch-all: rebid the opening minor.  No pass — the double is not
        // for penalty, and opener's trumps sit in front of the overcaller's.
        .rule(Bid::new(2, o), 0, hcp(0..))
}

/// The both-majors doubler's second turn after `1♣ (1♦) X - {answer} -`
///
/// Only the four common answers: the minimum raise `2M`, the major on three,
/// `1NT`, and `2♣`; the rest stay the floor's.
fn both_majors_doubler_rebid(answer: Bid) -> Rules {
    let d = Suit::Diamonds;
    let nt = |level| Bid::new(level, Strain::Notrump);
    match answer.strain {
        // `2M`: four trumps, 12–14 points — game on 12, invite on 10–11.
        Strain::Hearts | Strain::Spades if answer.level.get() == 2 => Rules::new()
            .rule(Bid::new(4, answer.strain), 100, points(12..))
            .rule(Bid::new(3, answer.strain), 100, points(10..=11))
            .rule(Call::Pass, 20, points(..=9)),
        // The major on three, a minimum with no diamond stopper: a 4-3 fit.
        Strain::Hearts | Strain::Spades => Rules::new()
            .rule(nt(3), 110, stopper_in(d) & hcp(13..))
            .rule(Bid::new(4, answer.strain), 100, hcp(13..))
            .rule(nt(2), 110, stopper_in(d) & hcp(11..=12))
            .rule(Bid::new(2, answer.strain), 100, hcp(11..=12))
            .rule(Call::Pass, 20, hcp(..=10)),
        // `1NT` 12–14 with diamonds stopped.
        Strain::Notrump => Rules::new()
            .rule(nt(3), 100, hcp(13..))
            .rule(nt(2), 100, hcp(11..=12))
            .rule(Call::Pass, 20, hcp(..=10)),
        // `2♣`: five clubs, or the catch-all; diamonds unstopped.
        _ => Rules::new()
            .rule(nt(3), 100, stopper_in(d) & hcp(13..))
            .rule(nt(2), 100, stopper_in(d) & hcp(11..=12))
            .rule(
                Bid::new(3, Strain::Clubs),
                90,
                len(Suit::Clubs, 3..) & hcp(11..=12),
            )
            // Game values with diamonds open: the club game on a singleton,
            // else `3NT` on hope.
            .rule(
                Bid::new(5, Strain::Clubs),
                80,
                len(Suit::Clubs, 3..) & len(d, ..=1) & hcp(13..),
            )
            .rule(nt(3), 60, hcp(13..))
            .rule(Call::Pass, 20, hcp(0..)),
    }
}

/// The package: `P* 1m (1M) X -` for both minors and both major overcalls,
/// and `P* 1♣ (1♦) X -` with the doubler's second turn behind its own knob
pub(super) fn modern_double_answer_package() -> Package {
    Package {
        name: "modern-double-answer",
        gate: |agreements| {
            let c = &agreements.competition;
            (c.modern_double_answer || c.modern_double_both_majors)
                && c.negative_double_shape == NegativeDoubleShape::Modern
        },
        entries: |agreements| {
            let c = &agreements.competition;
            let mut pairs = Vec::new();
            if c.modern_double_answer {
                for opening in [Suit::Clubs, Suit::Diamonds] {
                    pairs.extend([(opening, Suit::Hearts), (opening, Suit::Spades)]);
                }
            }
            if c.modern_double_both_majors {
                pairs.push((Suit::Clubs, Suit::Diamonds));
            }
            let mut entries = Vec::new();
            for (opening, overcall) in pairs {
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
            if c.modern_double_both_majors {
                let answers = [
                    Bid::new(1, Strain::Hearts),
                    Bid::new(1, Strain::Spades),
                    Bid::new(1, Strain::Notrump),
                    Bid::new(2, Strain::Clubs),
                    Bid::new(2, Strain::Hearts),
                    Bid::new(2, Strain::Spades),
                ];
                for answer in answers {
                    entries.extend(rows_of(
                        Pattern::after("P* 1♣ (1♦)", &format!("X - {answer} -")),
                        both_majors_doubler_rebid(answer),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
