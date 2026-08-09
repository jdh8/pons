//! Opener's strength-showing rebid ladder after a one-level response
//!
//! Three rungs above the minimum natural rebid — jump-rebid, reverse,
//! jump-shift — disjoint from it by crisp point bands.  Gated by
//! [`set_opener_extras_ladder`]; the matching `Inferences` reading gates on the
//! same knob.  Only the two minor-opening rebid nodes carry the full ladder;
//! the major-opening nodes carry the jump-rebid rung alone (see
//! [`super::major_jump_rebid`]).

use super::*;

// ponytail: same construction-time toggle idiom as the Meckstroth adjunct —
// read during `register()`, set it before building the `Pair`.
std::thread_local! {
    /// Whether opener's rebid tables carry the **strength-showing ladder**
    /// after a minor opening and a one-level response: a jump-rebid of opener's
    /// suit, a reverse, and a jump-shift.  Shipped **on** (BBA-gap bucket #3).
    static OPENER_EXTRAS_LADDER: Cell<bool> = const { Cell::new(true) };
}

/// Enable opener's strength-showing rebid ladder in books built after this call
///
/// After a one-level response, opener's only long-suit rebid is a minimum
/// natural `2m`/`2M` with no upper bound (weight 0.9, `len(..5..)`), so a strong
/// single- or two-suiter underbids and the auction dies below game — the
/// largest un-worked lever in the Constructive/book/round-2 anchor bucket.  This
/// adds three rungs above the minimum, disjoint from it by crisp point bands:
///
/// - **Jump-rebid** of opener's suit (`1♦ - 1♠ - 3♦`): a self-sufficient 6+
///   suit, 16+ points, invitational.
/// - **Reverse** into a higher new suit (`1♦ - 1♠ - 2♥`): 5+ first suit, 4+
///   second, 17+ points, forcing.
/// - **Jump-shift** into a new suit (`1♦ - 1♠ - 3♣`): 5-4, 18+ points,
///   game-forcing.
///
/// Read at book-construction time; shipped default-on (+0.0203/+0.0332 plain,
/// +0.0181/+0.0297 PD IMPs/board vs BBA, NV/vul, all CIs>0).  The matching
/// [`Inferences`](crate::bidding::inference) reading gates on the same toggle,
/// narrowing each rung's shape and strength.  The two minor-opening rebid nodes
/// carry the full ladder; the major-opening nodes carry the jump-rebid rung
/// alone (see [`set_opener_major_jump_rebid`](super::set_opener_major_jump_rebid)).
pub fn set_opener_extras_ladder(on: bool) {
    OPENER_EXTRAS_LADDER.with(|cell| cell.set(on));
}

/// Whether opener's strength-showing rebid ladder is currently enabled
///
/// Read at book-construction time by `register`, and at classify time by the
/// matching `Inferences` reading.
pub(crate) fn opener_extras_ladder() -> bool {
    OPENER_EXTRAS_LADDER.with(Cell::get)
}

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
