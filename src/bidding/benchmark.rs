//! Opt-in benchmark hooks.
//!
//! Cargo benchmarks are separate crates, so they cannot reach crate-private
//! implementation boundaries directly. Keep this module feature-gated and
//! intentionally tiny: it exposes computation, not serving state or semantics.

use super::array::Logits;
use super::context::Context;
use super::features::{
    FEATURES_LEN_EVAL, FEATURES_LEN_EVAL_V3, FEATURES_LEN_EVAL_V4, features_eval, features_eval_v3,
    features_eval_v4,
};
use super::inference::{Inferences, ReadingProfile};
use super::rules::Rules;
use super::trie::Classifier;
use super::trie::Provenance;
use super::{Stance, System};
use contract_bridge::Hand;
use contract_bridge::auction::Auction;
use contract_bridge::auction::{Call, RelativeVulnerability};

/// Run the frozen pre-cache classification path.
///
/// This is exposed only so the separate Criterion crate can measure the cache
/// treatment against its same-process semantic reference.
#[must_use]
pub fn classify_with_provenance_uncached(
    stance: &Stance,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> Option<(Logits, Provenance)> {
    stance.classify_with_provenance_uncached(hand, vul, auction)
}

/// Whether serving delegates this floor position bit-for-bit to deterministic instinct.
#[must_use]
pub fn is_deterministic_instinct_floor(
    stance: &Stance,
    deterministic: &Stance,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> bool {
    let Some((logits, provenance)) = stance.classify_with_provenance(hand, vul, auction) else {
        return false;
    };
    if provenance.depth != 0 || provenance.fallback.is_none() {
        return false;
    }
    deterministic
        .classify(hand, vul, auction)
        .is_some_and(|instinct| {
            logits
                .iter()
                .zip(instinct.iter())
                .all(|((one_call, one), (two_call, two))| {
                    one_call == two_call && one.to_bits() == two.to_bits()
                })
        })
}

/// Features for whichever evaluator variant serving would select right now.
#[derive(Clone, Debug)]
pub enum ActiveEvaluatorFeatures {
    /// Legacy v2 hull features.
    V2([f32; FEATURES_LEN_EVAL]),
    /// Shipped v3 calls-tail features.
    V3([f32; FEATURES_LEN_EVAL_V3]),
    /// Optional v4 shape-reading features.
    V4([f32; FEATURES_LEN_EVAL_V4]),
}

impl ActiveEvaluatorFeatures {
    /// Name used in benchmark diagnostics.
    #[must_use]
    pub const fn variant(&self) -> &'static str {
        match self {
            Self::V2(_) => "v2",
            Self::V3(_) => "v3",
            Self::V4(_) => "v4",
        }
    }
}

/// Extract features through the same variant selection as public convenience
/// serving, using the shipped [`ReadingProfile::default`] envelope-union
/// regime.  Stance-bound serving selects from its explicit pinned profile.
#[must_use]
pub fn active_evaluator_features(
    hand: Hand,
    inferences: &Inferences,
    auction: &[Call],
) -> ActiveEvaluatorFeatures {
    if !ReadingProfile::default().envelope_union {
        ActiveEvaluatorFeatures::V2(features_eval(hand, inferences))
    } else if super::evaluator::eval_shape() {
        ActiveEvaluatorFeatures::V4(features_eval_v4(hand, inferences, auction))
    } else if !super::evaluator::eval_auction() {
        ActiveEvaluatorFeatures::V2(features_eval(hand, inferences))
    } else {
        ActiveEvaluatorFeatures::V3(features_eval_v3(hand, inferences, auction))
    }
}

/// Run only the forward half of the active evaluator over prebuilt features.
#[must_use]
pub fn active_evaluator_forward(features: &ActiveEvaluatorFeatures) -> [f32; 40] {
    match features {
        ActiveEvaluatorFeatures::V2(features) => super::evaluator::forward(features),
        ActiveEvaluatorFeatures::V3(features) => super::evaluator::forward_v3(features),
        ActiveEvaluatorFeatures::V4(features) => super::evaluator::forward_v4(features),
    }
}

/// Select a call through the production legality mask and stable maximum.
#[must_use]
pub fn select_legal_call(logits: Logits, auction: &Auction) -> Call {
    super::table::select_legal_call(Some(logits), auction)
}

/// Classify the deterministic instinct ladder under one fresh decision scope.
#[must_use]
pub fn classify_instinct_scoped(
    stance: &Stance,
    ladder: &Rules,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> Logits {
    let trie = stance.trie_for(auction);
    let context = Context::new(vul, auction)
        .with_prefixes(trie.common_prefixes(auction))
        .with_system(stance)
        .with_decision_cache(hand);
    ladder.classify(hand, &context)
}

/// Classify the deterministic instinct ladder under the pre-cache context.
#[must_use]
pub fn classify_instinct_uncached(
    stance: &Stance,
    ladder: &Rules,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> Logits {
    let trie = stance.trie_for(auction);
    let context = Context::new(vul, auction)
        .with_prefixes(trie.common_prefixes(auction))
        .with_system(stance);
    ladder.classify(hand, &context)
}
