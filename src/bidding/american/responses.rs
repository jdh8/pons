//! Responses to one-level suit openings in the 2/1 game-forcing system
//!
//! This module is the **index** for first responses and four child agreements:
//!
//! | Module | Agreement | Knob |
//! | --- | --- | --- |
//! | [`two_over_one`] | major-suit 2/1 fit split, entry gate, and suit-length treatments | [`set_two_over_one_fit`], [`set_two_over_one_gate`], [`set_two_over_one_natural_lengths`], [`set_two_over_one_major_discount`], [`set_two_over_one_heart_light`] |
//! | [`longer_major`] | longer-major selection and the up-the-line minor-opening tree | [`set_longer_major_response`], [`set_up_the_line`] |
//! | [`choice_of_games`] | `1M - 3NT` choice of games | [`set_major_choice_of_games`] |
//! | [`inverted_minor`] | inverted-minor continuation tree | always on |

use super::super::Alert;
use super::super::Rules;
use super::super::Trie;
use super::super::constraint::{
    balanced, envelope_union_upgrade, hcp, len, points, support, support_points,
};
use crate::bidding::agreements::Agreements;
use crate::bidding::inference::{Envelope, EnvelopeUnion, Range};
use crate::bidding::rows::{Package, Pattern, compile_into, expand, rows_of};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

mod choice_of_games;
mod inverted_minor;
mod longer_major;
mod two_over_one;

use choice_of_games::with_choice_of_games;
use inverted_minor::inverted_minor_rows;
use longer_major::{with_major_selection, with_up_the_line};
use two_over_one::{two_over_one_gate, with_two_over_one};

pub(super) use choice_of_games::choice_of_games_continuations;
pub use choice_of_games::set_major_choice_of_games;
pub(super) use inverted_minor::minor_keycard_continuations;
pub(crate) use longer_major::{longer_major_response, up_the_line};
pub use longer_major::{set_longer_major_response, set_up_the_line};
pub use two_over_one::TwoOverOneGate;
pub use two_over_one::{
    set_two_over_one_fit, set_two_over_one_gate, set_two_over_one_heart_light,
    set_two_over_one_major_discount, set_two_over_one_natural_lengths,
};

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
        // Jacoby 2NT: game-forcing raise with four-card support.  The
        // `support` leg replays under the reader's seat and re-targets to
        // whatever suit partner showed *last* — a floor-RKCB 5♦ answer moved
        // this box to diamonds, erased the spade support, and stranded the
        // asker in 5♦ (chop F2b's worst family) — so knob-on the box pins the
        // node's own major statically.  Legacy carries eval/describe and the
        // knob-off reading unchanged.
        .rule(
            Bid::new(2, Strain::Notrump),
            300,
            envelope_union_upgrade(
                support(4..) & support_points(major, 13..),
                jacoby_box(major),
            ),
        )
        .alert(JACOBY_2NT)
        // Limit raise: four-card support, 10–12 points.
        .rule(
            Bid::new(3, trump),
            200,
            support(4..) & support_points(major, 10..=12),
        )
        // Weak jump to game: lots of trumps, few points.  Left on legacy
        // `points`: this preempt's ceiling gates obstruction, and revaluing
        // shortness here would demote shapely-weak hands into a constructive
        // single raise — a DD-flattering de-preemption (see the roadmap).
        .rule(Bid::new(4, trump), 160, support(5..) & points(..6))
        // Single raise.
        .rule(
            Bid::new(2, trump),
            150,
            support(3..) & support_points(major, 6..=9),
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
            50,
            hcp(6..=(two_over_one_gate().hcp_floor().max(13) - 1)),
        )
        .rule(Call::Pass, 0, hcp(..6));

    // 1♠ over 1♥: a new suit at the one level, preferred to a single raise.
    if major == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            170,
            len(Suit::Spades, 4..) & points(6..) & !support(4..),
        );
    }

    rules = with_choice_of_games(rules, major);

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
                280,
                support(4..) & support_points(major, 10..=13) & len(x, ..=1),
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
            .rule(Bid::new(level, strain), 100, len(x, 6..) & points(2..=5))
            .alert(WEAK_JUMP_SHIFT);
    }

    rules = with_two_over_one(rules, major);
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

/// The exact one-box knob-on reading of Jacoby 2NT: four-card support for the
/// node's own `major` (statically pinned — immune to the reader-seat
/// re-targeting of the `support` leg) with game-forcing support points.
/// Eval-equivalence to the legacy composite is pinned by
/// `jacoby_union_matches_composite`.
fn jacoby_box(major: Suit) -> EnvelopeUnion {
    let mut env = Envelope::unknown();
    env.lengths[major as usize] = Range::new(4, Range::FULL_LENGTH.max);
    env.narrow_support_points(major, Range::new(13, Range::FULL_POINTS.max));
    EnvelopeUnion::from(env)
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
/// Four-card majors up the line, a 2/1 game force (`1♦ - 2♣`), the notrump
/// ladder when no major fits, and inverted minor raises promising five-card
/// support (strong 2-of-minor forcing, weak preemptive 3-of-minor).
#[must_use]
pub fn minor_responses(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    let mut rules = Rules::new();
    rules = with_major_selection(rules);
    rules = with_up_the_line(rules, minor);
    rules = rules
        // Notrump ladder without a four-card major (3NT open-ended for game-plus).
        .rule(
            Bid::new(3, Strain::Notrump),
            100,
            hcp(13..) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            hcp(11..=12) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            50,
            hcp(6..=10) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // Inverted minor raises (five-card support required since opener may hold only three).
        // Strong raise: forcing one round — no majors, 10+ points.
        .rule(
            Bid::new(2, trump),
            125,
            support(5..)
                & support_points(minor, 10..)
                & len(Suit::Hearts, ..4)
                & len(Suit::Spades, ..4),
        )
        .alert(INVERTED_MINOR)
        // Weak preemptive raise.  `support_points` here is behaviour-neutral —
        // the strong-raise floor above dominates every hand it could promote —
        // so it rides along to keep every fit-known raise gate on one scale.
        .rule(
            Bid::new(3, trump),
            110,
            support(5..) & support_points(minor, ..=9),
        )
        .alert(INVERTED_MINOR)
        .rule(Call::Pass, 0, hcp(..6));

    // Weak jump shifts: 2♥ and 2♠ over either minor.
    for x in [Suit::Hearts, Suit::Spades] {
        rules = rules
            .rule(
                Bid::new(2, Strain::from(x)),
                100,
                len(x, 6..) & points(2..=5),
            )
            .alert(WEAK_JUMP_SHIFT);
    }

    // 2/1 game force: 1♦ - 2♣ (clubs are cheaper than diamonds).
    if minor == Suit::Diamonds {
        rules = rules
            .rule(
                Bid::new(2, Strain::Clubs),
                130,
                len(Suit::Clubs, 4..)
                    & points(13..)
                    & len(Suit::Hearts, ..4)
                    & len(Suit::Spades, ..4),
            )
            .alert(GAME_FORCE);
    }
    rules
}

/// Opener's rebid after responder splinters in support of `major`
fn opener_after_splinter(major: Suit) -> Rules {
    Rules::new()
        // Opener's seat: the trump is the own five-card major, +5.
        .rule(
            Bid::new(4, Strain::Notrump),
            100,
            support_points(major, 16..),
        )
        .alert(super::slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 50, hcp(0..))
}

/// First responses, splinter continuations, and the ungated inverted-minor tree
pub(super) fn package() -> Package {
    Package {
        name: "suit-opening-responses",
        gate: |_| true,
        entries: |_| {
            let mut entries = expand("P* 1M -", |_| true, |b| major_responses(b.suit('M')));
            entries.extend(expand(
                "P* 1m -",
                |_| true,
                |b| minor_responses(b.suit('m')),
            ));

            // Splinter continuations and their major-suit RKCB answer trees.
            for major in [Suit::Hearts, Suit::Spades] {
                let splinter_suits: &[Suit] = if major == Suit::Hearts {
                    &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
                } else {
                    &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
                };
                for &shortness in splinter_suits {
                    let (level, strain) = splinter_bid(major, shortness);
                    let prefix = format!(
                        "P* {} - {} -",
                        super::call(1, Strain::from(major)),
                        super::call(level, strain),
                    );
                    entries.extend(rows_of(
                        Pattern::node(&prefix),
                        opener_after_splinter(major),
                    ));
                    entries.extend(super::slam::rkcb_rows(&prefix, major));
                }
            }

            entries.extend(inverted_minor_rows());
            entries
        },
    }
}

/// Register the first responses and their response-level continuations
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(
        book,
        agreements,
        &[
            package(),
            choice_of_games_continuations(),
            minor_keycard_continuations(),
        ],
    );
}

#[cfg(test)]
mod tests;
