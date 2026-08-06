//! The Dutch system — a natural 2/1 built around a wide, non-forcing 1♣
//!
//! Dutch naturalises the Polish 1♣: a "lawyer's Polish Club" that keeps Polish
//! constructiveness while staying natural and less restricted.  The 1♣ opening
//! is non-forcing, 2+♣, 11–23 HCP, and hosts every strong hand that lacks the
//! strong-2♣ shape (the `1♣–1♦` relay sorts them out).  Otherwise it mirrors
//! `american()`: five-card majors, a 15–17 1NT, 2/1 game-forcing continuations.
//!
//! This is a **champion candidate**, built by copying `american()` and applying
//! the Dutch diff one measurable phase at a time.  Until it measures stronger,
//! it lives here as a sibling factory under the standard A/B discipline; see
//! `docs/dutch-system.md` for the campaign ledger.

mod openings;
mod responses;

use super::Pair;
use super::american::american_book;
use super::card::dutch_card;
use super::common::{call, insert_uncontested, with_floor, with_instinct_floor};
use super::features::Config;
use super::neural_floor::ConfiguredFloorBba;
use contract_bridge::auction::Call;
use contract_bridge::{Strain, Suit};

/// Build the Dutch system as one side's [`Pair`]
///
/// Bind it with [`Pair::against`] and seat two pairs with
/// [`Table::of_pairs`][super::Table::of_pairs], exactly like `american()`.
///
/// The contested books stand on [`ConfiguredFloorBba`] under
/// [`dutch_card`], so the net is told it is bidding
/// Dutch rather than 2/1 — the v4 corpus covers both base systems.  Before the
/// configured net this was `NeuralFloorBba`, which had never seen a WJ card and
/// invented a diamond suit over the `1♦` relay.
///
/// ```
/// use pons::dutch;
/// use pons::bidding::System;
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = dutch().against();
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = stance
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, Call::Bid(Bid::new(1, Strain::Notrump)));
/// ```
#[must_use]
pub fn dutch() -> Pair {
    with_floor(
        dutch_book(),
        ConfiguredFloorBba::new(Config::symmetric(&dutch_card())),
    )
}

/// The Dutch pair with the deterministic **instinct** floor (the pre-swap default)
///
/// Exactly [`dutch`] but for the floor: the BBA-distilled
/// [`ConfiguredFloorBba`] gives way to the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder.  Mirrors
/// [`american_instinct`][crate::american_instinct] — the fully-disclosable
/// reference, and the fixed baseline the Dutch campaign's floor A/Bs anchor on.
///
/// The floor is the *only* difference; both share the same authored books.
#[must_use]
pub fn dutch_instinct() -> Pair {
    with_instinct_floor(dutch_book())
}

/// Build the Dutch pair as the authored books alone, with no floor
///
/// Takes a full [`american_book`] pair and overwrites the **divergent nodes**
/// (`Trie::insert_arc` replaces the classifier at each key); every other
/// american continuation is reused verbatim.  Phase 1 overwrote the opening
/// table (`openings::dutch_openings`); Phase 2.1 overwrites the wide-1♣
/// response node and opener's rebid after the `1♦` relay; Phase 2.2 adds
/// responder's second call over opener's minimum rebids (`1♣-1♦-1M`,
/// `1♣-1♦-2♣`).  The rare 18–20 `1NT` / 21–23 `2♦!` continuations stay
/// american's — projection discloses their strength; see `docs/dutch-system.md`.
#[must_use]
pub fn dutch_book() -> Pair {
    let mut pair = american_book();
    let book = &mut pair.constructive.0;
    // `insert_uncontested` re-keys at the undisturbed auction for every seat,
    // and `Trie::insert_arc` replaces the classifier there — a clean overwrite.
    insert_uncontested(book, &[], openings::dutch_openings());
    let one_club = call(1, Strain::Clubs);
    let relay = call(1, Strain::Diamonds);
    insert_uncontested(book, &[one_club], responses::one_club_responses());
    insert_uncontested(
        book,
        &[one_club, relay],
        responses::opener_rebids_after_relay(),
    );
    // Phase 2.2 increment 1 — responder's second call after opener's *minimum*
    // relay rebids (11–17), the high-frequency landing spots.  Deeper opener
    // rebids (18–20 `1NT`, 21–23 `2♦!`) still fall to the floor, which reads
    // their self-disclosed strength off the alerted rule; see `docs/dutch-system.md`.
    insert_uncontested(
        book,
        &[one_club, relay, call(1, Strain::Hearts)],
        responses::relay_responses_after_major(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_club, relay, call(1, Strain::Spades)],
        responses::relay_responses_after_major(Suit::Spades),
    );
    insert_uncontested(
        book,
        &[one_club, relay, call(2, Strain::Clubs)],
        responses::relay_responses_after_club(),
    );
    // Phase 2.2 increment 2 — opener's rebid after responder's natural minor
    // two-level responses.  These overwrite american's inverted-raise (`2♣`) and
    // weak-jump-shift (`2♦`) continuations, which misread the Dutch meanings
    // (invite+ 5+♣ / game-forcing 5+♦); see `docs/dutch-system.md`.
    let two_diamonds = call(2, Strain::Diamonds);
    let two_clubs = call(2, Strain::Clubs);
    insert_uncontested(
        book,
        &[one_club, two_diamonds],
        responses::opener_rebids_after_two_diamonds(),
    );
    insert_uncontested(
        book,
        &[one_club, two_clubs],
        responses::opener_rebids_after_two_clubs(),
    );
    // Responder's continuation over each opener rebid.  The opener-only version
    // measured a loss (A/B: the floor dropped the game force and blasted slam);
    // these author responder to honour the force and cap at the right game.
    for rebid in [
        call(3, Strain::Diamonds),
        call(3, Strain::Clubs),
        call(3, Strain::Notrump),
        call(2, Strain::Hearts),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
    ] {
        let Call::Bid(bid) = rebid else { continue };
        insert_uncontested(
            book,
            &[one_club, two_diamonds, rebid],
            responses::responder_after_two_diamonds(bid),
        );
    }
    for rebid in [
        call(3, Strain::Notrump),
        call(3, Strain::Clubs),
        call(2, Strain::Notrump),
    ] {
        let Call::Bid(bid) = rebid else { continue };
        insert_uncontested(
            book,
            &[one_club, two_clubs, rebid],
            responses::responder_after_two_clubs(bid),
        );
    }
    pair
}

#[cfg(test)]
mod tests;
