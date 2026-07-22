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
use super::constraint::{Constraint, Description};
use super::context::Context;
use super::inference::Inference;
use super::trie::Classifier;
use contract_bridge::Hand;
use contract_bridge::auction::Call;
use core::fmt;
use std::sync::Arc;

/// A per-call alert: the name of the artificial convention a rule's call shows
///
/// In real bridge an artificial call is *alerted* so the opponents read it as the
/// convention rather than as natural — the per-call dual of a whole-system
/// [`Family`][super::Family].  Here an alert does two jobs: it is the build-time
/// **gate** (`[`Rules::alert`]` stamps a block, [`Rules::gated`] ships only the
/// active variant), and it marks a call as artificial so the inference reader
/// suppresses the natural single-suit reading and projects the convention instead.
///
/// The newtype is open — each convention mints its own alert as a constant, such
/// as `const STAYMAN: Alert = Alert("stayman");`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Alert(pub &'static str);

/// A single bidding rule: a call justified by a constraint
#[derive(Clone)]
pub struct Rule {
    call: Call,
    weight: f32,
    when: Arc<dyn Constraint>,
    label: &'static str,
    alert: Option<Alert>,
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

    /// The [`Alert`] this rule carries, or [`None`] if its call is natural
    ///
    /// Set per block with [`Rules::alert`].  An unalerted rule is always live; an
    /// alerted one survives [`Rules::gated`] only when its alert is active.  This
    /// is how one book holds two convention variants (e.g. Puppet vs European
    /// 1NT responses) and authors only the selected one into the trie.
    #[must_use]
    pub const fn alert(&self) -> Option<Alert> {
        self.alert
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
        // ponytail: hull the DNF to a single box, so the alert/`artificial`
        // checks and `authored_reading` stay on `Inference`.  The overlay that
        // the sampler consumes uses [`project_dnf`][Self::project_dnf] to keep
        // the boxes when `dnf_reading` is on.
        self.when.project(context).hull()
    }

    /// The forward reading as a union of boxes — [`project`][Self::project]
    /// without the hull
    ///
    /// The overlay [`Inferences::read`][super::inference::Inferences] feeds the
    /// sampler; keeps the disjunctive boxes under
    /// [`set_dnf_reading`][super::set_dnf_reading] (off → one box, the hull).
    #[must_use]
    pub fn project_dnf(&self, context: &Context<'_>) -> super::inference::Dnf {
        self.when.project(context)
    }

    /// The **two-sided** envelope of this rule's constraint — floors and
    /// ceilings ([`Constraint::project_band`])
    ///
    /// What a *declined* call asserts: a passed hand satisfied some Pass
    /// rule's gate, so it lies within the union of the gates' bands.  The
    /// reading-side fold behind [`set_pass_reading`][super::set_pass_reading].
    #[must_use]
    pub fn project_band(&self, context: &Context<'_>) -> Inference {
        self.when.project_band(context).hull()
    }

    /// The two-sided band as a union of boxes — [`project_band`][Self::project_band]
    /// without the hull (the DNF overlay's Pass reading)
    #[must_use]
    pub fn project_band_dnf(&self, context: &Context<'_>) -> super::inference::Dnf {
        self.when.project_band(context)
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("call", &self.call)
            .field("weight", &self.weight)
            .field("label", &self.label)
            .field("alert", &self.alert)
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
            alert: None,
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

    /// Alert the most recently added rule as the artificial convention `alert`
    ///
    /// Chains after [`rule`][Self::rule], mirroring [`note`][Self::note]:
    /// `….rule(call, w, when).alert(STAYMAN)`.  Marks the call artificial — the
    /// inference reader reads it as the convention rather than as a natural suit —
    /// and where the convention is a build-time variant (Puppet vs European), the
    /// alert doubles as the gate so [`gated`][Self::gated] keeps the rule only when
    /// the variant is active.
    ///
    /// # Panics
    ///
    /// Panics if no rule has been added yet.
    #[must_use]
    pub fn alert(mut self, alert: Alert) -> Self {
        self.rules
            .last_mut()
            .expect("alert() requires a preceding rule()")
            .alert = Some(alert);
        self
    }

    /// Append another block's rules after this one's
    #[must_use]
    pub fn chain(mut self, other: Rules) -> Self {
        self.rules.extend(other.rules);
        self
    }

    /// Drop rules whose [`alert`][Rule::alert] is set but not `active`
    ///
    /// Unalerted rules (`alert: None`) always survive; an alerted rule lives only
    /// when `active(alert)` holds.  Called before trie insertion so a book that
    /// authored two convention variants ships only the selected one — the
    /// build-time gate that keeps `classify()` free of any variant check.
    #[must_use]
    pub fn gated(mut self, active: impl Fn(Alert) -> bool) -> Self {
        self.rules.retain(|rule| rule.alert.is_none_or(&active));
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
    fn test_alert_marks_block_and_gated_filters() {
        const PUPPET: Alert = Alert("puppet");
        const EUROPEAN: Alert = Alert("european");

        // Shared (unalerted) rule, then one alerted block per variant chained in.
        let rules = Rules::new()
            .rule(Call::Pass, 0.0, hcp(..8))
            .chain(
                Rules::new()
                    .rule(Bid::new(3, Strain::Clubs), 1.0, hcp(9..))
                    .alert(PUPPET),
            )
            .chain(
                Rules::new()
                    .rule(Bid::new(3, Strain::Clubs), 1.0, hcp(9..))
                    .alert(EUROPEAN),
            );

        assert_eq!(rules.rules()[0].alert(), None);
        assert_eq!(rules.rules()[1].alert(), Some(PUPPET));
        assert_eq!(rules.rules()[2].alert(), Some(EUROPEAN));

        // Gating to Puppet keeps the unalerted rule and the Puppet block only.
        let puppet = rules.clone().gated(|alert| alert == PUPPET);
        assert_eq!(puppet.rules().len(), 2);
        assert_eq!(puppet.rules()[0].alert(), None);
        assert_eq!(puppet.rules()[1].alert(), Some(PUPPET));

        // Gating to European keeps the unalerted rule and the European block.
        let european = rules.gated(|alert| alert == EUROPEAN);
        assert_eq!(european.rules().len(), 2);
        assert_eq!(european.rules()[1].alert(), Some(EUROPEAN));
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
