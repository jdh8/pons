/// [`Call`]-indexed array
pub mod array;
pub mod constraint;
pub mod context;
pub mod fallback;
/// [`Call`]-keyed hash map
pub mod map;
pub mod rules;
/// [`Trie`] as a bidding system
pub mod trie;

pub use array::Array;
pub use context::Context;
pub use map::Map;
pub use rules::Rules;
pub use trie::{Trie, classifier};

use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};

/// Trait for a bidding system
///
/// A bidding system tries classifying a hand into logits for each call given
/// vulnerability and the auction.
///
/// # Vulnerability convention
///
/// `vul` is **relative to the side to act** — the side of the player whose
/// call is being classified.  Composite systems pass it through unchanged;
/// drivers convert from absolute vulnerability once per call with
/// [`context::relative`].
pub trait System {
    /// Classify a hand into logits for each call
    ///
    /// `auction` is the raw table auction (all four players' calls), and
    /// `vul` is relative to the side to act.
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
        let context = Context::new(vul, auction).with_prefixes(self.common_prefixes(auction));
        let (classifier, _) = self.resolve(&context, auction)?;
        Some(classifier.classify(hand, &context))
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
