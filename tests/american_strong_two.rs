//! Integration tests for the strong 2♣ opening structure

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

// --- Responses to 2♣ --------------------------------------------------------

/// At `[2♣, P]`: the right response for various hand types
#[test]
fn test_responses_to_two_clubs() {
    let system = stance();
    let auction = &[call(2, Strain::Clubs), Call::Pass][..];

    // 1 HCP — double negative (0–3 HCP).
    assert_eq!(
        best_call(&system, auction, "98532.J76.872.92"),
        call(2, Strain::Hearts),
    );
    // 6 HCP — waiting 2♦ (4+ HCP, not strong enough for a positive).
    assert_eq!(
        best_call(&system, auction, "Q543.K76.872.J92"),
        call(2, Strain::Diamonds),
    );
    // 10 HCP, five spades to AQJ — natural positive 2♠.
    assert_eq!(
        best_call(&system, auction, "AQJ85.K76.87.932"),
        call(2, Strain::Spades),
    );
    // 11 HCP balanced — 2NT positive.
    assert_eq!(
        best_call(&system, auction, "QJ54.K76.A87.J92"),
        call(2, Strain::Notrump),
    );
}

// --- Opener's rebid after 2♦ waiting ----------------------------------------

/// At `[2♣, P, 2♦, P]`: opener rebids shape or notrump range
#[test]
fn test_opener_rebid_after_waiting() {
    let system = stance();
    let auction = &[
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ][..];

    // 24 HCP balanced → 2NT.
    assert_eq!(
        best_call(&system, auction, "AKQ2.KQJ.KQ4.A32"),
        call(2, Strain::Notrump),
    );
    // 24 HCP, five hearts → 2♥.
    assert_eq!(
        best_call(&system, auction, "AK2.AKQJ5.A4.K32"),
        call(2, Strain::Hearts),
    );
}

// --- Responder after opener's suit rebid (waiting sequence) -----------------

/// At `[2♣, P, 2♦, P, 2♠, P]`: responder supports or retreats
#[test]
fn test_resp_after_waiting_spades() {
    let system = stance();
    let auction = &[
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ][..];

    // Q543 — three spades → raise to 3♠.
    assert_eq!(
        best_call(&system, auction, "Q543.K76.872.J92"),
        call(3, Strain::Spades),
    );
    // 54 — only two spades → retreat to 2NT.
    // Hand: 54.K762.8732.J92 (ranks must be in descending order within each suit).
    assert_eq!(
        best_call(&system, auction, "54.K762.8732.J92"),
        call(2, Strain::Notrump),
    );
}

// --- Opener after the major raise --------------------------------------------

/// At `[2♣, P, 2♦, P, 2♠, P, 3♠, P]`: sign off or launch RKCB
#[test]
fn test_opener_after_spades_raise() {
    let system = stance();
    let auction = &[
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ][..];

    // 23 HCP, 6 spades → sign off in 4♠ (28+ required for 4NT).
    assert_eq!(
        best_call(&system, auction, "AKQJ52.AK2.A4.32"),
        call(4, Strain::Spades),
    );
}
