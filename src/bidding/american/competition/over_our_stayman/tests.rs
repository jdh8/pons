use super::super::tests::{best_call, best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

// --- Competition over our 2♣ Stayman (Side A) + defense to theirs (Side B) ---

#[test]
fn stayman_doubled_opener_bids_major_with_stopper() {
    // 1NT - 2♣ (X): 4 hearts + a club stopper → 2♥ (the major + stopper).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    let (c, floored) = best_call(&auction, "A32.KQ32.A32.K32");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the coded answer must come from the book");
}

#[test]
fn stayman_doubled_opener_passes_without_stopper() {
    // 1NT - 2♣ (X): 4 hearts but NO club stopper → Pass (denies the stopper;
    // the major waits for responder's re-ask).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    let (c, floored) = best_call(&auction, "AQ2.KQ32.AQ32.32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the stopper-denying pass must come from the book");
}

#[test]
fn stayman_doubled_opener_redoubles_with_clubs() {
    // 1NT - 2♣ (X): five good clubs → XX (business, play 2♣XX).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    let (c, _) = best_call(&auction, "A2.K32.A32.KQ876");
    assert_eq!(c, Call::Redouble);
}

#[test]
fn stayman_doubled_reask_is_forcing() {
    // 1NT - 2♣ (X) - -: responder re-asks with XX (4 spades).
    let reask = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = best_call(&reask, "KQ32.A32.A32.432");
    assert_eq!(c, Call::Redouble);
    assert!(!floored, "the re-ask must come from the book");
    // … XX -: opener is forced to answer (no Pass), 4 spades → 2♠.
    let answer = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Redouble,
        Call::Pass,
    ];
    let (c, floored) = best_call(&answer, "AQ32.K32.KQ2.432");
    assert_eq!(c, call(2, Strain::Spades));
    assert!(!floored, "the forced re-answer must come from the book");
}

#[test]
fn stayman_overcalled_opener_bids_major() {
    // 1NT - 2♣ (2♦): 4 hearts → 2♥ (natural, outranks diamonds).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        call(2, Strain::Diamonds),
    ];
    let (c, floored) = best_call(&auction, "A32.KQ32.K32.A32");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural major must come from the book");
}

#[test]
fn stayman_overcalled_opener_doubles_their_suit() {
    // 1NT - 2♣ (2♦): no biddable major, length in diamonds → X (cards).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Clubs),
        call(2, Strain::Diamonds),
    ];
    let (c, _) = best_call(&auction, "K32.K32.AQ32.A32");
    assert_eq!(c, Call::Double);
}

#[test]
fn defense_to_their_stayman_doubles_clubs() {
    // (1NT) - (2♣ Stayman): our 4th-hand X = lead-directing clubs (5+ good).
    let mut arm = Agreements::current();
    arm.defense.stayman_defense_enabled = true;
    let auction = [call(1, Strain::Notrump), Call::Pass, call(2, Strain::Clubs)];
    let (c, floored) = best_call_with(&arm, &auction, "A2.K32.A32.KQ876");
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}
