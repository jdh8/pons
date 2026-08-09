//! What the partnership has agreed to play — the value a book is built from
//!
//! The knob layer's single source. Every `set_*` knob is one cell of this
//! value; a build captures it **once**
//! ([`Agreements::current`][crate::bidding::agreements::Agreements::current]) and threads
//! `&Agreements` to everything that reads a knob, instead of each reader
//! consulting the thread on its own.
//!
//! That join is the point. Four readers must agree about what we play —
//! [`american_book`][crate::bidding::american::american_book],
//! [`instinct`][crate::bidding::instinct()],
//! [`ConventionCard::capture`][crate::bidding::features::ConventionCard::capture]
//! and [`reading_profile`][crate::bidding::inference] — and until this value
//! existed nothing but call-site discipline made them. Two defects were paid
//! for by that gap: the forced rail froze into a process-wide `LazyLock` built
//! by whichever pair came first, and a card disclosed rows the rules were not
//! playing. Both become unrepresentable once one capture feeds all four.
//!
//! # Layout
//!
//! One field per area of the system — `competition`, `defense`, `notrump`,
//! `opening`, `response`, `rebid`, `game_force`, `instinct` — plus `decision`.
//! No cell appears twice; that is the "one cell, one home" invariant, and
//! `no_knob_lives_in_two_homes` enforces it by scanning this source.
//!
//! `decision` (crate-private until the cells are gone) holds the cells read
//! **per decision**, at classify time, rather than while the books are built.
//! It is split out only because it is the snapshot a
//! [`Stance`][crate::bidding::Stance] pins at
//! [`Pair::against`][crate::bidding::Pair::against], so a stance decides
//! identically on any thread; the eight build-time areas are baked into the
//! rules a build returns and need no such pin.  A cell read at *both* times
//! (there are 24) lives in `decision` and is read from there at build time too.

use super::american::{
    Competitive4333, DoubleShape, DoubleStyle, FreeBidStyle, LebensohlStyle, NegativeDoubleShape,
    NotrumpShape, SizeAskEight, TakeoutSupport, TwoOverOneGate, WeakTwoEval,
};
use super::context::DecisionProfile;

/// The competitive book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// (`free_bids_engaged`, the natural-floor pair) stay functions of the module
/// that owns them rather than becoming fields, so the "one cell, one home"
/// invariant survives the move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CompetitionKnobs {
    // --- competition/cue_raise.rs
    /// Answer partner's cue-raise of their major overcall
    pub cue_raise_answer: bool,
    /// Answer partner's cue-raise of their minor overcall
    pub cue_minor_raise_answer: bool,
    /// Bid (not merely recognize) the delayed cue — 2NT relay, then their suit
    pub delayed_cue: bool,
    // --- competition/free_bids.rs
    /// Author the free bids directly, rather than only as a negative-double outlet
    pub free_bids: bool,
    /// Minimum points/HCP for the 1-level free bids
    pub free_bid_floor: u8,
    /// Minimum HCP for the free `1NT`, decoupled from the suit floor
    pub free_1nt_floor: u8,
    /// Require a quality suit for a free bid
    pub free_bid_quality: bool,
    /// Whether a free bid is forcing, one-round forcing, or a transfer
    pub free_bid_style: FreeBidStyle,
    // --- competition/high_overcall.rs
    /// Author responses to our high-level overcalls
    pub high_overcall_responses: bool,
    // --- competition/lebensohl.rs
    /// Require a stopper for the direct `3NT` over their overcall
    pub direct_3nt_stopper: bool,
    /// `(hcp_floor, points_floor)` on responder's weak natural 2-level escape
    pub natural_floor: (u8, u8),
    /// Which Lebensohl package the competitive book carries
    pub lebensohl_style: LebensohlStyle,
    /// Read a `(2♦)` overcall of our `1NT` as a Multi
    pub defense_2d_multi: bool,
    // --- competition/negative_double.rs
    /// Which negative-double school the minor openings play
    pub negative_double_shape: NegativeDoubleShape,
    /// Author the Cachalot answers when the auction is contested
    pub cachalot_contested_x: bool,
    // --- competition/our_preempts.rs
    /// Author continuations when they contest our weak two
    pub weak_two_competition: bool,
    /// Author continuations when they contest our strong two
    pub strong_two_competition: bool,
    // --- competition/over_our_*.rs
    /// Author continuations when they contest our `2NT` diamond transfer
    pub competition_over_diamond_transfer: bool,
    /// Author continuations when they contest our Jacoby transfer
    pub competition_over_transfer: bool,
    /// Author continuations when they contest our `2♠` minor transfer
    pub competition_over_minor_transfer: bool,
    /// Author continuations when they contest our Stayman
    pub competition_over_stayman: bool,
    // --- competition/over_their_double.rs
    /// Jordan/Truscott `2NT` over their takeout double
    pub jordan_truscott: bool,
    /// Author answers to partner's redouble
    pub redouble_answer: bool,
    /// Rebase to systems-on when they double our splinter
    pub splinter_doubled: bool,
    // --- competition/penalty_double.rs
    /// Whether a double of their overcall is takeout, optional, or penalty
    pub double_style: DoubleStyle,
    /// Opener may leave in responder's penalty double
    pub penalty_double_leave_in: bool,
    /// `(min_len, max_len, hcp_floor)` override on responder's penalty double
    pub double_override: Option<(usize, usize, u8)>,
    /// `(min_club_len, min_club_hcp, convert_over_major)` on the stolen-Stayman pass
    pub penalty_pass: Option<(usize, u8, bool)>,
    /// Author the trap pass
    pub trap_pass: bool,
    // --- competition/rubensohl.rs
    /// How a flat 4-3-3-3 cue-Staymans when our `1NT` is overcalled
    pub competitive_4333: Competitive4333,
    // --- competition/support_double.rs
    /// Support doubles/redoubles for the majors
    pub major_support_double: bool,
    // --- competition/two_suiters.rs
    /// Unusual-vs-unusual over their two-suiter showing both majors
    pub uvu_over_majors: bool,
    // --- competition/uvu.rs
    /// Author unusual-vs-unusual at all
    pub uvu: bool,
    /// HCP floor on the unusual-vs-unusual double
    pub uvu_x_floor: u8,
    /// HCP floor on the unusual-vs-unusual cue
    pub uvu_cue_floor: u8,
    /// HCP floor on the natural call over their two-suiter
    pub uvu_natural_floor: u8,
}

impl Default for CompetitionKnobs {
    fn default() -> Self {
        Self {
            cue_raise_answer: true,
            cue_minor_raise_answer: true,
            delayed_cue: false,
            free_bids: false,
            free_bid_floor: 6,
            free_1nt_floor: 6,
            free_bid_quality: false,
            free_bid_style: FreeBidStyle::Forcing,
            high_overcall_responses: false,
            direct_3nt_stopper: true,
            natural_floor: (5, 0),
            lebensohl_style: LebensohlStyle::Transfer,
            defense_2d_multi: false,
            negative_double_shape: NegativeDoubleShape::Modern,
            cachalot_contested_x: true,
            weak_two_competition: false,
            strong_two_competition: true,
            competition_over_diamond_transfer: true,
            competition_over_transfer: false,
            competition_over_minor_transfer: true,
            competition_over_stayman: true,
            jordan_truscott: true,
            redouble_answer: true,
            splinter_doubled: true,
            double_style: DoubleStyle::Optional,
            penalty_double_leave_in: true,
            double_override: None,
            penalty_pass: Some((4, 4, true)),
            trap_pass: true,
            competitive_4333: Competitive4333::Suppress,
            major_support_double: true,
            uvu_over_majors: true,
            uvu: true,
            uvu_x_floor: 9,
            uvu_cue_floor: 8,
            uvu_natural_floor: 6,
        }
    }
}

impl CompetitionKnobs {
    /// Capture this thread's competitive build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::competition::capture()
    }
}

/// The defensive book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DefenseKnobs {
    // --- defense.rs
    /// Prefer the longest suit when advancing partner's takeout double
    pub longest_first_advance_enabled: bool,
    /// Let a weak penalty pass yield to a four-card unbid major
    pub advance_pass_yield_major_enabled: bool,
    // --- defense/overcall.rs
    /// Shape gate for the natural penalty double of their `1NT`
    pub natural_double_shape: DoubleShape,
    /// Logit weight of the natural penalty double of their `1NT`
    pub natural_double_weight: i16,
    /// Support gate on the takeout double's 12+ tier
    pub takeout_support: TakeoutSupport,
    /// Use disciplined strength bands for natural suit overcalls
    pub overcall_discipline: bool,
    /// Allow a good four-card natural overcall
    pub overcall_four_card: bool,
    /// Let a passed hand make the disciplined two-level overcall lighter
    pub passed_hand_overcall: bool,
    /// Demand extra strength for a two-level minor overcall
    pub two_level_minor_overcall_tight: bool,
    /// Bar a five-card major from the natural `1NT` overcall
    pub nt_overcall_no_major: bool,
    /// Optional HCP seam between natural overcalls and the strong double
    pub strong_double_hcp: Option<u8>,
    // --- defense/nt_dont.rs
    /// Raw minimum length cell for direct DONT's one-suiter
    pub direct_dont_one_suiter_min: u8,
    /// Allow four-four two-suiters in direct DONT
    pub direct_dont_four_four: bool,
    /// Raw points-floor cell for direct DONT's double
    pub direct_dont_x_floor: u8,
    // --- defense/nt_woolsey.rs
    // --- defense/weak_two_nt_advance.rs
    /// Author advances of our `2NT` overcall of their weak two
    pub weak_two_notrump_advances_enabled: bool,
    // --- defense/advance_minor_jump.rs
    /// Author invitational minor jumps after partner's takeout double
    pub advance_minor_jump_enabled: bool,
    // --- defense/nt_defense.rs
    /// Extend the notrump defense to the balancing seat
    pub notrump_balancing_enabled: bool,
    // --- defense/leaping_michaels.rs
    /// Author Leaping Michaels over their weak two
    pub leaping_michaels_enabled: bool,
    // --- defense/weak_two_defense.rs
    /// Author the weak-two pass as the complement of stronger actions
    pub weak_two_pass_gate: bool,
    /// Require the `2NT` overcall to have the wide-notrump shape
    pub weak_two_notrump_shape: bool,
    /// Author jump overcalls over their weak two
    pub weak_two_jump_overcall: bool,
    /// Use disciplined bands for suit overcalls of their weak two
    pub weak_two_overcall_discipline: bool,
    /// Author the natural cue-bid over their weak two
    pub weak_two_cue: bool,
    /// Inclusive points band for the `2NT` overcall of their weak two
    pub weak_two_notrump_points: (u8, u8),
    /// Points bands for two- and three-level overcalls of their weak two
    pub weak_two_overcall_points: (u8, u8, u8, u8),
    // --- defense/advance_rubens.rs
    /// Author Rubens advances of partner's takeout double
    pub advance_rubens_enabled: bool,
    // --- defense/nt_landy.rs
    /// Escape thresholds after their double of Landy `2♣`
    pub doubled_landy_escape: (usize, usize),
    /// Gauge the Landy band in HCP rather than points
    pub landy_use_hcp: bool,
    /// Raw four-four-shape cell for direct Landy's double
    pub direct_landy_four_four: bool,
    /// Points floor for direct Landy's double
    pub direct_landy_double_floor: u8,
    /// Author the direct-Landy penalty pass
    pub direct_landy_penalty_pass: bool,
    // --- defense/michaels.rs
    /// Optional strength band for the unusual `2NT`
    pub unusual_notrump_range: Option<(u8, u8)>,
    /// Optional HCP floor for defensive two-suiters
    pub two_suiter_hcp_floor: Option<u8>,
    // --- defense/advance_sohl.rs
    /// Which sohl advance structure partner's takeout double uses
    pub advance_sohl_style: LebensohlStyle,
    // --- defense/nt_meckwell.rs
    /// Allow four-four in Meckwell's minor-major calls
    pub meckwell_minor_major_44: bool,
    /// Allow a four-four two-suiter in Meckwell's double
    pub meckwell_x_four_four: bool,
    /// Raw points floor cell for Meckwell's double
    pub meckwell_x_floor: u8,
    // --- defense/advance_2nt.rs
    /// Author the continuation after advancer's invitational `2NT`
    pub advance_2nt_continuation_enabled: bool,
    // --- defense/nt_their_conventions.rs
    /// Defend their Stayman convention
    pub stayman_defense_enabled: bool,
    /// Shape and strength floor for the natural call over their Stayman
    pub stayman_defense_overcall: (usize, u8),
    /// Defend their major-suit transfers
    pub transfer_defense_enabled: bool,
    /// Defend their minor-suit transfer
    pub minor_transfer_defense_enabled: bool,
    /// Defend their diamond transfer
    pub diamond_transfer_defense_enabled: bool,
    // --- defense/gladiator.rs
    // --- defense/advance_rich.rs
    /// Author the rich advance of partner's takeout double
    pub rich_advance_double_enabled: bool,
    /// Optional HCP gate on advancer's penalty pass
    pub advance_sit_hcp_gate: Option<u8>,
    // --- defense/responsive.rs
    /// Author responsive doubles after partner's takeout double
    pub responsive_takeout_enabled: bool,
    /// Author responsive doubles after partner's natural overcall
    pub responsive_overcall_enabled: bool,
}

impl Default for DefenseKnobs {
    fn default() -> Self {
        Self {
            longest_first_advance_enabled: true,
            advance_pass_yield_major_enabled: false,
            natural_double_shape: DoubleShape::Balanced,
            natural_double_weight: 130,
            takeout_support: TakeoutSupport::Strict,
            overcall_discipline: true,
            overcall_four_card: false,
            passed_hand_overcall: true,
            two_level_minor_overcall_tight: false,
            nt_overcall_no_major: false,
            strong_double_hcp: Some(18),
            direct_dont_one_suiter_min: 5,
            direct_dont_four_four: true,
            direct_dont_x_floor: 0,
            weak_two_notrump_advances_enabled: false,
            advance_minor_jump_enabled: true,
            notrump_balancing_enabled: false,
            leaping_michaels_enabled: true,
            weak_two_pass_gate: false,
            weak_two_notrump_shape: false,
            weak_two_jump_overcall: false,
            weak_two_overcall_discipline: true,
            weak_two_cue: false,
            weak_two_notrump_points: (16, 17),
            weak_two_overcall_points: (10, 16, 10, 16),
            advance_rubens_enabled: false,
            doubled_landy_escape: (6, 2),
            landy_use_hcp: false,
            direct_landy_four_four: false,
            direct_landy_double_floor: 15,
            direct_landy_penalty_pass: false,
            unusual_notrump_range: Some((8, 13)),
            two_suiter_hcp_floor: Some(8),
            advance_sohl_style: LebensohlStyle::Transfer,
            meckwell_minor_major_44: false,
            meckwell_x_four_four: true,
            meckwell_x_floor: 0,
            advance_2nt_continuation_enabled: true,
            stayman_defense_enabled: false,
            stayman_defense_overcall: (6, 14),
            transfer_defense_enabled: false,
            minor_transfer_defense_enabled: false,
            diamond_transfer_defense_enabled: false,
            rich_advance_double_enabled: true,
            advance_sit_hcp_gate: None,
            responsive_takeout_enabled: true,
            responsive_overcall_enabled: false,
        }
    }
}

impl DefenseKnobs {
    /// Capture this thread's defensive build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::defense::capture()
    }
}

/// The notrump book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move. The three cells also
/// read at classify time live only in `DecisionProfile`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NotrumpKnobs {
    // --- notrump.rs
    // --- notrump/size_ask.rs
    /// How a balanced eight with no four-card major handles the size ask
    pub size_ask_eight: super::american::SizeAskEight,
    /// Opener's HCP floor for accepting the balanced-eight size ask
    pub size_ask_accept_floor: u8,
    // --- notrump/both_majors.rs
    /// Show both four-card majors in response to Stayman
    pub stayman_both_majors: bool,
    /// Show a five-card major when answering Stayman with a maximum
    pub stayman_5card_max: bool,
    // --- notrump/transfer_gf.rs
    /// Route minimum game-forcing minor side suits directly to `3NT`
    pub minor_min_to_3nt: bool,
    // --- notrump/transfers.rs
    /// Author super-accepts of Jacoby transfers
    pub transfer_super_accept: bool,
    /// Prefer the longer major when both majors can transfer
    pub transfer_longer_major: bool,
    // --- notrump/crawling_stayman.rs
    // --- notrump/sixcard_invitation.rs
    /// Raw strength floor for inviting with a six-card major
    pub sixcard_invite_floor: u8,
    /// Raw strength floor for accepting a six-card-major invitation
    pub sixcard_accept_floor: u8,
    // --- notrump/transfer_slam.rs
    /// Author the transfer slam-try structure
    pub transfer_slam_try: bool,
    // --- notrump/invitational_majors.rs
    /// Author the invitational five-card-major structure
    pub invitational_5card_majors: bool,
    // --- notrump/texas.rs
    /// Route strong Texas hands through the slam-drive continuations
    pub texas_slam_drive: bool,
    /// Raw strength floor for the Texas game transfer
    pub texas_game_floor: u8,
    // --- notrump/stayman.rs
    // --- notrump/splinter.rs
    /// Responder's HCP floor for the `1NT` splinter
    pub nt_splinter_floor: u8,
    // --- notrump/stayman_slam.rs
    /// Author the Stayman cue-bid continuation
    pub stayman_cue_continuation: bool,
    /// Author the Stayman minor-slam try
    pub stayman_minor_slam_try: bool,
    // --- notrump/long_minor.rs
    /// Author the source-of-tricks-eight long-minor force
    pub long_minor_force: bool,
}

impl Default for NotrumpKnobs {
    fn default() -> Self {
        Self {
            size_ask_eight: SizeAskEight::Shipped,
            size_ask_accept_floor: 16,
            stayman_both_majors: true,
            stayman_5card_max: true,
            minor_min_to_3nt: false,
            transfer_super_accept: false,
            transfer_longer_major: true,
            sixcard_invite_floor: 13,
            sixcard_accept_floor: 18,
            transfer_slam_try: true,
            invitational_5card_majors: true,
            texas_slam_drive: true,
            texas_game_floor: 14,
            nt_splinter_floor: 9,
            stayman_cue_continuation: true,
            stayman_minor_slam_try: true,
            long_minor_force: false,
        }
    }
}

impl NotrumpKnobs {
    /// Capture this thread's notrump build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::notrump::capture()
    }
}

/// The opening book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.  `two_notrump_wide` is
/// read at classify time as well and so lives only in `DecisionProfile`,
/// deliberately absent here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct OpeningKnobs {
    // --- openings/one_notrump.rs
    /// Open our strong `1NT` at all — **default on**
    ///
    /// Off, a strong balanced 15-17 opens a minor instead, so a diagnostic can
    /// isolate our *defense* to an opponent's `1NT` without our own `1NT`
    /// openings polluting the duplicate (`bba-match --no-our-1nt`).
    pub open_one_notrump: bool,
    /// Gauge the `1NT` range in Andrews' fifths rather than plain HCP
    ///
    /// **Default off.**  On restores the legacy `fifths(14.5..17.5)`, centre-matched
    /// to plain HCP 15-17; the shipped plain-HCP gauge opens `1NT` a touch more
    /// often.
    pub one_notrump_fifths: bool,
    /// Which balanced shapes the strong `1NT` opening admits
    ///
    /// **Default [`NotrumpShape::Wide6322`].**  The web Settings shape radio.
    pub notrump_shape: NotrumpShape,
    /// Admit the off-shape `1NT` (a singleton honour in 4441/5431)
    ///
    /// **Default off** (byte-identical); on also admits 5422.  Read by the
    /// generated convention card as well as by the rules.
    pub one_notrump_offshape: bool,
    // --- openings/weak_two.rs
    /// Optional raw-HCP band gauging the weak-two opening
    ///
    /// **Default `None`** — the shipped rule-of-N+8 `points(5..=10)`.  `Some`
    /// gauges `lo..=hi` in raw HCP instead.  The opening is *fit-unknown*, so a
    /// preempt's length is already pinned by the six-card requirement and
    /// gauging its *strength* in shape-crediting `points` double-counts that
    /// length: a six-card suit reads `+max(0, L2−8)`, +0 on 6-2-2-3 up to +2 on
    /// 6-4-2-1, so no single `points` shift restores a clean cutoff.  Only the
    /// fit-unknown *opening* moves — the Ogust min/max answers stay on `points`
    /// because responder's `2NT` promises support, mirroring the 2/1 gate's
    /// hcp/support-points fit-split.
    ///
    /// **Rejected default-on**: `hcp(5..=10)` measured a wash on the sd-lead
    /// scorer (−0.0045 NV / −0.0018 vul, CIs span 0) — a weak two is a preempt,
    /// and the plain-DD remnant the point-count campaign priced here is the
    /// obstruction/disclosure wall, not a fixable gauge.  A major-only carve
    /// measured strictly worse (sd-vul −0.0113).  Retained as a single-dummy
    /// re-measure candidate (`docs/point-count-threshold-campaign.md`).
    pub weak_two_hcp: Option<(u8, u8)>,
    /// Optional honour-location evaluator gauging the weak-two opening
    ///
    /// **Default `None`**; wins over [`weak_two_hcp`][Self::weak_two_hcp] when
    /// both are armed.  Tests the follow-up to the rejected raw-HCP re-gauge:
    /// that *where the honours sit* — concentrated in the six-card suit versus
    /// scattered through the short suits — separates the weak twos worth their
    /// disclosure from the rest.  Like the HCP knob, only the fit-unknown
    /// opening moves.
    pub weak_two_eval: Option<WeakTwoEval>,
    /// Open wild weak twos (five- or six-card suit, `points(3..=12)`)
    ///
    /// **Default off** (byte-identical).
    pub weak_two_wild: bool,
    // --- weak_twos.rs
    /// Prefer a good five-card major to the Ogust ask over a weak `2♦`
    ///
    /// **Default on.**  The old node ranked Ogust `2NT` (weight 2.0) above every
    /// new suit (1.5), so ♠AKT862 ♥AKJ92 ♦xx asked about diamond quality rather
    /// than showing eleven cards in the majors — the major only escaped when
    /// responder was short enough in diamonds to fail Ogust's `support(2..)`.
    /// On, the two major rules outrank Ogust **over `2♦` only**: there `2♥`/`2♠`
    /// is *cheaper* than `2NT` and forcing, so the ask is deferred rather than
    /// lost.  Over `2♥`/`2♠` a new suit costs `2♠` or the three level, so Ogust
    /// keeps priority.
    ///
    /// Expressed as a weight, not as a gate on the Ogust rule: `top_honors` is
    /// part of the new-suit gate, so excluding "any five-card major" from Ogust
    /// would strand 14+ hands like ♠QJxxx ♦xx, which have no new-suit rule at
    /// all.  The cost is that `2NT`'s projected reading still promises only
    /// "14+, 2+♦" and does not deny a major.
    ///
    /// Default-on on measurement: the enriched probe
    /// (`examples/probe-weak-two-major --mode ogust`, 20 000 accepted deals,
    /// 0.069% trigger density, 85% divergent) scored **+3.048 IMPs/accepted
    /// deal** under perfect defense, CI [+2.911, +3.184], and **+1.668** under
    /// plain DD, CI [+1.563, +1.773] — ≈+0.0021 IMPs/board.
    pub weak_two_major_priority: bool,
    /// Answer partner's weak two with the longest suit first
    ///
    /// **Default on**; off restores the pre-repair argmax race.  The new-suit
    /// rules all carry weight 1.5, so before the repair the winner was decided
    /// by `Table::next_call`'s tie-break — descending sort, first *legal* call,
    /// i.e. the **cheapest** bid.  ♠AKT862 ♥AKJ92 therefore responded `2♥` to a
    /// weak `2♦`, suppressing the longer and higher suit.  On, each rule gains
    /// `longest_unbid`, making the choice a constraint: longest first, an
    /// equal-length tie to the higher rank.  Same doctrine as
    /// [`longest_first_advance`][DefenseKnobs::longest_first_advance_enabled] on the
    /// advance side, and like it the condition is a `shapes` partition, so the
    /// *reading* also pins the relative length.
    ///
    /// Default-on as a doctrine repair, not a measured win: the collision needs
    /// two qualifying five-card suits opposite a weak two, roughly one board in
    /// 10⁴, so a random-deal A/B cannot resolve it.  The knob exists to ablate
    /// it in the enriched probe (`examples/probe-weak-two-major --mode tie`).
    pub weak_two_longest_first: bool,
}

impl Default for OpeningKnobs {
    fn default() -> Self {
        Self {
            open_one_notrump: true,
            one_notrump_fifths: false,
            notrump_shape: NotrumpShape::Wide6322,
            one_notrump_offshape: false,
            weak_two_hcp: None,
            weak_two_eval: None,
            weak_two_wild: false,
            weak_two_major_priority: true,
            weak_two_longest_first: true,
        }
    }
}

/// The response and raise books' build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.
/// `longer_major_response` is read at classify time as well — the M6.4
/// control-bid classifier reads the same discipline the response rule authors —
/// and so lives only in `DecisionProfile`, deliberately absent here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResponseKnobs {
    // --- responses/two_over_one.rs
    /// Author the fit leg of the major 2/1 game force
    pub two_over_one_fit: bool,
    /// The gauge for the no-fit leg of the major 2/1 game force
    pub two_over_one_gate: TwoOverOneGate,
    /// Name natural per-call suit lengths in the major 2/1 game force
    pub two_over_one_natural_lengths: bool,
    /// Force game one HCP light on `1♠ - 2♥`
    pub two_over_one_major_discount: bool,
    /// Force game on a flat twelve with five hearts on `1♠ - 2♥`
    pub two_over_one_heart_light: bool,
    // --- responses/longer_major.rs
    /// Complete the natural minor tree up the line
    pub up_the_line: bool,
    // --- responses/choice_of_games.rs
    /// Author `1M - 3NT` as a choice of games
    pub major_choice_of_games: bool,
    // --- raises/game_try.rs
    /// Author the long-suit and general game tries after `1M - 2M`
    pub major_game_tries: bool,
    // --- raises/limit_raise.rs
    /// Author opener's acceptance ladder after `1M - 3M`
    pub limit_raise_acceptance: bool,
}

impl Default for ResponseKnobs {
    fn default() -> Self {
        Self {
            two_over_one_fit: true,
            two_over_one_gate: TwoOverOneGate::Points13,
            two_over_one_natural_lengths: false,
            two_over_one_major_discount: false,
            two_over_one_heart_light: false,
            up_the_line: true,
            major_choice_of_games: true,
            major_game_tries: true,
            limit_raise_acceptance: true,
        }
    }
}

impl ResponseKnobs {
    /// Capture this thread's response build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::responses::capture()
    }
}

/// The rebid book's build-time knobs
///
/// Each field is one cell, named for the getter it replaces; *derived* readings
/// stay functions of the module that owns them rather than becoming fields, so
/// the "one cell, one home" invariant survives the move.  Three of the area's
/// twelve cells are read at classify time as well — `opener_extras_ladder`,
/// `opener_major_jump_rebid` and `xyz` — and so live only in `DecisionProfile`,
/// deliberately absent here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RebidKnobs {
    // --- rebids.rs
    /// Rebid `1NT` rather than a natural `2m` on a balanced 12-14
    pub balanced_1nt_rebid: bool,
    // --- rebids/major_tails.rs
    /// Author the full continuations after `1♥ - 1♠`
    pub major_rebid_tails: bool,
    /// Author fourth-suit forcing in the `1♥ - 1♠` tail
    pub fourth_suit_forcing: bool,
    /// Gauge responder's notrump invitation in raw HCP
    pub nt_invite_hcp: bool,
    // --- rebids/meckstroth.rs
    /// Author the complete Meckstroth adjunct
    pub meckstroth_adjunct: bool,
    /// Author the adjunct's invitational `3m` jumps
    pub meckstroth_minor_jumps: bool,
    // --- rebids/two_suiter.rs
    /// Author opener's two-suiter rebids over the forcing `1NT`
    pub forcing_nt_two_suiter: bool,
    // --- xyz.rs
    /// Let opener judge the checkback invitation rather than falling to the floor
    pub xyz_invite_judgment: bool,
    // --- nmf.rs
    /// Author New Minor Forcing on the four `1m - 1M - 1NT` slots
    pub new_minor_forcing: bool,
}

impl Default for RebidKnobs {
    fn default() -> Self {
        Self {
            balanced_1nt_rebid: true,
            major_rebid_tails: true,
            fourth_suit_forcing: true,
            nt_invite_hcp: true,
            meckstroth_adjunct: true,
            meckstroth_minor_jumps: true,
            forcing_nt_two_suiter: true,
            xyz_invite_judgment: true,
            new_minor_forcing: false,
        }
    }
}

impl RebidKnobs {
    /// Capture this thread's rebid build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::rebids::capture()
    }
}

/// The game-forcing book's build-time knobs
///
/// All three gate a *whole table* past round two of a two-over-one auction, and
/// all three trade against the floor rather than against another agreement: off
/// means the node falls through, not that a different rule fires.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameForceKnobs {
    // --- game_force/backstop.rs
    /// Re-register the retired wildcard game backstop over uncovered nodes
    pub game_backstop: bool,
    // --- game_force/opener_third.rs
    /// Author opener's third call after responder sets trump at `1M - 2r - R - 3M`
    pub opener_third: bool,
    // --- game_force/second_suit.rs
    /// Author opener's third call after responder raises opener's second suit
    pub second_suit_agreement: bool,
}

impl Default for GameForceKnobs {
    fn default() -> Self {
        Self {
            game_backstop: false,
            opener_third: true,
            second_suit_agreement: true,
        }
    }
}

impl GameForceKnobs {
    /// Capture this thread's game-forcing build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::game_force::capture()
    }
}

/// The deterministic floor's build-time knobs
///
/// The floor's *other* knobs are read while it classifies and live in
/// `InstinctProfile` alongside the rest of the decision half; these three are
/// read inside [`instinct`][crate::bidding::instinct()]'s table builder, so
/// they are baked into the ladder that comes back.  Membership is decided by
/// where the cell is read, not by what it configures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InstinctKnobs {
    /// Author the competitive long-suit rebid
    pub competitive_rebid: bool,
    /// Author opener's balanced-18-19 notrump actions in a contested auction
    pub reopening_notrump: bool,
    /// Author the doubler's runout after their redoubled penalty double
    pub doubler_xx_runout: bool,
}

impl Default for InstinctKnobs {
    fn default() -> Self {
        Self {
            competitive_rebid: true,
            reopening_notrump: true,
            doubler_xx_runout: true,
        }
    }
}

impl InstinctKnobs {
    /// Capture this thread's floor build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::instinct::capture_build()
    }
}

/// Everything the partnership has agreed to play
///
/// Constructed once per build and threaded down by reference. Cloning is cheap
/// but pointless: the whole design is that one capture serves a whole build.
///
/// One field per area of the system, plus `decision` for the cells read while
/// classifying rather than while building.  That last split is by *when* a
/// value is read, not by what it means — a build-time area and `decision` are
/// equally "what we agreed" — so it buys the `Stance` a small `Copy` snapshot
/// to pin and nothing else.
#[derive(Clone, Copy, PartialEq)]
pub struct Agreements {
    /// The classify-time cells, pinned into the stance at `Pair::against`
    pub(crate) decision: DecisionProfile,
    /// What we play when they contest our auction
    pub competition: CompetitionKnobs,
    /// What we play when they open the auction
    pub defense: DefenseKnobs,
    /// What we play after our notrump openings and rebids
    pub notrump: NotrumpKnobs,
    /// What we open, and how partner answers a weak two
    pub opening: OpeningKnobs,
    /// How partner answers our one-level opening, and how the raises continue
    pub response: ResponseKnobs,
    /// What opener rebids, and the checkback machinery over it
    pub rebid: RebidKnobs,
    /// Which continuations the game-forcing auction authors past round two
    pub game_force: GameForceKnobs,
    /// Which rules the deterministic floor's ladder authors
    pub instinct: InstinctKnobs,
}

impl Default for Agreements {
    /// The shipped system — what `american_default()` plays
    ///
    /// Equal to [`Agreements::current`] on a virgin thread
    /// (`build_defaults_match_the_cells`, `decision_defaults_match_the_cells`),
    /// which is what lets the cells be deleted without moving a bid.
    fn default() -> Self {
        Self {
            decision: DecisionProfile::default(),
            competition: CompetitionKnobs::default(),
            defense: DefenseKnobs::default(),
            notrump: NotrumpKnobs::default(),
            opening: OpeningKnobs::default(),
            response: ResponseKnobs::default(),
            rebid: RebidKnobs::default(),
            game_force: GameForceKnobs::default(),
            instinct: InstinctKnobs::default(),
        }
    }
}

impl Agreements {
    /// Capture this thread's knob state — the one read a build performs
    ///
    /// Every knob getter consulted here is consulted *here only*, so the
    /// readers downstream cannot disagree about what we play.
    #[must_use]
    pub fn current() -> Self {
        Self {
            decision: DecisionProfile::current(),
            competition: super::american::competition::capture(),
            defense: super::american::defense::capture(),
            notrump: super::american::notrump::capture(),
            opening: OpeningKnobs::default(),
            response: super::american::responses::capture(),
            rebid: super::american::rebids::capture(),
            game_force: super::american::game_force::capture(),
            instinct: super::instinct::capture_build(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Agreements;

    /// The literal defaults equal what a virgin thread's cells hold
    ///
    /// The safety net for deleting the cells: `Agreements::default()`
    /// transcribes 218 `Cell::new` initialisers into one value, and a
    /// transcription error would silently ship a different system.  libtest
    /// gives every test its own thread, so `current()` here reads cells nothing
    /// has armed.
    #[test]
    fn build_defaults_match_the_cells() {
        let (d, c) = (Agreements::default(), Agreements::current());
        assert_eq!(d.competition, c.competition);
        assert_eq!(d.defense, c.defense);
        assert_eq!(d.notrump, c.notrump);
        assert_eq!(d.opening, c.opening);
        assert_eq!(d.response, c.response);
        assert_eq!(d.rebid, c.rebid);
        assert_eq!(d.game_force, c.game_force);
        assert_eq!(d.instinct, c.instinct);
        assert!(d.decision == c.decision, "the classify half diverged");
        assert!(d == c);
    }

    /// The `pub`-ish field names of `struct name` in `src`
    ///
    /// A crude line scanner, not a parser: every knob struct in this crate is
    /// one field per line with a leading visibility, which is all this needs to
    /// see.  Field names are invisible to the type system, so a source scan is
    /// the only mechanism that can check the invariant below at all.
    fn fields(src: &str, name: &str) -> Vec<String> {
        let body = src
            .split_once(&format!("struct {name} {{"))
            .unwrap_or_else(|| panic!("{name} is declared"))
            .1
            .split_once("\n}")
            .expect("the struct body is closed")
            .0;
        body.lines()
            .filter_map(|line| {
                let line = line.trim_start().strip_prefix("pub")?;
                let line = line.strip_prefix(')').map_or(line, |rest| rest);
                let line = line.trim_start_matches(|c| c != ' ').trim_start();
                let (ident, _) = line.split_once(':')?;
                ident
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                    .then(|| ident.to_owned())
            })
            .collect()
    }

    /// One cell, one home: no knob is a field of both halves of `Agreements`
    ///
    /// A cell read at build time *and* at classify time must live in the
    /// classify-time profile alone, with the book reading it from there — see
    /// `two_notrump_wide`, `longer_major_response`, `xyz`.  Duplicating it into
    /// a `*Knobs` struct is invisible while the `thread_local!` cells still back
    /// both captures, since both read the same cell microseconds apart; it turns
    /// into a silent divergence the moment the cells go and the two fields
    /// become independently settable.  Twelve cells were duplicated exactly that
    /// way before this test existed.
    #[test]
    fn no_knob_lives_in_two_homes() {
        let agreements = include_str!("agreements.rs");
        let build: Vec<String> = [
            "CompetitionKnobs",
            "DefenseKnobs",
            "NotrumpKnobs",
            "OpeningKnobs",
            "ResponseKnobs",
            "RebidKnobs",
        ]
        .iter()
        .flat_map(|name| fields(agreements, name))
        .collect();
        assert!(
            build.len() > 100,
            "the build-time areas were found: {build:?}"
        );

        for (src, name) in [
            (include_str!("inference/knobs.rs"), "ReadingProfile"),
            (include_str!("instinct.rs"), "InstinctProfile"),
            (include_str!("context.rs"), "DecisionProfile"),
        ] {
            let classify = fields(src, name);
            assert!(!classify.is_empty(), "{name} was found");
            let both: Vec<&String> = build.iter().filter(|f| classify.contains(f)).collect();
            assert!(
                both.is_empty(),
                "{name} and the build-time areas share {} cell(s): {both:?} — a dual \
                 cell belongs to {name} alone, and the book should read it from \
                 the pinned profile",
                both.len(),
            );
        }
    }
}
