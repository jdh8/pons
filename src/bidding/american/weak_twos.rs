//! Responses to weak-two openings in the 2/1 game-forcing system
//!
//! Covers three auctions for each weak-two suit M ∈ {♦, ♥, ♠}:
//!
//! 1. **Responder's first bid** (`2M–?`): Ogust 2NT, preemptive raises, and a
//!    one-round-forcing new suit.
//! 2. **Opener's Ogust answers** (`2M–2NT–?`): a conventional five-rung ladder
//!    encoding suit quality and points range.
//! 3. **Asker's Ogust continuations** (`2M–2NT–<answer>–?`): sign-off or game
//!    based on what opener revealed.
//! 4. **Opener's reply to a forcing new suit** (`2M–<new suit>–?`): raise or
//!    rebid.
//!
//! # Raises are to play (RONF)
//!
//! Direct raises of the weak two are non-forcing by design: `2M–3M` and
//! `2M–4M` are both pre-emptive, not invitational.  Hands with game interest
//! and fit use Ogust (2NT) instead.
//!
//! # Forcing by omission
//!
//! Ogust answers carry no [`Pass`][contract_bridge::auction::Call::Pass] rule
//! — the auction is forced after 2NT.  Asker's continuations after a *max*
//! Ogust answer also omit pass (opener showed maximum values; game is
//! obligatory).

use super::{call, insert_uncontested};
use crate::bidding::constraint::{hcp, len, points, support, top_honors};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

// ---------------------------------------------------------------------------
// First response to 2M
// ---------------------------------------------------------------------------

/// Responder's first-round options over a weak-two opening in `our`
///
/// Priorities, highest first:
///
/// - **2NT** (Ogust ask, weight 2.0): at least two-card support and opening
///   values; asks opener to describe suit quality and HCP range on the
///   five-rung ladder.
/// - **Game raise** (weight 1.3): four-plus-card support, pre-emptive.  Uses
///   4♦ for M = ♦.
/// - **Forcing new suit** (weight 1.5): five-card suit with two of the top
///   three honors and opening values; one-round force.  Higher-ranking suits
///   are bid at the two level; lower-ranking suits at the three level.
/// - **Simple raise** (weight 1.2): three-plus-card support, preemptive.
/// - **Pass** (weight 0.0): catch-all.
#[must_use]
fn responses(our: Suit) -> Rules {
    let trump = Strain::from(our);
    let mut rules = Rules::new()
        // Ogust 2NT: at least two-card support and opening values.
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            points(14..) & support(2..),
        )
        // Pre-emptive game raise.
        .rule(Bid::new(4, trump), 1.3, support(4..))
        // Pre-emptive simple raise (RONF — raise is to play, not invitational).
        .rule(Bid::new(3, trump), 1.2, support(3..))
        // Pass: the catch-all.
        .rule(Call::Pass, 0.0, hcp(0..));

    // Forcing new suits: each suit other than `our`, with a natural two-level
    // bid (if the suit ranks higher) or three-level bid (if lower).
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == our {
            continue;
        }
        let level: u8 = if Strain::from(x) > trump { 2 } else { 3 };
        rules = rules.rule(
            Bid::new(level, Strain::from(x)),
            1.5,
            len(x, 5..) & top_honors(x, 2..) & points(14..),
        );
    }
    rules
}

// ---------------------------------------------------------------------------
// Opener's Ogust answers
// ---------------------------------------------------------------------------

/// Opener's answers to the Ogust 2NT inquiry after a weak-two in `our`
///
/// The five-rung ladder encodes points range and suit quality regardless of
/// which suit was opened:
///
/// | Call | Meaning | Weight |
/// |------|---------|--------|
/// | 3NT  | Solid suit (A-K-Q) | 1.5 |
/// | 3♣   | Minimum points, bad suit (< 2 top honors) | 1.0 |
/// | 3♦   | Minimum points, good suit (≥ 2 top honors) | 1.0 |
/// | 3♥   | Maximum points, bad suit | 1.0 |
/// | 3♠   | Maximum points, good suit | 1.0 |
///
/// A safety fallback of 3♣ at weight 0.2 guarantees a legal call when none
/// of the crisp rules fire (which should not happen with a legitimate weak
/// two, but is required by the "forcing by omission" model).
#[must_use]
fn ogust_answers(our: Suit) -> Rules {
    Rules::new()
        // Solid six-card suit (A-K-Q present): bid 3NT.
        .rule(Bid::new(3, Strain::Notrump), 1.5, top_honors(our, 3..))
        // Minimum values (5–7 points), bad suit (fewer than two of A/K/Q).
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            points(5..=7) & !top_honors(our, 2..),
        )
        // Minimum values, good suit (two or more of A/K/Q).
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.0,
            points(5..=7) & top_honors(our, 2..),
        )
        // Maximum values (8–10 points), bad suit.
        .rule(
            Bid::new(3, Strain::Hearts),
            1.0,
            points(8..=10) & !top_honors(our, 2..),
        )
        // Maximum values, good suit.
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            points(8..=10) & top_honors(our, 2..),
        )
        // Safety fallback: guarantees a legal response for any legitimate
        // weak-two hand even when the crisp constraints leave a gap.
        .rule(Bid::new(3, Strain::Clubs), 0.2, hcp(0..))
}

// ---------------------------------------------------------------------------
// Asker's continuations after Ogust answers (majors)
// ---------------------------------------------------------------------------

/// Asker's continuation after a *minimum* Ogust answer (3♣ or 3♦) to a
/// major-suit weak two in `our`
///
/// Opener showed a sub-maximum hand.  With 17+ HCP the asker can still make
/// game; otherwise a sign-off in three-of-the-major ends the auction.
///
/// The 3M sign-off may be dead if the Ogust answer already passed 3M; the
/// "forcing by omission" model leaves it dead (scoring −∞) rather than
/// escalating to 4M.
#[must_use]
fn asker_after_min_major(our: Suit) -> Rules {
    let trump = Strain::from(our);
    Rules::new()
        // With game-going values, push to game anyway.
        .rule(Bid::new(4, trump), 1.0, points(17..))
        // Sign-off at three: opener was minimum, asker is short of game force.
        .rule(Bid::new(3, trump), 0.5, hcp(0..))
}

/// Asker's continuation after a *maximum* Ogust answer (3♥ or 3♠) to a
/// major-suit weak two in `our`
///
/// Opener showed maximum weak-two values.  Since the asker held at least 14
/// HCP to invoke Ogust, the combined count is enough for game — bid it.
#[must_use]
fn asker_after_max_major(our: Suit) -> Rules {
    let trump = Strain::from(our);
    // Forcing node (no pass): opener showed maximum, game is in range.
    Rules::new().rule(Bid::new(4, trump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Asker's continuations after Ogust answers (diamonds)
// ---------------------------------------------------------------------------

/// Asker's continuation after the minimum-bad (3♣) Ogust answer to 2♦
///
/// 3NT on a power hand; otherwise 3♦ as sign-off.
#[must_use]
fn asker_after_diamonds_min_bad() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(17..))
        .rule(Bid::new(3, Strain::Diamonds), 0.5, hcp(0..))
}

/// Asker's continuation after the minimum-good (3♦) Ogust answer to 2♦
///
/// 3NT on a power hand; otherwise pass to play 3♦.
#[must_use]
fn asker_after_diamonds_min_good() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(17..))
        // Pass: 3♦ is already on the table; sign off there.
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Asker's continuation after a *maximum* Ogust answer (3♥ or 3♠) to 2♦
///
/// Opener showed maximum weak-two values.  With game-going HCP, bid 5♦;
/// otherwise 3NT is the practical game in a safe major suit.
#[must_use]
fn asker_after_diamonds_max() -> Rules {
    // Forcing node: game is in range given combined values.
    Rules::new()
        .rule(Bid::new(5, Strain::Diamonds), 1.0, points(17..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener's reply to a forcing new suit
// ---------------------------------------------------------------------------

/// Opener's reply when responder bids a forcing new suit `x` over weak two `our`
///
/// The response was at `response_level`.  Opener either raises `x` at the
/// cheapest available level (`response_level + 1`) with three-card support,
/// or rebids `our` at the cheapest legal level.
///
/// The rebid level for `our` is 3 when `3M > <response bid>` (i.e. the suit
/// ranks high enough to be legal), otherwise 4.
#[must_use]
fn reply_to_new_suit(our: Suit, x: Suit, response_level: u8) -> Rules {
    let raise_level = response_level + 1;
    let rebid_level: u8 =
        if Bid::new(3, Strain::from(our)) > Bid::new(response_level, Strain::from(x)) {
            3
        } else {
            4
        };

    Rules::new()
        // Raise partner's suit with adequate support.
        .rule(Bid::new(raise_level, Strain::from(x)), 1.0, support(3..))
        // Rebid our suit as the fallback description.
        .rule(Bid::new(rebid_level, Strain::from(our)), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all weak-two response machinery into the constructive book
///
/// For each weak-two suit M ∈ {♦, ♥, ♠}:
///
/// - First responses at `[2M]`
/// - Ogust answers at `[2M, 2NT]`
/// - Asker's Ogust continuations at `[2M, 2NT, <answer>]`
/// - Opener's reply to each forcing new suit at `[2M, <new suit>]`
pub(super) fn register(book: &mut Trie) {
    for our in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let trump = Strain::from(our);
        let open = call(2, trump);

        // First responses (at [2M]).
        insert_uncontested(book, &[open], responses(our));

        // Ogust: opener's answers (at [2M, 2NT]).
        let ogust = call(2, Strain::Notrump);
        insert_uncontested(book, &[open, ogust], ogust_answers(our));

        // Asker's Ogust continuations (at [2M, 2NT, <answer>]).
        register_ogust_continuations(book, our, open, ogust);

        // Opener's reply to each forcing new suit (at [2M, <new suit>]).
        register_new_suit_replies(book, our, open);
    }
}

/// Register asker's Ogust continuations for a given weak-two suit
fn register_ogust_continuations(book: &mut Trie, our: Suit, open: Call, ogust: Call) {
    let min_bad = call(3, Strain::Clubs); // min, bad suit
    let min_good = call(3, Strain::Diamonds); // min, good suit
    let max_bad = call(3, Strain::Hearts); // max, bad suit
    let max_good = call(3, Strain::Spades); // max, good suit
    // 3NT (solid): no continuation table — pass plays 3NT.

    if our == Suit::Diamonds {
        insert_uncontested(
            book,
            &[open, ogust, min_bad],
            asker_after_diamonds_min_bad(),
        );
        insert_uncontested(
            book,
            &[open, ogust, min_good],
            asker_after_diamonds_min_good(),
        );
        for max_ans in [max_bad, max_good] {
            insert_uncontested(book, &[open, ogust, max_ans], asker_after_diamonds_max());
        }
    } else {
        // Hearts and spades share the same continuation logic.
        for min_ans in [min_bad, min_good] {
            insert_uncontested(book, &[open, ogust, min_ans], asker_after_min_major(our));
        }
        for max_ans in [max_bad, max_good] {
            insert_uncontested(book, &[open, ogust, max_ans], asker_after_max_major(our));
        }
    }
}

/// Register opener's reply to each forcing new suit over `open` (= 2M)
fn register_new_suit_replies(book: &mut Trie, our: Suit, open: Call) {
    let trump = Strain::from(our);
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == our {
            continue;
        }
        let level: u8 = if Strain::from(x) > trump { 2 } else { 3 };
        let new_suit_call = call(level, Strain::from(x));
        insert_uncontested(
            book,
            &[open, new_suit_call],
            reply_to_new_suit(our, x, level),
        );
    }
}
