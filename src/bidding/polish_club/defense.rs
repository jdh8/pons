//! Defensive actions for Strawberry Polish Club: our bids when they open
//!
//! Authored from the system's notes (the `Defense/` chapters of
//! <https://polish.club>).  Polish Club's defense is its own animal, not the
//! 2/1 module's Michaels-and-points scheme:
//!
//! - **Strength is New Losing Trick Count.** Suit overcalls over a one-bid are
//!   gauged by [`nltc`] bands (e.g. a one-level overcall is `8.5–6.0` losers),
//!   not by an HCP/point range.  The balancing seat, where the auction is
//!   already half over, falls back to plain HCP.
//! - **Bailey cue-bids, not Michaels.** The cue of their suit shows the highest
//!   unbid suit plus another unbid suit (the two-suiters the Unusual 2NT — the
//!   two *lowest* unbid suits — does not cover).
//! - **Landy over their 1NT**, **natural-with-NLTC over their weak two**, and a
//!   takeout-flavored structure over **Multi 2♦**.
//!
//! Scope (first pass): the direct and balancing seats over a one-of-a-suit
//! opening, plus the direct seat over their 1NT, weak two, and Multi 2♦, and the
//! principal advances (raising an overcall, advancing a takeout double, the
//! Unusual 2NT and responsive doubles).  The deep transfer/relay advance tails
//! (Rubens, Rumpelsohl, the Bailey-cue and Multi transfer continuations) stay on
//! the [`instinct`][super::super::instinct()] floor, as the constructive
//! backbone leaves its relay tails.

use super::super::constraint::{
    balanced, described, hcp, len, min_level_is, nltc, points, short_in_their_suits,
    stopper_in_their_suits, top_honors,
};
use super::super::context::Context;
use super::super::{Defensive, Rules};
use super::{call, insert_all_seats};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

// ---------------------------------------------------------------------------
// Suit-variable helpers (the X < Y < Z notation of the notes)
// ---------------------------------------------------------------------------

/// The two lowest unbid suits over their opening `t` — the Unusual 2NT pair
///
/// Returns `(a, b)` with `a < b`.  Mirrors the 2/1 module's mapping.
const fn unusual_suits(t: Suit) -> (Suit, Suit) {
    match t {
        Suit::Clubs => (Suit::Diamonds, Suit::Hearts),
        Suit::Diamonds => (Suit::Clubs, Suit::Hearts),
        Suit::Hearts | Suit::Spades => (Suit::Clubs, Suit::Diamonds),
    }
}

/// The Bailey cue's suits over their opening `t`: the highest unbid plus the
/// other two unbid suits (the second suit is one of these)
///
/// Returns `(high, [other, other])`.  The cue shows `high` and at least one of
/// the others — the two-suiters the Unusual 2NT (the two lowest) does not cover.
const fn bailey_suits(t: Suit) -> (Suit, [Suit; 2]) {
    match t {
        Suit::Clubs => (Suit::Spades, [Suit::Diamonds, Suit::Hearts]),
        Suit::Diamonds => (Suit::Spades, [Suit::Clubs, Suit::Hearts]),
        Suit::Hearts => (Suit::Spades, [Suit::Clubs, Suit::Diamonds]),
        Suit::Spades => (Suit::Hearts, [Suit::Clubs, Suit::Diamonds]),
    }
}

// ---------------------------------------------------------------------------
// Direct seat over a one-of-a-suit opening
// ---------------------------------------------------------------------------

/// Our action over their one-of-a-suit opening (1Y), direct seat
///
/// Natural suit overcalls are gauged by NLTC: a one-level overcall (1Z) shows
/// `8.5–6.0` losers with five-plus, a two-level overcall (2X) the slightly
/// stronger `7.5–6.0`, and a jump is preemptive (`9.5–8.0`, six-plus; a
/// three-level jump wants seven).  The takeout double is `≤7.5` losers with
/// shortness in their suit; the 1NT overcall is the usual 15–18 balanced with a
/// stopper.  Two-suiters: the **Bailey cue** of their suit (highest unbid plus
/// another) and the **Unusual 2NT** (the two lowest unbid).
#[must_use]
pub fn defense_to_suit(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");

    let mut rules = Rules::new()
        .rule(
            Bid::new(1, Strain::Notrump),
            1.5,
            hcp(15..=18) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.3, nltc(..=7.5) & short_in_their_suits())
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls and preemptive jumps, gauged by NLTC.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        // Non-jump overcall: one-level for a higher suit, two for a lower.
        let level = if strain > theirs { 1 } else { 2 };
        let band = if level == 1 {
            nltc(6.0..=8.5)
        } else {
            nltc(6.0..=7.5)
        };
        let weight = if level == 1 { 1.4 } else { 1.0 };
        rules = rules.rule(Bid::new(level, strain), weight, band & len(suit, 5..));

        // Preemptive jump one rung higher: a two-level jump wants six, a
        // three-level jump seven, with a touch fewer losers.
        let jump = level + 1;
        let (jump_len, jump_lo) = if jump >= 3 { (7, 8.0) } else { (6, 8.0) };
        let jump_hi = if jump >= 3 { 8.5 } else { 9.5 };
        rules = rules.rule(
            Bid::new(jump, strain),
            0.8,
            nltc(jump_lo..=jump_hi) & len(suit, jump_len..),
        );
    }

    // Bailey cue: the highest unbid plus another unbid suit, 5-5, 8+ points.
    let (high, others) = bailey_suits(t);
    rules = rules.rule(
        Bid::new(2, theirs),
        2.0,
        len(high, 5..) & (len(others[0], 5..) | len(others[1], 5..)) & points(8..),
    );

    // Unusual 2NT: the two lowest unbid suits, 5-5, 8+ points.
    let (a, b) = unusual_suits(t);
    rules.rule(
        Bid::new(2, Strain::Notrump),
        1.9,
        len(a, 5..) & len(b, 5..) & points(8..),
    )
}

/// Our action in the balancing seat over their one-of-a-suit opening: `(1Y) P P`
///
/// The auction is half over and the bidding has died, so the balancer acts on
/// less and the scale is plain HCP, not NLTC: a takeout double on 8+, light
/// one-level (4+) and two-level (5+) overcalls (8–15), a 13–15 1NT with a
/// stopper, a strong 19–21 2NT, and preemptive jumps (11–15, 6+).
#[must_use]
pub fn balance_suit(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;

    let mut rules = Rules::new()
        .rule(Call::Double, 1.3, hcp(8..) & short_in_their_suits())
        .rule(
            Bid::new(1, Strain::Notrump),
            1.5,
            hcp(13..=15) & balanced() & stopper_in_their_suits(),
        )
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            hcp(19..=21) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        if strain > theirs {
            // One-level: a real five-card suit outbids the takeout double; a
            // bare four-card balance sits just below it, so a shapely 4-4-4-1
            // doubles rather than picking a four-card suit.
            rules = rules
                .rule(Bid::new(1, strain), 1.4, hcp(8..=15) & len(suit, 5..))
                .rule(Bid::new(1, strain), 1.0, hcp(8..=15) & len(suit, 4..));
        } else {
            // Two-level wants a real five-card suit.
            rules = rules.rule(Bid::new(2, strain), 1.0, hcp(8..=15) & len(suit, 5..));
        }
        // Preemptive jump: 11–15, six-plus.
        let jump = if strain > theirs { 2 } else { 3 };
        rules = rules.rule(Bid::new(jump, strain), 0.8, hcp(11..=15) & len(suit, 6..));
    }
    rules
}

// ---------------------------------------------------------------------------
// Direct seat over their 1NT, weak two, and Multi 2♦
// ---------------------------------------------------------------------------

/// Our action over their 1NT opening: Landy, naturals, and the Unusual 2NT
///
/// Landy 2♣ shows both majors (4-4, 10–15); 2♦/2♥/2♠ are natural; 2NT is the
/// Unusual notrump (both minors); a 15+ balanced hand can double for penalty.
#[must_use]
pub fn defense_to_notrump() -> Rules {
    Rules::new()
        .rule(Call::Double, 1.3, hcp(15..) & balanced())
        .rule(
            call(2, Strain::Clubs),
            1.6,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(10..=15),
        )
        .rule(
            call(2, Strain::Diamonds),
            1.0,
            len(Suit::Diamonds, 5..) & points(8..=15),
        )
        .rule(
            call(2, Strain::Hearts),
            1.0,
            len(Suit::Hearts, 5..) & points(8..=15),
        )
        .rule(
            call(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 5..) & points(8..=15),
        )
        .rule(
            call(2, Strain::Notrump),
            1.4,
            len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..) & points(8..),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Our action over their natural weak two (2X)
///
/// A takeout double (12+ with shortness), a natural 2NT (16–18 with a stopper),
/// and natural overcalls: a higher suit at the two level (12–17, 5+), a lower
/// suit at the three level (14–18, 6+).
#[must_use]
pub fn defense_to_weak_two(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.5,
            hcp(16..=18) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits())
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        if strain > theirs {
            // Higher suit at the cheapest (opening) level: 12–17, five-plus.
            rules = rules.rule(Bid::new(level, strain), 1.0, hcp(12..=17) & len(suit, 5..));
        } else {
            // Lower suit one rung up: 14–18, six-plus.
            rules = rules.rule(
                Bid::new(level + 1, strain),
                0.9,
                hcp(14..=18) & len(suit, 6..),
            );
        }
    }
    rules
}

/// Our action over their Multi 2♦ (a weak two in an unknown major)
///
/// A takeout-flavored double (12+ with a major, or 19+ balanced), a natural
/// 16–18 2NT, and natural three-level minors (8–13, 6+).  The two-under
/// transfers (2♥/2♠) and the 3M Michaels of the notes stay on the floor.
#[must_use]
pub fn defense_to_multi() -> Rules {
    Rules::new()
        .rule(
            Call::Double,
            1.3,
            (hcp(12..) & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..)))
                | (hcp(19..) & balanced()),
        )
        .rule(call(2, Strain::Notrump), 1.5, hcp(16..=18) & balanced())
        .rule(
            call(3, Strain::Clubs),
            0.9,
            len(Suit::Clubs, 6..) & hcp(8..=13),
        )
        .rule(
            call(3, Strain::Diamonds),
            0.9,
            len(Suit::Diamonds, 6..) & hcp(8..=13),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Advances
// ---------------------------------------------------------------------------

/// Advancer's action after partner's takeout double, RHO passing: `(1Y) X (P)`
///
/// Pass for penalty on a trump stack, jump to a major game with a fit and
/// values, bid 3NT with a stopper and game values, otherwise name the cheapest
/// four-card suit (escaping to the cheapest notrump when stuck).
#[must_use]
pub fn advance_double(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        .rule(Call::Pass, 1.5, len(t, 4..) & top_honors(t, 2..) & hcp(6..))
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            hcp(13..) & stopper_in_their_suits(),
        )
        .rule(Bid::new(level, Strain::Notrump), 0.3, hcp(0..))
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        rules = rules.rule(Bid::new(bid_level, strain), 1.0, len(suit, 4..));
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            rules = rules.rule(Bid::new(4, strain), 1.4, len(suit, 4..) & points(11..));
        }
    }
    rules
}

/// Advancer's response to partner's Unusual 2NT over their opening `t`
#[must_use]
pub fn unusual_nt_advances(t: Suit) -> Rules {
    let (a, b) = unusual_suits(t);
    let a_longer = described(
        format!("{a} at least as long as {b}"),
        move |hand: Hand, _: &Context<'_>| hand[a].len() >= hand[b].len(),
    );
    let b_longer = described(
        format!("{b} longer than {a}"),
        move |hand: Hand, _: &Context<'_>| hand[b].len() > hand[a].len(),
    );
    Rules::new()
        .rule(Bid::new(3, Strain::from(a)), 1.0, a_longer)
        .rule(Bid::new(3, Strain::from(b)), 1.0, b_longer)
}

/// Advancer's action when partner doubled for takeout and they raised `t`
///
/// Responsive double showing the two unbid suits of the opposite rank; natural
/// bids at the cheapest level for the other suits.
#[must_use]
pub fn responsive_doubles(t: Suit) -> Rules {
    let mut rules = if matches!(t, Suit::Hearts | Suit::Spades) {
        Rules::new().rule(
            Call::Double,
            1.5,
            len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..) & points(8..),
        )
    } else {
        Rules::new().rule(
            Call::Double,
            1.5,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & points(8..),
        )
    };
    rules = rules.rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t {
            continue;
        }
        let strain = Strain::from(suit);
        for bid_lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(bid_lvl, strain),
                1.0,
                min_level_is(bid_lvl, strain) & len(suit, 5..) & points(8..),
            );
        }
    }
    rules
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the Polish Club defensive book: our actions when the opponents open
///
/// Seat-fanned with `insert_all_seats(…, 3, …)`.  Keys are the raw table
/// auction from their opening: `[1♦]` is the direct seat, `[1♦, Pass, Pass]` the
/// balancing seat, `[1♦, 2♣, Pass]` an advance of our overcall, and so on.
#[must_use]
pub fn defensive() -> Defensive {
    let mut d = Defensive::new();

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(1, theirs);
        let open = Call::Bid(opening);

        // Direct seat and balancing seat.
        insert_all_seats(&mut d, &[open], 3, defense_to_suit(opening));
        insert_all_seats(
            &mut d,
            &[open, Call::Pass, Call::Pass],
            3,
            balance_suit(opening),
        );

        // Advancing partner's takeout double: [1t, X, P].
        insert_all_seats(
            &mut d,
            &[open, Call::Double, Call::Pass],
            3,
            advance_double(opening),
        );

        // Advancing partner's natural overcalls ([1t, overcall, Pass]) is left
        // to the instinct floor's Rubens transfers: the floor computes the
        // transfer band for every (opening, overcall) pair programmatically,
        // which the written WJ notes could not express cleanly as a table.

        // Advancing the Unusual 2NT: [1t, 2NT, P].
        insert_all_seats(
            &mut d,
            &[open, call(2, Strain::Notrump), Call::Pass],
            3,
            unusual_nt_advances(suit),
        );

        // Responsive doubles: partner doubled, they raised to 2 or 3.
        for raise_lvl in [2u8, 3] {
            insert_all_seats(
                &mut d,
                &[open, Call::Double, call(raise_lvl, theirs)],
                3,
                responsive_doubles(suit),
            );
        }
    }

    // Over each natural weak two (♦/♥/♠ — 2♣ is the strong artificial bid).
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let opening = Bid::new(2, Strain::from(suit));
        let open = Call::Bid(opening);
        insert_all_seats(&mut d, &[open], 3, defense_to_weak_two(opening));
        insert_all_seats(
            &mut d,
            &[open, Call::Double, Call::Pass],
            3,
            advance_double(opening),
        );
    }

    // Over their 1NT and their Multi 2♦.
    insert_all_seats(&mut d, &[call(1, Strain::Notrump)], 3, defense_to_notrump());
    insert_all_seats(&mut d, &[call(2, Strain::Diamonds)], 3, defense_to_multi());

    d
}
