use super::super::tests::best;
use super::*;
use crate::bidding::Bidder;
use crate::bidding::agreements::Agreements;

/// A fresh trie with Drury on
fn drury_trie() -> Trie {
    let mut agreements = Agreements::default();
    agreements.response.drury = true;
    let mut trie = Trie::new();
    super::super::register(&mut trie, &agreements);
    trie
}

const P: Call = Call::Pass;
const fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// `- - 1♠ - 2♣ -`: third-seat opener to answer
const THIRD_SEAT: &[Call] = &[P, P, bid(1, Strain::Spades), P, bid(2, Strain::Clubs), P];
/// `- - - 1♥ - 2♣ -`: fourth-seat opener to answer
const FOURTH_SEAT: &[Call] = &[P, P, P, bid(1, Strain::Hearts), P, bid(2, Strain::Clubs), P];

#[test]
fn opener_answers_by_strength() {
    let trie = drury_trie();
    // KQ752.A63.Q84.72: 11 — minimum, 2♠.
    assert_eq!(
        best(&trie, THIRD_SEAT, "KQ752.A63.Q84.72"),
        bid(2, Strain::Spades)
    );
    // AQ752.K63.Q84.K7: 14 — full opening, the 2♦! relay.
    assert_eq!(
        best(&trie, THIRD_SEAT, "AQ752.K63.Q84.K7"),
        bid(2, Strain::Diamonds)
    );
    // AKJ52.K63.A84.K7: 18 — game.
    assert_eq!(
        best(&trie, THIRD_SEAT, "AKJ52.K63.A84.K7"),
        bid(4, Strain::Spades)
    );
    // AKJ52.K63.AK4.K7: 20 — the keycard ask.
    assert_eq!(
        best(&trie, THIRD_SEAT, "AKJ52.K63.AK4.K7"),
        bid(4, Strain::Notrump)
    );
    // Fourth seat, hearts: A63.KQ752.Q84.72 is a minimum 2♥.
    assert_eq!(
        best(&trie, FOURTH_SEAT, "A63.KQ752.Q84.72"),
        bid(2, Strain::Hearts)
    );
}

#[test]
fn responder_signs_off_or_bids_game_after_the_relay() {
    let trie = drury_trie();
    let after_relay: Vec<Call> = [THIRD_SEAT, &[bid(2, Strain::Diamonds), P]].concat();
    // K84.Q73.KJ62.J53: 10 flat — 2♠, and opener passes it.
    assert_eq!(
        best(&trie, &after_relay, "K84.Q73.KJ62.J53"),
        bid(2, Strain::Spades)
    );
    let signoff: Vec<Call> = [&after_relay[..], &[bid(2, Strain::Spades), P]].concat();
    assert_eq!(best(&trie, &signoff, "AQ752.K63.Q84.K7"), P);
    // K84.Q73.KJ62.A53: 12 — game.
    assert_eq!(
        best(&trie, &after_relay, "K84.Q73.KJ62.A53"),
        bid(4, Strain::Spades)
    );
}

#[test]
fn responder_reraises_a_minimum_on_a_maximum() {
    let trie = drury_trie();
    let after_minimum: Vec<Call> = [THIRD_SEAT, &[bid(2, Strain::Spades), P]].concat();
    // K84.Q73.KJ62.J53: 10 — pass.
    assert_eq!(best(&trie, &after_minimum, "K84.Q73.KJ62.J53"), P);
    // K84.Q73.KJ62.A53: 12 — 3♠; opener bids game on 12, passes on 11.
    assert_eq!(
        best(&trie, &after_minimum, "K84.Q73.KJ62.A53"),
        bid(3, Strain::Spades)
    );
    // Opener: KQ752.A63.Q84.Q7 is 12 HCP and a doubleton, 13 support points
    // — game; KQ752.A63.J84.J72 is a flat 11 — pass.
    let reraised: Vec<Call> = [&after_minimum[..], &[bid(3, Strain::Spades), P]].concat();
    assert_eq!(
        best(&trie, &reraised, "KQ752.A63.Q84.Q7"),
        bid(4, Strain::Spades)
    );
    assert_eq!(best(&trie, &reraised, "KQ752.A63.J84.J72"), P);
}

#[test]
fn drury_continuations_absent_when_off() {
    let mut agreements = Agreements::default();
    agreements.response.drury = false;
    let mut trie = Trie::new();
    super::super::register(&mut trie, &agreements);
    assert!(
        trie.classify(
            "KQ752.A63.Q84.72".parse().expect("valid test hand"),
            contract_bridge::auction::RelativeVulnerability::NONE,
            THIRD_SEAT,
        )
        .is_none()
    );
}
