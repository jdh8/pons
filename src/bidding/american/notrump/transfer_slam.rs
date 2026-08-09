//! The non-forcing slam try after a Jacoby transfer
//!
//! Responder's jump to `4`-of-a-minor asks opener to co-operate below game.
//! Gated by [`set_transfer_slam_try`], and inert while the game-forcing
//! structure in [`super::transfer_gf`] owns the same slot.

use super::*;

thread_local! {
    /// Responder's single-suited slam try after a Jacoby transfer completes;
    /// **on by default**.  See [`set_transfer_slam_try`].
    static TRANSFER_SLAM_TRY: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's post-transfer single-suited slam try for books built
/// *after* this call (thread-local; **on by default**).
///
/// After a Jacoby transfer completes (`1NT - 2♦ - 2♥` / `1NT - 2♥ - 2♠`), a single-suited
/// five-card major with slam-invitational values (16+ HCP, opposite the 15–17
/// opener) bids the *other* major (`3♠` / `3♥`) as an artificial slam try agreeing
/// the transfer major; opener launches RKCB with a maximum (`4NT`) or signs off in
/// the major game (`4M`), and the `slam` 1430 ladder places the slam.  Mirrors
/// the Stayman `3OM` slam try, which the transfer path lacked — so a strong
/// balanced five-card-major responder used to rest in `3NT` while a major slam was
/// cold (the dominant double-dummy leak in our 1NT opening vs BBA).  A paired
/// on/off A/B (320k boards, shared seed, vs the BBA reference) measured **plain
/// +0.0012 IMPs/board (95% CI ±0.0004), PD +0.0012 — +1.42 IMPs/fired in both
/// regimes** (275 fired, 0.09%), every CI excluding 0.
pub fn set_transfer_slam_try(on: bool) {
    TRANSFER_SLAM_TRY.with(|cell| cell.set(on));
}

/// Whether the post-transfer slam try is currently authored
pub fn transfer_slam_try() -> bool {
    TRANSFER_SLAM_TRY.with(Cell::get)
}

/// Responder's artificial slam try after a Jacoby transfer completes
/// (`1NT - 2♦ - 2♥ - 3♠` / `1NT - 2♥ - 2♠ - 3♥`)
///
/// A single-suited five-card major with 16+ HCP agrees the transfer major and bids
/// the *other* major to ask for controls — opener cues with a maximum, else signs
/// off in game ([`stayman_slam_try_answer`]).  Denies a four-card other major (a
/// 5-4 hand shows its second suit instead).  Artificial — the bid is *not* that
/// major — so it carries the [`SLAM_TRY`] alert (the artificial-alert invariant).
/// Empty unless the slam try is on ([`set_transfer_slam_try`]).
pub(super) fn transfer_slam_try_rebid(major: Suit, agreements: &Agreements) -> Rules {
    if !agreements.build.notrump.transfer_slam_try {
        return Rules::new();
    }
    // The GF-majors structure repurposes the spade `3♥` (natural 5-5 slam try) and —
    // with the heart mirror on — the heart `3♠` (spade splinter), relocating each
    // single-suiter to a quantitative `4NT`, so yield the slot to that structure.
    if (major == Suit::Spades && agreements.decision.transfer_gf_majors)
        || (major == Suit::Hearts && agreements.decision.transfer_gf_hearts)
    {
        return Rules::new();
    }
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(other_major(major))),
            140,
            len(major, 5..) & len(other_major(major), ..4) & hcp(16..),
        )
        .alert(SLAM_TRY)
}

/// Opener's answer to the post-transfer slam try (`…3♠` / `…3♥`)
///
/// Mirrors the direct four-major slam try ([`slam_try_answer`]): a **maximum** (17)
/// launches RKCB (`4NT`) and the [`slam`] 1430 ladder places the slam — installed
/// alongside this node — while a **minimum** signs off in the agreed major game
/// (`4M`, *not* pass: responder's `3OM` is artificial, so passing would strand a
/// 3-level part-contract in the wrong strain).
fn transfer_slam_try_answer(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, hcp(17..))
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 0, hcp(..17))
}

/// Whether the original heart-agreeing transfer slam-try node owns its path
fn heart_transfer_slam_try_active(agreements: &Agreements) -> bool {
    agreements.build.notrump.transfer_slam_try && !agreements.decision.transfer_gf_hearts
}

/// Whether either treatment uses the spade-agreeing transfer slam-try node
fn spade_transfer_slam_try_active(agreements: &Agreements) -> bool {
    agreements.build.notrump.transfer_slam_try || agreements.decision.transfer_gf_majors
}

/// Opener's heart-agreeing transfer slam-try answer and RKCB subtree
pub(crate) fn heart_transfer_slam_try() -> Package {
    Package {
        name: "heart-transfer-slam-try",
        gate: |agreements| heart_transfer_slam_try_active(agreements),
        entries: |_| {
            let path = "P* 1NT - 2♦ - 2♥ - 3♠ -".to_owned();
            let mut entries = rows_of(Pattern::node(&path), transfer_slam_try_answer(Suit::Hearts));
            entries.extend(slam::rkcb_rows(&path, Suit::Hearts));
            entries
        },
    }
}

/// Opener's spade-agreeing transfer slam-try answer and RKCB subtree
pub(crate) fn spade_transfer_slam_try() -> Package {
    Package {
        name: "spade-transfer-slam-try",
        gate: |agreements| spade_transfer_slam_try_active(agreements),
        entries: |_| {
            let path = "P* 1NT - 2♥ - 2♠ - 3♥ -".to_owned();
            let mut entries = rows_of(Pattern::node(&path), transfer_slam_try_answer(Suit::Spades));
            entries.extend(slam::rkcb_rows(&path, Suit::Spades));
            entries
        },
    }
}
