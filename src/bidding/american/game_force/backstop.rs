//! The retired wildcard game backstop
//!
//! Gated by [`GameForceKnobs::game_backstop`].  **Off by
//! default**: every 2/1 continuation the three authored rounds do not cover
//! falls through to the floor rather than to this table's three crude rules.

use super::*;
use crate::bidding::fallback::Undisturbed;

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
            70,
            described("our side bid ♥", |_, ctx| ctx.we_bid(Strain::Hearts))
                & len(Suit::Hearts, 3..),
        )
        .rule(
            call(4, Strain::Spades),
            70,
            described("our side bid ♠", |_, ctx| ctx.we_bid(Strain::Spades))
                & len(Suit::Spades, 3..),
        )
        .rule(call(3, Strain::Notrump), 50, hcp(0..))
}

/// The retired wildcard game backstops, preserved verbatim behind their knob
pub(crate) fn backstops() -> Package {
    Package {
        name: "two-over-one-game-backstop",
        gate: |agreements| agreements.game_force.game_backstop,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Spades, Suit::Hearts] {
                for resp in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(resp) >= Strain::from(major) {
                        continue;
                    }
                    let key = format!(
                        "{} - {} -",
                        call(1, Strain::from(major)),
                        call(2, Strain::from(resp)),
                    );
                    entries.push(classified(
                        Pattern::guarded(&key, "2NT -", Undisturbed).with_fan(2),
                        game_backstop(),
                    ));
                }
            }
            entries.push(classified(
                Pattern::guarded("1♦ - 2♣ -", "2NT -", Undisturbed).with_fan(2),
                game_backstop(),
            ));
            entries
        },
    }
}
