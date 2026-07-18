//! Dutch openings — the wide, non-forcing 1♣ table (Phase 1)
//!
//! Diverges from american in the one-level suit partition and the strong 2♣;
//! the 1NT/2NT/weak-two/preempt rows are held at american's defaults for now
//! (Phase 3 replaces the 2-level rows with Multi/Muiderberg/UNT).  See
//! `docs/dutch-system.md` for the full spec.

use crate::bidding::american::{NotrumpShape, notrump_shape};
use crate::bidding::constraint::{balanced, fifths, hcp, len, nth_seat, or, points, rule_of_20};
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The strong, artificial 2♣ opening — the only artificial Dutch opening
const STRONG_2C: Alert = Alert("strong-2c");

/// The Dutch opening table, shared by every seat
///
/// Wide, non-forcing 1♣ (2+♣, ≤4♦, 11–23); natural 1♦ (5+♦ or the singleton-club
/// 4=4=4=1); five-card majors 10–20; the strong artificial 2♣ (21–23 with a
/// five-card major or six-card minor, or any 24+).  Strong balanced 20–21 still
/// opens 2NT here — the wide 1♣ only hosts 22–23 balanced until Phase 3 turns
/// 2NT into UNT.
///
/// Sharp on shape, and Rule-of-20 sound: a one-level opening needs its HCP band
/// **and** raw HCP + two longest suits ≥ 20, so a flat sub-Rule-of-20 minimum
/// (e.g. a 4-3-3-3 twelve-count) passes.  The finite `Pass` catch-all keeps the
/// table total.
pub(super) fn dutch_openings() -> Rules {
    let majors = [Suit::Hearts, Suit::Spades];
    let minors = [Suit::Clubs, Suit::Diamonds];
    let mut rules = Rules::new()
        // Strong, artificial 2♣ — top priority: 21–23 with a five-card major or
        // six-card minor, or any 24+.
        .rule(
            Bid::new(2, Strain::Clubs),
            3.0,
            (hcp(21..=23) & (or(majors, 5..) | or(minors, 6..))) | hcp(24..),
        )
        .alert(STRONG_2C)
        // Strong 1NT — 15–17, american's wide shape: balanced, or a 5422/6322
        // with a long minor ([`NotrumpShape::Wide6322`], american's default).
        .rule(
            Bid::new(1, Strain::Notrump),
            2.0,
            hcp(15..=17) & notrump_shape(NotrumpShape::Wide6322),
        )
        // Strong 2NT — balanced 20–21 (Phase 1 placeholder; Phase 3 → UNT).
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            fifths(20.0..22.0) & balanced(),
        )
        // Five-card majors, 10–20 HCP, Rule of 20; 1♠ ranks above 1♥ so 5-5 opens
        // the higher.
        .rule(
            Bid::new(1, Strain::Spades),
            1.6,
            hcp(10..=20) & rule_of_20() & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            hcp(10..=20) & rule_of_20() & len(Suit::Hearts, 5..),
        )
        // 1♦ — 5+♦, or exactly the singleton-club 4=4=4=1 (`≤1♣` with no five-card
        // major forces four diamonds).  No five-card major; 11–23, Rule of 20.
        .rule(
            Bid::new(1, Strain::Diamonds),
            1.0,
            hcp(11..=23)
                & rule_of_20()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5)
                & (len(Suit::Diamonds, 5..) | len(Suit::Clubs, ..=1)),
        )
        // 1♣ — the wide catch-all: 2+♣, ≤4♦ (deny 5+♦), no five-card major;
        // 11–23, Rule of 20.  Soaks up every four-diamond hand but the 4=4=4=1.
        .rule(
            Bid::new(1, Strain::Clubs),
            1.0,
            hcp(11..=23)
                & rule_of_20()
                & len(Suit::Clubs, 2..)
                & len(Suit::Diamonds, ..5)
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        );
    // Weak twos (Phase 1: american defaults; Phase 3 → Multi 2♦ / Muiderberg 2M).
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(suit)),
            1.0,
            len(suit, 6..=6) & points(5..=10) & !nth_seat(4),
        );
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            0.9,
            len(suit, 7..) & points(..12) & !nth_seat(4),
        );
    }
    // Finite, total catch-all.
    rules.rule(Call::Pass, 0.0, hcp(0..))
}
