//! The six-card major invitation after a transfer
//!
//! Responder's `3M` rebid over the completion: a six-card suit and invitational
//! values.  Both floors are knobs —  [`NotrumpKnobs::sixcard_invite_floor`][crate::bidding::agreements::NotrumpKnobs::sixcard_invite_floor] for
//! responder, [`NotrumpKnobs::sixcard_accept_floor`][crate::bidding::agreements::NotrumpKnobs::sixcard_accept_floor] for opener.

use super::texas::texas_game_floor;
use super::*;

/// The current six-card-major game-invite floor, widened to the DSL's length type
fn sixcard_invite_floor(agreements: &Agreements) -> usize {
    usize::from(agreements.notrump.sixcard_invite_floor)
}

/// Opener's current accept floor, widened to the DSL's length type
fn sixcard_accept_floor(agreements: &Agreements) -> usize {
    usize::from(agreements.notrump.sixcard_accept_floor)
}

/// Whether the six-card-major invite is authored: its floor sits below the Texas
/// game-blast floor, so the invitational band `[invite, blast)` is non-empty.
pub(super) fn sixcard_invite_active(agreements: &Agreements) -> bool {
    sixcard_invite_floor(agreements) < texas_game_floor(agreements)
}

/// Responder's invitational jump after a Jacoby transfer completes, holding a
/// six-card major just below the Texas game-blast floor (`1NT - 2♦ - 2♥ - 3♥` /
/// `1NT - 2♥ - 2♠ - 3♠`)
///
/// A natural invitational raise of responder's own suit: 6+ in `major`, ≤4 in the
/// other, and `point_count + length` at or above the invite floor.  No upper
/// bound is needed — the blast hands (`≥ 14`) jumped straight to `4♣/4♦` and never
/// transferred, so only the `[invite, 14)` band reaches here.  Opener then accepts
/// game or passes `3M` ([`accept_sixcard_invitation`]).  Empty unless the invite
/// is on ([`NotrumpKnobs::sixcard_invite_floor`][crate::bidding::agreements::NotrumpKnobs::sixcard_invite_floor]).  Natural — floors only its own strain, so
/// it stays unalerted (the artificial-alert invariant).
pub(super) fn sixcard_invite_rebid(major: Suit, agreements: &Agreements) -> Rules {
    if !sixcard_invite_active(agreements) {
        return Rules::new();
    }
    let floor = sixcard_invite_floor(agreements);
    Rules::new().rule(
        Bid::new(3, Strain::from(major)),
        130,
        len(major, 6..)
            & len(other_major(major), ..5)
            & described(
                "six-card invitational value",
                move |hand: Hand, context: &Context<'_>| {
                    // Fit-known: 6-card major opposite 1NT's 2+ is an 8-card fit.
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
            ),
    )
}

/// Opener's accept/decline of the six-card-major game invite (`…3M`)
///
/// Accept (`4M`) when `point_count + trump length` reaches
/// [`NotrumpKnobs::sixcard_accept_floor`][crate::bidding::agreements::NotrumpKnobs::sixcard_accept_floor]'s value (default 18); otherwise pass `3M`.
/// Authored because the keyless floor reads a three-level raise as forcing and so
/// could not decline.
fn accept_sixcard_invitation(major: Suit, agreements: &Agreements) -> Rules {
    let floor = sixcard_accept_floor(agreements);
    Rules::new()
        .rule(
            Bid::new(4, Strain::from(major)),
            100,
            described(
                "accept six-card invite",
                move |hand: Hand, context: &Context<'_>| {
                    // Fit-known: responder showed six, opener has 2+ — an 8-card fit.
                    // A doubleton trump holding earns no phantom ruffing value —
                    // the corner where the suit-indexed scale measurably won.
                    let profile = context.reading_profile();
                    let support = usize::from(support_point_count_in_on(
                        profile.support_points,
                        profile.point_scale,
                        hand,
                        major,
                    ));
                    support + hand[major].len() >= floor
                },
            ),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Opener's accept-or-decline tables for six-card-major invitations
pub(crate) fn sixcard_invite() -> Package {
    Package {
        name: "six-card-major-invite",
        gate: |agreements| sixcard_invite_active(agreements),
        entries: |agreements| {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ - 3♥ -"),
                accept_sixcard_invitation(Suit::Hearts, agreements),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♥ - 2♠ - 3♠ -"),
                accept_sixcard_invitation(Suit::Spades, agreements),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
