//! What the calls have shown, accumulated across an auction
//!
//! [`Context`] gives the *laws-only* facts of an auction; the authored books
//! and the [instinct floor][super::instinct()] read *system intent* off the
//! calls on demand (see [`Interpretation`][super::instinct()]).  This module is
//! the richest such reading: for every player, the range of cards each suit
//! may hold and the range of points shown, derived **purely from the calls**
//! under standard 2/1 meanings.
//!
//! Two consumers want this summary that the per-bid [`Constraint`][crate::bidding::constraint::Constraint]s cannot
//! give them — a `Constraint` is eval-only, so the length a `len(..)` rule
//! asserts can never be read back out:
//!
//! - the [instinct floor][super::instinct()], so a forced auction can pick a
//!   known major-suit fit over notrump instead of re-deriving partner's shape
//!   from scratch;
//! - constrained sampling (future), which needs per-player {suit → length} and
//!   points to deal hands consistent with an auction.
//!
//! # Soundness over tightness
//!
//! Every player starts at [`Envelope::unknown`] and each call only ever
//! *narrows* a range via [`Range::intersect`].  A rule that is unsure leaves
//! the range wide; a missing rule costs tightness, never soundness.  The
//! guarantee a consumer may rely on is one-directional: a hand that actually
//! made these calls always falls **within** every shown range.  The deriver
//! therefore reads only the meanings that hold robustly — natural suit
//! lengths, raises, rebids, overcalls — and stays silent on the artificial
//! structures (Stayman, transfers, the strong-2♣ responses) that a keyless
//! reading would misread as natural.
//!
//! # One system
//!
//! The meanings encoded here are those of [`american`][super::american()]
//! (five-card majors, strong 15–17 notrump, strong artificial 2♣); like the
//! instinct floor, this reading is tied to that system.

use super::context::Context;
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

/// The largest a suit length range may span
const LENGTH_CAP: u8 = 13;
/// The largest a point range may span (all forty HCP, then some)
const POINTS_CAP: u8 = 37;
/// The largest a single suit's HCP range may span (AKQJ)
const SUIT_HCP_CAP: u8 = 10;

std::thread_local! {
    /// Whether [`Inferences::read`] quantifies a natural notrump raise of our own
    /// 1NT opening (2NT invitational, 3NT game).  On by default; turn it off to
    /// reproduce the pre-fix behaviour where opener was blind to responder's
    /// strength and so could not accept an invitation.  The
    /// [`nt-invite-abc`](../../examples) example A/Bs the two.
    static NT_INVITE_INFERENCE: Cell<bool> = const { Cell::new(true) };
}

/// Toggle reading natural notrump raises of our 1NT opening (default on).
///
/// The fix this gates is what lets opener — and the sampler behind the search
/// floor — know responder is invitational (≈8–9) or game-going (10+), so the
/// keyless floor can judge whether game is good without a hand-authored node.
pub fn set_nt_invite_inference(on: bool) {
    NT_INVITE_INFERENCE.with(|cell| cell.set(on));
}

fn nt_invite_inference() -> bool {
    NT_INVITE_INFERENCE.with(Cell::get)
}

std::thread_local! {
    /// Whether a one-level Rubens transfer records its meaning (see
    /// [`set_rubens_transfer_reading`]).  On by default; off recovers the
    /// suppress-only reading, where the transfers showed nothing.
    static RUBENS_TRANSFER_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle recording the one-level Rubens transfers' meaning (default on).
///
/// The two-level cue-raise always recorded its limit-plus raise; the one-level
/// transfers were suppress-only, leaving the overcaller — and the sampler
/// behind the search floor — blind to the shown support, length, and strength.
/// On, the transfer into partner's suit records three-plus cards in the
/// overcall suit, a new-suit transfer records five-plus in its target, both
/// ten-plus points.  For A/B measurement (`bba-gen --no-ns-rubens-reading`).
pub fn set_rubens_transfer_reading(on: bool) {
    RUBENS_TRANSFER_READING.with(|cell| cell.set(on));
}

fn rubens_transfer_reading() -> bool {
    RUBENS_TRANSFER_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether [`project_authored`] treats a call as artificial because its
    /// authoring rule carries an [`Alert`][crate::bidding::Alert], on top of the
    /// structural [`artificial`] test.  On by default; turning it off recovers the
    /// pre-alert behaviour where a strength-showing artificial that floors no
    /// foreign suit — the strong 2♣ opening, its 2♦ waiting / 2♥ double negative,
    /// Puppet 3♣ — was misread as a natural suit.  The `ab-alert-reading` example
    /// A/Bs the two.
    static ALERT_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle reading an alerted call as artificial (default on).
///
/// This is the per-call defense switch: with it on, the floor recognises every
/// alerted convention — including the strength-showing artificials the structural
/// detector misses — and reads it as the convention rather than as a natural suit,
/// so a player switches its treatment the moment an opponent's alerted call lands.
pub fn set_alert_reading(on: bool) {
    ALERT_READING.with(|cell| cell.set(on));
}

fn alert_reading() -> bool {
    ALERT_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether the layout sampler accepts a candidate hand by *replaying the
    /// rule* — re-running the policy at each prior decision node and keeping the
    /// hand only if the policy would have made the call the player actually made
    /// — instead of projecting the auction into the hand-written [`Inferences`]
    /// ranges.  **On by default** — the search EV ([`ev_all`][crate::bidding::ev_all])
    /// samples its rollout worlds this way (pons's analog of BEN's soft NN-replay
    /// gate); the `ab-search-floor` example A/Bs it off with `--no-rule-accept`.
    /// See [`sample_layouts_replay`][super::sampler::sample_layouts_replay].
    static RULE_ACCEPT: Cell<bool> = const { Cell::new(true) };

    /// Whether the projection pass decodes calls authored by *guarded fallbacks*
    /// (every contested convention — transfers, Leaping Michaels, the Lebensohl
    /// cue), not just exact-node classifiers.  **On by default** (BBA A/B: plain
    /// +0.0006/board, +1.03/fired; PD +0.0014, +2.38/fired — both CIs exclude 0).
    /// On, [`project_authored`] re-resolves each prior call's authoring classifier
    /// through the trie's node-then-fallback chain so an alerted call survives later
    /// competition without a per-convention hand reader.  Off restores the
    /// exact-node-only projection (the A/B off arm).
    static FALLBACK_PROJECTION: Cell<bool> = const { Cell::new(true) };
}

/// Toggle decoding fallback-authored conventions in the projection (**default on**)
///
/// Off, `project_authored` sees only exact-node classifiers (via
/// [`common_prefixes`][super::Trie::common_prefixes]), so a contested convention
/// authored by a guarded fallback misreads under second-round intervention unless a
/// hand-written reader covers it.  On, it re-resolves each call's *authoring*
/// classifier (node or fallback) and projects its alerted rule — the general decode
/// for non-natural calls that subsumes the single-suit per-convention readers (the
/// OR-disjunction two-suiters and doubles still need their hand readers).  Read at
/// classification time, per-thread; A/B'd on the BBA match.
pub fn set_fallback_projection(on: bool) {
    FALLBACK_PROJECTION.with(|cell| cell.set(on));
}

/// Whether fallback-authored projection is enabled (default on)
#[must_use]
pub fn fallback_projection_enabled() -> bool {
    FALLBACK_PROJECTION.with(Cell::get)
}

std::thread_local! {
    /// Whether a call's reading is stored as a *union of boxes* (a DNF) and the
    /// sampler accepts a hand that lies in **any** box, rather than the single
    /// bounding-box hull (see [`set_dnf_reading`]).  **On by default** since
    /// chop F2b (docs/dnf-migration.md): with the knob-matched evaluator twin
    /// and the statically pinned Jacoby box, the flip measured a win in all
    /// four cells — plain +0.0094/+0.0080, PD +0.0118/+0.0085 NV/vul, CIs
    /// clear (204,800 boards/arm/vul, seed 1784809754).  Off is the legacy
    /// hull path, kept as the kill-switch.
    static DNF_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle union-of-boxes (DNF) readings for the sampler (**default on**, F2b)
///
/// Off, a disjunctive reading (`Or`, `AnyLen`, a call authored by several rules)
/// widens to its bounding box, so the sampler accepts the whole hull — today's
/// behaviour.  On, the reading keeps its separate boxes and the sampler accepts
/// a layout only if it lies in *some* box, pinning two-suiters / Multi / the
/// fit-split instead of the box that spans them.  Two hull regimes, measured
/// (docs/dnf-migration.md, chop F1): on a **bare** `Context::new` (no
/// projection overlay — the `dump-teacher` feature path) the hulls are
/// knob-invariant, byte-identical over a 21K-row dump; on a **prefixed**
/// context (`Stance::infer` — what the bidder, the floor net, and the
/// bilans evaluator actually see) the authored-projection overlay tightens
/// knob-on (⊤→box upgrades, `dnf_upgrade`), so those consumers' inputs move
/// with the knob.  Read at classification and acceptance time, per-thread.
pub fn set_dnf_reading(on: bool) {
    DNF_READING.with(|cell| cell.set(on));
}

/// Whether union-of-boxes readings are enabled (default on)
#[must_use]
pub fn dnf_reading() -> bool {
    DNF_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether the two opponent seats' readings are blanked (see
    /// [`set_blind_opponent_reading`]).  **Off by default.**
    static BLIND_OPPONENT_READING: Cell<bool> = const { Cell::new(false) };
}

/// Blank what the *opponents* have shown (**default off**, measurement only)
///
/// On, [`Inferences`] hands back [`Envelope::unknown`] / [`Dnf::unknown`] for
/// [`Relative::Lho`] and [`Relative::Rho`]; partner and the actor keep their
/// live readings.  This is the `blind` arm of the deviation panel
/// (docs/deviation-panel.md): the paired `seen − blind` score is what our
/// reading of *their* calls is worth against one perturbed opponent, a
/// statistic that stays honest when the deviant system is itself weaker.
///
/// Distinct from [`features::set_blind_inference`][super::features::set_blind_inference],
/// which blanks all four seats and only for the nets — this one cuts at the
/// source, so the sampler, the floor and the evaluator all go blind together.
/// The two are **not** comparable; the historical blind control was the other
/// knob.  Read at assembly time, per-thread.
pub fn set_blind_opponent_reading(on: bool) {
    BLIND_OPPONENT_READING.with(|cell| cell.set(on));
}

/// Whether the opponents' readings are blanked (default off)
#[must_use]
pub fn blind_opponent_reading() -> bool {
    BLIND_OPPONENT_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether box membership also tests the `hcp` and `support_points`
    /// gauges (see [`set_gauge_membership`]).  **Off by default** — the
    /// chop-E knob; every consumer stays on the lengths + `points` membership
    /// until the A/B measures the tighter acceptance.
    static GAUGE_MEMBERSHIP: Cell<bool> = const { Cell::new(false) };
}

/// Give the strength gauges membership teeth (**default off**, chop E of
/// docs/dnf-migration.md)
///
/// Off, [`Envelope::admits`] — and everything routed through it: sampler
/// acceptance, [`Dnf::contains`], the DNF overlay — tests suit lengths and
/// the legacy `points` gauge only, the pre-`Strength` behaviour.  On, a
/// sampled hand must also fall within the box's raw-HCP, support-points, and
/// per-suit HCP bands, so a 15–17 1NT stops admitting 13-counts the `points`
/// scale upgraded.  The measured "bidding-inert" verdict (chop E: 0 fired in
/// 409,600 boards) predates both the suit-indexed `supports` gauging and the
/// `suit_hcp` axis — those bands have real teeth (an Ogust quality ceiling
/// rejects hands no whole-hand gauge can), so a future flip owes a fresh
/// measurement.  Deliberately its **own** knob, never folded into
/// [`set_dnf_reading`]: the mechanisms are independent (gauge bands tighten
/// sampling even on the single-hull reading), the recorded DNF sd-lead WASH
/// stays meaningful, and this is the one chop that can *reject legal hands*
/// if a projection over-claims — the book-wide eval ⟹ membership sweep is
/// its soundness gate, and this knob its kill-switch.  Read at acceptance
/// time, per-thread (set it inside worker closures, like the scale flags).
pub fn set_gauge_membership(on: bool) {
    GAUGE_MEMBERSHIP.with(|cell| cell.set(on));
}

/// Whether gauge membership is enabled (default off)
#[must_use]
pub fn gauge_membership() -> bool {
    GAUGE_MEMBERSHIP.with(Cell::get)
}

std::thread_local! {
    /// Whether [`Dnf::tidy`] narrows suit lengths by `Σ len = 13` (see
    /// [`set_sum_closure`]).  **Off by default** — a hull change, so it owes
    /// an A/B.
    static SUM_CLOSURE: Cell<bool> = const { Cell::new(false) };
    /// Whether [`Dnf::tidy`] closes `hcp` against `points` through the shape
    /// upgrade (see [`set_upgrade_closure`]).  **Off by default**, same reason.
    static UPGRADE_CLOSURE: Cell<bool> = const { Cell::new(false) };
}

/// C1: narrow each box's suit lengths to what `Σ len = 13` implies
/// (**default off**, chop C of docs/dnf-migration.md)
///
/// `Envelope::sum_feasible` only *tests* the sum; nothing narrows with it, so a
/// both-majors reading stores `{♠ 5..13, ♥ 5..13, ♦ 0..13, ♣ 0..13}` when the
/// truth is `{♠ 5..8, ♥ 5..8, ♦ 0..3, ♣ 0..3}` — eight of the thirteen cards
/// are already spoken for.  See `Envelope::narrow_to_sum` for the sweep and its
/// exactness proof.
///
/// Its own knob, never folded into [`set_upgrade_closure`]: the two closures
/// read different axes and the A/B measures them apart before stacking.
/// Requires [`dnf_reading`] (the knob-off hull path stays byte-identical).
/// Read at classification time, per-thread.
#[doc = include_str!("closure-inertness.md")]
pub fn set_sum_closure(on: bool) {
    SUM_CLOSURE.with(|cell| cell.set(on));
}

/// Whether the `Σ len = 13` closure is enabled (default off)
#[must_use]
pub fn sum_closure() -> bool {
    SUM_CLOSURE.with(Cell::get)
}

/// C2: close `hcp` against `points` through the shape upgrade
/// (**default off**, chop C of docs/dnf-migration.md)
///
/// `points` is `hcp` plus a shape-and-honor
/// [`upgrade`][super::constraint::upgrade], and *balanced hands never upgrade*
/// (every balanced shape holds at most 9 cards in its two longest suits).  So a
/// box whose lengths force balanced reads `points == hcp`, where
/// [`points`][super::constraint::points]' projection slacks `hcp` by the
/// scale's **global** worst case — a 2-HCP leak at each end of the most-read
/// strength gate in the book.  See `Envelope::narrow_to_upgrade`.
///
/// Requires [`dnf_reading`].  Read at classification time, per-thread.
#[doc = include_str!("closure-inertness.md")]
pub fn set_upgrade_closure(on: bool) {
    UPGRADE_CLOSURE.with(|cell| cell.set(on));
}

/// Whether the shape-upgrade closure is enabled (default off)
#[must_use]
pub fn upgrade_closure() -> bool {
    UPGRADE_CLOSURE.with(Cell::get)
}

std::thread_local! {
    /// Whether the reading classifies high (four-plus level) new-suit bids as
    /// control bids vs to-play (**on by default**, M6.4).  The deterministic
    /// rule, distilled from Bridge World Standard: such a bid is *natural* iff
    /// the suit could still be the bidder's longest — the bidder has shown no
    /// other suit yet, or is rebidding a suit they themselves showed.
    /// Otherwise it is a control bid agreeing the partnership's most recently
    /// shown suit, and the phantom suit is suppressed rather than floored.
    static CONTROL_BID_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle the control-bid reading of high new-suit bids (**default on**, M6.4)
///
/// Off, a four-plus-level new suit falls back to the pre-M6.4 reading (double
/// jumps skipped, notrump-structure bids blanket-suppressed) — the A/B off arm.
pub fn set_control_bid_reading(on: bool) {
    CONTROL_BID_READING.with(|cell| cell.set(on));
}

/// Whether the control-bid reading is enabled (default on); shared with the
/// instinct floor so the reader and the signoff rules flip together
#[must_use]
pub(super) fn control_bid_reading() -> bool {
    CONTROL_BID_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether the natural walk recognises a bid of a suit only the *opponents*
    /// have naturally shown as a cue rather than a holding (see
    /// [`set_cue_reading`]).  On by default (shipped 2026-07-18: bid-inert,
    /// reading soundness).
    static CUE_READING: Cell<bool> = const { Cell::new(true) };

    /// Whether two over-tight natural length floors are relaxed to sound ones
    /// (see [`set_length_soundness`]).  On by default (shipped 2026-07-18:
    /// plain wash + PD win on both references).
    static LENGTH_SOUNDNESS: Cell<bool> = const { Cell::new(true) };

    /// Whether [`project_authored`] reads passes off their table's own Pass
    /// gate (see [`set_pass_reading`]).  On by default (shipped 2026-07-18:
    /// bid-inert, reading soundness).
    static PASS_READING: Cell<bool> = const { Cell::new(true) };

    /// Whether a pass additionally excludes the sibling gates it declined
    /// (see [`set_pass_exclusion_reading`]).  Off by default.
    static PASS_EXCLUSION_READING: Cell<bool> = const { Cell::new(false) };

    /// Whether [`project_authored`] folds the stance's behaviorally probed
    /// boxes (see [`set_probed_reading`]).  Off by default.
    static PROBED_READING: Cell<bool> = const { Cell::new(false) };

    /// Whether the probed overlay serves only own-side calls, and only onto
    /// axes the symbolic reading left fully open (see
    /// [`set_probed_vacuous_reading`]).  Off by default.
    static PROBED_VACUOUS_READING: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the cue reading of the natural walk (default on, shipped 2026-07-18)
///
/// On, a bid of a suit only the *opponents* have naturally shown is a cue,
/// never a holding: the phantom length the walk floored is suppressed, and the
/// two robust standard meanings are recorded — a defender's direct cue of
/// their one- or two-level minor opening is Michaels / Leaping Michaels (both
/// majors, five-five), and a non-jump cue opposite exactly one natural suit on
/// partner's side is the limit-plus cue-raise (three-plus support, ten-plus
/// points).  The BEN Info-net probe (docs/ben-gap-campaign.md) caught the
/// phantoms as truth violations on self-play: `(1♣) 2♣` Michaels read as five
/// clubs on a void, `1♥ (2♦) 3♦` cue-raise read as four diamonds on two.
pub fn set_cue_reading(on: bool) {
    CUE_READING.with(|cell| cell.set(on));
}

fn cue_reading() -> bool {
    CUE_READING.with(Cell::get)
}

/// Toggle sound natural length floors (default on, shipped 2026-07-18)
///
/// On: opener's immediate two-level rebid of the opened minor reads five-plus,
/// not six-plus — the floor routinely rebids a good five — and a player who
/// has doubled earlier no longer has a later jump in a new suit read as a weak
/// six-card jump (a doubler's jump is strength, made on as few as three cards,
/// so the walk claims nothing).  Both over-claims were caught by the BEN
/// Info-net probe as truth violations on self-play.
///
/// Shipped default-on by the dual-reference A/B (the one reading knob with a
/// live bidding delta — 23/6400 divergent boards): plain DD a wash on both
/// references, perfect-defense positive everywhere — vs BBA +0.0022/+0.0023
/// IMPs/board (CIs clear of zero, 204.8k boards/cell), vs BEN Tier F
/// +0.0020/+0.0015 (directionally consistent, 51.2k/cell); +0.4 to +1.1
/// IMPs per fired board.  `--no-ns-length-soundness` is the off-switch.
pub fn set_length_soundness(on: bool) {
    LENGTH_SOUNDNESS.with(|cell| cell.set(on));
}

fn length_soundness() -> bool {
    LENGTH_SOUNDNESS.with(Cell::get)
}

/// Toggle the pass reading (default on, shipped 2026-07-18)
///
/// The general reading of a pass is **negative inference — it excludes every
/// bid and double the passer's table offered** (jdh8).  In a well-authored
/// table that complement is already written down as the Pass rule's own gate
/// (the opening table passes on `points(..12)` *because* the bids cover 12+),
/// so on, the projection pass decodes each pass off the union of its authored
/// table's Pass gates, both bounds
/// ([`project_band`][super::constraint::Constraint::project_band]): a no-open
/// pass caps at 11 points, the silent responder to a suit one-opening at 5 HCP
/// (10 on the upgraded scale), a pass of partner's 1NT at 13 with no six-card
/// major, a direct-seat pass over their suit opening at 17 HCP ("strong hands
/// double first regardless").  Tables whose pass gate is a trivial catch-all
/// — trap-pass advances, deep continuations — and floor (unauthored) passes
/// correctly read nothing.  Own-side passes decode from the reader's own
/// book; the *opponents'* passes only under [`set_table_alert_reading`], off
/// their phase-routed book, like the rest of table-wide disclosure.  The BEN
/// Info-net probe (docs/ben-gap-campaign.md) found passes ~60% of all reading
/// vagueness: we showed 0–37 where BEN commits ~6.3 mean HCP on a passed
/// hand — the biggest reading hole by volume.
///
/// Shipped default-on with `set_cue_reading` and `set_table_alert_reading`
/// as reading soundness: all three are **bid-inert** in the default system
/// (0, 0, and 1 divergent boards per 211k same-seed guard cells — the
/// on-disk witness in `ab-results/reading-knobs/2026-07-17/`), so a
/// plain/PD A/B is a wash by construction and the ship gate was the probe's
/// soundness numbers (0 new truth violations; acted-seat vagueness −60%).
/// Their payoff realizes wherever readings are consumed: sd-lead pricing,
/// search-mode sampling, disclosure.
pub fn set_pass_reading(on: bool) {
    PASS_READING.with(|cell| cell.set(on));
}

fn pass_reading() -> bool {
    PASS_READING.with(Cell::get)
}

/// Toggle pass-exclusion: a pass also excludes the sibling gates it declined
/// (default off)
///
/// [`set_pass_reading`] reads a pass off the table's own Pass gates — which in
/// a catch-all table (`hcp(0..)`: the weak-two and 1NT defenses, trap-pass
/// advances) says nothing, and those tables own the plurality of the reading
/// census's fully-blind passes.  This knob completes the stated negative
/// inference: the bidder is argmax over `weight + eval`, so a hand inside a
/// sibling gate whose weight strictly beats **every** Pass rule's weight could
/// not have passed — the passer lies in that gate's complement
/// ([`Rule::project_complement_dnf`][super::rules::Rule::project_complement_dnf]),
/// and the pass band may be intersected with it.
///
/// Only **single-box** complements are folded in — a shape-free
/// single-conjunct tier, such as the weak-two defense's `points(17..)` strong
/// double, whose complement is exactly `points(..=16)`.  A shaped or bounded
/// gate complements to a union (or to ⊤ via an off-axis atom), where the
/// per-box precision is not worth the term growth; skipping it costs
/// precision, never soundness.
///
/// The knob ships **off**, permanently opt-in: the feature retrain
/// (`evaluator_v3_exclusion`, served automatically under this knob) recovered
/// the net-OOD half of the pre-retrain loss but the re-measure A/B was a wash
/// in all four cells — no PD win, no ship (details:
/// `docs/ai-bidder/sampled-projection.md` § "The exclusion retrain").  Its
/// payoff is wherever readings are consumed directly (sd-lead pricing,
/// search-mode sampling, disclosure).
pub fn set_pass_exclusion_reading(on: bool) {
    PASS_EXCLUSION_READING.with(|cell| cell.set(on));
}

/// Whether the pass-exclusion reading is enabled (default off)
#[must_use]
pub fn pass_exclusion_reading() -> bool {
    PASS_EXCLUSION_READING.with(Cell::get)
}

/// Toggle the probed reading — behaviorally measured boxes folded into the
/// projection overlay (default off)
///
/// The sampled-projection derivation, stored: [`Stance::probe`][super::Stance::probe]
/// bids self-play deals and records the widened bounding box of the hands
/// that actually made each call, keyed by auction prefix.  On, the projection
/// pass intersects each prior call's probed box into both overlays.  This is
/// the only reader that reaches the floor's calls — a net's pass has no rule
/// to project, and the census's residual blind head (their 1NT/2♣/2NT
/// openings' passers, fourth-seat passes) is exactly that territory.
///
/// The boxes are **behavioral estimates**, widened at the edges (a sample
/// bound is not a rule bound) — see `Observed::boxed` in
/// [`book`][super::Stance] for the exact slack.  A stance with an empty
/// probed map reads identically with the knob on or off.
pub fn set_probed_reading(on: bool) {
    PROBED_READING.with(|cell| cell.set(on));
}

pub(crate) fn probed_reading() -> bool {
    PROBED_READING.with(Cell::get)
}

/// Toggle the vacuous-scoped probed reading — coverage where the symbolic
/// reading has none, and nowhere else (default off)
///
/// The full probed fold ([`set_probed_reading`]) was refuted as a bidding
/// input: its boxes tightened axes that already read, on both sides, and the
/// worst boards were penalty doubles of opponents misread as limited
/// (docs/ai-bidder/sampled-projection.md, the v1 A/B).  This knob serves the
/// same probed map through the failure-free slice:
///
/// - **own-side calls only** — the probe replays *our* system, so its boxes
///   model partner correctly and opponents wrongly (the self-referential
///   caveat);
/// - **fully-open axes only** — a probed axis folds in only where the
///   symbolic reading says nothing at all (points `0..=37`, a length
///   `0..=13`), so an axis that already reads is never tightened.  Latest
///   call first: the longest prefix is the sharpest conditioning, and once
///   it fills an axis, earlier keys leave it alone.
///
/// The target is the measured coverage hole — contested free bids and raises
/// the natural walk stamps nothing for (`1♦ (2♣) 2♠ P 3♠` all `0..13`,
/// docs/reading-drift-handoff.md), which no symbolic reader reaches.  A
/// third gate scopes the fold to **contested prefixes** (both sides have
/// acted): filling constructive axes smoke-tested at −0.67 IMPs/board of
/// net-OOD grand blasts.  A stance with an empty probed map reads
/// identically with the knob on or off; when both probed knobs are on, the
/// full fold wins.
///
/// Ships **off**, opt-in pending a feature retrain: the A/B (2026-07-31,
/// SEED_BASE 1785493701, 204,800 bd/arm/vul) lost in all four cells — plain
/// −0.0467/−0.0658, PD −0.1118/−0.1337 at ~10% fired — the worst boards all
/// the contested floor net acting on tightened partner boxes it never
/// trained on (balancing on, doubling, getting redoubled) where the base
/// arm settles.  Pre-registered reading: a pre-retrain loss is a floor, not
/// a verdict (the pass-exclusion precedent) — the queued path is the
/// probe-first retrain gate, then the F2b twin served under this knob.
pub fn set_probed_vacuous_reading(on: bool) {
    PROBED_VACUOUS_READING.with(|cell| cell.set(on));
}

pub(crate) fn probed_vacuous_reading() -> bool {
    PROBED_VACUOUS_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether [`project_authored`] also projects **unalerted** (natural) calls
    /// into the overlay (see [`set_natural_reading`]).  Off by default.
    static NATURAL_READING: Cell<bool> = const { Cell::new(false) };
}

/// Project every authored call, not only the alerted ones (default off)
///
/// The projection pass decodes a call when its authoring rule **alerts** it —
/// correct as *disclosure*, since an unalerted call is natural and the natural
/// walk reads it. But that leaves a whole regime unread: a rule that is authored
/// and natural (`gladiator_advances`'s game-forcing `3♣`/`3♦`/`3O`, authored
/// `len(suit, 5..) & points(game..)`) contributes **nothing** to the reading, and
/// the walk's guess from auction shape is an unverified duplicate that can and
/// does contradict it. See `docs/reading-drift-handoff.md`.
///
/// On, an unalerted call's rules project the same sound union as an alerted
/// one's, and it is **intersected with** the walk's natural reading rather than
/// replacing it — the call keeps its suppression bit clear, so the walk's
/// bookkeeping (natural-suit lanes, agreed fits, later cue detection) is
/// untouched and only the rule's own claim is added. Two consequences follow
/// from intersecting rather than substituting:
///
/// - Where the walk is *right* the reading strictly tightens: the rule's
///   strength band (which the walk usually has no way to know) lands on a call
///   that previously published only a length floor.
/// - Where the walk is *wrong* the boxes can go **empty**, because a wrong walk
///   claim intersected with a sound rule claim is still wrong. That is a
///   diagnostic, not a regression of this knob: it surfaces walk defects the
///   alert gate had been hiding. Sweep with the `admits` invariant before
///   reading anything into an A/B.
///
/// Default **off** pending that A/B: it tightens thousands of readings at once,
/// and per `docs/dnf-migration.md`'s C1 finding a tightening that moves
/// *endpoints* without moving *mass* is close to pure feature perturbation for
/// the frozen nets. Read at reading time, per-thread.
pub fn set_natural_reading(on: bool) {
    NATURAL_READING.with(|cell| cell.set(on));
}

fn natural_reading() -> bool {
    NATURAL_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether [`project_authored`] folds a second, *agreement* overlay off
    /// [`Rule::announce_dnf`][super::rules::Rule::announce_dnf] (see
    /// [`set_announced_reading`]).  Off by default — knob-off the announce
    /// overlay is a clone of the projection overlay and every reading is
    /// byte-identical.
    static ANNOUNCED_READING: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the agreement overlay — what a call *announces*, beside what it projects
///
/// [`Inferences`] serves two masters that want opposite things.  The sampler
/// ([`sample_layouts`][super::sampler::sample_layouts]) needs a box that
/// **contains** the truth, or it rejects the very hands the auction was bid on;
/// disclosure needs the box the partnership **agreement** names, which is what
/// partner reasons from and what the opponents are owed.  For an authored rule
/// the two coincide and nothing here changes.  They part company exactly where a
/// learned criterion decides: the evaluator net accepts hands no box contains, so
/// its sound projection is ⊤ and always will be, and the call reads as nothing.
///
/// On, every rule contributes a second overlay via
/// [`announce`][super::constraint::Constraint::announce], which defaults to
/// `project` and diverges only where the rule used
/// [`announced`][super::constraint::announced].  The projection overlay — and so
/// the sampler, `admits`, and the opening-lead sampling — is untouched; the
/// agreement overlay is what [`features`][super::features] hands the nets.
///
/// Default **off** pending its A/B.  Read at reading time, per-thread.
#[doc(hidden)]
pub fn set_announced_reading(on: bool) {
    ANNOUNCED_READING.with(|cell| cell.set(on));
}

fn announced_reading() -> bool {
    ANNOUNCED_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether [`project_authored`] also decodes the *opponents'* alerted
    /// calls (see [`set_table_alert_reading`]).  On by default (shipped
    /// 2026-07-18: bid-inert, reading soundness).
    static TABLE_ALERT_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle table-wide alert reading (default on, shipped 2026-07-18)
///
/// Alerting exists *for the opponents*: an alerted call is disclosed to the
/// whole table, not just remembered by partner.  The projection pass honors
/// only half of that by default — it decodes a call off its authoring rule
/// when the *reader's own* book authored it, so the opponents' alerted
/// conventions (their Stayman, their splinter, their checkback) fall to the
/// natural walk and can read as phantom suits.  On, the pass also resolves
/// each opponent call in *their* phase-routed book — modeling them as playing
/// our own books, exact in self-play and an approximation against other
/// natural-family engines — under their at-the-time context, and decodes it
/// when their rule alerts it.  Their unalerted (natural) calls still read by
/// the walk.
pub fn set_table_alert_reading(on: bool) {
    TABLE_ALERT_READING.with(|cell| cell.set(on));
}

fn table_alert_reading() -> bool {
    TABLE_ALERT_READING.with(Cell::get)
}

/// Toggle rule-replay layout acceptance (**default on**).
///
/// On (the default), the sampler reads each bid by the rule that authored it —
/// the meaning is frozen at the node, surviving later competition — rather than
/// by the per-convention range readers; the search EV
/// ([`ev_all`][crate::bidding::ev_all]) samples its rollout worlds this way.  Off
/// restores range-only sampling; the `ab-search-floor` example A/Bs the two via
/// `--no-rule-accept`.
pub fn set_rule_accept(on: bool) {
    RULE_ACCEPT.with(|cell| cell.set(on));
}

/// Whether rule-replay layout acceptance is enabled (**default on**).
#[must_use]
pub fn rule_accept_enabled() -> bool {
    RULE_ACCEPT.with(Cell::get)
}

/// An inclusive `[min, max]` range of a shown quantity — a length or points
///
/// A plain `Copy` pair rather than [`core::ops::RangeInclusive`], so it can be
/// stored, compared, and (de)serialized, and carries [`intersect`][Self::intersect].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Range {
    /// The least the quantity can be
    pub min: u8,
    /// The most the quantity can be
    pub max: u8,
}

impl Range {
    /// Nothing known about a suit length yet: `0..=13`
    pub const FULL_LENGTH: Self = Self {
        min: 0,
        max: LENGTH_CAP,
    };
    /// Nothing known about points yet: `0..=37`
    pub const FULL_POINTS: Self = Self {
        min: 0,
        max: POINTS_CAP,
    };
    /// Nothing known about a suit's HCP yet: `0..=10`
    pub const FULL_SUIT_HCP: Self = Self {
        min: 0,
        max: SUIT_HCP_CAP,
    };

    /// An inclusive `[min, max]` range
    #[must_use]
    pub const fn new(min: u8, max: u8) -> Self {
        Self { min, max }
    }

    /// `min..=cap` — at least `min`, up to the quantity's natural ceiling
    #[must_use]
    const fn at_least(min: u8, cap: u8) -> Self {
        Self { min, max: cap }
    }

    /// Whether `n` falls within the range
    #[must_use]
    pub const fn contains(self, n: u8) -> bool {
        self.min <= n && n <= self.max
    }

    /// The conjunction of two ranges — the tighter bounds of each
    ///
    /// Two independently sound inferences about the same quantity both hold, so
    /// the truth lies in their intersection.  If the bounds cross (an empty
    /// intersection), some inference was unsound for this auction; rather than
    /// drop the truth, widen to the *union* — soundness over tightness.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min <= max {
            Self { min, max }
        } else {
            Self {
                min: self.min.min(other.min),
                max: self.max.max(other.max),
            }
        }
    }

    /// The disjunction of two ranges — the loosest bounds spanning both
    ///
    /// A hand satisfying *either* of two alternatives (an `Or` projection of a
    /// [`Constraint`][super::constraint::Constraint]) has its quantity in one
    /// range or the other, so the sound envelope is their span.  The dual of
    /// [`intersect`][Self::intersect], which keeps the tighter bounds.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// The overlap, or `None` when the ranges are disjoint (an empty product)
    ///
    /// Unlike [`intersect`][Self::intersect], which widens a crossed range to
    /// preserve soundness within a single box, this reports the empty product so
    /// a [`Dnf`] can **drop** the contradictory term.
    #[must_use]
    fn intersect_nonempty(self, other: Self) -> Option<Self> {
        let (min, max) = (self.min.max(other.min), self.max.min(other.max));
        (min <= max).then(|| Self::new(min, max))
    }

    /// Whether `other` lies entirely within this range
    const fn encloses(self, other: Self) -> bool {
        self.min <= other.min && other.max <= self.max
    }
}

/// Shown strength, gauged on the several scales bridge counts on
///
/// The scales are **not mutually ordered**: raw [`hcp`][super::constraint::hcp],
/// the length-upgraded [`point_count`][super::constraint::point_count] (`points`),
/// and the fit-known shortness-upgraded
/// [`support_point_count`][super::constraint::support_point_count]
/// (`support_points`).  A reader promise is an axis-aligned interval on *one*
/// scale, so each scale is its own [`Range`] and the combinators fold
/// field-by-field.  The gauges are **marginals, never a joint**: no cross-gauge
/// relation (`points == hcp`, i.e. "balanced") fits a box — that is a shape fact,
/// and lives in [`Envelope::lengths`].  The one exception is the monotone floor
/// `support_points >= hcp`, which *is* box-representable and `canonicalize`
/// restores after every narrow.  (`points >= hcp` holds too, but is written at
/// the source by [`hcp`][super::constraint::hcp]'s projection rather than
/// restored here — see `canonicalize`.)  The shape fact *is* recoverable per
/// box, just not in the `(hcp, points)` plane: the lengths sit in the same box,
/// so `Envelope::narrow_to_upgrade` closes the two gauges against each other
/// under [`set_upgrade_closure`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Strength {
    /// Raw HCP, crisp (no upgrade slack) — the notrump-valuation gauge
    pub hcp: Range,
    /// HCP + long-suit upgrade, on the [`points`][super::constraint::points] scale
    /// the suit-oriented rules gauge (raw HCP for the balanced openings) — the
    /// legacy single axis
    pub points: Range,
    /// HCP + shortness, on the fit-known suit-indexed
    /// [`support_point_count_in`][super::constraint::support_point_count_in]
    /// scale — one slot per candidate trump suit, indexed by `suit as usize`
    /// like [`Envelope::lengths`], each gauged with its own suit as trump (no
    /// shortness value in the trump suit itself).
    pub support_points: [Range; 4],
    /// Raw HCP held in each suit, indexed by `suit as usize` like
    /// [`Envelope::lengths`]; cap 10 (AKQJ).  The honor-*location* gauge the
    /// quality gates (`suit_hcp`, `top_honors`) read — deliberately uncoupled
    /// from the whole-hand gauges in `canonicalize`: every candidate coupling
    /// either writes an old axis (a shipped-reading change) or manufactures
    /// the containment that lets `Dnf::tidy`'s correct dedup swallow the arm
    /// holding the suit knowledge.
    pub suit_hcp: [Range; 4],
}

impl Strength {
    /// Nothing shown yet: every whole-hand gauge `0..=37`, every suit `0..=10`
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            hcp: Range::FULL_POINTS,
            points: Range::FULL_POINTS,
            support_points: [Range::FULL_POINTS; 4],
            suit_hcp: [Range::FULL_SUIT_HCP; 4],
        }
    }

    /// Restore the sound cross-gauge floor `support_points >= hcp`
    ///
    /// `hcp` bounds every scale from below (`point_count` and
    /// `support_point_count` both add a non-negative upgrade to raw HCP, bar the
    /// rule-of-N flat-4-3-3-3 read the `points` scale already slacks by
    /// [`flat_hcp_slack`][super::constraint::flat_hcp_slack]).  So a pure-HCP
    /// promise (a 15–17 1NT) floors the support gauge for free, without a
    /// fit-showing raise having fired.  Monotone (only raises a `.min`), so it
    /// never narrows past the truth.
    ///
    /// [`points`][Self::points] is **not** floored here even though the same
    /// implication holds: its floor is written at the source by
    /// [`Hcp::project`][super::constraint::hcp], and adding it here would be a
    /// shipped-reading change (a bare `narrow_hcp` currently leaves `points`
    /// alone).  [`Envelope::narrow_to_upgrade`] is where that coupling lands,
    /// knob-gated and two-sided.
    ///
    /// [`suit_hcp`][Self::suit_hcp] is deliberately **not** coupled here: the
    /// sound whole-hand implications (`hcp.min >= Σ suit mins`, a length-capped
    /// suit ceiling) all write an *old* axis — a shipped-reading change — and
    /// the floor coupling manufactures exactly the containment that lets
    /// [`Dnf::tidy`]'s correct dedup swallow the arm carrying the suit
    /// knowledge (the `points(22..) | hcp(22..)` lesson, in reverse).
    fn canonicalize(&mut self) {
        let slack = super::constraint::flat_hcp_slack();
        let floor = self.hcp.min.saturating_sub(slack);
        for slot in &mut self.support_points {
            slot.min = slot.min.max(floor);
        }
    }

    /// Field-by-field [`Range::intersect`], then [`canonicalize`][Self::canonicalize]
    #[must_use]
    fn intersect(mut self, other: Self) -> Self {
        self.hcp = self.hcp.intersect(other.hcp);
        self.points = self.points.intersect(other.points);
        self.support_points =
            core::array::from_fn(|i| self.support_points[i].intersect(other.support_points[i]));
        self.suit_hcp = core::array::from_fn(|i| self.suit_hcp[i].intersect(other.suit_hcp[i]));
        self.canonicalize();
        self
    }

    /// Field-by-field [`Range::union`] — the `|` dual, soundness over tightness
    #[must_use]
    fn union(self, other: Self) -> Self {
        Self {
            hcp: self.hcp.union(other.hcp),
            points: self.points.union(other.points),
            support_points: core::array::from_fn(|i| {
                self.support_points[i].union(other.support_points[i])
            }),
            suit_hcp: core::array::from_fn(|i| self.suit_hcp[i].union(other.suit_hcp[i])),
        }
    }

    /// Bounded intersection; `None` only when the `points` gauge is disjoint
    ///
    /// **Only `points` gates box-emptiness**, exactly as the pre-`Strength` box
    /// algebra did.  The new gauges combine by the widening [`Range::intersect`]
    /// so they never drop a box a `points`/length reading would have kept — they
    /// are inert until Edits 1/2, and must not perturb the [`Dnf`] the sampler
    /// reads through `admits` (which reads `points` only).
    fn intersect_nonempty(self, other: Self) -> Option<Self> {
        let mut out = Self {
            hcp: self.hcp.intersect(other.hcp),
            points: self.points.intersect_nonempty(other.points)?,
            support_points: core::array::from_fn(|i| {
                self.support_points[i].intersect(other.support_points[i])
            }),
            suit_hcp: core::array::from_fn(|i| self.suit_hcp[i].intersect(other.suit_hcp[i])),
        };
        out.canonicalize();
        Some(out)
    }

    /// The support-points floor for `suit` as trump if that slot was narrowed
    /// from unknown, else `None` — a consumer reads this dedicated axis only
    /// when populated, and otherwise falls back to the length-scale
    /// [`points`][Self::points].  Suit-blind (the default) every slot holds
    /// the same band; suit-indexed, the slot is on the same units minus the
    /// trump suit's phantom shortness — never any length term.
    #[must_use]
    pub fn support_floor(&self, suit: Suit) -> Option<u8> {
        let slot = self.support_points[suit as usize];
        (slot.max < Range::FULL_POINTS.max).then_some(slot.min)
    }

    /// The raw-HCP floor if the [`hcp`][Self::hcp] gauge was narrowed, else `None`.
    #[must_use]
    pub fn hcp_floor(&self) -> Option<u8> {
        (self.hcp.max < Range::FULL_POINTS.max).then_some(self.hcp.min)
    }

    /// The sharpest sound scalar floor of the shown strength — the legacy
    /// [`points`][Self::points] floor lifted by every per-suit support promise
    ///
    /// The raise readers once wrote their support-scale bands verbatim onto
    /// the legacy axis, and every floor gate summing "own count + partner's
    /// shown floor" calibrated on that number.  Now that the legacy axis
    /// keeps only a band's sound image (`support_band_to_points`), this max
    /// hands those gates the same figure from its correct home.  Sound
    /// either way: each slot floor is a true claim about the hand's value in
    /// play with that trump, and `canonicalize` seeds slots only from the
    /// raw-HCP floor, itself a lower bound of every scale.
    #[must_use]
    pub fn shown_floor(&self) -> u8 {
        self.support_points
            .iter()
            .map(|slot| slot.min)
            .fold(self.points.min, u8::max)
    }
}

/// What the calls have shown about one player, hand-independently
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Envelope {
    /// Shown length range per suit, indexed by `suit as usize` (the ascending
    /// [`Suit::ASC`] order: clubs, diamonds, hearts, spades)
    pub lengths: [Range; 4],
    /// Shown strength, gauged on every scale (see [`Strength`])
    pub strength: Strength,
}

impl Envelope {
    /// Nothing shown yet: every suit `0..=13`, every strength gauge `0..=37`
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            lengths: [Range::FULL_LENGTH; 4],
            strength: Strength::unknown(),
        }
    }

    /// The shown length range of a suit
    #[must_use]
    pub const fn length(&self, suit: Suit) -> Range {
        self.lengths[suit as usize]
    }

    /// Narrow a suit's shown length by intersecting in `range`
    fn narrow_length(&mut self, suit: Suit, range: Range) {
        let slot = &mut self.lengths[suit as usize];
        *slot = slot.intersect(range);
    }

    /// Narrow the shown points (length scale) by intersecting in `range`
    fn narrow_points(&mut self, range: Range) {
        self.strength.points = self.strength.points.intersect(range);
    }

    /// Narrow the shown support points (fit-known scale) by intersecting in
    /// `range`
    ///
    /// Only fit-showing raises call this: a raise's point promise is valued on
    /// the support scale once the fit is agreed.  `suit` is the agreed trump —
    /// the promise narrows that suit's slot alone.
    pub(super) fn narrow_support_points(&mut self, suit: Suit, range: Range) {
        let slot = &mut self.strength.support_points[suit as usize];
        *slot = slot.intersect(range);
    }

    /// Narrow the shown raw HCP by intersecting in `range`, then propagate
    fn narrow_hcp(&mut self, range: Range) {
        self.strength.hcp = self.strength.hcp.intersect(range);
        self.strength.canonicalize();
    }

    /// Pointwise intersection — the `&` projection (both sets of bounds hold)
    ///
    /// The forward dual of a constraint conjunction: a hand accepted by `a & b`
    /// lies within both envelopes, so each quantity takes the tighter bounds
    /// ([`Range::intersect`]).
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.narrow_length(suit, other.length(suit));
        }
        out.strength = out.strength.intersect(other.strength);
        out
    }

    /// Pointwise union — the `|` projection (either set of bounds may hold)
    ///
    /// The forward dual of a constraint disjunction: a hand accepted by `a | b`
    /// lies within one envelope or the other, so each quantity spans both
    /// ([`Range::union`]) — soundness over tightness.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.lengths[suit as usize] = out.length(suit).union(other.length(suit));
        }
        out.strength = out.strength.union(other.strength);
        out
    }

    /// The intersection box, or `None` when any axis is disjoint (empty product)
    ///
    /// Unlike [`intersect`][Self::intersect], which widens a crossed range to
    /// preserve soundness *within* a single box, this reports the empty product
    /// so a [`Dnf`] can **drop** the contradictory term — the surviving terms of
    /// a union still cover every hand, so the union stays sound while getting
    /// tighter (e.g. `1NT ∩ 4-5♥` drops the balanced-diamond box whose hearts
    /// cannot reach four).
    fn intersect_nonempty(&self, other: &Self) -> Option<Self> {
        let mut lengths = [Range::FULL_LENGTH; 4];
        for suit in Suit::ASC {
            let (a, b) = (self.length(suit), other.length(suit));
            let (min, max) = (a.min.max(b.min), a.max.min(b.max));
            if min > max {
                return None;
            }
            lengths[suit as usize] = Range::new(min, max);
        }
        let strength = self.strength.intersect_nonempty(other.strength)?;
        Some(Self { lengths, strength })
    }

    /// Whether a hand's suit lengths and point count all fall within this box
    ///
    /// The per-box membership test the sampler and [`Dnf::contains`] share.
    /// Reads the `points` (length) gauge only — until
    /// [`set_gauge_membership`] (chop E, default off) also gives the raw-HCP
    /// and support-points bands membership teeth.
    #[must_use]
    pub fn admits(&self, hand: Hand) -> bool {
        Suit::ASC.into_iter().all(|suit| {
            // SAFETY: a suit length is at most 13, so the cast cannot truncate.
            #[allow(clippy::cast_possible_truncation)]
            let length = hand[suit].len() as u8;
            self.length(suit).contains(length)
        }) && self
            .strength
            .points
            .contains(super::constraint::point_count(hand))
            && (!gauge_membership()
                || (self.strength.hcp.contains(super::constraint::raw_hcp(hand))
                    && self.supports(hand)
                    && self.suit_hcps(hand)))
    }

    /// Whether the hand's support points fit every suit's slot, each gauged by
    /// [`support_point_count_in`][super::constraint::support_point_count_in]
    /// with that suit as trump (clamped at the scale cap, whose ceiling means
    /// "unbounded")
    fn supports(&self, hand: Hand) -> bool {
        Suit::ASC.into_iter().all(|suit| {
            let value = super::constraint::support_point_count_in(hand, suit);
            self.strength.support_points[suit as usize].contains(value.min(Range::FULL_POINTS.max))
        })
    }

    /// Whether the hand's raw HCP in each suit fits that suit's
    /// [`suit_hcp`][Strength::suit_hcp] slot (no clamp — a suit holds at most
    /// 10 HCP physically)
    fn suit_hcps(&self, hand: Hand) -> bool {
        Suit::ASC.into_iter().all(|suit| {
            self.strength.suit_hcp[suit as usize]
                .contains(super::constraint::suit_raw_hcp(hand, suit))
        })
    }

    /// Whether some 13-card hand can realize this box's suit lengths
    ///
    /// A box born of an intersection can be range-nonempty on every axis yet
    /// sum-infeasible — `balanced ∩ {3..}⁴` leaves a 5-3-3-3 product summing
    /// 14 — and such a **ghost box** admits nothing.  Dropping it from a union
    /// is exact.
    fn sum_feasible(&self) -> bool {
        let min: u8 = self.lengths.iter().map(|range| range.min).sum();
        let max: u8 = self.lengths.iter().map(|range| range.max).sum();
        min <= 13 && 13 <= max
    }

    /// Narrow the suit lengths to the tightest bounds `Σ len = 13` implies
    ///
    /// [`sum_feasible`][Self::sum_feasible] only *tests* the sum; a box born of
    /// an `&` keeps whatever ⊤ its arms left, so a both-majors reading stores
    /// `{♠ 5..13, ♥ 5..13, ♦ 0..13, ♣ 0..13}` when eight of those thirteen
    /// cards are already spoken for.  One sweep achieves bounds consistency for
    /// a single linear equality, and it is **exact**: the largest realizable
    /// `len_i` is `min(hi_i, 13 − Σ_{j≠i} lo_j)`, attained by giving `i` that
    /// value and distributing the rest, which is feasible for any
    /// `sum_feasible` box.  Symmetric for the floor.
    ///
    /// **Membership-inert** — every 13-card hand satisfies the sum, so
    /// [`admits`][Self::admits] is unchanged.  Only [`Dnf::hull`] (tighter) and
    /// [`subset_of`][Self::subset_of] (more containments) move.  Idempotent.
    fn narrow_to_sum(&mut self) {
        let total_min: u8 = self.lengths.iter().map(|range| range.min).sum();
        let total_max: u8 = self.lengths.iter().map(|range| range.max).sum();
        // Every suit reads the *original* totals — that is what makes the one
        // sweep bounds-consistent (and idempotent) rather than order-dependent.
        let before = self.lengths;
        for (range, was) in self.lengths.iter_mut().zip(before) {
            range.max = was.max.min(13_u8.saturating_sub(total_min - was.min));
            range.min = was.min.max(13_u8.saturating_sub(total_max - was.max));
        }
    }

    /// Close `hcp` and `points` against each other through the shape upgrade
    ///
    /// `points = hcp + upgrade` on the shipped scale, and the upgrade is a
    /// function of shape and honor placement — so the box's own `lengths` bound
    /// it.  The case that pays: **balanced hands never upgrade**
    /// ([`upgrade`][super::constraint::upgrade]; every balanced shape has its
    /// two longest suits at 9 cards or fewer), so a box whose lengths force
    /// balanced has `points == hcp` exactly, where
    /// [`Points::project`][super::constraint::points] slacked `hcp` by the
    /// scale's global worst case in both directions.
    ///
    /// Exact — it drops no hand the box claims — hence sound for
    /// [`subset_of`][Self::subset_of] dedup.  But **not membership-inert**,
    /// unlike [`narrow_to_sum`][Self::narrow_to_sum]: it bounds `points`,
    /// which [`admits`][Self::admits] tests, using `hcp`, which `admits`
    /// ignores until [`set_gauge_membership`].  So it gives an unenforced HCP
    /// claim teeth through `points`, and the sampler *does* move
    /// (`upgrade_closure_gives_hcp_teeth`).  A no-op on scales whose upgrade a
    /// length box cannot bound (see `super::constraint::upgrade_ceiling`).
    fn narrow_to_upgrade(&mut self) {
        let Some(ceiling) = super::constraint::upgrade_ceiling(&self.lengths) else {
            return;
        };
        // The floor of `points − hcp` is 0 on every closable scale (a wasted
        // honor can eat the whole upgrade), so only the ceiling is two-sided.
        let strength = &mut self.strength;
        strength.points.min = strength.points.min.max(strength.hcp.min);
        strength.hcp.max = strength.hcp.max.min(strength.points.max);
        strength.points.max = strength
            .points
            .max
            .min(strength.hcp.max.saturating_add(ceiling));
        strength.hcp.min = strength
            .hcp
            .min
            .max(strength.points.min.saturating_sub(ceiling));
        // A raised `hcp` floor owes the support gauge its floor back.
        strength.canonicalize();
    }

    /// Whether every hand in this box also lies in `other` — axis-wise
    /// enclosure over the lengths and every gauge
    fn subset_of(&self, other: &Self) -> bool {
        Suit::ASC
            .into_iter()
            .all(|suit| other.length(suit).encloses(self.length(suit)))
            && other.strength.hcp.encloses(self.strength.hcp)
            && other.strength.points.encloses(self.strength.points)
            && (self.strength.support_points.iter())
                .zip(other.strength.support_points)
                .all(|(inner, outer)| outer.encloses(*inner))
            && (self.strength.suit_hcp.iter())
                .zip(other.strength.suit_hcp)
                .all(|(inner, outer)| outer.encloses(*inner))
    }

    /// Whether a hand lies within this box on **every** gauge — the strict,
    /// gate-side membership test
    ///
    /// [`admits`][Self::admits] plus the raw-HCP and support-points gauges,
    /// each scored on its own scale (raw HCP, per-suit
    /// [`support_point_count_in`][super::constraint::support_point_count_in]).
    /// This is what a natively authored `Envelope`/[`Dnf`] **gate** evaluates
    /// through `Constraint::eval`: the box is the whole rule, so every stored
    /// bound — ceilings included — is enforced.  The reading-side
    /// [`admits`][Self::admits] stays lenient (lengths + `points` only) for
    /// sampler compatibility with
    /// projected readings.  Strict ⟹ lenient, so a native gate's accepted
    /// hands always satisfy the eval-within-projection soundness sweep.
    #[must_use]
    pub fn accepts(&self, hand: Hand) -> bool {
        self.admits(hand)
            && self.strength.hcp.contains(super::constraint::raw_hcp(hand))
            && self.supports(hand)
            && self.suit_hcps(hand)
    }
}

/// A forward reading as a union of boxes — disjunctive normal form
///
/// One [`Envelope`] is a single axis-aligned box; a disjunction (`Multi`, a
/// two-suiter, a `!`-shape) needs a *union* of boxes, which a single box cannot
/// hold without widening to the bounding box (the "`Or` wall").  A [`Dnf`] keeps
/// the terms: a hand is consistent with the call iff it lies in **some** box.
///
/// Sound by construction — every operation is *exact or widening, never
/// narrowing*.  [`intersect`][Self::intersect] distributes (Cartesian product of
/// box-intersects, dropping empty products); [`union`][Self::union] concatenates.
/// [`hull`][Self::hull] collapses the union back to the single bounding box, the
/// migration escape hatch that reproduces today's single-box reading.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Dnf(
    /// The disjoined boxes; non-empty (`len >= 1`) by invariant.
    Vec<Envelope>,
);

impl Dnf {
    /// Nothing shown yet: a single [`Envelope::unknown`] box
    #[must_use]
    pub fn unknown() -> Self {
        Self(vec![Envelope::unknown()])
    }

    /// The bounding box of the union — fold [`Envelope::union`] over the terms
    ///
    /// Today's single-box behaviour: every consumer that still wants one
    /// [`Envelope`] hulls here.  Never narrows (a hand in some term is in the
    /// hull), so hulling a sound `Dnf` stays sound.
    #[must_use]
    pub fn hull(&self) -> Envelope {
        self.0
            .iter()
            .copied()
            .reduce(|a, b| a.union(&b))
            .unwrap_or_else(Envelope::unknown)
    }

    /// Whether **some** box admits the hand — tighter than `hull().admits()`
    #[must_use]
    pub fn contains(&self, hand: Hand) -> bool {
        self.0.iter().any(|b| b.admits(hand))
    }

    /// The disjoined boxes — non-empty (`len >= 1`) by invariant
    #[must_use]
    pub fn boxes(&self) -> &[Envelope] {
        &self.0
    }

    /// Concatenate the terms — the `|` projection (either box may hold)
    #[must_use]
    pub fn union(mut self, mut other: Self) -> Self {
        self.0.append(&mut other.0);
        self
    }

    /// The `|` combine the projection fold uses: separate boxes under
    /// [`dnf_reading`], else the single bounding-box hull
    ///
    /// Off (the default), reproduces [`Envelope::union`] exactly, so the hull
    /// path stays byte-identical; on, keeps the arms so an enclosing `&`
    /// distributes and the sampler pins the disjunction.
    #[must_use]
    pub fn disjoin(self, other: Self) -> Self {
        if dnf_reading() {
            self.union(other).tidy()
        } else {
            Self::from(self.hull().union(&other.hull()))
        }
    }

    /// Cartesian product of pairwise box-intersects — the `&` projection
    ///
    /// `(A ∪ B) ∩ (C ∪ D) = (A∩C) ∪ (A∩D) ∪ (B∩C) ∪ (B∩D)`, dropping empty
    /// products (`Envelope::intersect_nonempty`).  The common case is one box
    /// each → one box out (`and` is a cheap box-shrink); growth needs *both*
    /// sides to be genuine disjunctions.  If every product is empty the whole
    /// conjunction is unsatisfiable, so fall back to the widened hull-intersect
    /// — sound and loose, never an empty (unsound) `Dnf`.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        for a in &self.0 {
            for b in &other.0 {
                if let Some(product) = a.intersect_nonempty(b) {
                    out.push(product);
                }
            }
        }
        if out.is_empty() {
            out.push(self.hull().intersect(&other.hull()));
        }
        // ponytail: no cap — `and`-of-two-`or`s is the only multiplier and it is
        // rare, so the Vec stays short on the real book.  The assert fires
        // loudly if some auction blows up; add sound exact-merge (containment +
        // axis-adjacency) only then.
        let out = Self(out).tidy();
        debug_assert!(
            out.0.len() < 64,
            "DNF term explosion: {} boxes",
            out.0.len()
        );
        out
    }

    /// Knob-on box hygiene — drop what changes nothing, keep the union exact
    ///
    /// Two prunes, both union-preserving: **ghost boxes** whose suit lengths
    /// are sum-infeasible ([`Envelope::sum_feasible`]) admit no hand, and a
    /// box **contained** in another ([`Envelope::subset_of`]) adds no hands
    /// (equal boxes keep their first copy).  Between them, under
    /// [`set_sum_closure`] / [`set_upgrade_closure`], each surviving box is
    /// narrowed to the bounds its own contents imply
    /// (`Envelope::narrow_to_sum`, `Envelope::narrow_to_upgrade`) — exact, so
    /// the extra containments the dedup then finds are real.  Runs only under [`dnf_reading`] —
    /// the knob-off hull path must stay byte-identical — and restores the
    /// non-empty invariant with ⊤ if every box was a ghost (an unsatisfiable
    /// conjunction; sound, loose, rare).
    fn tidy(mut self) -> Self {
        if !dnf_reading() {
            return self;
        }
        self.0.retain(Envelope::sum_feasible);
        if sum_closure() || upgrade_closure() {
            // Exact and membership-inert, so running it *before* the dedup is
            // safe: every containment it exposes is a real one.  Sum first —
            // it can force a box balanced, which is what the upgrade closure
            // reads.
            for box_ in &mut self.0 {
                if sum_closure() {
                    box_.narrow_to_sum();
                }
                if upgrade_closure() {
                    box_.narrow_to_upgrade();
                }
            }
        }
        let mut kept = Vec::with_capacity(self.0.len());
        'boxes: for (i, a) in self.0.iter().enumerate() {
            for (j, b) in self.0.iter().enumerate() {
                if i != j && a.subset_of(b) && (!b.subset_of(a) || j < i) {
                    continue 'boxes;
                }
            }
            kept.push(*a);
        }
        if kept.is_empty() {
            kept.push(Envelope::unknown());
        }
        Self(kept)
    }
}

impl From<Envelope> for Dnf {
    fn from(box_: Envelope) -> Self {
        Self(vec![box_])
    }
}

/// A seat relative to the player about to act, clockwise
///
/// The discriminant is the seating order from the actor — the natural index
/// for a sampler walking the four hands — *not* the auction's parity, so map
/// an auction position through `relative_of` rather than casting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Relative {
    /// The player about to act
    Me = 0,
    /// Left-hand opponent (the next to act)
    Lho = 1,
    /// Partner
    Partner = 2,
    /// Right-hand opponent (the previous to act)
    Rho = 3,
}

/// The relative seat of the call at `index` in an auction of length `len`
///
/// Mirrors [`Context`]'s parity: the call before the actor's (`len - index ==
/// 1`) is RHO, two before is partner, three before is LHO, four before is the
/// actor again.
pub(crate) const fn relative_of(len: usize, index: usize) -> Relative {
    match (len - index) % 4 {
        0 => Relative::Me,
        1 => Relative::Rho,
        2 => Relative::Partner,
        _ => Relative::Lho,
    }
}

/// A systems-on advance of our 1NT overcall, with their opening stripped
///
/// When `set_nt_overcall_systems_on` is enabled the advancer plays the full
/// opening-1NT structure grafted below `[their 1-of-a-suit, our 1NT]`, so the
/// artificial Stayman/transfer calls need the *opening-1NT* reading, not the
/// natural walk.  This returns the auction with their opening removed, which
/// reads exactly like an opening 1NT: `(len - index) % 4` is invariant under
/// removing one earlier call, so every later call keeps its relative seat (only
/// their opening — their own natural suit — is lost, which the opponents' system
/// discloses anyway).  [`None`] (the fast path) unless the graft is on and the
/// shape is their 1-suit opening immediately overcalled `1NT`.
fn systems_on_overcall_strip(auction: &[Call]) -> Option<Vec<Call>> {
    if !crate::bidding::american::nt_overcall_systems_on() {
        return None;
    }
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[open] else {
        return None;
    };
    if opening.level.get() != 1 || !opening.strain.is_suit() {
        return None;
    }
    if auction.get(open + 1) != Some(&Call::Bid(Bid::new(1, Strain::Notrump))) {
        return None;
    }
    // Over a MAJOR, Gladiator replaces the opening-1NT graft with a differently
    // shaped structure (cue = Stayman, 2♣ = relay), so the strip identity fails
    // — but only where Gladiator actually *has* that structure.  RHO's call over
    // our 1NT decides:
    //
    // | RHO      | Gladiator plays          | systems-on plays  | strip? |
    // | -------- | ------------------------ | ----------------- | ------ |
    // | pass     | the Gladiator advances   | the 1NT responses | no     |
    // | `2♣`     | the stolen relay (rebase)| systems on        | no     |
    // | `2♦/M`   | Transfer Lebensohl       | its own sohl      | no     |
    // | **X**    | a natural runout         | a natural runout  | yes    |
    // | **3+**   | the floor                | the floor         | yes    |
    //
    // The last two rows are the same auction in both systems, and the floor that
    // answers them is inference-aware — so denying it the stripped picture it
    // was distilled on changes *calls*, not just readings.  That was ~40% of the
    // treatment's measured loss (`vs-X-*` and `contested-other`); see
    // `docs/reading-drift-handoff.md`.
    if crate::bidding::american::nt_overcall_gladiator()
        && matches!(opening.strain, Strain::Hearts | Strain::Spades)
    {
        let gladiator_owns_it = match auction.get(open + 2) {
            None | Some(&Call::Pass) => true,
            Some(&Call::Bid(rho)) => rho.level.get() == 2,
            Some(&Call::Double | &Call::Redouble) => false,
        };
        if gladiator_owns_it {
            return None;
        }
    }
    let mut stripped = auction.to_vec();
    stripped.remove(open);
    Some(stripped)
}

/// All four players' shown shape and strength, relative to the side to act
///
/// `Vec`-backed [`Dnf`] means this is `Clone`, not `Copy` (two convertible call
/// sites: `narrowed_points`, `single_dummy`).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inferences {
    /// Per-seat bounding-box hull of `dnf` — the single-[`Envelope`] reading the
    /// American engine consumes via [`get`][Self::get].  A redundant cache of
    /// `dnf[i].hull()` (`ponytail: keeps get()->&Envelope and all readers
    /// unchanged; collapse to get-by-value if the two ever drift`).
    players: [Envelope; 4],
    /// Per-seat union-of-boxes reading; the sampler tests any-box under
    /// [`dnf_reading`].  Off, every entry is a single box equal to `players[i]`.
    dnf: [Dnf; 4],
    /// Per-seat hull of `announced` — the *agreement* twin of `players`, and
    /// what [`features`][super::features] hands the nets.  Equal to `players`
    /// unless [`set_announced_reading`] is on and some rule split the two with
    /// [`announced`][super::constraint::announced].
    announced_players: [Envelope; 4],
    /// Per-seat agreement boxes; the twin of `dnf` (see `announced_players`).
    announced: [Dnf; 4],
    /// The last call the M6.4 classifier read as a control bid: its auction
    /// index and the suit it agrees.  The exact witness for the instinct
    /// signoff — "the named suit is unread" cannot tell a control bid from an
    /// unread to-play bid.  Not part of the shown-range payload
    /// (serialization skips it).
    #[cfg_attr(feature = "serde", serde(skip))]
    control_bid: Option<(u8, Suit)>,
}

/// The sound legacy-`points` image of a fit-known support-scale band
///
/// The two scales share raw HCP and diverge by side-suit shortness credit
/// plus the double-fit term against the capped shape [`upgrade`]
/// [`point_count`] adds.  With three-plus trumps (every fit-known site) the
/// skew is bounded both ways: the support count exceeds [`point_count`] by at
/// most **5** (two side voids credit 6 and the double fit 1, while that shape
/// necessarily earns the full upgrade of 2 with nothing wasted), and trails
/// it by at most **1** (an unbalanced hand whose only short suits are working
/// doubletons upgrades 1 with no shortness credit).  So a support promise
/// `[F, C]` pins the legacy scale only to `[F − 5, C + 1]` — publishing the
/// band verbatim excluded the shapely light raises that measurably make it
/// (the `1♠ P 2♠` divergence-meter defect: observed point counts 4–10
/// against a published 6..=10), and [`Envelope::admits`] gauges the legacy
/// axis unconditionally, so the sampler refused to deal partner those hands.
/// Pinned by `support_band_points_image_is_sound`.
///
/// [`point_count`]: super::constraint::point_count
/// [`upgrade`]: super::constraint::upgrade
fn support_band_to_points(band: Range) -> Range {
    Range::new(
        band.min.saturating_sub(5),
        band.max.saturating_add(1).min(POINTS_CAP),
    )
}

impl Inferences {
    /// The shown shape and strength of one relative seat (the hull)
    #[must_use]
    pub const fn get(&self, who: Relative) -> &Envelope {
        &self.players[who as usize]
    }

    /// What one relative seat's calls **announce** — the partnership agreement
    ///
    /// The disclosure twin of [`get`][Self::get].  Equal to it unless
    /// [`set_announced_reading`] is on and a rule split the two with
    /// [`announced`][super::constraint::announced], which is how a call decided
    /// by the evaluator net can still say what it means: the sound projection
    /// stays ⊤ for the sampler while this carries the agreement.
    ///
    /// Consume this for disclosure and for anything that *reasons* about the
    /// auction (the nets' feature vectors); consume [`get`][Self::get] wherever
    /// a hand must be tested for consistency, because only that one is bound to
    /// contain the truth.
    #[must_use]
    pub const fn announced(&self, who: Relative) -> &Envelope {
        &self.announced_players[who as usize]
    }

    /// One relative seat's announced **union of boxes** — the unhulled twin of
    /// [`announced`][Self::announced]
    ///
    /// The hull is what the nets have always read, and hulling is where the
    /// information goes: `♥5..13` and `♥5..8` are the same claim, yet their
    /// endpoints differ by a third of the column's range.  Anything that wants
    /// the *distribution* a reading describes rather than its bounding box —
    /// [`features_eval_shape`][super::features::features_eval_shape] — reads
    /// the boxes here and tests membership atom by atom.
    #[must_use]
    pub const fn announced_dnf(&self, who: Relative) -> &Dnf {
        &self.announced[who as usize]
    }

    /// Assemble a reading from the natural walk's hull and the two overlays
    ///
    /// The agreement side reuses `players`, so a box it *widens* cannot show
    /// through — an announce looser than its projection is silently clipped
    /// back.  Sound for the pilot, whose agreement is strictly tighter than ⊤.
    // ponytail: re-running the walk per overlay is the fix if a looser
    // agreement ever needs to show through; nothing wants one yet.
    fn assemble(
        players: [Envelope; 4],
        overlay: &[Dnf; 4],
        agreement: &[Dnf; 4],
        control_bid: Option<(u8, Suit)>,
    ) -> Self {
        let announced = dnf_of(&players, agreement);
        let mut this = Self {
            dnf: dnf_of(&players, overlay),
            announced_players: std::array::from_fn(|i| announced[i].hull()),
            announced,
            players,
            control_bid,
        };
        if blind_opponent_reading() {
            for who in [Relative::Lho, Relative::Rho] {
                let i = who as usize;
                this.players[i] = Envelope::unknown();
                this.announced_players[i] = Envelope::unknown();
                this.dnf[i] = Dnf::unknown();
                this.announced[i] = Dnf::unknown();
            }
        }
        this
    }

    /// Whether `hand` is consistent with one seat's reading
    ///
    /// Under [`dnf_reading`] a hand must lie in *some* box of that seat's union
    /// (tighter — pins two-suiters / Multi / the fit-split); off, it need only
    /// lie in the bounding-box hull (today's acceptance).  The sampler's per-seat
    /// test.
    #[must_use]
    pub fn admits(&self, who: Relative, hand: Hand) -> bool {
        if dnf_reading() {
            self.dnf[who as usize].contains(hand)
        } else {
            self.players[who as usize].admits(hand)
        }
    }

    /// What the player to act has shown by their own prior calls
    #[must_use]
    pub const fn me(&self) -> &Envelope {
        self.get(Relative::Me)
    }

    /// What partner has shown
    #[must_use]
    pub const fn partner(&self) -> &Envelope {
        self.get(Relative::Partner)
    }

    /// What the left-hand opponent has shown
    #[must_use]
    pub const fn lho(&self) -> &Envelope {
        self.get(Relative::Lho)
    }

    /// What the right-hand opponent has shown
    #[must_use]
    pub const fn rho(&self) -> &Envelope {
        self.get(Relative::Rho)
    }

    /// A copy with one player's shown points intersected down to `points`
    ///
    /// Splits a shown range into halves for what-if sampling: narrowing an
    /// opener's points to the upper or lower half of what they have shown lets a
    /// caller deal layouts from each half and ask, double-dummy, whether game is
    /// good opposite a maximum but not a minimum — the meaning of an invitation.
    /// Intersects (never widens), so the result stays within what was shown.
    #[must_use]
    pub fn narrowed_points(&self, who: Relative, points: Range) -> Self {
        let mut copy = self.clone();
        let i = who as usize;
        copy.players[i].strength.points = copy.players[i].strength.points.intersect(points);
        // Narrow the points of every box in the union to keep `dnf` == the hull's
        // source (a points-only slab drops no box: it never crosses a length axis).
        let slab = Envelope {
            strength: Strength {
                points,
                ..Strength::unknown()
            },
            ..Envelope::unknown()
        };
        copy.dnf[i] = copy.dnf[i].intersect(&slab.into());
        // An externally-imposed points slice is a fact about the hand, not a
        // reading of a call, so it narrows the agreement side identically —
        // otherwise the two drift apart on the one axis the caller sliced.
        copy.announced_players[i].strength.points =
            copy.announced_players[i].strength.points.intersect(points);
        copy.announced[i] = copy.announced[i].intersect(&slab.into());
        copy
    }

    /// Derive, hand-independently, what every player's calls have shown under
    /// standard 2/1 meanings, relative to the side to act
    ///
    /// **A bare `Context::new` reads far less than the bidder does.**
    /// Projection-based reading — the pass bands, the authored-rule overlay —
    /// needs the convention keys that only `Stance::prefixed_context` attaches,
    /// so on a keyless context every one of them is skipped silently: no error,
    /// and `0..=37` is a perfectly well-formed answer. A pass that the bidder
    /// reads as `0..=11` comes back vacuous here. Diagnostics that want *what
    /// the bidder actually sees* must go through `Stance::infer`; this entry
    /// point is for the hand-coded walk alone.
    #[must_use]
    pub fn read(context: &Context<'_>) -> Self {
        // A systems-on advance of our 1NT overcall reads as an opening-1NT
        // auction with their opening stripped: the advancer plays the grafted
        // 1NT structure, so the hand-coded notrump walk reads its artificial
        // Stayman/transfer calls instead of the natural walk raising a phantom
        // suit.  Re-key the stripped auction through the attached stance so the
        // projection overlay survives the strip — a bare `Context::new` has no
        // trie prefixes, so `project_authored` silently skips every authored
        // rule, and the calls only the *book* knows are conventional (the
        // alerted both-majors `3♦`, which the walk's off-book arm reads as
        // natural `♦ 5..`) excluded their own bidders (`readings_admit_the_
        // bidder`).  The stripped opening is 1NT, which the strip never fires
        // on, so this recurses at most once.
        if let Some(stripped) = systems_on_overcall_strip(context.auction()) {
            return match context.their_system() {
                Some(stance) => Self::read(&stance.prefixed_context(context.vul(), &stripped)),
                None => Self::read(&Context::new(context.vul(), &stripped)),
            };
        }
        let auction = context.auction();
        let len = auction.len();
        let mut players = [Envelope::unknown(); 4];
        // The disjunctive overlay, folded into `players` (the hull) below and into
        // `dnf` (the boxes) at each return.  Unknown until `project_authored` runs.
        let mut overlay_dnf: [Dnf; 4] = std::array::from_fn(|_| Dnf::unknown());
        // The agreement twin of `overlay_dnf`; a clone of it unless
        // [`set_announced_reading`] is on (see [`project_authored`]).
        let mut agreement_dnf: [Dnf; 4] = std::array::from_fn(|_| Dnf::unknown());
        let mut control_bid = None;

        let Some(opening_index) = auction.iter().position(|&c| c != Call::Pass) else {
            // Nothing but passes so far — each is still a call with a reading:
            // a no-open pass declines the whole opening table, decoded by the
            // projection pass off the table's own Pass gate (`points(..12)` —
            // see [`set_pass_reading`]).  The walk below needs an opening, so
            // apply the overlay here and return.
            if pass_reading() {
                let (overlay, agreement, _) = project_authored(context);
                for (player, projected) in players.iter_mut().zip(&overlay) {
                    *player = player.intersect(&projected.hull());
                }
                overlay_dnf = overlay;
                agreement_dnf = agreement;
            }
            return Self::assemble(players, &overlay_dnf, &agreement_dnf, control_bid);
        };
        let Call::Bid(opening_bid) = auction[opening_index] else {
            return Self::assemble(players, &overlay_dnf, &agreement_dnf, control_bid);
        };
        let opener_lane = opening_index % 4;
        // SAFETY: at most three passes precede the opening, so the cast is safe.
        #[allow(clippy::cast_possible_truncation)]
        let opener_seat = opening_index as u8 + 1;
        let opening_artificial =
            opening_bid.strain == Strain::Notrump || opening_bid == Bid::new(2, Strain::Clubs);
        let defending_parity = (opener_lane + 1) % 2;
        let read_nt_invite = nt_invite_inference();
        // A 1NT–2♣ Stayman auction (opponents silent): opener's major answer and
        // responder's strength are read below so the floor judges the fit and
        // accepts or declines invitations.  The artificial 3OM / Smolen jumps are
        // suppressed from the natural suit reading rather than re-derived.
        let stayman = opening_bid == Bid::new(1, Strain::Notrump)
            && auction.get(opening_index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Clubs)));

        // Suits bid and the count of bids made, per auction lane (`index % 4`);
        // lanes of equal parity are partners, the same side.
        let mut lane_suits = [0u8; 4];
        // The subset of `lane_suits` the walk actually read as a natural
        // holding — a cue names a suit without ever showing it.
        let mut natural_lane_suits = [0u8; 4];
        // The subset of `natural_lane_suits` the lane has shown *twice* (a
        // rebid or jump-rebid suit, six long or a good five) — a raise of one
        // is routinely made on a doubleton, even jumping to game.
        let mut rebid_lane_suits = [0u8; 4];
        let mut lane_bids = [0u8; 4];
        let mut lane_doubled = [false; 4];
        let mut side_acted = [false; 2];
        let mut highest: Option<Bid> = None;
        let read_cues = cue_reading();
        let sound_lengths = length_soundness();

        // Rubens advances name relay suits; identify them so the natural reading
        // skips them, and capture a cue-raise's strength to apply afterwards.
        let (rubens_suppress, rubens_cue, rubens_transfer) = rubens_reading(auction);

        // The three declarative conventions — Jacoby transfers over our notrump,
        // Leaping Michaels, and Landy's 2♣ — are read straight off their authored
        // rule's projection rather than re-derived by hand (M6.2c).  `overlay`
        // records each artificial call's projected shape (applied post-walk);
        // `suppressed` is a bitset of the indices whose natural single-suit reading
        // the walk must skip.
        let (overlay_boxes, agreement_boxes, suppressed) = project_authored(context);
        overlay_dnf = overlay_boxes;
        agreement_dnf = agreement_boxes;
        // The hulled overlay the natural walk consumes (`shown_suit`, the post-walk
        // intersect); the boxes are re-combined into `dnf` at the return.
        let overlay: [Envelope; 4] = std::array::from_fn(|i| overlay_dnf[i].hull());
        // The one suppression the projection cannot see: the advancer's 2♦ relay /
        // 2♥-2♠ preference over a Landy/Woolsey both-majors 2♣ names no length of its
        // own, so its rule projects nothing — suppress it by hand (the doc's stub).
        let landy_relay = landy_advance_suppress(auction);
        // The Woolsey Multi family: 2♦ (a single 6+ major — its diamond reading
        // suppressed) and the 2♥/2♠ Muiderberg, recorded post-walk.
        let multi = multi_reading(auction);
        // The Woolsey takeout double of their 1NT: the doubler's points are recorded
        // post-walk and the advancer's 2♣ minor relay is suppressed.
        let woolsey_x = woolsey_x_reading(auction);
        // The DONT defense of their 1NT: the artificial X/2♣/2♦/2♥ and the advancer's
        // relay are suppressed; what each genuinely shows is recorded post-walk.
        let dont = dont_reading(auction);
        // The Meckwell defense of their 1NT: the two-way X (single minor OR both
        // majors) records points only; the 2♣/2♦ minor + major and the advancer's
        // relay are suppressed like DONT's.
        let meckwell = meckwell_reading(auction);
        // Our natural penalty double of their 1NT (15+): a double names no suit, so the
        // generic walk reads it as nothing — the points floor is recorded post-walk.
        let penalty_x = penalty_x_reading(auction);
        // The latch's subsequent penalty doubles: each promises four-plus in the suit
        // it doubles, recorded post-walk so the sampler does not read them as takeout.
        let penalty_latch_doubles = penalty_latch_double_reading(auction);
        // Responder's double of an overcall of our 1NT shows 8+ (every DoubleStyle),
        // recorded post-walk so opener does not undercount the partnership's strength.
        let overcall_double = responder_overcall_double_reading(auction, len);
        // Our Gladiator advance of a 1NT overcall of their major: the 2♣ relay (and
        // its forced 2♦), the cue-Stayman, the 3M splinter, and the 4M both-minor
        // Leaping Michaels are bids of a suit the caller lacks — suppressed here,
        // real shape recorded post-walk.
        let gladiator = gladiator_reading(auction);

        // Which calls the walk has suppressed so far (any reason: projection,
        // convention readers, the notrump-structure blanket, control bids) —
        // the control-bid classifier scans it for the agreed suit (M6.4).
        let mut suppressed_so_far = 0u64;

        for (index, &call) in auction.iter().enumerate() {
            let lane = index % 4;
            let who = relative_of(len, index) as usize;
            let is_opening_side = lane % 2 == opener_lane % 2;
            let first_action_of_side = !side_acted[lane % 2];

            match call {
                Call::Pass | Call::Redouble => {}
                Call::Double => {
                    // A direct double of a natural suit opening, the defending
                    // side's first action, reads as takeout: opening values.
                    if !is_opening_side
                        && first_action_of_side
                        && index != opening_index
                        && opening_bid.strain.is_suit()
                    {
                        players[who].narrow_points(Range::at_least(11, POINTS_CAP));
                    }
                    lane_doubled[lane] = true;
                    side_acted[lane % 2] = true;
                }
                Call::Bid(bid) => {
                    if index == opening_index {
                        apply_opening(&mut players[who], bid, opener_seat);
                    } else if let Some(suit) = bid.strain.suit() {
                        // A three-level suit bid over our 1NT is off-book and
                        // forcing — the instinct reading takes it as natural,
                        // five-plus (see `opener_forced_past_invitation`).  The
                        // two-level responses are Stayman and transfers.
                        //
                        // Our 1NT *overcall* is the same structure one seat
                        // over, so the advancer's three-level suit is natural
                        // and forcing too — never the weak six-card jump the
                        // `jump >= 1` arm below would read.  Systems-on gets
                        // this free (`systems_on_overcall_strip` deletes their
                        // opening and the auction reads as an opening 1NT);
                        // Gladiator turns the strip off because its advances
                        // differ, so the walk has to recognise the overcall
                        // itself — `gladiator_advances` authors the game-forcing
                        // `3♣`/`3♦`/`3O` as `len(suit, 5..)`, and a 6+ reading
                        // excluded every five-card advancer from its own box.
                        let one_nt = Bid::new(1, Strain::Notrump);
                        let our_one_nt_overcall = !is_opening_side
                            && opening_bid.level.get() == 1
                            && opening_bid.strain.is_suit()
                            && auction.get(opening_index + 1) == Some(&Call::Bid(one_nt))
                            && index > opening_index + 1
                            && (index - opening_index - 1) % 4 == 2;
                        // Only the lane's *first* bid: a three-level call made
                        // after an earlier (artificial) response — a second
                        // suit behind a Jacoby transfer, a super-accept — is
                        // structure, not the direct natural-forcing response,
                        // and reading it five-plus excluded the four-card
                        // second-suiters from their own box.  Gated out here it
                        // falls back under the notrump-structure blanket.
                        let over_one_notrump = bid.level.get() == 3
                            && lane_bids[lane] == 0
                            && ((is_opening_side && opening_bid == one_nt) || our_one_nt_overcall);
                        // Responder's 3OM slam try and Smolen jumps are
                        // artificial three-level majors in a new suit (partner
                        // never bid it); never read them as a natural long suit.
                        let stayman_artificial = stayman
                            && is_opening_side
                            && lane != opener_lane
                            && lane_bids[lane] >= 1
                            && bid.level.get() == 3
                            && matches!(bid.strain, Strain::Hearts | Strain::Spades)
                            && lane_suits[(lane + 2) % 4] & (1u8 << suit as u8) == 0;
                        // Responder's 1NT–3M splinter, when authored, is the
                        // shortest possible major: never a natural five-plus.
                        // Suppressing this *one* index (rather than routing it
                        // through `nt_structure_artificial`, whose `entered` set
                        // marks the whole continuation subtree) leaves opener's
                        // natural `3♠`/`4♣`/`4♦` rebids reading off the walk.
                        let nt_splinter_artificial = crate::bidding::american::nt_splinter()
                            && is_opening_side
                            && opening_bid == Bid::new(1, Strain::Notrump)
                            && index == opening_index + 2
                            && bid.level.get() == 3
                            && matches!(bid.strain, Strain::Hearts | Strain::Spades);
                        let nt_blanket = is_opening_side && opening_artificial && !over_one_notrump;
                        let chain = stayman_artificial
                            || nt_splinter_artificial
                            || nt_structure_artificial(auction, index, opening_index)
                            || rubens_suppress.contains(&Some(index))
                            || (index < 64 && suppressed >> index & 1 != 0)
                            || landy_relay == Some(index)
                            || multi.is_some_and(|m| m.suppresses(index))
                            || woolsey_x.is_some_and(|w| w.suppresses(index))
                            || dont.is_some_and(|d| d.suppresses(index))
                            || meckwell.is_some_and(|m| m.suppresses(index))
                            || gladiator.is_some_and(|g| g.suppresses(index));

                        // M6.4: a four-plus-level suit bid in the slam zone is
                        // classified control-bid vs to-play before the natural
                        // walk (see [`classify_high_bid`]).  It may punch
                        // through the notrump-structure blanket (the
                        // post-transfer 4♠ control) — but only when the
                        // projection is present to have claimed the genuinely
                        // artificial calls (Texas transfers) first.
                        let slam = if control_bid_reading()
                            && index != opening_index
                            && is_opening_side
                            && !side_acted[defending_parity]
                            && (4..=5).contains(&bid.level.get())
                            && !chain
                            && (!nt_blanket || context.prefixes().is_some())
                        {
                            classify_high_bid(
                                auction,
                                index,
                                bid,
                                len,
                                opening_index,
                                &players,
                                &overlay,
                                suppressed_so_far,
                            )
                        } else {
                            HighBid::Unclaimed
                        };

                        let suppress = match slam {
                            // To play (or an unreadable splinter): no record —
                            // flooring a six here rerouted combined-33 hands
                            // from the winning 6NT power-blast into thin 6-2
                            // suit slams (round 4 of the A/B).
                            HighBid::ToPlay => true,
                            HighBid::Control { trump, shower } => {
                                // A control bid: the bid suit is a control, not
                                // length — it agrees `trump`.  Agreeing one's
                                // own shown suit past game promises a sixth
                                // card; agreeing partner's promises support.
                                // Either way the slam try shows opening values
                                // and up (a sound floor; the real hand is
                                // stronger).
                                let floor = if shower == who { 6 } else { 3 };
                                players[who]
                                    .narrow_length(trump, Range::at_least(floor, LENGTH_CAP));
                                players[who].narrow_points(Range::at_least(13, POINTS_CAP));
                                #[allow(clippy::cast_possible_truncation)]
                                {
                                    control_bid = Some((index as u8, trump));
                                }
                                true
                            }
                            HighBid::Unclaimed => nt_blanket || chain,
                        };
                        if suppress && index < 64 {
                            suppressed_so_far |= 1 << index;
                        }

                        // Opener's extras-ladder rebid: a minor opening, opener's
                        // first rebid, opponents silent.  The jump-shift and
                        // reverse rungs name a real 4-card second suit and show
                        // extras — read below, not as a weak jump.
                        let opener_ladder_rebid = crate::bidding::american::opener_extras_ladder()
                            && !side_acted[defending_parity]
                            && is_opening_side
                            && lane == opener_lane
                            && lane_bids[lane] == 1
                            && opening_bid.level.get() == 1
                            && matches!(opening_bid.strain, Strain::Clubs | Strain::Diamonds);

                        if !suppress {
                            let jump = bid
                                .level
                                .get()
                                .saturating_sub(cheapest_level(highest, bid.strain));
                            let mask = 1u8 << suit as u8;
                            let i_bid_it = lane_suits[lane] & mask != 0;
                            let partner_bid_it = lane_suits[(lane + 2) % 4] & mask != 0;
                            // A bid of a suit only the opponents have naturally
                            // shown is a cue, never a holding (`set_cue_reading`).
                            let opponents_natural = natural_lane_suits[(lane + 1) % 4]
                                | natural_lane_suits[(lane + 3) % 4];
                            let opponents_shown_it = read_cues && opponents_natural & mask != 0;

                            if i_bid_it {
                                // Rebidding our own suit shows a sixth card —
                                // except (`set_length_soundness`) a re-raise of
                                // a suit partner has also bid (agreed, so no new
                                // length) and opener's immediate two-level rebid
                                // of the opened suit, routinely a good five
                                // (a minor, or a major stuck over the forcing
                                // notrump).
                                let agreed_re_raise = sound_lengths && partner_bid_it;
                                let five_card_rebid = sound_lengths
                                    && lane == opener_lane
                                    && lane_bids[lane] == 1
                                    && bid.level.get() == 2
                                    && opening_bid.strain.suit() == Some(suit);
                                // Under XYZ (`set_xyz`) responder's two-level
                                // rebid of the one-level major is authored
                                // five-plus, both routes: the direct 2M weak
                                // sign-off and the invitational 2M through the
                                // 2♣ relay (`xyz_responder`/`xyz_after_relay`).
                                // Reading a sixth card excluded every five-card
                                // responder from their own box.
                                let xyz_rebid = crate::bidding::american::xyz()
                                    && !side_acted[defending_parity]
                                    && is_opening_side
                                    && lane != opener_lane
                                    && matches!(suit, Suit::Hearts | Suit::Spades)
                                    && bid.level.get() == 2
                                    && opening_bid.level.get() == 1
                                    && opening_bid.strain.is_suit()
                                    && auction.get(opening_index + 2)
                                        == Some(&Call::Bid(Bid::new(1, Strain::from(suit))))
                                    && matches!(
                                        auction.get(opening_index + 4),
                                        Some(Call::Bid(rebid)) if rebid.level.get() == 1
                                    )
                                    && (index == opening_index + 6
                                        || (index == opening_index + 10
                                            && auction.get(opening_index + 6)
                                                == Some(&Call::Bid(Bid::new(2, Strain::Clubs)))
                                            && auction.get(opening_index + 8)
                                                == Some(&Call::Bid(Bid::new(
                                                    2,
                                                    Strain::Diamonds,
                                                )))));
                                if !agreed_re_raise {
                                    let floor = if five_card_rebid || xyz_rebid { 5 } else { 6 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
                                }
                                if natural_lane_suits[lane] & mask != 0 {
                                    rebid_lane_suits[lane] |= mask;
                                }
                                natural_lane_suits[lane] |= mask;
                            } else if partner_bid_it {
                                // Raising partner's suit shows three-card support
                                // — unless partner has already shown six-plus (a
                                // preempt, a weak jump, a jump-rebid suit): a
                                // raise of a known-long suit to game is routinely
                                // made on a doubleton or stiff honour, so no
                                // length claim is sound there.  A *delayed*
                                // return to a suit partner has shown five-plus
                                // (the opened major back over the forcing
                                // notrump, an XYZ five-card rebid) floors at two
                                // — the false preference on Hx is the norm, not
                                // the exception.  A preference takes the cheapest
                                // route, so a jump return qualifies only when
                                // partner has bid the suit twice; direct raises
                                // and raises of 4-card or unknown-length suits
                                // keep the three-card claim.
                                let partner_length = players[(who + 2) % 4].length(suit).min;
                                if partner_length < 6 {
                                    let partner_rebid_it =
                                        rebid_lane_suits[(lane + 2) % 4] & mask != 0;
                                    let delayed = (jump == 0 || partner_rebid_it)
                                        && lane_bids[lane] >= 1
                                        && partner_length >= 5;
                                    let floor = if delayed { 2 } else { 3 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
                                }
                                natural_lane_suits[lane] |= mask;
                            } else if opponents_shown_it {
                                // A cue: no length in the named suit.  Record the
                                // two meanings that hold robustly across natural
                                // systems; anything else stays silent (soundness
                                // over tightness).
                                let partner_natural = natural_lane_suits[(lane + 2) % 4];
                                let michaels = !is_opening_side
                                    && first_action_of_side
                                    && partner_natural == 0
                                    && opponents_natural == mask
                                    && opening_bid.strain.suit() == Some(suit)
                                    && matches!(suit, Suit::Clubs | Suit::Diamonds)
                                    && (jump == 0 || (opening_bid.level.get() == 2 && jump == 1));
                                if michaels {
                                    // A direct cue of their minor opening —
                                    // Michaels (or Leaping Michaels over the weak
                                    // two): both majors, five-five.  Strength
                                    // stays open (mini-max styles run wide).
                                    players[who].narrow_length(
                                        Suit::Hearts,
                                        Range::at_least(5, LENGTH_CAP),
                                    );
                                    players[who].narrow_length(
                                        Suit::Spades,
                                        Range::at_least(5, LENGTH_CAP),
                                    );
                                } else if jump == 0 && partner_natural.count_ones() == 1 {
                                    // A non-jump cue opposite one natural suit:
                                    // the limit-plus cue-raise (mirrors the
                                    // Rubens cue-raise floors).
                                    let agreed =
                                        Suit::ASC[partner_natural.trailing_zeros() as usize];
                                    players[who]
                                        .narrow_length(agreed, Range::at_least(3, LENGTH_CAP));
                                    // Fit agreed (the cue names partner's suit), so
                                    // the raise's point promise is a support-scale
                                    // one; the legacy axis takes only its sound
                                    // image.
                                    let band = Range::at_least(10, POINTS_CAP);
                                    players[who].narrow_points(support_band_to_points(band));
                                    players[who].narrow_support_points(agreed, band);
                                }
                            } else if over_one_notrump {
                                // Natural, forcing five-card suit over our 1NT.
                                players[who].narrow_length(suit, Range::at_least(5, LENGTH_CAP));
                                natural_lane_suits[lane] |= mask;
                            } else if !is_opening_side && first_action_of_side {
                                // The defending side's first suit bid is an
                                // overcall: a five-card suit (six if jumping),
                                // opening values at the cheapest level.
                                let min = if jump >= 1 { 6 } else { 5 };
                                players[who].narrow_length(suit, Range::at_least(min, LENGTH_CAP));
                                natural_lane_suits[lane] |= mask;
                                if jump == 0 {
                                    players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                                }
                            } else if jump >= 1 {
                                // A single jump in a new suit is a weak jump: a
                                // six-card suit.  Skip splinters (double jumps)
                                // — and, under `set_length_soundness`, a player
                                // who has doubled (their jump is strength on as
                                // few as three cards; claim nothing).  Opener's
                                // extras-ladder jump-shift is instead a strong
                                // 5-4, so the jumped suit is only 4+.
                                if jump == 1 && !(sound_lengths && lane_doubled[lane]) {
                                    let floor = if opener_ladder_rebid { 4 } else { 6 };
                                    players[who]
                                        .narrow_length(suit, Range::at_least(floor, LENGTH_CAP));
                                    natural_lane_suits[lane] |= mask;
                                }
                            } else {
                                // A natural new suit at the cheapest level: four-plus.
                                players[who].narrow_length(suit, Range::at_least(4, LENGTH_CAP));
                                natural_lane_suits[lane] |= mask;
                                apply_response_points(
                                    &mut players[who],
                                    bid,
                                    opening_bid,
                                    is_opening_side
                                        && lane == (opener_lane + 2) % 4
                                        && lane_bids[lane] == 0
                                        && !side_acted[defending_parity],
                                );
                            }
                        }
                    }

                    // Strength shown by limited natural rebids and raises, read
                    // only when the opponents have stayed silent (a competitive
                    // 2NT or raise can be off-meaning).  Every branch narrows by
                    // a sound bound — the true point count always falls within.
                    if index != opening_index && !side_acted[defending_parity] {
                        let responder_lane = (opener_lane + 2) % 4;
                        let opener_rebid =
                            is_opening_side && lane == opener_lane && lane_bids[lane] == 1;
                        let responder_first =
                            is_opening_side && lane == responder_lane && lane_bids[lane] == 0;
                        let opening_one_suit =
                            opening_bid.level.get() == 1 && opening_bid.strain.is_suit();

                        if read_nt_invite
                            && bid.strain == Strain::Notrump
                            && opening_bid == Bid::new(1, Strain::Notrump)
                            && responder_first
                        {
                            // Responder's notrump action over our 1NT opening.
                            // 3NT forces game (9+) in both minor schemes; the 2NT
                            // meaning is scheme-dependent — Puppet's 2NT is the
                            // diamond transfer (5+ diamonds), European's is a
                            // balanced invitational ~8 (the size ask).  Stayman, the
                            // major transfers, and the artificial minor calls
                            // (Puppet 2♠/3♣, European 2♠ clubs / 3♣ diamonds) stay
                            // silent here — `project_authored` narrows the single
                            // suits.  This is what lets opener (or the sampler behind
                            // the search floor) judge responder.
                            match bid.level.get() {
                                2 => {
                                    if crate::bidding::american::notrump_minors()
                                        == crate::bidding::american::EUROPEAN
                                    {
                                        players[who].narrow_points(Range::new(8, 9));
                                    } else {
                                        players[who].narrow_length(
                                            Suit::Diamonds,
                                            Range::at_least(5, LENGTH_CAP),
                                        );
                                    }
                                }
                                3 => players[who].narrow_points(Range::at_least(9, POINTS_CAP)),
                                _ => {}
                            }
                        } else if bid.strain == Strain::Notrump && opening_one_suit {
                            if opener_rebid {
                                // A balanced rebid.  1NT is a minimum (12–16: a
                                // 17 would open the strong notrump); a *jump* to
                                // 2NT is the strong 18–19 rebid.  A non-jump 2NT
                                // (over a two-level response) is a minimum and is
                                // left to the opening's bound.
                                let nt_jump = bid
                                    .level
                                    .get()
                                    .saturating_sub(cheapest_level(highest, Strain::Notrump));
                                if bid.level.get() == 1 {
                                    players[who].narrow_points(Range::new(12, 16));
                                } else if bid.level.get() == 2 && nt_jump >= 1 {
                                    players[who].narrow_points(Range::new(18, 21));
                                }
                            } else if responder_first && bid.level.get() == 1 {
                                // A 1NT response: a natural or forcing notrump.
                                players[who].narrow_points(Range::new(6, 12));
                            }
                        } else if let Some(suit) = bid.strain.suit() {
                            // Responder raising opener's suit shows limited
                            // support strength: a single raise constructive, a
                            // jump raise invitational.  One-level openings only:
                            // a raise of a preempt is two-way — furthering the
                            // preempt on nothing OR bidding a game to make on
                            // 16+ — so no strength band is sound there (the
                            // `1..=11` image of the constructive band excluded
                            // every to-make raiser of `[3♥ P 4♥]` from its own
                            // box).
                            let partner_bid_it =
                                lane_suits[(lane + 2) % 4] & (1 << suit as u8) != 0;
                            if responder_first && partner_bid_it && opening_one_suit {
                                let jump = bid
                                    .level
                                    .get()
                                    .saturating_sub(cheapest_level(highest, bid.strain));
                                // Fit agreed (raising opener's suit), so the raise
                                // strength is a support-scale promise — the
                                // support gauge carries it exactly; the legacy
                                // axis takes only its sound image
                                // (`support_band_to_points`).
                                let band = match jump {
                                    0 => Some(Range::new(6, 10)),
                                    1 => Some(Range::new(10, 12)),
                                    _ => None,
                                };
                                if let Some(band) = band {
                                    players[who].narrow_points(support_band_to_points(band));
                                    players[who].narrow_support_points(suit, band);
                                }
                            }
                        }
                    }

                    // Opener's extras-ladder rebid shows extras and — for a
                    // new-suit rung — a five-card opened suit.  Sound floors: the
                    // jump-rebid is 16+, the reverse 17+, the jump-shift 18+.
                    if crate::bidding::american::opener_extras_ladder()
                        && !side_acted[defending_parity]
                        && is_opening_side
                        && lane == opener_lane
                        && lane_bids[lane] == 1
                        && opening_bid.level.get() == 1
                        && matches!(opening_bid.strain, Strain::Clubs | Strain::Diamonds)
                        && let (Some(bid_suit), Some(opened)) =
                            (bid.strain.suit(), opening_bid.strain.suit())
                    {
                        let jump = bid
                            .level
                            .get()
                            .saturating_sub(cheapest_level(highest, bid.strain));
                        let responder_bid_it =
                            lane_suits[(lane + 2) % 4] & (1 << bid_suit as u8) != 0;
                        if bid_suit == opened {
                            // Jump-rebid of opener's own suit.
                            if jump >= 1 {
                                players[who].narrow_points(Range::at_least(16, POINTS_CAP));
                            }
                        } else if !responder_bid_it {
                            // Reverse (non-jump two-level, higher suit) or
                            // jump-shift (single jump): a five-card opened suit.
                            let reverse = jump == 0
                                && bid.level.get() == 2
                                && (bid.strain as u8) > (Strain::from(opened) as u8);
                            let jump_shift = jump == 1;
                            if reverse || jump_shift {
                                players[who].narrow_length(opened, Range::at_least(5, LENGTH_CAP));
                                let floor = if jump_shift { 18 } else { 17 };
                                players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
                            }
                        }
                    }

                    // Opener's major jump-rebid (set_opener_major_jump_rebid):
                    // a 3M jump in opener's own opened major over 1♥-1♠ / 1M-1NT
                    // shows 16+.  Natural, so the six-card length is read above
                    // (the `i_bid_it` branch); add the strength floor here.
                    if crate::bidding::american::opener_major_jump_rebid()
                        && !side_acted[defending_parity]
                        && is_opening_side
                        && lane == opener_lane
                        && lane_bids[lane] == 1
                        && opening_bid.level.get() == 1
                        && matches!(opening_bid.strain, Strain::Hearts | Strain::Spades)
                        && bid.strain == opening_bid.strain
                        && bid
                            .level
                            .get()
                            .saturating_sub(cheapest_level(highest, bid.strain))
                            >= 1
                    {
                        players[who].narrow_points(Range::at_least(16, POINTS_CAP));
                    }

                    // Stayman: read opener's major answer and responder's
                    // strength (opponents silent) so the floor judges the fit and
                    // accepts or declines invitations.
                    if stayman && is_opening_side && !side_acted[defending_parity] {
                        let responder_lane = (opener_lane + 2) % 4;
                        if index == opening_index + 2 {
                            // Responder's 2♣ Stayman shows invitational+ values —
                            // unless garbage or crawling Stayman is on, where a weak
                            // hand may bid 2♣ to escape, so the floor must not assume
                            // 8+.
                            if !crate::bidding::american::garbage_stayman()
                                && !crate::bidding::american::crawling_stayman()
                            {
                                players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                            }
                        } else if index == opening_index + 4 && lane == opener_lane {
                            // Opener's answer names or denies a four-card major.
                            match bid.strain {
                                Strain::Hearts => players[who]
                                    .narrow_length(Suit::Hearts, Range::at_least(4, LENGTH_CAP)),
                                Strain::Spades => {
                                    players[who].narrow_length(
                                        Suit::Spades,
                                        Range::at_least(4, LENGTH_CAP),
                                    );
                                    players[who].narrow_length(Suit::Hearts, Range::new(0, 3));
                                }
                                Strain::Diamonds => {
                                    players[who].narrow_length(Suit::Hearts, Range::new(0, 3));
                                    players[who].narrow_length(Suit::Spades, Range::new(0, 3));
                                }
                                _ => {}
                            }
                        } else if index == opening_index + 6 && lane == responder_lane {
                            // Responder's invitational continuations pin strength
                            // for opener's accept/decline; game and quantitative
                            // calls speak for themselves.
                            let raise_of_major = bid
                                .strain
                                .suit()
                                .is_some_and(|s| lane_suits[opener_lane] & (1u8 << s as u8) != 0);
                            match (bid.level.get(), bid.strain) {
                                (2, Strain::Notrump) => {
                                    players[who].narrow_points(Range::new(8, 9));
                                }
                                (3, Strain::Notrump) => {
                                    players[who].narrow_points(Range::at_least(9, POINTS_CAP));
                                }
                                (3, s) if s.is_suit() && raise_of_major => {
                                    players[who].narrow_points(Range::new(8, 9));
                                }
                                _ => {}
                            }
                        }
                    }

                    if let Some(suit) = bid.strain.suit() {
                        lane_suits[lane] |= 1 << suit as u8;
                        // A natural suit opening is a shown holding (the strong
                        // 2♣ is not); later calls set their bits in the walk.
                        if index == opening_index && !opening_artificial {
                            natural_lane_suits[lane] |= 1 << suit as u8;
                        }
                    }
                    lane_bids[lane] += 1;
                    side_acted[lane % 2] = true;
                    if highest.is_none_or(|h| outranks(bid, h)) {
                        highest = Some(bid);
                    }
                }
            }
        }

        // A two-level cue-raise shows a limit-plus raise: three-plus cards in
        // partner's overcall and opening values.  Recorded after the walk (the
        // cue itself named the opponents' suit, suppressed above).
        if let Some((cue_index, overcall_suit)) = rubens_cue {
            let who = relative_of(len, cue_index) as usize;
            players[who].narrow_length(overcall_suit, Range::at_least(3, LENGTH_CAP));
            // Fit agreed (cue of partner's overcall), a support-scale promise;
            // the legacy axis takes only its sound image.
            let band = Range::at_least(10, POINTS_CAP);
            players[who].narrow_points(support_band_to_points(band));
            players[who].narrow_support_points(overcall_suit, band);
        }

        // A one-level Rubens transfer records its meaning likewise (see
        // [`set_rubens_transfer_reading`]) — but only for the advancer's own
        // side: the transfer semantics are *our* agreement, and an opponent's
        // in-band advance may be a genuine suit (asserting length in the suit
        // above would poison the sampler).  Suppression above stays side-blind:
        // it only loses information, never asserts any.
        if let Some((transfer_index, suit, min_len)) = rubens_transfer {
            let who = relative_of(len, transfer_index);
            if matches!(who, Relative::Me | Relative::Partner) {
                let who = who as usize;
                players[who].narrow_length(suit, Range::at_least(min_len, LENGTH_CAP));
                players[who].narrow_points(Range::at_least(10, POINTS_CAP));
            }
        }

        // The three declarative conventions (Jacoby transfers over our notrump,
        // Leaping Michaels, Landy's 2♣) are recorded from their authored rule's
        // projection — the `overlay` computed above — not a hand-written decoder
        // (M6.2c).  Sound but looser than the old readers: it pins the 2♦ transfer's
        // five-card floor, not the six-card jump upgrade the reader inferred from a
        // later call.  The DONT/Woolsey/Multi conventions below are now transparent
        // `or`/`and` shapes too (M6.2d), so the both-majors family (DONT `2♥`, the
        // direct-Landy `X`) also surfaces in `overlay` here — redundant with, and
        // identical to, the per-suit floors recorded by hand below (an idempotent
        // intersect).  The hand recordings stay: they carry the one-suiter/disjunction
        // floors the `or`-union washes out, which the projection cannot pin.
        for (seat, projected) in overlay.iter().enumerate() {
            players[seat] = players[seat].intersect(projected);
        }

        // A Woolsey Multi-family overcall.  The "6+ major" (2♦) and "4+ minor"
        // (2♥/2♠) are disjunctions the per-suit framework cannot pin to one suit, so
        // they are captured by the *residual*: capping the other three suits forces
        // the sampler to deal the length into the long suit (the same loose handling
        // Landy uses for its 5-4).
        if let Some(multi) = multi {
            let who = relative_of(len, multi.overcall_index) as usize;
            match multi.kind {
                // 2♦ Multi: a true one-suiter, so both minors ≤ 4 (the natural ≥5
                // diamond reading was suppressed above; clubs narrows from full).
                MultiKind::Major => {
                    players[who].narrow_length(Suit::Clubs, Range::new(0, 4));
                    players[who].narrow_length(Suit::Diamonds, Range::new(0, 4));
                }
                // 2♥/2♠ Muiderberg: exactly 5 in the major, ≤ 3 in the other major
                // (refining the natural ≥5 reading); the 4+ minor falls out of the
                // residual.
                MultiKind::Muiderberg(major) => {
                    let other = if major == Suit::Hearts {
                        Suit::Spades
                    } else {
                        Suit::Hearts
                    };
                    players[who].narrow_length(major, Range::new(5, 5));
                    players[who].narrow_length(other, Range::new(0, 3));
                }
            }
            let floor = crate::bidding::american::woolsey_points().0;
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // A Woolsey takeout double of their 1NT (4-card major + 5-6 card minor).  The
        // shape is a double disjunction the per-suit framework cannot pin, so only the
        // points floor is recorded — enough to stop the floor sampling the doubler as a
        // random weak hand (a double of 1NT is otherwise read as nothing).
        if let Some(woolsey_x) = woolsey_x {
            let who = relative_of(len, woolsey_x.double_index) as usize;
            let floor = crate::bidding::american::woolsey_double_floor();
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // A DONT overcall of their 1NT.  The X one-suiter and the 2♣/2♦ minor are
        // disjunctions (the long suit / the unknown major) the per-suit framework
        // cannot pin, so only the sound per-suit fact is recorded; the residual carries
        // the rest.  The 2♥ both-majors pins both like Landy.  In each case the points
        // floor stops the floor sampling the overcaller as a random hand.
        if let Some(dont) = dont {
            let who = relative_of(len, dont.overcall_index) as usize;
            match dont.kind {
                // One-suiter in ♣/♦/♥ (spades excluded); the long suit falls out of the
                // residual, only spades ≤ 3 is certain.
                DontKind::OneSuiter => players[who].narrow_length(Suit::Spades, Range::new(0, 3)),
                // 2♣/2♦: a real ≥ 4 minor (the natural ≥ 5 reading was suppressed); the
                // unknown major surfaces naturally if later named, else the residual.
                DontKind::ClubsMajor => {
                    players[who].narrow_length(Suit::Clubs, Range::at_least(4, LENGTH_CAP));
                }
                DontKind::DiamondsMajor => {
                    players[who].narrow_length(Suit::Diamonds, Range::at_least(4, LENGTH_CAP));
                }
                // 2♥: both majors, ≥ 4-4 (the natural ≥ 5 heart reading was suppressed).
                DontKind::BothMajors => {
                    players[who].narrow_length(Suit::Hearts, Range::at_least(4, LENGTH_CAP));
                    players[who].narrow_length(Suit::Spades, Range::at_least(4, LENGTH_CAP));
                }
            }
            players[who].narrow_points(Range::at_least(dont.floor, POINTS_CAP));
        }

        // A Meckwell overcall of their 1NT.  The two-way X (single 6+ minor OR both
        // majors) is a disjunction that shares no sound per-suit fact — the one-suiter
        // arm holds short majors, the both-majors arm long majors — so only the points
        // floor is recorded (as the Woolsey / penalty double).  The 2♣/2♦ pin the real
        // ≥4 minor (the natural ≥5 reading was suppressed); the unknown major surfaces
        // from the residual.  Natural 2♥/2♠ and the 2NT both-minors are read elsewhere.
        if let Some(meckwell) = meckwell {
            let who = relative_of(len, meckwell.overcall_index) as usize;
            match meckwell.kind {
                MeckwellKind::TwoWayDouble => {}
                MeckwellKind::ClubsMajor => {
                    players[who].narrow_length(Suit::Clubs, Range::at_least(4, LENGTH_CAP));
                }
                MeckwellKind::DiamondsMajor => {
                    players[who].narrow_length(Suit::Diamonds, Range::at_least(4, LENGTH_CAP));
                }
            }
            players[who].narrow_points(Range::at_least(meckwell.floor, POINTS_CAP));
        }

        // Our Gladiator advance: record the real shape the suppressed call hid.
        // Guarded to our own side (the advance is our agreement) — an opponent's
        // in-band call must never be narrowed to the phantom suit.
        if let Some(gladiator) = gladiator {
            let who = relative_of(len, gladiator.index);
            if matches!(who, Relative::Me | Relative::Partner) {
                let who = who as usize;
                match gladiator.advance {
                    // No band: the relay is a *three-way* disjunction — a weak
                    // ♦/`o` takeout, any invitational hand, **or a game-forcing
                    // balanced hand with exactly three `o`** heading for the
                    // delayed cue (`gladiator_advances`, the 2♣ rule; its
                    // continuation authors the delayed cue `points(inv..)`,
                    // unbounded).  A `0..=9` cap here was intersected into the
                    // projection's game-forcing box and emptied it — a wrong
                    // box, not a loose one.  The strength reading is the
                    // authored rule's own union of boxes, and the suit stays
                    // unread (the XYZ-style rebid over 2♦ reveals it, read
                    // naturally).
                    GladiatorAdvance::Relay => {}
                    GladiatorAdvance::Cue { o } => {
                        players[who].narrow_length(o, Range::at_least(4, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                    }
                    // Delayed cue: exactly 3 in the unbid major, INV+ (checks the
                    // 5-3 fit an exactly-5-major overcall can hold).
                    GladiatorAdvance::DelayedCue { o } => {
                        players[who].narrow_length(o, Range::new(3, 3));
                        players[who].narrow_points(Range::at_least(8, POINTS_CAP));
                    }
                    GladiatorAdvance::Splinter { o, m } => {
                        players[who].narrow_length(o, Range::at_least(4, LENGTH_CAP));
                        players[who].narrow_length(m, Range::new(0, 1));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    GladiatorAdvance::BothMinors => {
                        players[who].narrow_length(Suit::Clubs, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_length(Suit::Diamonds, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    GladiatorAdvance::Minor { o, minor } => {
                        players[who].narrow_length(o, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_length(minor, Range::at_least(5, LENGTH_CAP));
                        players[who].narrow_points(Range::at_least(10, POINTS_CAP));
                    }
                    // 2NT = weak transfer to clubs: 6+ clubs, sub-invitational.
                    GladiatorAdvance::ClubTransfer => {
                        players[who].narrow_length(Suit::Clubs, Range::at_least(6, LENGTH_CAP));
                        players[who].narrow_points(Range::new(0, 7));
                    }
                }
            }
        }

        // Our natural penalty double of their 1NT.  The shape gate only widens *which*
        // 15+ hands double, so only the points floor is a sound per-call fact; recording
        // it stops the floor sampling the doubler as a random weak hand and the advancer
        // pulling a phantom suit (cf. the Woolsey double, which records points alone too).
        if let Some(double_index) = penalty_x {
            let who = relative_of(len, double_index) as usize;
            let floor = crate::bidding::american::natural_double_floor();
            players[who].narrow_points(Range::at_least(floor, POINTS_CAP));
        }

        // The latch's later penalty doubles: four-plus in the doubled suit (the
        // floor makes them only on a trump stack), so partner reads them as penalty.
        for (double_index, suit) in penalty_latch_doubles {
            let who = relative_of(len, double_index) as usize;
            players[who].narrow_length(suit, Range::at_least(4, LENGTH_CAP));
        }

        // Responder's double of an overcall of our 1NT: 8+ values (every DoubleStyle).
        if let Some(double_index) = overcall_double {
            let who = relative_of(len, double_index) as usize;
            players[who].narrow_points(Range::at_least(8, POINTS_CAP));
        }

        // The vacuous-scoped probed overlay ([`set_probed_vacuous_reading`]):
        // own-side calls in *contested* prefixes only, folded onto axes every
        // symbolic source above — walk stamps, projections, and hand
        // recordings alike — left fully open.  It runs here, last, because
        // the mask must be judged against the complete symbolic reading (the
        // full fold's home, `project_authored`, runs before the walk stamps).
        // Latest call first: the longest prefix is the sharpest conditioning,
        // and once it fills an axis, earlier keys leave it alone.  The
        // contested gate scopes the fold to the measured coverage hole
        // (contested free bids the walk stamps nothing for): filling
        // constructive ⊤ axes moved ~23% of boards in the 2026-07-31 smoke,
        // all net-OOD grand blasts — the σ-shrink signature of the exclusion
        // retrain.  Under the full fold this is redundant — every probed box
        // already folded unmasked.
        if probed_vacuous_reading()
            && !probed_reading()
            && let Some(them) = context.their_system()
        {
            // The first index at which both sides have acted: keys through a
            // call before it read purely constructive traffic — not the hole.
            let contested_from = {
                let mut acted = [usize::MAX; 2];
                for (index, call) in auction.iter().enumerate() {
                    if *call != Call::Pass {
                        let side = &mut acted[index % 2];
                        if *side == usize::MAX {
                            *side = index;
                        }
                    }
                }
                acted[0].max(acted[1])
            };
            for index in (contested_from..len).rev() {
                if index % 2 != len % 2 {
                    continue;
                }
                let Some(&box_) = them.probed_box(&auction[..=index]) else {
                    continue;
                };
                let who = relative_of(len, index) as usize;
                let mut masked = Envelope::unknown();
                if players[who].strength.points == Range::FULL_POINTS {
                    masked.strength.points = box_.strength.points;
                }
                for suit in 0..4 {
                    if players[who].lengths[suit] == Range::FULL_LENGTH {
                        masked.lengths[suit] = box_.lengths[suit];
                    }
                }
                if masked != Envelope::unknown() {
                    players[who] = players[who].intersect(&masked);
                }
            }
        }

        Self::assemble(players, &overlay_dnf, &agreement_dnf, control_bid)
    }

    /// The last call the M6.4 classifier read as a control bid: its auction
    /// index and the agreed suit (see [`classify_high_bid`])
    #[must_use]
    pub(super) fn control_bid(&self) -> Option<(u8, Suit)> {
        self.control_bid
    }
}

/// Project the authored rule of every artificial prior call into [`Inferences`]
///
/// The generic dual of the per-convention `*_reading` decoders (M6.2b): walk the
/// authored nodes the context's trie carries ([`Context::prefixes`]) and, at each,
/// project the rule of the call actually made.  When that projection floors a suit
/// the call did not name, the call is *artificial* — a transfer, a two-suiter, a
/// Landy 2♣ — and its projected shape is recorded against the bidder's relative
/// seat, exactly as the hand-written readers do, but read straight off the rule.
///
/// A keyless context (no prefixes) or an all-natural auction leaves every seat at
/// [`Envelope::unknown`], so this is a sound, loose *overlay* — never the natural
/// reading itself (openings, raises, rebids stay in [`Inferences::read`]).
///
/// The projection in isolation: [`Inferences::read`] folds [`project_authored`]
/// directly, so this thin wrapper now serves only the M6.2b equivalence test.
#[cfg(test)]
#[must_use]
pub(crate) fn authored_reading(context: &Context<'_>) -> Inferences {
    let (dnf, announced, _) = project_authored(context);
    Inferences {
        players: std::array::from_fn(|i| dnf[i].hull()),
        announced_players: std::array::from_fn(|i| announced[i].hull()),
        dnf,
        announced,
        control_bid: None,
    }
}

/// How a four-plus-level suit bid in the slam zone reads (M6.4)
enum HighBid {
    /// To play (or an unreadable splinter) — suppressed from the natural
    /// walk, nothing recorded: the honest envelope is wide (a preempt, a
    /// two-suiter picture jump, a fast-arrival sign-off), and flooring a six
    /// here measurably rerouted 33-count hands into thin suit slams
    ToPlay,
    /// A control bid agreeing `trump`, most recently shown by seat `shower`
    Control { trump: Suit, shower: usize },
    /// Not the classifier's call — fall through to the generic walk
    Unclaimed,
}

/// Classify an unalerted suit bid at the four level or higher: control bid or
/// to play (M6.4)
///
/// The deterministic rule, calibrated to what this system actually bids: the
/// bid is a **control bid** iff the bidder *bypassed* the suit — it was
/// biddable more cheaply (same level, lower strain) at their first
/// suit-showing call and they chose another suit (`1♦–1♠–2♦–4♥`: 1♥ was
/// available under 1♠, so hearts are short and 4♥ agrees diamonds — the
/// partnership's most recently shown suit, BWS's priority).  A suit *above*
/// the first-shown one was never denied: both the book and the floor bid the
/// cheaper suit first holding a longer higher one (a 1♥ response or a heart
/// transfer on 6♠5♥ is real traffic — the first A/B bled six IMPs a fired
/// board pulling those natural 4♠s to the "agreed" minor), so it reads to
/// play: suppressed, but with nothing floored.
///
/// "Shown" folds the walk's floors so far with the projection overlay, so a
/// transferred suit counts for its transferee.  (The overlay is the
/// full-auction fold, so an artificial call *after* `index` could in principle
/// leak into the test; slam-zone auctions all but never continue artificially
/// after an unalerted four-level bid, and the leak can only re-label a control
/// bid — it never floors a phantom suit.)
#[allow(clippy::too_many_arguments)]
fn classify_high_bid(
    auction: &[Call],
    index: usize,
    bid: Bid,
    len: usize,
    opening_index: usize,
    players: &[Envelope; 4],
    overlay: &[Envelope; 4],
    suppressed_so_far: u64,
) -> HighBid {
    let Some(suit) = bid.strain.suit() else {
        return HighBid::Unclaimed;
    };
    let who = relative_of(len, index) as usize;
    let partner = (who + 2) % 4;
    let shown =
        |seat: usize, s: Suit| players[seat].length(s).min >= 4 || overlay[seat].length(s).min >= 4;

    // Rebids of one's own suit and raises of partner's stay with the generic
    // walk (six-plus / support) — both are to play.
    if shown(who, suit) || shown(partner, suit) {
        return HighBid::Unclaimed;
    }

    if !Suit::ASC.into_iter().any(|s| s != suit && shown(who, s)) {
        // The bidder has shown nothing: the suit can be their longest — to
        // play (which covers the possible splinter below game in partner's
        // major, `1♥–4♣`, since nothing is recorded either way).
        return HighBid::ToPlay;
    }

    // The bidder's first suit-showing call: its shown suit `r` and level — a
    // natural bid's own suit, or an artificial call's *projected* one (the
    // transfer's major, not its named diamond; the fold of the seat's floors
    // stands in for the per-call projection, a recency approximation).  Track
    // the highest bid standing before it for the bypass legality test.
    let mut first_shown: Option<(Suit, u8)> = None;
    let mut highest_before: Option<Bid> = None;
    for (j, &call) in auction.iter().enumerate().take(index).skip(opening_index) {
        let Call::Bid(prior) = call else {
            continue;
        };
        if j % 4 == index % 4 {
            let shown_suit = if j < 64 && suppressed_so_far >> j & 1 != 0 {
                Suit::ASC
                    .into_iter()
                    .filter(|&s| overlay[who].length(s).min >= 4)
                    .max_by_key(|&s| (overlay[who].length(s).min, s as u8))
            } else {
                prior.strain.suit()
            };
            if let Some(r) = shown_suit {
                first_shown = Some((r, prior.level.get()));
                break;
            }
        }
        if highest_before.is_none_or(|h| outranks(prior, h)) {
            highest_before = Some(prior);
        }
    }
    let Some((r, r_level)) = first_shown else {
        return HighBid::Unclaimed;
    };

    // The longer-major response discipline (`set_longer_major_response`)
    // swaps two verdicts when the bidder's first call was a one-level major
    // response to partner's minor opening: a 1♥ response denies longer
    // spades, so a later spade bid *is* a bypass (control) even though it
    // sits above the response; and a 1♠ response may conceal equal-length
    // five-plus hearts (5-5 responds 1♠), so the skipped 1♥ no longer proves
    // shortness (to play).
    let response_to_partners_minor = r_level == 1
        && relative_of(len, opening_index) as usize == partner
        && matches!(auction[opening_index], Call::Bid(opening)
            if opening.level.get() == 1
                && matches!(opening.strain.suit(), Some(Suit::Clubs | Suit::Diamonds)));
    let discipline =
        response_to_partners_minor && crate::bidding::american::longer_major_response();

    // Bypassed: the bid suit sat below the first-shown suit at the same level
    // and the bidder skipped it — it cannot be long, so this is a control bid.
    // Otherwise the suit was never denied and reads to play.
    let bypassed = match (discipline, r, suit) {
        (true, Suit::Hearts, Suit::Spades) => true,
        (true, Suit::Spades, Suit::Hearts) => false,
        _ => {
            (suit as u8) < (r as u8)
                && highest_before.is_none_or(|h| outranks(Bid::new(r_level, bid.strain), h))
        }
    };
    if !bypassed {
        return HighBid::ToPlay;
    }

    // The agreed suit: the partnership's most recently shown one.
    for j in (opening_index..index).rev() {
        if j % 2 != index % 2 {
            continue; // the opponents' calls agree nothing for us
        }
        let Call::Bid(prior) = auction[j] else {
            continue;
        };
        let seat = relative_of(len, j) as usize;
        if j < 64 && suppressed_so_far >> j & 1 != 0 {
            let candidate = Suit::ASC
                .into_iter()
                .filter(|&s| {
                    s != suit
                        && (overlay[seat].length(s).min >= 4 || players[seat].length(s).min >= 4)
                })
                .max_by_key(|&s| {
                    (
                        overlay[seat].length(s).min.max(players[seat].length(s).min),
                        s as u8,
                    )
                });
            if let Some(trump) = candidate {
                return HighBid::Control {
                    trump,
                    shower: seat,
                };
            }
        } else if let Some(s) = prior.strain.suit()
            && s != suit
        {
            return HighBid::Control {
                trump: s,
                shower: seat,
            };
        }
    }
    HighBid::Unclaimed
}

/// Project every artificial prior call into a per-seat overlay, plus a bitset of the
/// artificial calls' auction positions
///
/// The shared walk behind both halves of the retired declarative readers, folded
/// into [`Inferences::read`] (M6.2c): the overlay *records* each artificial call's
/// projected shape against the bidder's seat, and the bitset marks which calls to
/// *suppress* from the natural single-suit reading.  A call is artificial when its
/// projection floors a suit it did not name (see [`artificial`]).
///
/// The bitset indexes by auction position; a position past 64 (never reached by a
/// real auction) is simply left unmarked, falling back to the natural reading.
/// Combine the final hulled `players` with the disjunctive `overlay` into the
/// per-seat DNF the sampler consumes
///
/// `players[i]` already folds `overlay[i].hull()` and every hand-walk narrowing,
/// and each overlay box is `⊆` that hull, so re-intersecting recovers exactly
/// `⋃(hand-walk ∩ boxₖ)` — the tight union — while dropping boxes the walk
/// contradicts.  With [`dnf_reading`] off each overlay is one box, so the result
/// is the single box `players[i]` and `dnf[i].hull() == players[i]` (byte-identical).
fn dnf_of(players: &[Envelope; 4], overlay: &[Dnf; 4]) -> [Dnf; 4] {
    std::array::from_fn(|i| Dnf::from(players[i]).intersect(&overlay[i]))
}

/// One table's pass reading: the union of its Pass rules' bands, knob-on
/// intersected with the complements of the sibling gates the passer declined
///
/// The exclusion half (see [`set_pass_exclusion_reading`]) leans on argmax
/// selection: a hand inside a sibling gate whose weight strictly beats
/// **every** Pass rule's weight could not have let Pass win, so the passer
/// lies in that gate's complement.  Single-box complements only — the
/// shape-free tiers; skipping the rest costs precision, never soundness, and
/// holds the box count down.  [`None`] when the table authors no Pass rule at
/// all (the projection pass then records nothing, as before).
fn project_pass(rules: &super::rules::Rules, ctx: &Context<'_>) -> Option<Dnf> {
    let band = rules
        .rules()
        .iter()
        .filter(|rule| rule.call() == Call::Pass)
        .map(|rule| rule.project_band_dnf(ctx))
        .reduce(Dnf::disjoin)?;
    if !pass_exclusion_reading() {
        return Some(band);
    }
    let ceiling = rules
        .rules()
        .iter()
        .filter(|rule| rule.call() == Call::Pass)
        .map(super::rules::Rule::weight)
        .fold(f32::NEG_INFINITY, f32::max);
    Some(
        rules
            .rules()
            .iter()
            .filter(|rule| rule.call() != Call::Pass && rule.weight() > ceiling)
            .map(|rule| rule.project_complement_dnf(ctx))
            .filter(|complement| {
                complement.boxes().len() == 1 && complement.boxes()[0] != Envelope::unknown()
            })
            .fold(band, |acc, complement| acc.intersect(&complement)),
    )
}

fn project_authored(context: &Context<'_>) -> ([Dnf; 4], [Dnf; 4], u64) {
    let auction = context.auction();
    let len = auction.len();
    let mut players: [Dnf; 4] = std::array::from_fn(|_| Dnf::unknown());
    // The agreement overlay, folded in lockstep with `players` off
    // `Rule::announce_dnf`.  Knob-off it is never read separately — the caller
    // gets a clone of `players` — so the second reduce below costs nothing.
    let mut announced: [Dnf; 4] = std::array::from_fn(|_| Dnf::unknown());
    let mut suppressed = 0u64;

    let Some(prefixes) = context.prefixes() else {
        return (players.clone(), players, suppressed);
    };

    let read_passes = pass_reading();
    let announce_split = announced_reading();

    // Project the call made at `index`, authored by `classifier`, into the
    // overlay, evaluating its rules under `ctx` — always the bidder's
    // at-the-time context, for our own pair's calls as much as for
    // table-decoded opponent calls (see the `at_the_time` builder below).
    // `decode_pass` scopes the pass reading to the side whose
    // book resolved `classifier`: the own-side loops resolve *every* index in
    // the reader's trie, where an opponent's pass would land on the wrong
    // node (their direct-seat pass over our opening resolves our *responses*
    // table) — alerts are shielded by the alert filter, passes need the
    // explicit side gate.
    let mut project_call = |ctx: &Context<'_>,
                            index: usize,
                            classifier: &dyn super::trie::Classifier,
                            decode_pass: bool| {
        let (Some(&made), Some(rules)) = (auction.get(index), classifier.as_rules()) else {
            return;
        };

        let is_pass = made == Call::Pass;
        // The logit of a call is the max over its rules, so a hand could satisfy
        // any one of them — the sound forward envelope is their union.  A made
        // bid reads by what it promises (`project`, floors); a pass reads by
        // what its gate would have *allowed* (`project_band`, both bounds) —
        // the negative inference of declining every other call, which is what
        // the author wrote the table's Pass gate to document.
        let projection = if is_pass {
            project_pass(rules, ctx)
        } else {
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == made && rule.face_live(ctx))
                .map(|rule| rule.project_dnf(ctx))
                .reduce(Dnf::disjoin)
        };

        // A call is artificial — decode it — when its authoring rule *alerts* it.
        // The alert is now the complete, exhaustive signal: every artificial call
        // in the book carries one (guarded by the `artificial_calls_are_alerted`
        // invariant test), so the old structural `artificial(p, made)` fallback
        // has been retired (alert-by-disclosed-meaning, the move modern bridge
        // made retiring "X is self-alerting").  A pass is natural-by-default and
        // never alerted; it decodes on the pass-reading knob instead.
        let alerted = !is_pass
            && rules
                .rules()
                .iter()
                .any(|rule| rule.call() == made && rule.alert().is_some() && rule.face_live(ctx));
        let decode = if is_pass {
            decode_pass
        } else {
            // An *unalerted* authored call is natural, so it is read by the
            // natural walk — and the rule that produced it says nothing, which
            // is how a rule and its reading drift apart (the regime-2 class of
            // `docs/reading-drift-handoff.md`).  Knob-on, project it too: the
            // union is the same sound one, but the suppression bit below stays
            // clear, so the projection *adds to* the walk's natural reading
            // instead of replacing it.
            (alerted && alert_reading()) || natural_reading()
        };

        if let Some(projection) = projection.filter(|_| decode) {
            let who = relative_of(len, index) as usize;
            // The agreement, where it differs.  `announce` defaults to
            // `project`, so knob-off — and at every rule that never called
            // `announced` — this is the same union and the two overlays stay
            // identical.  A pass reads by its *band*, which `announce` does not
            // split.
            //
            // The union is over the **alerted** rules alone, and that is the
            // second half of the split.  A projection must cover every rule that
            // shares the bid, because any of them could have produced it and the
            // sampler may not exclude a hand one of them accepts — the floor's
            // 4NT keycard ask sits on the same call as an unalerted weight-0.3
            // catch-all, whose ⊤ would union the agreement away.  But disclosure
            // does not work like that: an alerted call is explained as *the
            // convention*, not as the residue sharing its bid.  An unalerted
            // call reaching here on `natural_reading()` has nothing to announce
            // beyond what it projects, and the `unwrap_or_else` catches it.
            let agreement = if announce_split && !is_pass {
                rules
                    .rules()
                    .iter()
                    .filter(|rule| {
                        rule.call() == made && rule.alert().is_some() && rule.face_live(ctx)
                    })
                    .map(|rule| rule.announce_dnf(ctx))
                    .reduce(Dnf::disjoin)
                    .unwrap_or_else(|| projection.clone())
            } else {
                projection.clone()
            };
            announced[who] = announced[who].intersect(&agreement);
            players[who] = players[who].intersect(&projection);
            // A pass suppresses nothing — it never had a natural suit reading.
            // Neither does an unalerted call read only because `natural_reading`
            // is on: it *is* natural, and suppressing it would delete the walk's
            // lane bookkeeping (natural-suit masks, agreed fits, cue detection)
            // that later calls read from.
            if alerted && index < 64 {
                suppressed |= 1 << index;
            }
        }
    };
    // `players` is a `[Dnf; 4]` closed over by `project_call`; every branch below
    // accumulates through it, and the caller hulls each entry for the natural walk.

    // A rule's constraint is a claim about the moment its call was made, so it
    // must project under the bidder's **at-the-time** context — exactly as the
    // table-alert and pass walks below already do.  Projecting under the
    // reader's full-auction context mis-resolved every auction-relative atom:
    // `support(3..)` reads `partner_last_suit()`, and the reader's "partner's
    // last bid" *is the projected call itself*, so a cue raise's support atom
    // stamped n+ cards of the **cue suit** — a wrong box that excluded the
    // bidder (probe-reading-sound: `[1♥ 1♠ 2♠]` 8/15, `[1♣ 1♦ 2♦]` 4/4).  The
    // same skew put the support double's 3-card claim on opener's own suit.
    // For plain raises the two contexts happen to agree (the raise suit is the
    // support suit), which is how this survived the raise-reader sweeps.
    let at_the_time = |index: usize| {
        let vul = if index % 2 == len % 2 {
            context.vul()
        } else {
            super::context::flipped(context.vul())
        };
        Context::new(vul, &auction[..index])
    };

    if fallback_projection_enabled() {
        // Decode every prior call by the classifier that *authored* it — node or
        // guarded fallback — so contested conventions (transfers, Leaping Michaels,
        // the Lebensohl cue) survive later competition without a per-convention reader.
        let trie = prefixes.root();
        for index in 0..len {
            if let Some(classifier) = trie.authoring_classifier(context, &auction[..index]) {
                project_call(&at_the_time(index), index, classifier, false);
            }
        }
    } else {
        // Exact-node classifiers only (fallback projection, the shipped
        // default, takes the branch above); fallback-authored conventions are
        // then read by the hand-written readers in [`Inferences::read`].
        for (prefix, classifier) in prefixes.clone() {
            project_call(&at_the_time(prefix.len()), prefix.len(), classifier, false);
        }
    }

    // Alerts are table-wide disclosure: the opponents' alerted calls are
    // explained to us, so decode them too — each resolved in *their*
    // phase-routed book (the attached stance models them as playing our own
    // books) under their at-the-time context, exactly as it was classified
    // when made.  Their unalerted (natural) calls keep the natural walk.
    if table_alert_reading()
        && let Some(them) = context.their_system()
    {
        let vul = super::context::flipped(context.vul());
        for index in ((len + 1) % 2..len).step_by(2) {
            let prefix = &auction[..index];
            let their_context = Context::new(vul, prefix);
            if let Some(classifier) = them
                .trie_for(prefix)
                .authoring_classifier(&their_context, prefix)
            {
                project_call(&their_context, index, classifier, false);
            }
        }
    }

    // Passes decode in a dedicated walk: a pass's authoring node lives in the
    // phase of *its* turn (a pre-opening pass belongs to the opening table
    // even after the auction goes defensive), so each resolves via the
    // slice-relative `trie_for` on the auction cut at its index, under its
    // at-the-time context — the reader's own side always, the opponents' only
    // under table-wide disclosure.  The attached stance models both sides with
    // the same books (`their_system` is the reader's own stance), so it serves
    // own-side resolution too.
    if read_passes && let Some(them) = context.their_system() {
        let their_vul = super::context::flipped(context.vul());
        for index in 0..len {
            if auction[index] != Call::Pass {
                continue;
            }
            let own_side = index % 2 == len % 2;
            if !own_side && !table_alert_reading() {
                continue;
            }
            let prefix = &auction[..index];
            let vul = if own_side { context.vul() } else { their_vul };
            let at_the_time = Context::new(vul, prefix);
            if let Some(classifier) = them
                .trie_for(prefix)
                .authoring_classifier(&at_the_time, prefix)
            {
                project_call(&at_the_time, index, classifier, true);
            }
        }
    }

    // The probed overlay (see [`set_probed_reading`]): fold each prior call's
    // behaviorally measured box, keyed by the prefix through it.  Runs last so
    // it composes with — never replaces — the symbolic folds above; an empty
    // probed map is a no-op.  The vacuous-scoped variant lives in
    // [`Inferences::read`] instead: its fill-only mask must be judged against
    // the *complete* symbolic reading, and the natural walk stamps after this
    // returns.
    if probed_reading()
        && let Some(them) = context.their_system()
    {
        for index in 0..len {
            if let Some(&box_) = them.probed_box(&auction[..=index]) {
                let who = relative_of(len, index) as usize;
                let dnf = Dnf::from(box_);
                players[who] = players[who].intersect(&dnf);
                announced[who] = announced[who].intersect(&dnf);
            }
        }
    }

    (players, announced, suppressed)
}

/// Whether a call's projection floors a suit other than the one it names
///
/// The structural artificial-call detector, falling out of the projection itself:
/// a natural call floors its own strain (1♠ → 5+♠) or no suit (1NT → points only);
/// an artificial one floors a suit it did not name (Jacoby 2♦ → 5+♥, Landy 2♣ →
/// 4-4 majors).  A min-length floor of four-plus on a non-named suit is the witness
/// — above any natural by-product, below every convention's real shape.
///
/// **The "named" suit generalizes past bids.**  A call is natural when it offers to
/// play what it declares, artificial when it points partner at some *other* suit:
/// - a **bid** names its own strain;
/// - a **double / redouble** names the *doubled strain* — the contract it offers to
///   defend.  A penalty double floors that strain (or nothing) → natural; a takeout
///   double floors an *unbid* suit (support for where it sends partner) → artificial;
/// - a **pass** redirects from nothing → never artificial (a trap pass defends what
///   is on the table);
/// - a **transfer completion** names the suit it will play, flooring no other →
///   natural, so already `false`.
///
/// This is a *sound sufficient* witness, not a complete one: a takeout double whose
/// authoring rule floors nothing (opaque shape predicates, e.g. the direct takeout
/// double) reads `false` here though it is takeout by meaning — such calls are
/// classified artificial by their `.alert(...)` instead, exactly as shape-only
/// artificial bids are.
///
/// **Retired from the decode gate** — alerts now carry the signal exhaustively.
/// This survives test-only, as the `artificial_calls_are_alerted` invariant guard:
/// any future artificial call added without an `.alert(...)` must fail that test
/// rather than silently lose its decoding.
#[cfg(test)]
fn artificial(projection: &Envelope, made: Call, doubled: Option<Strain>) -> bool {
    // The "named" suit is the one the call offers to play: a bid names its own
    // strain; a double/redouble "names" the doubled strain it offers to defend.
    // Artificial = the projection floors some *other* suit ≥4 — the call is really
    // pointing partner at a suit it did not name.  A pass redirects from nothing.
    let named = match made {
        Call::Bid(bid) => bid.strain.suit(),
        Call::Double | Call::Redouble => doubled.and_then(|strain| strain.suit()),
        Call::Pass => return false,
    };
    Suit::ASC
        .into_iter()
        .any(|suit| Some(suit) != named && projection.length(suit).min >= 4)
}

/// Whether the call at `index` is an artificial relay/puppet/splinter in the
/// minor-suit-response structure over our 1NT opening — so it must not be read as a
/// natural long suit
///
/// Once responder enters a structure as their first call, every later three-level
/// suit bid by our side is an artificial relay or splinter.  Which first calls
/// enter, and the lone exception, depend on the active minor scheme
/// ([`notrump_minors`][crate::bidding::american::notrump_minors]):
///
/// - **Puppet:** 3♣ Puppet, 2NT diamond transfer, or 2♠ two-way relay — except
///   opener's genuine five-card major show over Puppet (`1NT–3♣–3♥/3♠`).
/// - **European:** 2♠ (clubs) or 3♣ (diamonds) transfer — every continuation
///   (opener's completion, responder's splinter) is a relay, no exception; the
///   natural 2NT invite enters nothing.
///
/// Positions assume the standard uncontested auction; a contested one shifts them
/// and matches none.
fn nt_structure_artificial(auction: &[Call], index: usize, opening_index: usize) -> bool {
    let resp_first = auction.get(opening_index + 2);

    if crate::bidding::american::notrump_minors() == crate::bidding::american::EUROPEAN {
        // European: 2♠ (clubs) and 3♣ (diamonds) are transfers; every suit bid in
        // their continuations is a relay, never a natural suit.
        return matches!(
            resp_first,
            Some(&Call::Bid(b))
                if b == Bid::new(2, Strain::Spades) || b == Bid::new(3, Strain::Clubs)
        );
    }

    let entered = matches!(
        resp_first,
        Some(&Call::Bid(b))
            if b == Bid::new(3, Strain::Clubs)
                || b == Bid::new(2, Strain::Notrump)
                || b == Bid::new(2, Strain::Spades)
    );
    if !entered {
        return false;
    }
    // Opener's natural five-card major show over Puppet stays a real suit.
    let opener_puppet_major = index == opening_index + 4
        && resp_first == Some(&Call::Bid(Bid::new(3, Strain::Clubs)))
        && matches!(
            auction.get(index),
            Some(&Call::Bid(b))
                if b.level.get() == 3 && matches!(b.strain, Strain::Hearts | Strain::Spades)
        );
    !opener_puppet_major
}

/// Whether `bid` is higher than the standing `highest` contract
///
/// Bridge contracts rank by level first, then strain — `2♣` outranks `1♠`.
fn outranks(bid: Bid, highest: Bid) -> bool {
    bid.level.get() > highest.level.get()
        || (bid.level.get() == highest.level.get() && bid.strain > highest.strain)
}

/// The cheapest level a strain can be bid over the standing `highest` contract
const fn cheapest_level(highest: Option<Bid>, strain: Strain) -> u8 {
    match highest {
        None => 1,
        Some(h) if strain as u8 > h.strain as u8 => h.level.get(),
        Some(h) => h.level.get() + 1,
    }
}

/// The Rubens-artificial calls of an advance, and the advance's strength reading
///
/// In a [Rubens advance][super::instinct::overcall_shape] of a simple overcall,
/// some calls *name* a suit they do not *hold*: the advancer's transfer (a relay
/// to the next suit up) or cue-raise, and the overcaller's forced completion.
/// Returns `(suppress, cue, transfer)` — `suppress` lists those indices, whose
/// bid suit must not be read as natural length; `cue` is `(index, Y)` of a
/// two-level cue-raise, read separately as a limit-plus raise (three-plus cards
/// in partner's overcall `Y`, ten-plus points); `transfer` is
/// `(index, suit, min-len)` of a one-level transfer's meaning — the transfer
/// into partner's suit is the same limit-plus raise (`(index, Y, 3)`), a
/// new-suit transfer shows its own five-card target (`(index, target, 5)`),
/// both ten-plus points ([`set_rubens_transfer_reading`], recorded post-walk
/// for the advancer's *own side only* — an opponent's in-band advance may be
/// natural).
///
/// The shown values are what let the overcaller judge game — and the completion
/// is a forced relay, still never read as length (soundness over tightness, as
/// with transfers over our own notrump).
#[allow(clippy::type_complexity)]
fn rubens_reading(
    auction: &[Call],
) -> (
    [Option<usize>; 2],
    Option<(usize, Suit)>,
    Option<(usize, Suit, u8)>,
) {
    let none = ([None, None], None, None);
    // The bidder's knob governs the reading too: with Rubens advances off, an
    // advance in the band is a genuine suit and must be read naturally.
    if !super::instinct::rubens_advances_enabled() {
        return none;
    }
    let Some((x, y, overcall_index, level)) = super::instinct::overcall_shape(auction) else {
        return none;
    };
    // The advance comes after the overcaller's partner (RHO of the overcaller)
    // passes; the advancer's call sits two past the overcall.
    if auction.get(overcall_index + 1) != Some(&Call::Pass) {
        return none;
    }
    let advance_index = overcall_index + 2;
    let Some(&Call::Bid(advance)) = auction.get(advance_index) else {
        return none;
    };
    if level == 2 {
        // Two-level overcall: the cue-raise (2X) is the lone artificial call.
        return if advance == Bid::new(2, Strain::from(x)) {
            ([Some(advance_index), None], Some((advance_index, y)), None)
        } else {
            none
        };
    }
    // One-level overcall: a transfer 2S (X ≤ S < Y), then the completion 2(S+1).
    let Some(source) = advance.strain.suit() else {
        return none;
    };
    if advance.level.get() != 2 || (source as u8) < (x as u8) || (source as u8) >= (y as u8) {
        return none;
    }
    let target_suit = Suit::ASC[(source as u8 + 1) as usize];
    let target = Strain::from(target_suit);
    // The overcaller completes through opener's lead-directing double too, so
    // the completion stays a relay (never a holding) in both shapes.
    let completion = (matches!(
        auction.get(advance_index + 1),
        Some(Call::Pass | Call::Double)
    ) && auction.get(advance_index + 2) == Some(&Call::Bid(Bid::new(2, target))))
    .then_some(advance_index + 2);
    // The transfer's meaning, fixed the moment it is made (the completion is
    // not required): into partner's suit = the limit-plus raise, a new suit =
    // the advancer's own five-card target.
    let transfer = rubens_transfer_reading().then_some(if target_suit == y {
        (advance_index, y, 3)
    } else {
        (advance_index, target_suit, 5)
    });
    ([Some(advance_index), completion], None, transfer)
}

/// The advancer's `2♦` relay / `2♥`-`2♠` preference over a Landy/Woolsey both-majors
/// `2♣`, whose natural single-suit reading is suppressed
///
/// The one suppression the projection pass cannot supply: a relay names no length of
/// its own, so its authored rule projects nothing and the artificial detector (which
/// drives the rest of the suppression now, M6.2c) misses it.  The `2♣` overcall
/// itself, and every other retired convention's shape, are read straight off their
/// projected rule; this is the lone hand stub the doc keeps.
///
/// `None` unless Landy or Woolsey is on *and* the defending side's first action over
/// their `1NT` was the both-majors `2♣`, so a natural `2♣` is never mistaken for it.
// ponytail: a relay projects no info, so suppress it by hand; the upgrade path is to
// author the relay's rule with the negated lengths so the detector catches it too.
fn landy_advance_suppress(auction: &[Call]) -> Option<usize> {
    let on = crate::bidding::american::landy_range().is_some()
        || crate::bidding::american::woolsey_enabled();
    if !on {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The both-majors 2♣ must be the defending side's first action.
    let overcall_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => {
                Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
            }
            // The opener answered, or a defender did something else — not a 2♣ Landy.
            _ => Some(None),
        })
        .flatten()?;

    advancer_artificial(auction, overcall_index, opener_parity)
}

/// The index of the advancer's first `2♦`/`2♥`/`2♠` response over a both-majors /
/// Multi overcall at `overcall_index` — a relay or a preference among partner's
/// suits, never own length, so its natural reading is suppressed
///
/// The scan jumps over *every* opponent call (pass, double, or a competing suit
/// bid), so a quiet advance and a doubled / contested runout are all covered: a
/// `2♦`/`2♥`/`2♠` is only legal as the *immediate* response (once the auction climbs
/// past `2♠` it can never recur), so the first such call we find is always the
/// preference, whatever the opponents did.  Suppression is sound regardless — it only
/// ever *removes* a possibly-false length, never asserts one.  The suppression then
/// lives for the whole `Inferences::read`.  `None` if our first response was instead
/// an ask (`2NT`) or a genuine raise.
fn advancer_artificial(
    auction: &[Call],
    overcall_index: usize,
    opener_parity: usize,
) -> Option<usize> {
    auction
        .iter()
        .enumerate()
        .skip(overcall_index + 1)
        // Stop at our first *bid* (decide there); jump over everything the opponents do.
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => Some(
                matches!(
                    bid,
                    b if b == Bid::new(2, Strain::Diamonds)
                        || b == Bid::new(2, Strain::Hearts)
                        || b == Bid::new(2, Strain::Spades)
                )
                .then_some(index),
            ),
            _ => None,
        })
        .flatten()
}

/// Which Woolsey **Multi-family** overcall the defending side made over their 1NT
#[derive(Clone, Copy)]
enum MultiKind {
    /// `2♦` Multi — a single 6+ major (unknown which), nothing else long.  Names a
    /// diamond suit it does not hold, so its natural reading must be suppressed.
    Major,
    /// `2♥`/`2♠` Muiderberg — exactly 5 in the named major, ≤ 3 in the other major
    /// (and a 4+ minor, captured by the residual).  A real major: no suppression.
    Muiderberg(Suit),
}

/// A Woolsey Multi-family overcall and which call it was
#[derive(Clone, Copy)]
struct MultiReading {
    overcall_index: usize,
    kind: MultiKind,
    /// The advancer's `2♥`/`2♠` pass-or-correct over the Multi `2♦` (a preference
    /// among partner's unknown major — not own length), suppressed if present.
    advance_suppress: Option<usize>,
}

impl MultiReading {
    /// Whether the call at `index` is artificial: the `2♦` Multi naming diamonds it
    /// does not hold, or the advancer's `2♥`/`2♠` pass-or-correct (a preference, not
    /// own length).  The Muiderberg `2♥`/`2♠` overcall names a real 5-card major, so
    /// its natural reading is kept.
    fn suppresses(&self, index: usize) -> bool {
        (matches!(self.kind, MultiKind::Major) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a Woolsey **Multi-family** overcall of their 1NT: the `2♦` Multi (a single
/// 6+ major) or the `2♥`/`2♠` Muiderberg (exactly 5 in the major + a 4+ minor)
///
/// Gated on [`woolsey_enabled`][crate::bidding::american::woolsey_enabled] and the
/// auction being `1NT` then the defending side's first action being that bid.  The
/// both-majors `2♣` is read off its authored rule by the projection pass folded
/// into [`Inferences::read`] (Woolsey = Landy 2♣ + this family).
///
/// ponytail: kept separate so this Multi reading is reusable for a future Multi `2♦`
/// *opening* (an unknown-major weak two) — same shape, no 1NT prefix.
fn multi_reading(auction: &[Call]) -> Option<MultiReading> {
    if !crate::bidding::american::woolsey_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The defending side's FIRST action — a 2♦/2♥/2♠ Multi-family overcall.
    let reading = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Diamonds) {
                    Some(MultiKind::Major)
                } else if bid == Bid::new(2, Strain::Hearts) {
                    Some(MultiKind::Muiderberg(Suit::Hearts))
                } else if bid == Bid::new(2, Strain::Spades) {
                    Some(MultiKind::Muiderberg(Suit::Spades))
                } else {
                    None
                };
                Some(kind.map(|kind| MultiReading {
                    overcall_index: index,
                    kind,
                    advance_suppress: None,
                }))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // Over the Multi 2♦, the advancer's 2♥/2♠ pass-or-correct picks one of partner's
    // unknown majors — a preference, not own length — so suppress it too (including a
    // doubled runout; the shared helper handles both).
    let advance_suppress = matches!(reading.kind, MultiKind::Major)
        .then(|| advancer_artificial(auction, reading.overcall_index, opener_parity))
        .flatten();

    Some(MultiReading {
        advance_suppress,
        ..reading
    })
}

/// Our **Gladiator** advance of a 1NT overcall of their major
/// ([`set_nt_overcall_gladiator`][crate::bidding::american::set_nt_overcall_gladiator])
///
/// The advancer's artificial calls under `[1M, 1NT, P, ?]` — the `2♣` relay (and
/// its forced `2♦` completion), the cue of their major (Stayman for the unbid
/// major), the `3M` splinter, and the `4M` both-minor Leaping Michaels — are bids
/// of a suit the caller does *not* hold; the natural walk would floor a phantom
/// suit.  Their indices are suppressed and the real shape recorded post-walk.  The
/// natural advances (`2♦`/`2O`, the 3-level naturals, `4O`) read off the walk and
/// never enter here.
#[derive(Clone, Copy)]
enum GladiatorAdvance {
    /// `2♣` relay (weak / invitational) — no sound per-suit floor.
    Relay,
    /// Cue of their major = Stayman: 4+ in the unbid major `o`, INV+.
    Cue { o: Suit },
    /// Delayed cue (`2♣` relay → forced `2♦` → cue of their major): exactly 3 in
    /// the unbid major `o`, INV+ — the 5-3-fit check.
    DelayedCue { o: Suit },
    /// `3M` splinter: 4+ `o`, 0–1 in their major `m`, GF.
    Splinter { o: Suit, m: Suit },
    /// `4M` Leaping Michaels: both minors 5+, GF.
    BothMinors,
    /// `4♣`/`4♦` Leaping Michaels: 5+ `o` + 5+ the named `minor`, GF.
    Minor { o: Suit, minor: Suit },
    /// `2NT`: a weak transfer to clubs (6+♣) — not a balanced notrump.
    ClubTransfer,
}

#[derive(Clone, Copy)]
struct GladiatorReading {
    /// Index of the advancer's Gladiator call
    index: usize,
    advance: GladiatorAdvance,
    /// Bitset of indices whose natural suit reading the walk must skip
    suppress: u64,
}

impl GladiatorReading {
    const fn suppresses(self, index: usize) -> bool {
        index < 64 && self.suppress >> index & 1 != 0
    }
}

fn gladiator_reading(auction: &[Call]) -> Option<GladiatorReading> {
    if !crate::bidding::american::nt_overcall_gladiator() {
        return None;
    }
    let open = auction.iter().position(|&c| c != Call::Pass)?;
    let Call::Bid(opening) = auction[open] else {
        return None;
    };
    let m = opening.strain.suit()?;
    if opening.level.get() != 1 || !matches!(m, Suit::Hearts | Suit::Spades) {
        return None;
    }
    // Our 1NT overcall, then the advancer.  RHO usually passes; over RHO's (2♣)
    // systems-on overcall we mirror the book rebase — their 2♣ maps to a pass and
    // advancer's Double to the stolen 2♣ relay — and re-read, so every (2♣)
    // continuation (relay, delayed cue, cue-Stayman, club transfer) decodes
    // through the uncontested logic below with the same call indices.  Any other
    // RHO action leaves it to the natural walk.
    if auction.get(open + 1) != Some(&Call::Bid(Bid::new(1, Strain::Notrump))) {
        return None;
    }
    if auction.get(open + 2) == Some(&Call::Bid(Bid::new(2, Strain::Clubs))) {
        let mut stripped = auction.to_vec();
        stripped[open + 2] = Call::Pass;
        if auction.get(open + 3) == Some(&Call::Double) {
            stripped[open + 3] = Call::Bid(Bid::new(2, Strain::Clubs));
        }
        return gladiator_reading(&stripped);
    }
    if auction.get(open + 2) != Some(&Call::Pass) {
        return None;
    }
    let index = open + 3;
    let Some(&Call::Bid(bid)) = auction.get(index) else {
        return None;
    };
    let o = if m == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };

    // `index ≤ 6` (at most three leading passes), so the shifts never overflow.
    let mut suppress = 0u64;
    let advance = if bid == Bid::new(2, Strain::Clubs) {
        suppress |= 1 << index;
        // The overcaller's forced 2♦ completion (relay, P, 2♦) says nothing of
        // diamonds — suppress it too.
        let mut delayed = false;
        if auction.get(index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Diamonds))) {
            suppress |= 1 << (index + 2);
            // Delayed cue at index+4 (relay, P, 2♦, P, cue-of-their-major): a
            // phantom-suit call too (advancer holds exactly 3 `o`, not `m`).
            if auction.get(index + 4) == Some(&Call::Bid(Bid::new(2, opening.strain))) {
                suppress |= 1 << (index + 4);
                delayed = true;
            }
        }
        if delayed {
            GladiatorAdvance::DelayedCue { o }
        } else {
            GladiatorAdvance::Relay
        }
    } else if bid == Bid::new(2, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::Cue { o }
    } else if bid == Bid::new(3, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::Splinter { o, m }
    } else if bid == Bid::new(4, opening.strain) {
        suppress |= 1 << index;
        GladiatorAdvance::BothMinors
    } else if bid == Bid::new(2, Strain::Notrump) {
        suppress |= 1 << index;
        // The overcaller's forced 3♣ transfer completion says nothing of clubs.
        if auction.get(index + 2) == Some(&Call::Bid(Bid::new(3, Strain::Clubs))) {
            suppress |= 1 << (index + 2);
        }
        GladiatorAdvance::ClubTransfer
    } else if bid == Bid::new(4, Strain::Clubs) {
        GladiatorAdvance::Minor {
            o,
            minor: Suit::Clubs,
        }
    } else if bid == Bid::new(4, Strain::Diamonds) {
        GladiatorAdvance::Minor {
            o,
            minor: Suit::Diamonds,
        }
    } else {
        return None;
    };

    Some(GladiatorReading {
        index,
        advance,
        suppress,
    })
}

/// Our Woolsey takeout **double** of their 1NT and the advancer's `2♣` minor relay
///
/// The double shows a 4-card major plus a 5-6 card minor with the
/// [`woolsey_double_floor`][crate::bidding::american::woolsey_double_floor] points
/// floor.  The shape is a *double* disjunction (either major, either minor) the
/// per-suit framework cannot pin, so only the points floor is recorded post-walk —
/// but that alone matters: a double of 1NT names no suit, so the generic walk reads
/// it as *nothing* (the takeout-of-a-suit branch needs a suit opening), leaving the
/// floor to sample the doubler as a random hand.
///
/// The advancer's `2♣` over the double is a "name your minor" relay, not own clubs,
/// so its natural reading is suppressed.  Our own `2♥`/`2♠` advances are natural
/// majors and `2NT` is the notrump game-ask, so neither needs suppression.
#[derive(Clone, Copy)]
struct WoolseyXReading {
    double_index: usize,
    relay_suppress: Option<usize>,
}

impl WoolseyXReading {
    fn suppresses(&self, index: usize) -> bool {
        self.relay_suppress == Some(index)
    }
}

fn woolsey_x_reading(auction: &[Call]) -> Option<WoolseyXReading> {
    if !crate::bidding::american::woolsey_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The double must be the defending side's FIRST action over their 1NT.
    let double_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some(index)),
            // The opener's side acted, or a defender did something else (an overcall)
            // — not our takeout double.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's first bid; suppress it only if it is the 2♣ minor relay.  Jump
    // over every opponent call so a contested relay is covered too (the 2♣ relay is
    // only legal as the immediate response, so the first such call is always it).
    let relay_suppress = auction
        .iter()
        .enumerate()
        .skip(double_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Bid(bid) if index % 2 != opener_parity => {
                Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
            }
            _ => None,
        })
        .flatten();

    Some(WoolseyXReading {
        double_index,
        relay_suppress,
    })
}

/// The index of our natural **penalty** double of their 1NT (15+ HCP), or `None`
///
/// A double of 1NT names no suit, so the generic walk's takeout branch (which needs
/// a suit opening) reads it as nothing.  Returns the doubler's index so the post-walk
/// pass records the [`natural_double_floor`][crate::bidding::american::natural_double_floor]
/// points floor.  Mirrors [`woolsey_x_reading`].
///
/// Fires only when a double of their 1NT actually *means* the natural penalty double:
/// the natural defense is on and no convention has repurposed the double (DONT = a
/// one-suiter, direct Landy / Woolsey = both majors — each has its own reading).  A
/// *passed* doubler cannot hold 15+, so their double is the both-majors passed-hand
/// call, not penalty; an unpassed doubler is identified by lane (a seat that passed
/// before the opening occupies a lane below `opening_index`).
pub(super) fn penalty_x_reading(auction: &[Call]) -> Option<usize> {
    use crate::bidding::american as a;
    if !a::natural_defense_enabled()
        || a::direct_dont_enabled()
        || a::meckwell_enabled()
        || a::direct_landy_double().is_some()
        || a::woolsey_enabled()
    {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;

    // The double must be the defending side's FIRST action over their 1NT.
    let double_index = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some(index)),
            // The opener's side acted, or a defender overcalled — not the penalty double.
            _ => Some(None),
        })
        .flatten()?;

    // A passed doubler's double is the both-majors passed-hand call, never 15+ penalty.
    // Seats that passed before the opening fill lanes `0..opening_index` (all the calls
    // there are passes), so an unpassed doubler's lane is at or beyond `opening_index`.
    (double_index % 4 >= opening_index).then_some(double_index)
}

/// The index of responder's double of an opponent's overcall of *our* 1NT
/// (`[1NT,(2X),X]`), or `None`
///
/// Every [`DoubleStyle`][crate::bidding::american::DoubleStyle] makes this double
/// show **8+ values** (takeout ≤3/8, penalty 4+/9, optional 2-3/8), so the post-walk
/// records that points floor — without it the double reads as nothing and opener
/// undercounts the partnership's strength.  Fires only for our own 1NT (the opener
/// shares the actor's parity); their responder's double of our overcall is their
/// convention, not ours.
fn responder_overcall_double_reading(auction: &[Call], len: usize) -> Option<usize> {
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump))
        || opening_index % 2 != len % 2
    {
        return None;
    }
    // The opponent's suit overcall, then our responder's immediate double of it.
    match auction.get(opening_index + 1) {
        Some(Call::Bid(bid)) if bid.strain.is_suit() => {}
        _ => return None,
    }
    (auction.get(opening_index + 2) == Some(&Call::Double)).then_some(opening_index + 2)
}

/// Our side's *subsequent* penalty doubles after the natural penalty X of their
/// 1NT — the latch's later doubles — each paired with the suit it doubles
///
/// The penalty latch ([`set_penalty_latch`][crate::bidding::instinct::set_penalty_latch])
/// makes these via the trump-stack rule, so each promises four-plus cards in the
/// doubled suit.  Recording that length stops the sampler reading the double as
/// takeout — without it the advancer pulls a penalty double thinking partner is
/// short, the phantom-suit leak the [`penalty_x_reading`] doc names.  Empty unless
/// the latch is on, so it agrees with the floor on when a later double is penalty.
///
/// Once we penalty-double their 1NT the penalty stance holds for the rest of the
/// auction (mirrors `penalty_latched`) — a bid of our own does *not* un-latch it,
/// it only updates the suit a later penalty double refers to.
fn penalty_latch_double_reading(auction: &[Call]) -> Vec<(usize, Suit)> {
    if !crate::bidding::instinct::penalty_latch_enabled() {
        return Vec::new();
    }
    let Some(x_index) = penalty_x_reading(auction) else {
        return Vec::new();
    };
    let our_parity = x_index % 2;
    let mut out = Vec::new();
    let mut last_suit_bid: Option<(Suit, usize)> = None; // (suit, the bidder's parity)
    for (index, &call) in auction.iter().enumerate().skip(x_index + 1) {
        match call {
            // Our own bid does not un-latch the penalty stance; it just updates the
            // suit a later penalty double would refer to.
            Call::Bid(bid) => {
                last_suit_bid = bid.strain.suit().map(|suit| (suit, index % 2));
            }
            // Our double of their suit runout is penalty: four-plus in that suit.
            Call::Double if index % 2 == our_parity => {
                if let Some((suit, bidder_parity)) = last_suit_bid
                    && bidder_parity != our_parity
                {
                    out.push((index, suit));
                }
            }
            _ => {}
        }
    }
    out
}

/// Which DONT defense call the defending side made over their 1NT
#[derive(Clone, Copy)]
enum DontKind {
    /// `X` — a one-suiter in ♣/♦/♥ (a spade one-suiter bids the natural `2♠`), so
    /// spades are short.  The long suit is a triple disjunction the per-suit
    /// framework cannot pin; only `spades ≤ 3` is a sound per-suit fact.
    OneSuiter,
    /// `2♣` — clubs (real, ≥ 4) + an unknown higher major.  Names a real club suit,
    /// but the natural ≥ 5 reading is unsound (the 4-major-5-club hand has 4 clubs).
    ClubsMajor,
    /// `2♦` — diamonds (real, ≥ 4) + an unknown major.  As `ClubsMajor` for diamonds.
    DiamondsMajor,
    /// `2♥` — both majors, ≥ 5-4.  Exactly a Landy two-suiter on the `2♥` bid.
    BothMajors,
}

/// A DONT overcall of their 1NT (`X`/`2♣`/`2♦`/`2♥`) and the advancer's relay
///
/// DONT's calls name suits the hand may not hold (`X` names none; `2♣`/`2♦`/`2♥` can
/// be only 4 cards in the named suit) or are relays, so the generic walk misreads
/// them — leaving the floor to raise a phantom suit or sample a random hand.  The
/// natural `2♠` is a genuine spade suit and needs no reading.  Mirrors
/// [`multi_reading`] / [`woolsey_x_reading`].
#[derive(Clone, Copy)]
struct DontReading {
    overcall_index: usize,
    kind: DontKind,
    floor: u8,
    /// The advancer's relay — `2♣` over the `X`, or the `2♦`/`2♥`/`2♠` pass-or-correct
    /// over `2♣`/`2♦`/`2♥` (a preference among partner's suits, not own length).
    advance_suppress: Option<usize>,
}

impl DontReading {
    /// Whether the call at `index` is artificial.  The `X` (a double) names no suit,
    /// so only the `2♣`/`2♦`/`2♥` overcalls suppress their own natural reading; the
    /// advancer's relay is always suppressed.
    fn suppresses(&self, index: usize) -> bool {
        (!matches!(self.kind, DontKind::OneSuiter) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a DONT overcall of their 1NT, gated on
/// [`direct_dont_enabled`][crate::bidding::american::direct_dont_enabled] and the
/// auction being `1NT` then the defending side's first action being a DONT call
fn dont_reading(auction: &[Call]) -> Option<DontReading> {
    if !crate::bidding::american::direct_dont_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    let floor = crate::bidding::american::natural_overcall_points().0;

    // The defending side's FIRST action — a DONT `X`/`2♣`/`2♦`/`2♥` (the natural `2♠`
    // and anything else fall through to the generic reading).
    let (overcall_index, kind) = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => Some(Some((index, DontKind::OneSuiter))),
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Clubs) {
                    Some(DontKind::ClubsMajor)
                } else if bid == Bid::new(2, Strain::Diamonds) {
                    Some(DontKind::DiamondsMajor)
                } else if bid == Bid::new(2, Strain::Hearts) {
                    Some(DontKind::BothMajors)
                } else {
                    None
                };
                Some(kind.map(|kind| (index, kind)))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's relay: `2♣` over the `X` (it names a minor, not own clubs), or the
    // `2♦`/`2♥`/`2♠` preference over a two-suiter (one of partner's suits, not own
    // length).  Both scans jump over every opponent call so a contested relay is
    // covered (the relay is only legal as the immediate response).
    let advance_suppress = match kind {
        DontKind::OneSuiter => auction
            .iter()
            .enumerate()
            .skip(overcall_index + 1)
            .find_map(|(index, &call)| match call {
                Call::Bid(bid) if index % 2 != opener_parity => {
                    Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
                }
                _ => None,
            })
            .flatten(),
        _ => advancer_artificial(auction, overcall_index, opener_parity),
    };

    Some(DontReading {
        overcall_index,
        kind,
        floor,
        advance_suppress,
    })
}

/// Which Meckwell defense call the defending side made over their 1NT
#[derive(Clone, Copy)]
enum MeckwellKind {
    /// `X` — a single 6+ minor OR both majors.  A double naming no suit, and a
    /// disjunction (short majors OR long majors) the per-suit framework cannot pin, so
    /// only the points floor is a sound fact (as the Woolsey / penalty double).
    TwoWayDouble,
    /// `2♣` — clubs (real, ≥ 4) + an unknown major.  As DONT's `ClubsMajor`.
    ClubsMajor,
    /// `2♦` — diamonds (real, ≥ 4) + an unknown major.  As DONT's `DiamondsMajor`.
    DiamondsMajor,
}

/// A Meckwell overcall of their 1NT (`X`/`2♣`/`2♦`) and the advancer's relay
///
/// Meckwell's natural `2♥`/`2♠` single-suiters name real suits (read by the generic
/// walk) and the `2NT` both-minors is the Unusual overlay, so only the two-way `X` and
/// the `2♣`/`2♦` minor + major are decoded here.  Mirrors [`dont_reading`].
#[derive(Clone, Copy)]
struct MeckwellReading {
    overcall_index: usize,
    kind: MeckwellKind,
    floor: u8,
    /// The advancer's relay — `2♣` over the `X`, or the `2♦`/`2♥`/`2♠` pass-or-correct
    /// over `2♣`/`2♦` (a preference among partner's suits, not own length).
    advance_suppress: Option<usize>,
}

impl MeckwellReading {
    /// The `X` (a double) names no suit, so only the `2♣`/`2♦` overcalls suppress
    /// their own natural reading; the advancer's relay is always suppressed.
    fn suppresses(&self, index: usize) -> bool {
        (!matches!(self.kind, MeckwellKind::TwoWayDouble) && self.overcall_index == index)
            || self.advance_suppress == Some(index)
    }
}

/// Read a Meckwell overcall of their 1NT, gated on
/// [`meckwell_enabled`][crate::bidding::american::meckwell_enabled] and the auction
/// being `1NT` then the defending side's first action being a Meckwell call
fn meckwell_reading(auction: &[Call]) -> Option<MeckwellReading> {
    if !crate::bidding::american::meckwell_enabled() {
        return None;
    }
    let opening_index = auction.iter().position(|&c| c != Call::Pass)?;
    if auction[opening_index] != Call::Bid(Bid::new(1, Strain::Notrump)) {
        return None;
    }
    let opener_parity = opening_index % 2;
    let floor = crate::bidding::american::natural_overcall_points().0;

    // The defending side's FIRST action — a Meckwell `X`/`2♣`/`2♦` (natural `2♥`/`2♠`
    // and anything else fall through to the generic reading).
    let (overcall_index, kind) = auction
        .iter()
        .enumerate()
        .skip(opening_index + 1)
        .find_map(|(index, &call)| match call {
            Call::Pass => None,
            Call::Double if index % 2 != opener_parity => {
                Some(Some((index, MeckwellKind::TwoWayDouble)))
            }
            Call::Bid(bid) if index % 2 != opener_parity => {
                let kind = if bid == Bid::new(2, Strain::Clubs) {
                    Some(MeckwellKind::ClubsMajor)
                } else if bid == Bid::new(2, Strain::Diamonds) {
                    Some(MeckwellKind::DiamondsMajor)
                } else {
                    None
                };
                Some(kind.map(|kind| (index, kind)))
            }
            // The opener's side acted (a response), or a defender did something else.
            _ => Some(None),
        })
        .flatten()?;

    // The advancer's relay: `2♣` over the `X` (names a minor, not own clubs), or the
    // `2♦`/`2♥`/`2♠` preference over a two-suiter.  Both scans jump over every opponent
    // call so a contested relay is covered (the relay is only legal as the immediate
    // response).
    let advance_suppress = match kind {
        MeckwellKind::TwoWayDouble => auction
            .iter()
            .enumerate()
            .skip(overcall_index + 1)
            .find_map(|(index, &call)| match call {
                Call::Bid(bid) if index % 2 != opener_parity => {
                    Some((bid == Bid::new(2, Strain::Clubs)).then_some(index))
                }
                _ => None,
            })
            .flatten(),
        _ => advancer_artificial(auction, overcall_index, opener_parity),
    };

    Some(MeckwellReading {
        overcall_index,
        kind,
        floor,
        advance_suppress,
    })
}

/// Apply the meaning of the opening bid (the first non-pass call)
fn apply_opening(inf: &mut Envelope, bid: Bid, seat: u8) {
    // A one-level suit opening reads 10, not 12: `points(12..)` on the shipped
    // rule-of-N+8 scale is the Rule of 20, which admits sound 10-11 counts, and
    // the reading has to stay loose enough for a floor arm or an opponent whose
    // scale we do not control.  Third/fourth seat opens majors lighter still (9).
    let major_floor = if seat >= 3 { 9 } else { 10 };
    let minor_floor = 10;
    let majors_light = Range::new(major_floor, 21);
    match (bid.level.get(), bid.strain) {
        (1, Strain::Hearts) => {
            inf.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));
            inf.narrow_points(majors_light);
        }
        (1, Strain::Spades) => {
            inf.narrow_length(Suit::Spades, Range::at_least(5, LENGTH_CAP));
            inf.narrow_points(majors_light);
        }
        (1, Strain::Diamonds) => {
            inf.narrow_length(Suit::Diamonds, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(minor_floor, 21));
        }
        (1, Strain::Clubs) => {
            inf.narrow_length(Suit::Clubs, Range::at_least(3, LENGTH_CAP));
            inf.narrow_length(Suit::Hearts, Range::new(0, 4));
            inf.narrow_length(Suit::Spades, Range::new(0, 4));
            inf.narrow_points(Range::new(minor_floor, 21));
        }
        (1, Strain::Notrump) => {
            // Balanced, OR — since the shipped `Wide6322` shape also opens 1NT
            // on a 6322 with a six-card minor — a minor running to six.  Majors
            // stay 2–5 (a balanced 5332 major); minors widen to 2–6.  Set the
            // four suits directly: `narrow_length` only intersects, so clamping
            // via `balanced()` first would pin the minors back to five.
            inf.narrow_length(Suit::Spades, Range::new(2, 5));
            inf.narrow_length(Suit::Hearts, Range::new(2, 5));
            inf.narrow_length(Suit::Clubs, Range::new(2, 6));
            inf.narrow_length(Suit::Diamonds, Range::new(2, 6));
            // Plain HCP 15–17 gates the opening (fifths archived).  The plain
            // rule-of-N+8 opt-in scale reads a flat 4-3-3-3 one under its HCP
            // (the shipped floored scale doesn't) and a 5422/6322 one over
            // (9-card long suits − 8); the legacy upgrade scale adds at most
            // +1 the same way.  Sound band 15−slack..18 — the slack term
            // keeps every opt-in arm exact.  ponytail:
            // exact for the shipped plain-HCP gauge; the archived
            // `set_one_notrump_fifths` knob, if ever revived, would re-widen
            // this to 14–19.
            let slack = crate::bidding::constraint::flat_hcp_slack();
            inf.narrow_points(Range::new(15 - slack, 18));
            // The `hcp` gauge is crisp raw HCP — 15–17 gates the opening, with
            // no upgrade slack (notrump valuation, read behind Edit 2's knob).
            inf.narrow_hcp(Range::new(15, 17));
        }
        (2, Strain::Clubs) => {
            // Strong and artificial: 22+ points, but nothing about shape.
            inf.narrow_points(Range::at_least(20, POINTS_CAP));
        }
        (2, Strain::Notrump) => {
            if crate::bidding::american::two_notrump_wide() {
                // Chop G0: the wide-minor 2NT (`set_two_notrump_wide`) caps
                // majors at four (5M(332) opens one-of-a-major) and runs minors
                // to six (5m422/6m322).  `narrow_length` only intersects, so set
                // the four suits directly rather than clamping via `balanced()`.
                inf.narrow_length(Suit::Spades, Range::new(2, 4));
                inf.narrow_length(Suit::Hearts, Range::new(2, 4));
                inf.narrow_length(Suit::Clubs, Range::new(2, 6));
                inf.narrow_length(Suit::Diamonds, Range::new(2, 6));
            } else {
                balanced(inf);
            }
            // As with 1NT: `fifths(20.0..22.0)` admits a quack-heavy 23-count
            // (fifths within 1.6 of raw HCP), so the sound point envelope is
            // 19–23, not 19–22 — and the plain rule-of-N+8 opt-in gives a
            // flat 4-3-3-3 floor another point back.
            let slack = crate::bidding::constraint::flat_hcp_slack();
            inf.narrow_points(Range::new(19 - slack, 23));
        }
        (2, strain) if strain.is_suit() => {
            inf.narrow_length(strain.suit().unwrap(), Range::new(6, 6));
            inf.narrow_points(Range::new(5, 10));
        }
        (3, strain) if strain.is_suit() => {
            inf.narrow_length(strain.suit().unwrap(), Range::at_least(7, LENGTH_CAP));
            inf.narrow_points(Range::new(0, 11));
        }
        _ => {}
    }
}

/// Narrow a balanced opener: two to five cards in every suit
fn balanced(inf: &mut Envelope) {
    for suit in Suit::ASC {
        inf.narrow_length(suit, Range::new(2, 5));
    }
}

/// The point floor a responder's first natural new suit shows, when uncontested
///
/// A one-level new suit promises six-plus points; a game-forcing 2/1 (a
/// two-level new suit over a one-of-a-major opening, or `1♦`–`2♣`) promises
/// thirteen-plus.
fn apply_response_points(inf: &mut Envelope, response: Bid, opening: Bid, eligible: bool) {
    if !eligible {
        return;
    }
    match response.level.get() {
        1 => inf.narrow_points(Range::at_least(6, POINTS_CAP)),
        2 if is_american(opening, response) => {
            inf.narrow_points(Range::at_least(13, POINTS_CAP));
        }
        _ => {}
    }
}

/// Whether a two-level new suit is a game-forcing 2/1 over `opening`
fn is_american(opening: Bid, response: Bid) -> bool {
    response.level.get() == 2
        && match opening.strain {
            Strain::Hearts | Strain::Spades => true,
            Strain::Diamonds => response.strain == Strain::Clubs,
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::constraint::point_count;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Hand, Level};
    use proptest::prelude::*;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    fn read(auction: &[Call]) -> Inferences {
        Inferences::read(&Context::new(RelativeVulnerability::NONE, auction))
    }

    /// Read on a *prefixed* context, the trie access the projection pass needs to
    /// read a convention off its authored rule — what the production search floor
    /// hands `Inferences::read` (cf. `Stance::prefixed_context`).  The plain `read`
    /// above is keyless, so it sees no convention overlay.
    fn read_booked(auction: &[Call]) -> Inferences {
        let stance = crate::american().against();
        Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, auction))
    }

    /// Pins the skew bound `support_band_to_points` is derived from: at every
    /// fit-known trump (three-plus cards), `point_count` lies within the
    /// image of the hand's own support count — so a support band's image
    /// always contains the legacy count of every hand the band admits.
    #[test]
    fn support_band_points_image_is_sound() {
        use rand::SeedableRng as _;

        let mut rng = rand::rngs::StdRng::seed_from_u64(0x5B);
        let hands = crate::bidding::verify::random_hands(&mut rng)
            .take(4096)
            // Extremes the random pool cannot deal: two side voids attain
            // the +5 skew; working doubletons alone attain the −1 side.
            .chain(
                ["432.AKQJT98765..", "..432.AKQJT98765", "AQJT9.KQJT.A2.K2"]
                    .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
            );
        for hand in hands {
            for trump in Suit::ASC {
                if hand[trump].len() < 3 {
                    continue;
                }
                let support =
                    crate::bidding::constraint::support_point_count_in(hand, trump).min(POINTS_CAP);
                let image = support_band_to_points(Range::new(support, support));
                let points = point_count(hand);
                assert!(
                    image.contains(points),
                    "{hand}: trump {trump}, support {support}, points {points}"
                );
            }
        }
    }

    /// The `Dnf` box algebra: `intersect` distributes and **drops** the empty
    /// products, so a disjunctive reading stays tight instead of hulling to the
    /// bounding box.  The worked example is `1NT ∩ 4-5♥` (opener's Stayman `2♥`).
    #[test]
    fn dnf_intersect_drops_empty_products() {
        // A box literal: [♣, ♦, ♥, ♠] length ranges (ASC order) and points.
        let box_ = |c: (u8, u8), d: (u8, u8), h: (u8, u8), s: (u8, u8), p: (u8, u8)| Envelope {
            lengths: [
                Range::new(c.0, c.1),
                Range::new(d.0, d.1),
                Range::new(h.0, h.1),
                Range::new(s.0, s.1),
            ],
            strength: Strength {
                points: Range::new(p.0, p.1),
                ..Strength::unknown()
            },
        };

        // 1NT as three shapes, all 15-17: balanced, then each 5-card major.
        let one_nt = Dnf(vec![
            box_((2, 6), (2, 6), (2, 4), (2, 4), (15, 17)), // balanced
            box_((2, 3), (2, 3), (2, 3), (5, 5), (15, 17)), // 5=♠
            box_((2, 3), (2, 3), (5, 5), (2, 3), (15, 17)), // 5=♥
        ]);
        // Opener's `2♥` over Stayman = 1NT ∩ {4-5 hearts}, other suits free.
        let four_five_hearts = Dnf::from(box_((0, 13), (0, 13), (4, 5), (0, 13), (0, 37)));

        let two_hearts = one_nt.intersect(&four_five_hearts);

        // The 5=♠ box (hearts 2-3) contradicts 4-5♥ and is dropped: 2 boxes, not 3.
        assert_eq!(two_hearts.0.len(), 2, "empty product not dropped");
        // The survivors pin hearts to exactly 4 (from balanced) and exactly 5.
        let hearts: Vec<Range> = two_hearts
            .0
            .iter()
            .map(|b| b.length(Suit::Hearts))
            .collect();
        assert!(hearts.contains(&Range::new(4, 4)) && hearts.contains(&Range::new(5, 5)));

        // The hull re-widens to the bounding box — the slop the Dnf avoids: it
        // admits ♠4♥5, a hand *neither* surviving box holds (balanced caps ♠ at 4
        // only with ≤4♥; the 5♥ box caps ♠ at 3).
        let hull = two_hearts.hull();
        assert_eq!(hull.length(Suit::Hearts), Range::new(4, 5));
        assert_eq!(hull.length(Suit::Spades), Range::new(2, 4));
        assert!(two_hearts.0.iter().all(|b| {
            !(b.length(Suit::Spades).contains(4) && b.length(Suit::Hearts).contains(5))
        }));

        // Fully-contradictory intersect falls back to the widened hull, never empty.
        let empty = Dnf::from(box_((0, 0), (0, 13), (0, 13), (0, 13), (0, 37)));
        let clubs = Dnf::from(box_((5, 13), (0, 13), (0, 13), (0, 13), (0, 37)));
        assert_eq!(empty.intersect(&clubs).0.len(), 1);
    }

    /// `set_dnf_reading` gates the `Or` wall: off, `or([♥, ♠], 6..)` hulls to one
    /// box that admits a 5-4 hand with no six-card major; on, it keeps the two
    /// boxes and rejects that hand while still admitting each true one-suiter.
    #[test]
    fn dnf_reading_pins_the_two_suiter() {
        use crate::bidding::constraint::{Constraint, or};
        // Holdings are spades.hearts.diamonds.clubs.
        let six_spades: Hand = "AKQJ32.KQ4.32.32".parse().unwrap();
        let six_hearts: Hand = "KQ4.AKQJ32.32.32".parse().unwrap();
        let five_four: Hand = "AKQJ3.KQ42.32.32".parse().unwrap(); // no six-card major
        let ctx = Context::new(RelativeVulnerability::NONE, &[]);
        let reading = or([Suit::Hearts, Suit::Spades], 6..);

        set_dnf_reading(false);
        let hull = reading.project(&ctx);
        assert_eq!(hull.0.len(), 1, "off: one bounding box");
        assert!(
            hull.contains(five_four),
            "off: the hull admits the 5-4 slop"
        );

        set_dnf_reading(true);
        let boxes = reading.project(&ctx);
        assert_eq!(boxes.0.len(), 2, "on: one box per major");
        assert!(boxes.contains(six_spades) && boxes.contains(six_hearts));
        assert!(
            !boxes.contains(five_four),
            "on: neither box holds the 5-4 hand"
        );
        set_dnf_reading(true);
    }

    /// `set_blind_opponent_reading` blanks LHO and RHO and *only* those: the
    /// deviation panel's blind arm must leave partner and our own reading
    /// intact, or it stops measuring what reading *their* calls is worth.
    #[test]
    fn blind_opponent_reading_spares_our_side() {
        // 1♦ (me) - 1♥ (LHO) - 1♠ (partner) - 2♥ (RHO): all four seats have
        // shown something, so blanking two of them is visible.
        let auction = [
            bid(1, Strain::Diamonds),
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(2, Strain::Hearts),
        ];
        let seen = read(&auction);
        set_blind_opponent_reading(true);
        let blind = read(&auction);
        set_blind_opponent_reading(false);

        for who in [Relative::Lho, Relative::Rho] {
            assert_eq!(*blind.get(who), Envelope::unknown(), "{who:?} not blanked");
            assert_eq!(blind.announced_dnf(who), &Dnf::unknown());
        }
        assert_ne!(
            *seen.get(Relative::Rho),
            Envelope::unknown(),
            "the fixture must read RHO's 1♥, else the test proves nothing"
        );
        for who in [Relative::Me, Relative::Partner] {
            assert_eq!(*blind.get(who), *seen.get(who), "{who:?} moved");
            assert_eq!(blind.announced_dnf(who), seen.announced_dnf(who));
        }
        // Knob off is byte-identical to never having set it.
        let after = read(&auction);
        for who in [
            Relative::Me,
            Relative::Lho,
            Relative::Partner,
            Relative::Rho,
        ] {
            assert_eq!(*after.get(who), *seen.get(who), "{who:?} moved after reset");
            assert_eq!(after.announced_dnf(who), seen.announced_dnf(who));
        }
    }

    #[test]
    fn opening_shapes() {
        // [1♥]: the opener sits to our right (the call just before ours).
        let one_heart = read(&[bid(1, Strain::Hearts)]);
        assert_eq!(one_heart.rho().length(Suit::Hearts), Range::new(5, 13));
        // `points(12..)` is the Rule of 20, which opens sound 10-11 HCP counts,
        // so the floor is 10.
        assert_eq!(one_heart.rho().strength.points, Range::new(10, 21));

        // A strong notrump is balanced-or-6322-minor (the shipped Wide6322): a
        // major stays 2–5 (a balanced 5332 major), a minor widens to 2–6 (the
        // 6322's six-card minor); an artificial 2♣ says only "strong".
        let one_nt = read(&[bid(1, Strain::Notrump)]);
        assert_eq!(one_nt.rho().length(Suit::Spades), Range::new(2, 5));
        assert_eq!(one_nt.rho().length(Suit::Diamonds), Range::new(2, 6));
        // Plain HCP 15–17: no downgrade on the shipped floored scale, a
        // semi-balanced 5422/6322 reads one over → 15–18.
        assert_eq!(one_nt.rho().strength.points, Range::new(15, 18));

        let two_clubs = read(&[bid(2, Strain::Clubs)]);
        assert_eq!(two_clubs.rho().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(two_clubs.rho().strength.points, Range::new(20, 37));

        // Weak two: exactly six; three-level preempt: seven-plus.
        let weak_two = read(&[bid(2, Strain::Spades)]);
        assert_eq!(weak_two.rho().length(Suit::Spades), Range::new(6, 6));
        assert_eq!(weak_two.rho().strength.points, Range::new(5, 10));
        let preempt = read(&[bid(3, Strain::Diamonds)]);
        assert_eq!(preempt.rho().length(Suit::Diamonds), Range::new(7, 13));

        // A 1♣ opening denies a five-card major.
        let one_club = read(&[bid(1, Strain::Clubs)]);
        assert_eq!(one_club.rho().length(Suit::Clubs), Range::new(3, 13));
        assert_eq!(one_club.rho().length(Suit::Hearts), Range::new(0, 4));
    }

    /// A two-over-one denies four-card support, and the reading now says so.
    ///
    /// `Flip` had no projection at all, so `!support(4..)` — a plain box, "at
    /// most three of partner's suit" — read as ⊤ and responder's spades came
    /// back `0..=13` after `1♠–2♣`.  The strength half of the same rule is
    /// still blind (`Or::project` unions `hcp(13..)` away; see
    /// `docs/ai-bidder/sampled-projection.md`), which is why only the length
    /// axis is asserted here.
    #[test]
    fn two_over_one_denies_four_card_support() {
        let auction = [bid(1, Strain::Spades), Call::Pass, bid(2, Strain::Clubs)];
        let read = read_booked(&auction);
        let responder = read.rho();
        assert_eq!(responder.length(Suit::Spades), Range::new(0, 3));
        assert_eq!(responder.length(Suit::Clubs), Range::new(4, 13));
    }

    #[test]
    fn pass_reading_caps_the_no_open_pass() {
        let p = Call::Pass;
        // Knob off — the pre-ship identity: a pass reads nothing.
        set_pass_reading(false);
        assert_eq!(
            read_booked(&[p, p]).partner().strength.points,
            Range::FULL_POINTS
        );

        set_pass_reading(true);
        set_table_alert_reading(false);
        // Partner's no-open pass reads the opening table's own gate,
        // `points(..12)`; an opponent's pass stays unread until table-wide
        // disclosure is on too.
        let own = read_booked(&[p, p]);
        assert_eq!(own.partner().strength.points, Range::new(0, 11));
        assert_eq!(own.rho().strength.points, Range::FULL_POINTS);
        set_table_alert_reading(true);
        assert_eq!(read_booked(&[p]).rho().strength.points, Range::new(0, 11));
        // A capped passer leaves the opener's own band alone.
        let opened = read_booked(&[p, bid(1, Strain::Hearts)]);
        assert_eq!(opened.partner().strength.points, Range::new(0, 11));
        assert_eq!(opened.rho().strength.points, Range::new(10, 21));
    }

    #[test]
    fn pass_reading_caps_the_failed_compete() {
        let auction = [bid(1, Strain::Hearts), Call::Pass, Call::Pass];
        set_pass_reading(false);
        assert_eq!(
            read_booked(&auction).partner().strength.points,
            Range::FULL_POINTS
        );

        set_pass_reading(true);
        set_table_alert_reading(false);
        // Partner's direct-seat pass: the authored complement of the strong
        // tier ("strong hands double first regardless") — at most 17 raw HCP,
        // 19 on the point-count scale (17 + max upgrade 2).  Their responder's
        // pass stays unread until table-wide disclosure is on.
        let own = read_booked(&auction);
        assert_eq!(own.partner().strength.points, Range::new(0, 19));
        assert_eq!(own.rho().strength.points, Range::FULL_POINTS);
        set_table_alert_reading(true);
        // Their responder's pass: the response table's `hcp(..6)` gate — at
        // most 5 raw HCP, 7 on the point-count scale (5 + max upgrade 2).
        assert_eq!(
            read_booked(&auction).rho().strength.points,
            Range::new(0, 7)
        );
    }

    #[test]
    fn pass_reading_caps_the_silent_responder() {
        set_pass_reading(true);
        // Our 1♥, silent partner: the response table's `hcp(..6)` gate —
        // at most 5 raw HCP, 7 on the point-count scale (5 + max upgrade 2).
        let caps = read_booked(&[bid(1, Strain::Hearts), Call::Pass, Call::Pass, Call::Pass]);
        assert_eq!(caps.partner().strength.points, Range::new(0, 7));
    }

    #[test]
    fn pass_reading_caps_the_notrump_signoff() {
        set_pass_reading(true);
        // Pass of partner's 1NT: the authored union of the weak arm and the
        // flat-eight arm — at most 10 points (the flat-eight arm's 8 HCP + the
        // point-count max upgrade 2), no six-card major.
        let nt = read_booked(&[bid(1, Strain::Notrump), Call::Pass, Call::Pass, Call::Pass]);
        assert_eq!(nt.partner().strength.points, Range::new(0, 10));
        assert!(nt.partner().length(Suit::Hearts).max <= 5);
        assert!(nt.partner().length(Suit::Spades).max <= 5);
    }

    #[test]
    fn pass_reading_skips_trap_and_trivial_passes() {
        set_pass_reading(true);
        set_table_alert_reading(true);
        // The advance of a takeout double authors genuine strong sits (the
        // penalty conversion), so its pass-gate union is trivial: nothing is
        // claimed about the advancer even with every reading knob on.
        let trap = read_booked(&[bid(1, Strain::Hearts), Call::Double, Call::Pass, Call::Pass]);
        assert_eq!(trap.rho().strength.points, Range::FULL_POINTS);
    }

    /// Pass-exclusion (`set_pass_exclusion_reading`) caps the direct-seat pass
    /// over their weak two off the *declined* shape-free double tier
    /// (`points(17..)`, weight 1.2) — the catch-all `hcp(0..)` Pass gate says
    /// nothing on its own, which is why this key read 100% blind in the census.
    /// Shaped siblings (the overcalls, the 2NT arm) complement to unions or ⊤
    /// and are skipped by the single-box filter, so the lengths stay ⊤.
    #[test]
    fn pass_exclusion_caps_the_weak_two_defender() {
        let auction = [bid(2, Strain::Spades), Call::Pass, Call::Pass];
        set_pass_reading(true);
        set_table_alert_reading(false);

        // Knob off — today's identity: the catch-all gate reads nothing.
        set_pass_exclusion_reading(false);
        let off = read_booked(&auction);
        assert_eq!(off.partner().strength.points, Range::FULL_POINTS);

        // Knob on — declining the 17+ double caps the passer.
        set_pass_exclusion_reading(true);
        let on = read_booked(&auction);
        assert_eq!(on.partner().strength.points, Range::new(0, 16));
        // The overcall complements are multi-box and skipped: no length claim.
        assert_eq!(on.partner().length(Suit::Hearts), Range::new(0, 13));

        // Off again is byte-identical to never having been on.
        set_pass_exclusion_reading(false);
        assert_eq!(read_booked(&auction).partner(), off.partner());
        set_table_alert_reading(true);
    }

    #[test]
    fn opener_extras_ladder_reads_extras() {
        use crate::bidding::american::set_opener_extras_ladder;
        let d = bid(1, Strain::Diamonds);
        let s = bid(1, Strain::Spades);
        let p = Call::Pass;
        set_opener_extras_ladder(true);
        // Opener (partner of the hero to act) after 1♦ – 1♠ – X.
        // Jump-rebid 3♦: a self-sufficient six-plus diamonds, 16+.
        let jr = read(&[d, p, s, p, bid(3, Strain::Diamonds), p]);
        assert!(jr.partner().length(Suit::Diamonds).min >= 6);
        assert!(jr.partner().strength.points.min >= 16);
        // Reverse 2♥: five-plus diamonds, four-plus hearts, 17+.
        let rev = read(&[d, p, s, p, bid(2, Strain::Hearts), p]);
        assert!(rev.partner().length(Suit::Diamonds).min >= 5);
        assert!(rev.partner().length(Suit::Hearts).min >= 4);
        assert!(rev.partner().strength.points.min >= 17);
        // Jump-shift 3♣: five-plus diamonds, 18+, and clubs read as the strong
        // 4+ second suit — NOT the weak-jump six (the phantom-suit fix).
        let js = read(&[d, p, s, p, bid(3, Strain::Clubs), p]);
        assert!(js.partner().length(Suit::Diamonds).min >= 5);
        assert!(js.partner().strength.points.min >= 18);
        assert_eq!(
            js.partner().length(Suit::Clubs),
            Range::at_least(4, LENGTH_CAP)
        );
        set_opener_extras_ladder(true);
    }

    #[test]
    fn opener_major_jump_rebid_reads_extras() {
        use crate::bidding::american::set_opener_major_jump_rebid;
        let h = bid(1, Strain::Hearts);
        let s = bid(1, Strain::Spades);
        let p = Call::Pass;
        set_opener_major_jump_rebid(true);
        // Opener after 1♥ – 1♠ – 3♥: jump-rebid of a six-plus major, 16+.
        let jr = read(&[h, p, s, p, bid(3, Strain::Hearts), p]);
        assert!(jr.partner().length(Suit::Hearts).min >= 6);
        assert!(jr.partner().strength.points.min >= 16);
        set_opener_major_jump_rebid(true);
    }

    /// The M6.4 deterministic rule on its canonical auctions: a
    /// four-plus-level new suit is a control bid iff the bidder *bypassed*
    /// it (available below their first-shown suit at the same level);
    /// everything else stays to play — suppressed, nothing floored.
    #[test]
    fn high_bid_control_vs_natural() {
        use crate::bidding::american::set_longer_major_response;
        // Pin the historic hearts-first reading (knob off): these
        // minor-response verdicts are the knob-off ones — the longer-major
        // default is covered by `high_bid_under_longer_major_response`, and the
        // 1NT-transfer sub-cases below are knob-independent.
        set_longer_major_response(false);
        // 1♦–1♠–2♦–4♥: responder bid spades first, so hearts cannot be their
        // longest — a control bid agreeing diamonds.  Hearts stays unfloored;
        // diamond support and slam-try values are recorded instead.
        let control = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(control.partner().length(Suit::Hearts).min, 0);
        assert!(control.partner().length(Suit::Diamonds).min >= 3);
        assert!(control.partner().strength.points.min >= 13);

        // 1♦–1♠–2♦–4♠: rebidding one's own suit is natural — six-plus spades.
        let rebid = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert!(rebid.partner().length(Suit::Spades).min >= 6);

        // 1♦–4♥: the bidder has shown nothing, so hearts can be their
        // longest — to play, no control machinery (and no phantom floor:
        // the honest envelope of an unread jump stays wide).
        let preempt = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert!(preempt.control_bid().is_none());

        // 1♣–1♥–2♣–4♠: spades sit *above* the first-shown hearts, so they were
        // never denied — this system's response and transfer styles bid the
        // cheaper suit first holding a longer higher one (the first M6.4 A/B
        // bled six IMPs a fired board pulling these to the "agreed" minor).
        // To play, not a control bid.
        let above = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert!(above.control_bid().is_none());

        // 1NT–2♦–2♥–4♠: same shape through a transfer (the overlay attributes
        // the hearts to the bidder) — spades were never denied, so to play.
        let post_transfer = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert!(post_transfer.control_bid().is_none());
        assert!(post_transfer.partner().length(Suit::Hearts).min >= 5);

        // 1NT–2♥–2♠–4♥ — the mirror: hearts sit *below* the transferred
        // spades and the cheaper heart transfer was bypassed, so 4♥ cannot be
        // long — a control bid agreeing spades, promising a sixth.
        let mirror = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(mirror.partner().length(Suit::Hearts).min, 0);
        assert!(mirror.partner().length(Suit::Spades).min >= 6);
        set_longer_major_response(true); // restore the shipped default
    }

    /// The longer-major response discipline swaps the M6.4 verdicts on the
    /// two major-response auctions: a 1♥ response denies longer spades (so
    /// the spade jump becomes a control bid), and a 1♠ response may conceal
    /// equal-length five-plus hearts (so the heart jump reads to play).
    #[test]
    fn high_bid_under_longer_major_response() {
        use crate::bidding::american::set_longer_major_response;

        // 1♣–1♥–2♣–4♠, discipline on: 1♥ denied longer spades, so 4♠ is a
        // bypass — a control bid agreeing clubs, spades left unfloored.
        set_longer_major_response(true);
        let control = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        // The mirror 1♣–1♠–2♣–4♥: a 1♠ response no longer proves short
        // hearts (5-5 responds 1♠), so the heart jump reads to play.
        let to_play = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(control.partner().length(Suit::Spades).min, 0);
        assert!(control.partner().length(Suit::Clubs).min >= 3);
        assert!(control.partner().strength.points.min >= 13);
        assert!(to_play.control_bid().is_none());

        // Knob off (the historic hearts-first opt-in): the original verdicts
        // stand — the spade jump above the 1♥ response is to play.
        set_longer_major_response(false);
        let above = read(&[
            bid(1, Strain::Clubs),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        set_longer_major_response(true); // restore the shipped default
        assert!(above.control_bid().is_none());
    }

    #[test]
    fn gambling_3nt_over_double_reads_unbalanced() {
        use crate::bidding::instinct::set_gambling_3nt_over_double;
        // [1NT,(X),3NT,P]: opener reads partner's gambling 3NT.  The floor alerts the
        // call as the long-minor gamble, so the natural balanced-3NT reading is
        // suppressed and a six-card minor stays within range — the search sampler must
        // be free to deal responder its running suit, not pin it to a flat hand.
        set_gambling_3nt_over_double(true);
        let read = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Double,
            bid(3, Strain::Notrump),
            Call::Pass,
        ]);
        assert!(read.partner().length(Suit::Clubs).contains(6));
        assert!(read.partner().length(Suit::Diamonds).contains(6));
        set_gambling_3nt_over_double(false);
    }

    #[test]
    fn leaping_michaels_conditions_partner() {
        use crate::bidding::american::set_leaping_michaels;

        // (2♥)–4♣–(P): the advancer reads partner's two-suiter — five-plus clubs
        // AND five-plus spades, game-forcing — so the search sampler deals partner
        // the right shape rather than a natural club one-suiter.
        set_leaping_michaels(true);
        let advance = read_booked(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
        assert_eq!(advance.partner().length(Suit::Clubs), Range::new(5, 13));
        assert_eq!(advance.partner().length(Suit::Spades), Range::new(5, 13));
        assert_eq!(advance.partner().strength.points, Range::new(14, 37));

        // Over 2♦, the 4♦ cue shows both majors; 4♣ shows clubs + an unknown
        // major, so only clubs is pinned.
        let cue = read_booked(&[
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(cue.partner().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(cue.partner().length(Suit::Spades), Range::new(5, 13));

        // Disabled (the default): a 4♣ jump reads as a natural one-suiter, so
        // spades stay unconstrained — the convention must not leak when off.
        set_leaping_michaels(false);
        let off = read_booked(&[bid(2, Strain::Hearts), bid(4, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);
        set_leaping_michaels(true);
    }

    #[test]
    fn landy_conditions_partner() {
        use crate::bidding::american::{set_landy, set_unusual_notrump_defense};

        // (1NT)–2♣–(P): the advancer reads partner's both-majors two-suiter (at
        // least 4-4 in the majors, 8+ points) rather than a natural club suit.
        set_landy(Some((8, 15)));
        set_unusual_notrump_defense(Some((8, 15)));
        let advance = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(advance.partner().length(Suit::Hearts), Range::new(4, 13));
        assert_eq!(advance.partner().length(Suit::Spades), Range::new(4, 13));
        assert_eq!(advance.partner().length(Suit::Clubs), Range::FULL_LENGTH);
        assert_eq!(advance.partner().strength.points, Range::new(8, 37));

        // (1NT)–2NT–(P): both minors, 5-5 (the independent unusual-2NT toggle).
        let minors = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Notrump), Call::Pass]);
        assert_eq!(minors.partner().length(Suit::Clubs), Range::new(5, 13));
        assert_eq!(minors.partner().length(Suit::Diamonds), Range::new(5, 13));

        // The advancer's 2♦ relay is artificial — read from the overcaller's seat,
        // partner's (the relayer's) diamonds stay unconstrained.
        let relay = read_booked(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

        // Disabled: 2♣ reads as a natural club one-suiter, so spades stay
        // unconstrained — the convention must not leak when off.
        set_landy(None);
        set_unusual_notrump_defense(None);
        let off = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Spades), Range::FULL_LENGTH);

        // Restore the shipped defaults so sibling tests on this thread are unaffected
        // (unusual 2NT ships on; Landy 2♣ ships off).
        set_unusual_notrump_defense(Some((8, 13)));
    }

    #[test]
    fn woolsey_conditions_partner() {
        use crate::bidding::american::{
            set_landy, set_unusual_notrump_defense, set_woolsey, set_woolsey_points,
        };
        // Landy off, Woolsey on: the 2♣ must read through the Woolsey path.
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_woolsey(true);
        set_woolsey_points(10, 19);

        // (1NT)–2♣–(P): Woolsey's 2♣ is both majors, 10+, never a natural club suit.
        // Read off the authored rule's projection (on a prefixed/booked context),
        // which pins each major to 4-5 exactly — Woolsey sends a six-card major to
        // the Multi/Muiderberg calls, a distinction the old loose reader missed.
        let two_c = read_booked(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(two_c.partner().length(Suit::Hearts), Range::new(4, 5));
        assert_eq!(two_c.partner().length(Suit::Spades), Range::new(4, 5));
        assert_eq!(two_c.partner().length(Suit::Clubs), Range::FULL_LENGTH);
        assert_eq!(two_c.partner().strength.points, Range::new(10, 37));

        // (1NT)–2♦–(P): the Multi names diamonds it does NOT hold, so the natural
        // ≥5 reading is suppressed and BOTH minors narrow to ≤4 — the floor can no
        // longer "raise diamonds" into a doubled 5♦ (the 6+ major falls out of the
        // residual the per-suit framework cannot pin).
        let multi = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(multi.partner().length(Suit::Diamonds), Range::new(0, 4));
        assert_eq!(multi.partner().length(Suit::Clubs), Range::new(0, 4));

        // (1NT)–2♥–(P): Muiderberg — exactly 5 hearts, ≤3 spades.
        let muiderberg = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
        assert_eq!(muiderberg.partner().length(Suit::Hearts), Range::new(5, 5));
        assert_eq!(muiderberg.partner().length(Suit::Spades), Range::new(0, 3));

        // The advancer's 2♥/2♠ over 2♣ (both majors) or 2♦ (Multi) is a PREFERENCE
        // among partner's two majors — not own length — so its natural ≥4 reading is
        // suppressed throughout (here, read from the advancer's seat as partner).
        let pref_2c = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(pref_2c.partner().length(Suit::Hearts), Range::FULL_LENGTH);
        let pref_2d = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ]);
        assert_eq!(pref_2d.partner().length(Suit::Spades), Range::FULL_LENGTH);

        // Off: the Multi 2♦ reads as a natural diamond one-suiter again (≥5) — the
        // convention must not leak when disabled.
        set_woolsey(false);
        let off = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(off.partner().length(Suit::Diamonds), Range::new(5, 13));

        // Restore the shipped default (unusual 2NT ships on).
        set_unusual_notrump_defense(Some((8, 13)));
        set_woolsey_points(8, 19);
    }

    #[test]
    fn artificial_witness_covers_doubles() {
        // A projection that floors a suit it would not name — the witness a transfer
        // or two-suiter trips (5+ hearts).
        let mut floors_hearts = Envelope::unknown();
        floors_hearts.narrow_length(Suit::Hearts, Range::at_least(5, LENGTH_CAP));

        // A *bid* that did not name hearts is artificial (Jacoby 2♦ → 5+♥); a bid
        // naming its own suit is natural (1♥ → 5+♥).
        assert!(artificial(&floors_hearts, bid(2, Strain::Diamonds), None));
        assert!(!artificial(&floors_hearts, bid(1, Strain::Hearts), None));

        // A pass redirects from nothing → never artificial, even flooring a suit.
        assert!(!artificial(
            &floors_hearts,
            Call::Pass,
            Some(Strain::Spades)
        ));

        // A double/redouble "names" the *doubled strain*.  Doubling spades while the
        // projection floors hearts is takeout — it points partner at hearts → artificial;
        // doubling hearts while flooring hearts defends the doubled strain → natural
        // (penalty).  A redouble inherits the same doubled strain.
        assert!(artificial(
            &floors_hearts,
            Call::Double,
            Some(Strain::Spades)
        ));
        assert!(!artificial(
            &floors_hearts,
            Call::Double,
            Some(Strain::Hearts)
        ));
        assert!(artificial(
            &floors_hearts,
            Call::Redouble,
            Some(Strain::Spades)
        ));
        assert!(!artificial(
            &floors_hearts,
            Call::Redouble,
            Some(Strain::Hearts)
        ));

        // A double of notrump defends no suit, so any floored side suit is takeout.
        assert!(artificial(
            &floors_hearts,
            Call::Double,
            Some(Strain::Notrump)
        ));
    }

    #[test]
    fn woolsey_double_and_advances_read() {
        use crate::bidding::american::{
            set_landy, set_unusual_notrump_defense, set_woolsey, set_woolsey_double_floor,
            set_woolsey_points,
        };
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_woolsey(true);
        set_woolsey_points(10, 19);
        set_woolsey_double_floor(12);

        // (1NT)–X–(P): the takeout double names no suit, so nothing is misread — but
        // the doubler's strength (12+) is recorded, where a bare double of 1NT would
        // otherwise read as nothing.
        let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(x.partner().strength.points, Range::new(12, 37));

        // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your minor" relay, not own
        // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
        let relay = read(&[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

        // (1NT)–2♥–(P)–2NT–(P): the Muiderberg minor-ask 2NT is a relay in a
        // COMPETITIVE auction (our side already overcalled), so it is never read as a
        // natural notrump invite — the advancer's points stay unconstrained.
        let ask = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(ask.partner().strength.points, Range::new(0, 37));

        // Off: the Woolsey 12+ reading must not leak — the double now falls through to
        // the default-on natural penalty reading (15+), not Woolsey's 12+.
        set_woolsey(false);
        let off = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(off.partner().strength.points, Range::new(15, 37));

        set_unusual_notrump_defense(Some((8, 13)));
        set_woolsey_points(8, 19);
    }

    #[test]
    fn dont_overcalls_and_advances_read() {
        use crate::bidding::american::{set_direct_dont, set_landy, set_unusual_notrump_defense};
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_direct_dont(true);

        // (1NT)–X–(P): a one-suiter in ♣/♦/♥ — spades short (≤3, the one sound fact),
        // strength recorded (the default 8+ overcall floor) where a bare double of 1NT
        // would otherwise read as nothing.
        let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(x.partner().length(Suit::Spades), Range::new(0, 3));
        assert_eq!(x.partner().strength.points, Range::new(8, 37));

        // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your suit" relay, not own
        // clubs, so its natural ≥4 reading is suppressed (read from the advancer seat).
        let relay = read(&[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

        // (1NT)–2♣–(P): a real ≥4 club suit + an unknown major.  The natural ≥5 reading
        // is suppressed (a 4-club / 5-major DONT hand makes this call), re-pinned to ≥4.
        let two_c = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
        assert_eq!(two_c.partner().strength.points, Range::new(8, 37));

        // (1NT)–2♣–(P)–2♦–(P): the advancer's 2♦ is a "name your higher suit" relay,
        // not own diamonds — suppressed.
        let pref = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(pref.partner().length(Suit::Diamonds), Range::FULL_LENGTH);

        // (1NT)–2♥–(P): both majors, ≥4-4 — exactly a Landy two-suiter on the 2♥ bid.
        let two_h = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
        assert_eq!(two_h.partner().length(Suit::Hearts), Range::new(4, 13));
        assert_eq!(two_h.partner().length(Suit::Spades), Range::new(4, 13));

        // Off: the 2♣ reads as a natural club one-suiter again (≥5) — no leak.
        set_direct_dont(false);
        let off = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));
    }

    #[test]
    fn meckwell_overcalls_and_advances_read() {
        use crate::bidding::american::{set_landy, set_meckwell, set_unusual_notrump_defense};
        set_landy(None);
        set_unusual_notrump_defense(None);
        set_meckwell(true);

        // (1NT)–X–(P): the two-way double (single 6+ minor OR both majors) shares no
        // sound per-suit fact, so ONLY the points floor is recorded — no length is
        // narrowed (unlike DONT's X, which pins spades ≤ 3).
        let x = read(&[bid(1, Strain::Notrump), Call::Double, Call::Pass]);
        assert_eq!(x.partner().strength.points, Range::new(8, 37));
        assert_eq!(x.partner().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(x.partner().length(Suit::Hearts), Range::FULL_LENGTH);

        // (1NT)–X–(P)–2♣–(P): the advancer's 2♣ is a "name your suit" relay, not own
        // clubs, so its natural ≥ 4 reading is suppressed.
        let relay = read(&[
            bid(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(relay.partner().length(Suit::Clubs), Range::FULL_LENGTH);

        // (1NT)–2♣–(P): a real ≥ 4 club suit + an unknown major.  The natural ≥ 5
        // reading is suppressed (a 4-club / 5-major hand makes this call), re-pinned ≥ 4.
        let two_c = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(two_c.partner().length(Suit::Clubs), Range::new(4, 13));
        assert_eq!(two_c.partner().strength.points, Range::new(8, 37));

        // (1NT)–2♦–(P): diamonds + a major, real ≥ 4.
        let two_d = read(&[
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            Call::Pass,
        ]);
        assert_eq!(two_d.partner().length(Suit::Diamonds), Range::new(4, 13));

        // (1NT)–2♥–(P): NATURAL hearts (Meckwell's 2♥ is a single-suiter, not DONT's
        // both-majors), so spades are not floored — the DONT-vs-Meckwell fork.
        let two_h = read(&[bid(1, Strain::Notrump), bid(2, Strain::Hearts), Call::Pass]);
        assert_eq!(
            two_h.partner().length(Suit::Spades).min,
            0,
            "natural 2♥ shows no spades",
        );

        // Off: the 2♣ reads as a natural club one-suiter again (≥ 5) — no leak.
        set_meckwell(false);
        let off = read(&[bid(1, Strain::Notrump), bid(2, Strain::Clubs), Call::Pass]);
        assert_eq!(off.partner().length(Suit::Clubs), Range::new(5, 13));

        set_unusual_notrump_defense(Some((8, 13)));
    }

    #[test]
    fn narrowed_points_intersects_one_player() {
        // 1NT shows 15-18; narrow the opener (here our RHO) to the upper half.
        let inf = read(&[bid(1, Strain::Notrump)]);
        assert_eq!(inf.rho().strength.points, Range::new(15, 18));

        let upper = inf.narrowed_points(Relative::Rho, Range::new(17, 18));
        assert_eq!(
            upper.rho().strength.points,
            Range::new(17, 18),
            "narrowed to the half"
        );
        assert_eq!(
            inf.rho().strength.points,
            Range::new(15, 18),
            "original unchanged"
        );
        // Shape and the other players are untouched.
        assert_eq!(
            upper.rho().length(Suit::Spades),
            inf.rho().length(Suit::Spades)
        );
        assert_eq!(
            upper.partner().strength.points,
            inf.partner().strength.points
        );

        // Intersection, not replacement: a wider request cannot widen what was shown.
        let clamped = inf.narrowed_points(Relative::Rho, Range::new(0, POINTS_CAP));
        assert_eq!(clamped.rho().strength.points, Range::new(15, 18));
    }

    #[test]
    fn third_seat_openings_are_light() {
        // [P, P, 1♠]: a third-seat opener may be down to nine points.
        let third = read(&[Call::Pass, Call::Pass, bid(1, Strain::Spades)]);
        assert_eq!(third.rho().strength.points, Range::new(9, 21));
    }

    #[test]
    fn responses_narrow_partner_and_opener() {
        // [1♥, P, 2♣, P]: we opened 1♥ (partner is us at index 0... no — at
        // len 4, index 0 is Me), partner responded 2♣ (game-forcing 2/1).
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ];
        let inf = read(&auction);
        // Index 0 (1♥) is four before the actor → Me, the opener.
        assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
        // Index 2 (2♣) is two before → Partner, the 2/1 responder.
        assert_eq!(inf.partner().length(Suit::Clubs), Range::new(4, 13));
        assert_eq!(inf.partner().strength.points, Range::new(13, 37));
    }

    #[test]
    fn opener_rebid_reads_five_plus_by_default() {
        // [1♥, P, 1♠, P, 2♥, P]: the opener (who bid 1♥ and rebid 2♥) sits as
        // partner, and the 1♠ responder is us.  The shipped sound reading
        // keeps the rebid at five-plus (the floor routinely rebids a good
        // five); the legacy six-card claim needs the knob off.
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
        // Our 1♠ response showed four spades and six-plus points.
        assert_eq!(inf.me().length(Suit::Spades), Range::new(4, 13));
        assert_eq!(inf.me().strength.points, Range::new(6, 37));
        set_length_soundness(false);
        let legacy = read(&auction);
        assert_eq!(legacy.partner().length(Suit::Hearts), Range::new(6, 13));
        set_length_soundness(true);
    }

    #[test]
    fn competitive_opener_rebid_shows_sixth_card() {
        // [1♦, 1♥, P, 2♥, 3♦, P]: partner opened 1♦ and, over the opponents'
        // heart auction, rebid 3♦ (the opt-in `set_competitive_rebid` floor).
        // The natural length reading applies in competition too — only the
        // *strength* reading is suppressed when opponents act — so partner is
        // still read with six-plus diamonds, keeping the sampler and any further
        // interference sound.  Knob-independent: `read` interprets the auction.
        let auction = [
            bid(1, Strain::Diamonds),
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Hearts),
            bid(3, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(6, 13));
    }

    #[test]
    fn overcall_shows_five_cards() {
        // [1♦, 1♠]: their 1♦ opening, our partner's... no — at len 2, index 1
        // (1♠) is RHO.  Their 1♦ is two before → Partner? recompute below.
        let auction = [bid(1, Strain::Diamonds), bid(1, Strain::Spades)];
        let inf = read(&auction);
        // Index 0 (1♦ opening) → Partner; index 1 (1♠ overcall) → Rho.
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::new(3, 13));
        assert_eq!(inf.rho().length(Suit::Spades), Range::new(5, 13));
        assert_eq!(inf.rho().strength.points, Range::new(8, 37));
    }

    #[test]
    fn transfers_are_not_read_as_natural() {
        // [1NT, P, 2♦, P]: 2♦ is a Jacoby transfer, not diamonds — the
        // opening side's artificial response leaves shape unknown.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn three_level_suit_over_one_notrump_is_natural() {
        // [1NT, P, 3♥, P]: with the splinter *not* authored, a three-level suit
        // bid over 1NT is forcing and natural in the instinct reading —
        // five-plus hearts.  This is the knob-off control for
        // `nt_splinter_is_read_as_shortness_not_length`; the splinter is on by
        // default, so the walk has to be asked for explicitly.
        crate::bidding::american::set_nt_splinter(false);
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_splinter(true);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
    }

    #[test]
    fn nt_splinter_is_read_as_shortness_not_length() {
        // [1NT, P, 3♥, P] with the splinter authored: the *same* call that reads
        // as five-plus hearts above now decodes off its alert into the pinned
        // shape — short hearts, 2-3 spades, exactly four diamonds, 5-6 clubs.
        // The natural walk would floor a phantom heart suit responder is void in.
        crate::bidding::american::set_nt_splinter(true);
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        crate::bidding::american::set_nt_splinter(false);

        let partner = inf.partner();
        assert!(partner.length(Suit::Hearts).max <= 1);
        assert_eq!(partner.length(Suit::Spades), Range::new(2, 3));
        assert_eq!(partner.length(Suit::Diamonds), Range::new(4, 4));
        assert_eq!(partner.length(Suit::Clubs), Range::new(5, 6));

        // Knob off, the book has no 3♥ rule and the walk is back: five-plus.
        let off = read_booked(&auction);
        crate::bidding::american::set_nt_splinter(true); // restore the default
        assert_eq!(off.partner().length(Suit::Hearts), Range::new(5, 13));
    }

    #[test]
    fn systems_on_overcall_transfer_is_not_read_as_diamonds() {
        // [1♦, 1NT, P, 2♦, P]: their 1♦, our 1NT overcall, the advancer's 2♦ is a
        // Jacoby transfer (grafted opening-1NT structure), not natural diamonds.
        // Stripping their opening reads it as [1NT, P, 2♦, P], so the floor never
        // raises a phantom diamond suit into a doubled disaster (the iron rule).
        let auction = [
            bid(1, Strain::Diamonds),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert_eq!(inf.partner().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn gladiator_cue_is_not_read_as_their_major() {
        // [1♠, 1NT, P, 2♠, P]: our 1NT overcall of their 1♠; the advancer's 2♠ is
        // Gladiator Stayman for hearts (exactly 4, INV+) — NOT a natural spade
        // suit.  The major-strip is suppressed for Gladiator, so `gladiator_reading`
        // reads the cue.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        // Their major is never floored into the advancer's hand (the iron rule)...
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
        // ...and the cue pins the four-card heart holding it promised.
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(4, 13));
    }

    #[test]
    fn gladiator_relay_is_not_read_as_clubs() {
        // [1♠, 1NT, P, 2♣, P]: the advancer's 2♣ is the Gladiator relay (weak /
        // invitational, any suit), not a natural club suit.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    }

    #[test]
    fn gladiator_delayed_cue_is_read_as_exactly_three_not_spades() {
        // [1♠,1NT,P,2♣,P,2♦,P,2♠,P]: the advancer's SECOND 2♠ (after the 2♣ relay
        // and forced 2♦) is the Gladiator delayed cue — exactly 3 hearts, INV+ —
        // NOT a natural spade suit.  The suppression must cover it too, else the
        // floor raises a phantom spade suit into a doubled disaster (the iron rule).
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        // Their major is never floored into the advancer's hand...
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
        // ...and the delayed cue pins exactly 3 hearts.
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(3, 3));
    }

    #[test]
    fn gladiator_stolen_relay_double_is_read_as_the_relay() {
        // [1♠, 1NT, (2♣), X, P]: over RHO's systems-on 2♣, the advancer's Double is
        // the stolen Gladiator relay (weak-or-invitational, any suit) — NOT a
        // penalty double naming clubs.  The reader mirrors the book rebase.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            Call::Double,
            Call::Pass,
        ];
        let inf = read(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        // No phantom club suit raised from the doubled strain...
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
        // ...and no point cap: the relay's third arm is game-forcing, so the
        // `0..=9` this used to assert excluded hands the agreement admits (see
        // the `Relay` arm of the post-walk block).
        assert_eq!(inf.partner().strength.points, Range::FULL_POINTS);
    }

    /// The system's own choice at `auction` — the highest finite logit, book
    /// and floor together (the in-crate twin of `examples/common::next_call`,
    /// minus the legality filter: every call these tests expect is legal).
    fn chosen_call(stance: &crate::bidding::Stance, hand: Hand, auction: &[Call]) -> Call {
        let (logits, _) = stance
            .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
            .expect("the Gladiator node classifies");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    /// Do we play the card we claim to play?
    ///
    /// Our Gladiator (`set_nt_overcall_gladiator`) adapts the Crowborough card
    /// — <https://www.bridgewebs.com/crowborough/NT%20Responses.htm> — from a
    /// 1NT *opening* to our 1NT *overcall*, where `2♦` is natural and the cue
    /// is Stayman, so the relay must also park the hands that card's `2♦`
    /// Extended Stayman takes.  This replays the **bidder** (not the rule
    /// table) over one representative hand per advance and per relay
    /// continuation, so a floor that drifts under the structure shows up as a
    /// red test rather than as a convention that quietly stops firing.
    #[test]
    fn gladiator_advances_follow_the_card() {
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let stance = crate::american().against();
        let node = [bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass];
        // After the relay and its forced 2♦ puppet: the XYZ-style sort.
        let sorted: Vec<Call> = node
            .iter()
            .copied()
            .chain([
                bid(2, Strain::Clubs),
                Call::Pass,
                bid(2, Strain::Diamonds),
                Call::Pass,
            ])
            .collect();

        // (hand, auction, expected call, what the hand is)
        let rows: &[(&str, &[Call], Call, &str)] = &[
            // Their major is ♠, so the one unbid major `o` is ♥ throughout.
            (
                "873.93.KJ973.T94",
                &node,
                bid(2, Strain::Clubs),
                "weak with 5+♦ — the relay's weak takeout arm",
            ),
            (
                "K872.Q93.J84.Q93",
                &node,
                bid(2, Strain::Clubs),
                "invitational, nothing to bid directly — the relay's INV arm",
            ),
            (
                "K3.Q876.KJ84.972",
                &node,
                bid(2, Strain::Spades),
                "INV with exactly 4♥, not 4333 — the cue, Stayman for ♥",
            ),
            (
                "K3.972.KJ864.Q93",
                &node,
                bid(2, Strain::Diamonds),
                "INV with exactly 5♦ — natural",
            ),
            (
                "93.KJ864.K73.Q92",
                &node,
                bid(2, Strain::Hearts),
                "INV with exactly 5♥ — natural",
            ),
            (
                "93.874.J6.KQ9764",
                &node,
                bid(2, Strain::Notrump),
                "weak with 6+♣ — the transfer to clubs",
            ),
            (
                "3.KQ86.AJ84.K976",
                &node,
                bid(3, Strain::Spades),
                "GF raise of ♥ with a singleton spade — the splinter",
            ),
            // The relay's continuations over the forced 2♦.
            (
                "873.93.KJ973.T94",
                &sorted,
                Call::Pass,
                "weak with ♦ — pass the puppet",
            ),
            (
                "93.KJ864.T73.972",
                &sorted,
                bid(2, Strain::Hearts),
                "weak with 5+♥ — the takeout",
            ),
            (
                "K872.Q93.J84.Q93",
                &sorted,
                bid(2, Strain::Notrump),
                "balanced INV (flat 4333: no delayed cue)",
            ),
            (
                "K872.Q93.KJ84.9",
                &sorted,
                bid(2, Strain::Spades),
                "INV with exactly 3♥, not 4333 — the delayed cue",
            ),
            (
                "932.7.QJ9764.KJ2",
                &sorted,
                bid(3, Strain::Diamonds),
                "INV with a good 6-card suit",
            ),
            // The relay's *third* arm — a game-forcing balanced hand with
            // exactly 3♥ — is authored but weight-shadowed: at 0.5 it loses to
            // `3NT` (1.2) and to the 3-level naturals (1.3), so no hand plays
            // it.  Deliberate (the box is too confined to adjudicate an A/B on),
            // and pinned here so the divergence is documented rather than
            // hidden: the arm is read, never played.
            (
                "K942.Q76.AJ83.K4",
                &node,
                bid(3, Strain::Notrump),
                "GF balanced with exactly 3♥ — arm 3 is shadowed by 3NT",
            ),
        ];

        let mut failures: Vec<String> = Vec::new();
        for &(text, auction, expected, what) in rows {
            let hand: Hand = text.parse().expect("a hand");
            let made = chosen_call(&stance, hand, auction);
            if made != expected {
                failures.push(format!("{text} ({what}): bid {made}, carded {expected}"));
            }
        }
        crate::bidding::american::set_nt_overcall_gladiator(false);

        assert!(
            failures.is_empty(),
            "Gladiator diverges from the card:\n{}",
            failures.join("\n"),
        );
    }

    /// Every Gladiator reading admits the hand that actually made the call.
    ///
    /// The behavioural analogue of `authored_rules_eval_within_projection`,
    /// which cannot cover this table: that sweep walks the shipped tries, and
    /// `gladiator_advances` is only in one when the knob is on.  It also covers
    /// what no static sweep can — the hand-written stamps in the post-walk
    /// block, which may narrow past what the rules promise (this test is what
    /// caught the relay's `0..=9` band deleting the game-forcing box).
    #[test]
    fn gladiator_readings_admit_the_bidder() {
        use rand::SeedableRng as _;

        crate::bidding::american::set_nt_overcall_gladiator(true);
        set_dnf_reading(true);
        let stance = crate::american().against();
        let node = [bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass];

        let mut rng = rand::rngs::StdRng::seed_from_u64(0x61AD);
        let hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
            .take(256)
            .collect();

        let mut failures: Vec<String> = Vec::new();
        // The advancer sits two seats back once a pass follows their call, so
        // `Relative::Partner` is the seat that just bid.
        //
        // Every advance, not just the ones `gladiator_reading` decodes: the
        // card's *natural* advances are read by the walk, and the walk used to
        // read the game-forcing `3♣`/`3♦`/`3O` — authored `len(suit, 5..)` — as
        // a weak six-card jump, excluding every five-card advancer from its own
        // box.  Fixed by teaching the walk that our 1NT *overcall* takes the
        // same three-level reading as an opening 1NT (`over_one_notrump`), and
        // pinned here so the two layers cannot drift apart again.
        let check = |failures: &mut Vec<String>, hand: Hand, auction: &[Call], made: Call| {
            let mut read: Vec<Call> = auction.to_vec();
            read.push(made);
            read.push(Call::Pass);
            let inferences = stance.infer(RelativeVulnerability::NONE, &read);
            if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
                failures.push(format!(
                    "[{}] reading excludes the hand that bid it: {hand}",
                    read.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                ));
            }
        };

        // Both reading regimes.  Knob-on, the natural advances project their
        // authoring rule *on top of* the walk's reading, so a walk claim that
        // contradicts the rule empties the box instead of quietly overriding it
        // — the sweep is how `set_natural_reading` gets adjudicated per node.
        for natural in [false, true] {
            set_natural_reading(natural);
            for &hand in &hands {
                let made = chosen_call(&stance, hand, &node);
                check(&mut failures, hand, &node, made);
                // Relayers carry on through the forced 2♦ — the only route to
                // the delayed cue, whose stamp is the other narrowing one.
                if made != bid(2, Strain::Clubs) {
                    continue;
                }
                let sorted: Vec<Call> = node
                    .iter()
                    .copied()
                    .chain([
                        bid(2, Strain::Clubs),
                        Call::Pass,
                        bid(2, Strain::Diamonds),
                        Call::Pass,
                    ])
                    .collect();
                let continued = chosen_call(&stance, hand, &sorted);
                check(&mut failures, hand, &sorted, continued);
            }
            // The runout branch too — `[1♠, 1NT, (X)]` is authored, so its
            // escapes are read by the walk like any other natural call.
            let doubled = [
                bid(1, Strain::Spades),
                bid(1, Strain::Notrump),
                Call::Double,
            ];
            for &hand in &hands {
                let made = chosen_call(&stance, hand, &doubled);
                check(&mut failures, hand, &doubled, made);
            }
        }
        set_natural_reading(false);
        crate::bidding::american::set_nt_overcall_gladiator(false);

        assert!(
            failures.is_empty(),
            "Gladiator readings exclude their own bidders:\n{}",
            failures.join("\n"),
        );
    }

    /// Every reading admits the hand that actually made the call — the
    /// table-driven regime-2 invariant of `docs/reading-drift-handoff.md`.
    ///
    /// At each node the *bidder* is replayed over seeded hands and partner's
    /// reading of the chosen call must admit the hand, in both reading regimes
    /// — the only check that catches an authored-natural rule contradicting the
    /// walk's shape-guess (`authored_rules_eval_within_projection` compares a
    /// rule to *its own* projection and is blind to the walk).  Default knobs;
    /// the knob-gated twin is `gladiator_readings_admit_the_bidder`.
    ///
    /// A row lands **together with the repair that makes it green** — the
    /// unrepaired queue lives in the handoff doc's ledger, not here.
    #[test]
    fn readings_admit_the_bidder() {
        use rand::SeedableRng as _;

        set_dnf_reading(true);
        let stance = crate::american().against();

        // (what the node is, the auction up to the seat replayed).  Multi-call
        // seats are route-filtered below: a hand counts only when replaying
        // the seat's *earlier* decisions reproduces the script, so the reading
        // of the whole lane is tested against hands that actually bid it.
        let nodes: &[(&str, &[Call])] = &[
            ("opening", &[]),
            ("second-seat opening", &[Call::Pass]),
            ("response to 1♠", &[bid(1, Strain::Spades), Call::Pass]),
            ("response to 1♥", &[bid(1, Strain::Hearts), Call::Pass]),
            // A raise of a preempt is two-way (furthering or to-make), so the
            // walk stamps no band and no support floor on it — the `1..=11`
            // cap used to exclude every to-make raiser of `[3♥ P 4♥]`.
            (
                "raise of a 3♥ preempt",
                &[bid(3, Strain::Hearts), Call::Pass],
            ),
            (
                "raise of a 3♠ preempt",
                &[bid(3, Strain::Spades), Call::Pass],
            ),
            ("raise of a weak 2♥", &[bid(2, Strain::Hearts), Call::Pass]),
            // Delayed preferences/raises of a shown 5-6 suit floor at two (the
            // false preference on Hx is the norm) — the blanket 3-card stamp
            // excluded 81% of the actual preference bidders.
            (
                "preference after forcing NT, 2♦ rebid",
                &[
                    bid(1, Strain::Spades),
                    Call::Pass,
                    bid(1, Strain::Notrump),
                    Call::Pass,
                    bid(2, Strain::Diamonds),
                    Call::Pass,
                ],
            ),
            (
                "preference after forcing NT, 2♥ rebid",
                &[
                    bid(1, Strain::Spades),
                    Call::Pass,
                    bid(1, Strain::Notrump),
                    Call::Pass,
                    bid(2, Strain::Hearts),
                    Call::Pass,
                ],
            ),
            (
                "raise of the jump rebid",
                &[
                    bid(1, Strain::Spades),
                    Call::Pass,
                    bid(1, Strain::Notrump),
                    Call::Pass,
                    bid(3, Strain::Spades),
                    Call::Pass,
                ],
            ),
            (
                "raise of opener's rebid suit",
                &[
                    bid(1, Strain::Hearts),
                    Call::Pass,
                    bid(1, Strain::Spades),
                    Call::Pass,
                    bid(2, Strain::Hearts),
                    Call::Pass,
                ],
            ),
            // The XYZ 2M rebid is authored five-plus on both routes; the
            // walk's sixth-card stamp excluded every 5-carder.
            (
                "XYZ relay then 2♠ invite",
                &[
                    bid(1, Strain::Diamonds),
                    Call::Pass,
                    bid(1, Strain::Spades),
                    Call::Pass,
                    bid(1, Strain::Notrump),
                    Call::Pass,
                    bid(2, Strain::Clubs),
                    Call::Pass,
                    bid(2, Strain::Diamonds),
                    Call::Pass,
                ],
            ),
            (
                "XYZ direct 2♠ sign-off",
                &[
                    bid(1, Strain::Diamonds),
                    Call::Pass,
                    bid(1, Strain::Spades),
                    Call::Pass,
                    bid(1, Strain::Notrump),
                    Call::Pass,
                ],
            ),
            // Post-transfer continuations fall under the notrump-structure
            // blanket — the artificial 2♦ used to count as a first diamond
            // bid, reading responder's 3♦ as a six-card rebid.
            (
                "responder's second suit after a transfer",
                &[
                    bid(1, Strain::Notrump),
                    Call::Pass,
                    bid(2, Strain::Diamonds),
                    Call::Pass,
                    bid(2, Strain::Hearts),
                    Call::Pass,
                ],
            ),
            // The support double's `support(3..=3)` projects under the
            // bidder's at-the-time context — the reader-context skew used to
            // stamp the exactly-3 on the opened minor (100% exclusion).
            (
                "opener's support double",
                &[
                    bid(1, Strain::Diamonds),
                    Call::Pass,
                    bid(1, Strain::Hearts),
                    bid(1, Strain::Spades),
                ],
            ),
            // Cue raises: the same skew put the `support(n..)` atom on the
            // cue suit itself, excluding every cue-bidder over a minor.
            (
                "cue raise over their 1♠",
                &[bid(1, Strain::Hearts), bid(1, Strain::Spades)],
            ),
            (
                "cue raise over their 2♦",
                &[bid(1, Strain::Spades), bid(2, Strain::Diamonds)],
            ),
            (
                "cue raise over their 1♦",
                &[bid(1, Strain::Clubs), bid(1, Strain::Diamonds)],
            ),
            (
                "advance of our 1NT overcall (systems on)",
                &[bid(1, Strain::Spades), bid(1, Strain::Notrump), Call::Pass],
            ),
            (
                "runout of our doubled 1NT overcall (systems on)",
                &[
                    bid(1, Strain::Spades),
                    bid(1, Strain::Notrump),
                    Call::Double,
                ],
            ),
        ];

        // The four 5-5-major witnesses that caught the strip's keyless re-read
        // (each bids the authored both-majors 3♦ off `points(8..)` on the
        // upgrade scale), then a random sweep.
        let mut rng = rand::rngs::StdRng::seed_from_u64(0x5EAD);
        let hands: Vec<Hand> = [
            "Q9632.AT985.T53.",
            "QJ862.96543.K5.Q",
            "KJT84.AQ653.87.T",
            "KQ853.T7542.9.QJ",
        ]
        .iter()
        .map(|text| text.parse().expect("a hand"))
        .chain(crate::bidding::verify::random_hands(&mut rng).take(256))
        .collect();

        let mut failures: Vec<String> = Vec::new();
        for natural in [false, true] {
            set_natural_reading(natural);
            for &(what, node) in nodes {
                for &hand in &hands {
                    // Honest route only: the seat's earlier calls in the
                    // script must be the ones this hand actually chooses.
                    if (node.len() % 4..node.len())
                        .step_by(4)
                        .any(|i| chosen_call(&stance, hand, &node[..i]) != node[i])
                    {
                        continue;
                    }
                    let made = chosen_call(&stance, hand, node);
                    // After `made` and a pass, the seat to act is the bidder's
                    // partner, so `Relative::Partner` is the seat replayed.
                    let mut read: Vec<Call> = node.to_vec();
                    read.push(made);
                    read.push(Call::Pass);
                    let inferences = stance.infer(RelativeVulnerability::NONE, &read);
                    if !inferences.admits(Relative::Partner, hand) && failures.len() < 16 {
                        failures.push(format!(
                            "{what} [{}] (natural-reading {natural}) excludes the hand that bid it: {hand}",
                            read.iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join(" "),
                        ));
                    }
                }
            }
        }
        set_natural_reading(false);

        assert!(
            failures.is_empty(),
            "readings exclude their own bidders:\n{}",
            failures.join("\n"),
        );
    }

    /// A doubled 1NT overcall runs out — it does not jump to the three level.
    ///
    /// Gladiator turns off `systems_on_overcall_strip`, which is what let the
    /// floor read `[1M, 1NT, X]` as a doubled *opening* 1NT.  Without it the
    /// distilled net escaped a 1-count to `3♥`; `gladiator_doubled_runout` is
    /// the book node that shadows it.
    #[test]
    fn gladiator_runs_out_of_the_doubled_overcall() {
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let stance = crate::american().against();
        let node = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Double,
        ];

        // (hand, expected, what it is)
        let rows: &[(&str, Call, &str)] = &[
            ("873.93.KJ973.T94", bid(2, Strain::Diamonds), "bust, 5♦"),
            ("93.KJ864.T73.972", bid(2, Strain::Hearts), "bust, 5♥"),
            ("93.874.J6.KQ9764", bid(2, Strain::Clubs), "bust, 6♣"),
            (
                "8732.932.J973.T4",
                Call::Pass,
                "1-count, no five-bagger: sit",
            ),
            (
                "T9843.93.J973.T4",
                Call::Pass,
                "bust with five of THEIR major: sit, never run into it",
            ),
            ("K872.Q93.J84.Q93", Call::Redouble, "values: play 1NT××"),
        ];

        let mut failures: Vec<String> = Vec::new();
        for &(text, expected, what) in rows {
            let hand: Hand = text.parse().expect("a hand");
            let made = chosen_call(&stance, hand, &node);
            if made != expected {
                failures.push(format!("{text} ({what}): bid {made}, carded {expected}"));
            }
        }
        crate::bidding::american::set_nt_overcall_gladiator(false);

        assert!(
            failures.is_empty(),
            "the doubled 1NT overcall misplays its runout:\n{}",
            failures.join("\n"),
        );
    }

    /// `set_natural_reading` publishes what an unalerted authored rule promises.
    ///
    /// `gladiator_advances` authors the game-forcing `3♦` as
    /// `len(♦, 5..) & points(game..)`.  It is natural, so it carries no alert and
    /// the projection pass skips it: the walk supplies a length floor and the
    /// game force is simply lost.  Knob-on the rule's own box is intersected in.
    #[test]
    fn natural_reading_publishes_an_unalerted_rules_promise() {
        crate::bidding::american::set_nt_overcall_gladiator(true);
        set_dnf_reading(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Diamonds),
            Call::Pass,
        ];

        set_natural_reading(false);
        let off = read_booked(&auction);
        set_natural_reading(true);
        let on = read_booked(&auction);
        set_natural_reading(false);
        crate::bidding::american::set_nt_overcall_gladiator(false);

        assert_eq!(
            off.partner().strength.points,
            Range::FULL_POINTS,
            "knob-off the game force is unread"
        );
        assert!(
            on.partner().strength.points.min >= 10,
            "knob-on the rule's `points(game..)` reaches the reading, got {:?}",
            on.partner().strength.points,
        );
        // The walk's natural reading survives: the call is not suppressed, so
        // the diamond suit is still read from the auction, not only from the box.
        assert!(on.partner().length(Suit::Diamonds).min >= 5);
    }

    /// Every Gladiator continuation ends where the card says, not where the
    /// floor guesses.
    ///
    /// Authoring a node **shadows** the floor, so this sweep is also the record
    /// of what is deliberately *not* authored: every "advancer passes the game
    /// opposite a limited hand" leaf below is answered by the floor and answered
    /// right, and a bare `Pass` node there would only cost the floor its slam
    /// machinery.  The three that are authored are the ones the floor got wrong
    /// — it raised a weak signoff on three trumps, bid `3NT` opposite a hand
    /// that had denied 8 points, and answered Leaping Michaels `4♣` with `5NT`.
    #[test]
    fn gladiator_continuations_are_authored_to_the_leaf() {
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let stance = crate::american().against();
        let p = Call::Pass;
        let base = [bid(1, Strain::Spades), bid(1, Strain::Notrump), p];
        let seq = |tail: &[Call]| -> Vec<Call> {
            base.iter().copied().chain(tail.iter().copied()).collect()
        };
        let relay = bid(2, Strain::Clubs);
        let forced = bid(2, Strain::Diamonds);

        // (auction, hand, expected, what)
        let rows: Vec<(Vec<Call>, &str, Call, &str)> = vec![
            // --- authored: the floor was wrong here ---
            (
                seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
                "AQ8.AK9.Q852.A93",
                p,
                "16 with three hearts: pass the weak signoff (floor raised)",
            ),
            (
                seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
                "AQ86.AKJ.Q85.A93",
                p,
                "17 with three hearts: pass (floor bid 3NT opposite a bust)",
            ),
            (
                seq(&[relay, p, forced, p, bid(2, Strain::Hearts), p]),
                "AQ8.AKJ2.Q85.A9",
                bid(3, Strain::Hearts),
                "18 with four hearts: the one sound push",
            ),
            (
                seq(&[bid(4, Strain::Clubs), p]),
                "AQ8.AK9.Q852.A93",
                bid(4, Strain::Hearts),
                "Leaping 4♣ (5-5 hearts+clubs GF), three-card fit (floor bid 5NT)",
            ),
            (
                seq(&[bid(4, Strain::Diamonds), p]),
                "AQ86.AKJ.Q85.A93",
                bid(4, Strain::Hearts),
                "Leaping 4♦, three-card fit",
            ),
            (
                seq(&[bid(4, Strain::Spades), p]),
                "AQ8.AK9.Q852.A93",
                bid(5, Strain::Diamonds),
                "Leaping 4♠ (both minors), diamonds the longer",
            ),
            // --- deliberately left to the floor, and it answers right ---
            (
                seq(&[bid(2, Strain::Notrump), p, bid(3, Strain::Clubs), p]),
                "93.874.J6.KQ9764",
                p,
                "weak club transfer completed: pass",
            ),
            (
                seq(&[forced, p, bid(3, Strain::Notrump), p]),
                "K3.972.KJ864.Q93",
                p,
                "invitational 2♦ accepted to 3NT: pass",
            ),
            (
                seq(&[bid(2, Strain::Hearts), p, bid(4, Strain::Hearts), p]),
                "93.KJ864.K73.Q92",
                p,
                "invitational 2♥ raised to game: pass",
            ),
            (
                seq(&[
                    relay,
                    p,
                    forced,
                    p,
                    bid(2, Strain::Notrump),
                    p,
                    bid(3, Strain::Notrump),
                    p,
                ]),
                "K872.Q93.J84.Q93",
                p,
                "balanced invitation accepted: pass",
            ),
            (
                seq(&[bid(3, Strain::Spades), p, bid(4, Strain::Hearts), p]),
                "3.KQ86.AJ84.K976",
                p,
                "splinter raised to game: pass",
            ),
            (
                seq(&[bid(3, Strain::Diamonds), p, bid(3, Strain::Notrump), p]),
                "KQT.K8.AJT64.QJ4",
                p,
                "game-forcing 3♦ placed in 3NT: pass",
            ),
        ];

        let mut failures: Vec<String> = Vec::new();
        for (auction, text, expected, what) in rows {
            let hand: Hand = text.parse().expect("a hand");
            let made = chosen_call(&stance, hand, &auction);
            if made != expected {
                failures.push(format!("{text} ({what}): bid {made}, wanted {expected}"));
            }
        }
        crate::bidding::american::set_nt_overcall_gladiator(false);

        assert!(
            failures.is_empty(),
            "Gladiator continuations land in the wrong place:\n{}",
            failures.join("\n"),
        );
    }

    /// Gladiator keeps the systems-on strip where it has no structure of its own.
    ///
    /// Over RHO's **X** and over 3-level-or-higher interference, Gladiator and
    /// systems-on play the same auction (a natural runout, then the floor), so
    /// the strip identity still holds and the inference-aware floor keeps the
    /// picture it was distilled on.  Over a pass or a 2-level bid it does not —
    /// the advances, the stolen relay and Transfer Lebensohl all diverge.
    #[test]
    fn gladiator_keeps_the_strip_where_it_has_no_structure() {
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let p = Call::Pass;
        let one_s = bid(1, Strain::Spades);
        let one_nt = bid(1, Strain::Notrump);
        // (auction after [1♠, 1NT], stripped?)
        let rows: &[(&[Call], bool, &str)] = &[
            (&[Call::Double], true, "their X — a runout in both systems"),
            (
                &[bid(3, Strain::Clubs)],
                true,
                "3-level — the floor in both",
            ),
            (
                &[bid(4, Strain::Hearts)],
                true,
                "4-level — the floor in both",
            ),
            (&[], false, "quiet — the Gladiator advances"),
            (&[p], false, "quiet — the Gladiator advances"),
            (
                &[bid(2, Strain::Clubs)],
                false,
                "their 2♣ — the stolen relay",
            ),
            (
                &[bid(2, Strain::Hearts)],
                false,
                "their 2♥ — Transfer Lebensohl",
            ),
        ];
        let mut failures: Vec<String> = Vec::new();
        for &(tail, want, what) in rows {
            let auction: Vec<Call> = [one_s, one_nt]
                .into_iter()
                .chain(tail.iter().copied())
                .collect();
            let got = super::systems_on_overcall_strip(&auction).is_some();
            if got != want {
                failures.push(format!("{what}: stripped = {got}, wanted {want}"));
            }
        }
        crate::bidding::american::set_nt_overcall_gladiator(false);
        assert!(
            failures.is_empty(),
            "strip scope wrong:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn gladiator_contested_transfer_lebensohl_pins_the_target() {
        // [1♠, 1NT, (2♥), 3♦, P]: over RHO's 2♥ there is no room for the relay
        // tree, so advancer plays Transfer Lebensohl; 3♦ transfers up through their
        // hearts (showing spades), read via the builders' alerts — opener must not
        // raise a phantom diamond suit.
        crate::bidding::american::set_nt_overcall_gladiator(true);
        let auction = [
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            bid(2, Strain::Hearts),
            bid(3, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        crate::bidding::american::set_nt_overcall_gladiator(false);
        assert!(
            inf.partner().length(Suit::Spades).min >= 5,
            "transfer target pinned"
        );
        assert!(
            inf.partner().length(Suit::Diamonds).min < 5,
            "phantom suit not read"
        );
    }

    #[test]
    fn completed_major_transfer_shows_five() {
        // [1NT, P, 2♦, P, 2♥, P]: partner transferred to hearts and we
        // completed; at length 6 the responder is us (Me).  The transfer shows a
        // five-card major even before a jump confirms the sixth, while the
        // transferred-*from* suit stays unread.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert_eq!(inf.me().length(Suit::Hearts), Range::new(5, 13));
        assert_eq!(inf.me().length(Suit::Diamonds), Range::FULL_LENGTH);
    }

    #[test]
    fn transfer_jump_to_game_shows_at_least_five() {
        // [1NT, P, 2♦, P, 2♥, P, 4♥, P]: partner transferred then jumped to 4♥.
        // The projection reads the 2♦ transfer's authored rule — a five-card floor;
        // the old reader's six-card upgrade off the jump is dropped (soundness over
        // tightness, M6.2c).  At length 8 the responder sits as Partner.
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::new(5, 13));
    }

    #[test]
    fn transfer_then_three_major_shows_at_least_five() {
        // [1NT, P, 2♦, P, 2♥, P, 3♥, P]: a raise of the transferred suit.  The
        // projection pins the transfer's five-card floor; the old reader's six-card
        // upgrade and the 8–9 invitational points are dropped (soundness over
        // tightness, M6.2c).
        let auction = [
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert!(inf.partner().length(Suit::Hearts).min >= 5);
    }

    #[test]
    fn transfer_projection_covers_spades_and_two_notrump() {
        // Spade transfer (2♥ → 2♠) jumped to 4♠: the 2♥ transfer rule projects a
        // five-card spade floor.
        let spades = read_booked(&[
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ]);
        assert_eq!(spades.partner().length(Suit::Spades), Range::new(5, 13));

        // The same shape over a 2NT opening (3♦ → 3♥, jump 4♥).
        let two_nt = read_booked(&[
            bid(2, Strain::Notrump),
            Call::Pass,
            bid(3, Strain::Diamonds),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(two_nt.partner().length(Suit::Hearts), Range::new(5, 13));
    }

    #[test]
    fn contested_transfer_auction_is_not_specially_read() {
        // [1NT, 2♣, 2♦, P, 2♥, P, 4♥, P]: with the opponents in, the transfer
        // positions shift, so the special reading must not pin a six-card suit.
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Clubs),
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Hearts),
            Call::Pass,
        ];
        let inf = read(&auction);
        assert!(inf.partner().length(Suit::Hearts).min < 6);
    }

    #[test]
    fn contested_transfer_lebensohl_reads_the_target_under_intervention() {
        // Board 881510: [1NT, (2♠), 3♦, (3♠)] — responder's 3♦ is a Transfer-
        // Lebensohl transfer to hearts (up the line through their spade suit).  RHO's
        // (3♠) skips opener's completion node; the default-on fallback projection
        // re-resolves 3♦'s authoring rule and pins hearts, so opener does not read it
        // as natural diamonds and raise the phantom suit to 5♦x.  Needs the prefixed
        // `read_booked` (the projection reads the rule off the book).
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Spades),
            bid(3, Strain::Diamonds),
            bid(3, Strain::Spades),
        ];
        let inf = read_booked(&auction);
        assert!(
            inf.partner().length(Suit::Hearts).min >= 5,
            "transfer target pinned"
        );
        assert!(
            inf.partner().length(Suit::Diamonds).min < 5,
            "phantom suit not read"
        );
    }

    #[test]
    fn fallback_projection_decodes_contested_leaping_michaels() {
        // [1NT, (2♦), 4♦, (P)]: Leaping Michaels = both majors 5-5, authored as a
        // *guarded fallback* in the (2♦) Transfer block — invisible to the exact-node
        // projection, and with no hand reader.  The default-on fallback projection
        // re-resolves its authoring rule and pins both majors (no reader involved).
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert!(
            inf.partner().length(Suit::Hearts).min >= 5
                && inf.partner().length(Suit::Spades).min >= 5,
            "fallback projection pins both majors for contested Leaping Michaels"
        );
    }

    /// The §7.3.1 union poison (docs/ai-bidder/bba-kickback.md): with
    /// `set_kickback` on, the relocated-ask and answer rules on 4♥/4♠ were
    /// structurally alerted, so a **natural** 4♠'s box was unioned with the
    /// ask's ⊤ projection — partner's `length(Spades).min` collapsed to 0 and
    /// the natural walk's lane bookkeeping was suppressed on top.  The face
    /// gate makes those rules as-if-absent on faces where `kickback_ladder`
    /// claims nothing (here no suit is bid twice by one side, so the ladder is
    /// all-`None`): the knob-on reading must equal the knob-off one.
    #[test]
    fn kickback_face_gate_keeps_natural_four_spades_natural() {
        use crate::bidding::instinct::set_kickback;
        // The audited C−B shape: 1♦ P 1♠ P 2♦ P 4♠ P — the reader is the
        // opener, partner is the natural 4♠ bidder.
        let auction = [
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ];
        let baseline = read_booked(&auction).partner().length(Suit::Spades).min;
        set_kickback(true);
        let gated = read_booked(&auction).partner().length(Suit::Spades).min;
        set_kickback(false); // restore the default (off) for the rest of the suite
        assert!(baseline >= 4, "the natural walk floors responder's spades");
        assert_eq!(gated, baseline, "kickback must not erase the natural floor");
    }

    /// The face gate's positive control: where the ladder *does* claim the
    /// call (hearts agreed, spades unguarded → 4♠ asks), the rule stays live —
    /// alerted, so the ask is not read as a natural spade suit.
    #[test]
    fn kickback_relocated_ask_still_reads_as_the_convention() {
        use crate::bidding::instinct::set_kickback;
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Spades),
            Call::Pass,
        ];
        set_kickback(true);
        let spades = read_booked(&auction).partner().length(Suit::Spades).min;
        set_kickback(false); // restore the default (off) for the rest of the suite
        assert!(spades < 4, "the relocated ask is not a natural spade suit");
    }

    /// The default-system twin of the kickback poison: the plain 1430 answers
    /// (5♣–5♠) and DOPI/ROPI/DEPO on X/XX are present in every stance and
    /// always alerted, so a **natural** floor 5♦ — no ask anywhere on the
    /// face — reads as a keycard answer: the union with the answer rules' ⊤
    /// projection erases partner's diamond floor and the `alerted` bit
    /// suppresses the natural walk.  The `Rules::face` gates confine the
    /// rules to a live ask window, so the natural reading survives.
    ///
    /// This was a differential test against `set_keycard_answer_gates`.  That
    /// knob is gone — its off arm was the poison itself, not an agreement any
    /// partnership could play — so the guard is now absolute: partner's
    /// diamond floor must not be erased.  Remove the gates and it goes to
    /// nothing, which is exactly the regression being pinned.
    #[test]
    fn answer_gates_spare_a_natural_five_diamonds() {
        use crate::bidding::instinct::set_kickback;
        // The plain arm on purpose (also the default): the poison this pins is
        // the *default system's* five-level answers, not the relocated
        // ladder's.
        set_kickback(false);
        let auction = [
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
            Call::Pass,
            bid(5, Strain::Diamonds),
            Call::Pass,
        ];
        let diamonds = read_booked(&auction).partner().length(Suit::Diamonds).min;
        set_kickback(false); // restore the default (off) for the rest of the suite
        assert!(
            diamonds >= 2,
            "a natural 5♦ with no ask anywhere on the face must keep its \
             diamond floor, got {diamonds}"
        );
    }

    /// The gates' positive control: inside a live ask window the answer is
    /// still alerted — a 5♦ answering 4NT is a keycard count, not diamonds.
    #[test]
    fn answer_gates_keep_the_live_window_alerted() {
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
            bid(4, Strain::Notrump),
            Call::Pass,
            bid(5, Strain::Diamonds),
            Call::Pass,
        ];
        // The gates are the default: the in-window answer must stay alerted.
        let diamonds = read_booked(&auction).partner().length(Suit::Diamonds).min;
        assert!(
            diamonds < 4,
            "the in-window answer is not a natural diamond suit"
        );
    }

    #[test]
    fn contested_transfer_lebensohl_direct_jacoby_over_2d() {
        // Over (2♦) the transfers are direct Jacoby: 3♦→♥.  [1NT, (2♦), 3♦, (X)].
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Diamonds),
            bid(3, Strain::Diamonds),
            Call::Double,
        ];
        let inf = read_booked(&auction);
        assert!(inf.partner().length(Suit::Hearts).min >= 5);
    }

    #[test]
    fn contested_transfer_lebensohl_cue_is_not_a_transfer() {
        // The cue of their suit is Stayman (a 4-card unbid major), not a 5+ transfer:
        // [1NT, (2♠), 3♠, (P)] projects hearts as only 4-card interest, and the
        // natural-spades reading of the cue is suppressed (not a long spade suit).
        let auction = [
            bid(1, Strain::Notrump),
            bid(2, Strain::Spades),
            bid(3, Strain::Spades),
            Call::Pass,
        ];
        let inf = read_booked(&auction);
        assert!(inf.partner().length(Suit::Hearts).min < 5);
        assert!(inf.partner().length(Suit::Spades).min < 5);
    }

    #[test]
    fn relative_seat_tracks_the_actor() {
        // The same 1♥ opening lands on a different relative seat as the
        // auction grows by one call.
        assert_eq!(
            read(&[bid(1, Strain::Hearts)]).rho().strength.points,
            Range::new(10, 21)
        );
        assert_eq!(
            read(&[bid(1, Strain::Hearts), Call::Pass])
                .partner()
                .strength
                .points,
            Range::new(10, 21)
        );
    }

    #[test]
    fn limited_notrump_rebids_narrow_strength() {
        // [1♦, P, 1♥, P, 1NT, P]: the opener (partner) showed a 12–16 minimum.
        let one_nt = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(one_nt.partner().strength.points, Range::new(12, 16));

        // A jump to 2NT is the strong 18–19 rebid (sound bound 18–21).
        let two_nt = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(two_nt.partner().strength.points, Range::new(18, 21));
    }

    #[test]
    fn cheapest_two_notrump_over_a_response_is_not_strong() {
        // [1♦, P, 2♣, P, 2NT, P]: 2NT is the *cheapest* notrump over a 2/1, a
        // minimum — it must not be read as the 18–19 jump.  Opener stays at the
        // opening floor (10–21).
        let inf = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().strength.points, Range::new(10, 21));
    }

    #[test]
    fn raises_and_one_notrump_response_narrow_the_responder() {
        // [1♥, P, 2♥, P]: a single raise is 6–10 — a support-scale band, so
        // the dedicated gauge carries it exactly and the legacy axis holds
        // only its sound image (4-point shapely raises are measured fact:
        // the `1♠ P 2♠` divergence-meter defect).
        let single = read(&[
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        let hearts = Suit::Hearts as usize;
        assert_eq!(
            single.partner().strength.support_points[hearts],
            Range::new(6, 10)
        );
        assert_eq!(single.partner().strength.points, Range::new(1, 11));
        assert_eq!(single.partner().strength.shown_floor(), 6);
        // [1♥, P, 3♥, P]: a limit (jump) raise is 10–12.
        let limit = read(&[
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(
            limit.partner().strength.support_points[hearts],
            Range::new(10, 12)
        );
        assert_eq!(limit.partner().strength.points, Range::new(5, 13));
        // [1♥, P, 1NT, P]: a 1NT response is 6–12.
        let one_nt = read(&[
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(one_nt.partner().strength.points, Range::new(6, 12));
    }

    #[test]
    fn competition_suppresses_the_limited_rebid_reading() {
        // [1♦, P, 1♥, 1♠, 1NT, P]: with the opponents in, opener's 1NT is not
        // the quiet 12–16 rebid — leave the strength at the opening floor
        // (10–21).
        let inf = read(&[
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(1, Strain::Spades),
            bid(1, Strain::Notrump),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().strength.points, Range::new(10, 21));
    }

    #[test]
    fn rubens_cue_raise_shows_support() {
        // (1♠) 2♣ (P) 2♠ (P): we overcalled 2♣, partner cue-raised 2♠ — a
        // limit-plus club raise.  The overcaller reads three-plus clubs and
        // ten-plus points, but no spade length (the cue is a relay).
        let inf = read(&[
            bid(1, Strain::Spades),
            bid(2, Strain::Clubs),
            Call::Pass,
            bid(2, Strain::Spades),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Clubs).min >= 3);
        // A support-scale promise: exact on the club slot, only its sound
        // image on the legacy axis.
        assert!(inf.partner().strength.support_points[Suit::Clubs as usize].min >= 10);
        assert!(inf.partner().strength.points.min >= 5);
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_transfer_is_not_read_as_natural() {
        // (1♣) 1♠ (P) 2♣ (P): we overcalled 1♠, partner transferred 2♣ (a relay
        // to diamonds).  The bid suit must not be read as a club holding.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_reading_respects_the_knob() {
        // With Rubens advances off — the default since the layer A/B — the same
        // 2♣ is a genuine club suit: the suppression lifts and it reads naturally.
        crate::bidding::instinct::set_rubens_advances(false);
        set_cue_reading(false);
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Clubs).min >= 4);
        set_cue_reading(true);
    }

    #[test]
    fn their_minor_cue_reads_as_michaels() {
        // (1♣) 2♣: the direct cue of their minor opening is Michaels — both
        // majors, five-five, and no club length (the probe caught a club void
        // read as five clubs).  Off, the old overcall reading returns.
        set_cue_reading(true);
        let inf = read(&[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]);
        assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
        assert!(inf.rho().length(Suit::Hearts).min >= 5);
        assert!(inf.rho().length(Suit::Spades).min >= 5);
        set_cue_reading(false);
        let off = read(&[bid(1, Strain::Clubs), bid(2, Strain::Clubs)]);
        assert!(off.rho().length(Suit::Clubs).min >= 5);
        set_cue_reading(true);
    }

    #[test]
    fn their_jump_cue_over_a_weak_two_is_leaping_michaels() {
        // (2♦) 4♦: the jump cue of a weak-two minor is Leaping Michaels — both
        // majors, no diamond length (the probe: a diamond void read as six).
        set_cue_reading(true);
        let inf = read(&[
            Call::Pass,
            bid(2, Strain::Diamonds),
            bid(4, Strain::Diamonds),
        ]);
        assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
        assert!(inf.rho().length(Suit::Hearts).min >= 5);
        assert!(inf.rho().length(Suit::Spades).min >= 5);
    }

    #[test]
    fn their_cue_of_our_overcall_is_a_raise() {
        // 1♥ (2♦) 3♦: responder's cue of the overcalled suit is the limit-plus
        // heart raise — three-plus hearts, ten-plus points, and no diamond
        // length (the probe: two diamonds read as four).
        set_cue_reading(true);
        let inf = read(&[
            Call::Pass,
            Call::Pass,
            bid(1, Strain::Hearts),
            bid(2, Strain::Diamonds),
            bid(3, Strain::Diamonds),
        ]);
        assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
        assert!(inf.rho().length(Suit::Hearts).min >= 3);
        assert!(inf.rho().strength.support_points[Suit::Hearts as usize].min >= 10);
        assert!(inf.rho().strength.points.min >= 5);
    }

    #[test]
    fn a_doublers_jump_is_not_a_weak_jump() {
        // 2♠ (X) P (3♦) P (4♥): the doubler's jump to game is strength, made
        // on as few as three hearts — never a weak six-card jump.
        set_length_soundness(true);
        let auction = [
            bid(2, Strain::Spades),
            Call::Double,
            Call::Pass,
            bid(3, Strain::Diamonds),
            Call::Pass,
            bid(4, Strain::Hearts),
        ];
        let inf = read(&auction);
        assert_eq!(inf.rho().length(Suit::Hearts), Range::FULL_LENGTH);
        set_length_soundness(false);
        let off = read(&auction);
        assert!(off.rho().length(Suit::Hearts).min >= 6);
        set_length_soundness(true);
    }

    #[test]
    fn an_agreed_suit_re_raise_adds_no_length() {
        // 1♥ (P) 2♥ (P) 3♥: opener's game-try re-raise of the agreed suit adds
        // no length — the five from the opening stands, not a phantom sixth.
        set_length_soundness(true);
        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Hearts),
        ];
        let inf = read(&auction);
        assert_eq!(inf.rho().length(Suit::Hearts).min, 5);
        set_length_soundness(false);
        let off = read(&auction);
        assert_eq!(off.rho().length(Suit::Hearts).min, 6);
        set_length_soundness(true);
    }

    #[test]
    fn opener_minor_rebid_reads_five_plus() {
        // 1♦ (P) 1♠ (P) 2♦: opener's two-level rebid of the opened minor is
        // routinely a good five-card suit, not six (the probe: five of eight
        // rebids were made on five).
        set_length_soundness(true);
        let auction = [
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Diamonds),
        ];
        let inf = read(&auction);
        assert_eq!(inf.rho().length(Suit::Diamonds).min, 5);
        set_length_soundness(false);
        let off = read(&auction);
        assert_eq!(off.rho().length(Suit::Diamonds).min, 6);
        set_length_soundness(true);
    }

    #[test]
    fn their_splinter_is_disclosed_to_the_table() {
        // 1♠ (P) 4♦ read by a defender: their splinter is alerted and
        // explained at the table, so it decodes off their authoring rule —
        // diamond shortness with spade support, never diamond length.
        set_table_alert_reading(true);
        let auction = [bid(1, Strain::Spades), Call::Pass, bid(4, Strain::Diamonds)];
        let inf = read_booked(&auction);
        assert!(inf.rho().length(Suit::Diamonds).max <= 1);
        set_table_alert_reading(false);
        let off = read_booked(&auction);
        assert_eq!(off.rho().length(Suit::Diamonds).max, 13);
    }

    #[test]
    fn their_michaels_is_disclosed_to_the_table() {
        // 1♠ (2♠) read by the opening side: their Michaels cue resolves in
        // *their* phase-routed book (defensive at their turn) and decodes off
        // the authored rule — five-plus hearts *with the rule's strength
        // floor*, which the retired `two_suiter_reading` never knew (chop 1,
        // `docs/reader-retirement.md`).  This knob is now the only owner of
        // the reading, so its off arm is the honest record of what the
        // retirement gives up: the shape floor goes too.
        set_table_alert_reading(true);
        let auction = [bid(1, Strain::Spades), bid(2, Strain::Spades)];
        let inf = read_booked(&auction);
        assert!(inf.rho().length(Suit::Hearts).min >= 5);
        assert!(inf.rho().strength.points.min >= 8);
        assert_eq!(inf.rho().length(Suit::Spades).min, 0);
        set_table_alert_reading(false);
        let off = read_booked(&auction);
        assert_eq!(off.rho().strength.points.min, 0);
        assert_eq!(off.rho().length(Suit::Hearts).min, 0);
        set_table_alert_reading(true);
    }

    #[test]
    fn their_checkback_is_disclosed_to_the_table() {
        // 1♦ (P) 1♠ (P) 1NT (P) 2♣ read by a defender: their artificial
        // checkback 2♣ promises no clubs — the natural walk floored four (the
        // probe: four-plus clubs read on a singleton).
        set_table_alert_reading(true);
        let auction = [
            bid(1, Strain::Diamonds),
            Call::Pass,
            bid(1, Strain::Spades),
            Call::Pass,
            bid(1, Strain::Notrump),
            Call::Pass,
            bid(2, Strain::Clubs),
        ];
        let inf = read_booked(&auction);
        assert!(inf.rho().length(Suit::Clubs).min < 4);
        set_table_alert_reading(false);
        let off = read_booked(&auction);
        assert!(off.rho().length(Suit::Clubs).min >= 4);
        set_table_alert_reading(true);
    }

    #[test]
    fn rubens_limit_raise_transfer_records_support() {
        crate::bidding::instinct::set_rubens_advances(true);
        // (1♣) 1♠ (P) 2♥ (P): partner's transfer into our spades is the
        // limit-plus raise — the overcaller reads three-plus spades and
        // ten-plus points, while the named hearts stay unread (a relay).
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Spades).min >= 3);
        assert!(inf.partner().strength.points.min >= 10);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_new_suit_transfer_records_the_target() {
        crate::bidding::instinct::set_rubens_advances(true);
        // (1♣) 1♠ (P) 2♣ (P): the new-suit transfer shows the advancer's own
        // five-card diamond suit and ten-plus points; clubs stay unread.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Clubs),
            Call::Pass,
        ]);
        assert!(inf.partner().length(Suit::Diamonds).min >= 5);
        assert!(inf.partner().strength.points.min >= 10);
        assert_eq!(inf.partner().length(Suit::Clubs), Range::FULL_LENGTH);
    }

    #[test]
    fn rubens_transfer_records_despite_intervention() {
        crate::bidding::instinct::set_rubens_advances(true);
        // (1♣) 1♠ (P) 2♥ (X): opener doubles the transfer — the completion
        // never comes, but the shown limit raise is exactly what the
        // overcaller needs for the competitive decision.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Double,
        ]);
        assert!(inf.partner().length(Suit::Spades).min >= 3);
        assert!(inf.partner().strength.points.min >= 10);
    }

    #[test]
    fn rubens_transfer_is_not_read_for_the_opponents() {
        // Same auction read from the opening side (the advancer is now our
        // LHO): the opponents' agreement is not assumed — an in-band advance
        // from the other side may be a genuine suit, so nothing is recorded.
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
            Call::Pass,
        ]);
        assert_eq!(inf.lho().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(inf.lho().strength.points, Range::FULL_POINTS);
    }

    /// Their Michaels cue of our opened major, post-retirement (chop 1)
    ///
    /// The reading now comes from the authored `.alert(MICHAELS)` rule's own
    /// projection, so the auction must be read **keyed** (`read_booked`) and
    /// the knob that owns the reading is `set_table_alert_reading`, not
    /// `set_uvu_over_majors` (which kept only its book half).  The projection
    /// also carries the rule's strength floor, which the retired reader never
    /// did.
    #[test]
    fn michaels_cue_over_our_major_reads_the_other_major() {
        // [1♥, (2♥)]: their direct cue of our opened major is Michaels — 5+
        // spades with the rule's 8+ floor, and NOT a natural heart suit (the
        // walk's misread suppressed by the alert).
        let inf = read_booked(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
        assert!(inf.rho().length(Suit::Spades).min >= 5, "the shown major");
        assert!(inf.rho().strength.points.min >= 8, "the rule's floor");
        assert_eq!(
            inf.rho().length(Suit::Hearts),
            Range::FULL_LENGTH,
            "the cue is not natural hearts"
        );

        // Table-wide disclosure and the shipped cue reading both off: the
        // pre-package natural reading is preserved verbatim.
        set_table_alert_reading(false);
        set_cue_reading(false);
        let inf = read_booked(&[bid(1, Strain::Hearts), bid(2, Strain::Hearts)]);
        assert!(inf.rho().length(Suit::Hearts).min >= 5);
        assert_eq!(inf.rho().length(Suit::Spades), Range::FULL_LENGTH);
        set_cue_reading(true);
        set_table_alert_reading(true);
    }

    /// Their unusual `(2NT)` over our major, post-retirement (chop 1) — as
    /// above, but the authored rule is a single box, so it pins both minors
    /// *and* the strength floor.
    #[test]
    fn unusual_2nt_over_our_major_reads_both_minors() {
        let inf = read_booked(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
        assert!(inf.rho().length(Suit::Clubs).min >= 5);
        assert!(inf.rho().length(Suit::Diamonds).min >= 5);
        assert!(inf.rho().strength.points.min >= 8, "the rule's floor");

        // Table-wide disclosure off: nothing recorded for their 2NT (a notrump
        // bid never entered the natural suit walk either).
        set_table_alert_reading(false);
        let inf = read_booked(&[bid(1, Strain::Spades), bid(2, Strain::Notrump)]);
        assert_eq!(inf.rho().length(Suit::Clubs), Range::FULL_LENGTH);
        assert_eq!(inf.rho().length(Suit::Diamonds), Range::FULL_LENGTH);
        assert_eq!(inf.rho().strength.points, Range::FULL_POINTS);
        set_table_alert_reading(true);
    }

    /// The retirement guard for chop 1 (`docs/reader-retirement.md`)
    ///
    /// `two_suiter_reading` claimed `other_major >= 5` for their Michaels cue
    /// and `♣ >= 5 && ♦ >= 5` for their unusual `(2NT)`.  Every one of those
    /// claims is a **subset** of the authoring rule's projection on every
    /// auction the reader used to fire on — both seat-fans of the opening and
    /// both reading seats (the opponents' call decoded by the table-alert
    /// walk, and the same call decoded own-side at the advancer's turn) — and
    /// the projection adds the rule's `points >= 8` on top.  That subset
    /// property is why the chop needed no A/B: the reader's `narrow_length`
    /// was already an idempotent intersect against a hull folded in before it.
    #[test]
    fn retired_two_suiter_reader_is_subsumed_by_the_projection() {
        let michaels: [(&[Call], Relative); 3] = [
            (
                &[bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
                Relative::Rho,
            ),
            (
                &[Call::Pass, bid(1, Strain::Hearts), bid(2, Strain::Hearts)],
                Relative::Rho,
            ),
            // The advancer's turn: index 1 is now our own side, decoded by the
            // exact-node walk rather than the table-alert one.
            (
                &[bid(1, Strain::Hearts), bid(2, Strain::Hearts), Call::Pass],
                Relative::Partner,
            ),
        ];
        for (auction, who) in michaels {
            let inf = read_booked(auction);
            let shown = inf.get(who);
            assert!(
                shown.length(Suit::Spades).min >= 5,
                "{auction:?}: the retired reader's other-major floor"
            );
            assert!(
                shown.strength.points.min >= 8,
                "{auction:?}: the floor the reader never carried"
            );
            assert_eq!(
                shown.length(Suit::Hearts),
                Range::FULL_LENGTH,
                "{auction:?}: the cue is not natural hearts"
            );
        }

        let unusual: [(&[Call], Relative); 2] = [
            (
                &[bid(1, Strain::Spades), bid(2, Strain::Notrump)],
                Relative::Rho,
            ),
            (
                &[bid(1, Strain::Spades), bid(2, Strain::Notrump), Call::Pass],
                Relative::Partner,
            ),
        ];
        for (auction, who) in unusual {
            let inf = read_booked(auction);
            let shown = inf.get(who);
            assert!(
                shown.length(Suit::Clubs).min >= 5,
                "{auction:?}: the retired reader's club floor"
            );
            assert!(
                shown.length(Suit::Diamonds).min >= 5,
                "{auction:?}: the retired reader's diamond floor"
            );
            assert!(
                shown.strength.points.min >= 8,
                "{auction:?}: the floor the reader never carried"
            );
        }
    }

    #[test]
    fn uvu_major_cue_projects_the_raise() {
        use crate::bidding::american::set_uvu_over_majors;

        // [1♥, (2NT), 3♣, (P)] from opener's seat: partner's cheap cue is the
        // alerted limit-plus raise — decoded off its authored rule's
        // projection (3+ hearts, 10+), not as natural clubs.
        set_uvu_over_majors(true);
        let inf = read_booked(&[
            bid(1, Strain::Hearts),
            bid(2, Strain::Notrump),
            bid(3, Strain::Clubs),
            Call::Pass,
        ]);
        let cue_bidder = inf.partner();
        assert!(
            cue_bidder.length(Suit::Hearts).min >= 3,
            "the projected fit"
        );
        assert!(
            cue_bidder.strength.points.min >= 10,
            "the projected strength"
        );
        assert_eq!(
            cue_bidder.length(Suit::Clubs),
            Range::FULL_LENGTH,
            "not natural clubs"
        );
    }

    #[test]
    fn rubens_transfer_reading_knob_recovers_suppress_only() {
        crate::bidding::instinct::set_rubens_advances(true);
        // Stage-2 knob off: the transfer is still suppressed (not natural
        // hearts) but records nothing — the pre-fix shape.
        set_rubens_transfer_reading(false);
        let inf = read(&[
            bid(1, Strain::Clubs),
            bid(1, Strain::Spades),
            Call::Pass,
            bid(2, Strain::Hearts),
            Call::Pass,
        ]);
        assert_eq!(inf.partner().length(Suit::Spades), Range::FULL_LENGTH);
        assert_eq!(inf.partner().length(Suit::Hearts), Range::FULL_LENGTH);
        assert_eq!(inf.partner().strength.points, Range::FULL_POINTS);
        set_rubens_transfer_reading(true);
    }

    /// D1c: knob-on hygiene drops sum-infeasible ghosts and contained boxes,
    /// leaving the union exact and short.
    #[test]
    fn tidy_prunes_ghosts_and_contained() {
        use crate::bidding::constraint::{Constraint as _, and, balanced, points};

        set_dnf_reading(true);
        let context = Context::new(RelativeVulnerability::NONE, &[]);

        // `balanced & {3..}⁴`: the four 5(332) pan-handles intersect to
        // sum-infeasible 5-3-3-3 ghosts; only the {3..=4}⁴ flat cube survives.
        let flat = (balanced() & and(Suit::ASC, 3..)).project_band(&context);
        let mut expected = Envelope::unknown();
        expected.lengths = [Range::new(3, 4); 4];
        assert_eq!(flat.boxes(), &[expected]);

        // A strength-only `Or` duplicates the five shape boxes across its two
        // arms; the wider-points copy encloses the narrower, so five remain.
        let dup = (balanced() & (points(8..) | points(10..))).project_band(&context);
        assert_eq!(dup.boxes().len(), 5);

        set_dnf_reading(true);
    }

    /// The 560 ordered shapes — every 4-tuple of suit lengths summing to 13.
    fn all_shapes() -> Vec<[u8; 4]> {
        (0..=13u8)
            .flat_map(|a| {
                (0..=13 - a)
                    .flat_map(move |b| (0..=13 - a - b).map(move |c| [a, b, c, 13 - a - b - c]))
            })
            .collect()
    }

    fn shape_fits(lengths: &[Range; 4], shape: &[u8; 4]) -> bool {
        lengths
            .iter()
            .zip(shape)
            .all(|(range, &len)| range.contains(len))
    }

    /// C1: `narrow_to_sum` is **exact** — every narrowed bound is attained by a
    /// real 13-card shape inside the box — and **membership-inert**: the same
    /// shapes lie in the box before and after.  Idempotent, too.
    #[test]
    fn sum_closure_is_exact_and_inert() {
        let shapes = all_shapes();
        assert_eq!(shapes.len(), 560);

        // Deterministic xorshift — the point is coverage, not randomness.
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut tested = 0_u32;
        for _ in 0..8000 {
            let mut lengths = [Range::FULL_LENGTH; 4];
            for range in &mut lengths {
                let min = u8::try_from(next() % 8).expect("under 8");
                let max = min + u8::try_from(next() % u64::from(14 - min)).expect("under 14");
                *range = Range::new(min, max);
            }
            let mut envelope = Envelope::unknown();
            envelope.lengths = lengths;
            if !envelope.sum_feasible() {
                continue;
            }
            tested += 1;

            let inside: Vec<_> = shapes.iter().filter(|s| shape_fits(&lengths, s)).collect();
            assert!(
                !inside.is_empty(),
                "sum-feasible box {lengths:?} holds no shape"
            );
            envelope.narrow_to_sum();

            for (suit, range) in envelope.lengths.iter().enumerate() {
                let low = inside.iter().map(|s| s[suit]).min().expect("nonempty");
                let high = inside.iter().map(|s| s[suit]).max().expect("nonempty");
                assert_eq!(
                    (range.min, range.max),
                    (low, high),
                    "suit {suit} of {lengths:?} narrowed to {range:?}, truth {low}..={high}"
                );
            }
            assert!(
                shapes
                    .iter()
                    .all(|s| shape_fits(&lengths, s) == shape_fits(&envelope.lengths, s)),
                "closure moved membership on {lengths:?}"
            );

            let once = envelope.lengths;
            envelope.narrow_to_sum();
            assert_eq!(envelope.lengths, once, "not idempotent on {lengths:?}");
        }
        assert!(tested > 1000, "only {tested} feasible boxes sampled");
    }

    /// C2: a box whose lengths force balanced reads `points == hcp`, because a
    /// balanced hand never upgrades.  Knob-off the HCP floor carries the
    /// scale's *global* worst-case slack instead.
    #[test]
    fn upgrade_closure_crisps_the_balanced_band() {
        use crate::bidding::constraint::{Constraint as _, balanced, points};

        set_dnf_reading(true);
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let read_hcp = |on: bool| {
            set_upgrade_closure(on);
            let dnf = (balanced() & points(15..)).project(&context);
            set_upgrade_closure(false);
            dnf.hull().strength.hcp
        };

        assert_eq!(read_hcp(false), Range::new(13, Range::FULL_POINTS.max));
        assert_eq!(read_hcp(true), Range::new(15, Range::FULL_POINTS.max));
    }

    /// C2 is **not** membership-inert, unlike C1: it derives a bound on
    /// `points` — an axis `admits` tests — from `hcp`, an axis it does not
    /// (the write-only axis; see [`set_gauge_membership`]).  So the closure
    /// gives an otherwise unenforced HCP claim teeth *through* `points`.
    ///
    /// Found by `examples/probe-closure-features.rs`, which cross-tested
    /// sampled layouts against the other arm's reading: C1 rejected 0 of
    /// 409,708, C2 rejected 249 of 8,576.  The narrowing is exact relative to
    /// what the box *claims*; it is the sampler's acceptance that widens
    /// without it.
    #[test]
    fn upgrade_closure_gives_hcp_teeth() {
        use crate::bidding::constraint::{Constraint as _, balanced, hcp};

        // Flat 4333, 10 raw HCP: balanced ⇒ no upgrade ⇒ `points` == `hcp`.
        // Outside the `hcp(..=8)` claim, yet the loose reading admits it,
        // because `points` was slacked to `hcp + hcp_ceiling_slack()`.
        let hand: Hand = "AKQ2.J43.432.432".parse().expect("valid hand");
        set_dnf_reading(true);
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let reading = (balanced() & hcp(..=8)).project_band(&context);

        assert!(reading.clone().tidy().contains(hand));
        set_upgrade_closure(true);
        assert!(!reading.tidy().contains(hand));
        set_upgrade_closure(false);
    }

    /// Chop E: `set_gauge_membership` gives the raw-HCP and support-points
    /// bands membership teeth; off (the default) they are inert.
    #[test]
    fn gauge_membership_teeth() {
        // 15 raw HCP, flat 4333 (no upgrade on any scale).
        let hand: Hand = "AKQ2.K53.QJ4.T92".parse().expect("valid hand");
        let mut envelope = Envelope::unknown();
        envelope.strength.hcp = Range::new(16, 17);

        // Off: the `points` gauge alone doesn't exclude it…
        assert!(envelope.admits(hand));

        // …on: the raw-HCP band does, and widening the band re-admits.
        set_gauge_membership(true);
        assert!(!envelope.admits(hand));
        envelope.strength.hcp = Range::new(15, 17);
        assert!(envelope.admits(hand));
        envelope.strength.support_points = [Range::new(16, 37); 4];
        assert!(!envelope.admits(hand));
        set_gauge_membership(false);
    }

    #[test]
    fn range_intersect_widens_on_conflict() {
        // Disjoint ranges cannot both hold; widen to the union, never empty.
        assert_eq!(
            Range::new(5, 13).intersect(Range::new(6, 13)),
            Range::new(6, 13)
        );
        assert_eq!(
            Range::new(0, 3).intersect(Range::new(6, 13)),
            Range::new(0, 13)
        );
    }

    /// Walk every authored rule of a book trie under its authoring-time context
    ///
    /// The shared chassis of the book-wide invariant tests below: iterate the
    /// trie's `(auction, classifier)` nodes, skip non-rule classifiers, build the
    /// node's [`Context`] (with common prefixes), and visit each rule.
    fn for_each_authored_rule(
        trie: &crate::bidding::trie::Trie,
        mut visit: impl FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
    ) {
        for (auction, classifier) in trie {
            let auction: &[Call] = &auction;
            let Some(rules) = classifier.as_rules() else {
                continue;
            };
            let context = Context::new(RelativeVulnerability::NONE, auction)
                .with_prefixes(trie.common_prefixes(auction));
            for rule in rules.rules() {
                visit(auction, &context, rule);
            }
        }
    }

    /// The fallback sibling of [`for_each_authored_rule`]: walk every authored
    /// rule wired through a guarded [`Fallback::Classify`][crate::bidding::fallback::Fallback]
    ///
    /// Iterates [`Trie::fallbacks`][crate::bidding::trie::Trie::fallbacks],
    /// keeps the classifiers that expose authored
    /// [`Rules`][crate::bidding::rules::Rules] via `as_rules`, and visits each
    /// rule under the **node-key context** — the same authoring-time
    /// approximation the exact-node chassis makes (the fallback actually fires
    /// on longer auctions; the sniffer's `claims()` filters already exclude
    /// context-dependent atoms).  Classifiers with `as_rules() == None` are
    /// reported to `opaque` with their guard label: that list is the residue no
    /// rule walk can meter, and the conversion worklist for the pass-reading
    /// campaign (`docs/ai-bidder/sampled-projection.md`).
    fn for_each_fallback_rule(
        trie: &crate::bidding::trie::Trie,
        mut visit: impl FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
        mut opaque: impl FnMut(&[Call], Option<String>),
    ) {
        for (auction, guard, fallback) in trie.fallbacks() {
            let crate::bidding::fallback::Fallback::Classify(classifier) = fallback else {
                continue;
            };
            let auction: &[Call] = &auction;
            let Some(rules) = classifier.as_rules() else {
                opaque(auction, guard.describe());
                continue;
            };
            let context = Context::new(RelativeVulnerability::NONE, auction)
                .with_prefixes(trie.common_prefixes(auction));
            for rule in rules.rules() {
                visit(auction, &context, rule);
            }
        }
    }

    /// The alert-invariant worklist for one trie: rules whose projection the
    /// structural [`artificial`] detector flags but which carry no `.alert(...)`
    ///
    /// Walks under the **legacy hull projection** (`set_dnf_reading(false)`):
    /// the detector's "floors a suit it did not name" reading was defined
    /// against hulls, and knob-on box unions (the fit-split's major floors,
    /// `dnf_upgrade` boxes) legitimately carry other-suit information that
    /// would false-positive it.
    fn unalerted_artificial(label: &str, trie: &crate::bidding::trie::Trie) -> Vec<String> {
        set_dnf_reading(false);
        let mut worklist = Vec::new();
        for_each_authored_rule(trie, |auction, context, rule| {
            let made = rule.call();
            let doubled = context.last_bid().map(|last| last.strain);
            if super::artificial(&rule.project(context), made, doubled) && rule.alert().is_none() {
                worklist.push(format!(
                    "{label}: [{}] {made}  (label: {:?})",
                    auction
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                    rule.label(),
                ));
            }
        });
        set_dnf_reading(true);
        worklist
    }

    /// Assert an alert worklist is empty, listing the offenders
    fn assert_all_alerted(what: &str, mut worklist: Vec<String>) {
        worklist.sort();
        worklist.dedup();
        assert!(
            worklist.is_empty(),
            "{} {what} artificial calls lack an alert:\n{}",
            worklist.len(),
            worklist.join("\n"),
        );
    }

    /// Retirement invariant for [`artificial`]: every call the structural
    /// detector would read as artificial is *also* alerted by its authoring rule.
    ///
    /// `artificial(project(rule), call) ⟹ rule.alert().is_some()`, walked over
    /// every authored rule in the shipped `american()` book (all three phase
    /// tries).  This now holds with zero counterexamples, so `|| artificial(p,
    /// made)` has been dropped from the decode gate: alerts alone carry the "decode
    /// this call" signal (alert-by-disclosed-meaning, the move modern bridge made
    /// retiring "X is self-alerting").
    ///
    /// Kept as a **permanent regression guard**: a future artificial bid added
    /// without an `.alert(...)` makes this fail (the panic lists the exact call),
    /// rather than silently losing its decoding now that the structural fallback is
    /// gone.
    #[test]
    fn artificial_calls_are_alerted() {
        use crate::bidding::american::american;

        let pair = american();
        let mut worklist = Vec::new();
        for (phase, trie) in [
            ("constructive", &pair.constructive.0),
            ("competitive", &pair.competitive.0),
            ("defensive", &pair.defensive.0),
        ] {
            worklist.extend(unalerted_artificial(phase, trie));
        }
        assert_all_alerted("american", worklist);
    }

    /// The same invariant over the queen relay's own nodes
    ///
    /// The relay's rungs land on 5NT and 6♣–6♥ — ordinary contracts the
    /// general sweep reaches by other routes — so this walks the relay's own
    /// tables directly rather than trusting that sweep to have covered them.
    #[test]
    fn queen_relay_calls_are_alerted() {
        use crate::bidding::american::american;

        let pair = american();
        let mut worklist = Vec::new();
        for (phase, trie) in [
            ("constructive", &pair.constructive.0),
            ("competitive", &pair.competitive.0),
            ("defensive", &pair.defensive.0),
        ] {
            worklist.extend(unalerted_artificial(phase, trie));
        }
        assert_all_alerted("american + queen relay", worklist);
    }

    #[test]
    fn deviation_knobs_preserve_alert_invariant() {
        use crate::bidding::american::{
            american, set_one_notrump_offshape, set_overcall_four_card, set_weak_two_wild,
        };

        set_one_notrump_offshape(true);
        set_overcall_four_card(true);
        set_weak_two_wild(true);
        let pair = american();
        set_one_notrump_offshape(false);
        set_overcall_four_card(false);
        set_weak_two_wild(false);

        let mut worklist = Vec::new();
        for (phase, trie) in [
            ("constructive", &pair.constructive.0),
            ("competitive", &pair.competitive.0),
            ("defensive", &pair.defensive.0),
        ] {
            worklist.extend(unalerted_artificial(phase, trie));
        }
        assert_all_alerted("american deviation knobs", worklist);
    }

    /// Disclosure tripwire: the alerted call sites of the default `american()`
    /// book, counted per alert slug, against `tests/fixtures/alert-sites.txt`
    ///
    /// [`card`][crate::bidding::card] generates our `.bbsa` disclosure from the
    /// live knob state, so a row that *has* a knob can no longer drift.  What
    /// generation cannot catch is authoring a convention and never giving it a
    /// row at all — the card then silently under-describes us to BBA.  This is
    /// the artifact that fires on that: any new (or deleted) alerted rule moves
    /// a count, and the failure sends the author to the generator.
    ///
    /// Counts, not the call-site list: the list runs to four figures and would
    /// make every unrelated node edit an unreviewable diff, which is how a
    /// fixture degrades into a rubber stamp.  Counts are also the granularity
    /// that *works* — `Alert("splinter")` is shared by the major-raise splinter
    /// and the 1NT splinter, so the slug **set** was unchanged when
    /// `set_nt_splinter` shipped, and only the count moved.
    #[test]
    fn alerted_call_sites_match_the_disclosure_fixture() {
        use crate::bidding::american::american;
        use std::collections::BTreeMap;

        let pair = american();
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for trie in [&pair.constructive.0, &pair.competitive.0, &pair.defensive.0] {
            for_each_authored_rule(trie, |_auction, _context, rule| {
                if let Some(alert) = rule.alert() {
                    *counts.entry(alert.0).or_default() += 1;
                }
            });
        }
        let found = counts
            .iter()
            .map(|(slug, count)| format!("{slug} {count}\n"))
            .collect::<String>();
        assert_eq!(
            found,
            include_str!("../../tests/fixtures/alert-sites.txt"),
            "the book's alerted call sites moved.  If you authored or retired a \
             convention, give it a row in `src/bidding/card.rs` (or record there \
             why BBA's schema cannot express it), then bless this fixture:\n\n{found}",
        );
    }

    /// Per-column reading-leak lists over a set of book tries
    ///
    /// A **leak** is an authored rule whose [`Constraint::describe`] names an
    /// axis while **no box** of its [`Rule::project_band_dnf`] band constrains
    /// that axis.  Per-box (not hull) on purpose: a disjunction that constrains
    /// the axis in every arm — the fit-split's `points | support points` — is a
    /// *sound* reading knob-on even though its hull is full, but knob-off the
    /// band is a single hull box, so the same predicate degenerates to the
    /// original hull check.
    ///
    /// Columns: one per strength gauge (`HCP`, `points`, `support points` —
    /// each noun checked against **its own** gauge), `length` (suit-symbol
    /// atoms), `suit HCP` ("HCP in ♠" atoms against the per-suit HCP axis),
    /// and `support` ("card support for partner", resolved through
    /// [`Context::partner_last_suit`]).
    ///
    /// "Names an axis" is sniffed off the rendered atoms — `describe_int_range`
    /// puts the noun last, so the describe strings are **load-bearing test
    /// infrastructure**: reword a noun and this sniffer must follow.  The
    /// exclusions that keep the signal usable: per-suit gauges read "… in ♠"
    /// (excluded from `length`; "HCP in ♠" meters on its own `suit HCP`
    /// column), partner-facing atoms end in "partner"
    /// (excluded from every gauge column), vacuous `0+` floors are ⊤
    /// *correctly*, and `points` awards an atom to the most specific noun
    /// (`support points` is not a `points` claim).
    /// The rule walk `axis_leaks_with` meters over — exact-node or fallback
    type RuleWalk = fn(
        &crate::bidding::trie::Trie,
        &mut dyn FnMut(&[Call], &Context<'_>, &crate::bidding::rules::Rule),
    );

    fn axis_leaks(
        tries: &[(&str, &crate::bidding::trie::Trie)],
    ) -> std::collections::BTreeMap<&'static str, Vec<String>> {
        axis_leaks_with(tries, |trie, visit| for_each_authored_rule(trie, visit))
    }

    fn axis_leaks_with(
        tries: &[(&str, &crate::bidding::trie::Trie)],
        walk: RuleWalk,
    ) -> std::collections::BTreeMap<&'static str, Vec<String>> {
        use crate::bidding::constraint::Description;

        /// Flatten a description tree into its leaf atoms.
        fn atoms(description: &Description, out: &mut Vec<String>) {
            match description {
                Description::Atom(text) => out.push(text.to_string()),
                Description::Not(inner) => atoms(inner, out),
                Description::All(parts) | Description::Any(parts) => {
                    for part in parts {
                        atoms(part, out);
                    }
                }
                Description::Opaque => {}
            }
        }

        /// A non-vacuous claim of `noun`: `describe_int_range` puts the noun last.
        fn claims(atom: &str, noun: &str) -> bool {
            atom.ends_with(noun) && !atom.starts_with("0+")
        }

        let mut leaks = std::collections::BTreeMap::<&'static str, Vec<String>>::new();
        for &(system, trie) in tries {
            walk(trie, &mut |_, context, rule| {
                let mut leaves = Vec::new();
                atoms(&rule.describe(), &mut leaves);
                let band = rule.project_band_dnf(context);
                let boxes = band.boxes();
                let text = leaves.join(" | ");
                let entry = format!("{system}: {} :: {text}", rule.call());

                type Vacuous = fn(&Strength) -> bool;
                let gauges: [(&'static str, Vacuous); 3] = [
                    ("HCP", |s| s.hcp == Range::FULL_POINTS),
                    ("points", |s| s.points == Range::FULL_POINTS),
                    ("support points", |s| {
                        s.support_points
                            .iter()
                            .all(|slot| *slot == Range::FULL_POINTS)
                    }),
                ];
                for (noun, vacuous) in gauges {
                    let named = leaves.iter().any(|atom| {
                        claims(atom, noun) && (noun != "points" || !claims(atom, "support points"))
                    });
                    if named && boxes.iter().all(|b| vacuous(&b.strength)) {
                        leaks.entry(noun).or_default().push(entry.clone());
                    }
                }

                for suit in Suit::ASC {
                    let symbol = suit.to_string();
                    let named = leaves.iter().any(|atom| {
                        claims(atom, &symbol)
                            // Per-suit gauges read "… in ♠" and meter on their
                            // own columns; "partner's last suit is ♠" is a
                            // *context* claim, not a hand one; "≤13 ♠" is a
                            // deliberate no-op cap (`len(x, ..14)` for gating
                            // symmetry) — all vacuous on the length axis.
                            && !atom.contains(" in ")
                            && !atom.contains("last suit is")
                            && !atom.starts_with("≤13 ")
                    });
                    if named && boxes.iter().all(|b| b.length(suit) == Range::FULL_LENGTH) {
                        leaks
                            .entry("length")
                            .or_default()
                            .push(format!("{system}: {symbol} {} :: {text}", rule.call()));
                        break;
                    }
                }

                for suit in Suit::ASC {
                    let noun = format!("HCP in {suit}");
                    let named = leaves.iter().any(|atom| claims(atom, &noun));
                    if named
                        && (boxes.iter())
                            .all(|b| b.strength.suit_hcp[suit as usize] == Range::FULL_SUIT_HCP)
                    {
                        leaks
                            .entry("suit HCP")
                            .or_default()
                            .push(format!("{system}: {suit} {} :: {text}", rule.call()));
                        break;
                    }
                }

                if let Some(suit) = context.partner_last_suit() {
                    let named = leaves
                        .iter()
                        .any(|atom| claims(atom, "card support for partner"));
                    if named && boxes.iter().all(|b| b.length(suit) == Range::FULL_LENGTH) {
                        leaks.entry("support").or_default().push(entry.clone());
                    }
                }
            });
        }
        for column in leaks.values_mut() {
            column.sort();
            column.dedup();
        }
        leaks
    }

    /// E0: book-wide soundness — a finite `eval` implies strict membership of
    /// the knob-on projection, forward and band, for every authored rule of
    /// the shipped systems.
    ///
    /// This is the safety net under the whole DNF wave: every projection
    /// upgrade (complement halves, De Morgan, shape unions, `Support`'s
    /// forward box, `tidy`'s pruning) claims *at most* what its gate enforces,
    /// and here each claim is replayed against random hands — a hand the rule
    /// accepts must lie in some box of the rule's own reading, on **every**
    /// gauge ([`Envelope::accepts`]).  A few extreme hands ride along to probe
    /// the gauge ceilings (a 37-HCP maximum, a 13-0-0-0 freak).
    #[test]
    fn authored_rules_eval_within_projection() {
        use crate::bidding::american::american;
        use crate::bidding::dutch::dutch;
        use rand::SeedableRng as _;

        // ponytail: 128 hands keeps the sweep under ~10s in the default test
        // run; the deep-auction rules re-walk `Inferences::read` per eval and
        // dominate the cost.  Crank the pool when hunting a specific leak.
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xE0);
        let mut hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
            .take(128)
            .collect();
        hands.extend(
            [
                "AKQJ.AKQJ.AKQ.AK",
                "AKQJT98765432...",
                "..AKQJT98765432.",
                "AKQ2.K53.QJ4.T92",
            ]
            .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
        );

        set_dnf_reading(true);
        let american = american();
        let dutch = dutch();
        let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
            ("american constructive", &american.constructive.0),
            ("american competitive", &american.competitive.0),
            ("american defensive", &american.defensive.0),
            ("dutch constructive", &dutch.constructive.0),
        ];

        fn check(
            failures: &mut Vec<String>,
            hands: &[Hand],
            system: &str,
            auction: &[Call],
            context: &Context<'_>,
            rule: &crate::bidding::rules::Rule,
        ) {
            let forward = rule.project_dnf(context);
            let band = rule.project_band_dnf(context);
            for &hand in hands {
                if !rule.eval(hand, context).is_finite() {
                    continue;
                }
                for (fold, dnf) in [("project", &forward), ("band", &band)] {
                    if !dnf.boxes().iter().any(|envelope| envelope.accepts(hand))
                        && failures.len() < 16
                    {
                        failures.push(format!(
                            "{system}: [{}] {} {fold} excludes accepted hand {hand}",
                            auction
                                .iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>()
                                .join(" "),
                            rule.call(),
                        ));
                    }
                }
            }
        }

        let mut failures: Vec<String> = Vec::new();
        for (system, trie) in tries {
            for_each_authored_rule(trie, |auction, context, rule| {
                check(&mut failures, &hands, system, auction, context, rule);
            });
            // The same soundness claim for fallback-authored rules — the layer
            // the exact-node walk cannot see (`docs/ai-bidder/sampled-projection.md`
            // census: the meter blind spot).  Asserts, not pins: soundness has
            // no acceptable nonzero.
            for_each_fallback_rule(
                trie,
                |auction, context, rule| {
                    check(&mut failures, &hands, system, auction, context, rule);
                },
                |_, _| {},
            );
        }

        assert!(
            failures.is_empty(),
            "unsound projections (eval ⊄ reading):\n{}",
            failures.join("\n"),
        );
    }

    /// Pass-exclusion soundness: wherever a table's argmax is (or ties with)
    /// Pass, the knob-on pass projection must admit the hand.
    ///
    /// [`authored_rules_eval_within_projection`] replays each rule against its
    /// *own* reading; the exclusion reading is a claim about the **table** —
    /// "no passer holds a hand a strictly-heavier sibling gate accepts" — so
    /// this sweep replays the argmax itself.  Ties count as passes (stricter
    /// than the drivers, whose `max_by` keeps the later call), which is why
    /// the exclusion threshold is a strict `>` on weight.
    #[test]
    fn passes_read_within_their_table() {
        use crate::bidding::american::american;
        use crate::bidding::dutch::dutch;
        use crate::bidding::trie::Classifier as _;
        use rand::SeedableRng as _;

        let mut rng = rand::rngs::StdRng::seed_from_u64(0x9A55);
        let mut hands: Vec<Hand> = crate::bidding::verify::random_hands(&mut rng)
            .take(128)
            .collect();
        hands.extend(
            ["AKQJ.AKQJ.AKQ.AK", "AKQ2.K53.QJ4.T92"]
                .map(|text| text.parse::<Hand>().unwrap_or_else(|_| unreachable!())),
        );

        set_dnf_reading(true);
        set_pass_exclusion_reading(true);
        let american = american();
        let dutch = dutch();
        let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
            ("american constructive", &american.constructive.0),
            ("american competitive", &american.competitive.0),
            ("american defensive", &american.defensive.0),
            ("dutch constructive", &dutch.constructive.0),
        ];

        let mut failures: Vec<String> = Vec::new();
        let mut check = |system: &str,
                         auction: &[Call],
                         context: &Context<'_>,
                         rules: &crate::bidding::rules::Rules| {
            let Some(projection) = super::project_pass(rules, context) else {
                return;
            };
            for &hand in &hands {
                let logits = rules.classify(hand, context);
                let pass = *logits.0.get(Call::Pass);
                let best_other = (&logits.0)
                    .into_iter()
                    .filter(|(call, _)| *call != Call::Pass)
                    .map(|(_, logit)| *logit)
                    .fold(f32::NEG_INFINITY, f32::max);
                if !pass.is_finite() || pass < best_other {
                    continue;
                }
                if !projection.boxes().iter().any(|b| b.accepts(hand)) && failures.len() < 16 {
                    failures.push(format!(
                        "{system}: [{}] pass reading excludes passing hand {hand}",
                        auction
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                    ));
                }
            }
        };

        for (system, trie) in tries {
            for (auction, classifier) in trie {
                if let Some(rules) = classifier.as_rules() {
                    let context = Context::new(RelativeVulnerability::NONE, &auction)
                        .with_prefixes(trie.common_prefixes(&auction));
                    check(system, &auction, &context, rules);
                }
            }
            for (auction, _, fallback) in trie.fallbacks() {
                let crate::bidding::fallback::Fallback::Classify(classifier) = fallback else {
                    continue;
                };
                if let Some(rules) = classifier.as_rules() {
                    let context = Context::new(RelativeVulnerability::NONE, &auction)
                        .with_prefixes(trie.common_prefixes(&auction));
                    check(system, &auction, &context, rules);
                }
            }
        }
        set_pass_exclusion_reading(false);

        assert!(
            failures.is_empty(),
            "pass-exclusion excludes hands that pass:\n{}",
            failures.join("\n"),
        );
    }

    /// Sibling invariant to [`artificial_calls_are_alerted`]: an authored rule that
    /// *gates* on an axis must not *read* as ⊤ on that axis.
    ///
    /// The fit-split bug is the motivating case (see
    /// `docs/ai-bidder/sampled-projection.md`): `hcp(13..) | (support(3..) &
    /// support_points(13..))` is a correct bidding rule that measured as a win, yet
    /// its projection says nothing about points at all — `Or::project` is the union,
    /// and one box holding a union is the bounding box, so the union is `0..=37`.
    /// Nothing errored and no test went red; the reading simply stopped knowing
    /// anything and kept a straight face.  The principle this pins down: the
    /// machinery may be *imprecise*, but never imprecise **invisibly**.
    ///
    /// The leak notion and its describe-sniffing caveats live on [`axis_leaks`].
    /// The walk covers the shipped `american()` books plus `dutch()`'s
    /// constructive trie (Dutch reuses american's competitive and defensive
    /// books), and runs **twice**:
    ///
    /// - **knob-off** (`set_dnf_reading(false)`, the shipped reading) — the
    ///   byte-identity guard.  These counts must not move *in either direction*:
    ///   a fall means a knob-off hull tightened, which is a bidding change that
    ///   must ship through measurement, not slip in as a refactor.
    /// - **knob-on** — the migration meter.  DNF-wave chops drive these toward
    ///   zero; each re-pin is recorded in `docs/dnf-migration.md`'s ledger.
    ///
    /// **Pinned exactly, not as a `<=` ratchet**: a fix-one-add-one swap cannot
    /// hide, at the price of consciously re-pinning (same commit, ledger row)
    /// whenever authoring legitimately moves a count.
    #[test]
    fn authored_calls_read_what_they_gate() {
        use crate::bidding::american::american;
        use crate::bidding::dutch::dutch;

        let american = american();
        let dutch = dutch();
        let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
            ("american constructive", &american.constructive.0),
            ("american competitive", &american.competitive.0),
            ("american defensive", &american.defensive.0),
            ("dutch constructive", &dutch.constructive.0),
        ];

        set_dnf_reading(false);
        let off = axis_leaks(&tries);
        set_dnf_reading(true);
        let on = axis_leaks(&tries);

        // (column, knob-off pin, knob-on pin) — re-pins go in the
        // docs/dnf-migration.md ledger.  Chop G drove every knob-on column to
        // **zero**: comparative staircases, reroute `dnf_upgrade` boxes,
        // `top_honors`/`Points` gauge floors, and `Balanced`'s unbalanced
        // complement.  Knob-off pins are the byte-identity guard; `length`
        // dropped 71 → 59 when the sniffer stopped counting context claims
        // ("partner's last suit is ♠") and deliberate no-op caps ("≤13 ♠") —
        // a meter-precision change, not a reading change (the dump diff
        // stayed clean).  The 2026-07-25 `Points13` gate default (the major
        // no-fit 2/1 now gauges `points(13..)`, not `hcp(13..)`) swaps six
        // legacy-`Or` leaks from HCP (17 → 11) to points (3 → 9); the knob-on
        // DNF box pins both axes exactly, so both knob-on columns stay 0.
        let pinned: [(&str, usize, usize); 6] = [
            // 11/0 → 20/9 when the queen relay went default-on (2026-08-02).
            // The nine new leaks are the same three calls in each column —
            // the asker's continuations over a 1430 answer, which *gate* on
            // `19+ HCP` (the grand-zone strength bar) but *read* as keycard
            // counts and "the queen cannot change the call".  The reading is
            // the honest one; the HCP conjunct is a strength floor that the
            // reading deliberately does not project, so the meter scores it a
            // leak.  **Recorded, not resolved** — closing it means either
            // projecting the strength bar (which would over-narrow partner's
            // hand at every keycard answer) or dropping it (which would let
            // the relay fire without the values).  See
            // docs/ai-bidder/bba-kickback.md §7.7.
            ("HCP", 20, 9),
            ("length", 59, 0),
            ("points", 9, 0),
            // 0/0 measured at birth (2026-07-25): every `suit_hcp` gate the
            // walk reaches (Ogust, the Lebensohl trap pass) is `&`-chained, and
            // the exact base-axis projection is ungated, so even the knob-off
            // hull keeps the band.  The `Or`-shaped gates (UVU double, penalty
            // X, SOS runouts) are wired as `Fallback::classify` and the walk
            // never sees them — a pre-existing meter blind spot on EVERY
            // column, recorded in docs/dnf-migration.md.
            ("suit HCP", 0, 0),
            ("support", 84, 0),
            ("support points", 18, 0),
        ];
        let count = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
            leaks.get(column).map_or(0, Vec::len)
        };
        let dump = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
            leaks.get(column).map_or_else(String::new, |v| v.join("\n"))
        };
        let mut mismatches = Vec::new();
        for (column, pin_off, pin_on) in pinned {
            let (got_off, got_on) = (count(&off, column), count(&on, column));
            if got_off != pin_off || got_on != pin_on {
                mismatches.push(format!(
                    "{column}: knob-off {got_off} (pinned {pin_off}), \
                     knob-on {got_on} (pinned {pin_on})\n\
                     --- knob-off ---\n{}\n--- knob-on ---\n{}",
                    dump(&off, column),
                    dump(&on, column),
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "axis leak counts moved:\n{}",
            mismatches.join("\n\n"),
        );
    }

    /// The fallback-layer twin of [`authored_calls_read_what_they_gate`]: the
    /// same axis-leak meter over every rule wired through a guarded
    /// [`Fallback::classify`][crate::bidding::fallback::Fallback::classify] —
    /// the layer the exact-node walk cannot see (every contested convention:
    /// UVU, penalty-X and SOS runouts, transfer competition).
    ///
    /// Pinned exactly like its sibling, in a **separate table** so the
    /// exact-node pins never re-pin for fallback churn.  Pin-first discipline:
    /// the initial nonzero counts *are* the worklist
    /// (`docs/ai-bidder/sampled-projection.md`), not failures to fix before
    /// landing the meter.  The opaque census below is the residue even this
    /// walk cannot meter — closures with no `as_rules()` — pinned with labels
    /// so a new dark classifier is a conscious act; that list is the
    /// conversion worklist for the pass-reading campaign.
    #[test]
    fn fallback_rules_read_what_they_gate() {
        use crate::bidding::american::american;
        use crate::bidding::dutch::dutch;

        let american = american();
        let dutch = dutch();
        let tries: [(&str, &crate::bidding::trie::Trie); 4] = [
            ("american constructive", &american.constructive.0),
            ("american competitive", &american.competitive.0),
            ("american defensive", &american.defensive.0),
            ("dutch constructive", &dutch.constructive.0),
        ];
        let walk: RuleWalk = |trie, visit| for_each_fallback_rule(trie, visit, |_, _| {});

        set_dnf_reading(false);
        let off = axis_leaks_with(&tries, walk);
        set_dnf_reading(true);
        let on = axis_leaks_with(&tries, walk);

        // Pinned at birth (2026-07-27) — the meter getting honest, not a
        // regression: these are the worklist the exact-node walk never saw.
        // The knob-on residue (14 HCP, 19 length, 2 points) is dominated by
        // the competitive free-bid/responsive-double package and the 4NT
        // quantitative fallback; `suit HCP`'s two knob-off leaks (the UVU
        // double) already close knob-on.  Re-pins ride the
        // docs/dnf-migration.md ledger like the sibling's.
        //
        // `points` went 2 → 8 → **0** over 2026-08-02.  All three numbers are
        // one mechanism: the keycard ask carried
        // `announced(slam_entry_reached(), points(11..))`, whose *agreement*
        // half is pure disclosure — the judgment is the support-point entry
        // bar, so the 11 was never a gate on anything.  Two leaks while only
        // 4NT asked, eight once kickback added three more asks across the two
        // constructive columns, and none at all once `set_rkcb_announce` was
        // deleted for announcing a floor the ask does not honour.  Deleting a
        // false announcement closed the leak outright rather than deferring
        // it, which is why this row is not on the §7.7 worklist with the
        // sibling's nine HCP-axis leaks.
        let pinned: [(&str, usize, usize); 6] = [
            ("HCP", 14, 14),
            ("length", 28, 19),
            ("points", 0, 0),
            ("suit HCP", 2, 0),
            ("support", 0, 0),
            ("support points", 0, 0),
        ];
        let count = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
            leaks.get(column).map_or(0, Vec::len)
        };
        let dump = |leaks: &std::collections::BTreeMap<&str, Vec<String>>, column| {
            leaks.get(column).map_or_else(String::new, |v| v.join("\n"))
        };
        let mut mismatches = Vec::new();
        for (column, pin_off, pin_on) in pinned {
            let (got_off, got_on) = (count(&off, column), count(&on, column));
            if got_off != pin_off || got_on != pin_on {
                mismatches.push(format!(
                    "{column}: knob-off {got_off} (pinned {pin_off}), \
                     knob-on {got_on} (pinned {pin_on})\n\
                     --- knob-off ---\n{}\n--- knob-on ---\n{}",
                    dump(&off, column),
                    dump(&on, column),
                ));
            }
        }
        assert!(
            mismatches.is_empty(),
            "fallback axis leak counts moved:\n{}",
            mismatches.join("\n\n"),
        );

        // The opaque census: `Fallback::classify` installations whose
        // classifier exposes no rules.  Counts installations (a shared entry
        // under seat-fanned prefixes rows once per node key), labelled by the
        // guard's describe().
        let mut opaque = Vec::new();
        for (system, trie) in tries {
            for_each_fallback_rule(
                trie,
                |_, _, _| {},
                |auction, label| {
                    opaque.push(format!(
                        "{system}: [{}] guard: {}",
                        auction
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(" "),
                        label.unwrap_or_else(|| "<unlabelled>".into()),
                    ));
                },
            );
        }
        opaque.sort();
        // Census at birth (2026-07-27), the residue worklist for the
        // pass-reading campaign: the seat-fanned `[1NT 2♣]`
        // competition-over-Stayman closure (×4), and the two root `(always)`
        // catch-alls — the competitive and defensive floor layers, exactly the
        // `Fallback::classify` blind spot the ⊤-census named.  Converting one
        // to `Rules` shrinks this pin and grows the metered tables above.
        assert_eq!(
            opaque.len(),
            6,
            "opaque classify-fallback census moved (re-pin consciously):\n{}",
            opaque.join("\n"),
        );
    }

    /// The same alert invariant, but for the opt-in Gladiator book (off by default,
    /// so the walk above never sees it).  A Gladiator artificial call added without
    /// `.alert(...)` fails here.
    #[test]
    fn gladiator_artificial_calls_are_alerted() {
        use crate::bidding::american::{american, set_nt_overcall_gladiator};

        set_nt_overcall_gladiator(true);
        let pair = american();
        set_nt_overcall_gladiator(false);

        assert_all_alerted(
            "Gladiator",
            unalerted_artificial("defensive", &pair.defensive.0),
        );
    }

    /// The same alert invariant for the [`dutch`][crate::bidding::dutch] system's
    /// constructive book.  Dutch reuses american's competitive and defensive
    /// books (covered by `artificial_calls_are_alerted`) and overrides only the
    /// opening table, so this walks the constructive trie — guarding the strong
    /// 2♣ alert and any artificial call a future Dutch phase adds.
    #[test]
    fn dutch_artificial_calls_are_alerted() {
        use crate::bidding::dutch::dutch;

        let pair = dutch();
        assert_all_alerted(
            "Dutch",
            unalerted_artificial("constructive", &pair.constructive.0),
        );
    }

    /// The same alert invariant for the opt-in New Minor Forcing book (off by
    /// default, so the shipped-system walk never sees it).  Guards the one
    /// artificial call NMF adds — responder's `2`-of-the-new-minor checkback —
    /// against losing its `.alert(...)` and reading as a phantom minor suit.
    #[test]
    fn new_minor_forcing_artificial_calls_are_alerted() {
        use crate::bidding::american::{american, set_new_minor_forcing};

        set_new_minor_forcing(true);
        let pair = american();
        set_new_minor_forcing(false);

        assert_all_alerted(
            "New Minor Forcing",
            unalerted_artificial("constructive", &pair.constructive.0),
        );
    }

    /// The same alert invariant for the opt-in choice-of-games 3NT and 2/1
    /// fit-leg books (off by default, so the shipped-system walk never sees
    /// them).
    #[test]
    fn choice_of_games_artificial_calls_are_alerted() {
        use crate::bidding::american::{american, set_major_choice_of_games};

        // ponytail: `two_over_one_fit` now defaults on, so the old set/restore
        // pair here was stale (and restored to the *non*-default).
        set_major_choice_of_games(true);
        let pair = american();
        set_major_choice_of_games(false);

        assert_all_alerted(
            "choice-of-games",
            unalerted_artificial("constructive", &pair.constructive.0),
        );
        set_major_choice_of_games(true);
    }

    /// The alerted choice-of-games 3NT decodes: opener reads responder as
    /// (4333) with 3+ in every suit (so the 5-3 major fit is known), exactly
    /// three spades over 1♥, and 12+ points.
    #[test]
    fn choice_of_games_three_notrump_reads_support() {
        use crate::bidding::american::set_major_choice_of_games;

        set_major_choice_of_games(true);
        let stance = crate::american().against();
        set_major_choice_of_games(false);

        let auction = [
            bid(1, Strain::Hearts),
            Call::Pass,
            bid(3, Strain::Notrump),
            Call::Pass,
        ];
        let read =
            Inferences::read(&stance.prefixed_context(RelativeVulnerability::NONE, &auction));
        assert!(read.partner().length(Suit::Hearts).min >= 3);
        assert!(read.partner().length(Suit::Diamonds).min >= 3);
        assert!(read.partner().length(Suit::Clubs).min >= 3);
        assert_eq!(read.partner().length(Suit::Spades), Range::new(3, 3));
        assert!(read.partner().strength.points.min >= 12);
        set_major_choice_of_games(true);
    }

    proptest! {
        /// Soundness: a hand that opens the book's choice falls within the
        /// opening inference.  Tests rule 1 (the opening table) over random hands.
        #[test]
        fn opening_inference_contains_the_opener(seed in any::<u64>()) {
            use crate::bidding::trie::Classifier;
            use crate::bidding::american::openings;
            use contract_bridge::deck::full_deal;
            use rand::SeedableRng;

            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let deal = full_deal(&mut rng);
            let hand: Hand = deal[contract_bridge::Seat::North];

            let context = Context::new(RelativeVulnerability::NONE, &[]);
            let logits = openings().classify(hand, &context);
            let Some((call, _)) = (&logits.0)
                .into_iter()
                .filter(|(_, l)| l.is_finite())
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("not NaN"))
            else {
                return Ok(());
            };
            let Call::Bid(_) = call else { return Ok(()); };

            // The opener sits to the actor's right after a single call.
            let inf = read(&[call]);
            let opener = inf.rho();
            let points = point_count(hand);
            prop_assert!(
                opener.strength.points.contains(points),
                "{call} opener with {points} points outside {:?}",
                opener.strength.points
            );
            for suit in Suit::ASC {
                let length = hand[suit].len();
                // SAFETY: a suit length is at most 13.
                #[allow(clippy::cast_possible_truncation)]
                let length = length as u8;
                prop_assert!(
                    opener.length(suit).contains(length),
                    "{call} opener with {length} {suit:?} outside {:?}",
                    opener.length(suit)
                );
            }
        }

        /// The load-bearing C1/C2 pin: closing the boxes is **membership-inert**
        /// on the real reading path, so the sampler cannot move.  Every hand a
        /// reading admitted knob-off it still admits knob-on, and vice versa —
        /// on the lenient `Dnf::contains` the sampler uses *and* the strict
        /// `Envelope::accepts` gate.  If this ever fires, the closure is
        /// dropping legal hands and the A/B verdict means nothing.
        #[test]
        fn closure_is_membership_inert(seed in any::<u64>()) {
            use crate::bidding::constraint::{Constraint as _, and, balanced, hcp, len, or, points};
            use contract_bridge::deck::full_deal;
            use rand::SeedableRng;

            let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
            let deal = full_deal(&mut rng);
            let hand: Hand = deal[contract_bridge::Seat::North];

            set_dnf_reading(true);
            let context = Context::new(RelativeVulnerability::NONE, &[]);
            let readings = [
                (balanced() & points(15..17)).project_band(&context),
                (or([Suit::Hearts, Suit::Spades], 5..) & points(8..)).project(&context),
                (and([Suit::Hearts, Suit::Spades], 5..) & hcp(6..11)).project_band(&context),
                (len(Suit::Spades, 6..) & points(13..)).project(&context),
                (!balanced() & points(12..)).project(&context),
            ];

            for reading in readings {
                let loose = reading.clone().tidy();
                set_sum_closure(true);
                set_upgrade_closure(true);
                let closed = reading.tidy();
                set_sum_closure(false);
                set_upgrade_closure(false);

                prop_assert_eq!(
                    loose.contains(hand), closed.contains(hand),
                    "contains moved: {:?} vs {:?}", loose, closed
                );
                prop_assert_eq!(
                    loose.boxes().iter().any(|b| b.accepts(hand)),
                    closed.boxes().iter().any(|b| b.accepts(hand)),
                    "accepts moved: {:?} vs {:?}", loose, closed
                );
            }
        }
    }
}
