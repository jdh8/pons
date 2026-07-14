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
    Stance, System, Table, Trie, Versus, american, instinct,
};
#[cfg(feature = "neural-floor")]
pub use bidding::{
    american_neural, american_neural_search, american_neural_v2, american_neural_v3,
};
#[cfg(feature = "search")]
pub use bidding::{american_search, american_search_book, american_search_with};
#[cfg(feature = "dd")]
pub use single_dummy::{LeadQuestion, single_dummy, single_dummy_lead_tricks, single_dummy_leads};
#[cfg(feature = "dd")]
pub use stats::{Accumulator, Statistics};
