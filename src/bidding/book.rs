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
use super::inference::{Envelope, Inferences, Range};
use super::trie::{Provenance, Trie};
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::{FullDeal, Hand, Seat, Suit};
use core::ops::{Deref, DerefMut};
use std::collections::HashMap;

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
    let context = Context::new(vul, auction).with_prefixes(trie.common_prefixes(auction));
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
        let mut bound = self.competitive.0.clone();
        let collisions = bound.merge(self.constructive.0.clone());
        debug_assert!(
            collisions.is_empty(),
            "competitive and constructive books collide at {collisions:?}"
        );

        Stance {
            constructive: self.constructive.0.clone(),
            competitive: bound,
            defensive: self.defensive.0.clone(),
            probed: HashMap::new(),
        }
    }
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
#[derive(Clone, Debug, Default)]
pub struct Stance {
    constructive: Trie,
    competitive: Trie,
    defensive: Trie,
    /// Behaviorally probed readings, keyed by auction prefix with leading
    /// passes stripped (dealer rotations merge, as the books fan).  Empty
    /// until [`probe`][Self::probe] runs; consumed by the projection pass
    /// under [`set_probed_reading`][super::set_probed_reading].
    probed: HashMap<Vec<Call>, Envelope>,
}

impl Stance {
    /// The trie answering for the auction's [`Phase`]
    ///
    /// [`Phase::of`] is relative to the side to act on the slice, so this
    /// also routes an *opponent's* prior call correctly: querying with the
    /// auction cut at their turn yields their side's phase and book — how the
    /// table-wide alert decode resolves their calls (`project_authored`).
    pub(crate) fn trie_for(&self, auction: &[Call]) -> &Trie {
        match Phase::of(auction) {
            Phase::Constructive => &self.constructive,
            Phase::Competitive => &self.competitive,
            Phase::Defensive => &self.defensive,
        }
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
    #[must_use]
    pub(crate) fn prefixed_context<'a>(
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
        self.points = self.points.union(other.points);
        for (a, b) in self.lengths.iter_mut().zip(other.lengths) {
            *a = a.union(b);
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
            super::inference::set_probed_reading(probed_on);
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

impl System for Stance {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        self.classify_with_provenance(hand, vul, auction)
            .map(|(logits, _)| logits)
    }

    fn authored_at(&self, vul: RelativeVulnerability, auction: &[Call]) -> bool {
        // Delegate to the phase-routed trie's rebasing-aware check.
        self.trie_for(auction).authored_at(vul, auction)
    }
}

#[cfg(test)]
mod tests {
    use super::Phase;
    use contract_bridge::auction::Call;
    use contract_bridge::{Bid, Strain, Suit};

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
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

    const P: Call = Call::Pass;
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
}
