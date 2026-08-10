use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;

/// Build the full rebid Trie with the opener extras ladder on (the shipped
/// default).
fn ladder_trie() -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::current();
    agreements.decision.reading.opener_extras_ladder = true;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

/// The raw table auction `1♦ - 1♠ -` (opener to rebid).
const AFTER_1D_1S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Diamonds)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
];

#[test]
fn opener_extras_ladder_shows_strength() {
    let trie = ladder_trie();
    let b = |hand| best(&trie, AFTER_1D_1S, hand);
    // Self-sufficient 6+ diamonds, 16 HCP → jump-rebid 3♦.
    assert_eq!(
        b("653.K3.AKQT854.A"),
        Call::Bid(Bid::new(3, Strain::Diamonds))
    );
    // 5♦ 4♥, 18 HCP → jump-shift 3♥ (game-forcing two-suiter).
    assert_eq!(
        b("T64.AJ86.AKQ95.A"),
        Call::Bid(Bid::new(3, Strain::Hearts))
    );
    // 5-5 in the minors, 18 HCP → jump-shift 3♣.
    assert_eq!(b("K.62.AQJ94.AKJ85"), Call::Bid(Bid::new(3, Strain::Clubs)));
    // A dead minimum still takes the natural 2♦ rebid.
    assert_eq!(
        b("K54.Q3.KJ8542.32"),
        Call::Bid(Bid::new(2, Strain::Diamonds))
    );
}

#[test]
fn opener_extras_ladder_reverts_when_off() {
    let mut agreements = crate::bidding::agreements::Agreements::current();
    agreements.decision.reading.opener_extras_ladder = false;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    // Knob off: the 16-count monster reverts to the minimum 2♦ rebid.
    assert_eq!(
        best(&trie, AFTER_1D_1S, "653.K3.AKQT854.A"),
        Call::Bid(Bid::new(2, Strain::Diamonds))
    );
}
