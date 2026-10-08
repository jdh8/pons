//! Opener's natural jump shift over `1♥ - 1♠` and responder's answers
//!
//! Gated by [`RebidKnobs::heart_spade_jump_shift`] (default on, shipped
//! 2026-10-08).  Opener's
//! `3♣` / `3♦` shows exactly five hearts, 4+ in the minor and 18+ points,
//! game-forcing — the minor openings' rung (`extras_ladder.rs`) carried over,
//! so the `2m` rebid keeps 12–17.  Six hearts keep the `3♥` jump rebid: the
//! first cut (5+ hearts) took them from it and lost −66 / −62 plain IMPs on
//! 35 / 37 boards (none / both) to responder's `3NT` on a doubleton heart.  The floor has no forcing channel and passes the jump
//! shift with a weak hand, so responder's whole node is authored, every call
//! past `3m`:
//!
//! | Call | Meaning |
//! |------|---------|
//! | `4NT` | RKCB for hearts: 3+ hearts, 14+ points |
//! | `4♥`  | 3+ hearts |
//! | `4m`  | 4+ in opener's minor, 14+ points: opener asks keycards in it |
//! | `3♠`  | 6+ spades: opener bids `4♠` on 2+, `4♥` on six, else `3NT` |
//! | `5m`  | 4+ in opener's minor, at most one in the other minor |
//! | `3NT` | everything else |

use super::*;
use crate::bidding::american::slam;

/// Append the `3♣` / `3♦` jump shift to opener's `1♥ - 1♠` rebid table
///
/// Weight 1.70 as in the minor ladder: above the 18–19 `2NT` (1.20), below
/// the spade raises (1.80+); disjoint from the `3♥` jump rebid by length.
pub(super) fn with_heart_spade_jump_shift(mut rules: Rules, knobs: &RebidKnobs) -> Rules {
    if !knobs.heart_spade_jump_shift {
        return rules;
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(minor)),
                170,
                len(Suit::Hearts, 5..=5) & len(minor, 4..) & points(18..),
            )
            .alert(OPENER_JUMP_SHIFT);
    }
    rules
}

/// Responder over `1♥ - 1♠ - 3m`: the table in the module doc
fn responder(minor: Suit) -> Rules {
    let other = if minor == Suit::Clubs {
        Suit::Diamonds
    } else {
        Suit::Clubs
    };
    let m = Strain::from(minor);
    Rules::new()
        .rule(
            Bid::new(4, Strain::Notrump),
            160,
            len(Suit::Hearts, 3..) & points(14..),
        )
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Hearts), 150, len(Suit::Hearts, 3..))
        .rule(Bid::new(4, m), 140, len(minor, 4..) & points(14..))
        .rule(Bid::new(3, Strain::Spades), 130, len(Suit::Spades, 6..))
        .rule(Bid::new(5, m), 120, len(minor, 4..) & len(other, ..=1))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Opener over responder's `4m` raise: always the keycard ask (18+ opposite
/// 14+)
fn opener_over_raise() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(0..))
        .alert(slam::RKCB)
}

/// Opener over responder's six-card `3♠`
fn opener_over_spades() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 100, len(Suit::Spades, 2..))
        .rule(Bid::new(4, Strain::Hearts), 90, len(Suit::Hearts, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0, hcp(0..))
}

/// Responder's answers, opener's two answers and both RKCB subtrees
pub(crate) fn heart_spade_jump_shift_continuations() -> Package {
    Package {
        name: "heart-spade-jump-shift",
        gate: |a| a.rebid.heart_spade_jump_shift,
        entries: |_| {
            let mut entries = Vec::new();
            for minor in [Suit::Clubs, Suit::Diamonds] {
                let m = Strain::from(minor);
                for tail in ["-", "(X)"] {
                    let node = format!("P* 1♥ - 1♠ - 3{m} {tail}");
                    entries.extend(rows_of(Pattern::node(&node), responder(minor)));
                    entries.extend(slam::rkcb_rows(&node, Suit::Hearts));
                    for answer in ["-", "(X)"] {
                        let raise = format!("{node} 4{m} {answer}");
                        entries.extend(rows_of(Pattern::node(&raise), opener_over_raise()));
                        entries.extend(slam::rkcb_rows(&raise, minor));
                        let spades = format!("{node} 3♠ {answer}");
                        entries.extend(rows_of(Pattern::node(&spades), opener_over_spades()));
                    }
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
