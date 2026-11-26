//! The Strawberry opening table (<https://polish.club/Openings.html>)
//!
//! | Call | Meaning |
//! | --- | --- |
//! | `1♣!` | 12–14 with 2–4 cards in every suit, or 11–16 with 5+♣ or 4=4=1=4, or 17+ unsuitable for `1♦` and `1NT` |
//! | `1♦` | 11–20, 5+♦ or unbalanced with exactly four diamonds and 1–5 clubs |
//! | `1♥`, `1♠` | 11–16, five-plus; 9–16 in third and fourth seat |
//! | `1NT` | 15–17, the house shape policy |
//! | `3x` | a seven-card preempt |
//!
//! The two-level openings (Ekren `2♣`, Multi `2♦`, Muiderberg `2M`, the
//! minors `2NT`), the gambling `3NT` and Namyats are **not authored yet**: a
//! hand for one of them passes or opens at the three level
//! (`docs/strawberry.md`).
//!
//! Ranges are in upgraded [`points`], as the book counts them, except the
//! balanced bands, which are plain HCP (a balanced hand takes no upgrade).
//! Precedence is by weight, so each rule states its own hand type and the
//! rules above it take what they describe better: `1NT` over everything, a
//! five-card major over a minor, `1♦` over `1♣`.

use crate::bidding::agreements::Agreements;
use crate::bidding::american::openings::with_one_notrump;
use crate::bidding::constraint::{and, balanced, hcp, len, nth_seat, points};
use crate::bidding::rows::{Package, Pattern, rows_of};
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The Polish `1♣`: three hand types in one forcing opening
pub(super) const POLISH_CLUB: Alert = Alert("polish-club");

/// The opening table, shared by every seat
#[must_use]
pub fn openings(agreements: &Agreements) -> Rules {
    let knobs = &agreements.opening;
    // The strong `1NT` is the house opening, rule and all: the book's shapes
    // (2–4 in the majors or 5M332, up to six in a minor) are the house
    // `Wide6322` policy, and the notrump structure below it is shared.
    let mut rules = with_one_notrump(Rules::new(), knobs.notrump_shape, knobs);

    // Five-card majors, limited to 16.  `1♠` ranks above `1♥` so 5-5 opens
    // the higher.  A balanced 11-count opens; so does a shapely 10.
    for (suit, weight) in [(Suit::Spades, 160), (Suit::Hearts, 150)] {
        rules = rules.rule(
            Bid::new(1, Strain::from(suit)),
            weight,
            points(11..=16) & hcp(10..) & len(suit, 5..) & (nth_seat(1) | nth_seat(2)),
        );
    }
    // Third and fourth seat go down to nine; a light `1♥` holds three spades
    // so that opener can pass `1♠`.
    rules = rules
        .rule(
            Bid::new(1, Strain::Spades),
            165,
            points(..=16) & hcp(9..) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            155,
            points(..=16)
                & (hcp(11..) | (hcp(9..) & len(Suit::Spades, 3..)))
                & len(Suit::Hearts, 5..)
                & (nth_seat(3) | nth_seat(4)),
        );

    // 1♦: five-plus diamonds (six-plus beside a six-card major), or exactly
    // four in an unbalanced hand with at most five clubs: the three 4441s
    // with four diamonds, (31)45, and 2=2=4=5 below 18.  (40)45 opens `1♣`.
    let long = (len(Suit::Diamonds, 5..) & and([Suit::Hearts, Suit::Spades], ..6))
        | len(Suit::Diamonds, 6..);
    let canape = len(Suit::Diamonds, 4..=4)
        & and([Suit::Hearts, Suit::Spades], 1..=4)
        & len(Suit::Clubs, 1..=5)
        & !balanced()
        & !(hcp(18..) & and([Suit::Hearts, Suit::Spades], 2..=2));
    rules = rules.rule(
        Bid::new(1, Strain::Diamonds),
        120,
        points(12..=20) & hcp(10..) & (long | canape),
    );

    // 1♣!: whatever the rules above did not take.  The three legs are the
    // book's three variants, so the reading is their union.
    let weak_notrump = hcp(12..=14) & and(Suit::ASC, 2..=4);
    let four_four_one_four = and([Suit::Spades, Suit::Hearts, Suit::Clubs], 4..=4);
    let clubs = points(12..=16)
        & hcp(10..)
        & and([Suit::Hearts, Suit::Spades], ..5)
        & (len(Suit::Clubs, 5..) | four_four_one_four);
    rules = rules
        .rule(
            Bid::new(1, Strain::Clubs),
            100,
            weak_notrump | clubs | points(17..),
        )
        .alert(POLISH_CLUB);

    // Three-level preempts (seven-card suit, not in fourth seat), as the
    // house plays them.
    for suit in Suit::ASC {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            90,
            len(suit, 7..) & points(..12) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0, points(..12))
}

/// The opening table as a row package: one node, fanned over the four seats
pub(super) fn package() -> Package {
    Package {
        name: "strawberry-openings",
        gate: |_| true,
        entries: |agreements| rows_of(Pattern::node("P*"), openings(agreements)),
    }
}
