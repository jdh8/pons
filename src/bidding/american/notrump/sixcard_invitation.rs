//! The six-card major invitation after a transfer
//!
//! Responder's `3M` rebid over the completion: a six-card suit and invitational
//! values.  Both floors are knobs —  [`set_sixcard_invite_floor`] for
//! responder, [`set_sixcard_accept_floor`] for opener.

use super::texas::texas_game_floor;
use super::*;

thread_local! {
    /// The `point_count + trump length` floor at which a six-card-major responder
    /// *invites* game — transfer at the two level, then jump to `3M` — instead of
    /// resting in the passed two-level partscore.  **Default 13** (on): the
    /// invitational band is `[13, `[`TEXAS_GAME_FLOOR`]`)`, i.e. the just-below-blast
    /// sixes route through a `3M` invite; opener accepts on [`SIXCARD_ACCEPT_FLOOR`].
    /// Raise it to [`TEXAS_GAME_FLOOR`] (14) to empty the band and turn the invite
    /// *off*.
    ///
    /// On by default as standard, expected major-suit bidding.  A paired A/B vs BBA
    /// (1.536M boards/arm, `--filter-1nt`, floor 13 over 14, accept floor 18; 1607
    /// fired, 0.10%) measured **plain +0.619 IMPs/fired vul none, +1.820 both (CI
    /// excl 0); PD −0.211 / +0.561** — perfect-defense doubling trims the vul-none
    /// edge (the 3-level tax: the decline branch rests in `3M`), but a 6-card-fit
    /// `3M` partscore is not realistically doubled into a penalty at IMPs, so the
    /// PD-none figure overstates the downside.  Double-dummy can't see the invite's
    /// real edge anyway — the `3M` brake on the thin games real defenders beat — so
    /// the conventional invite is kept on.  `probe-jacoby-invite-eval` experiment I
    /// has the opener-threshold sweep.
    static SIXCARD_INVITE_FLOOR: Cell<u8> = const { Cell::new(13) };
    /// Opener's accept floor for the six-card-major invite (`…3M → 4M`) on
    /// `point_count + trump length`; below it opener passes `3M`.  **Default 18**:
    /// a flat 15 with a doubleton in the major (15 + 2) declines, a 15 with
    /// three-card support (15 + 3) or any 16+ accepts — the ≈15% decline the
    /// probe's opener sweep found optimal.  Consulted only when the invite is on
    /// ([`SIXCARD_INVITE_FLOOR`] < [`TEXAS_GAME_FLOOR`]).
    static SIXCARD_ACCEPT_FLOOR: Cell<u8> = const { Cell::new(18) };
}

/// Set the six-card-major game-*invite* floor on `point_count + trump length` for
/// books built *after* this call (thread-local; **default 13 = on**).
///
/// At or above [`set_texas_game_floor`]'s value the band is empty (no invite); the
/// default 13 routes the just-below-blast hands through a `3M` invite instead of a
/// passed two-level partscore.  Raise it to 14 to turn the invite off.
pub fn set_sixcard_invite_floor(floor: u8) {
    SIXCARD_INVITE_FLOOR.with(|cell| cell.set(floor));
}

/// Set opener's accept floor for the six-card-major invite (`…3M → 4M`) on
/// `point_count + trump length` for books built *after* this call (thread-local;
/// **default 18**).
pub fn set_sixcard_accept_floor(floor: u8) {
    SIXCARD_ACCEPT_FLOOR.with(|cell| cell.set(floor));
}

/// The current six-card-major game-invite floor (`point_count + trump length`)
fn sixcard_invite_floor() -> usize {
    usize::from(SIXCARD_INVITE_FLOOR.with(Cell::get))
}

/// Opener's current accept floor for the six-card-major invite
fn sixcard_accept_floor() -> usize {
    usize::from(SIXCARD_ACCEPT_FLOOR.with(Cell::get))
}

/// Whether the six-card-major invite is authored: its floor sits below the Texas
/// game-blast floor, so the invitational band `[invite, blast)` is non-empty.
pub(super) fn sixcard_invite_active() -> bool {
    sixcard_invite_floor() < texas_game_floor()
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
/// is on ([`set_sixcard_invite_floor`]).  Natural — floors only its own strain, so
/// it stays unalerted (the artificial-alert invariant).
pub(super) fn sixcard_invite_rebid(major: Suit) -> Rules {
    if !sixcard_invite_active() {
        return Rules::new();
    }
    let floor = sixcard_invite_floor();
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
                        profile.support_points(),
                        profile.point_scale(),
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
/// [`set_sixcard_accept_floor`]'s value (default 18); otherwise pass `3M`.
/// Authored because the keyless floor reads a three-level raise as forcing and so
/// could not decline.
fn accept_sixcard_invitation(major: Suit) -> Rules {
    let floor = sixcard_accept_floor();
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
                        profile.support_points(),
                        profile.point_scale(),
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
        gate: sixcard_invite_active,
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* 1NT - 2♦ - 2♥ - 3♥ -"),
                accept_sixcard_invitation(Suit::Hearts),
            );
            entries.extend(rows_of(
                Pattern::node("P* 1NT - 2♥ - 2♠ - 3♠ -"),
                accept_sixcard_invitation(Suit::Spades),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
