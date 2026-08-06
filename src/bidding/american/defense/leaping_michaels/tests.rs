use super::super::tests::{best_call, call};
use crate::bidding::american::{LebensohlStyle, set_advance_sohl_style, set_leaping_michaels};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

/// Best call with Leaping Michaels forced to `on` (and the sohl toggles reset,
/// independent of any other test on this thread)
fn leaping(on: bool, auction: &[Call], hand: &str) -> (Call, bool) {
    set_advance_sohl_style(LebensohlStyle::Off);
    set_leaping_michaels(on);
    best_call(auction, hand)
}

#[test]
fn leaping_michaels_minor_plus_other_major_over_a_major() {
    // Over (2♥): 5-5 clubs+spades, game values → 4♣; 5-5 diamonds+spades → 4♦.
    let over_2h = [call(2, Strain::Hearts)];
    let (c, floored) = leaping(true, &over_2h, "AKQ65.4.32.KQJ76");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "Leaping Michaels must come from the book node");

    let (d, _) = leaping(true, &over_2h, "AKQ65.4.KQJ76.32");
    assert_eq!(d, call(4, Strain::Diamonds));
}

#[test]
fn leaping_michaels_cue_shows_both_majors_over_2d() {
    // Over (2♦): 5-5 in the majors → 4♦ (the cue), both majors.
    let over_2d = [call(2, Strain::Diamonds)];
    let (c, floored) = leaping(true, &over_2d, "AKQ65.KQJ76.4.32");
    assert_eq!(c, call(4, Strain::Diamonds));
    assert!(!floored, "Leaping Michaels must come from the book node");
}

#[test]
fn leaping_michaels_advancer_picks_the_major_game() {
    // (2♥) 4♣ -: partner shows clubs + spades. With spade support the
    // advancer bids the 4♠ game; with none, the 5♣ minor game (never pass 4♣).
    let auction = [call(2, Strain::Hearts), call(4, Strain::Clubs), Call::Pass];
    let (fit, floored) = leaping(true, &auction, "KQ7.32.J865.A432");
    assert_eq!(fit, call(4, Strain::Spades));
    assert!(!floored, "the advance must come from the book node");

    // A doubleton (7-card fit) still takes the 4♠ game — it scores well and
    // needs only ten tricks.
    let (thin, _) = leaping(true, &auction, "K7.QJ32.8654.A32");
    assert_eq!(thin, call(4, Strain::Spades));

    // A genuine major misfit (≤1) retreats to the 5♣ game, not a passed 4♣.
    let (no_fit, _) = leaping(true, &auction, "2.QJ32.J8654.KQ4");
    assert_eq!(no_fit, call(5, Strain::Clubs));
}

#[test]
fn leaping_michaels_advancer_picks_longer_major_over_2d_cue() {
    // (2♦) 4♦ -: the cue shows both majors; advancer picks the longer.
    let auction = [
        call(2, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = leaping(true, &auction, "AQ32.K8.654.9432");
    assert_eq!(c, call(4, Strain::Spades));
    assert!(!floored, "the advance must come from the book node");
}

#[test]
fn leaping_michaels_2d_4c_pass_or_correct() {
    // (2♦) 4♣ -: clubs + an unknown major → 4♥ pass-or-correct, then the
    // overcaller with spades corrects to 4♠.
    let advance = [
        call(2, Strain::Diamonds),
        call(4, Strain::Clubs),
        Call::Pass,
    ];
    let (relay, _) = leaping(true, &advance, "K32.A87.9654.J32");
    assert_eq!(relay, call(4, Strain::Hearts));

    let rebid = [
        call(2, Strain::Diamonds),
        call(4, Strain::Clubs),
        Call::Pass,
        call(4, Strain::Hearts),
        Call::Pass,
    ];
    let (correct, _) = leaping(true, &rebid, "AKQ65.4.32.KQJ76");
    assert_eq!(correct, call(4, Strain::Spades));
}

#[test]
fn leaping_michaels_silent_when_disabled() {
    // Turned off: the same club-spade two-suiter never jumps to 4♣ (the
    // escape hatch back to the pre-Leaping-Michaels weak-two defense).
    let over_2h = [call(2, Strain::Hearts)];
    let (c, _) = leaping(false, &over_2h, "AKQ65.4.32.KQJ76");
    assert_ne!(c, call(4, Strain::Clubs));
}
