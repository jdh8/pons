use super::super::tests::{best_call, best_call_with, call};
use crate::bidding::agreements::Agreements;
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
    let mut arm = Agreements::default();
    arm.competition.uvu_over_majors = true;
    // `1♥ (2NT)`: their 2NT shows both minors; a 12-count with 3 hearts bids
    // 3♣ as a limit-plus raise.
    let auction = [call(1, Strain::Hearts), call(2, Strain::Notrump)];
    let (raise, floored) = best_call_with(&arm, &auction, "K52.QJ5.A964.Q32");
    assert_eq!(raise, call(3, Strain::Clubs), "the cheap cue raises");
    assert!(!floored, "an authored node, not the floor");
    // 14-count, 5 spades, 2 hearts → 3♦ = game force in the other major.
    let (fourth, _) = best_call_with(&arm, &auction, "AQJ54.K5.965.A43");
    assert_eq!(fourth, call(3, Strain::Diamonds), "the second cue forces");
}

#[test]
fn michaels_cue_of_our_major_gets_a_structure() {
    let mut arm = Agreements::default();
    arm.competition.uvu_over_majors = true;
    // `1♠ (2♠)`: their 2♠ is Michaels; a limit raise cues their known major
    // with 3♥...
    let auction = [call(1, Strain::Spades), call(2, Strain::Spades)];
    let (cue, floored) = best_call_with(&arm, &auction, "KQ5.A54.96432.Q2");
    assert_eq!(cue, call(3, Strain::Hearts), "the known-suit cue raises");
    assert!(!floored, "an authored node, not the floor");
    // ...while a competitive 7-count raises 3♠ naturally — the raise
    // keeps its meaning over their cue of our own suit.
    let (raise, _) = best_call_with(&arm, &auction, "Q542.95.9643.KQ3");
    assert_eq!(raise, call(3, Strain::Spades), "the natural raise survives");
}

#[test]
fn uvu_minor_cues_split_raise_and_fourth_suit() {
    let mut arm = Agreements::default();
    arm.competition.uvu_over_minors = true;
    // `1♣ (2♣)`: their 2♣ is Michaels, both majors.  A limit raise with 5+
    // clubs cues their lower major.
    let auction = [call(1, Strain::Clubs), call(2, Strain::Clubs)];
    let (raise, floored) = best_call_with(&arm, &auction, "K5.Q52.A96.QJ432");
    assert_eq!(raise, call(2, Strain::Hearts), "the cheap cue raises");
    assert!(!floored, "an authored node, not the floor");
    // 14-count with 5 diamonds → 2♠ = game force in the unbid minor.
    let (fourth, _) = best_call_with(&arm, &auction, "A4.K52.AQJ54.965");
    assert_eq!(fourth, call(2, Strain::Spades), "the second cue forces");
    // The generic negative double — 4-4 majors against a cue that *shows*
    // both majors — is retired at this node: values with a 4-card major
    // double to punish, not to ask.
    let (x, _) = best_call_with(&arm, &auction, "KQ42.KJ95.Q54.32");
    assert_eq!(x, Call::Double, "values with a punishable major double");
}

#[test]
fn opener_answers_the_uvu_minor_cues() {
    let mut arm = Agreements::default();
    arm.competition.uvu_over_minors = true;
    // `1♣ (2♣) 2♠ -`: partner's game force with 5+ diamonds; both majors
    // stopped bids the 3NT the force is looking for.
    let auction = [
        call(1, Strain::Clubs),
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
    ];
    let (c, floored) = best_call_with(&arm, &auction, "A54.KQ4.954.AQJ32");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "an authored node, not the floor");
    // No spade stopper: raise partner's diamonds with 4+ instead.
    let (c, _) = best_call_with(&arm, &auction, "543.AK4.Q954.AQ32");
    assert_eq!(c, call(3, Strain::Diamonds));
}

#[test]
fn opener_answers_the_uvu_major_cue() {
    let mut arm = Agreements::default();
    arm.competition.uvu_over_majors = true;
    // `1♥ (2NT) 3♣ -` (limit+ raise): a minimum declines in 3♥, a
    // maximum accepts to game — the shipped cue-raise answer, rewired.
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (decline, floored) = best_call_with(&arm, &auction, "965.AQJ54.K54.32");
    assert_eq!(decline, call(3, Strain::Hearts), "a minimum signs off");
    assert!(!floored, "an authored node, not the floor");
    let (accept, _) = best_call_with(&arm, &auction, "65.AKQ54.KJ54.A2");
    assert_eq!(accept, call(4, Strain::Hearts), "a maximum accepts");
}

#[test]
fn opener_answers_the_uvu_fourth_suit_force() {
    let mut arm = Agreements::default();
    arm.competition.uvu_over_majors = true;
    // `1♥ (2NT) 3♦ -` (GF, 5+ spades): three-card support raises the
    // shown major to game.
    let auction = [
        call(1, Strain::Hearts),
        call(2, Strain::Notrump),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (game, floored) = best_call_with(&arm, &auction, "K65.AQJ54.K54.32");
    assert_eq!(game, call(4, Strain::Spades), "raise the game force");
    assert!(!floored, "an authored node, not the floor");
}
