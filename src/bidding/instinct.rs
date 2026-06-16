//! The instinct bidder: a keyless floor for off-book auctions
//!
//! Competitive auctions cannot be enumerated — interference multiplies
//! sequences combinatorially, and a book that stops mid-auction leaves the
//! driver to pass by default.  The worst of those defaults is passing
//! partner's takeout double on a worthless hand, turning a routine advance
//! into a doubled partscore for the opponents.
//!
//! [`instinct()`] is the floor under the book: one context-driven [`Rules`]
//! ladder that answers *every* auction with a sane natural action.  Attach it
//! as a root [`Always`][super::fallback::Always] fallback — as
//! [`two_over_one()`][crate::bidding::two_over_one::two_over_one] does for its
//! competitive and defensive books — and the system never falls off the book.
//! By [`Trie::resolve`][super::Trie::resolve] precedence the root is reached
//! last, so instinct can never override an authored rule, only catch what
//! falls past all of them.
//!
//! # Everything is natural
//!
//! Instinct fires precisely where the book has no agreement, so partner's
//! continuation is usually off-book too — decoded by *partner's* instinct.
//! The two halves stay coherent because every instinct call is natural:
//! bids show the bid suit, raises show support, doubles are takeout.  No
//! conventional calls (in particular no strength-showing cue-bids) belong
//! here until both sides of the convention are authored.
//!
//! # The forced advance
//!
//! The one situation instinct must *not* treat as optional is partner's live
//! takeout double: the auction ends `… (bid) X (Pass)` with their suit bid at
//! the three level or below doubled by partner.  There, the pass rules are
//! replaced by an advance ladder — penalty pass only with a genuine trump
//! stack, a major-suit game jump or 3NT with values, the longest unbid suit
//! at the cheapest level, and a notrump escape so that *some* action is
//! always available.  The interpretation of the double is deliberately
//! mechanical: a classifier may know its system, and instinct's system is
//! plain standard.
//!
//! # Observability
//!
//! Instinct activations are visible in the
//! [`Provenance`][super::trie::Provenance] returned by
//! [`Trie::resolve`][super::Trie::resolve]: `depth == 0` with
//! `fallback == Some(_)` is the floor firing.  In simulation, count these —
//! the most-hit auctions are the next nodes worth authoring properly.

use super::Rules;
use super::constraint::{
    Cons, Constraint, balanced, hcp, len, min_level_is, partner_shown_len, partner_suit_is,
    point_count, points, pred, short_in_their_suits, stopper_in_their_suits, support, they_bid,
};
use super::context::Context;
use super::inference::Inferences;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Penalty, Rank, Strain, Suit};
use core::cell::Cell;

std::thread_local! {
    /// Whether the floor consults the auction interpretation for known fits
    static INFERENCE_AWARE: Cell<bool> = const { Cell::new(true) };
}

/// Enable or disable inference-aware instinct rules on the current thread
///
/// For A/B measurement only (see the `inference-floor` example): with it
/// disabled the floor ignores partner's shown shape, falling back to the
/// shape-blind 3NT / six-card-major game selection.  The flag is read at
/// classification time and is per-thread; classify on the thread that set it.
#[doc(hidden)]
pub fn set_inference_aware(enabled: bool) {
    INFERENCE_AWARE.with(|flag| flag.set(enabled));
}

/// The floor is consulting the auction interpretation (see [`set_inference_aware`])
fn inference_aware() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| INFERENCE_AWARE.with(Cell::get))
}

/// Partner's takeout double is live: the auction ends `… (bid) X (Pass)`
///
/// Mechanically: the last two calls are partner's double and RHO's pass, and
/// the doubled contract is their suit bid at the three level or below —
/// doubles of notrump or of game-level contracts read as penalty, not as a
/// request to act.
fn forced_advance_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let n = auction.len();
    n >= 2
        && auction[n - 1] == Call::Pass
        && auction[n - 2] == Call::Double
        && context
            .last_bid()
            .is_some_and(|bid| bid.strain.suit().is_some() && bid.level.get() <= 3)
}

/// [`forced_advance_now`] as a hand-ignoring [`Constraint`] for the ladder
fn forced_advance() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| forced_advance_now(context))
}

/// A trump stack in the doubled suit: four-plus cards with two top honors
///
/// The one holding that converts partner's takeout double into penalties.
fn doubled_suit_stack() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        context
            .last_bid()
            .and_then(|bid| bid.strain.suit())
            .is_some_and(|suit| {
                let holding = hand[suit];
                let honors = [Rank::A, Rank::K, Rank::Q]
                    .into_iter()
                    .filter(|&rank| holding.contains(rank))
                    .count();
                holding.len() >= 4 && honors >= 2
            })
    })
}

/// Our side has not bid yet (doubles and passes do not count)
///
/// The anchor for overcall-shaped actions: once we have shown a suit or
/// notrump, instinct competes by raising or doubling instead.
fn we_have_not_bid() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        !Suit::ASC
            .into_iter()
            .map(Strain::from)
            .chain([Strain::Notrump])
            .any(|strain| context.we_bid(strain))
    })
}

/// The opponents' undoubled suit bid at most `level` is the call to beat
///
/// This is the legality *and* sanity anchor for instinct doubles: the last
/// non-pass call is an opposing suit bid, not yet doubled, low enough that a
/// double still reads as takeout.
fn their_live_bid_at_most(level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        context.penalty() == Penalty::Undoubled
            && context
                .last_bid()
                .is_some_and(|bid| bid.strain.suit().is_some() && bid.level.get() <= level)
            && context
                .auction()
                .iter()
                .rposition(|&call| call != Call::Pass)
                .is_some_and(|index| (context.auction().len() - index) % 2 == 1)
    })
}

/// The strain is still biddable at or below the given level
fn level_available(level: u8, strain: Strain) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        context
            .min_level(strain)
            .is_some_and(|min| min.get() <= level)
    })
}

/// The opening bid (first non-pass call) and its index, if it is a bid
fn opening_bid(auction: &[Call]) -> Option<(usize, Bid)> {
    let index = auction.iter().position(|&call| call != Call::Pass)?;
    match auction[index] {
        Call::Bid(bid) => Some((index, bid)),
        _ => None,
    }
}

/// Our side opened a strong notrump of `level`, and the player to act is its
/// opener (`partner == false`) or its responder (`partner == true`)
///
/// This is one of the two conventions instinct reads (the other is the strong
/// 2♣ — see [`forcing_two_clubs_response`]): a strong notrump opening is the
/// anchor for completing transfers and refusing to pass below a forced game,
/// the deep conventional structures the book may not author.
fn our_strong_notrump(context: &Context<'_>, level: u8, partner: bool) -> bool {
    let auction = context.auction();
    let Some((index, bid)) = opening_bid(auction) else {
        return false;
    };
    // Our side owns the indices sharing the player-to-act's parity.
    if index % 2 != auction.len() % 2 {
        return false;
    }
    if bid.strain != Strain::Notrump || bid.level.get() != level {
        return false;
    }
    // Seats four apart are the same player; two apart are partners.
    match (auction.len() - index) % 4 {
        0 => !partner,
        2 => partner,
        _ => false,
    }
}

/// Partner's call immediately before ours, if it was a bid
fn partner_last_call(auction: &[Call]) -> Option<Bid> {
    match auction.len().checked_sub(2).map(|i| auction[i]) {
        Some(Call::Bid(bid)) => Some(bid),
        _ => None,
    }
}

/// The current contract is below game: no bid, or a partscore-level suit bid
fn below_game() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        context.last_bid().is_none_or(|bid| {
            let level = bid.level.get();
            level <= 2 || (level == 3 && bid.strain != Strain::Notrump)
        })
    })
}

/// The current contract is below slam: nothing above the five level yet
fn below_slam() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.last_bid().is_none_or(|bid| bid.level.get() <= 5))
}

/// Our side holds at least `threshold` combined points: our exact count plus the
/// *sound floor* of partner's shown points ([`Inferences`]), so the true total
/// is never less than the test admits
///
/// This is the general game/slam trigger.  Where the special-cased forces (a
/// strong-notrump responder, a strong 2♣) encode a single auction, this fires on
/// *any* auction whose shown strength reaches a milestone — the inference floor
/// makes it sound, never an overbid on a hand that could be weaker than counted.
///
/// [`Inferences`]: super::inference::Inferences
fn combined_points(threshold: u8) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        let partner_min = Inferences::read(context).partner().points.min;
        u16::from(point_count(hand)) + u16::from(partner_min) >= u16::from(threshold)
    })
}

/// Partner opened a strong notrump of `level` (we are the responder)
fn partner_strong_notrump(level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| our_strong_notrump(context, level, true))
}

/// We opened a strong notrump and partner forced past invitation with a
/// three-level suit bid — so passing below game is wrong, whatever our hand
fn opener_forced_past_invitation(context: &Context<'_>) -> bool {
    (our_strong_notrump(context, 1, false) || our_strong_notrump(context, 2, false))
        && partner_last_call(context.auction())
            .is_some_and(|bid| bid.level.get() == 3 && bid.strain != Strain::Notrump)
}

/// Our side opened a strong 2♣ and responder answered past the double negative
///
/// The artificial `2♣` promises 22+ and is forcing — but for one round only.
/// Responder's *answer* settles the game force: the 0–3 HCP double negative
/// (`2♥`) keeps open the option to stop short, while every other response — the
/// waiting `2♦` or a natural positive — commits *both* partners to at least
/// game.  So the force is read off responder's call, not off the 2♣ opening.
/// (Interference, where responder's seat holds a pass or double rather than a
/// response, is out of scope and reads as not forced.)
fn forcing_two_clubs_response(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let Some((index, bid)) = opening_bid(auction) else {
        return false;
    };
    // The player to act must be on the opening side — opener or responder.
    if index % 2 != auction.len() % 2 {
        return false;
    }
    if bid != Bid::new(2, Strain::Clubs) {
        return false;
    }
    // Responder sits two seats past the opening; the force is on once that
    // answer is in and is any bid other than the double-negative 2♥.
    matches!(
        auction.get(index + 2),
        Some(&Call::Bid(response)) if response != Bid::new(2, Strain::Hearts)
    )
}

/// We are sitting for a penalty: the live contract is the opponents' bid
/// doubled (or redoubled) by our side
///
/// Since a side may only double the other, a doubled contract whose last bid is
/// theirs was doubled by us — passing it out is the intended penalty action.
fn penalizing(context: &Context<'_>) -> bool {
    let auction = context.auction();
    context.penalty() != Penalty::Undoubled
        && auction
            .iter()
            .rposition(|call| matches!(call, Call::Bid(_)))
            .is_some_and(|index| (auction.len() - index) % 2 == 1)
}

/// Instinct's reading of an auction: the system intent the laws-only [`Context`]
/// deliberately omits, reconstructed from the immutable auction on demand
///
/// There is no per-classification scratchpad to cache this in, so each flag is
/// recovered by a short walk of the auction whenever the floor consults it.
/// Every flag here is *hand-independent* — it follows from the calls alone — so
/// hand-conditioned forces (a strong-notrump responder who holds game values)
/// stay as ordinary [`Constraint`]s rather than living here.
#[derive(Clone, Copy, Debug)]
struct Interpretation {
    /// Our side is committed to at least game by a prior call: a strong 2♣
    /// whose response cleared the double negative, or an opener forced past
    /// invitation opposite our strong notrump.
    forced_to_game: bool,
    /// We are sitting for our own penalty double, so passing below game is the
    /// intended action rather than a missed game.
    penalizing: bool,
}

impl Interpretation {
    /// Read the auction's intent from its [`Context`]
    fn read(context: &Context<'_>) -> Self {
        Self {
            forced_to_game: forcing_two_clubs_response(context)
                || opener_forced_past_invitation(context),
            penalizing: penalizing(context),
        }
    }
}

/// A prior call has committed our side to game (see [`Interpretation`])
fn auction_forces_game() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| Interpretation::read(context).forced_to_game)
}

/// We are not sitting for a penalty double of our own (see [`Interpretation`])
///
/// A game force forbids passing below game — *unless* we are penalizing the
/// opponents, where passing their doubled contract out is the whole point.
/// There the forced-to-game rules step aside and let the natural defense —
/// including the [forced advance][forced_advance] of partner's penalty double —
/// govern.
fn not_penalizing() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| !Interpretation::read(context).penalizing)
}

/// We opened the strong notrump of `nt_level` and partner just transferred with
/// the call `from` — the cue to complete the transfer
fn partner_transferred_now(context: &Context<'_>, from: Bid, nt_level: u8) -> bool {
    our_strong_notrump(context, nt_level, false)
        && partner_last_call(context.auction()) == Some(from)
}

/// [`partner_transferred_now`] as a hand-ignoring [`Constraint`] for the ladder
fn partner_transferred(from: Bid, nt_level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| partner_transferred_now(context, from, nt_level))
}

/// The transfers instinct completes opposite our own strong notrump, each
/// `(nt_level, partner's artificial call, completion)`
///
/// Standard Jacoby (2♦/2♥ over 1NT, 3♦/3♥ over 2NT) and South African Texas
/// (4♣/4♦).  Shared by the ladder's completion rules and the [`forced`] rail
/// predicate so the two never disagree on which transfers are in force.
const TRANSFERS: [(u8, Bid, Bid); 6] = [
    (
        1,
        Bid::new(2, Strain::Diamonds),
        Bid::new(2, Strain::Hearts),
    ),
    (1, Bid::new(2, Strain::Hearts), Bid::new(2, Strain::Spades)),
    (1, Bid::new(4, Strain::Clubs), Bid::new(4, Strain::Hearts)),
    (
        1,
        Bid::new(4, Strain::Diamonds),
        Bid::new(4, Strain::Spades),
    ),
    (
        2,
        Bid::new(3, Strain::Diamonds),
        Bid::new(3, Strain::Hearts),
    ),
    (2, Bid::new(3, Strain::Hearts), Bid::new(3, Strain::Spades)),
];

/// An auction-determined forced situation: partner's live takeout double, a
/// prior call committing our side to game, or partner's just-made transfer over
/// our strong notrump
///
/// Hand-independent — it follows from the calls alone.  The neural safety shell
/// consults it to decide when to delegate to the deterministic [`instinct()`]
/// ladder instead of trusting the learned net: the net handles the judgement
/// middle, but never these forced rails.  Hand-conditioned forces (a
/// strong-notrump responder who holds game values) are deliberately excluded —
/// they are judgement the net is trusted with, measured on the harness.
#[cfg(feature = "neural-floor")]
pub(crate) fn forced(context: &Context<'_>) -> bool {
    forced_advance_now(context)
        || Interpretation::read(context).forced_to_game
        || TRANSFERS
            .iter()
            .any(|&(nt_level, from, _)| partner_transferred_now(context, from, nt_level))
}

/// Build the instinct ladder: a sane natural action for any auction
///
/// Forced (partner's live takeout double — see the [module docs][self]):
/// penalty pass on a trump stack, a major-suit game jump or 3NT with values,
/// the longest unbid suit at the cheapest level (majors and five-card suits
/// preferred), and a cheapest-notrump escape as the guaranteed action.
///
/// Otherwise: raise partner's suit with three-card support and rising
/// strength per level, overcall notrump (15–18 balanced with stoppers) or a
/// five-card suit if we have not bid, double their low suit bid for takeout
/// on shape (or any 17+), and pass.
///
/// The unconditioned pass at weight `-5` is the absolute last resort: it
/// keeps the logits finite when every action is illegal, while sitting far
/// enough below every forced action (≥ 3 nats) that sampling drivers never
/// pass a forced auction by accident.
#[must_use]
pub fn instinct() -> Rules {
    let mut rules = Rules::new()
        // Forced: a trump stack sits for partner's takeout double.
        .rule(Call::Pass, 1.5, forced_advance() & doubled_suit_stack())
        // Forced: 3NT to play with game values and their suits stopped.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            forced_advance()
                & hcp(13..)
                & stopper_in_their_suits()
                & level_available(3, Strain::Notrump),
        )
        // Unforced default; the -5 entry below is the absolute last resort.
        .rule(Call::Pass, 0.0, !forced_advance())
        .rule(Call::Pass, -5.0, hcp(0..));

    // Forced: jump to a major-suit game with four-plus cards and values —
    // in an unbid major, never in the suit partner asked us to take out of.
    for major in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(major);
        rules = rules.rule(
            Bid::new(4, strain),
            1.45,
            forced_advance()
                & len(major, 4..)
                & points(11..)
                & level_available(4, strain)
                & !they_bid(strain),
        );
    }

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            0.05
        } else {
            0.0
        };

        // Forced: a new suit at the cheapest level; longer suits and majors
        // are preferred.  Bidding their suit would be a cue-bid — excluded.
        for level in 1u8..=4 {
            rules = rules
                .rule(
                    Bid::new(level, strain),
                    1.0 + major_bonus,
                    forced_advance()
                        & min_level_is(level, strain)
                        & len(suit, 4..)
                        & !they_bid(strain),
                )
                .rule(
                    Bid::new(level, strain),
                    1.1 + major_bonus,
                    forced_advance()
                        & min_level_is(level, strain)
                        & len(suit, 5..)
                        & !they_bid(strain),
                );
        }

        // Raise partner's suit with three-card support; each level up asks
        // for more strength, so competitive raises terminate by themselves.
        for (level, threshold) in [(2u8, 6u8), (3, 10), (4, 13)] {
            rules = rules.rule(
                Bid::new(level, strain),
                1.2,
                partner_suit_is(suit)
                    & min_level_is(level, strain)
                    & support(3..)
                    & points(threshold..),
            );
        }

        // Overcall a five-card suit if we have not bid; the strength floor
        // rises with the level and stronger hands double first.
        for (level, floor) in [(1u8, 8u8), (2, 10), (3, 13)] {
            rules = rules.rule(
                Bid::new(level, strain),
                1.0 + major_bonus,
                we_have_not_bid()
                    & min_level_is(level, strain)
                    & len(suit, 5..)
                    & points(floor..=16)
                    & !they_bid(strain),
            );
        }
    }

    for level in 1u8..=4 {
        // Forced: the notrump escape guarantees an action — no fit, no
        // stopper, no four-card suit outside theirs still has a call.
        rules = rules.rule(
            Bid::new(level, Strain::Notrump),
            0.3,
            forced_advance() & min_level_is(level, Strain::Notrump),
        );
    }

    for level in 1u8..=3 {
        // Notrump overcall: 15–18 balanced with their suits stopped.
        rules = rules.rule(
            Bid::new(level, Strain::Notrump),
            1.05,
            we_have_not_bid()
                & min_level_is(level, Strain::Notrump)
                & balanced()
                & hcp(15..=18)
                & stopper_in_their_suits(),
        );
    }

    // Opposite our own strong notrump: complete partner's transfer.  Standard
    // Jacoby (2♦/2♥, 3♦/3♥ over 2NT) and South African Texas (4♣/4♦); the book
    // authors these where it can, so this only catches off-book and competitive
    // continuations.  Bid the suit just above partner's artificial call.
    for (nt_level, from, to) in TRANSFERS {
        rules = rules.rule(
            to,
            1.5,
            partner_transferred(from, nt_level) & level_available(to.level.get(), to.strain),
        );
    }

    // Game values.  Three strands force game regardless of the point estimate:
    // the hand-conditioned strong-notrump responder forces (10+ opposite a 15–17
    // 1NT, 5+ opposite a 20–21 2NT), and the hand-independent forces from the
    // auction interpretation — a strong 2♣ past the double negative, or an opener
    // forced past invitation.  A fourth strand is *general*: our own count plus
    // the sound floor of partner's shown points reaching 25 (the inference makes
    // it sound, never an overbid).  Below game we take the cheapest milestone — a
    // known major fit, else 3NT, dropping to the minor game only when their suit
    // is unstopped — but step aside when penalizing the opponents.
    let game_values = ((partner_strong_notrump(1) & hcp(10..))
        | (partner_strong_notrump(2) & hcp(5..))
        | auction_forces_game()
        | combined_points(25))
        & not_penalizing();
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.40,
        game_values.clone() & below_game() & level_available(3, Strain::Notrump),
    );
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let strain = Strain::from(minor);
        // 3NT is the milestone of choice; reach for the minor game only when
        // notrump is unsafe (a suit they bid is unstopped) and we hold a known
        // eight-card fit.  Uncontested, their suits are vacuously stopped, so
        // this never fires and 3NT plays.
        let known_minor_fit = (len(minor, 5..) & partner_shown_len(minor, 3..))
            | (len(minor, 3..) & partner_shown_len(minor, 5..));
        rules = rules.rule(
            Bid::new(5, strain),
            1.42,
            game_values.clone()
                & below_game()
                & inference_aware()
                & known_minor_fit
                & !stopper_in_their_suits()
                & level_available(5, strain),
        );
    }
    for major in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(major);
        // A *known* eight-card major fit outranks 3NT: our five-card suit meets
        // partner's shown three-card support, or our three meet partner's shown
        // five.  The shown lengths come from the auction interpretation
        // ([`Inferences`]), so this fires only on a fit the calls have promised.
        //
        // [`Inferences`]: super::inference::Inferences
        let known_major_fit = (len(major, 5..) & partner_shown_len(major, 3..))
            | (len(major, 3..) & partner_shown_len(major, 5..));
        rules = rules.rule(
            Bid::new(4, strain),
            1.45,
            game_values.clone() & below_game() & len(major, 6..) & level_available(4, strain),
        );
        rules = rules.rule(
            Bid::new(4, strain),
            1.50,
            game_values.clone()
                & below_game()
                & inference_aware()
                & known_major_fit.clone()
                & level_available(4, strain),
        );
        // Slam is a milestone too: with a known major fit and the combined
        // minimum in the small- (33) or grand- (37) slam zone, bid it.
        rules = rules.rule(
            Bid::new(6, strain),
            1.65,
            combined_points(33)
                & not_penalizing()
                & below_slam()
                & inference_aware()
                & known_major_fit.clone()
                & level_available(6, strain),
        );
        rules = rules.rule(
            Bid::new(7, strain),
            1.75,
            combined_points(37)
                & not_penalizing()
                & below_slam()
                & inference_aware()
                & known_major_fit
                & level_available(7, strain),
        );
    }
    // Notrump slam when no major fit is known: small at 33, grand at 37, with
    // their suits stopped (vacuous when uncontested).
    rules = rules
        .rule(
            Bid::new(6, Strain::Notrump),
            1.60,
            combined_points(33)
                & not_penalizing()
                & below_slam()
                & stopper_in_their_suits()
                & level_available(6, Strain::Notrump),
        )
        .rule(
            Bid::new(7, Strain::Notrump),
            1.70,
            combined_points(37)
                & not_penalizing()
                & below_slam()
                & stopper_in_their_suits()
                & level_available(7, Strain::Notrump),
        );

    // Takeout double of their low suit bid: shape with opening values, or
    // any strong hand planning to bid again.
    rules
        .rule(
            Call::Double,
            0.9,
            their_live_bid_at_most(3) & short_in_their_suits() & hcp(12..),
        )
        .rule(Call::Double, 0.8, their_live_bid_at_most(3) & points(17..))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    const fn call(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The highest-logit instinct call for a hand in an auction
    fn best(auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let logits = instinct().classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    #[test]
    fn forced_advance_never_passes() {
        // Partner doubled their 3♣ for takeout; a worthless hand still bids
        // its five-card suit instead of converting to penalties.
        let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
        assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
        // A yarborough whose only length is in their suit escapes to the
        // cheapest notrump.
        assert_eq!(best(&auction, "964.J85.974.9632"), call(3, Strain::Notrump));
    }

    #[test]
    fn trump_stack_converts_to_penalties() {
        // KQ92 behind the 2♠ bidder sits for partner's takeout double.
        let auction = [call(2, Strain::Spades), Call::Double, Call::Pass];
        assert_eq!(best(&auction, "KQ92.A532.J42.96"), Call::Pass);
    }

    #[test]
    fn forced_advance_bids_game_with_values() {
        let auction = [call(2, Strain::Spades), Call::Double, Call::Pass];
        // 13 HCP with their suit stopped: 3NT to play.
        assert_eq!(best(&auction, "KJ92.A53.KQ42.96"), call(3, Strain::Notrump));
        // 11 HCP with four hearts: jump to the major-suit game.
        assert_eq!(best(&auction, "92.AQ53.KQ42.962"), call(4, Strain::Hearts));
    }

    #[test]
    fn unforced_raise_with_fit() {
        // Partner opened 1♠ and they overcalled 2♥: raise with three-card
        // support and 8 HCP.
        let auction = [call(1, Strain::Spades), call(2, Strain::Hearts)];
        assert_eq!(best(&auction, "Q32.953.A964.Q92"), call(2, Strain::Spades));
    }

    #[test]
    fn unforced_takeout_double_on_shape() {
        // Their 3♦ preempt: 13 HCP, short in diamonds, no five-card suit.
        let auction = [call(3, Strain::Diamonds)];
        assert_eq!(best(&auction, "KQ32.AJ53.2.A942"), Call::Double);
    }

    #[test]
    fn unforced_pass_without_values() {
        // Nothing to say over their 3♦: too weak to act at the three level.
        let auction = [call(3, Strain::Diamonds)];
        assert_eq!(best(&auction, "Q5432.J53.942.92"), Call::Pass);
    }

    #[test]
    fn doubles_only_their_live_bids() {
        // The call to beat is our own 2♠ (partner raised our overcall):
        // doubling our side is never on the table.
        let auction = [
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        let hand: Hand = "92.K53.AQJ42.962".parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let logits = instinct().classify(hand, &context);
        assert_eq!(*logits.0.get(Call::Double), f32::NEG_INFINITY);
    }

    #[test]
    fn completes_partners_transfer_over_notrump() {
        // We opened 1NT and partner transferred 2♦ (hearts): complete with 2♥,
        // even off-book, rather than passing or raising the artificial diamonds.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "AQ32.KJ5.KQ4.Q92"), call(2, Strain::Hearts));
    }

    #[test]
    fn forced_to_game_opposite_strong_notrump() {
        // Partner opened 1NT; after an artificial 2NT super-accept of our heart
        // transfer a game-forced 12-count bids 3NT, never passing below game.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "KQ52.AQ984.J6.32"), call(3, Strain::Notrump));
    }

    #[test]
    fn forced_to_game_picks_the_known_major_fit() {
        // We opened 1NT; partner's off-book, forcing 3♥ shows five-plus hearts.
        // With three-card support that is a known eight-card fit, so bid 4♥
        // rather than the stopperless-agnostic 3NT.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "AQ52.K53.KQ4.32"), call(4, Strain::Hearts));
    }

    #[test]
    fn keeps_passing_with_a_weak_responder() {
        // Partner opened 1NT but we are too weak to force game: still pass when
        // off-book (the forced-to-game floor must not fire on invitational-or-less).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "8632.J9842.96.42"), Call::Pass);
    }

    #[test]
    fn forced_to_game_after_strong_two_clubs() {
        // 2♣ (strong) – 2♦ (game-forcing waiting) – 2NT (22–24 balanced): the
        // auction is game forcing, so a flat 7-count bids 3NT, never passing.
        // 2♣–2♥ is the double negative, so 2♦ commits the partnership to game.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "QJ52.K43.T62.J32"), call(3, Strain::Notrump));
    }

    #[test]
    fn forced_two_clubs_bids_major_game() {
        // The same forcing 2♣–2♦–2NT auction, but holding six hearts: jump to
        // the major-suit game in preference to 3NT.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "3.QJ9854.K32.J32"), call(4, Strain::Hearts));
    }

    #[test]
    fn double_negative_two_clubs_may_pass() {
        // 2♣ – 2♥ is the double negative (0–3 HCP); after opener's 2NT the
        // partnership may still stop, so a yarborough passes off-book — the
        // forcing-2♣ floor must not fire once responder has shown the bust.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "8632.J9842.96.42"), Call::Pass);
    }

    #[test]
    fn forced_game_steps_aside_when_penalizing() {
        // 2♣ – 2♦ (game forcing) – 2NT, then they sacrifice in 3♦ and partner
        // doubles for penalty.  Passing the double out is the game-forcing
        // action, so the floor must not pull it to a stopperless 3NT; with six
        // clubs and no diamond guard, show the suit instead.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            call(3, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        assert_eq!(best(&auction, "K3.KQ4.65.QJ8765"), call(4, Strain::Clubs));
    }

    #[test]
    fn milestone_game_opposite_a_limited_rebid() {
        // 1♦–1♥–1NT: opposite the 12–16 rebid a balanced 16 has 28+ combined,
        // a cold 3NT the constructive book never reached (the board that started
        // this).  The floor reads the rebid's strength and bids the game.
        let auction = [
            call(1, Strain::Diamonds),
            Call::Pass,
            call(1, Strain::Hearts),
            Call::Pass,
            call(1, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "J9.AKJ7.K94.A852"), call(3, Strain::Notrump));
        // A 10-count is only invitational (22–24 combined): the floor uses the
        // *guaranteed* minimum, so it stays sound and passes rather than overbid.
        assert_eq!(best(&auction, "KJ9.QJ73.K94.852"), Call::Pass);
    }

    #[test]
    fn milestone_slam_opposite_a_strong_rebid() {
        // 1♦–1♥–2NT is the 18–19 jump rebid; a balanced 16 lifts the combined
        // minimum to 34, the small-slam zone, so bid 6NT instead of stranding in
        // game.  No known major fit, so notrump is the strain.
        let auction = [
            call(1, Strain::Diamonds),
            Call::Pass,
            call(1, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "KQ.AKJ7.K94.8542"), call(6, Strain::Notrump));
    }
}
