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
