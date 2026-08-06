use crate::bidding::american::american;
use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The ported row packages hold the compile-time invariants: guarded
/// tables total (the 7NT rule — a guarded table cannot fall through to
/// the floor), and artificial rows alerted (the row extension of
/// `artificial_calls_are_alerted`).
#[test]
fn row_package_invariants() {
    crate::bidding::rows::assert_package_invariants(&[
        super::direct_seat_package(),
        super::splinter_doubled_package(),
        super::support_double_package(),
        super::transfer_free_bid_package(),
        super::answer_negative_double_package(),
        super::cue_raise_answer_package(),
        super::cue_minor_raise_answer_package(),
        super::free_bid_answer_package(),
        super::high_overcall_package(),
        super::weak_two_competition_package(),
        super::strong_two_competition_package(),
        super::jordan_truscott_package(),
        super::uvu_over_majors_package(),
        super::cachalot_package(),
        super::sputnik_residual_answer_package(),
        super::uvu_package(),
        super::lebensohl_package(),
        super::competition_over_stayman_package(),
        super::competition_over_transfer_package(),
        super::competition_over_minor_transfer_package(),
        super::competition_over_diamond_transfer_package(),
    ]);
}

/// `american()`'s best call for a hand in an auction, and whether the instinct
/// floor (not a book node) produced it
fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
    let hand: Hand = hand.parse().expect("valid test hand");
    let (logits, prov) = american()
        .against()
        .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
        .expect("a legal auction classifies");
    let best = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    (best, prov.depth == 0 && prov.fallback.is_some())
}

/// As [`best_call`], with plain Lebensohl forced on (independent of any other
/// test on this thread having changed the style)
fn bid(auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_lebensohl_style(super::LebensohlStyle::Plain);
    best_call(auction, hand)
}

/// As [`best_call`], with Transfer Lebensohl forced on
fn bid_transfer(auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_lebensohl_style(super::LebensohlStyle::Transfer);
    best_call(auction, hand)
}

/// As [`best_call`], with the Unusual-vs-Unusual `(2NT)` structure forced on
/// at the default A/B floors
fn bid_uvu(auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_uvu(true);
    super::set_uvu_x_floor(9);
    super::set_uvu_cue_floor(8);
    best_call(auction, hand)
}

/// As [`best_call`], with our Jacoby-transfer competition + jump super-accept
/// enabled (both opt-in/default-off after the DD-negative A/B); restores the
/// defaults so a thread reused by a later test sees them off again.
fn bid_xfer(auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_competition_over_transfer(true);
    crate::bidding::american::set_transfer_super_accept(true);
    let result = best_call(auction, hand);
    super::set_competition_over_transfer(false);
    crate::bidding::american::set_transfer_super_accept(false);
    result
}

/// As [`best_call`], with our 2♠ minor-transfer competition (Side A) forced on
/// (it is also the default, but pin it so a thread that another test left off
/// still sees it); restores the on default afterward.
fn bid_minor(auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_competition_over_minor_transfer(true);
    let result = best_call(auction, hand);
    super::set_competition_over_minor_transfer(true);
    result
}

/// As [`best_call`], with our 2NT diamond-transfer competition (Side A) forced
/// on (it is also the default, but pin it so a thread that another test left off
/// still sees it); restores the on default afterward.
fn bid_diamond(auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_competition_over_diamond_transfer(true);
    let result = best_call(auction, hand);
    super::set_competition_over_diamond_transfer(true);
    result
}

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
    super::set_competition_over_diamond_transfer(false);
    let (_, floored) = best_call(&auction, "Axx.Kxx.Qxx.AKxx");
    super::set_competition_over_diamond_transfer(true); // restore the on default
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
fn opener_answers_cue_raise_instead_of_passing() {
    // 1♠ – (2♣) – 3♣ (cue-raise = limit-plus spade raise) – P: opener must not
    // leave the cuebid in. The screenshot deal's East (♠QT743 ♥KQ7 ♦832 ♣A9,
    // 11 HCP — a minimum) declines by signing off in 3♠, from the book.
    let auction = [
        call(1, Strain::Spades),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = best_call(&auction, "QT743.KQ7.832.A9");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(
        !floored,
        "opener's answer must come from the cue-raise book"
    );
}

#[test]
fn michaels_cue_of_our_major_is_not_a_cue_raise() {
    // 1♠ – (2♠ Michaels, a cue of OUR spades) – 3♠ (responder's NATURAL raise)
    // – P: the cue-raise answer table must not hijack this. A strong opener
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
fn opener_answers_minor_cue_raise() {
    // 1♦ – (2♣) – 3♣ (cue-raise = limit-plus diamond raise) – P.
    // Minimum, no club stopper (12 HCP, ♣Q doubleton) → sign off 3♦.
    let auction = [
        call(1, Strain::Diamonds),
        call(2, Strain::Clubs),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = best_call(&auction, "K43.Q43.AJ632.Q5");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the minor sign-off must come from the book");
    // Values + a club stopper (17 HCP, ♣Kx) → accept the best game, 3NT.
    let (c, floored) = best_call(&auction, "A54.Q43.AKJ32.K5");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(!floored, "the 3NT accept must come from the book");
}

#[test]
fn minor_cue_raise_decline_jumps_when_3m_is_below_the_cue() {
    // 1♣ – (2♦) – 3♦ (cue-raise = limit-plus club raise) – P: 3♣ now sits
    // *below* the cue and is illegal, so a minimum opener must decline in 4♣,
    // not pass the cuebid out. Guards the 4m fallback rung.
    let auction = [
        call(1, Strain::Clubs),
        call(2, Strain::Diamonds),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = best_call(&auction, "A32.K43.43.KQ432");
    assert_eq!(c, call(4, Strain::Clubs));
    assert!(!floored, "the 4♣ decline must come from the book");
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

#[test]
fn uvu_three_clubs_is_stayman() {
    // 1NT–(2NT both minors): a 4-4 majors hand bids 3♣ (Stayman), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, floored) = bid_uvu(&auction, "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "the cue must come from the book");
}

#[test]
fn uvu_three_diamonds_shows_hearts() {
    // 1NT–(2NT): 5+♥ with ≤3♠ bids 3♦ (the heart cue).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, _) = bid_uvu(&auction, "K3.KQ976.A32.432");
    assert_eq!(c, call(3, Strain::Diamonds));
}

#[test]
fn uvu_splinter_with_five_five() {
    // 1NT–(2NT): 5-5 majors short a club → 4♣ splinter (FG+).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, _) = bid_uvu(&auction, "AQ876.KJ987.32.A");
    assert_eq!(c, call(4, Strain::Clubs));
}

#[test]
fn uvu_penalty_double_on_values() {
    // 1NT–(2NT): flat values, no 4-card major, no minor stopper → penalty X.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (c, floored) = bid_uvu(&auction, "KJ2.AQ2.J532.532");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the penalty X must come from the book");
}

#[test]
fn uvu_smolen_shows_the_five_card_spade() {
    // 1NT–(2NT)–3♣–(P)–3♦ (denial): responder's 3♥ = Smolen 5+♠ (no ♥ promise).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_uvu(&auction, "AQ876.K32.A32.32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "Smolen must come from the book");
}

#[test]
fn uvu_disabled_falls_to_floor() {
    // Disabled, 1NT–(2NT) has no book node → instinct floor (the toggle works).
    super::set_uvu(false);
    let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
    let (_, floored) = best_call(&auction, "AQ32.KJ32.A2.432");
    super::set_uvu(true); // restore the default for sibling tests on this thread
    assert!(floored, "without the toggle the auction is unauthored");
}

/// 1NT-(2NT)-X, opponents run to 3♣: responder with a club stack doubles
/// (the UvU penalty chase), and partner would leave it in.
///
/// Asserted against `american_instinct()`, not `american()`, because the
/// chase is a rule of the **deterministic** ladder gated on
/// `set_uvu_encircle` — and `with_floor` attaches `instinct()` to the
/// *constructive* book only, so on a contested auction like this one the
/// learned floor is the sole answer and the rule never runs. Through
/// `american()` this test asserted that the net happened to *agree* with the
/// ladder, which the v3 net did and the configured v4 net does not (it
/// passes, X 7.813 against P 8.544 — a 0.73-logit margin). Pinning an
/// individual net call is what `tests/common/mod.rs` exists to forbid; the
/// net is validated in aggregate by the `ab-*` harnesses.
#[test]
fn uvu_encircling_doubles_the_runout() {
    super::set_uvu(true);
    crate::bidding::instinct::set_uvu_encircle(true);
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Notrump),
        Call::Double,
        call(3, Strain::Clubs),
        Call::Pass,
        Call::Pass,
    ];
    let hand: Hand = "K54.84.732.KQJT9".parse().expect("valid test hand");
    let (logits, _) = crate::bidding::american::american_instinct()
        .against()
        .classify_with_provenance(hand, RelativeVulnerability::NONE, &auction)
        .expect("a legal auction classifies");
    let c = (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty");
    crate::bidding::instinct::set_uvu_encircle(true); // the shipped default
    assert_eq!(c, Call::Double, "encircle the 3♣ runout with a club stack");
}

#[test]
fn transfer_smolen_three_clubs_is_stayman() {
    // 1NT–(2♦): a 4-4 majors game-force bids 3♣ Stayman (a book node).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "AQ32.KJ32.A2.432");
    assert_eq!(c, call(3, Strain::Clubs));
    assert!(!floored, "Stayman must come from the book");
}

#[test]
fn transfer_smolen_opener_answers_stayman() {
    // 1NT–(2♦)–3♣: opener shows a 4-card major (3♥ here).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "K2.AQ54.A32.Q432");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn transfer_smolen_three_diamonds_is_the_heart_transfer() {
    // The reshuffle: 1NT–(2♦)–3♦ shows hearts (the freed cue slot), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "K3.KQ976.A32.432");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the heart transfer must come from the book");

    // Opener auto-drives the INV+ transfer to game with a fit.
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&opener, "AQ5.A432.KQ4.J32");
    assert_eq!(c, call(4, Strain::Hearts));
}

#[test]
fn transfer_smolen_routes_five_four_to_stayman_not_a_transfer() {
    // A 5♠4♥ game-force must bid 3♣ Stayman (1.85), not the 3♥ spade transfer
    // (1.8) — else Smolen could never show the 5-4.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, _) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
    assert_eq!(c, call(3, Strain::Clubs));
}

#[test]
fn transfer_smolen_jumps_smolen_after_the_denial() {
    // 1NT–(2♦)–3♣–P–3♦(no major)–P: responder bids Smolen 3♥ to show 5 spades.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "Smolen must come from the book");

    // Opener completes in the five-card spade game.
    let mut full = auction.to_vec();
    full.push(call(3, Strain::Hearts));
    full.push(Call::Pass);
    let (c, _) = bid_transfer(&full, "Q32.A65.AQ43.K32");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_smolen_leaping_michaels_both_majors() {
    // 1NT–(2♦)–4♦ = both majors 5-5, game-forcing.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "KQ954.AJ876.2.32");
    assert_eq!(c, call(4, Strain::Diamonds));
    assert!(!floored, "Leaping Michaels must come from the book");

    // Opener bids game in the better major (4♠ on three-card support).
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(4, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&opener, "A32.K43.AQ32.Q42");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_smolen_keeps_cohen_over_a_major_overcall() {
    // Over (2♥), Transfer is plain Cohen: 5 spades transfers through
    // hearts — 3♦ shows spades (the Smolen reshuffle is (2♦)-only).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the Cohen transfer must come from the book");
}

#[test]
fn lebensohl_forcing_three_level_is_a_book_node() {
    // 1NT–(2♦); responder 5 spades, game values, no diamond stopper →
    // forcing 3♠ (a jump), not a partscore.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&auction, "KQT95.A43.32.J32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the forcing 3-level bid must come from the book");
}

#[test]
fn lebensohl_weak_long_suit_relays_then_completes() {
    // Weak hand (6 HCP), 6 clubs, over 2♦ → 2NT relay; opener forced to 3♣.
    let responder = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&responder, "J2.43.32.KQ9876");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the Lebensohl relay must come from the book");

    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (completion, _) = bid(&opener, "AQ32.KQ5.AQ4.A32");
    assert_eq!(completion, call(3, Strain::Clubs));
}

#[test]
fn lebensohl_weak_bids_natural_two_level() {
    // A weak hand with 5 hearts bids natural 2♥ (below 2NT), to play.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid(&auction, "K2.QJ976.432.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural 2-level bid must come from the book");
}

#[test]
fn lebensohl_cue_is_stayman() {
    // 1NT–(2♥): a game-force with 4 spades and no 5-card suit cues 3♥ = Stayman
    // (it cannot bid a forcing 3-level suit, and the cue outranks direct 3NT).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid(&auction, "AQ32.K43.A32.K32");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the cue must come from the book");

    // Opener answers Stayman with the 4-card spade fit.
    let opener = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (a, floored) = bid(&opener, "KJ54.A32.K43.Q32");
    assert_eq!(a, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn lebensohl_five_card_suit_relays_then_signs_off_at_the_three_level() {
    // Weak hand, a 5-card heart suit it cannot show at the 2 level (below
    // their 2♠): relay 2NT, then correct 3♣→3♥ as a 3-level sign-off.
    let responder = [call(1, Strain::Notrump), call(2, Strain::Spades)];
    let (c, floored) = bid(&responder, "32.KQJ32.432.432");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the relay must come from the book");

    let after_3c = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
    ];
    let (c, floored) = bid(&after_3c, "32.KQJ32.432.432");
    assert_eq!(c, call(3, Strain::Hearts));
    assert!(!floored, "the 3-level sign-off must come from the book");
}

#[test]
fn lebensohl_maximum_raises_weak_signoff_to_game() {
    // 1NT–(2♠)–2NT–P–3♣–P–3♥–P: responder's weak heart sign-off. A maximum
    // (17) opener with three-card support stretches to 4♥; a minimum passes.
    let after_signoff = [
        call(1, Strain::Notrump),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
        call(3, Strain::Clubs),
        Call::Pass,
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    // 4-3-4-2, 17: a flat 4-3-3-3 17-count would read 16 on the shipped
    // rule-of-N+8 scale and rightly decline the stretch.
    let (c, floored) = bid(&after_signoff, "AK32.K43.A432.K3");
    assert_eq!(c, call(4, Strain::Hearts));
    assert!(!floored, "the game raise must come from the book");

    let (c, _) = bid(&after_signoff, "AK32.K43.KQ3.432");
    assert_eq!(c, Call::Pass, "a minimum passes the weak sign-off");
}

#[test]
fn transfer_lebensohl_shows_spades_through_their_hearts() {
    // 1NT–(2♥); responder, 5 spades and game values, transfers *through*
    // hearts: 3♦ shows spades (not diamonds), a book node.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
    assert_eq!(c, call(3, Strain::Diamonds));
    assert!(!floored, "the transfer must come from the book");
}

#[test]
fn transfer_lebensohl_opener_bids_game_not_a_partscore() {
    // After 1NT–(2♥)–3♦ (transfer to spades), opener with a fit must bid
    // the spade *game*, never a 3♠ partscore (the Rubensohl-v1 failure).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    let (c, _) = bid_transfer(&auction, "AK5.KQ52.A43.432");
    assert_eq!(c, call(4, Strain::Spades));
}

#[test]
fn transfer_lebensohl_cue_is_stayman() {
    // 1NT–(2♥)–3♥ is the cue = Stayman; opener answers a 4-card major.
    // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Hearts),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "AQ32.K43.A32.K32");
    assert_eq!(c, call(3, Strain::Spades));
    assert!(!floored, "the Stayman answer must come from the book");
}

#[test]
fn transfer_lebensohl_keeps_the_penalty_double() {
    // Length and values in their suit, no game bid of our own: with the
    // `Penalty` style on, double from the book — Rubensohl v1 lost this by
    // shadowing the floor. (The default is now `Optional` (2-3 cards), which
    // would route this 4-card-diamond hand elsewhere; see
    // [`takeout_authored_double`].)
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(super::DoubleStyle::Penalty, &auction, "K2.K43.J932.Q432");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the penalty double must come from the book");
}

/// As [`bid_transfer`], with the given double meaning forced on; resets the
/// double style to the default afterward so it cannot leak across tests on
/// the same thread.
fn bid_transfer_dbl(style: super::DoubleStyle, auction: &[Call], hand: &str) -> (Call, bool) {
    super::set_lebensohl_style(super::LebensohlStyle::Transfer);
    super::set_double_style(style);
    let result = best_call(auction, hand);
    super::set_double_style(super::DoubleStyle::default());
    result
}

#[test]
fn takeout_authored_double() {
    // Takeout: short in their suit (2♦) with values doubles from the book —
    // a hand the `Penalty` style (4+ ♦) would never double.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K432.K432.32.Q43");
    assert_eq!(c, Call::Double);
    assert!(
        !floored,
        "the authored takeout double must come from the book"
    );
}

#[test]
fn optional_double_two_three_cards() {
    // Optional: exactly 3 cards in their suit (♦) with values doubles…
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer_dbl(super::DoubleStyle::Optional, &auction, "K43.K43.432.Q43");
    assert_eq!(c, Call::Double);
    assert!(!floored, "the optional double must come from the book");

    // …but a singleton in their suit does NOT double (it routes elsewhere).
    let (c, _) = bid_transfer_dbl(super::DoubleStyle::Optional, &auction, "K432.K432.2.Q432");
    assert_ne!(
        c,
        Call::Double,
        "short-in-their-suit must not make an optional double"
    );
}

#[test]
fn opener_pulls_a_takeout_double() {
    // After 1NT–(2♦)–X–(P), opener has no authored node and falls to the
    // floor: a maximum with a diamond stopper pulls to 3NT…
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let (c, floored) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "AQ2.AQ2.A32.Q432");
    assert_eq!(c, call(3, Strain::Notrump));
    assert!(floored, "opener's pull comes from the instinct floor");

    // …while a diamond stack sits for penalty (passes the double).
    let (c, _) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K32.A32.AKQ2.J32");
    assert_eq!(c, Call::Pass, "a trump stack converts to penalty");
}

#[test]
fn transfer_lebensohl_weak_bids_natural_two_level() {
    // Weak 5-card heart hand still bids natural 2♥ (transfers are INV+).
    let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
    let (c, floored) = bid_transfer(&auction, "K2.QJ976.432.432");
    assert_eq!(c, call(2, Strain::Hearts));
    assert!(!floored, "the natural 2-level bid must come from the book");
}

#[test]
fn transfer_lebensohl_top_step_is_a_clubs_transfer() {
    // The top step (no suit above to transfer into) is a forced game-force
    // transfer to clubs: 6+♣, game values, no stopper in their suit. The same
    // 10-HCP hand bids it over every overcall — 3♠ over (2♦)/(2♥), 3♥ over
    // (2♠) — a book node, never the natural floor. Tested under `Penalty`: the
    // default `Takeout` (≤3 in their suit, 1.55) outranks the clubs transfer
    // (1.45) and would hijack this short-suit hand into a takeout double — a
    // known weight interaction; the structural node is checked here in
    // isolation.
    let hand = "32.543.32.AKQJ86";
    for (over, top) in [
        (Strain::Diamonds, Strain::Spades),
        (Strain::Hearts, Strain::Spades),
        (Strain::Spades, Strain::Hearts),
    ] {
        let auction = [call(1, Strain::Notrump), call(2, over)];
        let (c, floored) = bid_transfer_dbl(super::DoubleStyle::Penalty, &auction, hand);
        assert_eq!(c, call(3, top), "top step → clubs over (2{over:?})");
        assert!(!floored, "the clubs transfer must come from the book");
    }
}

#[test]
fn transfer_lebensohl_traps_a_too_good_stopper() {
    // Over 1NT–(2♥) with game values, a *too-good* heart stopper (♥AQ86, 6
    // HCP in their suit) traps: pass and wait for opener's reopening takeout
    // double, then convert. A merely *adequate* stopper (♥A964, 4 HCP) is a
    // source of tricks and still declares 3NT. (Trap pass on by default.)
    // The trap is a takeout-style mechanism — under the default Penalty style
    // this 4-card-heart hand doubles for penalty directly — so it is pinned to
    // Takeout here; the 3NT line (1.7) outranks any double, so it is style-free.
    let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
    let (trap, _) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K32.AQ86.KJ5.J32");
    assert_eq!(
        trap,
        Call::Pass,
        "a too-good stopper (6 HCP in hearts) traps"
    );
    // Also pinned to Takeout: under Penalty default this 4-card-heart hand
    // prefers the penalty double (1.55) to the relay's direct 3NT (1.5) — four
    // trumps behind declarer beat one fragile stopper, which is sound.
    let (bid, _) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K32.A964.KJ5.Q32");
    assert_eq!(
        bid,
        call(3, Strain::Notrump),
        "an adequate stopper (4 HCP in hearts) still bids 3NT"
    );
}

#[test]
fn transfer_lebensohl_top_step_opener_completes_at_game() {
    // After 1NT–(2♥)–3♠ (transfer to clubs, forced GF): opener bids 3NT with
    // a heart stopper, else raises to 5♣ — 3♣ is unplayable, so it reaches game.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        call(3, Strain::Spades),
        Call::Pass,
    ];
    let (c, floored) = bid_transfer(&auction, "A432.KQ5.A32.432");
    assert_eq!(c, call(3, Strain::Notrump), "stopper → 3NT");
    assert!(!floored, "the completion must come from the book");

    let (c, _) = bid_transfer(&auction, "A432.543.AKQ.432");
    assert_eq!(c, call(5, Strain::Clubs), "no stopper → 5♣");
}

#[test]
fn opener_leaves_in_responder_penalty_double_when_penalty_style() {
    use super::{DoubleStyle, set_double_style, set_penalty_double_leave_in};
    // [1NT,(2♥),X,(P)] — responder penalty-doubled their heart overcall.
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    super::set_lebensohl_style(super::LebensohlStyle::Plain);
    // Penalty style + leave-in on: opener SITS, and it is an authored node.
    set_double_style(DoubleStyle::Penalty);
    set_penalty_double_leave_in(true);
    let (c_on, floored_on) = best_call(&auction, "AQ5.J42.KQ3.K842"); // flat 15, no ♥ stop
    assert_eq!(c_on, Call::Pass, "penalty double left in");
    assert!(
        !floored_on,
        "the leave-in must be a book node, not the floor"
    );
    // Leave-in off: the floor reads the double as takeout and pulls — not a Pass.
    set_penalty_double_leave_in(false);
    let (c_off, floored_off) = best_call(&auction, "AQ5.J42.KQ3.K842");
    assert!(
        floored_off,
        "off → the node is gone, opener falls to the floor"
    );
    assert_ne!(
        c_off,
        Call::Pass,
        "the floor advances the double instead of sitting"
    );
    // Restore the defaults for other tests sharing this thread.
    set_penalty_double_leave_in(true);
    set_double_style(DoubleStyle::Penalty);
    super::set_lebensohl_style(super::LebensohlStyle::Transfer);
    set_double_style(DoubleStyle::Optional);
}

#[test]
fn opener_cooperates_with_responder_optional_double() {
    use super::{DoubleStyle, set_double_style, set_penalty_double_leave_in};
    // [1NT,(2♥),X,(P)] — responder's OPTIONAL double (2-3 hearts + values).
    let auction = [
        call(1, Strain::Notrump),
        call(2, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    super::set_lebensohl_style(super::LebensohlStyle::Plain);
    set_double_style(DoubleStyle::Optional);
    set_penalty_double_leave_in(true);
    // Three-card fit (♥Q93): stand and defend the doubled overcall.
    let (fit, floored) = best_call(&auction, "AK5.Q93.KJ54.Q5");
    assert_eq!(fit, Call::Pass, "a three-card fit stands");
    assert!(!floored, "the cooperation must be an authored node");
    // Doubleton in their suit + a five-card suit (♣AKQ76): run with xx.
    let (run, _) = best_call(&auction, "A52.93.KJ5.AKQ76");
    assert_eq!(
        run,
        call(3, Strain::Clubs),
        "a doubleton runs to the five-card suit"
    );
    // Doubleton but no five-card suit: nowhere to run, so stand.
    let (stuck, _) = best_call(&auction, "A52.93.KJ54.AKQ6");
    assert_eq!(stuck, Call::Pass, "a doubleton with no suit stands");
    set_double_style(DoubleStyle::Penalty); // restore the default
    super::set_lebensohl_style(super::LebensohlStyle::Transfer);
    set_double_style(DoubleStyle::Optional);
}

#[test]
fn uvu_major_cues_split_raise_and_fourth_suit() {
    super::set_uvu_over_majors(true);
    // [1♥, (2NT both minors)]: 12-count with 3 hearts → 3♣ = limit+ raise.
    let auction = [call(1, Strain::Hearts), call(2, Strain::Notrump)];
    let (raise, floored) = best_call(&auction, "K52.QJ5.A964.Q32");
    assert_eq!(raise, call(3, Strain::Clubs), "the cheap cue raises");
    assert!(!floored, "an authored node, not the floor");
    // 14-count, 5 spades, 2 hearts → 3♦ = game force in the other major.
    let (fourth, _) = best_call(&auction, "AQJ54.K5.965.A43");
    assert_eq!(fourth, call(3, Strain::Diamonds), "the second cue forces");
    super::set_uvu_over_majors(true);
}

#[test]
fn michaels_cue_of_our_major_gets_a_structure() {
    super::set_uvu_over_majors(true);
    // [1♠, (2♠ Michaels)]: a limit raise cues their known major (3♥)...
    let auction = [call(1, Strain::Spades), call(2, Strain::Spades)];
    let (cue, floored) = best_call(&auction, "KQ5.A54.96432.Q2");
    assert_eq!(cue, call(3, Strain::Hearts), "the known-suit cue raises");
    assert!(!floored, "an authored node, not the floor");
    // ...while a competitive 7-count raises 3♠ naturally — the raise
    // keeps its meaning over their cue of our own suit.
    let (raise, _) = best_call(&auction, "Q542.95.9643.KQ3");
    assert_eq!(raise, call(3, Strain::Spades), "the natural raise survives");
    super::set_uvu_over_majors(true);
}

#[test]
fn opener_answers_the_uvu_major_cue() {
    super::set_uvu_over_majors(true);
    // [1♥, (2NT), 3♣ = limit+ raise, (P)]: a minimum declines in 3♥, a
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
    super::set_uvu_over_majors(true);
}

#[test]
fn opener_answers_the_uvu_fourth_suit_force() {
    super::set_uvu_over_majors(true);
    // [1♥, (2NT), 3♦ = GF 5+ spades, (P)]: three-card support raises the
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
    super::set_uvu_over_majors(true);
}

#[test]
fn weak_two_doubled_gets_business_redouble_and_systems_on() {
    super::set_weak_two_competition(true);
    // [2♠, (X)]: 17-count with a singleton spade — no Ogust fit — redoubles.
    let auction = [call(2, Strain::Spades), Call::Double];
    let (xx, floored) = best_call(&auction, "A.K654.A964.KQ32");
    assert_eq!(xx, Call::Redouble, "business redouble on values");
    assert!(!floored, "an authored node, not the floor");
    // A 3-card raise stays preemptive (RONF rides through unchanged).
    let (raise, _) = best_call(&auction, "954.Q542.964.432");
    assert_eq!(raise, call(3, Strain::Spades), "the raise stays preemptive");
    // Deeper continuations are systems-on: opener answers Ogust through
    // the rebase exactly as if undisturbed (min points, good suit → 3♦).
    let ogust = [
        call(2, Strain::Hearts),
        Call::Double,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (answer, _) = best_call(&ogust, "54.KQ9654.96.432");
    assert_eq!(answer, call(3, Strain::Diamonds), "Ogust survives their X");
    super::set_weak_two_competition(false);
}

#[test]
fn weak_two_overcalled_double_is_values_and_ogust_survives() {
    super::set_weak_two_competition(true);
    // [2♥, (2♠)]: a 12-count doubles for penalty-leaning values.
    let auction = [call(2, Strain::Hearts), call(2, Strain::Spades)];
    let (double, floored) = best_call(&auction, "KJ54.Q5.A964.Q32");
    assert_eq!(double, Call::Double, "values double");
    assert!(!floored, "an authored node, not the floor");
    // A 16-count with a doubleton heart still asks Ogust...
    let (ask, _) = best_call(&auction, "AK54.Q5.A964.K32");
    assert_eq!(ask, call(2, Strain::Notrump), "Ogust survives the overcall");
    // ...and opener's five-rung answer arrives through the targeted rebase.
    let answered = [
        call(2, Strain::Hearts),
        call(2, Strain::Spades),
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (answer, _) = best_call(&answered, "54.KQ9654.96.432");
    assert_eq!(answer, call(3, Strain::Diamonds), "min points, good suit");
    super::set_weak_two_competition(false);
}

#[test]
fn strong_two_contested_stays_strong() {
    super::set_strong_two_competition(true);
    // [2♣, (X)]: systems on — a bust still gives the 2♥ double negative.
    let doubled = [call(2, Strain::Clubs), Call::Double];
    let (negative, floored) = best_call(&doubled, "9542.Q54.964.432");
    assert_eq!(negative, call(2, Strain::Hearts), "systems on over their X");
    assert!(!floored, "the rebase resolves to the authored tree");
    // [2♣, (2♠)]: a positive with good hearts bids them naturally (3♥ —
    // the 2-level is gone); a values hand without a suit doubles; a bust
    // passes and waits.
    let overcalled = [call(2, Strain::Clubs), call(2, Strain::Spades)];
    let (positive, _) = best_call(&overcalled, "54.AQ542.964.Q32");
    assert_eq!(positive, call(3, Strain::Hearts), "natural positive");
    let (waiting, _) = best_call(&overcalled, "954.Q542.964.432");
    assert_eq!(waiting, Call::Pass, "the waiting pass");
    // ...backed by opener's forced reopening: 24 balanced with a spade
    // stopper rebids 2NT rather than selling out.
    let reopen = [
        call(2, Strain::Clubs),
        call(2, Strain::Spades),
        Call::Pass,
        Call::Pass,
    ];
    let (rebid, _) = best_call(&reopen, "AQ2.AKQ5.KQ54.A2");
    assert_eq!(rebid, call(2, Strain::Notrump), "opener never sells out");
    super::set_strong_two_competition(true);
}

#[test]
fn major_support_double_shows_three_spades() {
    super::set_major_support_double(true);
    // [1♥, (P), 1♠, (2♣)]: opener with exactly three spades doubles.
    let auction = [
        call(1, Strain::Hearts),
        Call::Pass,
        call(1, Strain::Spades),
        call(2, Strain::Clubs),
    ];
    let (support, floored) = best_call(&auction, "K32.AQ542.A95.32");
    assert_eq!(support, Call::Double, "exactly three = support double");
    assert!(!floored, "an authored node, not the floor");
    super::set_major_support_double(true);
}

#[test]
fn modern_negative_double_is_exactly_four_over_one_heart() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
    // [1♦, (1♥)]: five spades bid the free 1♠; exactly four double.
    let auction = [call(1, Strain::Diamonds), call(1, Strain::Hearts)];
    let (free, floored) = best_call(&auction, "AQ542.95.964.Q32");
    assert_eq!(free, call(1, Strain::Spades), "five spades bid the suit");
    assert!(!floored, "an authored node, not the floor");
    let (neg, _) = best_call(&auction, "AQ54.95.9642.Q32");
    assert_eq!(neg, Call::Double, "exactly four doubles");
    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
}

#[test]
fn free_bids_fill_the_natural_gaps() {
    super::set_free_bids(true);
    // [1♠, (2♦)]: an 11-count with five hearts bids the 2/1-ish 2♥.
    let auction = [call(1, Strain::Spades), call(2, Strain::Diamonds)];
    let (two_hearts, floored) = best_call(&auction, "K5.AQ542.964.Q32");
    assert_eq!(two_hearts, call(2, Strain::Hearts), "the 2-level free bid");
    assert!(!floored, "an authored node, not the floor");
    // [1♥, (1♠)]: a balanced 10 with a spade stopper bids 1NT.
    let one_nt_auction = [call(1, Strain::Hearts), call(1, Strain::Spades)];
    let (one_nt, _) = best_call(&one_nt_auction, "K52.95.KJ64.QJ32");
    assert_eq!(one_nt, call(1, Strain::Notrump), "the natural 1NT");
    super::set_free_bids(false);
}

#[test]
fn free_bid_floor_gates_the_marginal_hand() {
    super::set_free_bids(true);
    // [1♣, (1♦)]: a 6-ish balanced hand with five hearts. At the default
    // floor of 6 it makes the 1♥ free bid; raise the floor to 8 and it no
    // longer qualifies (falls through to the floor's pass).
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let hand = "T32.KJ542.94.Q32";
    let (bid_at_6, _) = best_call(&auction, hand);
    assert_eq!(
        bid_at_6,
        call(1, Strain::Hearts),
        "the 1♥ free bid at floor 6"
    );
    super::set_free_bid_floor(8);
    let (bid_at_8, _) = best_call(&auction, hand);
    assert_ne!(
        bid_at_8,
        call(1, Strain::Hearts),
        "floor 8 rejects the 6-count"
    );
    super::set_free_bid_floor(6);
    super::set_free_bids(false);
}

#[test]
fn cachalot_rotates_the_one_level() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    // X = 4+ hearts; 1♥ = 4+ spades; 1♠ = the residual takeout hand.
    let (x, floored) = best_call(&auction, "K52.QJ54.964.Q32");
    assert_eq!(x, Call::Double, "X shows the adjacent major");
    assert!(!floored, "an authored node, not the floor");
    let (transfer, _) = best_call(&auction, "QJ54.K52.964.Q32");
    assert_eq!(transfer, call(1, Strain::Hearts), "1♥ shows spades");
    let (takeout, _) = best_call(&auction, "K52.Q54.964.QJ32");
    assert_eq!(takeout, call(1, Strain::Spades), "1♠ is the takeout hand");
    // Opener completes the heart transfer with exactly three, raises four.
    let complete = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let (three, _) = best_call(&complete, "AQ54.K52.96.QJ32");
    assert_eq!(three, call(1, Strain::Hearts), "exactly three completes");
    let (four, _) = best_call(&complete, "AQ5.K542.96.QJ32");
    assert_eq!(four, call(2, Strain::Hearts), "four raises");
    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn cachalot_probe_spades3() {
    use contract_bridge::Strain::*;
    super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
    let h = "A2.KJ54.KQ543.A2"; // opener-ish, 4 spades
    let cases = vec![
        (
            "1D(1H)X (1S)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                Call::Double,
                call(1, Spades),
            ],
        ),
        (
            "1D(1H)X (1NT)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                Call::Double,
                call(1, Notrump),
            ],
        ),
        (
            "1D(1H)X (2C)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                Call::Double,
                call(2, Clubs),
            ],
        ),
        (
            "1C(1H)X (2C)",
            vec![
                call(1, Clubs),
                call(1, Hearts),
                Call::Double,
                call(2, Clubs),
            ],
        ),
        (
            "nat 1D(1H)1S(2C)",
            vec![
                call(1, Diamonds),
                call(1, Hearts),
                call(1, Spades),
                call(2, Clubs),
            ],
        ),
        (
            "1C(1D)X (2C) [hearts fam]",
            vec![
                call(1, Clubs),
                call(1, Diamonds),
                Call::Double,
                call(2, Clubs),
            ],
        ),
        (
            "nat 1C(1D)1H(2C)",
            vec![
                call(1, Clubs),
                call(1, Diamonds),
                call(1, Hearts),
                call(2, Clubs),
            ],
        ),
    ];
    for (t, a) in cases {
        eprintln!("{t:28}: {:?}", best_call(&a, h));
    }
    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn cachalot_probe_spades2() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
    // spades-family PASS-OUT: does the authored completion even fire over (1♥)?
    let passout = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Double,
        Call::Pass,
    ];
    eprintln!(
        "1D(1H)X P  opener 4sp: {:?}",
        best_call(&passout, "AQ54.K2.KQ543.A2")
    );
    eprintln!(
        "1D(1H)X P  opener 3sp: {:?}",
        best_call(&passout, "AQ5.K42.KQ543.A2")
    );
    // and the (1D) hearts pass-out for contrast:
    let ph = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    eprintln!(
        "1C(1D)X P  opener 4he: {:?}",
        best_call(&ph, "A2.KQ54.A3.KJ654")
    );
    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn cachalot_probe_spades() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
    // Is 1♦(1♥)X even the spade transfer? And does reveal fire?
    let respond = [call(1, Strain::Diamonds), call(1, Strain::Hearts)];
    eprintln!(
        "responder 4=spades: {:?}",
        best_call(&respond, "KJ54.952.A64.Q32")
    );
    for (a, tag) in [
        (
            vec![
                call(1, Strain::Diamonds),
                call(1, Strain::Hearts),
                Call::Double,
                call(2, Strain::Clubs),
            ],
            "X 2C",
        ),
        (
            vec![
                call(1, Strain::Diamonds),
                call(1, Strain::Hearts),
                call(1, Strain::Spades),
                call(2, Strain::Clubs),
            ],
            "1S 2C",
        ),
        (
            vec![
                call(1, Strain::Clubs),
                call(1, Strain::Hearts),
                Call::Double,
                call(2, Strain::Clubs),
            ],
            "1C:X 2C",
        ),
    ] {
        eprintln!("{tag}: {:?}", best_call(&a, "A2.KJ54.KQ543.A2"));
    }
    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn cachalot_x_contested_answer_raises_the_shown_major() {
    // Under competition the pass-out completion doesn't fire; opener's
    // authored contested answer raises the major the X showed at the level
    // the intervention forces — a fit the floor would otherwise leave for a
    // bare double. Hearts over (1♦), spades over (1♥).
    super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
    // [1♣, (1♦), X(=4+♥), (2♦)]: opener with four hearts jumps to 3♥.
    let x_hearts = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        Call::Double,
        call(2, Strain::Diamonds),
    ];
    let (raise, _) = best_call(&x_hearts, "A2.KQ42.A3.KJ654");
    assert_eq!(raise, call(3, Strain::Hearts), "four-card support jumps");
    // Three-card support makes the simple raise to 2♥ (2♥ is above 2♦).
    let (simple, _) = best_call(&x_hearts, "A32.KQ4.A63.KJ54");
    assert_eq!(simple, call(2, Strain::Hearts), "three-card simple raise");
    // No support ⇒ opener passes to defend, not a phantom bid.
    let (defend, _) = best_call(&x_hearts, "AQ32.J.KQ632.A54");
    assert_eq!(defend, Call::Pass, "no fit defends");

    // [1♦, (1♥), X(=4+♠), (2♣)]: over (1♥) the X shows spades — opener raises.
    let x_spades = [
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Double,
        call(2, Strain::Clubs),
    ];
    let (raise, _) = best_call(&x_spades, "KQ54.A2.KJ543.A2");
    assert_eq!(raise, call(3, Strain::Spades), "over (1♥) four spades jump");

    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn sputnik_negative_double_is_the_residual() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Sputnik);
    // [1♣, (1♦)]: a 4-card major is bid naturally at the 1-level...
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    // 7 HCP: a flat 6-count reads 5 on the rule-of-N+8 scale and passes.
    let (spades, floored) = best_call(&auction, "KJ54.952.964.QJ2");
    assert_eq!(spades, call(1, Strain::Spades), "four spades bid the suit");
    assert!(!floored, "an authored node, not the floor");
    // ...while X denies a biddable major — the residual, ≤3 in each.
    let (neg, _) = best_call(&auction, "K52.Q54.J964.Q32");
    assert_eq!(
        neg,
        Call::Double,
        "≤3 in both majors is the residual double"
    );
    super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn cachalot_natural_free_bids_get_the_forcing_answers() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
    // A natural 2-level free bid reaches Section 4d's forcing answers:
    // opener raises partner's freely bid diamonds with three.
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (raise, floored) = best_call(&answer, "A5.K52.Q64.KJ632");
    assert_eq!(raise, call(3, Strain::Diamonds), "the free bid is raised");
    assert!(!floored, "an authored node, not the floor");
    // The rotated 1-level call stays with its Section-9 completion —
    // 1♥ over (1♦) shows spades; exactly three completes 1♠.
    let complete = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    let (three, _) = best_call(&complete, "AQ5.K52.964.QJ32");
    assert_eq!(
        three,
        call(1, Strain::Spades),
        "the rotation completes, not answer_free_bid"
    );
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn sputnik_free_major_raise_needs_four() {
    super::set_negative_double_shape(super::NegativeDoubleShape::Sputnik);
    // Sputnik's natural 1-level major promises only four, so opener's
    // two-level raise demands four trumps — three would be a Moysian.
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Diamonds),
        call(1, Strain::Hearts),
        Call::Pass,
    ];
    let (three, floored) = best_call(&answer, "K52.Q52.K94.AJ63");
    assert_eq!(
        three,
        call(1, Strain::Notrump),
        "three trumps bid 1NT, not the Moysian raise"
    );
    assert!(!floored, "an authored node, not the floor");
    let (four, _) = best_call(&answer, "K5.Q542.K94.AJ63");
    assert_eq!(four, call(2, Strain::Hearts), "four trumps raise");
    super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
}

#[test]
fn negative_free_bid_is_weak_and_capped() {
    super::set_free_bid_style(super::FreeBidStyle::Negative);
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades)];
    // 8 points with a six-card suit: the classic NFB.
    let (weak, floored) = best_call(&auction, "52.Q4.KJ8642.T53");
    assert_eq!(weak, call(2, Strain::Diamonds), "the negative free bid");
    assert!(!floored, "an authored node, not the floor");
    // The same suit with game values starts with the widened double.
    let (strong, _) = best_call(&auction, "52.A4.AKJ642.Q53");
    assert_eq!(strong, Call::Double, "12+ doubles first");
    // Opener drops the capped free bid with a minimum (the Pass answer).
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (drop, drop_floored) = best_call(&answer, "A5.Q52.Q64.KJ632");
    assert_eq!(drop, Call::Pass, "the NFB is non-forcing");
    assert!(!drop_floored, "an authored node, not the floor");
    super::set_free_bid_style(super::FreeBidStyle::Forcing);
}

#[test]
fn negative_double_then_suit_is_game_forcing() {
    super::set_free_bid_style(super::FreeBidStyle::Negative);
    // The doubler clarifies with the concealed long suit — forcing to
    // game — and opener answers it with the forcing-answer table.
    let rebid = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let (fg, floored) = best_call(&rebid, "52.A4.AKJ642.Q53");
    assert_eq!(fg, call(2, Strain::Diamonds), "X then the suit is FG");
    assert!(!floored, "an authored node, not the floor");
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        Call::Double,
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (raise, raise_floored) = best_call(&answer, "A5.K52.Q64.KJ632");
    assert_eq!(
        raise,
        call(3, Strain::Diamonds),
        "opener answers the FG suit"
    );
    assert!(!raise_floored, "an authored node, not the floor");
    super::set_free_bid_style(super::FreeBidStyle::Forcing);
}

#[test]
fn free_bid_transfers_swap_the_two_level() {
    super::set_free_bid_style(super::FreeBidStyle::Transfer);
    // [1♣, (1♠)]: both red suits sit at the two level, so the slots swap
    // — 2♦ shows hearts, 2♥ shows diamonds (the wrap).
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades)];
    let (hearts, floored) = best_call(&auction, "52.KJ864.Q42.T53");
    assert_eq!(hearts, call(2, Strain::Diamonds), "2♦ transfers to hearts");
    assert!(!floored, "an authored node, not the floor");
    let (diamonds, _) = best_call(&auction, "52.Q42.KJ864.T53");
    assert_eq!(diamonds, call(2, Strain::Hearts), "2♥ wraps to diamonds");
    // Opener completes (2♥ on the true transfer, 3♦ on the wrap)…
    let complete = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (comp, comp_floored) = best_call(&complete, "A53.Q52.64.AJ632");
    assert_eq!(
        comp,
        call(2, Strain::Hearts),
        "opener completes and declares"
    );
    assert!(!comp_floored, "an authored node, not the floor");
    let wrap = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    let (wrap_comp, _) = best_call(&wrap, "A53.Q52.64.AJ632");
    assert_eq!(
        wrap_comp,
        call(3, Strain::Diamonds),
        "the wrap completes a level higher"
    );
    // …and the weak transferor passes the completion out.
    let clarify = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Hearts),
        Call::Pass,
    ];
    let (weak, weak_floored) = best_call(&clarify, "52.KJ864.Q42.T53");
    assert_eq!(weak, Call::Pass, "the weak hand passes the completion");
    assert!(!weak_floored, "an authored node, not the floor");
    // A lone two-level slot stays natural and forcing: over (1♥) only
    // diamonds sit at the two level.
    let lone = [call(1, Strain::Clubs), call(1, Strain::Hearts)];
    let (natural, _) = best_call(&lone, "K52.4.AQJ86.T532");
    assert_eq!(natural, call(2, Strain::Diamonds), "a lone slot is natural");
    super::set_free_bid_style(super::FreeBidStyle::Forcing);
}

#[test]
fn high_overcalls_get_a_structure() {
    super::set_high_overcall_responses(true);
    // [1♠, (3♦)]: 4 hearts + 12 HCP make the 3-level negative double; a
    // diamond stopper + 16 bids 3NT instead.
    let auction = [call(1, Strain::Spades), call(3, Strain::Diamonds)];
    let (neg, floored) = best_call(&auction, "K5.KQ54.965.A432");
    assert_eq!(neg, Call::Double, "the 3-level negative double");
    assert!(!floored, "an authored node, not the floor");
    let (game, _) = best_call(&auction, "K5.KQ54.A65.A432");
    assert_eq!(game, call(3, Strain::Notrump), "3NT with a stopper");
    // Opener answers the forcing double with the unbid major.
    let answer = [
        call(1, Strain::Spades),
        call(3, Strain::Diamonds),
        Call::Double,
        Call::Pass,
    ];
    let (major, _) = best_call(&answer, "AQ542.KJ54.96.32");
    assert_eq!(major, call(3, Strain::Hearts), "four hearts answer 3♥");
    super::set_high_overcall_responses(false);
}

#[test]
fn jordan_truscott_over_their_double() {
    super::set_jordan_truscott(true);
    let auction = [call(1, Strain::Spades), Call::Double];
    // Jordan 2NT: 4 trumps, limit+.
    let (jordan, floored) = best_call(&auction, "Q542.A5.K964.Q32");
    assert_eq!(jordan, call(2, Strain::Notrump), "Jordan/Truscott");
    assert!(!floored, "an authored node, not the floor");
    // Value redouble: 10+ without the fit.
    let (xx, _) = best_call(&auction, "K2.A54.K964.Q532");
    assert_eq!(xx, Call::Redouble, "the value redouble");
    // The jump raise flips preemptive.
    let (preempt, _) = best_call(&auction, "Q542.9.96432.Q32");
    assert_eq!(preempt, call(3, Strain::Spades), "preemptive jump raise");
    // A weak 2-level new suit is non-forcing — opener passes a minimum.
    let weak = [
        call(1, Strain::Spades),
        Call::Double,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    let (pass, weak_floored) = best_call(&weak, "AQ542.K54.96.432");
    assert_eq!(pass, Call::Pass, "the weak new suit is dropped");
    assert!(!weak_floored, "an authored node, not the floor");
    // Opener answers Jordan with the cue-raise ladder (not Jacoby 2NT,
    // which the systems-on rebase would have reached).
    let answer = [
        call(1, Strain::Spades),
        Call::Double,
        call(2, Strain::Notrump),
        Call::Pass,
    ];
    let (accept, _) = best_call(&answer, "AKQ54.K54.96.A32");
    assert_eq!(accept, call(4, Strain::Spades), "a maximum accepts");
    let (decline, _) = best_call(&answer, "AQ542.954.96.A32");
    assert_eq!(decline, call(3, Strain::Spades), "a minimum declines");
    super::set_jordan_truscott(true);
}

#[test]
fn redouble_answer_shadows_the_rebase_blast() {
    // [1♠ (X) XX (P)]: opener's rebid.  The systems-on rebase strips the
    // double and the redouble, so opener replays uncontested with
    // responder's shown 10+ unseen, and the floor re-prices this shaped
    // minimum (12 HCP, 15 points) as game-going — the remnant report's
    // worst per-board family (−16..−17 IMPs/board vulnerable).  The
    // authored answer passes — even with a long suit (one-of-a-suit
    // redoubled makes with overtricks; a 2M escape rung measured
    // −11 IMPs/fired and was deleted) — and shadows the floor.
    let auction = [
        call(1, Strain::Spades),
        Call::Double,
        Call::Redouble,
        Call::Pass,
    ];
    let opener = "KQ652..AKT764.85"; // 12 HCP 5=0=6=2, opened 1♠
    let (default_call, default_floored) = best_call(&auction, opener);
    assert_eq!(default_call, Call::Pass, "the authored answer passes");
    assert!(!default_floored, "the node shadows the floor");
    let (long, long_floored) = best_call(&auction, "KQJT65.2.KJ85.T4"); // 10 HCP, 6 spades
    assert_eq!(
        long,
        Call::Pass,
        "a long-suit minimum sits for the redoubled make"
    );
    assert!(!long_floored, "the sit is authored too");

    super::set_redouble_answer(false);
    let (off_call, _) = best_call(&auction, opener);
    super::set_redouble_answer(true);
    assert_ne!(
        off_call,
        Call::Pass,
        "the off arm: the rebase + floor bids on blindly"
    );
}

/// Renderability invariant: every guarded fallback in the competitive book
/// describes itself — the guard names its condition and a rebase names its
/// rewrite — so `render-book` and the web book show the whole book.  A new
/// bare `guard(closure)` fails here; wrap it in `described_guard`.
#[test]
fn competitive_fallbacks_are_renderable() {
    use crate::bidding::fallback::Fallback;

    let book = super::competition();
    let all = book.0.fallbacks();
    assert!(
        all.len() > 30,
        "the competitive book has {} guarded entries — the walk is broken",
        all.len()
    );

    for (auction, guard, fallback) in &all {
        let key = contract_bridge::auction::display_calls(auction).to_string();
        assert!(
            guard.describe().is_some(),
            "unlabeled guard at [{key}] — wrap it in described_guard"
        );
        if let Fallback::Rebase(rewrite) = fallback {
            assert!(
                rewrite.describe().is_some(),
                "opaque rebase at [{key}] — wrap it in described_rewrite"
            );
        }
    }

    // Two concrete probes.  The [1♠] systems-on rebase renders with its
    // first-call label (the direct-seat table itself became exact
    // per-overcall nodes and no longer lives in the fallback layer)…
    let (_, guard, fallback) = all
        .iter()
        .find(|(auction, ..)| auction.as_ref() == [Call::Bid(Bid::new(1, Strain::Spades))])
        .expect("a guarded entry at [1♠]");
    assert_eq!(guard.describe().as_deref(), Some("X …"));
    assert!(
        matches!(fallback, Fallback::Rebase(_)),
        "the systems-on entry is a rebase"
    );
    // …and the exact [1♠ (2♣)] node carries the negative double.
    let auction = [
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Bid(Bid::new(2, Strain::Clubs)),
    ];
    let rules = book
        .0
        .get(&auction)
        .expect("an exact node per overcall")
        .as_rules()
        .expect("an authored Rules table");
    assert!(
        rules.rules().iter().any(|rule| rule.call() == Call::Double),
        "the negative double renders"
    );
}

// --- Free 1NT floor + the natural 2NT jump over a 1-level overcall ---

/// `1♣ (1♦) 1NT`: a balanced 7-count with a diamond stopper takes the free
/// 1NT at the default floor of 6.
#[test]
fn free_1nt_fires_at_default_floor() {
    super::set_free_1nt_floor(6);
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let (c, floored) = best_call(&auction, "Q54.J54.KJ32.543");
    assert_eq!(c, call(1, Strain::Notrump));
    assert!(!floored, "the free 1NT is a book node");
}

/// Raising the isolated 1NT floor to 8 drops the 7-count from 1NT — and,
/// being decoupled, leaves the forcing 1-level suit bids untouched.
#[test]
fn free_1nt_dropped_above_raised_floor() {
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    super::set_free_1nt_floor(8);
    let (c, _) = best_call(&auction, "Q54.J54.KJ32.543");
    super::set_free_1nt_floor(6);
    assert_ne!(
        c,
        call(1, Strain::Notrump),
        "7 HCP is below the raised floor"
    );
}

/// `1♣ (1♦) 2NT`: a balanced 12-count with a diamond stopper — too strong
/// for the capped 1NT, no fit to cue — invites at 2NT (default-on).
#[test]
fn free_2nt_jump_fires_by_default() {
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let (c, floored) = best_call(&auction, "K54.K54.KJ32.Q54");
    assert_eq!(c, call(2, Strain::Notrump));
    assert!(!floored, "the 2NT jump is a book node");
}

/// The ladder boundary: a balanced 10-count with a stopper is still 1NT,
/// not the 2NT jump (which starts at 11).
#[test]
fn free_1nt_caps_below_the_jump() {
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let (c, _) = best_call(&auction, "K54.Q54.KJ32.J54");
    assert_eq!(c, call(1, Strain::Notrump), "10 HCP caps at 1NT");
}

/// Every overcall the retired `(≤2♠)` guard admitted over `1opening`:
/// the suit bids above the opening through 2♠, plus their 1NT.
fn admitted_overcalls(opening: Strain) -> Vec<Bid> {
    let open = Bid::new(1, opening);
    let mut bids: Vec<Bid> = (1..=2u8)
        .flat_map(|level| {
            [
                Strain::Clubs,
                Strain::Diamonds,
                Strain::Hearts,
                Strain::Spades,
            ]
            .into_iter()
            .map(move |strain| Bid::new(level, strain))
        })
        .filter(|&bid| bid > open)
        .collect();
    bids.push(Bid::new(1, Strain::Notrump));
    bids
}

/// The per-overcall exact tables evaluate identically to the retired
/// guarded table across the knob grid: the same logits for every hand,
/// in every column the guard admitted.
#[test]
fn per_overcall_tables_match_legacy() {
    use super::{
        FreeBidStyle, NegativeDoubleShape, over_their_overcall, over_their_overcall_legacy,
        set_free_bid_quality, set_free_bid_style, set_free_bids, set_negative_double_shape,
    };
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Suit;

    let hands: Vec<Hand> = [
        "QJ9862.43.752.83", // weak six spades
        "83.QJ9862.752.43", // weak six hearts
        "83.43.QJ9862.752", // weak six diamonds
        "83.43.752.QJ9862", // weak six clubs
        "KQ72.QJ84.652.83", // both majors, 8 HCP
        "KJ72.QT84.652.83", // both majors, 6 HCP
        "K53.Q42.J932.T87", // flat 7
        "K5.Q4.J9532.KT87", // five diamonds, 10
        "K5.AQJ96.532.T87", // five hearts, 10
        "KJ8.QT7.AJ94.986", // balanced 11, wide stoppers
        "AKQ2.KQ5.AQJ4.92", // 21 balanced
        "AQ2.K53.QJ42.T92", // 12 flat
        "2.98653.QJ742.92", // weak two-suiter
        "KQ842.75.652.J83", // five spades, 6
        "KQ84.752.652.J83", // four spades, 6
        "AQJ83.K4.KT7.J93", // five spades, 14
    ]
    .iter()
    .map(|hand| hand.parse().expect("valid probe hand"))
    .collect();

    for shape in [
        NegativeDoubleShape::BothMajors,
        NegativeDoubleShape::Modern,
        NegativeDoubleShape::Cachalot,
        NegativeDoubleShape::Sputnik,
    ] {
        set_negative_double_shape(shape);
        for style in [
            FreeBidStyle::Forcing,
            FreeBidStyle::Negative,
            FreeBidStyle::Transfer,
        ] {
            set_free_bid_style(style);
            for engaged in [false, true] {
                set_free_bids(engaged);
                for quality in [false, true] {
                    set_free_bid_quality(quality);
                    for opening in Suit::ASC {
                        let legacy = over_their_overcall_legacy(opening);
                        for overcall in admitted_overcalls(Strain::from(opening)) {
                            let table = over_their_overcall(opening, overcall);
                            let auction = [call(1, Strain::from(opening)), Call::Bid(overcall)];
                            for vul in [RelativeVulnerability::NONE, RelativeVulnerability::ALL] {
                                let context = Context::new(vul, &auction);
                                for &hand in &hands {
                                    assert_eq!(
                                        table.classify(hand, &context),
                                        legacy.classify(hand, &context),
                                        "shape {shape:?}, style {style:?}, free bids \
                                         {engaged}, quality {quality}: 1{opening} \
                                         ({overcall}), {hand}",
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    set_negative_double_shape(NegativeDoubleShape::Modern);
    set_free_bid_style(FreeBidStyle::Forcing);
    set_free_bids(false);
    set_free_bid_quality(false);
}

/// Build one package into a fresh trie.
fn compiled_package(package: super::Package) -> crate::bidding::Trie {
    let mut book = crate::bidding::Trie::new();
    super::compile_into(&mut book, &[package]);
    book
}

/// Assert two wirings of one package resolve and classify identically
/// over a superset of probe auctions: equal `Option<Logits>` per hand
/// catches both over- and under-expansion (an auction only one wiring
/// answers shows up as `Some` vs `None`).
fn assert_wirings_match(
    legacy: super::Package,
    current: super::Package,
    auctions: &[Vec<Call>],
    label: &str,
) {
    use crate::bidding::context::Context;

    let hands: Vec<Hand> = [
        "QJ9862.43.752.83",
        "83.QJ9862.752.43",
        "KQ72.QJ84.652.83",
        "K53.Q42.J932.T87",
        "K5.Q4.J9532.KT87",
        "KJ8.QT7.AJ94.986",
        "AKQ2.KQ5.AQJ4.92",
        "AQ2.K53.QJ42.T92",
        "2.98653.QJ742.92",
        "AQJ83.K4.KT7.J93",
    ]
    .iter()
    .map(|hand| hand.parse().expect("valid probe hand"))
    .collect();

    let old_book = compiled_package(legacy);
    let new_book = compiled_package(current);
    for auction in auctions {
        let context = Context::new(RelativeVulnerability::NONE, auction);
        for &hand in &hands {
            // Massless normalizes to unanswered: a guarded table that
            // rejects a hand is re-found on the fall-through pass (stuck
            // massless), while an exact node rejects-to-floor — the
            // documented exact-node semantic, and in the full book the
            // floor then answers where the guard wedged.  Mass-bearing
            // answers must match exactly.
            let classify = |book: &crate::bidding::Trie| {
                book.classify_floored(hand, &context, auction)
                    .map(|(logits, _)| logits)
                    .filter(super::super::super::array::Logits::has_mass)
            };
            assert_eq!(
                classify(&old_book),
                classify(&new_book),
                "{label}: {} with {hand}",
                contract_bridge::auction::display_calls(auction),
            );
        }
    }
}

/// Only legal auctions probe the wirings: a guard never checks legality
/// (it answers `2♣ (1♣)` if asked), but play can never ask.
fn ascending(auction: &[Call]) -> bool {
    let bids: Vec<Bid> = auction
        .iter()
        .filter_map(|call| match call {
            Call::Bid(bid) => Some(*bid),
            _ => None,
        })
        .collect();
    bids.windows(2).all(|pair| pair[0] < pair[1])
}

/// Every bid, for superset probing.
fn all_bids() -> Vec<Bid> {
    (1..=7u8)
        .flat_map(|level| {
            [
                Strain::Clubs,
                Strain::Diamonds,
                Strain::Hearts,
                Strain::Spades,
                Strain::Notrump,
            ]
            .into_iter()
            .map(move |strain| Bid::new(level, strain))
        })
        .collect()
}

/// Every converted package resolves and classifies exactly as its retired
/// guarded wiring, over a superset of the guard's auction space.
#[test]
fn converted_packages_match_legacy() {
    use super::{FreeBidStyle, NegativeDoubleShape};

    // Section 4: opener answers the negative double of a 2-level minor.
    let mut auctions = Vec::new();
    for major in [Strain::Hearts, Strain::Spades] {
        for bid in all_bids() {
            auctions.push(vec![
                call(1, major),
                Call::Bid(bid),
                Call::Double,
                Call::Pass,
            ]);
        }
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        super::answer_negative_double_package_legacy(),
        super::answer_negative_double_package(),
        &auctions,
        "answer-negative-double",
    );

    // Section 10: their jump / 3-level overcalls, and the double behind.
    let mut auctions = Vec::new();
    for opening in [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ] {
        for bid in all_bids() {
            auctions.push(vec![call(1, opening), Call::Bid(bid)]);
            auctions.push(vec![
                call(1, opening),
                Call::Bid(bid),
                Call::Double,
                Call::Pass,
            ]);
        }
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        super::high_overcall_package_legacy(),
        super::high_overcall_package(),
        &auctions,
        "high-overcall",
    );

    // Section 8: the contested strong 2♣, both seats.
    let mut auctions = Vec::new();
    for bid in all_bids() {
        auctions.push(vec![call(2, Strain::Clubs), Call::Bid(bid)]);
        auctions.push(vec![
            call(2, Strain::Clubs),
            Call::Bid(bid),
            Call::Pass,
            Call::Pass,
        ]);
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        super::strong_two_competition_package_legacy(),
        super::strong_two_competition_package(),
        &auctions,
        "strong-two-competition",
    );

    // Section 4d/4d′: opener answers the free bid, across the style knobs
    // that reshape the free-bid grammar.
    let mut auctions = Vec::new();
    for opening in [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ] {
        for ovc in all_bids() {
            for free in all_bids() {
                if free.level.get() > 2 {
                    continue;
                }
                auctions.push(vec![
                    call(1, opening),
                    Call::Bid(ovc),
                    Call::Bid(free),
                    Call::Pass,
                ]);
            }
        }
    }
    auctions.retain(|auction| ascending(auction));
    for shape in [
        NegativeDoubleShape::BothMajors,
        NegativeDoubleShape::Modern,
        NegativeDoubleShape::Cachalot,
        NegativeDoubleShape::Sputnik,
    ] {
        super::set_negative_double_shape(shape);
        for style in [
            FreeBidStyle::Forcing,
            FreeBidStyle::Negative,
            FreeBidStyle::Transfer,
        ] {
            super::set_free_bid_style(style);
            assert_wirings_match(
                super::free_bid_answer_package_legacy(),
                super::free_bid_answer_package(),
                &auctions,
                &format!("free-bid-answer ({shape:?}, {style:?})"),
            );
        }
    }
    super::set_negative_double_shape(NegativeDoubleShape::Modern);
    super::set_free_bid_style(FreeBidStyle::Forcing);

    // Section 4b/4c: opener answers the cue-raise, majors and minors.  One
    // auction space serves both — each package's own ceiling decides which
    // columns it claims.
    let mut auctions = Vec::new();
    for opening in [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ] {
        for ovc in all_bids() {
            for cue in all_bids() {
                auctions.push(vec![
                    call(1, opening),
                    Call::Bid(ovc),
                    Call::Bid(cue),
                    Call::Pass,
                ]);
            }
        }
    }
    auctions.retain(|auction| ascending(auction));
    assert_wirings_match(
        super::cue_raise_answer_package_legacy(),
        super::cue_raise_answer_package(),
        &auctions,
        "cue-raise-answer",
    );
    assert_wirings_match(
        super::cue_minor_raise_answer_package_legacy(),
        super::cue_minor_raise_answer_package(),
        &auctions,
        "cue-minor-raise-answer",
    );

    // Section 9: the Cachalot contested X — every intervention, plus the
    // pass-out the completions shadow.
    let mut auctions = Vec::new();
    for (opening, overcall) in [
        (Strain::Clubs, Strain::Diamonds),
        (Strain::Clubs, Strain::Hearts),
        (Strain::Diamonds, Strain::Hearts),
    ] {
        for intervention in all_bids()
            .into_iter()
            .map(Call::Bid)
            .chain([Call::Pass, Call::Redouble])
        {
            auctions.push(vec![
                call(1, opening),
                call(1, overcall),
                Call::Double,
                intervention,
            ]);
        }
    }
    auctions.retain(|auction| ascending(auction));
    super::set_negative_double_shape(NegativeDoubleShape::Cachalot);
    assert_wirings_match(
        super::cachalot_package_legacy(),
        super::cachalot_package(),
        &auctions,
        "cachalot-answer",
    );
    super::set_negative_double_shape(NegativeDoubleShape::Modern);
}
