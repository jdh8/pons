use super::*;
use crate::bidding::System;
use contract_bridge::Hand;
use contract_bridge::auction::RelativeVulnerability;

fn hand(s: &str) -> Hand {
    s.parse().expect("valid test hand")
}

/// The best call the trie makes for `hand_str` at `auction`
fn best(trie: &Trie, auction: &[Call], hand_str: &str) -> Call {
    let logits = trie
        .classify(hand(hand_str), RelativeVulnerability::NONE, auction)
        .expect("trie covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("logits array is never empty")
}

/// A fresh trie with the major game tries authored (the shipped default).
fn game_tries_trie() -> Trie {
    set_major_game_tries(true);
    let mut trie = Trie::new();
    register(&mut trie);
    trie
}

/// A fresh trie with limit-raise acceptance authored (the shipped
/// default).
fn limit_raise_trie() -> Trie {
    set_limit_raise_acceptance(true);
    let mut trie = Trie::new();
    register(&mut trie);
    trie
}

/// `1♥ - 2♥ -`: the single-raise node, undisturbed
const RAISE_AUCTION: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(2, Strain::Hearts)),
    Call::Pass,
];

/// `1♥ - 3♥ -`: the limit-raise node, undisturbed
const LIMIT_RAISE_AUCTION: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Hearts)),
    Call::Pass,
];

#[test]
fn game_tries_absent_when_off() {
    set_major_game_tries(false);
    set_limit_raise_acceptance(false);
    let mut trie = Trie::new();
    register(&mut trie);
    set_major_game_tries(true); // restore the shipped defaults
    set_limit_raise_acceptance(true);
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
    set_major_game_tries(false);
    set_limit_raise_acceptance(false);
    let mut trie = Trie::new();
    register(&mut trie);
    set_major_game_tries(true); // restore the shipped defaults
    set_limit_raise_acceptance(true);
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

#[test]
fn opener_tries_the_long_club_suit() {
    let trie = game_tries_trie();
    // K52.AK974.3.AQ65: 16 HCP + 1 (unbalanced) = 17 points, 4 clubs.
    // The club try (3♣, wt 1.45) beats the general re-raise (3♥, wt 1.2).
    assert_eq!(
        best(&trie, RAISE_AUCTION, "K52.AK974.3.AQ65"),
        Call::Bid(Bid::new(3, Strain::Clubs)),
    );
}

#[test]
fn opener_bids_game_with_a_maximum() {
    let trie = game_tries_trie();
    // AQ3.AKQ85.KJ4.92: balanced, 19 HCP/points — the non-asking maximum.
    assert_eq!(
        best(&trie, RAISE_AUCTION, "AQ3.AKQ85.KJ4.92"),
        Call::Bid(Bid::new(4, Strain::Hearts)),
    );
}

#[test]
fn opener_passes_a_flat_minimum() {
    let trie = game_tries_trie();
    // KQ3.AJ854.K63.93: balanced, 13 HCP/points — below every try.
    assert_eq!(best(&trie, RAISE_AUCTION, "KQ3.AJ854.K63.93"), Call::Pass);
}

/// `1♥ - 2♥ - 3♣ -`: responder's answer to the club try
fn after_club_try() -> Vec<Call> {
    RAISE_AUCTION
        .iter()
        .copied()
        .chain([Call::Bid(Bid::new(3, Strain::Clubs)), Call::Pass])
        .collect()
}

#[test]
fn responder_accepts_the_try_with_a_singleton() {
    let trie = game_tries_trie();
    // 863.K64.QJ8532.7: a singleton club accepts regardless of points.
    assert_eq!(
        best(&trie, &after_club_try(), "863.K64.QJ8532.7"),
        Call::Bid(Bid::new(4, Strain::Hearts)),
    );
}

#[test]
fn responder_declines_a_wasted_minimum() {
    let trie = game_tries_trie();
    // 863.K64.QJ85.972: 6 points, three small clubs — nothing to accept with.
    assert_eq!(
        best(&trie, &after_club_try(), "863.K64.QJ85.972"),
        Call::Bid(Bid::new(3, Strain::Hearts)),
    );
}

#[test]
fn limit_raise_accepts_and_declines() {
    let trie = limit_raise_trie();
    // A63.AK975.Q43.83: balanced, 13 points — the measured boundary
    // (the floor's raise ladder accepts at 13+; under-bidding it lost).
    assert_eq!(
        best(&trie, LIMIT_RAISE_AUCTION, "A63.AK975.Q43.83"),
        Call::Bid(Bid::new(4, Strain::Hearts)),
    );
    // AJ6.K8532.Q63.Q7: balanced, 12 points — decline.
    assert_eq!(
        best(&trie, LIMIT_RAISE_AUCTION, "AJ6.K8532.Q63.Q7"),
        Call::Pass
    );
}
