//! Integration tests for the notrump response structures in the 2/1 system
//!
//! Covers:
//! - Responses to the 2NT opening (3-level Stayman/transfers)
//! - System on after 2♣ - 2♦ - 2NT (22–24 balanced)
//! - Quantitative 4NT over the 1NT opening
//! - Acceptance of quantitative 4NT raises
//! - Simple responses after opener's 18–19 2NT rebid

mod common;
use common::*;
use pons::bidding::agreements::Agreements;

const P: Call = Call::Pass;

// --- 2NT opening response structure -----------------------------------------

#[test]
fn test_2nt_transfer_to_spades() {
    // Five spades → 3♥ (transfer to spades) over 2NT.
    let system = partnership();
    let auction = &[call(2, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "KJ542.Q32.943.92"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn test_2nt_stayman() {
    // 9 HCP, 4-4 majors → 3♣ (Stayman) over 2NT.
    let system = partnership();
    let auction = &[call(2, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "KJ54.Q932.K43.83"),
        call(3, Strain::Clubs),
    );
}

// --- System on after 2♣ - 2♦ - 2NT -----------------------------------------

#[test]
fn test_system_on_after_2c_2d_2nt() {
    // `2♣ - 2♦ - 2NT -`: five spades → 3♥ (transfer), system on.
    let system = partnership();
    let auction = &[
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        P,
        call(2, Strain::Notrump),
        P,
    ][..];
    assert_eq!(
        best_call(&system, auction, "KJ542.Q32.943.92"),
        call(3, Strain::Hearts),
    );
}

// --- Quantitative 4NT over 1NT opening --------------------------------------

#[test]
fn test_1nt_quantitative_4nt() {
    // 17 HCP balanced, no four-card major → 4NT (quantitative) over 1NT.
    let system = partnership();
    let auction = &[call(1, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "KQ5.AQ3.KJ42.J92"),
        call(4, Strain::Notrump),
    );
}

// --- Acceptance of quantitative 4NT over 1NT --------------------------------

#[test]
fn test_1nt_accept_quantitative_4nt() {
    // 17 HCP → accept with 6NT (maximum 15–17 range).
    let system = partnership();
    let auction = &[call(1, Strain::Notrump), P, call(4, Strain::Notrump), P][..];
    assert_eq!(
        best_call(&system, auction, "AQ32.KQ5.KJ4.Q92"),
        call(6, Strain::Notrump),
    );
}

#[test]
fn test_1nt_decline_quantitative_4nt() {
    // 15 HCP → decline (pass) the quantitative 4NT invite.
    let system = partnership();
    let auction = &[call(1, Strain::Notrump), P, call(4, Strain::Notrump), P][..];
    assert_eq!(best_call(&system, auction, "AQ32.KQ5.QJ4.J92"), Call::Pass,);
}

// --- Simple continuations after 18–19 2NT rebid ----------------------------

#[test]
fn test_rebid_2nt_response_3nt() {
    // 1♥ - 1♠ - 2NT: 10 HCP → bid 3NT opposite opener's 18–19.
    let system = partnership();
    let auction = &[
        call(1, Strain::Hearts),
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Notrump),
        P,
    ][..];
    assert_eq!(
        best_call(&system, auction, "KQ32.J5.A964.982"),
        call(3, Strain::Notrump),
    );
}

// --- notrump.quantitative_thirty_two -----------------------------------------

#[test]
fn quantitative_thirty_two() {
    let system_with = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.notrump.quantitative_thirty_two = on;
        american(&agreements).bind()
    };
    let (on, off) = (system_with(true), system_with(false));
    let six_nt = call(6, Strain::Notrump);
    let four_nt = call(4, Strain::Notrump);
    let one_nt = [call(1, Strain::Notrump), P];
    let both = |auction: &[Call], hand: &str| {
        (
            best_call(&off, auction, hand),
            best_call(&on, auction, hand),
        )
    };

    // Direct: 17 is a certain 32, 16 is the whole invite, 19 no longer stops in 3NT.
    assert_eq!(both(&one_nt, "AK3.K52.AJ42.Q72"), (four_nt, six_nt));
    assert_eq!(both(&one_nt, "AK3.K52.AJ42.J72"), (four_nt, four_nt));
    assert_eq!(
        both(&one_nt, "AK3.K52.AQ42.K72"),
        (call(3, Strain::Notrump), six_nt),
    );
    // Opener accepts the invite on 16, still declines on 15.
    let invited = [one_nt[0], P, four_nt, P];
    assert_eq!(both(&invited, "A642.AT4.KT.KQ98"), (P, six_nt));
    assert_eq!(both(&invited, "K43.A3.J942.AK32"), (P, P));
    // After Stayman finds no fit.
    let denied = [
        one_nt[0],
        P,
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        P,
    ];
    assert_eq!(both(&denied, "AK9.AT92.AJ75.J2"), (four_nt, six_nt));
    // The five-card major after the transfer: 17 bids the slam, opener accepts
    // a 16-count invite on 16 in the known fit.
    let transferred = [
        one_nt[0],
        P,
        call(2, Strain::Hearts),
        P,
        call(2, Strain::Spades),
        P,
    ];
    assert_eq!(both(&transferred, "AJ842.AK9.KQ6.94"), (four_nt, six_nt));
    assert_eq!(both(&transferred, "AK932.QT9.A83.KT"), (four_nt, four_nt));
    let invited = [&transferred[..], &[four_nt, P]].concat();
    assert_eq!(
        both(&invited, "T86.AKJ42.K62.AJ"),
        (P, call(6, Strain::Spades)),
    );
    // 2NT: 12 facing 20–21 is a certain 32.
    let two_nt = [call(2, Strain::Notrump), P];
    assert_eq!(both(&two_nt, "K7.Q63.A842.K976"), (four_nt, six_nt));
    assert_eq!(both(&two_nt, "K7.Q63.A842.Q976"), (four_nt, four_nt));
    // The 18–19 rebid: the 12-count invite is gone, 13 still invites.
    let rebid = [
        call(1, Strain::Diamonds),
        P,
        call(1, Strain::Hearts),
        P,
        call(2, Strain::Notrump),
        P,
    ];
    assert_eq!(
        both(&rebid, "A6.T732.K7.KQ753"),
        (four_nt, call(3, Strain::Notrump)),
    );
    assert_eq!(both(&rebid, "965.QJ64.K62.AK5"), (four_nt, four_nt));
}
