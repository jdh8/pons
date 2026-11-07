use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::{Bidder, Trie};
use contract_bridge::Hand;
use contract_bridge::auction::RelativeVulnerability;

fn build(on: bool) -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.minor_jump_shift_answers = on;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

fn over(extra: &[Call]) -> Vec<Call> {
    let mut auction = vec![
        call(1, Strain::Clubs),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    auction.extend_from_slice(extra);
    auction
}

/// Responder never passes the forcing jump shift (the anchor's loser), and
/// each answer lands on its hand.
#[test]
fn minor_jump_shift_is_answered() {
    let trie = build(true);
    let b = |hand| best(&trie, &over(&[]), hand);
    // ♠T865 ♥QJ74 ♦— ♣K5432 — 6 HCP, a diamond void: the floor passed `3♦`
    // on this shape.
    assert_eq!(b("T865.QJ74..K5432"), call(5, Strain::Clubs));
    // ♠T8652 ♥QJ74 ♦— ♣K542 — five spades show first.
    assert_eq!(b("T8652.QJ74..K542"), call(3, Strain::Spades));
    // ♠KQ964 ♥52 ♦T2 ♣A854 — four clubs, 9 HCP: too weak for the slam raise.
    assert_eq!(b("KQ964.52.T2.A854"), call(3, Strain::Spades));
    // ♠AKJ7 ♥32 ♦K5 ♣KT64 — four clubs, 14 HCP: the slam raise.
    assert_eq!(b("AKJ7.32.K5.KT64"), call(4, Strain::Clubs));
    // ♠AQ75 ♥K2 ♦KJ84 ♣732 — four diamonds, 13 HCP.
    assert_eq!(b("AQ75.K2.KJ84.732"), call(4, Strain::Diamonds));
    // ♠QJ874 ♥K4 ♦93 ♣Q42 — five spades.
    assert_eq!(b("QJ874.K4.93.Q42"), call(3, Strain::Spades));
    // ♠Q876 ♥KJ4 ♦93 ♣Q42 — nothing to show.
    assert_eq!(b("Q876.KJ4.93.Q42"), call(3, Strain::Notrump));
}

/// Opener's rebids: keycards over the raise, the raise over five spades, the
/// 6-5 pull of `3NT`.
#[test]
fn minor_jump_shift_opener_continues() {
    let trie = build(true);
    let raise = over(&[call(4, Strain::Clubs), Call::Pass]);
    assert_eq!(
        best(&trie, &raise, "A4.K.AQ74.AQJ93"),
        call(4, Strain::Notrump)
    );
    let spades = over(&[call(3, Strain::Spades), Call::Pass]);
    assert_eq!(
        best(&trie, &spades, "A42.K.AQ74.AQ973"),
        call(4, Strain::Spades)
    );
    assert_eq!(
        best(&trie, &spades, "A.K4.AQ74.AQJ973"),
        call(3, Strain::Notrump)
    );
    let notrump = over(&[call(3, Strain::Notrump), Call::Pass]);
    // ♠2 ♥— ♦AKQT9 ♣AKT8764 — the anchor's 7-5 that passed `3NT`.
    assert_eq!(
        best(&trie, &notrump, "2..AKQT9.AKT8764"),
        call(4, Strain::Clubs)
    );
    assert_eq!(best(&trie, &notrump, "A42.K.AQ74.AQ973"), Call::Pass);
}

/// Off, the node is not in the book at all.
#[test]
fn minor_jump_shift_off_leaves_the_floor() {
    let hand: Hand = "Q876.KJ4.93.Q42".parse().expect("valid test hand");
    assert!(
        build(false)
            .classify(hand, RelativeVulnerability::NONE, &over(&[]))
            .is_none()
    );
}

/// Every lane the ladder jump-shifts into has its node, including the
/// two-level `2♠` over a `1♥` response.
#[test]
fn minor_jump_shift_covers_the_two_spade_lane() {
    let trie = build(true);
    let auction = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ];
    // ♠Q4 ♥KJ874 ♦93 ♣Q742 — five hearts: the cheapest heart rebid is `3♥`.
    assert_eq!(
        best(&trie, &auction, "Q4.KJ874.93.Q742"),
        call(3, Strain::Hearts)
    );
    // ♠KT84 ♥QJ74 ♦9 ♣Q742 — four spades: game in opener's second suit.
    assert_eq!(
        best(&trie, &auction, "KT84.QJ74.9.Q742"),
        call(4, Strain::Spades)
    );
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

/// The A/B's worst boards: an 11+ raise of a major second suit asks keycards
/// instead of stopping in `4M`, and opener asks over the 6–10 game on 21+.
#[test]
fn minor_jump_shift_major_raise_asks_on_slam_values() {
    let trie = build(true);
    let node = auction("1♣ - 1♠ - 3♥ -");
    // ♠KJT63 ♥K743 ♦T5 ♣KJ — 11 HCP opposite 18+: `4♥` missed 6♥.
    assert_eq!(
        best(&trie, &node, "KJT63.K743.T5.KJ"),
        call(4, Strain::Notrump)
    );
    // ♠KQ964 ♥Q873 ♦T2 ♣85 — 7 HCP: the game.
    assert_eq!(
        best(&trie, &node, "KQ964.Q873.T2.85"),
        call(4, Strain::Hearts)
    );
    let game = auction("1♣ - 1♠ - 3♥ - 4♥ -");
    assert_eq!(
        best(&trie, &game, "A.AKQ8.KQ8.AQJ75"),
        call(4, Strain::Notrump)
    );
    assert_eq!(best(&trie, &game, "A.AJ63.K98.KQJ75"), Call::Pass);
}

/// A six-card major pulls opener's `3NT` denial of three-card support.
#[test]
fn minor_jump_shift_six_card_major_pulls_notrump() {
    let trie = build(true);
    let spades = auction("1♦ - 1♠ - 3♣ - 3♠ - 3NT -");
    // ♠AQJ643 ♥J2 ♦J5 ♣953 — 4♠ opposite K8 where 3NT failed.
    assert_eq!(
        best(&trie, &spades, "AQJ643.J2.J5.953"),
        call(4, Strain::Spades)
    );
    assert_eq!(best(&trie, &spades, "AQJ64.J32.J5.953"), Call::Pass);
    let hearts = auction("1♣ - 1♥ - 2♠ - 3♥ - 3NT -");
    assert_eq!(
        best(&trie, &hearts, "J8.KJ87643.K9.87"),
        call(4, Strain::Hearts)
    );
}
