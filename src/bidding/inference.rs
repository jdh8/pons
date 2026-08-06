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

/// How much of the authored book the projection pass decodes
///
/// The three *playable* stances of what used to be two independent bools
/// (`set_alert_reading` + `set_natural_reading`, folded 2026-08-03).  The read
/// site was `(alerted && alert_reading()) || natural_reading()`, so the natural
/// half short-circuited the alerted one: `(alert = off, natural = on)` and
/// `(alert = on, natural = on)` were the same reading, four cells for three
/// stances.  One `Cell` makes the honest domain the type, the
/// [`NotrumpDefense`][super::american::NotrumpDefense] precedent.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ReadingScope {
    /// Decode nothing off the authoring rules; every call falls to the natural
    /// walk's guess from auction shape.  The pre-alert behaviour, in which a
    /// strength-showing artificial that floors no foreign suit — the strong 2♣
    /// opening, its 2♦ waiting / 2♥ double negative, Puppet 3♣ — was misread as
    /// a natural suit.  The off arm of the `ab-alert-reading` A/B.
    None,

    /// Decode a call when its authoring rule carries an
    /// [`Alert`][crate::bidding::Alert], on top of the structural
    /// "points at a suit it did not name" test — the shipped default, and the
    /// per-call defense
    /// switch: the floor recognises every alerted convention and reads it as the
    /// convention rather than as a natural suit, so a player switches its
    /// treatment the moment an opponent's alerted call lands.
    #[default]
    Alerted,

    /// Decode **every** authored call, unalerted ones too.
    ///
    /// The alert gate is correct as *disclosure* — an unalerted call is natural
    /// and the natural walk reads it — but that leaves a whole regime unread: a
    /// rule that is authored and natural (`gladiator_advances`'s game-forcing
    /// `3♣`/`3♦`/`3O`, authored `len(suit, 5..) & points(game..)`) contributes
    /// **nothing**, and the walk's guess from auction shape is an unverified
    /// duplicate that can and does contradict it.  See
    /// `docs/reading-drift-handoff.md`.
    ///
    /// Here an unalerted call's rules project the same sound union as an alerted
    /// one's, and it is **intersected with** the walk's natural reading rather
    /// than replacing it — the call keeps its suppression bit clear, so the
    /// walk's bookkeeping (natural-suit lanes, agreed fits, later cue detection)
    /// is untouched and only the rule's own claim is added.  Two consequences
    /// follow from intersecting rather than substituting:
    ///
    /// - Where the walk is *right* the reading strictly tightens: the rule's
    ///   strength band (which the walk usually has no way to know) lands on a
    ///   call that previously published only a length floor.
    /// - Where the walk is *wrong* the boxes can go **empty**, because a wrong
    ///   walk claim intersected with a sound rule claim is still wrong.  That is
    ///   a diagnostic, not a regression of this arm: it surfaces walk defects the
    ///   alert gate had been hiding.  Sweep with the `admits` invariant before
    ///   reading anything into an A/B.
    ///
    /// **Unmeasured**, hence not the default: it tightens thousands of readings
    /// at once, and per `docs/dnf-migration.md`'s C1 finding a tightening that
    /// moves *endpoints* without moving *mass* is close to pure feature
    /// perturbation for the frozen nets.
    All,
}

std::thread_local! {
    /// How much of the authored book the projection decodes; **[`Alerted`] by
    /// default**.
    ///
    /// [`Alerted`]: ReadingScope::Alerted
    static READING_SCOPE: Cell<ReadingScope> = const { Cell::new(ReadingScope::Alerted) };
}

/// Select how much of the authored book the projection decodes (see
/// [`ReadingScope`]); read at reading time, per-thread
pub fn set_reading_scope(scope: ReadingScope) {
    READING_SCOPE.with(|cell| cell.set(scope));
}

/// The projection's current decoding scope (see [`set_reading_scope`])
fn reading_scope() -> ReadingScope {
    READING_SCOPE.with(Cell::get)
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
    /// Whether a call's reading is stored as an *envelope union* and the
    /// sampler accepts a hand that lies in **any** box, rather than the single
    /// bounding-box hull (see [`set_envelope_union_reading`]).  **On by default** since
    /// chop F2b (docs/dnf-migration.md): with the knob-matched evaluator twin
    /// and the statically pinned Jacoby box, the flip measured a win in all
    /// four cells — plain +0.0094/+0.0080, PD +0.0118/+0.0085 NV/vul, CIs
    /// clear (204,800 boards/arm/vul, seed 1784809754).  Off is the legacy
    /// hull path, kept as the kill-switch.
    static ENVELOPE_UNION_READING: Cell<bool> = const { Cell::new(true) };
}

/// Toggle envelope-union readings for the sampler (**default on**, F2b)
///
/// Off, a disjunctive reading (`Or`, `AnyLen`, a call authored by several rules)
/// widens to its bounding box, so the sampler accepts the whole hull — the legacy
/// behaviour.  On, the reading keeps its separate boxes and the sampler accepts
/// a layout only if it lies in *some* box, pinning two-suiters / Multi / the
/// fit-split instead of the box that spans them.  Two hull regimes, measured
/// (docs/dnf-migration.md, chop F1): on a **bare** `Context::new` (no
/// projection overlay — the `dump-teacher` feature path) the hulls are
/// knob-invariant, byte-identical over a 21K-row dump; on a **prefixed**
/// context (`Stance::infer` — what the bidder, the floor net, and the
/// bilans evaluator actually see) the authored-projection overlay tightens
/// knob-on (⊤→box upgrades, `envelope_union_upgrade`), so those consumers' inputs move
/// with the knob.  Read at classification and acceptance time, per-thread.
pub fn set_envelope_union_reading(on: bool) {
    ENVELOPE_UNION_READING.with(|cell| cell.set(on));
}

/// Whether envelope-union readings are enabled (default on)
#[must_use]
pub fn envelope_union_reading() -> bool {
    ENVELOPE_UNION_READING.with(Cell::get)
}

std::thread_local! {
    /// Whether the two opponent seats' readings are blanked (see
    /// [`set_blind_opponent_reading`]).  **Off by default.**
    static BLIND_OPPONENT_READING: Cell<bool> = const { Cell::new(false) };
}

/// Blank what the *opponents* have shown (**default off**, measurement only)
///
/// On, [`Inferences`] hands back [`Envelope::unknown`] / [`EnvelopeUnion::unknown`] for
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
/// acceptance, [`EnvelopeUnion::contains`], the envelope-union overlay — tests suit lengths and
/// the legacy `points` gauge only, the pre-`Strength` behaviour.  On, a
/// sampled hand must also fall within the box's raw-HCP, support-points, and
/// per-suit HCP bands, so a 15–17 1NT stops admitting 13-counts the `points`
/// scale upgraded.  The measured "bidding-inert" verdict (chop E: 0 fired in
/// 409,600 boards) predates both the suit-indexed `supports` gauging and the
/// `suit_hcp` axis — those bands have real teeth (an Ogust quality ceiling
/// rejects hands no whole-hand gauge can), so a future flip owes a fresh
/// measurement.  Deliberately its **own** knob, never folded into
/// [`set_envelope_union_reading`]: the mechanisms are independent (gauge bands tighten
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
    /// Whether [`EnvelopeUnion::tidy`] narrows suit lengths by `Σ len = 13` (see
    /// [`set_sum_closure`]).  **Off by default** — a hull change, so it owes
    /// an A/B.
    static SUM_CLOSURE: Cell<bool> = const { Cell::new(false) };
    /// Whether [`EnvelopeUnion::tidy`] closes `hcp` against `points` through the shape
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
/// Requires [`envelope_union_reading`] (the knob-off hull path stays byte-identical).
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
/// Requires [`envelope_union_reading`].  Read at classification time, per-thread.
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
/// ([`Rule::project_complement_union`][super::rules::Rule::project_complement_union]),
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
    /// Whether [`project_authored`] folds a second, *agreement* overlay off
    /// [`Rule::announce_union`][super::rules::Rule::announce_union] (see
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

/// Thread-local settings that can change a full-auction reading
///
/// Kept as a value rather than a hash so cache validation cannot collide.  It
/// is captured once when a decision scope is entered and compared only by
/// debug assertions on the cached path.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReadingProfile {
    nt_invite: bool,
    rubens_transfer: bool,
    scope: ReadingScope,
    fallback_projection: bool,
    envelope_union: bool,
    blind_opponents: bool,
    gauge_membership: bool,
    sum_closure: bool,
    upgrade_closure: bool,
    control_bid: bool,
    cue: bool,
    length_soundness: bool,
    pass: bool,
    pass_exclusion: bool,
    probed: bool,
    probed_vacuous: bool,
    announced: bool,
    table_alerts: bool,
    rule_accept: bool,
    point_scale: super::constraint::PointScale,
    rubens_advances: bool,
    penalty_latch: bool,
    nt_overcall_systems_on: bool,
    nt_overcall_gladiator: bool,
    nt_splinter: bool,
    opener_extras_ladder: bool,
    xyz: bool,
    notrump_minors: super::rules::Alert,
    opener_major_jump_rebid: bool,
    garbage_stayman: bool,
    crawling_stayman: bool,
    woolsey_points: (u8, u8),
    woolsey_double_floor: u8,
    natural_double_floor: u8,
    longer_major_response: bool,
    landy_range: Option<(u8, u8)>,
    notrump_defense: super::american::NotrumpDefense,
    natural_overcall_points: (u8, u8),
    two_notrump_wide: bool,
    floor_rkcb: bool,
    rkcb_variant: super::instinct::RkcbVariant,
}

impl ReadingProfile {
    pub(crate) const fn decodes_nonpass(self, alerted: bool) -> bool {
        match self.scope {
            ReadingScope::None => false,
            ReadingScope::Alerted => alerted,
            ReadingScope::All => true,
        }
    }

    pub(crate) const fn pass_reading(self) -> bool {
        self.pass
    }

    pub(crate) const fn pass_exclusion_reading(self) -> bool {
        self.pass_exclusion
    }

    pub(crate) const fn announced_reading(self) -> bool {
        self.announced
    }
}

/// Snapshot the reading settings active on this thread
pub(crate) fn reading_profile() -> ReadingProfile {
    ReadingProfile {
        nt_invite: nt_invite_inference(),
        rubens_transfer: rubens_transfer_reading(),
        scope: reading_scope(),
        fallback_projection: fallback_projection_enabled(),
        envelope_union: envelope_union_reading(),
        blind_opponents: blind_opponent_reading(),
        gauge_membership: gauge_membership(),
        sum_closure: sum_closure(),
        upgrade_closure: upgrade_closure(),
        control_bid: control_bid_reading(),
        cue: cue_reading(),
        length_soundness: length_soundness(),
        pass: pass_reading(),
        pass_exclusion: pass_exclusion_reading(),
        probed: probed_reading(),
        probed_vacuous: probed_vacuous_reading(),
        announced: announced_reading(),
        table_alerts: table_alert_reading(),
        rule_accept: rule_accept_enabled(),
        point_scale: super::constraint::point_scale(),
        rubens_advances: super::instinct::rubens_advances_enabled(),
        penalty_latch: super::instinct::penalty_latch_enabled(),
        nt_overcall_systems_on: super::american::nt_overcall_systems_on(),
        nt_overcall_gladiator: super::american::nt_overcall_gladiator(),
        nt_splinter: super::american::nt_splinter(),
        opener_extras_ladder: super::american::opener_extras_ladder(),
        xyz: super::american::xyz(),
        notrump_minors: super::american::notrump_minors(),
        opener_major_jump_rebid: super::american::opener_major_jump_rebid(),
        garbage_stayman: super::american::garbage_stayman(),
        crawling_stayman: super::american::crawling_stayman(),
        woolsey_points: super::american::woolsey_points(),
        woolsey_double_floor: super::american::woolsey_double_floor(),
        natural_double_floor: super::american::natural_double_floor(),
        longer_major_response: super::american::longer_major_response(),
        landy_range: super::american::landy_range(),
        notrump_defense: super::american::notrump_defense(),
        natural_overcall_points: super::american::natural_overcall_points(),
        two_notrump_wide: super::american::two_notrump_wide(),
        floor_rkcb: super::instinct::floor_rkcb_now(),
        rkcb_variant: super::instinct::rkcb_variant_now(),
    }
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
    /// drop the truth, widen to their *span* — soundness over tightness.
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
    pub fn span(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// The overlap, or `None` when the ranges are disjoint (an empty product)
    ///
    /// Unlike [`intersect`][Self::intersect], which widens a crossed range to
    /// preserve soundness within a single box, this reports the empty product so
    /// an [`EnvelopeUnion`] can **drop** the contradictory term.
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
    /// the containment that lets `EnvelopeUnion::tidy`'s correct dedup swallow the arm
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
    /// [`EnvelopeUnion::tidy`]'s correct dedup swallow the arm carrying the suit
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

    /// Field-by-field [`Range::span`] — the `|` dual, soundness over tightness
    #[must_use]
    fn span(self, other: Self) -> Self {
        Self {
            hcp: self.hcp.span(other.hcp),
            points: self.points.span(other.points),
            support_points: core::array::from_fn(|i| {
                self.support_points[i].span(other.support_points[i])
            }),
            suit_hcp: core::array::from_fn(|i| self.suit_hcp[i].span(other.suit_hcp[i])),
        }
    }

    /// Bounded intersection; `None` only when the `points` gauge is disjoint
    ///
    /// **Only `points` gates box-emptiness**, exactly as the pre-`Strength` box
    /// algebra did.  The new gauges combine by the widening [`Range::intersect`]
    /// so they never drop a box a `points`/length reading would have kept — they
    /// are inert until Edits 1/2, and must not perturb the [`EnvelopeUnion`] the sampler
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

    /// Pointwise span — the bounding box covering either set of bounds
    ///
    /// The forward dual of a constraint disjunction: a hand accepted by `a | b`
    /// lies within one envelope or the other, so each quantity spans both
    /// ([`Range::span`]) — soundness over tightness.
    #[must_use]
    pub fn span(&self, other: &Self) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.lengths[suit as usize] = out.length(suit).span(other.length(suit));
        }
        out.strength = out.strength.span(other.strength);
        out
    }

    /// The intersection box, or `None` when any axis is disjoint (empty product)
    ///
    /// Unlike [`intersect`][Self::intersect], which widens a crossed range to
    /// preserve soundness *within* a single box, this reports the empty product
    /// so an [`EnvelopeUnion`] can **drop** the contradictory term — the surviving terms of
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
    /// The per-box membership test the sampler and [`EnvelopeUnion::contains`] share.
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
    /// [`admits`][Self::admits] is unchanged.  Only [`EnvelopeUnion::hull`] (tighter) and
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
    /// This is what a natively authored [`Envelope`] / [`EnvelopeUnion`] **gate** evaluates
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum EnvelopeBoxes {
    One(Envelope),
    Many(Vec<Envelope>),
}

/// A forward reading as a nonempty union of envelope boxes
///
/// One [`Envelope`] is a single axis-aligned box; a disjunction (`Multi`, a
/// two-suiter, a `!`-shape) needs a *union* of boxes, which a single box cannot
/// hold without widening to the bounding box (the "`Or` wall").  An
/// [`EnvelopeUnion`] keeps the terms of that disjunctive normal form: a hand is
/// consistent with the call iff it lies in **some** box.
///
/// Sound by construction — every operation is *exact or widening, never
/// narrowing*.  [`intersect`][Self::intersect] distributes (Cartesian product of
/// box-intersects, dropping empty products); [`union`][Self::union] concatenates.
/// [`hull`][Self::hull] collapses the union back to the single bounding box, the
/// migration escape hatch that reproduces the legacy single-box reading.
#[derive(Clone, PartialEq, Eq)]
pub struct EnvelopeUnion(EnvelopeBoxes);

impl core::fmt::Debug for EnvelopeUnion {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("EnvelopeUnion")
            .field(&self.boxes())
            .finish()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EnvelopeUnion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(self.boxes(), serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for EnvelopeUnion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let boxes = <Vec<Envelope> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::from_boxes(boxes))
    }
}

impl EnvelopeUnion {
    /// Nothing shown yet: a single [`Envelope::unknown`] box
    #[must_use]
    pub fn unknown() -> Self {
        Self(EnvelopeBoxes::One(Envelope::unknown()))
    }

    fn from_boxes(mut boxes: Vec<Envelope>) -> Self {
        if boxes.len() == 1 {
            Self(EnvelopeBoxes::One(boxes.pop().expect("one envelope")))
        } else {
            // Preserve serde's historical acceptance of an empty sequence even
            // though all public constructors maintain the non-empty invariant.
            Self(EnvelopeBoxes::Many(boxes))
        }
    }

    /// The bounding box of the union — fold [`Envelope::span`] over the terms
    ///
    /// Today's single-box behaviour: every consumer that still wants one
    /// [`Envelope`] hulls here.  Never narrows (a hand in some term is in the
    /// hull), so hulling a sound `EnvelopeUnion` stays sound.
    #[must_use]
    pub fn hull(&self) -> Envelope {
        self.boxes()
            .iter()
            .copied()
            .reduce(|a, b| a.span(&b))
            .unwrap_or_else(Envelope::unknown)
    }

    /// Whether **some** box admits the hand — tighter than `hull().admits()`
    #[must_use]
    pub fn contains(&self, hand: Hand) -> bool {
        self.boxes().iter().any(|b| b.admits(hand))
    }

    /// The disjoined boxes — non-empty (`len >= 1`) by invariant
    #[must_use]
    pub fn boxes(&self) -> &[Envelope] {
        match &self.0 {
            EnvelopeBoxes::One(box_) => core::slice::from_ref(box_),
            EnvelopeBoxes::Many(boxes) => boxes,
        }
    }

    /// Concatenate the terms — the `|` projection (either box may hold)
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        match (self.0, other.0) {
            (EnvelopeBoxes::One(one), EnvelopeBoxes::One(two)) => {
                Self(EnvelopeBoxes::Many(vec![one, two]))
            }
            (EnvelopeBoxes::One(one), EnvelopeBoxes::Many(mut many)) => {
                many.insert(0, one);
                Self(EnvelopeBoxes::Many(many))
            }
            (EnvelopeBoxes::Many(mut many), EnvelopeBoxes::One(one)) => {
                many.push(one);
                Self(EnvelopeBoxes::Many(many))
            }
            (EnvelopeBoxes::Many(mut one), EnvelopeBoxes::Many(mut two)) => {
                one.append(&mut two);
                Self(EnvelopeBoxes::Many(one))
            }
        }
    }

    /// The `|` combine the projection fold uses: separate boxes under
    /// [`envelope_union_reading`], else the single bounding-box hull
    ///
    /// Off, reproduces [`Envelope::span`] exactly, so the hull
    /// path stays byte-identical; on, keeps the arms so an enclosing `&`
    /// distributes and the sampler pins the disjunction.
    #[must_use]
    pub fn disjoin(self, other: Self) -> Self {
        if envelope_union_reading() {
            self.union(other).tidy()
        } else {
            Self::from(self.hull().span(&other.hull()))
        }
    }

    /// Cartesian product of pairwise box-intersects — the `&` projection
    ///
    /// `(A ∪ B) ∩ (C ∪ D) = (A∩C) ∪ (A∩D) ∪ (B∩C) ∪ (B∩D)`, dropping empty
    /// products (`Envelope::intersect_nonempty`).  The common case is one box
    /// each → one box out (`and` is a cheap box-shrink); growth needs *both*
    /// sides to be genuine disjunctions.  If every product is empty the whole
    /// conjunction is unsatisfiable, so fall back to the widened hull-intersect
    /// — sound and loose, never an empty (unsound) `EnvelopeUnion`.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        self.clone().intersect_owned(other)
    }

    /// Consuming intersection used by append-only projection accumulators.
    pub(crate) fn intersect_owned(self, other: &Self) -> Self {
        let fallback = self.hull().intersect(&other.hull());
        match (self.0, &other.0) {
            (EnvelopeBoxes::One(one), EnvelopeBoxes::One(two)) => Self(EnvelopeBoxes::One(
                one.intersect_nonempty(two).unwrap_or(fallback),
            ))
            .tidy(),
            (EnvelopeBoxes::Many(mut boxes), EnvelopeBoxes::One(one)) => {
                boxes.retain_mut(|box_| {
                    let Some(product) = box_.intersect_nonempty(one) else {
                        return false;
                    };
                    *box_ = product;
                    true
                });
                if boxes.is_empty() {
                    Self(EnvelopeBoxes::One(fallback)).tidy()
                } else {
                    Self::from_boxes(boxes).tidy()
                }
            }
            (left, _) => {
                let left = Self(left);
                let mut out = Vec::new();
                for a in left.boxes() {
                    for b in other.boxes() {
                        if let Some(product) = a.intersect_nonempty(b) {
                            out.push(product);
                        }
                    }
                }
                if out.is_empty() {
                    out.push(fallback);
                }
                // ponytail: no cap — `and`-of-two-`or`s is the only multiplier and it is
                // rare, so the Vec stays short on the real book.  The assert fires
                // loudly if some auction blows up; add sound exact-merge (containment +
                // axis-adjacency) only then.
                let out = Self::from_boxes(out).tidy();
                debug_assert!(
                    out.boxes().len() < 64,
                    "envelope union term explosion: {} boxes",
                    out.boxes().len()
                );
                out
            }
        }
    }

    fn intersect_assign(&mut self, other: &Self) {
        let owned = core::mem::replace(self, Self::unknown());
        *self = owned.intersect_owned(other);
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
    /// the extra containments the dedup then finds are real.  Runs only under
    /// [`envelope_union_reading`] —
    /// the knob-off hull path must stay byte-identical — and restores the
    /// non-empty invariant with ⊤ if every box was a ghost (an unsatisfiable
    /// conjunction; sound, loose, rare).
    fn tidy(self) -> Self {
        if !envelope_union_reading() {
            return self;
        }
        let mut boxes = match self.0 {
            EnvelopeBoxes::One(mut box_) => {
                if !box_.sum_feasible() {
                    return Self::unknown();
                }
                if sum_closure() {
                    box_.narrow_to_sum();
                }
                if upgrade_closure() {
                    box_.narrow_to_upgrade();
                }
                return Self(EnvelopeBoxes::One(box_));
            }
            EnvelopeBoxes::Many(boxes) => boxes,
        };
        boxes.retain(Envelope::sum_feasible);
        if sum_closure() || upgrade_closure() {
            // Exact and membership-inert, so running it *before* the dedup is
            // safe: every containment it exposes is a real one.  Sum first —
            // it can force a box balanced, which is what the upgrade closure
            // reads.
            for box_ in &mut boxes {
                if sum_closure() {
                    box_.narrow_to_sum();
                }
                if upgrade_closure() {
                    box_.narrow_to_upgrade();
                }
            }
        }
        let mut i = 0;
        while i < boxes.len() {
            let mut redundant = false;
            for (j, b) in boxes.iter().enumerate() {
                let a = &boxes[i];
                if i != j && a.subset_of(b) && (!b.subset_of(a) || j < i) {
                    redundant = true;
                    break;
                }
            }
            if redundant {
                boxes.remove(i);
            } else {
                i += 1;
            }
        }
        if boxes.is_empty() {
            return Self::unknown();
        }
        Self::from_boxes(boxes)
    }
}

impl From<Envelope> for EnvelopeUnion {
    fn from(box_: Envelope) -> Self {
        Self(EnvelopeBoxes::One(box_))
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
/// `Vec`-backed [`EnvelopeUnion`] means this is `Clone`, not `Copy` (two convertible call
/// sites: `narrowed_points`, `single_dummy`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inferences {
    /// Per-seat bounding-box hull of `unions` — the single-[`Envelope`] reading the
    /// American engine consumes via [`get`][Self::get].  A redundant cache of
    /// `unions[i].hull()` (`ponytail: keeps get()->&Envelope and all readers
    /// unchanged; collapse to get-by-value if the two ever drift`).
    players: [Envelope; 4],
    /// Per-seat union-of-boxes reading; the sampler tests any-box under
    /// [`envelope_union_reading`].  Off, every entry is a single box equal to
    /// `players[i]`.
    unions: [EnvelopeUnion; 4],
    /// Per-seat hull of `announced_unions` — the *agreement* twin of `players`, and
    /// what [`features`][super::features] hands the nets.  Equal to `players`
    /// unless [`set_announced_reading`] is on and some rule split the two with
    /// [`announced`][super::constraint::announced].
    announced_players: [Envelope; 4],
    /// Per-seat agreement boxes; the twin of `unions` (see `announced_players`).
    announced_unions: [EnvelopeUnion; 4],
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
    pub const fn announced_union(&self, who: Relative) -> &EnvelopeUnion {
        &self.announced_unions[who as usize]
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
        overlay: &[EnvelopeUnion; 4],
        agreement: &[EnvelopeUnion; 4],
        control_bid: Option<(u8, Suit)>,
    ) -> Self {
        let announced_unions = intersect_overlay(&players, agreement);
        let mut this = Self {
            unions: intersect_overlay(&players, overlay),
            announced_players: std::array::from_fn(|i| announced_unions[i].hull()),
            announced_unions,
            players,
            control_bid,
        };
        if blind_opponent_reading() {
            for who in [Relative::Lho, Relative::Rho] {
                let i = who as usize;
                this.players[i] = Envelope::unknown();
                this.announced_players[i] = Envelope::unknown();
                this.unions[i] = EnvelopeUnion::unknown();
                this.announced_unions[i] = EnvelopeUnion::unknown();
            }
        }
        this
    }

    /// Whether `hand` is consistent with one seat's reading
    ///
    /// Under [`envelope_union_reading`] a hand must lie in *some* box of that seat's union
    /// (tighter — pins two-suiters / Multi / the fit-split); off, it need only
    /// lie in the bounding-box hull (today's acceptance).  The sampler's per-seat
    /// test.
    #[must_use]
    pub fn admits(&self, who: Relative, hand: Hand) -> bool {
        if envelope_union_reading() {
            self.unions[who as usize].contains(hand)
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
        // Narrow the points of every box in the union to keep `unions` == the hull's
        // source (a points-only slab drops no box: it never crosses a length axis).
        let slab = Envelope {
            strength: Strength {
                points,
                ..Strength::unknown()
            },
            ..Envelope::unknown()
        };
        copy.unions[i].intersect_assign(&slab.into());
        // An externally-imposed points slice is a fact about the hand, not a
        // reading of a call, so it narrows the agreement side identically —
        // otherwise the two drift apart on the one axis the caller sliced.
        copy.announced_players[i].strength.points =
            copy.announced_players[i].strength.points.intersect(points);
        copy.announced_unions[i].intersect_assign(&slab.into());
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
        // `unions` (the boxes) at each return.  Unknown until `project_authored` runs.
        let mut overlay_unions: [EnvelopeUnion; 4] =
            std::array::from_fn(|_| EnvelopeUnion::unknown());
        // The agreement twin of `overlay_unions`; a clone of it unless
        // [`set_announced_reading`] is on (see [`project_authored`]).
        let mut agreement_unions: [EnvelopeUnion; 4] =
            std::array::from_fn(|_| EnvelopeUnion::unknown());
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
                overlay_unions = overlay;
                agreement_unions = agreement;
            }
            return Self::assemble(players, &overlay_unions, &agreement_unions, control_bid);
        };
        let Call::Bid(opening_bid) = auction[opening_index] else {
            return Self::assemble(players, &overlay_unions, &agreement_unions, control_bid);
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
        overlay_unions = overlay_boxes;
        agreement_unions = agreement_boxes;
        // The hulled overlay the natural walk consumes (`shown_suit`, the post-walk
        // intersect); the boxes are re-combined into `unions` at the return.
        let overlay: [Envelope; 4] = std::array::from_fn(|i| overlay_unions[i].hull());
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

        Self::assemble(players, &overlay_unions, &agreement_unions, control_bid)
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
    let (unions, announced_unions, _) = project_authored(context);
    Inferences {
        players: std::array::from_fn(|i| unions[i].hull()),
        announced_players: std::array::from_fn(|i| announced_unions[i].hull()),
        unions,
        announced_unions,
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
/// per-seat envelope union the sampler consumes
///
/// `players[i]` already folds `overlay[i].hull()` and every hand-walk narrowing,
/// and each overlay box is `⊆` that hull, so re-intersecting recovers exactly
/// `⋃(hand-walk ∩ boxₖ)` — the tight union — while dropping boxes the walk
/// contradicts.  With [`envelope_union_reading`] off each overlay is one box,
/// so the result is the single box `players[i]` and
/// `unions[i].hull() == players[i]` (byte-identical).
fn intersect_overlay(players: &[Envelope; 4], overlay: &[EnvelopeUnion; 4]) -> [EnvelopeUnion; 4] {
    std::array::from_fn(|i| EnvelopeUnion::from(players[i]).intersect_owned(&overlay[i]))
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
#[inline]
fn project_pass<'a>(
    rules: &super::rules::Rules,
    compiled: Option<&'a super::rules::CompiledRules>,
    ctx: &Context<'_>,
) -> Option<super::rules::ProjectedUnion<'a>> {
    let band = if let Some(compiled) = compiled {
        compiled
            .pass_rule_indices()
            .iter()
            .map(|&index| compiled.project_band_union_matched(rules, index, ctx))
            .reduce(super::rules::ProjectedUnion::disjoin)?
    } else {
        super::rules::ProjectedUnion::Owned(
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == Call::Pass)
                .map(|rule| rule.project_band_union(ctx))
                .reduce(EnvelopeUnion::disjoin)?,
        )
    };
    if !pass_exclusion_reading() {
        return Some(band);
    }
    if let Some(compiled) = compiled {
        let pass = compiled
            .pass_plan()
            .expect("Pass indices imply a Pass plan");
        return Some(super::rules::ProjectedUnion::Owned(
            pass.stronger_nonpass_indices()
                .iter()
                .map(|&index| compiled.project_complement_union_matched(rules, index, ctx))
                .filter(|complement| {
                    complement.as_union().boxes().len() == 1
                        && complement.as_union().boxes()[0] != Envelope::unknown()
                })
                .fold(band.into_owned(), |acc, complement| {
                    acc.intersect_owned(complement.as_union())
                }),
        ));
    }
    let ceiling = rules
        .rules()
        .iter()
        .filter(|rule| rule.call() == Call::Pass)
        .map(super::rules::Rule::weight)
        .max()
        .unwrap_or(i16::MIN);
    Some(super::rules::ProjectedUnion::Owned(
        rules
            .rules()
            .iter()
            .filter(|rule| rule.call() != Call::Pass && rule.weight() > ceiling)
            .map(|rule| rule.project_complement_union(ctx))
            .filter(|complement| {
                complement.boxes().len() == 1 && complement.boxes()[0] != Envelope::unknown()
            })
            .fold(band.into_owned(), |acc, complement| {
                acc.intersect_owned(&complement)
            }),
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AuthoredProjection {
    unions: [EnvelopeUnion; 4],
    announced_unions: [EnvelopeUnion; 4],
    suppressed: u64,
}

impl AuthoredProjection {
    fn unknown() -> Self {
        Self {
            unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            announced_unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            suppressed: 0,
        }
    }

    fn apply(&mut self, len: usize, index: usize, effect: AuthoredEffect<'_>) {
        let who = relative_of(len, index) as usize;
        self.announced_unions[who].intersect_assign(effect.agreement());
        self.unions[who].intersect_assign(effect.projection.as_union());
        if effect.suppresses_natural && index < 64 {
            self.suppressed |= 1 << index;
        }
    }

    fn into_parts(self) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
        (self.unions, self.announced_unions, self.suppressed)
    }

    fn cloned_parts(&self) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
        (
            self.unions.clone(),
            self.announced_unions.clone(),
            self.suppressed,
        )
    }
}

struct AuthoredEffect<'a> {
    projection: super::rules::ProjectedUnion<'a>,
    agreement: Option<super::rules::ProjectedUnion<'a>>,
    suppresses_natural: bool,
}

impl AuthoredEffect<'_> {
    fn agreement(&self) -> &EnvelopeUnion {
        self.agreement
            .as_ref()
            .unwrap_or(&self.projection)
            .as_union()
    }
}

#[inline(always)]
fn authored_effect<'a>(
    made: Call,
    ctx: &Context<'_>,
    classifier: &dyn super::trie::Classifier,
    compiled: Option<&'a super::rules::CompiledRules>,
    decode_pass: bool,
    announce_split: bool,
) -> Option<AuthoredEffect<'a>> {
    let rules = classifier.as_rules()?;
    let is_pass = made == Call::Pass;
    let scope = (!is_pass).then(reading_scope);
    let mut face_memo = super::rules::FaceMemo::new();

    // A compiled call plan can reject structurally unreadable groups before
    // evaluating faces or materializing their projections.  The shipped
    // Alerted scope sees many natural authoring nodes while walking every
    // prefix; those groups cannot contribute regardless of context.  Keep the
    // non-compiled oracle's evaluation order intact for opaque public hooks.
    if let Some(compiled) = compiled {
        if is_pass {
            if !decode_pass && compiled.can_skip_pass_effect(pass_exclusion_reading()) {
                return None;
            }
        } else {
            match scope.expect("non-pass reading scope") {
                ReadingScope::None if compiled.can_skip_nonpass_effect(made) => return None,
                ReadingScope::Alerted
                    if compiled.alerted_rule_indices(made).is_empty()
                        && compiled.can_skip_nonpass_effect(made) =>
                {
                    return None;
                }
                ReadingScope::None | ReadingScope::Alerted | ReadingScope::All => {}
            }
        }
    }

    let projection = if is_pass {
        project_pass(rules, compiled, ctx)
    } else if let Some(compiled) = compiled {
        compiled
            .rule_indices(made)
            .iter()
            .filter(|&&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
            .map(|&index| compiled.project_union_matched(rules, index, ctx))
            .reduce(super::rules::ProjectedUnion::disjoin)
    } else {
        rules
            .rules()
            .iter()
            .filter(|rule| rule.call() == made && rule.face_live(ctx))
            .map(|rule| super::rules::ProjectedUnion::Owned(rule.project_union(ctx)))
            .reduce(super::rules::ProjectedUnion::disjoin)
    }?;

    let alerted = !is_pass
        && if let Some(compiled) = compiled {
            compiled
                .alerted_rule_indices(made)
                .iter()
                .any(|&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
        } else {
            rules
                .rules()
                .iter()
                .any(|rule| rule.call() == made && rule.alert().is_some() && rule.face_live(ctx))
        };
    let decode = if is_pass {
        decode_pass
    } else {
        match scope.expect("non-pass reading scope") {
            ReadingScope::None => false,
            ReadingScope::Alerted => alerted,
            ReadingScope::All => true,
        }
    };
    if !decode {
        return None;
    }

    let agreement = if announce_split && !is_pass {
        if let Some(compiled) = compiled {
            compiled
                .alerted_rule_indices(made)
                .iter()
                .filter(|&&index| compiled.face_live_memoized(rules, index, ctx, &mut face_memo))
                .map(|&index| compiled.announce_union_matched(rules, index, ctx))
                .reduce(super::rules::ProjectedUnion::disjoin)
        } else {
            rules
                .rules()
                .iter()
                .filter(|rule| rule.call() == made && rule.alert().is_some() && rule.face_live(ctx))
                .map(|rule| super::rules::ProjectedUnion::Owned(rule.announce_union(ctx)))
                .reduce(super::rules::ProjectedUnion::disjoin)
        }
    } else {
        None
    };
    Some(AuthoredEffect {
        projection,
        agreement,
        suppresses_natural: alerted,
    })
}

#[derive(Clone, Debug)]
struct AbsoluteProjection {
    unions: [EnvelopeUnion; 4],
    announced_unions: [EnvelopeUnion; 4],
    suppressed: u64,
}

impl AbsoluteProjection {
    fn unknown() -> Self {
        Self {
            unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            announced_unions: std::array::from_fn(|_| EnvelopeUnion::unknown()),
            suppressed: 0,
        }
    }

    fn push(&mut self, index: usize, effect: AuthoredEffect<'_>) {
        let seat = index % 4;
        self.unions[seat].intersect_assign(effect.projection.as_union());
        self.announced_unions[seat].intersect_assign(effect.agreement());
        if effect.suppresses_natural && index < 64 {
            self.suppressed |= 1 << index;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // retained as the per-call route/provenance cache record
struct CachedRoute {
    classifier_id: super::decoder::DecoderClassifierId,
    provenance: super::trie::Provenance,
    pattern_id: Option<super::rows::PatternId>,
    table_id: Option<super::rows::RuleTableId>,
    fast: bool,
}

impl From<super::decoder::DecodedAuthoring<'_>> for CachedRoute {
    fn from(answer: super::decoder::DecodedAuthoring<'_>) -> Self {
        Self {
            classifier_id: answer.classifier_id,
            provenance: answer.provenance,
            pattern_id: answer.pattern_id,
            table_id: answer.table_id,
            fast: answer.fast,
        }
    }
}

/// One route-safe append waiting for its projection effects to be committed.
///
/// A table side normally advances by two calls between decisions, so keep the
/// common transaction entirely on the stack.  Seeded auctions and other large
/// jumps spill only the excess entries below.
#[derive(Clone, Copy)]
struct PendingAuthoringStep<'a> {
    index: usize,
    call: Call,
    own: Option<super::decoder::DecodedAuthoring<'a>>,
    own_exact: Option<&'a dyn super::trie::Classifier>,
    own_compiled: Option<&'a super::rules::CompiledRules>,
    routed: Option<super::decoder::DecodedAuthoring<'a>>,
    routed_compiled: Option<&'a super::rules::CompiledRules>,
}

/// Whether retaining this call's authored effect can omit no observable work.
///
/// Non-rule classifiers have no projection effect. Rule-backed routes require
/// a plan compiled for the current profile and its explicit purity proof;
/// otherwise public face/projection hooks must keep replaying on every read.
fn authored_effect_is_reusable(
    made: Call,
    classifier: &dyn super::trie::Classifier,
    compiled: Option<&super::rules::CompiledRules>,
    profile: ReadingProfile,
) -> bool {
    classifier.as_rules().is_none()
        || compiled.is_some_and(|plan| {
            plan.can_reuse_authored_effect(made, profile.pass_exclusion_reading())
        })
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // retained for append-only auditability and future reuse
struct AuthoringStepRecord {
    call: Call,
    own: Option<CachedRoute>,
    routed: Option<CachedRoute>,
}

/// Deal-owned append-only cache for the authored-reading portion of inference.
///
/// It is created by `Table::bid_out_from`, never by `Context::new`.  Each call
/// is decoded and projected once, category accumulators retain the historical
/// own/opponent/Pass/probe fold order, and the snapshot is rotated to the next
/// actor in constant work.  A profile change, non-prefix query, or selected
/// opaque/dynamic route permanently drops this deal to the legacy reader.
pub(crate) struct AuthoringStepCache {
    stance_identity: Option<std::sync::Arc<super::book::StanceCacheIdentity>>,
    profile: Option<ReadingProfile>,
    vul: Option<contract_bridge::auction::RelativeVulnerability>,
    reader_parity: Option<usize>,
    phase: Option<super::book::Phase>,
    auction: Vec<Call>,
    context_cursor: super::context::ContextCursor,
    own_cursor: super::decoder::DecoderCursorState,
    routed_cursors: [super::decoder::DecoderCursorState; 3],
    own: AbsoluteProjection,
    opponents: AbsoluteProjection,
    passes: AbsoluteProjection,
    probed: AbsoluteProjection,
    records: Vec<AuthoringStepRecord>,
    snapshot: AuthoredProjection,
    disabled: bool,
    #[cfg(test)]
    successful_prepares: usize,
    #[cfg(test)]
    appended_steps: usize,
}

impl Default for AuthoringStepCache {
    fn default() -> Self {
        Self {
            stance_identity: None,
            profile: None,
            vul: None,
            reader_parity: None,
            phase: None,
            auction: Vec::new(),
            context_cursor: super::context::ContextCursor::new(),
            own_cursor: super::decoder::DecoderCursorState::default(),
            routed_cursors: std::array::from_fn(|_| super::decoder::DecoderCursorState::default()),
            own: AbsoluteProjection::unknown(),
            opponents: AbsoluteProjection::unknown(),
            passes: AbsoluteProjection::unknown(),
            probed: AbsoluteProjection::unknown(),
            records: Vec::new(),
            snapshot: AuthoredProjection::unknown(),
            disabled: false,
            #[cfg(test)]
            successful_prepares: 0,
            #[cfg(test)]
            appended_steps: 0,
        }
    }
}

impl AuthoringStepCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[cfg(test)]
    pub(crate) const fn coverage(&self) -> (usize, usize) {
        (self.successful_prepares, self.appended_steps)
    }

    /// Check the route identity cached for each record appended by the most
    /// recent successful prepare against both independent resolution paths.
    ///
    /// The decoder pool ID is the persisted classifier identity; the pointer
    /// comparison below is deliberately only a same-process oracle against the
    /// authoritative trie.  Comparing the full cached record also checks the
    /// exact fallback depth/index and typed-rebase count in
    /// [`Provenance`][super::trie::Provenance].
    #[cfg(test)]
    fn assert_new_route_records_match_one_shot(
        &self,
        stance: &super::book::Stance,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
        first_new: usize,
    ) {
        assert_eq!(self.auction, auction, "route audit auction differs");
        assert_eq!(
            self.records.len(),
            auction.len(),
            "route record count differs"
        );
        let phase = self.phase.expect("a successful prepare has a phase");
        let profile = self.profile.expect("a successful prepare has a profile");
        let reader_parity = self
            .reader_parity
            .expect("a successful prepare has a reader parity");
        let full_context = Context::new(vul, auction);

        for (index, record) in self.records.iter().enumerate().skip(first_new) {
            let prefix = &auction[..index];
            assert_eq!(
                record.call, auction[index],
                "cached call differs at {index}"
            );

            if profile.fallback_projection {
                assert_cached_route_matches_one_shot(
                    "own",
                    index,
                    record.own,
                    stance.decoder_for_phase(phase),
                    stance.trie_for(auction),
                    &full_context,
                    prefix,
                );
            } else {
                assert_eq!(record.own, None, "disabled own route cached at {index}");
            }

            let opponent = index % 2 != reader_parity;
            let routed_needed = (profile.table_alerts && opponent)
                || (profile.pass
                    && record.call == Call::Pass
                    && (!opponent || profile.table_alerts));
            if routed_needed {
                let actor_vul = if index % 2 == reader_parity {
                    vul
                } else {
                    super::context::flipped(vul)
                };
                let at_time = Context::new(actor_vul, prefix);
                assert_cached_route_matches_one_shot(
                    "routed",
                    index,
                    record.routed,
                    stance.decoder_for(prefix),
                    stance.trie_for(prefix),
                    &at_time,
                    prefix,
                );
            } else {
                assert_eq!(
                    record.routed, None,
                    "unneeded routed route cached at {index}"
                );
            }
        }
    }

    fn phase_index(phase: super::book::Phase) -> usize {
        match phase {
            super::book::Phase::Constructive => 0,
            super::book::Phase::Competitive => 1,
            super::book::Phase::Defensive => 2,
        }
    }

    fn clear_steps(&mut self, phase: super::book::Phase) {
        self.phase = Some(phase);
        self.auction.clear();
        self.context_cursor = super::context::ContextCursor::new();
        self.own_cursor = super::decoder::DecoderCursorState::default();
        self.routed_cursors =
            std::array::from_fn(|_| super::decoder::DecoderCursorState::default());
        self.own = AbsoluteProjection::unknown();
        self.opponents = AbsoluteProjection::unknown();
        self.passes = AbsoluteProjection::unknown();
        self.probed = AbsoluteProjection::unknown();
        self.records.clear();
        self.snapshot = AuthoredProjection::unknown();
    }

    fn disable(&mut self) -> Option<&AuthoredProjection> {
        self.disabled = true;
        None
    }

    pub(crate) fn prepare<'a>(
        &'a mut self,
        stance: &super::book::Stance,
        vul: contract_bridge::auction::RelativeVulnerability,
        auction: &[Call],
    ) -> Option<&'a AuthoredProjection> {
        if self.disabled {
            return None;
        }
        match &self.stance_identity {
            None => self.stance_identity = Some(std::sync::Arc::clone(stance.cache_identity())),
            Some(identity) if std::sync::Arc::ptr_eq(identity, stance.cache_identity()) => {}
            Some(_) => return self.disable(),
        }
        let profile = reading_profile();
        let reader_parity = auction.len() % 2;
        match (self.profile, self.vul, self.reader_parity) {
            (None, None, None) => {
                self.profile = Some(profile);
                self.vul = Some(vul);
                self.reader_parity = Some(reader_parity);
            }
            (Some(old_profile), Some(old_vul), Some(old_parity))
                if old_profile == profile && old_vul == vul && old_parity == reader_parity => {}
            _ => return self.disable(),
        }
        if !auction.starts_with(&self.auction) {
            return self.disable();
        }

        let mut full_cursor = self.context_cursor.clone();
        for &call in &auction[self.auction.len()..] {
            full_cursor.push(call);
        }
        let phase = full_cursor.phase();
        let phase_changed = self.phase != Some(phase);
        if phase_changed {
            full_cursor = super::context::ContextCursor::new();
            for &call in auction {
                full_cursor.push(call);
            }
        }
        let full_context = full_cursor.context(vul, auction);
        let announce_split = announced_reading();
        let read_passes = pass_reading();
        let table_alerts = table_alert_reading();
        let fallback_projection = fallback_projection_enabled();

        // Route the whole append before consulting any authored projection.
        // An opaque route at the second (normally opponent) call must not make
        // an earlier public face/projection hook run speculatively before the
        // caller falls back to the legacy full-auction reader.
        let start = if phase_changed { 0 } else { self.auction.len() };
        let mut scan_context_cursor = if phase_changed {
            super::context::ContextCursor::new()
        } else {
            self.context_cursor.clone()
        };
        let mut scan_own_cursor = if phase_changed {
            super::decoder::DecoderCursorState::default()
        } else {
            core::mem::take(&mut self.own_cursor)
        };
        let mut scan_routed_cursors = if phase_changed {
            std::array::from_fn(|_| super::decoder::DecoderCursorState::default())
        } else {
            core::mem::take(&mut self.routed_cursors)
        };
        let mut pending_inline: [Option<PendingAuthoringStep<'_>>; 2] = [None; 2];
        let mut pending_inline_len = 0usize;
        let mut pending_spill = Vec::<PendingAuthoringStep<'_>>::new();

        for index in start..auction.len() {
            let prefix = &auction[..index];
            let made = auction[index];
            let actor_vul = if index % 2 == reader_parity {
                vul
            } else {
                super::context::flipped(vul)
            };
            let at_time = scan_context_cursor.context(actor_vul, prefix);

            let own_decoder = stance.decoder_for_phase(phase);
            let own_answer = if fallback_projection {
                match own_decoder.resolve_checked_with_cursor(
                    &mut scan_own_cursor,
                    &full_context,
                    prefix,
                ) {
                    super::decoder::CheckedResolution::Decoded(answer) => answer,
                    super::decoder::CheckedResolution::Opaque => return self.disable(),
                }
            } else {
                None
            };
            if fallback_projection && !scan_own_cursor.cache_stable() {
                return self.disable();
            }
            let own_exact = if fallback_projection {
                None
            } else {
                stance.trie_for(auction).get(prefix)
            };
            let own_classifier = own_answer.map(|answer| answer.classifier).or(own_exact);
            let own_compiled = own_classifier
                .and_then(|classifier| stance.compiled_rules_for(auction, classifier, profile));
            if own_classifier.is_some_and(|classifier| {
                !authored_effect_is_reusable(made, classifier, own_compiled, profile)
            }) {
                return self.disable();
            }

            let opponent = index % 2 != reader_parity;
            let routed_needed = (table_alerts && opponent)
                || (read_passes && made == Call::Pass && (!opponent || table_alerts));
            let routed_phase = scan_context_cursor.phase();
            let routed_decoder = stance.decoder_for_phase(routed_phase);
            let routed_answer = if routed_needed {
                match routed_decoder.resolve_checked_with_cursor(
                    &mut scan_routed_cursors[Self::phase_index(routed_phase)],
                    &at_time,
                    prefix,
                ) {
                    super::decoder::CheckedResolution::Decoded(answer) => answer,
                    super::decoder::CheckedResolution::Opaque => return self.disable(),
                }
            } else {
                None
            };
            if routed_needed && !scan_routed_cursors[Self::phase_index(routed_phase)].cache_stable()
            {
                return self.disable();
            }
            let routed_compiled = routed_answer
                .and_then(|answer| stance.compiled_rules_for(prefix, answer.classifier, profile));
            if routed_answer.is_some_and(|answer| {
                !authored_effect_is_reusable(made, answer.classifier, routed_compiled, profile)
            }) {
                return self.disable();
            }

            let pending = PendingAuthoringStep {
                index,
                call: made,
                own: own_answer,
                own_exact,
                own_compiled,
                routed: routed_answer,
                routed_compiled,
            };
            if pending_inline_len < pending_inline.len() {
                pending_inline[pending_inline_len] = Some(pending);
                pending_inline_len += 1;
            } else {
                pending_spill.push(pending);
            }
            scan_context_cursor.push(made);
        }

        // Every needed route is now known cache-safe. Reset on a phase change
        // only at this commit point, then replay the append in its original
        // call/category order so observable public hooks retain legacy order.
        if phase_changed {
            self.clear_steps(phase);
        }
        for pending in pending_inline[..pending_inline_len]
            .iter()
            .flatten()
            .chain(&pending_spill)
        {
            let index = pending.index;
            let prefix = &auction[..index];
            let made = pending.call;
            let actor_vul = if index % 2 == reader_parity {
                vul
            } else {
                super::context::flipped(vul)
            };
            let at_time = self.context_cursor.context(actor_vul, prefix);

            if let Some(answer) = pending.own {
                if let Some(effect) = authored_effect(
                    made,
                    &at_time,
                    answer.classifier,
                    pending.own_compiled,
                    false,
                    announce_split,
                ) {
                    self.own.push(index, effect);
                }
            } else if !fallback_projection
                && let Some(classifier) = pending.own_exact
                && let Some(effect) = authored_effect(
                    made,
                    &at_time,
                    classifier,
                    pending.own_compiled,
                    false,
                    announce_split,
                )
            {
                self.own.push(index, effect);
            }

            let opponent = index % 2 != reader_parity;
            let routed_answer = pending.routed;
            if let Some(answer) = routed_answer {
                if table_alerts
                    && opponent
                    && let Some(effect) = authored_effect(
                        made,
                        &at_time,
                        answer.classifier,
                        pending.routed_compiled,
                        false,
                        announce_split,
                    )
                {
                    self.opponents.push(index, effect);
                }
                if read_passes
                    && made == Call::Pass
                    && (!opponent || table_alerts)
                    && let Some(effect) = authored_effect(
                        made,
                        &at_time,
                        answer.classifier,
                        pending.routed_compiled,
                        true,
                        announce_split,
                    )
                {
                    self.passes.push(index, effect);
                }
            }

            if probed_reading()
                && let Some(&box_) = stance.probed_box(&auction[..=index])
            {
                let union = EnvelopeUnion::from(box_);
                self.probed.push(
                    index,
                    AuthoredEffect {
                        projection: super::rules::ProjectedUnion::Owned(union),
                        agreement: None,
                        suppresses_natural: false,
                    },
                );
            }
            self.records.push(AuthoringStepRecord {
                call: made,
                own: pending.own.map(Into::into),
                routed: routed_answer.map(Into::into),
            });
            self.context_cursor.push(made);
            self.auction.push(made);
            #[cfg(test)]
            {
                self.appended_steps += 1;
            }
        }
        self.own_cursor = scan_own_cursor;
        self.routed_cursors = scan_routed_cursors;

        let mut snapshot = AuthoredProjection::unknown();
        for absolute in 0..4 {
            let relative = (absolute + 4 - auction.len() % 4) % 4;
            for category in [&self.own, &self.opponents, &self.passes, &self.probed] {
                snapshot.unions[relative].intersect_assign(&category.unions[absolute]);
                snapshot.announced_unions[relative]
                    .intersect_assign(&category.announced_unions[absolute]);
                snapshot.suppressed |= category.suppressed;
            }
        }
        self.snapshot = snapshot;
        #[cfg(test)]
        {
            self.successful_prepares += 1;
        }
        Some(&self.snapshot)
    }
}

fn project_authored(context: &Context<'_>) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
    if let Some(projection) = context.authored_projection() {
        return projection.cloned_parts();
    }
    project_authored_with(context, true)
}

/// Same-process semantic oracle retained for compilation parity tests.
#[cfg(test)]
fn project_authored_legacy(context: &Context<'_>) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
    project_authored_with(context, false)
}

#[cfg(test)]
pub(crate) fn assert_compiled_authoring_projection_parity(context: &Context<'_>) {
    let compiled = project_authored(context);
    let legacy = project_authored_legacy(context);
    assert_eq!(compiled.0, legacy.0, "authored projection boxes differ");
    assert_eq!(compiled.1, legacy.1, "authored announcement boxes differ");
    assert_eq!(compiled.2, legacy.2, "authored suppression mask differs");
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn assert_cached_route_matches_one_shot(
    lane: &str,
    index: usize,
    cached: Option<CachedRoute>,
    decoder: &super::decoder::AuthoringDecoder,
    trie: &super::trie::Trie,
    context: &Context<'_>,
    prefix: &[Call],
) {
    let decoded = decoder.resolve(context, prefix);
    let legacy = trie.resolve(context, prefix);
    match (decoded, legacy) {
        (None, None) => {}
        (Some(decoded), Some((legacy_classifier, legacy_provenance))) => {
            assert!(
                core::ptr::eq(decoded.classifier, legacy_classifier),
                "{lane} classifier differs at call {index}"
            );
            assert_eq!(
                decoded.provenance.depth, legacy_provenance.depth,
                "{lane} fallback depth differs at call {index}"
            );
            assert_eq!(
                decoded.provenance.fallback, legacy_provenance.fallback,
                "{lane} fallback index differs at call {index}"
            );
            assert_eq!(
                decoded.provenance.rebases, legacy_provenance.rebases,
                "{lane} typed-rebase count differs at call {index}"
            );
        }
        (decoded, legacy) => panic!(
            "{lane} route presence differs at call {index}: decoder={}, legacy={}",
            decoded.is_some(),
            legacy.is_some()
        ),
    }

    let decoded = decoded.map(CachedRoute::from);
    match (cached, decoded) {
        (None, None) => {}
        (Some(cached), Some(decoded)) => {
            assert_eq!(
                cached.classifier_id, decoded.classifier_id,
                "{lane} cached classifier ID differs at call {index}"
            );
            assert_eq!(
                cached.provenance.depth, decoded.provenance.depth,
                "{lane} cached fallback depth differs at call {index}"
            );
            assert_eq!(
                cached.provenance.fallback, decoded.provenance.fallback,
                "{lane} cached fallback index differs at call {index}"
            );
            assert_eq!(
                cached.provenance.rebases, decoded.provenance.rebases,
                "{lane} cached typed-rebase count differs at call {index}"
            );
            assert_eq!(
                cached.pattern_id, decoded.pattern_id,
                "{lane} cached pattern ID differs at call {index}"
            );
            assert_eq!(
                cached.table_id, decoded.table_id,
                "{lane} cached rule-table ID differs at call {index}"
            );
            assert_eq!(
                cached.fast, decoded.fast,
                "{lane} cached route stability differs at call {index}"
            );
        }
        (cached, decoded) => panic!(
            "{lane} cached route presence differs at call {index}: cached={}, decoded={}",
            cached.is_some(),
            decoded.is_some()
        ),
    }
}

#[cfg(test)]
pub(crate) fn assert_step_cache_projection_parity(
    stance: &super::book::Stance,
    vul: contract_bridge::auction::RelativeVulnerability,
    auction: &[Call],
    cache: &mut AuthoringStepCache,
) -> bool {
    let expected = project_authored_legacy(&stance.prefixed_context(vul, auction));
    let old_phase = cache.phase;
    let old_record_count = cache.records.len();
    let Some(actual) = cache.prepare(stance, vul, auction).cloned() else {
        return false;
    };
    let first_new = if old_phase == cache.phase {
        old_record_count
    } else {
        0
    };
    cache.assert_new_route_records_match_one_shot(stance, vul, auction, first_new);
    assert_eq!(actual.unions, expected.0, "step-cache projection differs");
    assert_eq!(
        actual.announced_unions, expected.1,
        "step-cache announcement differs"
    );
    assert_eq!(
        actual.suppressed, expected.2,
        "step-cache suppression differs"
    );
    true
}

fn project_authored_with(
    context: &Context<'_>,
    compiled_reader: bool,
) -> ([EnvelopeUnion; 4], [EnvelopeUnion; 4], u64) {
    let auction = context.auction();
    let len = auction.len();
    let mut projection = AuthoredProjection::unknown();

    let Some(prefixes) = context.prefixes() else {
        return projection.into_parts();
    };

    let read_passes = pass_reading();
    let table_alerts = table_alert_reading();
    let fallback_projection = fallback_projection_enabled();
    let announce_split = announced_reading();
    let profile = context.reading_profile();

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
                            compiled: Option<&super::rules::CompiledRules>,
                            decode_pass: bool| {
        let Some(&made) = auction.get(index) else {
            return;
        };
        if let Some(effect) =
            authored_effect(made, ctx, classifier, compiled, decode_pass, announce_split)
        {
            projection.apply(len, index, effect);
        }
    };

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
    // Resolve each enabled authoring route in one forward scan.  `own` deliberately
    // uses the reader's final-phase book and full-auction guard context, just
    // like the historical first pass.  `routed` uses the phase of each actor's
    // turn and their at-the-time context; the opponent-alert and Pass passes
    // consume the same answer later without resolving the prefix again.  Keep
    // the scans separate and in legacy category order: besides avoiding work
    // when a reading mode is disabled, public opaque guards may be stateful.
    let at_times = if compiled_reader {
        Context::at_each_turn(context.vul(), auction)
    } else {
        (0..=len)
            .map(|index| {
                let vul = if index % 2 == len % 2 {
                    context.vul()
                } else {
                    super::context::flipped(context.vul())
                };
                Context::new(vul, &auction[..index])
            })
            .collect()
    };
    let stance = compiled_reader.then(|| context.their_system()).flatten();

    let decoded_own = if fallback_projection {
        if let Some(stance) = stance {
            let mut own_cursor = stance.decoder_for(auction).cursor_with_capacity(len);
            let mut own = Vec::with_capacity(len);
            for index in 0..len {
                let prefix = &auction[..index];
                match own_cursor.resolve_checked(context, prefix) {
                    super::decoder::CheckedResolution::Decoded(answer) => own.push(answer),
                    super::decoder::CheckedResolution::Opaque => {
                        return project_authored_with(context, false);
                    }
                }
            }
            Some(own)
        } else {
            None
        }
    } else {
        None
    };
    let decode_routed = table_alerts || read_passes;
    let decoded_routed = if decode_routed {
        if let Some(stance) = stance {
            let mut constructive = stance
                .decoder_for_phase(super::book::Phase::Constructive)
                .cursor_with_capacity(len);
            let mut competitive = stance
                .decoder_for_phase(super::book::Phase::Competitive)
                .cursor_with_capacity(len);
            let mut defensive = stance
                .decoder_for_phase(super::book::Phase::Defensive)
                .cursor_with_capacity(len);
            let mut routed = Vec::with_capacity(len);
            for index in 0..len {
                let prefix = &auction[..index];
                let opponent = index % 2 != len % 2;
                let needed = (table_alerts && opponent)
                    || (read_passes && auction[index] == Call::Pass && (!opponent || table_alerts));
                if !needed {
                    routed.push(None);
                    continue;
                }
                let resolution = match super::book::Phase::of(prefix) {
                    super::book::Phase::Constructive => {
                        constructive.resolve_checked(&at_times[index], prefix)
                    }
                    super::book::Phase::Competitive => {
                        competitive.resolve_checked(&at_times[index], prefix)
                    }
                    super::book::Phase::Defensive => {
                        defensive.resolve_checked(&at_times[index], prefix)
                    }
                };
                match resolution {
                    super::decoder::CheckedResolution::Decoded(answer) => routed.push(answer),
                    super::decoder::CheckedResolution::Opaque => {
                        return project_authored_with(context, false);
                    }
                }
            }
            Some(routed)
        } else {
            None
        }
    } else {
        None
    };

    if fallback_projection {
        // Decode every prior call by the classifier that *authored* it — node or
        // guarded fallback — so contested conventions (transfers, Leaping Michaels,
        // the Lebensohl cue) survive later competition without a per-convention reader.
        if let Some(own) = &decoded_own {
            for (index, answer) in own.iter().enumerate() {
                let Some(answer) = answer else { continue };
                let classifier = answer.classifier;
                let compiled = context
                    .their_system()
                    .and_then(|stance| stance.compiled_rules_for(auction, classifier, profile));
                project_call(&at_times[index], index, classifier, compiled, false);
            }
        } else {
            let trie = prefixes.root();
            for index in 0..len {
                if let Some(classifier) = trie.authoring_classifier(context, &auction[..index]) {
                    project_call(&at_times[index], index, classifier, None, false);
                }
            }
        }
    } else {
        // Exact-node classifiers only (fallback projection, the shipped
        // default, takes the branch above); fallback-authored conventions are
        // then read by the hand-written readers in [`Inferences::read`].
        for (prefix, classifier) in prefixes.clone() {
            let compiled = compiled_reader
                .then(|| context.their_system())
                .flatten()
                .and_then(|stance| stance.compiled_rules_for(auction, classifier, profile));
            project_call(
                &at_times[prefix.len()],
                prefix.len(),
                classifier,
                compiled,
                false,
            );
        }
    }

    // Alerts are table-wide disclosure: the opponents' alerted calls are
    // explained to us, so decode them too — each resolved in *their*
    // phase-routed book (the attached stance models them as playing our own
    // books) under their at-the-time context, exactly as it was classified
    // when made.  Their unalerted (natural) calls keep the natural walk.
    if table_alerts && let Some(them) = context.their_system() {
        for index in ((len + 1) % 2..len).step_by(2) {
            let prefix = &auction[..index];
            let answer = decoded_routed.as_ref().and_then(|routed| routed[index]);
            if let Some(answer) = answer {
                let classifier = answer.classifier;
                let compiled = them.compiled_rules_for(prefix, classifier, profile);
                project_call(&at_times[index], index, classifier, compiled, false);
            } else if decoded_routed.is_none()
                && let Some(classifier) = them
                    .trie_for(prefix)
                    .authoring_classifier(&at_times[index], prefix)
            {
                project_call(&at_times[index], index, classifier, None, false);
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
        for index in 0..len {
            if auction[index] != Call::Pass {
                continue;
            }
            let own_side = index % 2 == len % 2;
            if !own_side && !table_alerts {
                continue;
            }
            let prefix = &auction[..index];
            let answer = decoded_routed.as_ref().and_then(|routed| routed[index]);
            if let Some(answer) = answer {
                let classifier = answer.classifier;
                let compiled = them.compiled_rules_for(prefix, classifier, profile);
                project_call(&at_times[index], index, classifier, compiled, true);
            } else if decoded_routed.is_none()
                && let Some(classifier) = them
                    .trie_for(prefix)
                    .authoring_classifier(&at_times[index], prefix)
            {
                project_call(&at_times[index], index, classifier, None, true);
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
                let union = EnvelopeUnion::from(box_);
                projection.unions[who].intersect_assign(&union);
                projection.announced_unions[who].intersect_assign(&union);
            }
        }
    }

    projection.into_parts()
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
pub(crate) fn artificial(projection: &Envelope, made: Call, doubled: Option<Strain>) -> bool {
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
    // One `Cell<NotrumpDefense>` holds one system, so "Natural is active" is the
    // whole test: the four "…but not DONT/Meckwell/direct-Landy/Woolsey"
    // disjuncts this used to carry were the pre-fold precedence cascade, and
    // every one of them was unreachable once the enum landed.
    if !a::natural_defense_enabled() {
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
mod tests;
