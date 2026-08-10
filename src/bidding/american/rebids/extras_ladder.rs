//! Opener's strength-showing rebid ladder after a one-level response
//!
//! Three rungs above the minimum natural rebid — jump-rebid, reverse,
//! jump-shift — disjoint from it by crisp point bands. Gated by
//! [`opener_extras_ladder`][crate::bidding::inference::ReadingProfile::opener_extras_ladder];
//! the matching `Inferences` reading gates on the same field. Only the two
//! minor-opening rebid nodes carry the full ladder; the major-opening nodes
//! carry the jump-rebid rung alone (see [`super::major_jump_rebid`]).

use super::*;

/// Append opener's strength-showing ladder to a one-level-response rebid table
///
/// `opener` is opener's opened suit, `highest` responder's one-level call, and
/// `responder` responder's suit when they bid one (a forcing `1NT` bids none).
/// The weights sit above the minimum natural rebid (0.9) but below the
/// support-raises (1.8+), so a hand with four-card support for responder still
/// raises and only a genuine extras hand takes the ladder; the crisp point
/// bands keep a minimum on the natural rebid.  All three rungs name a real
/// suit — natural, unalerted, floor-safe — so the reading (`inference.rs`)
/// narrows their shape and strength rather than an alert projecting it.
pub(super) fn with_extras_ladder(
    mut rules: Rules,
    opener: Suit,
    highest: Bid,
    responder: Option<Suit>,
    agreements: &Agreements,
) -> Rules {
    if !agreements.decision.reading.opener_extras_ladder() {
        return rules;
    }
    let opener_strain = Strain::from(opener);
    // Jump-rebid of opener's suit: a self-sufficient 6+ suit with extras.
    let jump_rebid_level = cheapest_level_over(highest, opener_strain) + 1;
    if jump_rebid_level <= 3 {
        rules = rules.rule(
            Bid::new(jump_rebid_level, opener_strain),
            150,
            len(opener, 6..) & points(16..),
        );
    }
    for second in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if second == opener || responder == Some(second) {
            continue;
        }
        let second_strain = Strain::from(second);
        let cheapest = cheapest_level_over(highest, second_strain);
        // Reverse: a non-jump two-level new suit ranking above opener's,
        // forcing partner past a return to opener's suit at the two level.
        // Alerted: the rule floors opener's (unbid-here) first suit, so it is
        // artificial by the house rule and decoded by rule projection.
        if cheapest == 2 && second_strain > opener_strain {
            rules = rules
                .rule(
                    Bid::new(2, second_strain),
                    160,
                    len(opener, 5..) & len(second, 4..) & points(17..),
                )
                .alert(OPENER_REVERSE);
        }
        // Jump-shift: a single jump in a new suit, game-forcing.  18+ rather
        // than 19+ so a shapely two-suiter (a 5-5 with controls upgrades past
        // the band) is not stranded in the minimum rebid.  Alerted for the same
        // reason as the reverse (it floors opener's first suit).
        let jump_shift_level = cheapest + 1;
        if jump_shift_level <= 3 {
            rules = rules
                .rule(
                    Bid::new(jump_shift_level, second_strain),
                    170,
                    len(opener, 5..) & len(second, 4..) & points(18..),
                )
                .alert(OPENER_JUMP_SHIFT);
        }
    }
    rules
}

#[cfg(test)]
mod tests;
