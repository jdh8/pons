//! A thin 2/1-flavored system slice as living documentation
//!
//! This test wires every layer together: constraint-built [`Rules`] in a
//! [`Forest`] (shared 1st/2nd-seat books, a lighter 3rd/4th-seat opening),
//! a negative-double package as a guarded fallback, and "system on over
//! their double" as a rebase.

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain, Suit};
use pons::bidding::array::Logits;
use pons::bidding::constraint::{balanced, hcp, len, support};
use pons::bidding::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext};
use pons::bidding::trie::Forest;
use pons::bidding::{Rules, SeatClasses, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// Opening table for an unpassed hand
fn opening() -> Rules {
    Rules::new()
        .rule(Bid::new(1, Strain::Notrump), 1.0, hcp(15..=17) & balanced())
        .rule(
            Bid::new(1, Strain::Spades),
            1.0,
            hcp(11..=21) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.0,
            hcp(11..=21) & len(Suit::Hearts, 5..),
        )
        .rule(Call::Pass, 0.0, hcp(..11))
}

/// Opening table for a passed hand: the same book plus a light 1♠
fn light_opening() -> Rules {
    opening().rule(
        Bid::new(1, Strain::Spades),
        2.0,
        hcp(9..=21) & len(Suit::Spades, 5..),
    )
}

/// Responses to our 1♥ opening
fn heart_responses() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            1.0,
            support(4..) & hcp(10..=13),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.0, support(3..) & hcp(6..=9))
        .rule(
            Bid::new(1, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & hcp(6..),
        )
        .rule(Call::Pass, 0.0, hcp(..6))
}

/// Negative doubles over their overcall of our 1♥ opening
fn negative_doubles() -> Rules {
    Rules::new()
        .rule(Call::Double, 1.0, len(Suit::Spades, 4..) & hcp(8..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

fn demo_system() -> Forest {
    let mut forest = Forest::new();
    let one_h = call(1, Strain::Hearts);

    forest.insert(&[], SeatClasses::UNPASSED, opening());
    forest.insert(&[], SeatClasses::PASSED, light_opening());
    forest.insert(&[one_h, Call::Pass], SeatClasses::ALL, heart_responses());

    // Negative doubles through 2♠ over interference with our 1♥ opening.
    forest.fallback_at(
        &[one_h],
        SeatClasses::ALL,
        OvercallAtMost(Bid::new(2, Strain::Spades)),
        Fallback::classify(negative_doubles()),
    );
    // System on over their double.
    forest.fallback_at(
        &[one_h],
        SeatClasses::ALL,
        FirstIs(Call::Double),
        Fallback::rebase(ReplaceNext(Call::Pass)),
    );
    forest
}

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

/// 16 HCP balanced
const NOTRUMP_OPENER: &str = "AQ32.K53.QJ4.A92";
/// 13 HCP with five hearts
const HEART_OPENER: &str = "A2.KQJ53.Q42.J92";
/// 9 HCP with five spades — a light opening in 3rd/4th seat only
const LIGHT_OPENER: &str = "AQJ32.853.Q42.92";
/// 9 HCP with three hearts
const RAISER: &str = "Q32.J53.A964.Q92";
/// 10 HCP with four spades
const DOUBLER: &str = "KQ32.J5.A964.982";

#[test]
fn test_openings_shared_by_first_and_second_seat() {
    let system = demo_system();

    for leading in 0..=1 {
        let auction = &[Call::Pass; 1][..leading];
        assert_eq!(
            best_call(&system, auction, NOTRUMP_OPENER),
            call(1, Strain::Notrump),
        );
        assert_eq!(
            best_call(&system, auction, HEART_OPENER),
            call(1, Strain::Hearts),
        );
        // Too light for an unpassed opening.
        assert_eq!(best_call(&system, auction, LIGHT_OPENER), Call::Pass);
    }
}

#[test]
fn test_light_third_seat_opening() {
    let system = demo_system();
    let two_passes = [Call::Pass; 2];

    // The same 9-count opens 1♠ in 3rd seat thanks to the passed book,
    // where the light rule outweighs the pass rule.
    assert_eq!(
        best_call(&system, &two_passes, LIGHT_OPENER),
        call(1, Strain::Spades),
    );
    // Stronger openings are unchanged: the books share those rules.
    assert_eq!(
        best_call(&system, &two_passes, HEART_OPENER),
        call(1, Strain::Hearts),
    );
}

#[test]
fn test_responses_shared_across_opening_seats() {
    let system = demo_system();
    let one_h = call(1, Strain::Hearts);

    // 1st-seat opening: 1♥ - (P) - ?
    assert_eq!(
        best_call(&system, &[one_h, Call::Pass], RAISER),
        call(2, Strain::Hearts),
    );
    // 2nd-seat opening: (P) - 1♥ - (P) - ? hits the same node.
    assert_eq!(
        best_call(&system, &[Call::Pass, one_h, Call::Pass], RAISER),
        call(2, Strain::Hearts),
    );
}

#[test]
fn test_negative_double_package() {
    let system = demo_system();
    let one_h = call(1, Strain::Hearts);

    // 1♥ - (2♣) - ?: the package handles any overcall through 2♠.
    assert_eq!(
        best_call(&system, &[one_h, call(2, Strain::Clubs)], DOUBLER),
        Call::Double,
    );
    assert_eq!(
        best_call(&system, &[one_h, call(1, Strain::Spades)], RAISER),
        Call::Pass,
    );
}

#[test]
fn test_system_on_over_their_double() {
    let system = demo_system();
    let one_h = call(1, Strain::Hearts);

    // 1♥ - (X) - ?: the rebase maps onto the undisturbed responses.
    assert_eq!(
        best_call(&system, &[one_h, Call::Double], RAISER),
        call(2, Strain::Hearts),
    );
}
