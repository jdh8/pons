use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;

/// Build the full rebid Trie with the opener extras ladder on (the shipped
/// default).
fn ladder_trie() -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
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
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.opener_extras_ladder = false;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    // Knob off: the 16-count monster reverts to the minimum 2♦ rebid.
    assert_eq!(
        best(&trie, AFTER_1D_1S, "653.K3.AKQT854.A"),
        Call::Bid(Bid::new(2, Strain::Diamonds))
    );
}

/// `minor_jump_notrump` sends a six-card minor with 18+ HCP to `3NT`; the
/// `3♦` jump keeps 16–17, and knob off every 16+ hand still jumps to `3♦`.
#[test]
fn minor_jump_notrump_takes_eighteen_plus() {
    let build = |on: bool| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.minor_jump_notrump = on;
        let mut trie = Trie::new();
        register(&mut trie, &agreements);
        trie
    };
    let (off, on) = (build(false), build(true));
    let three = |strain| Call::Bid(Bid::new(3, strain));
    // ♠K9 ♥Q7 ♦AQ7432 ♣AKT — 18 HCP, BBA's 3NT.
    assert_eq!(
        best(&off, AFTER_1D_1S, "K9.Q7.AQ7432.AKT"),
        three(Strain::Diamonds)
    );
    assert_eq!(
        best(&on, AFTER_1D_1S, "K9.Q7.AQ7432.AKT"),
        three(Strain::Notrump)
    );
    // 16 HCP keeps the 3♦ jump.
    assert_eq!(
        best(&on, AFTER_1D_1S, "653.K3.AKQT854.A"),
        three(Strain::Diamonds)
    );
}
