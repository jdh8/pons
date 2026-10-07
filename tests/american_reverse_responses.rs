//! Integration tests for responder's weak answers to opener's reverse
//! (`RebidKnobs::reverse_weak_responses`)

mod common;
use common::*;
use pons::bidding::agreements::Agreements;

fn with_knob(on: bool) -> Partnership {
    let mut agreements = Agreements::default();
    agreements.rebid.reverse_weak_responses = on;
    american(&agreements).bind()
}

/// `1♦ - 1♠ - 2♥` undisturbed: `[Pass, Pass, 1♦, Pass, 1♠, Pass, 2♥, Pass]`
fn undisturbed() -> Vec<Call> {
    vec![
        Call::Pass,
        Call::Pass,
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
    ]
}

/// The same reverse over their takeout double of `1♦`
fn doubled() -> Vec<Call> {
    let mut auction = undisturbed();
    auction[3] = Call::Double;
    auction
}

/// Armed, a weak hand with three diamonds prefers 3♦ instead of the floor's pass
#[test]
fn weak_hand_answers_the_reverse() {
    assert_eq!(
        best_call(&with_knob(true), &undisturbed(), "QJT75.J8.Q87.J53"),
        call(3, Strain::Diamonds)
    );
}

/// An 8+ HCP hand is rejected by the table and falls through to the floor:
/// the armed and disarmed systems agree
#[test]
fn eight_plus_is_the_floors() {
    let auction = undisturbed();
    assert_eq!(
        best_call(&with_knob(true), &auction, "AQ754.4.8753.K53"),
        best_call(&with_knob(false), &auction, "AQ754.4.8753.K53")
    );
}

/// Contested, the table is not reached through the systems-on rebase — a
/// rejection there would read as a pass of the forcing reverse — so the
/// weak hand and the 8-count alike are the floor's, armed or not
#[test]
fn contested_reverse_is_the_floors() {
    let auction = doubled();
    for hand in ["QJT75.J8.87.QJ53", "AJ85.T8.82.QJ653"] {
        assert_eq!(
            best_call(&with_knob(true), &auction, hand),
            best_call(&with_knob(false), &auction, hand),
            "{hand}"
        );
    }
}
