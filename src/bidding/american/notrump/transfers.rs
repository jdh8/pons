//! Jacoby transfers — `2♦`/`2♥`, completion, super-accept, and the invite round
//!
//! Responder transfers, opener completes (or super-accepts under
//! [`NotrumpKnobs::transfer_super_accept`][crate::bidding::agreements::NotrumpKnobs::transfer_super_accept]), and responder invites or signs off.  The
//! game-forcing and slam-try continuations live in [`super::transfer_gf`] and
//! [`super::transfer_slam`].

use super::sixcard_invitation::{sixcard_invite_active, sixcard_invite_rebid};
use super::transfer_gf::{transfer_heart_gf_rebid, transfer_spade_gf_rebid};
use super::transfer_slam::transfer_slam_try_rebid;
use super::*;

/// Complete a Jacoby transfer by bidding the anchor suit
///
/// With four-card support and a maximum opener instead jumps to the three-level
/// (the super-accept, gated by [`NotrumpKnobs::transfer_super_accept`][crate::bidding::agreements::NotrumpKnobs::transfer_super_accept]); otherwise it
/// simply names the anchor suit.
// ponytail: a plain jump super-accept; fit-/shortness-showing super-accepts are
// the upgrade path if the A/B asks for them.
pub(crate) fn complete_transfer(into: Suit, agreements: &Agreements) -> Rules {
    let alerts = agreements.decision.reading.completion_alerts;
    let mut rules = Rules::new();
    if agreements.notrump.transfer_super_accept {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(into)),
                150,
                len(into, 4..) & hcp(17..),
            )
            .alert_if(alerts, COMPLETION);
    }
    rules
        .rule(Bid::new(2, Strain::from(into)), 100, hcp(0..))
        .alert_if(alerts, COMPLETION)
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
fn heart_transfer_rebid_table(agreements: &Agreements) -> Rules {
    let mut heart_rebid = Rules::new();
    if agreements.notrump.invitational_5card_majors {
        heart_rebid = heart_rebid.chain(transfer_heart_invite_rebid());
    }
    heart_rebid = heart_rebid.chain(sixcard_invite_rebid(Suit::Hearts, agreements));
    heart_rebid = heart_rebid.chain(transfer_slam_try_rebid(Suit::Hearts, agreements));
    heart_rebid = heart_rebid.chain(transfer_heart_gf_rebid(agreements));
    heart_rebid
}

/// Chain the spade-transfer rebids into their shared table
///
/// The package gate and this table deliberately read the knobs at different arities.
fn spade_transfer_rebid_table(agreements: &Agreements) -> Rules {
    let mut spade_rebid = Rules::new();
    if agreements.notrump.invitational_5card_majors {
        spade_rebid = spade_rebid.chain(transfer_spade_invite_rebid());
    }
    spade_rebid = spade_rebid.chain(sixcard_invite_rebid(Suit::Spades, agreements));
    spade_rebid = spade_rebid.chain(transfer_slam_try_rebid(Suit::Spades, agreements));
    spade_rebid = spade_rebid.chain(transfer_spade_gf_rebid(agreements));
    spade_rebid
}

/// Whether any treatment contributes to the heart-transfer rebid table
fn heart_transfer_rebid_active(agreements: &Agreements) -> bool {
    agreements.notrump.invitational_5card_majors
        || sixcard_invite_active(agreements)
        || agreements.notrump.transfer_slam_try
        || agreements.decision.transfer_gf_heart_mirror()
}

/// Whether any treatment contributes to the spade-transfer rebid table
fn spade_transfer_rebid_active(agreements: &Agreements) -> bool {
    agreements.notrump.invitational_5card_majors
        || sixcard_invite_active(agreements)
        || agreements.notrump.transfer_slam_try
        || agreements.decision.transfer_gf_majors
}

/// Responder's chained rebids after transferring to hearts
pub(crate) fn heart_transfer_rebids() -> Package {
    Package {
        name: "heart-transfer-rebids",
        gate: |agreements| heart_transfer_rebid_active(agreements),
        entries: |agreements| {
            rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ -"),
                heart_transfer_rebid_table(agreements),
            )
        },
    }
}

/// Responder's chained rebids after transferring to spades
pub(crate) fn spade_transfer_rebids() -> Package {
    Package {
        name: "spade-transfer-rebids",
        gate: |agreements| spade_transfer_rebid_active(agreements),
        entries: |agreements| {
            rows_of(
                Pattern::node("P* 1NT - 2♥ - 2♠ -"),
                spade_transfer_rebid_table(agreements),
            )
        },
    }
}

#[cfg(test)]
mod tests;
