//! The competitive package over our openings
//!
//! This module builds the [`Competitive`] book that covers contested auctions
//! after our one-level openings: direct-seat responses to their overcall,
//! system-on over their double, support doubles and redoubles for minor
//! openings, and opener's answer to partner's negative double of a two-level
//! minor overcall.

use super::super::constraint::{hcp, len, min_level_is, pred, support};
use super::super::context::Context;
use super::super::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext, guard};
use super::super::{Competitive, Rules};
use super::{call, fallback_all_seats};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Constraint helpers
// ---------------------------------------------------------------------------

/// The opponents have bid `strain`
fn they_bid(
    strain: Strain,
) -> super::super::constraint::Cons<impl super::super::constraint::Constraint + Clone> {
    pred(move |_hand, ctx: &Context<'_>| ctx.they_bid(strain))
}

// ---------------------------------------------------------------------------
// Section 1: direct-seat response to their overcall
// ---------------------------------------------------------------------------

/// Responder's action after our opening `o` and their overcall (≤ 2♠)
///
/// Covers cue-bid limit-plus raises, preemptive and competitive raises of
/// the opening suit, negative doubles, and weak jump shifts.
fn over_their_overcall(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);

    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };
    let jump_min: usize = if is_major { 4 } else { 5 };

    let other_major = match o {
        Suit::Hearts => Suit::Spades,
        Suit::Spades => Suit::Hearts,
        _ => Suit::Hearts, // for minors, used only in negative double
    };

    let mut rules = Rules::new();

    // Cue-bid raises: for each suit t ≠ o, levels 2 and 3
    for t in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if t == o {
            continue;
        }
        let t_strain = Strain::from(t);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, t_strain),
                2.0,
                they_bid(t_strain) & min_level_is(lvl, t_strain) & support(raise_min..) & hcp(10..),
            );
        }
    }

    // Jump raise: preemptive (min_level=2 means we could bid 2o, so 3o is a jump)
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.6,
        min_level_is(2, o_strain) & support(jump_min..) & hcp(..=9),
    );

    // Competitive raise: 3o when it's the minimum legal bid
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.3,
        min_level_is(3, o_strain) & support(raise_min..) & hcp(6..=9),
    );

    // Single raise
    rules = rules.rule(
        Bid::new(2, o_strain),
        1.5,
        min_level_is(2, o_strain) & support(raise_min..) & hcp(6..=9),
    );

    // Negative double
    rules = if is_major {
        // Other major, 4+ cards, 8+ HCP
        rules.rule(Call::Double, 1.0, len(other_major, 4..) & hcp(8..))
    } else {
        // Both majors 4+, 8+ HCP
        rules.rule(
            Call::Double,
            1.0,
            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
        )
    };

    // Weak jump shifts: for each suit x ≠ o, levels 2 and 3
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let x_strain = Strain::from(x);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, x_strain),
                1.1,
                min_level_is(lvl - 1, x_strain) & len(x, 6..) & hcp(2..=5) & !they_bid(x_strain),
            );
        }
    }

    // Pass
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 3: support doubles and redoubles
// ---------------------------------------------------------------------------

/// Opener's support double/redouble rules showing three-card support for major M
///
/// `Call::Double` with exactly 3 (support double); `2M` with 4+ (natural raise);
/// Pass as the catch-all.
fn support_rules(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Call::Double, 1.5, support(3..=3))
        .rule(Bid::new(2, m), 1.4, support(4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 4: opener answers partner's negative double of a two-level minor
// ---------------------------------------------------------------------------

/// Opener's answer after `1M – (2m) – X – P` (partner doubled a minor overcall)
///
/// Shows four-card length in the other major or rebids the opening major on five.
/// No Pass rule — the double is forcing.
fn answer_neg_double_of_minor(opening_major: Suit) -> Rules {
    let m = Strain::from(opening_major);
    let other = if opening_major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let other_strain = Strain::from(other);
    Rules::new()
        .rule(Bid::new(2, other_strain), 1.0, len(other, 3..))
        .rule(Bid::new(2, m), 0.5, len(opening_major, 5..))
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// The competitive package over our openings: cue-bid raises, preemptive raises,
/// negative doubles for all four openings, support doubles/redoubles, and
/// opener's answers to negative doubles of minor overcalls
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`Pair::against`] (as [`two_over_one`][super::two_over_one] is meant to be
/// used) so it resolves into the uncontested core.
#[must_use]
pub fn competition() -> Competitive {
    let mut book = Competitive::new();

    // Section 1 & 2: over all four openings, attach direct-seat response rules
    // and system-on over their double.
    for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let opening_call = call(1, Strain::from(opening));
        fallback_all_seats(
            &mut book,
            &[opening_call],
            3,
            Arc::new(OvercallAtMost(Bid::new(2, Strain::Spades))),
            Fallback::classify(over_their_overcall(opening)),
        );
        fallback_all_seats(
            &mut book,
            &[opening_call],
            3,
            Arc::new(FirstIs(Call::Double)),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
    }

    // Section 3: support doubles and redoubles for each (minor, major) pair.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for major in [Suit::Hearts, Suit::Spades] {
            let suffix = [
                call(1, Strain::from(minor)),
                Call::Pass,
                call(1, Strain::from(major)),
            ];
            let just_below = if major == Suit::Hearts {
                Bid::new(2, Strain::Diamonds)
            } else {
                Bid::new(2, Strain::Hearts)
            };

            // Support double: they overcall at most `just_below`
            fallback_all_seats(
                &mut book,
                &suffix,
                3,
                Arc::new(OvercallAtMost(just_below)),
                Fallback::classify(support_rules(major)),
            );

            // Support redouble: they doubled
            fallback_all_seats(
                &mut book,
                &suffix,
                3,
                Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                    suffix == [Call::Double]
                })),
                Fallback::classify({
                    let m = Strain::from(major);
                    Rules::new()
                        .rule(Call::Redouble, 1.5, support(3..=3))
                        .rule(Bid::new(2, m), 1.4, support(4..))
                        .rule(Call::Pass, 0.0, hcp(0..))
                }),
            );
        }
    }

    // Section 4: opener answers partner's negative double of a two-level minor.
    // Suffix is [1M]; guard checks that suffix is [2m, X, P].
    for major in [Suit::Hearts, Suit::Spades] {
        fallback_all_seats(
            &mut book,
            &[call(1, Strain::from(major))],
            3,
            Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                matches!(
                    suffix,
                    [Call::Bid(b), Call::Double, Call::Pass]
                        if b.level.get() == 2
                            && (b.strain == Strain::Clubs || b.strain == Strain::Diamonds)
                )
            })),
            Fallback::classify(answer_neg_double_of_minor(major)),
        );
    }

    book
}
