use super::super::tests::{P, best_with, bid};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

const C: Strain = Strain::Clubs;
const D: Strain = Strain::Diamonds;
const H: Strain = Strain::Hearts;
const S: Strain = Strain::Spades;
const NT: Strain = Strain::Notrump;

fn on() -> Agreements {
    Agreements::default()
}

/// The classic ladder: `strong_two_grand` off
fn classic() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.rebid.strong_two_grand = false;
    agreements
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

/// The grand rung: all five keycards and the trump queen bid seven on two
/// side kings — at once with both, through `5NT` with one, where the classic
/// ladder stops in six short of all three.
#[test]
fn grand_rung_bids_seven_on_two_side_kings() {
    let answered = [bid(2, C), P, bid(2, S), P, bid(4, NT), P, bid(5, C), P];
    // Four keycards and the queen opposite one: two side kings of our own.
    assert_eq!(best_with(&on(), &answered, "KQ3.AK963.AK.A53"), bid(7, S));
    // One side king: ask.
    let asker = "KQ3.AQ963.AK.AJ3";
    assert_eq!(best_with(&on(), &answered, asker), bid(5, NT));

    let asked = [&answered[..], &[bid(5, NT), P]].concat();
    assert_eq!(best_with(&on(), &asked, "AJT864.K4.Q54.K8"), bid(7, S));
    assert_eq!(best_with(&on(), &asked, "AJT864.4.Q54.KT8"), bid(6, D));

    let one = [&asked[..], &[bid(6, D), P]].concat();
    assert_eq!(best_with(&on(), &one, asker), bid(7, S));
    assert_eq!(best_with(&classic(), &one, asker), bid(6, S));
    let none = [&asked[..], &[bid(6, C), P]].concat();
    assert_eq!(best_with(&on(), &none, asker), bid(6, S));

    // Their lead-directing double of the answer is systems on.
    let doubled = [&asked[..], &[bid(6, D), Call::Double]].concat();
    assert_eq!(best_with(&on(), &doubled, asker), bid(7, S));
}

/// Over `5♥` — two keycards, no queen — the asker's own queen settles the
/// suit, and the classic table has no king ask at all.
#[test]
fn grand_rung_asks_over_the_queenless_answer() {
    let answered = [bid(2, C), P, bid(2, S), P, bid(4, NT), P, bid(5, H), P];
    let asker = "KQ2.QJ2.AQJ6.AK9";
    assert_eq!(best_with(&on(), &answered, asker), bid(5, NT));
    assert_eq!(best_with(&classic(), &answered, asker), bid(6, S));
    // Without the queen, six.
    assert_eq!(best_with(&on(), &answered, "K92.QJ2.AQJ6.AKQ"), bid(6, S));
}

/// The same ladder answers the floor's `4NT` over a minor positive: seven on
/// two side kings, `5NT` on exactly one, and partner raises to seven with
/// another.
#[test]
fn grand_rung_reaches_the_minors() {
    let ask = [bid(2, C), P, bid(3, D), P, bid(4, NT), P];
    assert_eq!(best_with(&on(), &ask, "642.QJ73.KQT82.A"), bid(5, S));

    let queen = [&ask[..], &[bid(5, S), P]].concat();
    assert_eq!(best_with(&on(), &queen, "AKJ.AK.A63.KJ763"), bid(7, D));

    // Three trumps want all three kings: with two, ask for the third.
    assert_eq!(best_with(&on(), &queen, "AKJ.AQ.A63.KJ763"), bid(5, NT));

    let one = [&ask[..], &[bid(5, C), P]].concat();
    let asker = "AQJ.AK2.AJ63.A63";
    assert_eq!(best_with(&on(), &one, asker), bid(5, NT));
    let asked = [&one[..], &[bid(5, NT), P]].concat();
    assert_eq!(best_with(&on(), &asked, "K6.853.KQ842.J82"), bid(7, D));
    assert_eq!(best_with(&on(), &asked, "Q6.853.KQ842.Q82"), bid(6, D));
    let stopped = [&asked[..], &[bid(6, D), P]].concat();
    assert_eq!(best_with(&on(), &stopped, asker), P);
}
