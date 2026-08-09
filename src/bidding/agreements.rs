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
//! Two halves, split by *when* a cell is read, with no cell in both — the
//! "one cell, one home" invariant the profile structs already keep:
//!
//! - `DecisionProfile` (crate-private until the cells are gone) — the cells
//!   read per decision, at classify time.  Already snapshot-shaped, and
//!   already pinned into a [`Stance`][crate::bidding::Stance] at
//!   [`Pair::against`][crate::bidding::Pair::against], so a stance decides
//!   identically on any thread.
//! - [`Build`][crate::bidding::agreements::Build] — the cells read only while
//!   the books are being built, and therefore baked into the rules that come
//!   back.
//!
//! A cell read at *both* times (there are 24) lives in `DecisionProfile` and is
//! read from there at build time too; it is never duplicated here.

use super::american::{
    Competitive4333, DoubleShape, DoubleStyle, FreeBidStyle, LebensohlStyle, NegativeDoubleShape,
    NotrumpDefense, NotrumpShape, TakeoutSupport, WeakTwoEval,
};
use super::context::DecisionProfile;

/// The knobs read only at book construction
///
/// One field per area, each captured by the module that owns its cells (see
/// [`CompetitionKnobs`], [`DefenseKnobs`], and [`NotrumpKnobs`]); areas move in
/// one at a time as their read sites convert from a thread-local getter to a
/// field of this value, and `docs/declarative-rows.md` holds the ledger.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Build {
    /// What we play when they contest our auction
    pub competition: CompetitionKnobs,
    /// What we play when they open the auction
    pub defense: DefenseKnobs,
    /// What we play after our notrump openings and rebids
    pub notrump: NotrumpKnobs,
    /// What we open, and how partner answers a weak two
    pub opening: OpeningKnobs,
}

impl Default for Build {
    fn default() -> Self {
        Self::current()
    }
}

impl Build {
    /// Capture this thread's build-time knob state
    #[must_use]
    pub fn current() -> Self {
        Self {
            competition: super::american::competition::capture(),
            defense: super::american::defense::capture(),
            notrump: super::american::notrump::capture(),
            opening: super::american::openings::capture(),
        }
    }
}

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
        Self::current()
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
    /// HCP floor for the natural penalty double of their `1NT`
    pub natural_double_floor: u8,
    /// Logit weight of the natural penalty double of their `1NT`
    pub natural_double_weight: i16,
    /// Inclusive points band for natural overcalls of their `1NT`
    pub natural_overcall_points: (u8, u8),
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
    /// Inclusive points band for Woolsey's suit actions
    pub woolsey_points: (u8, u8),
    /// Points floor for Woolsey's double
    pub woolsey_double_floor: u8,
    // --- defense/weak_two_nt_advance.rs
    /// Author advances of our `2NT` overcall of their weak two
    pub weak_two_notrump_advances_enabled: bool,
    // --- defense/advance_minor_jump.rs
    /// Author invitational minor jumps after partner's takeout double
    pub advance_minor_jump_enabled: bool,
    // --- defense/nt_defense.rs
    /// Which defense we play over their `1NT`
    pub notrump_defense: NotrumpDefense,
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
    /// Optional natural-plus-Landy strength band
    pub landy_range: Option<(u8, u8)>,
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
    /// Graft systems-on advances below our `1NT` overcall
    pub nt_overcall_systems_on: bool,
    /// Replace the major-opening graft with Gladiator
    pub nt_overcall_gladiator: bool,
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
        Self::current()
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
    /// Which minor-suit response scheme we play over our `1NT`
    pub notrump_minors: super::rules::Alert,
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
    /// Author Crawling Stayman
    pub crawling_stayman: bool,
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
    /// Author Garbage Stayman
    pub garbage_stayman: bool,
    // --- notrump/splinter.rs
    /// Author the `1NT` splinter
    pub nt_splinter: bool,
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
        Self::current()
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
    /// Open our strong `1NT` at all
    pub open_one_notrump: bool,
    /// Gauge the `1NT` range in Andrews' fifths rather than plain HCP
    pub one_notrump_fifths: bool,
    /// Which balanced shapes the strong `1NT` opening admits
    pub notrump_shape: NotrumpShape,
    /// Admit the off-shape `1NT` (a singleton honour in 4441/5431)
    pub one_notrump_offshape: bool,
    // --- openings/weak_two.rs
    /// Optional raw-HCP band gauging the weak-two opening
    pub weak_two_hcp: Option<(u8, u8)>,
    /// Optional honour-location evaluator gauging the weak-two opening
    pub weak_two_eval: Option<WeakTwoEval>,
    /// Open wild weak twos (five- or six-card suit, `points(3..=12)`)
    pub weak_two_wild: bool,
    // --- weak_twos.rs
    /// Prefer a major when answering partner's weak two
    pub weak_two_major_priority: bool,
    /// Answer partner's weak two with the longest suit first
    pub weak_two_longest_first: bool,
}

impl Default for OpeningKnobs {
    fn default() -> Self {
        Self::current()
    }
}

impl OpeningKnobs {
    /// Capture this thread's opening build-time knob state
    #[must_use]
    pub fn current() -> Self {
        super::american::openings::capture()
    }
}

/// Everything the partnership has agreed to play
///
/// Constructed once per build and threaded down by reference. Cloning is cheap
/// but pointless: the whole design is that one capture serves a whole build.
#[derive(Clone, Copy, PartialEq)]
pub struct Agreements {
    /// The classify-time cells, pinned into the stance at `Pair::against`
    pub(crate) decision: DecisionProfile,
    /// The build-time cells, baked into the rules this build produces
    pub(crate) build: Build,
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
            build: Build::current(),
        }
    }
}
