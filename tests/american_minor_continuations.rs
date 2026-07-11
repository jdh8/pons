//! Integration tests for the minor-opening continuation knobs: the
//! longer-major response discipline (`set_longer_major_response`), the
//! up-the-line completion (`set_up_the_line`), and the XYZ two-way checkback
//! (`set_xyz`).  Each test builds its own stance with the knobs it needs and
//! restores the defaults, so the rest of the suite keeps measuring the
//! shipped system.

mod common;
use common::*;

use pons::bidding::american::{
    set_longer_major_response, set_new_minor_forcing, set_up_the_line, set_xyz,
};

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

// --- Knob D: New Minor Forcing (the opt-in XYZ alternative) -------------------

/// A stance running New Minor Forcing in place of XYZ, defaults restored
///
/// NMF overrides XYZ on the four `1m – 1M – 1NT` slots; the tests only touch
/// those, so `set_xyz` is left off to isolate the convention purely.
fn nmf_stance() -> Stance {
    set_longer_major_response(false);
    set_up_the_line(false);
    set_xyz(false);
    set_new_minor_forcing(true);
    let stance = american().against(Family::NATURAL);
    set_longer_major_response(true);
    set_up_the_line(false);
    set_xyz(false);
    set_new_minor_forcing(false);
    stance
}

#[test]
fn new_minor_forcing_checks_back_with_a_five_card_major() {
    let system = nmf_stance();
    let two_d = call(2, Strain::Diamonds);

    // Invitational with five hearts checks back with the new minor ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Hearts, &[]),
            "xx.AQJxx.Kxx.xxx"
        ),
        two_d,
    );
    // ... game values with five hearts likewise (NMF is invitational-or-better) ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Hearts, &[]),
            "Kx.AQJxx.Kxx.xxx"
        ),
        two_d,
    );
    // ... only four hearts has no fit to hunt, so it invites naturally with 2NT ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Hearts, &[]),
            "xxx.AQJx.Kxx.Qxx"
        ),
        call(2, Strain::Notrump),
    );
    // ... and a weak hand rebids its long major to play, never the checkback.
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Hearts, &[]),
            "x.KJxxxx.xxx.xx"
        ),
        call(2, Strain::Hearts),
    );
}

#[test]
fn new_minor_forcing_opener_answers() {
    let system = nmf_stance();
    let after_nmf = xyz_auction(Strain::Hearts, &[call(2, Strain::Diamonds)]);

    // Three-card support, minimum: a simple 2♥ ...
    assert_eq!(
        best_call(&system, &after_nmf, "Axx.Kxx.Qxx.Axxx"),
        call(2, Strain::Hearts),
    );
    // ... three-card support, maximum: the jump to 3♥ ...
    assert_eq!(
        best_call(&system, &after_nmf, "Axx.Kxx.Qxx.AJxx"),
        call(3, Strain::Hearts),
    );
    // ... and no fit forces a natural 2NT — opener may never pass the checkback.
    assert_eq!(
        best_call(&system, &after_nmf, "Axx.Kx.Qxxx.Axxx"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn new_minor_forcing_finds_the_other_major() {
    let system = nmf_stance();

    // Five spades and game values check back with 2♦ ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Spades, &[]),
            "AKxxx.xx.Qxx.KJx"
        ),
        call(2, Strain::Diamonds),
    );
    // ... and opener shows the four-card heart suit the 1NT rebid concealed.
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(Strain::Spades, &[call(2, Strain::Diamonds)]),
            "xx.AQxx.Kxx.Axxx",
        ),
        call(2, Strain::Hearts),
    );
}

#[test]
fn new_minor_forcing_invitation_placed() {
    let system = nmf_stance();
    let inviter = "Kx.AQJxx.Qxx.xxx"; // twelve with five hearts

    // Opener's minimum (the simple 2♥) declines: an invitational responder
    // passes out the 5-3 partscore ...
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(
                Strain::Hearts,
                &[call(2, Strain::Diamonds), call(2, Strain::Hearts)],
            ),
            inviter,
        ),
        Call::Pass,
    );
    // ... opener's maximum (the jump to 3♥) accepts to the 5-3 game.
    assert_eq!(
        best_call(
            &system,
            &xyz_auction(
                Strain::Hearts,
                &[call(2, Strain::Diamonds), call(3, Strain::Hearts)],
            ),
            inviter,
        ),
        call(4, Strain::Hearts),
    );
}

#[test]
fn new_minor_forcing_accepts_the_natural_2nt_invitation() {
    let system = nmf_stance();
    // With only four hearts responder cannot check back, so it invites naturally
    // with 2NT.  Opener must judge that invite: a maximum accepts to game, a
    // minimum passes.  (Without this authored acceptance the invite floated to
    // the floor, which passed a maximum — the dominant loss against XYZ.)
    let after_2nt = xyz_auction(Strain::Hearts, &[call(2, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &after_2nt, "Axx.Kxx.Qxx.AJxx"), // fourteen: accept
        call(3, Strain::Notrump),
    );
    assert_eq!(
        best_call(&system, &after_2nt, "Axx.Kxx.Qxx.Kxxx"), // twelve: decline
        Call::Pass,
    );
}
