//! Reverse Drury: a passed hand's `2♣!` limit raise of a third- or
//! fourth-seat `1M`, opener's answers and responder's rungs
//!
//! Gated by [`ResponseKnobs::drury`][crate::bidding::agreements::ResponseKnobs::drury],
//! default off.  The `2♣!` itself is a row of
//! [`passed_hand_major_responses`][super::super::responses::passed_hand_major_responses],
//! the responder table keyed only under the passed-hand seats; this module
//! is everything after it, keyed under the two passed-hand prefixes
//! (`- - 1M - 2♣ -` and `- - - 1M - 2♣ -`) so the first- and second-seat
//! `1M - 2♣` stays the 2/1 game force.
//!
//! The ladder is the anchor's, read off BBA's book under our declared card
//! (`Reverse drury = 1`): `2♣` 9–11 with three-plus trumps, `2♦!` a full
//! opening, `2M` the minimum, game on 16+.  Our gauge is support points with
//! the fit known; game is 24+ combined.
//!
//! | After `- - 1M - 2♣! -` | Opener |
//! |---|---|
//! | `2M` | minimum, ≤12 (a third/fourth-seat opening starts at 11) |
//! | `2♦!` | full opening, 13–15 |
//! | `4M` | 16–18 |
//! | `4NT` | RKCB, 19+ |
//!
//! | Responder after | Call | Meaning |
//! |---|---|---|
//! | `2♦!` | `2M` | ≤10, opener passes |
//! | `2♦!` | `4M` | 11+ |
//! | `2M` | Pass | ≤11 |
//! | `2M` | `3M` | 12+; opener bids `4M` on exactly 12, passes under |
//!
//! Not authored (BBA has them, the trace has not yet priced them): opener's
//! natural strong rungs — `2♥` over `1♠` with four hearts, `2NT`, a new suit
//! at the three level, splinters.  The contested tails live in
//! [`competition`][super::super::competition]: their double is systems on,
//! their overcall gets a small natural ladder.

use super::*;
use crate::bidding::constraint::support_points as sp;

/// A passed hand's `2♣!` limit raise of a third- or fourth-seat `1M`
pub(in crate::bidding::american) const DRURY: Alert = Alert("drury");
/// Opener's `2♦!` after Drury — a full opening, says nothing about diamonds
const DRURY_RELAY: Alert = Alert("drury-relay");

/// The two passed-hand prefixes ending just before our `2♣!`'s trailing pass
fn drury_prefixes(major: Suit) -> [String; 2] {
    let trump = Strain::from(major);
    ["- -", "- - -"].map(|seat| format!("{seat} {} - 2♣ -", call(1, trump)))
}

/// Opener's answer after `- - 1M - 2♣! -`: minimum, full opening, game, or
/// the keycard ask
fn drury_answers(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // 4NT: RKCB on 19+, the rung the limit-raise ladder measured as its
        // whole win (`limit_raise.rs`).
        .rule(Bid::new(4, Strain::Notrump), 250, sp(major, 19..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, trump), 200, sp(major, 16..))
        .rule(Bid::new(2, Strain::Diamonds), 150, sp(major, 13..=15))
        .alert(DRURY_RELAY)
        .rule(Bid::new(2, trump), 100, sp(major, ..=12))
}

/// Responder after opener's `2♦!` (13–15): sign off or bid game
fn after_relay(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, trump), 100, sp(major, 11..))
        .rule(Bid::new(2, trump), 50, sp(major, ..=10))
}

/// Responder after opener's minimum `2M`: pass, or re-raise on a maximum
fn after_minimum(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        .rule(Bid::new(3, trump), 100, sp(major, 12..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener after `2M - 3M`: a full 12 bids game
fn after_reraise(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, trump), 100, sp(major, 12..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// A settled contract: pass
fn settle() -> Rules {
    Rules::new().rule(Call::Pass, 0, hcp(0..))
}

/// Reverse Drury's continuations as a row package
pub(crate) fn drury_continuations() -> Package {
    Package {
        name: "drury-continuations",
        gate: |a| a.response.drury,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let two_major = call(2, Strain::from(major));
                let three_major = call(3, Strain::from(major));
                let four_major = call(4, Strain::from(major));
                for prefix in drury_prefixes(major) {
                    let node = |tail: &str| Pattern::node(&format!("{prefix} {tail}"));
                    entries.extend(rows_of(Pattern::node(&prefix), drury_answers(major)));
                    entries.extend(slam::rkcb_rows(&prefix, major));
                    entries.extend(rows_of(node("2♦ -"), after_relay(major)));
                    entries.extend(rows_of(node(&format!("2♦ - {two_major} -")), settle()));
                    entries.extend(rows_of(node(&format!("2♦ - {four_major} -")), settle()));
                    entries.extend(rows_of(
                        node(&format!("{two_major} -")),
                        after_minimum(major),
                    ));
                    entries.extend(rows_of(
                        node(&format!("{two_major} - {three_major} -")),
                        after_reraise(major),
                    ));
                    entries.extend(rows_of(
                        node(&format!("{two_major} - {three_major} - {four_major} -")),
                        settle(),
                    ));
                    entries.extend(rows_of(node(&format!("{four_major} -")), settle()));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
