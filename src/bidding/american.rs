//! A 2/1 game-forcing bidding system
//!
//! [`american()`][crate::bidding::american::american] assembles a
//! [`Pair`] for the Two-over-One Game Forcing system, the modern North
//! American standard: five-card majors, a strong 15–17 notrump, the strong
//! artificial 2♣, and — the defining feature — a new suit at the two level in
//! response to a one-of-a-major opening is **game forcing**.
//!
//! The system is authored entirely from the constraint vocabulary
//! ([`constraint`][crate::bidding::constraint]), the [`Rules`] classifier, and
//! the role-aware books — the strictly uncontested core in a [`Constructive`]
//! book, [`competition()`][crate::bidding::american::competition] over our
//! openings in a [`Competitive`][super::Competitive] book, and our actions
//! over their openings in a [`Defensive`][super::Defensive] book; nothing here
//! is system infrastructure.
//!
//! # Conventions
//!
//! - **Openings**: 15–17 1NT (balanced, or a 5422 with a five-card minor),
//!   20–21 2NT, strong artificial 2♣ (22+), five-card majors (light in 3rd/4th
//!   seat), better minor, weak twos, three-level preempts.
//! - **Responses**: 2/1 game forces with full continuations to game and the
//!   slam-try level, forcing 1NT (with the three-card limit raise rebid),
//!   Jacoby 2NT with shortness/second-suit rebids, splinters, inverted
//!   minors, weak jump shifts.
//! - **The 2♣ structure**: 2♦ waiting, 2♥ double negative, natural positives;
//!   notrump rebids carry the 2NT machinery ("system on").
//! - **Notrump structures**: Stayman and Jacoby transfers at the two and
//!   three levels, quantitative 4NT at every notrump strength.
//! - **Weak twos**: Ogust 2NT, RONF raises, forcing new suits.
//! - **Slam**: RKCB 1430 with the 5NT king ask
//!   (`slam`) below every major-suit trump agreement.
//! - **Competition**: cue-bid (limit-plus) raises, preemptive jump raises,
//!   negative doubles, system-on over their double, support
//!   doubles/redoubles.
//! - **Defense**: overcalls, takeout doubles, 1NT overcall, Michaels and the
//!   unusual 2NT with advances, advancing partner's takeout double, responsive
//!   doubles, defense to 1NT, and defense to weak twos (takeout double, natural
//!   2NT and suit overcalls).
//! - **Instinct floor**: both contested books carry the
//!   [`instinct`][crate::bidding::instinct()] ladder as a root fallback, so
//!   every contested auction gets a sane natural answer — in particular,
//!   partner's takeout double is never passed without a trump stack.
//!
//! Auctions no authored pass covers fall to the instinct floor, which answers
//! them with a sane natural call; see the crate changelog for what each
//! authored pass added (lebensohl, minor-suit keycard, reopening actions…).
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

use super::fallback::{Always, Fallback, Guard};
use super::instinct::instinct;
use super::trie::Classifier;
use super::{Constructive, Family, Pair, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain};
use std::sync::Arc;

mod competition;
mod defense;
mod game_force;
mod nmf;
mod notrump;
mod openings;
mod raises;
mod rebids;
mod responses;
pub(in crate::bidding) mod slam;
mod strong_two;
mod weak_twos;
mod xyz;

pub use competition::{
    Competitive4333, DoubleStyle, FreeBidStyle, LebensohlStyle, NegativeDoubleShape, competition,
    set_cachalot_contested_x, set_competition_over_diamond_transfer,
    set_competition_over_minor_transfer, set_competition_over_stayman,
    set_competition_over_transfer, set_competitive_4333, set_cue_minor_raise_answer,
    set_cue_raise_answer, set_defense_to_2d_multi, set_delayed_cue, set_direct_3nt_stopper,
    set_double_override, set_double_style, set_free_1nt_floor, set_free_bid_floor,
    set_free_bid_quality, set_free_bid_style, set_free_bids, set_high_overcall_responses,
    set_jordan_truscott, set_lebensohl, set_lebensohl_style, set_major_support_double,
    set_natural_floor, set_negative_double_shape, set_penalty_double_leave_in, set_penalty_pass,
    set_redouble_answer, set_splinter_doubled, set_strong_two_competition, set_trap_pass, set_uvu,
    set_uvu_cue_floor, set_uvu_natural_floor, set_uvu_over_majors, set_uvu_x_floor,
    set_weak_two_competition,
};
// The inference walk reads this knob at classify time (the two-suiter reading).
pub(crate) use competition::uvu_over_majors;
pub use defense::{
    DoubleShape, NotrumpDefense, TakeoutSupport, advance_double, defense_to_suit,
    defense_to_weak_two, set_advance_2nt_continuation, set_advance_minor_jump, set_advance_rubens,
    set_advance_sohl_style, set_always_pass_defense, set_diamond_transfer_defense, set_direct_dont,
    set_direct_dont_four_four, set_direct_dont_one_suiter_min, set_direct_dont_x_floor,
    set_direct_landy_double, set_direct_landy_double_floor, set_direct_landy_penalty_pass,
    set_doubled_landy_escape, set_landy, set_landy_hcp, set_leaping_michaels,
    set_longest_first_advance, set_meckwell, set_meckwell_minor_major_44, set_meckwell_x_floor,
    set_meckwell_x_four_four, set_minor_transfer_defense, set_natural_defense,
    set_natural_double_floor, set_natural_double_shape, set_natural_double_weight,
    set_natural_overcall_points, set_notrump_balancing, set_notrump_defense,
    set_nt_overcall_gladiator, set_nt_overcall_no_major, set_nt_overcall_systems_on,
    set_overcall_discipline, set_passed_hand_overcall, set_responsive_overcall,
    set_responsive_takeout, set_rich_advance_double, set_stayman_defense,
    set_stayman_defense_overcall, set_strong_double_hcp, set_takeout_support, set_transfer_defense,
    set_two_level_minor_overcall_tight, set_two_suiter_hcp_floor, set_unusual_notrump_defense,
    set_woolsey, set_woolsey_double_floor, set_woolsey_points,
};
pub(crate) use defense::{
    direct_dont_enabled, direct_landy_double, landy_range, meckwell_enabled,
    natural_defense_enabled, natural_double_floor, natural_overcall_points, nt_overcall_gladiator,
    nt_overcall_systems_on, woolsey_double_floor, woolsey_enabled, woolsey_points,
};
pub use game_force::{set_game_backstop, set_opener_third, set_second_suit_agreement};
pub use nmf::set_new_minor_forcing;
pub use notrump::{
    EUROPEAN, PUPPET, notrump_responses, set_crawling_stayman, set_garbage_stayman,
    set_invitational_5card_majors, set_long_minor_force, set_minor_min_to_3nt, set_notrump_minors,
    set_sixcard_accept_floor, set_sixcard_invite_floor, set_stayman_5card_max,
    set_stayman_both_majors, set_stayman_cue_continuation, set_stayman_minor_slam_try,
    set_texas_game_floor, set_texas_slam_drive, set_transfer_gf_hearts, set_transfer_gf_majors,
    set_transfer_longer_major, set_transfer_slam_try, set_transfer_super_accept,
};
pub(crate) use notrump::{crawling_stayman, garbage_stayman, notrump_minors};
pub use openings::{
    NotrumpShape, WeakTwoEval, openings, openings_with, set_notrump_shape, set_one_notrump_fifths,
    set_open_one_notrump, set_opening_hcp_floor, set_rule_of_20, set_weak_two_eval,
    set_weak_two_hcp,
};
pub(crate) use openings::{notrump_shape, rule_of_20_enabled};
pub use raises::{set_limit_raise_acceptance, set_major_game_tries};
pub(crate) use rebids::{opener_extras_ladder, opener_major_jump_rebid};
pub use rebids::{
    set_balanced_1nt_rebid, set_forcing_nt_two_suiter, set_fourth_suit_forcing,
    set_major_rebid_tails, set_meckstroth_adjunct, set_nt_invite_hcp, set_opener_extras_ladder,
    set_opener_major_jump_rebid,
};
pub(crate) use responses::longer_major_response;
pub use responses::{
    TwoOverOneGate, major_responses, minor_responses, set_longer_major_response,
    set_major_choice_of_games, set_two_over_one_fit, set_two_over_one_gate, set_up_the_line,
};
pub use slam::set_minor_keycard;
pub use xyz::{set_xyz, set_xyz_invite_judgment};

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

/// Interleave one opposing pass after each of our calls
///
/// The constructive book keys the *raw table auction*, so an undisturbed
/// sequence of our calls `[1♥, 1♠]` lives at `[1♥, P, 1♠, P]` (plus leading
/// passes for the opener's seat).  This is the one place that spells out the
/// interleaving; author keys through it, never by hand.
fn uncontested(our_calls: &[Call]) -> Vec<Call> {
    our_calls
        .iter()
        .flat_map(|&call| [call, Call::Pass])
        .collect()
}

/// Insert a continuation table after our undisturbed `our_calls`, every seat
///
/// Keys at `uncontested(our_calls)` under every leading-pass prefix
/// (`0..=3`), so the table answers regardless of which seat opened.  An empty
/// `our_calls` registers an opening table.
pub(in crate::bidding) fn insert_uncontested(
    book: &mut Trie,
    our_calls: &[Call],
    rules: impl Classifier + 'static,
) {
    insert_all_seats(book, &uncontested(our_calls), 3, rules);
}

/// Attach a guarded fallback at `suffix` under every leading-pass prefix
// ponytail: `guard`/`fallback` stay by-value — callers pass a freshly built
// `Arc::new(ConcreteGuard)`, which unsize-coerces to `Arc<dyn Guard>` only on
// the move; a `&Arc<dyn Guard>` param would force a `let` binding at all ~20
// call sites for no real gain.
#[allow(clippy::needless_pass_by_value)]
fn fallback_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    guard: Arc<dyn Guard>,
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
// Assembly
// ---------------------------------------------------------------------------

/// Build the basic 2/1 game-forcing system as one side's [`Pair`]
///
/// Bind it against the opponents' [`Family`] for a playable system, and seat
/// two pairs with [`Table::of_pairs`][super::Table::of_pairs] for a full
/// table.
///
/// ```
/// use pons::american;
/// use pons::bidding::{Family, System};
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = american().against(Family::NATURAL);
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
pub fn american() -> Pair {
    with_floor(bare_american(), super::neural_floor::NeuralFloorBba)
}

/// The 2/1 pair with the deterministic **instinct** floor (the pre-BBA default)
///
/// Exactly [`american`] but for the floor: the learned
/// [`NeuralFloorBba`][crate::bidding::neural_floor::NeuralFloorBba] gives way to
/// the deterministic [`instinct`][crate::bidding::instinct()] ladder.  This is the
/// fully-disclosable reference system — every off-book call is a described,
/// natural instinct call — and the fixed baseline the BBA-gap campaign anchors
/// on.  It is also the distillation teacher: the nets clone *this*, never the
/// net-floored [`american`].
#[must_use]
pub fn american_instinct() -> Pair {
    with_instinct_floor(bare_american())
}

/// The 2/1 pair with the **classic balanced** 1NT opening (pre-redesign)
///
/// Exactly [`american`] but for the opening table: the strong 1NT is only
/// the balanced patterns (4333/4432/5332), without the wide-shape redesign that
/// [`american`] now ships (a 5422 *or* 6322 with a long minor also opens 1NT —
/// [`openings_with`]).  The ablation handle for measuring that redesign; see the
/// `nt-shape-abc` (constructive) and `nt-shape-contested` examples.
#[must_use]
pub fn american_classic() -> Pair {
    with_instinct_floor(bare_american_with(NotrumpShape::Balanced))
}

/// The 2/1 pair with the **pre-6322 wide** 1NT shape ([`NotrumpShape::Wide`])
///
/// Exactly [`american`] but its 1NT opens only the balanced patterns plus a
/// 5422 with a five-card minor — *not* the 6322-with-a-six-card-minor that
/// [`american`] now ships.  The superseded baseline, retained as the ablation
/// handle for re-measuring the 6322 addition (`nt-shape-contested`, `bba-gen
/// --nt-shape wide`); the win that adopted 6322 was +0.004…0.006 IMPs/board
/// plain, confirmed over two seeds.
#[must_use]
pub fn american_wide() -> Pair {
    with_instinct_floor(bare_american_with(NotrumpShape::Wide))
}

/// The 2/1 pair with the distilled **neural** floor (AI-bidder M1.3)
///
/// Exactly [`american`] but for the floor: the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder is replaced by the
/// [`NeuralFloor`][crate::bidding::neural_floor::NeuralFloor] safety shell — the learned
/// net in the judgement middle, the forced rails preserved by delegation.  An
/// added option, never a replacement; [`american`] stays the baseline.  Bind
/// it against the opponents' [`Family`] with [`Pair::against`] and seat it the
/// same way.  Gated behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn american_neural() -> Pair {
    with_floor(bare_american(), super::neural_floor::NeuralFloor)
}

/// The 2/1 pair with the **tag-augmented** distilled neural floor (AI-bidder M5.1)
///
/// Exactly [`american_neural`] but for the floor's feature extractor: the net
/// also sees the WBF tags of the recent calls
/// ([`features_v2`][crate::bidding::features::features_v2]), wrapped in the same
/// [`NeuralFloorV2`][crate::bidding::neural_floor::NeuralFloorV2] safety shell —
/// the learned net in the judgement middle, the forced rails preserved by
/// delegation.  An added option, never a replacement: [`american`] stays the
/// baseline and [`american_neural`] the v1 learned floor.  Bind it against the
/// opponents' [`Family`] with [`Pair::against`] and seat it the same way.  Gated
/// behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn american_neural_v2() -> Pair {
    with_floor(bare_american(), super::neural_floor::NeuralFloorV2)
}

/// The 2/1 pair with the **restrictive disclosable** distilled neural floor (AI-bidder v3)
///
/// Exactly [`american_neural`] but for the floor's feature extractor: the net is
/// fed only the *disclosable* hand summary
/// ([`features_v3`][crate::bidding::features::features_v3]) — HCP, shape, and
/// per-suit length and quality, never specific cards — wrapped in the
/// [`NeuralFloorV3`][crate::bidding::neural_floor::NeuralFloorV3] safety shell
/// with the same forced-rail delegation.  It learned to clone [`american`] from
/// what a bidder could lawfully disclose to opponents (full disclosure being core
/// duplicate ethics), so it never keys on undisclosable card detail.  An added
/// option, never a replacement: [`american`] stays the baseline.  Bind it against
/// the opponents' [`Family`] with [`Pair::against`] and seat it the same way.
/// Gated behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn american_neural_v3() -> Pair {
    with_floor(bare_american(), super::neural_floor::NeuralFloorV3)
}

/// Alias of [`american`], retained for continuity
///
/// The BBA-distilled [`NeuralFloorBba`][crate::bidding::neural_floor::NeuralFloorBba]
/// floor this names *is* the [`american`] default as of the floor swap; kept as a
/// named handle for the `bba-gen` arms and older call sites.  For the deterministic
/// pre-swap system, use [`american_instinct`].
#[must_use]
pub fn american_bba_neural() -> Pair {
    american()
}

/// The 2/1 pair with the **search-target** distilled neural floor (AI-bidder M3.2)
///
/// Exactly [`american_neural`] in shape — v1 features, the
/// [`NeuralFloorSearch`][crate::bidding::neural_floor::NeuralFloorSearch] safety
/// shell with the same forced-rail delegation — but the net is distilled from the
/// **live-search teacher** (M2.3's EV-grounded targets, dumped at M3.1), *not* from
/// the deterministic [`american`] and *not* the live search bidder
/// [`american_search`] itself.  The fast net that learned the search's
/// judgement.  An added option, never a replacement: [`american`] stays the
/// baseline and [`american_neural`] the teacher-distilled floor.  Bind it
/// against the opponents' [`Family`] with [`Pair::against`] and seat it the same
/// way.  Gated behind the `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn american_neural_search() -> Pair {
    with_floor(bare_american(), super::neural_floor::NeuralFloorSearch)
}

/// The 2/1 pair with the gated live-**search** floor (AI-bidder M2.3)
///
/// Exactly [`american`] but for the floor: the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder is replaced by the
/// [`SearchFloor`][crate::bidding::search_floor::SearchFloor] safety shell, which
/// at each non-forced decision shortlists the distilled net's top calls and
/// scores them by cardplay EV before bidding the best — the policy *thinks*
/// before it bids.  Strong but slow; an added gated option, never a replacement.
/// Bind it against the opponents' [`Family`] with [`Pair::against`] and seat it
/// the same way.  Gated behind the `search` feature (which implies
/// `neural-floor`).
#[cfg(feature = "search")]
#[must_use]
pub fn american_search() -> Pair {
    american_search_with(super::search_floor::SearchFloor::default())
}

/// The 2/1 pair with a caller-tuned live-search floor (AI-bidder M3.1)
///
/// Like [`american_search`] but with explicit
/// [`SearchFloor`][crate::bidding::search_floor::SearchFloor] knobs, so
/// data-generation and tuning runs can trade strength for speed (smaller
/// `layouts`/`shortlist`) without re-wiring the floor.  `american_search()`
/// is exactly `american_search_with(SearchFloor::default())`.  Gated behind
/// the `search` feature.
#[cfg(feature = "search")]
#[must_use]
pub fn american_search_with(floor: super::search_floor::SearchFloor) -> Pair {
    with_floor(bare_american(), floor)
}

/// Build a 2/1 pair whose **constructive** book falls back to an arbitrary
/// classifier — the knob the standard constructors deny
///
/// [`american`] and the neural variants hard-wire the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder onto the constructive book —
/// the learned floors own only the contested books.  This
/// builder lifts that wiring so a learned floor (the live
/// [`SearchFloor`][crate::bidding::search_floor::SearchFloor] or the distilled
/// [`NeuralFloorSearch`][crate::bidding::neural_floor::NeuralFloorSearch]) can be
/// *measured* on uncontested constructive auctions against the instinct baseline
/// — the `constructive-abc` A/B/C example.  The contested books are left bare:
/// that harness silences the opponents, so they never resolve.  Gated behind the
/// `neural-floor` feature.
#[cfg(feature = "neural-floor")]
#[must_use]
pub fn american_constructive_floor<C: Classifier + 'static>(floor: C) -> Pair {
    let mut pair = bare_american();
    pair.constructive
        .fallback_at(&[], Always, Fallback::classify(floor));
    pair
}

/// Attach any classifier as the floor on a pair's contested books
///
/// A root `Always` fallback on both contested books, shared through the
/// `Fallback`'s `Arc`.  Resolution reaches the root last, so the floor never
/// overrides an authored rule — it only catches the auctions that fall past all
/// of them.  Generic over the floor so [`american`] (the deterministic
/// [`instinct`][crate::bidding::instinct()]) and
/// [`american_neural`] (the distilled net) share one wiring.
pub(in crate::bidding) fn with_floor<C: Classifier + 'static>(mut pair: Pair, floor: C) -> Pair {
    let floor = Fallback::classify(floor);
    pair.competitive.fallback_at(&[], Always, floor.clone());
    pair.defensive.fallback_at(&[], Always, floor);
    // Uncontested auctions never reach the contested floor, so an off-book
    // constructive sequence would pass out below a cold game (e.g. `1♦–1♥–1NT`
    // passed out on a balanced 16 opposite the 12–14 rebid).  Floor the
    // constructive book with the deterministic instinct ladder — the learned
    // floors are trained on contested auctions only, so the natural milestone
    // bidder is the right answer here — and those sequences reach game or slam.
    pair.constructive
        .fallback_at(&[], Always, Fallback::classify(instinct()));
    pair
}

/// Attach the deterministic instinct floor to a pair's contested books
pub(in crate::bidding) fn with_instinct_floor(pair: Pair) -> Pair {
    with_floor(pair, instinct())
}

/// Build the 2/1 pair *without* the instinct floor: the bare authored books
///
/// This is the ablation handle for measuring the floor.  A driver seating
/// this pair passes whenever the books run out — the pre-floor behavior,
/// including passing partner's takeout double on a worthless hand.
/// [`american()`] is exactly this pair with
/// [`instinct`][crate::bidding::instinct()] attached to both contested books;
/// see the `instinct-floor` example for an A/B match between the two.  The 1NT
/// [`NotrumpShape`] follows [`set_notrump_shape`] (default
/// [`NotrumpShape::Wide6322`] — a 5422 or 6322 with a long minor also opens 1NT);
/// [`american_wide`] and [`american_classic`] are the baked ablation baselines.
#[must_use]
pub fn bare_american() -> Pair {
    bare_american_with(openings::notrump_shape_setting())
}

/// [`bare_american`] with the 1NT [`NotrumpShape`] policy selectable
///
/// `shape` selects the opening table's 1NT shape ([`openings_with`]); everything
/// else is identical.  `bare_american()` ships [`NotrumpShape::Wide6322`]; the
/// pre-6322 [`NotrumpShape::Wide`] is behind [`american_wide`] and the classic
/// balanced baseline ([`NotrumpShape::Balanced`]) behind [`american_classic`].
#[must_use]
fn bare_american_with(shape: NotrumpShape) -> Pair {
    let mut c = Constructive::new();

    openings::register(&mut c, shape);
    responses::register(&mut c);
    notrump::register(&mut c);
    rebids::register(&mut c);
    xyz::register(&mut c);
    game_force::register(&mut c);
    raises::register(&mut c);
    strong_two::register(&mut c);
    weak_twos::register(&mut c);

    Pair::new(
        Family::NATURAL,
        c,
        competition::competition(),
        defense::defensive(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Rules;
    use crate::bidding::context::Context;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Hand, Suit};

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
    fn choice_of_games_three_notrump() {
        let a = [call(1, Strain::Hearts), Call::Pass];
        let on = major_responses(Suit::Hearts);
        set_major_choice_of_games(false);
        let off = major_responses(Suit::Hearts);
        set_major_choice_of_games(true);

        // Flat (4333) with four trumps, 13 HCP: 3NT outranks Jacoby 2NT.
        assert_eq!(best(&on, &a, "K32.KQ54.A96.J92"), call(3, Strain::Notrump));
        assert_eq!(best(&off, &a, "K32.KQ54.A96.J92"), call(2, Strain::Notrump));
        // Flat (4333) with three trumps, 12 HCP: 3NT; off it is a forcing 1NT.
        assert_eq!(best(&on, &a, "K32.K54.A964.Q92"), call(3, Strain::Notrump));
        assert_eq!(best(&off, &a, "K32.K54.A964.Q92"), call(1, Strain::Notrump));
        // 4=3=3=3 over 1♥ keeps bidding 1♠ — the spade exclusion is load-bearing.
        assert_eq!(best(&on, &a, "KQ32.K54.A96.Q92"), call(1, Strain::Spades));
        assert_eq!(best(&off, &a, "KQ32.K54.A96.Q92"), call(1, Strain::Spades));
    }

    #[test]
    fn two_over_one_fit_leg_and_gates() {
        let a = [call(1, Strain::Hearts), Call::Pass];
        // Arms are relative to the legacy gate; the shipped default is
        // fit + Hcp13 (restored at the end).
        set_two_over_one_fit(false);
        set_two_over_one_gate(TwoOverOneGate::Points13);
        let baseline = major_responses(Suit::Hearts);
        set_two_over_one_fit(true);
        let fit = major_responses(Suit::Hearts);
        set_two_over_one_fit(false);
        set_two_over_one_gate(TwoOverOneGate::Hcp13);
        let hcp13 = major_responses(Suit::Hearts);
        set_two_over_one_gate(TwoOverOneGate::Hcp12);
        let hcp12 = major_responses(Suit::Hearts);
        set_two_over_one_fit(true);
        set_two_over_one_gate(TwoOverOneGate::Hcp13);

        // Fit leg: exactly three trumps, 11 HCP + spade singleton reads 13
        // support points — a 2/1 preparing the heart raise; off, a 1NT.
        assert_eq!(best(&fit, &a, "7.K54.A964.KJ932"), call(2, Strain::Clubs));
        assert_eq!(
            best(&baseline, &a, "7.K54.A964.KJ932"),
            call(1, Strain::Notrump)
        );
        // Hcp13 demotes a shaped 12 (6-4 reads 13 points) back to 1NT.
        assert_eq!(
            best(&baseline, &a, "32.Q4.AKJ964.Q93"),
            call(2, Strain::Diamonds)
        );
        assert_eq!(
            best(&hcp13, &a, "32.Q4.AKJ964.Q93"),
            call(1, Strain::Notrump)
        );
        // Hcp12 admits a no-fit flat 12 the shipped gate leaves in 1NT.
        assert_eq!(
            best(&baseline, &a, "K32.54.A964.KQ92"),
            call(1, Strain::Notrump)
        );
        assert_eq!(best(&hcp12, &a, "K32.54.A964.KQ92"), call(2, Strain::Clubs));
    }

    #[test]
    fn notrump_responses_transfer_and_stayman() {
        let r = notrump_responses();
        let a = [call(1, Strain::Notrump), Call::Pass];
        assert_eq!(best(&r, &a, "KJ542.Q32.K43.92"), call(2, Strain::Hearts));
        // Four-four in the majors takes Stayman; a 4-3 hand would Puppet (3♣).
        assert_eq!(best(&r, &a, "KJ54.KQ32.43.Q92"), call(2, Strain::Clubs));
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
