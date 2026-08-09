//! Invitational 5-4 and 5-5 majors — `1NT - 2♣` then an invitational rebid
//!
//! The hand too strong to sign off and too weak to force, holding five of one
//! major and four (or five) of the other.  Gated by
//! [`NotrumpKnobs::invitational_5card_majors`][crate::bidding::agreements::NotrumpKnobs::invitational_5card_majors]; all four shapes are authored.

use super::transfers::{
    answer_transfer_heart_single, answer_transfer_heart_spade, answer_transfer_spade_single,
};
use super::*;

/// Opener's reply to the non-forcing `2♠` invite (`1NT - 2♣ - 2♦ - 2♠`, auction A)
///
/// Responder is a bare-8 5♠4♥; opener denied both majors (so 2-3 spades).  With a
/// maximum (17) accept game — `4♠` on three-card support, else `3NT`; a minimum
/// passes the 5-2/5-3 spade partscore.
fn answer_inv_5card_spades() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            120,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            110,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's reply to the forcing `2♠` (`1NT - 2♣ - 2♥ - 2♠`, auction B)
///
/// Responder is 5♠4♥, invitational through slam; opener has four hearts (so a 4-4
/// heart fit at least) and may hold three spades (a 5-3 spade fit).  Prefer the
/// spade fit when held.  A maximum (17) jumps to game; a minimum (15-16) signs the
/// invite back at the three level for responder to pass (8) or raise (9+).  Slam
/// past game is left to the floor's keycard/search.
// ponytail: a flat min/max split; control-showing replies are the upgrade path.
fn answer_inv_5card_both() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            130,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            120,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            hcp(..17) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            100,
            hcp(..17) & len(Suit::Spades, ..3),
        )
}

/// Responder passes or raises opener's three-level invite-back (auction B min)
///
/// Opener declined to `3♥`/`3♠` (a minimum); responder passes the bare 8 or accepts
/// game with 9+.
// ponytail: 9+ always bids game — slam tries past 4M are left to the floor.
fn inv_5card_raise(strain: Strain) -> Rules {
    Rules::new()
        .rule(Bid::new(4, strain), 100, hcp(9..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Invitational five-card-major continuations after Stayman and transfers
pub(crate) fn invitational_majors() -> Package {
    Package {
        name: "invitational-five-card-majors",
        gate: |agreements| agreements.notrump.invitational_5card_majors,
        entries: |_| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ - 2♠ -"),
                answer_inv_5card_spades(),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ - 2♠ -"),
                answer_inv_5card_both(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ - 2♠ - 3♥ -"),
                inv_5card_raise(Strain::Hearts),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♥ - 2♠ - 3♠ -"),
                inv_5card_raise(Strain::Spades),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ - 2♠ -"),
                answer_transfer_heart_single(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ - 2NT -"),
                answer_transfer_heart_spade(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♥ - 2♠ - 2NT -"),
                answer_transfer_spade_single(),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
