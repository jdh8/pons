//! Jacoby transfers — `2♦`/`2♥`, completion, super-accept, and the invite round
//!
//! Responder transfers, opener completes (or super-accepts under
//! [`set_transfer_super_accept`]), and responder invites or signs off.  The
//! game-forcing and slam-try continuations live in [`super::transfer_gf`] and
//! [`super::transfer_slam`].

use super::invitational_majors::invitational_5card_majors;
use super::sixcard_invitation::{sixcard_invite_active, sixcard_invite_rebid};
use super::transfer_gf::{
    transfer_gf_hearts, transfer_gf_majors, transfer_heart_gf_rebid, transfer_spade_gf_rebid,
};
use super::transfer_slam::{transfer_slam_try, transfer_slam_try_rebid};
use super::*;

thread_local! {
    /// Whether opener jump super-accepts a Jacoby transfer with four-card support
    /// and a maximum; **off by default** (opt-in A/B).  See
    /// [`set_transfer_super_accept`].
    static TRANSFER_SUPER_ACCEPT: Cell<bool> = const { Cell::new(false) };
}

/// Author opener's jump super-accept of a Jacoby transfer for books built *after*
/// this call (thread-local; **off by default**).
///
/// With four-card support for responder's major and a maximum (17), opener jumps
/// to the three-level instead of merely completing the transfer, so the
/// nine-card fit and the extra values are shown in one call.  Opt-in: a paired
/// double-dummy A/B vs BBA over 640 000 boards found the jump a DD wash leaning
/// negative (−0.055 IMPs/board it fires on) — opposite a transfer that may hold
/// nothing, committing to the three-level overbids — so it stays off by default.
pub fn set_transfer_super_accept(on: bool) {
    TRANSFER_SUPER_ACCEPT.with(|cell| cell.set(on));
}

/// Whether the jump super-accept is currently authored
pub fn transfer_super_accept() -> bool {
    TRANSFER_SUPER_ACCEPT.with(Cell::get)
}

thread_local! {
    /// The Jacoby transfer names the **longer** major, and equal-length
    /// two-suiters split by strength: weak prefers the heart transfer (safety),
    /// invitational and minimum game force show both at once via the
    /// both-majors 3♦, and slam tries prefer the spade transfer for the
    /// `1NT - 2♥ - 2♠ - 3♥` structure.  **On by default**; off restores the legacy
    /// guards (a 6♠5♥ hand could tie into the heart transfer, and 3♦ fired on
    /// any 5-5+).  See [`set_transfer_longer_major`].
    static TRANSFER_LONGER_MAJOR: Cell<bool> = const { Cell::new(true) };
}

/// Author the longer-major transfer discipline for books built *after* this
/// call (thread-local; **on by default**).
///
/// The Jacoby transfer names the longer major (a 6♠5♥ hand transfers to
/// spades, whatever its strength).  With **equal** lengths (5-5, 6-6) the
/// route splits by strength: weak transfers to *hearts* (the safe partscore —
/// nothing shows the spades below it anyway), invitational and minimum game
/// force bid the both-majors `3♦` (which this discipline also restricts to
/// equal lengths — a 6-5 hand prefers naming its longer suit first), and a
/// slam try (17+) transfers to *spades* for the `1NT - 2♥ - 2♠ - 3♥` natural
/// game-force structure.  Off restores the legacy guards for the A/B.
pub fn set_transfer_longer_major(on: bool) {
    TRANSFER_LONGER_MAJOR.with(|cell| cell.set(on));
}

/// Whether the longer-major transfer discipline is currently authored (read at
/// book construction)
pub fn transfer_longer_major() -> bool {
    TRANSFER_LONGER_MAJOR.with(Cell::get)
}

/// Complete a Jacoby transfer by bidding the anchor suit
///
/// With four-card support and a maximum opener instead jumps to the three-level
/// (the super-accept, gated by [`set_transfer_super_accept`]); otherwise it
/// simply names the anchor suit.
// ponytail: a plain jump super-accept; fit-/shortness-showing super-accepts are
// the upgrade path if the A/B asks for them.
pub(crate) fn complete_transfer(into: Suit) -> Rules {
    let mut rules = Rules::new();
    if transfer_super_accept() {
        rules = rules.rule(
            Bid::new(3, Strain::from(into)),
            150,
            len(into, 4..) & hcp(17..),
        );
    }
    rules.rule(Bid::new(2, Strain::from(into)), 100, hcp(0..))
}

/// Responder's invitational 5-4 rebid after the heart transfer completes
/// (`1NT - 2♦ - 2♥`, auctions C/D)
///
/// Both rebids are exactly-8 invitational with five hearts (shown by the
/// transfer).  `2NT` adds a four-card spade suit (auction D); `2♠` is an artificial
/// relay denying it (auction C, a single-suited heart invite).  Weaker and
/// game-forcing hands match no rule and fall through to the floor's natural
/// transfer continuations.
fn transfer_heart_invite_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 4..=4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
        .rule(
            Bid::new(2, Strain::Spades),
            120,
            len(Suit::Hearts, 5..) & len(Suit::Spades, ..4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
}

/// Opener's reply to the artificial single-suited-heart invite (`…2♥ - 2♠`, C)
///
/// Responder is a bare-8 with five hearts and no four-card spade suit.  A maximum
/// (17) accepts game — `4♥` on three-card support, else `3NT`; a minimum signs off
/// in `3♥` (5-3 fit) or `2NT` (no fit), which responder passes.
pub(super) fn answer_transfer_heart_single() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            hcp(17..) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            hcp(17..) & len(Suit::Hearts, ..3),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            110,
            hcp(..17) & len(Suit::Hearts, 3..),
        )
        .rule(Bid::new(2, Strain::Notrump), 0, hcp(0..))
}

/// Opener's reply to the `2NT` invite showing five hearts and four spades
/// (`…2♥ - 2NT`, D)
///
/// Prefer the 5-3 heart fit, then the 4-4 spade fit, then notrump.  A maximum (17)
/// bids game; a minimum signs off at the three level (or passes `2NT`), which
/// responder — a bare 8 — passes.
pub(super) fn answer_transfer_heart_spade() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            160,
            hcp(17..) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            150,
            hcp(17..) & len(Suit::Hearts, ..3) & len(Suit::Spades, 4..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            140,
            hcp(17..) & len(Suit::Hearts, ..3) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            120,
            hcp(..17) & len(Suit::Hearts, 3..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            hcp(..17) & len(Suit::Hearts, ..3) & len(Suit::Spades, 4..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's invitational single-suited 5-spade rebid after the spade transfer
/// completes (`1NT - 2♥ - 2♠`)
///
/// `2NT` shows five spades (the transfer), no four-card heart suit, and exactly-8
/// invitational values.  Unlike the heart side — where `2NT` is taken by the 5♥4♠
/// invite, forcing the single-suiter through an artificial `2♠` relay — here a 5♠4♥
/// hand Staymans, so `2NT` is free.  It pins the five-card spade suit, so it carries
/// the same `INV_5CARD` alert as its heart cousins (the alert reader decodes it);
/// six-card and game-forcing hands match no rule and fall to the floor.
fn transfer_spade_invite_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            len(Suit::Spades, 5..) & len(Suit::Hearts, ..4) & hcp(8..=8),
        )
        .alert(INV_5CARD)
}

/// Opener's reply to the single-suited-spade invite (`…2♠ - 2NT`)
///
/// Responder is a bare-8 with five spades and no four-card heart suit.  A maximum
/// (17) accepts game — `4♠` on three-card support, else `3NT`; a minimum signs off
/// in `3♠` (5-3 fit) or passes `2NT` (no fit), which responder passes.  The 5-3 fit
/// out-scores 3NT even opposite a flat 4-3-3-3 maximum — responder's 5-3-3-2 always
/// brings a ruffing doubleton — so there is no flat-4333→3NT carve here (cf.
/// `accept_major_invitation`'s 4-4 case); see `examples/probe-fivecard-invite-eval`.
pub(super) fn answer_transfer_spade_single() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            hcp(17..) & len(Suit::Spades, 3..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            hcp(17..) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            hcp(..17) & len(Suit::Spades, 3..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Chain the heart-transfer rebids into their shared table
///
/// The package gate and this table deliberately read the knobs at different arities.
fn heart_transfer_rebid_table() -> Rules {
    let mut heart_rebid = Rules::new();
    if invitational_5card_majors() {
        heart_rebid = heart_rebid.chain(transfer_heart_invite_rebid());
    }
    heart_rebid = heart_rebid.chain(sixcard_invite_rebid(Suit::Hearts));
    heart_rebid = heart_rebid.chain(transfer_slam_try_rebid(Suit::Hearts));
    heart_rebid = heart_rebid.chain(transfer_heart_gf_rebid());
    heart_rebid
}

/// Chain the spade-transfer rebids into their shared table
///
/// The package gate and this table deliberately read the knobs at different arities.
fn spade_transfer_rebid_table() -> Rules {
    let mut spade_rebid = Rules::new();
    if invitational_5card_majors() {
        spade_rebid = spade_rebid.chain(transfer_spade_invite_rebid());
    }
    spade_rebid = spade_rebid.chain(sixcard_invite_rebid(Suit::Spades));
    spade_rebid = spade_rebid.chain(transfer_slam_try_rebid(Suit::Spades));
    spade_rebid = spade_rebid.chain(transfer_spade_gf_rebid());
    spade_rebid
}

/// Whether any treatment contributes to the heart-transfer rebid table
fn heart_transfer_rebid_active() -> bool {
    invitational_5card_majors()
        || sixcard_invite_active()
        || transfer_slam_try()
        || transfer_gf_hearts()
}

/// Whether any treatment contributes to the spade-transfer rebid table
fn spade_transfer_rebid_active() -> bool {
    invitational_5card_majors()
        || sixcard_invite_active()
        || transfer_slam_try()
        || transfer_gf_majors()
}

/// Responder's chained rebids after transferring to hearts
pub(crate) fn heart_transfer_rebids() -> Package {
    Package {
        name: "heart-transfer-rebids",
        gate: heart_transfer_rebid_active,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ -"),
                heart_transfer_rebid_table(),
            )
        },
    }
}

/// Responder's chained rebids after transferring to spades
pub(crate) fn spade_transfer_rebids() -> Package {
    Package {
        name: "spade-transfer-rebids",
        gate: spade_transfer_rebid_active,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT - 2♥ - 2♠ -"),
                spade_transfer_rebid_table(),
            )
        },
    }
}

#[cfg(test)]
mod tests;
