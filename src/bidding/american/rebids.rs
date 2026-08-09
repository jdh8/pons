//! Opener's rebids (one round) and the forcing-1NT continuations
//!
//! This module is the **index**: the four base rebid tables after a one-level
//! response, the two shared acceptance tables, and `register`.  Each agreement
//! that overlays a base table — the strength ladders, the Meckstroth adjunct,
//! the invitational two-suiter — lives in its own submodule and is folded in by
//! a `with_*` combinator here:
//!
//! | Module | Agreement | Knob |
//! | --- | --- | --- |
//! | [`extras_ladder`] | jump-rebid / reverse / jump-shift after a minor opening | [`set_opener_extras_ladder`] |
//! | [`major_jump_rebid`] | `3M` on a six-card major with extras | [`set_opener_major_jump_rebid`] |
//! | [`meckstroth`] | the artificial GF `2NT` and the invitational `3m` jumps | [`RebidKnobs::meckstroth_adjunct`] |
//! | [`two_suiter`] | `1♥ - 1NT - 2♠` / `1♠ - 1NT - 3♥`, 15–17 | [`RebidKnobs::forcing_nt_two_suiter`] |
//! | [`forcing_notrump`] | responder's second call after the forcing `1NT` | always on |
//! | [`major_tails`] | full continuations after `1♥ - 1♠` (with 4SF) | [`RebidKnobs::major_rebid_tails`] |

use super::{call, other_major};
use crate::bidding::agreements::{Agreements, RebidKnobs};
use crate::bidding::constraint::{
    balanced, fifths, hcp, len, partner_suit_is, points, stopper_in, support,
};
use crate::bidding::rows::{Package, Pattern, compile_into, expand, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

mod extras_ladder;
mod forcing_notrump;
mod major_jump_rebid;
mod major_tails;
mod meckstroth;
mod two_suiter;

use extras_ladder::with_extras_ladder;
use major_jump_rebid::with_major_jump_rebid;
use meckstroth::with_invitational_minors;
use two_suiter::with_forcing_nt_two_suiter;

pub use extras_ladder::set_opener_extras_ladder;
pub use major_jump_rebid::set_opener_major_jump_rebid;

// Knobs the inference walk reads at classify time.
pub(crate) use extras_ladder::opener_extras_ladder;
pub(crate) use major_jump_rebid::opener_major_jump_rebid;

// The packages, re-exported so `american::tests::row_package_invariants` and
// `register` below name them at one path.
pub(super) use forcing_notrump::forcing_notrump_continuations;
pub(super) use major_jump_rebid::major_jump_rebid_continuations;
pub(super) use major_tails::{fourth_suit_forcing_continuations, major_rebid_tail_continuations};
pub(super) use meckstroth::{
    invitational_minor_continuations, meckstroth_two_notrump_continuations,
};
pub(super) use two_suiter::forcing_nt_two_suiter_continuations;

// ponytail: same construction-time toggle as the Meckstroth adjunct — read
// during `register()`, so set it before building the `Pair`.
/// The cheapest level at which `strain` may be bid over `highest`
fn cheapest_level_over(highest: Bid, strain: Strain) -> u8 {
    if strain > highest.strain {
        highest.level.get()
    } else {
        highest.level.get() + 1
    }
}

/// Opener's reverse — a higher new suit showing a five-card first suit and extras
const OPENER_REVERSE: Alert = Alert("opener-reverse");
/// Opener's jump-shift — a new suit showing a big two-suiter, game-forcing
const OPENER_JUMP_SHIFT: Alert = Alert("opener-jump-shift");

/// Opener's rebid after `1♥ - 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener — there is no pass rule.
fn rebid_one_heart_one_spade(agreements: &Agreements) -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            260,
            support(4..) & points(19..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            220,
            support(4..) & points(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            180,
            support(4..) & points(12..=15),
        )
        .rule(Bid::new(2, Strain::Hearts), 140, len(Suit::Hearts, 6..))
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            fifths(18.0..20.0) & balanced(),
        );
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules, &agreements.rebid);
    // Major jump-rebid: 1♥ - 1♠ - 3♥ on a six-card major with extras.
    rules = with_major_jump_rebid(rules, Suit::Hearts, Bid::new(1, Strain::Spades), agreements);
    rules
        .rule(Bid::new(2, Strain::Clubs), 90, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Diamonds), 90, len(Suit::Diamonds, 4..))
        // Balanced minimum, and the guaranteed-legal fallback.
        .rule(Bid::new(1, Strain::Notrump), 50, fifths(12.0..15.0))
        .rule(Bid::new(1, Strain::Notrump), 20, hcp(0..))
}

/// Opener's rebid after `1M - 1NT` (the forcing notrump)
///
/// Forcing on opener.  A five-card-major rebid is the guaranteed-legal
/// fallback when nothing more descriptive fits — a basic simplification.
fn rebid_after_forcing_notrump(major: Suit, agreements: &Agreements) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new();
    // 2NT: the Meckstroth adjunct's artificial 18+ game force (any shape) when
    // enabled, otherwise the natural 18–19 balanced rebid.  Weight 1.6 to outrank
    // the 3M major jump-rebid (1.5), so every 18+ hand routes through the game
    // force while the invitational 3m jumps stay 15–17.
    if agreements.rebid.meckstroth_adjunct {
        rules = rules
            .rule(Bid::new(2, Strain::Notrump), 160, points(18..))
            .alert(meckstroth::OPENER_GF_2NT);
    } else {
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            120,
            fifths(18.0..20.0) & balanced(),
        );
    }
    rules = rules.rule(Bid::new(2, trump), 100, len(major, 6..));
    // Meckstroth adjunct: invitational 3♣/3♦ jumps with a five-card minor.
    rules = with_invitational_minors(rules, &agreements.rebid);
    // Major jump-rebid: 1M - 1NT - 3M on a six-card major with extras.
    rules = with_major_jump_rebid(rules, major, Bid::new(1, Strain::Notrump), agreements);
    // Invitational two-suiter: 1♥ - 1NT - 2♠ reverse / 1♠ - 1NT - 3♥ jump.
    rules = with_forcing_nt_two_suiter(rules, major, &agreements.rebid);
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(Bid::new(2, Strain::from(suit)), 90, len(suit, 4..));
        }
    }
    // Opener always holds at least five of the major, so this always applies.
    rules.rule(Bid::new(2, trump), 30, len(major, 5..))
}

/// Opener's rebid raising responder's new major after a minor opening
///
/// Used at `1m - 1M`.  Forcing on opener; a 1NT rebid is the guaranteed-legal
/// fallback.  Under the up-the-line completion (`up_the_line`) opener
/// also shows four spades over a `1♥` response — without it the 4-4 spade
/// fit is lost to the 1NT rebid.
fn rebid_raise_major(responder_major: Suit, opener_minor: Suit, agreements: &Agreements) -> Rules {
    let m = Strain::from(responder_major);
    let mut rules = Rules::new()
        .rule(Bid::new(4, m), 260, support(4..) & points(19..))
        .rule(Bid::new(3, m), 220, support(4..) & points(16..=18))
        .rule(Bid::new(2, m), 180, support(4..) & points(12..=15))
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            fifths(18.0..20.0) & balanced(),
        );
    // Balanced 12–14 with a five-card minor: rebid 1NT rather than the natural
    // 2m below it (weight 0.92 — above the 2m rebid, below the up-the-line 1♠
    // so a 4-4 spade fit is still found).  Shipped default-on.
    if agreements.rebid.balanced_1nt_rebid {
        rules = rules.rule(
            Bid::new(1, Strain::Notrump),
            92,
            fifths(12.0..15.0) & balanced(),
        );
    }
    // Up the line: four spades over a 1♥ response, ahead of the minor rebid
    // and the notrump fallbacks (a heart raise with four-card support still
    // wins on weight).
    if responder_major == Suit::Hearts && agreements.response.up_the_line {
        rules = rules.rule(Bid::new(1, Strain::Spades), 95, len(Suit::Spades, 4..));
    }
    // Strength-showing ladder: jump-rebid, reverse, jump-shift (default off).
    rules = with_extras_ladder(
        rules,
        opener_minor,
        Bid::new(1, m),
        Some(responder_major),
        agreements,
    );
    rules
        .rule(
            Bid::new(2, Strain::from(opener_minor)),
            90,
            len(opener_minor, 5..),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            50,
            fifths(12.0..15.0) & balanced(),
        )
        .rule(Bid::new(1, Strain::Notrump), 20, hcp(0..))
}

/// Opener's rebid after `1♣ - 1♦`
///
/// Under the up-the-line completion (`up_the_line`) a six-plus club suit
/// rebids a natural `2♣` — without it those hands land in the misdescribed
/// 1NT catch-all.
fn rebid_one_club_one_diamond(agreements: &Agreements) -> Rules {
    let mut rules = Rules::new()
        .rule(Bid::new(1, Strain::Hearts), 130, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(1, Strain::Spades),
            130,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            150,
            support(4..) & points(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            120,
            support(4..) & points(12..=15),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            110,
            fifths(18.0..20.0) & balanced(),
        );
    if agreements.response.up_the_line {
        rules = rules.rule(Bid::new(2, Strain::Clubs), 90, len(Suit::Clubs, 6..));
    }
    // Strength-showing ladder: jump-rebid, reverse, jump-shift (default off).
    rules = with_extras_ladder(
        rules,
        Suit::Clubs,
        Bid::new(1, Strain::Diamonds),
        Some(Suit::Diamonds),
        agreements,
    );
    rules
        .rule(
            Bid::new(1, Strain::Notrump),
            50,
            fifths(12.0..15.0) & balanced(),
        )
        .rule(Bid::new(1, Strain::Notrump), 20, hcp(0..))
}

/// Opener accepts or declines responder's 2NT notrump invite
///
/// Accept with 14+ HCP (bid 3NT), decline with a pass.
fn opener_accept_notrump_invite() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(14..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener accepts or declines responder's 3M limit raise
///
/// Accept with 14+ points (bid game in the major), decline with a pass.
fn opener_accept_limit_raise(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, points(14..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's base rebid after `1♥ - 1♠`
pub(super) fn one_heart_one_spade_rebid() -> Package {
    Package {
        name: "one-heart-one-spade-rebid",
        gate: |_| true,
        entries: |agreements| {
            rows_of(
                Pattern::node("P* 1♥ - 1♠ -"),
                rebid_one_heart_one_spade(agreements),
            )
        },
    }
}

/// The remaining base rebid nodes after one-level responses
pub(super) fn remaining_rebid_bases() -> Package {
    Package {
        name: "remaining-rebid-bases",
        gate: |_| true,
        entries: |agreements| {
            let mut entries = expand(
                "P* 1M - 1NT -",
                |_| true,
                |b| rebid_after_forcing_notrump(b.suit('M'), agreements),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1♣ - 1♦ -"),
                rebid_one_club_one_diamond(agreements),
            ));
            entries.extend(expand(
                "P* 1m - 1M -",
                |_| true,
                |b| rebid_raise_major(b.suit('M'), b.suit('m'), agreements),
            ));
            entries
        },
    }
}

/// Register opener's rebids after a one-level new suit and the forcing 1NT
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(
        book,
        agreements,
        &[
            forcing_notrump_continuations(),
            invitational_minor_continuations(),
            major_jump_rebid_continuations(),
            forcing_nt_two_suiter_continuations(),
            meckstroth_two_notrump_continuations(),
            one_heart_one_spade_rebid(),
            major_rebid_tail_continuations(),
            fourth_suit_forcing_continuations(),
            remaining_rebid_bases(),
        ],
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
