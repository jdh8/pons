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
use super::constraint::{
    Constraint, ConstraintDependencies, Description, ProjectionDependencies, ProjectionKind,
};
use super::context::Context;
use super::inference::{Envelope, EnvelopeUnion, ReadingProfile, reading_profile};
use super::trie::Classifier;
use contract_bridge::Hand;
use contract_bridge::auction::Call;
use core::fmt;
use std::collections::HashMap;
use std::sync::Arc;

/// A per-call alert: the name of the artificial convention a rule's call shows
///
/// In real bridge an artificial call is *alerted* so the opponents read it as the
/// convention rather than as natural.  Per-call alerts are the whole of a
/// system's identity — even a strong club announces itself through its
/// opening's own alert.  Here an alert does two jobs: it is the build-time
/// **gate** (`[`Rules::alert`]` stamps a block, [`Rules::gated`] ships only the
/// active variant), and it marks a call as artificial so the inference reader
/// suppresses the natural single-suit reading and projects the convention instead.
///
/// The newtype is open — each convention mints its own alert as a constant, such
/// as `const STAYMAN: Alert = Alert("stayman");`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Alert(pub &'static str);

/// A face-of-auction predicate gating a rule's liveness (see [`Rules::face`])
type FaceGate = Arc<dyn Fn(&Context<'_>) -> bool + Send + Sync>;

/// Identity of a pure, hand-independent face recognizer.
///
/// Public [`Rules::face`] predicates stay opaque and are evaluated at every
/// authored occurrence.  Internal tables may opt a recognizer into this
/// identity contract when repeated occurrences are exactly the same pure
/// predicate.  The compiled executor can then evaluate it lazily at its first
/// declaration-order occurrence and reuse that bit for the rest of one
/// immutable decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct FaceId {
    family: &'static str,
    variant: u8,
}

impl FaceId {
    pub(crate) const fn new(family: &'static str, variant: u8) -> Self {
        Self { family, variant }
    }
}

/// A single bidding rule: a call justified by a constraint
#[derive(Clone)]
pub struct Rule {
    call: Call,
    weight: f32,
    when: Arc<dyn Constraint>,
    label: &'static str,
    alert: Option<Alert>,
    face: Option<FaceGate>,
    face_id: Option<FaceId>,
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

    /// Whether this rule is live on the current face of the auction
    ///
    /// A rule with a [`Rules::face`] gate exists only on faces where the gate
    /// holds — bidder ([`eval`][Self::eval]) and reader (the inference
    /// consult sites) both check it, so the two cannot drift.  Ungated rules
    /// are always live.
    #[must_use]
    pub fn face_live(&self, context: &Context<'_>) -> bool {
        self.face.as_ref().is_none_or(|face| face(context))
    }

    /// The logit this rule contributes for a hand
    #[must_use]
    pub fn eval(&self, hand: Hand, context: &Context<'_>) -> f32 {
        if !self.face_live(context) {
            return f32::NEG_INFINITY;
        }
        self.weight + self.when.eval(hand, context)
    }

    /// Runtime facts this rule's bidder-side evaluation may consult.
    ///
    /// An opaque face gate is conservatively treated as depending on every
    /// fact.  It remains evaluated before the constraint, exactly as in
    /// [`eval`][Self::eval].
    #[must_use]
    pub fn dependencies(&self) -> ConstraintDependencies {
        if self.face.is_some() {
            ConstraintDependencies::ALL
        } else {
            self.when.dependencies()
        }
    }

    /// Runtime dependencies of the constraint's four reading folds.
    #[must_use]
    pub fn projection_dependencies(&self) -> ProjectionDependencies {
        self.when.projection_dependencies()
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

    /// The forward [`Envelope`] this rule's constraint implies
    ///
    /// The reading-side dual of [`eval`][Self::eval]: where `eval` scores a
    /// known hand, `project` reports the per-suit length and point ranges every
    /// hand the rule accepts must fall within — what a *partner* who saw only
    /// the call may assert.  Mirrors [`describe`][Self::describe] (both delegate
    /// to the constraint fold); sound by construction (see
    /// [`Constraint::project`]).
    #[must_use]
    pub fn project(&self, context: &Context<'_>) -> Envelope {
        // ponytail: hull the envelope union to a single box, so the
        // alert/`artificial` checks and `authored_reading` stay on `Envelope`.
        // The overlay that the sampler consumes uses
        // [`project_union`][Self::project_union] to keep the boxes when
        // `envelope_union_reading` is on.
        self.when.project(context).hull()
    }

    /// The forward reading as a union of boxes — [`project`][Self::project]
    /// without the hull
    ///
    /// The overlay [`Inferences::read`][super::inference::Inferences] feeds the
    /// sampler; keeps the disjunctive boxes under
    /// [`set_envelope_union_reading`][super::set_envelope_union_reading]
    /// (off → one box, the hull).
    #[must_use]
    pub fn project_union(&self, context: &Context<'_>) -> super::inference::EnvelopeUnion {
        self.when.project(context)
    }

    /// The **two-sided** envelope of this rule's constraint — floors and
    /// ceilings ([`Constraint::project_band`])
    ///
    /// What a *declined* call asserts: a passed hand satisfied some Pass
    /// rule's gate, so it lies within the union of the gates' bands.  The
    /// reading-side fold behind [`set_pass_reading`][super::set_pass_reading].
    #[must_use]
    pub fn project_band(&self, context: &Context<'_>) -> Envelope {
        self.when.project_band(context).hull()
    }

    /// The two-sided band as a union of boxes — [`project_band`][Self::project_band]
    /// without the hull (the envelope-union overlay's Pass reading)
    #[must_use]
    pub fn project_band_union(&self, context: &Context<'_>) -> super::inference::EnvelopeUnion {
        self.when.project_band(context)
    }

    /// The two-sided band of this rule's **negation** as a union of boxes
    /// ([`Constraint::project_complement`])
    ///
    /// What *declining* this rule asserts: under argmax selection a hand
    /// inside a gate that outweighs every Pass rule cannot have passed, so
    /// the passer lies in the gate's complement.  The reading-side fold
    /// behind [`set_pass_exclusion_reading`][super::set_pass_exclusion_reading].
    #[must_use]
    pub fn project_complement_union(
        &self,
        context: &Context<'_>,
    ) -> super::inference::EnvelopeUnion {
        self.when.project_complement(context)
    }

    /// The **agreement** this rule announces, as a union of boxes
    /// ([`Constraint::announce`])
    ///
    /// The disclosure twin of [`project_union`][Self::project_union].  Identical to
    /// it unless the rule's constraint used
    /// [`announced`][super::constraint::announced], which is what keeps the two
    /// overlays byte-identical everywhere the split is not deliberately taken.
    #[must_use]
    pub fn announce_union(&self, context: &Context<'_>) -> super::inference::EnvelopeUnion {
        self.when.announce(context)
    }
}

impl fmt::Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rule")
            .field("call", &self.call)
            .field("weight", &self.weight)
            .field("label", &self.label)
            .field("alert", &self.alert)
            .field("face", &self.face.is_some())
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

    /// Wrap one [`Rule`] as a singleton list
    ///
    /// The regrouping seam for the row layer ([`rows`][super::rows]): a table
    /// split into rows reassembles through [`chain`][Self::chain] into the
    /// same rule list.
    #[must_use]
    pub(crate) fn of(rule: Rule) -> Self {
        Self { rules: vec![rule] }
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
            face: None,
            face_id: None,
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

    /// Gate the most recently added rule on a face-of-auction predicate
    ///
    /// Chains after [`rule`][Self::rule], mirroring [`alert`][Self::alert].
    /// Where the gate fails, the rule is as-if-absent: [`Rule::eval`] returns
    /// −∞ and the inference reader skips it, so a conditionally-artificial
    /// call (e.g. a Kickback 4♠ that is only an ask when the ladder proves
    /// one) never poisons the natural reading of the same call on other
    /// faces.
    ///
    /// # Panics
    ///
    /// Panics if no rule has been added yet.
    #[must_use]
    pub fn face(mut self, face: impl Fn(&Context<'_>) -> bool + Send + Sync + 'static) -> Self {
        let rule = self
            .rules
            .last_mut()
            .expect("face() requires a preceding rule()");
        rule.face = Some(Arc::new(face));
        rule.face_id = None;
        self
    }

    /// Mark the most recent rule with a repeated pure face recognizer.
    ///
    /// `id` is a semantic promise: every occurrence of the same identity in a
    /// stance must return the same bit for one [`Context`].  Unlike the public
    /// opaque [`Self::face`] escape hatch, compiled serving may reuse that bit
    /// across rules and a subsequent explanation of the same decision.
    #[must_use]
    pub(crate) fn shared_face(
        mut self,
        id: FaceId,
        face: impl Fn(&Context<'_>) -> bool + Send + Sync + 'static,
    ) -> Self {
        let rule = self
            .rules
            .last_mut()
            .expect("shared_face() requires a preceding rule()");
        rule.face = Some(Arc::new(face));
        rule.face_id = Some(id);
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

    /// Compile an immutable sidecar plan for this authored rule list.
    ///
    /// The returned plan borrows nothing and retains stable authored indices;
    /// callers keep this `Rules` value as the source of constraints, labels,
    /// alerts, and explanations.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn compile(&self, context: &Context<'_>) -> CompiledRules {
        CompiledRules::compile(self, context)
    }
}

/// Stable index of a rule in its authored [`Rules`] list.
pub(crate) type RuleIndex = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct IndexRange {
    start: RuleIndex,
    end: RuleIndex,
}

impl IndexRange {
    fn from_bounds(start: usize, end: usize) -> Self {
        Self {
            start: RuleIndex::try_from(start).expect("a rule table fits in u32 indices"),
            end: RuleIndex::try_from(end).expect("a rule table fits in u32 indices"),
        }
    }

    fn slice(self, indices: &[RuleIndex]) -> &[RuleIndex] {
        &indices[self.start as usize..self.end as usize]
    }
}

/// One call's authored rules, stored as ranges into compact shared index pools.
#[derive(Clone, Debug)]
pub(crate) struct CompiledCallPlan {
    call: Call,
    rules: IndexRange,
    alerted: IndexRange,
    max_weight: f32,
}

impl CompiledCallPlan {
    /// The call this group classifies/projects.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn call(&self) -> Call {
        self.call
    }

    /// The authored-order maximum rule weight for this call.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn max_weight(&self) -> f32 {
        self.max_weight
    }
}

/// Pass-specific exclusion inputs.
///
/// Pass indices deliberately include face-gated rules without checking their
/// face, matching the existing `project_pass` quirk.  Stronger siblings are
/// likewise selected solely by authored call and strict weight comparison.
#[derive(Clone, Debug)]
pub(crate) struct CompiledPassPlan {
    rules: IndexRange,
    #[allow(dead_code)] // retained as authored pass-plan metadata
    max_weight: f32,
    stronger_nonpass: Box<[RuleIndex]>,
}

impl CompiledPassPlan {
    /// The exact authored-order `f32::max` fold over Pass weights.
    #[must_use]
    #[cfg(test)]
    pub(crate) const fn max_weight(&self) -> f32 {
        self.max_weight
    }

    /// Non-Pass rules whose weight is strictly above the Pass ceiling.
    #[must_use]
    pub(crate) fn stronger_nonpass_indices(&self) -> &[RuleIndex] {
        &self.stronger_nonpass
    }
}

/// Index into a table-wide intern pool; `u32::MAX` denotes a live virtual fold.
#[derive(Clone, Copy, Debug)]
struct ProjectionPlan(RuleIndex);

/// Whole-stance pool of projections already proved context-independent.
///
/// Row tables are often cloned into several concrete routes while retaining
/// the same constraint `Arc`. Computing and owning those boxes once keeps
/// eager `Pair::against` compilation within its construction and memory gates.
#[derive(Default)]
pub(crate) struct ProjectionCache {
    values: HashMap<(usize, u8), Arc<EnvelopeUnion>>,
}

/// Stance-wide slot assignment for explicitly shareable face recognizers.
#[derive(Default)]
pub(crate) struct FaceRegistry {
    slots: HashMap<FaceId, u32>,
}

impl FaceRegistry {
    fn slot(&mut self, id: FaceId) -> u32 {
        let next = u32::try_from(self.slots.len()).expect("too many compiled face predicates");
        *self.slots.entry(id).or_insert(next)
    }

    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }
}

impl ProjectionPlan {
    const DYNAMIC: Self = Self(RuleIndex::MAX);

    // Keep these inputs explicit: the dependency/purity promises, finalized
    // context, and two stance-wide pools have independent semantics.
    #[allow(clippy::too_many_arguments)]
    fn compile(
        rule: &Rule,
        kind: ProjectionKind,
        eager: bool,
        dependencies: ConstraintDependencies,
        pure: bool,
        context: &Context<'_>,
        constants: &mut Vec<Arc<EnvelopeUnion>>,
        cache: &mut ProjectionCache,
    ) -> Self {
        if !eager || !pure || !dependencies.projection_context_independent() {
            return Self::DYNAMIC;
        }
        let constraint = Arc::as_ptr(&rule.when).cast::<()>() as usize;
        let value = Arc::clone(
            cache
                .values
                .entry((constraint, kind as u8))
                .or_insert_with(|| {
                    Arc::new(match kind {
                        ProjectionKind::Forward => rule.project_union(context),
                        ProjectionKind::Band => rule.project_band_union(context),
                        ProjectionKind::Complement => rule.project_complement_union(context),
                        ProjectionKind::Announcement => rule.announce_union(context),
                    })
                }),
        );
        let index = constants
            .iter()
            .position(|candidate| Arc::ptr_eq(candidate, &value))
            .unwrap_or_else(|| {
                constants.push(value);
                constants.len() - 1
            });
        Self(RuleIndex::try_from(index).expect("a projection pool fits in u32 indices"))
    }

    const fn is_constant(&self) -> bool {
        self.0 != RuleIndex::MAX
    }

    fn constant<'a>(&self, constants: &'a [Arc<EnvelopeUnion>]) -> Option<&'a EnvelopeUnion> {
        self.is_constant()
            .then(|| constants[self.0 as usize].as_ref())
    }
}

#[derive(Clone, Copy, Debug)]
enum CompiledFacePlan {
    Always,
    Opaque,
    Shared(u32),
}

/// Stack-first memo for repeated explicit face identities under one identical
/// context (for example, the three projection folds of one authored call).
///
/// The shipped stance has fewer than 64 identities, so two bitsets cover its
/// memo without clearing a large stack array at every historical call. A
/// future convention with more identities spills safely; the order of first
/// evaluation remains authored order in either representation.
pub(crate) struct FaceMemo {
    known: u64,
    live: u64,
    overflow: Vec<(u32, bool)>,
}

impl FaceMemo {
    pub(crate) const fn new() -> Self {
        Self {
            known: 0,
            live: 0,
            overflow: Vec::new(),
        }
    }

    fn get(&self, slot: u32) -> Option<bool> {
        if let Some(bit) = 1_u64.checked_shl(slot) {
            return ((self.known & bit) != 0).then_some((self.live & bit) != 0);
        }
        self.overflow
            .iter()
            .find_map(|&(candidate, value)| (candidate == slot).then_some(value))
    }

    fn insert(&mut self, slot: u32, live: bool) {
        if let Some(bit) = 1_u64.checked_shl(slot) {
            self.known |= bit;
            if live {
                self.live |= bit;
            }
            return;
        }
        self.overflow.push((slot, live));
    }
}

impl Default for FaceMemo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
struct CompiledRulePlan {
    authored_index: RuleIndex,
    call: Call,
    weight_bits: u32,
    dependencies: ConstraintDependencies,
    face: CompiledFacePlan,
    projection_dependencies: ProjectionDependencies,
    projections: [ProjectionPlan; 4],
}

impl CompiledRulePlan {
    // The compiler inputs stay separate here because each has distinct
    // ownership and profile semantics; bundling them would only hide those
    // invariants behind an ad-hoc parameter object.
    #[allow(clippy::too_many_arguments)]
    fn compile(
        index: usize,
        rule: &Rule,
        context: &Context<'_>,
        constants: &mut Vec<Arc<EnvelopeUnion>>,
        cache: &mut ProjectionCache,
        faces: &mut FaceRegistry,
        profile: ReadingProfile,
        eager_forward: bool,
    ) -> Self {
        let projection_dependencies = rule.projection_dependencies();
        let projections = [
            ProjectionKind::Forward,
            ProjectionKind::Band,
            ProjectionKind::Complement,
            ProjectionKind::Announcement,
        ]
        .map(|kind| {
            let eager = match kind {
                ProjectionKind::Forward => eager_forward,
                ProjectionKind::Band => rule.call == Call::Pass && profile.pass_reading(),
                ProjectionKind::Complement => profile.pass_exclusion_reading(),
                ProjectionKind::Announcement => rule.alert.is_some() && profile.announced_reading(),
            };
            ProjectionPlan::compile(
                rule,
                kind,
                eager,
                projection_dependencies.get(kind),
                projection_dependencies.is_pure(),
                context,
                constants,
                cache,
            )
        });
        Self {
            authored_index: RuleIndex::try_from(index).expect("a rule table fits in u32 indices"),
            call: rule.call,
            weight_bits: rule.weight.to_bits(),
            dependencies: rule.dependencies(),
            face: match (&rule.face, rule.face_id) {
                (None, _) => CompiledFacePlan::Always,
                (Some(_), Some(id)) => CompiledFacePlan::Shared(faces.slot(id)),
                (Some(_), None) => CompiledFacePlan::Opaque,
            },
            projection_dependencies,
            projections,
        }
    }

    const fn projection(&self, kind: ProjectionKind) -> &ProjectionPlan {
        &self.projections[kind as usize]
    }

    fn face_live(&self, rule: &Rule, context: &Context<'_>, memo: &mut FaceMemo) -> bool {
        match self.face {
            CompiledFacePlan::Always => true,
            CompiledFacePlan::Opaque => rule.face_live(context),
            CompiledFacePlan::Shared(slot) => {
                if let Some(live) = memo.get(slot) {
                    return live;
                }
                let live = context.compiled_rule_face(slot, || rule.face_live(context));
                memo.insert(slot, live);
                live
            }
        }
    }

    fn eval(&self, rule: &Rule, hand: Hand, context: &Context<'_>, memo: &mut FaceMemo) -> f32 {
        if !self.face_live(rule, context, memo) {
            return f32::NEG_INFINITY;
        }
        rule.weight + rule.when.eval(hand, context)
    }
}

/// Immutable execution and reading plan for an authored [`Rules`] table.
///
/// The sidecar owns only compact indices and frozen context-independent
/// projections.  It intentionally does not clone constraints or replace the
/// authored table.  Classification still walks every rule globally in
/// declaration order, preserving face-first evaluation, eager constraint
/// operands, floating-point folding, and explanation tie identity.
#[derive(Clone)]
pub(crate) struct CompiledRules {
    #[cfg(test)]
    profile: ReadingProfile,
    rules: Box<[CompiledRulePlan]>,
    grouped_indices: Box<[RuleIndex]>,
    alerted_indices: Box<[RuleIndex]>,
    calls: Box<[CompiledCallPlan]>,
    pass: Option<CompiledPassPlan>,
    constant_projections: Box<[Arc<EnvelopeUnion>]>,
    dependencies: ConstraintDependencies,
}

impl fmt::Debug for CompiledRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledRules")
            .field("rules", &self.rules.len())
            .field("calls", &self.calls)
            .field("pass", &self.pass)
            .field("constant_projections", &self.constant_projections.len())
            .field("dependencies", &self.dependencies)
            .finish_non_exhaustive()
    }
}

impl CompiledRules {
    /// Compile `authored`, freezing only projections whose dependency hook
    /// proves them context-independent.  Profile-sensitive constants are
    /// guarded by the captured [`ReadingProfile`].
    #[must_use]
    #[cfg(test)]
    pub(crate) fn compile(authored: &Rules, context: &Context<'_>) -> Self {
        Self::compile_with_cache(
            authored,
            context,
            &mut ProjectionCache::default(),
            &mut FaceRegistry::default(),
        )
    }

    /// Compile while interning context-independent projections across every
    /// rule table in one finalized stance.
    #[must_use]
    pub(crate) fn compile_with_cache(
        authored: &Rules,
        context: &Context<'_>,
        cache: &mut ProjectionCache,
        faces: &mut FaceRegistry,
    ) -> Self {
        let profile = reading_profile();
        let mut by_call: Map<usize> = Map::new();
        let mut alerted_by_call: Map<usize> = Map::new();
        let mut max_weight_by_call: Map<f32> = Map::new();
        for rule in &authored.rules {
            *by_call.entry(rule.call).get_or_insert(0) += 1;
            if rule.alert.is_some() {
                *alerted_by_call.entry(rule.call).get_or_insert(0) += 1;
            }
            let maximum = max_weight_by_call.entry(rule.call);
            *maximum = Some(maximum.map_or(rule.weight, |value| value.max(rule.weight)));
        }

        let mut rules = Vec::with_capacity(authored.rules.len());
        let mut constant_projections = Vec::new();
        let mut dependencies = ConstraintDependencies::NONE;

        for (index, rule) in authored.rules.iter().enumerate() {
            let index = RuleIndex::try_from(index).expect("a rule table fits in u32 indices");
            let call_is_alerted = rule.call != Call::Pass
                && alerted_by_call
                    .get(rule.call)
                    .is_some_and(|&count| count > 0);
            let plan = CompiledRulePlan::compile(
                index as usize,
                rule,
                context,
                &mut constant_projections,
                cache,
                faces,
                profile,
                profile.decodes_nonpass(call_is_alerted),
            );
            dependencies = dependencies | plan.dependencies;
            rules.push(plan);
        }

        let mut grouped_indices = vec![0; authored.rules.len()];
        let mut alerted_indices = vec![0; alerted_by_call.values().copied().sum()];
        let mut calls = Vec::with_capacity(by_call.keys().count());
        let mut grouped_next: Map<usize> = Map::new();
        let mut alerted_next: Map<usize> = Map::new();
        let mut rules_start = 0;
        let mut alerts_start = 0;
        for (call, &count) in &by_call {
            let rules_range = IndexRange::from_bounds(rules_start, rules_start + count);
            grouped_next.insert(call, rules_start);
            rules_start += count;

            let alert_count = alerted_by_call.get(call).copied().unwrap_or(0);
            let alerted_range = IndexRange::from_bounds(alerts_start, alerts_start + alert_count);
            alerted_next.insert(call, alerts_start);
            alerts_start += alert_count;

            calls.push(CompiledCallPlan {
                call,
                rules: rules_range,
                alerted: alerted_range,
                max_weight: *max_weight_by_call
                    .get(call)
                    .expect("a call group has a maximum"),
            });
        }
        for (index, rule) in authored.rules.iter().enumerate() {
            let index = RuleIndex::try_from(index).expect("a rule table fits in u32 indices");
            let grouped_slot = grouped_next
                .entry(rule.call)
                .as_mut()
                .expect("a rule call has a grouped range");
            grouped_indices[*grouped_slot] = index;
            *grouped_slot += 1;
            if rule.alert.is_some() {
                let alerted_slot = alerted_next
                    .entry(rule.call)
                    .as_mut()
                    .expect("an alerted call has an alerted range");
                alerted_indices[*alerted_slot] = index;
                *alerted_slot += 1;
            }
        }

        let pass = calls
            .iter()
            .find(|plan| plan.call == Call::Pass)
            .map(|plan| {
                let ceiling = plan.max_weight;
                let stronger_nonpass = authored
                    .rules
                    .iter()
                    .enumerate()
                    .filter(|(_, rule)| rule.call != Call::Pass && rule.weight > ceiling)
                    .map(|(index, _)| {
                        RuleIndex::try_from(index).expect("a rule table fits in u32 indices")
                    })
                    .collect();
                CompiledPassPlan {
                    rules: plan.rules,
                    max_weight: ceiling,
                    stronger_nonpass,
                }
            });

        Self {
            #[cfg(test)]
            profile,
            rules: rules.into_boxed_slice(),
            grouped_indices: grouped_indices.into_boxed_slice(),
            alerted_indices: alerted_indices.into_boxed_slice(),
            calls: calls.into_boxed_slice(),
            pass,
            constant_projections: constant_projections.into_boxed_slice(),
            dependencies,
        }
    }

    fn assert_compatible(&self, authored: &Rules) {
        debug_assert_eq!(
            self.rules.len(),
            authored.rules.len(),
            "CompiledRules used with a different authored table"
        );
    }

    fn rule<'a>(&self, authored: &'a Rules, index: RuleIndex) -> &'a Rule {
        self.assert_compatible(authored);
        let rule = &authored.rules[index as usize];
        let plan = &self.rules[index as usize];
        debug_assert_eq!(plan.authored_index, index);
        debug_assert_eq!(plan.call, rule.call);
        debug_assert_eq!(plan.weight_bits, rule.weight.to_bits());
        rule
    }

    /// One call's plan, if authored.
    #[must_use]
    pub(crate) fn call_plan(&self, call: Call) -> Option<&CompiledCallPlan> {
        self.calls.iter().find(|plan| plan.call == call)
    }

    /// Authored indices for `call`, in declaration order.
    #[must_use]
    pub(crate) fn rule_indices(&self, call: Call) -> &[RuleIndex] {
        self.call_plan(call)
            .map_or(&[], |plan| plan.rules.slice(&self.grouped_indices))
    }

    /// Alerted authored indices for `call`, in declaration order.
    #[must_use]
    pub(crate) fn alerted_rule_indices(&self, call: Call) -> &[RuleIndex] {
        self.call_plan(call)
            .map_or(&[], |plan| plan.alerted.slice(&self.alerted_indices))
    }

    /// Pass exclusion plan, if this table authors Pass.
    #[must_use]
    pub(crate) const fn pass_plan(&self) -> Option<&CompiledPassPlan> {
        self.pass.as_ref()
    }

    /// Pass indices in authored order, deliberately unfiltered by face.
    #[must_use]
    pub(crate) fn pass_rule_indices(&self) -> &[RuleIndex] {
        self.pass
            .as_ref()
            .map_or(&[], |pass| pass.rules.slice(&self.grouped_indices))
    }

    /// Whether dropping an unread Pass effect can omit only explicitly pure
    /// projection folds. Pass projection does not consult rule faces.
    pub(crate) fn can_skip_pass_effect(&self, pass_exclusion: bool) -> bool {
        let Some(pass) = &self.pass else {
            return true;
        };
        pass.rules
            .slice(&self.grouped_indices)
            .iter()
            .all(|&index| self.rules[index as usize].projection_dependencies.is_pure())
            && (!pass_exclusion
                || pass
                    .stronger_nonpass
                    .iter()
                    .all(|&index| self.rules[index as usize].projection_dependencies.is_pure()))
    }

    /// Whether dropping an unread non-Pass effect can omit only explicit pure
    /// faces and forward projections. Public `Rules::face` callbacks remain
    /// observable and therefore keep the legacy reader walk.
    pub(crate) fn can_skip_nonpass_effect(&self, call: Call) -> bool {
        self.rule_indices(call).iter().all(|&index| {
            let plan = &self.rules[index as usize];
            !matches!(plan.face, CompiledFacePlan::Opaque) && plan.projection_dependencies.is_pure()
        })
    }

    /// Whether one authored effect is observationally pure and may be retained
    /// by the append-only deal cache instead of replayed on every decision.
    ///
    /// This deliberately shares the stricter proof used by the structural skip
    /// paths: public face callbacks and opaque custom projection folds remain
    /// observable, while explicitly shared faces and pure folds are reusable.
    pub(crate) fn can_reuse_authored_effect(&self, call: Call, pass_exclusion: bool) -> bool {
        if call == Call::Pass {
            self.can_skip_pass_effect(pass_exclusion)
        } else {
            self.can_skip_nonpass_effect(call)
        }
    }

    /// Whether one rule's face is live with a caller-owned memo scoped to one
    /// identical context. This is the projection path's no-allocation
    /// once-per-effect cache; opaque faces deliberately bypass it.
    pub(crate) fn face_live_memoized(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
        memo: &mut FaceMemo,
    ) -> bool {
        let rule = self.rule(authored, index);
        self.rules[index as usize].face_live(rule, context, memo)
    }

    /// One rule projection fold's dependency mask.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn projection_dependencies(
        &self,
        index: RuleIndex,
        kind: ProjectionKind,
    ) -> ConstraintDependencies {
        self.rules[index as usize].projection_dependencies.get(kind)
    }

    /// Whether a fold was frozen under this plan's captured profile.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn projection_is_constant(&self, index: RuleIndex, kind: ProjectionKind) -> bool {
        self.rules[index as usize].projection(kind).is_constant()
    }

    #[cfg(test)]
    fn projection(
        &self,
        authored: &Rules,
        index: RuleIndex,
        kind: ProjectionKind,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        let rule = self.rule(authored, index);
        if self.profile == reading_profile()
            && let Some(projected) = self.rules[index as usize]
                .projection(kind)
                .constant(&self.constant_projections)
        {
            return projected.clone();
        }
        match kind {
            ProjectionKind::Forward => rule.project_union(context),
            ProjectionKind::Band => rule.project_band_union(context),
            ProjectionKind::Complement => rule.project_complement_union(context),
            ProjectionKind::Announcement => rule.announce_union(context),
        }
    }

    #[inline]
    fn projection_matched(
        &self,
        authored: &Rules,
        index: RuleIndex,
        kind: ProjectionKind,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        let rule = self.rule(authored, index);
        if let Some(projected) = self.rules[index as usize]
            .projection(kind)
            .constant(&self.constant_projections)
        {
            return projected.clone();
        }
        match kind {
            ProjectionKind::Forward => rule.project_union(context),
            ProjectionKind::Band => rule.project_band_union(context),
            ProjectionKind::Complement => rule.project_complement_union(context),
            ProjectionKind::Announcement => rule.announce_union(context),
        }
    }

    /// Forward projection when the registry has already matched the profile.
    pub(crate) fn project_union_matched(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection_matched(authored, index, ProjectionKind::Forward, context)
    }

    /// Pass band when the registry has already matched the profile.
    pub(crate) fn project_band_union_matched(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection_matched(authored, index, ProjectionKind::Band, context)
    }

    /// Complement when the registry has already matched the profile.
    pub(crate) fn project_complement_union_matched(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection_matched(authored, index, ProjectionKind::Complement, context)
    }

    /// Announcement when the registry has already matched the profile.
    pub(crate) fn announce_union_matched(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection_matched(authored, index, ProjectionKind::Announcement, context)
    }

    /// Resolve a rule's forward projection through its constant/dynamic plan.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn project_union(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection(authored, index, ProjectionKind::Forward, context)
    }

    /// Resolve a rule's two-sided band projection.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn project_band_union(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection(authored, index, ProjectionKind::Band, context)
    }

    /// Resolve a rule's complement projection.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn project_complement_union(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection(authored, index, ProjectionKind::Complement, context)
    }

    /// Resolve a rule's announced agreement projection.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn announce_union(
        &self,
        authored: &Rules,
        index: RuleIndex,
        context: &Context<'_>,
    ) -> EnvelopeUnion {
        self.projection(authored, index, ProjectionKind::Announcement, context)
    }

    /// Classify through the plan with bit-for-bit legacy folding.
    #[must_use]
    pub(crate) fn classify(&self, authored: &Rules, hand: Hand, context: &Context<'_>) -> Logits {
        self.assert_compatible(authored);
        let mut logits = Logits::new();
        let mut faces = FaceMemo::new();
        for plan in &self.rules {
            let rule = self.rule(authored, plan.authored_index);
            let slot = logits.0.get_mut(plan.call);
            *slot = slot.max(plan.eval(rule, hand, context, &mut faces));
        }
        logits
    }

    /// Explain through the plan while retaining the earliest authored tie.
    #[must_use]
    pub(crate) fn explain(
        &self,
        authored: &Rules,
        hand: Hand,
        context: &Context<'_>,
    ) -> Map<(usize, f32)> {
        self.assert_compatible(authored);
        let mut best = Map::new();
        let mut faces = FaceMemo::new();
        for plan in &self.rules {
            let rule = self.rule(authored, plan.authored_index);
            let logit = plan.eval(rule, hand, context, &mut faces);
            let entry = best.entry(plan.call);
            if logit > f32::NEG_INFINITY && entry.is_none_or(|(_, incumbent)| logit > incumbent) {
                entry.replace((plan.authored_index as usize, logit));
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
    use crate::bidding::constraint::{announced, balanced, hcp, len, pred, support};
    use crate::bidding::inference::{
        set_announced_reading, set_envelope_union_reading, set_pass_exclusion_reading,
    };
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Strain, Suit};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    fn assert_logits_bitwise_eq(left: &Logits, right: &Logits) {
        for (call, value) in &left.0 {
            assert_eq!(
                value.to_bits(),
                right.0.get(call).to_bits(),
                "different logit for {call}"
            );
        }
    }

    fn assert_explanations_bitwise_eq(left: &Map<(usize, f32)>, right: &Map<(usize, f32)>) {
        assert_eq!(
            left.keys().collect::<Vec<_>>(),
            right.keys().collect::<Vec<_>>()
        );
        for (call, &(index, value)) in left {
            let &(other_index, other_value) =
                right.get(call).expect("both explanations contain the call");
            assert_eq!(index, other_index, "different authored rule for {call}");
            assert_eq!(
                value.to_bits(),
                other_value.to_bits(),
                "different explained logit for {call}"
            );
        }
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
    fn test_face_gate() {
        let rules = Rules::new()
            .rule(Bid::new(1, Strain::Notrump), 1.0, hcp(15..=17) & balanced())
            .face(|context| !context.auction().is_empty());
        let rule = &rules.rules()[0];
        let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");

        // Gate false (opening seat): as-if-absent.
        let opening = Context::new(RelativeVulnerability::NONE, &[]);
        assert!(!rule.face_live(&opening));
        assert_eq!(rule.eval(hand, &opening), f32::NEG_INFINITY);

        // Gate true: normal evaluation.  Ungated rules default to live.
        let later = Context::new(RelativeVulnerability::NONE, &[Call::Pass]);
        assert!(rule.face_live(&later));
        assert!(rule.eval(hand, &later) > f32::NEG_INFINITY);
        assert!(opening_rules().rules()[0].face_live(&opening));
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

    #[test]
    fn compiled_classify_and_explain_are_bit_exact_and_keep_first_tie() {
        let one_spade = Call::Bid(Bid::new(1, Strain::Spades));
        let one_notrump = Call::Bid(Bid::new(1, Strain::Notrump));
        let rules = Rules::new()
            .rule(one_spade, 0.75, |_hand: Hand, _context: &Context<'_>| 0.25)
            // Equal same-call result: strict `>` explanation keeps index 0.
            .rule(one_spade, 1.0, |_hand: Hand, _context: &Context<'_>| 0.0)
            .rule(one_notrump, -0.0, hcp(15..=17) & balanced())
            .rule(Call::Double, 3.0, pred(|_, _| false))
            .rule(Call::Redouble, 9.0, pred(|_, _| true))
            .face(|context| !context.auction().is_empty());
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
        let compiled = rules.compile(&context);

        let legacy_logits = rules.classify(hand, &context);
        let compiled_logits = compiled.classify(&rules, hand, &context);
        assert_logits_bitwise_eq(&legacy_logits, &compiled_logits);

        let legacy_explanation = rules.explain(hand, &context);
        let compiled_explanation = compiled.explain(&rules, hand, &context);
        assert_explanations_bitwise_eq(&legacy_explanation, &compiled_explanation);
        assert_eq!(compiled_explanation.get(one_spade), Some(&(0, 1.0)));
        assert_eq!(compiled_explanation.get(Call::Redouble), None);
    }

    #[test]
    fn explicit_shared_faces_are_lazy_ordered_and_once_per_decision() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let evaluations = Arc::new(AtomicUsize::new(0));
        let face_id = FaceId::new("test:shared-face", 0);

        let first_face_events = Arc::clone(&events);
        let first_face_evaluations = Arc::clone(&evaluations);
        let first_constraint_events = Arc::clone(&events);
        let second_face_events = Arc::clone(&events);
        let second_face_evaluations = Arc::clone(&evaluations);
        let second_constraint_events = Arc::clone(&events);
        let rules = Rules::new()
            .rule(
                Bid::new(1, Strain::Clubs),
                1.0,
                move |_hand: Hand, _context: &Context<'_>| {
                    first_constraint_events.lock().unwrap().push("first");
                    0.0
                },
            )
            .shared_face(face_id, move |_| {
                first_face_evaluations.fetch_add(1, Ordering::Relaxed);
                first_face_events.lock().unwrap().push("face");
                true
            })
            .rule(
                Bid::new(1, Strain::Diamonds),
                1.0,
                move |_hand: Hand, _context: &Context<'_>| {
                    second_constraint_events.lock().unwrap().push("second");
                    0.0
                },
            )
            .shared_face(face_id, move |_| {
                second_face_evaluations.fetch_add(1, Ordering::Relaxed);
                second_face_events.lock().unwrap().push("face");
                true
            });

        let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
        let mut projections = ProjectionCache::default();
        let mut faces = FaceRegistry::default();
        let bare = Context::new(RelativeVulnerability::NONE, &[]);
        let compiled =
            CompiledRules::compile_with_cache(&rules, &bare, &mut projections, &mut faces);
        assert_eq!(faces.len(), 1);

        // The reader's at-the-time contexts deliberately carry no final
        // decision cache. Its effect-scoped memo still evaluates one shared
        // recognizer only once across projection/alert/announcement walks.
        let mut effect_faces = FaceMemo::new();
        assert!(compiled.face_live_memoized(&rules, 0, &bare, &mut effect_faces));
        assert!(compiled.face_live_memoized(&rules, 1, &bare, &mut effect_faces));
        assert!(compiled.face_live_memoized(&rules, 0, &bare, &mut effect_faces));
        assert_eq!(evaluations.load(Ordering::Relaxed), 1);
        evaluations.store(0, Ordering::Relaxed);
        events.lock().unwrap().clear();

        let context = bare.with_compiled_decision_cache(hand, faces.len());

        let _ = compiled.classify(&rules, hand, &context);
        assert_eq!(*events.lock().unwrap(), ["face", "first", "second"]);
        assert_eq!(evaluations.load(Ordering::Relaxed), 1);

        // Explanation is a second executor walk in the same immutable
        // decision. It reuses the face bit while retaining constraint order.
        let _ = compiled.explain(&rules, hand, &context);
        assert_eq!(
            *events.lock().unwrap(),
            ["face", "first", "second", "first", "second"]
        );
        assert_eq!(evaluations.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn opaque_public_faces_keep_every_consult() {
        let evaluations = Arc::new(AtomicUsize::new(0));
        let first = Arc::clone(&evaluations);
        let second = Arc::clone(&evaluations);
        let rules = Rules::new()
            .rule(Bid::new(1, Strain::Clubs), 1.0, pred(|_, _| true))
            .face(move |_| {
                first.fetch_add(1, Ordering::Relaxed);
                true
            })
            .rule(Bid::new(1, Strain::Diamonds), 1.0, pred(|_, _| true))
            .face(move |_| {
                second.fetch_add(1, Ordering::Relaxed);
                true
            });
        let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let compiled = rules.compile(&context);

        let _ = compiled.classify(&rules, hand, &context);
        let _ = compiled.explain(&rules, hand, &context);
        assert_eq!(evaluations.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn compiled_groups_alerts_and_pass_exclusion_keep_authored_indices() {
        const ARTIFICIAL: Alert = Alert("test artificial");
        let one_club = Call::Bid(Bid::new(1, Strain::Clubs));
        let one_diamond = Call::Bid(Bid::new(1, Strain::Diamonds));
        let rules = Rules::new()
            .rule(Call::Pass, 1.0, pred(|_, _| true))
            .face(|_| false)
            .rule(one_club, 2.0, pred(|_, _| true))
            .rule(Call::Pass, 2.0, pred(|_, _| true))
            .rule(one_club, 3.0, pred(|_, _| true))
            .alert(ARTIFICIAL)
            .rule(one_diamond, 4.0, pred(|_, _| true))
            .rule(one_club, 2.0, pred(|_, _| true));
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let compiled = rules.compile(&context);

        assert_eq!(compiled.rule_indices(Call::Pass), [0, 2]);
        assert_eq!(compiled.rule_indices(one_club), [1, 3, 5]);
        assert_eq!(compiled.alerted_rule_indices(one_club), [3]);
        assert!(compiled.alerted_rule_indices(one_diamond).is_empty());
        assert_eq!(
            compiled.call_plan(one_club).map(CompiledCallPlan::call),
            Some(one_club)
        );
        assert_eq!(
            compiled
                .call_plan(one_club)
                .map(CompiledCallPlan::max_weight),
            Some(3.0)
        );

        let pass = compiled.pass_plan().expect("Pass was authored");
        // Face-dead Pass rule 0 remains in the reading plan by design.
        assert_eq!(compiled.pass_rule_indices(), [0, 2]);
        assert_eq!(pass.max_weight().to_bits(), 2.0f32.to_bits());
        assert_eq!(pass.stronger_nonpass_indices(), [3, 4]);
    }

    #[test]
    fn projection_folds_compile_independently_and_profile_mismatch_falls_back() {
        let one_club = Bid::new(1, Strain::Clubs);
        let rules = Rules::new()
            .rule(one_club, 0.0, len(Suit::Spades, 4..=5))
            .rule(one_club, 0.0, support(3..))
            .rule(
                one_club,
                0.0,
                announced(pred(|_, _| true), len(Suit::Hearts, 5..)),
            )
            .alert(Alert("test announcement"));
        let auction = [Call::Bid(Bid::new(1, Strain::Hearts)), Call::Pass];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        set_envelope_union_reading(false);
        set_pass_exclusion_reading(true);
        set_announced_reading(true);
        let compiled = rules.compile(&context);
        assert!(compiled.projection_is_constant(0, ProjectionKind::Forward));
        assert!(compiled.projection_is_constant(0, ProjectionKind::Complement));
        assert!(!compiled.projection_is_constant(1, ProjectionKind::Forward));
        // A closure's default projection is the pure, vacuous fold.  The
        // announcement wrapper can therefore freeze it independently from
        // the concrete disclosure constraint below.
        assert!(compiled.projection_is_constant(2, ProjectionKind::Forward));
        assert!(compiled.projection_is_constant(2, ProjectionKind::Announcement));
        assert!(
            compiled
                .projection_dependencies(1, ProjectionKind::Forward)
                .intersects(ConstraintDependencies::CONTEXT)
        );

        assert_eq!(
            compiled.project_union(&rules, 0, &context),
            rules.rules()[0].project_union(&context)
        );
        assert_eq!(
            compiled.project_band_union(&rules, 0, &context),
            rules.rules()[0].project_band_union(&context)
        );
        assert_eq!(
            compiled.announce_union(&rules, 2, &context),
            rules.rules()[2].announce_union(&context)
        );

        // The complement was frozen knob-off, but a changed profile must use
        // the live virtual fold and retain both knob-on halves.
        set_envelope_union_reading(true);
        let expected = rules.rules()[0].project_complement_union(&context);
        assert_eq!(expected.boxes().len(), 2);
        assert_eq!(
            compiled.project_complement_union(&rules, 0, &context),
            expected
        );
        set_pass_exclusion_reading(false);
        set_announced_reading(false);
        set_envelope_union_reading(true);
    }
}
