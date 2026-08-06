use super::super::tests::{best_call, bid_diamond, bid_minor, bid_xfer, call};
use contract_bridge::Strain;
use contract_bridge::auction::Call;

// --- Competition over our 2♠ minor transfer (Side A) ---

#[test]
fn minor_doubled_opener_shows_min_with_stopper() {
    // 1NT-(P)-2♠-(X): minimum + spade stopper → 2NT.
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
    // 1NT-(P)-2♠-(X): maximum (17) + spade stopper → 3♣.
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
    // 1NT-(P)-2♠-(X): minimum, NO spade stopper → Pass.
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
    // 1NT-(P)-2♠-(X): maximum (17), NO spade stopper → XX.
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
    // 1NT-(P)-2♠-(X)-P-(P): opener denied a stopper; 6 clubs → 3♣ sign-off.
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
    // 1NT-(P)-2♠-(2NT): maximum + spade stopper → 3NT (to play).
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
    // 1NT-(P)-2♠-(3♦): systems-off, length in their suit → X (cards).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
        call(3, Strain::Diamonds),
    ];
    let (c, _) = bid_minor(&auction, "K32.K32.AQ32.A32");
    assert_eq!(c, Call::Double);
}

// --- Competition over our 2NT diamond transfer (Side A) ---

#[test]
fn diamond_doubled_opener_completes_with_a_fit() {
    // 1NT-(P)-2NT-(X): three diamonds → 3♦ (accept the transfer).
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
    // 1NT-(P)-2NT-(X): doubleton ♦ but 4 clubs → 3♣ (natural, Pass is the
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
    // 1NT-(P)-2NT-(X): maximum (18), no ♦ fit, no 4-card club → XX (values).
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
    // 1NT-(P)-2NT-(X)-P-(P): opener denied a fit; responder pulls to 3♦.
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
    // 1NT-(P)-2NT-(3♣): 3♦ still legal, three diamonds → complete to 3♦.
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
    // 1NT-(P)-2NT-(3♥): no 3♦ left; maximum (18) + heart stopper → 3NT.
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
    // Off-switch: with the toggle off, 1NT-(P)-2NT-(X) has no Side-A node.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Notrump),
        Call::Double,
    ];
    super::over_our_notrump_calls::set_competition_over_diamond_transfer(false);
    let (_, floored) = best_call(&auction, "Axx.Kxx.Qxx.AKxx");
    super::over_our_notrump_calls::set_competition_over_diamond_transfer(true); // restore the on default
    assert!(floored, "with the toggle off opener falls to the floor");
}

// --- Competition over our 2♣ Stayman (Side A) + defense to theirs (Side B) ---

#[test]
fn stayman_doubled_opener_bids_major_with_stopper() {
    // 1NT-(P)-2♣-(X): 4 hearts + a club stopper → 2♥ (the major + stopper).
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
    // 1NT-(P)-2♣-(X): 4 hearts but NO club stopper → Pass (denies the stopper;
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
    // 1NT-(P)-2♣-(X): five good clubs → XX (business, play 2♣XX).
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
    // 1NT-(P)-2♣-(X)-P-(P): responder re-asks with XX (4 spades).
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
    // …-XX-(P): opener is forced to answer (no Pass), 4 spades → 2♠.
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
    // 1NT-(P)-2♣-(2♦): 4 hearts → 2♥ (natural, outranks diamonds).
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
    // 1NT-(P)-2♣-(2♦): no biddable major, length in diamonds → X (cards).
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
    // (1NT)-P-(2♣ Stayman): our 4th-hand X = lead-directing clubs (5+ good).
    crate::bidding::american::set_stayman_defense(true);
    let auction = [call(1, Strain::Notrump), Call::Pass, call(2, Strain::Clubs)];
    let (c, floored) = best_call(&auction, "A2.K32.A32.KQ876");
    crate::bidding::american::set_stayman_defense(false); // restore default
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}

// --- Competition over our Jacoby transfers (Side A) + defense to theirs (B) ---

#[test]
fn transfer_super_accept_uncontested() {
    // 1NT-P-2♦-P: four hearts + a maximum → 3♥ (jump super-accept).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_xfer(&auction, "A2.KQ32.KQ32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the super-accept must come from the book");
}

#[test]
fn transfer_doubled_opener_completes_with_support() {
    // 1NT-(P)-2♦-(X): three hearts, not a maximum → 2♥ (complete the transfer).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, floored) = bid_xfer(&auction, "KQ2.K32.KQ32.Q32");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the completion must come from the book");
}

#[test]
fn transfer_doubled_opener_super_accepts() {
    // 1NT-(P)-2♦-(X): four hearts + a maximum → 3♥ (the double does not suppress it).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, _) = bid_xfer(&auction, "A2.KQ32.KQ32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
}

#[test]
fn transfer_doubled_opener_passes_with_doubleton() {
    // 1NT-(P)-2♦-(X): only a doubleton heart → Pass (declines; responder re-asks).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, floored) = bid_xfer(&auction, "KQ32.K2.KQ32.Q32");
    assert_eq!(c, Call::Pass);
    assert!(!floored, "the declining pass must come from the book");
}

#[test]
fn transfer_doubled_opener_redoubles_with_the_transfer_suit() {
    // 1NT-(P)-2♦-(X): the doubled diamonds are opener's own (5 to AKQ) → XX.
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    let (c, _) = bid_xfer(&auction, "Q43.K2.AKQ32.Q32");
    assert_eq!(c, Call::Redouble);
}

#[test]
fn transfer_doubled_reask_is_forcing() {
    // 1NT-(P)-2♦-(X)-P-(P): responder re-asks with XX (still holds five hearts).
    let reask = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
        Call::Pass,
    ];
    let (c, floored) = bid_xfer(&reask, "K2.QJ432.K32.432");
    assert_eq!(c, Call::Redouble);
    assert!(!floored, "the re-ask must come from the book");
    // …-XX-(P): opener is forced to complete (no Pass) → 2♥.
    let answer = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
        Call::Pass,
        Call::Redouble,
        Call::Pass,
    ];
    let (c, floored) = bid_xfer(&answer, "AQ32.K32.KQ2.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the forced completion must come from the book");
}

#[test]
fn transfer_overcalled_opener_super_accepts() {
    // 1NT-(P)-2♦-(2♠): four-card heart fit → 3♥ (cheapest level above their 2♠).
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
        call(2, Strain::Spades),
    ];
    let (c, floored) = bid_xfer(&auction, "K2.KQ32.AQ32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the natural super-accept must come from the book");
}

#[test]
fn defense_to_their_transfer_doubles_the_bid_suit() {
    // (1NT)-P-(2♦ →♥): our 4th-hand X = lead-directing diamonds (the bid suit).
    crate::bidding::american::set_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
    ];
    let (c, floored) = best_call(&auction, "K2.A32.KQ1054.432");
    crate::bidding::american::set_transfer_defense(false); // restore default
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}

#[test]
fn defense_to_their_transfer_cues_michaels() {
    // (1NT)-P-(2♦ →♥): 5 spades + 5 diamonds → 2♥ cue (the other major + a minor).
    crate::bidding::american::set_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Diamonds),
    ];
    let (c, floored) = best_call(&auction, "AQ1054.3.KJ1054.32");
    crate::bidding::american::set_transfer_defense(false); // restore default
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the Michaels cue must come from the defense book");
}

// --- Defense to their 2♠ minor transfer (Side B) ---

#[test]
fn defense_to_their_minor_transfer_doubles_spades() {
    // (1NT)-P-(2♠ minor): our 4th-hand X = lead-directing spades (the bid suit).
    crate::bidding::american::set_minor_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call(&auction, "KQJ54.A32.432.32");
    crate::bidding::american::set_minor_transfer_defense(false); // restore default
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the lead-directing X must come from the defense book"
    );
}

#[test]
fn defense_to_their_minor_transfer_cues_top_and_bottom() {
    // (1NT)-P-(2♠): 5 spades + 5 diamonds → 3♣ cue (top-and-bottom), beating the X.
    crate::bidding::american::set_minor_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call(&auction, "KQ1054.3.KJ1054.32");
    crate::bidding::american::set_minor_transfer_defense(false); // restore default
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the top-and-bottom cue must come from the book");
}

// --- Defense to their 2NT diamond transfer (Side B) ---

#[test]
fn defense_to_their_diamond_transfer_doubles_diamonds() {
    // (1NT)-P-(2NT →♦): our 4th-hand X = lead-directing diamonds (the shown suit).
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
    // (1NT)-P-(2NT →♦): 5 spades + 5 hearts → 3♦ cue (both majors), beating the X.
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

#[test]
fn defense_to_their_minor_transfer_two_notrump_is_reds() {
    // (1NT)-P-(2♠): 5 diamonds + 5 hearts → 2NT (the two lowest unbid suits).
    crate::bidding::american::set_minor_transfer_defense(true);
    let auction = [
        call(1, Strain::Notrump),
        Call::Pass,
        call(2, Strain::Spades),
    ];
    let (c, floored) = best_call(&auction, "3.KQ1054.KJ1054.32");
    crate::bidding::american::set_minor_transfer_defense(false); // restore default
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the red two-suiter must come from the book");
}
