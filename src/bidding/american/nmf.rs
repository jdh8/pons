//! New Minor Forcing — the classic one-bid checkback after a 1NT rebid
//!
//! An opt-in *alternative* to [XYZ](super::xyz) on the four `1m – 1M – 1NT`
//! auctions (`opening` a minor, `response` a major).  Where XYZ splits the
//! round into a `2♣` puppet and a `2♦` game force, NMF uses a single artificial
//! call — **two of the unbid ("new") minor** — as an invitational-or-better
//! force promising a real five-card major, and asks opener to describe his
//! majors: three-card support first, then the other four-card major, else a
//! natural notrump.  Every other rebid stays natural.
//!
//! Gated on [`set_new_minor_forcing`] — default **off**.  When on it
//! *overrides* XYZ on exactly those four slots (the dispatch lives in
//! [`super::xyz::register`]); the other six XYZ auctions are untouched, so an
//! A/B differs only in the four-slot treatment.
//!
//! The checkback's projection floors the *major* (real) and not the minor, so
//! it carries no phantom suit and the floor defends sanely if doubled.
//!
//! Placement is *authored*, not floored.  Classic NMF puts the strength-show on
//! opener (minimum vs a jump) and the placement on responder — but the floor
//! can't read the jump: `points` projects only a lower bound and the
//! `fifths`-based 1NT rebid projects none, so opener's maximum never reaches the
//! floor's `combined_points` and an invitational responder under-reaches game.
//! So both sides are authored in full through to game.

use super::{call, insert_uncontested};
use crate::bidding::constraint::{len, points};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

/// NMF — two of the unbid minor: invitational-or-better with a 5-card major
const NMF: Alert = Alert("new-minor-forcing");

std::thread_local! {
    /// Whether NMF replaces XYZ on the four `1m – 1M – 1NT` slots.  Default
    /// `false` — the shipped system uses XYZ (see the module doc).
    static NEW_MINOR_FORCING: Cell<bool> = const { Cell::new(false) };
}

/// Author New Minor Forcing in place of XYZ on the four `1m – 1M – 1NT` slots
/// for books built *after* this call (default `false`; off-switch
/// `--no-ns-new-minor-forcing` in `bba-gen`, opt-in `--nmf` in
/// `ab-minor-continuations`)
///
/// Read at book-construction time; set it before building the [`Pair`].  When
/// on it overrides XYZ on those four prefixes only — XYZ still owns the other
/// six one-level auctions.
///
/// [`Pair`]: crate::bidding::Pair
pub fn set_new_minor_forcing(on: bool) {
    NEW_MINOR_FORCING.with(|cell| cell.set(on));
}

/// Whether NMF is currently authored (read by [`super::xyz::register`])
pub(super) fn new_minor_forcing() -> bool {
    NEW_MINOR_FORCING.with(Cell::get)
}

/// The four NMF-eligible slots: a minor opening and a major response
pub(super) fn is_nmf_slot(opening: Suit, response: Suit) -> bool {
    matches!(opening, Suit::Clubs | Suit::Diamonds)
        && matches!(response, Suit::Hearts | Suit::Spades)
}

/// The minor opener did *not* open — the "new" minor NMF bids
fn new_minor(opening: Suit) -> Suit {
    match opening {
        Suit::Clubs => Suit::Diamonds,
        _ => Suit::Clubs,
    }
}

/// The major responder did *not* bid — the one opener may show four of
fn other_major(response: Suit) -> Suit {
    match response {
        Suit::Hearts => Suit::Spades,
        _ => Suit::Hearts,
    }
}

/// Responder's rebid at `1m – 1M – 1NT`: NMF, else a natural placement
///
/// The table is complete over responder's hand space, so nothing here relies on
/// floor fall-through.  Only the checkback is artificial; the rest are natural.
///
/// | Call | Wt | Meaning |
/// |------|----|---------|
/// | 2(new minor) | 1.5 | NMF — invitational+, a real 5-card major |
/// | 2NT | 1.2 | Natural invite (10–12), no 5-card major to check |
/// | 3NT | 1.1 | Natural game, to play, no 5-card major |
/// | 2M  | 1.0 | Weak (≤9), rebid the 5-card major to play |
/// | Pass | 0.0 | Weak, nothing to say |
fn nmf_responder(opening: Suit, response: Suit) -> Rules {
    let major = Strain::from(response);
    Rules::new()
        .rule(
            Bid::new(2, Strain::from(new_minor(opening))),
            1.5,
            points(10..) & len(response, 5..),
        )
        .alert(NMF)
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            points(10..=12) & len(response, ..=4),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.1,
            points(13..) & len(response, ..=4),
        )
        .rule(Bid::new(2, major), 1.0, len(response, 5..) & points(..=9))
        .rule(Call::Pass, 0.0, points(..=9))
}

/// Opener's answer to the forcing new minor: majors first, else natural notrump
///
/// Opener is a balanced 12–14 (the 1NT rebid) with at most three cards in
/// responder's major.  All answers are natural — each names the suit it shows —
/// so none is alerted, and the floor reads them to place the contract.  The
/// table is complete (the `2NT` catch-all guarantees a legal call: the force
/// may not be passed).
///
/// | Call | Wt | Meaning |
/// |------|----|---------|
/// | 2M  | 1.30 | 3-card support for responder's major, minimum |
/// | 3M  | 1.20 | 3-card support, maximum (14) |
/// | 2(other major) | 1.25 | 4 cards in the other major, no support |
/// | 2NT | 1.00 | Balanced minimum, no major fit |
/// | 3NT | 1.10 | Balanced maximum, no major fit |
fn nmf_opener_answers(response: Suit) -> Rules {
    let major = Strain::from(response);
    let other = other_major(response);
    Rules::new()
        // Three-card support for responder's major: minimum raise, maximum jump.
        .rule(Bid::new(2, major), 1.30, len(response, 3..) & points(..=13))
        .rule(Bid::new(3, major), 1.20, len(response, 3..) & points(14..))
        // Four cards in the other major, denying support.  Only reachable when
        // opener could not show it earlier (always for a 1♠ response; for a 1♥
        // response only with up-the-line off) — otherwise the guard never fires.
        .rule(
            Bid::new(2, Strain::from(other)),
            1.25,
            len(other, 4..) & len(response, ..=2),
        )
        // Balanced minimum, no fit — and the guaranteed-legal catch-all.
        .rule(Bid::new(2, Strain::Notrump), 1.00, points(..=13))
        .rule(Bid::new(2, Strain::Notrump), 0.10, points(0..))
        // Balanced maximum, no fit.
        .rule(Bid::new(3, Strain::Notrump), 1.10, points(14..))
}

/// Opener accepts (14+) or declines an invitation reached below game
///
/// The same min/max threshold as the rest of the system (`points(14..)`).
fn accept_or_decline(game: Bid) -> Rules {
    Rules::new()
        .rule(game, 1.0, points(14..))
        .rule(Call::Pass, 0.0, points(0..))
}

/// Responder places after opener shows three-card support for the major
///
/// Opener's minimum (`2M`) leaves the invite live — game needs game values;
/// opener's maximum (the `3M` jump) has accepted, so any invite bids game.
/// (Floor placement can't be trusted here: the `2M`/`3M` answers project only
/// a point *floor*, and the `fifths`-based 1NT rebid projects none, so the
/// jump's extra values never reach the floor's `combined_points`.)
fn placement_over_support(major: Strain, maxed: bool) -> Rules {
    if maxed {
        Rules::new().rule(Bid::new(4, major), 1.0, points(0..))
    } else {
        Rules::new()
            .rule(Bid::new(4, major), 1.0, points(13..))
            .rule(Call::Pass, 0.0, points(0..))
    }
}

/// Responder places after opener shows a balanced hand with no major fit
///
/// Opener's minimum is `2NT` (invite passed out below game unless responder has
/// game values); opener's maximum jumped to `3NT`, already game.
fn placement_no_fit(maxed: bool) -> Rules {
    if maxed {
        Rules::new().rule(Call::Pass, 0.0, points(0..))
    } else {
        Rules::new()
            .rule(Bid::new(3, Strain::Notrump), 1.0, points(13..))
            .rule(Call::Pass, 0.0, points(0..))
    }
}

/// Responder places after opener shows four cards in the other major
///
/// A four-card holding raises the 4-4 fit (game with game values, else an
/// invitational three-level raise opener judges).  With no fit but a sixth card
/// in responder's *own* major, play there (6-2 beats notrump); otherwise
/// notrump — 3NT with game values, an invitational 2NT opener judges below it.
fn placement_over_other_major(response: Suit, other: Suit) -> Rules {
    let major = Strain::from(response);
    let o = Strain::from(other);
    Rules::new()
        .rule(Bid::new(4, o), 1.1, len(other, 4..) & points(13..))
        .rule(Bid::new(3, o), 1.0, len(other, 4..) & points(10..=12))
        // No 4-4, but a seventh card in our own major: a 7-2 fit to insist on
        // (a 6-2 belongs in notrump — bid 2NT/3NT below).
        .rule(Bid::new(4, major), 1.05, len(response, 7..) & points(13..))
        .rule(
            Bid::new(3, major),
            0.95,
            len(response, 7..) & points(10..=12),
        )
        .rule(Bid::new(3, Strain::Notrump), 0.9, points(13..))
        .rule(Bid::new(2, Strain::Notrump), 0.2, points(10..))
}

/// Register NMF and its continuations under one `1m – 1M – 1NT` prefix
///
/// Called from [`super::xyz::register`].  Authors both sides in full: the
/// checkback and every opener answer, then responder's placement over each
/// answer and opener's accept/decline of the two invitations that stop below
/// game — so nothing load-bearing is left to floor placement.
pub(super) fn register_prefix(book: &mut Trie, opening: Suit, response: Suit) {
    let prefix = [
        call(1, Strain::from(opening)),
        call(1, Strain::from(response)),
        call(1, Strain::Notrump),
    ];
    let major = Strain::from(response);
    let other = other_major(response);
    let nmf_bid = call(2, Strain::from(new_minor(opening)));
    let after_nmf = |tail: &[Call]| -> Vec<Call> {
        let mut key = vec![prefix[0], prefix[1], prefix[2], nmf_bid];
        key.extend_from_slice(tail);
        key
    };

    // Responder's checkback round, then opener's answer.
    insert_uncontested(book, &prefix, nmf_responder(opening, response));
    insert_uncontested(book, &after_nmf(&[]), nmf_opener_answers(response));

    // Responder's placement over each answer.
    insert_uncontested(
        book,
        &after_nmf(&[call(2, major)]),
        placement_over_support(major, false),
    );
    insert_uncontested(
        book,
        &after_nmf(&[call(3, major)]),
        placement_over_support(major, true),
    );
    insert_uncontested(
        book,
        &after_nmf(&[call(2, Strain::Notrump)]),
        placement_no_fit(false),
    );
    insert_uncontested(
        book,
        &after_nmf(&[call(3, Strain::Notrump)]),
        placement_no_fit(true),
    );
    insert_uncontested(
        book,
        &after_nmf(&[call(2, Strain::from(other))]),
        placement_over_other_major(response, other),
    );

    // Opener judges responder's natural direct 2NT invitation (a balanced
    // invite with no five-card major to check back).  Without this the invite
    // floats to the floor, which passes a maximum instead of raising to game.
    insert_uncontested(
        book,
        &[prefix[0], prefix[1], prefix[2], call(2, Strain::Notrump)],
        accept_or_decline(Bid::new(3, Strain::Notrump)),
    );

    // Opener judges the two invitations that stop below game.
    insert_uncontested(
        book,
        &after_nmf(&[call(2, Strain::from(other)), call(3, Strain::from(other))]),
        accept_or_decline(Bid::new(4, Strain::from(other))),
    );
    insert_uncontested(
        book,
        &after_nmf(&[call(2, Strain::from(other)), call(3, major)]),
        accept_or_decline(Bid::new(4, major)),
    );
    insert_uncontested(
        book,
        &after_nmf(&[call(2, Strain::from(other)), call(2, Strain::Notrump)]),
        accept_or_decline(Bid::new(3, Strain::Notrump)),
    );
}
