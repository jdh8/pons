//! 2/1 game-forcing continuations
//!
//! After a two-over-one response, the auction is game forcing: neither player
//! may pass below game.  This module registers the decision tables for the
//! three rounds of the game-forcing auction (opener's rebid, responder's rebid,
//! opener's third call).
//!
//! This module is the **index** for the base tables and three gated agreements:
//!
//! | Module | Agreement | Knob |
//! | --- | --- | --- |
//! | [`backstop`] | the retired wildcard game backstop, default off | [`set_game_backstop`] |
//! | [`opener_third`] | opener's third call after responder sets trump at `1M - 2r - R - 3M` | [`set_opener_third`] |
//! | [`second_suit`] | opener's third call plus RKCB after responder raises opener's second suit | [`set_second_suit_agreement`] |
//!
//! # Forcing by omission
//!
//! None of the tables here carry a [`Pass`][contract_bridge::auction::Call::Pass]
//! rule.  That means the driver can never choose pass at these nodes — a bid
//! scores its weight, pass scores −∞.
//!
//! That holds the game force only where a table *exists*.  The floor owns
//! everything else, and it needs telling: see
//! [`set_two_over_one_force`][crate::bidding::instinct::set_two_over_one_force],
//! which marks an uncontested 2/1 forced to game so the floor takes the cheapest
//! game milestone rather than passing out a partscore.

use super::super::Trie;
use super::call;
use crate::bidding::Rules;
use crate::bidding::agreements::{Agreements, GameForceKnobs};
use crate::bidding::constraint::{
    balanced, described, fifths, hcp, len, partner_suit_is, points, support,
};
use crate::bidding::rows::{Package, Pattern, classified, compile_into, rows_of};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Level, Strain, Suit};
use std::cell::Cell;

mod backstop;
mod opener_third;
mod second_suit;

pub use backstop::set_game_backstop;
pub use opener_third::set_opener_third;
pub use second_suit::set_second_suit_agreement;

// The packages, re-exported so `american::tests::row_package_invariants` and
// `register` below name them at one path.
pub(super) use backstop::backstops;
pub(super) use opener_third::opener_third_continuations;
pub(super) use second_suit::second_suit_agreement_continuations;

/// Capture this area's build-time cells into [`GameForceKnobs`]
pub(in crate::bidding) fn capture() -> GameForceKnobs {
    GameForceKnobs {
        game_backstop: backstop::game_backstop_enabled(),
        opener_third: opener_third::opener_third_enabled(),
        second_suit_agreement: second_suit::second_suit_agreement(),
    }
}

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
pub(super) fn opener_rebid(major: Suit, resp: Suit) -> Rules {
    let major_strain = Strain::from(major);
    let resp_strain = Strain::from(resp);

    let mut rules = Rules::new()
        // Jump to 3M: solid six-card major.
        .rule(call(3, major_strain), 170, len(major, 6..) & points(15..))
        // Raise responder's suit.
        .rule(call(3, resp_strain), 160, support(4..))
        // Simple rebid of the major.
        .rule(call(2, major_strain), 140, len(major, 6..))
        // Balanced minimum (12–14) or balanced 18–19.
        .rule(
            call(2, Strain::Notrump),
            120,
            balanced() & (fifths(12.0..15.0) | fifths(18.0..20.0)),
        );

    // New suits x ∉ {major, resp}.  Collect them in ascending strain order
    // and assign weights: 1.0 / 0.95 when at the 2 level, 0.9 at the 3 level.
    let other_suits: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter(|&x| x != major && x != resp)
        .collect();

    // Partition into 2-level and 3-level candidates.
    let mut two_level_weight = 100;
    for &x in &other_suits {
        let x_strain = Strain::from(x);
        if x_strain > resp_strain {
            // Above resp → can be bid at the 2 level.
            rules = rules.rule(call(2, x_strain), two_level_weight, len(x, 4..));
            two_level_weight -= 5;
        }
    }
    for &x in &other_suits {
        let x_strain = Strain::from(x);
        if x_strain < resp_strain {
            // Below resp → must be bid at the 3 level.
            rules = rules.rule(call(3, x_strain), 90, len(x, 4..));
        }
    }

    // Guaranteed-legal fallback: opener always has 5+ of the major.
    rules.rule(call(2, major_strain), 30, len(major, 5..))
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
        .rule(call(3, major_strain), 200, len(major, 3..))
        // Rebid own suit with six.
        .rule(call(3, resp_strain), 120, len(resp, 6..))
        // Raise to game on a direct 6-card rebid by opener.
        .rule(
            call(4, major_strain),
            100,
            partner_suit_is(major) & len(major, 2..),
        )
        // Default game.
        .rule(call(3, Strain::Notrump), 80, hcp(13..));

    // Raise each suit opener might have bid (x ∉ {major, resp}).
    for &x in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x != major && x != resp {
            let x_strain = Strain::from(x);
            rules = rules.rule(call(3, x_strain), 140, partner_suit_is(x) & support(4..));
        }
    }
    rules
}

// ---------------------------------------------------------------------------
// Minor game force: 1♦ - 2♣
// ---------------------------------------------------------------------------

/// Opener's rebid after 1♦ - 2♣
///
/// The 1♦ opening may be as short as three cards (better-minor), so no suit
/// rebid is guaranteed.  The 2NT rule at weight 0.2 is the safe fallback;
/// it ranges over all HCP so it fires whenever nothing better applies.
///
/// No [`Pass`][Call::Pass] rule.
fn opener_rebid_1d_2c() -> Rules {
    Rules::new()
        // Raise clubs.
        .rule(call(3, Strain::Clubs), 160, support(4..))
        // Balanced hand.
        .rule(
            call(2, Strain::Notrump),
            120,
            balanced() & (fifths(12.0..15.0) | fifths(18.0..20.0)),
        )
        // New four-card majors.
        .rule(call(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(call(2, Strain::Spades), 95, len(Suit::Spades, 4..))
        // Long diamonds.
        .rule(call(2, Strain::Diamonds), 100, len(Suit::Diamonds, 6..))
        // Guaranteed-legal fallback (opener may have only three diamonds).
        .rule(call(2, Strain::Notrump), 20, hcp(0..))
}

/// Responder's rebid after `1♦ - 2♣ - R`
///
/// No [`Pass`][Call::Pass] rule.
fn responder_rebid_1d_2c() -> Rules {
    Rules::new()
        // Raise opener's diamonds.
        .rule(
            call(3, Strain::Diamonds),
            120,
            partner_suit_is(Suit::Diamonds) & len(Suit::Diamonds, 4..),
        )
        // Rebid clubs.
        .rule(call(3, Strain::Clubs), 110, len(Suit::Clubs, 6..))
        // Default game.
        .rule(call(3, Strain::Notrump), 80, hcp(13..))
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Distinct calls from a source table, preserving first-rule order
///
/// Continuation keys stay derived from the live, knob-built source table.  A
/// `HashSet` is only the membership test; iterating the rules themselves keeps
/// the legacy declaration order.
pub(super) fn distinct_calls(rules: &Rules) -> Vec<Call> {
    let mut seen = std::collections::HashSet::new();
    rules
        .rules()
        .iter()
        .filter_map(|rule| {
            let call = rule.call();
            seen.insert(call).then_some(call)
        })
        .collect()
}

/// The ungated 2/1 decision rounds and major-suit RKCB answer trees
pub(super) fn base() -> Package {
    Package {
        name: "two-over-one-continuations",
        gate: |_| true,
        entries: |_| {
            let mut entries = Vec::new();

            // Five major 2/1 sequences: opener's rebid, responder's rebid
            // after every distinct source-table call, and the RKCB answers
            // below each possible 3M agreement.  The answers deliberately do
            // not ride the `opener_third_enabled` gate.
            for major in [Suit::Spades, Suit::Hearts] {
                for resp in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(resp) >= Strain::from(major) {
                        continue;
                    }
                    let prefix = format!(
                        "P* {} - {} -",
                        call(1, Strain::from(major)),
                        call(2, Strain::from(resp)),
                    );
                    let rebid = opener_rebid(major, resp);
                    let rebid_calls = distinct_calls(&rebid);
                    entries.extend(rows_of(Pattern::node(&prefix), rebid));

                    let three_major = Bid::new(3, Strain::from(major));
                    for rebid_call in rebid_calls {
                        let after_rebid = format!("{prefix} {rebid_call} -");
                        entries.extend(rows_of(
                            Pattern::node(&after_rebid),
                            responder_rebid(major, resp),
                        ));
                        if let Call::Bid(rebid_bid) = rebid_call
                            && rebid_bid < three_major
                        {
                            let agreed =
                                format!("{after_rebid} {} -", call(3, Strain::from(major)),);
                            entries.extend(super::slam::rkcb_rows(&agreed, major));
                        }
                    }
                }
            }

            // The 1♦ - 2♣ minor game force: the same table-derived call set,
            // with no authored third round.
            let prefix = "P* 1♦ - 2♣ -";
            let rebid = opener_rebid_1d_2c();
            let rebid_calls = distinct_calls(&rebid);
            entries.extend(rows_of(Pattern::node(prefix), rebid));
            for rebid_call in rebid_calls {
                entries.extend(rows_of(
                    Pattern::node(&format!("{prefix} {rebid_call} -")),
                    responder_rebid_1d_2c(),
                ));
            }

            entries
        },
    }
}

/// Register all 2/1 game-forcing continuations into `book`
pub(super) fn register(book: &mut Trie, agreements: &Agreements) {
    compile_into(
        book,
        agreements,
        &[
            base(),
            opener_third_continuations(),
            second_suit_agreement_continuations(),
            backstops(),
        ],
    );
}
pub use backstop::game_backstop_enabled;
pub use second_suit::second_suit_agreement;
