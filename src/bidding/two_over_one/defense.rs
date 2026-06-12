//! Defensive actions for the 2/1 system: overcalls, advances, and doubles
//!
//! This module covers everything our side does when the opponents open the
//! auction: simple overcalls, the 1NT overcall, takeout doubles, the
//! Michaels cue-bid, the Unusual 2NT, advances of all of these, advancing
//! partner's takeout double, responsive doubles when partner has made a
//! takeout double and they raise, and defense to a weak-two opening (takeout
//! double, a natural 2NT overcall, and natural suit overcalls).

use super::super::constraint::{
    balanced, hcp, len, min_level_is, pred, short_in_their_suits, stopper_in_their_suits, support,
    top_honors,
};
use super::super::context::Context;
use super::super::{Defensive, Rules};
use super::{call, insert_all_seats};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};

// ---------------------------------------------------------------------------
// Direct overcalls and doubles
// ---------------------------------------------------------------------------

/// Our action over their one-of-a-suit opening
///
/// One decision: a natural overcall (five-card suit), a takeout double, a
/// 15–18 1NT overcall, or pass.  Strong hands (17+) double first regardless
/// of shape, planning to bid again — otherwise an opening-strength hand with
/// length in the opponents' suit would be stuck.
///
/// Two-suited overcalls are also available:
/// - **Michaels cue-bid** (2 of their suit, 8+ HCP, 5-5): over a minor,
///   both majors; over a major, the other major and an unspecified minor.
/// - **Unusual 2NT** (8+ HCP, 5-5 in the two lowest unbid suits): over 1♣
///   shows diamonds and hearts; over 1♦ shows clubs and hearts; over a major
///   shows both minors.
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
        .rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits())
        .rule(Call::Double, 1.2, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls: five-card suit, 8–16 HCP.
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

    // Michaels cue-bid: 2 of their suit, 5-5, 8+ HCP.
    rules = match t {
        // t minor → both majors
        Suit::Clubs | Suit::Diamonds => rules.rule(
            Bid::new(2, theirs),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & hcp(8..),
        ),
        // t = ♥ → spades + a minor
        Suit::Hearts => rules.rule(
            Bid::new(2, theirs),
            2.0,
            len(Suit::Spades, 5..) & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)) & hcp(8..),
        ),
        // t = ♠ → hearts + a minor
        Suit::Spades => rules.rule(
            Bid::new(2, theirs),
            2.0,
            len(Suit::Hearts, 5..) & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)) & hcp(8..),
        ),
    };

    // Unusual 2NT: 5-5 in the two lowest unbid suits, 8+ HCP.
    match t {
        Suit::Clubs => rules.rule(
            Bid::new(2, Strain::Notrump),
            1.9,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, 5..) & hcp(8..),
        ),
        Suit::Diamonds => rules.rule(
            Bid::new(2, Strain::Notrump),
            1.9,
            len(Suit::Clubs, 5..) & len(Suit::Hearts, 5..) & hcp(8..),
        ),
        Suit::Hearts | Suit::Spades => rules.rule(
            Bid::new(2, Strain::Notrump),
            1.9,
            len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..) & hcp(8..),
        ),
    }
}

/// Our action over their weak-two opening
///
/// A weak two steals a level of room, so the toolkit is leaner than over a
/// one-bid: a takeout double (the workhorse), a natural 2NT overcall (15–18
/// with a stopper), and natural suit overcalls at the cheapest legal level.
/// Strong hands (17+) still double first, planning to bid again.
///
/// Overcall levels are derived from `their_opening`, so the suits higher than
/// theirs sit at the opening level and the lower ones one rung up — over 2♥, a
/// spade overcall is 2♠ but a club overcall is 3♣.
#[must_use]
pub fn defense_to_weak_two(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.5,
            hcp(15..=18) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits())
        .rule(Call::Double, 1.2, hcp(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls: five-card suit, 10–16 HCP, at the cheapest legal level.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let overcall_level = if strain > theirs { level } else { level + 1 };
            rules = rules.rule(
                Bid::new(overcall_level, strain),
                1.0,
                len(suit, 5..) & hcp(10..=16),
            );
        }
    }
    rules
}

/// Our action over their 1NT opening: penalty double or natural two-level overcall
pub fn defense_to_notrump() -> Rules {
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

// ---------------------------------------------------------------------------
// Advances
// ---------------------------------------------------------------------------

/// Advancer's raise of partner's natural overcall in `our_suit`
pub fn advances(our_suit: Suit) -> Rules {
    let s = Strain::from(our_suit);
    Rules::new()
        .rule(Bid::new(4, s), 1.6, support(5..) & hcp(..6))
        .rule(Bid::new(3, s), 1.4, support(3..) & hcp(11..=12))
        .rule(Bid::new(2, s), 1.0, support(3..) & hcp(6..=10))
        .rule(Call::Pass, 0.0, hcp(..6))
}

/// Advancer's action after partner's takeout double, RHO passing: `(opening) X (P)`
///
/// Partner doubled for takeout and asked us to pick.  In priority order:
///
/// - **pass for penalty** with a trump stack (four-plus of their suit, two top
///   honors) — converting the takeout double into penalties;
/// - **jump to a major-suit game** with four-plus cards and opening values;
/// - **bid 3NT** with a stopper in their suit and game-going values;
/// - **bid a new suit** at the cheapest legal level with four-plus cards;
/// - **escape to the cheapest notrump** as a weak catch-all — no fit, no
///   stopper, nothing better to say (lebensohl in spirit);
/// - **pass** as the final fallback.
///
/// Suit and notrump levels are derived from `their_opening`, so the one builder
/// answers over a one-bid (advances at the one and two levels) and over a weak
/// two (advances at the two and three levels) alike.
#[must_use]
pub fn advance_double(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        // Convert for penalty: a trump stack sits for the double.
        .rule(Call::Pass, 1.5, len(t, 4..) & top_honors(t, 2..) & hcp(6..))
        // 3NT to play: a stopper in their suit and game values.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            hcp(13..) & stopper_in_their_suits(),
        )
        // Weak escape to the cheapest notrump: no fit, no stopper, no stack.
        .rule(Bid::new(level, Strain::Notrump), 0.3, hcp(0..))
        // Final fallback.
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        // Natural advance at the cheapest legal level.
        rules = rules.rule(Bid::new(bid_level, strain), 1.0, len(suit, 4..));
        // Major-suit game jump with support and opening values.
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            rules = rules.rule(Bid::new(4, strain), 1.4, len(suit, 4..) & hcp(11..));
        }
    }
    rules
}

/// Advancer's response to partner's Michaels cue-bid over their opening `t`
fn michaels_advances(t: Suit) -> Rules {
    match t {
        // Partner shows both majors: prefer the longer one.
        Suit::Clubs | Suit::Diamonds => {
            let hearts_longer = pred(|hand: Hand, _: &Context<'_>| {
                hand[Suit::Hearts].len() >= hand[Suit::Spades].len()
            });
            let spades_longer = pred(|hand: Hand, _: &Context<'_>| {
                hand[Suit::Spades].len() > hand[Suit::Hearts].len()
            });
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Hearts),
                    1.3,
                    hcp(10..) & len(Suit::Hearts, 3..) & hearts_longer.clone(),
                )
                .rule(
                    Bid::new(4, Strain::Spades),
                    1.3,
                    hcp(10..) & len(Suit::Spades, 3..) & spades_longer.clone(),
                )
                .rule(Bid::new(2, Strain::Hearts), 1.0, hearts_longer)
                .rule(Bid::new(2, Strain::Spades), 1.0, spades_longer)
        }
        // Partner shows spades + a minor: bid spades.
        Suit::Hearts => Rules::new()
            .rule(
                Bid::new(4, Strain::Spades),
                1.3,
                hcp(10..) & len(Suit::Spades, 3..),
            )
            .rule(Bid::new(2, Strain::Spades), 0.5, hcp(0..)),
        // Partner shows hearts + a minor: bid hearts.
        Suit::Spades => Rules::new()
            .rule(
                Bid::new(4, Strain::Hearts),
                1.3,
                hcp(10..) & len(Suit::Hearts, 3..),
            )
            .rule(Bid::new(3, Strain::Hearts), 0.5, hcp(0..)),
    }
}

/// The two suits shown by an Unusual 2NT over their opening `t`
///
/// Returns `(a, b)` where `a < b` (lower suit first).
const fn unusual_suits(t: Suit) -> (Suit, Suit) {
    match t {
        Suit::Clubs => (Suit::Diamonds, Suit::Hearts),
        Suit::Diamonds => (Suit::Clubs, Suit::Hearts),
        Suit::Hearts | Suit::Spades => (Suit::Clubs, Suit::Diamonds),
    }
}

/// Advancer's response to partner's Unusual 2NT over their opening `t`
fn unusual_nt_advances(t: Suit) -> Rules {
    let (a, b) = unusual_suits(t);
    let a_longer = pred(move |hand: Hand, _: &Context<'_>| hand[a].len() >= hand[b].len());
    let b_longer = pred(move |hand: Hand, _: &Context<'_>| hand[b].len() > hand[a].len());
    Rules::new()
        .rule(Bid::new(3, Strain::from(a)), 1.0, a_longer)
        .rule(Bid::new(3, Strain::from(b)), 1.0, b_longer)
}

// ---------------------------------------------------------------------------
// Responsive doubles
// ---------------------------------------------------------------------------

/// Advancer's action when partner made a takeout double and they raised `t` to `raise_lvl`
///
/// Responsive double: both suits of the rank opposite the opened suit (minor/major).
/// Natural bids at the minimum legal level (2–3) for suits other than `t`, 5-card, 8+ HCP.
fn responsive_doubles(t: Suit, _raise_lvl: u8) -> Rules {
    // Responsive double shows the two unbid suits of the same rank (minor or major).
    let mut rules = if matches!(t, Suit::Hearts | Suit::Spades) {
        // t major → both minors
        Rules::new().rule(
            Call::Double,
            1.5,
            len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..) & hcp(8..),
        )
    } else {
        // t minor → both majors
        Rules::new().rule(
            Call::Double,
            1.5,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
        )
    };

    rules = rules.rule(Call::Pass, 0.0, hcp(0..));

    // Natural bids for suits ≠ t at levels 2 and 3.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t {
            continue;
        }
        let strain = Strain::from(suit);
        for bid_lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(bid_lvl, strain),
                1.0,
                min_level_is(bid_lvl, strain) & len(suit, 5..) & hcp(8..),
            );
        }
    }
    rules
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the defensive book: all our actions when the opponents open
///
/// Seat-fanned with `insert_all_seats(…, 3, …)` so every seat is covered.
/// Keys for a defensive auction are the raw table auction starting from their
/// opening, e.g. `[1♦, 2♦, Pass]` means they opened 1♦, we cue-bid 2♦
/// (Michaels), opener's side passed, and we are the advancer.
#[must_use]
pub fn defensive() -> Defensive {
    let mut d = Defensive::new();

    // Over each one-of-a-suit opening: overcalls, double, 1NT, Michaels, Unusual.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(1, theirs);
        insert_all_seats(&mut d, &[Call::Bid(opening)], 3, defense_to_suit(opening));

        // Advancing partner's takeout double: [1t, X, P] — advancer to act.
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), Call::Double, Call::Pass],
            3,
            advance_double(opening),
        );

        // Advances of natural overcalls.
        for our in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let strain = Strain::from(our);
            if strain != theirs {
                let level = if strain > theirs { 1 } else { 2 };
                let overcall = call(level, strain);
                insert_all_seats(
                    &mut d,
                    &[Call::Bid(opening), overcall, Call::Pass],
                    3,
                    advances(our),
                );
            }
        }

        // Advances of Michaels: [1t, 2t, Pass] — advancer to act.
        let michaels_bid = call(2, theirs);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), michaels_bid, Call::Pass],
            3,
            michaels_advances(suit),
        );

        // Advances of Unusual 2NT: [1t, 2NT, Pass] — advancer to act.
        let unusual_bid = call(2, Strain::Notrump);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), unusual_bid, Call::Pass],
            3,
            unusual_nt_advances(suit),
        );

        // Responsive doubles: partner doubled for takeout, they raised to lvl.
        for raise_lvl in [2u8, 3] {
            let raise = call(raise_lvl, theirs);
            insert_all_seats(
                &mut d,
                &[Call::Bid(opening), Call::Double, raise],
                3,
                responsive_doubles(suit, raise_lvl),
            );
        }
    }

    // Over each weak-two opening: takeout double, natural overcalls, 2NT, and
    // advancing partner's takeout double.  Clubs is omitted — a 2♣ opening is
    // the strong artificial bid, not a weak two.
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(2, theirs);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening)],
            3,
            defense_to_weak_two(opening),
        );

        // Advancing partner's takeout double: [2t, X, P] — advancer to act.
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), Call::Double, Call::Pass],
            3,
            advance_double(opening),
        );
    }

    insert_all_seats(&mut d, &[call(1, Strain::Notrump)], 3, defense_to_notrump());
    d
}
