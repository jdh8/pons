//! Responder's `4m` slam try over `1m - 1x - 3NT`
//! (`rebid.minor_jump_notrump_slam_try`), through the whole stance.

mod common;
use common::*;

const P: Call = Call::Pass;

fn partnership(slam_try: Option<u8>) -> Partnership {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.rebid.minor_jump_notrump_slam_try = slam_try;
    american(&agreements).bind()
}

/// `1♦ - 1♠ - 3NT -`, responder to act.
const OVER_3NT: [Call; 6] = [
    call(1, Strain::Diamonds),
    P,
    call(1, Strain::Spades),
    P,
    call(3, Strain::Notrump),
    P,
];

#[test]
fn eleven_with_a_fit_tries_four_diamonds() {
    let on = partnership(Some(9));
    assert_eq!(
        best_call(&on, &OVER_3NT, "KQ832.A2.Q943.32"),
        call(4, Strain::Diamonds)
    );
    // Off, the floor passes it.
    assert_eq!(
        best_call(&partnership(None), &OVER_3NT, "KQ832.A2.Q943.32"),
        P
    );
}

/// The try rejects 13+, which falls through to the floor's `6NT`.
#[test]
fn thirteen_keeps_the_floor() {
    let hand = "KQ832.A2.KJ43.32";
    assert_eq!(
        best_call(&partnership(Some(9)), &OVER_3NT, hand),
        best_call(&partnership(None), &OVER_3NT, hand),
    );
}
