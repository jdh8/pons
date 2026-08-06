//! Advancing partner's takeout double — the base structure
//!
//! Advancer's forced bid, the cue, and the jump.  The richer opt-in structure
//! (transfers, the `2NT` relay, minor jumps) is [`super::advance_rich`]; the
//! weak-two case is [`super::advance_sohl`].

use super::advance_rich::{advance_double_rich, rich_advance_double_enabled};
use super::advance_sohl::{advance_sohl_style, sohl_rows_over};
use super::*;

/// Advancer's action after partner's takeout double, RHO passing: `(opening) X -`
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
///
/// # Panics
///
/// Panics if `their_opening` is a notrump bid; pass a suit opening.
#[must_use]
pub fn advance_double(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();

    // Convert for penalty: a trump stack sits for the double — yielding, under
    // `set_advance_pass_yield_major`, to a weak hand's 4+ unbid major.
    let sit = len(t, 4..) & top_honors(t, 2..) & hcp(6..);
    let mut rules = if advance_pass_yield_major_enabled() {
        Rules::new().rule(Call::Pass, 150, sit & (hcp(10..) | no_unbid_major(t)))
    } else {
        Rules::new().rule(Call::Pass, 150, sit)
    };
    rules = rules
        // 3NT to play: a stopper in their suit and game values.
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            hcp(13..) & stopper_in_their_suits(),
        )
        // Weak escape to the cheapest notrump: no fit, no stopper, no stack.
        .rule(Bid::new(level, Strain::Notrump), 30, hcp(0..))
        // Final fallback.
        .rule(Call::Pass, 0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        // Natural advance at the cheapest legal level (longest-first under the knob).
        rules = natural_advance(rules, t, suit, bid_level, 100, 4);
        // Major-suit game jump with support and opening values.
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            rules = rules.rule(Bid::new(4, strain), 140, len(suit, 4..) & points(11..));
        }
    }
    rules
}

/// Append the natural-suit advance of a takeout double: `suit`, bid at
/// `bid_level`, with weight `base` and minimum length `min_len`.
///
/// Off the [`set_longest_first_advance`] knob this is a single flat rule, so the
/// classifier's argmax tie-break advances the highest-ranking eligible suit.  On
/// it, the rule gains the [`longest_unbid`] condition, so the **longest** unbid
/// suit advances, an equal-length tie going to the higher rank (5♦4♠ → `1♦`,
/// 4-4 majors → `1♠`) — the same choice the retired weight ladder
/// (`base + 0.001·held + 0.0001·rank`) made, said as a constraint instead of a
/// race among rules.
pub(super) fn natural_advance(
    rules: Rules,
    theirs: Suit,
    suit: Suit,
    bid_level: u8,
    base: i16,
    min_len: usize,
) -> Rules {
    let bid = Bid::new(bid_level, Strain::from(suit));
    if longest_first_advance_enabled() {
        rules.rule(
            bid,
            base,
            len(suit, min_len..) & longest_unbid(suit, theirs),
        )
    } else {
        rules.rule(bid, base, len(suit, min_len..))
    }
}

/// `suit` is the cheapest-to-bid 3-card suit of a hand with no 4-card suit
/// outside `theirs` — the forced-advance rung's discipline
///
/// With a 4-card suit somewhere the longest-first rung takes over; stuck below
/// that, the priority flips from highest-ranking to **cheapest bid**, keeping
/// the forced auction as low as possible — `(1♥)` X - with 3=2=3=3 bids
/// `1♠`, but `(1♠)` X - with 2=3=3=3 bids `2♣`.  One exact box: `suit`
/// exactly three cards, every rival whose advance is cheaper capped at two
/// (it would be forced first), every dearer rival capped at three (a fourth
/// card there promotes the hand to the longest-first rung).  Knob-off the
/// reading stays ⊤, leaving the companion `len` floor as the whole legacy
/// reading.
pub(super) fn cheapest_forced(
    suit: Suit,
    theirs: Suit,
    their_level: u8,
) -> Cons<impl Constraint + Clone> {
    let bid_of = |s: Suit| {
        (
            if s > theirs {
                their_level
            } else {
                their_level + 1
            },
            s,
        )
    };
    let mut lengths = [Range::FULL_LENGTH; 4];
    lengths[suit as usize] = Range::new(3, 3);
    for rival in Suit::ASC {
        if rival == suit || rival == theirs {
            continue;
        }
        let cap = if bid_of(rival) < bid_of(suit) { 2 } else { 3 };
        lengths[rival as usize] = Range::new(0, cap);
    }
    shapes(
        format!("{suit} the cheapest 3-card suit"),
        vec![length_box(lengths)],
    )
}

/// No 4-card major outside `theirs` — the weak sit's license to convert
///
/// The [`set_advance_pass_yield_major`] yield: a weak advancer holding a 4+
/// unbid major has a constructive home the penalty conversion would bury, so
/// the sit is reserved for hands with none (or with cue-band strength, where
/// the conversion is a choice, not a default).  One box capping each unbid
/// major at three; knob-off the reading stays ⊤.
pub(super) fn no_unbid_major(theirs: Suit) -> Cons<impl Constraint + Clone> {
    let mut lengths = [Range::FULL_LENGTH; 4];
    for major in [Suit::Hearts, Suit::Spades] {
        if major != theirs {
            lengths[major as usize] = Range::new(0, 3);
        }
    }
    shapes(
        format!("no 4-card major outside {theirs}"),
        vec![length_box(lengths)],
    )
}

/// Advancing partner's takeout double of a weak two, honoring the selected
/// [`set_advance_sohl_style`]
///
/// `Off` keeps the flat [`advance_double`] ladder.  `Plain`/`Transfer` shadow it
/// with the reused Section-5 sohl builders under the `P* (2X) X -` prefix — the
/// `2NT` relay (and, for `Transfer`, the transfers + cue-Stayman) — plus the
/// doubler's continuations (relay completion, the rebid after `3♣`, and the
/// transfer / cue answers).  Over `(2♦)`, `Transfer` additionally plays
/// `3♣`-Stayman + Smolen + Leaping Michaels.  A forcing 3-level suit (`Plain`) or
/// a constructive advance is driven on by the instinct floor, which already
/// handles forced-to-game auctions.
pub(super) fn advance_of_double_package() -> Package {
    Package {
        name: "advance-of-weak-two-double",
        gate: || true,
        entries: || {
            let style = advance_sohl_style();
            [Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(2, Strain::from(suit));
                    let base = format!("P* ({opening}) X -");
                    if style == LebensohlStyle::Off {
                        rows_of(Pattern::node(&base), advance_double(opening))
                    } else {
                        // gate_4333 = false: advancing partner's takeout double —
                        // partner is short in their suit, so the 4-4 fit keeps its
                        // ruffing value (the 4333 curse does not apply here, and
                        // that A/B was never run).
                        sohl_rows_over(&base, suit, style, false)
                    }
                })
                .collect()
        },
    }
}

/// Advancing partner's takeout double of a one-of-a-suit opening
///
/// The rich cue + notrump ladder when [`set_rich_advance_double`] is on, else
/// the flat floor ladder; the continuations of the rich ladder's artificial
/// calls live in [`rich_advance_double_package`].
pub(super) fn advance_double_package() -> Package {
    Package {
        name: "advance-of-double",
        gate: || true,
        entries: || {
            [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(1, Strain::from(suit));
                    let advances = if rich_advance_double_enabled() {
                        advance_double_rich(opening)
                    } else {
                        advance_double(opening)
                    };
                    rows_of(Pattern::node(&format!("P* ({opening}) X -")), advances)
                })
                .collect()
        },
    }
}

#[cfg(test)]
mod tests;
