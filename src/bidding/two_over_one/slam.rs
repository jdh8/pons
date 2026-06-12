//! Slam machinery: Roman Keycard Blackwood 1430
//!
//! # RKCB 1430 ladder
//!
//! The 4NT ask is installed by the caller; this module registers the responses,
//! the asker's continuations, and the 5NT king-ask sequence.
//!
//! Responses encode the five *keycards* — the four aces plus the trump king:
//!
//! | answer | keycards    |
//! |--------|-------------|
//! | 5♣     | 1 or 4 ("14") |
//! | 5♦     | 0 or 3 ("30") |
//! | 5♥     | 2, without the trump queen |
//! | 5♠     | 2, with the trump queen    |
//!
//! # Ambiguity policy
//!
//! 5♣ and 5♦ are ambiguous between the lower and higher count.  The asker
//! resolves this by assuming the *encouraging* reading when holding 2 or fewer
//! keycards themselves (partner promised slam interest, so the higher count is
//! more plausible), and the *discouraging* reading otherwise.
//!
//! - After 5♣: asker with ≤2 keycards assumes partner has 4; 3+ assumes 1.
//! - After 5♦: asker with ≤2 keycards assumes partner has 3; 3+ knows 0.
//!
//! # 5NT king ask
//!
//! 5NT promises that the partnership holds all five keycards and asks for
//! kings outside trumps.  It is only available when the asker can certify that
//! (i.e., when their own count plus the assumed partner count equals five).
//!
//! Kings outside trumps are answered 6♣ (0), 6♦ (1), and — for spade trumps —
//! 6♥ (2) or 6♠ (signoff with 3).  For heart trumps 6♥ is the catch-all
//! signoff (2+ kings).
//!
//! # Scope
//!
//! Minor-suit keycard is out of scope: the signoff space below 5-of-a-minor
//! does not exist in this model.  Only major-suit trumps are supported.

use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Rank, Strain, Suit};
use core::ops::RangeBounds;
use std::sync::Arc;

use super::{insert_uncontested, uncontested};
use crate::bidding::constraint::{hcp, pred};
use crate::bidding::trie::Classifier;
use contract_bridge::Hand;

/// Insert an already-shared classifier at `suffix` under every leading-pass prefix
///
/// Identical to [`super::insert_all_seats`] but accepts an `Arc<dyn Classifier>`
/// directly so one allocation can be reused across multiple seat paths.
fn insert_arc_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    f: Arc<dyn Classifier>,
) {
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.insert_arc(&key, Arc::clone(&f));
    }
}

// ---------------------------------------------------------------------------
// Keycard constraint helpers
// ---------------------------------------------------------------------------

/// Count keycards: the four aces plus the trump king
///
/// Returns the number of keycards held by this hand: one point for each ace
/// in any suit, plus one point if the hand holds the king of trumps.
fn count_keycards(hand: Hand, trump: Suit) -> usize {
    let aces = Suit::ASC
        .into_iter()
        .filter(|&s| hand[s].contains(Rank::A))
        .count();
    let trump_king = usize::from(hand[trump].contains(Rank::K));
    aces + trump_king
}

/// Count kings outside the trump suit
fn count_kings_outside(hand: Hand, trump: Suit) -> usize {
    Suit::ASC
        .into_iter()
        .filter(|&s| s != trump && hand[s].contains(Rank::K))
        .count()
}

/// Keycard count in the given range
///
/// Satisfied when the count of keycards (four aces + trump king) is within
/// `range`.  Use for both responder and asker constraints.
fn keycards(
    trump: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync + 'static,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    pred(
        move |hand: Hand, _: &crate::bidding::context::Context<'_>| {
            range.contains(&count_keycards(hand, trump))
        },
    )
}

/// Whether the hand holds the queen of trumps
fn has_trump_queen(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    pred(move |hand: Hand, _: &crate::bidding::context::Context<'_>| hand[trump].contains(Rank::Q))
}

/// Count of kings in the three non-trump suits, in the given range
fn kings_outside(
    trump: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync + 'static,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    pred(
        move |hand: Hand, _: &crate::bidding::context::Context<'_>| {
            range.contains(&count_kings_outside(hand, trump))
        },
    )
}

// ---------------------------------------------------------------------------
// Rule-table builders
// ---------------------------------------------------------------------------

/// The four RKCB answers at the 4NT node (forcing — no Pass rule)
fn rkcb_answers(trump: Suit) -> Rules {
    Rules::new()
        // 5♣ = 1 or 4 keycards ("14")
        .rule(
            Bid::new(5, Strain::Clubs),
            1.0,
            keycards(trump, 1..=1) | keycards(trump, 4..=4),
        )
        // 5♦ = 0 or 3 keycards ("30")
        .rule(
            Bid::new(5, Strain::Diamonds),
            1.0,
            keycards(trump, 0..=0) | keycards(trump, 3..=3),
        )
        // 5♥ = 2 keycards without the trump queen
        .rule(
            Bid::new(5, Strain::Hearts),
            1.0,
            keycards(trump, 2..=2) & !has_trump_queen(trump),
        )
        // 5♠ = 2 keycards with the trump queen
        .rule(
            Bid::new(5, Strain::Spades),
            1.0,
            keycards(trump, 2..=2) & has_trump_queen(trump),
        )
}

/// Asker's continuation after a 5♣ response
///
/// Policy: asker with ≤2 keycards assumes partner has 4 (5NT = all five,
/// king ask); with 3+ asker knows partner has 1, signs off at 5T or bids 6T.
fn asker_after_5c(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        // 5NT: asker has 4 keycards + partner's 1 = all five → king ask
        .rule(Bid::new(5, Strain::Notrump), 1.4, keycards(trump, 4..=4))
        // 6T: asker has 3 keycards, assumes partner has 4 → interested in slam
        .rule(Bid::new(6, t), 1.0, keycards(trump, 3..=3))
        // 5T: signoff (asker doesn't want slam)
        .rule(Bid::new(5, t), 0.5, hcp(0..))
}

/// Asker's continuation after a 5♦ response
///
/// Policy: asker with ≤2 keycards assumes partner has 3 → bid 6T; with 3+
/// keycards asker knows partner has 0 → sign off at 5T.
fn asker_after_5d(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        // 6T: asker with ≤2 assumes partner has 3 (slam OK), or asker has 4+
        .rule(
            Bid::new(6, t),
            1.0,
            keycards(trump, 2..=2) | keycards(trump, 4..),
        )
        // 5T: signoff (asker has ≥3 and knows partner has 0)
        .rule(Bid::new(5, t), 0.5, hcp(0..))
}

/// Asker's continuation after a 5♥ response (2 keycards, no trump queen)
fn asker_after_5h(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        // 6T: asker has 3+ keycards → 5+ total, slam interest
        .rule(Bid::new(6, t), 1.0, keycards(trump, 3..))
        // 5T: signoff
        .rule(Bid::new(5, t), 0.5, hcp(0..))
}

/// Asker's continuation after a 5♠ response (2 keycards, with trump queen)
///
/// When trump is Hearts: 5♥ (the natural signoff) is illegal — the answer was
/// 5♠, which is already higher; passing would strand the auction in 5♠.
/// Instead, add a 6T catch-all.  For Spades: the 5♠ signoff rule is itself
/// dead (not higher than 5♠), and passing 5♠ is correct, so no catch-all.
fn asker_after_5s(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    let mut rules = Rules::new()
        // 5NT: asker has 3+ keycards + partner's 2 w/Q, and 2+ outside kings → grand
        .rule(
            Bid::new(5, Strain::Notrump),
            1.4,
            keycards(trump, 3..) & kings_outside(trump, 2..),
        )
        // 6T: asker has 2+ keycards → slam
        .rule(Bid::new(6, t), 1.0, keycards(trump, 2..))
        // 5T: signoff (dead for spades, catches hearts where 5♥ is illegal)
        .rule(Bid::new(5, t), 0.5, hcp(0..));

    if trump == Suit::Hearts {
        // Over a 5♠ answer the 5♥ signoff above is illegal; this 6♥ catch-all
        // ensures we don't pass 5♠ when we can't sign off naturally.
        rules = rules.rule(Bid::new(6, t), 0.3, hcp(0..));
    }
    rules
}

/// King answers at the 5NT node (for all answer paths — shared table)
///
/// 5NT promises all five keycards; this asks for kings outside trumps.
///
/// For spades: 6♣ (0), 6♦ (1), 6♥ (2), 6♠ signoff (3 kings).
/// For hearts: 6♣ (0), 6♦ (1), 6♥ catch-all signoff (2+).
fn king_answers(trump: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(Bid::new(6, Strain::Clubs), 1.0, kings_outside(trump, 0..=0))
        .rule(
            Bid::new(6, Strain::Diamonds),
            1.0,
            kings_outside(trump, 1..=1),
        );

    match trump {
        Suit::Spades => {
            rules = rules
                .rule(
                    Bid::new(6, Strain::Hearts),
                    1.0,
                    kings_outside(trump, 2..=2),
                )
                // 3 outside kings → 6♠ signoff (counting stops below 7)
                .rule(Bid::new(6, Strain::Spades), 0.5, hcp(0..));
        }
        Suit::Hearts => {
            // 6♥ is a catch-all signoff for 2+ outside kings
            rules = rules.rule(Bid::new(6, Strain::Hearts), 0.5, hcp(0..));
        }
        _ => unreachable!("install_rkcb only supports major-suit trumps"),
    }
    rules
}

/// Asker's call after a 6♣ king answer (0 outside kings)
fn asker_after_6c(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 1.0, kings_outside(trump, 3..))
        .rule(Bid::new(6, t), 0.5, hcp(0..))
}

/// Asker's call after a 6♦ king answer (1 outside king)
fn asker_after_6d(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 1.0, kings_outside(trump, 2..))
        .rule(Bid::new(6, t), 0.5, hcp(0..))
}

/// Asker's call after a 6♥ king answer (2 outside kings; only when trump == Spades)
fn asker_after_6h(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 1.0, kings_outside(trump, 1..))
        .rule(Bid::new(6, t), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Install RKCB 1430 below an agreed trump suit
///
/// `our_calls` is the undisturbed sequence of our side's calls so far (the
/// same form [`uncontested`][super::uncontested] takes); the 4NT ask, its
/// answers, and the 5NT king ask are inserted below it.  Major-suit trumps
/// only — minor-suit keycard needs signoff space this table does not model.
///
/// The 4NT bid itself must already be in the caller's table; this function
/// registers everything that comes *after* 4NT.
pub(super) fn install_rkcb(book: &mut Trie, our_calls: &[Call], trump: Suit) {
    // The ask and the four RKCB answer calls
    let c_4nt = Call::Bid(Bid::new(4, Strain::Notrump));
    let ans_5c = Call::Bid(Bid::new(5, Strain::Clubs));
    let ans_5d = Call::Bid(Bid::new(5, Strain::Diamonds));
    let ans_5h = Call::Bid(Bid::new(5, Strain::Hearts));
    let ans_5s = Call::Bid(Bid::new(5, Strain::Spades));
    let c_5nt = Call::Bid(Bid::new(5, Strain::Notrump));

    // Helper: build `our_calls + [4NT] + [tail…]`
    let extend = |tail: &[Call]| -> Vec<Call> {
        our_calls
            .iter()
            .copied()
            .chain(core::iter::once(c_4nt))
            .chain(tail.iter().copied())
            .collect()
    };

    // -----------------------------------------------------------------------
    // 1. Answers at `our_calls + [4NT]` (forcing, no Pass rule)
    // -----------------------------------------------------------------------
    insert_uncontested(book, &extend(&[]), rkcb_answers(trump));

    // -----------------------------------------------------------------------
    // 2. Asker's continuations after each answer
    //    `our_calls + [4NT, ans]`
    // -----------------------------------------------------------------------

    // Build the shared asker tables once
    let after_5c = Arc::new(asker_after_5c(trump)) as Arc<dyn Classifier>;
    let after_5d = Arc::new(asker_after_5d(trump)) as Arc<dyn Classifier>;
    let after_5h = Arc::new(asker_after_5h(trump)) as Arc<dyn Classifier>;
    let after_5s = Arc::new(asker_after_5s(trump)) as Arc<dyn Classifier>;

    let suffix_5c = uncontested(&extend(&[ans_5c]));
    let suffix_5d = uncontested(&extend(&[ans_5d]));
    let suffix_5h = uncontested(&extend(&[ans_5h]));
    let suffix_5s = uncontested(&extend(&[ans_5s]));

    insert_arc_all_seats(book, &suffix_5c, 3, Arc::clone(&after_5c));
    insert_arc_all_seats(book, &suffix_5d, 3, Arc::clone(&after_5d));
    insert_arc_all_seats(book, &suffix_5h, 3, Arc::clone(&after_5h));
    insert_arc_all_seats(book, &suffix_5s, 3, Arc::clone(&after_5s));

    // -----------------------------------------------------------------------
    // 3. King answers at `our_calls + [4NT, ans, 5NT]` — shared table
    // -----------------------------------------------------------------------
    let shared_king_answers = Arc::new(king_answers(trump)) as Arc<dyn Classifier>;

    for &ans in &[ans_5c, ans_5d, ans_5h, ans_5s] {
        let king_path = uncontested(&extend(&[ans, c_5nt]));
        insert_arc_all_seats(book, &king_path, 3, Arc::clone(&shared_king_answers));
    }

    // -----------------------------------------------------------------------
    // 4. Asker after king answers
    //    `our_calls + [4NT, ans, 5NT, kans]`
    // -----------------------------------------------------------------------
    let kans_6c = Call::Bid(Bid::new(6, Strain::Clubs));
    let kans_6d = Call::Bid(Bid::new(6, Strain::Diamonds));
    let kans_6h = Call::Bid(Bid::new(6, Strain::Hearts));

    let shared_after_6c = Arc::new(asker_after_6c(trump)) as Arc<dyn Classifier>;
    let shared_after_6d = Arc::new(asker_after_6d(trump)) as Arc<dyn Classifier>;

    // Register asker-after-king-answer for each of the four ans paths
    for &ans in &[ans_5c, ans_5d, ans_5h, ans_5s] {
        // after 6♣
        let suffix_6c = uncontested(&extend(&[ans, c_5nt, kans_6c]));
        insert_arc_all_seats(book, &suffix_6c, 3, Arc::clone(&shared_after_6c));

        // after 6♦
        let suffix_6d = uncontested(&extend(&[ans, c_5nt, kans_6d]));
        insert_arc_all_seats(book, &suffix_6d, 3, Arc::clone(&shared_after_6d));

        // after 6♥ (only when trump == Spades)
        if trump == Suit::Spades {
            let suffix_6h = uncontested(&extend(&[ans, c_5nt, kans_6h]));
            let after_6h = Arc::new(asker_after_6h(trump)) as Arc<dyn Classifier>;
            insert_arc_all_seats(book, &suffix_6h, 3, after_6h);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::System;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Hand, Strain};

    /// Build a Trie with RKCB installed for the test auction
    fn rkcb_trie() -> Trie {
        let mut trie = Trie::new();
        // Our calls: 1♠ – 2NT – 3♣ (the context before 4NT is asked;
        // install_rkcb appends the 4NT ask itself)
        let our_calls = [
            Call::Bid(Bid::new(1, Strain::Spades)),
            Call::Bid(Bid::new(2, Strain::Notrump)),
            Call::Bid(Bid::new(3, Strain::Clubs)),
        ];
        install_rkcb(&mut trie, &our_calls, Suit::Spades);
        trie
    }

    /// The best call made by the trie for the given hand at the given auction
    fn best(trie: &Trie, auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let logits = trie
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("trie covers this auction");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("logits array is never empty")
    }

    // The raw table auction interleaves opposing passes after each of our calls.
    // Opener (our side) is in seat 1 (no leading pass), so the auction is:
    //   [1♠, P, 2NT, P, 3♣, P, 4NT, P]
    const ANS_AUCTION: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(2, Strain::Notrump)),
        Call::Pass,
        Call::Bid(Bid::new(3, Strain::Clubs)),
        Call::Pass,
        Call::Bid(Bid::new(4, Strain::Notrump)),
        Call::Pass,
    ];

    /// RKCB answers at [1♠, P, 2NT, P, 3♣, P, 4NT, P]
    #[test]
    fn answers_keycard_counts() {
        let trie = rkcb_trie();

        // KQ732.K53.Q42.92 — no aces, trump K → 1 keycard → 5♣
        assert_eq!(
            best(&trie, ANS_AUCTION, "KQ732.K53.Q42.92"),
            Call::Bid(Bid::new(5, Strain::Clubs)),
            "1 keycard → 5♣"
        );

        // QJ732.K53.Q42.Q2 — no aces, heart K is NOT a keycard → 0 keycards → 5♦
        assert_eq!(
            best(&trie, ANS_AUCTION, "QJ732.K53.Q42.Q2"),
            Call::Bid(Bid::new(5, Strain::Diamonds)),
            "0 keycards → 5♦"
        );

        // AK732.A53.842.92 — 2 aces + trump K = 3 keycards → 5♦
        assert_eq!(
            best(&trie, ANS_AUCTION, "AK732.A53.842.92"),
            Call::Bid(Bid::new(5, Strain::Diamonds)),
            "3 keycards → 5♦"
        );

        // AQ732.A53.842.92 — 2 aces + trump Q → 2 keycards with Q → 5♠
        assert_eq!(
            best(&trie, ANS_AUCTION, "AQ732.A53.842.92"),
            Call::Bid(Bid::new(5, Strain::Spades)),
            "2 keycards + trump Q → 5♠"
        );

        // A8732.A53.842.92 — 2 aces, no trump Q or K → 2 keycards, no Q → 5♥
        assert_eq!(
            best(&trie, ANS_AUCTION, "A8732.A53.842.92"),
            Call::Bid(Bid::new(5, Strain::Hearts)),
            "2 keycards, no trump Q → 5♥"
        );
    }

    /// Asker's continuation after 5♦ response
    #[test]
    fn asker_after_5d_response() {
        let trie = rkcb_trie();
        // Auction: [1♠, P, 2NT, P, 3♣, P, 4NT, P, 5♦, P]
        let auction: Vec<Call> = ANS_AUCTION
            .iter()
            .copied()
            .chain([Call::Bid(Bid::new(5, Strain::Diamonds)), Call::Pass])
            .collect();

        // KQ52.AK76.A72.93 — 3 keycards (A♥, A♦, K♠) → knows partner has 0 → sign off 5♠
        assert_eq!(
            best(&trie, &auction, "KQ52.AK76.A72.93"),
            Call::Bid(Bid::new(5, Strain::Spades)),
            "asker with 3 keycards after 5♦ → knows 0, sign off 5♠"
        );

        // Q852.AK76.K72.A3 — 2 keycards (A♥, A♣) → assumes partner has 3 → 6♠
        assert_eq!(
            best(&trie, &auction, "Q852.AK76.K72.A3"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "asker with 2 keycards after 5♦ → assumes 3, bid 6♠"
        );
    }

    /// King ask after 5♣ response (asker has 4 keycards)
    #[test]
    fn king_ask_after_5c() {
        let trie = rkcb_trie();
        // Auction: [1♠, P, 2NT, P, 3♣, P, 4NT, P, 5♣, P]
        let auction: Vec<Call> = ANS_AUCTION
            .iter()
            .copied()
            .chain([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass])
            .collect();

        // AQ52.A876.A72.A3 — 4 keycards (all 4 aces) → partner has 1 → 5NT king ask
        assert_eq!(
            best(&trie, &auction, "AQ52.A876.A72.A3"),
            Call::Bid(Bid::new(5, Strain::Notrump)),
            "asker with 4 keycards after 5♣ → 5NT king ask"
        );
    }

    /// King answer at the 5NT node
    #[test]
    fn king_answer_after_5nt() {
        let trie = rkcb_trie();
        // Auction: [1♠, P, 2NT, P, 3♣, P, 4NT, P, 5♣, P, 5NT, P]
        let auction: Vec<Call> = ANS_AUCTION
            .iter()
            .copied()
            .chain([
                Call::Bid(Bid::new(5, Strain::Clubs)),
                Call::Pass,
                Call::Bid(Bid::new(5, Strain::Notrump)),
                Call::Pass,
            ])
            .collect();

        // K9732.K53.942.92 — trump K (keycard) + K♥ (1 outside king) → 6♦
        assert_eq!(
            best(&trie, &auction, "K9732.K53.942.92"),
            Call::Bid(Bid::new(6, Strain::Diamonds)),
            "1 outside king → 6♦"
        );
    }
}
