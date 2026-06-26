//! Integration tests for weak-two responses in the 2/1 game-forcing system
//!
//! Covers Ogust answers, first responses, and opener's reply to forcing new
//! suits — verifying each decision node with representative hands.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Stance, System, Tag};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The 2/1 pair bound against natural opponents
fn stance() -> Stance {
    american().against(Tag::NATURAL)
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
// Ogust answers (opener's side)
// ---------------------------------------------------------------------------

/// Opener answers 3♣ (min, bad suit) through 3NT (solid) after the Ogust ask
///
/// Auction: 2♥ – (P) – 2NT – (P) – ?
#[test]
fn test_ogust_answers_after_two_hearts() {
    let system = stance();
    // Opener is in seat 1; responder's 2NT is after one opponent pass.
    let auction = &[
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Pass,
    ][..];

    // 6 HCP, one top honor (Q) → minimum, bad suit → 3♣.
    assert_eq!(
        best_call(&system, auction, "94.QJ8632.K85.72"),
        call(3, Strain::Clubs),
        "min bad suit (QJ suit, 6 HCP) should answer 3♣",
    );

    // 5 HCP, two top honors (KQ) → minimum, good suit → 3♦.
    assert_eq!(
        best_call(&system, auction, "94.KQ8632.852.72"),
        call(3, Strain::Diamonds),
        "min good suit (KQ suit, 5 HCP) should answer 3♦",
    );

    // 9 HCP, one top honor (QJ) → maximum, bad suit → 3♥.
    assert_eq!(
        best_call(&system, auction, "K4.QJ8632.K85.72"),
        call(3, Strain::Hearts),
        "max bad suit (QJ suit, 9 HCP) should answer 3♥",
    );

    // 9 HCP, two top honors (AQ) → maximum, good suit → 3♠.
    assert_eq!(
        best_call(&system, auction, "94.AQ8632.K85.72"),
        call(3, Strain::Spades),
        "max good suit (AQ suit, 9 HCP) should answer 3♠",
    );

    // AKQ solid suit → 3NT.
    assert_eq!(
        best_call(&system, auction, "94.AKQ632.852.72"),
        call(3, Strain::Notrump),
        "solid suit (AKQ) should answer 3NT",
    );
}

// ---------------------------------------------------------------------------
// First responses to 2♥
// ---------------------------------------------------------------------------

/// Responder's first-round options over 2♥
///
/// Auction: 2♥ – (P) – ?
#[test]
fn test_responses_to_two_hearts() {
    let system = stance();
    let auction = &[call(2, Strain::Hearts), Call::Pass][..];

    // 16 HCP, three-card heart support → Ogust 2NT.
    assert_eq!(
        best_call(&system, auction, "AQ52.K76.AK72.93"),
        call(2, Strain::Notrump),
        "opening values + heart fit should invoke Ogust",
    );

    // 5 HCP, three-card heart support → pre-emptive raise to 3♥.
    assert_eq!(
        best_call(&system, auction, "Q52.K76.97532.93"),
        call(3, Strain::Hearts),
        "fit without values should raise pre-emptively",
    );

    // 16 HCP, five spades with AQJ (two top honors), singleton heart → 2♠ forcing.
    assert_eq!(
        best_call(&system, auction, "AQJ75.6.AQ72.K93"),
        call(2, Strain::Spades),
        "good five-card suit with opening values should bid forcing 2♠",
    );
}

// ---------------------------------------------------------------------------
// Opener's reply to a forcing new suit
// ---------------------------------------------------------------------------

/// Opener's reply after responder's one-round-forcing 2♠ over 2♥
///
/// Auction: 2♥ – (P) – 2♠ – (P) – ?
#[test]
fn test_opener_reply_to_two_spades_over_two_hearts() {
    let system = stance();
    let auction = &[
        call(2, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ][..];

    // Three-card spade support → raise to 3♠.
    assert_eq!(
        best_call(&system, auction, "Q32.KQ8632.85.72"),
        call(3, Strain::Spades),
        "three spades → raise partner's suit to 3♠",
    );

    // Two-card spade support → rebid the six-card heart suit at 3♥.
    assert_eq!(
        best_call(&system, auction, "92.KQ8632.985.72"),
        call(3, Strain::Hearts),
        "no spade support → rebid the heart suit at 3♥",
    );
}
