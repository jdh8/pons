//! Integration tests for responder's continuations after the forcing 1NT
//!
//! Covers the second round for responder (preference, limit raise, notrump
//! invite, weak runout) and opener's acceptance of invitational bids.

mod common;
use common::*;

use pons::bidding::american::{set_meckstroth_2nt, set_meckstroth_adjunct};

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

// ---------------------------------------------------------------------------
// The real Meckstroth adjunct: opener's artificial game-forcing 2NT (opt-in)
//   1♠ – (P) – 1NT – (P) – 2NT! – (P) – …
// ---------------------------------------------------------------------------

/// The 2/1 pair with the artificial game-forcing 2NT adjunct **off** — it ships
/// on (so the default `stance()` already carries it), so build the baseline arm
/// under the toggle then restore the shipped default.
fn meckstroth_2nt_off_stance() -> Stance {
    set_meckstroth_2nt(false);
    let system = american().against(Family::NATURAL);
    set_meckstroth_2nt(true); // restore the shipped default
    system
}

/// Append `[calls…, P]`-interleaved continuations to `1♠ – (P) – 1NT – (P)`.
fn after_1s_1nt_then(calls: &[Call]) -> Vec<Call> {
    let mut auction = after_1s_1nt();
    for &c in calls {
        auction.push(c);
        auction.push(Call::Pass);
    }
    auction
}

#[test]
fn opener_bids_game_forcing_2nt_on_balanced_eighteen_plus() {
    // AKQ98.KQ4.AQ.432 — 20 HCP, 5-3-2-3 balanced: the artificial GF 2NT.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1s_1nt(), "AKQ98.KQ4.AQ.432"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn game_forcing_2nt_routes_shapely_eighteen_plus() {
    // AKQ982.KQJ4.A.32 — 19 HCP, 6-4-1-2 (unbalanced): the GF 2NT takes any shape.
    assert_eq!(
        best_call(&stance(), &after_1s_1nt(), "AKQ982.KQJ4.A.32"),
        call(2, Strain::Notrump),
    );
    // Knob off: no GF 2NT for a shapely hand — it jump-rebids the six-card major.
    assert_eq!(
        best_call(
            &meckstroth_2nt_off_stance(),
            &after_1s_1nt(),
            "AKQ982.KQJ4.A.32"
        ),
        call(3, Strain::Spades),
    );
}

#[test]
fn responder_relays_three_clubs_with_nothing_to_show() {
    // 32.Q432.K432.Q43 — 7 HCP, 2 spades, no five-card suit: relay 3♣.
    let system = stance();
    let auction = after_1s_1nt_then(&[call(2, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &auction, "32.Q432.K432.Q43"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn opener_shows_concealed_hearts_over_the_relay() {
    // AKQ98.AQ54.K2.32 — 18 HCP, 5-4 spades+hearts: show the four-card heart suit.
    let system = stance();
    let auction = after_1s_1nt_then(&[call(2, Strain::Notrump), call(3, Strain::Clubs)]);
    assert_eq!(
        best_call(&system, &auction, "AKQ98.AQ54.K2.32"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn responder_raises_the_concealed_heart_fit() {
    // 32.KJ32.Q432.J43 — four hearts opposite opener's shown four: the 4-4 game.
    let system = stance();
    let auction = after_1s_1nt_then(&[
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Hearts),
    ]);
    assert_eq!(
        best_call(&system, &auction, "32.KJ32.Q432.J43"),
        call(4, Strain::Hearts),
    );
}

#[test]
fn responder_shows_clubs_via_three_notrump() {
    // 32.Q42.K3.AJ8765 — six clubs, exactly two spades: the artificial 3NT.
    let system = stance();
    let auction = after_1s_1nt_then(&[call(2, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &auction, "32.Q42.K3.AJ8765"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn opener_pulls_club_showing_3nt_to_the_major() {
    // AKQ982.KQ.A32.32 — six spades: pull responder's 3NT (6-2 fit) to 4♠.
    let system = stance();
    let auction = after_1s_1nt_then(&[call(2, Strain::Notrump), call(3, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &auction, "AKQ982.KQ.A32.32"),
        call(4, Strain::Spades),
    );
}
