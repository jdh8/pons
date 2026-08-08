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
use super::context::{Context, DecisionProfile};
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
/// opening prefix is empty, `-`, `- -`, or `- - -` for 1st through 4th seat,
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
/// Keyed by the raw table auction starting from their opening: `(1♠)` is our
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
/// Keyed by the raw table auction like its siblings: `1♥ (2♣)` is our
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
        let profile = DecisionProfile::current();
        debug_assert!(
            compiled_rules.profile == profile.reading,
            "the compiled-rule sidecar and the stance captured different reading profiles"
        );
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
            opponents: None,
            cache_identity: Arc::new(StanceCacheIdentity::default()),
            profile,
        }
    }
}

/// Build under a scratch knob state, without touching this thread's
///
/// Runs `build` on a fresh scoped thread, whose thread-local knobs start at
/// the shipped defaults, and returns its value; the closure's `set_*` calls
/// die with the thread.  Because a [`Stance`] pins the knob state it is built
/// under, the returned value keeps the armed behavior wherever it later
/// classifies:
///
/// ```
/// use pons::bidding::{american, scoped};
///
/// let stance = scoped(|| {
///     pons::american::set_garbage_stayman(false);
///     american().against()
/// });
/// ```
///
/// The caller's own knob state is never read and never modified — arming for
/// a *derived* baseline (defaults plus the caller's prior tweaks) needs the
/// plain set-then-build flow instead.
pub fn scoped<T: Send>(build: impl FnOnce() -> T + Send) -> T {
    match std::thread::scope(|scope| scope.spawn(build).join()) {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
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
#[derive(Clone)]
pub struct Stance {
    constructive: BoundBook,
    competitive: BoundBook,
    defensive: BoundBook,
    /// Behaviorally probed readings, keyed by auction prefix with leading
    /// passes stripped (dealer rotations merge, as the books fan).  Empty
    /// until [`probe`][Self::probe] runs; consumed by the projection pass
    /// under [`set_probed_reading`][super::set_probed_reading].
    probed: HashMap<Vec<Call>, Envelope>,
    /// The books the *opponents* are declared to play, for reading their
    /// calls ([`with_opponents`][Self::with_opponents]).  [`None`] — the default — models them as
    /// playing ours, which is exact in self-play.
    opponents: Option<Arc<Stance>>,
    /// Stable across clones, replaced whenever stance-local reading data
    /// changes. Deal caches retain this small token, so an allocator cannot
    /// recycle a raw address into a false identity match.
    cache_identity: Arc<StanceCacheIdentity>,
    /// The knob state pinned at build ([`Pair::against`]): classify-time knob
    /// reads serve off this copy, so the stance answers identically on every
    /// thread.  Re-captured only by [`repin`][Self::repin].
    profile: DecisionProfile,
}

impl Default for Stance {
    /// An empty stance pinned to the building thread's current knob state
    fn default() -> Self {
        Self {
            constructive: BoundBook::default(),
            competitive: BoundBook::default(),
            defensive: BoundBook::default(),
            probed: HashMap::new(),
            opponents: None,
            cache_identity: Arc::new(StanceCacheIdentity::default()),
            profile: DecisionProfile::current(),
        }
    }
}

impl core::fmt::Debug for Stance {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Stance")
            .field("constructive", &self.constructive.trie)
            .field("competitive", &self.competitive.trie)
            .field("defensive", &self.defensive.trie)
            .field("probed", &self.probed)
            .field("declared_opponents", &self.opponents.is_some())
            .finish()
    }
}

impl Stance {
    pub(crate) fn cache_identity(&self) -> &Arc<StanceCacheIdentity> {
        &self.cache_identity
    }

    /// The knob state pinned into this stance at build
    pub(crate) fn profile(&self) -> DecisionProfile {
        self.profile
    }

    /// Re-capture this thread's knob state into the stance
    ///
    /// A stance pins the classify-time knob state when it is built
    /// ([`Pair::against`]); later `set_*` calls do not move it.  This is the
    /// deliberate escape hatch for a set-*after*-build flow: call the setters,
    /// then `repin` the stance they should govern.  Prefer rebuilding when the
    /// changed knob also shapes the books — repinning moves only the
    /// classify-time reads.
    pub fn repin(&mut self) {
        self.profile = DecisionProfile::current();
        self.invalidate_cache_identity();
    }

    /// Move this stance's probed-overlay bit — [`probe`][Self::probe]'s driver
    fn set_probed_reading(&mut self, on: bool) {
        self.profile.reading.set_probed(on);
        self.invalidate_cache_identity();
    }

    /// Read the opponents' calls off `them` instead of off our own books
    ///
    /// The reading half of a declared opponent (rows Phase 2b): their alerted
    /// calls and passes decode in *their* phase-routed books, so a mixed table
    /// stops reading their 1♣ as ours.  It changes reading only — our own
    /// calls keep resolving in our books, and neither side's *bidding* table
    /// moves.  Nesting is ignored: only `them`'s own books are consulted, not
    /// whatever `them` in turn declares.
    ///
    /// The net's twin channel is
    /// [`Config`][super::features::Config], attached separately; the two are
    /// deliberately not wired together, because declaring a foreign card to
    /// the net measured as a loss (`docs/declarative-rows.md` §2a).
    #[must_use]
    pub fn with_opponents(mut self, them: &Self) -> Self {
        self.opponents = Some(Arc::new(them.clone()));
        self.invalidate_cache_identity();
        self
    }

    /// The books modeling the opponents — declared ([`with_opponents`][Self::with_opponents]) or ours
    pub(crate) fn opponents(&self) -> &Self {
        self.opponents.as_deref().unwrap_or(self)
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
            .with_system(self)
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
            .with_system(self);
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
            .with_system(self);
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
            .with_system(self);
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
            .with_system(self)
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
        let harvest = |stance: &Self| -> HashMap<Vec<Call>, Observed> {
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

        // The two passes differ only in whether the stance reads its own
        // probed map, which is now a bit of its pinned profile rather than a
        // thread-local — so the fixed point is driven by mutating this stance,
        // and a probe on one thread no longer leaks into another's readings.
        // The second pass must serve the first pass's boxes through the fold
        // consumption will use: build under `set_probed_vacuous_reading` and
        // the fixed point runs under the vacuous scope (the full-fold toggle
        // stays off); otherwise the full fold serves, as before.  A re-probe
        // of a stance with a stale map would serve that map in its first pass
        // — probe fresh stances.
        let was = self.profile.reading.probed_reading();
        let vacuous = self.profile.reading.probed_vacuous_reading();
        self.set_probed_reading(false);
        let first = boxed(harvest(self));
        self.probed = first;
        self.set_probed_reading(!vacuous);
        let second = boxed(harvest(self));
        self.set_probed_reading(was);

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
mod tests;
