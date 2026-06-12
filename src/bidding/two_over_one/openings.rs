//! Uncontested openings for every seat

use super::insert_uncontested;
use crate::bidding::constraint::{Cons, Constraint, balanced, hcp, len, nth_seat, pred};
use crate::bidding::context::Context;
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

/// Better-minor selector: open 1♦ rather than 1♣
///
/// Open the longer minor; with equal length open 1♦ on four-or-more (the
/// standard 4-4 → 1♦, 3-3 → 1♣ split).
fn prefers_diamonds() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, _: &Context<'_>| {
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
#[must_use]
pub fn openings() -> Rules {
    let mut rules = Rules::new()
        // Strong, artificial 2♣ — top priority.
        .rule(Bid::new(2, Strain::Clubs), 3.0, hcp(22..))
        // Strong notrumps.
        .rule(Bid::new(1, Strain::Notrump), 2.0, hcp(15..=17) & balanced())
        .rule(Bid::new(2, Strain::Notrump), 2.0, hcp(20..=21) & balanced())
        // Five-card majors; 1♠ ranks just above 1♥ so 5-5 opens the higher.
        .rule(
            Bid::new(1, Strain::Spades),
            1.6,
            hcp(12..=21) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            hcp(12..=21) & len(Suit::Hearts, 5..),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            2.6,
            hcp(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            2.5,
            hcp(9..=11) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Better-minor openings (deny a five-card major).
        .rule(
            Bid::new(1, Strain::Diamonds),
            1.0,
            hcp(12..=21) & prefers_diamonds() & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(1, Strain::Clubs),
            1.0,
            hcp(12..=21)
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
            len(suit, 6..=6) & hcp(5..=10) & !nth_seat(4),
        );
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            0.9,
            len(suit, 7..) & hcp(..12) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0.0, hcp(..12))
}

/// Register the opening table in the constructive book
pub(super) fn register(book: &mut Trie) {
    insert_uncontested(book, &[], openings());
}
