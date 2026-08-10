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
pub struct ReadingProfile {
    /// Quantify a natural notrump raise of our own `1NT` opening
    ///
    /// **Default on.**  `1NT - 2NT` is invitational (≈8–9) and `1NT - 3NT`
    /// game-going (10+); recording that is what lets opener — and the sampler
    /// behind the search floor — judge whether game is good without a
    /// hand-authored node.  Off reproduces the pre-fix behaviour, where opener
    /// was blind to responder's strength and so could not accept an
    /// invitation; the `nt-invite-abc` example A/Bs the two.
    pub nt_invite: bool,

    /// Record what a one-level Rubens transfer means
    ///
    /// **Default on.**  The two-level cue-raise always recorded its limit-plus
    /// raise, but the one-level transfers were suppress-only, leaving the
    /// overcaller — and the sampler behind the search floor — blind to the
    /// support, length and strength they show.  On, a transfer into partner's
    /// suit records three-plus cards in the overcall suit and a new-suit
    /// transfer five-plus in its target, both ten-plus points.  Off recovers
    /// the suppress-only reading, where the transfers showed nothing
    /// (`bba-gen --no-ns-rubens-reading` for that arm).
    pub rubens_transfer: bool,

    /// How much of the authored book the projection pass decodes
    ///
    /// **Default [`ReadingScope::Alerted`]**; see the enum for the three
    /// stances and what each one leaves unread.
    pub scope: ReadingScope,

    /// Decode calls authored by *guarded fallbacks*, not just exact-node
    /// classifiers
    ///
    /// Every contested convention — transfers, Leaping Michaels, the Lebensohl
    /// cue — is authored by a guarded fallback.  On, the projection pass
    /// re-resolves each prior call's *authoring* classifier through the trie's
    /// node-then-fallback chain and projects its alerted rule, so an alerted
    /// call survives later competition without a per-convention hand reader.
    /// Off restores the exact-node-only projection (via
    /// [`common_prefixes`][crate::bidding::Trie::common_prefixes]), where a
    /// contested convention misreads under second-round intervention unless a
    /// hand-written reader covers it.  This is the general decode for
    /// non-natural calls, subsuming the single-suit per-convention readers;
    /// the OR-disjunction two-suiters and the doubles still need theirs.
    ///
    /// **Default on**, A/B'd on the BBA match: plain +0.0006/board,
    /// +1.03/fired; PD +0.0014, +2.38/fired — both CIs exclude 0.
    pub fallback_projection: bool,

    /// Store a reading as an *envelope union* rather than one bounding hull
    ///
    /// Off, a disjunctive reading (`Or`, `AnyLen`, a call authored by several
    /// rules) widens to its bounding box and the sampler accepts the whole
    /// hull — the legacy path, kept as the kill-switch.  On, the reading keeps
    /// its separate boxes and the sampler accepts a layout only if it lies in
    /// *some* box, pinning two-suiters / Multi / the fit-split instead of the
    /// box that spans them.
    ///
    /// **Default on** since chop F2b (docs/dnf-migration.md): with the
    /// knob-matched evaluator twin and the statically pinned Jacoby box the
    /// flip measured a win in all four cells — plain +0.0094/+0.0080, PD
    /// +0.0118/+0.0085 NV/vul, CIs clear (204,800 boards/arm/vul, seed
    /// 1784809754).
    ///
    /// Two hull regimes were measured (chop F1).  On a **bare**
    /// `Context::new` — no projection overlay, the `dump-teacher` path — the
    /// hulls are knob-invariant, byte-identical over a 21K-row dump.  On a
    /// **prefixed** context (`Stance::infer`, what the bidder, the floor net
    /// and the bilans evaluator actually see) the authored-projection overlay
    /// tightens with this on (⊤→box upgrades, `envelope_union_upgrade`), so
    /// those consumers' inputs move with it.
    pub envelope_union: bool,

    /// Blank what the *opponents* have shown (**default off**, measurement
    /// only)
    ///
    /// On, [`Inferences`][super::Inferences] hands back [`Envelope::unknown`][super::Envelope::unknown] /
    /// [`EnvelopeUnion::unknown`][super::EnvelopeUnion::unknown] for
    /// [`Relative::Lho`][super::Relative::Lho] and
    /// [`Relative::Rho`][super::Relative::Rho]; partner and the actor keep
    /// their live readings.  This is the `blind` arm of the deviation panel
    /// (docs/deviation-panel.md): the paired `seen − blind` score is what our
    /// reading of *their* calls is worth against one perturbed opponent, a
    /// statistic that stays honest when the deviant system is itself weaker.
    ///
    /// Distinct from
    /// [`features::set_blind_inference`][crate::bidding::features::set_blind_inference],
    /// which blanks all four seats and only for the nets — this one cuts at
    /// the source, so the sampler, the floor and the evaluator all go blind
    /// together.  The two are **not** comparable; the historical blind control
    /// was the other knob.
    pub blind_opponents: bool,

    /// Give the strength gauges membership teeth (**default off**, chop E of
    /// docs/dnf-migration.md)
    ///
    /// Off, [`Envelope::admits`][super::Envelope::admits] — and everything
    /// routed through it: sampler acceptance,
    /// [`EnvelopeUnion::contains`][super::EnvelopeUnion::contains], the
    /// envelope-union overlay — tests suit lengths and the legacy `points`
    /// gauge only, the pre-`Strength` behaviour.  On, a sampled hand must also
    /// fall within the box's raw-HCP, support-points and per-suit HCP bands,
    /// so a 15–17 `1NT` stops admitting 13-counts the `points` scale upgraded.
    ///
    /// The measured "bidding-inert" verdict (chop E: 0 fired in 409,600
    /// boards) **predates** both the suit-indexed `supports` gauging and the
    /// `suit_hcp` axis — those bands have real teeth (an Ogust quality ceiling
    /// rejects hands no whole-hand gauge can), so a future flip owes a fresh
    /// measurement.  Deliberately its own cell, never folded into
    /// [`envelope_union`][field@Self::envelope_union]: the mechanisms are
    /// independent (gauge bands tighten sampling even on the single-hull
    /// reading), the recorded DNF sd-lead WASH stays meaningful, and this is
    /// the one chop that can *reject legal hands* if a projection over-claims
    /// — the book-wide eval ⟹ membership sweep is its soundness gate, and this
    /// cell its kill-switch.
    pub gauge_membership: bool,

    /// C1: narrow each box's suit lengths to what `Σ len = 13` implies
    ///
    /// **Default off** (chop C of docs/dnf-migration.md).
    /// `Envelope::sum_feasible` only *tests* the sum; nothing narrows with it,
    /// so a both-majors reading stores `{♠ 5..13, ♥ 5..13, ♦ 0..13, ♣ 0..13}`
    /// when the truth is `{♠ 5..8, ♥ 5..8, ♦ 0..3, ♣ 0..3}` — eight of the
    /// thirteen cards are already spoken for.  `Envelope::narrow_to_sum`
    /// carries the sweep and its exactness proof.
    ///
    /// Its own cell, never folded into
    /// [`upgrade_closure`][field@Self::upgrade_closure]: the two closures read
    /// different axes, and the A/B measures them apart before stacking.
    /// Requires [`envelope_union`][field@Self::envelope_union] — the union-off hull
    /// path stays byte-identical.
    ///
    /// The closure is exact (it drops no hand the box actually claims) and
    /// **membership-inert**: it reads and writes only `lengths`, an axis
    /// `admits` already enforces, and every real 13-card hand satisfies
    /// `Σ len = 13` — zero rejections in either direction over 409,708 sampled
    /// layouts (`examples/probe-closure-features.rs`).  What does move is
    /// `EnvelopeUnion::hull`: tighter, so the bilans evaluator and the feature
    /// nets see tighter bands.
    pub sum_closure: bool,

    /// C2: close `hcp` against `points` through the shape upgrade
    ///
    /// **Default off** (chop C of docs/dnf-migration.md).  `points` is `hcp`
    /// plus a shape-and-honor
    /// [`upgrade`][crate::bidding::constraint::upgrade], and *balanced hands
    /// never upgrade* (every balanced shape holds at most 9 cards in its two
    /// longest suits).  So a box whose lengths force balanced reads
    /// `points == hcp`, where [`points`][crate::bidding::constraint::points]'
    /// projection slacks `hcp` by the scale's **global** worst case — a 2-HCP
    /// leak at each end of the most-read strength gate in the book.  See
    /// `Envelope::narrow_to_upgrade`.
    ///
    /// Exact, but **not** membership-inert: it bounds `points`, which `admits`
    /// tests, using `hcp`, which `admits` ignores until
    /// [`gauge_membership`][field@Self::gauge_membership] — so it gives an otherwise
    /// unenforced HCP claim teeth *through* `points`.  A box reading
    /// `hcp ..=8` with `points` slacked to `..=10` narrows back to `..=8` once
    /// its lengths force balanced, and the sampler stops dealing the 9- and
    /// 10-counts it was accepting outside the stated band: 249 rejections in
    /// 8,576 layouts on the same probe.  Arguably the old acceptance was the
    /// wrong one — but this is a reading change, and it must be measured as
    /// one.  Requires [`envelope_union`][field@Self::envelope_union].
    pub upgrade_closure: bool,

    /// Read a high (four-plus level) new suit as a control bid, not to-play
    ///
    /// **Default on** (M6.4).  The deterministic rule, distilled from Bridge
    /// World Standard: such a bid is *natural* iff the suit could still be the
    /// bidder's longest — the bidder has shown no other suit yet, or is
    /// rebidding a suit they themselves showed.  Otherwise it is a control bid
    /// agreeing the partnership's most recently shown suit, and the phantom
    /// suit is suppressed rather than floored.  Off, a four-plus-level new
    /// suit falls back to the pre-M6.4 reading (double jumps skipped,
    /// notrump-structure bids blanket-suppressed) — the A/B off arm.
    ///
    /// Shared with the instinct floor, whose `partner_control_bid` witness is
    /// this reading's own, so the reader and the signoff rules flip together.
    pub control_bid: bool,

    /// Read a bid of a suit only the *opponents* have naturally shown as a cue
    ///
    /// **Default on**, shipped 2026-07-18.  On, such a bid is a cue, never a
    /// holding: the phantom length the natural walk floored is suppressed, and
    /// the two robust standard meanings are recorded — a defender's direct cue
    /// of their one- or two-level minor opening is Michaels / Leaping Michaels
    /// (both majors, five-five), and a non-jump cue opposite exactly one
    /// natural suit on partner's side is the limit-plus cue-raise (three-plus
    /// support, ten-plus points).  The BEN Info-net probe
    /// (docs/ben-gap-campaign.md) caught the phantoms as truth violations on
    /// self-play: `(1♣) 2♣` Michaels read as five clubs on a void,
    /// `1♥ (2♦) 3♦` cue-raise read as four diamonds on two.
    ///
    /// Shipped as reading soundness, not as a bidding win — it is
    /// **bid-inert** in the default system (see [`pass`][field@Self::pass] for the
    /// three-knob guard cells), so a plain/PD A/B is a wash by construction.
    pub cue: bool,

    /// Relax two over-tight natural length floors to sound ones
    ///
    /// **Default on**, shipped 2026-07-18.  On, opener's immediate two-level
    /// rebid of the opened minor reads five-plus, not six-plus — the floor
    /// routinely rebids a good five — and a player who has doubled earlier no
    /// longer has a later jump in a new suit read as a weak six-card jump (a
    /// doubler's jump is strength, made on as few as three cards, so the walk
    /// claims nothing).  Both over-claims were caught by the BEN Info-net
    /// probe as truth violations on self-play.
    ///
    /// The one reading knob with a live bidding delta — 23/6400 divergent
    /// boards — and shipped default-on by the dual-reference A/B: plain DD a
    /// wash on both references, perfect defense positive everywhere.  Vs BBA
    /// +0.0022/+0.0023 IMPs/board NV/vul (CIs clear of zero, 204.8k
    /// boards/cell), vs BEN Tier F +0.0020/+0.0015 (directionally consistent,
    /// 51.2k/cell); +0.4 to +1.1 IMPs per fired board.
    /// `--no-ns-length-soundness` is the off-switch.
    pub length_soundness: bool,

    /// Read a pass off its own table's Pass gate
    ///
    /// **Default on**, shipped 2026-07-18.  The general reading of a pass is
    /// **negative inference — it excludes every bid and double the passer's
    /// table offered** (jdh8).  In a well-authored table that complement is
    /// already written down as the Pass rule's own gate (the opening table
    /// passes on `points(..12)` *because* the bids cover 12+), so on, the
    /// projection decodes each pass off the union of its authored table's Pass
    /// gates, both bounds
    /// ([`project_band`][crate::bidding::constraint::Constraint::project_band]):
    /// a no-open pass caps at 11 points, the silent responder to a suit
    /// one-opening at 5 HCP (10 on the upgraded scale), a pass of partner's
    /// `1NT` at 13 with no six-card major, a direct-seat pass over their suit
    /// opening at 17 HCP ("strong hands double first regardless").  Tables
    /// whose pass gate is a trivial catch-all — trap-pass advances, deep
    /// continuations — and floor (unauthored) passes correctly read nothing.
    /// Own-side passes decode from the reader's own book; the *opponents'*
    /// passes only under [`table_alerts`][field@Self::table_alerts], off their
    /// phase-routed book, like the rest of table-wide disclosure.
    ///
    /// The BEN Info-net probe (docs/ben-gap-campaign.md) found passes ~60% of
    /// all reading vagueness: we showed 0–37 where BEN commits ~6.3 mean HCP
    /// on a passed hand — the biggest reading hole by volume.  Shipped
    /// default-on with [`cue`][field@Self::cue] and
    /// [`table_alerts`][field@Self::table_alerts] as reading soundness: all three
    /// are **bid-inert** in the default system (0, 0 and 1 divergent boards
    /// per 211k same-seed guard cells — the on-disk witness in
    /// `ab-results/reading-knobs/2026-07-17/`), so a plain/PD A/B is a wash by
    /// construction and the ship gate was the probe's soundness numbers (0 new
    /// truth violations; acted-seat vagueness −60%).  Their payoff realizes
    /// wherever readings are consumed: sd-lead pricing, search-mode sampling,
    /// disclosure.
    pub pass: bool,

    /// A pass also excludes the sibling gates it declined
    ///
    /// **Default off, permanently opt-in.**  [`pass`][field@Self::pass] reads a pass
    /// off the table's own Pass gates — which in a catch-all table
    /// (`hcp(0..)`: the weak-two and `1NT` defenses, trap-pass advances) says
    /// nothing, and those tables own the plurality of the reading census's
    /// fully-blind passes.  This completes the stated negative inference: the
    /// bidder is argmax over `weight + eval`, so a hand inside a sibling gate
    /// whose weight strictly beats **every** Pass rule's weight could not have
    /// passed — the passer lies in that gate's complement
    /// ([`Rule::project_complement_union`][crate::bidding::rules::Rule::project_complement_union]),
    /// and the pass band may be intersected with it.
    ///
    /// Only **single-box** complements are folded in — a shape-free
    /// single-conjunct tier, such as the weak-two defense's `points(17..)`
    /// strong double, whose complement is exactly `points(..=16)`.  A shaped
    /// or bounded gate complements to a union (or to ⊤ via an off-axis atom),
    /// where the per-box precision is not worth the term growth; skipping it
    /// costs precision, never soundness.
    ///
    /// It stays off because the feature retrain (`evaluator_v3_exclusion`,
    /// served automatically under this cell) recovered the net-OOD half of the
    /// pre-retrain loss but the re-measure A/B was a **wash in all four
    /// cells** — no PD win, no ship (details:
    /// `docs/ai-bidder/sampled-projection.md` § "The exclusion retrain").  Its
    /// payoff is wherever readings are consumed directly: sd-lead pricing,
    /// search-mode sampling, disclosure.
    pub pass_exclusion: bool,

    /// Fold the stance's behaviorally probed boxes into the projection overlay
    ///
    /// **Default off.**  The sampled-projection derivation, stored:
    /// [`Stance::probe`][crate::bidding::Stance::probe] bids self-play deals
    /// and records the widened bounding box of the hands that actually made
    /// each call, keyed by auction prefix.  On, the projection pass intersects
    /// each prior call's probed box into both overlays.  This is the only
    /// reader that reaches the floor's calls — a net's pass has no rule to
    /// project, and the census's residual blind head (their `1NT`/`2♣`/`2NT`
    /// openings' passers, fourth-seat passes) is exactly that territory.
    ///
    /// The boxes are **behavioral estimates**, widened at the edges (a sample
    /// bound is not a rule bound) — see `Observed::boxed` in
    /// [`book`][crate::bidding::Stance] for the exact slack.  A stance with an
    /// empty probed map reads identically either way.
    pub probed: bool,

    /// The probed overlay, scoped to where the symbolic reading has none
    ///
    /// The full probed fold ([`probed`][field@Self::probed]) was refuted as a
    /// bidding input: its boxes tightened axes that already read, on both
    /// sides, and the worst boards were penalty doubles of opponents misread
    /// as limited (docs/ai-bidder/sampled-projection.md, the v1 A/B).  This
    /// serves the same probed map through the failure-free slice:
    ///
    /// - **own-side calls only** — the probe replays *our* system, so its
    ///   boxes model partner correctly and opponents wrongly (the
    ///   self-referential caveat);
    /// - **fully-open axes only** — a probed axis folds in only where the
    ///   symbolic reading says nothing at all (points `0..=37`, a length
    ///   `0..=13`), so an axis that already reads is never tightened.  Latest
    ///   call first: the longest prefix is the sharpest conditioning, and once
    ///   it fills an axis, earlier keys leave it alone.
    ///
    /// The target is the measured coverage hole — contested free bids and
    /// raises the natural walk stamps nothing for (`1♦ (2♣) 2♠ - 3♠` all
    /// `0..13`, docs/reading-drift-handoff.md), which no symbolic reader
    /// reaches.  A third gate scopes the fold to **contested prefixes** (both
    /// sides have acted): filling constructive axes smoke-tested at −0.67
    /// IMPs/board of net-OOD grand blasts.  A stance with an empty probed map
    /// reads identically either way; when both probed cells are on, the full
    /// fold wins.
    ///
    /// **Default off**, opt-in pending a feature retrain: the A/B (2026-07-31,
    /// SEED_BASE 1785493701, 204,800 bd/arm/vul) lost in all four cells —
    /// plain −0.0467/−0.0658, PD −0.1118/−0.1337 at ~10% fired — the worst
    /// boards all the contested floor net acting on tightened partner boxes it
    /// never trained on (balancing on, doubling, getting redoubled) where the
    /// base arm settles.  Pre-registered reading: a pre-retrain loss is a
    /// floor, not a verdict (the pass-exclusion precedent) — the queued path
    /// is the probe-first retrain gate, then the F2b twin served under this
    /// cell.
    pub probed_vacuous: bool,

    /// Fold a second, *agreement* overlay off each rule's `announce_union` —
    /// what a call announces, beside what it projects
    ///
    /// **Default off, pending its A/B**; off, the announce overlay is a clone
    /// of the projection overlay and every reading is byte-identical.
    ///
    /// [`Inferences`][super::Inferences] serves two masters that want
    /// opposite things.  The
    /// sampler ([`sample_layouts`][crate::bidding::sampler::sample_layouts])
    /// needs a box that **contains** the truth, or it rejects the very hands
    /// the auction was bid on; disclosure needs the box the partnership
    /// **agreement** names, which is what partner reasons from and what the
    /// opponents are owed.  For an authored rule the two coincide and nothing
    /// here changes.  They part company exactly where a learned criterion
    /// decides: the evaluator net accepts hands no box contains, so its sound
    /// projection is ⊤ and always will be, and the call reads as nothing.
    ///
    /// On, every rule contributes a second overlay via
    /// [`announce`][crate::bidding::constraint::Constraint::announce], which
    /// defaults to `project` and diverges only where the rule used
    /// [`announced`][crate::bidding::constraint::announced].  The projection
    /// overlay — and so the sampler, `admits` and the opening-lead sampling —
    /// is untouched; the agreement overlay is what
    /// [`features`][crate::bidding::features] hands the nets.
    pub announced: bool,

    /// Decode the *opponents'* alerted calls too
    ///
    /// **Default on**, shipped 2026-07-18.  Alerting exists *for the
    /// opponents*: an alerted call is disclosed to the whole table, not just
    /// remembered by partner.  Off, the projection pass honors only half of
    /// that — it decodes a call off its authoring rule when the *reader's own*
    /// book authored it, so the opponents' alerted conventions (their Stayman,
    /// their splinter, their checkback) fall to the natural walk and can read
    /// as phantom suits.  On, the pass also resolves each opponent call in
    /// *their* phase-routed book — modeling them as playing our own books,
    /// exact in self-play and an approximation against other natural-family
    /// engines — under their at-the-time context, and decodes it when their
    /// rule alerts it.  Their unalerted (natural) calls still read by the
    /// walk.
    ///
    /// Shipped with [`cue`][field@Self::cue] and [`pass`][field@Self::pass] as reading
    /// soundness; it is the one of the three that was not perfectly bid-inert
    /// (1 divergent board per 211k same-seed guard cells).
    pub table_alerts: bool,

    /// Accept a sampled layout by *replaying the rule* that authored each bid
    ///
    /// **Default on.**  The meaning is frozen at the node, so it survives
    /// later competition, instead of being re-derived by the per-convention
    /// range readers; the search EV ([`ev_all`][crate::bidding::ev_all])
    /// samples its rollout worlds this way — pons's analog of BEN's soft
    /// NN-replay gate.  Off restores range-only sampling; the
    /// `ab-search-floor` example A/Bs the two via `--no-rule-accept`.  See
    /// [`sample_layouts_replay`][crate::bidding::sampler::sample_layouts_replay].
    pub rule_accept: bool,

    /// The point scale every strength gauge counts on
    ///
    /// **Default [`PointCount`][crate::bidding::constraint::PointScale::PointCount]**
    /// — raw HCP plus the capped shape upgrade (0–2: +1 unbalanced, +1 for a
    /// ten-card two-suiter, −1 per wasted short honor).  History: the
    /// deprecation A/B/C once deposed this legacy scale for rule of N+8, but
    /// that was against the *cliff* upgrade (the first wasted honor voided the
    /// whole bonus); linearising the upgrade flipped the verdict back — vs
    /// rule-of-N+8-floored, PointCount measured a plain-DD wash at both
    /// vulnerabilities and **PD +0.023 NV / +0.037 vul** (two disjoint ~90k
    /// board/vul seeds, deterministic floor so the CI is the whole error
    /// budget; no retrain — the net consumes the upgrade directly,
    /// scale-free).  The capped shape reads wild two-suiters and freaks
    /// lighter than N+8's bonus-of-5, cutting the overbids PD punishes.  Rule
    /// of N+8 ([`RuleOfNFloored`][crate::bidding::constraint::PointScale::RuleOfNFloored])
    /// is now the opt-out, and
    /// [`Hcp`][crate::bidding::constraint::PointScale::Hcp] gauges raw HCP.
    pub point_scale: crate::bidding::constraint::PointScale,

    /// Gauge the fit-known shortness scale on `support_points`
    ///
    /// **Default on**: HCP plus useful shortness, after BBO GIB, instead of
    /// the legacy raw-HCP-plus-upgrade `point_count`.  Shortness is a ruffing
    /// value, real only once a trump fit exists, so the scale is scoped to the
    /// **fit-known** gates only — never the global `point_count`, whose
    /// fit-unknown gates keep legacy `points` untouched.  A measured win on
    /// every scorer (`examples/ab-point-count`, 200k–500k boards/vul): plain
    /// DD +0.033/+0.054, perfect defense +0.005/+0.020, sd-lead +0.052
    /// (NV/vul) — all CIs clearing zero.  The unscoped global flip won bigger
    /// (sd-lead +0.28) but broke legacy gates on shaped hands before a fit;
    /// the scoped form captures the fit-known fraction without that
    /// regression.  Off is the A/B arm.
    ///
    /// Lives here beside [`point_scale`][field@Self::point_scale] rather than in
    /// `DecisionProfile`, because the reading layer gauges box membership on
    /// it too (`Envelope::supports`) — one cell, one home.
    pub support_points: bool,

    /// The deviation panel's antisymmetric strength adjustment (**default 0**,
    /// measurement only)
    ///
    /// The panel's B axis (docs/deviation-panel.md): a simulated natural
    /// bidder whose openings and overcalls are this many points lighter and
    /// whose responses and advances are this many points heavier.  0 is
    /// byte-identical to the authored card.  The antisymmetry is the point —
    /// pair-level calibration is preserved, so the partnership still stops in
    /// the same places and every authored continuation stays coherent.  Only
    /// the magnitude lives here; the direction is chosen per decision from the
    /// auction (`dial_shift`).
    ///
    /// Projections stay undialled either way — the dial appears only in
    /// [`Constraint::eval`][crate::bidding::constraint::Constraint::eval],
    /// never in the reading folds — so the deviant opponent keeps disclosing
    /// the authored meanings, and that mismatch is the deviation being
    /// measured.
    pub strength_dial: u8,

    /// Advance partner's simple overcall with Rubens transfers and the
    /// cue-raise
    ///
    /// **Default off since 2026-07-31 — a reversal on re-measure.**  The layer
    /// won its M6.3 A/B (2026-07-02, plain +0.0016 ±0.0015 with the CI
    /// excluding zero, PD −0.0009 wash, 1144 fired) once both sides'
    /// continuations were authored, and shipped default-on on that.
    /// Re-measured on the current system (`scripts/ab-rubens.sh`, 204,800
    /// bd/arm/vul, SEED_BASE 1785426828, sha 4485555) it **loses in all four
    /// cells** — plain −0.0009 ±0.0009 NV / −0.0008 ±0.0011 vul, PD −0.0014
    /// ±0.0011 / −0.0014 ±0.0013 (both PD CIs clear of zero), fired
    /// 0.11%/0.09%, −0.83/−0.85 plain and −1.29/−1.51 PD per fired board.  The
    /// default is now the natural ladder: raises stay natural (the limit
    /// distinction is lost, the honest natural price) and a natural two-level
    /// new-suit advance covers the hands the transfers covered.
    ///
    /// On, the calls from the cue up to just below partner's suit are
    /// transfers over a one-level overcall, and the cue is the limit-plus
    /// raise over a two-level one.  The firing rate fell with the verdict —
    /// 1144 fired at M6.3 against 218/193 now on the same 204,800 boards, so
    /// the floor around it moved and the transfers are reached far less often
    /// than when they won.  Kept for re-measure: the losing tail is over-reach
    /// (`1♦ (1♠) - (2♥)` climbing to a failing `4♠` where the natural arm
    /// stops), not one unauthored continuation, and the transfers were mostly
    /// not reached in the first place — over `1♣ (1♥) -` the bidder took the
    /// `2♦` transfer 0.4% of the time, holding six-plus *diamonds*
    /// (`probe-bba-constraints --mode rub-ch --ours`,
    /// docs/reader-retirement.md § The Rubens layer).
    ///
    /// One cell for the bidder and the reader: off, an advance in the band is
    /// a genuine suit, so this default also silences `rubens_reading`.
    pub rubens_advances: bool,

    /// The "once penalty, always penalty" latch
    ///
    /// **Default on** — DD-measured a clear penalty-X-bucket win with no
    /// regression: self-play X bucket −0.621 → −0.464 IMPs/action-board, vs
    /// BBA −2.716 → −2.329 IMPs/X-board.  The human rule: after our side's
    /// natural penalty double of their `1NT`, our later doubles read as
    /// **penalty** — we double their runout on a trump stack rather than for
    /// takeout on shortness, and partner leaves our double in rather than
    /// advancing it.  Keyed off the one penalty double the floor classifies
    /// today, so it is a no-op unless the natural defense is on.
    ///
    /// The floor and the inference walk's matching reading
    /// (`penalty_latch_double_reading`) read this one cell, so they cannot
    /// disagree about when a later double is penalty rather than takeout.
    pub penalty_latch: bool,

    /// Run **systems-on** advances after our natural `1NT` overcall
    ///
    /// **Default on** (measured a clean single-dummy-lead win over both minor
    /// and major openings).  On, the whole opening-`1NT` response structure —
    /// Stayman, Jacoby/minor transfers, Smolen — is grafted below `(1t) 1NT`,
    /// so `(1♦) 1NT` equals `(1♣) 1NT` equals an opening `1NT`, and a 15–18
    /// balanced overcall finds 4-4 major fits and right-sides via transfers
    /// (the strong overcaller declares).  Off leaves the `(1t) 1NT -` advance
    /// to the instinct floor's naturals; `bba-gen
    /// --no-ns-nt-overcall-systems-on` is that arm.
    pub nt_overcall_systems_on: bool,

    /// Run **Gladiator** advances after our `1NT` overcall of their *major*
    ///
    /// **Default off** — an A/B candidate: the major graft washes plain and
    /// PD, and wins only on sd-lead.  On, it replaces the systems-on graft for
    /// major openings only (minors keep systems-on) with `2♣` = weak relay
    /// (pass-or-correct to the best partscore), the cue of their major =
    /// Stayman for the single unbid major, natural `2♦`/`2M` = 5-card
    /// invitational, `2NT` = non-forcing invitational clubs, plus splinter and
    /// Leaping-Michaels shape actions — so the strong hand declares and
    /// advancer can invite, force, or escape a double.  Independent of
    /// [`nt_overcall_systems_on`][field@Self::nt_overcall_systems_on], which governs
    /// the minor branch.  `bba-gen --ns-nt-overcall-gladiator` is the on arm.
    pub nt_overcall_gladiator: bool,

    /// Author responder's `1NT - 3M` splinter
    ///
    /// **Default on** since 2026-07-28: 5M boards per vulnerability, a win in
    /// all four cells — +0.56/+0.67 IMPs per fired board at none, +0.69/+0.81
    /// at both (plain/PD), measured by `examples/ab-nt-splinter`.
    ///
    /// Shortness in the *bid* major, 2–3 in the other, exactly four diamonds
    /// and five or six clubs — the Bridge World Standard / Polish Club
    /// treatment of the two slots our ladder leaves empty.  The hand class it
    /// serves (`3-1-4-5` and mirrors) has no other home: too few majors for
    /// Stayman, too few diamonds for the `2NT` transfer, too few clubs for the
    /// `2♠` transfer, and not `balanced()` for Puppet `3♣`, so at 9+ HCP it
    /// blasts `3NT` and at 8 it *passes* `1NT` holding a singleton opposite
    /// 15-17.  Since `♣ = 9 − (both majors)` the shape is closed: `3-1-4-5`,
    /// `1-2-4-6` and `0-3-4-6` (and mirrors); the `♣6` rows also qualify for
    /// the `2♠` club transfer and this outranks it deliberately, because after
    /// `2♠` responder can never show the four diamonds — the whole 6-4 slam
    /// lane.
    ///
    /// On, the inference engine also stops reading a three-level major over
    /// our `1NT` as a natural five-card suit and decodes it off the alert
    /// instead, and opener answers from an authored table
    /// (`nt_splinter_answer`) rather than the floor, which measured inert here
    /// — given the whole pinned shape it bid `3NT` on every hand.
    pub nt_splinter: bool,

    /// Author opener's strength-showing rebid ladder after a one-level
    /// response
    ///
    /// **Default on** (BBA-gap bucket #3): +0.0203/+0.0332 plain,
    /// +0.0181/+0.0297 PD IMPs/board vs BBA, NV/vul, all CIs > 0.  Without it
    /// opener's only long-suit rebid is a minimum natural `2m`/`2M` with no
    /// upper bound, so a strong single- or two-suiter underbids and the
    /// auction dies below game — the largest un-worked lever in the
    /// Constructive/book/round-2 anchor bucket.  The three rungs, disjoint
    /// from the minimum by crisp point bands:
    ///
    /// - **jump-rebid** of opener's suit (`1♦ - 1♠ - 3♦`): a self-sufficient
    ///   6+ suit, 16+ points, invitational;
    /// - **reverse** into a higher new suit (`1♦ - 1♠ - 2♥`): 5+ first suit,
    ///   4+ second, 17+ points, forcing;
    /// - **jump-shift** into a new suit (`1♦ - 1♠ - 3♣`): 5-4, 18+ points,
    ///   game-forcing.
    ///
    /// Only the two minor-opening rebid nodes carry the full ladder; the
    /// major-opening nodes carry the jump-rebid rung alone
    /// ([`opener_major_jump_rebid`][field@Self::opener_major_jump_rebid]).  The
    /// matching reading gates on this same cell, narrowing each rung's shape
    /// and strength.
    pub opener_extras_ladder: bool,

    /// Author XYZ — the two-way checkback after three one-level bids
    ///
    /// **Default on**, shipped with `up_the_line` (`ab-minor-continuations`,
    /// 300k boards: the pair is plain +0.0382/+0.0559 IMPs/board NV/vul, PD
    /// +0.0289/+0.0407; XYZ alone is plain +0.504/+0.795 per divergent, PD
    /// +0.332/+0.472 — a win on both scorers).  `--no-ns-xyz` in `bba-gen` is
    /// the off arm, where nothing is authored.
    ///
    /// On the ten uncontested auctions where our side made three bids at the
    /// one level (`1x - 1y - 1z`, `z` a suit or notrump): responder's **`2♣`
    /// puppets opener to `2♦`** — either a weak hand signing off in diamonds
    /// (passes `2♦`) or any invitational hand (continues naturally) — and
    /// **`2♦` is an artificial game force**, after which bidding is natural.
    /// Direct two-level rebids are weak sign-offs.  The known cost: the
    /// natural `2♣` sign-off becomes an orphan.
    pub xyz: bool,

    /// Which minor scheme our `1NT` plays — the alert its `2♠`/`2NT`/`3♣`
    /// calls carry
    ///
    /// **Default [`PUPPET`][crate::bidding::american::PUPPET]**; the opt-in
    /// [`EUROPEAN`][crate::bidding::american::EUROPEAN] is BBA's Atlantic
    /// style — `2♠` = clubs (transfer), `2NT` = a balanced invite / size ask,
    /// `3♣` = diamonds (transfer), no Puppet Stayman — the standard Polish
    /// Club / WJ and common continental response set.  Both variants are
    /// authored; only the selected one's `2♠`/`2NT`/`3♣` rules are gated into
    /// the trie, and the inference engine reads the same cell to decode them.
    pub notrump_minors: crate::bidding::rules::Alert,

    /// Author opener's `3M` jump rebid of a six-card major with extras
    ///
    /// **Default on** (the BBA-gap bucket #3 residual): +0.0059/+0.0125 plain,
    /// +0.0046/+0.0104 PD IMPs/board vs BBA, NV/vul, all CIs > 0.  The
    /// [extras ladder][field@Self::opener_extras_ladder] covers only the two
    /// minor-opening rebid nodes; the major-opening nodes (`1♥ - 1♠` and the
    /// forcing-`1NT` rebid) otherwise cap opener's own-major rebid at a
    /// minimum `2M` with no upper bound, so a 16+ hand with a strong six-card
    /// major underbids and misses the game BBA reaches (`3♥ → 4♥`, `2♥ → 3♥`,
    /// `3♠ → 4♠`).
    ///
    /// This adds the single jump-rebid `3M` (6+ suit, 16+ points), disjoint
    /// from the `2M` minimum by a crisp point band, **plus responder's
    /// continuation** (raise `4M` on an eight-card fit, `3NT` with no fit,
    /// pass with a minimum).  The bare rung *without* the continuation
    /// measured a loss — −0.005/−0.009 plain, responder passing the
    /// invitational `3M` and stranding below game — and authoring both sides
    /// flipped it to a win.  Scoped to opener's *own* suit to avoid the
    /// Meckstroth `3m` collision on the jump-shift rung; natural, so unalerted
    /// and floor-safe, with the matching reading on this same cell.
    pub opener_major_jump_rebid: bool,

    /// Author garbage (drop-dead) Stayman
    ///
    /// **Default on** — a paired DD A/B vs BBA (205k boards, vul none)
    /// measured +0.51 IMPs/fired plain (+0.0009/board, 95% CI excluding 0) and
    /// +0.70 PD.  A weak responder with short clubs and a four-card major bids
    /// `2♣` to escape a likely-doomed `1NT`, intending to pass opener's
    /// `2♦`/`2♥`/`2♠`.  It is looser the weaker responder is: broke hands
    /// accept a thinner `2♦` landing, since any ~7-card fit beats a broke
    /// `1NT`.  The inference engine reads the same cell, to widen the `2♣`
    /// point range it reads.
    pub garbage_stayman: bool,

    /// Author Crawling Stayman — `1NT - 2♣ - 2♦ - 2♥` as pass-or-correct
    ///
    /// **Default on.**  The strict superset of garbage Stayman for 4-4 majors
    /// *short in diamonds* (4414/4405): garbage needs a safe `2♦` landing (3+
    /// diamonds) and so cannot escape a singleton or void diamond, while
    /// crawling bids `2♣` anyway and, over opener's `2♦` denial, crawls to
    /// `2♥` — both majors, pass-or-correct — rather than passing a doomed
    /// diamond partscore.  4-4 majors with ≤1 diamond forces 4+ clubs, so the
    /// `2♥` pass-or-correct (and opener's `3♣` flee) always finds a fit.  Weak
    /// only (`hcp(..8)`), disjoint from the constructive and garbage `2♣`
    /// tiers; the inference engine reads the same cell to widen the `2♣` point
    /// range.
    pub crawling_stayman: bool,

    /// `(min, max)` inclusive `points` band on the Woolsey suit overcalls
    ///
    /// The `2♣`/`2♦`/`2♥`/`2♠` calls of the Woolsey defense; **default
    /// (8, 19)**, level with the natural overcall floor.  A 2026-06-26
    /// re-probe (continuations by then fully authored) found honest plain-DD
    /// self-play *peaks at 8* and flattens below it — 6 and 7 add no value —
    /// and the BBA head-to-head agrees; perfect defense still mildly prefers
    /// 10, but PD over-deters by assuming a perfect doubler.  The conventions
    /// only rearrange *which* call shows a hand, so the strength floor tracks
    /// natural's 8.  Swept by `examples/ab-landy --ns-woolsey-range`; also the
    /// points floor the inference engine reads for those overcalls.  No effect
    /// unless [`notrump_defense`][field@Self::notrump_defense] is
    /// [`Woolsey`][crate::bidding::american::NotrumpDefense::Woolsey], and
    /// [`landy_range`][field@Self::landy_range] feeds this band when Landy is on.
    pub woolsey_points: (u8, u8),

    /// `points` floor on the Woolsey takeout `X` (4-card major + longer minor)
    ///
    /// **Default 12** — the `X` is the most constructive Woolsey action, so it
    /// floors above the preemptive suit overcalls.  Swept by
    /// `examples/ab-landy --ns-woolsey-x-floor`; no effect unless
    /// [`notrump_defense`][field@Self::notrump_defense] is
    /// [`Woolsey`][crate::bidding::american::NotrumpDefense::Woolsey].
    pub woolsey_double_floor: u8,

    /// HCP floor on the natural defense's penalty double of their `1NT`
    ///
    /// **Default 15.**  An A/B knob (`bba-match --ns-double-floor`); it pairs
    /// with [`natural_overcall_points`][field@Self::natural_overcall_points], whose
    /// ceiling decides which strong hands double rather than overcall.
    pub natural_double_floor: u8,

    /// Name the longer major when responding to our minor
    ///
    /// **Default on** — the established American treatment: a response to
    /// `1♣`/`1♦` names the longer major (`1♠` on 5♠4♥ or any 5-5+, `1♥` up the
    /// line only on 4-4), so partner can infer "spades are not longer than
    /// hearts" from `1♥`.  Off is the historic unconditional hearts-first
    /// pair (`--no-ns-longer-major-response` in `bba-gen`).
    ///
    /// The two measured a **null** (`ab-minor-continuations`, 2M boards:
    /// plain-DD wash, PD −0.12/−0.22 per divergent NV/vul; −0.003..−0.005
    /// IMPs/board marginal on the shipped XYZ + up-the-line package), and a
    /// push against a natural default goes to the natural method — the
    /// naturalness tiebreak (docs/measurement.md) — so the simplification is
    /// the opt-in.  The M6.4 control-bid classifier reads the same discipline
    /// at classify time: the response rule, the rebid structure and the
    /// classifier move together.
    pub longer_major_response: bool,

    /// The Landy `2♣` band, or `None` when Landy is off
    ///
    /// **Default `None`** — the natural defense's penalty double and natural
    /// two-level suit overcalls stand.  `Some((lo, hi))` turns Landy on: `2♣`
    /// shows at least 5-4 in the majors and `2NT` at least 5-4 in the minors,
    /// both on `points(lo..=hi)`, at the cost of the natural `2♣` club
    /// overcall.  The range is the A/B sweep knob (`examples/ab-landy
    /// --ns-majors`); the advancer's invite/game thresholds and the
    /// overcaller's min/med/max rebid track it, so a lighter overcall asks
    /// more of the advancer.
    ///
    /// It also *is* the shared two-suiter band: Landy's and Woolsey's
    /// both-majors `2♣` are the identical call, so setting a Landy range also
    /// feeds [`woolsey_points`][field@Self::woolsey_points] and the two can never
    /// carry divergent strengths.  (Measured: the `:19` cap binds on ~0 hands
    /// and the floor barely moves the IMPs, so one knob loses nothing.)
    pub landy_range: Option<(u8, u8)>,

    /// Which mutually-exclusive defense we play over their `1NT` opening
    ///
    /// **Default [`Natural`][crate::bidding::american::NotrumpDefense::Natural]**
    /// — penalty `X`, the four natural two-level overcalls and the owning
    /// `Pass` catch-all.  A defensive "system" is a *bundle* of per-call
    /// conventions: only the call carries a convention, not the system, so
    /// "Woolsey" is `X` = Woolsey + `2♣` = Landy + `2♦` = Multi + `2♥`/`2♠` =
    /// Muiderberg.  Every artificial call is authored once as an alerted
    /// block, all of them are chained at the `(1NT)` node, and only the active
    /// system's calls are gated into the trie.
    ///
    /// One cell, so the old "two families authored at once" state — possible
    /// with the independent per-system booleans and resolved only by a
    /// read-time precedence cascade — is unrepresentable.  The `DirectLandy`
    /// payload rides on `agreements.defense.direct_landy_four_four`, since the
    /// flat-4-4 flag has no meaning without the double it configures.
    pub notrump_defense: crate::bidding::american::NotrumpDefense,

    /// `(min, max)` inclusive `points` band on the natural two-level suit
    /// overcall of their `1NT`
    ///
    /// **Default (8, 14).**  Lifting the ceiling routes a strong shapely
    /// one-suiter into a suit overcall instead of letting it fall through to
    /// the penalty double, whose floor is
    /// [`natural_double_floor`][field@Self::natural_double_floor].  An A/B knob
    /// (`bba-match --ns-overcall LO:HI`).
    pub natural_overcall_points: (u8, u8),

    /// Open the strong `2NT` on the wide-minor shape
    ///
    /// **Default off**, which is byte-identical to the shipped card.  On, the
    /// 20-21 `2NT` opening admits `{M 2..=4, m 2..=6}` instead of plain
    /// `balanced()` — DNF-ledger chop G0 (docs/dnf-migration.md), the `1NT`
    /// `Wide6322` treatment carried up to the strong opening.  It **drops the
    /// 5M(332)** balanced hands (a five-card major now opens one-of-a-major
    /// and jump-rebuilds) and **adds the wide minors** (5m422/6m322),
    /// mirroring
    /// [`Wide6322`][crate::bidding::american::NotrumpShape::Wide6322] minus
    /// its two major pan-handles.  The opening reading widens opener's minors
    /// to six off this same cell.
    pub two_notrump_wide: bool,

    /// The floor asks and answers RKCB 1430 (M6.4)
    ///
    /// **Default on**: with a known eight-card fit and combined small-slam
    /// values the floor asks `4NT` before committing — the milestone
    /// six-of-the-fit only fires directly at the grand-zone 37, or when the
    /// ask has no room.  The answers reuse the book's 1430 ladder, so instinct
    /// decodes instinct on both sides.  Off recovers the direct-milestone
    /// floor (`bba-gen --no-ns-floor-rkcb`).
    ///
    /// **The outer gate of the whole keycard package.**  Off, there is no ask
    /// to relocate, so [`rkcb_variant`][field@Self::rkcb_variant] has no effect: the
    /// card discloses plain `4NT` and the plain twin serves the floor.
    pub floor_rkcb: bool,

    /// Where the keycard ask lives — the relocation stance of the 1430
    /// machinery
    ///
    /// **Default [`Plain`][crate::bidding::instinct::RkcbVariant::Plain]** —
    /// `4NT` for every trump.  [`Kickback`][crate::bidding::instinct::RkcbVariant::Kickback]
    /// was default-on 2026-08-02 to 2026-08-03 only, and measured a loss that
    /// survived the defence (see the variant's own doc for the numbers);
    /// [`Redwood`][crate::bidding::instinct::RkcbVariant::Redwood] is
    /// unmeasured as its own arm.  Either relocation implies the minors'
    /// reach whatever the minors knob says, because a ladder whose payoff is
    /// the minor lanes needs a minor to ask in.
    ///
    /// **The stance also selects the floor's weights.**  A relocation is not a
    /// rule the reader can hold alone: a net distilled from a kickback-blind
    /// teacher keeps bidding a natural `4♥` into the relocated ask, so
    /// `classify_bba` serves the kickback twin whenever a relocation is live,
    /// and `Plain` restores both halves.  Rule *presence* is gated at
    /// [`instinct`][crate::bidding::instinct()] build time, because the
    /// reading's `alerted` test is structural — it asks whether any rule on
    /// the made call carries an alert and never evaluates the constraint — so
    /// an always-present alerted rule on `4♠` would suppress the natural
    /// reading of *every* floor-classified `4♠` even in the plain stance.
    pub rkcb_variant: crate::bidding::instinct::RkcbVariant,
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
        crate::bidding::constraint::set_strength_dial(1);
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

    /// The deviation panel's antisymmetric strength adjustment
    /// ([`set_strength_dial`][crate::bidding::constraint::set_strength_dial],
    /// **default 0**, measurement only).  The magnitude only: the direction is
    /// chosen per decision from the auction (`dial_shift`).
    ///
    /// It used to be baked into each gauge as the constraint was *built*, on
    /// the grounds that a classify-time read would leak the dial into the book
    /// seated opposite on the same thread.  Pinning removed that leak — the
    /// deviant stance carries its own profile — so the dial reads from here
    /// like every other gauge setting, and no longer needs an `&Agreements` at
    /// 1431 `.rule()` sites to reach the three constructors.
    pub(crate) const fn strength_dial(self) -> u8 {
        self.strength_dial
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

    /// Whether opener's strength-showing ladder is authored.  Read at build
    /// time too (`american/rebids/extras_ladder.rs`), so the rebid book takes
    /// it from here — one cell, one home.
    pub(crate) const fn opener_extras_ladder(self) -> bool {
        self.opener_extras_ladder
    }

    /// Whether opener's `3M` jump rebid is authored.  Read at build time too
    /// (`american/rebids/major_jump_rebid.rs`), so the rebid book takes it from
    /// here — one cell, one home.
    pub(crate) const fn opener_major_jump_rebid(self) -> bool {
        self.opener_major_jump_rebid
    }

    /// Whether two-way checkback (XYZ) is authored.  Read at build time too
    /// (`american/xyz.rs` gates the whole package on it), so the rebid book
    /// takes it from here — one cell, one home.
    pub(crate) const fn xyz(self) -> bool {
        self.xyz
    }

    // The twelve dual cells the defensive and notrump books also build from.
    // They were briefly duplicated into `DefenseKnobs`/`NotrumpKnobs`, which
    // put one cell in two homes: harmless while the cells still back both
    // captures, silently divergent the moment the cells are deleted and the two
    // fields become independently settable.  The books read them from here.

    /// Which notrump defense we play against their `1NT`
    pub(crate) const fn notrump_defense(self) -> crate::bidding::american::NotrumpDefense {
        self.notrump_defense
    }

    /// Optional `(min, max)` HCP band overriding the Landy `2♣` overcall
    pub(crate) const fn landy_range(self) -> Option<(u8, u8)> {
        self.landy_range
    }

    /// `(min, max)` HCP band on the Woolsey defense's overcalls
    pub(crate) const fn woolsey_points(self) -> (u8, u8) {
        self.woolsey_points
    }

    /// HCP floor on the Woolsey defense's double
    pub(crate) const fn woolsey_double_floor(self) -> u8 {
        self.woolsey_double_floor
    }

    /// HCP floor on the natural defense's double
    pub(crate) const fn natural_double_floor(self) -> u8 {
        self.natural_double_floor
    }

    /// `(min, max)` HCP band on the natural defense's overcalls
    pub(crate) const fn natural_overcall_points(self) -> (u8, u8) {
        self.natural_overcall_points
    }

    /// Whether our structure stays on over their notrump overcall
    pub(crate) const fn nt_overcall_systems_on(self) -> bool {
        self.nt_overcall_systems_on
    }

    /// Whether the Gladiator major overcall of their `1NT` is authored
    pub(crate) const fn nt_overcall_gladiator(self) -> bool {
        self.nt_overcall_gladiator
    }

    /// Whether the `1NT - 3M` splinter is authored
    pub(crate) const fn nt_splinter(self) -> bool {
        self.nt_splinter
    }

    /// Which minor scheme our `1NT` plays — the alert the `2♠`/`3♣` calls carry
    pub(crate) const fn notrump_minors(self) -> crate::bidding::rules::Alert {
        self.notrump_minors
    }

    /// Whether garbage Stayman is authored
    pub(crate) const fn garbage_stayman(self) -> bool {
        self.garbage_stayman
    }

    /// Whether crawling Stayman is authored
    pub(crate) const fn crawling_stayman(self) -> bool {
        self.crawling_stayman
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
}

/// The shipped reading agreements
///
/// The literals the `thread_local!` cells above were initialised with, lifted
/// to where a value can carry them.  Held equal to
/// `reading_profile` on a virgin thread by
/// `decision_defaults_match_the_cells`.
impl Default for ReadingProfile {
    fn default() -> Self {
        Self {
            nt_invite: true,
            rubens_transfer: true,
            scope: ReadingScope::Alerted,
            fallback_projection: true,
            envelope_union: true,
            blind_opponents: false,
            gauge_membership: false,
            sum_closure: false,
            upgrade_closure: false,
            control_bid: true,
            cue: true,
            length_soundness: true,
            pass: true,
            pass_exclusion: false,
            probed: false,
            probed_vacuous: false,
            announced: false,
            table_alerts: true,
            rule_accept: true,
            point_scale: crate::bidding::constraint::PointScale::PointCount,
            support_points: true,
            strength_dial: 0,
            rubens_advances: false,
            penalty_latch: true,
            nt_overcall_systems_on: true,
            nt_overcall_gladiator: false,
            nt_splinter: true,
            opener_extras_ladder: true,
            xyz: true,
            notrump_minors: crate::bidding::american::notrump::PUPPET,
            opener_major_jump_rebid: true,
            garbage_stayman: true,
            crawling_stayman: true,
            woolsey_points: (8, 19),
            woolsey_double_floor: 12,
            natural_double_floor: 15,
            longer_major_response: true,
            landy_range: None,
            notrump_defense: crate::bidding::american::NotrumpDefense::Natural,
            natural_overcall_points: (8, 14),
            two_notrump_wide: false,
            floor_rkcb: true,
            rkcb_variant: crate::bidding::instinct::RkcbVariant::Plain,
        }
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
        strength_dial: crate::bidding::constraint::strength_dial(),
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
