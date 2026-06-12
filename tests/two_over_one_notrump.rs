//! Integration tests for the notrump response structures in the 2/1 system
//!
//! Covers:
//! - Responses to the 2NT opening (3-level Stayman/transfers)
//! - System on after 2♣–2♦–2NT (22–24 balanced)
//! - Quantitative 4NT over the 1NT opening
//! - Acceptance of quantitative 4NT raises
//! - Simple responses after opener's 18–19 2NT rebid

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::bidding::array::Logits;
use pons::bidding::{Family, Stance, System};
use pons::two_over_one;

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents
fn stance() -> Stance {
    two_over_one().against(Family::NATURAL)
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

const P: Call = Call::Pass;

// --- 2NT opening response structure -----------------------------------------

#[test]
fn test_2nt_transfer_to_spades() {
    // Five spades → 3♥ (transfer to spades) over 2NT.
    let system = stance();
    let auction = &[call(2, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "KJ542.Q32.943.92"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn test_2nt_stayman() {
    // 9 HCP, 4-4 majors → 3♣ (Stayman) over 2NT.
    let system = stance();
    let auction = &[call(2, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "KJ54.Q932.K43.83"),
        call(3, Strain::Clubs),
    );
}

// --- System on after 2♣–2♦–2NT ---------------------------------------------

#[test]
fn test_system_on_after_2c_2d_2nt() {
    // 2♣ – (P) – 2♦ – (P) – 2NT – (P): five spades → 3♥ (transfer), system on.
    let system = stance();
    let auction = &[
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        P,
        call(2, Strain::Notrump),
        P,
    ][..];
    assert_eq!(
        best_call(&system, auction, "KJ542.Q32.943.92"),
        call(3, Strain::Hearts),
    );
}

// --- Quantitative 4NT over 1NT opening --------------------------------------

#[test]
fn test_1nt_quantitative_4nt() {
    // 17 HCP balanced, no four-card major → 4NT (quantitative) over 1NT.
    let system = stance();
    let auction = &[call(1, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "KQ5.AQ3.KJ42.J92"),
        call(4, Strain::Notrump),
    );
}

// --- Acceptance of quantitative 4NT over 1NT --------------------------------

#[test]
fn test_1nt_accept_quantitative_4nt() {
    // 17 HCP → accept with 6NT (maximum 15–17 range).
    let system = stance();
    let auction = &[call(1, Strain::Notrump), P, call(4, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "AQ32.KQ5.KJ4.Q92"),
        call(6, Strain::Notrump),
    );
}

#[test]
fn test_1nt_decline_quantitative_4nt() {
    // 15 HCP → decline (pass) the quantitative 4NT invite.
    let system = stance();
    let auction = &[call(1, Strain::Notrump), P, call(4, Strain::Notrump), P][..];
    assert_eq!(best_call(&system, auction, "AQ32.KQ5.QJ4.J92"), Call::Pass,);
}

// --- Simple continuations after 18–19 2NT rebid ----------------------------

#[test]
fn test_rebid_2nt_response_3nt() {
    // 1♥ – 1♠ – 2NT: 10 HCP → bid 3NT opposite opener's 18–19.
    let system = stance();
    let auction = &[
        call(1, Strain::Hearts),
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Notrump),
        P,
    ][..];
    assert_eq!(
        best_call(&system, auction, "KQ32.J5.A964.982"),
        call(3, Strain::Notrump),
    );
}
