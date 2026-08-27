/// What the partnership has agreed to play — the value a book is built from
pub mod agreements;
/// The basic 2/1 game-forcing system
pub mod american;
/// [`Call`]-indexed array
pub mod array;
/// Narrow, opt-in hooks for the out-of-crate performance harness.
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub mod benchmark;
/// Role-aware partnership books
pub mod book;
/// `.bbsa` convention cards generated from the live system
pub mod card;
/// System-independent build helpers shared across bidding systems
pub(in crate::bidding) mod common;
pub mod compose;
pub mod constraint;
pub mod context;
/// Finalized reader-side routing plans.
pub(in crate::bidding) mod decoder;
/// The Dutch system — a natural 2/1 built around a wide, non-forcing 1♣
pub mod dutch;
/// Call-EV evaluator: a candidate call's cardplay-grounded worth by rollout
#[cfg(feature = "dd")]
pub mod ev;
/// Learned trick evaluator: hidden-seat ranges → double-dummy trick mean/spread
pub mod evaluator;
pub mod fallback;
/// Versioned feature extractor for the AI instinct bidder
pub mod features;
/// Per-player shape and strength accumulated from the calls
pub mod inference;
pub mod instinct;
/// [`Call`]-keyed hash map
pub mod map;
/// Hand-rolled forward pass for the distilled neural floor
///
/// Always compiled: the configured BBA-distilled net
/// ([`neural::classify_bba_v4`]) backs the default
/// [`american`][american::american] floor.
pub mod neural;
/// Deterministic safety shell over the distilled neural floor
pub mod neural_floor;
/// Declarative book layer: entry rows compiled into the existing [`Trie`]
pub(in crate::bidding) mod rows;
pub mod rules;
/// Constrained layout sampling: deals consistent with an auction's inferences
pub mod sampler;
pub mod table;
/// Structural tag reading of a call — the shared corpus/feature vocabulary
pub mod tags;
/// [`Trie`] as a bidding system
pub mod trie;
/// Behavioral verification of authored constraints (AI-bidder M4.2)
pub mod verify;

pub use american::{
    american, american_book, american_book_default, american_default, american_floor,
    american_floor_default, american_instinct, american_instinct_default, american_with_card,
    american_with_config,
};
pub use array::Array;
pub use book::{
    Competitive, Constructive, Defensive, ExplainedRule, Partnership, Phase, ProbeReport, System,
};
pub use compose::{OrElse, Versus};
pub use context::Context;
pub use dutch::{
    dutch, dutch_book, dutch_book_default, dutch_default, dutch_instinct, dutch_instinct_default,
    dutch_with_card, dutch_with_config,
};
#[cfg(feature = "dd")]
pub use ev::ev_all;
pub use features::{
    CALLS_EVAL_V3, Config, FEATURES_LEN_EVAL, FEATURES_LEN_EVAL_V3, FEATURES_LEN_V3,
    FEATURES_LEN_V4, FEATURES_VERSION_EVAL, FEATURES_VERSION_V3, FEATURES_VERSION_V4,
    LEN_CALL_EVAL_V3, LEN_CARD, LEN_CARD_ROWS, LEN_SYSTEM, OFFSET_OUR_CARD, OFFSET_THEIR_CARD,
    features_eval, features_eval_v3, features_v3, features_v4,
};
pub use inference::{
    Envelope, EnvelopeUnion, Inferences, Range, ReadingProfile, ReadingScope, Relative,
};
pub use instinct::instinct;
pub use map::Map;
pub use rules::{Alert, Rules};
pub use sampler::{sample_defender_remnants, sample_layouts};
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
pub trait Bidder {
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

    /// Allocate optional state for one table-driven deal.
    ///
    /// Hidden serving hook: ordinary systems remain stateless, while finalized
    /// partnerships use it for their append-only authored-reading cache.
    #[doc(hidden)]
    fn new_deal_state(&self) -> Option<Box<dyn std::any::Any>> {
        None
    }

    /// Classify inside a table-driven deal using the state from
    /// [`Bidder::new_deal_state`].
    #[doc(hidden)]
    fn classify_in_deal(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        state: Option<&mut dyn std::any::Any>,
    ) -> Option<array::Logits> {
        let _ = state;
        self.classify(hand, vul, auction)
    }

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
    /// preserving behaviour for flat systems; structured ones like [`Partnership`]
    /// override it.  `vul` is needed only because resolution's fallback guards
    /// consult the context.
    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        let _ = (vul, auction);
        true
    }

    /// Whether `call` is **tombstoned** at `auction` — advised against, with no
    /// agreement behind it
    ///
    /// The state one step below unauthored, and the query that tells the two
    /// apart: an unauthored call is merely uncovered, and the floor bids it
    /// freely; a tombstoned one is masked to `-∞` before the floor's logits are
    /// used, while `authored_at` stays false because a veto-only node carries no
    /// classifier.  The system holds no opinion about the call *except* that we
    /// do not make it here, so there is nothing to alert, read, or disclose.
    ///
    /// The [replay sampler][crate::bidding::sampler::sample_layouts_replay]
    /// abstains on such a call: when a foreign engine makes it anyway, no
    /// candidate hand could have chosen it under our policy, so enforcing the
    /// reading would reject every world.
    ///
    /// Defaults to `false` — flat systems tombstone nothing; [`Trie`] and
    /// [`Partnership`] override it.
    fn tombstoned_at(&self, vul: RelativeVulnerability, auction: &[Call], call: Call) -> bool {
        let _ = (vul, auction, call);
        false
    }

    /// Compose a table where `self`'s partnership is the dealer's side
    ///
    /// `a.vs(b)` dispatches by parity: `a` answers at even auction lengths,
    /// `b` at odd ones.  Pick the seating per board by dealer — `ns.vs(ew)`
    /// when North/South deal, `ew.vs(ns)` otherwise.
    fn vs<B: Bidder>(self, other: B) -> Versus<Self, B>
    where
        Self: Sized,
    {
        Versus::new(self, other)
    }

    /// Layer `self` over a fallback system
    ///
    /// `a.or_else(b)` answers from `a`, falling through to `b` when `a`
    /// returns [`None`] or logits without any probability mass.
    fn or_else<B: Bidder>(self, other: B) -> OrElse<Self, B>
    where
        Self: Sized,
    {
        OrElse::new(self, other)
    }
}

/// References delegate to the referent, so `(&a).vs(&a)` needs no clone
impl<S: Bidder + ?Sized> Bidder for &S {
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

    fn tombstoned_at(&self, vul: RelativeVulnerability, auction: &[Call], call: Call) -> bool {
        (**self).tombstoned_at(vul, auction, call)
    }

    fn new_deal_state(&self) -> Option<Box<dyn std::any::Any>> {
        (**self).new_deal_state()
    }

    fn classify_in_deal(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        state: Option<&mut dyn std::any::Any>,
    ) -> Option<array::Logits> {
        (**self).classify_in_deal(hand, vul, auction, state)
    }
}

/// A bare trie is a hand-built *table* model: all four players bid from this
/// one book, keyed by the literal auction.
///
/// This is the low-level escape hatch — handy for a small, fixed table (such as
/// an analysis fragment) or a system whose pass semantics the role-aware books
/// cannot express (the [`Phase`] router assumes a standard pass).  Author a
/// pair's notes from its own side with [`Constructive`], [`Competitive`], and
/// [`Defensive`] instead, assembled into a [`System`] and bound with
/// [`System::bind`].
impl Bidder for Trie {
    fn classify(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<array::Logits> {
        let context = Context::new(vul, auction)
            .with_prefixes(self.common_prefixes(auction))
            .with_decision_cache(hand);
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
            Some((_, prov)) if prov.is_authored()
        )
    }

    fn tombstoned_at(&self, vul: RelativeVulnerability, auction: &[Call], call: Call) -> bool {
        // Node metadata, keyed at the exact auction: hand-independent and
        // vulnerability-independent, so no context is built and no rebase is
        // followed — unlike `authored_at`, which must resolve.
        let _ = vul;
        self.vetoes(auction, call)
    }
}
