//! Rubens (transfer) advances of partner's overcall
//!
//! Generalized Rubens advances: when a fit is likely we prefer transfer
//! responses, freeing the step just below partner's suit for the limit-plus
//! raise.  Over partner's natural overcall of suit `Z`, advancer's bid one
//! strain below `Z` is a **transfer raise** — limit-plus values with three-card
//! support — which partner completes by bidding `Z`.  The direct raise stays
//! natural (weak/competitive), so the two paths to a fit separate by strength.
//!
//! This module overlays those transfer raises onto the natural advances in the
//! defensive book ([`super::defense`]); [`super::Trie::insert_arc`] replaces the
//! natural advance classifier at each shared node.
//!
//! # Scope
//!
//! This authors the transfer *raise* of an overcall — the canonical Rubens
//! advance.  The fuller generalized scheme (transfers for lower new suits, and
//! the transfer advances of a takeout double) is left for a later pass; the
//! defensive book carries the instinct floor, so the auctions not authored here
//! still get a natural answer.

use super::{call, insert_all_seats};
use crate::bidding::constraint::{hcp, points, support};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// The suit one strain below `suit`, or [`None`] for clubs (the lowest)
const fn step_below(suit: Suit) -> Option<Suit> {
    match suit {
        Suit::Clubs => None,
        Suit::Diamonds => Some(Suit::Clubs),
        Suit::Hearts => Some(Suit::Diamonds),
        Suit::Spades => Some(Suit::Hearts),
    }
}

/// Advancer's Rubens raises of partner's overcall of `over` against `theirs`
///
/// `level` is the level of partner's overcall (1 over a higher-ranking suit, 2
/// over a lower one).  Beyond the natural simple raise and weak game raise, the
/// bid one strain below partner's suit (when one exists) is a limit-plus
/// transfer raise.
fn overcall_advances(over: Suit, level: u8) -> Rules {
    let o = Strain::from(over);

    let mut rules = Rules::new()
        // Weak game raise: a long trump fit, few points.
        .rule(Bid::new(4, o), 1.6, support(5..) & points(..6))
        // Natural simple raise: three-card support, constructive but not strong.
        .rule(Bid::new(level + 1, o), 1.0, support(3..) & points(6..=9))
        .rule(Call::Pass, 0.0, hcp(..6));

    // Transfer raise: the step below partner's suit, limit-plus with support.
    if let Some(below) = step_below(over) {
        rules = rules.rule(
            Bid::new(level + 1, Strain::from(below)),
            1.5,
            support(3..) & points(10..=12),
        );
    }
    rules
}

/// Partner's completion of advancer's transfer raise of `over`
///
/// Forcing — partner must complete the transfer.  A maximum overcall accepts to
/// game; otherwise partner bids the agreed suit at the cheapest level.
fn completion(over: Suit, level: u8) -> Rules {
    let o = Strain::from(over);
    Rules::new()
        // Accept to game with a maximum overcall.
        .rule(Bid::new(4, o), 1.0, points(14..))
        // Complete the transfer at the cheapest level (sign-off).
        .rule(Bid::new(level + 1, o), 0.5, hcp(0..))
}

/// Overlay Rubens transfer raises onto the defensive book
///
/// For every one-of-a-suit opening and every natural overcall over it, replaces
/// the advance node with [`overcall_advances`] and, where a transfer raise
/// exists, registers partner's [`completion`] node — both seat-fanned to match
/// the defensive book.
pub(super) fn register(book: &mut Trie) {
    for theirs in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let opening = Bid::new(1, Strain::from(theirs));
        let t = Strain::from(theirs);

        for over in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let o = Strain::from(over);
            if o == t {
                continue;
            }
            let level = if o > t { 1 } else { 2 };
            let overcall = call(level, o);

            // Replace the natural advance with the Rubens version.
            insert_all_seats(
                book,
                &[Call::Bid(opening), overcall, Call::Pass],
                3,
                overcall_advances(over, level),
            );

            // Partner's completion after the transfer raise, when one exists.
            if let Some(below) = step_below(over) {
                let transfer = call(level + 1, Strain::from(below));
                insert_all_seats(
                    book,
                    &[
                        Call::Bid(opening),
                        overcall,
                        Call::Pass,
                        transfer,
                        Call::Pass,
                    ],
                    3,
                    completion(over, level),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Rules;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::Hand;
    use contract_bridge::auction::RelativeVulnerability;

    fn best(rules: &Rules, auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let logits = rules.classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    // They open 1♥, partner overcalls 1♠, RHO passes — advancer to act.
    const ADVANCE: [Call; 3] = [call(1, Strain::Hearts), call(1, Strain::Spades), Call::Pass];

    #[test]
    fn limit_raise_transfers_below_partners_suit() {
        let r = overcall_advances(Suit::Spades, 1);
        // 3 spades, 11 HCP — limit-plus raise transfers via 2♥ (the cue).
        assert_eq!(
            best(&r, &ADVANCE, "KJ4.952.AQ32.J92"),
            call(2, Strain::Hearts)
        );
    }

    #[test]
    fn simple_raise_stays_natural() {
        let r = overcall_advances(Suit::Spades, 1);
        // 3 spades, 8 HCP — constructive simple raise to 2♠.
        assert_eq!(
            best(&r, &ADVANCE, "KJ4.952.Q832.Q92"),
            call(2, Strain::Spades)
        );
    }

    #[test]
    fn weak_hand_with_a_long_fit_jumps_to_game() {
        let r = overcall_advances(Suit::Spades, 1);
        // 5 spades, 5 HCP — weak game raise.
        assert_eq!(
            best(&r, &ADVANCE, "KQ542.952.832.92"),
            call(4, Strain::Spades)
        );
    }

    #[test]
    fn overcaller_completes_or_accepts() {
        let r = completion(Suit::Spades, 1);
        // After 1♥–1♠–P–2♥(transfer)–P, the overcaller is to act.
        let auction = [
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
        ];
        // Maximum overcall accepts to game.
        assert_eq!(
            best(&r, &auction, "AQJ52.K2.KQ32.92"),
            call(4, Strain::Spades)
        );
        // Minimum overcall completes the transfer.
        assert_eq!(
            best(&r, &auction, "KQ542.K2.Q832.92"),
            call(2, Strain::Spades)
        );
    }
}
