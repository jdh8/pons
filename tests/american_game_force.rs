//! Integration tests for the 2/1 game-forcing continuations

mod common;
use common::*;

// --- Opener's rebid after 1♠–(P)–2♣–(P) ------------------------------------

#[test]
fn opener_rebid_1s_2c_four_diamonds() {
    // 12 HCP, five spades, four diamonds → show the new suit (2♦).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ52.32.KQ54.92"),
        call(2, Strain::Diamonds),
    );
}

#[test]
fn opener_rebid_1s_2c_six_spades() {
    // 13 HCP (14 points after the clean-shape upgrade), six spades — short
    // of the 15 a jump to 3♠ would promise → rebid 2♠.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ752.A2.Q54.92"),
        call(2, Strain::Spades),
    );
}

#[test]
fn opener_rebid_1s_2c_balanced() {
    // 14 HCP, balanced → 2NT.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ52.K32.Q54.Q9"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn opener_rebid_1s_2c_club_support() {
    // 12 HCP, four-card club support → raise to 3♣.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ52.32.K5.Q942"),
        call(3, Strain::Clubs),
    );
}

// --- Responder's rebid after 1♠–(P)–2♣–(P)–2♦–(P) -------------------------

#[test]
fn responder_rebid_three_spades() {
    // 17 HCP, three-card spade support → 3♠ (sets trump).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "K32.A2.Q54.AKJ92"),
        call(3, Strain::Spades),
    );
}

#[test]
fn responder_rebid_raise_diamonds() {
    // 17 HCP, four diamonds, only two spades → raise diamonds (3♦).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "Q2.A32.KQ54.AQ92"),
        call(3, Strain::Diamonds),
    );
}

// --- Opener's third call after 1♠–(P)–2♣–(P)–2♦–(P)–3♠–(P) ---------------

#[test]
fn opener_third_sign_off() {
    // 12 HCP → sign off at 4♠.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ52.32.KQ54.92"),
        call(4, Strain::Spades),
    );
}

#[test]
fn opener_third_keycard_ask() {
    // 17 HCP → 4NT (key card ask).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ52.A2.KQJ4.92"),
        call(4, Strain::Notrump),
    );
}

// --- Game backstop keeps the force alive ------------------------------------

#[test]
fn second_suit_agreed_minimum_bids_3nt() {
    // 1♠–(P)–2♣–(P)–2♦–(P)–3♦–(P): responder agreed opener's second suit.
    // A minimum opener signs off at 3NT — NOT the game backstop's 4♠, which
    // would revert to the 5-2 spade fit after the diamond fit was found.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AQJ52.32.KQ54.92"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn second_suit_agreed_extras_asks_rkcb() {
    // Same node with extras (18 HCP): opener asks 4NT RKCB with diamonds set.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&stance(), auction, "AKJ52.A2.AQ54.K2"),
        call(4, Strain::Notrump),
    );
}
