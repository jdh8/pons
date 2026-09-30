//! Integration tests for splinter raises, inverted minor raises, and weak jump shifts

mod common;
use common::*;

// --- Splinters over 1♠ ------------------------------------------------------

#[test]
fn test_splinter_over_one_spade() {
    let system = partnership();
    let after_1s = &[call(1, Strain::Spades), Call::Pass][..];

    // 10 HCP, singleton heart, four-card spade support -> support points 12 (the
    // singleton adds 2 opposite the fit), still below the 13 Jacoby crossover, so
    // it splinters -> 4♥.
    assert_eq!(
        best_call(&system, after_1s, "KQ52.8.A9762.J43"),
        call(4, Strain::Hearts),
    );
    // 14 HCP, singleton heart, four-card support -> Jacoby 2NT outranks splinter at 13+.
    assert_eq!(
        best_call(&system, after_1s, "KQ52.8.AK762.Q43"),
        call(2, Strain::Notrump),
    );
}

// --- Weak jump shift over 1♥ -------------------------------------------------

#[test]
fn test_wjs_over_one_heart() {
    let system = partnership();
    let after_1h = &[call(1, Strain::Hearts), Call::Pass][..];

    // 3 HCP, six spades -> 2♠ weak jump shift.
    assert_eq!(
        best_call(&system, after_1h, "QJ8632.85.972.92"),
        call(2, Strain::Spades),
    );
}

// --- Inverted minor raises over 1♦ ------------------------------------------

#[test]
fn test_inverted_minor_raises_over_one_diamond() {
    let system = partnership();
    let after_1d = &[call(1, Strain::Diamonds), Call::Pass][..];

    // 12 HCP, five diamonds, no major -> inverted strong raise (2♦ forcing).
    assert_eq!(
        best_call(&system, after_1d, "A32.K53.KQ942.32"),
        call(2, Strain::Diamonds),
    );
    // 8 HCP, five diamonds -> inverted weak preemptive raise (3♦).
    assert_eq!(
        best_call(&system, after_1d, "T32.J53.KQ942.Q2"),
        call(3, Strain::Diamonds),
    );
}

// --- Opener's rebid after inverted raise -------------------------------------

#[test]
fn test_opener_rebid_after_inverted_raise() {
    let system = partnership();
    let after_inv_raise = &[
        call(1, Strain::Diamonds),
        Call::Pass,
        call(2, Strain::Diamonds),
        Call::Pass,
    ][..];

    // 14 HCP balanced -> 2NT (opener shows 12–14 balanced).
    assert_eq!(
        best_call(&system, after_inv_raise, "Q32.A53.AJ42.K92"),
        call(2, Strain::Notrump),
    );
}

// --- Reverse Drury (`response.drury`, shipped default-on 2026-10-01) ----------

/// The shipped partnership — Drury on
fn drury_partnership() -> Partnership {
    partnership()
}

/// The control arm: Drury withheld
fn no_drury_partnership() -> Partnership {
    let mut agreements = pons::bidding::agreements::Agreements::default();
    agreements.response.drury = false;
    american(&agreements).bind()
}

/// Both hands bid out from `auction`, opener first, until one passes
fn play_out(system: &Partnership, mut auction: Vec<Call>, hands: [&str; 2]) -> Vec<Call> {
    for turn in 0.. {
        let call = best_call(system, &auction, hands[turn % 2]);
        auction.push(call);
        auction.push(Call::Pass);
        if call == Call::Pass {
            break;
        }
    }
    auction
}

/// A passed hand's three-card limit raise goes through `2♣!` in third and
/// fourth seat only; the same hand unpassed has no Drury, and the control
/// arm never bids it.
#[test]
fn drury_is_a_passed_hands_limit_raise() {
    let system = drury_partnership();
    const P: Call = Call::Pass;
    let third = [P, P, call(1, Strain::Spades), P];
    let fourth = [P, P, P, call(1, Strain::Spades), P];
    let first = [call(1, Strain::Spades), P];
    // K84.Q73.KJ62.Q53: a flat 11 with three spades — below the 2/1 fit
    // leg (13 support points) and the choice-of-games `3NT` (12 HCP), so
    // unpassed it is the forcing `1NT`.
    let raise = "K84.Q73.KJ62.Q53";
    assert_eq!(best_call(&system, &third, raise), call(2, Strain::Clubs));
    assert_eq!(best_call(&system, &fourth, raise), call(2, Strain::Clubs));
    assert_ne!(best_call(&system, &first, raise), call(2, Strain::Clubs));
    assert_ne!(
        best_call(&no_drury_partnership(), &third, raise),
        call(2, Strain::Clubs)
    );
    // Four trumps and 11 support points go the same way — the `3♠` limit
    // raise is dead for a passed hand (13 with four trumps is still Jacoby).
    assert_eq!(
        best_call(&system, &third, "K842.Q73.KJ62.J5"),
        call(2, Strain::Clubs)
    );
    assert_eq!(
        best_call(&no_drury_partnership(), &third, "K842.Q73.KJ62.J5"),
        call(3, Strain::Spades)
    );
    // A single raise stays a single raise.
    assert_eq!(
        best_call(&system, &third, "K84.Q73.J962.J53"),
        call(2, Strain::Spades)
    );
}

/// The whole lane through the real stance: the relay to game, the relay
/// signed off, and the minimum re-raised.
#[test]
fn drury_lane_plays_out() {
    let system = drury_partnership();
    const P: Call = Call::Pass;
    let start = || vec![P, P, call(1, Strain::Spades), P, call(2, Strain::Clubs), P];
    // 14 opposite 12: 2♦! then 4♠.
    assert_eq!(
        play_out(&system, start(), ["AQ752.K63.Q84.K7", "K84.Q73.KJ62.A53"]),
        [
            start(),
            vec![
                call(2, Strain::Diamonds),
                P,
                call(4, Strain::Spades),
                P,
                P,
                P
            ]
        ]
        .concat()
    );
    // 14 opposite 10: 2♦! then 2♠, passed.
    assert_eq!(
        play_out(&system, start(), ["AQ752.K63.Q84.K7", "K84.Q73.KJ62.J53"]),
        [
            start(),
            vec![
                call(2, Strain::Diamonds),
                P,
                call(2, Strain::Spades),
                P,
                P,
                P
            ]
        ]
        .concat()
    );
    // A flat 11 opposite 12: 2♠, re-raised to 3♠, declined.
    assert_eq!(
        play_out(&system, start(), ["KQ752.A63.J84.J72", "K84.Q73.KJ62.A53"]),
        [
            start(),
            vec![call(2, Strain::Spades), P, call(3, Strain::Spades), P, P, P]
        ]
        .concat()
    );
}

/// The contested tails: their double is systems on, their overcall gets
/// opener's natural ladder.
#[test]
fn drury_contested_tails() {
    let system = drury_partnership();
    const P: Call = Call::Pass;
    let doubled = [
        P,
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Clubs),
        Call::Double,
    ];
    assert_eq!(
        best_call(&system, &doubled, "AQ752.K63.Q84.K7"),
        call(2, Strain::Diamonds)
    );
    assert_eq!(
        best_call(&system, &doubled, "KQ752.A63.J84.J72"),
        call(2, Strain::Spades)
    );
    // A passed hand's `2♣` over their takeout double is the natural club bid,
    // not Drury: the systems-on strip declines it, so opener's later call is
    // the floor's, knob on or off.
    for their in [call(2, Strain::Diamonds), Call::Double] {
        let over_double = [
            P,
            P,
            call(1, Strain::Spades),
            Call::Double,
            call(2, Strain::Clubs),
            their,
        ];
        for opener in ["AT532.J973.AQ.K7", "AQJ94.53.84.KT53"] {
            assert_eq!(
                best_call(&system, &over_double, opener),
                best_call(&no_drury_partnership(), &over_double, opener),
                "{their:?} {opener}"
            );
        }
    }
    // Their double of the 2♦! relay: systems on, responder bids game or signs
    // off instead of passing the doubled relay.
    let doubled_relay = [
        P,
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        Call::Double,
    ];
    assert_eq!(
        best_call(&system, &doubled_relay, "K84.Q73.KJ62.A53"),
        call(4, Strain::Spades)
    );
    assert_eq!(
        best_call(&system, &doubled_relay, "K84.Q73.KJ62.J53"),
        call(2, Strain::Spades)
    );
    let overcalled = [
        P,
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Clubs),
        call(2, Strain::Diamonds),
    ];
    assert_eq!(
        best_call(&system, &overcalled, "KQ752.A63.J84.J72"),
        call(2, Strain::Spades)
    );
    assert_eq!(
        best_call(&system, &overcalled, "AQ752.K63.Q84.K7"),
        call(3, Strain::Spades)
    );
    assert_eq!(
        best_call(&system, &overcalled, "AKJ52.K63.A84.K7"),
        call(4, Strain::Spades)
    );
}
