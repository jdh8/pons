#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
pub mod scoring;
/// Statistics
pub mod stats;

pub use bidding::{
    Competitive, Constructive, Context, Defensive, Family, OrElse, Pair, Phase, Rules, Stance,
    System, Table, Trie, Versus, bare_polish_club, instinct, polish_club, two_over_one,
};
#[cfg(feature = "neural-floor")]
pub use bidding::{two_over_one_neural, two_over_one_neural_search, two_over_one_neural_v2};
#[cfg(feature = "search")]
pub use bidding::{two_over_one_search, two_over_one_search_with};
pub use stats::{Accumulator, Statistics};
