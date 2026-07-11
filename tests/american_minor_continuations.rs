//! Integration tests for the minor-opening continuation knobs: the
//! longer-major response discipline (`set_longer_major_response`), the
//! up-the-line completion (`set_up_the_line`), and the XYZ two-way checkback
//! (`set_xyz`).  Each test builds its own stance with the knobs it needs and
//! restores the defaults, so the rest of the suite keeps measuring the
//! shipped system.

mod common;
use common::*;

use pons::bidding::american::{set_longer_major_response, set_up_the_line, set_xyz};

const P: Call = Call::Pass;

/// A stance built with the given knobs, defaults restored afterwards
fn stance_with(longer_major: bool, up_the_line: bool, xyz: bool) -> Stance {
    set_longer_major_response(longer_major);
    set_up_the_line(up_the_line);
    set_xyz(xyz);
    let stance = american().against(Family::NATURAL);
    set_longer_major_response(true); // restore the shipped default (longer-major is now on)
    set_up_the_line(false);
    set_xyz(false);
    stance
}

// --- Knob A: the longer-major response discipline ---------------------------

#[test]
fn longer_major_response_discipline() {
    let system = stance_with(true, false, false);
    let one_club = [call(1, Strain::Clubs), P];

    // 5♠4♥ responds 1♠ (longest first) ...
    assert_eq!(
        best_call(&system, &one_club, "AKxxx.QJxx.xx.xx"),
        call(1, Strain::Spades),
    );
    // ... 5-5 responds 1♠ (higher of two equal five-card suits) ...
    assert_eq!(
        best_call(&system, &one_club, "AQxxx.KJxxx.x.xx"),
        call(1, Strain::Spades),
    );
    // ... 4-4 responds 1♥ up the line ...
    assert_eq!(
        best_call(&system, &one_club, "AQxx.KJxx.xx.xxx"),
        call(1, Strain::Hearts),
    );
    // ... and longer hearts respond 1♥.
    assert_eq!(
        best_call(&system, &one_club, "QJxx.AKxxx.xx.xx"),
        call(1, Strain::Hearts),
    );

    // Opt-in off (`set_longer_major_response(false)`): the historic
    // unconditional hearts-first responds 1♥ on 5♠4♥ — the simplification that
    // washed against the longer-major default and stays available as a knob.
    let hearts_first = stance_with(false, false, false);
    assert_eq!(
        best_call(&hearts_first, &one_club, "AKxxx.QJxx.xx.xx"),
        call(1, Strain::Hearts),
    );
}

// --- Knob C: the up-the-line completion --------------------------------------

#[test]
fn up_the_line_completion() {
    let system = stance_with(false, true, false);
    let one_club = [call(1, Strain::Clubs), P];
    let after_one_heart = [call(1, Strain::Clubs), P, call(1, Strain::Hearts), P];
    let after_one_diamond = [call(1, Strain::Clubs), P, call(1, Strain::Diamonds), P];

    // A diamond hand without a four-card major responds a natural 1♦ ...
    assert_eq!(
        best_call(&system, &one_club, "xx.xxx.AQJxx.Kxx"),
        call(1, Strain::Diamonds),
    );
    // ... opener shows four spades over the 1♥ response ...
    assert_eq!(
        best_call(&system, &after_one_heart, "KQxx.xx.Kxx.AQxx"),
        call(1, Strain::Spades),
    );
    // ... and rebids a natural 2♣ on six clubs after 1♣ – 1♦.
    assert_eq!(
        best_call(&system, &after_one_diamond, "xx.Kx.Kx.AQJxxx"),
        call(2, Strain::Clubs),
    );

    // Off: the diamond hand squeezes into 1NT, opener hides the spades
    // behind a 1NT rebid, and the six-card club hand lands in the
    // misdescribed 1NT catch-all.
    let off = stance_with(false, false, false);
    assert_eq!(
        best_call(&off, &one_club, "xx.xxx.AQJxx.Kxx"),
        call(1, Strain::Notrump),
    );
    assert_eq!(
        best_call(&off, &after_one_heart, "KQxx.xx.Kxx.AQxx"),
        call(1, Strain::Notrump),
    );
    assert_eq!(
        best_call(&off, &after_one_diamond, "xx.Kx.Kx.AQJxxx"),
        call(1, Strain::Notrump),
    );
}

// --- Knob B: XYZ -------------------------------------------------------------

/// `1♣ P 1♥ P 1NT P` plus the given tail of our-side calls (RHO passes
/// interleaved)
fn xyz_auction(response: Strain, tail: &[Call]) -> Vec<Call> {
    let mut auction = vec![
        call(1, Strain::Clubs),
        P,
        call(1, response),
        P,
        call(1, Strain::Notrump),
        P,
    ];
    for &c in tail {
        auction.push(c);
        auction.push(P);
    }
    auction
}

#[test]
fn xyz_relay_signs_off_in_diamonds() {
    let system = stance_with(false, false, true);
    let weak_diamonds = "x.Qxxx.KJxxxx.xx";
    let opener = "Axxx.Kxx.Qx.KJxx";

    // A weak hand with six diamonds relays 2♣ ...
    assert_eq!(
        best_call(&system, &xyz_auction(Strain::Hearts, &[]), weak_diamonds),
        call(2, Strain::Clubs),
    );
    // ... opener completes the puppet with the forced 2♦ ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Hearts, &[call(2, Strain::Clubs)]),
            opener,
        ),
        call(2, Strain::Diamonds),
    );
    // ... and responder passes it out: the sign-off the relay promised.
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(
                Strain::Hearts,
                &[call(2, Strain::Clubs), call(2, Strain::Diamonds)],
            ),
            weak_diamonds,
        ),
        Call::Pass,
    );
}

#[test]
fn xyz_game_force_finds_the_spade_fit() {
    let system = stance_with(false, false, true);

    // Game values check back with the artificial 2♦ ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Spades, &[]),
            "AKxxx.Axx.Kxx.xx",
        ),
        call(2, Strain::Diamonds),
    );
    // ... and opener shows the three-card spade support the 1NT rebid hid.
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Spades, &[call(2, Strain::Diamonds)]),
            "Qxx.Kxx.Qxx.AKxx",
        ),
        call(2, Strain::Spades),
    );
}

#[test]
fn xyz_invitation_accepted_to_game() {
    let system = stance_with(false, false, true);
    let responder = "xx.AQJxx.Kxx.xxx";
    let opener = "Axx.Kxx.Qxx.AQxx";

    // An eleven-count with five hearts relays 2♣ (all invites go through it) ...
    assert_eq!(
        best_call(&system, &xyz_auction(Strain::Hearts, &[]), responder),
        call(2, Strain::Clubs),
    );
    // ... then bids the invitational 2♥ over the forced 2♦ ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(
                Strain::Hearts,
                &[call(2, Strain::Clubs), call(2, Strain::Diamonds)],
            ),
            responder,
        ),
        call(2, Strain::Hearts),
    );
    // ... and opener accepts to the 5-3 game with a maximum.
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(
                Strain::Hearts,
                &[
                    call(2, Strain::Clubs),
                    call(2, Strain::Diamonds),
                    call(2, Strain::Hearts),
                ],
            ),
            opener,
        ),
        call(4, Strain::Hearts),
    );
}
