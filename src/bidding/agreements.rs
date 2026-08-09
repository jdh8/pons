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

use super::context::DecisionProfile;

/// The knobs read only at book construction
///
/// Cells move in here module by module as their read sites convert from a
/// thread-local getter to a field of this value; see `docs/declarative-rows.md`
/// for the ledger. Empty until the first module lands — the plumbing that
/// carries it is deliberately a separate, behaviour-free step.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Build {}

impl Build {
    /// Capture this thread's build-time knob state
    #[must_use]
    pub const fn current() -> Self {
        Self {}
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
