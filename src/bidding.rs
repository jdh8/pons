/// [`Call`]-indexed array
pub mod array;
/// [`Call`]-keyed hash map
pub mod map;
/// [`Trie`] as a bidding system
pub mod trie;

pub use array::Array;
pub use map::Map;
pub use trie::Trie;

use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};

/// Trait for a bidding system
///
/// A bidding system tries classifying a hand into logits for each call given
/// vulnerability and the auction.
pub trait System {
    /// Classify a hand into logits for each call
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits>;
}

impl System for Trie {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits> {
        self.get(auction)
            .map(|f| f.classify(hand, vul, self.common_prefixes(auction)))
    }
}

impl System for trie::Forest {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits> {
        self[vul].classify(hand, vul, auction)
    }
}
