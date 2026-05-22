#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
/// Statistics
pub mod stats;

pub use bidding::{Trie, trie::Forest};
pub use stats::{Accumulator, Statistics};
