use super::super::tests::{best_call, best_call_with, bid_diamond, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

// --- Competition over our 2NT diamond transfer (Side A) ---

#[test]
fn diamond_doubled_opener_completes_with_a_fit() {
    // 1NT - 2NT (X): three diamonds → 3♦ (accept the transfer).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let (c, floored) = bid_diamond(&auction, "Axx.Kxx.Qxx.AKxx");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the contested completion must come from the book");
}

#[test]
fn diamond_doubled_opener_bids_natural_clubs() {
    // 1NT - 2NT (X): doubleton ♦ but 4 clubs → 3♣ (natural, Pass is the
    // catch-all, so 3♣ promises real clubs).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let (c, floored) = bid_diamond(&auction, "AQx.Kxx.xx.AQxx");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the natural 3♣ must come from the book");
}

#[test]
fn diamond_doubled_opener_redoubles_max_no_fit() {
    // 1NT - 2NT (X): maximum (18), no ♦ fit, no 4-card club → XX (values).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let (c, floored) = bid_diamond(&auction, "AKxx.AQxx.Jx.Axx");
    assert_eq!(c, Call::Redouble);
    assert!(!floored, "the values redouble must come from the book");
}

#[test]
fn diamond_no_fit_responder_signs_off_in_diamonds() {
    // 1NT - 2NT (X) - -: opener denied a fit; responder pulls to 3♦.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = bid_diamond(&auction, "xx.xx.KJxxxx.xxx");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the signoff must come from the book");
}

#[test]
fn diamond_overcalled_low_still_completes() {
    // 1NT - 2NT (3♣): 3♦ still legal, three diamonds → complete to 3♦.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
    ];
    let (c, floored) = bid_diamond(&auction, "Axx.Kxx.Qxx.AKxx");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the completion over 3♣ must come from the book");
}

#[test]
fn diamond_overcalled_high_three_notrump_with_stopper() {
    // 1NT - 2NT (3♥): no 3♦ left; maximum (18) + heart stopper → 3NT.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        call(3, Strain::Hearts),
    ];
    let (c, floored) = bid_diamond(&auction, "AQx.KJx.Qx.AKxxx");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the 3NT must come from the book");
}

#[test]
fn diamond_competition_disabled_falls_to_floor() {
    // Off-switch: with the toggle off, 1NT - 2NT (X) has no Side-A node.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Double,
    ];
    let mut off = Agreements::current();
    off.competition.competition_over_diamond_transfer = false;
    let (_, floored) = best_call_with(&off, &auction, "Axx.Kxx.Qxx.AKxx");
    assert!(floored, "with the toggle off opener falls to the floor");
}

// --- Defense to their 2NT diamond transfer (Side B) ---

#[test]
fn defense_to_their_diamond_transfer_doubles_diamonds() {
    // After `(1NT) - (2NT)`, their 2NT transfers to diamonds; our fourth-hand X
    // is lead-directing in diamonds, the shown suit.
    crate::bidding::american::set_diamond_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
    ];
    let (c, floored) = best_call(&auction, "A32.32.KQJ54.432");
    crate::bidding::american::set_diamond_transfer_defense(false); // restore default
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}

#[test]
fn defense_to_their_diamond_transfer_cues_both_majors() {
    // After `(1NT) - (2NT)`, their 2NT transfers to diamonds; 5 spades and 5
    // hearts cue 3♦ to show both majors, beating the X.
    crate::bidding::american::set_diamond_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
    ];
    let (c, floored) = best_call(&auction, "KQ1054.KJ1054.3.32");
    crate::bidding::american::set_diamond_transfer_defense(false); // restore default
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the both-majors cue must come from the book");
}
