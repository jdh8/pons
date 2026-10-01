use super::super::tests::{P, best_with, bid};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

const C: Strain = Strain::Clubs;
const H: Strain = Strain::Hearts;
const S: Strain = Strain::Spades;
const NT: Strain = Strain::Notrump;

fn on() -> Agreements {
    Agreements::default()
}

fn off() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.rebid.strong_two_positive = false;
    agreements
}

/// Opener shows a five-card major over the `2NT` positive (spades first on
/// 5-5); without one the table rejects the hand and the floor bids, as off.
#[test]
fn opener_shows_a_major_over_the_notrump_positive() {
    let auction = [bid(2, C), P, bid(2, NT), P];
    assert_eq!(best_with(&on(), &auction, "AQ.AKJ72.K42.KQ8"), bid(3, H));
    assert_eq!(best_with(&on(), &auction, "AKJ94.AQ952.A.K2"), bid(3, S));
    let flat = "AQ4.KQ5.AKJ3.QJ2";
    assert_eq!(
        best_with(&on(), &auction, flat),
        best_with(&off(), &auction, flat)
    );
}

/// Over opener's major responder raises with three, else bids `6NT` on 9+ or
/// `3NT`; opener always asks over the raise.
#[test]
fn responder_raises_or_denies_the_major() {
    let shown = [bid(2, C), P, bid(2, NT), P, bid(3, H), P];
    assert_eq!(best_with(&on(), &shown, "K63.Q94.J92.KT64"), bid(4, H));
    assert_eq!(best_with(&on(), &shown, "KJ63.94.Q92.KT64"), bid(6, NT));
    assert_eq!(best_with(&on(), &shown, "KJ63.94.Q92.QT64"), bid(3, NT));

    let raised = [bid(2, C), P, bid(2, NT), P, bid(3, H), P, bid(4, H), P];
    assert_eq!(best_with(&on(), &raised, "AQ.AKJ72.K42.QJ8"), bid(4, NT));

    let denied = [bid(2, C), P, bid(2, NT), P, bid(3, H), P, bid(3, NT), P];
    assert_eq!(best_with(&on(), &denied, "AQ.AKJ72.K42.KQ8"), P);
    assert_eq!(best_with(&on(), &denied, "AK.AKJ72.K42.KQ8"), bid(6, NT));
    assert_eq!(best_with(&on(), &denied, "AK.AKJ942.KJ3.Q7"), bid(4, H));
}

/// Opener asks at once with three-card support for a major positive, and
/// shows his own major without it.
#[test]
fn opener_asks_over_the_major_positive() {
    let auction = [bid(2, C), P, bid(2, H), P];
    assert_eq!(best_with(&on(), &auction, "AQ4.KQ5.AKJ3.KJ2"), bid(4, NT));
    assert_eq!(best_with(&on(), &auction, "AKJ94.Q5.AKJ.KJ2"), bid(2, S));
}

/// With `strong_two_waiting` off, `2♥` is the double negative and keeps its
/// own subtree.
#[test]
fn the_double_negative_is_untouched() {
    let mut agreements = on();
    agreements.decision.strong_two_waiting = false;
    let auction = [bid(2, C), P, bid(2, H), P];
    assert_eq!(
        best_with(&agreements, &auction, "AQ4.KQ5.AKJ3.KJ2"),
        bid(2, NT)
    );
}

/// Over their double of `2♣` these tables are reached through the systems-on
/// rebase, where a rejected hand cannot fall through to the floor: opener
/// bids `3NT` rather than pass a game force.
#[test]
fn opener_does_not_pass_the_doubled_positive() {
    let auction = [bid(2, C), Call::Double, bid(2, NT), P];
    assert_eq!(best_with(&on(), &auction, "A54.AK2.AKJ.QJ82"), bid(3, NT));
    assert_eq!(best_with(&on(), &auction, "AQ.AKJ72.K42.KQ8"), bid(3, H));
}
