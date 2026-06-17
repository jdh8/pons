//! First responses for the Strawberry Polish Club
//!
//! A backbone authored from the chapter sources (`src/1C.md`, `src/1D.md`,
//! `src/1H.md`, `src/1S.md`).  The artificial 1♣ response framework (the
//! negative 1♦ relay and the positives) is the defining slice; the natural
//! responses to 1♦/1♥/1♠ mirror standard practice, and the 15–17 1NT reuses the
//! verified [`american`][crate::bidding::american] notrump responses
//! (Stayman and Jacoby transfers) — the BTU response set is a later refinement.
//!
//! The deep relay tails (Checkback Gladiator, Odwrotka, the strong-club rebid
//! relays, and the preempt continuations) are left to the
//! [`instinct`][crate::bidding::instinct()] floor.  Forcing is by omission: the
//! 1♣ response node carries no [`Pass`][Call::Pass] rule, so the artificial
//! opening can never be passed out.

use super::{call, insert_uncontested};
use crate::bidding::american::notrump_responses;
use crate::bidding::constraint::{balanced, hcp, len, points, support};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};

/// Responses to the artificial forcing 1♣ (no pass — forcing by omission)
///
/// Positives show a real suit (the 2♣/2♦ game forces are swapped so the club
/// fit saves space); the 1♦ rebid is the artificial negative-or-relay that
/// catches everything else (0–11, or a balanced 16+ with no four-card major).
fn one_club_responses() -> Rules {
    Rules::new()
        // One-level positives: 7+ with a four-card major, up the line.
        .rule(
            Bid::new(1, Strain::Hearts),
            1.4,
            points(7..) & len(Suit::Hearts, 4..),
        )
        .note("F, 7+, 4+♥")
        .rule(
            Bid::new(1, Strain::Spades),
            1.3,
            points(7..) & len(Suit::Spades, 4..),
        )
        .note("F, 7+, 4+♠")
        // Game-forcing minor positives (swapped 2♣/2♦).
        .rule(
            Bid::new(2, Strain::Clubs),
            1.2,
            points(12..) & len(Suit::Diamonds, 5..),
        )
        .note("FG, 5+♦")
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.2,
            points(12..) & len(Suit::Clubs, 5..),
        )
        .note("FG, 5+♣")
        // Balanced invitations and the balanced game force.
        .rule(Bid::new(1, Strain::Notrump), 0.9, hcp(8..=10) & balanced())
        .note("BAL CONST, 8–10")
        .rule(Bid::new(2, Strain::Notrump), 1.0, hcp(10..=11) & balanced())
        .note("BAL INV, 10–11")
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.1,
            hcp(12..=15) & balanced() & len(Suit::Spades, ..4) & len(Suit::Hearts, ..4),
        )
        .note("BAL FG, 12–15")
        // Single-suited minor invitations.
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            points(9..=11) & len(Suit::Diamonds, 6..),
        )
        .note("INV, 9–11, 6+♦")
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            points(9..=11) & len(Suit::Clubs, 6..),
        )
        .note("INV, 9–11, 6+♣")
        // The artificial negative / relay: everything else.
        .rule(
            Bid::new(1, Strain::Diamonds),
            0.5,
            points(..7)
                | (points(7..=11) & len(Suit::Spades, ..4) & len(Suit::Hearts, ..4))
                | (hcp(16..) & balanced() & len(Suit::Spades, ..4) & len(Suit::Hearts, ..4)),
        )
        .note("(R) negative: 0–11, or balanced 16+")
}

/// Natural responses to 1♦ (5+♦, or unbalanced four diamonds)
fn one_diamond_responses() -> Rules {
    Rules::new()
        .rule(
            Bid::new(1, Strain::Hearts),
            1.2,
            points(6..) & len(Suit::Hearts, 4..),
        )
        .note("F, 4+♥")
        .rule(
            Bid::new(1, Strain::Spades),
            1.1,
            points(6..) & len(Suit::Spades, 4..),
        )
        .note("F, 4+♠")
        .rule(
            Bid::new(2, Strain::Clubs),
            1.3,
            points(12..) & (len(Suit::Clubs, 4..) | len(Suit::Diamonds, 4..)),
        )
        .note("FG, 4+♣ or 4+♦")
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.0,
            support(4..) & points(6..=8),
        )
        .note("CONST, 5–9, 4+♦")
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.05,
            support(4..) & points(9..=11),
        )
        .note("INV, 9–11, 4+♦")
        .rule(Bid::new(2, Strain::Notrump), 1.1, hcp(10..=11) & balanced())
        .note("BAL INV, 10–11")
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(11..=15) & balanced() & len(Suit::Spades, ..4) & len(Suit::Hearts, ..4),
        )
        .note("11–15, 1–3♠, 1–3♥")
        .rule(
            Bid::new(1, Strain::Notrump),
            0.9,
            points(6..=10) & len(Suit::Spades, ..4) & len(Suit::Hearts, ..4),
        )
        .note("NF, 6–10")
        .rule(Call::Pass, 0.0, points(..6))
}

/// Natural responses to 1♥ (11–17, 5+♥)
fn one_heart_responses() -> Rules {
    Rules::new()
        .rule(
            Bid::new(1, Strain::Spades),
            1.2,
            points(6..) & len(Suit::Spades, 4..),
        )
        .note("F, 4+♠")
        .rule(
            Bid::new(2, Strain::Clubs),
            1.3,
            points(12..) & len(Suit::Clubs, 4..),
        )
        .note("NAT FG")
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.3,
            points(12..) & len(Suit::Diamonds, 4..),
        )
        .note("NAT FG")
        .rule(
            Bid::new(2, Strain::Hearts),
            1.1,
            support(3..) & points(6..=10),
        )
        .note("CONST, 3+♥")
        .rule(
            Bid::new(2, Strain::Notrump),
            1.4,
            support(4..) & points(12..),
        )
        .note("FG, 4+♥")
        .rule(
            Bid::new(3, Strain::Hearts),
            1.2,
            support(4..) & points(10..=11),
        )
        .note("LIM, 4+♥")
        .rule(
            Bid::new(4, Strain::Hearts),
            1.0,
            support(4..) & points(..10) & !balanced(),
        )
        .note("PRE, UNBAL 4+♥")
        .rule(
            Bid::new(1, Strain::Notrump),
            0.9,
            points(6..=11) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .note("usually 7–11, 0–3♠, 0–3♥")
        .rule(Call::Pass, 0.0, points(..6))
}

/// Natural responses to 1♠ (11–17, 5+♠)
fn one_spade_responses() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Clubs),
            1.3,
            points(12..) & len(Suit::Clubs, 4..),
        )
        .note("NAT FG")
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.3,
            points(12..) & len(Suit::Diamonds, 4..),
        )
        .note("FG, 4+♦")
        .rule(
            Bid::new(2, Strain::Hearts),
            1.3,
            points(12..) & len(Suit::Hearts, 5..),
        )
        .note("FG, 5+♥")
        .rule(
            Bid::new(2, Strain::Spades),
            1.1,
            support(3..) & points(6..=10),
        )
        .note("CONST, 3+♠")
        .rule(
            Bid::new(2, Strain::Notrump),
            1.4,
            support(4..) & points(12..),
        )
        .note("FG, 4+♠")
        .rule(
            Bid::new(3, Strain::Spades),
            1.2,
            support(4..) & points(10..=11),
        )
        .note("LIM, 4+♠")
        .rule(
            Bid::new(4, Strain::Spades),
            1.0,
            support(4..) & points(..10) & !balanced(),
        )
        .note("PRE, UNBAL 4+♠")
        .rule(
            Bid::new(1, Strain::Notrump),
            0.9,
            points(6..=11) & len(Suit::Spades, ..4),
        )
        .note("usually 7–11, 0–3♠")
        .rule(Call::Pass, 0.0, points(..6))
}

/// Register the response tables in the constructive book
pub(super) fn register(book: &mut Trie) {
    insert_uncontested(book, &[call(1, Strain::Clubs)], one_club_responses());
    insert_uncontested(book, &[call(1, Strain::Diamonds)], one_diamond_responses());
    insert_uncontested(book, &[call(1, Strain::Hearts)], one_heart_responses());
    insert_uncontested(book, &[call(1, Strain::Spades)], one_spade_responses());
    // The shared 15–17 1NT reuses the verified 2/1 notrump responses.
    insert_uncontested(book, &[call(1, Strain::Notrump)], notrump_responses());
}

#[cfg(test)]
mod tests {
    use super::*;
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
            .filter(|(_, l)| l.is_finite())
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("some response")
    }

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    #[test]
    fn one_club_is_forcing_and_relays() {
        let r = one_club_responses();
        let a = [call(1, Strain::Clubs), Call::Pass];
        // A bust relays with the artificial negative 1♦ — never passes.
        assert_eq!(best(&r, &a, "832.752.9643.J42"), bid(1, Strain::Diamonds));
        // With four-four majors the positive goes up the line to 1♥.
        assert_eq!(best(&r, &a, "KQ32.AJ52.96.J42"), bid(1, Strain::Hearts));
        // A game-forcing five-card club hand uses the swapped 2♦.
        assert_eq!(best(&r, &a, "A2.K5.Q43.KJ8543"), bid(2, Strain::Diamonds));
    }

    #[test]
    fn one_club_never_passes() {
        // Forcing by omission: no hand the responder holds maps to Pass.
        let r = one_club_responses();
        let auction = [call(1, Strain::Clubs), Call::Pass];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        for hand in ["832.752.9643.J42", "2.32.QJ8642.9876", "KQJ.QJ.KQJ9.KQJ9"] {
            let hand: Hand = hand.parse().expect("valid hand");
            let logits = r.classify(hand, &context);
            assert!(
                logits.0.get(Call::Pass).is_infinite(),
                "1♣ must not be passed on {hand}"
            );
        }
    }

    #[test]
    fn major_raises_and_new_suits() {
        let h = one_heart_responses();
        let a = [call(1, Strain::Hearts), Call::Pass];
        // Three-card constructive raise.
        assert_eq!(best(&h, &a, "K32.Q53.A964.J92"), bid(2, Strain::Hearts));
        // Game-forcing four-card heart raise (Jacoby/Stenberg 2NT).
        assert_eq!(best(&h, &a, "K2.KQ54.A964.K92"), bid(2, Strain::Notrump));
        // A four-card spade response, forcing.
        assert_eq!(best(&h, &a, "KQ52.J3.A964.J92"), bid(1, Strain::Spades));
    }
}
