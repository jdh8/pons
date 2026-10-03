//! Opener's rebid over the preemptive minor raise: `1m - 3m`
//!
//! Responder's `3m` is the weak inverted raise — five-card support, at most 9
//! support points.  Without a node the instinct floor's raise rung bids `4m`
//! on any 13+ hand, a level past the part score and off the `3NT` road.
//! Gated by [`ResponseKnobs::preemptive_minor_raise_pass`][crate::bidding::agreements::ResponseKnobs::preemptive_minor_raise_pass].

use super::*;
use crate::bidding::constraint::balanced;

/// Opener's rebid after `1m - 3m -`: pass a minimum, `3NT` on a balanced
/// maximum
///
/// | Call | Meaning |
/// |---|---|
/// | 3NT | Balanced, 17+ HCP |
/// | Pass | At most `ceiling` HCP |
///
/// Deliberately partial: an unbalanced hand above the ceiling rejects the
/// table and falls through to the floor, whose `4m`, splinter and keycard
/// ask are where BBA bids those hands (`4m` 16–20, the rest 18+).
#[must_use]
fn opener_after_preemptive_raise(ceiling: u8) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 100, balanced() & hcp(17..))
        .rule(Call::Pass, 50, hcp(..=ceiling))
}

/// Opener's rebid after `1♣ - 3♣` and `1♦ - 3♦`
pub(crate) fn preemptive_minor_raise_continuations() -> Package {
    Package {
        name: "preemptive-minor-raise-continuations",
        gate: |a| a.response.preemptive_minor_raise_pass.is_some(),
        entries: |agreements| {
            let Some(ceiling) = agreements.response.preemptive_minor_raise_pass else {
                return Vec::new();
            };
            let mut entries = Vec::new();
            for minor in [Suit::Clubs, Suit::Diamonds] {
                let trump = Strain::from(minor);
                let prefix = format!("P* {} - {} -", call(1, trump), call(3, trump));
                entries.extend(rows_of(
                    Pattern::node(&prefix),
                    opener_after_preemptive_raise(ceiling),
                ));
            }
            entries
        },
    }
}
