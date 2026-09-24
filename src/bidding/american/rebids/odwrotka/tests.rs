use crate::bidding::Bidder;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Strain};

const P: Call = Call::Pass;

fn odwrotka() -> Agreements {
    let mut agreements = Agreements::default();
    agreements.rebid.odwrotka = true;
    agreements
}

fn bid(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

fn calls(agreements: &Agreements, auction: &[Call], hand: &str) -> Call {
    let partnership = crate::bidding::american::american(agreements).bind();
    let hand = hand.parse().unwrap();
    let logits = partnership
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("a decision");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(call, _)| call)
        .unwrap()
}

#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(
        &odwrotka(),
        &[super::odwrotka_continuations()],
    );
}

/// Opener's `2♦!`: game-forcing, or invitational with exactly three trumps.
#[test]
fn opener_reverses_artificially() {
    let a = odwrotka();
    let one_heart = [bid(1, Strain::Clubs), P, bid(1, Strain::Hearts), P];
    // 17 HCP, exactly three hearts: the invitational odwrotka.
    assert_eq!(
        calls(&a, &one_heart, "AQx.KJx.Kx.AJxxx"),
        bid(2, Strain::Diamonds)
    );
    // Off the knob, the same hand is american's natural 2NT / club rebid world.
    assert_ne!(
        calls(&Agreements::default(), &one_heart, "AQx.KJx.Kx.AJxxx"),
        bid(2, Strain::Diamonds)
    );
    // 19 HCP with two hearts: game-forcing, still 2♦!.
    assert_eq!(
        calls(&a, &one_heart, "AQx.Kx.KQx.AJxxx"),
        bid(2, Strain::Diamonds)
    );
    // 17 HCP with four hearts: the natural jump raise outranks it.
    assert_eq!(
        calls(&a, &one_heart, "Ax.KJxx.Kx.AJxxx"),
        bid(3, Strain::Hearts)
    );
    // A minimum with three hearts: the natural club rebid or 1NT, never 2♦.
    assert_ne!(
        calls(&a, &one_heart, "Qxx.Kxx.xx.AJxxx"),
        bid(2, Strain::Diamonds)
    );
    // The natural diamond reverse is gone: a 16-count 5♣ 4♦ (17 points) is
    // american's 2♦ reverse off the knob and something else on it.
    let reverse = "Kx.Kx.AQxx.KJxxx";
    assert_eq!(
        calls(&Agreements::default(), &one_heart, reverse),
        bid(2, Strain::Diamonds)
    );
    assert_ne!(calls(&a, &one_heart, reverse), bid(2, Strain::Diamonds));
}

/// Responder's steps pin the major's length and the strength.
#[test]
fn responder_steps() {
    let a = odwrotka();
    let after_hearts = [
        bid(1, Strain::Clubs),
        P,
        bid(1, Strain::Hearts),
        P,
        bid(2, Strain::Diamonds),
        P,
    ];
    // Four hearts, 11: 2♠! (game-forcing four).
    assert_eq!(
        calls(&a, &after_hearts, "Kxx.QJxx.Kxx.Qxx"),
        bid(2, Strain::Spades)
    );
    // Four hearts, 7: 2♥! (minimum four).
    assert_eq!(
        calls(&a, &after_hearts, "xxx.QJxx.Kxx.xxx"),
        bid(2, Strain::Hearts)
    );
    // Five hearts, 11: 2NT!.
    assert_eq!(
        calls(&a, &after_hearts, "Kx.QJxxx.Kxx.Qxx"),
        bid(2, Strain::Notrump)
    );
    // Five hearts, 7: 3♣!.
    assert_eq!(
        calls(&a, &after_hearts, "xx.QJxxx.Kxx.xxx"),
        bid(3, Strain::Clubs)
    );
    // Six hearts, 11: 3♦!.
    assert_eq!(
        calls(&a, &after_hearts, "Kx.QJxxxx.Kx.Qxx"),
        bid(3, Strain::Diamonds)
    );
    // Six hearts, 6: 3♥.
    assert_eq!(
        calls(&a, &after_hearts, "xx.QJxxxx.Kx.xxx"),
        bid(3, Strain::Hearts)
    );
    // Over 1♠ the "other major" step is 2♥, below 2♠.
    let after_spades = [
        bid(1, Strain::Clubs),
        P,
        bid(1, Strain::Spades),
        P,
        bid(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(
        calls(&a, &after_spades, "QJxx.Kxx.Kxx.Qxx"),
        bid(2, Strain::Hearts)
    );
    assert_eq!(
        calls(&a, &after_spades, "QJxx.xxx.Kxx.xxx"),
        bid(2, Strain::Spades)
    );
}
