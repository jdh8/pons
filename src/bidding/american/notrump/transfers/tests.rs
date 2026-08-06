use super::super::tests::{P, best, bid};
use contract_bridge::Strain;

/// The longer-major transfer discipline (default on): a two-suiter
/// transfers to its longer major, and equal lengths split by strength —
/// weak to hearts, invitational/minimum-game-force to the both-majors 3♦,
/// slam tries to spades for the `1NT - 2♥ - 2♠ - 3♥` structure.
#[test]
fn transfers_prefer_the_longer_major() {
    let one_nt = [bid(1, Strain::Notrump), P];

    // 6♠5♥ transfers to spades whatever the strength (the legacy guards
    // tied on the weak hand, and 3♦ grabbed the strong one, losing the
    // sixth spade).
    assert_eq!(best(&one_nt, "QJ9642.98763.4.3"), bid(2, Strain::Hearts));
    assert_eq!(best(&one_nt, "KJ9642.AKJ63.J.3"), bid(2, Strain::Hearts));
    // 6♥5♠ transfers to hearts.
    assert_eq!(best(&one_nt, "98763.QJ9642.4.3"), bid(2, Strain::Diamonds));

    // Equal 5-5: weak prefers hearts for safety...
    assert_eq!(best(&one_nt, "J9863.J9642.4.3"), bid(2, Strain::Diamonds));
    // ...invitational / minimum game force shows both at once via 3♦...
    assert_eq!(best(&one_nt, "KJ863.KJ642.4.3"), bid(3, Strain::Diamonds));
    // ...and a slam try transfers to spades, then bids the natural
    // game-forcing 3♥ — the 5-5 slam-try structure.
    assert_eq!(best(&one_nt, "AKJ63.AKJ42.4.3"), bid(2, Strain::Hearts));
    let over_completion = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Hearts),
        P,
        bid(2, Strain::Spades),
        P,
    ];
    assert_eq!(
        best(&over_completion, "AKJ63.AKJ42.4.3"),
        bid(3, Strain::Hearts)
    );

    // The 2NT-strength table follows the same discipline: longer major,
    // hearts on every tie (no both-majors bid at this level).
    let two_nt = [bid(2, Strain::Notrump), P];
    assert_eq!(best(&two_nt, "QJ9642.98763.4.3"), bid(3, Strain::Hearts));
    assert_eq!(best(&two_nt, "J9863.J9642.4.3"), bid(3, Strain::Diamonds));
}
