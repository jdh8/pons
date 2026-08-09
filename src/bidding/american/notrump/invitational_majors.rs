//! Invitational 5-4 and 5-5 majors — `1NT - 2♣` then an invitational rebid
//!
//! The hand too strong to sign off and too weak to force, holding five of one
//! major and four (or five) of the other.  Gated by
//! [`set_invitational_5card_majors`]; all four shapes are authored.

use super::transfers::{
    answer_transfer_heart_single, answer_transfer_heart_spade, answer_transfer_spade_single,
};
use super::*;

thread_local! {
    /// The invitational 5-4-majors structure: 5♠4♥ invites via Stayman (a 2♠ rebid
    /// over opener's 2♦/2♥), 5♥4♠ via the heart transfer (`2NT` shows the spades,
    /// `2♠` denies them).  **On by default** — a paired A/B vs BBA (1.28M boards/arm,
    /// `--filter-1nt`, vul none) measured **+0.375 IMPs/fired plain (+0.0020/board,
    /// 95% CI ±0.0004) and +0.134 PD (+0.0007/board, 95% CI ±0.0005)**, both excl 0.
    /// The win needed the doubled-2♦ escape (`1NT - 2♣ - 2♦ (X)` systems-on rebase in
    /// `competition.rs`): without it the reroute walked 5♠4♥ into a doubled artificial
    /// 2♦ it passed out, and PD was a wash (−0.0001).  Flipped per
    /// [`set_invitational_5card_majors`].
    static INVITATIONAL_5CARD_MAJORS: Cell<bool> = const { Cell::new(true) };
}

/// Author the invitational 5-4-majors structure for books built *after* this call
/// (thread-local; **off by default**).
///
/// 5♠4♥ at invitational+ values keeps off the spade transfer and bids Stayman,
/// inviting with a 2♠ rebid over opener's 2♦ (non-forcing) or 2♥ (forcing); 5♥4♠
/// transfers to hearts and rebids `2NT` (showing the four spades) or `2♠` (an
/// artificial relay denying them).  A Muppet-style swap brought down to the
/// two-level over 1NT — see CHANGELOG.
pub fn set_invitational_5card_majors(on: bool) {
    INVITATIONAL_5CARD_MAJORS.with(|cell| cell.set(on));
}

/// Whether the invitational 5-4-majors structure is currently authored (read at
/// book construction to gate the reroute, the Stayman 2♠ rebids, and the
/// heart-transfer invitational node)
pub fn invitational_5card_majors() -> bool {
    INVITATIONAL_5CARD_MAJORS.with(Cell::get)
}

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
        gate: |agreements| agreements.build.notrump.invitational_5card_majors,
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
