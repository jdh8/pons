//! A thin 2/1-flavored system slice as living documentation
//!
//! This wires every layer together: constraint-built [`Rules`] in a
//! [`Constructive`] book (openings authored per seat by their leading passes,
//! a lighter 3rd/4th-seat opening selected with [`nth_seat`]), our competitive
//! bidding over interference as guarded fallbacks (a negative-double package
//! and "system on over their double" as a rebase), a small [`Defensive`] book
//! of overcalls, and the two paired into a [`Partnership`].

use contract_bridge::auction::{Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Strain, Suit};
use pons::bidding::array::Logits;
use pons::bidding::constraint::{balanced, hcp, len, nth_seat, support};
use pons::bidding::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext};
use pons::bidding::{Constructive, Defensive, Partnership, Rules, System};

const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// One opening table for every seat
///
/// The strong rules apply in any seat; the light 1♠ is enabled only in 3rd/4th
/// seat, where [`nth_seat`] admits it.  Inserting this at each seat's
/// leading-pass prefix is what replaces the old passed/unpassed split.
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
        .rule(
            Bid::new(1, Strain::Spades),
            2.0,
            hcp(9..=21) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(Call::Pass, 0.0, hcp(..11))
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

/// A 1♠ overcall over their 1♣ opening
fn overcalls() -> Rules {
    Rules::new()
        .rule(
            Bid::new(1, Strain::Spades),
            1.0,
            len(Suit::Spades, 5..) & hcp(8..),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

fn demo_system() -> Partnership {
    let mut constructive = Constructive::new();
    let one_h = call(1, Strain::Hearts);

    // Openings: one table, authored explicitly at each seat's leading-pass
    // prefix — [], [P], [P, P], [P, P, P] for 1st through 4th seat.
    let passes = [Call::Pass; 3];
    for seat in 0..=3 {
        constructive.insert(&passes[..seat], opening());
    }

    // Responses to our 1♥, authored for the 1st- and 2nd-seat openings.
    constructive.insert(&[one_h, Call::Pass], heart_responses());
    constructive.insert(&[Call::Pass, one_h, Call::Pass], heart_responses());

    // Negative doubles through 2♠ over interference with our 1♥ opening.
    constructive.fallback_at(
        &[one_h],
        OvercallAtMost(Bid::new(2, Strain::Spades)),
        Fallback::classify(negative_doubles()),
    );
    // System on over their double.
    constructive.fallback_at(
        &[one_h],
        FirstIs(Call::Double),
        Fallback::rebase(ReplaceNext(Call::Pass)),
    );

    // A small defensive book: a 1♠ overcall over their 1♣ opening.
    let mut defensive = Defensive::new();
    defensive.insert(&[call(1, Strain::Clubs)], overcalls());

    Partnership::new(constructive, defensive)
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
fn test_first_and_second_seat_openings_match() {
    let system = demo_system();

    // Each seat is authored explicitly, but the same opening table makes 1st
    // and 2nd seat agree.
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

    // The same 9-count opens 1♠ in 3rd seat: nth_seat(3) enables the light rule,
    // whose higher weight outvotes the pass rule.
    assert_eq!(
        best_call(&system, &two_passes, LIGHT_OPENER),
        call(1, Strain::Spades),
    );
    // Stronger openings are unchanged: the same table covers every seat.
    assert_eq!(
        best_call(&system, &two_passes, HEART_OPENER),
        call(1, Strain::Hearts),
    );
}

#[test]
fn test_responses_authored_for_each_opening_seat() {
    let system = demo_system();
    let one_h = call(1, Strain::Hearts);

    // 1st-seat opening: 1♥ - (P) - ?
    assert_eq!(
        best_call(&system, &[one_h, Call::Pass], RAISER),
        call(2, Strain::Hearts),
    );
    // 2nd-seat opening: (P) - 1♥ - (P) - ?, authored at its own prefix.
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

#[test]
fn test_defensive_overcall_when_they_open() {
    let system = demo_system();

    // (1♣) - ?: the auction routes to the defensive book, where a 9-count with
    // five spades overcalls 1♠.
    assert_eq!(
        best_call(&system, &[call(1, Strain::Clubs)], LIGHT_OPENER),
        call(1, Strain::Spades),
    );
}
