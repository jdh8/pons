use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::{Bidder as _, Trie};

fn trie(slam_try: Option<u8>) -> Trie {
    let mut agreements = Agreements::default();
    agreements.rebid.minor_jump_notrump = true;
    agreements.rebid.minor_jump_notrump_slam_try = slam_try;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

fn auction(calls: &str) -> Vec<Call> {
    calls
        .split_whitespace()
        .map(|c| {
            if c == "-" {
                Call::Pass
            } else {
                c.parse().expect("call")
            }
        })
        .collect()
}

/// A 9–12 count with a fit tries `4♦`; opener asks keycards on 20+ and signs
/// off in `5♦` below it.
#[test]
fn slam_try_and_answer() {
    let trie = trie(Some(9));
    let four = auction("1♦ - 1♠ - 3NT -");
    // ♠KQ832 ♥A2 ♦Q943 ♣32 — 11 HCP, four diamonds.
    assert_eq!(
        best(&trie, &four, "KQ832.A2.Q943.32"),
        Call::Bid(Bid::new(4, Strain::Diamonds))
    );
    let answer = auction("1♦ - 1♠ - 3NT - 4♦ -");
    // ♠Q9 ♥AQ ♦AKJ432 ♣KJ2 — 20 HCP: keycards.
    assert_eq!(
        best(&trie, &answer, "Q9.AQ.AKJ432.KJ2"),
        Call::Bid(Bid::new(4, Strain::Notrump))
    );
    // ♠K9 ♥Q7 ♦AQ7432 ♣AKT — 18 HCP: sign off.
    assert_eq!(
        best(&trie, &answer, "K9.Q7.AQ7432.AKT"),
        Call::Bid(Bid::new(5, Strain::Diamonds))
    );
}

/// Knob off, no node: the seat stays the floor's.
#[test]
fn off_registers_nothing() {
    let trie = trie(None);
    let four = auction("1♦ - 1♠ - 3NT -");
    assert!(
        trie.classify(
            "KQ832.A2.Q943.32".parse().expect("hand"),
            contract_bridge::auction::RelativeVulnerability::NONE,
            &four,
        )
        .is_none()
    );
}
