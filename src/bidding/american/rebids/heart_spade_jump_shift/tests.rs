use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;

fn build(on: bool) -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.heart_spade_jump_shift = on;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

/// `heart_spade_jump_shift`: over `1♥ - 1♠` an 18+ 5-4 jump-shifts into its
/// minor; a minimum keeps the `2m` rebid.
#[test]
fn heart_spade_jump_shift_shows_the_big_two_suiter() {
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
    ];
    let (off, on) = (build(false), build(true));
    // ♠K4 ♥AKJ84 ♦3 ♣AQJ5 — 19 HCP, 5-4.
    let clubs = "K4.AKJ84.3.AQJ5";
    assert_eq!(best(&off, &auction, clubs), call(2, Strain::Clubs));
    assert_eq!(best(&on, &auction, clubs), call(3, Strain::Clubs));
    // ♠5 ♥AKQ76 ♦AKJ4 ♣K82 — 19 HCP into diamonds.
    assert_eq!(
        best(&on, &auction, "5.AKQ76.AKJ4.K82"),
        call(3, Strain::Diamonds)
    );
    // ♠54 ♥AQ976 ♦3 ♣KQJ5 — 12 HCP: the minimum `2♣`.
    assert_eq!(
        best(&on, &auction, "54.AQ976.3.KQJ5"),
        call(2, Strain::Clubs)
    );
}

/// Responder never passes the forcing jump shift, and opener asks keycards
/// over the minor raise.
#[test]
fn heart_spade_jump_shift_is_answered() {
    let trie = build(true);
    let over = |extra: &[Call]| {
        let mut auction = vec![
            call(1, Strain::Hearts),
            Call::Pass,
            call(1, Strain::Spades),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        auction.extend_from_slice(extra);
        auction
    };
    let b = |hand| best(&trie, &over(&[]), hand);
    // ♠KJ8752 ♥42 ♦Q93 ♣84 — six spades, 6 HCP: the floor passed this.
    assert_eq!(b("KJ8752.42.Q93.84"), call(3, Strain::Spades));
    // ♠QJ76 ♥K32 ♦T84 ♣932 — three hearts.
    assert_eq!(b("QJ76.K32.T84.932"), call(4, Strain::Hearts));
    // ♠AKJ7 ♥Q32 ♦K5 ♣Q642 — three hearts, 15 HCP: keycards.
    assert_eq!(b("AKJ7.Q32.K5.Q642"), call(4, Strain::Notrump));
    // ♠AKJ7 ♥32 ♦K5 ♣KT64 — four clubs, 14 HCP: the slam raise.
    assert_eq!(b("AKJ7.32.K5.KT64"), call(4, Strain::Clubs));
    // ♠Q8765 ♥J4 ♦K93 ♣Q42 — nothing to show.
    assert_eq!(b("Q8765.J4.K93.Q42"), call(3, Strain::Notrump));
    // Opener over the raise asks keycards.
    let raise = over(&[call(4, Strain::Clubs), Call::Pass]);
    assert_eq!(
        best(&trie, &raise, "K4.AKJ84.3.AQJ5"),
        call(4, Strain::Notrump)
    );
}

/// Six hearts keep the `3♥` jump rebid.
#[test]
fn heart_spade_jump_shift_leaves_six_hearts_to_the_jump_rebid() {
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
    ];
    // ♠K ♥AQT732 ♦K6 ♣KQT7 — the seed-1 loser.
    assert_eq!(
        best(&build(true), &auction, "K.AQT732.K6.KQT7"),
        call(3, Strain::Hearts)
    );
}
