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
//! only for its phase.  A [`Pair`] assembles the three books with a [`Family`]
//! identity; binding it against the opponents' family with [`Pair::against`]
//! yields a [`Stance`], the system that actually classifies.
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
use super::inference::Inferences;
use super::trie::{Provenance, Trie};
use contract_bridge::Hand;
use contract_bridge::auction::{Call, RelativeVulnerability};
use core::ops::{Deref, DerefMut};

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

/// An opponent-visible system identity (the "convention card")
///
/// Defensive agreements target what the opponents' calls *mean*, so a [`Pair`]
/// declares the family it plays and selects its competitive and defensive
/// books against the opponents' family — once, at table assembly
/// ([`Pair::against`]).  A family is one convention card: a system that varies
/// by seat or vulnerability is still one family, because the variation is
/// visible to both sides (the seat through the auction keys, the vulnerability
/// through the [`Context`]).
///
/// This is the *system-level* disclosure that picks a base defense; an
/// individual artificial call carries a per-call [`Alert`][super::Alert]
/// instead.  The newtype is open — downstream systems mint their own families
/// as constants, such as `const MOSCITO: Family = Family("moscito");`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Family(pub &'static str);

impl Family {
    /// Natural systems: mostly natural openings with a strong notrump,
    /// such as Standard American, 2/1, and Acol
    pub const NATURAL: Self = Self("natural");
    /// Strong club systems, such as Precision
    pub const STRONG_CLUB: Self = Self("strong-club");
    /// Natural systems with a weak notrump
    pub const WEAK_NOTRUMP: Self = Self("weak-notrump");
}

impl Default for Family {
    fn default() -> Self {
        Self::NATURAL
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

/// One pair's authored system: its identity and its three books
///
/// A pair writes a [`Constructive`] book (strictly uncontested), a
/// [`Competitive`] book (we open, they intervene), and a [`Defensive`] book
/// (they open), and may override the latter two against specific opposing
/// tags.  A pair is *authoring material*, not yet a [`System`]: bind it
/// against the opponents' [`Family`] with [`against`][Self::against] — once,
/// at table assembly — to get a [`Stance`] that classifies.
///
/// The books occupy disjoint keys by construction: a constructive key has all
/// opposing calls as passes, while a competitive key contains an opposing
/// non-pass call.
#[derive(Clone, Debug, Default)]
pub struct Pair {
    /// The family this pair plays, which the opponents defend against
    pub family: Family,
    /// The book for the strictly uncontested auctions
    pub constructive: Constructive,
    /// The default book for when we open and they intervene
    pub competitive: Competitive,
    /// The default book for when they open
    pub defensive: Defensive,
    defensive_vs: Vec<(Family, Defensive)>,
}

impl Pair {
    /// Assemble a pair from its tag and its three default books
    #[must_use]
    pub const fn new(
        family: Family,
        constructive: Constructive,
        competitive: Competitive,
        defensive: Defensive,
    ) -> Self {
        Self {
            family,
            constructive,
            competitive,
            defensive,
            defensive_vs: Vec::new(),
        }
    }

    /// Override the defensive book against one opposing tag
    ///
    /// The first matching override wins; opponents with no override get the
    /// default book.
    #[must_use]
    pub fn defensive_vs(mut self, them: Family, book: Defensive) -> Self {
        self.defensive_vs.push((them, book));
        self
    }

    /// Bind this pair against an opposing tag
    ///
    /// Selects the competitive and defensive books for `them` and merges a
    /// clone of the constructive trie into the bound competitive trie
    /// ([`Trie::merge`], classifiers stay shared), so that competitive rebases
    /// — the "system on" idiom — resolve into the uncontested core.  Bind once
    /// per table, not per call.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the competitive and constructive books
    /// classify the same exact auction; by the key disjointness above, such a
    /// collision is an authoring bug.
    #[must_use]
    pub fn against(&self, them: Family) -> Stance {
        let competitive = &self.competitive;
        let defensive = self
            .defensive_vs
            .iter()
            .find(|entry| entry.0 == them)
            .map_or(&self.defensive, |entry| &entry.1);

        let mut bound = competitive.0.clone();
        let collisions = bound.merge(self.constructive.0.clone());
        debug_assert!(
            collisions.is_empty(),
            "competitive and constructive books collide at {collisions:?}"
        );

        Stance {
            constructive: self.constructive.0.clone(),
            competitive: bound,
            defensive: defensive.0.clone(),
        }
    }
}

/// A pair's system bound against one opposing tag
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
}

impl Stance {
    /// The prefixed [`Context`] this stance classifies an auction under
    ///
    /// The same trie-routed, prefix-bearing context the [`System`] impl builds
    /// (cf. [`classify_with_provenance`][Self::classify_with_provenance]).  It
    /// hands the otherwise-keyless reading paths the trie access the projection
    /// pass needs: [`SearchBook`][super::search_floor::SearchBook] prefixes its
    /// search context with it (M6.2c) so [`Inferences::read`][super::inference::Inferences::read]
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
    use contract_bridge::{Bid, Strain};

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
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

        let stance = american_instinct().against(super::Family::NATURAL);

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
