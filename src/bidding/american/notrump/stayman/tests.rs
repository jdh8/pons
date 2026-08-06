use super::super::tests::{P, best, bid};
use contract_bridge::Strain;

#[test]
fn stayman_fit_raise_by_value() {
    // 1NT - 2♣ - 2♥ (opener's four-card major): responder raises on `fit_value`,
    // not raw HCP — any upgrade past a flat eight reaches game.
    let stayman = [
        bid(1, Strain::Notrump),
        P,
        bid(2, Strain::Clubs),
        P,
        bid(2, Strain::Hearts),
        P,
    ];

    // Flat 4-3-3-3 eight, four-card fit, no upgrade: invitational raise (value 8).
    assert_eq!(best(&stayman, "K32.Q654.K32.432"), bid(3, Strain::Hearts));
    // 4-4-4-1 eight with a working singleton: the shape upgrades to value 9, so
    // the same eight now bids game instead of merely inviting.
    assert_eq!(best(&stayman, "Q543.K654.K432.2"), bid(4, Strain::Hearts));
    // Flat 4-3-3-3 seven: value 7, below the invite — passes the partscore.
    assert_eq!(best(&stayman, "K32.Q654.Q32.432"), P);
}
