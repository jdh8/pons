//! Integration tests for responder's `1NT - 3♥/3♠` splinter
//! ([`nt_splinter`][field@pons::bidding::inference::ReadingProfile::nt_splinter]), the Bridge World Standard / Polish Club treatment of
//! the two slots our response ladder leaves empty.
//!
//! The agreement is a *pinned* shape rather than a floored one — shortness in
//! the **bid** major, 2–3 in the other, exactly four diamonds, five or six clubs
//! — because every neighbouring slot already owns its half of the residue.
//! These tests pin both halves: that the shape bids `3M`, and that each
//! neighbour keeps the hands it already had.
//!
mod common;
use common::*;

/// The 2/1 stance with the splinter authored
///
/// Each call builds a stance whose agreements author the splinter.
fn stance() -> Stance {
    let mut agreements = pons::bidding::agreements::Agreements::current();
    agreements.decision.reading.nt_splinter = true;
    american(&agreements).against()
}

/// The 2/1 stance with the splinter *not* authored — the pre-2026-07-28 ladder
fn without() -> Stance {
    let mut agreements = pons::bidding::agreements::Agreements::current();
    agreements.decision.reading.nt_splinter = false;
    american(&agreements).against()
}

const P: Call = Call::Pass;

/// `1NT -` — the direct-response auction
fn after_1nt() -> Vec<Call> {
    vec![call(1, Strain::Notrump), P]
}

// --- the shape bids 3M ------------------------------------------------------

#[test]
fn the_homeless_shape_splinters() {
    // ♠xxx ♥x ♦AQxx ♣KQxxx — 3-1-4-5, 11 HCP.  The hand this convention exists
    // for: too few majors for Stayman, too few diamonds for the 2NT transfer,
    // too few clubs for the 2♠ transfer, not `balanced()` for Puppet 3♣.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xxx.x.AQxx.KQxxx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn the_same_hand_blasts_3nt_without_the_knob() {
    // Knob off, the slot is empty and the hand has nowhere to go but 3NT —
    // opposite a possible ♥KQx, exactly the guess the splinter removes.
    let system = without();
    assert_eq!(
        best_call(&system, &after_1nt(), "xxx.x.AQxx.KQxxx"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn short_spades_splinters_in_spades() {
    // ♠x ♥xxx ♦AQxx ♣KQxxx — the mirror, 1-3-4-5.  Responder names the *short*
    // major (splinter, not fragment).
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "x.xxx.AQxx.KQxxx"),
        call(3, Strain::Spades),
    );
}

#[test]
fn a_void_splinters() {
    // ♠xxx ♥— ♦AQxx ♣KJxxxx — 3-0-4-6, 10 HCP.  Voids only occur in the ♣6 row,
    // since ♣5 forces the majors to 3-1.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xxx..AQxx.KJxxxx"),
        call(3, Strain::Hearts),
    );
}

#[test]
fn the_six_four_outranks_the_club_transfer() {
    // ♠xx ♥x ♦Axxx ♣KQxxxx — 2-1-4-6, 9 HCP.  This *does* qualify for the 2♠
    // club transfer (♣6+), and the splinter takes it deliberately: after 2♠
    // responder can never show the four diamonds, which is the 6-4 slam lane.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xx.x.Axxx.KQxxxx"),
        call(3, Strain::Hearts),
    );
}

// --- the neighbours keep their hands ----------------------------------------

#[test]
fn a_stiff_ace_does_not_splinter() {
    // ♠xxx ♥A ♦KQxx ♣KJxxx — the shape, but the shortness is an ace.  Partner's
    // ♥KQxx opposite our ♥A is three tricks, not wasted, and a stiff ace is a
    // real notrump stopper — so the hand belongs in 3NT, and `splinter_short`
    // (void or *low* singleton) keeps it there.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xxx.A.KQxx.KJxxx"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn five_diamonds_keeps_the_2nt_transfer() {
    // ♠xxx ♥x ♦AQxxx ♣KQxx — 3-1-5-4.  Pinning diamonds at exactly four is what
    // buys zero overlap: the diamond-long half of (31)(54) already has a home.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xxx.x.AQxxx.KQxx"),
        call(2, Strain::Notrump),
    );
}

#[test]
fn seven_clubs_keeps_the_2s_transfer() {
    // ♠xx ♥— ♦Axxx ♣KQxxxxx — 2-0-4-7.  Past six clubs the hand is a one-suiter
    // and the 2♠ transfer, which agrees clubs a level lower, bids it better.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xx..Axxx.KQxxxxx"),
        call(2, Strain::Spades),
    );
}

#[test]
fn a_four_card_other_major_staymans() {
    // ♠xxxx ♥x ♦AQxx ♣KQxx — four spades.  The four-card major is Stayman's
    // job; excluding it is the whole difference from BBA's GIB-form splinter,
    // which pins the other major at exactly four.
    let system = stance();
    assert_eq!(
        best_call(&system, &after_1nt(), "xxxx.x.AQxx.KQxx"),
        call(2, Strain::Clubs),
    );
}

// --- opener places the game -------------------------------------------------
//
// These pin the judgement the convention exists for.  Without the authored
// answer the floor bid `3NT` on *every* hand below — a 600 000-board A/B found
// 217 firings and 9 divergent boards, i.e. responder described perfectly and
// opener ignored it.

#[test]
fn opener_takes_3nt_with_a_guard() {
    // ♥KQx opposite responder's stiff: wasted honors, but the opponents' nine
    // hearts are guarded, so nine tricks beat eleven.
    let system = stance();
    let auction = vec![call(1, Strain::Notrump), P, call(3, Strain::Hearts), P];
    assert_eq!(
        best_call(&system, &auction, "AQx.KQx.Kxx.Axxx"),
        call(3, Strain::Notrump),
    );
}

#[test]
fn opener_places_the_club_game_without_a_guard() {
    // ♥xxx opposite a stiff: the opponents cash out against 3NT.  Responder's
    // 5-6 clubs opposite three is a nine-card fit — take the eleven-trick game.
    let system = stance();
    let auction = vec![call(1, Strain::Notrump), P, call(3, Strain::Hearts), P];
    assert_eq!(
        best_call(&system, &auction, "AQx.xxx.KQx.AKxx"),
        call(5, Strain::Clubs),
    );
}

#[test]
fn opener_prefers_diamonds_when_short_in_clubs() {
    // ♥xxx unguarded and only two clubs — the 4-4 diamond fit is the trump suit.
    let system = stance();
    let auction = vec![call(1, Strain::Notrump), P, call(3, Strain::Hearts), P];
    assert_eq!(
        best_call(&system, &auction, "AQxx.xxx.KQxx.Ax"),
        call(5, Strain::Diamonds),
    );
}

#[test]
fn opener_answers_the_spade_splinter_too() {
    // The mirror: 3♠ shows short spades, so the guard that matters is in spades.
    let system = stance();
    let auction = vec![call(1, Strain::Notrump), P, call(3, Strain::Spades), P];
    assert_eq!(
        best_call(&system, &auction, "xxx.AQx.KQx.AKxx"),
        call(5, Strain::Clubs),
    );
}

// --- the strength floor -----------------------------------------------------

#[test]
fn the_eight_count_passes_at_the_default_floor() {
    // ♠Jxx ♥x ♦AQxx ♣Jxxxx — the shape at 8 HCP, one under the floor.  It still
    // passes 1NT holding a singleton opposite 15-17; whether that is right is
    // the 8-versus-9 sweep, not the shipped default.
    let system = stance();
    assert_eq!(best_call(&system, &after_1nt(), "Jxx.x.AQxx.Jxxxx"), P);
}

#[test]
fn lowering_the_floor_catches_the_eight() {
    // The same hand with the floor at 8 — the sweep arm.
    let mut agreements = pons::bidding::agreements::Agreements::current();
    agreements.decision.reading.nt_splinter = true;
    agreements.notrump.nt_splinter_floor = 8;
    let system = american(&agreements).against();
    let bid = best_call(&system, &after_1nt(), "Jxx.x.AQxx.Jxxxx");
    assert_eq!(bid, call(3, Strain::Hearts));
}
