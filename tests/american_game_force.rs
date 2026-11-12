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

// --- 6♠4♥ shows the hearts first (`two_over_one_six_four_hearts`) ----------

/// On, a minimum 6♠4♥ rebids `2♥` over `1♠ - 2m` and bids `4♠` over
/// responder's `3NT`; off, it rebids `2♠`.  6♠3♥ keeps `2♠` either way.
#[test]
fn six_four_hearts_shows_hearts_then_spades() {
    use pons::bidding::agreements::Agreements;

    let arm = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.rebid.two_over_one_six_four_hearts = on;
        american(&agreements).bind()
    };
    let (off, on) = (arm(false), arm(true));
    let p = Call::Pass;
    let six_four = "AQJ962.KJ74.T.54"; // 6=4=1=2, 11 HCP
    for minor in [Strain::Clubs, Strain::Diamonds] {
        let auction = [call(1, Strain::Spades), p, call(2, minor), p];
        assert_eq!(best_call(&off, &auction, six_four), call(2, Strain::Spades));
        assert_eq!(best_call(&on, &auction, six_four), call(2, Strain::Hearts));
        assert_eq!(
            best_call(&on, &auction, "AQJ962.KJ7.T4.54"),
            call(2, Strain::Spades),
        );
        let notrump = [
            auction.as_slice(),
            &[call(2, Strain::Hearts), p, call(3, Strain::Notrump), p],
        ]
        .concat();
        assert_eq!(best_call(&on, &notrump, six_four), call(4, Strain::Spades));
    }
}

// --- Responder's quantitative 4NT (rebid.two_over_one_quantitative) --------

/// Deals from the `7f3d57bd` anchor: responder with 18+ HCP and no fit
/// bid the uncapped `3NT` and a 31+ HCP slam was passed out.  On, `3NT`
/// keeps 13–17, `4NT` takes 18–20, `6NT` 21+; opener accepts `4NT` on 13+.
#[test]
fn quantitative_four_notrump_after_two_over_one() {
    use pons::bidding::agreements::Agreements;

    let arm = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.rebid.two_over_one_quantitative = on;
        american(&agreements).bind()
    };
    let (off, on) = (arm(false), arm(true));
    let p = Call::Pass;

    // 1♠ - 2♣ - 3♣ -: responder K6.KQ.A986.AKJ94, 20 HCP, two spades.
    let auction = [
        call(1, Strain::Spades),
        p,
        call(2, Strain::Clubs),
        p,
        call(3, Strain::Clubs),
        p,
    ];
    let twenty = "K6.KQ.A986.AKJ94";
    assert_eq!(best_call(&off, &auction, twenty), call(3, Strain::Notrump));
    assert_eq!(best_call(&on, &auction, twenty), call(4, Strain::Notrump));
    // 15 HCP still bids 3NT; 22 HCP blasts 6NT.
    assert_eq!(
        best_call(&on, &auction, "K6.KQ.J986.AQJ94"),
        call(3, Strain::Notrump),
    );
    assert_eq!(
        best_call(&on, &auction, "K6.AKQ.A986.AKJ94"),
        call(6, Strain::Notrump),
    );

    // Opener's answer: AQ985.AJ3.K.Q832 (16 HCP, five spades) → 6NT;
    // a minimum passes; six spades and 13+ → 6♠.
    let answer = [auction.as_slice(), &[call(4, Strain::Notrump), p]].concat();
    assert_eq!(
        best_call(&on, &answer, "AQ985.AJ3.K.Q832"),
        call(6, Strain::Notrump),
    );
    assert_eq!(best_call(&on, &answer, "AQ985.J73.K4.Q832"), p);
    assert_eq!(
        best_call(&on, &answer, "AQ9854.AJ3.K.Q83"),
        call(6, Strain::Spades),
    );

    // 1♦ - 2♣ - 2NT -: responder AK9.A.KQT3.AK984 (23 HCP) → 6NT; a 19-count
    // asks with 4NT and opener's 13+ accepts.
    let minor = [
        call(1, Strain::Diamonds),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Notrump),
        p,
    ];
    assert_eq!(
        best_call(&on, &minor, "AK9.A.KQT3.AK984"),
        call(6, Strain::Notrump),
    );
    assert_eq!(
        best_call(&on, &minor, "K98.A4.KQT3.AK984"),
        call(4, Strain::Notrump),
    );
    assert_eq!(
        best_call(&off, &minor, "K98.A4.KQT3.AK984"),
        call(3, Strain::Notrump),
    );
    let minor_answer = [minor.as_slice(), &[call(4, Strain::Notrump), p]].concat();
    assert_eq!(
        best_call(&on, &minor_answer, "Q7.KQT5.AJ64.Q62"),
        call(6, Strain::Notrump),
    );
}

// --- Opener's invite over the capped 3NT (rebid.two_over_one_opener_invite) ---

/// Off, opener's 16–18 passes the 13–17 `3NT`.  On, 18+ bids `6NT`, 16–17
/// invites with `4NT`, and responder accepts on 15+.
#[test]
fn opener_invites_over_capped_three_notrump() {
    use pons::bidding::agreements::Agreements;

    let arm = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.rebid.two_over_one_opener_invite = on;
        american(&agreements).bind()
    };
    let (off, on) = (arm(false), arm(true));
    let p = Call::Pass;

    // 1♠ - 2♣ - 2♦ - 3NT -: the run's worst board, opener AQJ97.K6.KT82.KT
    // (17 HCP) facing K4.AJ9.A76.A8762 (16).
    let auction = [
        call(1, Strain::Spades),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Diamonds),
        p,
        call(3, Strain::Notrump),
        p,
    ];
    let seventeen = "AQJ97.K6.KT82.KT";
    assert_eq!(best_call(&off, &auction, seventeen), p);
    assert_eq!(
        best_call(&on, &auction, seventeen),
        call(4, Strain::Notrump)
    );
    // 19 HCP blasts; a minimum still passes.
    assert_eq!(
        best_call(&on, &auction, "AQJ97.K6.AK82.KT"),
        call(6, Strain::Notrump),
    );
    assert_eq!(best_call(&on, &auction, "AQJ97.86.KT82.K7"), p);

    // Responder's answer: 16 accepts, 13 passes.
    let answer = [auction.as_slice(), &[call(4, Strain::Notrump), p]].concat();
    assert_eq!(
        best_call(&on, &answer, "K4.AJ9.A76.A8762"),
        call(6, Strain::Notrump),
    );
    assert_eq!(best_call(&on, &answer, "K4.J93.A76.A8762"), p);

    // 1♠ - 2♣ - 2♥ - 3NT -: the six-four `4♠` table keeps its node.
    let hearts = [
        call(1, Strain::Spades),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Hearts),
        p,
        call(3, Strain::Notrump),
        p,
    ];
    assert_eq!(
        best_call(&on, &hearts, "AQ9875.KJ73.4.Q8"),
        call(4, Strain::Spades),
    );
    assert_eq!(
        best_call(&on, &hearts, "AQJ97.AKJ3.K4.Q8"),
        call(6, Strain::Notrump),
    );

    // 1♦ - 2♣ - 2♦ - 3NT -: the minor lane, 16 HCP invites.
    let minor = [
        call(1, Strain::Diamonds),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Diamonds),
        p,
        call(3, Strain::Notrump),
        p,
    ];
    assert_eq!(
        best_call(&on, &minor, "K7.A85.AKJ964.J2"),
        call(4, Strain::Notrump),
    );
}

// --- Lowered bands after a reverse (rebid.two_over_one_reverse_quantitative) ---

/// Off, responder's bands are flat: 16–17 bids `3NT` over the reverse.  On,
/// 16 invites with `4NT`, 17 bids `6NT`, and opener accepts on 15+.
#[test]
fn reverse_lowers_quantitative_bands() {
    use pons::bidding::agreements::Agreements;

    let arm = |on: bool| {
        let mut agreements = Agreements::default();
        agreements.rebid.two_over_one_reverse_quantitative = on;
        american(&agreements).bind()
    };
    let (off, on) = (arm(false), arm(true));
    let p = Call::Pass;
    let (three, four, six) = (
        call(3, Strain::Notrump),
        call(4, Strain::Notrump),
        call(6, Strain::Notrump),
    );

    // 1♥ - 2♣ - 2♠ -: responder's 15, 16 and 17.
    let reverse = [
        call(1, Strain::Hearts),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Spades),
        p,
    ];
    for (hand, flat, lowered) in [
        ("K4.J9.AQ76.AJ762", three, three),
        ("K4.Q9.AQ76.AJ762", three, four),
        ("K4.K9.AQ76.AJ762", three, six),
    ] {
        assert_eq!(best_call(&off, &reverse, hand), flat, "{hand}");
        assert_eq!(best_call(&on, &reverse, hand), lowered, "{hand}");
    }

    // Opener's answer: 14 passes the 16-count's `4NT`, 15 accepts.
    let answer = [reverse.as_slice(), &[four, p]].concat();
    assert_eq!(best_call(&off, &answer, "AQJ8.KQT52.7.Q54"), six);
    assert_eq!(best_call(&on, &answer, "AQJ8.KQT52.7.Q54"), p);
    assert_eq!(best_call(&on, &answer, "AQJ8.KQT52.7.K54"), six);

    // 1♥ - 2♣ - 2♦ -: no reverse, so the flat bands stay.
    let plain = [
        call(1, Strain::Hearts),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Diamonds),
        p,
    ];
    assert_eq!(best_call(&on, &plain, "KJ4.K9.A76.AJ762"), three);

    // 1♦ - 2♣ - 2♥ -: the minor lane's reverse.
    let minor = [
        call(1, Strain::Diamonds),
        p,
        call(2, Strain::Clubs),
        p,
        call(2, Strain::Hearts),
        p,
    ];
    assert_eq!(best_call(&off, &minor, "KQ4.J9.K76.AKJ62"), three);
    assert_eq!(best_call(&on, &minor, "KQ4.J9.K76.AKJ62"), six);
}
