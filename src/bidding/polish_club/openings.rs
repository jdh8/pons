//! Uncontested Strawberry Polish Club openings for every seat
//!
//! Authored from the master table in [`src/Openings.md`](https://github.com/jdh8/polish.club/blob/HEAD/src/Openings.md).
//! Strength gauges follow the notes: suit openings count upgraded [`points`]
//! with an HCP ceiling that routes strong hands to the artificial 1♣; the
//! notrump and the Ekren strong variant count [`hcp`].

use super::insert_uncontested;
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, hcp, len, nth_seat, points, vulnerable,
};
use crate::bidding::context::Context;
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

/// The artificial forcing 1♣: Quasi-natural clubs, the weak balanced minimum,
/// or any strong hand
///
/// Three variants from the notes:
/// - 12–14, 2–4 in every suit (the weak balanced minimum);
/// - 11–17, 5+♣ or the 4=♠ 4=♥ 1=♦ 4=♣ shape;
/// - 18+ that is not suitable for 1♦ (no five-card diamond suit), or anything 21+.
fn one_club() -> Cons<impl Constraint + Clone> {
    // 12–14, balanced, 2–4 in every suit (no five-card suit).
    let weak_balanced = hcp(12..=14)
        & balanced()
        & len(Suit::Spades, ..5)
        & len(Suit::Hearts, ..5)
        & len(Suit::Diamonds, ..5)
        & len(Suit::Clubs, ..5);
    // 11–17 natural clubs: 5+♣ and no five-card major.
    let natural_clubs = points(11..)
        & hcp(..=17)
        & len(Suit::Clubs, 5..)
        & len(Suit::Spades, ..5)
        & len(Suit::Hearts, ..5);
    // 11–17, the 4=♠ 4=♥ 1=♦ 4=♣ shape (4414).
    let four_four_one_four = points(11..)
        & hcp(..=17)
        & len(Suit::Spades, 4..=4)
        & len(Suit::Hearts, 4..=4)
        & len(Suit::Diamonds, 1..=1)
        & len(Suit::Clubs, 4..=4);
    // 18+ unsuitable for 1♦ (no five-card diamond suit), or anything 21+.
    let strong = (hcp(18..) & len(Suit::Diamonds, ..5)) | hcp(21..);
    weak_balanced | natural_clubs | four_four_one_four | strong
}

/// The natural 1♦: 5+♦, or unbalanced with exactly four diamonds, 1–5 clubs,
/// and no four-card major
fn one_diamond() -> Cons<impl Constraint + Clone> {
    let five_card = len(Suit::Diamonds, 5..);
    let four_card = len(Suit::Diamonds, 4..=4)
        & !balanced()
        & len(Suit::Clubs, 1..=5)
        & len(Suit::Spades, ..4)
        & len(Suit::Hearts, ..4);
    points(11..)
        & hcp(..=20)
        & len(Suit::Spades, ..5)
        & len(Suit::Hearts, ..5)
        & (five_card | four_card)
}

/// Ekren 2♣: a both-majors preempt, or the strong 5–6♠ 4=♥ variant
fn ekren() -> Cons<impl Constraint + Clone> {
    // The vulnerable exclusion: not a flat 4=♠ 4=♥ with a 3–2 minor split.
    let flat_4432 = described(
        "4=♠ 4=♥ with a 3–2 minor split",
        |hand: Hand, _: &Context<'_>| {
            hand[Suit::Spades].len() == 4 && hand[Suit::Hearts].len() == 4 && {
                let clubs = hand[Suit::Clubs].len();
                let diamonds = hand[Suit::Diamonds].len();
                (clubs == 3 && diamonds == 2) || (clubs == 2 && diamonds == 3)
            }
        },
    );
    let preempt = points(4..=10)
        & len(Suit::Spades, 4..)
        & len(Suit::Hearts, 4..)
        & !nth_seat(4)
        & !(vulnerable() & flat_4432);
    let strong = hcp(15..=17) & len(Suit::Spades, 5..=6) & len(Suit::Hearts, 4..=4);
    preempt | strong
}

/// Multi 2♦: a weak two in exactly one major
fn multi() -> Cons<impl Constraint + Clone> {
    points(4..=10)
        & !nth_seat(4)
        & described("a six-card major suit", |hand: Hand, _: &Context<'_>| {
            (hand[Suit::Spades].len() >= 6) ^ (hand[Suit::Hearts].len() >= 6)
        })
}

/// The opening table, shared by every seat
///
/// Weights order the overlapping calls: 1NT (15–17 balanced envelope) outranks
/// the natural 1♦, which outranks the artificial 1♣ on the shared four-diamond
/// hands; the Ekren strong variant outranks the five-card-major openings on a
/// 15–17 5–6♠ 4=♥; the light third/fourth-seat majors outrank a pass.
#[must_use]
pub fn openings() -> Rules {
    let mut rules = Rules::new()
        // Ekren 2♣ — its strong variant must outrank a 1♠/1NT opening.
        .rule(Bid::new(2, Strain::Clubs), 2.2, ekren())
        .note("Ekren: both majors, or strong 5–6♠ 4=♥")
        // 1NT outranks 1♦ on the shared balanced four-diamond hands.
        .rule(
            Bid::new(1, Strain::Notrump),
            2.0,
            hcp(15..=17)
                & len(Suit::Spades, 2..=5)
                & len(Suit::Hearts, 2..=5)
                & len(Suit::Diamonds, 2..=6)
                & len(Suit::Clubs, 2..=6),
        )
        // 1♦ outranks 1♣ on the shared (xx)45-style four-diamond hands.
        .rule(Bid::new(1, Strain::Diamonds), 1.8, one_diamond())
        // The artificial forcing 1♣.
        .rule(Bid::new(1, Strain::Clubs), 1.6, one_club())
        .note("Polish Club: quasi-natural clubs, weak balanced, or strong")
        // Five-card majors; 1♠ ranks above 1♥ so 5-5 opens the higher.
        .rule(
            Bid::new(1, Strain::Spades),
            1.5,
            points(11..) & hcp(..=17) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.4,
            points(11..) & hcp(..=17) & len(Suit::Hearts, 5..),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            2.5,
            points(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            2.4,
            points(9..=11) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Multi 2♦ — a weak two in a major.
        .rule(Bid::new(2, Strain::Diamonds), 1.0, multi())
        .note("Multi: a weak two in one major")
        // Unusual 2NT — 5+♦ and 5+♣.
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            points(4..=10) & !nth_seat(4) & len(Suit::Diamonds, 5..) & len(Suit::Clubs, 5..),
        )
        .note("unusual: 5+♦ and 5+♣");

    // Muiderberg 2♥/2♠: exactly five in the major and a four-card minor.
    for major in [Suit::Hearts, Suit::Spades] {
        rules = rules
            .rule(
                Bid::new(2, Strain::from(major)),
                1.0,
                points(4..=10)
                    & !nth_seat(4)
                    & len(major, 5..=5)
                    & (len(Suit::Clubs, 4..) | len(Suit::Diamonds, 4..)),
            )
            .note("Muiderberg: five in the major and a 4+ card minor");
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            0.9,
            len(suit, 7..) & points(..11) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0.0, points(..12))
}

/// Register the opening table in the constructive book
pub(super) fn register(book: &mut Trie) {
    insert_uncontested(book, &[], openings());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    /// The highest-logit call the opening table makes for a hand
    fn best(auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let logits = openings().classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    #[test]
    fn openings_pick_the_descriptive_bid() {
        // Strong balanced and natural clubs both open 1♣.
        assert_eq!(best(&[], "AQ5.KJ4.KQ72.K43"), bid(1, Strain::Clubs)); // 18 bal
        assert_eq!(best(&[], "43.K43.Q82.AKJ95"), bid(1, Strain::Clubs)); // 5♣
        // Strong notrump, natural diamond, five-card majors.
        assert_eq!(best(&[], "KJ4.AQ5.Q872.K32"), bid(1, Strain::Notrump)); // 15 bal
        assert_eq!(best(&[], "K3.842.AQJ95.KJ3"), bid(1, Strain::Diamonds)); // 5♦
        assert_eq!(best(&[], "K3.AQ952.KJ3.842"), bid(1, Strain::Hearts)); // 5♥
        assert_eq!(best(&[], "AQ952.K3.KJ3.842"), bid(1, Strain::Spades)); // 5♠
        // Preemptive two-level.
        assert_eq!(best(&[], "KQJ976.43.852.42"), bid(2, Strain::Diamonds)); // Multi
    }

    #[test]
    fn weak_balanced_opens_one_club() {
        // 13 HCP, 4-3-3-3: the weak balanced minimum opens 1♣ (not 1♦/1NT).
        assert_eq!(best(&[], "AQ54.K53.QJ4.T92"), bid(1, Strain::Clubs));
    }

    #[test]
    fn muiderberg_needs_exactly_five_in_the_major() {
        // 5♠ + 4♣, weak: Muiderberg 2♠.
        assert_eq!(best(&[], "KQ982.43.85.A872"), bid(2, Strain::Spades));
        // 6♠, weak: Multi 2♦, not Muiderberg.
        assert_eq!(best(&[], "KQJ976.43.852.42"), bid(2, Strain::Diamonds));
    }

    #[test]
    fn unusual_two_notrump_shows_both_minors() {
        // 5♦ 5♣, weak: 2NT.
        assert_eq!(best(&[], "32.3.QJ987.QJ852"), bid(2, Strain::Notrump));
    }

    #[test]
    fn fourth_seat_suppresses_preempts() {
        // A weak six-spade hand opens Multi in first seat but passes in fourth.
        assert_eq!(best(&[], "KQJ976.43.852.42"), bid(2, Strain::Diamonds));
        assert_eq!(best(&[Call::Pass; 3], "KQJ976.43.852.42"), Call::Pass);
    }
}
