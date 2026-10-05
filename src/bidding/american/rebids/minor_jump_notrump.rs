//! Responder's slam try over opener's `3NT` on a six-card minor
//!
//! [`RebidKnobs::minor_jump_notrump`] sends a six-card minor with 18–21 HCP to
//! `3NT` over `1m - 1x`.  Responder's floor answers it on points alone — `6NT`
//! on 13+, else a pass — so a 9–12 count with a fit plays `3NT` when the minor
//! slam is on.  This package adds BBA's channel: `4m` raises opener's minor as
//! a slam try, and opener asks keycards on a maximum or signs off in `5m`.
//! Gated by [`RebidKnobs::minor_jump_notrump_slam_try`].  Default on at 9 HCP.

use super::*;
use crate::bidding::american::slam;

/// Responder's `4m` slam try over `1m - 1x - 3NT`
///
/// No catch-all: a hand the try rejects falls through to the floor, which
/// keeps its `6NT` on 13+, its pass, and its own-suit games.  The 12-HCP
/// ceiling leaves 13+ to that `6NT`; the six-card cap leaves a long suit of
/// responder's own to the floor's game in it.
fn slam_try(minor: Suit, responder: Suit, floor: u8) -> Rules {
    Rules::new().rule(
        Bid::new(4, Strain::from(minor)),
        100,
        len(minor, 2..) & hcp(floor..=12) & len(responder, ..6),
    )
}

/// Opener's answer to the `4m` slam try: `4NT` keycard on 20+, else `5m`
///
/// ponytail: a fixed 20-HCP accept, the top two counts of the 18–21 band; the
/// anchor's losses concentrate there.  Make it a knob if a sweep wants it.
fn slam_answer(minor: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(20..))
        .alert(slam::RKCB)
        .rule(Bid::new(5, Strain::from(minor)), 90, hcp(..20))
}

/// The `4m` slam try, opener's answer, and the RKCB subtree below it
pub(crate) fn minor_jump_notrump_slam_continuations() -> Package {
    Package {
        name: "minor-jump-notrump-slam",
        gate: |a| a.rebid.minor_jump_notrump && a.rebid.minor_jump_notrump_slam_try.is_some(),
        entries: |agreements| {
            let Some(floor) = agreements.rebid.minor_jump_notrump_slam_try else {
                return Vec::new();
            };
            let mut entries = Vec::new();
            for (minor, responder) in [
                (Suit::Clubs, Suit::Diamonds),
                (Suit::Clubs, Suit::Hearts),
                (Suit::Clubs, Suit::Spades),
                (Suit::Diamonds, Suit::Hearts),
                (Suit::Diamonds, Suit::Spades),
            ] {
                let (m, x) = (Strain::from(minor), Strain::from(responder));
                let prefix = format!("P* 1{m} - 1{x} - 3NT -");
                entries.extend(rows_of(
                    Pattern::node(&prefix),
                    slam_try(minor, responder, floor),
                ));
                for tail in ["-", "(X)"] {
                    let path = format!("{prefix} 4{m} {tail}");
                    entries.extend(rows_of(Pattern::node(&path), slam_answer(minor)));
                    entries.extend(slam::rkcb_rows(&path, minor));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
