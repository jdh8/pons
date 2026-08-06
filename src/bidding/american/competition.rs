//! The competitive package over our openings
//!
//! This module builds the [`Competitive`] book that covers contested auctions
//! after our one-level openings.  It is the **index**: the per-call [`Alert`]
//! constants that the whole book shares, and [`competition()`], which folds
//! each agreement's package into the book.  Every agreement lives in its own
//! submodule:
//!
//! | Module | Agreement |
//! | --- | --- |
//! | [`over_overcall`] | responder's direct-seat action over their overcall |
//! | [`penalty_double`] | and responder's `X`/`Pass` options within it |
//! | [`free_bids`], [`negative_double`], [`cue_raise`] | and opener's answer to each |
//! | [`support_double`] | opener's three-card-support `X`/`XX` |
//! | [`over_their_double`] | Jordan/Truscott, and our doubled splinter |
//! | [`high_overcall`] | their jump and three-level overcalls |
//! | [`two_suiters`] | Michaels / unusual `2NT` over our `1M` |
//! | [`our_preempts`] | our contested weak twos and strong `2♣` |
//! | [`lebensohl`], [`rubensohl`], [`uvu`] | over their overcall of our `1NT` |
//! | [`over_our_notrump_calls`] | when they compete over our Stayman or transfer |

use super::super::constraint::{
    Cons, Constraint, balanced, described, has_stopper, hcp, len, min_level_is, partner_suit_is,
    points, stopper_in, stopper_in_their_suits, suit_hcp, support, they_bid, top_honors,
    vulnerable,
};
use super::super::context::Context;
use super::super::fallback::{ReplaceNext, described_guard, described_rewrite, guard, rewriter};
use super::super::rows::{
    Bindings, Entry, Package, Pattern, classified, compile_into, expand, rebase, row, rows_of,
};
use super::super::trie::{Classifier, classifier};
use super::super::{Alert, Competitive, Rules};
use super::call;
use super::notrump::{
    PUPPET, complete_transfer, notrump_minors, notrump_responses, smolen_at_three,
    smolen_completion, stayman_answers, transfer_super_accept,
};
use super::weak_twos;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

mod cue_raise;
mod free_bids;
mod high_overcall;
mod lebensohl;
mod negative_double;
mod our_preempts;
mod over_our_notrump_calls;
mod over_overcall;
mod over_their_double;
mod penalty_double;
mod rubensohl;
mod support_double;
mod two_suiters;
mod uvu;

use cue_raise::{cue_minor_raise_answer_package, cue_raise_answer_package};
use free_bids::{free_bid_answer_package, transfer_free_bid_package};
use high_overcall::high_overcall_package;
use lebensohl::lebensohl_package;
use negative_double::{
    answer_negative_double_package, cachalot_package, sputnik_residual_answer_package,
};
use our_preempts::{strong_two_competition_package, weak_two_competition_package};
use over_our_notrump_calls::{
    competition_over_diamond_transfer_package, competition_over_minor_transfer_package,
    competition_over_stayman_package, competition_over_transfer_package,
};
use over_overcall::direct_seat_package;
use over_their_double::{jordan_truscott_package, splinter_doubled_package};
use support_double::support_double_package;
use two_suiters::uvu_over_majors_package;
use uvu::uvu_package;

pub(super) use cue_raise::delayed_cue;
pub use cue_raise::{set_cue_minor_raise_answer, set_cue_raise_answer, set_delayed_cue};
pub use free_bids::{
    FreeBidStyle, set_free_1nt_floor, set_free_bid_floor, set_free_bid_quality, set_free_bid_style,
    set_free_bids,
};
pub use high_overcall::set_high_overcall_responses;
pub(crate) use lebensohl::lebensohl_style;
pub use lebensohl::{LebensohlStyle, set_defense_to_2d_multi, set_lebensohl_style};
pub(super) use lebensohl::{complete_lebensohl_relay, lebensohl_relay_rebid, lebensohl_responder};
pub use negative_double::{
    NegativeDoubleShape, set_cachalot_contested_x, set_negative_double_shape,
};
pub use our_preempts::{set_strong_two_competition, set_weak_two_competition};
pub use over_our_notrump_calls::{
    set_competition_over_diamond_transfer, set_competition_over_minor_transfer,
    set_competition_over_stayman, set_competition_over_transfer,
};
pub use over_overcall::{set_direct_3nt_stopper, set_natural_floor};
pub(crate) use over_their_double::jordan_truscott;
pub use over_their_double::{set_jordan_truscott, set_redouble_answer, set_splinter_doubled};
pub use penalty_double::{
    DoubleStyle, set_double_override, set_double_style, set_penalty_double_leave_in,
    set_penalty_pass, set_trap_pass,
};
pub use rubensohl::{Competitive4333, set_competitive_4333};
pub(super) use rubensohl::{
    clubs_transfer_completion, cue_stayman_answer, cue_stayman_answer_no_stopper,
    lm_2d_both_majors_advance, lm_2d_clubs_ask, lm_2d_clubs_major, stayman_2d_answer,
    stayman_2d_fit_rebid, transfer_completion, transfer_lebensohl_responder,
    transfer_stayman_2d_responder, transfer_target,
};
pub(crate) use support_double::major_support_double;
pub use support_double::set_major_support_double;
pub use two_suiters::set_uvu_over_majors;
pub use uvu::{set_uvu, set_uvu_cue_floor, set_uvu_natural_floor, set_uvu_x_floor};

// Per-call alerts for the competitive book's artificial calls.  An [`Alert`] marks
// a call as *conventional*: the inference reader decodes it as the convention
// rather than as a natural suit.  Natural raises, natural suit rebids, natural
// notrump, penalty passes, and the catch-all `Pass` stay unalerted.

/// Cue-bid raise — a cue of the opponents' suit as a limit-plus raise of partner's
/// opening (not natural).
const CUE_RAISE: Alert = Alert("comp:cue-raise");
/// Negative double — responder's takeout double showing the unbid suit(s) after
/// partner opens and RHO overcalls.
const NEGATIVE_DOUBLE: Alert = Alert("comp:negative-double");
/// Support double / redouble — opener's `X`/`XX` showing exactly three-card support.
const SUPPORT_DOUBLE: Alert = Alert("comp:support-double");
/// Multi takeout double — `X` of their `(2♦)` Multi, values/takeout of the unknown
/// major (8+).  Takeout by meaning, so alerted (the reading is a sound 8+ points
/// floor — their suit is unknown, so no single side suit to floor).
const MULTI_TAKEOUT: Alert = Alert("comp:multi-takeout");
/// Lebensohl `2NT` — the weak relay to `3♣` over their overcall of our `1NT`.
const LEBENSOHL_RELAY: Alert = Alert("comp:lebensohl-relay");
/// Lebensohl cue — a cue of their suit as game-forcing Stayman.
const LEBENSOHL_CUE: Alert = Alert("comp:lebensohl-cue");
/// Transfer-Lebensohl 3-level transfer — bids the next suit up *through* the
/// adverse suit (INV+).
const LEBENSOHL_TRANSFER: Alert = Alert("comp:lebensohl-transfer");
/// Stayman over `(2♦)` — `3♣` as game-forcing Stayman (with Smolen after the
/// `3♦` denial).
const STAYMAN: Alert = Alert("comp:stayman");
/// Smolen — showing a 5-card major right-sided after the Stayman denial.
const SMOLEN: Alert = Alert("comp:smolen");
/// Leaping Michaels — `4♣`/`4♦` jumps naming a 5-5 game-forcing two-suiter.
const LEAPING_MICHAELS: Alert = Alert("comp:leaping-michaels");
/// Unusual-vs-Unusual cue — `3♣`/`3♦` cues finding a major fit over their
/// both-minors `2NT`.
const UVU_CUE: Alert = Alert("comp:uvu-cue");
/// Unusual-vs-Unusual splinter — `4♣`/`4♦` as a FG+ 5-5-majors splinter into the
/// short minor.
const UVU_SPLINTER: Alert = Alert("comp:uvu-splinter");
/// Stayman re-ask — responder's `XX` after the opponents doubled our 2♣ Stayman
/// and opener passed to deny a club stopper: re-asks the major (forcing).
const STAYMAN_REDOUBLE: Alert = Alert("comp:stayman-redouble");
/// Transfer re-ask — responder's `XX` after the opponents doubled our Jacoby
/// transfer and opener passed to decline: forces opener to complete (forcing).
const TRANSFER_REDOUBLE: Alert = Alert("comp:transfer-redouble");
/// Unusual-vs-unusual over our 1M — the cheaper cue of the two-suiter's suits
/// (`3♣` over their both-minors `(2NT)`, the other-major cue over their
/// Michaels) as a limit-plus raise of our major.
const UVU_MAJOR_RAISE: Alert = Alert("comp:uvu-major-raise");
/// The second cue over their both-minors `(2NT)` — `3♦` as a game force with
/// 5+ cards in the other major.
const UVU_MAJOR_FOURTH: Alert = Alert("comp:uvu-major-fourth");
/// Business redouble of their takeout double of our weak two — 13+ values
/// (redoubles are natural-by-default; the alert buys the points-floor decode).
const WEAK_TWO_XX: Alert = Alert("comp:weak-two-xx");
/// Ogust survives their overcall of our weak two — the contested `2NT` still
/// asks (2+ card support, 14+), alerted so the fit and strength project.
const CONTESTED_OGUST: Alert = Alert("comp:ogust");
/// Cachalot rotated double — 4+ cards in the *adjacent* major (hearts over
/// `(1♦)`, spades over `(1♥)`), not a classic unbid-majors negative double.
const CACHALOT_X: Alert = Alert("comp:cachalot-x");
/// Cachalot transfer — `1♥` over `(1♦)` showing 4+ **spades**.
const CACHALOT_TRANSFER: Alert = Alert("comp:cachalot-transfer");
/// Cachalot residual — `1♠` over `(1♦)`/`(1♥)` as the takeout hand, ≤3 in
/// each major the rotation could have shown.
const CACHALOT_TAKEOUT: Alert = Alert("comp:cachalot-takeout");
/// 2-level free-bid transfer (`FreeBidStyle::Transfer`) — a non-jump 2-level
/// new suit over their overcall showing the *other* unbid suit when exactly
/// two unbid suits sit at the two level; opener completes and declares.
const FREE_TRANSFER: Alert = Alert("comp:free-transfer");
/// Cachalot completion — opener's 1-level completion of the transfer shows
/// **exactly three** trumps (forcing one round; the raise shows four).
const CACHALOT_THREE: Alert = Alert("comp:cachalot-three");
/// Jordan/Truscott `2NT` over their takeout double — a limit-plus raise of
/// the opening (4+ support for a major, 5+ for a minor), not natural.
const JORDAN: Alert = Alert("comp:jordan");
/// Value redouble over their takeout double — 10+ without the Jordan fit
/// (redoubles are natural-by-default; the alert buys the points-floor decode).
const VALUE_REDOUBLE: Alert = Alert("comp:value-redouble");

/// The competitive package over our openings: cue-bid raises, preemptive raises,
/// negative doubles for all four openings, support doubles/redoubles, and
/// opener's answers to negative doubles of minor overcalls
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`Pair::against`][crate::bidding::Pair::against] (as [`american`][super::american] is meant to be
/// used) so it resolves into the uncontested core.
#[must_use]
pub fn competition() -> Competitive {
    let mut book = Competitive::new();

    // Section 1 & 2: over all four openings, attach direct-seat response rules
    // and system-on over their double.
    compile_into(&mut book, &[direct_seat_package()]);

    // Section 2b: systems-on over their double of our splinter.
    // Section 3: support doubles and redoubles for each (opening, major) pair.
    compile_into(
        &mut book,
        &[splinter_doubled_package(), support_double_package()],
    );

    // Section 4: opener answers partner's negative double of a two-level minor.
    // Section 4b/4c: opener answers partner's cue-raise of the opening suit.
    compile_into(
        &mut book,
        &[
            answer_negative_double_package(),
            cue_raise_answer_package(),
            cue_minor_raise_answer_package(),
        ],
    );

    // Section 4d/4d′/4d″/4d‴: opener answers responder's natural free bid,
    // and the Negative style's capped-free-bid continuations.
    compile_into(&mut book, &[free_bid_answer_package()]);

    // Section 4f (`FreeBidStyle::Transfer`): opener completes the 2-level
    // free-bid transfer and responder clarifies. The swap contexts are a
    // closed enumeration — (opening, their overcall, lower slot → shown,
    // wrap slot → shown, completing a level higher on the wrap):
    compile_into(&mut book, &[transfer_free_bid_package()]);

    // Section 6: their two-suiters over our 1M.
    compile_into(&mut book, &[uvu_over_majors_package()]);

    // Section 11: over their takeout double (`set_jordan_truscott`, default
    // on). Responder's first call at the deeper `1x (X)` key — it wins over
    // the `1x` FirstIs(X) systems-on rebase structurally, and the rebase
    // survives untouched below it for every deeper suffix the package's
    // exact-suffix guards don't claim.
    compile_into(&mut book, &[jordan_truscott_package()]);

    // Section 10: their jump / 3-level suit overcalls
    // (`set_high_overcall_responses`, default off). A guarded entry at `1x` —
    // its bid range (2NT, 3♠] sits above the shipped per-overcall exact nodes
    // (which stop at 2♠), so nothing races it. Their (2NT) and their 3-level
    // cue of our own suit are excluded (the first is a two-suiter, the second
    // is rare enough for the floor).
    compile_into(&mut book, &[high_overcall_package()]);

    // Section 9: opener's Cachalot answers (`NegativeDoubleShape::Cachalot`
    // only). Section 9b: opener's answers to the Sputnik residual double.
    compile_into(
        &mut book,
        &[cachalot_package(), sputnik_residual_answer_package()],
    );

    // Section 7: our contested weak twos (`set_weak_two_competition`, default
    // off). Their double: responder's first call at the deeper `2M (X)` node
    // (business XX riding on the uncontested responses), everything deeper
    // systems-on. Their overcall (≤ 3♠): responder's direct action, and a
    // targeted rebase so an Ogust 2NT bid over the overcall still gets
    // opener's undisturbed five-rung answer.
    compile_into(&mut book, &[weak_two_competition_package()]);

    // Section 8: our contested strong 2♣ (`set_strong_two_competition`,
    // default on). Their double steals no room → systems on wholesale; their
    // overcall gets responder's natural-GF / values-X / waiting-pass table,
    // backed by opener's forced reopening in the pass-out seat.
    compile_into(&mut book, &[strong_two_competition_package()]);

    // Section 5 / 5b / 5c: Lebensohl after our 1NT is overcalled at the 2
    // level. Purely additive — nothing else lands at `1NT` in the competitive
    // book. Plain or Transfer Lebensohl per [`LebensohlStyle`]; both keep the
    // weak 2NT relay, and (2♣) gets the systems-on rebase instead.
    compile_into(&mut book, &[lebensohl_package()]);

    // Competition over our own conventions: the opponents double or overcall
    // our Stayman, our Jacoby transfer, our two-way 2♠, or our 2NT diamond
    // transfer.  Each is keyed under the uncontested `1NT - <our call>`
    // node — a distinct trie path from the systems-on blocks where their call
    // sits at depth 1 — and each shares the `X (bid) …` systems-on rebase.
    compile_into(
        &mut book,
        &[
            competition_over_stayman_package(),
            competition_over_transfer_package(),
            competition_over_minor_transfer_package(),
            competition_over_diamond_transfer_package(),
        ],
    );

    // Section 5d: Unusual vs Unusual over their (2NT) overcall of our 1NT.
    compile_into(&mut book, &[uvu_package()]);

    book
}

#[cfg(test)]
mod tests;
