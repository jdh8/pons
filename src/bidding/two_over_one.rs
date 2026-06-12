//! A basic 2/1 game-forcing bidding system
//!
//! [`two_over_one()`][crate::bidding::two_over_one::two_over_one] assembles a
//! [`Pair`] for the Two-over-One Game Forcing system, the modern North
//! American standard: five-card majors, a strong 15–17 notrump, the strong
//! artificial 2♣, and — the defining feature — a new suit at the two level in
//! response to a one-of-a-major opening is **game forcing**.
//!
//! This is a *basic* slice: it covers the uncontested openings, the first
//! response to every one-level opening, the 1NT response structure (Stayman and
//! Jacoby transfers), one round of opener's rebids, and a small competitive and
//! defensive layer.  It is authored entirely from the existing constraint
//! vocabulary ([`constraint`][crate::bidding::constraint]), the [`Rules`]
//! classifier, and the role-aware books — the strictly uncontested core in a
//! [`Constructive`] book,
//! [`competition()`][crate::bidding::two_over_one::competition] over our
//! openings in a [`Competitive`] book, and our actions over their openings in
//! a [`Defensive`] book; nothing here is system infrastructure.  Several
//! deeper layers (2/1 opener rebids, inverted minors, slam machinery, fuller
//! competition) are deliberately left for later passes — see the crate
//! changelog.
//!
//! # Forcing by omission
//!
//! There is no "forcing" flag.  A bid is forcing when the *next* node for our
//! side carries no [`Pass`][Call::Pass] rule, so passing scores
//! [`f32::NEG_INFINITY`].  Responders keep a pass below their action threshold;
//! opener-rebid nodes after a response omit it entirely.
//!
//! # Weights
//!
//! Within one decision node the highest-weighted *satisfied* call wins (a
//! satisfied crisp constraint contributes `0`, so the logit is its weight).
//! Constraints are kept disjoint where practical; where calls can both apply,
//! the weights order them so the more descriptive bid wins.

use super::constraint::{
    Cons, Constraint, balanced, hcp, len, nth_seat, pred, stopper_in_their_suits, support,
};
use super::context::Context;
use super::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext};
use super::trie::Classifier;
use super::{Competitive, Constructive, Defensive, Family, Pair, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::sync::Arc;

/// A bid as a [`Call`], for trie keys
const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

// ---------------------------------------------------------------------------
// Seat-fan helpers
// ---------------------------------------------------------------------------

/// Insert one classifier at `suffix` under every leading-pass prefix
///
/// For each `n` in `0..=max_passes` the classifier is keyed at `[P; n] ++
/// suffix`, sharing one [`Arc`] across all of them (pointer-cheap, see
/// [`insert_arc`][super::Trie::insert_arc]).  This authors a table once and
/// makes it answer in every seat that could have reached it.
fn insert_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    rules: impl Classifier + 'static,
) {
    let shared: Arc<dyn Classifier> = Arc::new(rules);
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.insert_arc(&key, Arc::clone(&shared));
    }
}

/// Insert an opening table at every seat (`[]`, `[P]`, `[P, P]`, `[P, P, P]`)
fn insert_opening(book: &mut Trie, rules: Rules) {
    insert_all_seats(book, &[], 3, rules);
}

/// Insert a response table under our `opening`, for every seat that opened it
fn insert_response(book: &mut Trie, opening: Call, rules: Rules) {
    insert_all_seats(book, &[opening, Call::Pass], 2, rules);
}

/// Attach a guarded fallback at `suffix` under every leading-pass prefix
fn fallback_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    guard: Arc<dyn super::fallback::Guard>,
    fallback: Fallback,
) {
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.fallback_arc_at(&key, Arc::clone(&guard), fallback.clone());
    }
}

// ---------------------------------------------------------------------------
// Shape predicates not in the core vocabulary
// ---------------------------------------------------------------------------

/// Better-minor selector: open 1♦ rather than 1♣
///
/// Open the longer minor; with equal length open 1♦ on four-or-more (the
/// standard 4-4 → 1♦, 3-3 → 1♣ split).
fn prefers_diamonds() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, _: &Context<'_>| {
        let clubs = hand[Suit::Clubs].len();
        let diamonds = hand[Suit::Diamonds].len();
        diamonds > clubs || (diamonds == clubs && diamonds >= 4)
    })
}

/// Takeout shape: at most three cards in each suit the opponents have bid
fn short_in_their_suits() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        context.their_suits().all(|suit| hand[suit].len() <= 3)
    })
}

// ---------------------------------------------------------------------------
// Openings
// ---------------------------------------------------------------------------

/// The opening table, shared by every seat
///
/// Strong notrumps (15–17 / 20–21), the artificial 2♣ (22+), five-card majors,
/// better-minor one-of-a-minor openings, weak twos, and three-level preempts.
/// A lighter five-card major is allowed in third and fourth seat.
#[must_use]
pub fn openings() -> Rules {
    let mut rules = Rules::new()
        // Strong, artificial 2♣ — top priority.
        .rule(Bid::new(2, Strain::Clubs), 3.0, hcp(22..))
        // Strong notrumps.
        .rule(Bid::new(1, Strain::Notrump), 2.0, hcp(15..=17) & balanced())
        .rule(Bid::new(2, Strain::Notrump), 2.0, hcp(20..=21) & balanced())
        // Five-card majors; 1♠ ranks just above 1♥ so 5-5 opens the higher.
        .rule(
            Bid::new(1, Strain::Spades),
            1.6,
            hcp(12..=21) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            hcp(12..=21) & len(Suit::Hearts, 5..),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            2.6,
            hcp(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            2.5,
            hcp(9..=11) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Better-minor openings (deny a five-card major).
        .rule(
            Bid::new(1, Strain::Diamonds),
            1.0,
            hcp(12..=21) & prefers_diamonds() & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(1, Strain::Clubs),
            1.0,
            hcp(12..=21)
                & len(Suit::Clubs, 3..)
                & !prefers_diamonds()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        );

    // Weak twos (six-card suit, not in fourth seat).
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(suit)),
            1.0,
            len(suit, 6..=6) & hcp(5..=10) & !nth_seat(4),
        );
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            0.9,
            len(suit, 7..) & hcp(..12) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0.0, hcp(..12))
}

// ---------------------------------------------------------------------------
// Responses to a major opening
// ---------------------------------------------------------------------------

/// Responses to our `1♥`/`1♠` opening
///
/// The 2/1 core: a new suit at the two level is game forcing
/// (`hcp(13..)`), the forcing 1NT is the catch-all below it, raises are
/// graded by strength (single / limit / Jacoby 2NT / weak jump to game), and
/// over 1♥ a four-card spade suit takes the one level.
#[must_use]
pub fn major_responses(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        // Jacoby 2NT: game-forcing raise with four-card support.
        .rule(Bid::new(2, Strain::Notrump), 3.0, support(4..) & hcp(13..))
        // Limit raise.
        .rule(Bid::new(3, trump), 2.0, support(3..) & hcp(10..=12))
        // Weak jump to game: lots of trumps, few points.
        .rule(Bid::new(4, trump), 1.6, support(5..) & hcp(..6))
        // Single raise.
        .rule(Bid::new(2, trump), 1.5, support(3..) & hcp(6..=9))
        // Forcing 1NT: the catch-all when nothing more descriptive fits.
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(6..=12))
        .rule(Call::Pass, 0.0, hcp(..6));

    // 1♠ over 1♥: a new suit at the one level, preferred to a single raise.
    if major == Suit::Hearts {
        rules = rules.rule(
            Bid::new(1, Strain::Spades),
            1.7,
            len(Suit::Spades, 4..) & hcp(6..) & !support(4..),
        );
    }

    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.
    let mut weight = 1.1;
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(
                Bid::new(2, Strain::from(suit)),
                weight,
                len(suit, 4..) & hcp(13..) & !support(4..),
            );
            weight -= 0.05;
        }
    }
    rules
}

// ---------------------------------------------------------------------------
// Responses to a minor opening
// ---------------------------------------------------------------------------

/// Responses to our `1♣`/`1♦` opening
///
/// Four-card majors up the line, a 2/1 game force (`1♦–2♣`), the notrump
/// ladder when no major fits, and simple (not inverted) minor raises promising
/// five-card support, since opener's minor may be only three cards.
#[must_use]
pub fn minor_responses(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    let mut rules = Rules::new()
        // Four-card majors up the line (hearts before spades).
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            len(Suit::Hearts, 4..) & hcp(6..),
        )
        .rule(
            Bid::new(1, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..) & hcp(6..) & len(Suit::Hearts, ..4),
        )
        // Notrump ladder without a four-card major (3NT open-ended for game-plus).
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(13..) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(11..=12) & balanced() & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        .rule(
            Bid::new(1, Strain::Notrump),
            0.5,
            hcp(6..=10) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
        // Simple minor raises (five-card support).
        .rule(Bid::new(3, trump), 1.2, support(5..) & hcp(10..))
        .rule(Bid::new(2, trump), 1.1, support(5..) & hcp(6..=9))
        .rule(Call::Pass, 0.0, hcp(..6));

    // 2/1 game force: 1♦–2♣ (clubs are cheaper than diamonds).
    if minor == Suit::Diamonds {
        rules = rules.rule(
            Bid::new(2, Strain::Clubs),
            1.3,
            len(Suit::Clubs, 4..) & hcp(13..) & len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        );
    }
    rules
}

// ---------------------------------------------------------------------------
// Responses to a 1NT opening
// ---------------------------------------------------------------------------

/// Responses to our 1NT opening: Stayman, Jacoby transfers, and notrump raises
#[must_use]
pub fn notrump_responses() -> Rules {
    Rules::new()
        // Jacoby transfers, any strength.
        .rule(Bid::new(2, Strain::Diamonds), 2.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Hearts), 2.0, len(Suit::Spades, 5..))
        // Stayman: a four-card major and at least invitational values.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.5,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4)) & hcp(8..),
        )
        // Natural notrump raises (no five-card major — that would transfer).
        // 3NT is open-ended: a strong balanced hand bids game and leaves slam
        // exploration to a later pass rather than being stranded without a call.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(10..) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            hcp(8..=9) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Call::Pass,
            0.0,
            hcp(..8) & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
}

/// Opener's answer to Stayman: a four-card major, else 2♦
fn stayman_answers() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4),
        )
}

/// Complete a Jacoby transfer by bidding the anchor suit
fn complete_transfer(into: Suit) -> Rules {
    Rules::new().rule(Bid::new(2, Strain::from(into)), 1.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Opener's rebids (one round)
// ---------------------------------------------------------------------------

/// Opener's rebid after `1♥ – 1♠`: raise spades, rebid hearts, or show shape
///
/// Forcing on opener — there is no pass rule.
fn rebid_one_heart_one_spade() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 2.6, support(4..) & hcp(19..))
        .rule(
            Bid::new(3, Strain::Spades),
            2.2,
            support(4..) & hcp(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.8,
            support(4..) & hcp(12..=15),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.4, len(Suit::Hearts, 6..))
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(Bid::new(2, Strain::Clubs), 0.9, len(Suit::Clubs, 4..))
        .rule(Bid::new(2, Strain::Diamonds), 0.9, len(Suit::Diamonds, 4..))
        // Balanced minimum, and the guaranteed-legal fallback.
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14))
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Opener's rebid after `1M – 1NT` (the forcing notrump)
///
/// Forcing on opener.  A five-card-major rebid is the guaranteed-legal
/// fallback when nothing more descriptive fits — a basic simplification.
fn rebid_after_forcing_notrump(major: Suit) -> Rules {
    let trump = Strain::from(major);
    let mut rules = Rules::new()
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(Bid::new(2, trump), 1.0, len(major, 6..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            rules = rules.rule(Bid::new(2, Strain::from(suit)), 0.9, len(suit, 4..));
        }
    }
    // Opener always holds at least five of the major, so this always applies.
    rules.rule(Bid::new(2, trump), 0.3, len(major, 5..))
}

/// Opener's rebid raising responder's new major after a minor opening
///
/// Used at `1m – 1M`.  Forcing on opener; a 1NT rebid is the guaranteed-legal
/// fallback.
fn rebid_raise_major(responder_major: Suit, opener_minor: Suit) -> Rules {
    let m = Strain::from(responder_major);
    Rules::new()
        .rule(Bid::new(4, m), 2.6, support(4..) & hcp(19..))
        .rule(Bid::new(3, m), 2.2, support(4..) & hcp(16..=18))
        .rule(Bid::new(2, m), 1.8, support(4..) & hcp(12..=15))
        .rule(Bid::new(2, Strain::Notrump), 1.2, hcp(18..=19) & balanced())
        .rule(
            Bid::new(2, Strain::from(opener_minor)),
            0.9,
            len(opener_minor, 5..),
        )
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

/// Opener's rebid after `1♣ – 1♦`
fn rebid_one_club_one_diamond() -> Rules {
    Rules::new()
        .rule(Bid::new(1, Strain::Hearts), 1.3, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(1, Strain::Spades),
            1.3,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.5,
            support(4..) & hcp(16..=18),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.2,
            support(4..) & hcp(12..=15),
        )
        .rule(Bid::new(2, Strain::Notrump), 1.1, hcp(18..=19) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.5, hcp(12..=14) & balanced())
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(12..))
}

// ---------------------------------------------------------------------------
// Competition over our opening
// ---------------------------------------------------------------------------

/// Negative double of an overcall of our major opening, showing the other major
fn negative_doubles(opening_major: Suit) -> Rules {
    let other = if opening_major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    Rules::new()
        .rule(Call::Double, 1.0, len(other, 4..) & hcp(8..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Defense (they open)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// The competitive package over our openings: negative doubles and system-on
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`Pair::against`] (as [`two_over_one`] is meant to be used) so it resolves
/// into the uncontested core.
#[must_use]
pub fn competition() -> Competitive {
    let mut book = Competitive::new();

    // Over our major openings: negative doubles and system-on.
    for major in [Suit::Hearts, Suit::Spades] {
        let opening = call(1, Strain::from(major));
        fallback_all_seats(
            &mut book,
            &[opening],
            2,
            Arc::new(OvercallAtMost(Bid::new(2, Strain::Spades))),
            Fallback::classify(negative_doubles(major)),
        );
        fallback_all_seats(
            &mut book,
            &[opening],
            2,
            Arc::new(FirstIs(Call::Double)),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
    }
    book
}

/// Build the basic 2/1 game-forcing system as one side's [`Pair`]
///
/// Bind it against the opponents' [`Family`] for a playable system, and seat
/// two pairs with [`Table::of_pairs`][super::Table::of_pairs] for a full
/// table.
///
/// ```
/// use pons::two_over_one;
/// use pons::bidding::{Family, System};
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = two_over_one().against(Family::NATURAL);
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = stance
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, Call::Bid(Bid::new(1, Strain::Notrump)));
/// ```
#[must_use]
pub fn two_over_one() -> Pair {
    let mut c = Constructive::new();

    // Openings, one table for every seat.
    insert_opening(&mut c, openings());

    // First responses to every one-level opening.
    for major in [Suit::Hearts, Suit::Spades] {
        insert_response(&mut c, call(1, Strain::from(major)), major_responses(major));
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        insert_response(&mut c, call(1, Strain::from(minor)), minor_responses(minor));
    }
    insert_response(&mut c, call(1, Strain::Notrump), notrump_responses());

    // 1NT continuations: Stayman answers and transfer completions.
    let p = Call::Pass;
    insert_all_seats(
        &mut c,
        &[call(1, Strain::Notrump), p, call(2, Strain::Clubs), p],
        2,
        stayman_answers(),
    );
    insert_all_seats(
        &mut c,
        &[call(1, Strain::Notrump), p, call(2, Strain::Diamonds), p],
        2,
        complete_transfer(Suit::Hearts),
    );
    insert_all_seats(
        &mut c,
        &[call(1, Strain::Notrump), p, call(2, Strain::Hearts), p],
        2,
        complete_transfer(Suit::Spades),
    );

    // Opener's rebids (one round): after a one-level new suit and the forcing 1NT.
    insert_all_seats(
        &mut c,
        &[call(1, Strain::Hearts), p, call(1, Strain::Spades), p],
        2,
        rebid_one_heart_one_spade(),
    );
    for major in [Suit::Hearts, Suit::Spades] {
        insert_all_seats(
            &mut c,
            &[call(1, Strain::from(major)), p, call(1, Strain::Notrump), p],
            2,
            rebid_after_forcing_notrump(major),
        );
    }
    insert_all_seats(
        &mut c,
        &[call(1, Strain::Clubs), p, call(1, Strain::Diamonds), p],
        2,
        rebid_one_club_one_diamond(),
    );
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for responder_major in [Suit::Hearts, Suit::Spades] {
            insert_all_seats(
                &mut c,
                &[
                    call(1, Strain::from(minor)),
                    p,
                    call(1, Strain::from(responder_major)),
                    p,
                ],
                2,
                rebid_raise_major(responder_major, minor),
            );
        }
    }

    // Defensive book: our action when they open, plus advances of our overcalls.
    let mut d = Defensive::new();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(1, theirs);
        d.insert(&[Call::Bid(opening)], defense_to_suit(opening));

        for our in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let strain = Strain::from(our);
            if strain != theirs {
                let level = if strain > theirs { 1 } else { 2 };
                let overcall = call(level, strain);
                d.insert(&[Call::Bid(opening), overcall, Call::Pass], advances(our));
            }
        }
    }
    d.insert(&[call(1, Strain::Notrump)], defense_to_notrump());

    Pair::new(Family::NATURAL, c, competition(), d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    /// The highest-logit call a sub-builder makes for a hand in a context
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

    #[test]
    fn openings_pick_the_descriptive_bid() {
        let o = openings();
        // 16 balanced -> 1NT; 22 -> 2♣; five hearts -> 1♥; six spades, weak -> 2♠.
        assert_eq!(best(&o, &[], "AQ32.K53.QJ4.A92"), call(1, Strain::Notrump));
        assert_eq!(best(&o, &[], "AKQ2.AKJ.KQ4.932"), call(2, Strain::Clubs));
        assert_eq!(best(&o, &[], "A2.KQJ53.Q42.J92"), call(1, Strain::Hearts));
        assert_eq!(best(&o, &[], "KQJ732.53.842.92"), call(2, Strain::Spades));
    }

    #[test]
    fn openings_suppress_weak_twos_in_fourth_seat() {
        // The same six-spade 6-count opens 2♠ in first seat but passes in fourth.
        let o = openings();
        assert_eq!(best(&o, &[], "KQJ732.53.842.92"), call(2, Strain::Spades));
        assert_eq!(best(&o, &[Call::Pass; 3], "KQJ732.53.842.92"), Call::Pass,);
    }

    #[test]
    fn major_responses_run_the_2_over_1_ladder() {
        let r = major_responses(Suit::Hearts);
        let a = [call(1, Strain::Hearts), Call::Pass];
        assert_eq!(best(&r, &a, "K2.KQ54.A964.Q92"), call(2, Strain::Notrump));
        assert_eq!(best(&r, &a, "Q32.J53.A964.Q92"), call(2, Strain::Hearts));
        assert_eq!(best(&r, &a, "A2.K3.Q543.KJ85"), call(2, Strain::Clubs));
    }

    #[test]
    fn notrump_responses_transfer_and_stayman() {
        let r = notrump_responses();
        let a = [call(1, Strain::Notrump), Call::Pass];
        assert_eq!(best(&r, &a, "KJ542.Q32.K43.92"), call(2, Strain::Hearts));
        assert_eq!(best(&r, &a, "KJ54.Q32.K43.Q92"), call(2, Strain::Clubs));
    }

    #[test]
    fn defense_doubles_with_strength() {
        let r = defense_to_suit(Bid::new(1, Strain::Diamonds));
        let a = [call(1, Strain::Diamonds)];
        // 18 HCP with length in their suit still doubles (planning to bid again).
        assert_eq!(best(&r, &a, "A.Q6.KJ852.AKJ42"), Call::Double);
        // A light five-card major overcalls.
        assert_eq!(best(&r, &a, "AQJ32.853.42.K92"), call(1, Strain::Spades));
    }
}
