#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
/// Statistics
pub mod stats;

pub use bidding::{Context, OrElse, Rules, System, Trie, Versus, trie::Forest};
pub use stats::{Accumulator, Statistics};
