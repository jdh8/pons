use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;

fn build(on: bool) -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.spade_raise_ladder = on;
    agreements.rebid.spade_raise_answers = on;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    trie
}

fn after(opening: Strain, extra: &[Call]) -> Vec<Call> {
    let mut auction = vec![
        call(1, opening),
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
    ];
    auction.extend_from_slice(extra);
    auction
}

/// The ladder moves down a rung after every opening; hearts over `1m - 1♥`
/// follow their own cuts, not this knob.
#[test]
fn spade_raise_ladder_moves_opener_down_a_rung() {
    let (off, on) = (build(false), build(true));
    for opening in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
        let auction = after(opening, &[]);
        // ♠KQ84 ♥A6 ♦AJ87 ♣K53 — 17 HCP 4432: invite off, game on.
        let seventeen = if opening == Strain::Hearts {
            "KQ84.AJ876.A6.K5"
        } else {
            "KQ84.A6.AJ87.K53"
        };
        assert_eq!(best(&off, &auction, seventeen), call(3, Strain::Spades));
        assert_eq!(best(&on, &auction, seventeen), call(4, Strain::Spades));
    }
    // ♠KQ84 ♥A6 ♦KJ87 ♣Q53 — 15 HCP: `2♠` off, `3♠` on.
    let auction = after(Strain::Diamonds, &[]);
    assert_eq!(
        best(&off, &auction, "KQ84.A6.KJ87.Q53"),
        call(2, Strain::Spades)
    );
    assert_eq!(
        best(&on, &auction, "KQ84.A6.KJ87.Q53"),
        call(3, Strain::Spades)
    );
    // ♠KQ84 ♥A6 ♦Q987 ♣Q53 — 13 HCP: `2♠` either way.
    assert_eq!(
        best(&on, &auction, "KQ84.A6.Q987.Q53"),
        call(2, Strain::Spades)
    );
    // Hearts over `1♦ - 1♥` ignore the spade knob: 17 is `4♥` on their own
    // default cuts either way.
    let hearts = [
        call(1, Strain::Diamonds),
        Call::Pass,
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    // ♠A6 ♥KQ84 ♦AJ87 ♣K53 — 17 HCP.
    for trie in [&off, &on] {
        assert_eq!(
            best(trie, &hearts, "A6.KQ84.AJ87.K53"),
            call(4, Strain::Hearts)
        );
    }
}

/// The two heart cuts default to the spade ladder's 15 / 17; at the old
/// 16 / 19 the raises over `1m - 1♥` move back up a rung.
#[test]
fn heart_raise_ladder_moves_opener_down_a_rung() {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    assert_eq!(
        (
            agreements.rebid.heart_raise_invite,
            agreements.rebid.heart_raise_game
        ),
        (15, 17)
    );
    let on = build(true);
    agreements.rebid.heart_raise_invite = 16;
    agreements.rebid.heart_raise_game = 19;
    let mut off = Trie::new();
    register(&mut off, &agreements);
    for opening in [Strain::Clubs, Strain::Diamonds] {
        let hearts = [
            call(1, opening),
            Call::Pass,
            call(1, Strain::Hearts),
            Call::Pass,
        ];
        // 17 HCP: invite off, game on.
        assert_eq!(
            best(&off, &hearts, "A6.KQ84.AJ87.K53"),
            call(3, Strain::Hearts)
        );
        assert_eq!(
            best(&on, &hearts, "A6.KQ84.AJ87.K53"),
            call(4, Strain::Hearts)
        );
        // 15 HCP: `2♥` off, `3♥` on.
        assert_eq!(
            best(&off, &hearts, "A6.KQ84.KJ87.Q53"),
            call(2, Strain::Hearts)
        );
        assert_eq!(
            best(&on, &hearts, "A6.KQ84.KJ87.Q53"),
            call(3, Strain::Hearts)
        );
        // 13 HCP: `2♥` either way.
        assert_eq!(
            best(&on, &hearts, "A6.KQ84.Q987.Q53"),
            call(2, Strain::Hearts)
        );
    }
}

/// Responder accepts the 15–16 `3♠` on 7+ and asks keycards on 14+, also
/// after a minor opening and over their double.
#[test]
fn spade_raise_ladder_answers_three_and_four_spades() {
    let trie = build(true);
    let three = |opening, tail| after(opening, &[call(3, Strain::Spades), tail]);
    let x = Call::Double;
    for opening in [Strain::Clubs, Strain::Diamonds, Strain::Hearts] {
        for tail in [Call::Pass, x] {
            // ♠J854 ♥K63 ♦92 ♣Q842 — 6 HCP: pass.
            assert_eq!(
                best(&trie, &three(opening, tail), "J854.K63.92.Q842"),
                Call::Pass
            );
            // ♠J854 ♥K63 ♦92 ♣K842 — 7 HCP: game.
            assert_eq!(
                best(&trie, &three(opening, tail), "J854.K63.92.K842"),
                call(4, Strain::Spades)
            );
            // ♠AJ85 ♥K63 ♦K2 ♣K842 — 14 HCP: keycards.
            assert_eq!(
                best(&trie, &three(opening, tail), "AJ85.K63.K2.K842"),
                call(4, Strain::Notrump)
            );
        }
        let four = after(opening, &[call(4, Strain::Spades), Call::Pass]);
        // ♠J854 ♥K63 ♦K2 ♣Q842 — 9 HCP: pass the 17+ game.
        assert_eq!(best(&trie, &four, "J854.K63.K2.Q842"), Call::Pass);
        // ♠AJ85 ♥KJ3 ♦92 ♣K842 — 12 HCP: keycards.
        assert_eq!(
            best(&trie, &four, "AJ85.KJ3.92.K842"),
            call(4, Strain::Notrump)
        );
    }
    // Their `(4♥)` over `1♦ - 1♠ - 3♠`: game on 7+.
    let overcalled = after(
        Strain::Diamonds,
        &[call(3, Strain::Spades), call(4, Strain::Hearts)],
    );
    assert_eq!(
        best(&trie, &overcalled, "J854.K63.92.K842"),
        call(4, Strain::Spades)
    );
}

/// Game over `1♥ - 1♠ - 2♠` moves down to 11+, or 10+ with a fifth spade;
/// the minor lanes keep the floor.
#[test]
fn spade_raise_game_gate() {
    let build = |on| {
        let mut agreements = crate::bidding::agreements::Agreements::default();
        agreements.rebid.spade_raise_game = on;
        let mut trie = Trie::new();
        register(&mut trie, &agreements);
        trie
    };
    let (off, on) = (build(false), build(true));
    let game = call(4, Strain::Spades);
    let invite = call(3, Strain::Spades);
    let hearts = after(Strain::Hearts, &[call(2, Strain::Spades), Call::Pass]);
    // ♠KJ85 ♥K63 ♦92 ♣A842 — 11 HCP: invite off, game on.
    assert_eq!(best(&off, &hearts, "KJ85.K63.92.A842"), invite);
    assert_eq!(best(&on, &hearts, "KJ85.K63.92.A842"), game);
    // ♠QJ852 ♥K63 ♦92 ♣A84 — 10 points, 5 spades: game on the fifth spade.
    assert_eq!(best(&on, &hearts, "QJ852.K63.92.A84"), game);
    // ♠KJ85 ♥K63 ♦92 ♣Q842 — 9 HCP, 4 spades, and 10 with four: no game.
    assert_eq!(best(&on, &hearts, "KJ85.K63.92.Q842"), Call::Pass);
    assert_eq!(best(&on, &hearts, "KJ85.K63.92.K842"), invite);
}

/// The two RKCB floors are knobs: lowering them moves the ask, not the game
/// acceptance.
#[test]
fn spade_raise_ladder_asks_are_knobs() {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.spade_raise_answers = true;
    agreements.rebid.spade_raise_three_ask = 11;
    agreements.rebid.spade_raise_four_ask = 10;
    let mut trie = Trie::new();
    register(&mut trie, &agreements);
    let three = after(Strain::Diamonds, &[call(3, Strain::Spades), Call::Pass]);
    let four = after(Strain::Diamonds, &[call(4, Strain::Spades), Call::Pass]);
    // ♠J854 ♥K63 ♦K2 ♣KQ42 — 13 HCP, 12 points: keycards over `3♠` at 11,
    // game at 14.
    assert_eq!(
        best(&trie, &three, "J854.K63.K2.KQ42"),
        call(4, Strain::Notrump)
    );
    assert_eq!(
        best(&build(true), &three, "J854.K63.K2.KQ42"),
        call(4, Strain::Spades)
    );
    // ♠J854 ♥K63 ♦K2 ♣KJ42 — 11 HCP, 11 points: keycards over `4♠` at 10,
    // pass at 12.
    assert_eq!(
        best(&trie, &four, "J854.K63.K2.KJ42"),
        call(4, Strain::Notrump)
    );
    assert_eq!(best(&build(true), &four, "J854.K63.K2.KJ42"), Call::Pass);
    // ♠J854 ♥K63 ♦92 ♣K842 — 7 HCP: still `4♠` / pass.
    assert_eq!(
        best(&trie, &three, "J854.K63.92.K842"),
        call(4, Strain::Spades)
    );
    assert_eq!(best(&trie, &four, "J854.K63.92.K842"), Call::Pass);
}
