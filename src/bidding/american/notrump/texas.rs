//! Texas transfers — `4♦`/`4♥`, and the slam drive above them
//!
//! The direct game transfer, right-siding a `4M` contract, plus the
//! [`NotrumpKnobs::texas_slam_drive`][crate::bidding::agreements::NotrumpKnobs::texas_slam_drive] continuation that keeps a slam try alive above it.
//! [`NotrumpKnobs::texas_game_floor`][crate::bidding::agreements::NotrumpKnobs::texas_game_floor] sets the strength at which Texas is taken.

use super::stayman_slam::slam_try_answer;
use super::*;

/// Responder's RKCB drive over opener's Texas completion (`1NT - 4♣ - 4♥ - 4NT` /
/// `1NT - 4♦ - 4♠ - 4NT`)
///
/// A 17+ six-card-major hand transferred at the four level and now keycards: `4NT`
/// is RKCB, the [`slam`] 1430 ladder (installed alongside) places the slam.  Weaker
/// (game-only) transfers match no rule and pass opener's `4M`.  Empty unless the
/// reroute is on ([`NotrumpKnobs::texas_slam_drive`][crate::bidding::agreements::NotrumpKnobs::texas_slam_drive]).
fn texas_slam_drive_rebid(agreements: &Agreements) -> Rules {
    if !agreements.notrump.texas_slam_drive {
        return Rules::new();
    }
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 140, hcp(16..))
        .alert(slam::RKCB)
}

/// The current game-blast floor, widened to the DSL's length type
pub(super) fn texas_game_floor(agreements: &Agreements) -> usize {
    usize::from(agreements.notrump.texas_game_floor)
}

/// The South African Texas game-blast strength gate for `major`:
/// `point_count + trump length ≥ T` (default `T = 14`).
///
/// Point count plus the full suit length, so a longer trump suit needs fewer
/// points: a 6-bagger blasts at 8 points, a 7-bagger at 7, an 8-bagger at 6.
/// (This is the Stayman [`fit_value`] less its 4-4-fit baseline, which is
/// meaningless for a one-suiter — here the whole suit is the trump length.)  The
/// `len` guards (`6+` in `major`, `≤4` in the other) live with the rule; this is
/// just the strength term.
pub(super) fn texas_strength_gate(
    major: Suit,
    agreements: &Agreements,
) -> Cons<impl Constraint + Clone + use<>> {
    let floor = texas_game_floor(agreements);
    described(
        "six-card-major game blast",
        move |hand: Hand, context: &Context<'_>| {
            // Fit-known: a 6-card major opposite 1NT's 2+ is an 8-card fit.
            // Side-suit shortness counts; the length term is explicit.
            let profile = context.reading_profile();
            let support = usize::from(support_point_count_in_on(
                profile.support_points,
                profile.point_scale,
                hand,
                major,
            ));
            support + hand[major].len() >= floor
        },
    )
}

/// Complete a four-level Texas transfer by bidding game in the anchor major
///
/// `4♣ → 4♥`, `4♦ → 4♠`.  Responder showed 6+ with game-no-slam values, so
/// opener simply names the game and declares.
fn complete_texas(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(4, Strain::from(into)), 100, hcp(0..))
}

/// South African Texas transfers, direct slam tries, and their RKCB subtrees
pub(crate) fn texas_transfers() -> Package {
    Package {
        name: "texas-transfers",
        gate: |_| true,
        entries: |_| {
            let heart_slam = "P* 1NT - 4♥ -";
            let spade_slam = "P* 1NT - 4♠ -";
            let mut entries = rows_of(Pattern::node("P* 1NT - 4♣ -"), complete_texas(Suit::Hearts));
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 4♦ -"),
                complete_texas(Suit::Spades),
            ));
            entries.extend(rows_of(Pattern::node(heart_slam), slam_try_answer()));
            entries.extend(rows_of(Pattern::node(spade_slam), slam_try_answer()));
            entries.extend(slam::rkcb_rows(heart_slam, Suit::Hearts));
            entries.extend(slam::rkcb_rows(spade_slam, Suit::Spades));
            entries
        },
    }
}

/// Responder's Texas slam drive and its RKCB subtrees
pub(crate) fn texas_drive() -> Package {
    Package {
        name: "texas-slam-drive",
        gate: |agreements| agreements.notrump.texas_slam_drive,
        entries: |agreements| {
            let heart_drive = "P* 1NT - 4♣ - 4♥ -";
            let spade_drive = "P* 1NT - 4♦ - 4♠ -";
            let mut entries = rows_of(
                Pattern::node(heart_drive),
                texas_slam_drive_rebid(agreements),
            );
            entries.extend(rows_of(
                Pattern::node(spade_drive),
                texas_slam_drive_rebid(agreements),
            ));
            entries.extend(slam::rkcb_rows(heart_drive, Suit::Hearts));
            entries.extend(slam::rkcb_rows(spade_drive, Suit::Spades));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
