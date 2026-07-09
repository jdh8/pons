//! Integration tests for the **European** 1NT minor scheme
//! ([`set_notrump_minors`]`(`[`EUROPEAN`]`)`): `2♠` = clubs (transfer), `2NT` = a
//! balanced invite / size ask, `3♣` = diamonds (transfer); no Puppet Stayman, so a
//! game-forcing balanced hand with only a three-card major bids 3NT and a 4-3 game
//! force takes Stayman.  Mirrors `american_minor_transfers.rs` (the Puppet default).
//!
//! [`set_notrump_minors`]: pons::american::set_notrump_minors
//! [`EUROPEAN`]: pons::american::EUROPEAN

mod common;
use common::*;

use pons::american::{EUROPEAN, set_notrump_minors};

/// The American 2/1 stance with the **European** minor scheme selected
///
/// `set_notrump_minors` is a thread-local read at book-construction time, so it is
/// set here on every call — each test thread builds a European book.
fn stance() -> Stance {
    set_notrump_minors(EUROPEAN);
    american().against(Family::NATURAL)
}

const P: Call = Call::Pass;

/// `1NT P` plus the given tail of our-side calls (RHO passes interleaved)
fn after_1nt(tail: &[Call]) -> Vec<Call> {
    let mut auction = vec![call(1, Strain::Notrump), P];
    for &c in tail {
        auction.push(c);
        auction.push(P);
    }
    auction
}

// --- 2♠ = transfer to clubs -------------------------------------------------

#[test]
fn two_spades_is_a_transfer_to_clubs() {
    let system = stance();
    // Six clubs, sub-game: the European club transfer, 2♠ (not the natural-spades
    // bid, and not Stayman — there is no four-card major).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "xxx.xxx.x.KQxxxx"),
        call(2, Strain::Spades),
    );
}

#[test]
fn opener_completes_the_club_transfer() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Spades)]);
    // Opener always completes the transfer to 3♣ (no super-accept).
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.Kxx.Axxx"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn weak_clubs_pass_the_completion() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    // Weak six-card club one-suiter: pass the club partscore.
    assert_eq!(best_call(&system, &auction, "xxx.xxx.x.KQxxxx"), P);
}

#[test]
fn game_going_clubs_splinter_over_the_completion() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Spades), call(3, Strain::Clubs)]);
    // Six clubs, game values, a singleton spade: splinter 3♠ so opener picks
    // between 3NT and 5♣.
    assert_eq!(
        best_call(&system, &auction, "x.Kxx.Kxx.AQxxxx"),
        call(3, Strain::Spades),
    );
}

// --- 2NT = balanced invitational (size ask) ---------------------------------

#[test]
fn two_nt_is_a_balanced_invite() {
    let system = stance();
    // Balanced 8, no four-card major, *not* a flat 4-3-3-3 (a 4-4 in the minors):
    // the European size ask, 2NT (the Puppet default would route this hand through
    // the two-way 2♠ instead).  A flat 4-3-3-3 eight would pass 1NT, not invite.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "Kx.Qxx.Jxxx.Qxxx"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn opener_accepts_the_invite_with_a_maximum() {
    let system = stance();
    let auction = after_1nt(&[call(2, Strain::Notrump)]);
    // Maximum (17): accept game, 3NT.
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.Kxx.Axxx"),
        call(3, Strain::Notrump),
    );
    // Minimum (15): decline, pass and play 2NT.
    assert_eq!(best_call(&system, &auction, "KQx.KJx.Qxx.Axxx"), P);
}

// --- 3♣ = transfer to diamonds ----------------------------------------------

#[test]
fn three_clubs_is_a_transfer_to_diamonds() {
    let system = stance();
    // Six diamonds, sub-game: the European diamond transfer, 3♣ (no Puppet Stayman
    // claims 3♣ here).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "xx.xxx.KQxxxx.xx"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn opener_completes_the_diamond_transfer() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs)]);
    // Opener always completes the diamond transfer to 3♦.
    assert_eq!(
        best_call(&system, &auction, "AQx.KJx.Kxx.Axxx"),
        call(3, Strain::Diamonds),
    );
}

#[test]
fn weak_diamonds_pass_the_completion() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // Six diamonds, sub-game: pass the 3♦ partscore.
    assert_eq!(best_call(&system, &auction, "xx.xxx.KQxxxx.xx"), P);
}

#[test]
fn game_going_diamonds_raise_to_3nt() {
    let system = stance();
    let auction = after_1nt(&[call(3, Strain::Clubs), call(3, Strain::Diamonds)]);
    // Six diamonds, game values: bid 3NT over the completion.
    assert_eq!(
        best_call(&system, &auction, "xx.Axx.KQJxxx.xx"),
        call(3, Strain::Notrump),
    );
}

// --- No Puppet: the GF balanced / 4-3 hands route elsewhere ------------------

#[test]
fn game_force_three_card_major_bids_3nt() {
    let system = stance();
    // 3-3 majors, balanced 11: the hand Puppet routes through 3♣ has no home in
    // the European scheme (3♣ is diamonds) — it simply bids 3NT.
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "K32.Q43.KJ4.Q932"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn game_force_four_three_takes_stayman() {
    let system = stance();
    // 4♠-3♥-4♦-2♣ game force (non-flat): with no Puppet, the 4-3 hand uses plain
    // Stayman (2♣).  A flat 4-3-3-3 would instead bid 3NT (see below).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.K432.Q9"),
        call(2, Strain::Clubs),
    );
}

#[test]
fn flat_four_three_three_three_game_force_bids_3nt() {
    let system = stance();
    // Flat 4-3-3-3 (four spades) game force: no Stayman with a flat hand — it plays
    // 3NT, not the 4-4 fit (European has no Puppet either, so it simply bids 3NT).
    assert_eq!(
        best_call(&system, &after_1nt(&[]), "KJ54.Q32.K43.Q92"),
        call(3, Strain::Notrump),
    );
}
