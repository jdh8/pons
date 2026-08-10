//! Slam machinery: Roman Keycard Blackwood 1430
//!
//! # RKCB 1430 ladder
//!
//! The 4NT ask is installed by the caller; this module registers the responses,
//! the asker's continuations, and the 5NT king-ask sequence.
//!
//! | Module | Agreement |
//! | --- | --- |
//! | [`queen_relay`] | the 5♣/5♦ queen relay and its replies |
//! | [`minor_lane`] | minor-trump asker continuations (cramped signoff, no king ask) |
//! | [`king_ask`] | the 5NT king ask and the asker's placements |
//!
//! Responses encode the five *keycards* — the four aces plus the trump king:
//!
//! | answer | keycards    |
//! |--------|-------------|
//! | 5♣     | 1 or 4 ("14") |
//! | 5♦     | 0 or 3 ("30") |
//! | 5♥     | 2 or 5, without the trump queen |
//! | 5♠     | 2 or 5, with the trump queen    |
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

use crate::bidding::agreements::Agreements;
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Rank, Strain, Suit};
use core::ops::RangeBounds;

/// Roman Keycard Blackwood — the artificial keycard ask, answers, and king ask
pub(super) const RKCB: Alert = Alert("rkcb");

/// Whether RKCB reaches agreed **minor** trumps — the book half of
/// [`keycard_minors`][crate::bidding::instinct::InstinctProfile::keycard_minors]
///
/// One agreement, two layers: the book authors the minor-suit plain-4NT
/// keycard at its two vehicles — the strong-2♣ minor raise (`2♣ - 2♦ - 3m - 4m`,
/// opener asks with 28+ HCP instead of blind-jumping `6m` on 27+) and the
/// inverted minor raise (`1m - 2m - 3NT`, responder asks on `points(14..)` instead
/// of resting in the 18–19 3NT) — while the floor lifts `keycard_trump`'s
/// majors-only carve.  They used to be *two* knobs, `set_minor_keycard` here
/// and `set_keycard_minors` there, whose four combinations included two
/// stances no partnership could play: a book that asks on a minor over a floor
/// that cannot answer, and the reverse.  Now one knob drives both, read at book
/// construction here and at classification there.  A live relocation
/// ([`RkcbVariant`][crate::bidding::instinct::RkcbVariant]) implies the reach
/// the same way — its whole payoff is the minor lanes, so a stance that
/// relocates them cannot leave the book unable to ask there.
///
/// Measured against the pre-keycard book: **+6.80/+8.76 IMPs/divergent**
/// (none/both, 2M boards), PD re-measure **+5.41/+7.05 IMPs/divergent** (10M
/// boards, 202 divergent, ~1 in 49.5k) — rare but decisively positive per fire.
///
/// A *derived* reading, so it stays a function of the two fields it is made of
/// rather than becoming a `Build` field — the same rule the competitive book's
/// `free_bids_engaged` follows. Both fields are pinned classify-time state, so
/// the build reads them back off the stance's [`DecisionProfile`], which is
/// exactly what [`minor_asks`][crate::bidding::instinct] does for the floor.
pub(super) fn minor_keycard(agreements: &Agreements) -> bool {
    crate::bidding::instinct::minor_asks(&agreements.decision)
}

use crate::bidding::constraint::{described, hcp};
use crate::bidding::instinct::{
    KingRelay, RelayMap, king_relay, queen_ask_room, queen_fit, relay_map,
};
use crate::bidding::rows::{Entry, Pattern, rows_of};
use contract_bridge::Hand;

mod king_ask;
mod minor_lane;
mod queen_relay;

use king_ask::{asker_after_6c, asker_after_6d, asker_after_6h, king_answers};
use minor_lane::{asker_after_5c_minor, asker_after_5d_minor, no_room_six};
use queen_relay::{
    asker_after_denial, asker_after_queen, asker_after_relay_kings, king_replies, queen_replies,
    relay_first,
};

// ---------------------------------------------------------------------------
// Keycard constraint helpers
// ---------------------------------------------------------------------------

/// Count keycards: the four aces plus the trump king
///
/// Returns the number of keycards held by this hand: one point for each ace
/// in any suit, plus one point if the hand holds the king of trumps.  Shared
/// with the instinct floor's RKCB (M6.4) so both ladders count identically.
pub(in crate::bidding) fn count_keycards(hand: Hand, trump: Suit) -> usize {
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

/// Whether the hand holds the queen of trumps — or the side has shown the
/// ten-card fit that stands in for it
///
/// A fit long enough to draw trumps without the honour makes the honour not
/// worth a round of bidding.  The threshold is `QUEEN_FIT` (ten,
/// BBA's bar, because this rung has to serve the grand); counted as our own
/// length plus the sound floor of partner's shown length, so neither seat can
/// claim a fit the auction has not shown.  A ninth trump is not a queen — it
/// answers on the buff jump instead (`QUEEN_BUFF_FIT`).
///
/// The threshold is sampled **here, at book construction**, not inside the
/// closure — the regime every book-level input lives in.  (It was a knob while
/// the relay was tuned; the tuning settled on BBA's ten and the constant
/// remains read at construction so book and closure can never disagree.)
fn has_trump_queen(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    let long_fit_counts = usize::from(queen_fit());
    described(
        format!("holds the {trump} queen"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].contains(Rank::Q)
                || hand[trump].len() + usize::from(context.inferences().partner().length(trump).min)
                    >= long_fit_counts
        },
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
            100,
            keycards(trump, 1..=1) | keycards(trump, 4..=4),
        )
        .alert(RKCB)
        // 5♦ = 0 or 3 keycards ("30")
        .rule(
            Bid::new(5, Strain::Diamonds),
            100,
            keycards(trump, 0..=0) | keycards(trump, 3..=3),
        )
        .alert(RKCB)
        // 5♥ = 2 or 5 keycards without the trump queen
        .rule(
            Bid::new(5, Strain::Hearts),
            100,
            (keycards(trump, 2..=2) | keycards(trump, 5..=5)) & !has_trump_queen(trump),
        )
        .alert(RKCB)
        // 5♠ = 2 or 5 keycards with the trump queen
        .rule(
            Bid::new(5, Strain::Spades),
            100,
            (keycards(trump, 2..=2) | keycards(trump, 5..=5)) & has_trump_queen(trump),
        )
        .alert(RKCB)
}

/// Asker's continuation after a 5♣ response
///
/// Policy: asker with ≤2 keycards assumes partner has 4 (5NT = all five,
/// king ask); with 3+ asker knows partner has 1, signs off at 5T or bids 6T.
fn asker_after_5c(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    // Ask only what we will act on.  Three keycards decodes to four combined —
    // one missing, and the queen decides five against six, so the answer is
    // worth a round.  Four keycards decodes to all five, where six is bid
    // whatever comes back: only a hand exploring seven has a use for the reply.
    relay_first(
        trump,
        Bid::new(5, Strain::Clubs),
        keycards(trump, 3..=3) | (keycards(trump, 4..=4) & hcp(19..)),
    )
    // 5NT: asker has 4 keycards + partner's 1 = all five → king ask, spent
    // only in the grand zone — the same veto-not-seeker gate the relay's own
    // king asks ride
    .rule(
        Bid::new(5, Strain::Notrump),
        140,
        keycards(trump, 4..=4) & hcp(19..),
    )
    .alert(RKCB)
    // 6T: all five on the table without the grand values, or three of our own
    // plus partner's one when the queen question is already settled
    .rule(Bid::new(6, t), 100, keycards(trump, 3..=4))
    // 5T: signoff (asker doesn't want slam)
    .rule(Bid::new(5, t), 50, hcp(0..))
}

/// Asker's continuation after a 5♦ response
///
/// Policy: asker with ≤2 keycards assumes partner has 3 → bid 6T; with 3+
/// keycards asker knows partner has 0 → sign off at 5T.
fn asker_after_5d(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    // Four keycards knows partner has none, so the total is four and the queen
    // is the whole difference between five and six — worth a round.  Two
    // keycards reads partner for three, and five keycards is all of them: both
    // are bidding six whatever the queen does, so they ask only when the values
    // put seven in range.
    relay_first(
        trump,
        Bid::new(5, Strain::Diamonds),
        keycards(trump, 4..=4) | ((keycards(trump, 2..=2) | keycards(trump, 5..)) & hcp(19..)),
    )
    // 6T: asker with ≤2 assumes partner has 3 (slam OK), or asker has 4+
    .rule(
        Bid::new(6, t),
        100,
        keycards(trump, 2..=2) | keycards(trump, 4..),
    )
    // 5T: signoff (asker has ≥3 and knows partner has 0)
    .rule(Bid::new(5, t), 50, hcp(0..))
}

/// Asker's continuation after a 5♥ response (2 keycards, no trump queen)
fn asker_after_5h(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        // 6T: asker has 3+ keycards → 5+ total, slam interest
        .rule(Bid::new(6, t), 100, keycards(trump, 3..))
        // 5T: signoff
        .rule(Bid::new(5, t), 50, hcp(0..))
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
            140,
            keycards(trump, 3..) & kings_outside(trump, 2..),
        )
        .alert(RKCB)
        // 6T: asker has 2+ keycards → slam
        .rule(Bid::new(6, t), 100, keycards(trump, 2..))
        // 5T: signoff (dead for spades, catches hearts where 5♥ is illegal)
        .rule(Bid::new(5, t), 50, hcp(0..));

    if trump == Suit::Hearts {
        // Over a 5♠ answer the 5♥ signoff above is illegal; this 6♥ catch-all
        // ensures we don't pass 5♠ when we can't sign off naturally.
        rules = rules.rule(Bid::new(6, t), 30, hcp(0..));
    }
    rules
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// The RKCB 1430 subtree as rows, below the auction `prefix`
///
/// `prefix` is the row layer's auction string ending just before **our 4NT
/// ask**, for example `"P* 1♠ - 3♠ -"`.  The ask, its answers and every
/// continuation are keyed below it.  Both majors and minors are supported; for
/// minors the asker's signoff is cramped (see the module docs) and the 5NT king
/// ask is skipped.
///
/// The 4NT bid itself must already be in the caller's table; this produces
/// everything that comes *after* 4NT.
pub(super) fn rkcb_rows(prefix: &str, trump: Suit) -> Vec<Entry> {
    let ans_5c = Bid::new(5, Strain::Clubs);
    let ans_5d = Bid::new(5, Strain::Diamonds);
    let ans_5h = Bid::new(5, Strain::Hearts);
    let ans_5s = Bid::new(5, Strain::Spades);

    // Every key hangs off our 4NT; the trailing `-` is the opposing pass the
    // retired imperative helper used to interleave by hand.
    let ask = format!("{prefix} 4NT -");
    let node = |tail: &str| Pattern::node(format!("{ask} {tail}").trim_end());

    // -----------------------------------------------------------------------
    // 1. Answers at the ask itself (forcing, no Pass rule)
    // -----------------------------------------------------------------------
    let mut entries = rows_of(node(""), rkcb_answers(trump));

    // -----------------------------------------------------------------------
    // 2. Asker's continuations after each answer
    // -----------------------------------------------------------------------
    //
    // Majors use the full ladder; minors use the cramped-signoff tables (and
    // skip the king ask further down).
    let (after_5c, after_5d, after_5h, after_5s) = if matches!(trump, Suit::Hearts | Suit::Spades) {
        (
            asker_after_5c(trump),
            asker_after_5d(trump),
            asker_after_5h(trump),
            asker_after_5s(trump),
        )
    } else {
        (
            asker_after_5c_minor(trump),
            asker_after_5d_minor(trump),
            no_room_six(trump),
            no_room_six(trump),
        )
    };

    for (answer, table) in [
        (ans_5c, after_5c),
        (ans_5d, after_5d),
        (ans_5h, after_5h),
        (ans_5s, after_5s),
    ] {
        entries.extend(rows_of(node(&format!("{answer} -")), table));
    }

    // -----------------------------------------------------------------------
    // 2b. The queen relay, where the lane has room for it
    // -----------------------------------------------------------------------
    //
    // Only the two ambiguous answers grow one — 5♥ and 5♠ already disclose the
    // queen — and only where `queen_ask_room` says the no-queen rung still
    // lands at or below five of trump, which excludes both plain-4NT minors and
    // hearts after a 0-or-3.  Those lanes keep betting the small slam on four
    // keycards, exactly as they do today.
    for answer in [ans_5c, ans_5d] {
        let Some(map) = relay_map(answer, trump) else {
            continue;
        };
        let relay = format!("{answer} - {} -", map.ask);

        // The answer lane owns the combined-count decode.  Over 5♣ partner
        // showed one, so four of our own is all five; over 5♦ partner showed
        // none or three, so two of our own (partner's three) or all five of our
        // own is — and four of our own is exactly four, the hand the relay
        // exists for and the hand a grand must never tempt.
        let (exact, at_least) = if answer == ans_5c { (4, 4) } else { (2, 5) };
        let five = || keycards(trump, exact..=exact) | keycards(trump, at_least..);

        entries.extend(rows_of(node(&relay), queen_replies(trump, &map)));
        for denial in [map.weak, map.deny] {
            entries.extend(rows_of(
                node(&format!("{relay} {denial} -")),
                asker_after_denial(trump, denial, five()),
            ));
        }
        entries.extend(rows_of(
            node(&format!("{relay} {} -", map.no_king)),
            asker_after_queen(trump, false, None, five()),
        ));
        for &(_, shown) in &map.kings {
            let second = king_relay(shown, trump);
            entries.extend(rows_of(
                node(&format!("{relay} {shown} -")),
                asker_after_queen(trump, true, second, five()),
            ));
            let Some(second) = second else {
                continue;
            };
            let asked = format!("{relay} {shown} - {} -", second.ask);
            entries.extend(rows_of(node(&asked), king_replies(trump, second)));
            for (reply, more) in [(second.more, true), (second.none, false)] {
                entries.extend(rows_of(
                    node(&format!("{asked} {reply} -")),
                    asker_after_relay_kings(trump, more),
                ));
            }
        }
    }

    // ponytail: no grand-slam king ask for minors — plain 4NT has no room for it
    // (5NT misreads as the ask; 6♣/6♦ king answers collide with the trump slam).
    // Grand-in-minor stays under-bid; the upgrade path is Kickback (out of scope).
    if matches!(trump, Suit::Clubs | Suit::Diamonds) {
        return entries;
    }

    // -----------------------------------------------------------------------
    // 3. King answers after the 5NT ask
    // -----------------------------------------------------------------------
    for answer in [ans_5c, ans_5d, ans_5h, ans_5s] {
        entries.extend(rows_of(
            node(&format!("{answer} - 5NT -")),
            king_answers(trump),
        ));
    }

    // -----------------------------------------------------------------------
    // 4. Asker after king answers
    // -----------------------------------------------------------------------
    for answer in [ans_5c, ans_5d, ans_5h, ans_5s] {
        let kings = format!("{answer} - 5NT -");
        entries.extend(rows_of(
            node(&format!("{kings} 6♣ -")),
            asker_after_6c(trump),
        ));
        entries.extend(rows_of(
            node(&format!("{kings} 6♦ -")),
            asker_after_6d(trump),
        ));
        // 6♥ is a king answer only when trumps are spades; over hearts it is
        // the catch-all signoff.
        if trump == Suit::Spades {
            entries.extend(rows_of(
                node(&format!("{kings} 6♥ -")),
                asker_after_6h(trump),
            ));
        }
    }

    entries
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
