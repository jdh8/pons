//! Mechanical context of an auction
//!
//! [`Context`] packages everything a [`Classifier`][super::trie::Classifier]
//! or a [`Constraint`][super::constraint::Constraint] may consult besides the
//! hand itself: vulnerability, the raw table auction, and facts derived from
//! it (who bid which strains, the contract to beat, passed-hand status).
//!
//! All facts here are *mechanical*: they follow from the laws of the game
//! alone.  System interpretation (such as forcing status) deliberately does
//! not live here — it belongs to classifiers, which know their system.

use super::book::Stance;
use super::evaluator::{TrickEstimates, trick_estimates_with_auction};
use super::features::Config;
use super::inference::{AuthoredProjection, Inferences, ReadingProfile, reading_profile};
use super::instinct::Interpretation;
use super::trie::CommonPrefixes;
use contract_bridge::auction::{AbsoluteVulnerability, Call, RelativeVulnerability};
use contract_bridge::{Bid, Hand, Level, Penalty, Seat, Strain, Suit};
use core::fmt;
use std::borrow::Cow;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::{self, ThreadId};

#[cfg(test)]
use std::sync::atomic::AtomicUsize;

/// Convert absolute vulnerability to the perspective of a seat
///
/// This is the only vulnerability conversion in the crate: drivers call it
/// once per [`classify`][super::System::classify] call, and systems pass the
/// relative value through unchanged.
#[must_use]
pub fn relative(vul: AbsoluteVulnerability, seat: Seat) -> RelativeVulnerability {
    let (we, they) = match seat {
        Seat::North | Seat::South => (AbsoluteVulnerability::NS, AbsoluteVulnerability::EW),
        Seat::East | Seat::West => (AbsoluteVulnerability::EW, AbsoluteVulnerability::NS),
    };
    let mut relative = RelativeVulnerability::NONE;
    relative.set(RelativeVulnerability::WE, vul.contains(we));
    relative.set(RelativeVulnerability::THEY, vul.contains(they));
    relative
}

/// The same vulnerability seen from the other side of the table
#[must_use]
pub(crate) fn flipped(vul: RelativeVulnerability) -> RelativeVulnerability {
    let mut flipped = RelativeVulnerability::NONE;
    flipped.set(
        RelativeVulnerability::WE,
        vul.contains(RelativeVulnerability::THEY),
    );
    flipped.set(
        RelativeVulnerability::THEY,
        vul.contains(RelativeVulnerability::WE),
    );
    flipped
}

/// Mechanical facts about an auction from the perspective of the side to act
///
/// A context is computed once per classification from the raw table auction
/// (all four players' calls).  "We" always refers to the partnership of the
/// player about to call, and the vulnerability is relative to that side.
#[derive(Clone)]
pub struct Context<'a> {
    vul: RelativeVulnerability,
    auction: &'a [Call],
    our_strains: u8,
    their_strains: u8,
    partner_last_bid: Option<Bid>,
    last_bid: Option<Bid>,
    penalty: Penalty,
    undisturbed: bool,
    passed_hand: bool,
    partner_passed_hand: bool,
    opening_index: Option<usize>,
    prefixes: Option<CommonPrefixes<'a, 'a>>,
    their_system: Option<&'a Stance>,
    config: Option<&'a Config>,
    authored_projection: Option<&'a AuthoredProjection>,
    revision: u64,
    decision_cache: Option<Arc<DecisionCache>>,
}

/// The thread-local inputs whose values must stay fixed during one decision
#[derive(Clone, Copy, PartialEq, Eq)]
struct DecisionProfile {
    reading: ReadingProfile,
    eval_auction: bool,
    eval_shape: bool,
    blind_inference: bool,
    two_over_one_force: bool,
}

impl DecisionProfile {
    fn current() -> Self {
        Self {
            reading: reading_profile(),
            eval_auction: super::evaluator::eval_auction(),
            eval_shape: super::evaluator::eval_shape(),
            blind_inference: super::features::blind_inference(),
            two_over_one_force: super::instinct::two_over_one_force(),
        }
    }
}

/// Values shared by every classifier consulted for one immutable decision
struct DecisionCache {
    hand: Hand,
    revision: u64,
    thread: ThreadId,
    profile: DecisionProfile,
    inferences: OnceLock<Inferences>,
    trick_estimates: OnceLock<TrickEstimates>,
    interpretation: OnceLock<Interpretation>,
    rule_face_slots: usize,
    rule_faces: [AtomicU8; 16],
    rule_face_overflow: Box<[AtomicU8]>,
    #[cfg(test)]
    inference_inits: AtomicUsize,
    #[cfg(test)]
    trick_estimate_inits: AtomicUsize,
    #[cfg(test)]
    interpretation_inits: AtomicUsize,
}

/// Owned mechanical auction cursor used by the deal-level append cache.
///
/// It carries only laws-derived facts; constructing a borrowed [`Context`]
/// from the current prefix is constant work.
#[derive(Clone, Debug)]
pub(crate) struct ContextCursor {
    depth: usize,
    strains: [u8; 2],
    side_has_nonpass: [bool; 2],
    last_by_seat: [Option<Bid>; 4],
    last_bid: Option<Bid>,
    penalty: Penalty,
    opening_index: Option<usize>,
}

impl ContextCursor {
    pub(crate) const fn new() -> Self {
        Self {
            depth: 0,
            strains: [0; 2],
            side_has_nonpass: [false; 2],
            last_by_seat: [None; 4],
            last_bid: None,
            penalty: Penalty::Undoubled,
            opening_index: None,
        }
    }

    pub(crate) fn context<'a>(
        &self,
        vul: RelativeVulnerability,
        auction: &'a [Call],
    ) -> Context<'a> {
        debug_assert_eq!(self.depth, auction.len());
        let side = self.depth % 2;
        Context {
            vul,
            auction,
            our_strains: self.strains[side],
            their_strains: self.strains[1 - side],
            partner_last_bid: self.last_by_seat[(self.depth + 2) % 4],
            last_bid: self.last_bid,
            penalty: self.penalty,
            undisturbed: !self.side_has_nonpass[1 - side],
            passed_hand: self.depth >= 4 && matches!(auction[self.depth % 4], Call::Pass),
            partner_passed_hand: self.depth >= 2
                && matches!(auction[(self.depth - 2) % 4], Call::Pass),
            opening_index: self.opening_index,
            prefixes: None,
            their_system: None,
            config: None,
            authored_projection: None,
            revision: 0,
            decision_cache: None,
        }
    }

    pub(crate) fn push(&mut self, call: Call) {
        let side = self.depth % 2;
        if call != Call::Pass {
            self.side_has_nonpass[side] = true;
            self.opening_index.get_or_insert(self.depth);
        }
        match call {
            Call::Pass => {}
            Call::Double => self.penalty = Penalty::Doubled,
            Call::Redouble => self.penalty = Penalty::Redoubled,
            Call::Bid(bid) => {
                self.strains[side] |= 1 << bid.strain as u8;
                self.last_by_seat[self.depth % 4] = Some(bid);
                self.last_bid = Some(bid);
                self.penalty = Penalty::Undoubled;
            }
        }
        self.depth += 1;
    }

    pub(crate) fn phase(&self) -> super::book::Phase {
        let Some(opening) = self.opening_index else {
            return super::book::Phase::Constructive;
        };
        let side = self.depth % 2;
        if opening % 2 != side {
            super::book::Phase::Defensive
        } else if self.side_has_nonpass[1 - side] {
            super::book::Phase::Competitive
        } else {
            super::book::Phase::Constructive
        }
    }
}

impl Default for ContextCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionCache {
    fn new(
        hand: Hand,
        revision: u64,
        thread: ThreadId,
        profile: DecisionProfile,
        rule_face_slots: usize,
    ) -> Self {
        Self {
            hand,
            revision,
            thread,
            profile,
            inferences: OnceLock::new(),
            trick_estimates: OnceLock::new(),
            interpretation: OnceLock::new(),
            rule_face_slots,
            rule_faces: [const { AtomicU8::new(0) }; 16],
            rule_face_overflow: (16..rule_face_slots).map(|_| AtomicU8::new(0)).collect(),
            #[cfg(test)]
            inference_inits: AtomicUsize::new(0),
            #[cfg(test)]
            trick_estimate_inits: AtomicUsize::new(0),
            #[cfg(test)]
            interpretation_inits: AtomicUsize::new(0),
        }
    }

    fn reusable(
        &self,
        hand: Hand,
        revision: u64,
        thread: ThreadId,
        profile: DecisionProfile,
        rule_face_slots: usize,
    ) -> bool {
        self.hand == hand
            && self.revision == revision
            && self.thread == thread
            && self.profile == profile
            && self.rule_face_slots == rule_face_slots
    }

    fn assert_fixed_call(&self) {
        debug_assert!(
            self.thread == thread::current().id(),
            "a decision cache crossed its creating thread"
        );
        debug_assert!(
            self.profile == DecisionProfile::current(),
            "a reading or evaluator setting changed during one decision"
        );
    }
}

// Keep `Context`'s diagnostic representation stable: the cache and its
// structural revision are serving mechanics, not auction facts.
impl fmt::Debug for Context<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Context")
            .field("vul", &self.vul)
            .field("auction", &self.auction)
            .field("our_strains", &self.our_strains)
            .field("their_strains", &self.their_strains)
            .field("partner_last_bid", &self.partner_last_bid)
            .field("last_bid", &self.last_bid)
            .field("penalty", &self.penalty)
            .field("undisturbed", &self.undisturbed)
            .field("passed_hand", &self.passed_hand)
            .field("partner_passed_hand", &self.partner_passed_hand)
            .field("opening_index", &self.opening_index)
            .field("prefixes", &self.prefixes)
            .field("their_system", &self.their_system)
            .field("config", &self.config)
            .finish()
    }
}

impl<'a> Context<'a> {
    /// Compute the context of an auction
    ///
    /// `vul` must be relative to the side to act, and `auction` must be the
    /// raw table auction including all passes.
    #[must_use]
    pub fn new(vul: RelativeVulnerability, auction: &'a [Call]) -> Self {
        let len = auction.len();
        let mut context = Self {
            vul,
            auction,
            our_strains: 0,
            their_strains: 0,
            partner_last_bid: None,
            last_bid: None,
            penalty: Penalty::Undoubled,
            undisturbed: true,
            passed_hand: len >= 4 && matches!(auction[len % 4], Call::Pass),
            partner_passed_hand: len >= 2 && matches!(auction[(len - 2) % 4], Call::Pass),
            opening_index: auction.iter().position(|&call| call != Call::Pass),
            prefixes: None,
            their_system: None,
            config: None,
            authored_projection: None,
            revision: 0,
            decision_cache: None,
        };

        for (index, &call) in auction.iter().enumerate() {
            let ours = (len - index).is_multiple_of(2);

            match call {
                Call::Pass => {}
                Call::Double => {
                    context.penalty = Penalty::Doubled;
                    context.undisturbed &= ours;
                }
                Call::Redouble => {
                    context.penalty = Penalty::Redoubled;
                    context.undisturbed &= ours;
                }
                Call::Bid(bid) => {
                    context.last_bid = Some(bid);
                    context.penalty = Penalty::Undoubled;
                    context.undisturbed &= ours;

                    if ours {
                        context.our_strains |= 1 << bid.strain as u8;
                        if (len - index) % 4 == 2 {
                            context.partner_last_bid = Some(bid);
                        }
                    } else {
                        context.their_strains |= 1 << bid.strain as u8;
                    }
                }
            }
        }
        context
    }

    /// Build the at-the-time context for every auction prefix in one scan.
    ///
    /// Entry `i` is byte-for-byte the mechanical state of
    /// `Context::new(vul_at_i, &auction[..i])`, with vulnerability flipped on
    /// turns belonging to the other partnership.  Maintaining both parity
    /// sides at once avoids rescanning every historical prefix in the authored
    /// reader.
    pub(crate) fn at_each_turn(
        vul_at_end: RelativeVulnerability,
        auction: &'a [Call],
    ) -> Vec<Self> {
        let len = auction.len();
        let mut contexts = Vec::with_capacity(len + 1);
        let mut cursor = ContextCursor::new();

        for depth in 0..=len {
            let vul = if (len - depth).is_multiple_of(2) {
                vul_at_end
            } else {
                flipped(vul_at_end)
            };
            contexts.push(cursor.context(vul, &auction[..depth]));
            if let Some(&call) = auction.get(depth) {
                cursor.push(call);
            }
        }
        contexts
    }

    /// Attach the common prefixes of the auction in the queried [`Trie`]
    ///
    /// [`Trie`]: super::Trie
    #[must_use]
    pub const fn with_prefixes(mut self, prefixes: CommonPrefixes<'a, 'a>) -> Self {
        self.prefixes = Some(prefixes);
        // Keep this builder `const`: retaining an attached `Arc` and rejecting
        // it by revision avoids a const-incompatible drop.
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// Attach a model of the *opponents'* system for table-wide alert reading
    ///
    /// Alerts are disclosure to the whole table, so the reading layer may
    /// decode the opponents' alerted calls off their authoring rules — when it
    /// has a model of their books.  [`Stance::prefixed_context`] attaches the
    /// stance itself: the opponents are modeled as playing our own books,
    /// which is exact in self-play and an approximation against other
    /// natural-family engines.  Consumed by `project_authored` behind
    /// [`set_table_alert_reading`][super::inference::set_table_alert_reading].
    #[must_use]
    pub(crate) const fn with_their_system(mut self, them: &'a Stance) -> Self {
        self.their_system = Some(them);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The opponents' modeled system, if attached ([`Self::with_their_system`])
    #[must_use]
    pub(crate) const fn their_system(&self) -> Option<&'a Stance> {
        self.their_system
    }

    /// Attach both partnerships' convention cards, for
    /// [`features_v4`][super::features::features_v4]
    ///
    /// The sibling of the crate-internal `with_their_system`, and the same idea
    /// one layer down: that one gives the *reader* a model of the opponents'
    /// books, this one gives the *net* the agreements of both sides as inputs.
    ///
    /// Carried by reference and encoded once per configuration cell, so the
    /// per-decision path neither allocates nor reads ambient knob state — the
    /// point of putting this on the context rather than in a thread-local
    /// beside the knobs, where it would silently change what a feature vector
    /// means.
    #[must_use]
    pub const fn with_config(mut self, config: &'a Config) -> Self {
        self.config = Some(config);
        self
    }

    /// Attach the append-only deal cache's authored projection snapshot.
    #[must_use]
    pub(crate) const fn with_authored_projection(
        mut self,
        projection: &'a AuthoredProjection,
    ) -> Self {
        self.authored_projection = Some(projection);
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The deal-cached authored projection, when serving through a table loop.
    #[must_use]
    pub(crate) const fn authored_projection(&self) -> Option<&'a AuthoredProjection> {
        self.authored_projection
    }

    /// Attach the cache shared by every route attempted for this decision
    ///
    /// Calling this on an already-scoped clone preserves the existing cache
    /// when its hand, auction structure, thread, and active profile all match.
    /// A bare [`Context::new`] remains uncached until serving code enters this
    /// scope explicitly.
    #[must_use]
    pub(crate) fn with_decision_cache(mut self, hand: Hand) -> Self {
        self.install_decision_cache(hand, 0);
        self
    }

    /// Enter a serving decision with fixed slots for compiled pure face gates.
    ///
    /// The stance-wide compiler assigns the slots. Standalone public rule
    /// tables keep using [`Self::with_decision_cache`] with zero slots and
    /// retain opaque face evaluation semantics.
    #[must_use]
    pub(crate) fn with_compiled_decision_cache(
        mut self,
        hand: Hand,
        rule_face_slots: usize,
    ) -> Self {
        self.install_decision_cache(hand, rule_face_slots);
        self
    }

    fn install_decision_cache(&mut self, hand: Hand, rule_face_slots: usize) {
        let thread = thread::current().id();
        let profile = DecisionProfile::current();
        let reusable = self.decision_cache.as_deref().is_some_and(|cache| {
            cache.reusable(hand, self.revision, thread, profile, rule_face_slots)
        });
        if !reusable {
            self.decision_cache = Some(Arc::new(DecisionCache::new(
                hand,
                self.revision,
                thread,
                profile,
                rule_face_slots,
            )));
        }
    }

    /// The active cache, if no structural builder has invalidated it
    fn active_decision_cache(&self) -> Option<&DecisionCache> {
        let cache = self.decision_cache.as_deref()?;
        if cache.revision != self.revision {
            return None;
        }
        cache.assert_fixed_call();
        Some(cache)
    }

    /// Reading profile already captured at decision entry, or the current
    /// thread profile for an uncached diagnostic context.
    pub(crate) fn reading_profile(&self) -> ReadingProfile {
        self.active_decision_cache()
            .map_or_else(reading_profile, |cache| cache.profile.reading)
    }

    /// Evaluate one explicitly pure compiled face gate at most once in this
    /// immutable decision. Opaque gates never call this method.
    pub(crate) fn compiled_rule_face(&self, slot: u32, evaluate: impl FnOnce() -> bool) -> bool {
        let Some(cache) = self.active_decision_cache() else {
            return evaluate();
        };
        let slot = slot as usize;
        if slot >= cache.rule_face_slots {
            return evaluate();
        };
        let value = if slot < cache.rule_faces.len() {
            &cache.rule_faces[slot]
        } else {
            &cache.rule_face_overflow[slot - cache.rule_faces.len()]
        };
        match value.load(Ordering::Relaxed) {
            1 => false,
            2 => true,
            _ => {
                let live = evaluate();
                value.store(if live { 2 } else { 1 }, Ordering::Relaxed);
                live
            }
        }
    }

    /// Read the auction once within a decision, or return an owned uncached read
    #[must_use]
    pub(crate) fn inferences(&self) -> Cow<'_, Inferences> {
        let Some(cache) = self.active_decision_cache() else {
            return Cow::Owned(Inferences::read(self));
        };
        Cow::Borrowed(cache.inferences.get_or_init(|| {
            #[cfg(test)]
            cache.inference_inits.fetch_add(1, Ordering::Relaxed);
            Inferences::read(self)
        }))
    }

    /// Evaluate tricks once for the hand that owns this decision scope
    #[must_use]
    pub(crate) fn trick_estimates(&self, hand: Hand) -> TrickEstimates {
        let Some(cache) = self.active_decision_cache() else {
            let inferences = self.inferences();
            return trick_estimates_with_auction(hand, &inferences, self.auction());
        };
        if cache.hand != hand {
            let inferences = self.inferences();
            return trick_estimates_with_auction(hand, &inferences, self.auction());
        }
        *cache.trick_estimates.get_or_init(|| {
            #[cfg(test)]
            cache.trick_estimate_inits.fetch_add(1, Ordering::Relaxed);
            let inferences = self.inferences();
            trick_estimates_with_auction(hand, &inferences, self.auction())
        })
    }

    /// Read the auction-only instinct flags once within a decision
    #[must_use]
    pub(crate) fn interpretation(&self) -> Interpretation {
        let Some(cache) = self.active_decision_cache() else {
            return Interpretation::read(self);
        };
        *cache.interpretation.get_or_init(|| {
            #[cfg(test)]
            cache.interpretation_inits.fetch_add(1, Ordering::Relaxed);
            Interpretation::read(self)
        })
    }

    /// Test-only initialization counts for the active decision cache
    #[cfg(test)]
    pub(crate) fn decision_cache_init_counts(&self) -> Option<(usize, usize, usize)> {
        let cache = self.active_decision_cache()?;
        Some((
            cache.inference_inits.load(Ordering::Relaxed),
            cache.trick_estimate_inits.load(Ordering::Relaxed),
            cache.interpretation_inits.load(Ordering::Relaxed),
        ))
    }

    /// The attached configuration, if any ([`Self::with_config`])
    #[must_use]
    pub const fn config(&self) -> Option<&'a Config> {
        self.config
    }

    /// Vulnerability relative to the side to act
    #[must_use]
    pub const fn vul(&self) -> RelativeVulnerability {
        self.vul
    }

    /// The raw table auction
    #[must_use]
    pub const fn auction(&self) -> &'a [Call] {
        self.auction
    }

    /// Whether our side has bid the strain
    #[must_use]
    pub const fn we_bid(&self, strain: Strain) -> bool {
        self.our_strains & (1 << strain as u8) != 0
    }

    /// Whether the opponents have bid the strain
    #[must_use]
    pub const fn they_bid(&self, strain: Strain) -> bool {
        self.their_strains & (1 << strain as u8) != 0
    }

    /// Iterate over the suits the opponents have bid
    pub fn their_suits(&self) -> impl Iterator<Item = Suit> + use<> {
        let strains = self.their_strains;
        Suit::ASC
            .into_iter()
            .filter(move |&suit| strains & (1 << Strain::from(suit) as u8) != 0)
    }

    /// The last bid made by partner, if any
    #[must_use]
    pub const fn partner_last_bid(&self) -> Option<Bid> {
        self.partner_last_bid
    }

    /// The suit of partner's last bid, if it was a suit bid
    #[must_use]
    pub fn partner_last_suit(&self) -> Option<Suit> {
        self.partner_last_bid.and_then(|bid| bid.strain.suit())
    }

    /// The highest bid so far — the contract to beat
    #[must_use]
    pub const fn last_bid(&self) -> Option<Bid> {
        self.last_bid
    }

    /// Doubling state of the last bid
    #[must_use]
    pub const fn penalty(&self) -> Penalty {
        self.penalty
    }

    /// Whether the opponents have made nothing but passes
    #[must_use]
    pub const fn undisturbed(&self) -> bool {
        self.undisturbed
    }

    /// Whether the player to act passed on their first turn
    #[must_use]
    pub const fn passed_hand(&self) -> bool {
        self.passed_hand
    }

    /// Whether partner passed on their first turn
    #[must_use]
    pub const fn partner_passed_hand(&self) -> bool {
        self.partner_passed_hand
    }

    /// Number of passes before the first non-pass call
    #[must_use]
    pub fn leading_passes(&self) -> usize {
        self.opening_index.unwrap_or(self.auction.len())
    }

    /// The seat number (1–4) about to make the first non-pass call
    ///
    /// Returns [`None`] once anyone has acted, or when the auction has been
    /// passed out.
    #[must_use]
    pub fn seat_to_open(&self) -> Option<u8> {
        // SAFETY: the auction length is at most 3 here, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        (self.opening_index.is_none() && self.auction.len() < 4)
            .then(|| self.auction.len() as u8 + 1)
    }

    /// The opening bid — the first non-pass call — if anyone has acted
    ///
    /// Always a [`Bid`]: a double needs a prior bid to double, so the first
    /// non-pass call of a legal auction cannot be `Double` or `Redouble`.
    #[must_use]
    pub fn opening_bid(&self) -> Option<Bid> {
        match self.auction.get(self.opening_index?) {
            Some(&Call::Bid(bid)) => Some(bid),
            _ => None,
        }
    }

    /// The seat number (1–4) of the first non-pass call, if any
    #[must_use]
    pub fn opener_seat(&self) -> Option<u8> {
        // SAFETY: at most 3 passes may precede the opening, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        self.opening_index.map(|index| index as u8 + 1)
    }

    /// Whether **our** side made the opening bid
    ///
    /// False when nobody has opened yet.  Seats alternate, so our side opened
    /// exactly when an even number of calls separates the opening from ours.
    #[must_use]
    pub fn we_opened(&self) -> bool {
        match self.opener_seat() {
            Some(seat) => (self.auction.len() - (seat as usize - 1)).is_multiple_of(2),
            None => false,
        }
    }

    /// The cheapest level at which the strain can legally be bid
    ///
    /// Returns [`None`] when no bid in the strain is available anymore.
    #[must_use]
    pub fn min_level(&self, strain: Strain) -> Option<Level> {
        match self.last_bid {
            None => Some(Level::new(1)),
            Some(last) if strain > last.strain => Some(last.level),
            Some(last) => Level::try_new(last.level.get() + 1).ok(),
        }
    }

    /// Common prefixes of the auction in the queried [`Trie`], if attached
    ///
    /// [`Trie`]: super::Trie
    #[must_use]
    pub const fn prefixes(&self) -> Option<&CommonPrefixes<'a, 'a>> {
        self.prefixes.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::card::american_card;
    use crate::bidding::trie::Trie;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    fn test_hand() -> Hand {
        "AKQ2.KQ53.QJ4.92".parse().expect("valid test hand")
    }

    #[test]
    fn test_relative_vulnerability() {
        assert_eq!(
            relative(AbsoluteVulnerability::NS, Seat::North),
            RelativeVulnerability::WE,
        );
        assert_eq!(
            relative(AbsoluteVulnerability::NS, Seat::East),
            RelativeVulnerability::THEY,
        );
        assert_eq!(
            relative(AbsoluteVulnerability::ALL, Seat::West),
            RelativeVulnerability::ALL,
        );
        assert_eq!(
            relative(AbsoluteVulnerability::NONE, Seat::South),
            RelativeVulnerability::NONE,
        );
    }

    #[test]
    fn test_contested_auction_facts() {
        // We opened 1♠, LHO passed, partner bid 2♣, RHO doubled; we act next.
        let auction = [
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Double,
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert!(context.we_bid(Strain::Spades));
        assert!(context.we_bid(Strain::Clubs));
        assert!(!context.they_bid(Strain::Spades));
        assert_eq!(context.partner_last_bid(), Some(Bid::new(2, Strain::Clubs)));
        assert_eq!(context.partner_last_suit(), Some(Suit::Clubs));
        assert_eq!(context.last_bid(), Some(Bid::new(2, Strain::Clubs)));
        assert_eq!(context.penalty(), Penalty::Doubled);
        assert!(!context.undisturbed());
        assert!(!context.passed_hand());
        assert!(!context.partner_passed_hand());
        assert_eq!(context.opener_seat(), Some(1));
    }

    #[test]
    fn incremental_turn_contexts_match_fresh_construction() {
        let auction = [
            bid(1, Strain::Clubs),
            Call::Double,
            Call::Redouble,
            Call::Pass,
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
        ];
        for final_vul in [
            RelativeVulnerability::NONE,
            RelativeVulnerability::WE,
            RelativeVulnerability::THEY,
            RelativeVulnerability::ALL,
        ] {
            let timeline = Context::at_each_turn(final_vul, &auction);
            assert_eq!(timeline.len(), auction.len() + 1);
            for (depth, actual) in timeline.iter().enumerate() {
                let vul = if (auction.len() - depth).is_multiple_of(2) {
                    final_vul
                } else {
                    flipped(final_vul)
                };
                let expected = Context::new(vul, &auction[..depth]);
                assert_eq!(
                    format!("{actual:?}"),
                    format!("{expected:?}"),
                    "context differs at depth {depth}",
                );
            }
        }

        let mut cursor = ContextCursor::new();
        for depth in 0..=auction.len() {
            assert_eq!(
                cursor.phase(),
                super::super::book::Phase::of(&auction[..depth])
            );
            if let Some(&call) = auction.get(depth) {
                cursor.push(call);
            }
        }
    }

    #[test]
    fn test_their_suits_and_min_level() {
        // They opened 1♥ and raised to 2♥ over partner's 1♠ overcall.
        let auction = [
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(2, Strain::Hearts),
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert_eq!(context.their_suits().collect::<Vec<_>>(), [Suit::Hearts]);
        assert!(context.we_bid(Strain::Spades));
        assert_eq!(context.min_level(Strain::Hearts), Some(Level::new(3)));
        assert_eq!(context.min_level(Strain::Spades), Some(Level::new(2)));
        assert_eq!(context.min_level(Strain::Clubs), Some(Level::new(3)));
    }

    #[test]
    fn test_min_level_exhausted() {
        let auction = [bid(7, Strain::Notrump)];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        assert_eq!(context.min_level(Strain::Spades), None);
        assert_eq!(context.min_level(Strain::Notrump), None);
    }

    #[test]
    fn test_passed_hands() {
        // We passed, they overcalled 1♠ over partner's 1♥; we act next.
        let auction = [
            Call::Pass,
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
        ];
        let context = Context::new(RelativeVulnerability::NONE, &auction);

        assert!(context.passed_hand());
        assert!(!context.partner_passed_hand());
        assert_eq!(context.leading_passes(), 2);
        assert_eq!(context.opener_seat(), Some(3));
        assert_eq!(context.seat_to_open(), None);
        // Past the leading passes, and their 1♠ overcall does not overwrite it.
        assert_eq!(context.opening_bid(), Some(Bid::new(1, Strain::Hearts)));
    }

    #[test]
    fn test_opening_bid() {
        assert_eq!(
            Context::new(RelativeVulnerability::NONE, &[]).opening_bid(),
            None,
        );
        assert_eq!(
            Context::new(RelativeVulnerability::NONE, &[Call::Pass; 3]).opening_bid(),
            None,
        );
    }

    #[test]
    fn test_seat_to_open() {
        let passes = [Call::Pass; 4];

        for len in 0..=3 {
            let context = Context::new(RelativeVulnerability::NONE, &passes[..len]);
            // SAFETY: `len` is at most 3, so the cast is safe.
            #[allow(clippy::cast_possible_truncation)]
            let seat = len as u8 + 1;
            assert_eq!(context.seat_to_open(), Some(seat));
        }

        let passed_out = Context::new(RelativeVulnerability::NONE, &passes);
        assert_eq!(passed_out.seat_to_open(), None);
    }

    #[test]
    fn bare_context_stays_uncached() {
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        assert!(context.decision_cache.is_none());
        assert!(matches!(context.inferences(), Cow::Owned(_)));
        assert!(matches!(context.inferences(), Cow::Owned(_)));
    }

    #[test]
    fn decision_values_initialize_once() {
        let hand = test_hand();
        let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(hand);

        let uncached_inferences = Inferences::read(&context);
        let cached_inferences = context.inferences();
        assert!(matches!(cached_inferences, Cow::Borrowed(_)));
        assert_eq!(*cached_inferences, uncached_inferences);
        assert!(matches!(context.inferences(), Cow::Borrowed(_)));
        let first = context.trick_estimates(hand);
        let second = context.trick_estimates(hand);
        let uncached_tricks =
            trick_estimates_with_auction(hand, &Inferences::read(&context), context.auction());
        assert_eq!(first.bit_pattern(), second.bit_pattern());
        assert_eq!(first.bit_pattern(), uncached_tricks.bit_pattern());
        let first_interpretation = context.interpretation();
        let second_interpretation = context.interpretation();
        assert_eq!(first_interpretation, Interpretation::read(&context));
        assert_eq!(first_interpretation, second_interpretation);

        assert_eq!(context.decision_cache_init_counts(), Some((1, 1, 1)));
    }

    #[test]
    fn configured_clone_preserves_decision_cache() {
        let context =
            Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
        let cache = Arc::clone(context.decision_cache.as_ref().expect("attached cache"));
        let config = Config::symmetric(&american_card());
        let configured = context.with_config(&config);

        assert_eq!(configured.revision, cache.revision);
        assert!(Arc::ptr_eq(
            configured.decision_cache.as_ref().expect("preserved cache"),
            &cache,
        ));
        assert!(configured.active_decision_cache().is_some());
    }

    #[test]
    fn structural_builders_reject_an_attached_cache() {
        let auction = [];
        let trie = Trie::new();
        let stance = Stance::default();
        let context =
            Context::new(RelativeVulnerability::NONE, &auction).with_decision_cache(test_hand());
        let cache = Arc::clone(context.decision_cache.as_ref().expect("attached cache"));

        let prefixed = context
            .clone()
            .with_prefixes(trie.common_prefixes(&auction));
        assert_eq!(prefixed.revision, cache.revision + 1);
        assert!(prefixed.active_decision_cache().is_none());
        assert!(Arc::ptr_eq(
            prefixed
                .decision_cache
                .as_ref()
                .expect("logically retained"),
            &cache,
        ));

        let opposed = context.with_their_system(&stance);
        assert_eq!(opposed.revision, cache.revision + 1);
        assert!(opposed.active_decision_cache().is_none());
    }

    #[test]
    fn wrong_hand_does_not_fill_scoped_trick_cache() {
        let owner = test_hand();
        let other = "98432.K53.QJ4.92".parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(owner);

        let _ = context.trick_estimates(other);
        let cache = context.decision_cache.as_deref().expect("attached cache");
        assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
        assert!(cache.trick_estimates.get().is_none());
    }

    #[test]
    fn debug_omits_cache_mechanics() {
        let context =
            Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
        let debug = format!("{context:?}");
        assert!(!debug.contains("revision"));
        assert!(!debug.contains("decision_cache"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn decision_cache_rejects_profile_changes() {
        use crate::bidding::evaluator::{eval_auction, set_eval_auction};
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let original = eval_auction();
        let context =
            Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
        let _ = context.inferences();
        set_eval_auction(!original);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = context.inferences();
        }));
        set_eval_auction(original);

        assert!(result.is_err());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn decision_cache_rejects_rkcb_face_profile_changes() {
        use crate::bidding::instinct::{
            RkcbVariant, floor_rkcb_now, rkcb_variant_now, set_floor_rkcb, set_rkcb_variant,
        };
        use std::panic::{AssertUnwindSafe, catch_unwind};

        let original_floor = floor_rkcb_now();
        let floor_context =
            Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
        let _ = floor_context.inferences();
        set_floor_rkcb(!original_floor);
        let floor_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = floor_context.inferences();
        }));
        set_floor_rkcb(original_floor);
        assert!(floor_result.is_err());

        let original_variant = rkcb_variant_now();
        let other_variant = if original_variant == RkcbVariant::Plain {
            RkcbVariant::Kickback
        } else {
            RkcbVariant::Plain
        };
        let variant_context =
            Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
        let _ = variant_context.inferences();
        set_rkcb_variant(other_variant);
        let variant_result = catch_unwind(AssertUnwindSafe(|| {
            let _ = variant_context.inferences();
        }));
        set_rkcb_variant(original_variant);
        assert!(variant_result.is_err());
    }

    #[cfg(debug_assertions)]
    #[test]
    fn decision_cache_rejects_cross_thread_use() {
        let context =
            Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());

        std::thread::scope(|scope| {
            let result = scope
                .spawn(move || {
                    let _ = context.inferences();
                })
                .join();
            assert!(result.is_err());
        });
    }

    #[test]
    fn threads_capture_their_own_profiles() {
        let handles = [false, true].map(|eval_auction| {
            std::thread::spawn(move || {
                super::super::evaluator::set_eval_auction(eval_auction);
                let context =
                    Context::new(RelativeVulnerability::NONE, &[]).with_decision_cache(test_hand());
                let uncached = Inferences::read(&context);
                assert_eq!(*context.inferences(), uncached);
                assert_eq!(context.decision_cache_init_counts(), Some((1, 0, 0)));
                context
                    .decision_cache
                    .as_deref()
                    .expect("decision cache attached")
                    .profile
                    .eval_auction
            })
        });

        let profiles = handles.map(|handle| handle.join().expect("profile thread"));
        assert_eq!(profiles, [false, true]);
    }
}
