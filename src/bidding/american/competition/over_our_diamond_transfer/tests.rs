use super::super::tests::{best_call_with, bid_diamond, call};
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
    let mut off = Agreements::default();
    off.competition.competition_over_diamond_transfer = false;
    let (_, floored) = best_call_with(&off, &auction, "Axx.Kxx.Qxx.AKxx");
    assert!(floored, "with the toggle off opener falls to the floor");
}

// --- Defense to their 2NT diamond transfer (Side B) ---

#[test]
fn defense_to_their_diamond_transfer_doubles_diamonds() {
    // After `(1NT) - (2NT)`, their 2NT transfers to diamonds; our fourth-hand X
    // is lead-directing in diamonds, the shown suit.
    let mut arm = Agreements::default();
    arm.defense.diamond_transfer_defense_enabled = true;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
    ];
    let (c, floored) = best_call_with(&arm, &auction, "A32.32.KQJ54.432");
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
    let mut arm = Agreements::default();
    arm.defense.diamond_transfer_defense_enabled = true;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
    ];
    let (c, floored) = best_call_with(&arm, &auction, "KQ1054.KJ1054.3.32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the both-majors cue must come from the book");
}

// --- Responder captain over a high overcall (`diamond_transfer_responder_captain`) ---

/// `1NT - 2NT (3♥)`, with the given captain-knob state
fn overcalled_hearts(captain: bool) -> (Agreements, [Call; 4]) {
    let mut arm = Agreements::default();
    arm.competition.competition_over_diamond_transfer = true;
    arm.competition.diamond_transfer_responder_captain = captain;
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        call(3, Strain::Hearts),
    ];
    (arm, auction)
}

#[test]
fn captain_opener_passes_a_maximum_with_a_stopper() {
    // On: the same 17-count passes — responder, not opener, knows whether the
    // transfer holds 3 or 13.
    let (arm, auction) = overcalled_hearts(true);
    let (c, floored) = best_call_with(&arm, &auction, "AQx.KJx.Axx.KJxx");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the captain pass comes from the book");
}

#[test]
fn captain_opener_doubles_with_four_of_their_suit() {
    let (arm, auction) = overcalled_hearts(true);
    let (c, floored) = best_call_with(&arm, &auction, "AQx.KJ9x.Ax.KJxx");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the penalty X comes from the book");
}

/// `1NT - 2NT (3♥) X -`: responder's seat after opener's penalty double
fn doubled_hearts(captain: bool) -> (Agreements, Vec<Call>) {
    let (arm, auction) = overcalled_hearts(captain);
    let mut auction = auction.to_vec();
    auction.extend([Call::Double, Call::Pass]);
    (arm, auction)
}

#[test]
fn captain_responder_leaves_the_penalty_double_in() {
    // Five diamonds and two hearts: opener holds their suit, defend.
    let (arm, auction) = doubled_hearts(true);
    let (c, floored) = best_call_with(&arm, &auction, "xxx.xx.KJxxx.xxx");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the leave-in comes from the book");
}

#[test]
fn captain_responder_pulls_to_4d_with_six_and_a_stiff() {
    // Six diamonds, a singleton heart: cannot defend, play 4♦.
    let (arm, auction) = doubled_hearts(true);
    let (c, floored) = best_call_with(&arm, &auction, "xxx.x.KJxxxx.xxx");
    assert_eq!(c, call(4, Strain::Diamonds));
    assert!(!floored, "the pull comes from the book");
}

#[test]
fn captain_responder_defends_their_doubled_diamond_cue() {
    // `1NT - 2NT (3♦) X -`: no pull shape exists over their diamonds, so even
    // a long-diamond hand leaves the double in — from the book.
    let (arm, mut auction) = doubled_hearts(true);
    auction[3] = call(3, Strain::Diamonds);
    let (c, floored) = best_call_with(&arm, &auction, "xxx.xx.KJxxxx.xx");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the leave-in comes from the book");
}

#[test]
fn shipped_responder_answer_to_the_penalty_double_is_floor_owned() {
    // Off: the seat is unauthored — the hole the knob fills.
    let (arm, auction) = doubled_hearts(false);
    let (_, floored) = best_call_with(&arm, &auction, "xxx.x.KJxxxx.xxx");
    assert!(floored, "shipped, responder's answer falls to the floor");
}
