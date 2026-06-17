#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
pub mod scoring;
/// Statistics
pub mod stats;

pub use bidding::{
    Competitive, Constructive, Context, Defensive, Family, OrElse, Pair, Phase, Rules, Stance,
    System, Table, Trie, Versus, american, bare_polish_club, instinct, polish_club,
};
#[cfg(feature = "neural-floor")]
pub use bidding::{american_neural, american_neural_search, american_neural_v2};
#[cfg(feature = "search")]
pub use bidding::{american_search, american_search_with};
pub use stats::{Accumulator, Statistics};
