use super::super::tests::best;
use super::*;
use crate::bidding::{Bidder, Trie};
use contract_bridge::auction::{Call, RelativeVulnerability};

fn trie_of(on: bool) -> Trie {
    let mut agreements = Agreements::default();
    agreements.rebid.reverse_weak_responses = on;
    let mut trie = Trie::new();
    compile_into(&mut trie, &agreements, &[reverse_response_continuations()]);
    trie
}

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

fn auction(s: &str) -> Vec<Call> {
    s.split(' ')
        .map(|t| {
            if t == "-" {
                Call::Pass
            } else {
                t.parse().expect("call")
            }
        })
        .collect()
}

/// Whether the trie claims the hand at the auction at all
fn claims(trie: &Trie, auction: &[Call], hand: &str) -> bool {
    trie.classify(
        hand.parse().expect("valid test hand"),
        RelativeVulnerability::NONE,
        auction,
    )
    .is_some_and(|logits| (&logits.0).into_iter().any(|(_, w)| w.is_finite()))
}

/// Off, the node is unregistered; on, every weak rung fires and 8+ HCP or a
/// weak misfit is the floor's.
#[test]
fn weak_responses_over_the_reverse() {
    let node = auction("1♦ - 1♠ - 2♥ -");
    assert!(!claims(&trie_of(false), &node, "QJT75.J8.87.QJ53"));
    let on = trie_of(true);
    // ♠QJT754 ♥4 ♦875 ♣J53 — 4, six spades: weak rebid.
    assert_eq!(best(&on, &node, "QJT754.4.875.J53"), bid(2, Strain::Spades));
    // ♠QJT75 ♥J864 ♦87 ♣J53 — 4, four hearts: weak raise.
    assert_eq!(
        best(&on, &node, "QJT75.J864.87.J53"),
        bid(3, Strain::Hearts)
    );
    // ♠QJT75 ♥J8 ♦Q87 ♣J53 — 6, three diamonds: preference.
    assert_eq!(
        best(&on, &node, "QJT75.J8.Q87.J53"),
        bid(3, Strain::Diamonds)
    );
    // ♠QJT75 ♥J8 ♦87 ♣QJ53 — 6, no fit: rejected, the floor's.
    assert!(!claims(&on, &node, "QJT75.J8.87.QJ53"));
    // ♠AQ754 ♥4 ♦8753 ♣K53 — 9 HCP: rejected, the floor's.
    assert!(!claims(&on, &node, "AQ754.4.8753.K53"));
    // ♠QJT75 ♥J864 ♦K7 ♣J53 — 8 HCP: likewise.
    assert!(!claims(&on, &node, "QJT75.J864.K7.J53"));

    // Opener over the weak raise: pass below 19, else game.
    let raised = auction("1♦ - 1♠ - 2♥ - 3♥ -");
    assert_eq!(best(&on, &raised, "K7.AQJ5.KQ964.J8"), Call::Pass);
    assert_eq!(
        best(&on, &raised, "K7.AQJ5.AKQ64.K8"),
        bid(4, Strain::Hearts)
    );
    let raised_minor = auction("1♣ - 1♥ - 2♦ - 3♦ -");
    assert_eq!(
        best(&on, &raised_minor, "K7.J8.AKQ5.AKJ64"),
        bid(3, Strain::Notrump)
    );

    // The other three reverses register: a weak hand with three clubs
    // prefers 3♣ over each.
    for prefix in ["1♣ - 1♥ - 2♦ -", "1♣ - 1♠ - 2♦ -", "1♣ - 1♠ - 2♥ -"] {
        assert_eq!(
            best(&on, &auction(prefix), "J9753.J86.Q8.753"),
            bid(3, Strain::Clubs),
            "{prefix}"
        );
    }
}
