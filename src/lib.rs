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
pub mod gib;
pub mod scoring;
/// Statistics
pub mod stats;

pub use bidding::{
    Competitive, Constructive, Context, Defensive, Family, OrElse, Pair, Phase, Rules, Stance,
    System, Table, Trie, Versus, american, instinct,
};
#[cfg(feature = "neural-floor")]
pub use bidding::{
    american_neural, american_neural_search, american_neural_v2, american_neural_v3,
};
#[cfg(feature = "search")]
pub use bidding::{american_search, american_search_book, american_search_with};
pub use stats::{Accumulator, Statistics};
