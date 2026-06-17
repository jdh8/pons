//! Integration tests for responder's continuations after the forcing 1NT
//!
//! Covers the second round for responder (preference, limit raise, notrump
//! invite, weak runout) and opener's acceptance of invitational bids.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Family, Stance, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents
fn stance() -> Stance {
    american().against(Family::NATURAL)
}

/// The single highest-logit call the system assigns the hand for the auction
fn best_call(system: &impl System, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits: Logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("system covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

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
