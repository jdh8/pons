//! Integration tests for two-suited overcalls, their advances, and responsive doubles
//! in the 2/1 defensive book

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

// --- Michaels cue-bid -------------------------------------------------------

/// (1♦) with 5-5 majors → Michaels 2♦
#[test]
fn test_michaels_over_minor() {
    let system = stance();
    // 11 HCP, five spades and five hearts over their 1♦
    assert_eq!(
        best_call(&system, &[call(1, Strain::Diamonds)], "KQJ54.AJ965.2.92"),
        call(2, Strain::Diamonds),
    );
}

// --- Unusual 2NT ------------------------------------------------------------

/// (1♠) with 5-5 minors → Unusual 2NT
#[test]
fn test_unusual_2nt_over_spades() {
    let system = stance();
    // 11 HCP, five diamonds and five clubs over their 1♠
    assert_eq!(
        best_call(&system, &[call(1, Strain::Spades)], "2.95.KQJ54.AJ965"),
        call(2, Strain::Notrump),
    );
}

// --- Michaels over major ----------------------------------------------------

/// (1♥) with spades + clubs → Michaels 2♥
#[test]
fn test_michaels_over_heart() {
    let system = stance();
    // 11 HCP, five spades and five clubs over their 1♥
    assert_eq!(
        best_call(&system, &[call(1, Strain::Hearts)], "KQJ54.2.95.AJ965"),
        call(2, Strain::Hearts),
    );
}

// --- Responsive double ------------------------------------------------------

/// (1♥) – X – (2♥) with 4-4 minors → responsive double
#[test]
fn test_responsive_double() {
    let system = stance();
    // 11 HCP, four-four in clubs and diamonds; partner made takeout double
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Hearts),
                Call::Double,
                call(2, Strain::Hearts),
            ],
            "KQ5.32.K964.QJ92"
        ),
        Call::Double,
    );
}

// --- Unusual 2NT advance ----------------------------------------------------

/// (1♠) – 2NT – (P) with diamonds longer than clubs → 3♦
#[test]
fn test_unusual_nt_advance_longer_diamond() {
    let system = stance();
    // 7 HCP, three diamonds and two clubs — prefer the longer suit
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                call(2, Strain::Notrump),
                Call::Pass,
            ],
            "Q432.Q765.K54.92"
        ),
        call(3, Strain::Diamonds),
    );
}

// --- Regression: single five-card suit still overcalls naturally ------------

/// (1♣) with only one five-card suit → 1♠, not a two-suited bid
#[test]
fn test_regression_single_suit_overcall() {
    let system = stance();
    // 9 HCP, five spades only — should still overcall 1♠, not Michaels
    assert_eq!(
        best_call(&system, &[call(1, Strain::Clubs)], "AQJ32.853.Q42.92"),
        call(1, Strain::Spades),
    );
}
