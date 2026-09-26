//! Natural strong jump shifts over the forcing `1NT` — the rival to Meckstroth
//!
//! Gated by [`RebidKnobs::forcing_nt_jump_shifts`], and inert while
//! [`RebidKnobs::meckstroth_adjunct`] is on (both designs own the same 18+
//! seam).  Opener's 18+ rebids after `1M - 1NT` become:
//!
//! | Rebid | Meaning |
//! |-------|---------|
//! | `2NT` | 18+ balanced (the natural rebid, uncapped) |
//! | `3x`  | natural jump shift: 4+ cards, 18+, game-forcing |
//! | `3NT!` | 18+, 6+ card major, no side suit (a `6-4` jump-shifts instead) |
//!
//! The `1♠ - 1NT - 3♥` leg of the invitational two-suiter is displaced (it is
//! the 18+ jump shift now): that 5-5 15–17 rebids `2♥` and invites with `3♥`
//! over the spade preference.  The `3M` jump-rebid keeps its 16–17 band by
//! weight, exactly as it did under the Meckstroth `2NT`.

use super::*;
use crate::bidding::american::slam;
use crate::bidding::constraint::{Cons, Constraint};

// ponytail: construction-time toggle like the Meckstroth adjunct — read during
// `register()`, set it before building the `System`.
/// Opener's `3NT` over the forcing `1NT` — 18+, six-plus of the major, no side suit
const LONG_MAJOR_3NT: Alert = Alert("forcing-nt-3nt-long-major");
/// Responder's `4♣` over `1♥ - 1NT - 3♠` — the heart fit slam-try (`3♥` is unavailable)
const FIT_SLAM_TRY_4C: Alert = Alert("forcing-nt-jump-shift-4c-fit");
/// The displaced two-suiter's 5-5 15–17: its `2♥` rebid, the delayed `3♥` invite
/// and the `4♥` over the notrump invite — alerted because each floors opener's
/// first suit (the invariant's definition of artificial), as the two-suiter's own
/// calls are
const FIVE_FIVE_INVITE: Alert = Alert("forcing-nt-five-five-invite");

/// Whether the jump shifts are live: the knob on and Meckstroth off
pub(super) fn forcing_nt_jump_shifts_on(knobs: &RebidKnobs) -> bool {
    knobs.forcing_nt_jump_shifts && !knobs.meckstroth_adjunct
}

/// Whether `rebid` is one of the jump-shift rungs over `1M - 1NT` (`3x` in a
/// new suit, or the `3NT!` long-major call)
pub(super) fn is_forcing_nt_jump_shift(major: Suit, rebid: Call) -> bool {
    match rebid {
        Call::Bid(bid) if bid.level.get() == 3 => {
            bid.strain == Strain::Notrump || bid.strain != Strain::from(major)
        }
        _ => false,
    }
}

/// Append the natural jump shifts and the long-major `3NT!` when live
///
/// Weights: `3x` (1.70 + rank, so a 5-5 bids the higher suit first) above the
/// `3NT!` (1.60) above the `3M` jump-rebid (1.50, so 18+ leaves it at 16–17);
/// the balanced `2NT` (1.20) is disjoint by shape — a 5332 has no four-card
/// side suit.  Over `1♠` the displaced two-suiter's 5-5 15–17 rebids `2♥`
/// (1.05, over the six-card `2♠` at 1.00) and invites with `3♥` after the
/// preference — see [`opener_after_preference`].
pub(super) fn with_forcing_nt_jump_shifts(
    mut rules: Rules,
    major: Suit,
    knobs: &RebidKnobs,
) -> Rules {
    if !forcing_nt_jump_shifts_on(knobs) {
        return rules;
    }
    for (weight, suit) in (170..).zip([Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]) {
        if suit != major {
            rules = rules.rule(
                Bid::new(3, Strain::from(suit)),
                weight,
                len(suit, 4..) & points(18..),
            );
        }
    }
    if major == Suit::Spades {
        rules = rules
            .rule(Bid::new(2, Strain::Hearts), 105, five_five_invite())
            .alert(FIVE_FIVE_INVITE);
    }
    rules
        .rule(
            Bid::new(3, Strain::Notrump),
            160,
            len(major, 6..) & points(18..),
        )
        .alert(LONG_MAJOR_3NT)
}

/// The displaced two-suiter's hand: 5+ spades, 5+ hearts, 15–17
fn five_five_invite() -> Cons<impl Constraint + Clone> {
    len(Suit::Spades, 5..) & len(Suit::Hearts, 5..) & points(15..=17)
}

/// Opener after responder's spade preference (`1♠ - 1NT - 2♥ - 2♠`)
///
/// The 5-5 15–17 invites with `3♥` (responder answers with the two-suiter's
/// own table).  Deliberately **no catch-all**: every other hand rejects the
/// table (all-−∞) and falls through to the floor by the documented escape
/// hatch — a `Pass` here shadowed the floor's own tries (measured −242 IMPs
/// on 47 boards in 200k).
fn opener_after_preference() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, five_five_invite())
        .alert(FIVE_FIVE_INVITE)
}

/// Opener after responder's notrump invite (`1♠ - 1NT - 2♥ - 2NT`)
///
/// The generic accept-or-pass table, plus the 5-5 15–17's `4♥` — the invite
/// found values, and a 5-5 wants the suit game, not `3NT`.
fn opener_after_invite() -> Rules {
    opener_accept_notrump_invite()
        .rule(Bid::new(4, Strain::Hearts), 110, five_five_invite())
        .alert(FIVE_FIVE_INVITE)
}

/// Responder's game-forcing placement — shared by the natural jump shift and the
/// natural 18+ `2NT`
///
/// Opener has 5+ of the major and 18+; `second` is the jump-shift suit (`None`
/// after `2NT`).  Forcing — `3NT` is the finite catch-all, so there is no
/// `Pass`.  The fit slam-try is `3M` (or the alerted `4♣` over `1♥ - 1NT -
/// 3♠`, where `3♥` is below the jump shift), so that **opener** asks
/// keycards: an 18+ opener holds most of them, and the shared RKCB asker
/// table judges the slam by the asker's own count.
///
/// | Call    | Wt   | Meaning |
/// |---------|------|---------|
/// | 3M / 4♣ | 1.45 | Fit + slam interest (3+ support, 10+) → opener asks on 20+ |
/// | 4M      | 1.40 | Fit, no slam interest (3+ support, ≤9) |
/// | 4♥      | 1.35 | 4-4 heart game over a `3♥` jump shift |
/// | 3♥      | 1.30 | Natural 5+ hearts (below opener's spades) |
/// | 3NT     | 1.20 | Catch-all — to play; opener pulls on six |
// ponytail: no minor-fit rung — a 4-4 minor fit plays 3NT; add a `4m` slam
// try if the trace shows stranded minor games or slams.
fn responder_game_force(major: Suit, second: Option<Suit>) -> Rules {
    let m = Strain::from(major);
    let slam_try = len(major, 3..) & points(10..);
    let mut rules = if let Some(four_clubs) = fit_slam_try_is_four_clubs(major, second) {
        Rules::new()
            .rule(four_clubs, 145, slam_try)
            .alert(FIT_SLAM_TRY_4C)
    } else {
        Rules::new().rule(Bid::new(3, m), 145, slam_try)
    };
    rules = rules.rule(Bid::new(4, m), 140, len(major, 3..) & points(..=9));
    if second == Some(Suit::Hearts) {
        rules = rules.rule(Bid::new(4, Strain::Hearts), 135, len(Suit::Hearts, 4..));
    }
    if major == Suit::Spades && second != Some(Suit::Hearts) {
        rules = rules.rule(Bid::new(3, Strain::Hearts), 130, len(Suit::Hearts, 5..));
    }
    rules.rule(Bid::new(3, Strain::Notrump), 120, points(0..))
}

/// `Some(4♣)` when the fit slam-try cannot be `3M` (`1♥ - 1NT - 3♠`)
fn fit_slam_try_is_four_clubs(major: Suit, second: Option<Suit>) -> Option<Bid> {
    (major == Suit::Hearts && second == Some(Suit::Spades)).then(|| Bid::new(4, Strain::Clubs))
}

/// Opener over responder's `3NT` catch-all: pull to `4M` on a six-card suit
fn opener_over_three_notrump(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 100, len(major, 6..))
        .rule(Call::Pass, 0, points(0..))
}

/// Responder over opener's `3NT!` (18+, six of the major, no side suit)
///
/// Non-forcing.  A doubleton is an eight-card fit opposite six, so the major
/// game needs only `len(major, 2..)`; with a maximum responder asks keycards.
///
/// | Call | Wt   | Meaning |
/// |------|------|---------|
/// | 4NT  | 1.30 | RKCB: 2+ support, 11+ |
/// | 4M   | 1.10 | 6-2 (or better) major game |
/// | 4♥   | 1.05 | Six-plus hearts, short in spades (over `1♠` only) |
/// | Pass | 0.00 | Singleton or void in the major — play `3NT` |
fn responder_over_long_major_3nt(major: Suit) -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(4, Strain::Notrump),
            130,
            len(major, 2..) & points(11..),
        )
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 110, len(major, 2..));
    if major == Suit::Spades {
        rules = rules.rule(Bid::new(4, Strain::Hearts), 105, len(Suit::Hearts, 6..));
    }
    rules.rule(Call::Pass, 0, points(0..))
}

/// Opener over responder's `4M` correction of the `3NT!`: keycards on a big hand
fn opener_over_correction() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 100, points(21..))
        .alert(slam::RKCB)
        .rule(Call::Pass, 0, points(0..))
}

/// Responder's game-forcing round below `node` (a jump shift or the natural
/// `2NT`), with opener's answers: RKCB after the fit slam-try, the red-suit
/// placement, the `3NT` pull.
fn game_force_rows(
    node: &str,
    major: Suit,
    second: Option<Suit>,
) -> Vec<crate::bidding::rows::Entry> {
    let mut entries = rows_of(Pattern::node(node), responder_game_force(major, second));
    let slam_try =
        fit_slam_try_is_four_clubs(major, second).unwrap_or(Bid::new(3, Strain::from(major)));
    let fit_node = format!("{node} {slam_try} -");
    entries.extend(rows_of(
        Pattern::node(&fit_node),
        meckstroth::opener_over_fit_slamtry(major),
    ));
    entries.extend(slam::rkcb_rows(&fit_node, major));
    if major == Suit::Spades && second != Some(Suit::Hearts) {
        entries.extend(rows_of(
            Pattern::node(&format!("{node} 3♥ -")),
            meckstroth::opener_over_resp_red(major, Suit::Hearts),
        ));
    }
    entries.extend(rows_of(
        Pattern::node(&format!("{node} 3NT -")),
        opener_over_three_notrump(major),
    ));
    entries
}

/// Both sides below the jump shifts, the natural `2NT` and the long-major `3NT!`
///
/// The `1M - 1NT - 2NT -` node **overrides** the crude shared natural-2NT tail
/// (`notrump::two_notrump_rebids`, 3NT-or-pass) — `rebids::register` runs
/// after `notrump::register`, so the on-knob insert wins.
pub(crate) fn forcing_nt_jump_shift_continuations() -> Package {
    Package {
        name: "forcing-nt-jump-shift-continuations",
        gate: |a| forcing_nt_jump_shifts_on(&a.rebid),
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let base = format!("P* {} - 1NT -", call(1, Strain::from(major)));
                for second in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if second == major {
                        continue;
                    }
                    let node = format!("{base} {} -", call(3, Strain::from(second)));
                    entries.extend(game_force_rows(&node, major, Some(second)));
                }
                entries.extend(game_force_rows(&format!("{base} 2NT -"), major, None));
                let node = format!("{base} 3NT -");
                entries.extend(rows_of(
                    Pattern::node(&node),
                    responder_over_long_major_3nt(major),
                ));
                entries.extend(slam::rkcb_rows(&node, major));
                let correction = format!("{node} {} -", call(4, Strain::from(major)));
                entries.extend(rows_of(
                    Pattern::node(&correction),
                    opener_over_correction(),
                ));
                entries.extend(slam::rkcb_rows(&correction, major));
            }
            // The displaced two-suiter's invite one round later.
            entries.extend(rows_of(
                Pattern::node("P* 1♠ - 1NT - 2♥ - 2♠ -"),
                opener_after_preference(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♠ - 1NT - 2♥ - 2♠ - 3♥ -"),
                two_suiter::responder_over_forcing_nt_5_5(),
            ));
            entries.extend(rows_of(
                Pattern::node("P* 1♠ - 1NT - 2♥ - 2NT -"),
                opener_after_invite(),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
