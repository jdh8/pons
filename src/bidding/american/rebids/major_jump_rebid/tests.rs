use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;

/// Build the full rebid Trie with opener's major jump-rebid rung on (the
/// shipped default).
fn major_jump_trie() -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.opener_major_jump_rebid = true;
    agreements.rebid.major_jump_rebid_shape = false;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

/// The raw table auction `1♥ - 1♠ -` (opener to rebid).
const AFTER_1H_1S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
];

/// The raw table auction `1♠ - 1NT -` (opener rebids over forcing 1NT).
const AFTER_1S_1NT: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Notrump)),
    Call::Pass,
];

#[test]
fn opener_major_jump_rebid_shows_strength() {
    let trie = major_jump_trie();
    // 6+ hearts, 16 HCP, no spade fit → jump-rebid 3♥.
    assert_eq!(
        best(&trie, AFTER_1H_1S, "3.AKQJ72.KQ5.J54"),
        Call::Bid(Bid::new(3, Strain::Hearts))
    );
    // A minimum 6-card heart hand still takes the natural 2♥.
    assert_eq!(
        best(&trie, AFTER_1H_1S, "A2.KQ9872.Q43.J5"),
        Call::Bid(Bid::new(2, Strain::Hearts))
    );
    // The forcing-1NT node carries the same rung: 1♠ - 1NT - 3♠.
    assert_eq!(
        best(&trie, AFTER_1S_1NT, "AKQJ72.3.KQ5.J54"),
        Call::Bid(Bid::new(3, Strain::Spades))
    );
}

#[test]
fn opener_major_jump_rebid_reverts_when_off() {
    // Knob off: the 16-count 6-heart hand reverts to the minimum 2♥ rebid.
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.decision.reading.opener_major_jump_rebid = false;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    assert_eq!(
        best(&trie, AFTER_1H_1S, "3.AKQJ72.KQ5.J54"),
        Call::Bid(Bid::new(2, Strain::Hearts))
    );
}

/// `1♥ - 1♠ - 3♥ -` — responder to act over opener's jump-rebid.
const AFTER_1H_1S_3H: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Hearts)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Hearts)),
    Call::Pass,
];

/// `1♠ - 1NT - 3♠ -` — responder to act over opener's jump-rebid.
const AFTER_1S_1NT_3S: &[Call] = &[
    Call::Bid(Bid::new(1, Strain::Spades)),
    Call::Pass,
    Call::Bid(Bid::new(1, Strain::Notrump)),
    Call::Pass,
    Call::Bid(Bid::new(3, Strain::Spades)),
    Call::Pass,
];

#[test]
fn responder_accepts_major_jump_rebid() {
    let trie = major_jump_trie();
    // Fit (3 hearts) + values → raise to game 4♥.
    assert_eq!(
        best(&trie, AFTER_1H_1S_3H, "KQ85.K76.542.J43"),
        Call::Bid(Bid::new(4, Strain::Hearts))
    );
    // No heart fit + values → notrump game 3NT.
    assert_eq!(
        best(&trie, AFTER_1H_1S_3H, "KQ85.6.KJ43.Q642"),
        Call::Bid(Bid::new(3, Strain::Notrump))
    );
    // Minimum → pass the invitational jump, play 3♥.
    assert_eq!(best(&trie, AFTER_1H_1S_3H, "Q985.42.J8532.K4"), Call::Pass);
    // Forcing-1NT node: a doubleton spade fit (8 cards) + values → 4♠.
    assert_eq!(
        best(&trie, AFTER_1S_1NT_3S, "87.KQ86.KJ43.T92"),
        Call::Bid(Bid::new(4, Strain::Spades))
    );
}

fn shape_trie() -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.major_jump_rebid_shape = true;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

/// `major_jump_rebid_shape`: seven hearts jump to game; six with a singleton
/// pull responder's `3NT`; a 6-3-2-2 passes it.
#[test]
fn major_jump_rebid_shape_bids_the_seventh_trump() {
    let (off, on) = (major_jump_trie(), shape_trie());
    // ♠9 ♥AKJ6532 ♦KQ5 ♣KQ — the anchor's 18-count BBA opened `4♥` on.
    let seven = "9.AKJ6532.KQ5.KQ";
    assert_eq!(best(&off, AFTER_1H_1S, seven), call(3, Strain::Hearts));
    assert_eq!(best(&on, AFTER_1H_1S, seven), call(4, Strain::Hearts));
    let mut notrump = AFTER_1H_1S.to_vec();
    notrump.extend([
        call(3, Strain::Hearts),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ]);
    // ♠3 ♥AKQJ76 ♦JT5 ♣A52 — six with a singleton: pull.
    assert_eq!(
        best(&on, &notrump, "3.AKQJ76.JT5.A52"),
        call(4, Strain::Hearts)
    );
    // ♠K3 ♥AKQJ76 ♦J5 ♣A52 — 6-3-2-2: `3NT` stands.
    assert_eq!(best(&on, &notrump, "K3.AKQJ76.J5.A52"), Call::Pass);
    let mut spades = AFTER_1S_1NT.to_vec();
    spades.extend([
        call(3, Strain::Spades),
        Call::Pass,
        call(3, Strain::Notrump),
        Call::Pass,
    ]);
    assert_eq!(
        best(&on, &spades, "AQ96543.85.A.AJ8"),
        call(4, Strain::Spades)
    );
}
