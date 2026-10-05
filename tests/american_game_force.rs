//! Integration tests for the 2/1 game-forcing continuations

mod common;
use common::*;

// --- Opener's rebid after 1♠ - 2♣ - ----------------------------------------

#[test]
fn opener_rebid_1s_2c_four_diamonds() {
    // 12 HCP, five spades, four diamonds → show the new suit (2♦).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ52.32.KQ54.92"),
        call(2, Strain::Diamonds),
    );
}

#[test]
fn opener_rebid_1s_2c_six_spades() {
    // 13 HCP (14 points after the clean-shape upgrade), six spades — short
    // of the 15 a jump to 3♠ would promise → rebid 2♠.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ752.A2.Q54.92"),
        call(2, Strain::Spades),
    );
}

#[test]
fn opener_rebid_1s_2c_balanced() {
    // 14 HCP, balanced → 2NT.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ52.K32.Q54.Q9"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn opener_rebid_1s_2c_club_support() {
    // 12 HCP, four-card club support → raise to 3♣.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ52.32.K5.Q942"),
        call(3, Strain::Clubs),
    );
}

// --- Responder's rebid after 1♠ - 2♣ - 2♦ - -------------------------------

#[test]
fn responder_rebid_three_spades() {
    // 17 HCP, three-card spade support → 3♠ (sets trump).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "K32.A2.Q54.AKJ92"),
        call(3, Strain::Spades),
    );
}

#[test]
fn responder_rebid_raise_diamonds() {
    // 17 HCP, four diamonds, only two spades → raise diamonds (3♦).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "Q2.A32.KQ54.AQ92"),
        call(3, Strain::Diamonds),
    );
}

// --- Opener's third call after 1♠ - 2♣ - 2♦ - 3♠ - -------------------------

#[test]
fn opener_third_sign_off() {
    // 12 HCP → sign off at 4♠.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ52.32.KQ54.92"),
        call(4, Strain::Spades),
    );
}

#[test]
fn opener_third_keycard_ask() {
    // 17 HCP → 4NT (key card ask).
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Spades),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ52.A2.KQJ4.92"),
        call(4, Strain::Notrump),
    );
}

// --- Game backstop keeps the force alive ------------------------------------

#[test]
fn second_suit_agreed_minimum_bids_3nt() {
    // `1♠ - 2♣ - 2♦ - 3♦ -`: responder agreed opener's second suit.
    // A minimum opener signs off at 3NT — NOT the game backstop's 4♠, which
    // would revert to the 5-2 spade fit after the diamond fit was found.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AQJ52.32.KQ54.92"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn second_suit_agreed_extras_asks_rkcb() {
    // Same node with extras (18 HCP): opener asks 4NT RKCB with diamonds set.
    let auction = &[
        call(1, Strain::Spades),
        Call::Pass,
        call(2, Strain::Clubs),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
        call(3, Strain::Diamonds),
        Call::Pass,
    ];
    assert_eq!(
        best_call(&partnership(), auction, "AKJ52.A2.AQ54.K2"),
        call(4, Strain::Notrump),
    );
}

// --- Opener's side suit before the jump (`two_over_one_side_suit_first`) ----

/// On, six of the major with extras shows a four-card side suit instead of
/// jumping to `3M`, and the jump reads as denying one; off, it jumps.
#[test]
fn side_suit_first_keeps_the_jump_one_suited() {
    use contract_bridge::Suit;
    use pons::bidding::agreements::Agreements;
    use pons::bidding::inference::Relative;

    let arm = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.rebid.two_over_one_side_suit_first = on;
        american(&agreements).bind()
    };
    let (off, on) = (arm(false), arm(true));
    let p = Call::Pass;
    let auction = [call(1, Strain::Hearts), p, call(2, Strain::Diamonds), p];
    let jump = call(3, Strain::Hearts);
    for (hand, now) in [
        ("KQ75.AKQJ72.84.3", call(2, Strain::Spades)), // 4=6=2=1
        ("84.AKQJ72.3.KQ75", call(3, Strain::Clubs)),  // 2=6=1=4
        ("K84.AKQJ72.A3.75", jump),                    // one-suited
    ] {
        assert_eq!(best_call(&off, &auction, hand), jump, "off: {hand}");
        assert_eq!(best_call(&on, &auction, hand), now, "on: {hand}");
    }
    // A minimum 6-4 is not the jump's hand in either arm.
    let minimum = "KQ75.KJ9872.84.3";
    assert_eq!(
        best_call(&on, &auction, minimum),
        best_call(&off, &auction, minimum),
    );

    let jumped = [auction.as_slice(), &[jump, p]].concat();
    let spades = |system: &Partnership| {
        system
            .infer(RelativeVulnerability::NONE, &jumped)
            .get(Relative::Partner)
            .length(Suit::Spades)
            .max
    };
    assert!(spades(&off) > 3, "off: the jump may hide four spades");
    assert!(spades(&on) <= 3, "on: the jump denies four spades");
}

// --- Opener's reverse promises extras (`two_over_one_reverse_extras`) -------

/// On, `1♥ - 2m - 2♠` needs extras, the minimum rebids `2NT`, and responder
/// shows four spades over it with `3♠`; off, the reverse is shape only.
#[test]
fn reverse_extras_sends_the_minimum_to_2nt() {
    use pons::bidding::agreements::Agreements;
    use pons::bidding::inference::Relative;

    let arm = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.rebid.two_over_one_reverse_extras = on;
        american(&agreements).bind()
    };
    let (off, on) = (arm(false), arm(true));
    let p = Call::Pass;
    let (reverse, notrump) = (call(2, Strain::Spades), call(2, Strain::Notrump));
    let auction = [call(1, Strain::Hearts), p, call(2, Strain::Clubs), p];
    for (hand, now) in [
        ("KQ75.AJ872.84.Q3", notrump), // 4=5=2=2, 12 HCP
        ("KQ75.AKJ72.84.K3", reverse), // 4=5=2=2, 16 HCP
    ] {
        assert_eq!(best_call(&off, &auction, hand), reverse, "off: {hand}");
        assert_eq!(best_call(&on, &auction, hand), now, "on: {hand}");
    }

    // The reverse reads as extras only when armed.
    let reversed = [auction.as_slice(), &[reverse, p]].concat();
    let floor = |system: &Partnership| {
        system
            .infer(RelativeVulnerability::NONE, &reversed)
            .get(Relative::Partner)
            .strength
            .points
            .min
    };
    assert!(floor(&off) < 15, "off: the reverse is shape only");
    assert!(floor(&on) >= 15, "on: the reverse shows extras");

    // Responder's four spades over the minimum's 2NT, and opener's answer
    // without a fit.
    let over_notrump = [auction.as_slice(), &[notrump, p]].concat();
    assert_eq!(
        best_call(&on, &over_notrump, "AJ84.3.A84.AQJ72"),
        call(3, Strain::Spades),
    );
    let shown = [over_notrump.as_slice(), &[call(3, Strain::Spades), p]].concat();
    assert_eq!(
        best_call(&on, &shown, "Q75.AJ872.K84.Q3"),
        call(3, Strain::Notrump),
    );

    // `1♦ - 2♣ - 2M` is a reverse too.
    let minor = [call(1, Strain::Diamonds), p, call(2, Strain::Clubs), p];
    let minimum = "84.KQ75.AJ872.Q3";
    assert_eq!(best_call(&off, &minor, minimum), call(2, Strain::Hearts));
    assert_eq!(best_call(&on, &minor, minimum), notrump);
}
