//! What one bidding decision may consult, besides the hand
//!
//! [`Context`] is three things in one struct, and telling them apart is most
//! of understanding it:
//!
//! 1. **Mechanical facts** — vulnerability, the raw table auction, and the
//!    eleven facts derived from it (who bid which strains, the contract to
//!    beat, passed-hand status, the opening index).  These follow from the
//!    laws of the game alone.  System interpretation — forcing status, what a
//!    `2♣` means — deliberately does not live here; it belongs to classifiers,
//!    which know their system.
//! 2. **Attachments** — borrows of things the caller already built, each
//!    `Option` because a bare context has none of them: the serving
//!    [`Partnership`] and the one it models for the opponents, the convention
//!    cards the nets read, the authored projection, the trie prefixes, and the
//!    [`DecisionProfile`] this decision serves under.  These are *not* ambient
//!    state — nothing here is discovered, only handed over.
//! 3. **A per-decision memo** — `DecisionCache`, plus the `revision` counter
//!    that invalidates it.  One classification asks for the same readings,
//!    features and gate results many times; the memo makes the repeats free
//!    and is keyed by hand, thread and profile so it can never answer a
//!    question it was not built for.
//!
//! Only the first stratum was here originally.  Strata 2 and 3 arrived on
//! 2026-08-05 in two large unnarrated commits (`42a35cc`, `6a109be`), whose
//! design record is [docs/bidding-performance-handoff.md]; read that before
//! changing anything about the cache or the compiled rule path.
//!
//! [docs/bidding-performance-handoff.md]: ../../../docs/bidding-performance-handoff.md

use super::agreements::TheirDisclosures;
use super::book::Partnership;
use super::constraint::FifthsCompanion;
use super::evaluator::{TrickEstimates, trick_estimates_with_auction_on};
use super::features::{CompactConfig, Config};
use super::inference::{AuthoredProjection, Inferences, ReadingProfile};
use super::instinct::{InstinctProfile, Interpretation};
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
/// once per [`classify`][super::Bidder::classify] call, and systems pass the
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
    // --- Mechanical facts: derived from the auction, true under the laws ---
    /// Vulnerability from the perspective of the side to act
    vul: RelativeVulnerability,
    /// The raw table auction — all four players' calls, oldest first
    auction: &'a [Call],
    /// Bitmask over [`Strain`] of the strains our side has bid
    our_strains: u8,
    /// Bitmask over [`Strain`] of the strains they have bid
    their_strains: u8,
    /// Partner's last *bid* (not double or pass), if any
    partner_last_bid: Option<Bid>,
    /// The last bid by anyone — the contract to beat
    last_bid: Option<Bid>,
    /// Whether the last bid stands doubled or redoubled
    penalty: Penalty,
    /// Whether they have yet to make a non-pass call
    undisturbed: bool,
    /// Whether the player to act passed earlier in this auction
    passed_hand: bool,
    /// Whether partner passed earlier in this auction
    partner_passed_hand: bool,
    /// Index in `auction` of the first non-pass call, if the bidding has opened
    opening_index: Option<usize>,

    // --- Attachments: borrows of what the caller already built ---
    /// The queried trie's common prefixes, for the exact-node projection path
    prefixes: Option<CommonPrefixes<'a, 'a>>,
    /// The partnership serving this decision, attached by
    /// [`Context::with_system`]; `None` for a bare diagnostic context
    own_system: Option<&'a Partnership>,
    /// The system we model the opponents as playing — our own unless the
    /// partnership was built against a declared opponent
    their_system: Option<&'a Partnership>,
    /// Both sides' convention cards, as the `features_v5` net block reads them
    config: Option<&'a Config>,
    /// The compact card encoding, the smaller sibling of `config`
    compact: Option<&'a CompactConfig>,
    /// Precomputed authored projections, so a reading walk need not re-resolve
    /// each prior call's authoring classifier
    authored_projection: Option<&'a AuthoredProjection>,
    /// The knob state this decision serves under
    ///
    /// One value, always present: [`Context::new`] starts it at the shipped
    /// [`DecisionProfile::default`], [`Context::with_system`] replaces it with
    /// the attached partnership's pin, and [`Context::with_profile`] sets it
    /// directly — which is how a context built off the reader's auction
    /// (`Context::at_each_turn`), carrying no partnership, still reads under the
    /// reader's settings.  There is no ambient fallback: since 0.11 no bidding
    /// knob lives on a thread.
    profile: DecisionProfile,

    // --- The per-decision memo ---
    /// Bumped by every builder that changes what a cached answer would be, so
    /// a cache carried across such a builder is recognised as stale rather
    /// than trusted.  Attaching a system bumps it; setting the profile does
    /// not, because the cache compares profiles itself.
    revision: u64,
    /// Memo of this decision's readings, features and pure gate results.
    /// `None` for a bare context, which recomputes and is the slow path by
    /// design — `Arc` because a derived context shares its parent's memo.
    decision_cache: Option<Arc<DecisionCache>>,
}

/// The knob inputs whose values must stay fixed during one decision
///
/// Taken from the [`Agreements`][super::agreements::Agreements] when a
/// [`Partnership`] is built ([`System::bind`][super::book::System::bind]) and
/// pinned there: every classify-time knob read serves off the partnership's copy,
/// so a built partnership is a pure value that any thread can classify through.  A
/// A bare context with no attached system uses [`DecisionProfile::default`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DecisionProfile {
    /// The settings that can change a full-auction reading
    ///
    /// One field per setting the reading walk consults while turning the auction
    /// into [`Inferences`]; see [`ReadingProfile`] for the membership rule.
    pub reading: ReadingProfile,
    /// What the opponents' disclosed methods say — facts about them, not
    /// choices of ours; see [`TheirDisclosures`]
    ///
    /// Consulted by the book while building (`1NT (2♣)` routing) and by the
    /// reading walk while classifying, so it lives here per the dual-read
    /// house rule and is part of every decision's cache identity.
    pub their: TheirDisclosures,
    /// The settings the deterministic floor consults *during* classification
    ///
    /// Build-time settings are deliberately absent: each is read only inside
    /// [`instinct`][super::instinct()]'s table builder, so it is already baked
    /// into the rules that come back and needs no pin.  See
    /// [`InstinctProfile`].
    pub instinct: InstinctProfile,
    /// Serve the v3 calls-tail evaluator (**default on**, shipped 2026-07-27)
    ///
    /// On, [`trick_estimates_with_auction`][super::evaluator::trick_estimates_with_auction]
    /// feeds [`features_eval_v3`][super::features::features_eval_v3] — the
    /// hull vector plus the last four call identities — to the v3 artifact,
    /// which the 2026-07-27 NLL ablation priced at 0.038 over the hull-only
    /// vector (bare calls; docs/ai-bidder/evaluator-net.md §auction-input
    /// ablation).  The A/B shipped it default-on with a `win | win` verdict:
    /// plain DD +0.0180 ± 0.0042 (none) / +0.0284 ± 0.0056 (both), PD +0.0222
    /// / +0.0360, on 204,800 boards/arm/vul at `SEED_BASE` 1785138816 — fired
    /// 1.3–1.6%, +1.3 to +2.3 IMPs per fired board at the accountant game/slam
    /// gates.
    ///
    /// The v3 twin was trained on the envelope-union reading regime only, so
    /// the knob is honoured only there; anywhere else the v2 path serves as
    /// before.
    pub eval_auction: bool,
    /// Serve the v4 shape-reading evaluator (**default off**, pending its A/B)
    ///
    /// On, the evaluator is fed
    /// [`features_eval_v4`][super::features::features_eval_v4]: v3's vector
    /// with each hidden seat's four length `{min, max}` pairs replaced by its
    /// **shape distribution** — `E[len]` and `sd[len]` per suit over the
    /// 560-shape lattice, plus one column for how much the reading pins the
    /// seat down.  Three columns wider than v3, and worth nothing in NLL: the
    /// round-two ablation scored the encoding at +0.00004 against a matched
    /// control on 8.15M rows, inside a 0.0006 seed spread.
    ///
    /// The prize is **invariance**, not accuracy.  A hull is not a
    /// well-defined function of a reading — `♥5..13` and `♥5..8` are the same
    /// claim yet differ by a third of the column's range — so
    /// [`sum_closure`][field@crate::bidding::ReadingProfile::sum_closure],
    /// which provably rejects no hand, still displaces the
    /// endpoint columns at 81% of nodes by up to 4.19σ and has to buy a
    /// retrain before it can be judged on merit.  The shape columns move at
    /// 0.11% of nodes by up to 0.07σ, and that 0.11% is where the reading
    /// genuinely changed.  Under this knob the reading-fidelity chops become
    /// measurable on their own terms.
    ///
    /// Supersedes [`eval_auction`](Self::eval_auction) when both are on — v4
    /// carries the calls tail verbatim.  Like the v3 twin it was trained on
    /// the envelope-union reading regime only, and its shape block reads the
    /// *union* of announced boxes, so it is honoured only there.
    pub eval_shape: bool,
    /// Blank every inference block the nets see — the reading program's
    /// *negative control*
    ///
    /// Every generator of readings (authored `project`, the agreement overlay
    /// behind the announced-reading knob, and any future sampled projection)
    /// competes for one prize: the IMPs that flow from the nets reasoning
    /// about what the other three seats have shown.  Tightening a reading
    /// measures the *derivative* of that prize and lands in the noise.  This
    /// knob measures its **level** — on, all four seats read as
    /// `Envelope::unknown` and the nets reason from the auction alone.
    ///
    /// The A/B against the shipped default is therefore a ceiling on the whole
    /// program: no reading, however well generated, can be worth more than
    /// what deleting every reading costs.  Nothing else consumes it — the
    /// sampler's containment test, `admits`, and the opening-lead sampling all
    /// read the [`Inferences`] directly and are untouched.
    ///
    /// Diagnostic only, **off by default**; never ship it on.
    pub blind_inference: bool,
    /// Serve the nets the **pre-ceilings** reading (**default on**)
    ///
    /// The hedge for
    /// [`strength_ceilings`][field@crate::bidding::ReadingProfile::strength_ceilings]:
    /// every shipped net was trained on floor-only strength boxes, so
    /// tightening the reading moves their inputs off the training
    /// distribution even where the tighter reading is the true one.  On, the
    /// nets — [`features`][super::features] and the evaluator's trick
    /// estimates — are fed a second [`Inferences`] read with the ceilings
    /// switched back off, while the sampler, the authored gates and the
    /// instinct floor keep the true one.  That isolates "the reading was
    /// wrong" from "the nets are stale".  It shipped **on** with the ceilings
    /// on 2026-08-16 because that isolation is what the measurement liked:
    /// held nets, the ceilings move 3 boards in 204,800 and win all four
    /// scoring cells, against a raw arm that leans plain-DD-negative on three
    /// independent seeds.  **This is scaffolding**: Phase 5 of
    /// docs/authored-reading-handoff.md retires it by retraining the nets on
    /// honest readings, and it should not outlive that.
    ///
    /// ponytail: the second reading is a second walk on the *uncompiled*
    /// projection path — a compiled rule plan is served only to the profile
    /// it was compiled under, and this one deliberately differs.  Measured at
    /// **~1%** end to end (4,000 deals: 22.68s against 22.42s), because it is
    /// memoised per decision behind a `OnceLock` and double-dummy solving
    /// dominates regardless.  A second compiled registry would buy that 1%
    /// back and is not worth it while Phase 5 is the plan.
    pub legacy_view: bool,
    /// The floor's two-over-one game force (**on by default**)
    ///
    /// The authored book has always held this invariant by *omission* — no
    /// table in the 2/1 game-force book carries a `Pass` rule, so pass scores
    /// −∞ and a 2/1 auction cannot die below game.  The floor never learned
    /// it, which did not matter while the game backstop covered every
    /// uncovered continuation.  Deleting that node
    /// ([`game_backstop`][super::agreements::GameForceKnobs::game_backstop],
    /// now the default) exposed the gap: against BBA, 24% of the affected
    /// boards had our side settling below game in an established 2/1 — opener
    /// passing responder's 3♣ out in a partscore.
    ///
    /// On, an uncontested 2/1 sets `Interpretation::forced_to_game`, so the
    /// floor takes the cheapest game milestone instead of passing.  Measured
    /// on top of the deletion: **+0.0067/+0.0102 plain, +0.0060/+0.0094 PD**
    /// IMPs/board NV/vul vs BBA (409,600×2, all CI>0), firing on 606/622
    /// boards — exactly the set that abandoned the force — at +4.5/+6.7 IMPs
    /// each.  It costs routing those nodes through the deterministic ladder
    /// rather than the learned net, since the [shell][super::neural_floor]
    /// delegates wholesale on a forced auction; that price is inside the
    /// measurement.
    ///
    /// Uncontested only, matching the `Undisturbed` guard the deleted node
    /// carried: over interference a two-level new suit is a free bid, not a
    /// game force.
    pub two_over_one_force: bool,
    /// Evaluate Fifths, rather than raw HCP, in the `fifths` gauge
    ///
    /// **Default off**: the Fifths NT-gauge measured a clean net loss vs raw
    /// HCP in the A6 audit (self-play plain −0.012/−0.018 NV/vul, PD alike,
    /// CIs excluding 0), and it dragged the `points` upgrade — points-only
    /// beat points+fifths on both scorers.  See docs/bidding-options.md A6.
    ///
    /// The `points` half of the old "fuzzy strength" umbrella is the separate
    /// point-scale setting.  That umbrella and its bool `points` wrapper were
    /// deleted 2026-08-03: one wrote *two* sibling cells (so flipping it
    /// silently moved a knob the caller never named), and the other was a bool
    /// over a three-valued scale, unable to name `PointScale::RuleOfNFloored`
    /// and destroying it on write.
    pub fuzzy_fifths: bool,
    /// The honor count averaged with Fifths in the `fifths` gauge
    ///
    /// Fifths is tuned for 3NT — it rewards aces and tens and discounts kings
    /// and queens — so on its own it misjudges a hand headed for a suit
    /// contract.  A notrump-defining range never gauges Fifths alone; it
    /// averages Fifths with one of these honor counts, so a tens-rich hand
    /// can't reach the band on Fifths and a quack-heavy hand isn't shut out of
    /// it.
    ///
    /// **Default BUM-RAP** — it edged HCP across every vulnerability in the
    /// `fifths-companion` A/B match.
    pub fifths_companion: FifthsCompanion,
    /// Price responder's Stayman-rebid invite/force seams with the evaluator
    /// net instead of the point tests (**off by default — measured a loss**,
    /// kept for re-measurement)
    ///
    /// The `probe-nt-invite-eval` screen (30 000 deals per class, seed
    /// 1784718391) found the net's game make-probability is the first
    /// evaluator to out-rank raw HCP at the `1NT` invite/force boundary — but
    /// only on the Stayman class (+0.030 ±0.017 IMPs/board vul none, +0.044
    /// ±0.025 vul both, rising to +0.048/+0.069 opposite exactly-15 openers);
    /// the balanced no-major seam stays HCP (net ≈ 0, third evaluator family
    /// to fail there).  On, exactly the Stayman-rebid seams convert: with a
    /// fit the `4M`/`3M`/`3OM` split, without one the `3NT`/`2NT` revert —
    /// each force arm becomes "the net clears the game's IMP break-even at the
    /// live vulnerability", its invite twin the declined half.  The `2♣`
    /// entry, Smolen, garbage/crawling and the quantitative `4NT` are
    /// untouched.
    ///
    /// **The live A/B refuted it** (`ab-stayman-net-force --slice`, 200k
    /// sliced boards per vul, seed 1784719896): vul none −0.022 plain DD /
    /// +0.003 PD, vul both −0.021 plain / −0.027 PD.  The forensic split
    /// explains the screen-vs-live reversal: at the *fit* seam the incumbent
    /// is not raw HCP but `fit_value` — already an upgrade evaluator, and the
    /// net loses to it on both scorers in both directions; at the *no-fit NT*
    /// seam the net's flips are plain-DD-positive (matching the screen, which
    /// scored plain DD) but PD-negative — the DD-trained net promotes `3NT`s
    /// that die against perfect defense, the decision table's "doubling
    /// artifact, don't ship" row.  A frequency-matched NT-seam-only gate
    /// re-scored under `single_dummy_leads` is the remaining open refinement.
    pub stayman_net_force: bool,
    /// The GF-majors transfer structure's master flag
    ///
    /// Also read at book construction to *place* its rules; this copy serves
    /// the classify-time guards that cap or reroute a hand once the structure
    /// is on.
    pub transfer_gf_majors: bool,
    /// Whether the structure is mirrored onto the heart transfer — the **raw**
    /// setting, meaningless on its own
    ///
    /// The mirror is a no-op unless `transfer_gf_majors` is on too, so the book
    /// gates on [`transfer_gf_heart_mirror`](Self::transfer_gf_heart_mirror),
    /// never on this field.
    pub transfer_gf_hearts: bool,
}

impl DecisionProfile {
    /// Whether the heart-transfer mirror is authored — both gates open
    #[must_use]
    pub const fn transfer_gf_heart_mirror(&self) -> bool {
        self.transfer_gf_majors && self.transfer_gf_hearts
    }
}

impl Default for DecisionProfile {
    /// The shipped classify-time agreements — see [`ReadingProfile::default`]
    fn default() -> Self {
        Self {
            reading: ReadingProfile::default(),
            their: TheirDisclosures::default(),
            instinct: InstinctProfile::default(),
            eval_auction: true,
            eval_shape: false,
            blind_inference: false,
            legacy_view: true,
            two_over_one_force: true,
            fuzzy_fifths: false,
            fifths_companion: FifthsCompanion::Bumrap,
            stayman_net_force: false,
            transfer_gf_majors: true,
            transfer_gf_hearts: true,
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
    /// The nets' reading under `legacy_view`; never initialised without it
    legacy_inferences: OnceLock<Inferences>,
    trick_estimates: OnceLock<TrickEstimates>,
    interpretation: OnceLock<Interpretation>,
    rule_face_slots: usize,
    rule_faces: [AtomicU8; 16],
    rule_face_overflow: Box<[AtomicU8]>,
    #[cfg(test)]
    inference_inits: AtomicUsize,
    #[cfg(test)]
    legacy_inference_inits: AtomicUsize,
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
            own_system: None,
            their_system: None,
            config: None,
            compact: None,
            authored_projection: None,
            profile: DecisionProfile::default(),
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
            legacy_inferences: OnceLock::new(),
            trick_estimates: OnceLock::new(),
            interpretation: OnceLock::new(),
            rule_face_slots,
            rule_faces: [const { AtomicU8::new(0) }; 16],
            rule_face_overflow: (16..rule_face_slots).map(|_| AtomicU8::new(0)).collect(),
            #[cfg(test)]
            inference_inits: AtomicUsize::new(0),
            #[cfg(test)]
            legacy_inference_inits: AtomicUsize::new(0),
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
            .field("own_system", &self.own_system)
            .field("their_system", &self.their_system)
            .field("config", &self.config)
            .field("compact", &self.compact)
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
            own_system: None,
            their_system: None,
            config: None,
            compact: None,
            authored_projection: None,
            profile: DecisionProfile::default(),
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

    /// Serve this decision under an explicit knob state
    ///
    /// The reading walks each turn of the auction through a context of its own
    /// (`at_each_turn`), and those carry no partnership; without this they would
    /// read under shipped defaults rather than the profile the reader was built
    /// from.  Diagnostic contexts use it the same way, to classify a rule table
    /// under settings no partnership is holding.
    ///
    /// Attaching a system sets the profile too, from that system's pin, so
    /// attach it first if you mean to override.
    #[must_use]
    pub const fn with_profile(mut self, profile: DecisionProfile) -> Self {
        self.profile = profile;
        self
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

    /// Attach the reader's books, and with them a model of the *opponents'*
    ///
    /// Two channels from one argument, because they must not disagree: the
    /// reader's own calls resolve in `ours`, and the opponents' in
    /// [`Partnership::opponents`] — which is `ours` again unless the partnership was
    /// built against a declared opponent ([`Partnership::with_opponents`]). Alerts are
    /// disclosure to the whole table, so the reading layer may decode the
    /// opponents' alerted calls off their authoring rules; modeling them as
    /// playing our own books is exact in self-play and an approximation
    /// against other natural-family engines. Consumed by `project_authored`
    /// behind
    /// [`table_alerts`][field@crate::bidding::ReadingProfile::table_alerts].
    #[must_use]
    pub(crate) fn with_system(mut self, ours: &'a Partnership) -> Self {
        self.own_system = Some(ours);
        self.their_system = Some(ours.opponents());
        // The partnership's pin *is* this decision's knob state — taking it here is
        // what leaves one profile field instead of a precedence chain.
        self.profile = ours.profile();
        self.revision = self.revision.wrapping_add(1);
        self
    }

    /// The reader's own books, if attached ([`Self::with_system`])
    #[must_use]
    pub(crate) const fn own_system(&self) -> Option<&'a Partnership> {
        self.own_system
    }

    /// The opponents' modeled system, if attached ([`Self::with_system`])
    #[must_use]
    pub(crate) const fn their_system(&self) -> Option<&'a Partnership> {
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

    /// Attach both partnerships' compact agreements, for
    /// [`features_v5`][super::features::features_v5]
    ///
    /// The v5 sibling of [`Self::with_config`]: the same seam with a narrower
    /// payload — the axes pons owns instead of the whole `.bbsa` card.
    /// Carried by reference and encoded once per configuration cell, so the
    /// per-decision path neither allocates nor reads ambient knob state.
    #[must_use]
    pub const fn with_compact(mut self, compact: &'a CompactConfig) -> Self {
        self.compact = Some(compact);
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
    /// The partnership-wide compiler assigns the slots. Standalone public rule
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
        let profile = self.profile;
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

    /// The reading knobs governing this decision
    ///
    /// One field read: whatever [`with_system`][Self::with_system] or
    /// [`with_profile`][Self::with_profile] put there, or the shipped defaults
    /// for a bare diagnostic context.  Constraint projection calls this per
    /// node, so it stays the cheapest thing it can be.
    ///
    /// Until 0.11 this was a four-arm cascade — decision cache, attached
    /// partnership, explicit pin, thread-local — because knobs could still be armed
    /// ambiently and a decision had to freeze them.  With one home per knob
    /// there is nothing left to disagree.
    pub(crate) const fn reading_profile(&self) -> ReadingProfile {
        self.profile.reading
    }

    /// Every knob governing this decision, not just the reading half
    ///
    /// What the [instinct floor][super::instinct()] reads from inside a
    /// predicate: the closures run per decision and must use the partnership's pin.
    pub(crate) const fn decision_profile(&self) -> DecisionProfile {
        self.profile
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

    /// The reading the **nets** are fed — [`inferences`][Self::inferences]
    /// unless [`legacy_view`][DecisionProfile::legacy_view] asks for the
    /// pre-ceilings one
    ///
    /// Only the net-facing callers use this: the sampler, the authored gates
    /// and the instinct floor read [`inferences`][Self::inferences] and see
    /// the truth.
    #[must_use]
    pub(crate) fn net_inferences(&self) -> Cow<'_, Inferences> {
        if !self.profile.legacy_view {
            return self.inferences();
        }
        // A context is not reusable across profiles: drop the precomputed
        // overlay (it carries the ceilings) and the memo (its slots were
        // filled under the other profile), then read the auction again.
        let legacy = || {
            let mut legacy = self.clone();
            legacy.profile.reading.strength_ceilings = false;
            legacy.authored_projection = None;
            legacy.decision_cache = None;
            Inferences::read(&legacy)
        };
        let Some(cache) = self.active_decision_cache() else {
            return Cow::Owned(legacy());
        };
        Cow::Borrowed(cache.legacy_inferences.get_or_init(|| {
            #[cfg(test)]
            cache.legacy_inference_inits.fetch_add(1, Ordering::Relaxed);
            legacy()
        }))
    }

    /// Evaluate tricks once for the hand that owns this decision scope
    #[must_use]
    pub(crate) fn trick_estimates(&self, hand: Hand) -> TrickEstimates {
        let profile = self.decision_profile();
        let Some(cache) = self.active_decision_cache() else {
            let inferences = self.net_inferences();
            return trick_estimates_with_auction_on(&profile, hand, &inferences, self.auction());
        };
        if cache.hand != hand {
            let inferences = self.net_inferences();
            return trick_estimates_with_auction_on(&profile, hand, &inferences, self.auction());
        }
        *cache.trick_estimates.get_or_init(|| {
            #[cfg(test)]
            cache.trick_estimate_inits.fetch_add(1, Ordering::Relaxed);
            let inferences = self.net_inferences();
            trick_estimates_with_auction_on(&profile, hand, &inferences, self.auction())
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

    /// Test-only initialization count for the `legacy_view` reading
    #[cfg(test)]
    pub(crate) fn legacy_inference_inits(&self) -> Option<usize> {
        Some(
            self.active_decision_cache()?
                .legacy_inference_inits
                .load(Ordering::Relaxed),
        )
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

    /// The attached compact agreements, if any ([`Self::with_compact`])
    #[must_use]
    pub const fn compact(&self) -> Option<&'a CompactConfig> {
        self.compact
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
mod tests;
