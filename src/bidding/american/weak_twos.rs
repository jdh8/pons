//! Responses to weak-two openings in the 2/1 game-forcing system
//!
//! Covers three auctions for each weak-two suit M ∈ {♦, ♥, ♠}:
//!
//! 1. **Responder's first bid** (`2M - ?`): Ogust 2NT, preemptive raises, and a
//!    one-round-forcing new suit.
//! 2. **Opener's Ogust answers** (`2M - 2NT - ?`): a conventional five-rung ladder
//!    encoding suit quality and points range.
//! 3. **Asker's Ogust continuations** (`2M - 2NT - <answer> - ?`): sign-off or game
//!    based on what opener revealed.
//! 4. **Opener's reply to a forcing new suit** (`2M - <new suit> - ?`): raise or
//!    rebid.
//!
//! # Raises are to play (RONF)
//!
//! Direct raises of the weak two are non-forcing by design: `2M - 3M` and
//! `2M - 4M` are both pre-emptive, not invitational.  Hands with game interest
//! and fit use Ogust (2NT) instead.
//!
//! # Forcing by omission
//!
//! Ogust answers carry no [`Pass`][contract_bridge::auction::Call::Pass] rule
//! — the auction is forced after 2NT.  Asker's continuations after a *max*
//! Ogust answer also omit pass (opener showed maximum values; game is
//! obligatory).

use crate::bidding::agreements::Agreements;
use crate::bidding::constraint::{hcp, len, longest_unbid, points, suit_hcp, support, top_honors};
use crate::bidding::rows::{Bindings, Package, compile_into, expand};
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
/// - **Forcing new suit** (weight 1.5): the **longest** five-card-plus suit
///   with two of the top three honors and opening values; one-round force.
///   Higher-ranking suits are bid at the two level; lower-ranking suits at the
///   three level.  Under [`OpeningKnobs::weak_two_major_priority`] a major outranks Ogust
///   over 2♦.
/// - **Simple raise** (weight 1.2): three-plus-card support, preemptive.
/// - **Pass** (weight 0.0): catch-all.
#[must_use]
pub(super) fn responses(our: Suit, agreements: &Agreements) -> Rules {
    let trump = Strain::from(our);
    let mut rules = Rules::new()
        // Ogust 2NT: at least two-card support and opening values.
        .rule(
            Bid::new(2, Strain::Notrump),
            200,
            points(14..) & support(2..),
        )
        // Pre-emptive game raise.
        .rule(Bid::new(4, trump), 130, support(4..))
        // Pre-emptive simple raise (RONF — raise is to play, not invitational).
        .rule(Bid::new(3, trump), 120, support(3..))
        // Pass: the catch-all.
        .rule(Call::Pass, 0, hcp(0..));

    // Forcing new suits: each suit other than `our`, with a natural two-level
    // bid (if the suit ranks higher) or three-level bid (if lower).
    //
    // `longest_unbid` makes the choice among them a constraint rather than an
    // argmax race: the rules all shared weight 1.5, so `Table::next_call` broke
    // the tie by *cheapest* and 5-5 majors showed the lower one — ♠AKT862
    // ♥AKJ92 responded 2♥ over 2♦.  Doctrine (shared with the advance side):
    // longest first, an equal-length tie to the higher rank.
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == our {
            continue;
        }
        let level: u8 = if Strain::from(x) > trump { 2 } else { 3 };
        // Over 2♦ only, and only for the majors, the knob lifts the new suit
        // above the 2.0 Ogust ask.
        let weight = if agreements.opening.weak_two_major_priority
            && our == Suit::Diamonds
            && Strain::from(x) > trump
        {
            210
        } else {
            150
        };
        let gate = len(x, 5..) & top_honors(x, 2..) & points(14..);
        rules = if agreements.opening.weak_two_longest_first {
            rules.rule(
                Bid::new(level, Strain::from(x)),
                weight,
                gate & longest_unbid(x, our),
            )
        } else {
            rules.rule(Bid::new(level, Strain::from(x)), weight, gate)
        };
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
/// | 3♣   | Minimum points, bad suit (< 5 HCP in trumps) | 1.0 |
/// | 3♦   | Minimum points, good suit (≥ 5 HCP in trumps) | 1.0 |
/// | 3♥   | Maximum points, bad suit | 1.0 |
/// | 3♠   | Maximum points, good suit | 1.0 |
///
/// A safety fallback of 3♣ at weight 0.2 guarantees a legal call when none
/// of the crisp rules fire (which should not happen with a legitimate weak
/// two, but is required by the "forcing by omission" model).
#[must_use]
fn ogust_answers(our: Suit) -> Rules {
    // Min (5–7) / max (8–10) points, each split bad-suit / good-suit.
    //
    // The strength gauge is `points` (rule-of-N+8) not raw HCP: responder's 2NT
    // promised 2+ support, so the fit is known here and opener re-evaluates with
    // the 6th trump and side shape credited — unlike the fit-unknown opening gate
    // (`weak_two_hcp`), which is disciplined in raw HCP.  BBA splits min/max
    // on raw HCP 7/8; we deliberately keep ours.
    //
    // The *quality* gate is BBA's, probed exactly (`probe-bba-constraints
    // --mode weak2-{d,h,s}`): "good suit" is trump HCP ≥ 5, which separates its
    // good and bad rungs with zero overlap on all three openings.  Rival
    // predicates do not: A/K/Q count overlaps at 1 (good 28, bad 202) and
    // A/K/Q/J at 2.  Our old `top_honors(our, 2..)` was a strict subset (KQ = 5
    // is the cheapest two of A/K/Q), so the sole hands that change rung are
    // A-J-x-x-x-x — 4 + 1 = 5 HCP with only one top honor.
    Rules::new()
        // Solid six-card suit (A-K-Q present): bid 3NT.  Matches BBA exactly.
        .rule(Bid::new(3, Strain::Notrump), 150, top_honors(our, 3..))
        // Minimum values (5–7 points), bad suit (under 5 HCP in trumps).
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            points(5..=7) & suit_hcp(our, ..5),
        )
        // Minimum values, good suit (5+ HCP in trumps).
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            points(5..=7) & suit_hcp(our, 5..),
        )
        // Maximum values (8–10 points), bad suit.
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            points(8..=10) & suit_hcp(our, ..5),
        )
        // Maximum values, good suit.
        .rule(
            Bid::new(3, Strain::Spades),
            100,
            points(8..=10) & suit_hcp(our, 5..),
        )
        // Safety fallback: guarantees a legal response for any legitimate
        // weak-two hand even when the crisp constraints leave a gap.
        .rule(Bid::new(3, Strain::Clubs), 20, hcp(0..))
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
        .rule(Bid::new(4, trump), 100, points(17..))
        // Sign-off at three: opener was minimum, asker is short of game force.
        .rule(Bid::new(3, trump), 50, hcp(0..))
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
    Rules::new().rule(Bid::new(4, trump), 50, hcp(0..))
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
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(17..))
        .rule(Bid::new(3, Strain::Diamonds), 50, hcp(0..))
}

/// Asker's continuation after the minimum-good (3♦) Ogust answer to 2♦
///
/// 3NT on a power hand; otherwise pass to play 3♦.
#[must_use]
fn asker_after_diamonds_min_good() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(17..))
        // Pass: 3♦ is already on the table; sign off there.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Asker's continuation after a *maximum* Ogust answer (3♥ or 3♠) to 2♦
///
/// Opener showed maximum weak-two values.  With game-going HCP, bid 5♦;
/// otherwise 3NT is the practical game in a safe major suit.
#[must_use]
fn asker_after_diamonds_max() -> Rules {
    // Forcing node: game is in range given combined values.
    Rules::new()
        .rule(Bid::new(5, Strain::Diamonds), 100, points(17..))
        .rule(Bid::new(3, Strain::Notrump), 50, hcp(0..))
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
        .rule(Bid::new(raise_level, Strain::from(x)), 100, support(3..))
        // Rebid our suit as the fallback description.
        .rule(Bid::new(rebid_level, Strain::from(our)), 50, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// All weak-two response machinery as one row package
///
/// For each weak-two suit M ∈ {♦, ♥, ♠}:
///
/// - First responses at `2M -`
/// - Ogust answers at `2M - 2NT -`
/// - Asker's Ogust continuations at `2M - 2NT - <answer> -`
/// - Opener's reply to each forcing new suit at `2M - <new suit> -`
pub(super) fn package() -> Package {
    Package {
        name: "weak-two-responses",
        gate: |_| true,
        entries: |agreements| {
            // `x` is the weak-two suit; clubs is the strong 2♣, not a weak two.
            let weak_two = |bindings: &Bindings| bindings.suit('x') != Suit::Clubs;

            // First responses (at `2M -`).
            let mut entries = expand("P* 2x -", weak_two, |b| responses(b.suit('x'), agreements));

            // Ogust: opener's answers (at `2M - 2NT -`).
            entries.extend(expand("P* 2x - 2NT -", weak_two, |b| {
                ogust_answers(b.suit('x'))
            }));

            // Asker's Ogust continuations (at `2M - 2NT - <answer> -`): the
            // answer suit encodes the rung — ♣/♦ a minimum with a bad/good
            // suit, ♥/♠ a maximum — and diamonds split from the majors in
            // the dispatch.  3NT (solid) has no continuation table; pass
            // plays it.
            entries.extend(expand("P* 2x - 2NT - 3y -", weak_two, |b| {
                match (b.suit('x'), b.suit('y')) {
                    (Suit::Diamonds, Suit::Clubs) => asker_after_diamonds_min_bad(),
                    (Suit::Diamonds, Suit::Diamonds) => asker_after_diamonds_min_good(),
                    (Suit::Diamonds, _) => asker_after_diamonds_max(),
                    (our, Suit::Clubs | Suit::Diamonds) => asker_after_min_major(our),
                    (our, _) => asker_after_max_major(our),
                }
            }));

            // Opener's reply to each forcing new suit (at `2M - <new suit> -`):
            // ascension pins the 2-level row to suits above the weak two,
            // the domain pins the 3-level row to suits below it.
            entries.extend(expand("P* 2x - 2y -", weak_two, |b| {
                reply_to_new_suit(b.suit('x'), b.suit('y'), 2)
            }));
            entries.extend(expand(
                "P* 2x - 3y -",
                |b| weak_two(b) && b.suit('y') < b.suit('x'),
                |b| reply_to_new_suit(b.suit('x'), b.suit('y'), 3),
            ));
            entries
        },
    }
}

/// Register all weak-two response machinery into the constructive book
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(book, agreements, &[package()]);
}

#[cfg(test)]
mod tests;
