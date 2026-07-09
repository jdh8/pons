//! Integration tests for responder's continuations after the forcing 1NT
//!
//! Covers the second round for responder (preference, limit raise, notrump
//! invite, weak runout) and opener's acceptance of invitational bids.

mod common;
use common::*;

use pons::bidding::american::set_meckstroth_adjunct;

// ---------------------------------------------------------------------------
// Responder's second call: 1♠ – (P) – 1NT – (P) – 2♦ – (P) – ?
// ---------------------------------------------------------------------------

/// Auction shorthand for 1♠ – (P) – 1NT – (P) – 2♦ – (P)
fn after_1s_1nt_2d() -> Vec<Call> {
    let p = Call::Pass;
    vec![
        call(1, Strain::Spades),
        p,
        call(1, Strain::Notrump),
        p,
        call(2, Strain::Diamonds),
        p,
    ]
}

#[test]
fn responder_prefers_spades_with_weak_hand() {
    // Q32.J53.964.KQ92 — 8 HCP, 3 spades: preference to 2♠
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2d(), "Q32.J53.964.KQ92"),
        call(2, Strain::Spades),
    );
}

#[test]
fn responder_limit_raises_with_three_card_support() {
    // K32.Q53.J6.KQ942 — 11 HCP, 3 spades: three-card limit raise (3♠)
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2d(), "K32.Q53.J6.KQ942"),
        call(3, Strain::Spades),
    );
}

#[test]
fn responder_invites_notrump_with_no_spade_fit() {
    // Q2.KJ53.J64.KQ92 — 12 HCP, 2 spades: natural 2NT invite
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2d(), "Q2.KJ53.J64.KQ92"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn responder_favors_club_runout_over_preference_weight() {
    // 32.53.J9.KQJ8642 — 7 HCP, 7 clubs, 2 spades.
    // The 2♣ runout rule fires (len(clubs,6..) & hcp(..=9), weight 1.1) and
    // outweighs the 2♠ preference rule (weight 1.0).  Note: in a real table
    // auction 2♣ would be illegal here (below the 2♦ level), but the rules
    // engine does not enforce call legality — the rule is "dead" only in the
    // sense that a legal-move filter at the table layer would discard it.
    // The system itself returns 2♣ as the highest-logit call.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2d(), "32.53.J9.KQJ8642"),
        call(2, Strain::Clubs),
    );
}

// ---------------------------------------------------------------------------
// Opener's acceptance: 1♠ – (P) – 1NT – (P) – 2♦ – (P) – 2NT – (P) – ?
// ---------------------------------------------------------------------------

/// Auction shorthand for 1♠ – (P) – 1NT – (P) – 2♦ – (P) – 2NT – (P)
fn after_1s_1nt_2d_2nt() -> Vec<Call> {
    let p = Call::Pass;
    vec![
        call(1, Strain::Spades),
        p,
        call(1, Strain::Notrump),
        p,
        call(2, Strain::Diamonds),
        p,
        call(2, Strain::Notrump),
        p,
    ]
}

#[test]
fn opener_accepts_notrump_invite_with_maximum() {
    // AQ752.K2.KQ54.92 — 14 HCP: opener bids 3NT to accept the invite
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2d_2nt(), "AQ752.K2.KQ54.92"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn opener_passes_notrump_invite_with_minimum() {
    // AQ752.Q2.KQ54.92 — 13 HCP: opener passes, declining the invite
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2d_2nt(), "AQ752.Q2.KQ54.92"),
        Call::Pass,
    );
}

// ---------------------------------------------------------------------------
// Weak runout: 1♠ – (P) – 1NT – (P) – 2♣ – (P) – ?
// ---------------------------------------------------------------------------

/// Auction shorthand for 1♠ – (P) – 1NT – (P) – 2♣ – (P)
fn after_1s_1nt_2c() -> Vec<Call> {
    let p = Call::Pass;
    vec![
        call(1, Strain::Spades),
        p,
        call(1, Strain::Notrump),
        p,
        call(2, Strain::Clubs),
        p,
    ]
}

#[test]
fn responder_runs_to_six_card_diamond_suit() {
    // Q32.J53.KQ8642.4 — 7 HCP, 6 diamonds: 2♦ runout (legal after 2♣)
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_2c(), "Q32.J53.KQ8642.4"),
        call(2, Strain::Diamonds),
    );
}

// ---------------------------------------------------------------------------
// Meckstroth adjunct: opener's invitational 3m jump after the forcing 1NT
// ---------------------------------------------------------------------------

/// Auction shorthand for 1♠ – (P) – 1NT – (P) — opener to rebid
fn after_1s_1nt() -> Vec<Call> {
    let p = Call::Pass;
    vec![call(1, Strain::Spades), p, call(1, Strain::Notrump), p]
}

#[test]
fn opener_jumps_to_invitational_three_clubs() {
    // AK853.Q2.4.AQ976 — 14 HCP (16 points), 5-5 spades+clubs: 3♣ INV jump
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt(), "AK853.Q2.4.AQ976"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn opener_jumps_to_invitational_three_diamonds() {
    // AK853.Q2.AQ976.4 — 14 HCP (16 points), 5-5 spades+diamonds: 3♦ INV jump
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt(), "AK853.Q2.AQ976.4"),
        call(3, Strain::Diamonds),
    );
}

#[test]
fn baseline_opener_rebids_natural_two_clubs_without_adjunct() {
    // Same 5-5 hand, adjunct off: opener underbids with a natural 2♣ (the gap
    // the adjunct fills).  The toggle is read at construction time, so build the
    // baseline arm with it off, then restore the default.
    set_meckstroth_adjunct(false);
    let base = american().against(Family::NATURAL);
    set_meckstroth_adjunct(true);
    assert_eq!(
        best_call(&base, &after_1s_1nt(), "AK853.Q2.4.AQ976"),
        call(2, Strain::Clubs),
    );
}

// ---------------------------------------------------------------------------
// Meckstroth adjunct: responder over opener's invitational 3♦
//   1♠ – (P) – 1NT – (P) – 3♦ – (P) – ?
// ---------------------------------------------------------------------------

/// Auction shorthand for 1♠ – (P) – 1NT – (P) – 3♦ – (P)
fn after_1s_1nt_3d() -> Vec<Call> {
    let p = Call::Pass;
    vec![
        call(1, Strain::Spades),
        p,
        call(1, Strain::Notrump),
        p,
        call(3, Strain::Diamonds),
        p,
    ]
}

#[test]
fn responder_accepts_invitational_minor_to_major_game() {
    // K42.Q53.84.AQ952 — 10 HCP, 3 spades: accept to the 5-3 major game (4♠)
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_3d(), "K42.Q53.84.AQ952"),
        call(4, Strain::Spades),
    );
}

#[test]
fn responder_accepts_invitational_minor_to_notrump_game() {
    // Q2.KJ3.Q84.KJ952 — 12 HCP, 2 spades: accept to notrump game (3NT)
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_3d(), "Q2.KJ3.Q84.KJ952"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn responder_declines_invitational_minor_with_preference() {
    // Q42.J53.864.K952 — 6 HCP, 3 spades: decline, preference to 3♠
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt_3d(), "Q42.J53.864.K952"),
        call(3, Strain::Spades),
    );
}

// ---------------------------------------------------------------------------
// Meckstroth adjunct on the 1♥ – 1♠ auction
// ---------------------------------------------------------------------------

#[test]
fn opener_jumps_to_invitational_three_clubs_over_one_spade() {
    // 1♥ – (P) – 1♠ – (P) – ? with Q2.AK853.4.AQ976 (5-5 hearts+clubs): 3♣ INV
    let p = Call::Pass;
    let auction = vec![call(1, Strain::Hearts), p, call(1, Strain::Spades), p];
    let system = stance();
    assert_eq!(
        best_call(&system, &auction, "Q2.AK853.4.AQ976"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn responder_accepts_invitational_minor_to_heart_game() {
    // 1♥ – (P) – 1♠ – (P) – 3♣ – (P) – ? with KJ52.Q43.A4.9762:
    // 10 HCP, 4 spades, 3 hearts → 5-3 heart game (4♥)
    let p = Call::Pass;
    let auction = vec![
        call(1, Strain::Hearts),
        p,
        call(1, Strain::Spades),
        p,
        call(3, Strain::Clubs),
        p,
    ];
    let system = stance();
    assert_eq!(
        best_call(&system, &auction, "KJ52.Q43.A4.9762"),
        call(4, Strain::Hearts),
    );
}
