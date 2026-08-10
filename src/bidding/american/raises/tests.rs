use super::*;
use crate::bidding::System;
use contract_bridge::Hand;
use contract_bridge::auction::RelativeVulnerability;

fn hand(s: &str) -> Hand {
    s.parse().expect("valid test hand")
}

/// The best call the trie makes for `hand_str` at `auction`
pub(super) fn best(trie: &Trie, auction: &[Call], hand_str: &str) -> Call {
    let logits = trie
        .classify(hand(hand_str), RelativeVulnerability::NONE, auction)
        .expect("trie covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("logits array is never empty")
}

/// `1♥ - 2♥ -`: the single-raise node, undisturbed
pub(super) const RAISE_AUCTION: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Hearts)),
    Call::Pass,
];

/// `1♥ - 3♥ -`: the limit-raise node, undisturbed
pub(super) const LIMIT_RAISE_AUCTION: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Hearts)),
    Call::Pass,
];

#[test]
fn game_tries_absent_when_off() {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.response.major_game_tries = false;
    agreements.response.limit_raise_acceptance = false;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    assert!(
        trie.classify(
            hand("K52.AK974.3.AQ65"),
            RelativeVulnerability::NONE,
            RAISE_AUCTION
        )
        .is_none(),
        "major game tries must be absent with the knob off"
    );
}

#[test]
fn limit_raise_acceptance_absent_when_off() {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.response.major_game_tries = false;
    agreements.response.limit_raise_acceptance = false;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    assert!(
        trie.classify(
            hand("A63.AK975.QJ3.83"),
            RelativeVulnerability::NONE,
            LIMIT_RAISE_AUCTION
        )
        .is_none(),
        "limit-raise acceptance must be absent with the knob off"
    );
}
