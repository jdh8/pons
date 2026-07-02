/// The basic 2/1 game-forcing system
pub mod american;
/// [`Call`]-indexed array
pub mod array;
/// Role-aware partnership books
pub mod book;
pub mod compose;
pub mod constraint;
pub mod context;
/// Call-EV evaluator: a candidate call's cardplay-grounded worth by rollout
pub mod ev;
pub mod fallback;
/// Versioned feature extractor for the AI instinct bidder
pub mod features;
/// Per-player shape and strength accumulated from the calls
pub mod inference;
pub mod instinct;
/// [`Call`]-keyed hash map
pub mod map;
/// Hand-rolled forward pass for the distilled neural floor (feature-gated)
#[cfg(feature = "neural-floor")]
pub mod neural;
/// Deterministic safety shell over the distilled neural floor (feature-gated)
#[cfg(feature = "neural-floor")]
pub mod neural_floor;
pub mod rules;
/// Constrained layout sampling: deals consistent with an auction's inferences
pub mod sampler;
/// Gated live double-dummy search bidder over the net prior (feature-gated)
#[cfg(feature = "search")]
pub mod search_floor;
pub mod table;
/// Structural tag reading of a call — the shared corpus/feature vocabulary
pub mod tags;
/// [`Trie`] as a bidding system
pub mod trie;
/// Behavioral verification of authored constraints (AI-bidder M4.2)
pub mod verify;

pub use american::american;
#[cfg(feature = "neural-floor")]
pub use american::{
    american_neural, american_neural_search, american_neural_v2, american_neural_v3,
};
#[cfg(feature = "search")]
pub use american::{american_search, american_search_with};
pub use array::Array;
pub use book::{Competitive, Constructive, Defensive, Family, Pair, Phase, Stance};
pub use compose::{OrElse, Versus};
pub use context::Context;
pub use ev::{ev, ev_all};
pub use features::{FEATURES_LEN, FEATURES_VERSION, features};
pub use inference::{
    Inference, Inferences, Range, Relative, set_alert_reading, set_control_bid_reading,
    set_fallback_projection, set_nt_invite_inference, set_rubens_transfer_reading,
};
pub use instinct::instinct;
pub use map::Map;
pub use rules::{Alert, Rules};
pub use sampler::sample_layouts;
#[cfg(feature = "search")]
pub use search_floor::{SearchBook, SearchFloor, american_search_book};
pub use table::Table;
pub use trie::{Trie, classifier};
pub use verify::{Report, accepts, compare};

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

    /// Whether `auction` resolves to an *authored* node rather than the floor
    ///
    /// True unless resolution (following `Rebase` fallbacks) falls all the way to
    /// the keyless floor — the depth-0 root fallback that answers a position no
    /// rule covers.  At an authored node a `-∞` logit for a call is a real "this
    /// hand does not bid that here"; at the floor it is mere absence of an opinion.
    /// The [replay sampler][crate::bidding::sampler::sample_layouts_replay]
    /// enforces its reading only at authored nodes and abstains at the floor,
    /// deferring to the range reader (so a competitive raise/rebid the floor
    /// handles is read the old way).  Defaults to `true` (assume authored),
    /// preserving behaviour for flat systems; structured ones like [`Stance`]
    /// override it.  `vul` is needed only because resolution's fallback guards
    /// consult the context.
    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        let _ = (vul, auction);
        true
    }

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

    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        (**self).authored_at(vul, auction)
    }
}

/// A bare trie is a hand-built *table* model: all four players bid from this
/// one book, keyed by the literal auction.
///
/// This is the low-level escape hatch — handy for a small, fixed table (such as
/// an analysis fragment) or a system whose pass semantics the role-aware books
/// cannot express (the [`Phase`] router assumes a standard pass).  Author a
/// pair's notes from its own side with [`Constructive`], [`Competitive`], and
/// [`Defensive`] instead, assembled into a [`Pair`] and bound with
/// [`Pair::against`].
impl System for Trie {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits> {
        let context = Context::new(vul, auction).with_prefixes(self.common_prefixes(auction));
        self.classify_floored(hand, &context, auction)
            .map(|(logits, _)| logits)
    }

    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        // Resolve as the bidder would (following `Rebase` fallbacks to the
        // canonical node).  Authored rules — primary nodes *and* guarded fallbacks
        // (responses, raises, Stayman, transfers, 2/1) — resolve either with
        // `fallback: None` or at depth ≥ 1; only the keyless floor answers at the
        // depth-0 root fallback.  A literal `get` would miss the fallback-authored
        // continuations entirely, abstaining far more than intended.
        let context = Context::new(vul, auction).with_prefixes(self.common_prefixes(auction));
        matches!(
            self.resolve(&context, auction),
            Some((_, prov)) if prov.fallback.is_none() || prov.depth > 0
        )
    }
}
