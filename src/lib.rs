#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
// Categorical `clippy::pedantic` noise on a numerics-heavy bridge engine: int
// casts on card counts, big book/match tables, and suit/holding names that
// collide by design.  Allowed so a pedantic lint run is quiet on `src/` without
// rewriting correct code; no-ops under the default CI lint set.
#![allow(
    clippy::similar_names,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::doc_markdown
)]

/// Bidding in contract bridge
pub mod bidding;
/// GIB hand-record format (deal + cached double-dummy table)
#[cfg(feature = "dd")]
pub mod gib;
/// Compact binary DD database format (`.pdd`)
#[cfg(feature = "dd")]
pub mod pdd;
pub mod scoring;
/// Single-dummy trick estimation by Monte-Carlo double-dummy
#[cfg(feature = "dd")]
pub mod single_dummy;
/// Statistics
#[cfg(feature = "dd")]
pub mod stats;

pub use bidding::{
    Alert, Competitive, Constructive, Context, Defensive, Family, OrElse, Pair, Phase, Rules,
    Stance, System, Table, Trie, Versus, american, american_book, american_floor,
    american_instinct, dutch, dutch_instinct, instinct,
};
#[cfg(feature = "dd")]
pub use single_dummy::{
    LeadQuestion, single_dummy, single_dummy_declarer_tricks, single_dummy_lead_tricks,
    single_dummy_leads, single_dummy_playout,
};
#[cfg(feature = "dd")]
pub use stats::{Accumulator, Statistics};
