use super::super::super::call;
use super::super::register;
use crate::bidding::agreements::Agreements;
use crate::bidding::{Bidder, Trie};
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Hand, Strain};

fn build(floor: Option<u8>) -> Trie {
    let mut agreements = Agreements::default();
    agreements.response.heart_spade_strong_jump_shift = floor;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

fn best(trie: &Trie, auction: &[Call], hand: &str) -> Call {
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

const ONE_HEART: [Call; 2] = [
    Call::Bid(contract_bridge::Bid::new(1, Strain::Hearts)),
    Call::Pass,
];

/// The knob turns the unpassed `2♠` from weak to strong; a passed hand's
/// stays weak, and three hearts keep the `1♠` response.
#[test]
fn two_spades_is_strong_only_when_unpassed() {
    let (off, on) = (build(None), build(Some(16)));
    // ♠AQJ85 ♥K2 ♦A2 ♣K853 — 17 HCP, the anchor's worst board.
    let strong = "AQJ85.K2.A2.K853";
    // ♠KJ9752 ♥4 ♦T93 ♣842 — 4 HCP, the weak jump shift.
    let weak = "KJ9752.4.T93.842";
    assert_eq!(best(&off, &ONE_HEART, strong), call(1, Strain::Spades));
    assert_eq!(best(&off, &ONE_HEART, weak), call(2, Strain::Spades));
    assert_eq!(best(&on, &ONE_HEART, strong), call(2, Strain::Spades));
    assert_ne!(best(&on, &ONE_HEART, weak), call(2, Strain::Spades));
    // 15 HCP is under the floor; three hearts are a raise hand.
    assert_eq!(
        best(&on, &ONE_HEART, "AQJ85.K2.A2.8532"),
        call(1, Strain::Spades)
    );
    assert_eq!(
        best(&on, &ONE_HEART, "AQJ85.K32.A2.K85"),
        call(1, Strain::Spades)
    );
    // Third seat: responder has passed.
    let passed = [Call::Pass, Call::Pass, ONE_HEART[0], Call::Pass];
    assert_eq!(best(&on, &passed, weak), call(2, Strain::Spades));
}

/// Opener describes and both placements are reached without a pass below
/// game.
#[test]
fn the_jump_shift_is_answered_to_game() {
    let trie = build(Some(16));
    let over = |extra: &[Call]| {
        let mut auction = vec![
            ONE_HEART[0],
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        for &c in extra {
            auction.extend([c, Call::Pass]);
        }
        auction
    };
    let opener = |hand| best(&trie, &over(&[]), hand);
    assert_eq!(opener("K73.AQ763.K4.Q96"), call(3, Strain::Spades));
    assert_eq!(opener("7.AQJ763.K42.Q96"), call(3, Strain::Hearts));
    assert_eq!(opener("73.AQ763.K.AQJ96"), call(3, Strain::Clubs));
    assert_eq!(opener("73.AQ763.KJ4.Q96"), call(2, Strain::Notrump));
    // Their double changes nothing.
    let doubled = [
        ONE_HEART[0],
        Call::Pass,
        call(2, Strain::Spades),
        Call::Double,
    ];
    assert_eq!(
        best(&trie, &doubled, "K73.AQ763.K4.Q96"),
        call(3, Strain::Spades)
    );

    let raised = over(&[call(3, Strain::Spades)]);
    assert_eq!(
        best(&trie, &raised, "AQJ85.K2.A2.K853"),
        call(4, Strain::Spades)
    );
    assert_eq!(
        best(&trie, &raised, "AQJ85.K2.AQ.K853"),
        call(4, Strain::Notrump)
    );
    // Opener over the placed game: a minimum passes, 15+ asks.
    let placed = over(&[call(3, Strain::Spades), call(4, Strain::Spades)]);
    assert_eq!(best(&trie, &placed, "K73.AQ763.K4.Q96"), Call::Pass);
    assert_eq!(
        best(&trie, &placed, "K73.AKJ63.K4.A96"),
        call(4, Strain::Notrump)
    );

    let clubs = over(&[call(3, Strain::Clubs)]);
    assert_eq!(
        best(&trie, &clubs, "AQJ85.K2.A2.K853"),
        call(4, Strain::Clubs)
    );
    assert_eq!(
        best(&trie, &clubs, "AQJ852.K2.A2.K85"),
        call(3, Strain::Spades)
    );
    assert_eq!(
        best(&trie, &clubs, "AQJ85.K2.AJ2.K85"),
        call(3, Strain::Notrump)
    );

    let notrump = over(&[call(2, Strain::Notrump)]);
    assert_eq!(
        best(&trie, &notrump, "AQJ852.K2.A2.K85"),
        call(4, Strain::Spades)
    );
    assert_eq!(
        best(&trie, &notrump, "AQJ85.K2.AQ2.K85"),
        call(4, Strain::Notrump)
    );
}
