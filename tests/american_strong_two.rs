//! Integration tests for the strong 2♣ opening structure

mod common;
use common::*;

// --- Responses to 2♣ --------------------------------------------------------

/// At `2♣ -`: the right response for various hand types
#[test]
fn test_responses_to_two_clubs() {
    let system = partnership();
    let auction = &[call(2, Strain::Clubs), Call::Pass][..];

    // 1 HCP — waiting 2♦: no double negative since `strong_two_waiting`
    // shipped on (2026-10-01).
    assert_eq!(
        best_call(&system, auction, "98532.J76.872.92"),
        call(2, Strain::Diamonds),
    );
    // 10 HCP, five hearts to AKJ — natural positive 2♥.
    assert_eq!(
        best_call(&system, auction, "87.AKJ85.762.932"),
        call(2, Strain::Hearts),
    );
    // 6 HCP — waiting 2♦ (not strong enough for a positive).
    assert_eq!(
        best_call(&system, auction, "Q543.K76.872.J92"),
        call(2, Strain::Diamonds),
    );
    // 10 HCP, five spades to AQJ — natural positive 2♠.
    assert_eq!(
        best_call(&system, auction, "AQJ85.K76.87.932"),
        call(2, Strain::Spades),
    );
    // 11 HCP balanced — 2NT positive.
    assert_eq!(
        best_call(&system, auction, "QJ54.K76.A87.J92"),
        call(2, Strain::Notrump),
    );
}

// --- Opener's rebid after 2♦ waiting ----------------------------------------

/// At `2♣ - 2♦ -`: opener rebids shape or notrump range
#[test]
fn test_opener_rebid_after_waiting() {
    let system = partnership();
    let auction = &[
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ][..];

    // 24 HCP balanced → 2NT.
    assert_eq!(
        best_call(&system, auction, "AKQ2.KQJ.KQ4.A32"),
        call(2, Strain::Notrump),
    );
    // 24 HCP, five hearts → 2♥.
    assert_eq!(
        best_call(&system, auction, "AK2.AKQJ5.A4.K32"),
        call(2, Strain::Hearts),
    );
}

// --- Responder after opener's suit rebid (waiting sequence) -----------------

/// At `2♣ - 2♦ - 2♠ -`: responder supports or retreats
#[test]
fn test_resp_after_waiting_spades() {
    let system = partnership();
    let auction = &[
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
    ][..];

    // Q543 — three spades → raise to 3♠.
    assert_eq!(
        best_call(&system, auction, "Q543.K76.872.J92"),
        call(3, Strain::Spades),
    );
    // 54 — only two spades → retreat to 2NT.
    // Hand: 54.K762.8732.J92 (ranks must be in descending order within each suit).
    assert_eq!(
        best_call(&system, auction, "54.K762.8732.J92"),
        call(2, Strain::Notrump),
    );
}

// --- Opener after the major raise --------------------------------------------

/// At `2♣ - 2♦ - 2♠ - 3♠ -`: sign off or launch RKCB
#[test]
fn test_opener_after_spades_raise() {
    let system = partnership();
    let auction = &[
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Spades),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ][..];

    // 23 HCP, 6 spades → sign off in 4♠ (28+ required for 4NT).
    assert_eq!(
        best_call(&system, auction, "AKQJ52.AK2.A4.32"),
        call(4, Strain::Spades),
    );
}

// --- After a natural positive -----------------------------------------------

/// `2♣ - 2NT - 3♥ - 4♥ - 4NT - 5♦ - 5♥`: the looser positive, opener's
/// five-card major, the raise, and RKCB finding two keycards missing
#[test]
fn test_major_fit_after_the_notrump_positive() {
    let system = partnership();
    let (opener, responder) = ("AQ.AKJ72.K42.KQ8", "K63.Q94.J92.KT64");
    let pass = Call::Pass;
    let mut auction = vec![call(2, Strain::Clubs), pass];
    for (hand, expected) in [
        (responder, call(2, Strain::Notrump)),
        (opener, call(3, Strain::Hearts)),
        (responder, call(4, Strain::Hearts)),
        (opener, call(4, Strain::Notrump)),
        (responder, call(5, Strain::Diamonds)),
        (opener, call(5, Strain::Hearts)),
        (responder, pass),
    ] {
        assert_eq!(best_call(&system, &auction, hand), expected, "{auction:?}");
        auction.extend([expected, pass]);
    }
}

/// A one-honor five-card suit on 8 HCP is a positive, and a 7-count balanced
/// hand bids `2NT`
#[test]
fn test_loose_positive() {
    let system = partnership();
    let auction = &[call(2, Strain::Clubs), Call::Pass][..];
    assert_eq!(
        best_call(&system, auction, "KT842.Q3.J432.Q2"),
        call(2, Strain::Spades),
    );
    assert_eq!(
        best_call(&system, auction, "K84.Q93.J432.J32"),
        call(2, Strain::Notrump),
    );
}

// --- The grand rung ---------------------------------------------------------

/// The 2/1 pair with `rebid.strong_two_grand` off: the classic king ask
fn classic() -> Partnership {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.rebid.strong_two_grand = false;
    american(&agreements).bind()
}

/// Bid `opener` and `responder` out from `2♣ -`, asserting each call
fn bid_out(system: &Partnership, opener: &str, responder: &str, calls: &[Call]) {
    let mut auction = vec![call(2, Strain::Clubs), Call::Pass];
    for (index, &expected) in calls.iter().enumerate() {
        let hand = if index % 2 == 0 { responder } else { opener };
        assert_eq!(best_call(system, &auction, hand), expected, "{auction:?}");
        auction.extend([expected, Call::Pass]);
    }
}

/// `2♣ - 2♠ - 4NT - 5♣ - 5NT - 6♦ - 7♠`: all five keycards, the queen, and
/// a side king each — the classic ladder stops in `6♠` for want of the third
#[test]
fn test_grand_rung_on_two_side_kings() {
    let (opener, responder) = ("KQ3.AQ963.AK.AJ3", "AJT864.4.Q54.KT8");
    let ask = [
        call(2, Strain::Spades),
        call(4, Strain::Notrump),
        call(5, Strain::Clubs),
        call(5, Strain::Notrump),
        call(6, Strain::Diamonds),
    ];
    bid_out(
        &partnership(),
        opener,
        responder,
        &[&ask[..], &[call(7, Strain::Spades), Call::Pass]].concat(),
    );
    bid_out(
        &classic(),
        opener,
        responder,
        &[&ask[..], &[call(6, Strain::Spades)]].concat(),
    );
}

/// `2♣ - 3♦ - 4NT - 5♠ - 7♦`: the ask over a minor positive is the floor's,
/// and the grand rung answers it
#[test]
fn test_grand_rung_over_a_minor_positive() {
    let (opener, responder) = ("AKJ.AK.A63.KJ763", "642.QJ73.KQT82.A");
    let ask = [
        call(3, Strain::Diamonds),
        call(4, Strain::Notrump),
        call(5, Strain::Spades),
    ];
    bid_out(
        &partnership(),
        opener,
        responder,
        &[&ask[..], &[call(7, Strain::Diamonds), Call::Pass]].concat(),
    );
    bid_out(
        &classic(),
        opener,
        responder,
        &[&ask[..], &[call(6, Strain::Diamonds)]].concat(),
    );
}
