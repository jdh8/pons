//! The retired wildcard game backstop
//!
//! Gated by [`set_game_backstop`].  Read at book-construction time.  **Off by
//! default**: every 2/1 continuation the three authored rounds do not cover
//! falls through to the floor rather than to this table's three crude rules.

use super::*;
use crate::bidding::fallback::Undisturbed;

std::thread_local! {
    /// Whether the game backstop ([`game_backstop`]) is registered at all.
    /// **Off by default** since 2026-07-20 — the floor answers these nodes
    /// better than the table did; see [`set_game_backstop`].
    static GAME_BACKSTOP: Cell<bool> = const { Cell::new(false) };
}

/// Re-register the game backstop over uncovered game-forcing continuations
///
/// Read at book-construction time.  **Off by default**: every 2/1 continuation
/// the three authored rounds do not cover falls through to the floor rather
/// than to this table's three crude rules.
///
/// The backstop was authored against the deterministic `instinct()` ladder; the
/// floor became the BBA-distilled net on 2026-07-19, and the table stopped
/// earning its keep.  Deleting it measures **+0.0117/+0.0142 plain,
/// +0.0132/+0.0160 PD** IMPs/board NV/vul vs BBA (409,600×2, all CI>0) *paired
/// with* [`set_two_over_one_force`][crate::bidding::instinct::set_two_over_one_force],
/// which restores by rule the game force this node used to hold by omission.
/// On alone the deletion is worth only +0.005, because the floor then abandons
/// partner's 2/1 on 24% of the boards it touches.
///
/// Deleting it also cures a replay-sampler starvation: the table is *partial*,
/// so every call it does not name sat at −∞ while its unconditional 3NT kept the
/// node's best finite, and the gate rejected those calls for every hand
/// (`sample_layouts_replay` returned 0%).  With no node the floor answers,
/// `authored_at` is false, and the gate abstains.  Kept as a knob so the table
/// can be re-measured if the floor changes again.
pub fn set_game_backstop(on: bool) {
    GAME_BACKSTOP.with(|cell| cell.set(on));
}

fn game_backstop_enabled() -> bool {
    GAME_BACKSTOP.with(Cell::get)
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
        gate: game_backstop_enabled,
        entries: || {
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
