#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
/// Statistics
pub mod stats;

pub use bidding::{
    Constructive, Context, Defensive, OrElse, Partnership, Rules, System, Trie, Versus,
    two_over_one,
};
pub use stats::{Accumulator, Statistics};
