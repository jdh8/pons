use super::super::tests::best_call_with;
use super::*;
use crate::bidding::agreements::Agreements;
use contract_bridge::auction::Call;

/// The seam's whole point: a hand too strong for a natural overcall doubles
/// first and names the suit next.  Over `(1♥) X - 1♠ -` an 18-HCP club
/// one-suiter rebids `2♣`; a bare minimum passes.
#[test]
fn strong_one_suiter_names_its_suit() {
    let mut on = Agreements::default();
    on.defense.defensive_seam_split = true;
    let auction = [
        call(1, Strain::Hearts),
        Call::Double,
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
    ];

    // 2♠-2♥-3♦-6♣, 18 HCP — nothing else describes it.
    let (strong, _) = best_call_with(&on, &auction, "AQ.K4.AQ7.AJT962");
    assert_eq!(strong, call(2, Strain::Clubs), "18 shows the club suit");

    // 3♠-1♥-4♦-5♣, 12 HCP — a bare takeout minimum.
    let (minimum, _) = best_call_with(&on, &auction, "Q52.4.KJ87.KJ862");
    assert_eq!(minimum, Call::Pass, "a minimum double has nothing to add");
}

/// The knob gates the whole package: with it off the node is unauthored and the
/// floor answers, so the two arms must differ somewhere.
#[test]
fn the_package_is_gated() {
    let off = Agreements::default();
    assert!(
        !(doubler_rebid_package().gate)(&off),
        "the package is off by default"
    );
    let mut on = Agreements::default();
    on.defense.defensive_seam_split = true;
    assert!((doubler_rebid_package().gate)(&on), "the knob turns it on");
    assert!(
        !(doubler_rebid_package().entries)(&on).is_empty(),
        "the package authors rows when on"
    );
}

/// Advancer forced to the **two** level costs the doubler a level of room, so
/// every floor rises by two: the 15-count that would rebid over a one-level
/// advance passes over a two-level one.
#[test]
fn the_two_level_advance_raises_every_floor() {
    let mut on = Agreements::default();
    on.defense.defensive_seam_split = true;
    // 2♠-6♥-3♦-2♣, 15 HCP.
    let hand = "K5.AQJ862.K87.Q2";

    let over_1c = [
        call(1, Strain::Clubs),
        Call::Double,
        Call::Pass,
        call(1, Strain::Spades),
        Call::Pass,
    ];
    let (cheap, _) = best_call_with(&on, &over_1c, hand);
    assert_eq!(
        cheap,
        call(2, Strain::Hearts),
        "15 bids over a 1-level advance"
    );

    let over_1s = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let (dear, _) = best_call_with(&on, &over_1s, hand);
    assert_eq!(
        dear,
        Call::Pass,
        "the same 15 passes over a 2-level advance"
    );
}
