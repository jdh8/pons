//! Integration tests for the instinct floor under the 2/1 contested books
//!
//! These auctions have no authored rules — before the floor, the system
//! returned [`None`] and drivers passed by default.

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

// --- The headline disaster, fixed -------------------------------------------

/// (3♣) X (P): the defensive book has no 3♣ entry, but the floor still
/// advances partner's takeout double instead of passing it out
#[test]
fn test_advance_of_takeout_double_over_preempt() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(3, Strain::Clubs), Call::Double, Call::Pass],
            "92.J8532.9742.92",
        ),
        call(3, Strain::Hearts),
    );
}

// --- Their three-level preempts ----------------------------------------------

/// (3♦) direct seat: takeout double on shape with opening values
#[test]
fn test_takeout_double_of_preempt() {
    let system = stance();
    assert_eq!(
        best_call(&system, &[call(3, Strain::Diamonds)], "KQ32.AJ53.2.A942"),
        Call::Double,
    );
}

/// (3♦) direct seat: nothing to say with a weak hand
#[test]
fn test_pass_of_preempt_without_values() {
    let system = stance();
    assert_eq!(
        best_call(&system, &[call(3, Strain::Diamonds)], "Q5432.J53.942.92"),
        Call::Pass,
    );
}

// --- Competitive auctions past the authored package --------------------------

/// 1♥ (3♣): their jump overcall is past the negative-double package
/// (`OvercallAtMost(2♠)`), but responder still raises with a limit hand
#[test]
fn test_raise_over_jump_overcall() {
    let system = stance();
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), call(3, Strain::Clubs)],
            "Q32.K53.A964.Q92",
        ),
        call(3, Strain::Hearts),
    );
}

/// A deep contested continuation no book authors still gets an answer
#[test]
fn test_deep_contested_auction_is_covered() {
    let system = stance();
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
        Call::Pass,
    ];
    let hand: Hand = "92.AQJ53.KQ42.96".parse().expect("valid test hand");
    assert!(
        system
            .classify(hand, RelativeVulnerability::NONE, &auction)
            .is_some()
    );
}

// --- Precedence ---------------------------------------------------------------

/// The floor never shadows an authored rule: the direct overcall node still
/// answers from the defensive book
#[test]
fn test_authored_rules_still_win() {
    let system = stance();
    // A light five-card major overcalls 1♠ over (1♦) — the authored
    // `defense_to_suit` answer, not an instinct one.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Diamonds)], "AQJ32.853.42.K92"),
        call(1, Strain::Spades),
    );
}
