//! Behavioral verification for the authoring compiler (AI-bidder M4.2)
//!
//! The authoring compiler (`docs/ai-bidder/dsl-spec.md`) turns an English
//! gloss into a [`Constraint`][crate::bidding::constraint::Constraint].  Milestone M4.1's round-trip check is a *string*
//! compare — `compiled.describe().to_string() == gloss` — which proves the
//! compiled tree *renders* to the intended meaning.  That check is blind in
//! exactly two places the compiler can still go wrong:
//!
//! 1. The body of a [`described`][crate::bidding::constraint::described()] escape hatch.
//!    `describe()` renders only the *label*, so a closure for "prefers diamonds"
//!    or "exactly 2 keycards" could accept the wrong hands and round-trip anyway.
//! 2. Porting from looser human notes (M4.3), where "matches the original rule"
//!    is a question about *which hands are accepted*, not about a string.
//!
//! This module closes both with a **behavioral** check: sample random hands,
//! compare a candidate's accept/reject set against an intent oracle (the original
//! rule when porting, or hand-labeled examples), and surface counterexamples.
//! The model proposes; this deterministic Rust check disposes — an LLM
//! mis-compilation becomes a failing test, not a silent bidding bug.
//!
//! # What "accepts" means
//!
//! A crisp [`Constraint`][crate::bidding::constraint::Constraint] contributes `0.0` when satisfied and
//! [`f32::NEG_INFINITY`] when violated; the trait forbids `+∞`, so **finite ⇔
//! satisfied**.  [`accepts`] is therefore `eval(hand, ctx) > f32::NEG_INFINITY`,
//! the very test `classify` and
//! [`explain`][super::Rules::explain] use to admit a call.  All current
//! primitives are crisp; a fuzzy evaluator would need a threshold instead, which
//! this first cut does not model.
//!
//! # Scope and honest limits
//!
//! - **Fixed context.** Comparison is over the *hand* space at a caller-supplied
//!   [`Context`] (an empty one by default).  The dominant intent disagreements —
//!   shape, strength, and every [`described`][crate::bidding::constraint::described()] hand predicate — are context-free,
//!   as is the canonical soundness case ("5+ ♥" must not accept four-card
//!   holdings).  Varying the context across legal auctions is future work.
//! - **Sampling, not proof.** A disagreement confined to a single rare holding
//!   can be missed by a finite sample, so callers pick `n` large (the tests and
//!   example use several thousand) — enough that any off-by-one bound or wrong
//!   comparator surfaces with overwhelming probability.  Agreement here is strong
//!   evidence, not a proof of equivalence.

use super::constraint::Constraint;
use super::context::Context;
use contract_bridge::auction::RelativeVulnerability;
use contract_bridge::deck::full_deal;
use contract_bridge::{Hand, Seat};
use rand::Rng;

/// The most counterexample hands a [`Report`] retains
///
/// A disagreement is a bug to fix, not a statistic to total precisely; a handful
/// of witnesses is enough to diagnose it, and the bound keeps the report cheap to
/// build and print.
const MAX_COUNTEREXAMPLES: usize = 16;

/// Whether a constraint accepts a hand in a context
///
/// The crisp-accept convention of the module: a finite logit means satisfied.
#[must_use]
pub fn accepts(constraint: &impl Constraint, hand: Hand, context: &Context<'_>) -> bool {
    constraint.eval(hand, context) > f32::NEG_INFINITY
}

/// View a constraint as a fixed-context hand predicate
///
/// The common adapter for [`compare`]: borrows the constraint and a context and
/// returns `|hand| accepts(constraint, hand, context)`.  A book [`Rule`] is used
/// directly instead — `|hand| rule.eval(hand, ctx).is_finite()` — since its
/// constraint is private but its [`eval`][super::rules::Rule::eval] is not.
///
/// [`Rule`]: super::rules::Rule
pub fn predicate<'a>(
    constraint: &'a impl Constraint,
    context: &'a Context<'a>,
) -> impl Fn(Hand) -> bool + 'a {
    move |hand| accepts(constraint, hand, context)
}

/// Outcome of a behavioral comparison over sampled hands
///
/// `reference` is the intent oracle (the original rule, or hand labels);
/// `candidate` is the compiler's output.  The accept counts are a cheap sanity
/// signal on their own: a candidate that accepts *nothing* (`candidate_accepts ==
/// 0`) is a common mis-compile visible before reading any counterexample.
#[derive(Clone, Debug)]
pub struct Report {
    /// How many hands were drawn and evaluated
    pub tested: usize,
    /// How many hands the two predicates agreed on
    pub agreed: usize,
    /// How many hands the reference accepted (its accept rate over `tested`)
    pub reference_accepts: usize,
    /// How many hands the candidate accepted
    pub candidate_accepts: usize,
    /// A bounded sample of hands where the two disagreed (the witnesses)
    pub disagreements: Vec<Hand>,
}

impl Report {
    /// Whether the candidate matched the reference on every sampled hand
    #[must_use]
    pub fn agrees(&self) -> bool {
        self.disagreements.is_empty()
    }
}

/// An iterator of uniform random hands drawn from full deals
///
/// Each `full_deal` is a uniform shuffle, so its four hands are four uniform
/// 13-card hands (a sound population though not mutually independent — they
/// partition one deck).  Taking all four amortizes the shuffle across four
/// samples, which matters when `compare` draws several thousand.
pub(crate) fn random_hands(rng: &mut impl Rng) -> impl Iterator<Item = Hand> + '_ {
    core::iter::repeat_with(move || full_deal(rng))
        .flat_map(|deal| Seat::ALL.map(|seat| deal[seat]))
}

/// Sample `n` random hands and report where two predicates disagree
///
/// `reference` is intent, `candidate` is the compiler's output (see [`Report`]).
/// For a constraint, wrap it with [`predicate`]; for a book rule, pass
/// `|hand| rule.eval(hand, ctx).is_finite()`.  Up to `MAX_COUNTEREXAMPLES`
/// disagreeing hands are retained as witnesses; the counts always cover all `n`.
pub fn compare(
    reference: impl Fn(Hand) -> bool,
    candidate: impl Fn(Hand) -> bool,
    rng: &mut impl Rng,
    n: usize,
) -> Report {
    let mut report = Report {
        tested: 0,
        agreed: 0,
        reference_accepts: 0,
        candidate_accepts: 0,
        disagreements: Vec::new(),
    };

    for hand in random_hands(rng).take(n) {
        let want = reference(hand);
        let got = candidate(hand);
        report.tested += 1;
        report.reference_accepts += usize::from(want);
        report.candidate_accepts += usize::from(got);
        if want == got {
            report.agreed += 1;
        } else if report.disagreements.len() < MAX_COUNTEREXAMPLES {
            report.disagreements.push(hand);
        }
    }
    report
}

/// Check a constraint against hand-labeled intent, returning the failing hands
///
/// Each example is a hand and whether intent says the constraint should accept
/// it; a returned hand is one the constraint classified against its label.  This
/// is the oracle for meanings with no natural reference constraint — a handful of
/// textbook hands the author *knows* the right verdict for.
#[must_use]
pub fn check_examples(
    constraint: &impl Constraint,
    context: &Context<'_>,
    examples: &[(Hand, bool)],
) -> Vec<Hand> {
    examples
        .iter()
        .filter(|&&(hand, want)| accepts(constraint, hand, context) != want)
        .map(|&(hand, _)| hand)
        .collect()
}

/// An empty auction context — the default ground for a context-free comparison
///
/// Most constraints the compiler authors (shape, strength, every [`described`]
/// hand predicate) ignore the auction, so an empty, non-vulnerable context is the
/// natural place to verify them.
///
/// [`described`]: super::constraint::described
#[must_use]
pub fn empty_context() -> Context<'static> {
    Context::new(RelativeVulnerability::NONE, &[])
}

#[cfg(test)]
mod tests;
