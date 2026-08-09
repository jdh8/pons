use super::super::tests::{RAISE_AUCTION, best};
use super::*;

/// A fresh trie with the major game tries authored (the shipped default).
fn game_tries_trie() -> Trie {
    set_major_game_tries(true);
    let mut trie = Trie::new();
    super::super::register(
        &mut trie,
        &crate::bidding::agreements::Agreements::current(),
    );
    trie
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
