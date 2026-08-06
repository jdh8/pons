//! Defensive actions for the 2/1 system: overcalls, advances, and doubles
//!
//! This module covers everything our side does when the opponents open the
//! auction.  It is the **index**: the per-call [`Alert`] constants the whole
//! book shares, and [`defensive()`], which folds each agreement's package into
//! the book.  Every agreement lives in its own submodule:
//!
//! | Module | Agreement |
//! | --- | --- |
//! | [`overcall`] | natural overcalls, the `1NT` overcall, the takeout double |
//! | [`michaels`], [`leaping_michaels`] | our two-suited overcalls |
//! | [`weak_two_defense`] | defending their weak two |
//! | [`advance_double`], [`advance_rich`], [`advance_sohl`] | advancing partner's double |
//! | [`responsive`] | the responsive double when they raise |
//! | [`gladiator`] | the relay structure after our `1NT` overcall |
//! | [`nt_defense`] | defending their `1NT` — the bundle and the natural chain |
//! | [`nt_landy`], [`nt_dont`], [`nt_meckwell`], [`nt_woolsey`] | the four systems' calls and advances |
//! | [`nt_their_conventions`] | defending their Stayman and transfers |

use super::super::constraint::{
    Cons, Constraint, and, at_least_as_long, balanced, equal_length, hcp, len, length_box,
    long_suit_box, longer_suit, longest_unbid, min_level_is, or, passed_hand, points,
    points_by_vul, shapes, short_in_their_suits, stopper_in_their_suits, suit_hcp,
    takeout_double_shape_ok, top_honors, unbid_support,
};
use super::super::context::Context;
use super::super::fallback::{described_rewrite, rewriter};
use super::super::inference::Range;
use super::super::rows::{Entry, Package, Pattern, classified, compile_into, rebase, rows_of};
use super::super::trie::{Classifier, classifier};
use super::super::{Alert, Defensive, Rules, Trie};
use super::competition::{
    LebensohlStyle, clubs_transfer_completion, complete_lebensohl_relay, cue_stayman_answer,
    cue_stayman_answer_no_stopper, delayed_cue, lebensohl_relay_rebid, lebensohl_responder,
    lm_2d_both_majors_advance, lm_2d_clubs_ask, lm_2d_clubs_major, stayman_2d_answer,
    stayman_2d_fit_rebid, transfer_completion, transfer_lebensohl_responder,
    transfer_stayman_2d_responder, transfer_target,
};
use super::notrump::{flat_4333, smolen_at_three, smolen_completion};
use super::openings::two_notrump_wide_shape;
use super::{call, other_major};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

mod advance_double;
mod advance_rich;
mod advance_sohl;
mod gladiator;
mod leaping_michaels;
mod michaels;
mod nt_defense;
mod nt_dont;
mod nt_landy;
mod nt_meckwell;
mod nt_their_conventions;
mod nt_woolsey;
mod overcall;
mod responsive;
mod weak_two_defense;

use advance_double::{advance_double_package, advance_of_double_package};
use advance_rich::rich_advance_double_package;
use gladiator::{gladiator_package, gladiator_sohl_package};
use leaping_michaels::leaping_michaels_package;
use michaels::unusual_notrump_advance_package;
use nt_defense::notrump_defense_package;
use nt_dont::direct_dont_advance_package;
use nt_landy::{both_majors_double_package, landy_advance_package};
use nt_meckwell::meckwell_advance_package;
use nt_their_conventions::{
    their_diamond_transfer_defense_package, their_minor_transfer_defense_package,
    their_stayman_defense_package, their_transfer_defense_package,
};
use nt_woolsey::woolsey_package;
use overcall::suit_defense_package;
use responsive::{responsive_double_package, responsive_overcall_package};
use weak_two_defense::{weak_two_defense_package, weak_two_notrump_advance_package};

pub use advance_double::advance_double;
pub use advance_rich::{
    set_advance_2nt_continuation, set_advance_minor_jump, set_advance_pass_yield_major,
    set_advance_rubens, set_advance_sit_hcp_gate, set_longest_first_advance,
    set_rich_advance_double,
};
pub use advance_sohl::set_advance_sohl_style;
pub(crate) use gladiator::{nt_overcall_gladiator, nt_overcall_systems_on};
pub use gladiator::{set_nt_overcall_gladiator, set_nt_overcall_systems_on};
pub(crate) use leaping_michaels::leaping_michaels_enabled;
pub use leaping_michaels::set_leaping_michaels;
pub use michaels::{set_two_suiter_hcp_floor, set_unusual_notrump_defense};
pub use nt_defense::{NotrumpDefense, set_notrump_balancing, set_notrump_defense};
pub(crate) use nt_defense::{natural_defense_enabled, notrump_defense};
pub(crate) use nt_dont::direct_dont_enabled;
pub use nt_dont::{
    set_direct_dont_four_four, set_direct_dont_one_suiter_min, set_direct_dont_x_floor,
};
pub(crate) use nt_landy::landy_range;
pub use nt_landy::{
    set_direct_landy_double, set_direct_landy_double_floor, set_direct_landy_penalty_pass,
    set_doubled_landy_escape, set_landy, set_landy_hcp,
};
pub(crate) use nt_meckwell::meckwell_enabled;
pub use nt_meckwell::{
    set_meckwell_minor_major_44, set_meckwell_x_floor, set_meckwell_x_four_four,
};
pub use nt_their_conventions::{
    set_diamond_transfer_defense, set_minor_transfer_defense, set_stayman_defense,
    set_stayman_defense_overcall, set_transfer_defense,
};
pub use nt_woolsey::{set_woolsey_double_floor, set_woolsey_points};
pub(crate) use nt_woolsey::{woolsey_double_floor, woolsey_enabled, woolsey_points};
pub use overcall::{
    DoubleShape, TakeoutSupport, defense_to_suit, set_natural_double_floor,
    set_natural_double_shape, set_natural_double_weight, set_natural_overcall_points,
    set_nt_overcall_no_major, set_overcall_discipline, set_overcall_four_card,
    set_passed_hand_overcall, set_strong_double_hcp, set_takeout_support,
    set_two_level_minor_overcall_tight,
};
pub(crate) use overcall::{natural_double_floor, natural_overcall_points};
pub(crate) use responsive::responsive_takeout_enabled;
pub use responsive::{set_responsive_overcall, set_responsive_takeout};
pub use weak_two_defense::{
    defense_to_weak_two, set_weak_two_cue, set_weak_two_jump_overcall,
    set_weak_two_notrump_advances, set_weak_two_notrump_points, set_weak_two_notrump_shape,
    set_weak_two_overcall_discipline, set_weak_two_overcall_points, set_weak_two_pass_gate,
};

/// Michaels cue-bid — 2 of their suit, 5-5, 8+ HCP (a two-suiter)
const MICHAELS: Alert = Alert("michaels");

/// Advancer's cue of opener's suit after partner's takeout double — general
/// invitational (10–11) with a 4-card unbid major, asking partner to raise it
const ADVANCE_CUE: Alert = Alert("advance-cue");

/// Advancer's jump-cue Rubens transfer — invitational-or-better with a 5+ unbid
/// major (the suit one rank above the bid), asking the doubler to declare it
const ADVANCE_TRANSFER: Alert = Alert("advance-transfer");

/// Unusual 2NT over a suit opening — 5-5 in the two lowest unbid suits
const UNUSUAL: Alert = Alert("unusual-2nt");

/// Leaping Michaels — a 4♣/4♦ jump over their weak two, a 5-5 game-forcing
/// two-suiter (distinct from the responder-side `comp:leaping-michaels`)
const LEAPING: Alert = Alert("leaping-michaels");

/// `3♣` relay over our 2NT overcall of their weak two — a weak *diamond* hand
/// looking for a 3-level partscore, routed through the forced `3♦`.  Says
/// nothing about clubs.
const WEAK_TWO_NT_RELAY: Alert = Alert("weak-two-nt:relay");
/// The forced `3♦` completion of that relay — pass-or-correct, and blind: it
/// says nothing about diamonds.
const WEAK_TWO_NT_RELAY_PC: Alert = Alert("weak-two-nt:relay-pc");
/// Cue of their weak two over our 2NT overcall = Stayman for the one unbid
/// major (exactly four, game values, not flat).
const WEAK_TWO_NT_STAYMAN: Alert = Alert("weak-two-nt:stayman");
/// Delayed cue (`3♣` relay → forced `3♦` → cue) — six-plus diamonds, long
/// enough that `4♦` is safe.  Says nothing about their major.
const WEAK_TWO_NT_DIAMONDS: Alert = Alert("weak-two-nt:diamonds");

/// Gladiator `2♣` relay over our 1NT overcall of their major — a weak takeout
/// (any suit) or an invitational hand routed through the forced `2♦`.
const GLADIATOR_RELAY: Alert = Alert("gladiator:relay");
/// Gladiator's forced `2♦` completion of the `2♣` relay — pass-or-correct, says
/// nothing about diamonds.
const GLADIATOR_RELAY_PC: Alert = Alert("gladiator:relay-pc");
/// Gladiator cue of their major — Stayman for the *one* unbid major (exactly 4,
/// invitational-or-better).
const GLADIATOR_STAYMAN: Alert = Alert("gladiator:stayman");
/// Gladiator `2NT` — a weak transfer to clubs (6+♣): a weak long-club hand
/// cannot sign off naturally below the cue, so it detours through `2NT` and the
/// overcaller completes to `3♣` (invitational clubs go `2♣`→`2♦`→`3♣` instead).
const GLADIATOR_CLUB_TRANSFER: Alert = Alert("gladiator:club-transfer");
/// Gladiator splinter (`3` of their major) — a game-forcing raise of the unbid
/// major with a singleton/void in their suit.
const GLADIATOR_SPLINTER: Alert = Alert("gladiator:splinter");
/// Gladiator **delayed cue** (`2♣` relay → forced `2♦` → cue of their major) —
/// exactly 3 in the unbid major, INV+, **not flat (4333)**.  The direct cue
/// promises 4; this delayed route promises exactly 3 with a doubleton (ruffing
/// value), checking for the 5-3 fit a balanced 5-card-major 1NT overcall can hold
/// (over `1♠`, a balanced 15–18 may hold 5 hearts).
const GLADIATOR_DELAYED_CUE: Alert = Alert("gladiator:delayed-cue");

/// Responsive double — partner doubled/overcalled, they raised, advancer's double
/// shows the two unbid suits (4-4, 8+).  A takeout call (asks partner to pick a
/// suit), not a desire to defend, so it is alerted rather than read structurally.
const RESPONSIVE: Alert = Alert("responsive-double");

/// Direct takeout double of their suit opening — 12+ (or the 17+ any-shape tier),
/// short in their suit, asking partner to pick an unbid suit.  Takeout by meaning,
/// so alerted even though its shape predicates project no length floor (the reading
/// is a sound points floor only — the 17+ tier admits any shape).
const TAKEOUT_DOUBLE: Alert = Alert("takeout-double");

/// Landy SOS redouble — after `(1NT) 2♣ (X)`, equal majors asking the overcaller to
/// name the longer one.  A "pick a suit" call, not a desire to sit, so alerted.
const LANDY_SOS: Alert = Alert("landy:sos-redouble");

/// Answers to the Landy `2NT` game-force ask — conventional step responses, not
/// natural suits: `3♣`/`3♦` are the 5-4 strength steps and name no minor at all,
/// `3♥`/`3♠`/`3NT` are 5-5 minimum / medium / maximum.  The row-layer invariant
/// caught `3♥`: it floors *spades* at five while naming hearts.
const LANDY_2NT_ANSWER: Alert = Alert("landy:2nt-answer");

const WOOLSEY_X: Alert = Alert("1ntd:woolsey-x");
const LANDY_X: Alert = Alert("1ntd:landy-x");
const DONT_X: Alert = Alert("1ntd:dont-x");
const LANDY_2C: Alert = Alert("1ntd:landy-2c");
const WOOLSEY_2C: Alert = Alert("1ntd:woolsey-2c");
const DONT_2C: Alert = Alert("1ntd:dont-2c");
const MULTI_2D: Alert = Alert("1ntd:multi-2d");
const DONT_2D: Alert = Alert("1ntd:dont-2d");
const MUIDERBERG_2H: Alert = Alert("1ntd:muiderberg-2h");
const DONT_2H: Alert = Alert("1ntd:dont-2h");
const MUIDERBERG_2S: Alert = Alert("1ntd:muiderberg-2s");
const UNUSUAL_2NT: Alert = Alert("1ntd:unusual-2nt");
/// Meckwell two-way `X` — a single 6+ minor OR both majors.
const MECKWELL_X: Alert = Alert("1ntd:meckwell-x");
/// Meckwell `2♣` — clubs + a major (5-4+).
const MECKWELL_2C: Alert = Alert("1ntd:meckwell-2c");
/// Meckwell `2♦` — diamonds + a major (5-4+).
const MECKWELL_2D: Alert = Alert("1ntd:meckwell-2d");
/// Lead-directing double of the opponents' 2♣ Stayman — shows clubs (the bid
/// suit), not takeout.
const STAYMAN_DEFENSE_X: Alert = Alert("staydef:x-clubs");
/// Lead-directing double of the opponents' Jacoby transfer — shows the bid
/// (transfer) suit, not takeout.
const TRANSFER_DEFENSE_X: Alert = Alert("xferdef:x-bidsuit");
/// Cue of the suit the opponents showed via transfer — the other major + a minor
/// (Michaels).
const TRANSFER_DEFENSE_CUE: Alert = Alert("xferdef:cue-michaels");
/// Lead-directing double of the opponents' two-way 2♠ minor response — shows
/// spades (the bid suit), not takeout.
const MINOR_TRANSFER_DEFENSE_X: Alert = Alert("minorxferdef:x-spades");
/// `2NT` over their 2♠ — the two lowest unbid suits (diamonds + hearts, 5-5).
const MINOR_TRANSFER_DEFENSE_2NT: Alert = Alert("minorxferdef:2nt-reds");
/// Cue of their shown-clubs anchor (`3♣`) — the top-and-bottom two-suiter
/// (spades + diamonds, 5-5).
const MINOR_TRANSFER_DEFENSE_CUE: Alert = Alert("minorxferdef:cue-top-bottom");
/// Lead-directing double of the opponents' 2NT diamond transfer — shows diamonds
/// (the shown suit), not takeout.
const DIAMOND_TRANSFER_DEFENSE_X: Alert = Alert("diaxferdef:x-diamonds");
/// Cue of their shown-diamonds anchor (`3♦`) — both majors (5-5, Michaels).
const DIAMOND_TRANSFER_DEFENSE_CUE: Alert = Alert("diaxferdef:cue-majors");

/// M6.2d guard: every re-authored `or`/`and` defense shape accepts exactly the hands
/// its intended spec does, on every sampled hand — the proof the combinator forms say
/// what they should (and the only check that the simplified shapes match their gloss).
#[cfg(test)]
mod shape_guards;

/// Build the defensive book: all our actions when the opponents open
///
/// Every package leads with `P*`, the seat fan, so every seat is covered.  A
/// defensive auction string starts from their opening, e.g. `P* (1♦) 2♦ -`
/// means they opened 1♦, we cue-bid 2♦ (Michaels), opener's side passed, and we
/// are the advancer.  The one hand-rolled site left is the systems-on graft of
/// the whole opening-1NT book below our 1NT overcall — `compile_into` writes
/// rows, not a subtree.
#[must_use]
pub fn defensive() -> Defensive {
    let mut d = Defensive::new();

    // Systems-on advances of our 1NT overcall: the whole 1NT-opening response
    // structure (Stayman, transfers, Smolen — reflecting the same knobs), built
    // once and grafted below each `(their-suit) 1NT` so the advancer plays it
    // verbatim.  On by default; see `set_nt_overcall_systems_on`.
    let nt_overcall_book = nt_overcall_systems_on().then(|| {
        let mut nt = Trie::new();
        super::notrump::register_one_nt(&mut nt);
        nt
    });

    // Over each one-of-a-suit opening: our direct defense, and the advances of
    // partner's Michaels cue and Unusual 2NT.
    compile_into(&mut d, &[suit_defense_package()]);

    // Advancing partner's takeout double of a one-of-a-suit opening, and — when
    // the rich ladder is on — the continuations of its artificial calls.
    compile_into(
        &mut d,
        &[advance_double_package(), rich_advance_double_package()],
    );

    // Advances of our 1NT overcall (`(1t) 1NT -`).  Over a MINOR the advancer
    // plays the full opening-1NT structure (Stayman/transfers/Smolen) grafted
    // below `(1t) 1NT` — `(1♦) 1NT` equals `(1♣) 1NT` equals an opening 1NT,
    // transfers preserving right-siding.  Grafted in every seat the opening could
    // have been made (mirrors the overcall's fan).  This is the one permanently
    // imperative site: `compile_into` writes rows, not a whole subtree.
    //
    // Advances of a *natural* overcall (`(1t) overcall -`) are left to the
    // instinct floor's Rubens transfers — the programmatic floor expresses the
    // transfer band for every (opening, overcall) pair in one place, where a
    // per-suit authored table cannot.
    if let Some(nt) = &nt_overcall_book {
        let one_nt = call(1, Strain::Notrump);
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            // Over a major the graft's Stayman would look for a major they own;
            // `gladiator_package` replaces it with the geometry that fits.
            if matches!(suit, Suit::Hearts | Suit::Spades) && nt_overcall_gladiator() {
                continue;
            }
            let opening = Bid::new(1, Strain::from(suit));
            for n in 0..=3 {
                let prefix: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
                    .chain([Call::Bid(opening), one_nt])
                    .collect();
                let collisions = d.graft(&prefix, nt, &[one_nt]);
                debug_assert!(
                    collisions.is_empty(),
                    "1NT-overcall systems-on graft collides at {prefix:?}: {collisions:?}"
                );
            }
        }
    }

    // Gladiator, when on: the advances of our 1NT overcall of their major, then
    // the tail for their 2-level action over it.
    compile_into(&mut d, &[gladiator_package(), gladiator_sohl_package()]);

    // Responsive doubles: partner acted (double or overcall) and they raised.
    compile_into(
        &mut d,
        &[responsive_double_package(), responsive_overcall_package()],
    );

    // Over each weak-two opening (the row packages): takeout double, natural
    // overcalls, 2NT; then the advances of our 2NT overcall and of Leaping
    // Michaels.
    compile_into(
        &mut d,
        &[
            weak_two_defense_package(),
            weak_two_notrump_advance_package(),
            leaping_michaels_package(),
        ],
    );
    // Advancing partner's takeout double: `(2t) X -` — advancer to act.
    compile_into(&mut d, &[advance_of_double_package()]);

    // Their 1NT opening and the three artificial responses we have a defense to
    // (Stayman, Jacoby, the two-way 2♠ and the 2NT diamond transfer); all three
    // response defenses are opt-in, default off.
    compile_into(
        &mut d,
        &[
            notrump_defense_package(),
            their_stayman_defense_package(),
            their_transfer_defense_package(),
            their_minor_transfer_defense_package(),
            their_diamond_transfer_defense_package(),
        ],
    );

    // Advancing partner's Landy 2♣ (both majors) over their 1NT, when on.  Woolsey's
    // 2♣ is the identical both-majors call on the same shared band, so it reuses this
    // same advance wiring.
    compile_into(&mut d, &[landy_advance_package()]);

    // Woolsey "Multi-Landy" continuations, when on.
    compile_into(&mut d, &[woolsey_package()]);

    // Advancing partner's both-minors 2NT over their 1NT, when on.
    compile_into(&mut d, &[unusual_notrump_advance_package()]);

    // Direct-seat DONT and Meckwell advances.  Both write `(1NT) X -` and
    // friends; with both knobs on, Meckwell wins the shared keys exactly as it
    // did in the consecutive imperative blocks, so the package order here is
    // load-bearing.
    compile_into(
        &mut d,
        &[direct_dont_advance_package(), meckwell_advance_package()],
    );

    // Direct-seat both-majors X advances.
    compile_into(&mut d, &[both_majors_double_package()]);
    d
}

#[cfg(test)]
mod tests;
