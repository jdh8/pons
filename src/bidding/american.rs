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

use super::agreements::Agreements;
use super::common::{call, other_major, with_floor, with_floor_v5, with_instinct_floor};
use super::{Competitive, Constructive, Defensive, Pair};

pub(in crate::bidding) mod competition;
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
    set_jordan_truscott, set_lebensohl_style, set_major_support_double, set_natural_floor,
    set_negative_double_shape, set_penalty_double_leave_in, set_penalty_pass, set_redouble_answer,
    set_splinter_doubled, set_strong_two_competition, set_trap_pass, set_uvu, set_uvu_cue_floor,
    set_uvu_natural_floor, set_uvu_over_majors, set_uvu_x_floor, set_weak_two_competition,
};
// Knobs the inference walk reads at classify time.
pub use defense::{
    DoubleShape, NotrumpDefense, TakeoutSupport, advance_double, defense_to_suit,
    defense_to_weak_two, set_advance_2nt_continuation, set_advance_minor_jump,
    set_advance_pass_yield_major, set_advance_rubens, set_advance_sit_hcp_gate,
    set_advance_sohl_style, set_diamond_transfer_defense, set_direct_dont_four_four,
    set_direct_dont_one_suiter_min, set_direct_dont_x_floor, set_direct_landy_double,
    set_direct_landy_double_floor, set_direct_landy_penalty_pass, set_doubled_landy_escape,
    set_landy, set_landy_hcp, set_leaping_michaels, set_longest_first_advance,
    set_meckwell_minor_major_44, set_meckwell_x_floor, set_meckwell_x_four_four,
    set_minor_transfer_defense, set_natural_double_floor, set_natural_double_shape,
    set_natural_double_weight, set_natural_overcall_points, set_notrump_balancing,
    set_notrump_defense, set_nt_overcall_gladiator, set_nt_overcall_no_major,
    set_nt_overcall_systems_on, set_overcall_discipline, set_overcall_four_card,
    set_passed_hand_overcall, set_responsive_overcall, set_responsive_takeout,
    set_rich_advance_double, set_stayman_defense, set_stayman_defense_overcall,
    set_strong_double_hcp, set_takeout_support, set_transfer_defense,
    set_two_level_minor_overcall_tight, set_two_suiter_hcp_floor, set_unusual_notrump_defense,
    set_weak_two_cue, set_weak_two_jump_overcall, set_weak_two_notrump_advances,
    set_weak_two_notrump_points, set_weak_two_notrump_shape, set_weak_two_overcall_discipline,
    set_weak_two_overcall_points, set_weak_two_pass_gate, set_woolsey_double_floor,
    set_woolsey_points,
};
pub(crate) use defense::{
    direct_dont_enabled, landy_range, meckwell_enabled, natural_defense_enabled,
    natural_double_floor, natural_overcall_points, nt_overcall_systems_on, woolsey_double_floor,
    woolsey_enabled, woolsey_points,
};
pub use game_force::{set_game_backstop, set_opener_third, set_second_suit_agreement};
pub(crate) use nmf::new_minor_forcing;
pub use nmf::set_new_minor_forcing;
pub use notrump::{
    EUROPEAN, PUPPET, SizeAskEight, notrump_responses, set_crawling_stayman, set_garbage_stayman,
    set_invitational_5card_majors, set_long_minor_force, set_minor_min_to_3nt, set_notrump_minors,
    set_nt_splinter, set_nt_splinter_floor, set_sixcard_accept_floor, set_sixcard_invite_floor,
    set_size_ask_accept_floor, set_size_ask_eight, set_stayman_5card_max, set_stayman_both_majors,
    set_stayman_cue_continuation, set_stayman_minor_slam_try, set_stayman_net_force,
    set_texas_game_floor, set_texas_slam_drive, set_transfer_gf_hearts, set_transfer_gf_majors,
    set_transfer_longer_major, set_transfer_slam_try, set_transfer_super_accept,
};
pub(crate) use openings::notrump_shape;
pub(crate) use openings::one_notrump_offshape;
pub(crate) use openings::two_notrump_wide;
pub use openings::{
    NotrumpShape, WeakTwoEval, openings, openings_with, set_notrump_shape, set_one_notrump_fifths,
    set_one_notrump_offshape, set_open_one_notrump, set_two_notrump_wide, set_weak_two_eval,
    set_weak_two_hcp, set_weak_two_wild,
};
pub use weak_twos::{set_weak_two_longest_first, set_weak_two_major_priority};

pub use raises::{set_limit_raise_acceptance, set_major_game_tries};
pub(crate) use rebids::{opener_extras_ladder, opener_major_jump_rebid};
pub use rebids::{
    set_balanced_1nt_rebid, set_forcing_nt_two_suiter, set_fourth_suit_forcing,
    set_major_rebid_tails, set_meckstroth_adjunct, set_meckstroth_minor_jumps, set_nt_invite_hcp,
    set_opener_extras_ladder, set_opener_major_jump_rebid,
};
pub(crate) use responses::longer_major_response;
pub use responses::{
    TwoOverOneGate, major_responses, minor_responses, set_longer_major_response,
    set_major_choice_of_games, set_two_over_one_fit, set_two_over_one_gate,
    set_two_over_one_heart_light, set_two_over_one_major_discount,
    set_two_over_one_natural_lengths, set_up_the_line,
};
pub(crate) use xyz::xyz;
pub use xyz::{set_xyz, set_xyz_invite_judgment};

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the basic 2/1 game-forcing system as one side's [`Pair`]
///
/// Bind it with [`against`][Pair::against] for a playable system, and seat
/// two pairs with [`Table::of_pairs`][super::Table::of_pairs] for a full
/// table.
///
/// The contested books stand on
/// [`ConfiguredFloorV5`][crate::bidding::neural_floor::ConfiguredFloorV5] —
/// one artifact whose convention-regime input is both partnerships'
/// [`ConventionCard`][super::features::ConventionCard], **captured here, at build
/// time**, from whatever the `set_*` knobs say when this is called, in the
/// same expression that reads them for [`american_book`].  That is what keeps
/// regime and rules from disagreeing: an A/B arm builds its stance with its
/// own knobs armed, exactly as it already does for rule presence, and gets a
/// matching regime vector for free.  Opponents are modeled as playing our own
/// agreements, matching every other undeclared-opposition default in the
/// crate; a genuinely mixed table wants [`american_with_config`], which also
/// remains the card-input v4 floor's entry point.  The v5 floor became the
/// default 2026-08-08 on its gate A/B (+0.0353/+0.0262 plain DD per board at
/// none/both vul, PD wash — `docs/ai-bidder/card-manifold.md` §"The retrain,
/// measured").
///
/// ```
/// use pons::american;
/// use pons::bidding::System;
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = american().against();
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
    with_floor_v5(
        american_book(),
        super::features::CompactConfig::symmetric(&super::features::ConventionCard::capture(false)),
    )
}

/// [`american`] against a **declared** opponent — the mixed table
///
/// The two arms of an A/B *play each other*, so at every table one side
/// relocates its asks and the other does not.  That asymmetric cell is in the
/// v4 corpus, and this is how a harness reaches it: build each arm's card from
/// its own knob state, then hand each side both.
///
/// `config` is taken verbatim and the **book still comes from the live knobs**,
/// so set them to match — a card claiming an agreement the rules do not play is
/// a misdisclosure to the net, and nothing checks it.  [`american`] cannot make
/// that mistake (it reads regime and book from one knob state in one
/// expression); this entry point can, which is the price of declaring an
/// opponent the knobs cannot describe.
///
/// Since the 2026-08-08 default-floor swap this is also the only entry point
/// that still builds the 2/1 book over the card-input **v4** floor
/// ([`ConfiguredFloorBba`][crate::bidding::neural_floor::ConfiguredFloorBba]);
/// `american_with_config(Config::symmetric(&american_card()))` reproduces the
/// pre-swap [`american`] exactly.  A declared opponent on the *shipped* floor
/// wants [`american_with_card`] instead — an arm built here and compared
/// against one built by [`american`] measures the two nets, not the declaration.
#[must_use]
pub fn american_with_config(config: super::features::Config) -> Pair {
    with_floor(american_book(), config)
}

/// [`american`] against a **declared** opponent, on the shipped v5 floor
///
/// The v5 twin of [`american_with_config`], and a strictly narrower seam: only
/// `theirs` is declared, while our own half is captured from the live knobs in
/// the same expression as the book.  So unlike the v4 entry point this *cannot*
/// misdisclose our own side — the mistake it warns about is unavailable here —
/// and the only judgement left to the caller is what the opposition plays.
///
/// Build `theirs` with [`ConventionCard::capture`][super::features::ConventionCard::capture]
/// under their armed knobs when they are a pons book, or with
/// [`ConventionCard::from_card`][super::features::ConventionCard::from_card] when they
/// are a foreign engine and a card is all there is.  At our own defaults the two
/// agree (`projection_agrees_with_capture_at_defaults`), so declaring an
/// undeviating pons opponent reproduces [`american`] board for board — the
/// inertness gate for this channel.
#[must_use]
pub fn american_with_card(theirs: &super::features::ConventionCard) -> Pair {
    with_floor_v5(
        american_book(),
        super::features::CompactConfig::new(
            &super::features::ConventionCard::capture(false),
            theirs,
        ),
    )
}

/// Alias of [`american`] — the v5 floor is the default since 2026-08-08
///
/// This was the retrain candidate's entry point while the v5-vs-v4 gate A/B
/// ran (`docs/ai-bidder/card-manifold.md` §"The retrain, measured"); the gate
/// shipped it, so the name is kept only so harnesses and scripts written
/// against `--our-floor american-v5` keep meaning what they measured.
#[must_use]
pub fn american_v5() -> Pair {
    american()
}

/// The 2/1 pair with the deterministic **instinct** floor (the pre-BBA default)
///
/// Exactly [`american`] but for the floor: the learned
/// [`ConfiguredFloorBba`][crate::bidding::neural_floor::ConfiguredFloorBba]
/// gives way to
/// the deterministic [`instinct`][crate::bidding::instinct()] ladder.  This is the
/// fully-disclosable reference system — every off-book call is a described,
/// natural instinct call — and the fixed baseline the BBA-gap campaign anchors
/// on.  It is also the distillation teacher: the nets clone *this*, never the
/// net-floored [`american`].
#[must_use]
pub fn american_instinct() -> Pair {
    with_instinct_floor(american_book())
}

/// The 2/1 pair with **no authored book** — every call comes from the floor
///
/// Exactly [`american`] but for the books: all three are empty, so every
/// auction falls straight through to the same floor wiring [`american`] uses —
/// [`ConfiguredFloorV5`][crate::bidding::neural_floor::ConfiguredFloorV5] on
/// the contested books, the deterministic [`instinct`][crate::bidding::instinct()]
/// ladder on the constructive one.  The ablation handle that prices the
/// authored book: `american` − `american_floor` is what [`american_book`] is
/// worth.
///
/// The floor takes the **same** agreements [`american`] would, even though there
/// is no book behind it to play them: the ablation isolates the book only if the
/// floor's inputs are identical on both arms — which means this function has to
/// follow [`american`] onto every future floor, in the same commit.  It did not
/// follow the 2026-08-08 v5 swap, and for one commit `scripts/ab-book-value.sh`
/// priced the book plus a whole net swap (+0.0353 plain DD per board, 3.5× the
/// run's own CI).
///
/// Note it prices the book's *total* contribution.  An empty book also stops
/// projecting authored constraints into
/// [`Inferences`][crate::bidding::inference::Inferences], so the net's
/// `features_v3` inference block collapses to unknown — the measured gap is the
/// book as authored calls **and** as disclosure, not the calls alone.
#[must_use]
pub fn american_floor() -> Pair {
    with_floor_v5(
        Pair::new(Constructive::new(), Competitive::new(), Defensive::new()),
        super::features::CompactConfig::symmetric(&super::features::ConventionCard::capture(false)),
    )
}

/// Build the 2/1 pair as the **authored books alone**, with no floor
///
/// The book half of [`american`], and the ablation handle for measuring the
/// floor: a driver seating this pair passes whenever the books run out — the
/// pre-floor behavior, including passing partner's takeout double on a
/// worthless hand.  [`american`] is exactly this pair with the BBA-distilled
/// net attached to both contested books, and [`american_floor`] is the
/// complementary ablation (the floor alone, with no book at all); see the
/// `instinct-floor` example for an A/B match.
///
/// The 1NT [`NotrumpShape`] follows [`set_notrump_shape`] (default
/// [`NotrumpShape::Wide6322`] — a 5422 or 6322 with a long minor also opens
/// 1NT).
#[must_use]
pub fn american_book() -> Pair {
    let agreements = Agreements::current();
    let mut c = Constructive::new();

    openings::register(&mut c, &agreements);
    responses::register(&mut c, &agreements);
    notrump::register(&mut c, &agreements);
    rebids::register(&mut c, &agreements);
    xyz::register(&mut c, &agreements);
    game_force::register(&mut c, &agreements);
    raises::register(&mut c, &agreements);
    strong_two::register(&mut c, &agreements);
    weak_twos::register(&mut c, &agreements);

    Pair::new(
        c,
        competition::competition(&agreements),
        defense::defensive(&agreements),
    )
}

#[cfg(test)]
mod tests;
pub use competition::competition_over_diamond_transfer;
pub use competition::competition_over_minor_transfer;
pub use competition::competition_over_stayman;
pub use competition::cue_minor_raise_answer;
pub use competition::cue_raise_answer;
pub use competition::defense_to_2d_multi;
pub use competition::delayed_cue;
pub use competition::direct_3nt_stopper;
pub use competition::high_overcall_responses;
pub use competition::jordan_truscott;
pub use competition::lebensohl_style;
pub use competition::major_support_double;
pub use competition::negative_double_shape;
pub use competition::splinter_doubled;
pub use competition::uvu;
pub use competition::uvu_over_majors;
pub use defense::advance_rubens_enabled;
pub use defense::advance_sohl_style;
pub use defense::direct_dont_four_four;
pub use defense::leaping_michaels_enabled;
pub use defense::minor_transfer_defense_enabled;
pub use defense::notrump_defense;
pub use defense::nt_overcall_gladiator;
pub use defense::passed_hand_overcall;
pub use defense::responsive_takeout_enabled;
pub use defense::rich_advance_double_enabled;
pub use defense::stayman_defense_enabled;
pub use defense::transfer_defense_enabled;
pub use game_force::game_backstop_enabled;
pub use game_force::second_suit_agreement;
pub use notrump::crawling_stayman;
pub use notrump::garbage_stayman;
pub use notrump::invitational_5card_majors;
pub use notrump::notrump_minors;
pub use notrump::nt_splinter;
pub use notrump::stayman_5card_max;
pub use notrump::stayman_both_majors;
pub use notrump::stayman_cue_continuation;
pub use notrump::stayman_minor_slam_try;
pub(crate) use notrump::stayman_net_force;
pub use notrump::texas_slam_drive;
pub use notrump::transfer_gf_hearts;
pub use notrump::transfer_gf_majors;
pub use notrump::transfer_longer_major;
pub use notrump::transfer_slam_try;
pub use notrump::transfer_super_accept;
pub use openings::notrump_shape_setting;
pub use openings::open_one_notrump;
pub use raises::limit_raise_acceptance;
pub use rebids::fourth_suit_forcing;
pub use rebids::meckstroth_adjunct;
