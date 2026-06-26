//! Rule lists as hand classifiers
//!
//! [`Rules`] is the authored form of a [`Classifier`]: an ordered list of
//! [`Rule`]s, each tying a call to a [`Constraint`] with a weight.  The logit
//! of a call is the **maximum** of `weight + constraint` over its rules —
//! alternative justifications for the same call do not multiply its
//! probability.
//!
//! Weights are soft priority: a gap of about 3 nats is near-deterministic
//! after softmax, while equal weights yield a genuine mixed strategy.

use super::Map;
use super::Tag;
use super::array::Logits;
use super::constraint::{Constraint, Description};
use super::context::Context;
use super::inference::Inference;
use super::trie::Classifier;
use contract_bridge::Hand;
use contract_bridge::auction::Call;
use core::fmt;
use std::sync::Arc;

/// A single bidding rule: a call justified by a constraint
#[derive(Clone)]
pub struct Rule {
    call: Call,
    weight: f32,
    when: Arc<dyn Constraint>,
    label: &'static str,
    tag: Option<Tag>,
}

impl Rule {
    /// The call this rule justifies
    #[must_use]
    pub const fn call(&self) -> Call {
        self.call
    }

    /// The weight (soft priority) of this rule
    #[must_use]
    pub const fn weight(&self) -> f32 {
        self.weight
    }

    /// The human-readable meaning of this rule, or `""` if unlabeled
    ///
    /// Set with [`Rules::note`].  Feeds the description corpus and any
    /// `explain()`-style tooling that names a bid's meaning; the empty default
    /// keeps the 510 authored rules churn-free until a meaning is worth adding.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        self.label
    }

    /// The [`Tag`] gating this rule, or [`None`] if it is unconditional
    ///
    /// Set per block with [`Rules::only`].  An untagged rule is always live; a
    /// tagged one survives [`Rules::gated`] only when its tag is active.  This
    /// is how one book holds two convention variants (e.g. Puppet vs European
    /// 1NT responses) and authors only the selected one into the trie.
    #[must_use]
    pub const fn tag(&self) -> Option<Tag> {
        self.tag
    }

    /// The logit this rule contributes for a hand
    #[must_use]
    pub fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.weight + self.when.eval(hand, context)
    }

    /// The constraint's meaning as a [`Description`]
    ///
    /// Renders the *actual* constraint behind the call — `15–17 HCP, and
    /// balanced` — rather than the hand-authored [`label`][Self::label] or a
    /// structurally-guessed gloss.  This is the readable face of a book: the
    /// meaning is read straight from the logic it bids on, so the two cannot
    /// drift.  A bare [`pred`][super::constraint::pred] renders
    /// [`Opaque`][Description::Opaque]; use
    /// [`described`][super::constraint::described] to give one a meaning.
    #[must_use]
    pub fn describe(&self) -> Description {
        self.when.describe()
    }

    /// The forward [`Inference`] envelope this rule's constraint implies
    ///
    /// The reading-side dual of [`eval`][Self::eval]: where `eval` scores a
    /// known hand, `project` reports the per-suit length and point ranges every
    /// hand the rule accepts must fall within — what a *partner* who saw only
    /// the call may assert.  Mirrors [`describe`][Self::describe] (both delegate
    /// to the constraint fold); sound by construction (see
    /// [`Constraint::project`]).
    #[must_use]
    pub fn project(&self, context: &Context<'_>) -> Inference {
        self.when.project(context)
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("call", &self.call)
            .field("weight", &self.weight)
            .field("label", &self.label)
            .field("tag", &self.tag)
            .finish_non_exhaustive()
    }
}

/// An ordered list of [`Rule`]s acting as a [`Classifier`]
#[derive(Clone, Debug, Default)]
pub struct Rules {
    rules: Vec<Rule>,
}

impl Rules {
    /// Construct an empty rule list
    #[must_use]
    pub const fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Append a rule justifying a call (builder style)
    #[must_use]
    pub fn rule(
        mut self,
        call: impl Into<Call>,
        weight: f32,
        when: impl Constraint + 'static,
    ) -> Self {
        self.rules.push(Rule {
            call: call.into(),
            weight,
            when: Arc::new(when),
            label: "",
            tag: None,
        });
        self
    }

    /// Label the most recently added rule with a human-readable meaning
    ///
    /// Chains after [`rule`][Self::rule]: `….rule(call, w, when).note("T/O")`.
    /// The label feeds the description corpus (see [`Rule::label`]).  Labeling
    /// is opt-in and incremental — most rules stay unlabeled and have their
    /// meaning derived structurally at export time.
    ///
    /// # Panics
    ///
    /// Panics if no rule has been added yet.
    #[must_use]
    pub fn note(mut self, label: &'static str) -> Self {
        self.rules
            .last_mut()
            .expect("note() requires a preceding rule()")
            .label = label;
        self
    }

    /// Gate every rule in this block behind `tag` (block-level [`note`][Self::note])
    ///
    /// Builds a tagged variant: `european_minors().only(Tag::EUROPEAN)` marks the
    /// whole block so that [`gated`][Self::gated] keeps it only when European is
    /// the active variant.  Chain two tagged blocks into one [`Rules`] (with
    /// [`chain`][Self::chain]) to hold both variants at a single auction key.
    #[must_use]
    pub fn only(mut self, tag: Tag) -> Self {
        for rule in &mut self.rules {
            rule.tag = Some(tag);
        }
        self
    }

    /// Append another block's rules after this one's
    #[must_use]
    pub fn chain(mut self, other: Rules) -> Self {
        self.rules.extend(other.rules);
        self
    }

    /// Drop rules whose [`tag`][Rule::tag] is set but not `active`
    ///
    /// Untagged rules (`tag: None`) always survive; a tagged rule lives only when
    /// `active(tag)` holds.  Called before trie insertion so a book that authored
    /// two convention variants ships only the selected one — the build-time gate
    /// that keeps `classify()` free of any variant check.
    #[must_use]
    pub fn gated(mut self, active: impl Fn(Tag) -> bool) -> Self {
        self.rules.retain(|rule| rule.tag.is_none_or(&active));
        self
    }

    /// View the rules in declaration order
    #[must_use]
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    /// Explain a classification: the winning rule per call
    ///
    /// For every call with finite logit, returns the index of the rule that
    /// produced its maximum together with that logit.  This answers "why did
    /// you bid that" — and "why did you not" for absent calls.
    #[must_use]
    pub fn explain(&self, hand: Hand, context: &Context<'_>) -> Map<(usize, f32)> {
        let mut best = Map::new();

        for (index, rule) in self.rules.iter().enumerate() {
            let logit = rule.eval(hand, context);

            let entry = best.entry(rule.call);
            if logit > f32::NEG_INFINITY && entry.is_none_or(|(_, incumbent)| logit > incumbent) {
                entry.replace((index, logit));
            }
        }
        best
    }
}

impl Classifier for Rules {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        let mut logits = Logits::new();

        for rule in &self.rules {
            let slot = logits.0.get_mut(rule.call);
            *slot = slot.max(rule.eval(hand, context));
        }
        logits
    }

    fn as_rules(&self) -> Option<&Rules> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::constraint::{balanced, hcp, len};
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Strain, Suit};

    fn opening_rules() -> Rules {
        Rules::new()
            .rule(Bid::new(1, Strain::Notrump), 1.0, hcp(15..=17) & balanced())
            .rule(
                Bid::new(1, Strain::Spades),
                1.0,
                hcp(11..=21) & len(Suit::Spades, 5..),
            )
            .rule(Call::Pass, 0.0, hcp(..11))
    }

    fn best_call(logits: &Logits) -> Call {
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    #[test]
    fn test_classification() {
        let rules = opening_rules();
        let context = Context::new(RelativeVulnerability::NONE, &[]);

        let notrump = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
        assert_eq!(
            best_call(&rules.classify(notrump, &context)),
            Call::Bid(Bid::new(1, Strain::Notrump)),
        );

        let spades = "AKQ32.K532.QJ4.9".parse().expect("valid hand");
        assert_eq!(
            best_call(&rules.classify(spades, &context)),
            Call::Bid(Bid::new(1, Strain::Spades)),
        );

        let weak = "98432.K53.QJ4.92".parse().expect("valid hand");
        assert_eq!(best_call(&rules.classify(weak, &context)), Call::Pass);
    }

    #[test]
    fn test_note_labels_last_rule_and_downcasts() {
        let rules = Rules::new()
            .rule(Bid::new(1, Strain::Notrump), 1.0, hcp(15..=17) & balanced())
            .note("15-17 BAL")
            .rule(Call::Pass, 0.0, hcp(..11));

        // note() labels the immediately preceding rule; the unlabeled one is "".
        assert_eq!(rules.rules()[0].label(), "15-17 BAL");
        assert_eq!(rules.rules()[1].label(), "");

        // The corpus hook recovers the authored rules through a type-erased ref.
        let erased: &dyn Classifier = &rules;
        let recovered = erased.as_rules().expect("Rules downcasts to itself");
        assert_eq!(recovered.rules().len(), 2);
        assert_eq!(recovered.rules()[0].label(), "15-17 BAL");
    }

    #[test]
    fn test_only_tags_block_and_gated_filters() {
        const PUPPET: Tag = Tag("puppet");
        const EUROPEAN: Tag = Tag("european");

        // Shared (untagged) rule, then one tagged block per variant chained in.
        let rules = Rules::new()
            .rule(Call::Pass, 0.0, hcp(..8))
            .chain(
                Rules::new()
                    .rule(Bid::new(3, Strain::Clubs), 1.0, hcp(9..))
                    .only(PUPPET),
            )
            .chain(
                Rules::new()
                    .rule(Bid::new(3, Strain::Clubs), 1.0, hcp(9..))
                    .only(EUROPEAN),
            );

        assert_eq!(rules.rules()[0].tag(), None);
        assert_eq!(rules.rules()[1].tag(), Some(PUPPET));
        assert_eq!(rules.rules()[2].tag(), Some(EUROPEAN));

        // Gating to Puppet keeps the untagged rule and the Puppet block only.
        let puppet = rules.clone().gated(|tag| tag == PUPPET);
        assert_eq!(puppet.rules().len(), 2);
        assert_eq!(puppet.rules()[0].tag(), None);
        assert_eq!(puppet.rules()[1].tag(), Some(PUPPET));

        // Gating to European keeps the untagged rule and the European block.
        let european = rules.gated(|tag| tag == EUROPEAN);
        assert_eq!(european.rules().len(), 2);
        assert_eq!(european.rules()[1].tag(), Some(EUROPEAN));
    }

    #[test]
    fn test_explain() {
        let rules = opening_rules();
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let hand = "AKQ32.K532.QJ4.9".parse().expect("valid hand");
        let explanation = rules.explain(hand, &context);

        let spades = Call::Bid(Bid::new(1, Strain::Spades));
        assert_eq!(explanation.get(spades), Some(&(1, 1.0)));
        assert_eq!(explanation.get(Call::Pass), None);
        assert_eq!(explanation.get(Call::Double), None);
    }
}
