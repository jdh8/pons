//! Continuations after the strong raises: Jacoby 2NT and splinters
//!
//! Two further continuations ship default-on (measured, silenced-opponent
//! A/B, 200k boards/vul, plain-DD + perfect-defense both winning):
//! **major game tries** after a single raise (`1M - 2M`) — a long-suit try,
//! the general re-raise, or a keycard-asking maximum — gated by
//! [`set_major_game_tries`] (+0.042/+0.065 IMPs/board NV/vul); and
//! **limit-raise acceptance** after `1M - 3M` — accept, decline, or ask for
//! keycards — gated by [`set_limit_raise_acceptance`] (+0.002/+0.002, the
//! whole win being the keycard ask at +4.4/+5.2 IMPs/divergent).

use super::{call, slam};
use crate::bidding::constraint::{fifths, hcp, len, support_points, top_honors};
use crate::bidding::rows::{Package, Pattern, compile_into, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether opener's long-suit game tries after a single raise (`1M - 2M`)
    /// are authored.  Default on (measured +0.042/+0.065 IMPs/board NV/vul).
    static MAJOR_GAME_TRIES: Cell<bool> = const { Cell::new(true) };
    /// Whether opener's acceptance ladder after a limit raise (`1M - 3M`) is
    /// authored.  Default on (the win is the keycard ask: +4.4/+5.2
    /// IMPs/divergent NV/vul).
    static LIMIT_RAISE_ACCEPTANCE: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's major game tries after `1M - 2M` for books built *after*
/// this call
///
/// Read at book construction; **default on** (`--no-ns-major-game-tries` in
/// `bba-gen` for the off arm).
pub fn set_major_game_tries(on: bool) {
    MAJOR_GAME_TRIES.with(|cell| cell.set(on));
}

/// Whether major game tries are currently authored
pub(crate) fn major_game_tries() -> bool {
    MAJOR_GAME_TRIES.with(Cell::get)
}

/// Author opener's limit-raise acceptance ladder after `1M - 3M` for books
/// built *after* this call
///
/// Read at book construction; **default on** (`--no-ns-limit-raise-acceptance`
/// in `bba-gen` for the off arm).
pub fn set_limit_raise_acceptance(on: bool) {
    LIMIT_RAISE_ACCEPTANCE.with(|cell| cell.set(on));
}

/// Whether limit-raise acceptance is currently authored
fn limit_raise_acceptance() -> bool {
    LIMIT_RAISE_ACCEPTANCE.with(Cell::get)
}

/// Shortness — opener's `3`-of-a-side-suit singleton/void show after Jacoby 2NT
const SHORTNESS: Alert = Alert("shortness");

/// Opener's rebid after `1M - 2NT -`: describe shape and strength
///
/// Jacoby 2NT is a game-forcing raise promising four-card support and 13+ HCP,
/// so opener can safely describe at a high level.  This node is **forcing** —
/// there is no pass rule.
///
/// | Call | Meaning |
/// |---|---|
/// | 4♣/4♦ (below major) | Good five-card second suit (two of top three honors) |
/// | 3♣/3♦/3♥ (side suit shortness) | Singleton or void |
/// | 3M | 18+ balanced-ish acceptance (no side shortness) |
/// | 3NT | 15–17 balanced, no side shortness |
/// | 4M | Minimum opener (12–14) |
fn jacoby_rebids(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let side_suits: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter(|&s| s != major)
        .collect();

    let mut rules = Rules::new();

    // 4-of-x for each side suit x with Strain::from(x) < trump:
    // a good five-card second suit with two of the top three honors.
    for &side in &side_suits {
        if Strain::from(side) < trump {
            rules = rules.rule(
                Bid::new(4, Strain::from(side)),
                220,
                len(side, 5..) & top_honors(side, 2..),
            );
        }
    }

    // 3-of-x for each side suit: singleton or void (shortness).
    for &side in &side_suits {
        rules = rules
            .rule(Bid::new(3, Strain::from(side)), 200, len(side, ..=1))
            .alert(SHORTNESS);
    }

    // No-shortness conjunct: none of the three side suits is short.
    let [a, b, c] = [side_suits[0], side_suits[1], side_suits[2]];
    let no_shortness = !len(a, ..=1) & !len(b, ..=1) & !len(c, ..=1);

    // 3M: 18+ points, no side shortness (big balanced-ish raise acceptance).
    // Opener's seat: the trump is the own five-card major, +5.
    rules = rules.rule(
        Bid::new(3, trump),
        150,
        support_points(major, 18..) & no_shortness.clone(),
    );

    // 3NT: 15–17 Fifths, no side shortness (medium, balanced).
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        140,
        fifths(15.0..18.0) & no_shortness,
    );

    // 4M: minimum opener, always applies (guaranteed legal).
    rules.rule(Bid::new(4, trump), 50, hcp(0..))
}

/// Responder's continuation after opener's Jacoby rebid
///
/// After a forcing rebid that is not the minimum 4M, responder can drive to
/// slam with 4NT (16+) or settle in game.  After the minimum 4M, slam needs
/// substantially more (18+).
fn responder_after_jacoby(major: Suit, opener_bid: Call) -> Rules {
    let four_major = call(4, Strain::from(major));
    let four_nt = call(4, Strain::Notrump);

    // Responder's seat: Jacoby promised four-card support, +4.
    if opener_bid == four_major {
        // Opener showed a minimum; slam needs extra values.
        Rules::new()
            .rule(four_nt, 100, support_points(major, 18..))
            .alert(slam::RKCB)
            .rule(Call::Pass, 0, hcp(0..))
    } else {
        // Opener showed something descriptive; slam is in range with 16+.
        Rules::new()
            .rule(four_nt, 100, support_points(major, 16..))
            .alert(slam::RKCB)
            .rule(four_major, 50, hcp(0..))
    }
}

// ---------------------------------------------------------------------------
// Major game tries after a single raise: 1M - 2M (set_major_game_tries)
// ---------------------------------------------------------------------------

/// The level of the cheapest available call in `suit` over `2` of `major`
///
/// A suit ranked above the major is still open at the two level; a suit
/// ranked below it must jump to the three level to be bid at all.
fn try_level(major: Suit, suit: Suit) -> u8 {
    if Strain::from(suit) > Strain::from(major) {
        2
    } else {
        3
    }
}

/// The three side suits available as a long-suit game try, cheapest first
///
/// At most one suit outranks the major (the other major, over `1♥`), so the
/// order is: that suit at the two level, if any, then the rest at the three
/// level in ascending rank.  Hearts: `[♠, ♣, ♦]`; spades: `[♣, ♦, ♥]`.
fn game_try_suits(major: Suit) -> Vec<Suit> {
    let major_strain = Strain::from(major);
    let mut above = Vec::new();
    let mut below = Vec::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == major {
            continue;
        }
        if Strain::from(suit) > major_strain {
            above.push(suit);
        } else {
            below.push(suit);
        }
    }
    above.into_iter().chain(below).collect()
}

/// Opener's continuation after `1M - 2M -`: game tries toward a
/// non-forcing raise
///
/// Responder's single raise promises three-plus trumps and 6–9 points, so
/// opener needs real extras to move: a maximum drives to game outright (or
/// asks for keycards on a huge hand), 16–18 explores with a long-suit game
/// try or the general re-raise, and anything below settles in the part score.
///
/// | Call | Meaning |
/// |---|---|
/// | 4NT | RKCB ask (22+) |
/// | 4M | Non-asking maximum (19+) |
/// | 2♠/3♣/3♦ (hearts) or 3♣/3♦/3♥ (spades) | Long-suit game try (16–18, 4+ in the suit) |
/// | 3M | The general re-raise try (16–18), below every suit try in weight |
/// | Pass | Minimum, nothing more to show |
#[must_use]
fn opener_after_raise(major: Suit) -> Rules {
    let trump = Strain::from(major);

    // Opener's seat throughout: the trump is the own five-card major, +5.
    let mut rules = Rules::new()
        // 4NT: RKCB ask on a maximum.
        .rule(
            Bid::new(4, Strain::Notrump),
            260,
            support_points(major, 22..),
        )
        .alert(slam::RKCB)
        // 4M: a non-asking maximum.
        .rule(Bid::new(4, trump), 220, support_points(major, 19..));

    // Long-suit game tries, cheapest first: natural, no alert.
    for (suit, weight) in game_try_suits(major).into_iter().zip([150_i16, 145, 140]) {
        rules = rules.rule(
            Bid::new(try_level(major, suit), Strain::from(suit)),
            weight,
            len(suit, 4..) & support_points(major, 16..=18),
        );
    }

    rules
        // 3M: the general re-raise try, deliberately below the suit tries.
        .rule(Bid::new(3, trump), 120, support_points(major, 16..=18))
        // Pass: a minimum, the finite catch-all.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's answer to a long-suit game try: accept with a maximum, a
/// shortage, or two top honors in the tried suit — decline otherwise
///
/// Forcing by omission below `3M`: every try sits under it, so the decline is
/// always legal.
#[must_use]
fn responder_after_try(major: Suit, try_suit: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // Accept: a maximum single raise, or good shape in the try suit.
        // Responder's seat: the single raise promised 3+ trumps, +3.
        .rule(
            Bid::new(4, trump),
            100,
            support_points(major, 8..=9) | len(try_suit, ..=1) | top_honors(try_suit, 2..),
        )
        // Decline, guaranteed legal (every try sits below 3M).
        .rule(Bid::new(3, trump), 50, hcp(0..))
}

/// Responder's answer to the general re-raise try: accept with a maximum,
/// passable
#[must_use]
fn responder_after_general_try(major: Suit) -> Rules {
    Rules::new()
        // Responder's seat: the single raise promised 3+ trumps, +3.
        .rule(
            Bid::new(4, Strain::from(major)),
            100,
            support_points(major, 8..=9),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's follow-up after a long-suit try is declined: push on with
/// extras, passable
#[must_use]
fn opener_after_decline(major: Suit) -> Rules {
    Rules::new()
        // Opener's seat: the trump is the own five-card major, +5.
        .rule(
            Bid::new(4, Strain::from(major)),
            100,
            support_points(major, 18..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Limit-raise acceptance: 1M - 3M (set_limit_raise_acceptance)
// ---------------------------------------------------------------------------

/// Opener's continuation after `1M - 3M -`: accept, ask, or
/// decline the limit raise
///
/// | Call | Meaning |
/// |---|---|
/// | 4NT | RKCB ask (19+) |
/// | 4M | Accept (13+) |
/// | Pass | Decline |
///
/// The accept sits at 13, not the textbook 14: the instinct floor's
/// raise-partner ladder already accepts at 13+ (`instinct.rs`, the
/// `(4, 13)` raise rung), and the 14/15-threshold experiments — which only
/// *under*-bid relative to that baseline — lost −4.6/−5.2 IMPs per divergent
/// board vulnerable (probe-limit-raise).  With a nine-card fit known, DD
/// prices the 23-combined game as a clear bid, so the authored value of this
/// node is the keycard ask (+5.2 IMPs/divergent), not the accept threshold.
#[must_use]
fn opener_after_limit_raise(major: Suit) -> Rules {
    let trump = Strain::from(major);
    // Opener's seat: the trump is the own five-card major, +5.
    Rules::new()
        // 4NT: RKCB ask.
        .rule(
            Bid::new(4, Strain::Notrump),
            150,
            support_points(major, 19..),
        )
        .alert(slam::RKCB)
        // 4M: accept.
        .rule(Bid::new(4, trump), 100, support_points(major, 13..))
        // Pass: decline.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Jacoby 2NT opener rebids, responder continuations and RKCB answer trees
pub(super) fn jacoby_continuations() -> Package {
    Package {
        name: "jacoby-two-notrump-continuations",
        gate: || true,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let prefix = format!("P* {} - 2NT -", call(1, Strain::from(major)),);
                let rebids = jacoby_rebids(major);

                // Derive continuation keys from the live source table while
                // preserving first-declaration order.  The HashSet is only
                // the membership test; iterating the rules carries the order.
                let distinct: Vec<Call> = {
                    let mut seen = std::collections::HashSet::new();
                    rebids
                        .rules()
                        .iter()
                        .filter_map(|rule| seen.insert(rule.call()).then_some(rule.call()))
                        .collect()
                };

                entries.extend(rows_of(Pattern::node(&prefix), rebids));
                for opener_bid in distinct {
                    let response = format!("{prefix} {opener_bid} -");
                    entries.extend(rows_of(
                        Pattern::node(&response),
                        responder_after_jacoby(major, opener_bid),
                    ));
                    entries.extend(slam::rkcb_rows(&response, major));
                }
            }
            entries
        },
    }
}

/// Major game tries after `1M - 2M`, with every answer and RKCB subtree
pub(super) fn major_game_try_continuations() -> Package {
    Package {
        name: "major-game-try-continuations",
        gate: major_game_tries,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let trump = Strain::from(major);
                let prefix = format!("P* {} - {} -", call(1, trump), call(2, trump));
                entries.extend(rows_of(Pattern::node(&prefix), opener_after_raise(major)));
                entries.extend(slam::rkcb_rows(&prefix, major));

                for suit in game_try_suits(major) {
                    let try_call = call(try_level(major, suit), Strain::from(suit));
                    let tried = format!("{prefix} {try_call} -");
                    entries.extend(rows_of(
                        Pattern::node(&tried),
                        responder_after_try(major, suit),
                    ));

                    let declined = format!("{tried} {} -", call(3, trump));
                    entries.extend(rows_of(
                        Pattern::node(&declined),
                        opener_after_decline(major),
                    ));
                }

                let general = format!("{prefix} {} -", call(3, trump));
                entries.extend(rows_of(
                    Pattern::node(&general),
                    responder_after_general_try(major),
                ));
            }
            entries
        },
    }
}

/// Limit-raise acceptance after `1M - 3M`, including its RKCB subtree
pub(super) fn limit_raise_acceptance_continuations() -> Package {
    Package {
        name: "limit-raise-acceptance-continuations",
        gate: limit_raise_acceptance,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let trump = Strain::from(major);
                let prefix = format!("P* {} - {} -", call(1, trump), call(3, trump));
                entries.extend(rows_of(
                    Pattern::node(&prefix),
                    opener_after_limit_raise(major),
                ));
                entries.extend(slam::rkcb_rows(&prefix, major));
            }
            entries
        },
    }
}

/// Register all strong-raise continuations into the constructive book
pub(super) fn register(book: &mut Trie) {
    compile_into(
        book,
        &[
            jacoby_continuations(),
            major_game_try_continuations(),
            limit_raise_acceptance_continuations(),
        ],
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
