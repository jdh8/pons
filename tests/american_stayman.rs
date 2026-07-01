//! Integration tests for the authored Stayman (1NT–2♣) continuations:
//! responder's further bidding, the artificial 3OM slam try, Smolen, and the
//! "ignore 2♣ ⇒ revert to notrump" rule (including the inference that lets the
//! floor accept or decline an invitation).

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::american;
use pons::bidding::array::Logits;
use pons::bidding::{Family, Stance, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

fn stance() -> Stance {
    american().against(Family::NATURAL)
}

/// The single highest-logit call the system assigns the hand for the auction
fn best_call(system: &impl System, auction: &[Call], hand: &str) -> Call {
    let hand: Hand = hand.parse().expect("valid test hand");
    let logits: Logits = system
        .classify(hand, RelativeVulnerability::NONE, auction)
        .expect("system covers this auction");
    (&logits.0)
        .into_iter()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
        .map(|(call, _)| call)
        .expect("array is never empty")
}

const P: Call = Call::Pass;

/// `1NT P 2♣ P` plus the given tail of our-side calls (RHO passes interleaved)
fn after_stayman(tail: &[Call]) -> Vec<Call> {
    let mut auction = vec![call(1, Strain::Notrump), P, call(2, Strain::Clubs), P];
    for &c in tail {
        auction.push(c);
        auction.push(P);
    }
    auction
}

// --- Responder's rebid after opener shows a major ---------------------------

#[test]
fn responder_signs_off_in_the_major_game_with_a_fit() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts)]);
    // Four hearts, game values, unbalanced (singleton spade): sign off in 4♥.
    assert_eq!(
        best_call(&system, &auction, "x.KQxx.Kxxxx.Axx"),
        call(4, Strain::Hearts),
    );
}

#[test]
fn responder_invites_with_a_fit_and_eight() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts)]);
    // Four hearts, a bare 8: invitational raise to 3♥.
    assert_eq!(
        best_call(&system, &auction, "xxx.KQxx.Kxxx.xx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn responder_bids_3om_as_a_slam_try_with_a_balanced_fit() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts)]);
    // Four hearts, balanced, game-forcing: the artificial 3♠ (other major).
    assert_eq!(
        best_call(&system, &auction, "Kxx.KQxx.Kxx.Qxx"),
        call(3, Strain::Spades),
    );
}

#[test]
fn responder_reverts_to_quantitative_4nt_without_a_fit() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts)]);
    // Four spades (no heart fit), balanced 16: 4NT exactly as over a bare 1NT.
    assert_eq!(
        best_call(&system, &auction, "AQJx.xx.Kxx.AQxx"),
        call(4, Strain::Notrump),
    );
}

// --- Opener's reply to the 3OM slam try -------------------------------------

#[test]
fn opener_answers_3om_by_shape_and_strength() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts), call(3, Strain::Spades)]);

    // Flat 4-3-3-3: choose notrump.
    assert_eq!(
        best_call(&system, &auction, "Kxx.AQxx.Kxx.Qxx"),
        call(3, Strain::Notrump),
    );
    // Minimum, not flat: sign off in the major game.
    assert_eq!(
        best_call(&system, &auction, "Kx.AQxx.Kxxx.Qxx"),
        call(4, Strain::Hearts),
    );
    // Maximum with a club control, not flat: cue the cheapest control.
    assert_eq!(
        best_call(&system, &auction, "Ax.AQxx.xxxx.AKx"),
        call(4, Strain::Clubs),
    );
}

// --- Responder's continuation after opener's cue ----------------------------

#[test]
fn responder_keycards_or_signs_off_over_openers_cue() {
    let system = stance();
    // 1NT–2♣–2♥–3♠(slam try)–4♣(opener cues a max club control).
    let auction = after_stayman(&[
        call(2, Strain::Hearts),
        call(3, Strain::Spades),
        call(4, Strain::Clubs),
    ]);
    // Slam values opposite the shown maximum: launch RKCB, don't pass the cue.
    assert_eq!(
        best_call(&system, &auction, "Axx.KQxx.Axxx.Kx"),
        call(4, Strain::Notrump),
    );
    // Plain choice-of-game values: sign off in the major game (never below it).
    assert_eq!(
        best_call(&system, &auction, "Kxx.Qxxx.Kxx.Qxx"),
        call(4, Strain::Hearts),
    );
}

// --- Smolen -----------------------------------------------------------------

#[test]
fn responder_jumps_smolen_over_two_diamonds() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Diamonds)]);
    // Five spades, four hearts, game-forcing: 3♥ shows the five-card spade suit.
    assert_eq!(
        best_call(&system, &auction, "AKxxx.Qxxx.xx.Kx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn opener_completes_smolen_into_the_long_major() {
    let system = stance();
    let auction = after_stayman(&[
        call(2, Strain::Diamonds),
        call(3, Strain::Hearts), // Smolen: responder holds five spades
    ]);
    // Three spades: bid game in spades so opener declares.
    assert_eq!(
        best_call(&system, &auction, "Qxx.Axx.KQxx.Axx"),
        call(4, Strain::Spades),
    );
    // Doubleton spade: no fit, notrump game.
    assert_eq!(
        best_call(&system, &auction, "Jx.AKx.KQxx.Axxx"),
        call(3, Strain::Notrump),
    );
}

// --- Inference: opener accepts/declines an invitation off-book ---------------

#[test]
fn opener_accepts_the_invitational_raise_into_the_major_with_a_maximum() {
    let system = stance();
    // 1NT P 2♣ P 2♥ P 3♥ P — responder invited with a fit.
    let auction = after_stayman(&[call(2, Strain::Hearts), call(3, Strain::Hearts)]);
    // Maximum 17, four hearts, not flat: accept in the major game.
    assert_eq!(
        best_call(&system, &auction, "Qx.KQJx.AQx.Kxxx"),
        call(4, Strain::Hearts),
    );
}

#[test]
fn opener_accepts_the_invitational_raise_in_notrump_when_flat() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts), call(3, Strain::Hearts)]);
    // Maximum 17 but a flat 4-3-3-3: choose 3NT over the eight-card fit.
    assert_eq!(
        best_call(&system, &auction, "Qxx.KQJx.AQx.Kxx"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn opener_declines_the_invitational_raise_with_a_minimum() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Hearts), call(3, Strain::Hearts)]);
    // Minimum 15 with four hearts: pass the partscore.
    assert_eq!(best_call(&system, &auction, "Kxx.KQJx.Axx.Qxx"), P);
}

// --- Quantitative accept after the no-fit revert ----------------------------

#[test]
fn opener_accepts_the_no_fit_quantitative_with_a_maximum() {
    let system = stance();
    let auction = after_stayman(&[call(2, Strain::Diamonds), call(4, Strain::Notrump)]);
    // 17 balanced, no four-card major (opener denied one with 2♦): bid 6NT.
    assert_eq!(
        best_call(&system, &auction, "Kx.Kxx.AQxx.AJxx"),
        call(6, Strain::Notrump),
    );
}

// --- 2NT-strength Smolen ----------------------------------------------------

#[test]
fn smolen_works_at_the_two_notrump_level() {
    let system = stance();
    // 2NT P 3♣ P 3♦ P — opener denied a major; responder jumps Smolen.
    let auction = &[
        call(2, Strain::Notrump),
        P,
        call(3, Strain::Clubs),
        P,
        call(3, Strain::Diamonds),
        P,
    ][..];
    assert_eq!(
        best_call(&system, auction, "AKxxx.Qxxx.xx.xx"),
        call(3, Strain::Hearts),
    );
}

// --- Stayman treatments (garbage, opener's max-showing answers) --------------
//
// All three toggles are thread-local and read at book-construction time, so each
// test sets them, builds the stance, then restores the library defaults before
// asserting (the book is already captured) so a reused worker thread cannot leak
// into a `stance()` test that expects the defaults. Defaults: garbage on,
// both-majors on, five-card-max on.

fn stance_with(garbage: bool, both_majors: bool, five_card_max: bool) -> Stance {
    pons::bidding::american::set_garbage_stayman(garbage);
    pons::bidding::american::set_stayman_both_majors(both_majors);
    pons::bidding::american::set_stayman_5card_max(five_card_max);
    let system = american().against(Family::NATURAL);
    pons::bidding::american::set_garbage_stayman(true);
    pons::bidding::american::set_stayman_both_majors(true);
    pons::bidding::american::set_stayman_5card_max(true);
    system
}

// --- Max-only both-majors relay (2NT = 16-17, 3♣/3♦ name responder's major) --

#[test]
fn both_majors_responder_relays_hearts_via_3c() {
    let system = stance_with(false, true, false);
    // Opener showed both majors, maximum (2NT); responder with four hearts names
    // them via 3♣ so opener declares (right-siding).
    let auction = after_stayman(&[call(2, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &auction, "xx.KQxx.Kxxx.Qxx"),
        call(3, Strain::Clubs),
    );
}

#[test]
fn both_majors_opener_completes_relay_to_hearts() {
    let system = stance_with(false, true, false);
    // Responder relayed 3♣ (hearts); opener completes 3♥ so opener declares.
    let auction = after_stayman(&[call(2, Strain::Notrump), call(3, Strain::Clubs)]);
    assert_eq!(
        best_call(&system, &auction, "AKxx.AQxx.xx.Kxx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn both_majors_responder_raises_completion_to_game() {
    let system = stance_with(false, true, false);
    // Over opener's 3♥ completion, responder with game values raises to 4♥.
    let auction = after_stayman(&[
        call(2, Strain::Notrump),
        call(3, Strain::Clubs),
        call(3, Strain::Hearts),
    ]);
    assert_eq!(
        best_call(&system, &auction, "xx.KQxx.Kxxx.Qxx"),
        call(4, Strain::Hearts),
    );
}

#[test]
fn garbage_weak_both_majors_staymans() {
    let system = stance_with(true, false, false);
    // 6 HCP, 4-4-4-1 (short clubs): too weak for constructive Stayman, but with
    // garbage on it bids 2♣ to escape 1NT.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Notrump), P], "Qxxx.Jxxx.Kxxx.x"),
        call(2, Strain::Clubs),
    );
}

#[test]
fn garbage_responder_passes_opener_answer() {
    let system = stance_with(true, false, false);
    // Same weak hand: over opener's 2♥ it sits in the 4-4 fit (drop-dead).
    let auction = after_stayman(&[call(2, Strain::Hearts)]);
    assert_eq!(best_call(&system, &auction, "Qxxx.Jxxx.Kxxx.x"), P);
}

#[test]
fn garbage_off_the_weak_hand_passes_one_nt() {
    let system = stance_with(false, false, false);
    // With garbage off, the weak hand has no Stayman and passes 1NT.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Notrump), P], "Qxxx.Jxxx.Kxxx.x"),
        P,
    );
}

#[test]
fn both_majors_minimum_opener_bids_2h() {
    let system = stance_with(false, true, false);
    // 15 HCP, 4-4-3-2 both majors, minimum: 2♥ up-the-line (no jump).
    let auction = after_stayman(&[]);
    assert_eq!(
        best_call(&system, &auction, "AKxx.KQxx.Kxx.xx"),
        call(2, Strain::Hearts),
    );
}

#[test]
fn both_majors_maximum_opener_bids_2nt() {
    let system = stance_with(false, true, false);
    // 16 HCP, 4-4-2-3 both majors, maximum: jump to 2NT.
    let auction = after_stayman(&[]);
    assert_eq!(
        best_call(&system, &auction, "AKxx.AQxx.xx.Kxx"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn both_majors_off_opener_bids_2h_up_the_line() {
    let system = stance_with(false, false, false);
    // Toggles off: the both-majors hand answers 2♥ up-the-line, as today.
    let auction = after_stayman(&[]);
    assert_eq!(
        best_call(&system, &auction, "AKxx.KQxx.Kxx.xx"),
        call(2, Strain::Hearts),
    );
}

#[test]
fn five_card_max_opener_jumps_3h() {
    let system = stance_with(false, false, true);
    // 16 HCP, 3-5-3-2 (five hearts), maximum: jump to 3♥.
    let auction = after_stayman(&[]);
    assert_eq!(
        best_call(&system, &auction, "AQx.AKxxx.Kxx.xx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn five_card_minimum_opener_bids_2h() {
    let system = stance_with(false, false, true);
    // 15 HCP, 3-5-3-2 (five hearts), minimum: natural 2♥ (no jump).
    let auction = after_stayman(&[]);
    assert_eq!(
        best_call(&system, &auction, "AQx.AKxxx.Qxx.xx"),
        call(2, Strain::Hearts),
    );
}

#[test]
fn both_majors_responder_relays_spades_via_3d() {
    let system = stance_with(false, true, false);
    // Opener showed both majors, maximum (2NT); responder with four spades names
    // them via 3♦ so opener declares.
    let auction = after_stayman(&[call(2, Strain::Notrump)]);
    assert_eq!(
        best_call(&system, &auction, "AQxx.xxx.Kxxx.Qx"),
        call(3, Strain::Diamonds),
    );
}
