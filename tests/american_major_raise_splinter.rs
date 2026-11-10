//! Opener's splinter raise over `1x - 1♠` (`rebid.spade_raise_splinter`) and
//! `1m - 1♥` (`rebid.heart_raise_splinter`), through the whole stance: the
//! splinter, the keycard ask, the answer and the grand.

mod common;
use common::*;

const P: Call = Call::Pass;

/// `1♦ - 1♠ - 4♣ - 4NT - 5♠ - 7♠`, every call off the real partnership.
#[test]
fn splinter_keycards_and_the_grand() {
    let default = american(&pons::bidding::agreements::Agreements::default()).bind();
    // ♠KQ84 ♥KJ7 ♦AQJ83 ♣7 — 16 HCP, 17 points, a small club singleton.
    let opener = "KQ84.KJ7.AQJ83.7";
    // ♠A852 ♥A63 ♦K2 ♣A842 — 15 HCP, three keycards and the ♦K.
    let responder = "A852.A63.K2.A842";
    let mut auction = vec![call(1, Strain::Diamonds), P, call(1, Strain::Spades), P];
    for (hand, expected) in [
        (opener, call(4, Strain::Clubs)),
        (responder, call(4, Strain::Notrump)),
        (opener, call(5, Strain::Spades)),
        (responder, call(7, Strain::Spades)),
        (opener, P),
    ] {
        assert_eq!(best_call(&default, &auction, hand), expected, "{auction:?}");
        auction.extend([expected, P]);
    }
    // Off, the same opener raises to `4♠`.
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.rebid.spade_raise_splinter = None;
    assert_eq!(
        best_call(&american(&agreements).bind(), &auction[..4], opener),
        call(4, Strain::Spades)
    );
}

/// `1♦ - 1♥ - 3♠ - 4NT - 5♠ - 7♥`, every call off the real partnership.
#[test]
fn heart_splinter_keycards_and_the_grand() {
    let system = american(&pons::bidding::agreements::Agreements::default()).bind();
    // ♠7 ♥KQJ8 ♦AQJ83 ♣KJ7 — 17 HCP, a spade singleton.
    let opener = "7.KQJ8.AQJ83.KJ7";
    // ♠A85 ♥A963 ♦K2 ♣A842 — 15 HCP, three keycards and the ♦K.
    let responder = "A85.A963.K2.A842";
    let mut auction = vec![call(1, Strain::Diamonds), P, call(1, Strain::Hearts), P];
    for (hand, expected) in [
        (opener, call(3, Strain::Spades)),
        (responder, call(4, Strain::Notrump)),
        (opener, call(5, Strain::Spades)),
        (responder, call(7, Strain::Hearts)),
        (opener, P),
    ] {
        assert_eq!(best_call(&system, &auction, hand), expected, "{auction:?}");
        auction.extend([expected, P]);
    }
    // Off, the same opener raises to `4♥`.
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.rebid.heart_raise_splinter = None;
    assert_eq!(
        best_call(&american(&agreements).bind(), &auction[..4], opener),
        call(4, Strain::Hearts)
    );
}
