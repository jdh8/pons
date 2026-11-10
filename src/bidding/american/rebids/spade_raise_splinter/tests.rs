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

/// Responder asks keycards on points against the K/Q/J wasted in the short
/// suit, a fifth spade worth two, else signs off in `4♠`; the same over their
/// double.
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
        // ♠AJ85 ♥K63 ♦K2 ♣J842 — 12 HCP with ♦K wasted: slam try.
        assert_eq!(best(&trie, &auction, "AJ85.K63.K2.J842"), rkcb);
        // ♠AJ85 ♥K63 ♦QJ ♣8642 — 11 HCP with ♦QJ wasted, four spades: sign off.
        assert_eq!(best(&trie, &auction, "AJ85.K63.QJ.8642"), game);
        // ♠AJ854 ♥K63 ♦92 ♣842 — 8 HCP, no waste, a fifth spade: slam try.
        assert_eq!(best(&trie, &auction, "AJ854.K63.92.842"), rkcb);
        // ♠AJ85 ♥K63 ♦92 ♣8642 — the same 8 HCP on four spades: sign off.
        assert_eq!(best(&trie, &auction, "AJ85.K63.92.8642"), game);
        // ♠QJ85 ♥K63 ♦A2 ♣8642 — 10 HCP, the ♦A is not waste: slam try.
        assert_eq!(best(&trie, &auction, "QJ85.K63.A2.8642"), rkcb);
        // ♠QJ85 ♥763 ♦AK ♣Q842 — 12 HCP with only the ♦K wasted: slam try.
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

/// With all five keycards and the trump queen the asker bids `7♠` on two side
/// kings between the hands, or on one with 13+ HCP of its own.
#[test]
fn asker_bids_the_grand_on_keycards_and_kings() {
    let trie = build(Some(1));
    let seven = call(7, Strain::Spades);
    let six = call(6, Strain::Spades);
    let king_ask = call(5, Strain::Notrump);
    // `1♣ - 1♠ - 4♦ - 4NT - 5♠`: two keycards and the ♠Q.
    let answered = after(
        Strain::Clubs,
        &[
            call(4, Strain::Diamonds),
            Call::Pass,
            call(4, Strain::Notrump),
            Call::Pass,
            call(5, Strain::Spades),
            Call::Pass,
        ],
    );
    // ♠A852 ♥A63 ♦A2 ♣K842 — three keycards, the ♣K, 15 HCP: seven at once.
    assert_eq!(best(&trie, &answered, "A852.A63.A2.K842"), seven);
    // ♠A852 ♥K63 ♦A2 ♣K842 — two keycards, one missing: six.
    assert_eq!(best(&trie, &answered, "A852.K63.A2.K842"), six);
    // ♠A852 ♥A63 ♦A2 ♣Q842 — three keycards, no side king: the king ask.
    assert_eq!(best(&trie, &answered, "A852.A63.A2.Q842"), king_ask);
    let asked = |reply: Call| {
        let mut auction = answered.clone();
        auction.extend([king_ask, Call::Pass, reply, Call::Pass]);
        auction
    };
    // One king shown: seven on 14 HCP, six on 12; none shown: six.
    let one_king = asked(call(6, Strain::Diamonds));
    assert_eq!(best(&trie, &one_king, "A852.A63.A2.Q842"), seven);
    assert_eq!(best(&trie, &one_king, "A852.A63.A2.8642"), six);
    let no_king = asked(call(6, Strain::Clubs));
    assert_eq!(best(&trie, &no_king, "A852.A63.A2.Q842"), six);
    // Opener over the king ask: ♠KQ84 ♥KJ7 ♦7 ♣AK953 — two side kings, seven.
    let mut kings = answered.clone();
    kings.extend([king_ask, Call::Pass]);
    assert_eq!(best(&trie, &kings, "KQ84.KJ7.7.AK953"), seven);
}
