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

#[cfg(test)]
use crate::bidding::Trie;
use crate::bidding::{Alert, Rules};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Rank, Strain, Suit};
use core::ops::RangeBounds;

/// Roman Keycard Blackwood — the artificial keycard ask, answers, and king ask
pub(super) const RKCB: Alert = Alert("rkcb");

/// Whether RKCB reaches agreed **minor** trumps — the book half of
/// [`set_rkcb_minors`][crate::bidding::instinct::set_rkcb_minors]
///
/// One agreement, two layers: the book authors the minor-suit plain-4NT
/// keycard at its two vehicles — the strong-2♣ minor raise (`2♣–2♦–3m–4m`,
/// opener asks with 28+ HCP instead of blind-jumping `6m` on 27+) and the
/// inverted minor raise (`1m–2m–3NT`, responder asks on `points(14..)` instead
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
pub(super) fn minor_keycard() -> bool {
    crate::bidding::instinct::minor_asks_now()
}

use crate::bidding::constraint::{described, hcp};
use crate::bidding::instinct::{
    KingRelay, RelayMap, king_relay, queen_ask_room, queen_fit, relay_map,
};
#[cfg(test)]
use crate::bidding::rows::compile_entries;
use crate::bidding::rows::{Entry, Pattern, rows_of};
use contract_bridge::Hand;

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

/// The queen cannot change what the asker bids: we hold it, or the fit alone
/// carries the small slam
///
/// The asker's test, and deliberately a rung below the answerer's
/// [`has_trump_queen`]: a nine-card fit is not a queen, but hearing "no queen"
/// over one changes nothing — six is bid anyway — so the round is not worth
/// spending.  Threshold `QUEEN_BUFF_FIT`.
fn queen_moot(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    let threshold = usize::from(crate::bidding::instinct::queen_buff_fit());
    described(
        format!("the {trump} queen cannot change the call"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].contains(Rank::Q)
                || hand[trump].len() + usize::from(context.inferences().partner().length(trump).min)
                    >= threshold
        },
    )
}

/// A ninth trump or a side-suit void — the values RKCB has no rung for
///
/// Paired with `!has_trump_queen` at the buff jump: partner asked for the queen
/// holding four keycards and will pass five over a denial, never learning that
/// the fit is a card longer than promised or that a suit is stopped by a void.
/// The threshold is `QUEEN_BUFF_FIT`.
fn trump_buff(
    trump: Suit,
) -> crate::bidding::constraint::Cons<impl crate::bidding::constraint::Constraint + Clone> {
    let threshold = usize::from(crate::bidding::instinct::queen_buff_fit());
    described(
        format!("a ninth {trump} or a void"),
        move |hand: Hand, context: &crate::bidding::context::Context<'_>| {
            hand[trump].len() + usize::from(context.inferences().partner().length(trump).min)
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

// ---------------------------------------------------------------------------
// The queen relay
// ---------------------------------------------------------------------------
//
// The book's 5♣ and 5♦ answers say nothing about the trump queen, so the asker
// has been betting six on four keycards blind.  The relay is one step above the
// answer, partner's two replies the next two rungs, and — on the queen-shown
// branch only — a king ask above that.  Geometry is shared with the floor
// ([`queen_ask_room`], [`relay_map`], [`king_relay`]) so the two ladders
// cannot drift.

/// Open an asker table with the queen relay, when the lane has room and
/// `interested` says the queen is what the placement turns on
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
    match queen_ask_room(answer, trump) {
        Some(relay) => rules
            .rule(relay, 160, interested & !queen_moot(trump))
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
/// for.
fn queen_replies(trump: Suit, map: &RelayMap) -> Rules {
    let mut rules = Rules::new();
    for (index, &(suit, call)) in map.kings.iter().enumerate() {
        let cheaper = map.kings[..index].iter().map(|&(s, _)| s).collect();
        rules = rules
            .rule(
                call,
                100,
                has_trump_queen(trump) & cheapest_king(suit, cheaper),
            )
            .alert(RKCB);
    }
    rules = rules
        .rule(
            map.no_king,
            100,
            has_trump_queen(trump) & kings_outside(trump, 0..=0),
        )
        .alert(RKCB)
        .rule(map.deny, 60, !has_trump_queen(trump) & trump_buff(trump));
    rules.rule(map.weak, 50, hcp(0..))
}

/// Asker's placement over a denied queen
///
/// Both denials are the agreed trump, so both are already a contract: five of
/// it stops there unless all five keycards are on the table, and six of it —
/// the ninth trump or the void — is passed.
///
/// `five` is the **combined**-count decode, which the answer lane owns: over a
/// one-or-four answer four of our own is all five, while over a none-or-three
/// answer four of our own is exactly four — the caller passes the right test
/// because only it knows which answer was heard.
fn asker_after_denial(
    trump: Suit,
    denial: Bid,
    five: crate::bidding::constraint::Cons<
        impl crate::bidding::constraint::Constraint + Clone + 'static,
    >,
) -> Rules {
    let t = Strain::from(trump);
    if denial == Bid::new(6, t) {
        return Rules::new().rule(Call::Pass, 50, hcp(0..));
    }
    Rules::new()
        .rule(Bid::new(6, t), 100, five)
        .rule(Call::Pass, 50, hcp(0..))
}

/// Asker's placement over a queen-and-king reply
///
/// Seven needs two side kings between the hands.  Partner named its cheapest,
/// so one of our own already makes two; with none, the **second relay** asks
/// for one more — and that is where kickback pays a second time, because the
/// relay is a step above partner's reply rather than an absolute 5NT.
///
/// Both ride the grand-zone **strength** gate on top of the combined count —
/// RKCB is a slam veto, not a slam seeker, so a partnership short of the grand
/// zone never spends the round.  `hcp(19..)` is the book's available proxy at
/// this node, and `five` is the lane's combined-count decode (see
/// [`asker_after_denial`]): a grand is never touched with a keycard out.
///
/// ponytail: raw HCP, because the book carries no combined-point machinery
/// here; the upgrade path is the floor's `points_and_net(combined_points(37))`
/// once the book's asker tables can see partner's shown strength.
fn asker_after_queen(
    trump: Suit,
    partner_king: bool,
    relay: Option<KingRelay>,
    five: crate::bidding::constraint::Cons<
        impl crate::bidding::constraint::Constraint + Clone + 'static,
    >,
) -> Rules {
    let t = Strain::from(trump);
    let mut rules = Rules::new();
    if partner_king {
        rules = rules.rule(
            Bid::new(7, t),
            150,
            five.clone() & kings_outside(trump, 1..) & hcp(19..),
        );
        if let Some(relay) = relay {
            rules = rules
                .rule(
                    relay.ask,
                    140,
                    five & kings_outside(trump, 0..=0) & hcp(19..),
                )
                .alert(RKCB);
        }
    } else {
        rules = rules.rule(
            Bid::new(7, t),
            150,
            five & kings_outside(trump, 2..) & hcp(19..),
        );
    }
    rules.rule(Bid::new(6, t), 100, hcp(0..))
}

/// Partner's reply to the second relay: one more king, or six of trumps
fn king_replies(trump: Suit, relay: KingRelay) -> Rules {
    Rules::new()
        .rule(relay.more, 100, kings_outside(trump, 2..))
        .alert(RKCB)
        // Six of the agreed trump is a contract, not a code.
        .rule(relay.none, 50, hcp(0..))
}

/// Asker's placement over the second relay's reply: seven on the second king,
/// and passing partner's six otherwise
fn asker_after_relay_kings(trump: Suit, more: bool) -> Rules {
    let rules = Rules::new();
    if more {
        rules.rule(Bid::new(7, Strain::from(trump)), 100, hcp(0..))
    } else {
        rules.rule(Call::Pass, 50, hcp(0..))
    }
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
    let rules = Rules::new().rule(Bid::new(6, t), 100, keycards(trump, 3..));
    if trump == Suit::Diamonds {
        rules.rule(Bid::new(5, t), 50, hcp(0..))
    } else {
        rules.rule(Call::Pass, 50, hcp(0..))
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
                100,
                keycards(trump, 2..=2) | keycards(trump, 4..),
            )
            .rule(Call::Pass, 50, hcp(0..))
    } else {
        no_room_six(trump)
    }
}

/// Asker with no room to stop below slam: bid 6-of-the-minor
///
/// Used for the 5♥/5♠ answers (both minors) and the clubs 5♦ answer — all sit
/// above 5-of-either-minor, so signing off below slam is impossible.
fn no_room_six(trump: Suit) -> Rules {
    Rules::new().rule(Bid::new(6, Strain::from(trump)), 100, hcp(0..))
}

/// King answers at the 5NT node (for all answer paths — shared table)
///
/// 5NT promises all five keycards; this asks for kings outside trumps.
///
/// For spades: 6♣ (0), 6♦ (1), 6♥ (2), 6♠ signoff (3 kings).
/// For hearts: 6♣ (0), 6♦ (1), 6♥ catch-all signoff (2+).
fn king_answers(trump: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(Bid::new(6, Strain::Clubs), 100, kings_outside(trump, 0..=0))
        .alert(RKCB)
        .rule(
            Bid::new(6, Strain::Diamonds),
            100,
            kings_outside(trump, 1..=1),
        )
        .alert(RKCB);

    match trump {
        Suit::Spades => {
            rules = rules
                .rule(
                    Bid::new(6, Strain::Hearts),
                    100,
                    kings_outside(trump, 2..=2),
                )
                .alert(RKCB)
                // 3 outside kings → 6♠ signoff (counting stops below 7)
                .rule(Bid::new(6, Strain::Spades), 50, hcp(0..));
        }
        Suit::Hearts => {
            // 6♥ is a catch-all signoff for 2+ outside kings
            rules = rules.rule(Bid::new(6, Strain::Hearts), 50, hcp(0..));
        }
        _ => unreachable!("the 5NT king ask is major-only; minors never install it"),
    }
    rules
}

/// Asker's call after a 6♣ king answer (0 outside kings)
fn asker_after_6c(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 100, kings_outside(trump, 3..))
        .rule(Bid::new(6, t), 50, hcp(0..))
}

/// Asker's call after a 6♦ king answer (1 outside king)
fn asker_after_6d(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 100, kings_outside(trump, 2..))
        .rule(Bid::new(6, t), 50, hcp(0..))
}

/// Asker's call after a 6♥ king answer (2 outside kings; only when trump == Spades)
fn asker_after_6h(trump: Suit) -> Rules {
    let t = Strain::from(trump);
    Rules::new()
        .rule(Bid::new(7, t), 100, kings_outside(trump, 1..))
        .rule(Bid::new(6, t), 50, hcp(0..))
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// The RKCB 1430 subtree as rows, below the auction `prefix`
///
/// `prefix` is an auction string ending just before **our 4NT ask** — the row
/// layer's spelling of the undisturbed sequence
/// [`install_rkcb`] takes as `&[Call]` (`"P* 1♠ (P) 3♠ (P)"`).  The ask, its
/// answers and every continuation are keyed below it.  Both majors and minors
/// are supported; for minors the asker's signoff is cramped (see the module
/// docs) and the 5NT king ask is skipped.
///
/// The 4NT bid itself must already be in the caller's table; this produces
/// everything that comes *after* 4NT.
pub(super) fn rkcb_rows(prefix: &str, trump: Suit) -> Vec<Entry> {
    let ans_5c = Bid::new(5, Strain::Clubs);
    let ans_5d = Bid::new(5, Strain::Diamonds);
    let ans_5h = Bid::new(5, Strain::Hearts);
    let ans_5s = Bid::new(5, Strain::Spades);

    // Every key hangs off our 4NT; the trailing `(P)` is the opposing pass
    // `uncontested` used to interleave by hand.
    let ask = format!("{prefix} 4NT (P)");
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
        entries.extend(rows_of(node(&format!("{answer} (P)")), table));
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
        let relay = format!("{answer} (P) {} (P)", map.ask);

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
                node(&format!("{relay} {denial} (P)")),
                asker_after_denial(trump, denial, five()),
            ));
        }
        entries.extend(rows_of(
            node(&format!("{relay} {} (P)", map.no_king)),
            asker_after_queen(trump, false, None, five()),
        ));
        for &(_, shown) in &map.kings {
            let second = king_relay(shown, trump);
            entries.extend(rows_of(
                node(&format!("{relay} {shown} (P)")),
                asker_after_queen(trump, true, second, five()),
            ));
            let Some(second) = second else {
                continue;
            };
            let asked = format!("{relay} {shown} (P) {} (P)", second.ask);
            entries.extend(rows_of(node(&asked), king_replies(trump, second)));
            for (reply, more) in [(second.more, true), (second.none, false)] {
                entries.extend(rows_of(
                    node(&format!("{asked} {reply} (P)")),
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
            node(&format!("{answer} (P) 5NT (P)")),
            king_answers(trump),
        ));
    }

    // -----------------------------------------------------------------------
    // 4. Asker after king answers
    // -----------------------------------------------------------------------
    for answer in [ans_5c, ans_5d, ans_5h, ans_5s] {
        let kings = format!("{answer} (P) 5NT (P)");
        entries.extend(rows_of(
            node(&format!("{kings} 6♣ (P)")),
            asker_after_6c(trump),
        ));
        entries.extend(rows_of(
            node(&format!("{kings} 6♦ (P)")),
            asker_after_6d(trump),
        ));
        // 6♥ is a king answer only when trumps are spades; over hearts it is
        // the catch-all signoff.
        if trump == Suit::Spades {
            entries.extend(rows_of(
                node(&format!("{kings} 6♥ (P)")),
                asker_after_6h(trump),
            ));
        }
    }

    entries
}

/// Install RKCB 1430 below an agreed trump suit
///
/// `our_calls` is the undisturbed sequence of our side's calls so far (the
/// same form [`uncontested`][super::uncontested] takes); the 4NT ask and its
/// answers are inserted below it.  A thin test-only shim over [`rkcb_rows`],
/// kept for the three fixtures that T1 converts before deleting it.
#[cfg(test)]
pub(super) fn install_rkcb(book: &mut Trie, our_calls: &[Call], trump: Suit) {
    let prefix = core::iter::once("P*".to_owned())
        .chain(our_calls.iter().map(|call| format!("{call} (P)")))
        .collect::<Vec<_>>()
        .join(" ");
    compile_entries(book, "rkcb", rkcb_rows(&prefix, trump));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
