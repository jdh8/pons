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
//! Opener's splinters (behind
//! [`ResponseKnobs::drury_splinters`][crate::bidding::agreements::ResponseKnobs::drury_splinters],
//! default on):
//! a jump in a new suit — `3♦`, the other major at the three level, `4♣` —
//! is a singleton or void there with 16–20 support points, and outranks `4M`
//! and `4NT`.  Responder bids `4M`, or the next step (never the trump suit)
//! with no king, queen or jack in the short suit and 10+ HCP, over which opener asks
//! `4NT` on 19+ and otherwise bids `4M`.  Priced off the finished Drury
//! A/B's dumps (boards where both tables reach `1♠ - 2♣ -`): BBA's three
//! `1♠` splinters won +122 / +159 plain IMPs per 204,800 boards (none /
//! both), its `1♥` ones about zero, nearly all of it slam decisions our
//! `4NT` on 19+ support points gets wrong opposite a wasted honor.
//!
//! Not authored: BBA's natural `2♥` over `1♠` and its `2NT` / `3♣` / `3NT`
//! rungs — on the same boards its `2♥` *loses* to our ladder and the rest
//! are small and noisy.  The contested tails live in
//! [`competition`][super::super::competition]: their double is systems on,
//! their overcall gets a small natural ladder.

use super::*;
use crate::bidding::constraint::suit_hcp;
use crate::bidding::constraint::support_points as sp;

/// A passed hand's `2♣!` limit raise of a third- or fourth-seat `1M`
pub(in crate::bidding::american) const DRURY: Alert = Alert("drury");
/// Opener's `2♦!` after Drury — a full opening, says nothing about diamonds
const DRURY_RELAY: Alert = Alert("drury-relay");

/// Opener's splinter after `- - 1M - 2♣! -` — the short suit, not a strain to play
const DRURY_SPLINTER: Alert = Alert("drury-splinter");
/// Responder's step over opener's splinter — slam interest, no wasted honor
const DRURY_SPLINTER_INTEREST: Alert = Alert("drury-splinter-interest");

/// Opener's splinters after Drury: `(short suit, the jump, responder's
/// interest step)` — the step is the cheapest bid above the jump that is not
/// the trump suit
pub(in crate::bidding::american) fn drury_splinters(major: Suit) -> [(Suit, Bid, Bid); 3] {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    // `3♦`'s step is the other major (3♥ over 1♠, 3♠ over 1♥, skipping the
    // trump 3♥); the other major's is 3NT; `4♣`'s is 4♦.
    [
        (
            Suit::Diamonds,
            Bid::new(3, Strain::Diamonds),
            Bid::new(3, Strain::from(other)),
        ),
        (
            other,
            Bid::new(3, Strain::from(other)),
            Bid::new(3, Strain::Notrump),
        ),
        (
            Suit::Clubs,
            Bid::new(4, Strain::Clubs),
            Bid::new(4, Strain::Diamonds),
        ),
    ]
}

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

/// Opener's answers with the splinters on top
fn drury_answers_with_splinters(major: Suit) -> Rules {
    drury_splinters(major)
        .into_iter()
        .fold(drury_answers(major), |rules, (short, jump, _)| {
            rules
                .rule(jump, 260, len(short, ..=1) & sp(major, 16..=20))
                .alert(DRURY_SPLINTER)
        })
}

/// Responder over opener's splinter: game, or the step with no king, queen
/// or jack in the short suit and 10+ HCP
///
/// The ace is a working control opposite shortness, not waste: the first run
/// barred it too and signed off in `4M` on `A76` / `AJ652` opposite a stiff
/// while slam made (the worst boards, seed 1790805316).
fn after_splinter(major: Suit, short: Suit, step: Bid) -> Rules {
    // ponytail: 4 suit HCP also admits a bare KJ; a rank predicate would
    // exclude it at the cost of the projected box.
    let working = suit_hcp(short, ..=0) | suit_hcp(short, 4..=4);
    Rules::new()
        .rule(step, 100, working & hcp(10..))
        .alert(DRURY_SPLINTER_INTEREST)
        .rule(Bid::new(4, Strain::from(major)), 50, hcp(0..))
}

/// Opener over responder's interest step: the keycard ask on 19+, else game
fn after_interest(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, sp(major, 19..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 0, hcp(0..))
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
        entries: |agreements| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let two_major = call(2, Strain::from(major));
                let three_major = call(3, Strain::from(major));
                let four_major = call(4, Strain::from(major));
                for prefix in drury_prefixes(major) {
                    let node = |tail: &str| Pattern::node(&format!("{prefix} {tail}"));
                    let answers = if agreements.response.drury_splinters {
                        drury_answers_with_splinters(major)
                    } else {
                        drury_answers(major)
                    };
                    entries.extend(rows_of(Pattern::node(&prefix), answers));
                    if agreements.response.drury_splinters {
                        for (short, jump, step) in drury_splinters(major) {
                            let jump = format!("{prefix} {} -", Call::Bid(jump));
                            let stepped = format!("{jump} {} -", Call::Bid(step));
                            entries.extend(rows_of(
                                Pattern::node(&jump),
                                after_splinter(major, short, step),
                            ));
                            entries.extend(rows_of(Pattern::node(&stepped), after_interest(major)));
                            entries.extend(slam::rkcb_rows(&stepped, major));
                            for game in [&jump, &stepped] {
                                entries.extend(rows_of(
                                    Pattern::node(&format!("{game} {four_major} -")),
                                    settle(),
                                ));
                            }
                        }
                    }
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
