//! Integration tests for the major-suit continuation knobs: opener's game
//! tries after a single raise (`set_major_game_tries`), opener's acceptance
//! ladder after a limit raise (`set_limit_raise_acceptance`), the full
//! continuations after `1♥–1♠` (`set_major_rebid_tails`), and fourth-suit-
//! forcing riding that adjunct (`set_fourth_suit_forcing`).  Each test builds
//! its own stance with the knobs it needs and restores the defaults, so the
//! rest of the suite keeps measuring the shipped system.
//!
//! Every test plays a *whole* auction through the real stance — the opening
//! bid through the final contract — rather than probing a bare rule table in
//! isolation, so a hand's hcp/hand-shape choices are checked against every
//! node they pass through, not just the one under test.

mod common;
use common::*;

use pons::bidding::american::{
    set_fourth_suit_forcing, set_limit_raise_acceptance, set_major_game_tries,
    set_major_rebid_tails,
};

const P: Call = Call::Pass;

/// A stance built with the given knobs, the (on) defaults restored afterwards
fn stance_with(tries: bool, limit: bool, tails: bool, fsf: bool) -> Stance {
    set_major_game_tries(tries);
    set_limit_raise_acceptance(limit);
    set_major_rebid_tails(tails);
    set_fourth_suit_forcing(fsf);
    let stance = american().against(Family::NATURAL);
    set_major_game_tries(true);
    set_limit_raise_acceptance(true);
    set_major_rebid_tails(true);
    set_fourth_suit_forcing(true);
    stance
}

/// Append our call and the opponent's pass — the uncontested interleaving
/// every constructive-book auction uses
fn extend(auction: &[Call], next: Call) -> Vec<Call> {
    let mut auction = auction.to_vec();
    auction.push(next);
    auction.push(P);
    auction
}

// =============================================================================
// Major game tries: 1M – (P) – 2M – (P) (set_major_game_tries)
// =============================================================================

#[test]
fn game_try_reaches_game_on_help() {
    let system = stance_with(true, false, false, false);

    // Kx.AKxxx.xx.AQJx: 17 HCP + 1 (unbalanced 5=2=2=4) = 18 points, 5
    // hearts, 4 clubs — a maximum single-raise try (16–18), short of the
    // non-asking maximum (19+).
    let opener = "Kx.AKxxx.xx.AQJx";
    // Qxx.Kxx.Jxxxxx.x: 6 HCP + 1 (unbalanced, singleton club) = 7 points,
    // 3-card heart support, singleton club.
    let responder = "Qxx.Kxx.Jxxxxx.x";

    assert_eq!(
        best_call(&system, &[], opener),
        call(1, Strain::Hearts),
        "18 points, 5=2=2=4 opens 1♥"
    );
    let auction = extend(&[], call(1, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(2, Strain::Hearts),
        "7 points, 3-card support -> single raise"
    );
    let auction = extend(&auction, call(2, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(3, Strain::Clubs),
        "18 points, 4 clubs -> the club game try beats the general re-raise"
    );
    let auction = extend(&auction, call(3, Strain::Clubs));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(4, Strain::Hearts),
        "a singleton in the tried suit accepts regardless of points"
    );
}

#[test]
fn game_try_declined_stops_in_three() {
    let system = stance_with(true, false, false, false);

    // Kx.AKxxx.xx.AQxx: 16 HCP + 1 (unbalanced 5=2=2=4) = 17 points, 4
    // clubs — enough to try, short of the opener_after_decline push (18+).
    let opener = "Kx.AKxxx.xx.AQxx";
    // xxx.Kxx.QJxx.xxx: 6 HCP, balanced (4=3=3=3) = 6 points — a minimum
    // raise with no shortness or top-honor quality in the tried suit.
    let responder = "xxx.Kxx.QJxx.xxx";

    assert_eq!(
        best_call(&system, &[], opener),
        call(1, Strain::Hearts),
        "17 points, 5=2=2=4 opens 1♥"
    );
    let auction = extend(&[], call(1, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(2, Strain::Hearts),
        "6 points, 3-card support -> single raise"
    );
    let auction = extend(&auction, call(2, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(3, Strain::Clubs),
        "17 points, 4 clubs -> club try"
    );
    let auction = extend(&auction, call(3, Strain::Clubs));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(3, Strain::Hearts),
        "a wasted minimum (no shortness, no top honors) declines"
    );
    let auction = extend(&auction, call(3, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, opener),
        Call::Pass,
        "17 points is below the 18-point push-on threshold"
    );
}

#[test]
fn single_raise_passed_without_extras() {
    let system = stance_with(true, false, false, false);

    // KQx.AJxxx.Kxx.xx: 13 HCP, balanced (5=3=3=2) = 13 points — a flat
    // minimum with no side suit long enough for any try.
    let opener = "KQx.AJxxx.Kxx.xx";
    // Kxx.Jxx.Qxxxx.xx: 6 HCP, balanced (5=3=3=2) = 6 points, 3-card
    // heart support.
    let responder = "Kxx.Jxx.Qxxxx.xx";

    assert_eq!(
        best_call(&system, &[], opener),
        call(1, Strain::Hearts),
        "13 points, 5=3=3=2 opens 1♥"
    );
    let auction = extend(&[], call(1, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(2, Strain::Hearts),
        "6 points, 3-card support -> single raise"
    );
    let auction = extend(&auction, call(2, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, opener),
        Call::Pass,
        "a flat 13 has no 4-card side suit and is below every try's 16-point \
         floor — the authored Pass fires here, not the (absent) floor"
    );
}

// =============================================================================
// Limit-raise acceptance: 1M – (P) – 3M – (P) (set_limit_raise_acceptance)
// =============================================================================

#[test]
fn limit_raise_accepted_and_declined() {
    let system = stance_with(false, true, false, false);

    // AKxxx.Kxx.Qxx.xx: 12 HCP, 5=3=3=2.  Opposite the known 9-card fit the
    // small club doubleton is a ruffing value, so support points read 13 (12 +
    // the working doubleton) — right at the acceptance floor, so it accepts.
    let opener_accept = "AKxxx.Kxx.Qxx.xx";
    // AKxxx.Kxx.xxx.Qx: also 12 HCP, but the ♣Q sits *in* the doubleton — a
    // wasted honor with no ruffing value, so support points stay 12, below the
    // 13 floor, and it declines.  The two 12-counts differ only in where the
    // shortness honor sits: that is exactly what support points price.
    let opener_decline = "AKxxx.Kxx.xxx.Qx";
    // Kxxx.Axx.Qxx.Jxx: 10 HCP, flat 4=3=3=3, 4-card spade support, no shortness
    // anywhere — a clean 10-point limit raise (support points 10 == HCP).
    let responder = "Kxxx.Axx.Qxx.Jxx";

    for opener in [opener_accept, opener_decline] {
        assert_eq!(
            best_call(&system, &[], opener),
            call(1, Strain::Spades),
            "both opener hands open 1♠"
        );
    }
    let auction = extend(&[], call(1, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(3, Strain::Spades),
        "10 points, 4-card support -> limit raise"
    );
    let auction = extend(&auction, call(3, Strain::Spades));

    assert_eq!(
        best_call(&system, &auction, opener_accept),
        call(4, Strain::Spades),
        "13 support points (12 HCP + a working doubleton) accepts to game"
    );
    assert_eq!(
        best_call(&system, &auction, opener_decline),
        Call::Pass,
        "12 support points (the doubleton honor wasted) is below the 13 floor"
    );
}

#[test]
fn limit_raise_keycard_ladder() {
    let system = stance_with(false, true, false, false);

    // AKQxx.AKx.Kxxx.x: 19 HCP + 1 (unbalanced 5=3=4=1) = 20 points, three
    // aces plus the spade king (3 keycards toward a spade trump).
    let opener = "AKQxx.AKx.Kxxx.x";
    // Kxxx.Axx.Qxx.Jxx: the same 10-point limit raise as above; toward
    // spades it holds the spade king (a keycard) and the heart ace, but not
    // the spade queen -> 2 keycards, no trump queen.
    let responder = "Kxxx.Axx.Qxx.Jxx";

    assert_eq!(
        best_call(&system, &[], opener),
        call(1, Strain::Spades),
        "20 points, 5=3=4=1 opens 1♠"
    );
    let auction = extend(&[], call(1, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(3, Strain::Spades),
        "10 points, 4-card support -> limit raise"
    );
    let auction = extend(&auction, call(3, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(4, Strain::Notrump),
        "20 points (19+) asks for keycards"
    );
    let auction = extend(&auction, call(4, Strain::Notrump));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(5, Strain::Hearts),
        "2 keycards (♠K, ♥A), no trump queen -> 5♥"
    );
    let auction = extend(&auction, call(5, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(6, Strain::Spades),
        "asker holds 3 keycards (♠A, ♥A, ♠K) -> five between the hands, bid \
         the slam"
    );
}

// =============================================================================
// Major-rebid tails: full continuations after 1♥ – (P) – 1♠ (set_major_rebid_tails)
// =============================================================================

#[test]
fn spade_raise_invite_accepted() {
    let system = stance_with(false, false, true, false);

    // Kxxx.AKxxx.QJx.x: 13 HCP + 1 (unbalanced 4=5=3=1) = 14 points, four
    // spades — raises, then accepts the invite.
    let opener_accept = "Kxxx.AKxxx.QJx.x";
    // Kxxx.AKxxx.Qxx.x: 12 HCP + 1 (unbalanced 4=5=3=1) = 13 points, four
    // spades — raises, then declines.
    let opener_decline = "Kxxx.AKxxx.Qxx.x";
    // AJxx.xx.AQxx.xxx: 11 HCP, balanced (4=2=4=3) = 11 points, four
    // spades, fewer than four hearts.
    let responder = "AJxx.xx.AQxx.xxx";

    for opener in [opener_accept, opener_decline] {
        assert_eq!(
            best_call(&system, &[], opener),
            call(1, Strain::Hearts),
            "13/14 points, 5+ hearts opens 1♥"
        );
    }
    let auction = extend(&[], call(1, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(1, Strain::Spades),
        "11 points, 4 spades, fewer than 4 hearts -> natural 1♠"
    );
    let auction = extend(&auction, call(1, Strain::Spades));

    for opener in [opener_accept, opener_decline] {
        assert_eq!(
            best_call(&system, &auction, opener),
            call(2, Strain::Spades),
            "12/13 points, 4-card spade support -> the 12-15 raise"
        );
    }
    let auction = extend(&auction, call(2, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(3, Strain::Spades),
        "11 points -> the 10-11 invitational raise"
    );
    let auction = extend(&auction, call(3, Strain::Spades));

    assert_eq!(
        best_call(&system, &auction, opener_accept),
        call(4, Strain::Spades),
        "14 points accepts the invite to game"
    );
    assert_eq!(
        best_call(&system, &auction, opener_decline),
        Call::Pass,
        "13 points declines -> the final contract is 3♠"
    );
}

#[test]
fn heart_rebid_preference_structure() {
    let system = stance_with(false, false, true, false);

    // xx.AKQxxx.Kxx.xx: 12 HCP + 1 (unbalanced 2=6=3=2) = 13 points, six
    // hearts, fewer than four spades — a minimum that rebids hearts.
    let opener = "xx.AKQxxx.Kxx.xx";
    // KQxx.Ax.Jxx.Jxxx: 11 HCP, balanced (4=2=3=4) = 11 points, four
    // spades, exactly two hearts.
    let responder = "KQxx.Ax.Jxx.Jxxx";

    assert_eq!(
        best_call(&system, &[], opener),
        call(1, Strain::Hearts),
        "13 points, 6 hearts opens 1♥"
    );
    let auction = extend(&[], call(1, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(1, Strain::Spades),
        "11 points, 4 spades, 2 hearts -> natural 1♠"
    );
    let auction = extend(&auction, call(1, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(2, Strain::Hearts),
        "13 points, 6 hearts, fewer than 4 spades -> rebid the suit"
    );
    let auction = extend(&auction, call(2, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(3, Strain::Hearts),
        "11 points with 2-card heart support beats the 2NT notrump invite"
    );
    let auction = extend(&auction, call(3, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, opener),
        Call::Pass,
        "13 points is a minimum, below the 14-point acceptance floor -> the \
         final contract is 3♥"
    );
}

// =============================================================================
// Fourth-suit-forcing: 1♥ – (P) – 1♠ – (P) – 2♣ – (P) – 2♦ (set_fourth_suit_forcing)
// =============================================================================

#[test]
fn fourth_suit_forcing_end_to_end() {
    let system = stance_with(false, false, true, true);

    // Kxx.AQxxx.x.Kxxx: 12 HCP + 1 (unbalanced 3=5=1=4) = 13 points, three
    // spades, four clubs — a minimum-ish hand that rebids the new minor.
    let opener = "Kxx.AQxxx.x.Kxxx";
    // AKxxx.xx.Qxx.AQx: 15 HCP, balanced (5=2=3=3) = 15 points, five
    // spades, fewer than four hearts.
    let responder = "AKxxx.xx.Qxx.AQx";

    assert_eq!(
        best_call(&system, &[], opener),
        call(1, Strain::Hearts),
        "13 points, 5 hearts opens 1♥"
    );
    let auction = extend(&[], call(1, Strain::Hearts));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(1, Strain::Spades),
        "15 points, 5 spades, fewer than 4 hearts -> natural 1♠"
    );
    let auction = extend(&auction, call(1, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(2, Strain::Clubs),
        "13 points, 4 clubs, fewer than 4 diamonds -> the new-minor rebid"
    );
    let auction = extend(&auction, call(2, Strain::Clubs));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(2, Strain::Diamonds),
        "15 points (12+) -> fourth-suit-forcing beats the natural 3NT route"
    );
    let auction = extend(&auction, call(2, Strain::Diamonds));
    assert_eq!(
        best_call(&system, &auction, opener),
        call(2, Strain::Spades),
        "3-card spade support answers the fourth-suit-forcing game force"
    );
    let auction = extend(&auction, call(2, Strain::Spades));
    assert_eq!(
        best_call(&system, &auction, responder),
        call(4, Strain::Spades),
        "opener's answer showed 3-card support and responder holds 5 -> \
         place the 5-3 spade game"
    );
}

#[test]
fn fsf_without_tails_is_inert() {
    // AKxxx.xx.Qxx.AQx: the same 15-point fourth-suit-forcing candidate
    // used above.
    let responder = "AKxxx.xx.Qxx.AQx";
    let auction = [
        call(1, Strain::Hearts),
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Clubs),
        P,
    ];

    // Fourth-suit-forcing rides the major-rebid-tails adjunct: with tails
    // off, register_major_rebid_tails bails out before ever consulting the
    // fsf flag, so turning fsf on alone must not change a single call.
    let fsf_only = stance_with(false, false, false, true);
    let baseline = stance_with(false, false, false, false);

    assert_eq!(
        best_call(&fsf_only, &auction, responder),
        best_call(&baseline, &auction, responder),
        "fsf without the tails adjunct must be inert — both stances fall to \
         the same floor at 1♥-1♠-2♣"
    );
}

// =============================================================================
// Default parity: a freshly-built stance carries all four shipped-on knobs
// =============================================================================

#[test]
fn default_state_matches_all_on() {
    let all_on = stance_with(true, true, true, true);
    let fresh = american().against(Family::NATURAL);

    // The 1♥ – 2♥ opener decision (the game-tries node): reuse
    // `single_raise_passed_without_extras`'s flat 13-point opener.
    let opener = "KQx.AJxxx.Kxx.xx";
    let raise_auction = [call(1, Strain::Hearts), P, call(2, Strain::Hearts), P];
    assert_eq!(
        best_call(&all_on, &raise_auction, opener),
        best_call(&fresh, &raise_auction, opener),
        "a freshly built default stance must match stance_with(true, true, \
         true, true) at 1♥-2♥"
    );

    // The 1♥ – 1♠ – 2♣ responder decision (the fsf node): reuse the
    // fourth-suit-forcing candidate, which must now bid 2♦ by default.
    let responder = "AKxxx.xx.Qxx.AQx";
    let fsf_auction = [
        call(1, Strain::Hearts),
        P,
        call(1, Strain::Spades),
        P,
        call(2, Strain::Clubs),
        P,
    ];
    assert_eq!(
        best_call(&fresh, &fsf_auction, responder),
        call(2, Strain::Diamonds),
        "the default stance plays fourth-suit-forcing"
    );
    assert_eq!(
        best_call(&all_on, &fsf_auction, responder),
        call(2, Strain::Diamonds),
        "stance_with(true, true, true, true) agrees"
    );
}
