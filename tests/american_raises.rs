//! Integration tests for splinter raises, inverted minor raises, and weak jump shifts

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

// --- Splinters over 1♠ ------------------------------------------------------

#[test]
fn test_splinter_over_one_spade() {
    let system = stance();
    let after_1s = &[call(1, Strain::Spades), Call::Pass][..];

    // 11 HCP, singleton heart, four-card spade support -> 4♥ splinter.
    assert_eq!(
        best_call(&system, after_1s, "KQ52.8.A9762.Q43"),
        call(4, Strain::Hearts),
    );
    // 14 HCP, singleton heart, four-card support -> Jacoby 2NT outranks splinter at 13+.
    assert_eq!(
        best_call(&system, after_1s, "KQ52.8.AK762.Q43"),
        call(2, Strain::Notrump),
    );
}

// --- Weak jump shift over 1♥ -------------------------------------------------

#[test]
fn test_wjs_over_one_heart() {
    let system = stance();
    let after_1h = &[call(1, Strain::Hearts), Call::Pass][..];

    // 3 HCP, six spades -> 2♠ weak jump shift.
    assert_eq!(
        best_call(&system, after_1h, "QJ8632.85.972.92"),
        call(2, Strain::Spades),
    );
}

// --- Inverted minor raises over 1♦ ------------------------------------------

#[test]
fn test_inverted_minor_raises_over_one_diamond() {
    let system = stance();
    let after_1d = &[call(1, Strain::Diamonds), Call::Pass][..];

    // 12 HCP, five diamonds, no major -> inverted strong raise (2♦ forcing).
    assert_eq!(
        best_call(&system, after_1d, "A32.K53.KQ942.32"),
        call(2, Strain::Diamonds),
    );
    // 8 HCP, five diamonds -> inverted weak preemptive raise (3♦).
    assert_eq!(
        best_call(&system, after_1d, "T32.J53.KQ942.Q2"),
        call(3, Strain::Diamonds),
    );
}

// --- Opener's rebid after inverted raise -------------------------------------

#[test]
fn test_opener_rebid_after_inverted_raise() {
    let system = stance();
    let after_inv_raise = &[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ][..];

    // 14 HCP balanced -> 2NT (opener shows 12–14 balanced).
    assert_eq!(
        best_call(&system, after_inv_raise, "Q32.A53.AJ42.K92"),
        call(2, Strain::Notrump),
    );
}
