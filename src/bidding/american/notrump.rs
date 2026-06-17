//! Notrump response structures for the 2/1 game-forcing system
//!
//! This module centralises every notrump continuation:
//!
//! - Responses to a **1NT** opening (Stayman 2♣, Jacoby transfers 2♦/2♥,
//!   notrump raises, and the quantitative 4NT invite).
//! - Responses to a **2NT-strength** notrump — both the direct 2NT opening
//!   (20–21 balanced) and opener's 2NT rebid after a 2♣ opening (22–24
//!   balanced): 3-level Stayman / transfers and the quantitative 4NT.
//! - Simple continuations after opener's **18–19 2NT rebid** over a one-level
//!   new-suit response.
//!
//! The public surface is [`register`], called once by
//! [`american`][super::american] during system assembly.

use super::{call, insert_uncontested};
use crate::bidding::constraint::{hcp, len};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

// ---------------------------------------------------------------------------
// 1NT response structure
// ---------------------------------------------------------------------------

/// Responses to our 1NT opening: Stayman, Jacoby transfers, and notrump raises
///
/// Stayman requires at least invitational values (8+ HCP) and a four-card
/// major; transfers apply at any strength; the quantitative 4NT invites
/// slam opposite 15–17 with a balanced 16–17 HCP hand and no four-card major.
#[must_use]
pub fn notrump_responses() -> Rules {
    Rules::new()
        // Jacoby transfers, any strength.
        .rule(Bid::new(2, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        // Stayman: a four-card major and at least invitational values.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(16..=17) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // Natural notrump raises (no five-card major — that would transfer).
        // 3NT is open-ended: a strong balanced hand bids game and leaves slam
        // exploration to a later pass rather than being stranded without a call.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(10..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(8..=9) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Call::Pass,
            0.0,
            hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Opener's answer to Stayman: a four-card major, else 2♦
fn stayman_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a Jacoby transfer by bidding the anchor suit
fn complete_transfer(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(2, Strain::from(into)), 1.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// 2NT-strength response structure (2NT opening and 2♣–2x–2NT rebid)
// ---------------------------------------------------------------------------

/// Responses to a 2NT-strength notrump (3-level Stayman/transfers, 4NT invite)
///
/// Used after both the direct 2NT opening (20–21 balanced) and opener's 2NT
/// rebid after 2♣ (22–24 balanced).
fn two_notrump_responses() -> Rules {
    Rules::new()
        // 3-level Jacoby transfers.
        .rule(Bid::new(3, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(3, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        // 3-level Stayman: a four-card major and at least some values.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(5..),
        )
        // Quantitative 4NT slam invite (balanced, no four-card major).
        .rule(
            Bid::new(4, Strain::Notrump),
            1.2,
            hcp(11..=12) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        // 3NT to play: game values, no major fit.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(5..=10) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(Call::Pass, 0.0, hcp(..5))
}

/// Opener's answer to 3-level Stayman: a four-card major, else 3♦
fn stayman_answers_at_three() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a 3-level transfer by bidding the anchor suit
fn complete_transfer_at_three(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(3, Strain::from(into)), 1.0, hcp(0..))
}

/// Opener's answer to the quantitative 4NT: accept or decline the slam invite
///
/// `accept_hcp` is the minimum HCP to accept: 21 after a 2NT opening (20–21),
/// 24 after a 2♣–2x–2NT sequence (22–24).
fn quantitative_answer(accept_hcp: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(accept_hcp..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Simple continuations after an 18–19 2NT rebid
// ---------------------------------------------------------------------------

/// Responder's call after opener's 18–19 2NT rebid
///
/// 6+ HCP bids 3NT; 12–13 makes a quantitative 4NT invite; fewer points pass.
fn after_rebid_two_notrump() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 1.2, hcp(12..=13))
        .rule(Bid::new(3, Strain::Notrump), 1.0, hcp(6..))
        .rule(Call::Pass, 0.0, hcp(..6))
}

/// Opener's reply to the quantitative raise opposite the 18–19 rebid
///
/// Accept (6NT) with a maximum 19 HCP, decline (pass) otherwise.
fn accept_quantitative_nineteen() -> Rules {
    Rules::new()
        .rule(Bid::new(6, Strain::Notrump), 1.0, hcp(19..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all notrump continuations into the constructive book
///
/// Registers the 1NT structure (Stayman, transfers, 4NT quantitative), the
/// 2NT-strength structure (3-level Stayman/transfers, 4NT invite) under three
/// base prefixes (direct 2NT opening and the two 2♣–2x–2NT auctions), and
/// simple responses after opener's 18–19 2NT rebid.
pub(super) fn register(book: &mut Trie) {
    register_one_nt(book);
    register_two_nt_and_rebids(book);
}

/// Register the standard 1NT-opening response structure
///
/// Stayman 2♣, Jacoby transfers 2♦/2♥, notrump raises, and the quantitative
/// 4NT invite — the baseline 2/1 treatment.  Factored from the
/// 2NT-strength/18–19-rebid block ([`register_two_nt_and_rebids`]) so an
/// alternative 1NT scheme could replace just this part.
pub(super) fn register_one_nt(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);

    insert_uncontested(book, &[one_nt], notrump_responses());
    // Stayman answers and transfer completions.
    insert_uncontested(book, &[one_nt, call(2, Strain::Clubs)], stayman_answers());
    insert_uncontested(
        book,
        &[one_nt, call(2, Strain::Diamonds)],
        complete_transfer(Suit::Hearts),
    );
    insert_uncontested(
        book,
        &[one_nt, call(2, Strain::Hearts)],
        complete_transfer(Suit::Spades),
    );
    // Quantitative 4NT answer.
    insert_uncontested(book, &[one_nt, four_nt], quantitative_answer(17));
}

/// Register the 2NT-strength structure and the 18–19 2NT-rebid continuations
///
/// The half of the notrump book that an alternative 1NT-opening scheme would
/// keep unchanged — only [`register_one_nt`] varies.
pub(super) fn register_two_nt_and_rebids(book: &mut Trie) {
    let one_nt = call(1, Strain::Notrump);
    let two_nt = call(2, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);

    // --- 2NT-strength structure ----------------------------------------------
    //
    // Three base prefixes (our calls only; passes are interleaved by
    // `insert_uncontested`):
    //   1. [2NT]                  → direct 2NT opening (20–21), accept_hcp = 21
    //   2. [2♣, 2♦, 2NT]         → 2♣–2♦–2NT sequence (22–24), accept_hcp = 24
    //   3. [2♣, 2♥, 2NT]         → 2♣–2♥–2NT sequence (22–24), accept_hcp = 24

    let bases: &[(&[Call], u8)] = &[
        (&[two_nt], 21),
        (
            &[call(2, Strain::Clubs), call(2, Strain::Diamonds), two_nt],
            24,
        ),
        (
            &[call(2, Strain::Clubs), call(2, Strain::Hearts), two_nt],
            24,
        ),
    ];

    for (base, accept_hcp) in bases {
        // Responses to the 2NT bid.
        insert_uncontested(book, base, two_notrump_responses());

        // Stayman answers and transfer completions at the three level.
        let extend = |tail: Call| -> Vec<Call> {
            base.iter().copied().chain(core::iter::once(tail)).collect()
        };
        insert_uncontested(
            book,
            &extend(call(3, Strain::Clubs)),
            stayman_answers_at_three(),
        );
        insert_uncontested(
            book,
            &extend(call(3, Strain::Diamonds)),
            complete_transfer_at_three(Suit::Hearts),
        );
        insert_uncontested(
            book,
            &extend(call(3, Strain::Hearts)),
            complete_transfer_at_three(Suit::Spades),
        );

        // Quantitative 4NT answer.
        insert_uncontested(book, &extend(four_nt), quantitative_answer(*accept_hcp));
    }

    // --- 18–19 2NT rebid continuations --------------------------------------
    //
    // The auctions where opener's existing rebid table carries 2NT = 18–19.
    // Each prefix is [opener's opening, responder's first call] — our side's
    // two calls that precede the rebid.

    let rebid_prefixes: &[&[Call]] = &[
        &[call(1, Strain::Hearts), call(1, Strain::Spades)],
        &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
        &[call(1, Strain::Clubs), call(1, Strain::Hearts)],
        &[call(1, Strain::Clubs), call(1, Strain::Spades)],
        &[call(1, Strain::Diamonds), call(1, Strain::Hearts)],
        &[call(1, Strain::Diamonds), call(1, Strain::Spades)],
        &[call(1, Strain::Hearts), one_nt],
        &[call(1, Strain::Spades), one_nt],
    ];

    for prefix in rebid_prefixes {
        // Responder's action over opener's 2NT rebid.
        let mut our = prefix.to_vec();
        our.push(two_nt);
        insert_uncontested(book, &our, after_rebid_two_notrump());

        // Opener's reply to the quantitative 4NT raise.
        our.push(four_nt);
        insert_uncontested(book, &our, accept_quantitative_nineteen());
    }
}
