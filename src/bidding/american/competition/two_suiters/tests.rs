use super::super::tests::{best_call, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn michaels_cue_of_our_major_is_not_a_cue_raise() {
    // `1♠ (2♠) 3♠ -`: their 2♠ is Michaels, a cue of our spades, while
    // responder's 3♠ is a natural raise.  The cue-raise answer table must not
    // hijack this. A strong opener
    // (this hand tripped the old over-broad guard into a passed-out 4NT) must
    // NOT bid 4NT here.
    let auction = [
        call(1, Strain::Spades),
        call(2, Strain::Spades),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let (c, _) = best_call(&auction, "AKQT98.Q.AQT73.Q");
    assert_ne!(
        c,
        call(4, Strain::Notrump),
        "a natural spade raise must not be answered as a cue-raise"
    );
}

#[test]
fn uvu_major_cues_split_raise_and_fourth_suit() {
    super::two_suiters::set_uvu_over_majors(true);
    // `1♥ (2NT)`: their 2NT shows both minors; a 12-count with 3 hearts bids
    // 3♣ as a limit-plus raise.
    let auction = [call(1, Strain::Hearts), call(2, Strain::Notrump)];
    let (raise, floored) = best_call(&auction, "K52.QJ5.A964.Q32");
    assert_eq!(raise, call(3, Strain::Clubs), "the cheap cue raises");
    assert!(!floored, "an authored node, not the floor");
    // 14-count, 5 spades, 2 hearts → 3♦ = game force in the other major.
    let (fourth, _) = best_call(&auction, "AQJ54.K5.965.A43");
    assert_eq!(fourth, call(3, Strain::Diamonds), "the second cue forces");
    super::two_suiters::set_uvu_over_majors(true);
}

#[test]
fn michaels_cue_of_our_major_gets_a_structure() {
    super::two_suiters::set_uvu_over_majors(true);
    // `1♠ (2♠)`: their 2♠ is Michaels; a limit raise cues their known major
    // with 3♥...
    let auction = [call(1, Strain::Spades), call(2, Strain::Spades)];
    let (cue, floored) = best_call(&auction, "KQ5.A54.96432.Q2");
    assert_eq!(cue, call(3, Strain::Hearts), "the known-suit cue raises");
    assert!(!floored, "an authored node, not the floor");
    // ...while a competitive 7-count raises 3♠ naturally — the raise
    // keeps its meaning over their cue of our own suit.
    let (raise, _) = best_call(&auction, "Q542.95.9643.KQ3");
    assert_eq!(raise, call(3, Strain::Spades), "the natural raise survives");
    super::two_suiters::set_uvu_over_majors(true);
}

#[test]
fn opener_answers_the_uvu_major_cue() {
    super::two_suiters::set_uvu_over_majors(true);
    // `1♥ (2NT) 3♣ -` (limit+ raise): a minimum declines in 3♥, a
    // maximum accepts to game — the shipped cue-raise answer, rewired.
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (decline, floored) = best_call(&auction, "965.AQJ54.K54.32");
    assert_eq!(decline, call(3, Strain::Hearts), "a minimum signs off");
    assert!(!floored, "an authored node, not the floor");
    let (accept, _) = best_call(&auction, "65.AKQ54.KJ54.A2");
    assert_eq!(accept, call(4, Strain::Hearts), "a maximum accepts");
    super::two_suiters::set_uvu_over_majors(true);
}

#[test]
fn opener_answers_the_uvu_fourth_suit_force() {
    super::two_suiters::set_uvu_over_majors(true);
    // `1♥ (2NT) 3♦ -` (GF, 5+ spades): three-card support raises the
    // shown major to game.
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Notrump),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (game, floored) = best_call(&auction, "K65.AQJ54.K54.32");
    assert_eq!(game, call(4, Strain::Spades), "raise the game force");
    assert!(!floored, "an authored node, not the floor");
    super::two_suiters::set_uvu_over_majors(true);
}
