/// [`Call`]-indexed array
pub mod array;
pub mod compose;
pub mod constraint;
pub mod context;
pub mod fallback;
/// [`Call`]-keyed hash map
pub mod map;
pub mod rules;
/// [`Trie`] as a bidding system
pub mod trie;

pub use array::Array;
pub use compose::{OrElse, Versus};
pub use context::Context;
pub use map::Map;
pub use rules::Rules;
pub use trie::{SeatClasses, Trie, classifier};

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

    /// Compose a table where `self`'s partnership is the dealer's side
    ///
    /// `a.vs(b)` dispatches by parity: `a` answers at even auction lengths,
    /// `b` at odd ones.  Pick the seating per board by dealer — `ns.vs(ew)`
    /// when North/South deal, `ew.vs(ns)` otherwise.
    fn vs<B: System>(self, other: B) -> Versus<Self, B>
    where
        Self: Sized,
    {
        Versus::new(self, other)
    }

    /// Layer `self` over a fallback system
    ///
    /// `a.or_else(b)` answers from `a`, falling through to `b` when `a`
    /// returns [`None`] or logits without any probability mass.
    fn or_else<B: System>(self, other: B) -> OrElse<Self, B>
    where
        Self: Sized,
    {
        OrElse::new(self, other)
    }
}

/// References delegate to the referent, so `(&a).vs(&a)` needs no clone
impl<S: System + ?Sized> System for &S {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits> {
        (**self).classify(hand, vul, auction)
    }
}

/// A bare trie is a *table* model: all four players bid from this book.
///
/// For a partnership-specific system composed against opponents, see
/// [`Forest`][trie::Forest].
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

/// A forest is a *partnership* system: it strips the leading passes off the
/// table auction and selects the [`passed`][trie::Forest::passed] or
/// [`unpassed`][trie::Forest::unpassed] book for the side to act.  The
/// classifier still receives the context of the raw auction.
impl System for trie::Forest {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits> {
        let leading = auction
            .iter()
            .take_while(|&&call| call == Call::Pass)
            .count();
        let openers = (auction.len() - leading).is_multiple_of(2);
        let passed = leading >= if openers { 2 } else { 1 };
        let trie = if passed { &self.passed } else { &self.unpassed };
        let stripped = &auction[leading..];

        let context = Context::new(vul, auction).with_prefixes(trie.common_prefixes(stripped));
        let (classifier, _) = trie.resolve(&context, stripped)?;
        Some(classifier.classify(hand, &context))
    }
}
