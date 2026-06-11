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
use super::array::Logits;
use super::constraint::Constraint;
use super::context::Context;
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

    /// The logit this rule contributes for a hand
    #[must_use]
    pub fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        self.weight + self.when.eval(hand, context)
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("call", &self.call)
            .field("weight", &self.weight)
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
        });
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
