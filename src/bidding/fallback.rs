//! Guarded fallbacks for auctions outside the book
//!
//! A [`Trie`][super::Trie] cannot enumerate competitive auctions literally —
//! interference multiplies sequences combinatorially.  Instead, any node may
//! carry an ordered list of fallbacks, each behind a [`Guard`].  When an
//! auction has no exact classifier, resolution walks back up from the deepest
//! reachable node and takes the first admitted fallback (see
//! [`Trie::resolve`][super::Trie::resolve]).
//!
//! A [`Fallback`] either classifies directly or *rebases*: it rewrites the
//! auction and resolves again.  Rebasing is the structural workhorse of
//! competitive bidding — "system on over their double" is one
//! [`ReplaceNext`]`(Pass)` entry instead of a copy of the whole book under
//! the double.

use super::context::Context;
use super::trie::Classifier;
use contract_bridge::Bid;
use contract_bridge::auction::Call;
use core::fmt;
use std::borrow::Cow;
use std::sync::Arc;

/// A machine-readable description of a [`Guard`]
///
/// This is deliberately conservative: guards which do not explicitly expose
/// one of the built-in shapes are [`Opaque`](Self::Opaque).
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuardPlan {
    /// Admit every uncovered auction.
    Always,
    /// Admit only while the opponents have not disturbed the auction.
    Undisturbed,
    /// Admit when the first uncovered call is the given call.
    FirstIs(Call),
    /// Admit a single uncovered overcall no higher than the given bid.
    OvercallAtMost(Bid),
    /// Admit exactly the given uncovered suffix.
    SuffixIs(Vec<Call>),
    /// No structured description is available.
    Opaque,
}

/// Trait deciding whether a fallback applies to an uncovered auction
pub trait Guard: Send + Sync {
    /// Whether the fallback applies
    ///
    /// `suffix` is the part of the auction below the node holding the
    /// fallback — the calls the book did not cover.
    fn admits(&self, context: &Context<'_>, suffix: &[Call]) -> bool;

    /// Human-readable admit condition for the book renderers
    ///
    /// [`None`] (the default, inherited by closure guards) renders as an
    /// unlabeled section; wrap the closure in [`described_guard`] to name it.
    fn describe(&self) -> Option<String> {
        None
    }

    /// Machine-readable admit condition for internal decoders
    #[doc(hidden)]
    fn plan(&self) -> GuardPlan {
        GuardPlan::Opaque
    }
}

impl fmt::Debug for dyn Guard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Guard({:p})", &self)
    }
}

/// Closures are natural guards
impl<F> Guard for F
where
    F: Fn(&Context<'_>, &[Call]) -> bool + Send + Sync,
{
    fn admits(&self, context: &Context<'_>, suffix: &[Call]) -> bool {
        self(context, suffix)
    }
}

/// Coerce a closure into a [`Guard`]
///
/// Like [`classifier`][super::trie::classifier], this identity function
/// provides the expected signature that the compiler cannot generalize on
/// its own.
pub const fn guard<F>(f: F) -> F
where
    F: Fn(&Context<'_>, &[Call]) -> bool + Send + Sync,
{
    f
}

/// Guard admitting every auction
///
/// At the root of a trie, this is the global default: a system whose root
/// carries an `Always` fallback never falls off the book.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Always;

impl Guard for Always {
    fn admits(&self, _: &Context<'_>, _: &[Call]) -> bool {
        true
    }

    fn describe(&self) -> Option<String> {
        Some("(always)".into())
    }

    fn plan(&self) -> GuardPlan {
        GuardPlan::Always
    }
}

/// Guard admitting auctions the opponents have not disturbed
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Undisturbed;

impl Guard for Undisturbed {
    fn admits(&self, context: &Context<'_>, _: &[Call]) -> bool {
        context.undisturbed()
    }

    fn describe(&self) -> Option<String> {
        Some("(undisturbed)".into())
    }

    fn plan(&self) -> GuardPlan {
        GuardPlan::Undisturbed
    }
}

/// Guard admitting auctions whose first uncovered call is the given one
///
/// `FirstIs(Call::Double)` together with a [`ReplaceNext`] rebase expresses
/// "system on over their double" for the entire subtree below a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FirstIs(
    /// The expected first uncovered call
    pub Call,
);

impl Guard for FirstIs {
    fn admits(&self, _: &Context<'_>, suffix: &[Call]) -> bool {
        suffix.first() == Some(&self.0)
    }

    fn describe(&self) -> Option<String> {
        Some(format!("{} …", self.0))
    }

    fn plan(&self) -> GuardPlan {
        GuardPlan::FirstIs(self.0)
    }
}

/// Guard admitting exactly one uncovered call: a bid at most the given one
///
/// This is the natural guard for a competitive package (e.g. negative
/// doubles through 2♠) handling the call directly over an overcall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OvercallAtMost(
    /// The highest admitted overcall
    pub Bid,
);

impl Guard for OvercallAtMost {
    fn admits(&self, _: &Context<'_>, suffix: &[Call]) -> bool {
        matches!(*suffix, [Call::Bid(bid)] if bid <= self.0)
    }

    fn describe(&self) -> Option<String> {
        Some(format!("(overcall ≤{})", self.0))
    }

    fn plan(&self) -> GuardPlan {
        GuardPlan::OvercallAtMost(self.0)
    }
}

/// Guard admitting exactly the given uncovered suffix
///
/// The workhorse of authored competitive continuations: the node names our
/// side's key, the suffix pins the exact calls (theirs and ours) below it.
/// Self-describing — the renderers print the suffix as more auction, so these
/// sections read like ordinary book keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuffixIs(
    /// The expected uncovered suffix
    pub Vec<Call>,
);

impl Guard for SuffixIs {
    fn admits(&self, _: &Context<'_>, suffix: &[Call]) -> bool {
        suffix == self.0.as_slice()
    }

    fn describe(&self) -> Option<String> {
        Some(contract_bridge::auction::display_calls(&self.0).to_string())
    }

    fn plan(&self) -> GuardPlan {
        GuardPlan::SuffixIs(self.0.clone())
    }
}

/// A machine-readable description of a [`Rewrite`]
///
/// This is deliberately conservative: rewrites which do not explicitly
/// expose a built-in shape are [`Opaque`](Self::Opaque).
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RewritePlan {
    /// Replace the first uncovered call with the given call.
    ReplaceNext(Call),
    /// No structured description is available.
    Opaque,
}

/// Trait rewriting an auction for re-resolution
pub trait Rewrite: Send + Sync {
    /// Rewrite the auction, or return [`None`] when inapplicable
    ///
    /// `depth` is the depth of the node holding the rebase, i.e. the index
    /// of the first uncovered call.  Returning [`None`] skips this fallback
    /// and resolution continues with the next one.
    fn rewrite(&self, auction: &[Call], depth: usize) -> Option<Vec<Call>>;

    /// Human-readable summary for the book renderers
    ///
    /// [`None`] (the default, inherited by closure rewrites) renders as an
    /// opaque rebase; wrap the closure in [`described_rewrite`] to name it.
    fn describe(&self) -> Option<String> {
        None
    }

    /// Machine-readable rewrite for internal decoders
    #[doc(hidden)]
    fn plan(&self) -> RewritePlan {
        RewritePlan::Opaque
    }
}

impl fmt::Debug for dyn Rewrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rewrite({:p})", &self)
    }
}

/// Closures are natural rewrites
impl<F> Rewrite for F
where
    F: Fn(&[Call], usize) -> Option<Vec<Call>> + Send + Sync,
{
    fn rewrite(&self, auction: &[Call], depth: usize) -> Option<Vec<Call>> {
        self(auction, depth)
    }
}

/// Coerce a closure into a [`Rewrite`]
///
/// Like [`classifier`][super::trie::classifier], this identity function
/// provides the expected signature that the compiler cannot generalize on
/// its own.
pub const fn rewriter<F>(f: F) -> F
where
    F: Fn(&[Call], usize) -> Option<Vec<Call>> + Send + Sync,
{
    f
}

/// Rewrite replacing the first uncovered call
///
/// `ReplaceNext(Call::Pass)` maps every continuation after an uncovered call
/// onto the corresponding continuation after a pass — the "system on"
/// rewrite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplaceNext(
    /// The replacement call
    pub Call,
);

impl Rewrite for ReplaceNext {
    fn rewrite(&self, auction: &[Call], depth: usize) -> Option<Vec<Call>> {
        (depth < auction.len()).then(|| {
            let mut rewritten = auction.to_vec();
            rewritten[depth] = self.0;
            rewritten
        })
    }

    fn describe(&self) -> Option<String> {
        Some(format!("systems on: their call is treated as {}", self.0))
    }

    fn plan(&self) -> RewritePlan {
        RewritePlan::ReplaceNext(self.0)
    }
}

/// A [`Guard`] or [`Rewrite`] carrying a renderer label
///
/// Closures can't describe themselves; this wrapper names them for the book
/// renderers without touching their behavior.  The dual of
/// [`described`][super::constraint::described] for constraints.
struct Described<T> {
    inner: T,
    label: Cow<'static, str>,
}

impl<G: Guard> Guard for Described<G> {
    fn admits(&self, context: &Context<'_>, suffix: &[Call]) -> bool {
        self.inner.admits(context, suffix)
    }

    fn describe(&self) -> Option<String> {
        Some(self.label.clone().into_owned())
    }

    fn plan(&self) -> GuardPlan {
        self.inner.plan()
    }
}

impl<R: Rewrite> Rewrite for Described<R> {
    fn rewrite(&self, auction: &[Call], depth: usize) -> Option<Vec<Call>> {
        self.inner.rewrite(auction, depth)
    }

    fn describe(&self) -> Option<String> {
        Some(self.label.clone().into_owned())
    }

    fn plan(&self) -> RewritePlan {
        self.inner.plan()
    }
}

/// Label a [`Guard`] for the book renderers
pub fn described_guard(
    label: impl Into<Cow<'static, str>>,
    inner: impl Guard + 'static,
) -> impl Guard + 'static {
    Described {
        inner,
        label: label.into(),
    }
}

/// Label a [`Rewrite`] for the book renderers
pub fn described_rewrite(
    label: impl Into<Cow<'static, str>>,
    inner: impl Rewrite + 'static,
) -> impl Rewrite + 'static {
    Described {
        inner,
        label: label.into(),
    }
}

/// Action taken when a guard admits an uncovered auction
#[derive(Clone, Debug)]
pub enum Fallback {
    /// Classify the hand directly
    Classify(Arc<dyn Classifier>),
    /// Rewrite the auction and resolve again
    Rebase(Arc<dyn Rewrite>),
}

impl Fallback {
    /// Wrap a classifier as a fallback
    pub fn classify(classifier: impl Classifier + 'static) -> Self {
        Self::Classify(Arc::new(classifier))
    }

    /// Wrap a rewrite as a fallback
    pub fn rebase(rewrite: impl Rewrite + 'static) -> Self {
        Self::Rebase(Arc::new(rewrite))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::Strain;
    use contract_bridge::auction::RelativeVulnerability;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: contract_bridge::Level::new(level),
            strain,
        })
    }

    fn empty_context() -> Context<'static> {
        Context::new(RelativeVulnerability::NONE, &[])
    }

    #[test]
    fn test_first_is() {
        let guard = FirstIs(Call::Double);
        let context = empty_context();
        assert!(guard.admits(&context, &[Call::Double]));
        assert!(guard.admits(&context, &[Call::Double, Call::Pass]));
        assert!(!guard.admits(&context, &[Call::Pass, Call::Double]));
        assert!(!guard.admits(&context, &[]));
    }

    #[test]
    fn test_overcall_at_most() {
        let guard = OvercallAtMost(Bid::new(2, Strain::Spades));
        let context = empty_context();
        assert!(guard.admits(&context, &[bid(1, Strain::Spades)]));
        assert!(guard.admits(&context, &[bid(2, Strain::Spades)]));
        assert!(!guard.admits(&context, &[bid(2, Strain::Notrump)]));
        assert!(!guard.admits(&context, &[Call::Double]));
        assert!(!guard.admits(&context, &[bid(1, Strain::Spades), Call::Pass]));
    }

    #[test]
    fn test_suffix_is() {
        let guard = SuffixIs(vec![bid(2, Strain::Hearts), Call::Double, Call::Pass]);
        let context = empty_context();
        assert!(guard.admits(
            &context,
            &[bid(2, Strain::Hearts), Call::Double, Call::Pass]
        ));
        assert!(!guard.admits(&context, &[bid(2, Strain::Hearts), Call::Double]));
        assert!(!guard.admits(&context, &[]));
        assert!(SuffixIs(vec![]).admits(&context, &[]));
        assert_eq!(guard.describe().expect("self-describing"), "2♥ X -");
    }

    #[test]
    fn test_described_wrappers() {
        let guard = described_guard("(their overcall) cue -", Always);
        let context = empty_context();
        assert!(guard.admits(&context, &[Call::Double]));
        assert_eq!(guard.describe().as_deref(), Some("(their overcall) cue -"));
        assert_eq!(guard.plan(), GuardPlan::Always);

        let rewrite = described_rewrite("systems on", ReplaceNext(Call::Pass));
        assert_eq!(rewrite.describe().as_deref(), Some("systems on"));
        assert_eq!(rewrite.plan(), RewritePlan::ReplaceNext(Call::Pass));
        assert_eq!(
            rewrite.rewrite(&[bid(1, Strain::Notrump), Call::Double], 1),
            Some(vec![bid(1, Strain::Notrump), Call::Pass]),
        );
    }

    #[test]
    fn test_replace_next() {
        let rewrite = ReplaceNext(Call::Pass);
        let auction = [bid(1, Strain::Notrump), Call::Double, Call::Pass];

        assert_eq!(
            rewrite.rewrite(&auction, 1),
            Some(vec![bid(1, Strain::Notrump), Call::Pass, Call::Pass]),
        );
        assert_eq!(rewrite.rewrite(&auction, 3), None);
    }

    #[test]
    fn test_builtin_plans() {
        let suffix = vec![bid(2, Strain::Hearts), Call::Double];

        assert_eq!(Always.plan(), GuardPlan::Always);
        assert_eq!(Undisturbed.plan(), GuardPlan::Undisturbed);
        assert_eq!(
            FirstIs(Call::Double).plan(),
            GuardPlan::FirstIs(Call::Double),
        );
        assert_eq!(
            OvercallAtMost(Bid::new(2, Strain::Spades)).plan(),
            GuardPlan::OvercallAtMost(Bid::new(2, Strain::Spades)),
        );
        assert_eq!(SuffixIs(suffix.clone()).plan(), GuardPlan::SuffixIs(suffix),);
        assert_eq!(
            ReplaceNext(Call::Pass).plan(),
            RewritePlan::ReplaceNext(Call::Pass),
        );
    }

    #[test]
    fn test_closure_plans_are_opaque() {
        let closure_guard = guard(|_: &Context<'_>, suffix: &[Call]| suffix.is_empty());
        let closure_rewrite = rewriter(|auction: &[Call], _: usize| Some(auction.to_vec()));

        assert_eq!(closure_guard.plan(), GuardPlan::Opaque);
        assert_eq!(closure_rewrite.plan(), RewritePlan::Opaque);

        let context = empty_context();
        assert!(closure_guard.admits(&context, &[]));
        assert_eq!(
            closure_rewrite.rewrite(&[Call::Pass], 0),
            Some(vec![Call::Pass]),
        );
    }
}
