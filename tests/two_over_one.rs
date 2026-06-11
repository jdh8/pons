//! Representative auctions for the basic 2/1 game-forcing system

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain};
use pons::bidding::System;
use pons::bidding::array::Logits;
use pons::two_over_one;

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
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

// --- Openings ---------------------------------------------------------------

#[test]
fn test_openings() {
    let system = two_over_one();
    let open = &[][..];

    // 16 HCP balanced -> 1NT, even though 2♣ exists for the very strong.
    assert_eq!(
        best_call(&system, open, "AQ32.K53.QJ4.A92"),
        call(1, Strain::Notrump)
    );
    // 20 HCP balanced -> 2NT.
    assert_eq!(
        best_call(&system, open, "AKQ2.KQ5.KJ4.Q92"),
        call(2, Strain::Notrump)
    );
    // 22 HCP -> the strong, artificial 2♣.
    assert_eq!(
        best_call(&system, open, "AKQ2.AKJ.KQ4.932"),
        call(2, Strain::Clubs)
    );
    // 13 HCP, five spades -> 1♠.
    assert_eq!(
        best_call(&system, open, "AQJ32.K53.Q42.J2"),
        call(1, Strain::Spades)
    );
    // 13 HCP, five hearts -> 1♥.
    assert_eq!(
        best_call(&system, open, "A2.KQJ53.Q42.J92"),
        call(1, Strain::Hearts)
    );
    // 13 HCP, 4-4 minors -> 1♦ (better minor).
    assert_eq!(
        best_call(&system, open, "K2.A53.KJ42.Q982"),
        call(1, Strain::Diamonds)
    );
    // 14 HCP, 4-3-3-3 with 3-3 minors -> 1♣.
    assert_eq!(
        best_call(&system, open, "KQ52.A53.Q43.K92"),
        call(1, Strain::Clubs)
    );
    // 6 HCP, six spades -> a weak two.
    assert_eq!(
        best_call(&system, open, "KQJ732.53.842.92"),
        call(2, Strain::Spades)
    );
    // 9 HCP, too light to open in first seat.
    assert_eq!(best_call(&system, open, "AQJ32.853.Q42.92"), Call::Pass);
}

#[test]
fn test_light_third_seat_major() {
    let system = two_over_one();
    // The same 9-count that passes in first seat opens 1♠ in third.
    assert_eq!(
        best_call(&system, &[Call::Pass, Call::Pass], "AQJ32.853.Q42.92"),
        call(1, Strain::Spades),
    );
}

// --- Major responses --------------------------------------------------------

#[test]
fn test_major_responses() {
    let system = two_over_one();
    let after_1h = &[call(1, Strain::Hearts), Call::Pass][..];

    // 9 HCP, three-card support -> single raise.
    assert_eq!(
        best_call(&system, after_1h, "Q32.J53.A964.Q92"),
        call(2, Strain::Hearts)
    );
    // 14 HCP, four-card support -> Jacoby 2NT (game-forcing raise).
    assert_eq!(
        best_call(&system, after_1h, "K2.KQ54.A964.Q92"),
        call(2, Strain::Notrump)
    );
    // 10 HCP, four spades, no heart fit -> 1♠ (a new suit at the one level).
    assert_eq!(
        best_call(&system, after_1h, "KQ32.J5.A964.982"),
        call(1, Strain::Spades)
    );
    // 13 HCP, four clubs, no fit, no spades -> a 2/1 game force.
    assert_eq!(
        best_call(&system, after_1h, "A2.K3.Q543.KJ85"),
        call(2, Strain::Clubs)
    );
    // 8 HCP, no fit, no four-card spade suit -> the forcing 1NT.
    assert_eq!(
        best_call(&system, after_1h, "Q2.J3.K9643.Q982"),
        call(1, Strain::Notrump)
    );
}

// --- Minor responses --------------------------------------------------------

#[test]
fn test_minor_responses() {
    let system = two_over_one();

    // 1♣ - four hearts up the line -> 1♥.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Clubs), Call::Pass],
            "K32.KQ54.J643.92"
        ),
        call(1, Strain::Hearts),
    );
    // 1♦ - 13 HCP, five clubs, no major -> a 2/1 game force.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Diamonds), Call::Pass],
            "A32.K3.Q43.KJ842"
        ),
        call(2, Strain::Clubs),
    );
}

// --- 1NT structure ----------------------------------------------------------

#[test]
fn test_notrump_responses_and_completions() {
    let system = two_over_one();
    let p = Call::Pass;
    let one_nt = call(1, Strain::Notrump);

    // Stayman with a four-card major and invitational values.
    assert_eq!(
        best_call(&system, &[one_nt, p], "KJ54.Q32.K43.Q92"),
        call(2, Strain::Clubs)
    );
    // Transfer to spades on a five-card suit.
    assert_eq!(
        best_call(&system, &[one_nt, p], "KJ542.Q32.K43.92"),
        call(2, Strain::Hearts)
    );

    // Opener completes the spade transfer.
    assert_eq!(
        best_call(
            &system,
            &[one_nt, p, call(2, Strain::Hearts), p],
            "AQ32.K53.QJ4.A92"
        ),
        call(2, Strain::Spades),
    );
    // Opener answers Stayman with four hearts.
    assert_eq!(
        best_call(
            &system,
            &[one_nt, p, call(2, Strain::Clubs), p],
            "A32.KJ54.KQ4.A92"
        ),
        call(2, Strain::Hearts),
    );
}

// --- Opener's rebid ---------------------------------------------------------

#[test]
fn test_opener_rebid_raises_spades() {
    let system = two_over_one();
    let p = Call::Pass;
    // 1♥ - 1♠ - ?: 14 HCP with four spades raises to 2♠.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Hearts), p, call(1, Strain::Spades), p],
            "KQ32.AQ542.K43.2"
        ),
        call(2, Strain::Spades),
    );
}

// --- Competition over our opening ------------------------------------------

#[test]
fn test_negative_double_and_system_on() {
    let system = two_over_one();
    let one_h = call(1, Strain::Hearts);

    // 1♥ - (2♣) - ?: 10 HCP with four spades makes a negative double.
    assert_eq!(
        best_call(
            &system,
            &[one_h, call(2, Strain::Clubs)],
            "KQ32.J5.A964.982"
        ),
        Call::Double,
    );
    // 1♥ - (X) - ?: system on, the responses apply through the double.
    assert_eq!(
        best_call(&system, &[one_h, Call::Double], "Q32.J53.A964.Q92"),
        call(2, Strain::Hearts),
    );
}

// --- Defense ----------------------------------------------------------------

#[test]
fn test_defense() {
    let system = two_over_one();

    // (1♣) - ?: 9 HCP with five spades overcalls 1♠.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Clubs)], "AQJ32.853.Q42.92"),
        call(1, Strain::Spades),
    );
    // (1♠) - ?: 15 HCP short in spades makes a takeout double.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Spades)], "2.KQ54.AJ43.KQ92"),
        Call::Double,
    );
    // (1♣) - 1♠ - (P) - ?: advancer raises with three-card support.
    assert_eq!(
        best_call(
            &system,
            &[call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass],
            "Q32.K54.A432.J92",
        ),
        call(2, Strain::Spades),
    );
}

// --- Full table -------------------------------------------------------------

#[test]
fn test_full_board_smoke() {
    // Two copies paired into a table: the dealer's side opens, the other defends.
    let table = two_over_one().vs(two_over_one());

    assert_eq!(
        best_call(&table, &[], "AQ32.K53.QJ4.A92"),
        call(1, Strain::Notrump)
    );
    // After the opening, the opposing side's defensive book answers.
    assert!(
        table
            .classify(
                "AQJ32.853.Q42.92".parse().unwrap(),
                RelativeVulnerability::NONE,
                &[call(1, Strain::Clubs)]
            )
            .is_some()
    );
}
