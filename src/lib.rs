#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
pub mod scoring;
/// Statistics
pub mod stats;

#[cfg(feature = "neural-floor")]
pub use bidding::two_over_one_neural;
pub use bidding::{
    Competitive, Constructive, Context, Defensive, Family, OrElse, Pair, Phase, Rules, Stance,
    System, Table, Trie, Versus, instinct, two_over_one, two_over_one_strawberry,
};
pub use stats::{Accumulator, Statistics};
