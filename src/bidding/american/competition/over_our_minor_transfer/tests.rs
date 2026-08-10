use super::super::tests::{best_call_with, bid_minor, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

// --- Competition over our 2♠ minor transfer (Side A) ---

#[test]
fn minor_doubled_opener_shows_min_with_stopper() {
    // 1NT - 2♠ (X): minimum + spade stopper → 2NT.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Double,
    ];
    let (c, floored) = bid_minor(&auction, "KJ2.A32.K432.Q32");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the coded answer must come from the book");
}

#[test]
fn minor_doubled_opener_jumps_max_with_stopper() {
    // 1NT - 2♠ (X): maximum (17) + spade stopper → 3♣.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Double,
    ];
    let (c, floored) = bid_minor(&auction, "KQ2.AQ2.KJ32.A32");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the coded max answer must come from the book");
}

#[test]
fn minor_doubled_opener_passes_min_no_stopper() {
    // 1NT - 2♠ (X): minimum, NO spade stopper → Pass.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Double,
    ];
    let (c, floored) = bid_minor(&auction, "432.AQ2.KQ32.K32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the no-stopper pass must come from the book");
}

#[test]
fn minor_doubled_opener_redoubles_max_no_stopper() {
    // 1NT - 2♠ (X): maximum (17), NO spade stopper → XX.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Double,
    ];
    let (c, _) = bid_minor(&auction, "432.AKQ.AQJ2.K32");
    assert_eq!(c, Call::Redouble);
}

#[test]
fn minor_no_stopper_responder_signs_off_in_clubs() {
    // 1NT - 2♠ (X) - -: opener denied a stopper; 6 clubs → 3♣ sign-off.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = bid_minor(&auction, "32.43.32.KJ98765");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the club sign-off must come from the book");
}

#[test]
fn minor_overcalled_high_bids_game_with_stopper() {
    // 1NT - 2♠ (2NT): maximum + spade stopper → 3NT (to play).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
    ];
    let (c, floored) = bid_minor(&auction, "KQ2.AQ2.KJ32.A32");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the coded game must come from the book");
}

#[test]
fn minor_overcalled_low_is_systems_off() {
    // 1NT - 2♠ (3♦): systems-off, length in their suit → X (cards).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        call(3, Strain::Diamonds),
    ];
    let (c, _) = bid_minor(&auction, "K32.K32.AQ32.A32");
    assert_eq!(c, Call::Double);
}

// --- Defense to their 2♠ minor transfer (Side B) ---

#[test]
fn defense_to_their_minor_transfer_doubles_spades() {
    // After `(1NT) - (2♠)`, their 2♠ is a minor transfer; our fourth-hand X is
    // lead-directing in spades, the bid suit.
    let mut arm = Agreements::default();
    arm.defense.minor_transfer_defense_enabled = true;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call_with(&arm, &auction, "KQJ54.A32.432.32");
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}

#[test]
fn defense_to_their_minor_transfer_cues_top_and_bottom() {
    // (1NT) - (2♠): 5 spades + 5 diamonds → 3♣ cue (top-and-bottom), beating the X.
    let mut arm = Agreements::default();
    arm.defense.minor_transfer_defense_enabled = true;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call_with(&arm, &auction, "KQ1054.3.KJ1054.32");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the top-and-bottom cue must come from the book");
}

#[test]
fn defense_to_their_minor_transfer_two_notrump_is_reds() {
    // (1NT) - (2♠): 5 diamonds + 5 hearts → 2NT (the two lowest unbid suits).
    let mut arm = Agreements::default();
    arm.defense.minor_transfer_defense_enabled = true;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call_with(&arm, &auction, "3.KQ1054.KJ1054.32");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the red two-suiter must come from the book");
}
