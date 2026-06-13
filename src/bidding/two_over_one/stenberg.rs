//! Strawberry Stenberg 2NT: opener's rebid after `1M – (P) – 2NT`
//!
//! The strawberry variant's replacement for Jacoby 2NT ([`super::raises`]).
//! The 2NT response itself is unchanged — a game-forcing raise with four-card
//! support, authored in [`super::responses`] — but opener's rebid follows
//! Stenberg (Swedish Jacoby): the principle of fast arrival should not apply to
//! an unlimited game-forcing raise, so the cheapest step shows a *minimum* and
//! every other rebid is a *maximum* that further describes shape.
//!
//! | Rebid | Meaning |
//! |---|---|
//! | 3♣ | Minimum opener |
//! | 3♦ | Maximum, no side shortness |
//! | 3♥/3♠/3NT | Maximum, a fragment (0–1) in the ascending side suit |
//! | 4♣/4♦ | Maximum, a good five-card side suit |
//! | 4♥ (over 1♥) | Maximum two-suiter, 6+♥ and 4+♠ |
//! | 4♥/4♠ (over 1♠) | A five-card heart side suit, maximum / minimum |
//!
//! The fragment bids are always 3♥, 3♠, 3NT regardless of trump, mapping to
//! shortness in the three side suits taken in ascending strain order.
//!
//! # Scope
//!
//! Opener's rebid table is ported faithfully.  On the responder side this
//! authors slam tries (RKCB 1430 below the agreed major) and game sign-offs;
//! the deeper minimum-relay distribution ask from the notes is left for a later
//! pass.  Responder's calls always reach game or a keycard ask — the
//! constructive book carries no instinct floor, so an unauthored continuation
//! would be passed below game.

use super::{call, insert_uncontested, slam};
use crate::bidding::constraint::{balanced, fifths, hcp, len, points};
use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Strain, Suit};

/// The three non-trump suits, in ascending strain order
fn side_suits(major: Suit) -> [Suit; 3] {
    match major {
        Suit::Hearts => [Suit::Clubs, Suit::Diamonds, Suit::Spades],
        Suit::Spades => [Suit::Clubs, Suit::Diamonds, Suit::Hearts],
        _ => unreachable!("Stenberg 2NT only applies to a major-suit opening"),
    }
}

/// Opener's Stenberg rebid after `1M – 2NT`: minimum, or maximum with a feature
///
/// This node is **forcing** — there is no pass rule.  A maximum is 15+ points; the
/// fragment and five-card-suit rebids outrank the plain shape bids so the most
/// descriptive call wins.
fn stenberg_rebids(major: Suit) -> Rules {
    let [a, b, c] = side_suits(major);

    let rules = Rules::new()
        // 3♣: a minimum opener (the cheapest step) — the catch-all below.
        .rule(call(3, Strain::Clubs), 0.5, points(..15))
        // 3♦: maximum, no side shortness.
        .rule(
            call(3, Strain::Diamonds),
            1.5,
            points(15..) & !len(a, ..=1) & !len(b, ..=1) & !len(c, ..=1),
        )
        // 3♥/3♠/3NT: maximum with a fragment (0–1) in the ascending side suit.
        // Tiny weight steps show the cheapest shortness when two suits are short.
        .rule(call(3, Strain::Hearts), 2.00, points(15..) & len(a, ..=1))
        .rule(call(3, Strain::Spades), 1.98, points(15..) & len(b, ..=1))
        .rule(call(3, Strain::Notrump), 1.96, points(15..) & len(c, ..=1))
        // 4♣/4♦: maximum with a good five-card side suit.
        .rule(
            call(4, Strain::Clubs),
            2.20,
            points(15..) & len(Suit::Clubs, 5..),
        )
        .rule(
            call(4, Strain::Diamonds),
            2.15,
            points(15..) & len(Suit::Diamonds, 5..),
        );

    if major == Suit::Hearts {
        // 4♥: maximum two-suiter, 6+♥ and 4+♠.  The spade side suit sits above
        // game, so it is shown by jumping to game with the big shape.
        rules.rule(
            call(4, Strain::Hearts),
            2.30,
            points(15..) & len(Suit::Hearts, 6..) & len(Suit::Spades, 4..),
        )
    } else {
        // 4♥: maximum with a five-card heart side suit.  4♠: the same five-card
        // heart suit but a minimum — spades has room below game to split these.
        rules
            .rule(
                call(4, Strain::Hearts),
                2.20,
                points(15..) & len(Suit::Hearts, 5..),
            )
            .rule(
                call(4, Strain::Spades),
                1.0,
                points(..15) & len(Suit::Hearts, 5..),
            )
    }
}

/// Responder's continuation after opener's minimum 3♣
///
/// Forcing — no pass.  4NT launches RKCB; a flat hand offers 3NT as a choice of
/// games; otherwise responder signs off in the major game.
fn responder_after_min(major: Suit) -> Rules {
    let t = Strain::from(major);
    Rules::new()
        .rule(call(4, Strain::Notrump), 1.0, points(15..))
        .rule(
            call(3, Strain::Notrump),
            0.8,
            fifths(13.0..16.0) & balanced(),
        )
        .rule(call(4, t), 0.5, hcp(0..))
}

/// Responder's continuation after a maximum descriptive rebid: slam or sign-off
///
/// Forcing below game.  4NT is RKCB for slam interest; otherwise responder
/// sets the major game (or passes if opener's rebid already reached it).
fn responder_after_descriptive(major: Suit, rebid: Call) -> Rules {
    let t = Strain::from(major);
    let game = call(4, t);
    let rules = Rules::new().rule(call(4, Strain::Notrump), 1.0, points(15..));
    if rebid == game {
        rules.rule(Call::Pass, 0.0, hcp(0..))
    } else {
        rules.rule(game, 0.5, hcp(0..))
    }
}

/// Register the Stenberg 2NT opener-rebid and continuation nodes
///
/// For each major M inserts opener's rebid at `[1M, 2NT]`, then for each
/// distinct rebid R the responder continuation at `[1M, 2NT, R]` with RKCB
/// 1430 installed below it (responder asks with 4NT).
pub(super) fn register(book: &mut Trie) {
    for major in [Suit::Hearts, Suit::Spades] {
        let t = Strain::from(major);
        let our = [call(1, t), call(2, Strain::Notrump)];
        let rebids = stenberg_rebids(major);

        // Collect distinct rebid calls before moving `rebids` into the trie.
        let distinct: Vec<Call> = {
            let mut seen = std::collections::HashSet::new();
            rebids
                .rules()
                .iter()
                .filter_map(|r| seen.insert(r.call()).then_some(r.call()))
                .collect()
        };

        insert_uncontested(book, &our, rebids);

        for rebid in distinct {
            let resp_calls = [call(1, t), call(2, Strain::Notrump), rebid];
            let resp = if rebid == call(3, Strain::Clubs) {
                responder_after_min(major)
            } else {
                responder_after_descriptive(major, rebid)
            };
            insert_uncontested(book, &resp_calls, resp);
            slam::install_rkcb(book, &resp_calls, major);
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

    /// The highest-logit call the rules make for a hand at an auction
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

    const RAISE: [Call; 2] = [call(1, Strain::Hearts), call(2, Strain::Notrump)];

    #[test]
    fn minimum_opener_bids_three_clubs() {
        let r = stenberg_rebids(Suit::Hearts);
        // 13 HCP, no singleton — a minimum.
        assert_eq!(best(&r, &RAISE, "A8.KQ954.K84.J92"), call(3, Strain::Clubs));
    }

    #[test]
    fn maximum_no_shortness_bids_three_diamonds() {
        let r = stenberg_rebids(Suit::Hearts);
        // 17 HCP, 5332 — maximum, no side shortness.
        assert_eq!(
            best(&r, &RAISE, "A8.AQJ54.KQ4.Q92"),
            call(3, Strain::Diamonds)
        );
    }

    #[test]
    fn maximum_club_fragment_bids_three_hearts() {
        let r = stenberg_rebids(Suit::Hearts);
        // 16 HCP, singleton club — fragment 0–1♣ → 3♥.
        assert_eq!(
            best(&r, &RAISE, "AQ4.KQ954.KJ84.2"),
            call(3, Strain::Hearts)
        );
    }

    #[test]
    fn maximum_two_suiter_bids_four_hearts_over_one_heart() {
        let r = stenberg_rebids(Suit::Hearts);
        // 15 HCP, 6♥ + 4♠ — the big two-suiter jumps to 4♥.
        assert_eq!(
            best(&r, &RAISE, "AQ84.AK9542.Q2.2"),
            call(4, Strain::Hearts)
        );
    }

    #[test]
    fn spade_minimum_with_five_hearts_bids_four_spades() {
        let r = stenberg_rebids(Suit::Spades);
        let raise = [call(1, Strain::Spades), call(2, Strain::Notrump)];
        // 12 HCP, 5-5 but a wasted ♦Q — minimum with a heart side suit → 4♠.
        assert_eq!(
            best(&r, &raise, "KQ954.AJ842.Q4.2"),
            call(4, Strain::Spades)
        );
    }

    #[test]
    fn clean_five_five_upgrades_to_maximum_four_hearts() {
        let r = stenberg_rebids(Suit::Spades);
        let raise = [call(1, Strain::Spades), call(2, Strain::Notrump)];
        // 13 HCP, clean 5-5 (Kx and a small singleton waste nothing):
        // worth 15 points, so the heart side suit shows as a maximum.
        assert_eq!(
            best(&r, &raise, "KQ954.AJ842.K4.2"),
            call(4, Strain::Hearts)
        );
    }
}
