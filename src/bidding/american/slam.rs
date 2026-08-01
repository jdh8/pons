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

use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Rank, Strain, Suit};
use core::cell::Cell;
use core::ops::RangeBounds;
use std::sync::Arc;

/// Roman Keycard Blackwood — the artificial keycard ask, answers, and king ask
pub(super) const RKCB: Alert = Alert("rkcb");

thread_local! {
    /// Plain-4NT keycard for agreed **minor** trumps; **on by default**.
    /// See [`set_minor_keycard`].
    static MINOR_KEYCARD: Cell<bool> = const { Cell::new(true) };
}

/// Author the plain-4NT minor-suit keycard for books built *after* this call
/// (thread-local; **on by default**).
///
/// Extends RKCB 1430 to agreed minor trumps at its two vehicles: the strong-2♣
/// minor raise (`2♣–2♦–3m–4m`, opener asks with 28+ HCP instead of
/// blind-jumping `6m` on 27+) and the inverted minor raise (`1m–2m–3NT`,
/// responder asks on `points(14..)` instead of resting in the 18–19 3NT).  The
/// answers reuse the trump-generic 1430 table; the asker's signoff is cramped
/// and the 5NT king ask skipped (module docs).  Off restores the pre-keycard
/// book byte-identically — the A/B off arm, since the original baseline
/// (reverting commit `99da1b3`) no longer applies to main.
///
/// Measured vs that floor: **+6.80/+8.76 IMPs/divergent** (none/both, 2M
/// boards), PD re-measure **+5.41/+7.05 IMPs/divergent** (10M boards, 202
/// divergent, ~1 in 49.5k) — rare but decisively positive per fire.
pub fn set_minor_keycard(on: bool) {
    MINOR_KEYCARD.with(|cell| cell.set(on));
}

/// Whether the minor-suit plain-4NT keycard is currently authored
pub(super) fn minor_keycard() -> bool {
    MINOR_KEYCARD.with(Cell::get)
}

use super::{insert_uncontested, uncontested};
use crate::bidding::constraint::{described, hcp};
use crate::bidding::instinct::{
    KingRelay, RelayMap, king_relay, queen_ask_now, queen_ask_room, queen_fit, relay_map,
};
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
/// worth a round of bidding.  The threshold is [`set_queen_fit`] (default 10,
/// BBA's bar, because this rung has to serve the grand); counted as our own
/// length plus the sound floor of partner's shown length, so neither seat can
/// claim a fit the auction has not shown.  A ninth trump is not a queen — it
/// answers on the buff jump instead ([`set_queen_buff_fit`]).  The arm rides
/// [`set_queen_ask`], so the knob-off answers keep their literal holding test
/// and stay byte-identical.
///
/// [`set_queen_buff_fit`]: crate::bidding::instinct::set_queen_buff_fit
///
/// [`set_queen_fit`]: crate::bidding::instinct::set_queen_fit
///
/// The knob is sampled **here, at book construction**, not inside the closure —
/// the regime every book knob lives in ([`set_minor_keycard`]).  Reading it at
/// classification time instead would leave a book built with the relay on
/// answering by the literal holding whenever a harness cleared the flag between
/// building and bidding, which is exactly the split the two-regime discipline
/// exists to prevent.
fn has_trump_queen(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    use crate::bidding::inference::Inferences;
    let long_fit_counts = queen_ask_now().then(|| usize::from(queen_fit()));
    described(
        format!("holds the {trump} queen"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].contains(Rank::Q)
                || long_fit_counts.is_some_and(|threshold| {
                    hand[trump].len()
                        + usize::from(Inferences::read(context).partner().length(trump).min)
                        >= threshold
                })
        },
    )
}

/// The queen cannot change what the asker bids: we hold it, or the fit alone
/// carries the small slam
///
/// The asker's test, and deliberately a rung below the answerer's
/// [`has_trump_queen`]: a nine-card fit is not a queen, but hearing "no queen"
/// over one changes nothing — six is bid anyway — so the round is not worth
/// spending.  Threshold [`set_queen_buff_fit`].
///
/// [`set_queen_buff_fit`]: crate::bidding::instinct::set_queen_buff_fit
fn queen_moot(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    use crate::bidding::inference::Inferences;
    let threshold =
        queen_ask_now().then(|| usize::from(crate::bidding::instinct::queen_buff_fit()));
    described(
        format!("the {trump} queen cannot change the call"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].contains(Rank::Q)
                || threshold.is_some_and(|threshold| {
                    hand[trump].len()
                        + usize::from(Inferences::read(context).partner().length(trump).min)
                        >= threshold
                })
        },
    )
}

/// A ninth trump or a side-suit void — the values RKCB has no rung for
///
/// Paired with `!has_trump_queen` at the buff jump: partner asked for the queen
/// holding four keycards and will pass five over a denial, never learning that
/// the fit is a card longer than promised or that a suit is stopped by a void.
/// The threshold is [`set_queen_buff_fit`], sampled at book construction like
/// every other book knob.
///
/// [`set_queen_buff_fit`]: crate::bidding::instinct::set_queen_buff_fit
fn trump_buff(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    use crate::bidding::inference::Inferences;
    let threshold = usize::from(crate::bidding::instinct::queen_buff_fit());
    described(
        format!("a ninth {trump} or a void"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].len() + usize::from(Inferences::read(context).partner().length(trump).min)
                >= threshold
                || Suit::ASC
                    .into_iter()
                    .any(|suit| suit != trump && hand[suit].is_empty())
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
            1.0,
            keycards(trump, 1..=1) | keycards(trump, 4..=4),
        )
        .alert(RKCB)
        // 5♦ = 0 or 3 keycards ("30")
        .rule(
            Bid::new(5, Strain::Diamonds),
            1.0,
            keycards(trump, 0..=0) | keycards(trump, 3..=3),
        )
        .alert(RKCB)
        // 5♥ = 2 or 5 keycards without the trump queen
        .rule(
            Bid::new(5, Strain::Hearts),
            1.0,
            (keycards(trump, 2..=2) | keycards(trump, 5..=5)) & !has_trump_queen(trump),
        )
        .alert(RKCB)
        // 5♠ = 2 or 5 keycards with the trump queen
        .rule(
            Bid::new(5, Strain::Spades),
            1.0,
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
    // 5NT: asker has 4 keycards + partner's 1 = all five → king ask
    .rule(Bid::new(5, Strain::Notrump), 1.4, keycards(trump, 4..=4))
    .alert(RKCB)
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
        1.0,
        keycards(trump, 2..=2) | keycards(trump, 4..),
    )
    // 5T: signoff (asker has ≥3 and knows partner has 0)
    .rule(Bid::new(5, t), 0.5, hcp(0..))
}

// ---------------------------------------------------------------------------
// The queen relay (see `set_queen_ask`)
// ---------------------------------------------------------------------------
//
// The book's 5♣ and 5♦ answers say nothing about the trump queen, so the asker
// has been betting six on four keycards blind.  The relay is one step above the
// answer, partner's two replies the next two rungs, and — on the queen-shown
// branch only — a king ask above that.  Geometry is shared with the floor
// ([`queen_ask_room`], [`relay_ladder`]) so the two ladders cannot drift.

/// Open an asker table with the queen relay, when the knob is on, the lane has
/// room, and `interested` says the queen is what the placement turns on
///
/// Weighted above every placement in the table it opens, so a queenless asker
/// relays instead of guessing; holding the queen (or the ten-card fit) the rule
/// is dead and the table below is exactly what shipped.
fn relay_first(
    trump: Suit,
    answer: Bid,
    interested: crate::bidding::constraint::Cons<
        impl crate::bidding::constraint::Constraint + Clone + 'static,
    >,
) -> Rules {
    let rules = Rules::new();
    if !queen_ask_now() {
        return rules;
    }
    match queen_ask_room(answer, trump) {
        Some(relay) => rules
            .rule(relay, 1.6, interested & !queen_moot(trump))
            .alert(RKCB),
        None => rules,
    }
}

/// Holds the king of `suit` and of none of the suits on `cheaper` rungs — the
/// "skipped steps deny" half of a king rung
fn cheapest_king(
    suit: Suit,
    cheaper: Vec<Suit>,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    described(
        format!("holds the {suit} king and no cheaper side king"),
        move |hand: Hand, _: &crate::bidding::context::Context<'_>| {
            hand[suit].contains(Rank::K) && cheaper.iter().all(|&s| !hand[s].contains(Rank::K))
        },
    )
}

/// Partner's merged reply to the queen relay
///
/// One round carries the queen and a king both: each side suit's rung shows the
/// queen plus that king and denies every king on a cheaper rung, 5NT shows the
/// queen with no side king at all, and the two denials are five and six of the
/// agreed trump — contracts rather than codes, so neither is alerted.  Six is
/// the *stronger* denial, the ninth trump or the void the ladder has no rung
/// for; where five of trump is already gone it carries every no-queen hand.
fn queen_replies(trump: Suit, map: &RelayMap) -> Rules {
    let mut rules = Rules::new();
    for (index, &(suit, call)) in map.kings.iter().enumerate() {
        let cheaper = map.kings[..index].iter().map(|&(s, _)| s).collect();
        rules = rules
            .rule(
                call,
                1.0,
                has_trump_queen(trump) & cheapest_king(suit, cheaper),
            )
            .alert(RKCB);
    }
    rules = rules
        .rule(
            map.no_king,
            1.0,
            has_trump_queen(trump) & kings_outside(trump, 0..=0),
        )
        .alert(RKCB)
        .rule(map.deny, 0.6, !has_trump_queen(trump) & trump_buff(trump));
    rules.rule(map.weak, 0.5, hcp(0..))
}

/// Asker's placement over a denied queen
///
/// Both denials are the agreed trump, so both are already a contract: five of
/// it stops there unless all five keycards are on the table, and six of it —
/// the ninth trump or the void — is passed.
fn asker_after_denial(trump: Suit, denial: Bid) -> Rules {
    let t = Strain::from(trump);
    if denial == Bid::new(6, t) {
        return Rules::new().rule(Call::Pass, 0.5, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(6, t), 1.0, keycards(trump, 4..))
        .rule(Call::Pass, 0.5, hcp(0..))
}

/// Asker's placement over a queen-and-king reply
///
/// Seven needs two side kings between the hands.  Partner named its cheapest,
/// so one of our own already makes two; with none, the **second relay** asks
/// for one more — and that is where kickback pays a second time, because the
/// relay is a step above partner's reply rather than an absolute 5NT.
///
/// Both are gated on **strength**, not on the keycard count — RKCB is a slam
/// veto, not a slam seeker, so a partnership short of the grand zone never
/// spends the round.  `hcp(19..)` is the book's available proxy at this node.
///
/// ponytail: raw HCP, because the book carries no combined-point machinery
/// here; the upgrade path is the floor's `points_and_net(combined_points(37))`
/// once the book's asker tables can see partner's shown strength.
fn asker_after_queen(trump: Suit, partner_king: bool, relay: Option<KingRelay>) -> Rules {
    let t = Strain::from(trump);
    let mut rules = Rules::new();
    if partner_king {
        rules = rules.rule(
            Bid::new(7, t),
            1.5,
            keycards(trump, 4..=4) & kings_outside(trump, 1..) & hcp(19..),
        );
        if let Some(relay) = relay {
            rules = rules
                .rule(
                    relay.ask,
                    1.4,
                    keycards(trump, 4..=4) & kings_outside(trump, 0..=0) & hcp(19..),
                )
                .alert(RKCB);
        }
    } else {
        rules = rules.rule(
            Bid::new(7, t),
            1.5,
            keycards(trump, 4..=4) & kings_outside(trump, 2..) & hcp(19..),
        );
    }
    rules.rule(Bid::new(6, t), 1.0, hcp(0..))
}

/// Partner's reply to the second relay: one more king, or six of trumps
fn king_replies(trump: Suit, relay: KingRelay) -> Rules {
    Rules::new()
        .rule(relay.more, 1.0, kings_outside(trump, 2..))
        .alert(RKCB)
        // Six of the agreed trump is a contract, not a code.
        .rule(relay.none, 0.5, hcp(0..))
}

/// Asker's placement over the second relay's reply: seven on the second king,
/// and passing partner's six otherwise
fn asker_after_relay_kings(trump: Suit, more: bool) -> Rules {
    let rules = Rules::new();
    if more {
        rules.rule(Bid::new(7, Strain::from(trump)), 1.0, hcp(0..))
    } else {
        rules.rule(Call::Pass, 0.5, hcp(0..))
    }
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
        .alert(RKCB)
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
        .alert(RKCB)
        .rule(
            Bid::new(6, Strain::Diamonds),
            1.0,
            kings_outside(trump, 1..=1),
        )
        .alert(RKCB);

    match trump {
        Suit::Spades => {
            rules = rules
                .rule(
                    Bid::new(6, Strain::Hearts),
                    1.0,
                    kings_outside(trump, 2..=2),
                )
                .alert(RKCB)
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

    // -----------------------------------------------------------------------
    // 2b. The queen relay, where the lane has room for it
    //     `our_calls + [4NT, ans, relay, reply…]`
    // -----------------------------------------------------------------------
    //
    // Only the two ambiguous answers grow one — 5♥ and 5♠ already disclose the
    // queen — and only where `queen_ask_room` says the no-queen rung still
    // lands at or below five of trump, which excludes both plain-4NT minors and
    // hearts after a 0-or-3.  Those lanes keep betting the small slam on four
    // keycards, exactly as they do today.
    if queen_ask_now() {
        for &answer in &[ans_5c, ans_5d] {
            let Call::Bid(answer_bid) = answer else {
                continue;
            };
            let Some(map) = relay_map(answer_bid, trump) else {
                continue;
            };
            let relay = Call::Bid(map.ask);

            insert_uncontested(book, &extend(&[answer, relay]), queen_replies(trump, &map));
            for denial in [map.weak, map.deny] {
                insert_uncontested(
                    book,
                    &extend(&[answer, relay, Call::Bid(denial)]),
                    asker_after_denial(trump, denial),
                );
            }
            insert_uncontested(
                book,
                &extend(&[answer, relay, Call::Bid(map.no_king)]),
                asker_after_queen(trump, false, None),
            );
            for &(_, shown) in &map.kings {
                let second = king_relay(shown, trump);
                insert_uncontested(
                    book,
                    &extend(&[answer, relay, Call::Bid(shown)]),
                    asker_after_queen(trump, true, second),
                );
                let Some(second) = second else {
                    continue;
                };
                let ask = Call::Bid(second.ask);
                insert_uncontested(
                    book,
                    &extend(&[answer, relay, Call::Bid(shown), ask]),
                    king_replies(trump, second),
                );
                for (reply, more) in [(second.more, true), (second.none, false)] {
                    insert_uncontested(
                        book,
                        &extend(&[answer, relay, Call::Bid(shown), ask, Call::Bid(reply)]),
                        asker_after_relay_kings(trump, more),
                    );
                }
            }
        }
    }

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

        // AK732.A53.A42.A2 — 4 aces + trump K = 5 keycards, no Q → 5♥ (same step as 2)
        assert_eq!(
            best(&trie, ANS_AUCTION, "AK732.A53.A42.A2"),
            Call::Bid(Bid::new(5, Strain::Hearts)),
            "5 keycards, no trump Q → 5♥"
        );

        // AKQ32.A53.A42.A2 — 4 aces + trump K + trump Q = 5 keycards with Q → 5♠
        assert_eq!(
            best(&trie, ANS_AUCTION, "AKQ32.A53.A42.A2"),
            Call::Bid(Bid::new(5, Strain::Spades)),
            "5 keycards, with trump Q → 5♠"
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

    // -----------------------------------------------------------------------
    // The queen relay (`set_queen_ask`)
    // -----------------------------------------------------------------------

    /// A spade-trump book with the relay authored.  The knob is read at book
    /// *construction*, so an arm has to build its own trie — the regime
    /// [`set_minor_keycard`] already lives in.
    fn relay_trie() -> Trie {
        crate::bidding::instinct::set_queen_ask(true);
        let trie = rkcb_trie();
        crate::bidding::instinct::set_queen_ask(false);
        trie
    }

    /// A spade book reached through a **limit raise**, so partner is shown for
    /// only three trumps and the fit is eight — the one length where the trump
    /// queen still decides between five and six ([`set_queen_fit`], default 9).
    /// The Jacoby-2NT book above promises four-plus opposite a five-card major,
    /// so its fit is nine and the relay is correctly dead there.
    ///
    /// [`set_queen_fit`]: crate::bidding::instinct::set_queen_fit
    fn eight_card_relay_trie() -> Trie {
        crate::bidding::instinct::set_queen_ask(true);
        let mut trie = Trie::new();
        let our_calls = [
            Call::Bid(Bid::new(1, Strain::Spades)),
            Call::Bid(Bid::new(3, Strain::Spades)),
        ];
        install_rkcb(&mut trie, &our_calls, Suit::Spades);
        crate::bidding::instinct::set_queen_ask(false);
        trie
    }

    /// `[1♠, P, 3♠, P, 4NT, P]` — the limit-raise ask node
    const LIMIT_ANS_AUCTION: &[Call] = &[
        Call::Bid(Bid::new(1, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(3, Strain::Spades)),
        Call::Pass,
        Call::Bid(Bid::new(4, Strain::Notrump)),
        Call::Pass,
    ];

    /// A nine-card fit answers the queen question by itself, so the relay never
    /// starts — Jacoby 2NT promises four-plus opposite five.
    #[test]
    fn nine_card_fit_needs_no_relay() {
        let trie = relay_trie();
        let mut auction = ANS_AUCTION.to_vec();
        auction.extend([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass]);
        assert_eq!(
            best(&trie, &auction, "AKJ8.AK2.KJ32.42"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "four trumps opposite a shown five is nine: bid six, do not ask"
        );
    }

    /// Knob off, the book is byte-identical: no relay node exists, and the
    /// queenless asker bets the small slam as it always has.
    #[test]
    fn relay_absent_off_the_knob() {
        let trie = rkcb_trie();
        let mut auction = ANS_AUCTION.to_vec();
        auction.extend([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass]);
        // Four spades (the Jacoby minimum, so no ten-card fit to stand in for
        // the queen) and three keycards: ♠A, ♥A, ♠K.
        assert_eq!(
            best(&trie, &auction, "AKJ8.AK2.KJ32.42"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "off the knob: four combined keycards bets six without asking"
        );
        auction.extend([Call::Bid(Bid::new(5, Strain::Diamonds)), Call::Pass]);
        let hand: Hand = "KQ432.53.842.932".parse().unwrap();
        assert!(
            trie.classify(hand, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "off the knob there is no relay node for 5♦ to land on"
        );
    }

    /// The relay itself: the queenless asker asks, partner answers on the two
    /// rungs above, and the asker places the contract on the reply.
    #[test]
    fn relay_asks_answers_and_places() {
        let trie = eight_card_relay_trie();
        let mut auction = LIMIT_ANS_AUCTION.to_vec();
        auction.extend([Call::Bid(Bid::new(5, Strain::Clubs)), Call::Pass]);

        // Three keycards, no trump queen → 5♦ relays instead of guessing.
        assert_eq!(
            best(&trie, &auction, "AKJ8.AK2.KJ32.42"),
            Call::Bid(Bid::new(5, Strain::Diamonds)),
            "queenless: ask the queen"
        );
        // The same count holding it → the relay is dead, bid the slam.
        assert_eq!(
            best(&trie, &auction, "AKQ8.AK2.KJ32.42"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "our own queen settles it: no relay"
        );
        // Four keycards decodes to all five combined, so six is bid whatever
        // the queen does.  Without the values to look at seven the reply is
        // worth nothing, so the book does not spend the round asking for it.
        assert_eq!(
            best(&trie, &auction, "AK98.A32.A432.32"),
            Call::Bid(Bid::new(5, Strain::Notrump)),
            "all five keycards, no grand values: no queen relay"
        );

        // Partner replies in one round: 5♠ denies flat, 6♠ denies with a buff,
        // 5♥/6♣/6♦ show the queen *and* the cheapest side king, 5NT shows the
        // queen with none.  Three trumps opposite the opener's shown five is
        // eight, the one length where only the honour itself can answer.
        auction.extend([Call::Bid(Bid::new(5, Strain::Diamonds)), Call::Pass]);
        assert_eq!(
            best(&trie, &auction, "K74.A653.8432.92"),
            Call::Bid(Bid::new(5, Strain::Spades)),
            "no trump queen → five of trump, which is the signoff too"
        );
        assert_eq!(
            best(&trie, &auction, "KQ4.A653.8432.92"),
            Call::Bid(Bid::new(5, Strain::Notrump)),
            "trump queen, no side king → 5NT"
        );
        assert_eq!(
            best(&trie, &auction, "KQ4.K653.8432.92"),
            Call::Bid(Bid::new(5, Strain::Hearts)),
            "trump queen and the ♥ king → the cheapest king rung"
        );
        assert_eq!(
            best(&trie, &auction, "KQ4.6532.K843.92"),
            Call::Bid(Bid::new(6, Strain::Diamonds)),
            "the ♦ king with no cheaper one → the rung above, skipping denies"
        );
        // A fifth trump opposite the opener's shown five is ten, and ten runs
        // the suit without the honour — the one length that may claim it.
        assert_eq!(
            best(&trie, &auction, "K7432.A65.843.92"),
            Call::Bid(Bid::new(5, Strain::Notrump)),
            "the tenth trump stands in for the queen"
        );
        // Nine is the in-between: not a queen, but far too good to let partner
        // pass five over a denial.  Jump to six and say so.
        assert_eq!(
            best(&trie, &auction, "K743.A653.843.92"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "the ninth trump is a buff, not a queen: bid six"
        );
        // The same nine-card fit holding the honour still shows it — the buff
        // jump is for hands that have nothing to show, not a substitute for
        // the rung above.
        assert_eq!(
            best(&trie, &auction, "KQ43.A653.843.92"),
            Call::Bid(Bid::new(5, Strain::Notrump)),
            "queen in hand: a show rung, not the jump"
        );
        // A void rides the same jump: worth a trick the ladder cannot show,
        // and partner is about to pass five without ever hearing about it.
        assert_eq!(
            best(&trie, &auction, "K74.A6532.8432."),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "an eight-card fit with a void: still worth six"
        );

        // The asker places it.  Three keycards is four combined: a denied
        // queen leaves a keycard *and* the queen out, so stop at five.
        let mut denied = auction.clone();
        denied.extend([Call::Bid(Bid::new(5, Strain::Spades)), Call::Pass]);
        assert_eq!(
            best(&trie, &denied, "AKJ8.AK2.KJ32.42"),
            Call::Pass,
            "queen denied on four keycards: the denial is already the contract"
        );
        let mut shown = auction;
        shown.extend([Call::Bid(Bid::new(5, Strain::Notrump)), Call::Pass]);
        assert_eq!(
            best(&trie, &shown, "AKJ8.AK2.KJ32.42"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "queen shown on four keycards: bid the slam"
        );
    }

    /// Seven is explored only when the values are there, and bid on two of the
    /// three side kings — RKCB is a slam veto, not a slam seeker.  The merged
    /// reply names one of them, so the second relay is spent only when the
    /// asker holds none of its own.
    #[test]
    fn relay_king_ask_needs_the_grand_values() {
        let trie = eight_card_relay_trie();
        let mut shown = LIMIT_ANS_AUCTION.to_vec();
        // 5♥ shows the trump queen and the ♥ king, denying nothing cheaper.
        shown.extend([
            Call::Bid(Bid::new(5, Strain::Clubs)),
            Call::Pass,
            Call::Bid(Bid::new(5, Strain::Diamonds)),
            Call::Pass,
            Call::Bid(Bid::new(5, Strain::Hearts)),
            Call::Pass,
        ]);
        // ♠AK98 ♥A32 ♦A432 ♣32 — four keycards, so all five are on the table
        // and the queen is shown, but 15 HCP is not a grand-zone hand: six.
        assert_eq!(
            best(&trie, &shown, "AK98.A32.A432.32"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "all five keycards and the queen, no grand values: six, never a second relay"
        );
        // ♠AK98 ♥AK2 ♦AK32 ♣32 — 21 HCP and a side king of its own opposite
        // partner's: two are already shown, so bid seven without asking again.
        assert_eq!(
            best(&trie, &shown, "AK98.AK2.AK32.32"),
            Call::Bid(Bid::new(7, Strain::Spades)),
            "one king each, shown in a single round: grand"
        );
        // ♠AKQJ ♥A32 ♦AQ32 ♣32 — 20 HCP, four keycards, and not one side king:
        // the second king is the whole question, so relay again at 5♠.
        assert_eq!(
            best(&trie, &shown, "AKQJ.A32.AQ32.32"),
            Call::Bid(Bid::new(5, Strain::Spades)),
            "grand values but no side king of our own: ask for a second"
        );

        let mut asked = shown;
        asked.extend([Call::Bid(Bid::new(5, Strain::Spades)), Call::Pass]);
        assert_eq!(
            best(&trie, &asked, "Q743.K65.K42.92"),
            Call::Bid(Bid::new(5, Strain::Notrump)),
            "a second side king → the cheap rung"
        );
        assert_eq!(
            best(&trie, &asked, "Q743.K65.842.92"),
            Call::Bid(Bid::new(6, Strain::Spades)),
            "only the king already shown → six of trumps ends it"
        );

        let mut answered = asked.clone();
        answered.extend([Call::Bid(Bid::new(5, Strain::Notrump)), Call::Pass]);
        assert_eq!(
            best(&trie, &answered, "AKQJ.A32.AQ32.32"),
            Call::Bid(Bid::new(7, Strain::Spades)),
            "two of the three side kings between the hands: grand"
        );
        let mut stopped = asked;
        stopped.extend([Call::Bid(Bid::new(6, Strain::Spades)), Call::Pass]);
        assert_eq!(
            best(&trie, &stopped, "AKQJ.A32.AQ32.32"),
            Call::Pass,
            "only partner's king: six is already the contract"
        );
    }
}
