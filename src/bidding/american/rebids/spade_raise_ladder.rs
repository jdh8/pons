//! Opener's spade raise ladder after `1x - 1♠` and responder's answers
//!
//! Gated by [`RebidKnobs::spade_raise_ladder`] (default on since 2026-11-08;
//! *Open work* item 9 of `docs/bba-gap-campaign.md`).  The default ladder raises on
//! 12–15 / 16–18 / 19+ points; this one moves each rung down to where BBA
//! raises and where the double-dummy census over 5.4M `1x - 1♠` raises puts
//! the IMP optimum:
//!
//! | Opener | Points | Responder |
//! |--------|--------|-----------|
//! | `2♠`   | 12–14  | the existing tables |
//! | `3♠`   | 15–16  | `4♠` on 7+, `4NT` RKCB on 14+, else pass |
//! | `4♠`   | 17+    | `4NT` RKCB on 12+, else pass |
//!
//! Responder's answers ride their own knob,
//! [`RebidKnobs::spade_raise_answers`] (default off: it washed on top of
//! the ranges and lost alone); the two RKCB floors are
//! [`RebidKnobs::spade_raise_three_ask`] and
//! [`RebidKnobs::spade_raise_four_ask`].
//! Responder's `3♠` and `4♠` nodes are authored after all three openings
//! (`1♣`, `1♦`, `1♥`); without them the minor lanes fall to the floor,
//! which jumps straight to `6♠`.  Their double is systems on (the same
//! table), and their overcall of `3♠` through `4♥` leaves responder bidding
//! `4♠` on the accepting range.

use super::*;
use crate::bidding::american::slam;

/// Point ranges of opener's `2♠` / `3♠` / `4♠` raise: the knob's ladder when
/// `low`, else the default 12–15 / 16–18 / 19+
pub(super) fn spade_raise_ranges(
    low: bool,
) -> (
    core::ops::RangeInclusive<u8>,
    core::ops::RangeInclusive<u8>,
    core::ops::RangeFrom<u8>,
) {
    if low {
        (12..=14, 15..=16, 17..)
    } else {
        (12..=15, 16..=18, 19..)
    }
}

/// Responder over opener's 15–16 `3♠`: RKCB from `ask` points
fn over_three_spades(ask: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 150, points(ask..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 100, points(7..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder over opener's 17+ `4♠`: RKCB from `ask` points
fn over_four_spades(ask: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 150, points(ask..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder over their overcall of our `3♠`: game on the accepting range
fn over_three_spades_overcalled() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 100, points(7..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Responder's `3♠` / `4♠` nodes after `1♣` / `1♦` / `1♥ - 1♠`, both tails
pub(crate) fn spade_raise_ladder_continuations() -> Package {
    Package {
        name: "spade-raise-ladder",
        gate: |a| a.rebid.spade_raise_answers,
        entries: |agreements| {
            let (three_ask, four_ask) = (
                agreements.rebid.spade_raise_three_ask,
                agreements.rebid.spade_raise_four_ask,
            );
            let mut entries = Vec::new();
            for opening in ["1♣", "1♦", "1♥"] {
                let three = format!("P* {opening} - 1♠ - 3♠");
                let four = format!("P* {opening} - 1♠ - 4♠");
                for tail in ["-", "(X)"] {
                    let node = format!("{three} {tail}");
                    entries.extend(rows_of(Pattern::node(&node), over_three_spades(three_ask)));
                    entries.extend(slam::rkcb_rows(&node, Suit::Spades));
                    let node = format!("{four} {tail}");
                    entries.extend(rows_of(Pattern::node(&node), over_four_spades(four_ask)));
                    entries.extend(slam::rkcb_rows(&node, Suit::Spades));
                }
                entries.extend(rows_of(
                    Pattern::up_to(&three, "4♥"),
                    over_three_spades_overcalled(),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
