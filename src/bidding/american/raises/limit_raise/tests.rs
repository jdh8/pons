use super::super::tests::{LIMIT_RAISE_AUCTION, best};
use super::*;

/// A fresh trie with limit-raise acceptance authored (the shipped
/// default).
fn limit_raise_trie() -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.response.limit_raise_acceptance = true;
    let mut trie = Trie::new();
    super::super::register(&mut trie, &agreements);
    trie
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
