//! 2/1 game-forcing continuations
//!
//! After a two-over-one response, the auction is game forcing: neither player
//! may pass below game.  This module registers the decision tables for the
//! three rounds of the game-forcing auction (opener's rebid, responder's rebid,
//! opener's third call) and a game backstop fallback that keeps any uncovered
//! continuation alive.
//!
//! # Forcing by omission
//!
//! None of the tables here carry a [`Pass`][contract_bridge::auction::Call::Pass]
//! rule.  That means the driver can never choose pass at these nodes — a bid
//! scores its weight, pass scores −∞.

use super::fallback_all_seats;
use super::uncontested;
use super::{Trie, call};
use crate::bidding::Rules;
use crate::bidding::constraint::{
    balanced, described, fifths, hcp, len, partner_suit_is, points, support,
};
use crate::bidding::fallback::{Fallback, Undisturbed};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Major 2/1 sequences
// ---------------------------------------------------------------------------

/// Opener's rebid after a 2/1 game-forcing response
///
/// Tables every descriptive rebid: a jump to three of the major on a solid
/// six-card suit, raising responder, rebidding the major, showing a balanced
/// minimum or maximum, and introducing a new suit.  A second rule for
/// two-of-the-major at weight 0.3 is the guaranteed-legal fallback — opener
/// always holds five of the major so the bid is always available.
///
/// No [`Pass`][Call::Pass] rule: the auction is game forcing.
fn opener_rebid(major: Suit, resp: Suit) -> Rules {
    let major_strain = Strain::from(major);
    let resp_strain = Strain::from(resp);

    let mut rules = Rules::new()
        // Jump to 3M: solid six-card major.
        .rule(call(3, major_strain), 1.7, len(major, 6..) & points(15..))
        // Raise responder's suit.
        .rule(call(3, resp_strain), 1.6, support(4..))
        // Simple rebid of the major.
        .rule(call(2, major_strain), 1.4, len(major, 6..))
        // Balanced minimum (12–14) or balanced 18–19.
        .rule(
            call(2, Strain::Notrump),
            1.2,
            balanced() & (fifths(12.0..15.0) | fifths(18.0..20.0)),
        );

    // New suits x ∉ {major, resp}.  Collect them in ascending strain order
    // and assign weights: 1.0 / 0.95 when at the 2 level, 0.9 at the 3 level.
    let other_suits: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter(|&x| x != major && x != resp)
        .collect();

    // Partition into 2-level and 3-level candidates.
    let mut two_level_weight = 1.0_f32;
    for &x in &other_suits {
        let x_strain = Strain::from(x);
        if x_strain > resp_strain {
            // Above resp → can be bid at the 2 level.
            rules = rules.rule(call(2, x_strain), two_level_weight, len(x, 4..));
            two_level_weight -= 0.05;
        }
    }
    for &x in &other_suits {
        let x_strain = Strain::from(x);
        if x_strain < resp_strain {
            // Below resp → must be bid at the 3 level.
            rules = rules.rule(call(3, x_strain), 0.9, len(x, 4..));
        }
    }

    // Guaranteed-legal fallback: opener always has 5+ of the major.
    rules.rule(call(2, major_strain), 0.3, len(major, 5..))
}

/// Responder's rebid after opener has rebid at the two-over-one node
///
/// Registered at each distinct bid call `R` that appears in
/// [`opener_rebid`] for the same `(major, resp)` pair.  The four-trump
/// agreement (3M) takes priority; raising opener's second suit, rebidding
/// own suit, raising opener's 6-card rebid, and the default 3NT game
/// follow in order.
///
/// No [`Pass`][Call::Pass] rule: the auction is still game forcing.
fn responder_rebid(major: Suit, resp: Suit) -> Rules {
    let major_strain = Strain::from(major);
    let resp_strain = Strain::from(resp);

    let mut rules = Rules::new()
        // Sets trump: at least three-card support for opener's major.
        .rule(call(3, major_strain), 2.0, len(major, 3..))
        // Rebid own suit with six.
        .rule(call(3, resp_strain), 1.2, len(resp, 6..))
        // Raise to game on a direct 6-card rebid by opener.
        .rule(
            call(4, major_strain),
            1.0,
            partner_suit_is(major) & len(major, 2..),
        )
        // Default game.
        .rule(call(3, Strain::Notrump), 0.8, hcp(13..));

    // Raise each suit opener might have bid (x ∉ {major, resp}).
    for &x in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x != major && x != resp {
            let x_strain = Strain::from(x);
            rules = rules.rule(call(3, x_strain), 1.4, partner_suit_is(x) & support(4..));
        }
    }
    rules
}

/// Opener's third call after 1M–2r–R–3M
///
/// Once trump has been set at three of the major, opener shows strength:
/// the 4NT key card ask on extras or a sign-off at four of the major.
///
/// No [`Pass`][Call::Pass] rule.
fn opener_third(major: Suit) -> Rules {
    let major_strain = Strain::from(major);
    Rules::new()
        .rule(call(4, Strain::Notrump), 1.0, points(15..))
        .alert(super::slam::RKCB)
        .rule(call(4, major_strain), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Minor game force: 1♦–2♣
// ---------------------------------------------------------------------------

/// Opener's rebid after 1♦–2♣
///
/// The 1♦ opening may be as short as three cards (better-minor), so no suit
/// rebid is guaranteed.  The 2NT rule at weight 0.2 is the safe fallback;
/// it ranges over all HCP so it fires whenever nothing better applies.
///
/// No [`Pass`][Call::Pass] rule.
fn opener_rebid_1d_2c() -> Rules {
    Rules::new()
        // Raise clubs.
        .rule(call(3, Strain::Clubs), 1.6, support(4..))
        // Balanced hand.
        .rule(
            call(2, Strain::Notrump),
            1.2,
            balanced() & (fifths(12.0..15.0) | fifths(18.0..20.0)),
        )
        // New four-card majors.
        .rule(call(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(call(2, Strain::Spades), 0.95, len(Suit::Spades, 4..))
        // Long diamonds.
        .rule(call(2, Strain::Diamonds), 1.0, len(Suit::Diamonds, 6..))
        // Guaranteed-legal fallback (opener may have only three diamonds).
        .rule(call(2, Strain::Notrump), 0.2, hcp(0..))
}

/// Responder's rebid after 1♦–2♣–R
///
/// No [`Pass`][Call::Pass] rule.
fn responder_rebid_1d_2c() -> Rules {
    Rules::new()
        // Raise opener's diamonds.
        .rule(
            call(3, Strain::Diamonds),
            1.2,
            partner_suit_is(Suit::Diamonds) & len(Suit::Diamonds, 4..),
        )
        // Rebid clubs.
        .rule(call(3, Strain::Clubs), 1.1, len(Suit::Clubs, 6..))
        // Default game.
        .rule(call(3, Strain::Notrump), 0.8, hcp(13..))
}

// ---------------------------------------------------------------------------
// Game backstop
// ---------------------------------------------------------------------------

/// Default game bid for any uncovered game-forcing continuation
///
/// When the auction is already in the trump suit we play game there; otherwise
/// 3NT is the default.  No [`Pass`][Call::Pass] rule — at nodes where every
/// rule is illegal (game already bid) the driver passes, which is correct.
fn game_backstop() -> Rules {
    Rules::new()
        .rule(
            call(4, Strain::Hearts),
            0.7,
            described("our side bid ♥", |_, ctx| ctx.we_bid(Strain::Hearts))
                & len(Suit::Hearts, 3..),
        )
        .rule(
            call(4, Strain::Spades),
            0.7,
            described("our side bid ♠", |_, ctx| ctx.we_bid(Strain::Spades))
                & len(Suit::Spades, 3..),
        )
        .rule(call(3, Strain::Notrump), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register all 2/1 game-forcing continuations into `book`
///
/// Iterates over the five major 2/1 pairs (♠–♣, ♠–♦, ♠–♥, ♥–♣, ♥–♦), the
/// 1♦–2♣ minor game force, and installs three rounds of decision tables plus a
/// game backstop at each.
pub(super) fn register(book: &mut Trie) {
    // Five major 2/1 sequences.
    for &major in &[Suit::Spades, Suit::Hearts] {
        for &resp in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
            if Strain::from(resp) >= Strain::from(major) {
                // resp must be below major to be a 2/1 response.
                continue;
            }
            register_major(book, major, resp);
        }
    }

    // Minor game force: 1♦–2♣.
    register_minor(book);
}

/// Register tables for one major 2/1 pair
fn register_major(book: &mut Trie, major: Suit, resp: Suit) {
    let major_strain = Strain::from(major);
    let resp_strain = Strain::from(resp);

    let opener_calls = &[call(1, major_strain), call(2, resp_strain)];

    // Round 1: opener's rebid.
    let rebid = opener_rebid(major, resp);
    let rebid_calls: Vec<Call> = {
        let mut seen = std::collections::HashSet::new();
        rebid
            .rules()
            .iter()
            .filter_map(|r| {
                let c = r.call();
                if seen.insert(c) { Some(c) } else { None }
            })
            .collect()
    };
    super::insert_uncontested(book, opener_calls, rebid);

    let three_m_bid = Bid::new(3, major_strain);

    // Round 2: responder's rebid after each distinct opener bid R.
    for &r_call in &rebid_calls {
        let resp_calls = &[call(1, major_strain), call(2, resp_strain), r_call];
        super::insert_uncontested(book, resp_calls, responder_rebid(major, resp));

        // Round 3: opener's third call after 3M (only when R is a bid below 3M).
        if let Call::Bid(r_bid) = r_call
            && r_bid < three_m_bid
        {
            let third_calls = &[
                call(1, major_strain),
                call(2, resp_strain),
                r_call,
                call(3, major_strain),
            ];
            super::insert_uncontested(book, third_calls, opener_third(major));
            super::slam::install_rkcb(book, third_calls, major);
        }
    }

    // Game backstop: anchor at the 2/1 response node, guard = Undisturbed.
    let anchor = uncontested(opener_calls);
    fallback_all_seats(
        book,
        &anchor,
        2,
        Arc::new(Undisturbed),
        Fallback::classify(game_backstop()),
    );
}

/// Register tables for the 1♦–2♣ minor game force
fn register_minor(book: &mut Trie) {
    let opener_calls = &[call(1, Strain::Diamonds), call(2, Strain::Clubs)];

    // Opener's rebid.
    let rebid = opener_rebid_1d_2c();
    let rebid_calls: Vec<Call> = {
        let mut seen = std::collections::HashSet::new();
        rebid
            .rules()
            .iter()
            .filter_map(|r| {
                let c = r.call();
                if seen.insert(c) { Some(c) } else { None }
            })
            .collect()
    };
    super::insert_uncontested(book, opener_calls, rebid);

    // Responder's rebid after each distinct opener bid R.
    for &r_call in &rebid_calls {
        let resp_calls = &[call(1, Strain::Diamonds), call(2, Strain::Clubs), r_call];
        super::insert_uncontested(book, resp_calls, responder_rebid_1d_2c());
    }

    // Game backstop.
    let anchor = uncontested(opener_calls);
    fallback_all_seats(
        book,
        &anchor,
        2,
        Arc::new(Undisturbed),
        Fallback::classify(game_backstop()),
    );
}
