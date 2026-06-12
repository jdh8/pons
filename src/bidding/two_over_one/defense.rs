//! Defense: our actions when they open

use super::{call, insert_all_seats};
use crate::bidding::constraint::{
    Cons, Constraint, balanced, hcp, len, pred, stopper_in_their_suits, support,
};
use crate::bidding::context::Context;
use crate::bidding::{Defensive, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

/// Takeout shape: at most three cards in each suit the opponents have bid
fn short_in_their_suits() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        context.their_suits().all(|suit| hand[suit].len() <= 3)
    })
}

/// Our action over their one-of-a-suit opening
///
/// One decision: a natural overcall (five-card suit), a takeout double, a
/// 15–18 1NT overcall, or pass.  Strong hands (17+) double first regardless of
/// shape, planning to bid again — otherwise an opening-strength hand with
/// length in the opponents' suit would be stuck.
#[must_use]
pub fn defense_to_suit(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let mut rules = Rules::new()
        .rule(
            Bid::new(1, Strain::Notrump),
            1.5,
            hcp(15..=18) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits())
        .rule(Call::Double, 1.2, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let level = if strain > theirs { 1 } else { 2 };
            let weight = if level == 1 { 1.4 } else { 1.0 };
            rules = rules.rule(
                Bid::new(level, strain),
                weight,
                len(suit, 5..) & hcp(8..=16),
            );
        }
    }
    rules
}

/// Our action over their 1NT opening: penalty double or natural two-level overcall
fn defense_to_notrump() -> Rules {
    let mut rules = Rules::new()
        .rule(Call::Double, 1.3, hcp(15..) & balanced())
        .rule(Call::Pass, 0.0, hcp(0..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(suit)),
            1.0,
            len(suit, 5..) & hcp(8..=14),
        );
    }
    rules
}

/// Advancer's raise of partner's natural overcall in `our_suit`
fn advances(our_suit: Suit) -> Rules {
    let s = Strain::from(our_suit);
    Rules::new()
        .rule(Bid::new(4, s), 1.6, support(5..) & hcp(..6))
        .rule(Bid::new(3, s), 1.4, support(3..) & hcp(11..=12))
        .rule(Bid::new(2, s), 1.0, support(3..) & hcp(6..=10))
        .rule(Call::Pass, 0.0, hcp(..6))
}

/// The defensive book: our actions over their openings, plus advances
///
/// Every key is fanned under `0..=3` leading passes — the defensive book
/// keys the raw table auction, so their opening may arrive after any mix of
/// leading passes (ours and theirs).
pub(super) fn defensive() -> Defensive {
    let mut d = Defensive::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(1, theirs);
        insert_all_seats(&mut d, &[Call::Bid(opening)], 3, defense_to_suit(opening));

        for our in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let strain = Strain::from(our);
            if strain != theirs {
                let level = if strain > theirs { 1 } else { 2 };
                let overcall = call(level, strain);
                insert_all_seats(
                    &mut d,
                    &[Call::Bid(opening), overcall, Call::Pass],
                    3,
                    advances(our),
                );
            }
        }
    }
    insert_all_seats(&mut d, &[call(1, Strain::Notrump)], 3, defense_to_notrump());
    d
}
