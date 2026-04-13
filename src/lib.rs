#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

/// Bidding in contract bridge
pub mod bidding;
/// Card shuffling
pub mod deck;
/// Hand evaluation
pub mod eval;
/// Statistics
pub mod stats;

pub use bidding::{Auction, Call};
pub use deck::{Deck, full_deal};
pub use eval::HandEvaluator;
pub use stats::{Accumulator, Statistics};
