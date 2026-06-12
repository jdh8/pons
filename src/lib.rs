#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
/// Statistics
pub mod stats;

pub use bidding::{
    Competitive, Constructive, Context, Defensive, Family, OrElse, Pair, Phase, Rules, Stance,
    System, Table, Trie, Versus, two_over_one,
};
pub use stats::{Accumulator, Statistics};
