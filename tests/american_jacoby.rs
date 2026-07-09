//! Integration tests for Jacoby 2NT opener rebids in the 2/1 game-forcing system

mod common;
use common::*;

// --- Opener's Jacoby rebid after 1♠ – (P) – 2NT – (P) ----------------------

/// Opener shows a singleton club (3♣ = shortness) over 1♠–2NT
#[test]
fn jacoby_1s_rebid_shortness_club_singleton() {
    let system = stance();
    // 13 HCP, 5332 in spades with singleton club → 3♣ (shortness)
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass
            ],
            "AKJ52.K765.Q72.9",
        ),
        call(3, Strain::Clubs),
    );
}

/// Opener shows a good five-card diamond suit (4♦) over 1♠–2NT
#[test]
fn jacoby_1s_rebid_second_suit_diamonds() {
    let system = stance();
    // 14 HCP, five good diamonds (KQJ) beats showing the heart singleton
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass
            ],
            "AKJ52.7.KQJ75.92",
        ),
        call(4, Strain::Diamonds),
    );
}

/// Opener shows a big balanced hand (3♠) over 1♠–2NT
#[test]
fn jacoby_1s_rebid_big_balanced() {
    let system = stance();
    // 19 HCP, no shortness → 3♠ (18+ balanced-ish)
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass
            ],
            "AKJ52.KQ7.AQ7.92",
        ),
        call(3, Strain::Spades),
    );
}

/// Opener shows a medium balanced hand (3NT) over 1♠–2NT
#[test]
fn jacoby_1s_rebid_medium_balanced() {
    let system = stance();
    // 16 HCP (16.6 Fifths), 5-4-2-2, no shortness → 3NT (15–17)
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass
            ],
            "AKT52.KQT4.A2.92",
        ),
        call(3, Strain::Notrump),
    );
}

/// Opener shows a minimum hand (4♠) over 1♠–2NT
#[test]
fn jacoby_1s_rebid_minimum() {
    let system = stance();
    // 12 HCP, minimum opener → 4♠
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass
            ],
            "AKJ52.Q76.Q72.92",
        ),
        call(4, Strain::Spades),
    );
}

// --- Responder's continuation after opener's Jacoby rebid -------------------

/// Responder with 16 HCP bids 4NT (slam try) after opener's 3♣ shortness bid
#[test]
fn jacoby_responder_slam_try_after_shortness() {
    let system = stance();
    // 16 HCP → 4NT (slam try) after opener showed club shortness
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass,
                call(3, Strain::Clubs),
                Call::Pass,
            ],
            "KQ52.AK76.A72.93",
        ),
        call(4, Strain::Notrump),
    );
}

/// Responder with 14 HCP settles in game (4♠) after opener's 3♣ shortness bid
#[test]
fn jacoby_responder_game_only_after_shortness() {
    let system = stance();
    // 14 HCP → 4♠ (game only) after opener showed club shortness
    assert_eq!(
        best_call(
            &system,
            &[
                call(1, Strain::Spades),
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass,
                call(3, Strain::Clubs),
                Call::Pass,
            ],
            "KQ52.A976.K72.Q3",
        ),
        call(4, Strain::Spades),
    );
}
