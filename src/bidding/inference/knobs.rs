//! Thread-local reading knobs and the profile snapshot that keys the caches
//!
//! Every knob here is read at **classify time**, not at book construction, so a
//! parallel harness must arm it inside the worker closure.  The
//! [`ReadingProfile`] snapshot exists so a cached projection can tell whether the
//! knob state it was computed under still holds.

use std::cell::Cell;

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

/// Whether the [`set_nt_invite_inference`] knob is on
pub fn nt_invite_inference() -> bool {
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

/// Whether the [`set_rubens_transfer_reading`] knob is on
pub fn rubens_transfer_reading() -> bool {
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
/// [`NotrumpDefense`][crate::bidding::american::NotrumpDefense] precedent.
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
pub(super) fn reading_scope() -> ReadingScope {
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
    /// See [`sample_layouts_replay`][crate::bidding::sampler::sample_layouts_replay].
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
/// [`common_prefixes`][crate::bidding::Trie::common_prefixes]), so a contested convention
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
/// On, [`Inferences`][super::Inferences] hands back
/// [`Envelope::unknown`][super::Envelope::unknown] /
/// [`EnvelopeUnion::unknown`][super::EnvelopeUnion::unknown] for
/// [`Relative::Lho`][super::Relative::Lho] and
/// [`Relative::Rho`][super::Relative::Rho]; partner and the actor keep their
/// live readings.  This is the `blind` arm of the deviation panel
/// (docs/deviation-panel.md): the paired `seen − blind` score is what our
/// reading of *their* calls is worth against one perturbed opponent, a
/// statistic that stays honest when the deviant system is itself weaker.
///
/// Distinct from [`features::set_blind_inference`][crate::bidding::features::set_blind_inference],
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
/// Off, [`Envelope::admits`][super::Envelope::admits] — and everything routed
/// through it: sampler acceptance,
/// [`EnvelopeUnion::contains`][super::EnvelopeUnion::contains], the
/// envelope-union overlay — tests suit lengths and
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
#[doc = include_str!("../closure-inertness.md")]
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
/// [`upgrade`][crate::bidding::constraint::upgrade], and *balanced hands never upgrade*
/// (every balanced shape holds at most 9 cards in its two longest suits).  So a
/// box whose lengths force balanced reads `points == hcp`, where
/// [`points`][crate::bidding::constraint::points]' projection slacks `hcp` by the
/// scale's **global** worst case — a 2-HCP leak at each end of the most-read
/// strength gate in the book.  See `Envelope::narrow_to_upgrade`.
///
/// Requires [`envelope_union_reading`].  Read at classification time, per-thread.
#[doc = include_str!("../closure-inertness.md")]
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
pub fn control_bid_reading() -> bool {
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

pub(super) fn cue_reading() -> bool {
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

pub(super) fn length_soundness() -> bool {
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
/// ([`project_band`][crate::bidding::constraint::Constraint::project_band]): a no-open
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

pub(super) fn pass_reading() -> bool {
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
/// ([`Rule::project_complement_union`][crate::bidding::rules::Rule::project_complement_union]),
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
/// The sampled-projection derivation, stored: [`Stance::probe`][crate::bidding::Stance::probe]
/// bids self-play deals and records the widened bounding box of the hands
/// that actually made each call, keyed by auction prefix.  On, the projection
/// pass intersects each prior call's probed box into both overlays.  This is
/// the only reader that reaches the floor's calls — a net's pass has no rule
/// to project, and the census's residual blind head (their 1NT/2♣/2NT
/// openings' passers, fourth-seat passes) is exactly that territory.
///
/// The boxes are **behavioral estimates**, widened at the edges (a sample
/// bound is not a rule bound) — see `Observed::boxed` in
/// [`book`][crate::bidding::Stance] for the exact slack.  A stance with an empty
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
/// the natural walk stamps nothing for (`1♦ (2♣) 2♠ - 3♠` all `0..13`,
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
    /// [`Rule::announce_union`][crate::bidding::rules::Rule::announce_union] (see
    /// [`set_announced_reading`]).  Off by default — knob-off the announce
    /// overlay is a clone of the projection overlay and every reading is
    /// byte-identical.
    static ANNOUNCED_READING: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the agreement overlay — what a call *announces*, beside what it projects
///
/// [`Inferences`] serves two masters that want opposite things.  The sampler
/// ([`sample_layouts`][crate::bidding::sampler::sample_layouts]) needs a box that
/// **contains** the truth, or it rejects the very hands the auction was bid on;
/// disclosure needs the box the partnership **agreement** names, which is what
/// partner reasons from and what the opponents are owed.  For an authored rule
/// the two coincide and nothing here changes.  They part company exactly where a
/// learned criterion decides: the evaluator net accepts hands no box contains, so
/// its sound projection is ⊤ and always will be, and the call reads as nothing.
///
/// On, every rule contributes a second overlay via
/// [`announce`][crate::bidding::constraint::Constraint::announce], which defaults to
/// `project` and diverges only where the rule used
/// [`announced`][crate::bidding::constraint::announced].  The projection overlay — and so
/// the sampler, `admits`, and the opening-lead sampling — is untouched; the
/// agreement overlay is what [`features`][crate::bidding::features] hands the nets.
///
/// Default **off** pending its A/B.  Read at reading time, per-thread.
#[doc(hidden)]
pub fn set_announced_reading(on: bool) {
    ANNOUNCED_READING.with(|cell| cell.set(on));
}

pub(super) fn announced_reading() -> bool {
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

pub(super) fn table_alert_reading() -> bool {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ReadingProfile {
    pub(super) nt_invite: bool,
    pub(super) rubens_transfer: bool,
    pub(super) scope: ReadingScope,
    pub(super) fallback_projection: bool,
    pub(super) envelope_union: bool,
    pub(super) blind_opponents: bool,
    pub(super) gauge_membership: bool,
    pub(super) sum_closure: bool,
    pub(super) upgrade_closure: bool,
    pub(super) control_bid: bool,
    pub(super) cue: bool,
    pub(super) length_soundness: bool,
    pub(super) pass: bool,
    pub(super) pass_exclusion: bool,
    pub(super) probed: bool,
    pub(super) probed_vacuous: bool,
    pub(super) announced: bool,
    pub(super) table_alerts: bool,
    pub(super) rule_accept: bool,
    pub(super) point_scale: crate::bidding::constraint::PointScale,
    pub(super) support_points: bool,
    pub(super) rubens_advances: bool,
    pub(super) penalty_latch: bool,
    pub(super) nt_overcall_systems_on: bool,
    pub(super) nt_overcall_gladiator: bool,
    pub(super) nt_splinter: bool,
    pub(super) opener_extras_ladder: bool,
    pub(super) xyz: bool,
    pub(super) notrump_minors: crate::bidding::rules::Alert,
    pub(super) opener_major_jump_rebid: bool,
    pub(super) garbage_stayman: bool,
    pub(super) crawling_stayman: bool,
    pub(super) woolsey_points: (u8, u8),
    pub(super) woolsey_double_floor: u8,
    pub(super) natural_double_floor: u8,
    pub(super) longer_major_response: bool,
    pub(super) landy_range: Option<(u8, u8)>,
    pub(super) notrump_defense: crate::bidding::american::NotrumpDefense,
    pub(super) natural_overcall_points: (u8, u8),
    pub(super) two_notrump_wide: bool,
    pub(super) floor_rkcb: bool,
    pub(super) rkcb_variant: crate::bidding::instinct::RkcbVariant,
}

impl ReadingProfile {
    /// Whether the Woolsey defense is the active notrump defense.
    pub(crate) fn woolsey_enabled(self) -> bool {
        self.notrump_defense == crate::bidding::american::NotrumpDefense::Woolsey
    }

    /// Whether the Meckwell defense is the active notrump defense.
    pub(crate) fn meckwell_enabled(self) -> bool {
        self.notrump_defense == crate::bidding::american::NotrumpDefense::Meckwell
    }

    /// Whether direct-seat DONT is the active notrump defense.
    pub(crate) fn direct_dont_enabled(self) -> bool {
        self.notrump_defense == crate::bidding::american::NotrumpDefense::DirectDont
    }

    /// Whether the natural defense is the active notrump defense.
    pub(crate) fn natural_defense_enabled(self) -> bool {
        self.notrump_defense == crate::bidding::american::NotrumpDefense::Natural
    }

    /// Drive every cell of this profile off its shipped default, so the
    /// cross-thread pinning test (`stance_pins_knobs_across_threads`) can tell
    /// a pinned read from a live one: a classify-time read that bypasses the
    /// pinned profile diverges on a virgin thread.
    ///
    /// The values are *different*, not meaningful — this arms a system nobody
    /// plays.  What must hold is only that both threads see the same one.
    #[cfg(test)]
    pub(crate) fn arm_all_nondefault() {
        use crate::bidding::american;
        use crate::bidding::instinct;

        set_nt_invite_inference(false);
        set_rubens_transfer_reading(false);
        set_reading_scope(ReadingScope::All);
        set_fallback_projection(false);
        set_envelope_union_reading(false);
        set_blind_opponent_reading(true);
        set_gauge_membership(true);
        set_sum_closure(true);
        set_upgrade_closure(true);
        set_control_bid_reading(false);
        set_cue_reading(false);
        set_length_soundness(false);
        set_pass_reading(false);
        set_pass_exclusion_reading(true);
        set_probed_reading(true);
        set_probed_vacuous_reading(true);
        set_announced_reading(true);
        set_table_alert_reading(false);
        set_rule_accept(false);
        crate::bidding::constraint::set_point_scale(crate::bidding::constraint::PointScale::Hcp);
        crate::bidding::constraint::set_support_points(false);
        instinct::set_rubens_advances(true);
        instinct::set_penalty_latch(false);
        instinct::set_floor_rkcb(false);
        instinct::set_rkcb_variant(instinct::RkcbVariant::Kickback);
        american::set_nt_overcall_systems_on(false);
        american::set_nt_overcall_gladiator(true);
        american::set_nt_splinter(false);
        american::set_opener_extras_ladder(false);
        american::set_xyz(false);
        american::set_notrump_minors(american::EUROPEAN);
        american::set_opener_major_jump_rebid(false);
        american::set_garbage_stayman(false);
        american::set_crawling_stayman(false);
        american::set_woolsey_points(9, 18);
        american::set_woolsey_double_floor(13);
        american::set_natural_double_floor(16);
        american::set_longer_major_response(false);
        american::set_landy(Some((8, 14)));
        american::set_notrump_defense(american::NotrumpDefense::Woolsey);
        american::set_natural_overcall_points(9, 13);
        american::set_two_notrump_wide(true);
    }

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

    pub(crate) const fn probed_reading(self) -> bool {
        self.probed
    }

    pub(crate) const fn probed_vacuous_reading(self) -> bool {
        self.probed_vacuous
    }

    pub(crate) const fn gauge_membership(self) -> bool {
        self.gauge_membership
    }

    /// Only [`ev`][crate::bidding::ev] gauges on this, so it follows that
    /// module's feature gate.
    #[cfg(feature = "dd")]
    pub(crate) const fn rule_accept(self) -> bool {
        self.rule_accept
    }

    /// The point scale every strength gauge counts on — [`point_count`] and the
    /// slack its projections owe raw HCP.
    ///
    /// [`point_count`]: crate::bidding::constraint::point_count
    pub(crate) const fn point_scale(self) -> crate::bidding::constraint::PointScale {
        self.point_scale
    }

    /// Whether the fit-known shortness scale gauges `support_points`.  Lives
    /// here beside [`point_scale`][Self::point_scale] rather than in
    /// `DecisionProfile`, because the reading layer gauges box membership on it
    /// too (`Envelope::supports`) — one cell, one home.
    pub(crate) const fn support_points(self) -> bool {
        self.support_points
    }

    /// Whether the strong `2NT` opening carries the wide-minor shape.  Read at
    /// build time too (`american/openings/two_notrump.rs` picks the shape gate
    /// off it), which is why the opening book takes it from here rather than
    /// keeping a cell of its own — one cell, one home.
    pub(crate) const fn two_notrump_wide(self) -> bool {
        self.two_notrump_wide
    }

    /// Whether a response to our minor names the longer major.  Read at build
    /// time too (`american/responses/longer_major.rs` picks the selector pair
    /// off it), which is why the response book takes it from here rather than
    /// keeping a cell of its own — one cell, one home.
    pub(crate) const fn longer_major_response(self) -> bool {
        self.longer_major_response
    }

    // The five cells the [instinct floor][crate::bidding::instinct()] shares
    // with the reading layer.  They live here, not in
    // [`InstinctProfile`][crate::bidding::instinct::InstinctProfile]: one cell,
    // one home, so the two halves of a decision profile cannot disagree about
    // what the partnership plays.

    /// Whether control bids are read.  The floor's `partner_control_bid`
    /// witness is the reading's own, so it must gate on the same cell.
    pub(crate) const fn control_bid(self) -> bool {
        self.control_bid
    }

    pub(crate) const fn floor_rkcb(self) -> bool {
        self.floor_rkcb
    }

    pub(crate) const fn rkcb_variant(self) -> crate::bidding::instinct::RkcbVariant {
        self.rkcb_variant
    }

    pub(crate) const fn rubens_advances(self) -> bool {
        self.rubens_advances
    }

    pub(crate) const fn penalty_latch(self) -> bool {
        self.penalty_latch
    }

    /// Drive the probed-overlay bit — [`Stance::probe`]'s fixed point runs by
    /// mutating the stance it probes, not the thread.
    ///
    /// [`Stance::probe`]: crate::bidding::Stance::probe
    pub(crate) const fn set_probed(&mut self, on: bool) {
        self.probed = on;
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
        point_scale: crate::bidding::constraint::point_scale(),
        support_points: crate::bidding::constraint::support_points_now(),
        rubens_advances: crate::bidding::instinct::rubens_advances_enabled(),
        penalty_latch: crate::bidding::instinct::penalty_latch_enabled(),
        nt_overcall_systems_on: crate::bidding::american::nt_overcall_systems_on(),
        nt_overcall_gladiator: crate::bidding::american::nt_overcall_gladiator(),
        nt_splinter: crate::bidding::american::nt_splinter(),
        opener_extras_ladder: crate::bidding::american::opener_extras_ladder(),
        xyz: crate::bidding::american::xyz(),
        notrump_minors: crate::bidding::american::notrump_minors(),
        opener_major_jump_rebid: crate::bidding::american::opener_major_jump_rebid(),
        garbage_stayman: crate::bidding::american::garbage_stayman(),
        crawling_stayman: crate::bidding::american::crawling_stayman(),
        woolsey_points: crate::bidding::american::woolsey_points(),
        woolsey_double_floor: crate::bidding::american::woolsey_double_floor(),
        natural_double_floor: crate::bidding::american::natural_double_floor(),
        longer_major_response: crate::bidding::american::longer_major_response(),
        landy_range: crate::bidding::american::landy_range(),
        notrump_defense: crate::bidding::american::notrump_defense(),
        natural_overcall_points: crate::bidding::american::natural_overcall_points(),
        two_notrump_wide: crate::bidding::american::two_notrump_wide(),
        floor_rkcb: crate::bidding::instinct::floor_rkcb_now(),
        rkcb_variant: crate::bidding::instinct::rkcb_variant_now(),
    }
}
