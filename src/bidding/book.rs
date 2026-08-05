//! Role-aware pair books
//!
//! A pair writes its notes from its own side of the table.  The natural split
//! is by the [`Phase`] of the auction — who opened, and whether the opponents
//! have intervened:
//!
//! - a [`Constructive`] book covers the strictly uncontested auctions — our
//!   openings (in every seat, keyed by their leading passes) and the
//!   continuations while the opponents only pass;
//! - a [`Competitive`] book covers the auctions where **we** open and **they**
//!   intervene — negative doubles, competitive raises, and "system on"
//!   rebases ([`Fallback`][crate::bidding::fallback::Fallback]s);
//! - a [`Defensive`] book covers the auctions where **they** open — our
//!   overcalls, takeout doubles, and defense to their conventional openings.
//!
//! All three wrap the low-level [`Trie`] engine and add nothing to authoring:
//! they deref to it, so [`insert`][Trie::insert],
//! [`fallback_at`][Trie::fallback_at], and friends are available directly.
//! What the newtype adds is a *gated* [`System`] implementation that answers
//! only for its phase.  A [`Pair`] assembles the three books; binding it with
//! [`Pair::against`] yields a [`Stance`], the system that actually
//! classifies.  There is no whole-system identity label: a system announces
//! itself through its calls' own [`Alert`][super::Alert]s and their readings.
//!
//! # Key disjointness
//!
//! The books occupy disjoint keys by construction: every opposing call in a
//! constructive key is a pass, while a competitive key contains an opposing
//! non-pass call.  [`Pair::against`] exploits this to merge a clone of the
//! constructive trie into the bound competitive trie collision-free, which is
//! what lets a competitive rebase land in the uncontested core.
//!
//! # Standard pass only
//!
//! These types assume a **standard pass**: a leading [`Pass`][Call::Pass] is
//! neutral and the opener is whoever makes the first non-pass call.  This
//! assumption lives in exactly one routing point, [`Phase::of`].  Forcing or
//! strong-pass systems, where the opening pass itself carries meaning, are out
//! of scope — author them as a bare [`Trie`] table model (which keys on the
//! literal auction with no pass semantics) until a dedicated router exists.

use super::System;
use super::array::Logits;
use super::context::Context;
use super::decoder::AuthoringDecoder;
use super::inference::{
    AuthoringStepCache, Envelope, Inferences, Range, ReadingProfile, reading_profile,
};
use super::rules::{CompiledRules, FaceRegistry, ProjectionCache};
use super::trie::{Classifier, Provenance, Trie};
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{FullDeal, Hand, Seat, Suit};
use core::ops::{Deref, DerefMut};
use std::collections::HashMap;
use std::sync::Arc;

/// Resolve `auction` against `trie` exactly like the bare table model
///
/// The standalone book impls route here; a bare trie has no [`Stance`], so no
/// opponents' system is attached and the table-wide alert decode abstains.
fn resolve(
    trie: &Trie,
    hand: Hand,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> Option<Logits> {
    let context = Context::new(vul, auction)
        .with_prefixes(trie.common_prefixes(auction))
        .with_decision_cache(hand);
    trie.classify_floored(hand, &context, auction)
        .map(|(logits, _)| logits)
}

/// Our book for the strictly uncontested auctions
///
/// Keyed by the raw table auction, so seats are explicit leading passes: the
/// opening lives at `[]`, `[P]`, `[P, P]`, `[P, P, P]` for 1st through 4th seat,
/// and continuations hang off the matching prefix.  As a [`System`] it answers
/// only while nobody has opened or we opened and the opponents have only
/// passed; see the [module docs][self].
#[derive(Clone, Debug, Default)]
pub struct Constructive(pub Trie);

impl Constructive {
    /// Construct an empty constructive book
    #[must_use]
    pub const fn new() -> Self {
        Self(Trie::new())
    }
}

impl Deref for Constructive {
    type Target = Trie;

    fn deref(&self) -> &Trie {
        &self.0
    }
}

impl DerefMut for Constructive {
    fn deref_mut(&mut self) -> &mut Trie {
        &mut self.0
    }
}

impl System for Constructive {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        (Phase::of(auction) == Phase::Constructive)
            .then(|| resolve(&self.0, hand, vul, auction))
            .flatten()
    }
}

/// Our book for the auctions where **they** open
///
/// Keyed by the raw table auction starting from their opening: `[1♠]` is our
/// overcall decision over their 1♠, and continuations hang off it.  As a
/// [`System`] it answers only when the opponents opened; see the
/// [module docs][self].
#[derive(Clone, Debug, Default)]
pub struct Defensive(pub Trie);

impl Defensive {
    /// Construct an empty defensive book
    #[must_use]
    pub const fn new() -> Self {
        Self(Trie::new())
    }
}

impl Deref for Defensive {
    type Target = Trie;

    fn deref(&self) -> &Trie {
        &self.0
    }
}

impl DerefMut for Defensive {
    fn deref_mut(&mut self) -> &mut Trie {
        &mut self.0
    }
}

impl System for Defensive {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        (Phase::of(auction) == Phase::Defensive)
            .then(|| resolve(&self.0, hand, vul, auction))
            .flatten()
    }
}

/// The role of the side to act, given who opened
///
/// Each phase selects one of a pair's three books.  [`Phase::of`] is also the
/// **single point** that assumes a standard pass: a leading pass is neutral and
/// the opener is whoever makes the first non-pass call.  A future strong-pass
/// router would replace this one function; until then, author such systems as
/// a bare [`Trie`] table model (see the [module docs][self]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Nobody has opened yet, or we opened and the opponents have only passed
    Constructive,
    /// We opened and the opponents have intervened
    Competitive,
    /// The opponents opened
    Defensive,
}

impl Phase {
    /// The phase of the auction for the side to act
    ///
    /// The side to act owns the indices with `auction.len()` parity and the
    /// opener owns the indices with the opening's parity: the opponents opened
    /// iff those parities differ.  When our side opened, the auction is
    /// competitive iff the opponents have intervened — any non-pass call
    /// (a bid, a double, even a redouble) at every other index after the
    /// opening.  With no opening yet (all passes) the side to act may still
    /// open, which is constructive.
    #[must_use]
    pub fn of(auction: &[Call]) -> Self {
        let Some(opening) = auction.iter().position(|&call| call != Call::Pass) else {
            return Self::Constructive;
        };

        if opening % 2 != auction.len() % 2 {
            return Self::Defensive;
        }

        // We opened, so the opponents' calls start right after the opening
        // and sit at every other index; before it they only passed.
        let mut their_calls = auction[opening + 1..].iter().step_by(2);

        if their_calls.any(|&call| call != Call::Pass) {
            Self::Competitive
        } else {
            Self::Constructive
        }
    }
}

/// Our book for the auctions where **we** open and **they** intervene
///
/// Keyed by the raw table auction like its siblings: `[1♥, 2♣]` is our
/// decision after our 1st-seat 1♥ opening and their 2♣ overcall.  As a
/// [`System`] it answers only in its [`Phase`].
///
/// Standalone, a rebase ([`Fallback::Rebase`][super::fallback::Fallback]) sees
/// only this trie; bind through [`Pair::against`] so that "system on" rebases
/// reach the uncontested core.
#[derive(Clone, Debug, Default)]
pub struct Competitive(pub Trie);

impl Competitive {
    /// Construct an empty competitive book
    #[must_use]
    pub const fn new() -> Self {
        Self(Trie::new())
    }
}

impl Deref for Competitive {
    type Target = Trie;

    fn deref(&self) -> &Trie {
        &self.0
    }
}

impl DerefMut for Competitive {
    fn deref_mut(&mut self) -> &mut Trie {
        &mut self.0
    }
}

impl System for Competitive {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        (Phase::of(auction) == Phase::Competitive)
            .then(|| resolve(&self.0, hand, vul, auction))
            .flatten()
    }
}

/// One pair's authored system: its three books
///
/// A pair writes a [`Constructive`] book (strictly uncontested), a
/// [`Competitive`] book (we open, they intervene), and a [`Defensive`] book
/// (they open).  A pair is *authoring material*, not yet a [`System`]: bind
/// it with [`against`][Self::against] — once, at table assembly — to get a
/// [`Stance`] that classifies.
///
/// The books occupy disjoint keys by construction: a constructive key has all
/// opposing calls as passes, while a competitive key contains an opposing
/// non-pass call.
#[derive(Clone, Debug, Default)]
pub struct Pair {
    /// The book for the strictly uncontested auctions
    pub constructive: Constructive,
    /// The book for when we open and they intervene
    pub competitive: Competitive,
    /// The book for when they open
    pub defensive: Defensive,
}

impl Pair {
    /// Assemble a pair from its three books
    #[must_use]
    pub const fn new(
        constructive: Constructive,
        competitive: Competitive,
        defensive: Defensive,
    ) -> Self {
        Self {
            constructive,
            competitive,
            defensive,
        }
    }

    /// Bind this pair into a playable [`Stance`]
    ///
    /// Merges a clone of the constructive trie into the bound competitive
    /// trie ([`Trie::merge`], classifiers stay shared), so that competitive
    /// rebases — the "system on" idiom — resolve into the uncontested core.
    /// Bind once per table, not per call.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the competitive and constructive books
    /// classify the same exact auction; by the key disjointness above, such a
    /// collision is an authoring bug.
    #[must_use]
    pub fn against(&self) -> Stance {
        let constructive = self.constructive.0.clone();
        let mut bound = self.competitive.0.clone();
        let collisions = bound.merge(constructive.clone());
        debug_assert!(
            collisions.is_empty(),
            "competitive and constructive books collide at {collisions:?}"
        );

        let (constructive_trie, constructive_decoder) = BoundBook::finalize(constructive);
        let (competitive_trie, competitive_decoder) = BoundBook::finalize(bound);
        let (defensive_trie, defensive_decoder) = BoundBook::finalize(self.defensive.0.clone());
        let compiled_rules = Arc::new(CompiledRuleRegistry::compile([
            constructive_decoder.as_ref(),
            competitive_decoder.as_ref(),
            defensive_decoder.as_ref(),
        ]));
        let constructive = BoundBook::new(
            constructive_trie,
            constructive_decoder,
            Arc::clone(&compiled_rules),
        );
        let competitive = BoundBook::new(
            competitive_trie,
            competitive_decoder,
            Arc::clone(&compiled_rules),
        );
        let defensive = BoundBook::new(defensive_trie, defensive_decoder, compiled_rules);
        Stance {
            constructive,
            competitive,
            defensive,
            probed: HashMap::new(),
            cache_identity: Arc::new(StanceCacheIdentity::default()),
        }
    }
}

/// One finalized runtime book
///
/// The mutable [`Trie`] remains the public authoring surface. Binding freezes
/// the concrete row sites after every merge, graft, overwrite, and floor has
/// happened, producing owned metadata that can safely back compiled plans.
#[derive(Clone, Debug)]
struct BoundBook {
    trie: Trie,
    decoder: Arc<AuthoringDecoder>,
    compiled_rules: Arc<CompiledRuleRegistry>,
}

impl BoundBook {
    fn finalize(mut trie: Trie) -> (Trie, Arc<AuthoringDecoder>) {
        let authoring = trie.take_authoring_ledger();
        let decoder = Arc::new(AuthoringDecoder::compile_ledger(&trie, authoring));
        (trie, decoder)
    }

    fn new(
        trie: Trie,
        decoder: Arc<AuthoringDecoder>,
        compiled_rules: Arc<CompiledRuleRegistry>,
    ) -> Self {
        Self {
            trie,
            decoder,
            compiled_rules,
        }
    }

    fn classify(&self, classifier: &dyn Classifier, hand: Hand, context: &Context<'_>) -> Logits {
        classifier.as_rules().map_or_else(
            || classifier.classify(hand, context),
            |rules| {
                self.compiled_for(classifier).map_or_else(
                    || rules.classify(hand, context),
                    |plan| plan.classify(rules, hand, context),
                )
            },
        )
    }

    fn resolve_floored<'a>(
        &'a self,
        hand: Hand,
        context: &Context<'_>,
        auction: &[Call],
    ) -> Option<(&'a dyn Classifier, Logits, Provenance)> {
        if let Some((classifier, provenance)) = self.trie.resolve(context, auction) {
            let logits = self.classify(classifier, hand, context);
            if logits.has_mass() {
                return Some((classifier, logits, provenance));
            }
        }
        let (classifier, provenance) = self.trie.resolve_after_exact_rejection(context, auction)?;
        Some((
            classifier,
            self.classify(classifier, hand, context),
            provenance,
        ))
    }

    fn compiled_for(&self, classifier: &dyn Classifier) -> Option<&CompiledRules> {
        self.compiled_rules.get(classifier)
    }
}

/// Rule sidecars keyed by the data pointer of the authoritative classifier.
///
/// The registry owns no classifier or constraint objects.  Those remain in
/// the trie, while the sidecar stores compact authored indices and frozen
/// context-independent projections.
#[derive(Clone)]
struct CompiledRuleRegistry {
    profile: ReadingProfile,
    tables: HashMap<usize, Arc<CompiledRules>>,
    face_slots: usize,
}

impl core::fmt::Debug for CompiledRuleRegistry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompiledRuleRegistry")
            .field("tables", &self.tables.len())
            .finish()
    }
}

impl CompiledRuleRegistry {
    fn key(classifier: &dyn Classifier) -> usize {
        std::ptr::from_ref(classifier).cast::<()>() as usize
    }

    fn compile<'a>(decoders: impl IntoIterator<Item = &'a AuthoringDecoder>) -> Self {
        let mut registry = Self {
            profile: reading_profile(),
            tables: HashMap::new(),
            face_slots: 0,
        };
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let mut projections = ProjectionCache::default();
        let mut faces = FaceRegistry::default();
        let mut insert = |classifier: &dyn Classifier| {
            let key = Self::key(classifier);
            if registry.tables.contains_key(&key) {
                return;
            }
            if let Some(rules) = classifier.as_rules() {
                registry.tables.insert(
                    key,
                    Arc::new(CompiledRules::compile_with_cache(
                        rules,
                        &context,
                        &mut projections,
                        &mut faces,
                    )),
                );
            }
        };
        for decoder in decoders {
            for classifier in decoder.classifiers() {
                insert(classifier);
            }
        }
        registry.face_slots = faces.len();
        registry
    }

    fn get(&self, classifier: &dyn Classifier) -> Option<&CompiledRules> {
        self.tables.get(&Self::key(classifier)).map(AsRef::as_ref)
    }

    fn get_for_profile(
        &self,
        classifier: &dyn Classifier,
        profile: ReadingProfile,
    ) -> Option<&CompiledRules> {
        (self.profile == profile)
            .then(|| self.get(classifier))
            .flatten()
    }
}

impl Default for BoundBook {
    fn default() -> Self {
        let (trie, decoder) = Self::finalize(Trie::new());
        let compiled_rules = Arc::new(CompiledRuleRegistry::compile([decoder.as_ref()]));
        Self::new(trie, decoder, compiled_rules)
    }
}

/// Clone-stable identity for stance-local data consumed by deal caches.
#[derive(Debug, Default)]
pub(crate) struct StanceCacheIdentity {
    revision: u64,
}

/// A pair's system, bound and ready to classify
///
/// Built by [`Pair::against`].  As a [`System`] it routes each query by
/// [`Phase`]: the constructive trie answers the strictly uncontested auctions,
/// the bound competitive trie (which contains the uncontested core for its
/// rebases) answers when they intervene over our opening, and the defensive
/// trie answers when they open.  Constructive-phase queries use the *unmerged*
/// constructive trie, so no competitive fallback can leak into undisturbed
/// auctions.
#[derive(Clone, Default)]
pub struct Stance {
    constructive: BoundBook,
    competitive: BoundBook,
    defensive: BoundBook,
    /// Behaviorally probed readings, keyed by auction prefix with leading
    /// passes stripped (dealer rotations merge, as the books fan).  Empty
    /// until [`probe`][Self::probe] runs; consumed by the projection pass
    /// under [`set_probed_reading`][super::set_probed_reading].
    probed: HashMap<Vec<Call>, Envelope>,
    /// Stable across clones, replaced whenever stance-local reading data
    /// changes. Deal caches retain this small token, so an allocator cannot
    /// recycle a raw address into a false identity match.
    cache_identity: Arc<StanceCacheIdentity>,
}

impl core::fmt::Debug for Stance {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stance")
            .field("constructive", &self.constructive.trie)
            .field("competitive", &self.competitive.trie)
            .field("defensive", &self.defensive.trie)
            .field("probed", &self.probed)
            .finish()
    }
}

impl Stance {
    pub(crate) fn cache_identity(&self) -> &Arc<StanceCacheIdentity> {
        &self.cache_identity
    }

    fn invalidate_cache_identity(&mut self) {
        self.cache_identity = Arc::new(StanceCacheIdentity {
            revision: self.cache_identity.revision.wrapping_add(1),
        });
    }

    fn bound_for(&self, auction: &[Call]) -> &BoundBook {
        self.bound_for_phase(Phase::of(auction))
    }

    fn bound_for_phase(&self, phase: Phase) -> &BoundBook {
        match phase {
            Phase::Constructive => &self.constructive,
            Phase::Competitive => &self.competitive,
            Phase::Defensive => &self.defensive,
        }
    }

    /// The trie answering for the auction's [`Phase`]
    ///
    /// [`Phase::of`] is relative to the side to act on the slice, so this
    /// also routes an *opponent's* prior call correctly: querying with the
    /// auction cut at their turn yields their side's phase and book — how the
    /// table-wide alert decode resolves their calls (`project_authored`).
    pub(crate) fn trie_for(&self, auction: &[Call]) -> &Trie {
        &self.bound_for(auction).trie
    }

    /// Finalized authoring decoder for the auction's phase.
    pub(crate) fn decoder_for(&self, auction: &[Call]) -> &AuthoringDecoder {
        &self.bound_for(auction).decoder
    }

    pub(crate) fn decoder_for_phase(&self, phase: Phase) -> &AuthoringDecoder {
        &self.bound_for_phase(phase).decoder
    }

    /// A rule plan compiled for the active reading profile at this route.
    pub(crate) fn compiled_rules_for(
        &self,
        auction: &[Call],
        classifier: &dyn Classifier,
        profile: ReadingProfile,
    ) -> Option<&CompiledRules> {
        self.bound_for(auction)
            .compiled_rules
            .get_for_profile(classifier, profile)
    }

    /// Enter one cached serving decision after attaching its structural context
    fn decision_context<'a>(
        &'a self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &'a [Call],
    ) -> Context<'a> {
        let bound = self.bound_for(auction);
        Context::new(vul, auction)
            .with_prefixes(bound.trie.common_prefixes(auction))
            .with_their_system(self)
            .with_compiled_decision_cache(hand, bound.compiled_rules.face_slots)
    }

    fn classify_with_step_cache(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        cache: &mut AuthoringStepCache,
    ) -> Option<Logits> {
        self.classify_with_step_cache_provenance(hand, vul, auction, cache)
            .map(|(logits, _)| logits)
    }

    pub(crate) fn classify_with_step_cache_provenance(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        cache: &mut AuthoringStepCache,
    ) -> Option<(Logits, Provenance)> {
        let bound = self.bound_for(auction);
        let projection = cache.prepare(self, vul, auction);
        let mut context = Context::new(vul, auction)
            .with_prefixes(bound.trie.common_prefixes(auction))
            .with_their_system(self);
        if let Some(projection) = projection {
            context = context.with_authored_projection(projection);
        }
        let context = context.with_compiled_decision_cache(hand, bound.compiled_rules.face_slots);
        bound
            .resolve_floored(hand, &context, auction)
            .map(|(_, logits, provenance)| (logits, provenance))
    }

    /// Classify with the resolution [`Provenance`] — where the answer came from
    ///
    /// Same routing and result as the [`System`] implementation, with the
    /// provenance of the winning classifier alongside the logits.  This is
    /// the telemetry hook for the instinct floor
    /// ([`bidding::instinct`][mod@crate::bidding::instinct]): `depth == 0` with
    /// `fallback == Some(_)` is the floor firing, and the auctions that fire
    /// it most often are the next nodes worth authoring properly.
    #[must_use]
    pub fn classify_with_provenance(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<(Logits, Provenance)> {
        let bound = self.bound_for(auction);
        let context = self.decision_context(hand, vul, auction);
        bound
            .resolve_floored(hand, &context, auction)
            .map(|(_, logits, provenance)| (logits, provenance))
    }

    /// The pre-decision-cache classification path
    ///
    /// Keep this as a same-process semantic reference for cache parity tests:
    /// it deliberately constructs a fresh, cache-free [`Context`] from the
    /// causal auction prefix and must not inherit a classification scope.
    #[must_use]
    #[allow(dead_code)] // retained as the same-process cache-parity oracle
    pub(crate) fn classify_with_provenance_uncached(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
    ) -> Option<(Logits, Provenance)> {
        let trie = self.trie_for(auction);
        let context = Context::new(vul, auction)
            .with_prefixes(trie.common_prefixes(auction))
            .with_their_system(self);
        trie.classify_floored(hand, &context, auction)
    }

    /// Explain one decision: where the answer came from, and which rule made it
    ///
    /// Resolves exactly as [`classify_with_provenance`][Self::classify_with_provenance]
    /// (same routing, same mass fall-through), returning the [`Provenance`]
    /// plus — when the winning classifier is a [`Rules`][super::rules::Rules]
    /// ladder — the rule that produced `call`'s best logit.  The rule half is
    /// [`None`] when the winner is not rule-backed (a learned floor) or gives
    /// `call` no finite logit (the call did not come from this table).  This
    /// is the attribution hook for divergence forensics: the first differing
    /// call of a divergent board names the exact book node or floor rule that
    /// chose it.
    #[must_use]
    pub fn explain_call(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        call: Call,
    ) -> Option<(Provenance, Option<ExplainedRule>)> {
        let context = self.decision_context(hand, vul, auction);
        self.explain_call_in_context(hand, &context, call)
    }

    /// Explain through one already-built context, retaining its decision scope
    fn explain_call_in_context(
        &self,
        hand: Hand,
        context: &Context<'_>,
        call: Call,
    ) -> Option<(Provenance, Option<ExplainedRule>)> {
        let auction = context.auction();
        let bound = self.bound_for(auction);
        let (classifier, _, provenance) = bound.resolve_floored(hand, context, auction)?;
        let rule = classifier.as_rules().and_then(|rules| {
            let explanation = bound.compiled_for(classifier).map_or_else(
                || rules.explain(hand, context),
                |plan| plan.explain(rules, hand, context),
            );
            let &(index, _) = explanation.get(call)?;
            let rule = &rules.rules()[index];
            Some(ExplainedRule {
                index,
                label: rule.label(),
                description: rule.describe().to_string(),
                alert: rule.alert().map(|alert| alert.0),
            })
        });
        Some((provenance, rule))
    }

    /// The pre-decision-cache explanation path
    ///
    /// Like [`classify_with_provenance_uncached`][Self::classify_with_provenance_uncached],
    /// this always evaluates under a fresh cache-free context.  It remains a
    /// crate-private reference after the public path acquires a decision scope.
    #[must_use]
    #[allow(dead_code)] // retained as the same-process cache-parity oracle
    pub(crate) fn explain_call_uncached(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        call: Call,
    ) -> Option<(Provenance, Option<ExplainedRule>)> {
        let trie = self.trie_for(auction);
        let context = Context::new(vul, auction)
            .with_prefixes(trie.common_prefixes(auction))
            .with_their_system(self);
        let (classifier, _, provenance) = trie.resolve_floored(hand, &context, auction)?;
        let rule = classifier.as_rules().and_then(|rules| {
            let &(index, _) = rules.explain(hand, &context).get(call)?;
            let rule = &rules.rules()[index];
            Some(ExplainedRule {
                index,
                label: rule.label(),
                description: rule.describe().to_string(),
                alert: rule.alert().map(|alert| alert.0),
            })
        });
        Some((provenance, rule))
    }
}

/// The winning rule behind one call — [`Stance::explain_call`]'s attribution
#[derive(Clone, Debug)]
pub struct ExplainedRule {
    /// Index of the rule in its [`Rules`][super::rules::Rules] table, in
    /// declaration order — stable within one build of the books
    pub index: usize,
    /// The authored [`note`][super::rules::Rules::note] label, `""` when unset
    pub label: &'static str,
    /// The rule's constraint rendered as prose ([`Rule::describe`][super::rules::Rule::describe])
    pub description: String,
    /// The [`Alert`][super::rules::Alert] name the rule carries, or [`None`]
    /// for a natural (unalerted) rule
    pub alert: Option<&'static str>,
}

impl Stance {
    /// The prefixed [`Context`] this stance classifies an auction under
    ///
    /// The same trie-routed, prefix-bearing context the [`System`] impl builds
    /// (cf. [`classify_with_provenance`][Self::classify_with_provenance]).  It
    /// hands the otherwise-keyless reading paths the trie access the projection
    /// pass needs, so [`Inferences::read`][super::inference::Inferences::read]
    /// can project each artificial prior call straight off its authored rule.
    ///
    /// Public because a corpus dump has to build the *same* context serving
    /// does. `dump-teacher` historically extracted from a bare `Context::new`,
    /// which carries no prefixes, so `project_authored` silently skipped every
    /// authored rule — a train/serve skew measured at 3 of 40 inference floats
    /// by `bare_and_prefixed_contexts_disagree`.
    #[must_use]
    pub fn prefixed_context<'a>(
        &'a self,
        vul: RelativeVulnerability,
        auction: &'a [Call],
    ) -> Context<'a> {
        let trie = self.trie_for(auction);
        Context::new(vul, auction)
            .with_prefixes(trie.common_prefixes(auction))
            .with_their_system(self)
    }

    /// Read what an auction has shown, exactly as this stance would at the table
    ///
    /// Builds the same trie-routed, prefix-bearing [`Context`] classification
    /// uses, then reads it with [`Inferences::read`] — so alerted conventional
    /// calls decode off their authoring rules instead of misreading as natural
    /// suits.  `vul` is relative to the player to act after `auction` (the
    /// reader); the returned ranges are relative to that same player.  This is
    /// the entry point for harnesses that need an auction's shown ranges
    /// outside a live classification (e.g. sampling layouts for an opening
    /// lead).
    #[must_use]
    pub fn infer(&self, vul: RelativeVulnerability, auction: &[Call]) -> Inferences {
        Inferences::read(&self.prefixed_context(vul, auction))
    }
}

/// What one [`Stance::probe`] run stored, and how stable the fixed point was
#[derive(Clone, Copy, Debug)]
pub struct ProbeReport {
    /// Prefix keys with a stored box (≥ the sample floor, binding some axis)
    pub keys: usize,
    /// Keys whose box moved between the two probe iterations — the fixed-point
    /// drift.  A large fraction means the probed readings materially changed
    /// the bidder's own auctions and a third iteration is worth considering.
    pub drifted: usize,
}

/// Per-key sample aggregate of the probe harvest: observed per-axis extremes
#[derive(Clone, Copy)]
struct Observed {
    count: u64,
    points: Range,
    lengths: [Range; 4],
}

impl Observed {
    fn new() -> Self {
        Self {
            count: 0,
            points: Range::new(37, 0), // inverted: first `add` overwrites
            lengths: [Range::new(13, 0); 4],
        }
    }

    fn add(&mut self, hand: Hand) {
        fn widen(range: &mut Range, value: u8) {
            range.min = range.min.min(value);
            range.max = range.max.max(value);
        }
        self.count += 1;
        widen(&mut self.points, super::constraint::point_count(hand));
        for suit in Suit::ASC {
            widen(&mut self.lengths[suit as usize], {
                #[allow(clippy::cast_possible_truncation)]
                let len = hand[suit].len() as u8;
                len
            });
        }
    }

    fn merge(&mut self, other: &Self) {
        if other.count == 0 {
            return;
        }
        if self.count == 0 {
            *self = *other;
            return;
        }
        self.count += other.count;
        self.points = self.points.span(other.points);
        for (a, b) in self.lengths.iter_mut().zip(other.lengths) {
            *a = a.span(b);
        }
    }

    /// The stored box: observed support **widened** on every axis — a sample
    /// edge is not a rule edge (docs/ai-bidder/sampled-projection.md), and a
    /// too-narrow box is the catastrophic side (the sampler rejects hands the
    /// bidder actually holds).  `None` when nothing binds after widening.
    fn boxed(&self) -> Option<Envelope> {
        const POINT_SLACK: u8 = 2;
        const LENGTH_SLACK: u8 = 1;
        let pad = |range: Range, slack: u8, cap: u8| {
            Range::new(
                range.min.saturating_sub(slack),
                range.max.saturating_add(slack).min(cap),
            )
        };
        let mut envelope = Envelope::unknown();
        envelope.strength.points = pad(self.points, POINT_SLACK, Range::FULL_POINTS.max);
        for suit in Suit::ASC {
            envelope.lengths[suit as usize] = pad(
                self.lengths[suit as usize],
                LENGTH_SLACK,
                Range::FULL_LENGTH.max,
            );
        }
        (envelope != Envelope::unknown()).then_some(envelope)
    }
}

/// A prefix key with its leading passes stripped — dealer rotations merge,
/// exactly as the books fan seats.  `None` when every call is a pass.
fn stripped(prefix: &[Call]) -> Option<&[Call]> {
    let start = prefix.iter().position(|&call| call != Call::Pass)?;
    Some(&prefix[start..])
}

impl Stance {
    /// The probed box for a prefix, if [`probe`][Self::probe] stored one
    pub(crate) fn probed_box(&self, prefix: &[Call]) -> Option<&Envelope> {
        if self.probed.is_empty() {
            return None;
        }
        self.probed.get(stripped(prefix)?)
    }

    /// Probe this stance's own behavior and store the answers as readings
    ///
    /// The sampled-projection derivation
    /// (docs/ai-bidder/sampled-projection.md), keyed by **traffic** rather
    /// than authorship: bid `boards` deals in self-play, record the actor's
    /// hand at every decision, and store a widened bounding box per prefix
    /// key with at least [`MIN_SAMPLES`](Self::MIN_SAMPLES) observations.
    /// This reaches what no symbolic projection can: the floor's calls (a
    /// net's pass has no rule to project), rule competition, and off-axis
    /// shadows.  Consumed by [`Inferences::read`] only under
    /// [`set_probed_reading`][super::set_probed_reading].
    ///
    /// Runs **two** iterations: the first probes the bidder under the current
    /// (symbolic) readings, the second re-probes with the first pass's boxes
    /// installed — the fixed-point check the design demands.  The returned
    /// [`ProbeReport`] counts the keys that moved between the two.
    ///
    /// Probes at no vulnerability, the census methodology; vulnerability-gated
    /// ranges (the weak-two bands) are pooled, which the widening slack
    /// absorbs.  Reading-affecting knobs must not change between this call
    /// and consumption — the boxes bake in the knob state at probe time.
    pub fn probe(&mut self, boards: usize, seed: u64) -> ProbeReport {
        // ponytail: sequential — rayon is dev-only and the library must keep
        // building for wasm (`default-features = false`).  ~2 ms/board, so an
        // A/B-scale probe is minutes once per arm; add an optional rayon
        // feature only if probe time ever dominates a harness.
        let harvest = |stance: &Self, probed_on: bool| -> HashMap<Vec<Call>, Observed> {
            // The second pass must serve the first pass's boxes through the
            // fold consumption will use: arm `set_probed_vacuous_reading`
            // *before* probing and the fixed point runs under the vacuous
            // scope (the full-fold toggle stays off); otherwise the full fold
            // serves, as before.  A re-probe of a stance with a stale map
            // under an armed vacuous knob would serve that map in its first
            // pass — probe fresh stances.
            let vacuous = super::inference::probed_vacuous_reading();
            super::inference::set_probed_reading(probed_on && !vacuous);
            let mut into: HashMap<Vec<Call>, Observed> = HashMap::new();
            for board in 0..boards {
                let deal = contract_bridge::deck::full_deal(&mut {
                    use rand::SeedableRng as _;
                    rand::rngs::StdRng::seed_from_u64(seed.wrapping_add(board as u64))
                });
                for (key, agg) in stance.harvest_board(board, &deal) {
                    into.entry(key)
                        .and_modify(|existing| existing.merge(&agg))
                        .or_insert(agg);
                }
            }
            into
        };
        let boxed = |observed: HashMap<Vec<Call>, Observed>| -> HashMap<Vec<Call>, Envelope> {
            observed
                .into_iter()
                .filter(|(_, agg)| agg.count >= Self::MIN_SAMPLES)
                .filter_map(|(key, agg)| agg.boxed().map(|envelope| (key, envelope)))
                .collect()
        };

        let was = super::inference::probed_reading();
        let first = boxed(harvest(self, false));
        self.probed = first;
        self.invalidate_cache_identity();
        let second = boxed(harvest(self, true));
        super::inference::set_probed_reading(was);

        let drifted = second
            .iter()
            .filter(|(key, envelope)| self.probed.get(*key) != Some(envelope))
            .count()
            + self
                .probed
                .keys()
                .filter(|key| !second.contains_key(*key))
                .count();
        self.probed = second;
        self.invalidate_cache_identity();
        ProbeReport {
            keys: self.probed.len(),
            drifted,
        }
    }

    /// The sample floor under which a key stores nothing
    pub const MIN_SAMPLES: u64 = 200;

    /// One self-play board's harvest: `(stripped key → actor's hand)` per call
    fn harvest_board(&self, board: usize, deal: &FullDeal) -> HashMap<Vec<Call>, Observed> {
        let mut auction = Auction::new();
        let mut keys: HashMap<Vec<Call>, Observed> = HashMap::new();
        while !auction.has_ended() {
            let seat = Seat::ALL[(board + auction.len()) % 4];
            let call = self
                .classify(deal[seat], RelativeVulnerability::NONE, &auction)
                .and_then(|logits| {
                    let mut scored: Vec<(Call, f32)> = logits
                        .iter()
                        .map(|(call, &logit)| (call, logit))
                        .filter(|&(_, logit)| logit.is_finite())
                        .collect();
                    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
                    scored
                        .into_iter()
                        .map(|(call, _)| call)
                        .find(|&call| auction.can_push(call).is_ok())
                })
                .unwrap_or(Call::Pass);
            auction.push(call);
            if let Some(key) = stripped(&auction) {
                keys.entry(key.to_vec())
                    .or_insert_with(Observed::new)
                    .add(deal[seat]);
            }
        }
        keys
    }
}

// At shallow depths the already-linear reader is cheaper than initializing
// four route cursors and projection accumulators.  Start the deal cache where
// repeated prefix reading becomes the dominant cost; its first prepare can
// reconstruct the prefix, then every continuation is append-only.
const DEAL_CACHE_MIN_DEPTH: usize = 8;

#[derive(Default)]
struct StanceDealState {
    authoring: Option<AuthoringStepCache>,
}

impl System for Stance {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        self.classify_with_provenance(hand, vul, auction)
            .map(|(logits, _)| logits)
    }

    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        // Delegate to the phase-routed trie's rebasing-aware check.
        self.trie_for(auction).authored_at(vul, auction)
    }

    fn new_deal_state(&self) -> Option<Box<dyn std::any::Any>> {
        Some(Box::new(StanceDealState::default()))
    }

    fn classify_in_deal(
        &self,
        hand: Hand,
        vul: RelativeVulnerability,
        auction: &[Call],
        state: Option<&mut dyn std::any::Any>,
    ) -> Option<Logits> {
        let Some(state) = state.and_then(|state| state.downcast_mut::<StanceDealState>()) else {
            return self.classify(hand, vul, auction);
        };
        if auction.len() < DEAL_CACHE_MIN_DEPTH {
            return self.classify(hand, vul, auction);
        }
        let cache = state.authoring.get_or_insert_with(AuthoringStepCache::new);
        self.classify_with_step_cache(hand, vul, auction, cache)
    }
}

#[cfg(test)]
#[path = "../../benches/support/mod.rs"]
mod performance_support;

#[cfg(test)]
mod tests {
    use super::{DEAL_CACHE_MIN_DEPTH, Phase, StanceDealState, performance_support};
    use contract_bridge::auction::Call;
    use contract_bridge::{Bid, Strain, Suit};

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    #[test]
    fn table_deal_state_activates_the_causal_cache_at_deep_prefixes() {
        use crate::bidding::System;
        use crate::bidding::american::american_book;
        use contract_bridge::auction::RelativeVulnerability;

        let stance = american_book().against();
        let hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
        let mut state = stance.new_deal_state().expect("stance deal state");
        assert!(
            state
                .downcast_ref::<StanceDealState>()
                .expect("typed stance state")
                .authoring
                .is_none()
        );

        let shallow = vec![Call::Pass; DEAL_CACHE_MIN_DEPTH - 1];
        let _ = stance.classify_in_deal(
            hand,
            RelativeVulnerability::NONE,
            &shallow,
            Some(state.as_mut()),
        );
        assert!(
            state
                .downcast_ref::<StanceDealState>()
                .expect("typed stance state")
                .authoring
                .is_none(),
            "shallow auctions should avoid cache setup",
        );

        let deep = vec![Call::Pass; DEAL_CACHE_MIN_DEPTH];
        let _ = stance.classify_in_deal(
            hand,
            RelativeVulnerability::NONE,
            &deep,
            Some(state.as_mut()),
        );
        assert!(
            state
                .downcast_ref::<StanceDealState>()
                .expect("typed stance state")
                .authoring
                .is_some(),
            "deep auctions should enter the append-only causal cache",
        );
    }

    #[test]
    fn finalized_decoder_matches_legacy_resolution_on_frozen_corpus() {
        use crate::bidding::american::american_book;
        use crate::bidding::context::{Context, flipped};

        let stance = american_book().against();
        let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
        for position in corpus {
            for depth in 0..=position.auction.len() {
                let prefix = &position.auction[..depth];
                let vul = if (position.auction.len() - depth).is_multiple_of(2) {
                    position.vul
                } else {
                    flipped(position.vul)
                };
                let context = Context::new(vul, prefix);
                let legacy = stance.trie_for(prefix).resolve(&context, prefix);
                let compiled = stance.decoder_for(prefix).resolve(&context, prefix);
                match (legacy, compiled) {
                    (None, None) => {}
                    (Some((legacy, legacy_provenance)), Some(compiled)) => {
                        assert!(
                            std::ptr::eq(legacy, compiled.classifier),
                            "classifier differs at corpus {} prefix {depth}",
                            position.id,
                        );
                        assert_eq!(
                            legacy_provenance, compiled.provenance,
                            "provenance differs at corpus {} prefix {depth}",
                            position.id,
                        );
                    }
                    (legacy, compiled) => panic!(
                        "resolution presence differs at corpus {} prefix {depth}: legacy={}, compiled={}",
                        position.id,
                        legacy.is_some(),
                        compiled.is_some(),
                    ),
                }
            }
        }
    }

    #[test]
    fn compiled_authored_projection_matches_legacy_on_frozen_corpus() {
        use crate::bidding::american::american_book;
        use crate::bidding::inference::{
            ReadingScope, assert_compiled_authoring_projection_parity, set_announced_reading,
            set_envelope_union_reading, set_fallback_projection, set_pass_exclusion_reading,
            set_pass_reading, set_reading_scope, set_table_alert_reading,
        };

        let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
        let profiles = [
            (ReadingScope::Alerted, true, true, true, false, false, true),
            (ReadingScope::All, false, false, false, false, true, false),
            (ReadingScope::None, true, true, true, true, true, true),
        ];
        for (scope, union, fallback, pass, exclusion, announced, table) in profiles {
            set_reading_scope(scope);
            set_envelope_union_reading(union);
            set_fallback_projection(fallback);
            set_pass_reading(pass);
            set_pass_exclusion_reading(exclusion);
            set_announced_reading(announced);
            set_table_alert_reading(table);
            let stance = american_book().against();
            for position in &corpus {
                let context = stance.prefixed_context(position.vul, &position.auction);
                assert_compiled_authoring_projection_parity(&context);
            }
        }

        set_reading_scope(ReadingScope::Alerted);
        set_envelope_union_reading(true);
        set_fallback_projection(true);
        set_pass_reading(true);
        set_pass_exclusion_reading(false);
        set_announced_reading(false);
        set_table_alert_reading(true);
    }

    #[test]
    fn append_only_step_cache_matches_from_scratch_frozen_prefixes() {
        use crate::bidding::american::american_book;
        use crate::bidding::context::flipped;
        use crate::bidding::inference::{AuthoringStepCache, assert_step_cache_projection_parity};

        let stance = american_book().against();
        let corpus = performance_support::parse_corpus().expect("valid frozen corpus");
        let mut cache_hits = 0usize;
        for position in corpus {
            let mut caches = [AuthoringStepCache::new(), AuthoringStepCache::new()];
            for depth in 0..=position.auction.len() {
                let vul = if (position.auction.len() - depth).is_multiple_of(2) {
                    position.vul
                } else {
                    flipped(position.vul)
                };
                cache_hits += usize::from(assert_step_cache_projection_parity(
                    &stance,
                    vul,
                    &position.auction[..depth],
                    &mut caches[depth % 2],
                ));
            }
        }
        assert!(
            cache_hits > 2_000,
            "too many prefixes dropped to the slow path"
        );
    }

    #[test]
    fn step_cache_drops_to_legacy_after_middeal_profile_change() {
        use crate::bidding::american::american_book;
        use crate::bidding::inference::{AuthoringStepCache, set_envelope_union_reading};
        use contract_bridge::auction::RelativeVulnerability;

        let stance = american_book().against();
        let auction = [bid(1, Strain::Notrump), Call::Pass];
        let mut cache = AuthoringStepCache::new();
        assert!(
            cache
                .prepare(&stance, RelativeVulnerability::NONE, &[])
                .is_some()
        );
        set_envelope_union_reading(false);
        assert!(
            cache
                .prepare(&stance, RelativeVulnerability::NONE, &auction)
                .is_none(),
            "profile change must disable this deal cache"
        );
        set_envelope_union_reading(true);
    }

    #[test]
    fn step_cache_is_bound_to_stance_identity_and_probe_revision() {
        use crate::bidding::american::american_book;
        use crate::bidding::inference::AuthoringStepCache;
        use contract_bridge::auction::RelativeVulnerability;

        let stance = american_book().against();
        let clone = stance.clone();
        let mut clone_cache = AuthoringStepCache::new();
        assert!(
            clone_cache
                .prepare(&stance, RelativeVulnerability::NONE, &[])
                .is_some()
        );
        assert!(
            clone_cache
                .prepare(
                    &clone,
                    RelativeVulnerability::NONE,
                    &[Call::Pass, Call::Pass],
                )
                .is_some(),
            "a clone preserves the stance cache identity"
        );

        let other = american_book().against();
        let mut other_cache = AuthoringStepCache::new();
        assert!(
            other_cache
                .prepare(&stance, RelativeVulnerability::NONE, &[])
                .is_some()
        );
        assert!(
            other_cache
                .prepare(&other, RelativeVulnerability::NONE, &[])
                .is_none(),
            "a cache must not cross independently bound stances"
        );

        let mut probed = stance.clone();
        let mut probe_cache = AuthoringStepCache::new();
        assert!(
            probe_cache
                .prepare(&probed, RelativeVulnerability::NONE, &[])
                .is_some()
        );
        let _ = probed.probe(0, 0xCA_C4E);
        assert!(
            probe_cache
                .prepare(&probed, RelativeVulnerability::NONE, &[])
                .is_none(),
            "probing must replace the stance cache identity"
        );
    }

    /// [`Stance::probe`] stores boxes for high-traffic keys; the knob-on
    /// reading tightens and the knob-off reading is byte-identical to an
    /// unprobed stance.  The **floorless** book keeps the self-play cheap in
    /// debug (rule evaluation only — the neural floor is ~75 ms/board
    /// unoptimized); 2,000 boards give the `1♦ P` key ~240 expected samples,
    /// clearing the [`Stance::MIN_SAMPLES`] floor.
    #[test]
    fn probe_stores_and_reads_high_traffic_keys() {
        use contract_bridge::auction::RelativeVulnerability;

        let mut stance = crate::bidding::american::american_book().against();
        let plain = crate::bidding::american::american_book().against();
        let report = stance.probe(2000, 0x9B0BE);
        assert!(report.keys > 0, "probe stored nothing");
        assert!(report.drifted <= report.keys);

        let auction = [bid(1, Strain::Diamonds), Call::Pass];
        // Knob off — byte-identical to an unprobed stance.
        let off = stance.infer(RelativeVulnerability::NONE, &auction);
        let unprobed = plain.infer(RelativeVulnerability::NONE, &auction);
        assert_eq!(off.rho(), unprobed.rho());

        crate::bidding::set_probed_reading(true);
        let on = stance.infer(RelativeVulnerability::NONE, &auction);
        crate::bidding::set_probed_reading(false);
        // The probed box only tightens the symbolic band — and it reads suit
        // lengths on the passer, which no symbolic path can (the pass gate is
        // points-only).
        assert!(on.rho().strength.points.max <= off.rho().strength.points.max);
        assert!(
            on.rho().length(Suit::Diamonds).max < 13,
            "no probed length ceiling on the passer"
        );
    }

    /// The vacuous-scoped fold serves coverage and nothing else, on the
    /// ledger's own hole (`1♦ (2♣) 2♠` read partner ♠ `0..13`,
    /// docs/reading-drift-handoff.md): partner's contested free bid fills
    /// exactly the axes the symbolic reading left fully open, an opponent's
    /// probed box never folds (v1's refuted tightening was the opponents
    /// read as limited), and a key through a still-constructive prefix never
    /// folds (filling constructive axes was the 2026-07-31 smoke's −0.67
    /// IMPs/board of net-OOD grand blasts).
    #[test]
    fn probed_vacuous_fills_only_open_axes_on_contested_own_calls() {
        use contract_bridge::auction::RelativeVulnerability;

        use crate::bidding::{Envelope, Range};

        let mut stance = crate::bidding::american::american_book().against();
        let boxed = |spades: Range, points: Range| {
            let mut envelope = Envelope::unknown();
            envelope.lengths[Suit::Spades as usize] = spades;
            envelope.strength.points = points;
            envelope
        };
        // Partner's free bid in a contested prefix — the served key.
        stance.probed.insert(
            vec![ONE_DIAMOND, TWO_CLUBS, TWO_SPADES],
            boxed(Range::new(4, 8), Range::new(8, 16)),
        );
        // Their overcall — an opponent's key, never served.
        stance.probed.insert(
            vec![ONE_DIAMOND, TWO_CLUBS],
            boxed(Range::new(0, 2), Range::new(6, 14)),
        );
        // Our opening — a key through a still-constructive prefix, never served.
        stance.probed.insert(
            vec![ONE_DIAMOND],
            boxed(Range::new(0, 3), Range::new(10, 20)),
        );

        let auction = [ONE_DIAMOND, TWO_CLUBS, TWO_SPADES, P];
        let off = stance.infer(RelativeVulnerability::NONE, &auction);
        crate::bidding::set_probed_vacuous_reading(true);
        let on = stance.infer(RelativeVulnerability::NONE, &auction);
        crate::bidding::set_probed_vacuous_reading(false);

        assert_eq!(on.rho(), off.rho(), "an opponent's probed box folded");
        assert_eq!(on.me(), off.me(), "a constructive-prefix key folded");

        let (off_p, on_p) = (off.partner(), on.partner());
        // The hole this knob exists for: the free bid's suit reads nothing.
        assert_eq!(off_p.length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(on_p.length(Suit::Spades), Range::new(4, 8));
        // Already-read axes stay byte-identical; open ones take the box.
        if off_p.strength.points == Range::FULL_POINTS {
            assert_eq!(on_p.strength.points, Range::new(8, 16));
        } else {
            assert_eq!(on_p.strength.points, off_p.strength.points);
        }
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
            assert_eq!(on_p.length(suit), off_p.length(suit));
        }
    }

    const P: Call = Call::Pass;
    const ONE_DIAMOND: Call = bid(1, Strain::Diamonds);
    const ONE_HEART: Call = bid(1, Strain::Hearts);
    const ONE_SPADE: Call = bid(1, Strain::Spades);
    const TWO_CLUBS: Call = bid(2, Strain::Clubs);
    const TWO_HEARTS: Call = bid(2, Strain::Hearts);
    const TWO_SPADES: Call = bid(2, Strain::Spades);

    #[test]
    fn test_phase_before_any_opening() {
        assert_eq!(Phase::of(&[]), Phase::Constructive);
        assert_eq!(Phase::of(&[P]), Phase::Constructive);
        assert_eq!(Phase::of(&[P, P, P]), Phase::Constructive);
        assert_eq!(Phase::of(&[P, P, P, P]), Phase::Constructive);
    }

    #[test]
    fn test_phase_when_we_opened_undisturbed() {
        assert_eq!(Phase::of(&[ONE_HEART, P]), Phase::Constructive);
        assert_eq!(
            Phase::of(&[ONE_HEART, P, TWO_HEARTS, P]),
            Phase::Constructive
        );
        assert_eq!(Phase::of(&[P, P, ONE_SPADE, P]), Phase::Constructive);
    }

    #[test]
    fn test_phase_when_they_intervened() {
        assert_eq!(Phase::of(&[ONE_HEART, TWO_CLUBS]), Phase::Competitive);
        assert_eq!(Phase::of(&[ONE_HEART, Call::Double]), Phase::Competitive);
        assert_eq!(Phase::of(&[P, ONE_HEART, Call::Double]), Phase::Competitive);
        assert_eq!(
            Phase::of(&[ONE_HEART, P, TWO_HEARTS, TWO_SPADES]),
            Phase::Competitive
        );
        // Our own redouble is not a disturbance, but their double is.
        assert_eq!(
            Phase::of(&[ONE_SPADE, Call::Double, Call::Redouble, P]),
            Phase::Competitive
        );
    }

    #[test]
    fn test_phase_when_they_opened() {
        assert_eq!(Phase::of(&[ONE_HEART]), Phase::Defensive);
        assert_eq!(Phase::of(&[P, P, ONE_SPADE]), Phase::Defensive);
        assert_eq!(Phase::of(&[ONE_HEART, TWO_CLUBS, P]), Phase::Defensive);
        assert_eq!(
            Phase::of(&[ONE_HEART, P, TWO_HEARTS, TWO_SPADES, P]),
            Phase::Defensive
        );
    }

    /// `explain_call` attributes a book call to its exact node and a floor
    /// call to the instinct fallback, each with a renderable rule.
    #[test]
    fn explain_call_names_book_and_floor_rules() {
        use crate::bidding::american::american_instinct;
        use contract_bridge::Hand;
        use contract_bridge::auction::RelativeVulnerability;

        let stance = american_instinct().against();

        // A book decision: the routine 1♠ opening resolves at the exact root
        // node (no fallback taken) and names the rule that produced it.
        let opener: Hand = "AKJ84.K52.Q4.982".parse().expect("valid test hand");
        let (provenance, rule) = stance
            .explain_call(opener, RelativeVulnerability::NONE, &[], ONE_SPADE)
            .expect("an opening classifies");
        assert_eq!(provenance.fallback, None);
        let rule = rule.expect("the opening table is a Rules ladder");
        assert!(!rule.description.is_empty());

        // A floor decision: opener's competitive long-suit rebid comes from the
        // instinct floor (depth 0 + fallback), mirroring the provenance the
        // instinct tests assert, and its winning rule still renders.
        let auction = [
            bid(1, Strain::Diamonds),
            ONE_HEART,
            P,
            TWO_HEARTS, // they raise; opener holds a self-sufficient 7-card suit
        ];
        let one_suiter: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
        let (provenance, rule) = stance
            .explain_call(
                one_suiter,
                RelativeVulnerability::NONE,
                &auction,
                bid(3, Strain::Diamonds),
            )
            .expect("a legal auction classifies");
        assert_eq!(provenance.depth, 0);
        assert!(provenance.fallback.is_some());
        let rule = rule.expect("the instinct floor is a Rules ladder");
        assert!(!rule.description.is_empty());
    }

    /// Explanation re-evaluates the winning Rules ladder after resolution;
    /// both passes must share the same causal decision cache.
    #[test]
    fn explanation_reuses_decision_initializers() {
        use crate::bidding::american::american_instinct;
        use contract_bridge::Hand;
        use contract_bridge::auction::RelativeVulnerability;

        let stance = american_instinct().against();
        let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
        let hand: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
        let context = stance.decision_context(hand, RelativeVulnerability::NONE, &auction);
        assert_eq!(context.decision_cache_init_counts(), Some((0, 0, 0)));

        let explained = stance
            .explain_call_in_context(hand, &context, bid(3, Strain::Diamonds))
            .expect("the scoped public helper explains the floor");
        assert!(explained.0.fallback.is_some());
        assert!(explained.1.is_some());
        let after_explanation = context
            .decision_cache_init_counts()
            .expect("decision cache attached");
        assert_eq!(
            after_explanation.0, 1,
            "resolution and explanation share inference"
        );
        assert!(after_explanation.0 <= 1);
        assert!(after_explanation.1 <= 1);
        assert!(after_explanation.2 <= 1);

        // Re-running the same helper cannot initialize anything again.  This
        // is the exact helper public `explain_call` invokes after entering its
        // decision scope, rather than a hand-copied approximation of that path.
        let repeated = stance
            .explain_call_in_context(hand, &context, bid(3, Strain::Diamonds))
            .expect("the scoped public helper explains twice");
        assert_eq!(repeated.0, explained.0);
        assert_eq!(
            context.decision_cache_init_counts(),
            Some(after_explanation)
        );
    }

    /// The fresh-context path is the in-process oracle for the scoped cache.
    /// Compare float representations rather than `f32` equality so signed
    /// zeroes, infinities, and any NaN payloads cannot drift unnoticed.
    #[test]
    fn public_paths_match_uncached_reference_bit_for_bit() {
        use crate::bidding::american::american_instinct;
        use contract_bridge::Hand;
        use contract_bridge::auction::RelativeVulnerability;

        let stance = american_instinct().against();
        let assert_classification = |hand: Hand, auction: &[Call]| {
            let (actual, actual_provenance) = stance
                .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
                .expect("the public path classifies");
            let (reference, reference_provenance) = stance
                .classify_with_provenance_uncached(hand, RelativeVulnerability::NONE, auction)
                .expect("the legacy path classifies");

            assert_eq!(actual_provenance, reference_provenance);
            for ((call, actual), (reference_call, reference)) in
                (&actual.0).into_iter().zip(&reference.0)
            {
                assert_eq!(call, reference_call);
                assert_eq!(
                    actual.to_bits(),
                    reference.to_bits(),
                    "logit bits differ for {call:?} at {auction:?}"
                );
            }
        };

        // Exact authored node in the constructive book.
        let opener: Hand = "AKJ84.K52.Q4.982".parse().expect("valid test hand");
        assert_classification(opener, &[]);

        // Their opening routes through the defensive book.
        let overcaller: Hand = "AQJ9.K42.763.542".parse().expect("valid test hand");
        assert_classification(overcaller, &[ONE_HEART]);

        // Rejected/missing exact continuation falling through to the
        // deterministic floor in the competitive book.
        let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
        let one_suiter: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");
        assert_classification(one_suiter, &auction);

        let assert_explanation = |hand: Hand, auction: &[Call], call: Call| {
            let (actual_provenance, actual) = stance
                .explain_call(hand, RelativeVulnerability::NONE, auction, call)
                .expect("the public path explains");
            let (reference_provenance, reference) = stance
                .explain_call_uncached(hand, RelativeVulnerability::NONE, auction, call)
                .expect("the legacy path explains");
            assert_eq!(actual_provenance, reference_provenance);

            match (actual, reference) {
                (Some(actual), Some(reference)) => {
                    assert_eq!(actual.index, reference.index);
                    assert_eq!(actual.label, reference.label);
                    assert_eq!(actual.description, reference.description);
                    assert_eq!(actual.alert, reference.alert);
                }
                (None, None) => {}
                (actual, reference) => panic!(
                    "explanation presence differs at {auction:?}: actual={actual:?}, reference={reference:?}"
                ),
            }
        };

        assert_explanation(opener, &[], ONE_SPADE);
        assert_explanation(one_suiter, &auction, bid(3, Strain::Diamonds));
    }

    #[test]
    fn cached_reference_parity_across_reading_and_evaluator_profiles() {
        use crate::bidding::american::{american_configured, american_instinct};
        use crate::bidding::inference::ReadingScope;
        use contract_bridge::Hand;
        use contract_bridge::auction::RelativeVulnerability;

        struct Profile {
            scope: ReadingScope,
            union: bool,
            exclusion: bool,
            eval_auction: bool,
            eval_shape: bool,
            blind: bool,
            bilans: bool,
            collar: bool,
            configured: bool,
        }

        let profiles = [
            Profile {
                scope: ReadingScope::Alerted,
                union: true,
                exclusion: false,
                eval_auction: true,
                eval_shape: false,
                blind: false,
                bilans: false,
                collar: false,
                configured: false,
            },
            Profile {
                scope: ReadingScope::None,
                union: false,
                exclusion: false,
                eval_auction: false,
                eval_shape: false,
                blind: false,
                bilans: false,
                collar: false,
                configured: false,
            },
            Profile {
                scope: ReadingScope::All,
                union: true,
                exclusion: true,
                eval_auction: true,
                eval_shape: true,
                blind: false,
                bilans: true,
                collar: false,
                configured: false,
            },
            Profile {
                scope: ReadingScope::Alerted,
                union: true,
                exclusion: false,
                eval_auction: true,
                eval_shape: false,
                blind: false,
                bilans: true,
                collar: true,
                configured: false,
            },
            Profile {
                scope: ReadingScope::Alerted,
                union: true,
                exclusion: false,
                eval_auction: true,
                eval_shape: false,
                blind: false,
                bilans: false,
                collar: false,
                configured: true,
            },
            Profile {
                scope: ReadingScope::Alerted,
                union: true,
                exclusion: false,
                eval_auction: true,
                eval_shape: true,
                blind: true,
                bilans: false,
                collar: false,
                configured: true,
            },
        ];
        let vulnerabilities = [
            RelativeVulnerability::NONE,
            RelativeVulnerability::WE,
            RelativeVulnerability::THEY,
            RelativeVulnerability::ALL,
        ];
        let auction = [bid(1, Strain::Diamonds), ONE_HEART, P, TWO_HEARTS];
        let hand: Hand = "765.A.AKJT984.63".parse().expect("valid test hand");

        for (index, profile) in profiles.into_iter().enumerate() {
            crate::bidding::set_reading_scope(profile.scope);
            crate::bidding::set_envelope_union_reading(profile.union);
            crate::bidding::set_pass_exclusion_reading(profile.exclusion);
            crate::bidding::evaluator::set_eval_auction(profile.eval_auction);
            crate::bidding::evaluator::set_eval_shape(profile.eval_shape);
            crate::bidding::features::set_blind_inference(profile.blind);
            crate::bidding::instinct::set_bilans_floor(profile.bilans);
            crate::bidding::instinct::set_net_collar(profile.collar);

            let stance = if profile.configured {
                american_configured().against()
            } else {
                american_instinct().against()
            };
            let vul = vulnerabilities[index % vulnerabilities.len()];
            let cached = stance
                .classify_with_provenance(hand, vul, &auction)
                .expect("cached profile classifies");
            let uncached = stance
                .classify_with_provenance_uncached(hand, vul, &auction)
                .expect("reference profile classifies");
            assert_eq!(cached.1, uncached.1, "profile {index}");
            for ((call, actual), (reference_call, reference)) in
                (&cached.0.0).into_iter().zip(&uncached.0.0)
            {
                assert_eq!(call, reference_call);
                assert_eq!(
                    actual.to_bits(),
                    reference.to_bits(),
                    "profile {index}, call {call:?}"
                );
            }
        }

        // Restore this worker thread's shipped defaults for later tests.
        crate::bidding::set_reading_scope(ReadingScope::Alerted);
        crate::bidding::set_envelope_union_reading(true);
        crate::bidding::set_pass_exclusion_reading(false);
        crate::bidding::evaluator::set_eval_auction(true);
        crate::bidding::evaluator::set_eval_shape(false);
        crate::bidding::features::set_blind_inference(false);
        crate::bidding::instinct::set_bilans_floor(true);
        crate::bidding::instinct::set_net_collar(false);
    }

    /// Component-by-component release parity over the frozen stage-1 corpus.
    ///
    /// Unlike the live-deal sweep below, this names every cached intermediate:
    /// full inference unions (including announced-box order), every evaluator
    /// feature generation, the forward-pass result, interpretation, routing,
    /// legal selection, and explanation attribution.
    #[test]
    #[ignore = "release component parity over 512 frozen positions"]
    fn cached_and_uncached_match_frozen_performance_corpus() {
        use super::ExplainedRule;
        use crate::bidding::american::american;
        use crate::bidding::array::Logits;
        use crate::bidding::evaluator::trick_estimates_with_auction;
        use crate::bidding::features::{features_eval, features_eval_v3, features_eval_v4};
        use crate::bidding::inference::{Inferences, ReadingScope};
        use crate::bidding::instinct::Interpretation;
        use crate::bidding::trie::Provenance;
        use contract_bridge::auction::Auction;

        fn set_shipped_profile() {
            crate::bidding::set_reading_scope(ReadingScope::Alerted);
            crate::bidding::set_envelope_union_reading(true);
            crate::bidding::set_pass_exclusion_reading(false);
            crate::bidding::evaluator::set_eval_auction(true);
            crate::bidding::evaluator::set_eval_shape(false);
            crate::bidding::features::set_blind_inference(false);
            crate::bidding::instinct::set_bilans_floor(true);
            crate::bidding::instinct::set_net_collar(false);
        }

        fn assert_float_bits(actual: &[f32], reference: &[f32], component: &str, position: u16) {
            assert_eq!(
                actual.len(),
                reference.len(),
                "{component} width at position {position}"
            );
            for (index, (&actual, &reference)) in actual.iter().zip(reference).enumerate() {
                assert_eq!(
                    actual.to_bits(),
                    reference.to_bits(),
                    "{component}[{index}] bits at position {position}"
                );
            }
        }

        fn assert_logits_bits(
            actual: &Logits,
            reference: &Logits,
            auction: &[Call],
            position: u16,
        ) {
            for ((call, actual), (reference_call, reference)) in
                (&actual.0).into_iter().zip(&reference.0)
            {
                assert_eq!(call, reference_call);
                assert_eq!(
                    actual.to_bits(),
                    reference.to_bits(),
                    "logit bits for {call:?} at position {position}, {auction:?}"
                );
            }
        }

        fn legal_call(logits: &Logits, auction: &[Call]) -> Call {
            let mut played = Auction::new();
            played
                .try_extend(auction.iter().copied())
                .expect("the frozen corpus contains legal prefixes");
            let mut scored: Vec<(Call, f32)> = logits
                .iter()
                .map(|(call, &logit)| (call, logit))
                .filter(|&(_, logit)| logit.is_finite())
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
            scored
                .into_iter()
                .map(|(call, _)| call)
                .find(|&call| played.can_push(call).is_ok())
                .unwrap_or(Call::Pass)
        }

        fn assert_explanation_identity(
            actual: Option<(Provenance, Option<ExplainedRule>)>,
            reference: Option<(Provenance, Option<ExplainedRule>)>,
            position: u16,
        ) {
            match (actual, reference) {
                (
                    Some((actual_provenance, actual_rule)),
                    Some((reference_provenance, reference_rule)),
                ) => {
                    assert_eq!(
                        actual_provenance, reference_provenance,
                        "explanation provenance at position {position}"
                    );
                    match (actual_rule, reference_rule) {
                        (Some(actual), Some(reference)) => {
                            assert_eq!(actual.index, reference.index, "position {position}");
                            assert_eq!(actual.label, reference.label, "position {position}");
                            assert_eq!(
                                actual.description, reference.description,
                                "position {position}"
                            );
                            assert_eq!(actual.alert, reference.alert, "position {position}");
                        }
                        (None, None) => {}
                        (actual, reference) => panic!(
                            "explanation presence differs at position {position}: actual={actual:?}, reference={reference:?}"
                        ),
                    }
                }
                (None, None) => {}
                (actual, reference) => panic!(
                    "explanation resolution differs at position {position}: actual={actual:?}, reference={reference:?}"
                ),
            }
        }

        set_shipped_profile();
        let positions = performance_support::parse_corpus().expect("valid frozen corpus");
        assert_eq!(positions.len(), performance_support::POSITION_COUNT);
        let stance = american().against();

        for position in positions {
            let id = position.id;
            let hand = position.hand;
            let auction = position.auction.as_slice();
            let cached_context = stance.decision_context(hand, position.vul, auction);
            let uncached_context = stance.prefixed_context(position.vul, auction);

            let cached_inferences = cached_context.inferences();
            let uncached_inferences = Inferences::read(&uncached_context);
            assert_eq!(
                *cached_inferences, uncached_inferences,
                "full inference payload and box order at position {id}"
            );

            for (name, cached, uncached) in [
                (
                    "evaluator-v2-features",
                    features_eval(hand, &cached_inferences),
                    features_eval(hand, &uncached_inferences),
                ),
                (
                    "evaluator-v3-features",
                    features_eval_v3(hand, &cached_inferences, auction),
                    features_eval_v3(hand, &uncached_inferences, auction),
                ),
                (
                    "evaluator-v4-features",
                    features_eval_v4(hand, &cached_inferences, auction),
                    features_eval_v4(hand, &uncached_inferences, auction),
                ),
            ] {
                assert_float_bits(&cached, &uncached, name, id);
            }

            let cached_tricks = cached_context.trick_estimates(hand);
            let uncached_tricks = trick_estimates_with_auction(hand, &uncached_inferences, auction);
            assert_eq!(
                cached_tricks.bit_pattern(),
                uncached_tricks.bit_pattern(),
                "trick-estimate bits at position {id}"
            );

            assert_eq!(
                cached_context.interpretation(),
                Interpretation::read(&uncached_context),
                "auction interpretation at position {id}"
            );

            let cached = stance
                .trie_for(auction)
                .classify_floored(hand, &cached_context, auction)
                .expect("the cached default stance is total");
            let uncached = stance
                .classify_with_provenance_uncached(hand, position.vul, auction)
                .expect("the uncached default stance is total");
            assert_eq!(cached.1, uncached.1, "provenance at position {id}");
            assert_logits_bits(&cached.0, &uncached.0, auction, id);

            let cached_call = legal_call(&cached.0, auction);
            let uncached_call = legal_call(&uncached.0, auction);
            assert_eq!(cached_call, uncached_call, "legal call at position {id}");
            assert_explanation_identity(
                stance.explain_call_in_context(hand, &cached_context, cached_call),
                stance.explain_call_uncached(hand, position.vul, auction, uncached_call),
                id,
            );

            let counts = cached_context
                .decision_cache_init_counts()
                .expect("the cached corpus path has a decision scope");
            assert!(
                counts.0 <= 1,
                "inference initialized {counts:?} at position {id}"
            );
            assert!(
                counts.1 <= 1,
                "evaluator initialized {counts:?} at position {id}"
            );
            assert!(
                counts.2 <= 1,
                "interpretation initialized {counts:?} at position {id}"
            );
        }

        // This ignored test is often selected beside other release checks on a
        // reused harness worker; leave every setting it pins at the shipped value.
        set_shipped_profile();
    }

    /// Same-process release sweep of the causal cached path against its
    /// pre-cache oracle. Uses the identical deal/dealer/vulnerability schedule
    /// as `examples/smoke-default` and checks every live decision before either
    /// call is allowed to advance the auction.
    #[test]
    #[ignore = "release parity sweep over 20,000 complete deals"]
    fn cached_and_uncached_match_over_twenty_thousand_deals() {
        use crate::bidding::american::american;
        use crate::bidding::array::Logits;
        use crate::bidding::context::relative;
        use contract_bridge::auction::Auction;
        use contract_bridge::deck::full_deal;
        use contract_bridge::{AbsoluteVulnerability, Seat};
        use rand::SeedableRng as _;

        fn legal_call(logits: &Logits, auction: &Auction) -> Call {
            let mut scored: Vec<(Call, f32)> = logits
                .iter()
                .map(|(call, &logit)| (call, logit))
                .filter(|&(_, logit)| logit.is_finite())
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("logits are never NaN"));
            scored
                .into_iter()
                .map(|(call, _)| call)
                .find(|&call| auction.can_push(call).is_ok())
                .unwrap_or(Call::Pass)
        }

        let stance = american().against();
        let vulnerabilities = [
            AbsoluteVulnerability::NONE,
            AbsoluteVulnerability::NS,
            AbsoluteVulnerability::EW,
            AbsoluteVulnerability::ALL,
        ];
        let mut decisions = 0usize;

        for board in 0..20_000usize {
            let deal = full_deal(&mut rand::rngs::StdRng::seed_from_u64(
                1u64.wrapping_add(board as u64),
            ));
            let dealer = Seat::ALL[board % 4];
            let table_vul = vulnerabilities[(board / 4) % 4];
            let mut auction = Auction::new();

            while !auction.has_ended() {
                let seat = Seat::ALL[(dealer as usize + auction.len()) % 4];
                let vul = relative(table_vul, seat);
                let cached = stance
                    .classify_with_provenance(deal[seat], vul, &auction)
                    .expect("the default stance is total");
                let uncached = stance
                    .classify_with_provenance_uncached(deal[seat], vul, &auction)
                    .expect("the reference stance is total");

                assert_eq!(cached.1, uncached.1, "provenance on board {board}");
                for ((call, actual), (reference_call, reference)) in
                    (&cached.0.0).into_iter().zip(&uncached.0.0)
                {
                    assert_eq!(call, reference_call);
                    assert_eq!(
                        actual.to_bits(),
                        reference.to_bits(),
                        "logit bits for {call:?} on board {board} at {auction:?}"
                    );
                }

                let cached_call = legal_call(&cached.0, &auction);
                let uncached_call = legal_call(&uncached.0, &auction);
                assert_eq!(
                    cached_call, uncached_call,
                    "legal selection on board {board} at {auction:?}"
                );
                auction.push(cached_call);
                decisions += 1;
            }
        }

        assert!(
            decisions >= 80_000,
            "unexpectedly shallow corpus: {decisions}"
        );
    }
}
