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
//! # Minor-suit trumps (plain 4NT)
//!
//! Minor trumps use the same `5♣/5♦/5♥/5♠` answers, but those answers overshoot
//! the natural 5-of-a-minor signoff, so the asker is cramped.  When it wants to
//! stop it signs off in 5-of-the-minor *only when that call is still legal*
//! (i.e. higher than partner's answer — diamonds after a 5♣ answer), passes when
//! partner's answer *is* 5-of-the-minor (clubs after 5♣, diamonds after 5♦), and
//! otherwise has no room below slam and simply bids 6-of-the-minor.
//!
//! The 5NT king ask is **major-only**: over a minor, 5NT would be misread as a
//! king ask and the king responses (6♣/6♦) collide with the trump slam, so
//! grand-slam exploration in a minor is not supported.  Kickback (4♣/4♦), the
//! usual remedy, is out of scope.

use crate::bidding::{Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Rank, Strain, Suit};
use core::ops::RangeBounds;
use std::sync::Arc;

use super::{insert_uncontested, uncontested};
use crate::bidding::constraint::{described, hcp};
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
    f: &Arc<dyn Classifier>,
) {
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.insert_arc(&key, Arc::clone(f));
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

/// Format a count range as a constraint label, mirroring the prose of the
/// constraint DSL's range primitives ("exactly 2 keycards", "3+ keycards").
fn count_label(range: &impl RangeBounds<usize>, noun: &str) -> String {
    use core::ops::Bound;
    let lo = match range.start_bound() {
        Bound::Included(&x) => Some(x),
        Bound::Excluded(&x) => Some(x + 1),
        Bound::Unbounded => None,
    };
    let hi = match range.end_bound() {
        Bound::Included(&x) => Some(x),
        Bound::Excluded(&x) => Some(x.saturating_sub(1)),
        Bound::Unbounded => None,
    };
    match (lo, hi) {
        (Some(a), Some(b)) if a == b => format!("exactly {a} {noun}"),
        (Some(a), Some(b)) => format!("{a}–{b} {noun}"),
        (Some(a), None) => format!("{a}+ {noun}"),
        (None, Some(b)) => format!("≤{b} {noun}"),
        (None, None) => noun.to_string(),
    }
}

/// Keycard count in the given range
///
/// Satisfied when the count of keycards (four aces + trump king) is within
/// `range`.  Use for both responder and asker constraints.
fn keycards(
    trump: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync + 'static,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    described(
        count_label(&range, "keycards"),
        move |hand: Hand, _: &crate::bidding::context::Context<'_>| {
            range.contains(&count_keycards(hand, trump))
        },
    )
}

/// Whether the hand holds the queen of trumps
fn has_trump_queen(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    described(
        format!("holds the {trump} queen"),
        move |hand: Hand, _: &crate::bidding::context::Context<'_>| hand[trump].contains(Rank::Q),
    )
}

/// Count of kings in the three non-trump suits, in the given range
fn kings_outside(
    trump: Suit,
    range: impl RangeBounds<usize> + Clone + Send + Sync + 'static,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    described(
        count_label(&range, "kings outside trumps"),
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

// ---------------------------------------------------------------------------
// Minor-trump asker continuations (plain 4NT; cramped signoff; no king ask)
// ---------------------------------------------------------------------------
//
// The keycard counts mirror the major tables; only the signoff differs, because
// the answers overshoot 5-of-a-minor.  Every table keeps a legal finite call for
// every hand (a `6m` or `Pass` catch-all): a node whose only finite logits are
// *illegal* calls would not fall through to the floor — `Table::next_call` would
// filter them and silently pass, stranding a bad contract.

/// Asker after a 5♣ answer when trumps are a minor
///
/// 3+ keycards (≥5 total) drive to 6-of-the-minor.  To stop: diamonds can sign
/// off in 5♦ (legal over 5♣); clubs must Pass to play partner's 5♣.
fn asker_after_5c_minor(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    let rules = Rules::new().rule(Bid::new(6, t), 1.0, keycards(trump, 3..));
    if trump == Suit::Diamonds {
        rules.rule(Bid::new(5, t), 0.5, hcp(0..))
    } else {
        rules.rule(Call::Pass, 0.5, hcp(0..))
    }
}

/// Asker after a 5♦ answer when trumps are a minor
///
/// Diamonds: slam set mirrors the major `asker_after_5d` (≤2 assume partner 3,
/// or 4+); to stop, Pass to play partner's 5♦.  Clubs: no room below 6♣.
fn asker_after_5d_minor(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    if trump == Suit::Diamonds {
        Rules::new()
            .rule(
                Bid::new(6, t),
                1.0,
                keycards(trump, 2..=2) | keycards(trump, 4..),
            )
            .rule(Call::Pass, 0.5, hcp(0..))
    } else {
        no_room_six(trump)
    }
}

/// Asker with no room to stop below slam: bid 6-of-the-minor
///
/// Used for the 5♥/5♠ answers (both minors) and the clubs 5♦ answer — all sit
/// above 5-of-either-minor, so signing off below slam is impossible.
fn no_room_six(trump: Suit) -> Rules {
    Rules::new().rule(Bid::new(6, Strain::from(trump)), 1.0, hcp(0..))
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
        _ => unreachable!("the 5NT king ask is major-only; minors never install it"),
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
/// same form [`uncontested`][super::uncontested] takes); the 4NT ask and its
/// answers are inserted below it.  Both majors and minors are supported; for
/// minors the asker's signoff is cramped (see the module docs) and the 5NT king
/// ask is skipped.
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

    // Build the shared asker tables once.  Majors use the full ladder; minors
    // use the cramped-signoff tables (and skip the king ask further down).
    let (after_5c, after_5d, after_5h, after_5s) = if matches!(trump, Suit::Hearts | Suit::Spades) {
        (
            Arc::new(asker_after_5c(trump)) as Arc<dyn Classifier>,
            Arc::new(asker_after_5d(trump)) as Arc<dyn Classifier>,
            Arc::new(asker_after_5h(trump)) as Arc<dyn Classifier>,
            Arc::new(asker_after_5s(trump)) as Arc<dyn Classifier>,
        )
    } else {
        (
            Arc::new(asker_after_5c_minor(trump)) as Arc<dyn Classifier>,
            Arc::new(asker_after_5d_minor(trump)) as Arc<dyn Classifier>,
            Arc::new(no_room_six(trump)) as Arc<dyn Classifier>,
            Arc::new(no_room_six(trump)) as Arc<dyn Classifier>,
        )
    };

    let suffix_5c = uncontested(&extend(&[ans_5c]));
    let suffix_5d = uncontested(&extend(&[ans_5d]));
    let suffix_5h = uncontested(&extend(&[ans_5h]));
    let suffix_5s = uncontested(&extend(&[ans_5s]));

    insert_arc_all_seats(book, &suffix_5c, 3, &after_5c);
    insert_arc_all_seats(book, &suffix_5d, 3, &after_5d);
    insert_arc_all_seats(book, &suffix_5h, 3, &after_5h);
    insert_arc_all_seats(book, &suffix_5s, 3, &after_5s);

    // ponytail: no grand-slam king ask for minors — plain 4NT has no room for it
    // (5NT misreads as the ask; 6♣/6♦ king answers collide with the trump slam).
    // Grand-in-minor stays under-bid; the upgrade path is Kickback (out of scope).
    if matches!(trump, Suit::Clubs | Suit::Diamonds) {
        return;
    }

    // -----------------------------------------------------------------------
    // 3. King answers at `our_calls + [4NT, ans, 5NT]` — shared table
    // -----------------------------------------------------------------------
    let shared_king_answers = Arc::new(king_answers(trump)) as Arc<dyn Classifier>;

    for &ans in &[ans_5c, ans_5d, ans_5h, ans_5s] {
        let king_path = uncontested(&extend(&[ans, c_5nt]));
        insert_arc_all_seats(book, &king_path, 3, &shared_king_answers);
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
        insert_arc_all_seats(book, &suffix_6c, 3, &shared_after_6c);

        // after 6♦
        let suffix_6d = uncontested(&extend(&[ans, c_5nt, kans_6d]));
        insert_arc_all_seats(book, &suffix_6d, 3, &shared_after_6d);

        // after 6♥ (only when trump == Spades)
        if trump == Suit::Spades {
            let suffix_6h = uncontested(&extend(&[ans, c_5nt, kans_6h]));
            let after_6h = Arc::new(asker_after_6h(trump)) as Arc<dyn Classifier>;
            insert_arc_all_seats(book, &suffix_6h, 3, &after_6h);
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

    // -----------------------------------------------------------------------
    // Minor-suit keycard (plain 4NT)
    // -----------------------------------------------------------------------

    /// A trie with minor RKCB installed below `[1m, 2m, 4NT]`
    fn minor_trie(trump: Suit) -> Trie {
        let mut trie = Trie::new();
        let m = Strain::from(trump);
        let our_calls = [Call::Bid(Bid::new(1, m)), Call::Bid(Bid::new(2, m))];
        install_rkcb(&mut trie, &our_calls, trump);
        trie
    }

    /// The answer node auction `[1m, P, 2m, P, 4NT, P]`
    fn minor_ans_auction(trump: Suit) -> Vec<Call> {
        let m = Strain::from(trump);
        vec![
            Call::Bid(Bid::new(1, m)),
            Call::Pass,
            Call::Bid(Bid::new(2, m)),
            Call::Pass,
            Call::Bid(Bid::new(4, Strain::Notrump)),
            Call::Pass,
        ]
    }

    /// `minor_ans_auction` extended by one keycard answer `+ [answer, P]`
    fn after_minor_answer(trump: Suit, answer: Bid) -> Vec<Call> {
        let mut a = minor_ans_auction(trump);
        a.push(Call::Bid(answer));
        a.push(Call::Pass);
        a
    }

    /// The generic answer table still fires for a minor trump (clubs).
    #[test]
    fn minor_answers_keycard_counts() {
        let trie = minor_trie(Suit::Clubs);
        let auction = minor_ans_auction(Suit::Clubs);

        // A654.832.K65.987 — A♠ only (K is diamonds) → 1 keycard → 5♣
        assert_eq!(
            best(&trie, &auction, "A654.832.K65.987"),
            Call::Bid(Bid::new(5, Strain::Clubs)),
            "1 keycard → 5♣"
        );
        // Q654.832.K65.J87 — no aces, no K♣ → 0 keycards → 5♦
        assert_eq!(
            best(&trie, &auction, "Q654.832.K65.J87"),
            Call::Bid(Bid::new(5, Strain::Diamonds)),
            "0 keycards → 5♦"
        );
        // A654.A32.65.J987 — A♠ A♥, clubs J987 (no K/Q) → 2 keycards no Q → 5♥
        assert_eq!(
            best(&trie, &auction, "A654.A32.65.J987"),
            Call::Bid(Bid::new(5, Strain::Hearts)),
            "2 keycards, no trump Q → 5♥"
        );
        // A654.A32.65.Q987 — A♠ A♥ + Q♣ → 2 keycards with Q → 5♠
        assert_eq!(
            best(&trie, &auction, "A654.A32.65.Q987"),
            Call::Bid(Bid::new(5, Strain::Spades)),
            "2 keycards with trump Q → 5♠"
        );
    }

    /// Clubs after a 5♣ answer: 3+ keycards → 6♣; otherwise Pass to play 5♣.
    #[test]
    fn clubs_after_5c_signoff_is_pass() {
        let trie = minor_trie(Suit::Clubs);
        let auction = after_minor_answer(Suit::Clubs, Bid::new(5, Strain::Clubs));

        // A654.A32.65.KQ87 — A♠ A♥ + K♣ → 3 keycards → 6♣
        assert_eq!(
            best(&trie, &auction, "A654.A32.65.KQ87"),
            Call::Bid(Bid::new(6, Strain::Clubs)),
            "asker 3 keycards after 5♣ → 6♣"
        );
        // A654.832.K65.987 — 1 keycard → off two → Pass to play partner's 5♣
        assert_eq!(
            best(&trie, &auction, "A654.832.K65.987"),
            Call::Pass,
            "asker ≤2 keycards after 5♣ → Pass (play 5♣)"
        );
    }

    /// Clubs after a 5♦/5♥/5♠ answer: no room — always 6♣, never Pass or 5♣.
    #[test]
    fn clubs_no_room_always_six() {
        let trie = minor_trie(Suit::Clubs);
        for answer in [
            Bid::new(5, Strain::Diamonds),
            Bid::new(5, Strain::Hearts),
            Bid::new(5, Strain::Spades),
        ] {
            let auction = after_minor_answer(Suit::Clubs, answer);
            for hand in ["A654.A32.65.KQ87", "Q654.Q32.Q65.Q98"] {
                let call = best(&trie, &auction, hand);
                assert_eq!(
                    call,
                    Call::Bid(Bid::new(6, Strain::Clubs)),
                    "clubs after {answer:?}, hand {hand}: must be 6♣ (no room to stop)"
                );
            }
        }
    }

    /// Diamonds after a 5♣ answer: 3+ keycards → 6♦; otherwise sign off in 5♦.
    #[test]
    fn diamonds_after_5c_signoff_is_5d() {
        let trie = minor_trie(Suit::Diamonds);
        let auction = after_minor_answer(Suit::Diamonds, Bid::new(5, Strain::Clubs));

        // A654.A32.K65.987 — A♠ A♥ + K♦ → 3 keycards → 6♦
        assert_eq!(
            best(&trie, &auction, "A654.A32.K65.987"),
            Call::Bid(Bid::new(6, Strain::Diamonds)),
            "asker 3 keycards after 5♣ → 6♦"
        );
        // A654.832.J65.987 — A♠ only, no K♦ → 1 keycard → 5♦ signoff (legal over 5♣)
        assert_eq!(
            best(&trie, &auction, "A654.832.J65.987"),
            Call::Bid(Bid::new(5, Strain::Diamonds)),
            "asker ≤2 keycards after 5♣ → 5♦ signoff"
        );
    }

    /// Diamonds after a 5♦ answer: 3+ keycards (knows partner 0) → Pass; 2 → 6♦.
    #[test]
    fn diamonds_after_5d_signoff_is_pass() {
        let trie = minor_trie(Suit::Diamonds);
        let auction = after_minor_answer(Suit::Diamonds, Bid::new(5, Strain::Diamonds));

        // A654.A32.K65.987 — 3 keycards → knows partner 0 → Pass to play 5♦
        assert_eq!(
            best(&trie, &auction, "A654.A32.K65.987"),
            Call::Pass,
            "asker 3 keycards after 5♦ → Pass (play 5♦)"
        );
        // A654.A32.J65.987 — A♠ A♥, no K♦ → 2 keycards → assumes partner 3 → 6♦
        assert_eq!(
            best(&trie, &auction, "A654.A32.J65.987"),
            Call::Bid(Bid::new(6, Strain::Diamonds)),
            "asker 2 keycards after 5♦ → 6♦"
        );
    }

    /// The asker never bids 5NT for a minor (the king ask is major-only).
    #[test]
    fn minors_never_bid_5nt() {
        for trump in [Suit::Clubs, Suit::Diamonds] {
            let trie = minor_trie(trump);
            for answer in [
                Bid::new(5, Strain::Clubs),
                Bid::new(5, Strain::Diamonds),
                Bid::new(5, Strain::Hearts),
                Bid::new(5, Strain::Spades),
            ] {
                let auction = after_minor_answer(trump, answer);
                for hand in ["A654.A32.AK5.AQ8", "Q654.Q32.Q65.Q98"] {
                    assert_ne!(
                        best(&trie, &auction, hand),
                        Call::Bid(Bid::new(5, Strain::Notrump)),
                        "{trump:?} after {answer:?}, hand {hand}: must never bid 5NT"
                    );
                }
            }
        }
    }

    /// The 5NT king-ask node is never installed for a minor trump.
    #[test]
    fn minor_king_ask_node_absent() {
        let trie = minor_trie(Suit::Clubs);
        // [1♣, P, 2♣, P, 4NT, P, 5♣, P, 5NT, P] — the major king-ask path
        let mut auction = after_minor_answer(Suit::Clubs, Bid::new(5, Strain::Clubs));
        auction.push(Call::Bid(Bid::new(5, Strain::Notrump)));
        auction.push(Call::Pass);
        let hand: Hand = "A654.A32.65.KQ87".parse().unwrap();
        assert!(
            trie.classify(hand, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "no king-answer table should exist for a minor trump"
        );
    }
}
