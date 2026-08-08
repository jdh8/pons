//! Texas transfers — `4♦`/`4♥`, and the slam drive above them
//!
//! The direct game transfer, right-siding a `4M` contract, plus the
//! [`set_texas_slam_drive`] continuation that keeps a slam try alive above it.
//! [`set_texas_game_floor`] sets the strength at which Texas is taken.

use super::stayman_slam::slam_try_answer;
use super::*;

thread_local! {
    /// Route slam-driving six-card-major hands through Texas + responder RKCB
    /// instead of the opener-decides direct `1NT - 4♥/4♠`; **on by default**.
    /// See [`set_texas_slam_drive`].
    static TEXAS_SLAM_DRIVE: Cell<bool> = const { Cell::new(true) };
}

/// Route slam-driving six-card-major hands through a Texas transfer + responder
/// RKCB for books built *after* this call (thread-local; **on by default**).
///
/// The direct `1NT - 4♥/4♠` is a *non-forcing* slam try — opener moves only with a
/// maximum, else passes the major game.  That strands the strong responder: a
/// 16+ six-card-major hand opposite a *minimum* 1NT (the majority) has a cold slam
/// the opener vetoes by passing.  When on, the direct `4♥/4♠` is capped at the bare
/// 15 invitational cusp (opener-decides is right there), and a 16+ hand instead
/// Texas-transfers (`4♣/4♦`) and, over opener's completion, drives its own RKCB
/// (`4NT`) — reaching the slam regardless of opener's minimum, exactly as the
/// reference bidder does.  A paired on/off A/B (320k boards, shared seed, vs the
/// BBA reference) measured **plain +0.0024 IMPs/board (95% CI ±0.0006), PD +0.0024
/// — +5.87 IMPs/fired in both regimes** (131 fired, 0.04%), every CI excluding 0.
pub fn set_texas_slam_drive(on: bool) {
    TEXAS_SLAM_DRIVE.with(|cell| cell.set(on));
}

/// Whether the Texas slam-drive reroute is currently authored
pub fn texas_slam_drive() -> bool {
    TEXAS_SLAM_DRIVE.with(Cell::get)
}

/// Responder's RKCB drive over opener's Texas completion (`1NT - 4♣ - 4♥ - 4NT` /
/// `1NT - 4♦ - 4♠ - 4NT`)
///
/// A 17+ six-card-major hand transferred at the four level and now keycards: `4NT`
/// is RKCB, the [`slam`] 1430 ladder (installed alongside) places the slam.  Weaker
/// (game-only) transfers match no rule and pass opener's `4M`.  Empty unless the
/// reroute is on ([`set_texas_slam_drive`]).
fn texas_slam_drive_rebid() -> Rules {
    if !texas_slam_drive() {
        return Rules::new();
    }
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 140, hcp(16..))
        .alert(slam::RKCB)
}

thread_local! {
    /// The `point_count + trump length` floor at which a 6-card-major responder
    /// blasts game via South African Texas (`4♣/4♦`) instead of transferring at
    /// the two level.  **Default 14** (a 6-bagger needs 8 points, a 7-bagger 7).
    ///
    /// The book inherited a *raw-HCP* floor of **9** verbatim from the old
    /// transfer-then-game route (only the 15-18 slam edge was ever measured).  A
    /// double-dummy screen (`probe-jacoby-invite-eval`) found that 7-8 HCP 6-card
    /// hands score far better in `4M` than the partscore they stop in, that opener
    /// should *never decline* (so an invite degenerates to a blast), and that the
    /// `3M` invite-landing is a *worse* contract than `2M` at every strength (these
    /// one-suiters make 8 or 10 tricks, rarely 9) — so the choice is binary,
    /// pass-`2M` or blast-`4M`, with no invitational band.  At this *fit-rich*
    /// boundary distribution is a real trick (the 6th trump, ruffs), so the screen
    /// (experiments F/G) ranked `point_count + length` > CCCC > points > raw HCP
    /// for the blast decision — unlike the no-fit invite line
    /// (`probe-nt-invite-eval`) and the slam edge (`probe-texas-slam-eval`) where
    /// honors dominate and HCP won.
    ///
    /// Paired A/Bs vs BBA (1.024M boards/arm, `--filter-1nt`): `point_count+len≥14`
    /// over the old HCP-9 baseline measured **plain +0.0102/board vul none, +0.0171
    /// both; PD +0.0082 / +0.0141**, and over a raw-HCP≥7 floor (the same
    /// aggressiveness) **plain +0.0013 / +0.0018; PD +0.0014 / +0.0019** — every
    /// regime a win, all 95% CI excl 0.  `14` matches the HCP≥7 blast rate while
    /// promoting shapely sixes (a 6-4 makes the cut at a bare 6) and demoting
    /// wasted-honor sevens.  See [`set_texas_game_floor`].
    static TEXAS_GAME_FLOOR: Cell<u8> = const { Cell::new(14) };
}

/// Set the South African Texas game-blast floor on `point_count + trump length`
/// (`4♣/4♦`) for books built *after* this call (thread-local; **default 14**).
///
/// Below this floor a 6-card-major hand transfers at the two level (and passes
/// the partscore); at or above it, it jumps to game.  No explicit upper cap: the
/// slam-try `4♥/4♠` (weight 2.6) outranks the game blast (2.5) for the 15-18
/// band, so a slam-interested hand takes the direct slam try regardless.
pub fn set_texas_game_floor(floor: u8) {
    TEXAS_GAME_FLOOR.with(|cell| cell.set(floor));
}

/// The current South African Texas game-blast floor (`point_count + trump length`)
pub(super) fn texas_game_floor() -> usize {
    usize::from(TEXAS_GAME_FLOOR.with(Cell::get))
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
pub(super) fn texas_strength_gate(major: Suit) -> Cons<impl Constraint + Clone> {
    let floor = texas_game_floor();
    described(
        "six-card-major game blast",
        move |hand: Hand, context: &Context<'_>| {
            // Fit-known: a 6-card major opposite 1NT's 2+ is an 8-card fit.
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
        gate: || true,
        entries: || {
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
        gate: texas_slam_drive,
        entries: || {
            let heart_drive = "P* 1NT - 4♣ - 4♥ -";
            let spade_drive = "P* 1NT - 4♦ - 4♠ -";
            let mut entries = rows_of(Pattern::node(heart_drive), texas_slam_drive_rebid());
            entries.extend(rows_of(
                Pattern::node(spade_drive),
                texas_slam_drive_rebid(),
            ));
            entries.extend(slam::rkcb_rows(heart_drive, Suit::Hearts));
            entries.extend(slam::rkcb_rows(spade_drive, Suit::Spades));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
