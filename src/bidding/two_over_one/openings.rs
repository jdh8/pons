//! Uncontested openings for every seat

use super::insert_uncontested;
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, fifths, len, nth_seat, points,
};
use crate::bidding::context::Context;
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

/// Shapes eligible for a 1NT opening
///
/// Always the balanced patterns (4333/4432/5332).  With the wide redesign
/// (`wide`) it also admits a 5422 whose five-card suit is a minor — a five-card
/// major prefers a one-of-a-major opening it can rebid, so it is left out.
/// Strength ([`fifths`] 15–17) and the inference side are untouched; this is the
/// shape-only A/B knob for the deferred redesign (see the `nt-shape-abc` and
/// `nt-shape-contested` examples).
fn notrump_shape(wide: bool) -> Cons<impl Constraint + Clone> {
    balanced()
        | described("wide 1NT shape", move |hand: Hand, _: &Context<'_>| {
            if !wide {
                return false;
            }
            let mut lengths = Suit::ASC.map(|suit| hand[suit].len());
            lengths.sort_unstable();
            lengths == [2, 2, 4, 5]
                && (hand[Suit::Clubs].len() == 5 || hand[Suit::Diamonds].len() == 5)
        })
}

/// Better-minor selector: open 1♦ rather than 1♣
///
/// Open the longer minor; with equal length open 1♦ on four-or-more (the
/// standard 4-4 → 1♦, 3-3 → 1♣ split).
fn prefers_diamonds() -> Cons<impl Constraint + Clone> {
    described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        let clubs = hand[Suit::Clubs].len();
        let diamonds = hand[Suit::Diamonds].len();
        diamonds > clubs || (diamonds == clubs && diamonds >= 4)
    })
}

/// The opening table, shared by every seat
///
/// Strong notrumps (15–17 / 20–21), the artificial 2♣ (22+), five-card majors,
/// better-minor one-of-a-minor openings, weak twos, and three-level preempts.
/// A lighter five-card major is allowed in third and fourth seat.
///
/// Sharp on shape, fuzzy on strength: suit openings gauge upgraded
/// [`points`], notrump ranges gauge [`fifths`].  A clean shapely maximum
/// upgrades out of a weak two — it is too good for one.
#[must_use]
pub fn openings() -> Rules {
    openings_with(false)
}

/// [`openings`] with the wide 1NT shape selectable
///
/// `wide` is the deferred shape-only redesign: a 5422 with a five-card minor
/// also opens 1NT.  `openings()` is `openings_with(false)`, the shipped table.
#[must_use]
pub fn openings_with(wide: bool) -> Rules {
    let mut rules = Rules::new()
        // Strong, artificial 2♣ — top priority.
        .rule(Bid::new(2, Strain::Clubs), 3.0, points(22..))
        // Strong notrumps.
        .rule(
            Bid::new(1, Strain::Notrump),
            2.0,
            fifths(15.0..18.0) & notrump_shape(wide),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            fifths(20.0..22.0) & balanced(),
        )
        // Five-card majors; 1♠ ranks just above 1♥ so 5-5 opens the higher.
        .rule(
            Bid::new(1, Strain::Spades),
            1.6,
            points(12..=21) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            points(12..=21) & len(Suit::Hearts, 5..),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            2.6,
            points(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            2.5,
            points(9..=11) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Better-minor openings (deny a five-card major).
        .rule(
            Bid::new(1, Strain::Diamonds),
            1.0,
            points(12..=21) & prefers_diamonds() & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(1, Strain::Clubs),
            1.0,
            points(12..=21)
                & len(Suit::Clubs, 3..)
                & !prefers_diamonds()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        );

    // Weak twos (six-card suit, not in fourth seat).
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
    rules.rule(Call::Pass, 0.0, points(..12))
}

/// Register the opening table in the constructive book
pub(super) fn register(book: &mut Trie, wide: bool) {
    insert_uncontested(book, &[], openings_with(wide));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    /// The highest-logit opening `rules` makes for a hand
    fn opens(rules: &Rules, hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let logits = rules.classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a): &(Call, &f32), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    #[test]
    fn wide_notrump_shape_gate() {
        let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
        let one_s = Call::Bid(Bid::new(1, Strain::Spades));
        let one_c = Call::Bid(Bid::new(1, Strain::Clubs));
        // 5422 / 6322, ~15–17 fifths, long suit a minor (joins the wide 1NT) or a
        // major (stays a suit); the long-minor 6322 also stays a suit.
        let five422_minor = "Q432.KQ.K2.AK432";
        let five422_major = "AK432.KQ.Q432.K2";
        let six322_minor = "Q2.K3.AQ4.KQ8765";
        let six322_major = "KQ8765.K3.AQ4.Q2";
        let balanced16 = "AQ32.K53.QJ4.A92";

        // Classic: only the balanced hand opens 1NT; the shapely ones open a suit.
        let narrow = openings_with(false);
        assert_eq!(opens(&narrow, balanced16), one_nt);
        assert_eq!(opens(&narrow, five422_minor), one_c);
        assert_eq!(opens(&narrow, five422_major), one_s);
        assert_eq!(opens(&narrow, six322_minor), one_c);
        assert_eq!(opens(&narrow, six322_major), one_s);

        // Redesign: only the long-minor 5422 joins 1NT; majors and 6322 stay suits.
        let wide = openings_with(true);
        assert_eq!(opens(&wide, balanced16), one_nt);
        assert_eq!(opens(&wide, five422_minor), one_nt);
        assert_eq!(opens(&wide, five422_major), one_s);
        assert_eq!(opens(&wide, six322_minor), one_c);
        assert_eq!(opens(&wide, six322_major), one_s);
    }
}
