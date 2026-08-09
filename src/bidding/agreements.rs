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
    Competitive4333, DoubleStyle, FreeBidStyle, LebensohlStyle, NegativeDoubleShape,
};
use super::context::DecisionProfile;

/// The knobs read only at book construction
///
/// One field per area, each captured by the module that owns its cells (see
/// [`CompetitionKnobs`]); areas move in one at a time as their read sites
/// convert from a thread-local getter to a field of this value, and
/// `docs/declarative-rows.md` holds the ledger.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Build {
    /// What we play when they contest our auction
    pub competition: CompetitionKnobs,
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

/// Everything the partnership has agreed to play
///
/// Constructed once per build and threaded down by reference. Cloning is cheap
/// but pointless: the whole design is that one capture serves a whole build.
#[derive(Clone, Copy, PartialEq, Eq)]
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
