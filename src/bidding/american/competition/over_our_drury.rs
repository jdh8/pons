//! Competition over a passed hand's Drury `2♣!`
//!
//! Gated with the convention itself (`response.drury`).  Their double of
//! the `2♣!`, or of opener's `2♦!` relay, is systems on — stripped to a
//! pass, so opener answers the uncontested ladder in `raises/drury.rs` and
//! responder's rungs ride the same tree.
//! Their overcall gets a small natural ladder for opener; responder's
//! continuation is the floor's, which reads `2♣!` as three-plus trumps and
//! 10+ support points.
//!
//! Under `response.drury_splinters`, their double of opener's splinter or of
//! responder's interest step is systems on as well, and their overcall of
//! the splinter leaves responder bidding the game the splinter forced.

use super::*;
use crate::bidding::constraint::support_points;

/// Opener after their overcall of Drury: game on 16+, `3M` on a full
/// opening, `2M` on a minimum while the two level is still there, else pass
fn drury_overcalled_opener(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, trump), 200, support_points(major, 16..))
        // `3M` is legal under every overcall the guard admits except `(3♠)`
        // over hearts, where the cheapest heart bid is `4♥`.
        .rule(
            Bid::new(3, trump),
            150,
            !min_level_is(4, trump) & support_points(major, 13..=15),
        )
        .rule(
            Bid::new(2, trump),
            100,
            min_level_is(2, trump) & support_points(major, ..=12),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Competition over our Drury `2♣!` as a row package (`response.drury`)
pub(super) fn competition_over_drury_package() -> Package {
    Package {
        name: "competition-over-drury",
        gate: |agreements| agreements.response.drury,
        entries: |agreements| {
            let mut entries = Vec::new();
            for seat in ["- -", "- - -"] {
                for major in [Suit::Hearts, Suit::Spades] {
                    let key = format!("{seat} {} - 2♣", call(1, Strain::from(major)));
                    entries.push(rebase(Pattern::first(&key, "X"), ReplaceNext(Call::Pass)));
                    // Their double of opener's `2♦!` relay: systems on as
                    // well, or responder falls to the floor, which passes the
                    // doubled relay (the worst boards of the first clean run).
                    entries.push(rebase(
                        Pattern::first(&format!("{key} - 2♦"), "X"),
                        ReplaceNext(Call::Pass),
                    ));
                    entries.extend(rows_of(
                        Pattern::up_to(&key, "3♠"),
                        drury_overcalled_opener(major),
                    ));
                    if agreements.response.drury_splinters {
                        let game = Bid::new(4, Strain::from(major));
                        for (_, jump, step) in
                            crate::bidding::american::raises::drury_splinters(major)
                        {
                            let jumped = format!("{key} - {}", Call::Bid(jump));
                            entries.push(rebase(
                                Pattern::first(&jumped, "X"),
                                ReplaceNext(Call::Pass),
                            ));
                            entries.push(rebase(
                                Pattern::first(&format!("{jumped} - {}", Call::Bid(step)), "X"),
                                ReplaceNext(Call::Pass),
                            ));
                            // Below our game only: 4♦ over hearts, 4♥ over spades.
                            let below = if major == Suit::Hearts {
                                "4♦"
                            } else {
                                "4♥"
                            };
                            entries.extend(rows_of(
                                Pattern::up_to(&jumped, below),
                                Rules::new().rule(game, 100, hcp(0..)),
                            ));
                        }
                    }
                }
            }
            entries
        },
    }
}
