//! Hearts agreed at `1♠ - 2♥ - 3♥` (`game_force.heart_raise_keycard`),
//! through the whole stance.

mod common;
use common::*;

const P: Call = Call::Pass;

fn partnership(ask: Option<u8>) -> Partnership {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.game_force.heart_raise_keycard = ask;
    american(&agreements).bind()
}

/// `1♠ - 2♥ - 3♥ -`, responder to act.
const RAISED: [Call; 6] = [
    call(1, Strain::Spades),
    P,
    call(2, Strain::Hearts),
    P,
    call(3, Strain::Hearts),
    P,
];

/// A 14-count with no spade fit bid the no-fit `3NT`; on, it signs off.
#[test]
fn minimum_signs_off_in_hearts() {
    let hand = "Q3.AJT96.A82.QT5";
    assert_eq!(
        best_call(&partnership(None), &RAISED, hand),
        call(3, Strain::Notrump)
    );
    assert_eq!(
        best_call(&partnership(Some(17)), &RAISED, hand),
        call(4, Strain::Hearts)
    );
}

/// Three spades no longer redirect the trump suit.
#[test]
fn three_spades_do_not_outrank_the_fit() {
    let hand = "Q73.AJT96.A8.T54";
    assert_eq!(
        best_call(&partnership(None), &RAISED, hand),
        call(3, Strain::Spades)
    );
    assert_eq!(
        best_call(&partnership(Some(17)), &RAISED, hand),
        call(4, Strain::Hearts)
    );
}

#[test]
fn extras_ask_keycards() {
    assert_eq!(
        best_call(&partnership(Some(17)), &RAISED, "A3.AKJ96.AQ8.T54"),
        call(4, Strain::Notrump)
    );
}

/// Over the sign-off opener passes a minimum and asks on extras.
#[test]
fn opener_acts_over_the_sign_off() {
    let mut auction = RAISED.to_vec();
    auction.extend([call(4, Strain::Hearts), P]);
    let on = partnership(Some(17));
    assert_eq!(best_call(&on, &auction, "AQJ98.Q543.K9.J2"), P);
    assert_eq!(
        best_call(&on, &auction, "AKJ98.KQ43.A9.K2"),
        call(4, Strain::Notrump)
    );
}
