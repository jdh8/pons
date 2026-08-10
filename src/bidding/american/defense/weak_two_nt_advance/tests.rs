use super::super::tests::{best_call_with, call};
use crate::bidding::agreements::Agreements;
use crate::bidding::american::american;
use contract_bridge::Strain;
use contract_bridge::auction::{Call, RelativeVulnerability};

#[test]
fn advance_game_threshold_tracks_the_notrump_band() {
    // There is no invitational tier because eight opposite 16 is game
    // values — so the threshold has to *move* when the band's floor does,
    // or a widened band drives advancer to game on 23 total points.  Same
    // hand, same auction, one point of band: 8 HCP and five diamonds.
    let auction = [
        call(2, Strain::Hearts),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let hand = "K84.732.KQT43.42";

    let mut at_16_band = Agreements::default();
    at_16_band.defense.weak_two_notrump_advances_enabled = true;
    let mut at_15_band = at_16_band;
    at_15_band.defense.weak_two_notrump_points = (15, 17);
    let (at_16, _) = best_call_with(&at_16_band, &auction, hand);
    let (at_15, _) = best_call_with(&at_15_band, &auction, hand);

    assert_eq!(
        at_16,
        call(3, Strain::Diamonds),
        "opposite 16-17, eight is game values: force with the five-card suit"
    );
    assert_eq!(
        at_15,
        call(3, Strain::Clubs),
        "opposite 15-17 the same eight is not, so it relays for a partscore"
    );
}

#[test]
fn weak_two_notrump_advances_route_each_hand_class() {
    // Over their (2♥) our 2NT is 16–17 with a stopper, so eight opposite is
    // game values and there is no invitational tier: 3♣ or game.
    let mut arm = Agreements::default();
    arm.defense.weak_two_notrump_advances_enabled = true;
    let auction = [
        call(2, Strain::Hearts),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (stayman, stayman_floored) = best_call_with(&arm, &auction, "KQ85.732.K843.42");
    let (natural, _) = best_call_with(&arm, &auction, "A84.732.KQT43.42");
    let (relay, relay_floored) = best_call_with(&arm, &auction, "843.732.QT843.42");

    assert_eq!(
        stayman,
        call(3, Strain::Hearts),
        "exactly 4 spades with game values cues for Stayman"
    );
    assert!(!stayman_floored, "the cue is a book node, not the floor");
    assert_eq!(
        natural,
        call(3, Strain::Diamonds),
        "5 diamonds with game values bids them naturally"
    );
    assert_eq!(
        relay,
        call(3, Strain::Clubs),
        "a weak 5-card diamond hand relays instead of passing 2NT"
    );
    assert!(!relay_floored, "the relay is a book node, not the floor");
}

#[test]
fn weak_two_notrump_relay_lands_in_diamonds() {
    // 3♣ is forced to 3♦, which advancer passes — or cues with six-plus
    // diamonds to say 4♦ is safe.  Both halves must come off the book.
    let mut arm = Agreements::default();
    arm.defense.weak_two_notrump_advances_enabled = true;
    let opening = || call(2, Strain::Hearts);
    let nt = || call(2, Strain::Notrump);
    let c = || call(3, Strain::Clubs);
    let d = || call(3, Strain::Diamonds);
    let (forced, forced_floored) = best_call_with(
        &arm,
        &[opening(), nt(), Call::Pass, c(), Call::Pass],
        "AQ2.KQ8.A54.K932", // the 16–17 overcall itself: reply is blind
    );
    let (sign_off, _) = best_call_with(
        &arm,
        &[
            opening(),
            nt(),
            Call::Pass,
            c(),
            Call::Pass,
            d(),
            Call::Pass,
        ],
        "843.732.QT843.42", // exactly five — play 3♦
    );
    let (cue, cue_floored) = best_call_with(
        &arm,
        &[
            opening(),
            nt(),
            Call::Pass,
            c(),
            Call::Pass,
            d(),
            Call::Pass,
        ],
        "84.732.QT8432.42", // six — 4♦ is safe
    );

    assert_eq!(forced, d(), "the relay reply is a forced 3♦");
    assert!(!forced_floored, "the forced reply is a book node");
    assert_eq!(sign_off, Call::Pass, "five diamonds plays 3♦");
    assert_eq!(
        cue,
        call(3, Strain::Hearts),
        "six diamonds cues to show 4♦ is safe"
    );
    assert!(!cue_floored, "the delayed cue is a book node");
}

#[test]
fn weak_two_notrump_relay_reads_as_diamonds_not_clubs() {
    // The phantom-suit guard.  `3♣` shows *diamonds*; if the natural walk
    // floors clubs off it, the floor raises a suit advancer does not hold
    // the moment the opponents come back in.  The alert is what suppresses
    // the walk, so this test is what proves the alert is wired.
    use crate::bidding::Relative;
    use contract_bridge::Suit;

    let mut arm = Agreements::default();
    arm.defense.weak_two_notrump_advances_enabled = true;
    let read = american(&arm).against().infer(
        RelativeVulnerability::NONE,
        &[
            call(2, Strain::Hearts),
            call(2, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
        ],
    );
    let shown = read.announced(Relative::Rho);

    assert!(
        shown.length(Suit::Diamonds).min >= 5,
        "the relay reads as five-plus diamonds, got {:?}",
        shown.length(Suit::Diamonds)
    );
    assert_eq!(
        shown.length(Suit::Clubs).min,
        0,
        "the relay must promise no clubs at all"
    );
}
