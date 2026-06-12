//! Opener's rebids (one round)

use super::{call, insert_uncontested};
use crate::bidding::constraint::{balanced, hcp, len, support};
use crate::bidding::{Rules, Trie};
use contract_bridge::{Bid, Strain, Suit};

/// Opener's rebid after `1♥ – 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener — there is no pass rule.
fn rebid_one_heart_one_spade() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 2.6, support(4..) & hcp(19..))
        .rule(
            Bid::new(3, Strain::Spades),
            2.2,
            support(4..) & hcp(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.8,
            support(4..) & hcp(12..=15),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.4, len(Suit::Hearts, 6..))
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Diamonds), 0.9, len(Suit::Diamonds, 4..))
        // Balanced minimum, and the guaranteed-legal fallback.
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14))
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Opener's rebid after `1M – 1NT` (the forcing notrump)
///
/// Forcing on opener.  A five-card-major rebid is the guaranteed-legal
/// fallback when nothing more descriptive fits — a basic simplification.
fn rebid_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(Bid::new(2, trump), 1.0, len(major, 6..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(Bid::new(2, Strain::from(suit)), 0.9, len(suit, 4..));
        }
    }
    // Opener always holds at least five of the major, so this always applies.
    rules.rule(Bid::new(2, trump), 0.3, len(major, 5..))
}

/// Opener's rebid raising responder's new major after a minor opening
///
/// Used at `1m – 1M`.  Forcing on opener; a 1NT rebid is the guaranteed-legal
/// fallback.
fn rebid_raise_major(responder_major: Suit, opener_minor: Suit) -> Rules {
    let m = Strain::from(responder_major);
    Rules::new()
        .rule(Bid::new(4, m), 2.6, support(4..) & hcp(19..))
        .rule(Bid::new(3, m), 2.2, support(4..) & hcp(16..=18))
        .rule(Bid::new(2, m), 1.8, support(4..) & hcp(12..=15))
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(
            Bid::new(2, Strain::from(opener_minor)),
            0.9,
            len(opener_minor, 5..),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Opener's rebid after `1♣ – 1♦`
fn rebid_one_club_one_diamond() -> Rules {
    Rules::new()
        .rule(Bid::new(1, Strain::Hearts), 1.3, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(1, Strain::Spades),
            1.3,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.5,
            support(4..) & hcp(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.2,
            support(4..) & hcp(12..=15),
        )
        .rule(Bid::new(2, Strain::Notrump), 1.1, hcp(18..=19) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Register opener's rebids after a one-level new suit and the forcing 1NT
pub(super) fn register(book: &mut Trie) {
    insert_uncontested(
        book,
        &[call(1, Strain::Hearts), call(1, Strain::Spades)],
        rebid_one_heart_one_spade(),
    );
    for major in [Suit::Hearts, Suit::Spades] {
        insert_uncontested(
            book,
            &[call(1, Strain::from(major)), call(1, Strain::Notrump)],
            rebid_after_forcing_notrump(major),
        );
    }
    insert_uncontested(
        book,
        &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
        rebid_one_club_one_diamond(),
    );
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for responder_major in [Suit::Hearts, Suit::Spades] {
            insert_uncontested(
                book,
                &[
                    call(1, Strain::from(minor)),
                    call(1, Strain::from(responder_major)),
                ],
                rebid_raise_major(responder_major, minor),
            );
        }
    }
}
