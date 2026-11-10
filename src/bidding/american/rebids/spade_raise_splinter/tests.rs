use super::super::register;
use super::super::tests::best;
use super::*;
use crate::bidding::Trie;

fn build(splinter: Option<u8>) -> Trie {
    let mut agreements = crate::bidding::agreements::Agreements::default();
    agreements.rebid.spade_raise_splinter = splinter;
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

/// Opener splinters on the `4♠` range with a short suit; the knob's value
/// caps the honours in it, and off the hand raises to `4♠`.
#[test]
fn opener_splinters_instead_of_the_four_spade_raise() {
    let (off, small, any) = (build(None), build(Some(1)), build(Some(4)));
    let four_spades = call(4, Strain::Spades);
    // ♠KQ84 ♥A6 ♦AJ87 ♣K53 — 17 HCP 4432: no short suit, `4♠` all round.
    for trie in [&off, &small, &any] {
        assert_eq!(
            best(trie, &after(Strain::Diamonds, &[]), "KQ84.A6.AJ87.K53"),
            four_spades
        );
    }
    // ♠KQ84 ♥7 ♦AJ87 ♣AQ53 — 16 HCP, 17 points, small heart singleton: `4♥` after
    // `1♦`, `4♥` after `1♣` too.
    for opening in [Strain::Clubs, Strain::Diamonds] {
        let auction = after(opening, &[]);
        assert_eq!(best(&off, &auction, "KQ84.7.AJ87.AQ53"), four_spades);
        assert_eq!(
            best(&small, &auction, "KQ84.7.AJ87.AQ53"),
            call(4, Strain::Hearts)
        );
        assert_eq!(
            best(&any, &auction, "KQ84.7.AJ87.AQ53"),
            call(4, Strain::Hearts)
        );
    }
    // ♠AQ84 ♥K ♦AJ87 ♣KQ53 — 19 HCP, singleton ♥K: BBA's rule raises `4♠`,
    // the any-singleton arm splinters.
    let auction = after(Strain::Diamonds, &[]);
    assert_eq!(best(&small, &auction, "AQ84.K.AJ87.KQ53"), four_spades);
    assert_eq!(
        best(&any, &auction, "AQ84.K.AJ87.KQ53"),
        call(4, Strain::Hearts)
    );
    // ♠KQ84 ♥AJ876 ♦7 ♣AK5 — 17 HCP, 18 points, after `1♥`: `4♦`.
    assert_eq!(
        best(&small, &after(Strain::Hearts, &[]), "KQ84.AJ876.7.AK5"),
        call(4, Strain::Diamonds)
    );
    // ♠KQ84 ♥7 ♦KJ87 ♣KQ53 — 14 HCP, 15 points, with a singleton: the invite, no
    // splinter below the `4♠` range.
    assert_eq!(
        best(&small, &auction, "KQ84.7.KJ87.KQ53"),
        call(3, Strain::Spades)
    );
}

/// Responder asks keycards on points less half the waste in the short
/// suit reaching 10, else signs off in `4♠`; the same over their double.
#[test]
fn responder_answers_the_splinter() {
    let trie = build(Some(1));
    let rkcb = call(4, Strain::Notrump);
    let game = call(4, Strain::Spades);
    for tail in [Call::Pass, Call::Double] {
        // `1♣ - 1♠ - 4♦`, the short suit diamonds.
        let auction = after(Strain::Clubs, &[call(4, Strain::Diamonds), tail]);
        // ♠J854 ♥K63 ♦92 ♣Q842 — 6 HCP: sign off.
        assert_eq!(best(&trie, &auction, "J854.K63.92.Q842"), game);
        // ♠AJ85 ♥K63 ♦92 ♣Q842 — 10 HCP, no waste: slam try.
        assert_eq!(best(&trie, &auction, "AJ85.K63.92.Q842"), rkcb);
        // ♠AJ85 ♥763 ♦K2 ♣Q842 — 10 HCP with ♦K wasted: sign off.
        assert_eq!(best(&trie, &auction, "AJ85.763.K2.Q842"), game);
        // ♠AJ85 ♥K63 ♦K2 ♣J842 — 12 HCP with ♦K wasted (12 − 1½): slam try.
        assert_eq!(best(&trie, &auction, "AJ85.K63.K2.J842"), rkcb);
        // ♠QJ85 ♥K63 ♦A2 ♣8642 — 10 HCP, the ♦A is not waste: slam try.
        assert_eq!(best(&trie, &auction, "QJ85.K63.A2.8642"), rkcb);
        // ♠QJ85 ♥763 ♦AK ♣Q842 — 12 HCP with only the ♦K wasted (12 − 1½): slam try.
        assert_eq!(best(&trie, &auction, "QJ85.763.AK.Q842"), rkcb);
    }
    // Opener passes the sign-off.
    let signed_off = after(
        Strain::Clubs,
        &[call(4, Strain::Diamonds), Call::Pass, game, Call::Pass],
    );
    assert_eq!(best(&trie, &signed_off, "KQ84.AJ7.7.AK953"), Call::Pass);
    // Their `(4♥)` over `1♦ - 1♠ - 4♣`: the game.
    let overcalled = after(
        Strain::Diamonds,
        &[call(4, Strain::Clubs), call(4, Strain::Hearts)],
    );
    assert_eq!(best(&trie, &overcalled, "J854.K63.92.Q842"), game);
}
