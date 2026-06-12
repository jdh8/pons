//! The 1NT response structure: Stayman, Jacoby transfers, and completions

use super::{call, insert_all_seats, insert_response};
use crate::bidding::constraint::{hcp, len};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// Responses to our 1NT opening: Stayman, Jacoby transfers, and notrump raises
#[must_use]
pub fn notrump_responses() -> Rules {
    Rules::new()
        // Jacoby transfers, any strength.
        .rule(Bid::new(2, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        // Stayman: a four-card major and at least invitational values.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..),
        )
        // Natural notrump raises (no five-card major — that would transfer).
        // 3NT is open-ended: a strong balanced hand bids game and leaves slam
        // exploration to a later pass rather than being stranded without a call.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(10..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(8..=9) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Call::Pass,
            0.0,
            hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Opener's answer to Stayman: a four-card major, else 2♦
fn stayman_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a Jacoby transfer by bidding the anchor suit
fn complete_transfer(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(2, Strain::from(into)), 1.0, hcp(0..))
}

/// Register the 1NT response structure and its continuations
pub(super) fn register(book: &mut Trie) {
    let p = Call::Pass;
    let one_nt = call(1, Strain::Notrump);

    insert_response(book, one_nt, notrump_responses());

    // 1NT continuations: Stayman answers and transfer completions.
    insert_all_seats(
        book,
        &[one_nt, p, call(2, Strain::Clubs), p],
        2,
        stayman_answers(),
    );
    insert_all_seats(
        book,
        &[one_nt, p, call(2, Strain::Diamonds), p],
        2,
        complete_transfer(Suit::Hearts),
    );
    insert_all_seats(
        book,
        &[one_nt, p, call(2, Strain::Hearts), p],
        2,
        complete_transfer(Suit::Spades),
    );
}
