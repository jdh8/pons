use super::super::tests::{best_call, best_call_with, call};
use crate::bidding::agreements::Agreements;
use contract_bridge::Strain;
use contract_bridge::auction::Call;

#[test]
fn free_bids_fill_the_natural_gaps() {
    let mut arm = Agreements::current();
    arm.competition.free_bids = true;
    // `1♠ (2♦)`: an 11-count with five hearts bids the 2/1-ish 2♥.
    let auction = [call(1, Strain::Spades), call(2, Strain::Diamonds)];
    let (two_hearts, floored) = best_call_with(&arm, &auction, "K5.AQ542.964.Q32");
    assert_eq!(two_hearts, call(2, Strain::Hearts), "the 2-level free bid");
    assert!(!floored, "an authored node, not the floor");
    // `1♥ (1♠)`: a balanced 10 with a spade stopper bids 1NT.
    let one_nt_auction = [call(1, Strain::Hearts), call(1, Strain::Spades)];
    let (one_nt, _) = best_call_with(&arm, &one_nt_auction, "K52.95.KJ64.QJ32");
    assert_eq!(one_nt, call(1, Strain::Notrump), "the natural 1NT");
}

#[test]
fn free_bid_floor_gates_the_marginal_hand() {
    let mut at_6 = Agreements::current();
    at_6.competition.free_bids = true;
    at_6.competition.free_bid_floor = 6;
    let mut at_8 = at_6;
    at_8.competition.free_bid_floor = 8;
    // `1♣ (1♦)`: a 6-ish balanced hand with five hearts. At the default
    // floor of 6 it makes the 1♥ free bid; raise the floor to 8 and it no
    // longer qualifies (falls through to the floor's pass).
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let hand = "T32.KJ542.94.Q32";
    let (bid_at_6, _) = best_call_with(&at_6, &auction, hand);
    assert_eq!(
        bid_at_6,
        call(1, Strain::Hearts),
        "the 1♥ free bid at floor 6"
    );
    let (bid_at_8, _) = best_call_with(&at_8, &auction, hand);
    assert_ne!(
        bid_at_8,
        call(1, Strain::Hearts),
        "floor 8 rejects the 6-count"
    );
}

#[test]
fn negative_free_bid_is_weak_and_capped() {
    let mut arm = Agreements::current();
    arm.competition.free_bid_style = super::free_bids::FreeBidStyle::Negative;
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades)];
    // 8 points with a six-card suit: the classic NFB.
    let (weak, floored) = best_call_with(&arm, &auction, "52.Q4.KJ8642.T53");
    assert_eq!(weak, call(2, Strain::Diamonds), "the negative free bid");
    assert!(!floored, "an authored node, not the floor");
    // The same suit with game values starts with the widened double.
    let (strong, _) = best_call_with(&arm, &auction, "52.A4.AKJ642.Q53");
    assert_eq!(strong, Call::Double, "12+ doubles first");
    // Opener drops the capped free bid with a minimum (the Pass answer).
    let answer = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (drop, drop_floored) = best_call_with(&arm, &answer, "A5.Q52.Q64.KJ632");
    assert_eq!(drop, Call::Pass, "the NFB is non-forcing");
    assert!(!drop_floored, "an authored node, not the floor");
}

#[test]
fn free_bid_transfers_swap_the_two_level() {
    let mut arm = Agreements::current();
    arm.competition.free_bid_style = super::free_bids::FreeBidStyle::Transfer;
    // `1♣ (1♠)`: both red suits sit at the two level, so the slots swap
    // — 2♦ shows hearts, 2♥ shows diamonds (the wrap).
    let auction = [call(1, Strain::Clubs), call(1, Strain::Spades)];
    let (hearts, floored) = best_call_with(&arm, &auction, "52.KJ864.Q42.T53");
    assert_eq!(hearts, call(2, Strain::Diamonds), "2♦ transfers to hearts");
    assert!(!floored, "an authored node, not the floor");
    let (diamonds, _) = best_call_with(&arm, &auction, "52.Q42.KJ864.T53");
    assert_eq!(diamonds, call(2, Strain::Hearts), "2♥ wraps to diamonds");
    // Opener completes (2♥ on the true transfer, 3♦ on the wrap)…
    let complete = [
        call(1, Strain::Clubs),
        call(1, Strain::Spades),
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    let (comp, comp_floored) = best_call_with(&arm, &complete, "A53.Q52.64.AJ632");
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
    let (wrap_comp, _) = best_call_with(&arm, &wrap, "A53.Q52.64.AJ632");
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
    let (weak, weak_floored) = best_call_with(&arm, &clarify, "52.KJ864.Q42.T53");
    assert_eq!(weak, Call::Pass, "the weak hand passes the completion");
    assert!(!weak_floored, "an authored node, not the floor");
    // A lone two-level slot stays natural and forcing: over (1♥) only
    // diamonds sit at the two level.
    let lone = [call(1, Strain::Clubs), call(1, Strain::Hearts)];
    let (natural, _) = best_call_with(&arm, &lone, "K52.4.AQJ86.T532");
    assert_eq!(natural, call(2, Strain::Diamonds), "a lone slot is natural");
}

// --- Free 1NT floor + the natural 2NT jump over a 1-level overcall ---

/// `1♣ (1♦) 1NT`: a balanced 7-count with a diamond stopper takes the free
/// 1NT at the default floor of 6.
#[test]
fn free_1nt_fires_at_default_floor() {
    let mut arm = Agreements::current();
    arm.competition.free_1nt_floor = 6;
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let (c, floored) = best_call_with(&arm, &auction, "Q54.J54.KJ32.543");
    assert_eq!(c, call(1, Strain::Notrump));
    assert!(!floored, "the free 1NT is a book node");
}

/// Raising the isolated 1NT floor to 8 drops the 7-count from 1NT — and,
/// being decoupled, leaves the forcing 1-level suit bids untouched.
#[test]
fn free_1nt_dropped_above_raised_floor() {
    let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
    let mut arm = Agreements::current();
    arm.competition.free_1nt_floor = 8;
    let (c, _) = best_call_with(&arm, &auction, "Q54.J54.KJ32.543");
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
