//! First responses to our one-of-a-suit openings

use super::{call, insert_response};
use crate::bidding::constraint::{balanced, hcp, len, support};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// Responses to our `1♥`/`1♠` opening
///
/// The 2/1 core: a new suit at the two level is game forcing
/// (`hcp(13..)`), the forcing 1NT is the catch-all below it, raises are
/// graded by strength (single / limit / Jacoby 2NT / weak jump to game), and
/// over 1♥ a four-card spade suit takes the one level.
#[must_use]
pub fn major_responses(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Jacoby 2NT: game-forcing raise with four-card support.
        .rule(Bid::new(2, Strain::Notrump), 3.0, support(4..) & hcp(13..))
        // Limit raise.
        .rule(Bid::new(3, trump), 2.0, support(3..) & hcp(10..=12))
        // Weak jump to game: lots of trumps, few points.
        .rule(Bid::new(4, trump), 1.6, support(5..) & hcp(..6))
        // Single raise.
        .rule(Bid::new(2, trump), 1.5, support(3..) & hcp(6..=9))
        // Forcing 1NT: the catch-all when nothing more descriptive fits.
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(6..=12))
        .rule(Call::Pass, 0.0, hcp(..6));

    // 1♠ over 1♥: a new suit at the one level, preferred to a single raise.
    if major == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            1.7,
            len(Suit::Spades, 4..) & hcp(6..) & !support(4..),
        );
    }

    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.
    let mut weight = 1.1;
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(
                Bid::new(2, Strain::from(suit)),
                weight,
                len(suit, 4..) & hcp(13..) & !support(4..),
            );
            weight -= 0.05;
        }
    }
    rules
}

/// Responses to our `1♣`/`1♦` opening
///
/// Four-card majors up the line, a 2/1 game force (`1♦–2♣`), the notrump
/// ladder when no major fits, and simple (not inverted) minor raises promising
/// five-card support, since opener's minor may be only three cards.
#[must_use]
pub fn minor_responses(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    let mut rules = Rules::new()
        // Four-card majors up the line (hearts before spades).
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            len(Suit::Hearts, 4..) & hcp(6..),
        )
        .rule(
            Bid::new(1, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..) & hcp(6..) & len(Suit::Hearts, ..4),
        )
        // Notrump ladder without a four-card major (3NT open-ended for game-plus).
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(13..) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(11..=12) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            hcp(6..=10) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // Simple minor raises (five-card support).
        .rule(Bid::new(3, trump), 1.2, support(5..) & hcp(10..))
        .rule(Bid::new(2, trump), 1.1, support(5..) & hcp(6..=9))
        .rule(Call::Pass, 0.0, hcp(..6));

    // 2/1 game force: 1♦–2♣ (clubs are cheaper than diamonds).
    if minor == Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::Clubs),
            1.3,
            len(Suit::Clubs, 4..) & hcp(13..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        );
    }
    rules
}

/// Register the first responses to every one-of-a-suit opening
pub(super) fn register(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        insert_response(book, call(1, Strain::from(major)), major_responses(major));
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        insert_response(book, call(1, Strain::from(minor)), minor_responses(minor));
    }
}
