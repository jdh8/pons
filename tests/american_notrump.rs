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

// --- notrump.two_notrump_super_accept ----------------------------------------

#[test]
fn two_notrump_super_accept() {
    let system_with = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.notrump.two_notrump_super_accept = on;
        american(&agreements).bind()
    };
    let (on, off) = (system_with(true), system_with(false));
    let both = |auction: &[Call], hand: &str| {
        (
            best_call(&off, auction, hand),
            best_call(&on, auction, hand),
        )
    };
    let hearts = [call(2, Strain::Notrump), P, call(3, Strain::Diamonds), P];
    let spades = [call(2, Strain::Notrump), P, call(3, Strain::Hearts), P];
    // Four trumps jump to game; three complete.
    assert_eq!(
        both(&hearts, "A7.AJ42.AKQ6.KT7"),
        (call(3, Strain::Hearts), call(4, Strain::Hearts)),
    );
    assert_eq!(
        both(&spades, "A954.AK5.A8.AQT4"),
        (call(3, Strain::Spades), call(4, Strain::Spades)),
    );
    assert_eq!(
        both(&hearts, "AK7.AJ4.KQ62.KT7"),
        (call(3, Strain::Hearts), call(3, Strain::Hearts)),
    );
    // The 22–24 lane shares the structure.
    let strong = [
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        P,
        call(2, Strain::Notrump),
        P,
        call(3, Strain::Diamonds),
        P,
    ];
    assert_eq!(
        both(&strong, "AK7.AQJ4.AKQ6.K7"),
        (call(3, Strain::Hearts), call(4, Strain::Hearts)),
    );
    // Responder over the jump: a weak hand passes, 9 support points pass, 11
    // ask keycards — on shape as well as on high cards.
    let jumped = [
        call(2, Strain::Notrump),
        P,
        call(3, Strain::Diamonds),
        P,
        call(4, Strain::Hearts),
        P,
    ];
    let four_nt = call(4, Strain::Notrump);
    assert_eq!(best_call(&on, &jumped, "86.T9743.J52.943"), P);
    assert_eq!(best_call(&on, &jumped, "K6.QT743.J52.Q43"), P);
    assert_eq!(best_call(&on, &jumped, "K6.KT743.K52.Q43"), four_nt);
    assert_eq!(best_call(&on, &jumped, "T.AKQ8543.753.94"), four_nt);
    // Facing 20+, `5♣` is four keycards unless we hold two ourselves.
    let answered = [&jumped[..], &[four_nt, P, call(5, Strain::Clubs), P]].concat();
    assert_eq!(
        best_call(&on, &answered, "K73.KQJ76.74.KQ9"),
        call(6, Strain::Hearts),
    );
    assert_eq!(
        best_call(&on, &answered, "A73.KQJ76.74.Q93"),
        call(5, Strain::Hearts),
    );
}

// --- notrump.rebid_checkback_shape_keycard -----------------------------------

#[test]
fn rebid_checkback_shape_keycard() {
    let system_with = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.notrump.rebid_checkback_shape_keycard = on;
        american(&agreements).bind()
    };
    let (on, off) = (system_with(true), system_with(false));
    let both = |auction: &[Call], hand: &str| {
        (
            best_call(&off, auction, hand),
            best_call(&on, auction, hand),
        )
    };
    let four_nt = call(4, Strain::Notrump);
    let rebid = [
        call(1, Strain::Diamonds),
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Notrump),
        P,
    ];
    // The 5-3 fit: 13 HCP and a singleton asks; a flat 8-count never does.
    let fit = [
        &rebid[..],
        &[call(3, Strain::Clubs), P, call(3, Strain::Spades), P],
    ]
    .concat();
    assert_eq!(
        both(&fit, "Q9753.AQT4.KQ5.8"),
        (call(4, Strain::Spades), four_nt),
    );
    assert_eq!(
        both(&fit, "K9753.Q64.K52.82"),
        (call(4, Strain::Spades), call(4, Strain::Spades)),
    );
    // The 6-2 fit: 11 HCP and a singleton asks over the raise.
    let raised = [
        &rebid[..],
        &[call(3, Strain::Spades), P, call(4, Strain::Spades), P],
    ]
    .concat();
    assert_eq!(both(&raised, "QJT432.AT64.8.A5"), (P, four_nt));
}

// --- notrump.two_notrump_transfer_rebid --------------------------------------

#[test]
fn two_notrump_transfer_rebid() {
    let mut agreements = Agreements::default();
    agreements.notrump.two_notrump_transfer_rebid = true;
    let on = american(&agreements).bind();
    let hearts = [
        call(2, Strain::Notrump),
        P,
        call(3, Strain::Diamonds),
        P,
        call(3, Strain::Hearts),
        P,
    ];
    let three_nt = call(3, Strain::Notrump);
    // Five trumps walk the notrump ladder a point below the direct table: a
    // 9-count offers the choice of games, 10 invites (the one-point invite
    // of `quantitative_thirty_two`), 11 to 13 bid the slam.
    assert_eq!(best_call(&on, &hearts, "53.AJT86.J872.QJ"), three_nt);
    assert_eq!(
        best_call(&on, &hearts, "53.AJT86.J872.KJ"),
        call(4, Strain::Notrump),
    );
    for slam in ["53.AJT86.Q872.KJ", "K3.AJT86.K872.Q4"] {
        assert_eq!(best_call(&on, &hearts, slam), call(6, Strain::Notrump));
    }
    // Six trumps place the contract: game on a 6-count, slam on shape and 9.
    assert_eq!(
        best_call(&on, &hearts, "T8.AQ9765.T32.65"),
        call(4, Strain::Hearts),
    );
    assert_eq!(
        best_call(&on, &hearts, "K7.A98752.8.QJ74"),
        call(6, Strain::Hearts),
    );
    // A bust passes the completion; opener declines the invite on 20.
    assert_eq!(best_call(&on, &hearts, "532.T8643.872.J4"), P);
    let invited = [&hearts[..], &[call(4, Strain::Notrump), P]].concat();
    assert_eq!(best_call(&on, &invited, "KQJ2.K4.AK3.A652"), P);
    // Opener passes the sign-off.
    let signed_off = [&hearts[..], &[call(4, Strain::Hearts), P]].concat();
    assert_eq!(best_call(&on, &signed_off, "KQJ2.K4.AK3.A652"), P);
}
