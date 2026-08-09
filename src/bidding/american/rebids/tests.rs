use super::*;
use crate::bidding::System;
use contract_bridge::Hand;
use contract_bridge::auction::RelativeVulnerability;

/// The highest-logit call the trie makes for a hand at an auction
///
/// Shared with the per-agreement test modules below this one.
pub(super) fn best(trie: &Trie, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits = trie
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("trie covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("logits array is never empty")
}

/// After `1♦ - 1♥`, a balanced 12–14 with a five-card diamond suit rebids
/// the natural `2♦` by default but `1NT` once `set_balanced_1nt_rebid` is
/// on — the only shape the knob moves (4333/4432 hold no five-card minor).
#[test]
fn balanced_1nt_rebid_knob_flips_2m_to_1nt() {
    let one_d_one_h = &[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    // ♠KQ4 ♥Q3 ♦AK762 ♣853 — 3=2=5=3, 14 HCP, no four-card heart support.
    let hand = "KQ4.Q3.AK762.853";
    let build = || {
        let mut trie = Trie::new();
        crate::bidding::rows::compile_into(
            &mut trie,
            &crate::bidding::agreements::Agreements::current(),
            &[remaining_rebid_bases()],
        );
        trie
    };

    set_balanced_1nt_rebid(false);
    assert_eq!(best(&build(), one_d_one_h, hand), call(2, Strain::Diamonds));

    set_balanced_1nt_rebid(true); // the shipped default
    let on = build();
    assert_eq!(best(&on, one_d_one_h, hand), call(1, Strain::Notrump));
}
