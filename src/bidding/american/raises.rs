//! Continuations after the strong raises: Jacoby 2NT and splinters
//!
//! Two further continuations ship default-on (measured, silenced-opponent
//! A/B, 200k boards/vul, plain-DD + perfect-defense both winning):
//! **major game tries** after a single raise (`1M – 2M`) — a long-suit try,
//! the general re-raise, or a keycard-asking maximum — gated by
//! [`set_major_game_tries`] (+0.042/+0.065 IMPs/board NV/vul); and
//! **limit-raise acceptance** after `1M – 3M` — accept, decline, or ask for
//! keycards — gated by [`set_limit_raise_acceptance`] (+0.002/+0.002, the
//! whole win being the keycard ask at +4.4/+5.2 IMPs/divergent).

use super::{call, insert_uncontested, slam};
use crate::bidding::constraint::{fifths, hcp, len, support_points, top_honors};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether opener's long-suit game tries after a single raise (`1M – 2M`)
    /// are authored.  Default on (measured +0.042/+0.065 IMPs/board NV/vul).
    static MAJOR_GAME_TRIES: Cell<bool> = const { Cell::new(true) };
    /// Whether opener's acceptance ladder after a limit raise (`1M – 3M`) is
    /// authored.  Default on (the win is the keycard ask: +4.4/+5.2
    /// IMPs/divergent NV/vul).
    static LIMIT_RAISE_ACCEPTANCE: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's major game tries after `1M – 2M` for books built *after*
/// this call
///
/// Read at book construction; **default on** (`--no-ns-major-game-tries` in
/// `bba-gen` for the off arm).
pub fn set_major_game_tries(on: bool) {
    MAJOR_GAME_TRIES.with(|cell| cell.set(on));
}

/// Whether major game tries are currently authored
fn major_game_tries() -> bool {
    MAJOR_GAME_TRIES.with(Cell::get)
}

/// Author opener's limit-raise acceptance ladder after `1M – 3M` for books
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

/// Opener's rebid after `1M – (P) – 2NT – (P)`: describe shape and strength
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
                2.2,
                len(side, 5..) & top_honors(side, 2..),
            );
        }
    }

    // 3-of-x for each side suit: singleton or void (shortness).
    for &side in &side_suits {
        rules = rules
            .rule(Bid::new(3, Strain::from(side)), 2.0, len(side, ..=1))
            .alert(SHORTNESS);
    }

    // No-shortness conjunct: none of the three side suits is short.
    let [a, b, c] = [side_suits[0], side_suits[1], side_suits[2]];
    let no_shortness = !len(a, ..=1) & !len(b, ..=1) & !len(c, ..=1);

    // 3M: 18+ points, no side shortness (big balanced-ish raise acceptance).
    rules = rules.rule(
        Bid::new(3, trump),
        1.5,
        support_points(18..) & no_shortness.clone(),
    );

    // 3NT: 15–17 Fifths, no side shortness (medium, balanced).
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.4,
        fifths(15.0..18.0) & no_shortness,
    );

    // 4M: minimum opener, always applies (guaranteed legal).
    rules.rule(Bid::new(4, trump), 0.5, hcp(0..))
}

/// Responder's continuation after opener's Jacoby rebid
///
/// After a forcing rebid that is not the minimum 4M, responder can drive to
/// slam with 4NT (16+) or settle in game.  After the minimum 4M, slam needs
/// substantially more (18+).
fn responder_after_jacoby(major: Suit, opener_bid: Call) -> Rules {
    let four_major = call(4, Strain::from(major));
    let four_nt = call(4, Strain::Notrump);

    if opener_bid == four_major {
        // Opener showed a minimum; slam needs extra values.
        Rules::new()
            .rule(four_nt, 1.0, support_points(18..))
            .alert(slam::RKCB)
            .rule(Call::Pass, 0.0, hcp(0..))
    } else {
        // Opener showed something descriptive; slam is in range with 16+.
        Rules::new()
            .rule(four_nt, 1.0, support_points(16..))
            .alert(slam::RKCB)
            .rule(four_major, 0.5, hcp(0..))
    }
}

// ---------------------------------------------------------------------------
// Major game tries after a single raise: 1M – 2M (set_major_game_tries)
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

/// Opener's continuation after `1M – (P) – 2M – (P)`: game tries toward a
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

    let mut rules = Rules::new()
        // 4NT: RKCB ask on a maximum.
        .rule(Bid::new(4, Strain::Notrump), 2.6, support_points(22..))
        .alert(slam::RKCB)
        // 4M: a non-asking maximum.
        .rule(Bid::new(4, trump), 2.2, support_points(19..));

    // Long-suit game tries, cheapest first: natural, no alert.
    for (suit, weight) in game_try_suits(major).into_iter().zip([1.5_f32, 1.45, 1.40]) {
        rules = rules.rule(
            Bid::new(try_level(major, suit), Strain::from(suit)),
            weight,
            len(suit, 4..) & support_points(16..=18),
        );
    }

    rules
        // 3M: the general re-raise try, deliberately below the suit tries.
        .rule(Bid::new(3, trump), 1.2, support_points(16..=18))
        // Pass: a minimum, the finite catch-all.
        .rule(Call::Pass, 0.0, hcp(0..))
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
        .rule(
            Bid::new(4, trump),
            1.0,
            support_points(8..=9) | len(try_suit, ..=1) | top_honors(try_suit, 2..),
        )
        // Decline, guaranteed legal (every try sits below 3M).
        .rule(Bid::new(3, trump), 0.5, hcp(0..))
}

/// Responder's answer to the general re-raise try: accept with a maximum,
/// passable
#[must_use]
fn responder_after_general_try(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, support_points(8..=9))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's follow-up after a long-suit try is declined: push on with
/// extras, passable
#[must_use]
fn opener_after_decline(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.0, support_points(18..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Register opener's major game tries after `1M – 2M` and their
/// continuations — a no-op unless [`major_game_tries`] is on
fn register_major_game_tries(book: &mut Trie) {
    if !major_game_tries() {
        return;
    }
    for major in [Suit::Hearts, Suit::Spades] {
        let trump = Strain::from(major);
        let raise_calls = [call(1, trump), call(2, trump)];
        insert_uncontested(book, &raise_calls, opener_after_raise(major));
        slam::install_rkcb(book, &raise_calls, major);

        for suit in game_try_suits(major) {
            let try_call = call(try_level(major, suit), Strain::from(suit));
            let try_calls = [raise_calls[0], raise_calls[1], try_call];
            insert_uncontested(book, &try_calls, responder_after_try(major, suit));

            let decline_calls = [try_calls[0], try_calls[1], try_calls[2], call(3, trump)];
            insert_uncontested(book, &decline_calls, opener_after_decline(major));
        }

        let general_try_calls = [raise_calls[0], raise_calls[1], call(3, trump)];
        insert_uncontested(book, &general_try_calls, responder_after_general_try(major));
    }
}

// ---------------------------------------------------------------------------
// Limit-raise acceptance: 1M – 3M (set_limit_raise_acceptance)
// ---------------------------------------------------------------------------

/// Opener's continuation after `1M – (P) – 3M – (P)`: accept, ask, or
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
    Rules::new()
        // 4NT: RKCB ask.
        .rule(Bid::new(4, Strain::Notrump), 1.5, support_points(19..))
        .alert(slam::RKCB)
        // 4M: accept.
        .rule(Bid::new(4, trump), 1.0, support_points(13..))
        // Pass: decline.
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Register opener's limit-raise acceptance after `1M – 3M` — a no-op
/// unless [`limit_raise_acceptance`] is on
fn register_limit_raise_acceptance(book: &mut Trie) {
    if !limit_raise_acceptance() {
        return;
    }
    for major in [Suit::Hearts, Suit::Spades] {
        let limit_calls = [call(1, Strain::from(major)), call(3, Strain::from(major))];
        insert_uncontested(book, &limit_calls, opener_after_limit_raise(major));
        slam::install_rkcb(book, &limit_calls, major);
    }
}

/// Register the Jacoby 2NT opener-rebid and responder-continuation nodes,
/// plus the opt-in major-game-try and limit-raise-acceptance ladders
///
/// For each major M, inserts:
/// - opener's rebid at `[1M, 2NT]`, and
/// - for every distinct call R in that rebid table, responder's continuation
///   at `[1M, 2NT, R]`, followed by RKCB hooks.
///
/// When [`major_game_tries`] is on, also installs opener's game tries at
/// `[1M, 2M]` and their continuations; when [`limit_raise_acceptance`] is on,
/// also installs opener's acceptance ladder at `[1M, 3M]`.  Both are no-ops
/// while their knob is off.
pub(super) fn register(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        let our_calls = [call(1, Strain::from(major)), call(2, Strain::Notrump)];
        let rebids = jacoby_rebids(major);

        // Collect distinct bid calls before moving `rebids` into the trie.
        let distinct: Vec<Call> = {
            let mut seen = std::collections::HashSet::new();
            rebids
                .rules()
                .iter()
                .filter_map(|r| seen.insert(r.call()).then_some(r.call()))
                .collect()
        };

        insert_uncontested(book, &our_calls, rebids);

        // Responder's continuation after each of opener's rebids.
        for opener_bid in distinct {
            let resp = responder_after_jacoby(major, opener_bid);
            let resp_calls: [Call; 3] = [
                call(1, Strain::from(major)),
                call(2, Strain::Notrump),
                opener_bid,
            ];
            insert_uncontested(book, &resp_calls, resp);
            slam::install_rkcb(book, &resp_calls, major);
        }
    }

    register_major_game_tries(book);
    register_limit_raise_acceptance(book);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::System;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    fn hand(s: &str) -> Hand {
        s.parse().expect("valid test hand")
    }

    /// The best call the trie makes for `hand_str` at `auction`
    fn best(trie: &Trie, auction: &[Call], hand_str: &str) -> Call {
        let logits = trie
            .classify(hand(hand_str), RelativeVulnerability::NONE, auction)
            .expect("trie covers this auction");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("logits array is never empty")
    }

    /// A fresh trie with the major game tries authored (the shipped default).
    fn game_tries_trie() -> Trie {
        set_major_game_tries(true);
        let mut trie = Trie::new();
        register(&mut trie);
        trie
    }

    /// A fresh trie with limit-raise acceptance authored (the shipped
    /// default).
    fn limit_raise_trie() -> Trie {
        set_limit_raise_acceptance(true);
        let mut trie = Trie::new();
        register(&mut trie);
        trie
    }

    /// `[1♥, P, 2♥, P]`: the single-raise node, undisturbed
    const RAISE_AUCTION: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Hearts)),
        Call::Pass,
    ];

    /// `[1♥, P, 3♥, P]`: the limit-raise node, undisturbed
    const LIMIT_RAISE_AUCTION: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Hearts)),
        Call::Pass,
        Call::Bid(Bid::new(3, Strain::Hearts)),
        Call::Pass,
    ];

    #[test]
    fn game_tries_absent_when_off() {
        set_major_game_tries(false);
        set_limit_raise_acceptance(false);
        let mut trie = Trie::new();
        register(&mut trie);
        set_major_game_tries(true); // restore the shipped defaults
        set_limit_raise_acceptance(true);
        assert!(
            trie.classify(
                hand("K52.AK974.3.AQ65"),
                RelativeVulnerability::NONE,
                RAISE_AUCTION
            )
            .is_none(),
            "major game tries must be absent with the knob off"
        );
    }

    #[test]
    fn limit_raise_acceptance_absent_when_off() {
        set_major_game_tries(false);
        set_limit_raise_acceptance(false);
        let mut trie = Trie::new();
        register(&mut trie);
        set_major_game_tries(true); // restore the shipped defaults
        set_limit_raise_acceptance(true);
        assert!(
            trie.classify(
                hand("A63.AK975.QJ3.83"),
                RelativeVulnerability::NONE,
                LIMIT_RAISE_AUCTION
            )
            .is_none(),
            "limit-raise acceptance must be absent with the knob off"
        );
    }

    #[test]
    fn opener_tries_the_long_club_suit() {
        let trie = game_tries_trie();
        // K52.AK974.3.AQ65: 16 HCP + 1 (unbalanced) = 17 points, 4 clubs.
        // The club try (3♣, wt 1.45) beats the general re-raise (3♥, wt 1.2).
        assert_eq!(
            best(&trie, RAISE_AUCTION, "K52.AK974.3.AQ65"),
            Call::Bid(Bid::new(3, Strain::Clubs)),
        );
    }

    #[test]
    fn opener_bids_game_with_a_maximum() {
        let trie = game_tries_trie();
        // AQ3.AKQ85.KJ4.92: balanced, 19 HCP/points — the non-asking maximum.
        assert_eq!(
            best(&trie, RAISE_AUCTION, "AQ3.AKQ85.KJ4.92"),
            Call::Bid(Bid::new(4, Strain::Hearts)),
        );
    }

    #[test]
    fn opener_passes_a_flat_minimum() {
        let trie = game_tries_trie();
        // KQ3.AJ854.K63.93: balanced, 13 HCP/points — below every try.
        assert_eq!(best(&trie, RAISE_AUCTION, "KQ3.AJ854.K63.93"), Call::Pass);
    }

    /// `[1♥, P, 2♥, P, 3♣, P]`: responder's answer to the club try
    fn after_club_try() -> Vec<Call> {
        RAISE_AUCTION
            .iter()
            .copied()
            .chain([Call::Bid(Bid::new(3, Strain::Clubs)), Call::Pass])
            .collect()
    }

    #[test]
    fn responder_accepts_the_try_with_a_singleton() {
        let trie = game_tries_trie();
        // 863.K64.QJ8532.7: a singleton club accepts regardless of points.
        assert_eq!(
            best(&trie, &after_club_try(), "863.K64.QJ8532.7"),
            Call::Bid(Bid::new(4, Strain::Hearts)),
        );
    }

    #[test]
    fn responder_declines_a_wasted_minimum() {
        let trie = game_tries_trie();
        // 863.K64.QJ85.972: 6 points, three small clubs — nothing to accept with.
        assert_eq!(
            best(&trie, &after_club_try(), "863.K64.QJ85.972"),
            Call::Bid(Bid::new(3, Strain::Hearts)),
        );
    }

    #[test]
    fn limit_raise_accepts_and_declines() {
        let trie = limit_raise_trie();
        // A63.AK975.Q43.83: balanced, 13 points — the measured boundary
        // (the floor's raise ladder accepts at 13+; under-bidding it lost).
        assert_eq!(
            best(&trie, LIMIT_RAISE_AUCTION, "A63.AK975.Q43.83"),
            Call::Bid(Bid::new(4, Strain::Hearts)),
        );
        // AJ6.K8532.Q63.Q7: balanced, 12 points — decline.
        assert_eq!(
            best(&trie, LIMIT_RAISE_AUCTION, "AJ6.K8532.Q63.Q7"),
            Call::Pass
        );
    }
}
