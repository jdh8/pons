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
//! | [`extras_ladder`] | jump-rebid / reverse / jump-shift after a minor opening | [`opener_extras_ladder`][field@crate::bidding::inference::ReadingProfile::opener_extras_ladder] |
//! | [`major_jump_rebid`] | `3M` on a six-card major with extras | [`opener_major_jump_rebid`][field@crate::bidding::inference::ReadingProfile::opener_major_jump_rebid] |
//! | [`minor_jump_notrump`] | responder's `4m` slam try over `1m - 1x - 3NT` and opener's answer | [`RebidKnobs::minor_jump_notrump_slam_try`] |
//! | [`meckstroth`] | the artificial GF `2NT` and the invitational `3m` jumps | [`RebidKnobs::meckstroth_adjunct`] |
//! | [`two_suiter`] | `1♥ - 1NT - 2♠` / `1♠ - 1NT - 3♥`, 15–17 | [`RebidKnobs::forcing_nt_two_suiter`] |
//! | [`jump_shifts`] | natural 18+ jump shifts and the long-major `3NT!` over the forcing `1NT` (the Meckstroth rival) | [`RebidKnobs::forcing_nt_jump_shifts`] |
//! | [`odwrotka`] | `1♣ - 1M - 2♦!` artificial reverse and its 445566 steps | [`RebidKnobs::odwrotka`] |
//! | [`forcing_notrump`] | responder's second call after the forcing `1NT` | always on |
//! | [`major_tails`] | full continuations after `1♥ - 1♠` (with 4SF) | [`RebidKnobs::major_rebid_tails`] |

use super::{call, other_major};
use crate::bidding::agreements::{Agreements, RebidKnobs};
use crate::bidding::constraint::{
    balanced, fifths, hcp, len, longer_suit, partner_passed_hand, partner_suit_is, points,
    stopper_in, support,
};
use crate::bidding::rows::{Package, Pattern, compile_into, expand, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

mod extras_ladder;
mod forcing_notrump;
mod jump_shifts;
mod major_jump_rebid;
mod major_tails;
mod meckstroth;
mod minor_jump_notrump;
mod odwrotka;
mod two_suiter;

use extras_ladder::with_extras_ladder;
use jump_shifts::{forcing_nt_jump_shifts_on, with_forcing_nt_jump_shifts};
use major_jump_rebid::with_major_jump_rebid;
use meckstroth::with_invitational_minors;
use odwrotka::with_odwrotka;
use two_suiter::with_forcing_nt_two_suiter;

// The packages, re-exported so `american::tests::row_package_invariants` and
// `register` below name them at one path.
pub(super) use forcing_notrump::forcing_notrump_continuations;
pub(super) use jump_shifts::forcing_nt_jump_shift_continuations;
pub(super) use major_jump_rebid::major_jump_rebid_continuations;
pub(super) use major_tails::{fourth_suit_forcing_continuations, major_rebid_tail_continuations};
pub(super) use meckstroth::{
    invitational_minor_continuations, meckstroth_two_notrump_continuations,
};
pub(super) use minor_jump_notrump::minor_jump_notrump_slam_continuations;
pub(super) use odwrotka::odwrotka_continuations;
pub(super) use two_suiter::forcing_nt_two_suiter_continuations;

// ponytail: same construction-time toggle as the Meckstroth adjunct — read
// during `register()`, so set it before building the `System`.
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

/// Opener's pass of a passed hand's `1M` response
/// ([`RebidKnobs::passed_hand_major_pass`])
///
/// A passed hand's new suit is not forcing: a balanced minimum with exactly
/// three-card support plays the one-level major.  Weight 93 sits above the
/// balanced `1NT` rebid (92) and below the up-the-line `1♠` (95).
fn with_passed_hand_pass(rules: Rules, rebid: &RebidKnobs) -> Rules {
    match rebid.passed_hand_major_pass {
        Some(ceiling) => rules.rule(
            Call::Pass,
            93,
            partner_passed_hand() & support(3..=3) & balanced() & hcp(..=ceiling),
        ),
        None => rules,
    }
}

/// Opener's rebid after `1♥ - 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener unless partner is a passed hand
/// ([`with_passed_hand_pass`]).
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
    rules = with_passed_hand_pass(rules, &agreements.rebid);
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
    } else if forcing_nt_jump_shifts_on(&agreements.rebid) {
        // Natural jump shifts: the balanced 18+ is uncapped (a 5332 with more
        // has no other rebid), the shapely 18+ takes the `3x`/`3NT!` rungs.
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            120,
            fifths(18.0..) & balanced(),
        );
    } else {
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            120,
            fifths(18.0..20.0) & balanced(),
        );
    }
    rules = rules.rule(Bid::new(2, trump), 100, len(major, 6..));
    // Natural strong jump shifts and the long-major 3NT! (default off).
    rules = with_forcing_nt_jump_shifts(rules, major, &agreements.rebid);
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
    // A 5-3-3-2 bids its three-card minor, clubs with 3-3, rather than
    // rebidding a five-card major (just above the 2M fallback).
    if agreements.decision.reading.forcing_nt_three_card_minor {
        rules = rules
            .rule(Bid::new(2, Strain::Clubs), 40, len(Suit::Clubs, 3..))
            .rule(Bid::new(2, Strain::Diamonds), 35, len(Suit::Diamonds, 3..));
    }
    // Opener always holds at least five of the major, so this always applies.
    rules.rule(Bid::new(2, trump), 30, len(major, 5..))
}

/// Opener's rebid raising responder's new major after a minor opening
///
/// Used at `1m - 1M`.  Forcing on opener unless partner is a passed hand
/// ([`with_passed_hand_pass`]); a 1NT rebid is the guaranteed-legal fallback.  Under the up-the-line completion (`up_the_line`) opener
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
    // A minimum 5m-4♥ over `1♠` rebids 1NT, so the minor rebid promises six
    // (BBA's book) and responder's four hearts can still be found.  Weight 93
    // breaks the same-call tie with the balanced 1NT (92); the passed-hand
    // pass at 93 needs a balanced three-card raise, which no 5m-4♥ is.
    if responder_major == Suit::Spades && agreements.rebid.unbalanced_1nt_rebid {
        rules = rules
            .rule(
                Bid::new(1, Strain::Notrump),
                93,
                fifths(..15.0) & len(opener_minor, 5..=5) & len(Suit::Hearts, 4..=4),
            )
            // The call is natural: its balanced sibling floors no hearts.
            .natural();
    }
    // Up the line: four spades over a 1♥ response, ahead of the minor rebid
    // and the notrump fallbacks (a heart raise with four-card support still
    // wins on weight).
    if responder_major == Suit::Hearts && agreements.response.up_the_line {
        rules = rules.rule(Bid::new(1, Strain::Spades), 95, len(Suit::Spades, 4..));
    }
    // New lower suit: `1♦ - 1M - 2♣` on four-plus clubs, ahead of the `2♦`
    // rebid (0.9) unless diamonds are six-plus.
    if opener_minor == Suit::Diamonds && agreements.rebid.one_diamond_two_clubs {
        rules = rules.rule(
            Bid::new(2, Strain::Clubs),
            91,
            len(Suit::Clubs, 4..) & len(Suit::Diamonds, ..=5),
        );
    }
    rules = with_passed_hand_pass(rules, &agreements.rebid);
    // Odwrotka (default off): `2♦!` as the artificial reverse over `1♣ - 1M`.
    rules = with_odwrotka(rules, opener_minor, agreements);
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

/// Responder's preference after `1♦ - 1M - 2♣`
///
/// A weak responder returns to `2♦` with diamonds at least as long as clubs —
/// the 5-2 over the 4-2.  Deliberately partial: every other hand (longer
/// clubs, a six-card major, invitational values) rejects and falls through to
/// the floor, which the A/B showed plays them well; only its pass of `2♣`
/// with diamond preference lost (2026-09-27, 652 boards).
fn responder_after_one_diamond_two_clubs(major: Suit) -> Rules {
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        100,
        !longer_suit(Suit::Clubs, Suit::Diamonds)
            & len(Suit::Diamonds, 2..)
            & len(major, ..=5)
            & hcp(..=10),
    )
}

/// Responder's preference after the `1♦ - 1M - 2♣` new-suit rebid
pub(super) fn one_diamond_two_clubs_preference() -> Package {
    Package {
        name: "one-diamond-two-clubs-preference",
        gate: |a| a.rebid.one_diamond_two_clubs,
        entries: |_| {
            expand(
                "P* 1♦ - 1M - 2♣ -",
                |_| true,
                |b| responder_after_one_diamond_two_clubs(b.suit('M')),
            )
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
            odwrotka_continuations(),
            invitational_minor_continuations(),
            major_jump_rebid_continuations(),
            minor_jump_notrump_slam_continuations(),
            forcing_nt_two_suiter_continuations(),
            forcing_nt_jump_shift_continuations(),
            meckstroth_two_notrump_continuations(),
            one_heart_one_spade_rebid(),
            one_diamond_two_clubs_preference(),
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
