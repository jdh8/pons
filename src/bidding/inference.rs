//! What the calls have shown, accumulated across an auction
//!
//! [`Context`][crate::bidding::context::Context] gives the *laws-only* facts of an
//! auction; the authored books and the [instinct floor][super::instinct()] read
//! *system intent* off the calls on demand (see
//! [`Interpretation`][super::instinct()]).  This module is the richest such
//! reading: for every player, the range of cards each suit may hold and the
//! range of points shown, derived **purely from the calls** under standard 2/1
//! meanings.
//!
//! Two consumers want this summary that the per-bid [`Constraint`][crate::bidding::constraint::Constraint]s cannot
//! give them — a `Constraint` is eval-only, so the length a `len(..)` rule
//! asserts can never be read back out:
//!
//! - the [instinct floor][super::instinct()], so a forced auction can pick a
//!   known major-suit fit over notrump instead of re-deriving partner's shape
//!   from scratch;
//! - constrained sampling (future), which needs per-player {suit → length} and
//!   points to deal hands consistent with an auction.
//!
//! # Soundness over tightness
//!
//! Every player starts at [`Envelope::unknown`] and each call only ever
//! *narrows* a range via [`Range::intersect`].  A rule that is unsure leaves
//! the range wide; a missing rule costs tightness, never soundness.  The
//! guarantee a consumer may rely on is one-directional: a hand that actually
//! made these calls always falls **within** every shown range.  The deriver
//! therefore reads only the meanings that hold robustly — natural suit
//! lengths, raises, rebids, overcalls — and stays silent on the artificial
//! structures (Stayman, transfers, the strong-2♣ responses) that a keyless
//! reading would misread as natural.
//!
//! # One system
//!
//! The meanings encoded here are those of [`american`][super::american()]
//! (five-card majors, strong 15–17 notrump, strong artificial 2♣); like the
//! instinct floor, this reading is tied to that system.
//!
//! # Layout
//!
//! The submodules are private — everything public reaches the outside through
//! the re-exports below, so these names are orientation, not paths.
//!
//! | Submodule | What lives there |
//! | --- | --- |
//! | `envelope` | [`Range`], [`Strength`][crate::bidding::inference::Strength], [`Envelope`], [`EnvelopeUnion`] — the box types and their DNF |
//! | `knobs` | the classify-time reading knobs and the `ReadingProfile` snapshot that keys the caches |
//! | `read` | [`Inferences`] and the natural walk that accumulates it |
//! | `projection` | decoding an alerted call by replaying its authoring rule's `project` fold |
//! | `readers` | the hand-written convention readers still awaiting retirement |

mod envelope;
mod knobs;
mod projection;
mod read;
mod readers;

pub use envelope::{Envelope, EnvelopeUnion, Range, Relative, Strength};
pub use knobs::{
    ReadingScope, blind_opponent_reading, control_bid_reading, envelope_union_reading,
    pass_exclusion_reading, rule_accept_enabled, set_announced_reading, set_blind_opponent_reading,
    set_control_bid_reading, set_cue_reading, set_envelope_union_reading, set_fallback_projection,
    set_gauge_membership, set_length_soundness, set_nt_invite_inference,
    set_pass_exclusion_reading, set_pass_reading, set_probed_reading, set_probed_vacuous_reading,
    set_reading_scope, set_rubens_transfer_reading, set_rule_accept, set_sum_closure,
    set_table_alert_reading, set_upgrade_closure,
};
pub use read::Inferences;

pub(crate) use envelope::relative_of;
pub(crate) use knobs::{ReadingProfile, probed_reading, probed_vacuous_reading, reading_profile};
pub(crate) use projection::{AuthoredProjection, AuthoringStepCache};
pub(in crate::bidding) use readers::penalty_x_reading;

#[cfg(test)]
pub(crate) use projection::{
    artificial, assert_compiled_authoring_projection_parity, assert_step_cache_projection_parity,
};
#[cfg(test)]
pub(crate) use read::authored_reading;

/// The largest a suit length range may span
const LENGTH_CAP: u8 = 13;
/// The largest a point range may span (all forty HCP, then some)
const POINTS_CAP: u8 = 37;
/// The largest a single suit's HCP range may span (AKQJ)
const SUIT_HCP_CAP: u8 = 10;

#[cfg(test)]
mod tests;
