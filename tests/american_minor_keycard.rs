//! Integration tests for plain-4NT minor-suit keycard (RKCB 1430 over an agreed
//! minor): the two wired vehicles — the inverted minor raise (after opener's
//! 18–19 jump to 3NT) and the strong-2♣ minor raise — and the floor correctly
//! passing the slam placement.

mod common;
use common::*;

const P: Call = Call::Pass;

// --- Inverted minor raise: 1♣ – 2♣ – 3NT (18–19) ---------------------------

/// Responder with slam values launches keycard (4NT) over opener's 3NT.
#[test]
fn responder_launches_minor_keycard_over_3nt() {
    let system = stance();
    // 1♣ P 2♣ P 3NT P — responder to act.
    let auction = [
        call(1, Strain::Clubs),
        P,
        call(2, Strain::Clubs),
        P,
        call(3, Strain::Notrump),
        P,
    ];
    // 15 HCP, six clubs, no four-card major: a sound inverted raise with extras.
    assert_eq!(
        best_call(&system, &auction, "Qx.Kxx.Kx.AQJxxx"),
        call(4, Strain::Notrump),
    );
}

/// Without extra values, responder passes the cold 3NT (no keycard).
#[test]
fn responder_passes_3nt_without_slam_values() {
    let system = stance();
    let auction = [
        call(1, Strain::Clubs),
        P,
        call(2, Strain::Clubs),
        P,
        call(3, Strain::Notrump),
        P,
    ];
    // Minimum inverted raise (~11 HCP, five clubs): no slam, pass 3NT.
    assert_eq!(best_call(&system, &auction, "xxx.Kxx.Qx.AQxxx"), P);
}

/// Over responder's 4NT, opener gives a keycard answer (the ask is wired).
#[test]
fn opener_answers_keycard_in_inverted_minor() {
    let system = stance();
    // 1♣ P 2♣ P 3NT P 4NT P — opener to answer.
    let auction = [
        call(1, Strain::Clubs),
        P,
        call(2, Strain::Clubs),
        P,
        call(3, Strain::Notrump),
        P,
        call(4, Strain::Notrump),
        P,
    ];
    // 18 HCP, balanced: A♠ A♥ A♦ K♣ = 4 keycards → 5♣ ("1 or 4").
    assert_eq!(
        best_call(&system, &auction, "Axxx.Axx.Axx.Kxx"),
        call(5, Strain::Clubs),
    );
}

// --- Strong 2♣ minor raise: 2♣ – 2♦ – 3♣ – 4♣ ------------------------------

/// A 28+ HCP opener launches keycard rather than blasting the slam blind.
#[test]
fn strong_two_opener_launches_minor_keycard() {
    let system = stance();
    // 2♣ P 2♦ P 3♣ P 4♣ P — opener to act.
    let auction = [
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        P,
        call(3, Strain::Clubs),
        P,
        call(4, Strain::Clubs),
        P,
    ];
    // 29 HCP, six clubs: monster that opened 2♣ and showed clubs.
    assert_eq!(
        best_call(&system, &auction, "Ax.AKQ.AK.AKQxxx"),
        call(4, Strain::Notrump),
    );
}

// --- Floor passes the slam placement ---------------------------------------

/// After the asker places the minor slam (6♣), partner passes (handled by the
/// instinct floor — no node is authored after the placement).
#[test]
fn opener_passes_the_minor_slam_placement() {
    let system = stance();
    // 1♣ P 2♣ P 3NT P 4NT P 5♣ P 6♣ P — opener (the answerer) to act.
    let auction = [
        call(1, Strain::Clubs),
        P,
        call(2, Strain::Clubs),
        P,
        call(3, Strain::Notrump),
        P,
        call(4, Strain::Notrump),
        P,
        call(5, Strain::Clubs),
        P,
        call(6, Strain::Clubs),
        P,
    ];
    // The 18–19 opener that answered 5♣ — passes the slam responder placed.
    assert_eq!(best_call(&system, &auction, "AQxx.AQx.Kxx.Axx"), P);
}

// --- The knob's off arm (the A7 re-measure baseline) ------------------------

/// With `set_minor_keycard(false)`, the pre-keycard book returns: the strong-2♣
/// monster blind-jumps 6♣ (27+) instead of asking, and the inverted-minor
/// responder rests in the 18–19 3NT instead of launching 4NT.
#[test]
fn knob_off_restores_the_pre_keycard_book() {
    pons::bidding::american::set_minor_keycard(false);
    let system = stance();
    pons::bidding::american::set_minor_keycard(true);

    let strong_two = [
        call(2, Strain::Clubs),
        P,
        call(2, Strain::Diamonds),
        P,
        call(3, Strain::Clubs),
        P,
        call(4, Strain::Clubs),
        P,
    ];
    assert_eq!(
        best_call(&system, &strong_two, "Ax.AKQ.AK.AKQxxx"),
        call(6, Strain::Clubs),
    );

    let inverted = [
        call(1, Strain::Clubs),
        P,
        call(2, Strain::Clubs),
        P,
        call(3, Strain::Notrump),
        P,
    ];
    assert_eq!(best_call(&system, &inverted, "Qx.Kxx.Kx.AQJxxx"), P);
}
