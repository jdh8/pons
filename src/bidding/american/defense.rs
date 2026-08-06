//! Defensive actions for the 2/1 system: overcalls, advances, and doubles
//!
//! This module covers everything our side does when the opponents open the
//! auction: simple overcalls, the 1NT overcall, takeout doubles, the
//! Michaels cue-bid, the Unusual 2NT, advances of all of these, advancing
//! partner's takeout double, responsive doubles when partner has made a
//! takeout double and they raise, and defense to a weak-two opening (takeout
//! double, a natural 2NT overcall, and natural suit overcalls).

use super::super::constraint::{
    Cons, Constraint, and, at_least_as_long, balanced, equal_length, hcp, len, length_box,
    long_suit_box, longer_suit, longest_unbid, min_level_is, or, passed_hand, points,
    points_by_vul, shapes, short_in_their_suits, stopper_in_their_suits, suit_hcp,
    takeout_double_shape_ok, top_honors, unbid_support,
};
use super::super::context::Context;
use super::super::fallback::{described_rewrite, rewriter};
use super::super::inference::Range;
use super::super::rows::{Entry, Package, Pattern, classified, compile_into, rebase, rows_of};
use super::super::trie::{Classifier, classifier};
use super::super::{Alert, Defensive, Rules, Trie};
use super::competition::{
    LebensohlStyle, clubs_transfer_completion, complete_lebensohl_relay, cue_stayman_answer,
    cue_stayman_answer_no_stopper, delayed_cue, lebensohl_relay_rebid, lebensohl_responder,
    lm_2d_both_majors_advance, lm_2d_clubs_ask, lm_2d_clubs_major, stayman_2d_answer,
    stayman_2d_fit_rebid, transfer_completion, transfer_lebensohl_responder,
    transfer_stayman_2d_responder, transfer_target,
};
use super::notrump::{flat_4333, smolen_at_three, smolen_completion};
use super::openings::two_notrump_wide_shape;
use super::{call, other_major};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

// ---------------------------------------------------------------------------
// Sohl after a takeout double (advancing partner's takeout double of a weak two)
// ---------------------------------------------------------------------------

thread_local! {
    /// Which sohl package the advancer carries after partner's takeout double of
    /// a weak two (`(2X)–X–(P)`); see [`set_advance_sohl_style`].
    static ADVANCE_SOHL: Cell<LebensohlStyle> = const { Cell::new(LebensohlStyle::Transfer) };
}

/// Select the sohl package the **advancer** carries after partner's takeout
/// double of a weak two, for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// Reuses [`LebensohlStyle`]: `Off` keeps the flat [`advance_double`] ladder;
/// `Plain` adds the weak `2NT` relay vs a forcing 3-level suit; `Transfer` (the
/// **default**) adds Larry Cohen's transfers-through + cue-Stayman, plus, over
/// `(2♦)`, `3♣`-Stayman + Smolen + Leaping Michaels. The geometry matches Lebensohl
/// after our overcalled `1NT` (the opponents' suit is at the two level in both),
/// so the Section-5 builders are reused verbatim under the `(2X)–X–(P)` prefix.
/// `Transfer` is the default because it is a clear perfect-defense win over the
/// flat ladder (+0.145/+0.227 IMPs/board none/both, 200k filtered).
/// See `docs/ai-bidder/21gf-ledger.md` for the full A/B numbers.
pub fn set_advance_sohl_style(style: LebensohlStyle) {
    ADVANCE_SOHL.with(|cell| cell.set(style));
}

/// The currently selected advance-of-double sohl package
fn advance_sohl_style() -> LebensohlStyle {
    ADVANCE_SOHL.with(Cell::get)
}

thread_local! {
    /// Whether Leaping Michaels (4♣/4♦ strong two-suiters over their weak two)
    /// is active; see [`set_leaping_michaels`].
    static LEAPING_MICHAELS: Cell<bool> = const { Cell::new(true) };
}

/// Toggle Leaping Michaels for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// Over their weak two, a jump to `4♣`/`4♦` names a 5-5 two-suiter with
/// game-forcing values: over a major it is a minor plus the *other* major; over
/// `2♦` the `4♦` cue shows both majors and `4♣` shows clubs plus a major.  **On by
/// default** — the authored advances make it a clear DD win (+1.090/+1.452
/// IMPs/board, none/both), and the inference reader lets the live-search bidder
/// price the advance (and reach slam) on top; see `docs/ai-bidder/21gf-ledger.md`.
/// Turn it off to recover the pre-Leaping-Michaels weak-two defense.
pub fn set_leaping_michaels(on: bool) {
    LEAPING_MICHAELS.with(|cell| cell.set(on));
}

/// Whether Leaping Michaels is currently enabled
///
/// Crate-visible so the inference projection pass can condition partner's hand on
/// the two-suiter when the search bidder samples (see `inference::authored_reading`).
pub(crate) fn leaping_michaels_enabled() -> bool {
    LEAPING_MICHAELS.with(Cell::get)
}

thread_local! {
    /// Landy defense to their 1NT: `None` = off (the default natural overcalls +
    /// penalty double); `Some((lo, hi))` = on, with `2♣` = both majors and
    /// `2NT` = both minors on `points(lo..=hi)`.  See [`set_landy`].
    static LANDY: Cell<Option<(u8, u8)>> = const { Cell::new(None) };
}

/// Configure the Landy defense to an opponent's 1NT for books built *after* this
/// call (thread-local, read once at book-construction time)
///
/// `None` (the **default**) keeps today's natural defense: a penalty double
/// (15+ balanced) and natural two-level suit overcalls.  `Some((lo, hi))` turns
/// Landy on: `2♣` shows at least 5-4 in the majors and `2NT` at least 5-4 in the
/// minors, both on `points(lo..=hi)`, at the cost of the natural `2♣` club
/// overcall.  The range is the A/B sweep knob (`examples/ab-landy --ns-majors`);
/// the advancer's invite/game thresholds and the overcaller's min/med/max
/// rebid track it, so a lighter overcall asks more of the advancer.  It also
/// *is* the shared two-suiter band — see [`set_woolsey_points`] — so Landy's and
/// Woolsey's identical both-majors `2♣` always overcall at the same strength.
pub fn set_landy(range: Option<(u8, u8)>) {
    LANDY.with(|cell| cell.set(range));
    // Coupled with Woolsey: the both-majors `2♣` is the identical call in both
    // conventions, so they share one strength band — the [`woolsey_points`] cell.
    // A Landy range feeds that band, so the two can never carry divergent strengths.
    // (Measured: the `:19` cap binds on ~0 hands and the floor barely moves the IMPs,
    // so one knob loses nothing; see `examples/ab-landy` / `bba-gen --ns-landy`.)
    if let Some((lo, hi)) = range {
        set_woolsey_points(lo, hi);
    }
}

/// The configured Landy range, or `None` when Landy is off
///
/// Crate-visible so the inference projection pass and the Landy relay stub can
/// condition partner on the two-suiter (see `inference::authored_reading` and
/// `inference::landy_advance_suppress`).
pub(crate) fn landy_range() -> Option<(u8, u8)> {
    LANDY.with(Cell::get)
}

thread_local! {
    /// The `(min minor length, max length in each major)` gate for the doubled-Landy
    /// minor escapes (`Pass` = clubs, `2♦` = diamonds).  **Default `(6, 2)`**.  See
    /// [`set_doubled_landy_escape`].
    static DOUBLED_LANDY_ESCAPE: Cell<(usize, usize)> = const { Cell::new((6, 2)) };
}

/// Tune the doubled-Landy minor-escape gate for books built *after* this call
/// (thread-local, read once at book-construction time)
///
/// After `[1NT, 2♣, X]` the advancer may run to a long minor — `Pass` to play `2♣`
/// doubled with clubs, `2♦` to play diamonds — but only with `min_minor`+ in that
/// minor and at most `max_major` in *each* major (a longer major has an 8-card fit
/// opposite the overcaller's 5-carder worth more than a doubled minor).  **The
/// default `(6, 2)`** is the A/B-tuned shipped gate; the knob is
/// `examples/landy-ab --ns-doubled-escape MIN:MAJ`.  Only reachable when Landy is
/// on ([`set_landy`]), so the convention stays opt-in.
pub fn set_doubled_landy_escape(gate: (usize, usize)) {
    DOUBLED_LANDY_ESCAPE.with(|cell| cell.set(gate));
}

/// The configured doubled-Landy minor-escape gate
fn doubled_landy_escape() -> (usize, usize) {
    DOUBLED_LANDY_ESCAPE.with(Cell::get)
}

thread_local! {
    /// The both-minors `2NT` overcall of their 1NT: `None` = off (the floor's
    /// natural — and near-useless — 2NT); `Some((lo, hi))` = both minors (5-5) on
    /// `points(lo..=hi)`.  **On by default** at `8..=13`; see
    /// [`set_unusual_notrump_defense`].
    static UNUSUAL_NT: Cell<Option<(u8, u8)>> = const { Cell::new(Some((8, 13))) };
}

/// Configure the both-minors `2NT` overcall of an opponent's 1NT for books built
/// *after* this call (thread-local, read once at book-construction time)
///
/// Independent of [`set_landy`]: a natural `2NT` over their strong 1NT is nearly
/// worthless, so this repurposes the bid as a both-minors (5-5) two-suiter on
/// `points(lo..=hi)` — purely additive, it sacrifices no natural call.  **On by
/// default at `Some((8, 13))`**: A/B'd vs the floor (`examples/landy-ab
/// --ns-minors`) it is a vulnerability-dependent wash on plain double-dummy
/// (≈+0.0001 IMPs/board non-vul, ≈−0.0001 vul), shipped on because it is additive
/// and its obstruction/lead-direction value is invisible to the DD measure; the
/// `8`-floor `13`-ceiling and the 5-5 shape were the best-measured settings
/// (capping strong hands and requiring 5-5 both helped).  `None` reverts to the
/// floor's natural `2NT`.
pub fn set_unusual_notrump_defense(range: Option<(u8, u8)>) {
    UNUSUAL_NT.with(|cell| cell.set(range));
}

/// The configured both-minors `2NT` range, or `None` when off
///
/// Crate-visible so the inference reader can condition partner on the two-suiter.
pub(crate) fn unusual_notrump_range() -> Option<(u8, u8)> {
    UNUSUAL_NT.with(Cell::get)
}

thread_local! {
    /// Whether the Landy `2♣` / unusual `2NT` strength range gauges raw [`hcp`]
    /// rather than the default shape-upgraded [`points`]; see [`set_landy_hcp`].
    static LANDY_HCP: Cell<bool> = const { Cell::new(false) };
}

/// Gauge the two-suiter overcall strength on raw HCP instead of upgraded points,
/// for books built *after* this call (thread-local, read once at book-construction)
///
/// A 5-4/5-5 two-suiter earns a distributional bonus, so [`points`] runs ~2 above
/// HCP — letting thin hands clear the floor.  `true` gauges the `2♣`/`2NT` range on
/// raw [`hcp`] (tighter); `false` (the **default**) keeps [`points`].  An A/B knob
/// (`examples/landy-ab --strength hcp`).
pub fn set_landy_hcp(on: bool) {
    LANDY_HCP.with(|cell| cell.set(on));
}

/// Whether the two-suiter strength range gauges raw HCP
fn landy_use_hcp() -> bool {
    LANDY_HCP.with(Cell::get)
}

/// Which mutually-exclusive defense our side plays over the opponents' 1NT opening
///
/// Exactly one system is active at a time.  Storing the choice in a single `Cell`
/// makes the old "two families authored at once" state — previously possible with
/// the independent `NATURAL_DEFENSE` / `DIRECT_DONT` / `MECKWELL` / `WOOLSEY` /
/// `ALWAYS_PASS_DEFENSE` booleans and resolved only by a read-time precedence
/// cascade — unrepresentable.  Read once at book-construction time.
///
/// [`set_notrump_defense`] is the only setter.  The five per-system bool shims
/// that survived the original fold were deleted 2026-08-03: their `false` arms
/// reverted to Natural *only if that system was the active one*, so a harness
/// resetting by calling every `set_*(false)` got an order-dependent result — and
/// keeping them let `bba-gen` and `ab-landy` re-implement the very cascade this
/// cell deleted.  The `DirectLandy` payload keeps a setter
/// ([`set_direct_landy_double`]) because the flat-4-4 flag has no meaning
/// without the double it configures.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum NotrumpDefense {
    /// Natural one-suiter defense: penalty `X` + the four natural two-level overcalls
    /// + the owning `Pass` catch-all.  The **default**.
    #[default]
    Natural,
    /// Direct-seat DONT (one-suiter `X`, two-suiter `2♣`/`2♦`/`2♥`, natural `2♠`).
    DirectDont,
    /// Direct-seat Meckwell (two-way `X`, minor+major `2♣`/`2♦`, natural majors).
    Meckwell,
    /// Woolsey "Multi-Landy" (`X` = 4-card major + longer minor, `2♣` = both majors,
    /// `2♦` = Multi, `2♥`/`2♠` = Muiderberg).
    Woolsey,
    /// Direct-seat both-majors takeout `X` (Landy-style); the 5-4-vs-4-4 shape flag
    /// lives in `DIRECT_LANDY_FOUR_FOUR`.
    DirectLandy,
    /// Author only `Pass` for every hand — our side never competes.
    AlwaysPass,
    /// Author nothing; the `[1NT]` node falls through to the bare instinct floor
    /// (what selecting this variant gives: no authored rules at the `[1NT]` node).
    Off,
}

thread_local! {
    /// The mutually-exclusive 1NT defense in force; **[`Natural`](NotrumpDefense::Natural)
    /// by default**.
    static NOTRUMP_DEFENSE: Cell<NotrumpDefense> = const { Cell::new(NotrumpDefense::Natural) };
    /// Whether the direct-Landy both-majors `X` accepts a flat 4-4 (else 5-4+) — the
    /// payload of the former `DIRECT_LANDY_DOUBLE` `Option`.  No effect unless the
    /// active system is [`NotrumpDefense::DirectLandy`].
    static DIRECT_LANDY_FOUR_FOUR: Cell<bool> = const { Cell::new(false) };
}

/// Select the mutually-exclusive 1NT defense for books built *after* this call
/// (thread-local, read once at book-construction time)
pub fn set_notrump_defense(system: NotrumpDefense) {
    NOTRUMP_DEFENSE.with(|cell| cell.set(system));
}

/// The mutually-exclusive 1NT defense currently selected
pub(crate) fn notrump_defense() -> NotrumpDefense {
    NOTRUMP_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Whether to also author the natural defense in the *balancing* seat
    /// `(1NT) P P ?`; **off by default** (opt-in A/B). Off leaves the balancing
    /// seat to the instinct floor — the source of the toxic balancing doubles.
    static NOTRUMP_BALANCING: Cell<bool> = const { Cell::new(false) };
}

/// Whether the natural one-suiter defense is currently the active system
pub(crate) fn natural_defense_enabled() -> bool {
    notrump_defense() == NotrumpDefense::Natural
}

/// Extend the natural 1NT defense to the *balancing* seat `(1NT) P P ?` for books
/// built *after* this call (thread-local; **off by default**). On, the balancing
/// seat reuses `defense_to_notrump` instead of falling to the instinct floor's
/// undisciplined balancing doubles. An A/B knob (`bba-match --ns-balancing`).
pub fn set_notrump_balancing(on: bool) {
    NOTRUMP_BALANCING.with(|cell| cell.set(on));
}

fn notrump_balancing_enabled() -> bool {
    NOTRUMP_BALANCING.with(Cell::get)
}

/// Whether the direct-seat DONT defense is the active system
pub(crate) fn direct_dont_enabled() -> bool {
    notrump_defense() == NotrumpDefense::DirectDont
}

thread_local! {
    /// Whether Meckwell's `2♣`/`2♦` (minor + a major) accept a flat 4-4 (else 5-4+);
    /// **off by default** (5-4).  A **probe** knob — the 5-4-vs-4-4 boundary is
    /// measured, not fixed by theory.  No effect unless Meckwell is on.
    static MECKWELL_MINOR_MAJOR_44: Cell<bool> = const { Cell::new(false) };
    /// Whether Meckwell's both-majors `X` accepts a flat 4-4 (else 5-4+); **on by
    /// default** (4-4, the standard weak Meckwell takeout double).  A **probe** knob.
    /// No effect unless Meckwell is on.
    static MECKWELL_X_FOUR_FOUR: Cell<bool> = const { Cell::new(true) };
    /// `points` floor for Meckwell's two-way `X`; **0 by default = inherit the natural
    /// overcall floor (8)**, byte-identical.  Raise it (e.g. 12, the Woolsey `X` floor)
    /// so only strong hands make the broad two-way double and 8-11 both-majors /
    /// single-minor hands pass — fewer sacrificial doubles over a strong 1NT.  A
    /// **probe** knob (the tournament's dominant Meckwell loss is the low-floor `X`).
    static MECKWELL_X_FLOOR: Cell<u8> = const { Cell::new(0) };
}

/// Whether the direct-seat Meckwell defense is the active system
pub(crate) fn meckwell_enabled() -> bool {
    notrump_defense() == NotrumpDefense::Meckwell
}

/// Whether Meckwell's `2♣`/`2♦` accept a flat 4-4 (default `false` = 5-4+).  A
/// **probe** knob.  See [`NotrumpDefense::Meckwell`].
pub fn set_meckwell_minor_major_44(on: bool) {
    MECKWELL_MINOR_MAJOR_44.with(|cell| cell.set(on));
}

fn meckwell_minor_major_44() -> bool {
    MECKWELL_MINOR_MAJOR_44.with(Cell::get)
}

/// Whether Meckwell's both-majors `X` accepts a flat 4-4 (default `true` = 4-4).  A
/// **probe** knob.  See [`NotrumpDefense::Meckwell`].
pub fn set_meckwell_x_four_four(on: bool) {
    MECKWELL_X_FOUR_FOUR.with(|cell| cell.set(on));
}

fn meckwell_x_four_four() -> bool {
    MECKWELL_X_FOUR_FOUR.with(Cell::get)
}

/// Set the `points` floor for Meckwell's two-way `X` (default 0 = inherit the natural
/// overcall floor of 8; set e.g. 12 for a Woolsey-strength double).  A **probe** knob.
/// See [`NotrumpDefense::Meckwell`].
pub fn set_meckwell_x_floor(floor: u8) {
    MECKWELL_X_FLOOR.with(|cell| cell.set(floor));
}

/// The configured Meckwell `X` floor, resolving the 0 sentinel to the natural
/// overcall floor.
fn meckwell_x_floor() -> u8 {
    match MECKWELL_X_FLOOR.with(Cell::get) {
        0 => natural_overcall_points().0,
        floor => floor,
    }
}

thread_local! {
    /// Whether we author a defense to the opponents' 2♣ Stayman
    /// (`(1NT)-P-(2♣)-?`); **off by default** (opt-in A/B).  See
    /// [`set_stayman_defense`].
    static STAYMAN_DEFENSE: Cell<bool> = const { Cell::new(false) };
    /// `(min suit length, points floor)` for the natural `2♦/2♥/2♠` overcalls in
    /// the Stayman defense (the `3♣` jump tracks the same points floor at a fixed
    /// 6-card length).  **Default `(6, 14)`** — the A/B-searched setting (see
    /// [`set_stayman_defense_overcall`]).
    static STAYMAN_DEF_OVERCALL: Cell<(usize, u8)> = const { Cell::new((6, 14)) };
}

/// Author our defense to the opponents' 2♣ Stayman (`(1NT)-P-(2♣)`), for books
/// built *after* this call (thread-local; **off by default**).
///
/// `X` = lead-directing clubs (5+ with values), `2♦/2♥/2♠` = a natural 6-card
/// suit (`points(14..)`), `3♣` = a strong natural club one-suiter; the floor
/// passes everything else (~80%).  No Michaels cue — their 2♣ is artificial, so
/// a cue would be natural.  The overcall length and strength were A/B-searched
/// (see [`set_stayman_defense_overcall`]).
pub fn set_stayman_defense(on: bool) {
    STAYMAN_DEFENSE.with(|cell| cell.set(on));
}

/// Tune the natural `2♦/2♥/2♠` overcall `(min length, points floor)` in the
/// Stayman defense, for books built *after* this call (the `3♣` jump tracks the
/// same points floor).  **Default `(6, 14)`**, the A/B-searched setting: a paired
/// PD sweep (`bba-gen --ns-staydef-overcall LEN:FLOOR`, 1M boards/setting) found
/// length-6 beats length-5 (the 5-card overcalls' plain-DD edge is the
/// light-sacrifice artifact PD prices away) and the points floor is best near 14
/// — below it the overcalls are perfect-defense-negative, at it they turn
/// DD-harmless; tighter still gains only within-noise DD while deleting the sound
/// overcalls that carry the convention's (DD-invisible) competitive value.  No
/// effect unless [`set_stayman_defense`] is on.
pub fn set_stayman_defense_overcall(min_len: usize, points_floor: u8) {
    STAYMAN_DEF_OVERCALL.with(|cell| cell.set((min_len, points_floor)));
}

/// The configured Stayman-defense overcall `(min length, points floor)`
fn stayman_defense_overcall() -> (usize, u8) {
    STAYMAN_DEF_OVERCALL.with(Cell::get)
}

/// Whether the defense to their 2♣ Stayman is currently authored
fn stayman_defense_enabled() -> bool {
    STAYMAN_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Whether we author a defense to the opponents' Jacoby transfers
    /// (`(1NT)-P-(2♦/2♥)-?`); **off by default** (opt-in A/B).  See
    /// [`set_transfer_defense`].
    static TRANSFER_DEFENSE: Cell<bool> = const { Cell::new(false) };
}

/// Author our defense to the opponents' Jacoby transfers (`(1NT)-P-(2♦/2♥)`), for
/// books built *after* this call (thread-local; **off by default**).
///
/// `X` = lead-directing the bid (transfer) suit — not takeout; a cue of the suit
/// they showed = the other major + a minor (Michaels 5-5); natural one-suiter
/// overcalls (six-card, `points(14..)`, the A/B-searched Stayman-defense floor);
/// the floor passes everything else.  Matches BBA's distilled defense (probe
/// modes `xfer-h`/`xfer-s`).  Opt-in: like the Stayman defense its value is
/// mostly lead-directing (invisible to the double-dummy harness), and a paired
/// A/B vs BBA over 640 000 boards confirms a PD wash (+0.006 IMPs/board it fires
/// on, CI straddles 0); the plain-DD loss is the light-sacrifice artifact PD
/// prices away.
pub fn set_transfer_defense(on: bool) {
    TRANSFER_DEFENSE.with(|cell| cell.set(on));
}

/// Whether the defense to their Jacoby transfers is currently authored
fn transfer_defense_enabled() -> bool {
    TRANSFER_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Whether we author a defense to the opponents' two-way 2♠ minor response
    /// (`(1NT)-P-(2♠)-?` — their clubs-or-size-ask); **off by default** (opt-in
    /// A/B).  See [`set_minor_transfer_defense`].
    static MINOR_TRANSFER_DEFENSE: Cell<bool> = const { Cell::new(false) };
}

/// Author our defense to the opponents' two-way 2♠ minor response
/// (`(1NT)-P-(2♠)`), for books built *after* this call (thread-local; **off by
/// default**).
///
/// `X` = lead-directing spades (the bid suit — not takeout); `2NT` = the two lowest
/// unbid suits (diamonds + hearts, 5-5); `3♣` (a cue of their shown-clubs anchor) =
/// the top-and-bottom two-suiter (spades + diamonds, 5-5), weighted above the `X` so
/// the two-suiter shows rather than lead-directs; natural `3♦`/`3♥` one-suiters; the
/// floor passes everything else.  Opt-in like the Stayman/transfer defenses: the
/// value is mostly lead-directing (invisible to the double-dummy harness), so it
/// ships off for A/B measurement.
pub fn set_minor_transfer_defense(on: bool) {
    MINOR_TRANSFER_DEFENSE.with(|cell| cell.set(on));
}

/// Whether the defense to their two-way 2♠ minor response is currently authored
fn minor_transfer_defense_enabled() -> bool {
    MINOR_TRANSFER_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Whether we author a defense to the opponents' 2NT diamond transfer
    /// (`(1NT)-P-(2NT)-?`); **off by default** (opt-in A/B).  See
    /// [`set_diamond_transfer_defense`].
    static DIAMOND_TRANSFER_DEFENSE: Cell<bool> = const { Cell::new(false) };
}

/// Author our defense to the opponents' 2NT diamond transfer (`(1NT)-P-(2NT)`),
/// for books built *after* this call (thread-local; **off by default**).
///
/// `X` = lead-directing diamonds (the shown suit — not takeout); `3♦` (a cue of
/// their diamond anchor) = both majors (5-5, Michaels), weighted **above** the `X`
/// so a genuine two-suiter shows rather than lead-directs; natural `3♣`/`3♥`/`3♠`
/// six-card one-suiters (`points(14..)`); the floor passes everything else.
/// Opt-in like the Stayman/transfer defenses: the value is mostly lead-directing
/// (invisible to the double-dummy harness).  A paired A/B vs BBA over 1 000 000
/// `--filter-1nt` boards (387 fired, 0.04 %) measured a clear **loss** on both
/// scorers (−1.91 IMPs/board it fires on plain, −2.32 PD), the light-sacrifice cost
/// of doubling/cueing into a strong-1NT auction — so it ships off.
pub fn set_diamond_transfer_defense(on: bool) {
    DIAMOND_TRANSFER_DEFENSE.with(|cell| cell.set(on));
}

/// Whether the defense to their 2NT diamond transfer is currently authored
fn diamond_transfer_defense_enabled() -> bool {
    DIAMOND_TRANSFER_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Minimum length to insist on a DONT one-suiter (the `X` for ♣/♦/♥, the
    /// natural `2♠` for spades); **5 by default**.  Set to 6 to bid only with a
    /// six-card suit, passing five-card one-suiters (the X bucket is the DD loser,
    /// so insisting only with real shape trades action for safety — toward the
    /// always-pass optimum).  An A/B knob, no effect unless DONT is on.
    static DIRECT_DONT_ONE_SUITER_MIN: Cell<u8> = const { Cell::new(5) };
    /// Whether DONT two-suiters (`2♣`/`2♦`/`2♥`) accept a flat 4-4 (else 5-4+);
    /// **on by default** — DONT is traditionally a 4-4 method (M6.2d).  Off, only
    /// 5-4+ two-suiters compete (tighter, fewer auctions).  An A/B knob, no effect
    /// unless DONT is on.
    static DIRECT_DONT_FOUR_FOUR: Cell<bool> = const { Cell::new(true) };
    /// `points` floor for the DONT one-suiter `X`; **0 by default = inherit the natural
    /// overcall floor (8)**, byte-identical.  Raise it so only strong one-suiters
    /// double and 8-11 hands pass (the `X` bucket is the DD loser — trade action for
    /// safety, as [`DIRECT_DONT_ONE_SUITER_MIN`] does for length).  A/B knob, no effect
    /// unless DONT is on.
    static DIRECT_DONT_X_FLOOR: Cell<u8> = const { Cell::new(0) };
}

/// Minimum one-suiter length for the DONT `X`/`2♠` (default 5; set 6 to pass
/// five-card one-suiters).  See [`NotrumpDefense::DirectDont`].
pub fn set_direct_dont_one_suiter_min(min: u8) {
    DIRECT_DONT_ONE_SUITER_MIN.with(|cell| cell.set(min));
}

fn direct_dont_one_suiter_min() -> usize {
    DIRECT_DONT_ONE_SUITER_MIN.with(Cell::get) as usize
}

/// Whether DONT two-suiters accept a flat 4-4 (default true = traditional 4-4; false =
/// 5-4+).  See [`NotrumpDefense::DirectDont`].
pub fn set_direct_dont_four_four(on: bool) {
    DIRECT_DONT_FOUR_FOUR.with(|cell| cell.set(on));
}

/// Set the `points` floor for the DONT one-suiter `X` (default 0 = inherit the natural
/// overcall floor of 8; raise it to double only with strong one-suiters).  See
/// [`NotrumpDefense::DirectDont`].
pub fn set_direct_dont_x_floor(floor: u8) {
    DIRECT_DONT_X_FLOOR.with(|cell| cell.set(floor));
}

/// The configured DONT `X` floor, resolving the 0 sentinel to the natural overcall floor.
fn direct_dont_x_floor() -> u8 {
    match DIRECT_DONT_X_FLOOR.with(Cell::get) {
        0 => natural_overcall_points().0,
        floor => floor,
    }
}

fn direct_dont_four_four() -> bool {
    DIRECT_DONT_FOUR_FOUR.with(Cell::get)
}

thread_local! {
    /// The `points` floor for the direct-seat both-majors double; **15 by default**
    /// — the clean partition just above the natural-overcall ceiling (14), so an
    /// intermediate both-majors hand overcalls a major (8–14) and the `X` is reserved
    /// for the strong hands too good to overcall (15+).  Competing less (fewer thin
    /// doubles to be punished) and carrying more defense when we act both helped on the
    /// A/B sweep, which peaked near 15–16; 15 captures it with no orphaned point-count.
    /// The advancer's invite/game thresholds track it.  See [`set_direct_landy_double_floor`].
    static DIRECT_LANDY_DOUBLE_FLOOR: Cell<u8> = const { Cell::new(15) };
    /// Whether the advancer may **pass the both-majors `X` for penalty** (defend
    /// `1NTx`) at `[1NT, X, P]`; **off by default**.  On, a hand with no major fit
    /// (both majors ≤2) and enough defense converts the takeout double to penalties
    /// rather than running to a 5-2 major; the threshold tracks the X floor (a
    /// stronger X needs less from the advancer).  See [`set_direct_landy_penalty_pass`].
    static DIRECT_LANDY_PENALTY_PASS: Cell<bool> = const { Cell::new(false) };
}

/// Replace the direct-seat 15+ penalty double of their 1NT with a both-majors
/// takeout double, for books built *after* this call (thread-local, read once at
/// book-construction time)
///
/// `None` (the **default**) keeps the natural penalty-X defense.  `Some(false)`
/// makes `X` show at least 5-4 in the majors at every seat; `Some(true)` accepts a
/// flat 4-4.  The penalty double is dropped entirely (a 15+ balanced hand passes or
/// overcalls), the four natural two-level suit overcalls are kept, and the advancer
/// answers through the Landy machinery (`landy_advances`).  Mutually exclusive
/// with the natural penalty-X arm and the Landy `2♣` overlay (this covers the
/// passed seat too).  The A/B knob for `examples/ab-landy --ns-landy-x`.
///
/// A back-compat shim over [`set_notrump_defense`]: `Some(four_four)` selects
/// [`NotrumpDefense::DirectLandy`] and stores the shape flag; `None` reverts to
/// [`NotrumpDefense::Natural`] when direct-Landy is the active system (else a no-op).
pub fn set_direct_landy_double(shape: Option<bool>) {
    match shape {
        Some(four_four) => {
            set_notrump_defense(NotrumpDefense::DirectLandy);
            DIRECT_LANDY_FOUR_FOUR.with(|cell| cell.set(four_four));
        }
        None if notrump_defense() == NotrumpDefense::DirectLandy => {
            set_notrump_defense(NotrumpDefense::Natural);
        }
        None => {}
    }
}

/// The configured direct-seat both-majors double shape, or `None` when off
pub(crate) fn direct_landy_double() -> Option<bool> {
    (notrump_defense() == NotrumpDefense::DirectLandy)
        .then(|| DIRECT_LANDY_FOUR_FOUR.with(Cell::get))
}

/// Set the `points` floor for the direct-seat both-majors double (default 8), for
/// books built *after* this call.  A higher floor reserves the `X` for stronger
/// hands (lighter both-majors hands overcall a major naturally) — competing less
/// and penalizing more.  The advancer's invite/game thresholds track it.  No effect
/// unless [`set_direct_landy_double`] is on.  The A/B knob for `examples/ab-landy
/// --ns-landy-x-floor`.
pub fn set_direct_landy_double_floor(floor: u8) {
    DIRECT_LANDY_DOUBLE_FLOOR.with(|cell| cell.set(floor));
}

/// The configured both-majors double `points` floor
fn direct_landy_double_floor() -> u8 {
    DIRECT_LANDY_DOUBLE_FLOOR.with(Cell::get)
}

/// Allow the advancer to pass the both-majors `X` for penalty (defend `1NTx`) when it
/// has no major fit and enough defense, for books built *after* this call (default
/// off).  No effect unless [`set_direct_landy_double`] is on.  The A/B knob for
/// `examples/ab-landy --ns-landy-x-penalty`.
pub fn set_direct_landy_penalty_pass(on: bool) {
    DIRECT_LANDY_PENALTY_PASS.with(|cell| cell.set(on));
}

fn direct_landy_penalty_pass() -> bool {
    DIRECT_LANDY_PENALTY_PASS.with(Cell::get)
}

thread_local! {
    /// Inclusive `points` band for the Woolsey suit overcalls (`2♣`/`2♦`/`2♥`/`2♠`);
    /// **(8, 19) by default** — level with the natural overcall floor.  A 2026-06-26
    /// re-probe (continuations now fully authored) found honest plain-DD self-play
    /// *peaks at 8* and flattens below it (6/7 add no value), and the BBA head-to-head
    /// agrees; perfect-defense (PD) still mildly prefers 10, but PD over-deters by
    /// assuming a perfect doubler.  The conventions only rearrange *which* call shows a
    /// hand — the strength floor tracks natural's 8.  See `docs/` re-probe note.
    static WOOLSEY_POINTS: Cell<(u8, u8)> = const { Cell::new((8, 19)) };
    /// `points` floor for the Woolsey takeout `X` (4-card major + longer minor);
    /// **12 by default** — the X is the most constructive Woolsey action, so it
    /// floors above the preemptive suit overcalls.  See [`set_woolsey_double_floor`].
    static WOOLSEY_DOUBLE_FLOOR: Cell<u8> = const { Cell::new(12) };
}

/// Whether the Woolsey defense is the active system (read by the inference engine
/// to decode our artificial 2♣/2♦/2♥/2♠ overcalls; see `inference::multi_reading`)
pub(crate) fn woolsey_enabled() -> bool {
    notrump_defense() == NotrumpDefense::Woolsey
}

/// Set the inclusive `points` band for the Woolsey suit overcalls (`2♣`/`2♦`/`2♥`/
/// `2♠`, default 8–19) for books built *after* this call.  No effect unless
/// [`NotrumpDefense::Woolsey`] is selected.  The A/B knob for
/// `examples/ab-landy --ns-woolsey-range`.
pub fn set_woolsey_points(lo: u8, hi: u8) {
    WOOLSEY_POINTS.with(|cell| cell.set((lo, hi)));
}

/// The configured Woolsey suit-overcall `points` band (also the points floor the
/// inference engine reads for our 2♣/2♦/2♥/2♠ overcalls)
pub(crate) fn woolsey_points() -> (u8, u8) {
    WOOLSEY_POINTS.with(Cell::get)
}

/// Set the `points` floor for the Woolsey takeout `X` (default 12) for books built
/// *after* this call.  No effect unless [`NotrumpDefense::Woolsey`] is selected.  The A/B knob for
/// `examples/ab-landy --ns-woolsey-x-floor`.
pub fn set_woolsey_double_floor(floor: u8) {
    WOOLSEY_DOUBLE_FLOOR.with(|cell| cell.set(floor));
}

/// The configured Woolsey takeout-`X` `points` floor
pub(crate) fn woolsey_double_floor() -> u8 {
    WOOLSEY_DOUBLE_FLOOR.with(Cell::get)
}

/// Woolsey **Multi** `2♦`: a single 6+ card major (unknown which), nothing else long —
/// both minors at most four.  M6.2d simplified the shape and states it with `or`/`and`
/// so it projects straight off the rule: the strictly-longer-major and no-6-6 guards
/// are dropped, so a 6-5 or 6-6 major hand now qualifies as Multi.
fn woolsey_multi() -> Cons<impl Constraint + Clone> {
    or([Suit::Hearts, Suit::Spades], 6..) & and([Suit::Clubs, Suit::Diamonds], ..=4)
}

/// Woolsey **Muiderberg** `2M`: exactly 5 in `major`, at most 3 in the other major, and
/// a 4+ card minor.  M6.2d states the minor side with `or` so it projects; the shape is
/// otherwise unchanged — `5..=5` keeps it disjoint from the 6+ Multi `2♦`, and the
/// other-major ≤3 cap keeps it disjoint from the 2♣ both-majors (the Woolsey structure
/// relies on disjoint shapes so its uniform 1.9 weights never tie).
fn woolsey_muiderberg(major: Suit) -> Cons<impl Constraint + Clone> {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    len(major, 5..=5) & len(other, ..=3) & or([Suit::Clubs, Suit::Diamonds], 4..)
}

/// Woolsey takeout `X`: exactly 4 in one major, at most 3 in the other, and a longer
/// (5-6) minor (a 7+ minor one-suiter passes — no natural minor overcall).  A 4-card
/// major can co-exist with at most a 5-card minor here, so the `or([♣,♦],5..=6)` needs
/// no upper cap on the second minor — the major half bars a 7+ minor anyway.
fn woolsey_double_shape() -> Cons<impl Constraint + Clone> {
    ((len(Suit::Hearts, 4..=4) & len(Suit::Spades, ..=3))
        | (len(Suit::Spades, 4..=4) & len(Suit::Hearts, ..=3)))
        & or([Suit::Clubs, Suit::Diamonds], 5..=6)
}

/// The advancer's action over partner's both-majors `X` (RHO passing, `[1NT, X, P]`)
///
/// The Landy advance ([`landy_advances`]) plus — when [`set_direct_landy_penalty_pass`]
/// is on — a **penalty pass**: with no major fit (both majors ≤2) and enough defense
/// (`points(22 - lo ..)`, so a stronger `X` asks less), pass and defend `1NTx` rather
/// than run to a 5-2 major.  Weight 1.25 beats the `2NT` game-ask (1.2) and the weak
/// signoffs for exactly these no-fit hands.  After the advancer's pass it is the
/// *opener's* turn, so a following opener pass ends the auction in `1NTx` (declared by
/// them, defended by us) — no doubler node is needed.
fn both_majors_x_advance(lo: u8) -> Rules {
    let base = landy_advances(lo);
    if direct_landy_penalty_pass() {
        let penalty = 22u8.saturating_sub(lo);
        base.rule(
            Call::Pass,
            125,
            len(Suit::Hearts, ..=2) & len(Suit::Spades, ..=2) & points(penalty..),
        )
    } else {
        base
    }
}

/// Both majors: at least 5-4 either way, or a flat 4-4 when `four_four`.  Both majors
/// four-plus, with the longer at least `4` (flat 4-4) or `5` (5-4) — the `and` floors
/// both, the `or` demands the length.
fn both_majors_shape(four_four: bool) -> Cons<impl Constraint + Clone> {
    let longer = if four_four { 4 } else { 5 };
    and([Suit::Hearts, Suit::Spades], 4..) & or([Suit::Hearts, Suit::Spades], longer..)
}

/// Which shapes qualify for the natural penalty double of their 1NT (the 15+ HCP
/// floor is fixed; this only widens the *shape* gate). See [`set_natural_double_shape`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DoubleShape {
    /// 4333/4432/5332 only — the 15+ penalty double restricted to balanced hands
    /// (**the default**).  A flat hand has no escape for the opener to punish, so it
    /// is the shape that actually wants to defend `1NT` doubled; a shapely 15+ hand
    /// would rather declare its own suit, and the opponents run from the double into
    /// a making contract.  Isolated plain-DD self-play prefers this to [`Self::Any`]
    /// by −0.70 IMPs/divergent (−0.92 under perfect-defense doubling, ~17k divergent
    /// boards); the `bba-match --isolate-defense` edge that once favored `Any` is a
    /// within-noise wash (+0.33/divergent over 138 boards, CI straddles 0).
    #[default]
    Balanced,
    /// Balanced plus the semi-balanced single-long-suit hands 5422/6322/7222.
    SemiBalanced,
    /// Any shape — the 15+ HCP floor alone gates the double.  The scheme reads clean
    /// (15+ doubles, 8-14 with a five-card suit overcalls, and a 15+ hand has no
    /// overcall outlet since the range stops at 14), but self-play punishes doubling
    /// a shapely 15+ hand: the opponents escape the penalty double into a making
    /// contract.  See [`Self::Balanced`] for the A/B.
    Any,
}

thread_local! {
    /// Which shapes earn the natural penalty double of their 1NT; **[`Balanced`]
    /// by default** (a flat 15+; shapely hands would rather declare). See
    /// [`set_natural_double_shape`].
    ///
    /// [`Balanced`]: DoubleShape::Balanced
    static NATURAL_DOUBLE_SHAPE: Cell<DoubleShape> = const { Cell::new(DoubleShape::Balanced) };
    /// HCP floor for the natural penalty double of their 1NT; **15 by default**.
    static NATURAL_DOUBLE_FLOOR: Cell<u8> = const { Cell::new(15) };
    /// Logit weight of the natural penalty double; **1.3 by default** (above the
    /// 1.0 suit overcall, so a strong one-suiter doubles). Drop below 1.0 to make
    /// suit overcalls outrank the double — the realistic "strong suit vs X" test.
    static NATURAL_DOUBLE_WEIGHT: Cell<i16> = const { Cell::new(130) };
    /// Inclusive `points` range for the natural two-level suit overcall of their
    /// 1NT; **(8, 14) by default**. Lifting the ceiling lets a strong one-suiter
    /// overcall its suit instead of falling through to the penalty double.
    static NATURAL_OVERCALL_POINTS: Cell<(u8, u8)> = const { Cell::new((8, 14)) };
}

/// Widen (or narrow) the shape gate of the natural penalty double for books built
/// *after* this call (thread-local, read once at book-construction time)
///
/// [`DoubleShape::Balanced`] (the **default**) doubles only 15+ balanced hands.
/// [`DoubleShape::SemiBalanced`] adds 5422/6322/7222, and [`DoubleShape::Any`]
/// doubles every 15+ hand regardless of shape. The HCP floor (15+) is unchanged.
/// An A/B knob (`examples/ab-landy --ns-double-shape balanced|semibal|any`).
pub fn set_natural_double_shape(shape: DoubleShape) {
    NATURAL_DOUBLE_SHAPE.with(|cell| cell.set(shape));
}

/// The shape gate currently authored for the natural penalty double
fn natural_double_shape() -> DoubleShape {
    NATURAL_DOUBLE_SHAPE.with(Cell::get)
}

/// Support gate added to the 12+ takeout double of a suit / weak-two opening.
///
/// The 12+ tier of the takeout double only checks shortness in *their* suit(s),
/// so an off-shape one-suiter short in an unbid suit doubles at 12 and — when its
/// suit ranks below theirs — outranks the 2-level overcall (weight 1.3 > 1.0),
/// gets pulled to the 3-level, and lands doubled.  This gate demands genuine
/// support for the unbid suits on the 12+ tier, forcing off-shape hands down to
/// an overcall or up to the 17+ any-shape tier (matching BBA's two-regime X:
/// 12+ with 3-suit support, else 17+).  See [`set_takeout_support`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum TakeoutSupport {
    /// No support requirement — the 12+ double gates on shortness in their suit
    /// alone (reproduces the historical pre-fix book).
    Off,
    /// Tolerate one doubleton in an unbid suit (admits 4-4-3-2 / 5-3-3-2, rejects
    /// one-suiters short in two unbid suits).
    Lenient,
    /// Demand 3+ cards in every unbid suit (a textbook shapely takeout double —
    /// **the default**, the shipped fix).
    #[default]
    Strict,
}

thread_local! {
    /// Support gate on the 12+ takeout double; **[`TakeoutSupport::Strict`] by
    /// default** (the shipped fix — takeout-support A/B, see the 21gf-ledger).
    /// [`TakeoutSupport::Off`] reproduces the historical book. See
    /// [`set_takeout_support`].
    static TAKEOUT_SUPPORT: Cell<TakeoutSupport> = const { Cell::new(TakeoutSupport::Strict) };
    /// Whether the natural suit overcall of a one-suit opening uses disciplined
    /// bands (1-level `points(8..=17)`, 2-level `points(11..=17)`) instead of the
    /// flat `points(8..=16)`; **true by default** (the shipped fix). See
    /// [`set_overcall_discipline`].
    static OVERCALL_DISCIPLINE: Cell<bool> = const { Cell::new(true) };
    /// Whether a natural direct overcall may be made on a good four-card suit.
    /// Default `false` (byte-identical). See [`set_overcall_four_card`].
    static OVERCALL_FOUR_CARD: Cell<bool> = const { Cell::new(false) };
    /// Whether a **passed hand** may take the disciplined 2-level overcall a shade
    /// lighter (9+ instead of the opening 11+); **true by default** (folded into
    /// base in the A5 pass — a passed hand is captain-limited, so the 11+ floor
    /// all but forbids the safe light overcall; wash-positive on every scorer).
    /// See [`set_passed_hand_overcall`].
    static PASSED_HAND_OVERCALL: Cell<bool> = const { Cell::new(true) };
    /// Whether the 2-level **minor** overcall demands 15+ (a strong single-suiter)
    /// instead of the disciplined 11+; **false by default** (an A/B candidate —
    /// the anchor bleeds on these across every strength/shape/vul band, sd-lead
    /// confirms the loss is real not obstruction). See
    /// [`set_two_level_minor_overcall_tight`].
    static TWO_LEVEL_MINOR_OVERCALL_TIGHT: Cell<bool> = const { Cell::new(false) };
    /// Whether a hand with a five-card **major** is barred from the natural 1NT
    /// overcall (it overcalls the major instead, to find the fit); **false by
    /// default** (an A/B candidate — the anchor shows 5-card majors buried in the
    /// 1NT overcall miss the major game). See [`set_nt_overcall_no_major`].
    static NT_OVERCALL_NO_MAJOR: Cell<bool> = const { Cell::new(false) };
    /// When `Some(n)`, the "too strong to overcall" partition edge is gauged in
    /// raw HCP: the strong-tier double becomes `hcp(n..)` and every natural
    /// overcall band trades its `points` top for `hcp(..n)`.  **`Some(18)` by
    /// default** (fix-vs-shipped, 1M boards/vul + 50k sd/vul, 24.pdd
    /// 12.3M–14.3M + 22.3M: plain DD +0.0105 ± 0.0012 NV / +0.0115 ± 0.0016
    /// vul, PD +0.0114/+0.0126, **sd-lead +0.0159 ± 0.0054 / +0.0115 ±
    /// 0.0072** — every bracket, both vuls, CIs clear).  `None` restores the
    /// legacy `points(17..)` tier / `points(..=17)` tops.  See
    /// [`set_strong_double_hcp`].
    static STRONG_DOUBLE_HCP: Cell<Option<u8>> = const { Cell::new(Some(18)) };
    /// Whether the direct-seat pass over their weak two documents the strong
    /// tier's complement (`points(..17)`) instead of the `hcp(0..)` catch-all.
    /// **Off by default** — REFUTED by A/B (204.8k bd/vul, `SEED_BASE`
    /// 1785083246: plain DD **−0.0028 ± 0.0017** NV / −0.0012 ± 0.0022 vul, PD
    /// +0.0005 ± 0.0022 / +0.0015 ± 0.0026, 0.45%/0.38% fired).  A sounder
    /// reading that bids worse.  See [`set_weak_two_pass_gate`].
    static WEAK_TWO_PASS_GATE: Cell<bool> = const { Cell::new(false) };
    /// Whether the 2NT overcall of their weak two takes the wider
    /// [`notrump_shape`] instead of strict [`balanced`].  **Off by default** —
    /// WASH over two seeds (204.8k bd/arm/vul; seed 1785085719 plain +0.0008 ±
    /// 0.0008 NV / +0.0008 ± 0.0009 vul, PD +0.0010/+0.0011; seed 1785086925
    /// plain +0.0002 ± 0.0008 / −0.0001 ± 0.0008, PD +0.0005/+0.0002).  Seed 1
    /// went 4/4 positive and did not replicate.  See
    /// [`set_weak_two_notrump_shape`].
    static WEAK_TWO_NOTRUMP_SHAPE: Cell<bool> = const { Cell::new(false) };
    /// Whether a jump in a new suit below 3NT is authored over their weak two:
    /// one trick more, so six-plus cards and three more points.  **Off by
    /// default** — LOST, 4/4 (204.8k bd/vul, seed 1785085719: plain −0.0008 ±
    /// 0.0007 NV / −0.0010 ± 0.0008 vul, PD −0.0012/−0.0011).  Overlapping
    /// bands, see [`set_weak_two_jump_overcall`].
    static WEAK_TWO_JUMP_OVERCALL: Cell<bool> = const { Cell::new(false) };
    /// Whether the natural suit overcall of their weak two demands more when
    /// **we** are vulnerable.  **On by default** — `win | win`, 8/8 cells over
    /// two seeds; see [`set_weak_two_overcall_discipline`].
    static WEAK_TWO_OVERCALL_DISCIPLINE: Cell<bool> = const { Cell::new(true) };
    /// Whether advancer's Gladiator structure over our 2NT overcall of their
    /// weak two in a major is authored.  **Off by default** — measured null and
    /// faintly negative; see [`set_weak_two_notrump_advances`].
    static WEAK_TWO_NOTRUMP_ADVANCES: Cell<bool> = const { Cell::new(false) };
    /// Whether the direct cue of their *major* weak two is authored as Michaels
    /// — the other major plus a minor, 5-5.  **Off by default** — the A/B is
    /// VOID (no advancer node; see [`set_weak_two_cue`]).
    static WEAK_TWO_CUE: Cell<bool> = const { Cell::new(false) };
    /// Inclusive `hcp` band of the 2NT overcall of their weak two; **(16, 17) by
    /// default** — 15-counts pass and 18-counts double, two disjoint wins that
    /// compose.  BBA's own bucket is 15–17 (median 16).  See
    /// [`set_weak_two_notrump_points`].
    static WEAK_TWO_NOTRUMP_POINTS: Cell<(u8, u8)> = const { Cell::new((16, 17)) };
    /// Inclusive `points` bands of the natural suit overcall of their weak two,
    /// by the level it lands on: `(two_lo, two_hi, three_lo, three_hi)`.
    /// **(10, 16, 10, 16) by default** — the shipped flat band at both levels.
    /// See [`set_weak_two_overcall_points`].
    static WEAK_TWO_OVERCALL_POINTS: Cell<(u8, u8, u8, u8)> =
        const { Cell::new((10, 16, 10, 16)) };
    /// When `Some(n)`, the Michaels cue-bid and the Unusual 2NT require
    /// `hcp(n..)` on top of the shipped `points(8..)`.  **`Some(8)` by
    /// default** (fix-vs-shipped, 1M boards/vul + 50k sd/vul, 24.pdd
    /// 14.3M–16.3M + 22.4M: plain DD +0.0023 ± 0.0008 NV / +0.0031 ± 0.0010
    /// vul, PD +0.0028/+0.0036, sd-lead +0.0024 ± 0.0035 / +0.0046 ± 0.0043 —
    /// no wall inversion).  See [`set_two_suiter_hcp_floor`].
    static TWO_SUITER_HCP_FLOOR: Cell<Option<u8>> = const { Cell::new(Some(8)) };
    /// Whether the advancer runs **systems-on** after our natural 1NT overcall:
    /// the whole opening-1NT response structure (Stayman, transfers, Smolen)
    /// grafted below `[1t,1NT]`, so a 15–18 balanced overcall finds 4-4 major
    /// fits and right-sides via transfers; **true by default** (measured a
    /// clean single-dummy-lead win over both minor and major openings). See
    /// [`set_nt_overcall_systems_on`].
    static NT_OVERCALL_SYSTEMS_ON: Cell<bool> = const { Cell::new(true) };
    /// Whether the advancer runs **Gladiator** (not systems-on) after our 1NT
    /// overcall of their **major**: a `2♣` weak relay, a cue-of-their-major
    /// Stayman for the *one* unbid major, natural INV bids, and shape actions
    /// (splinter, Leaping Michaels).  Replaces the opening-1NT graft for major
    /// openings only (minors keep systems-on); **false by default** (an A/B
    /// candidate — the major graft washes plain/PD, wins only on sd-lead). See
    /// [`set_nt_overcall_gladiator`].
    static NT_OVERCALL_GLADIATOR: Cell<bool> = const { Cell::new(false) };
}

/// Add a support gate to the 12+ takeout double for books built *after* this call
/// (thread-local, read once at book-construction time)
///
/// [`TakeoutSupport::Strict`] (the **default**, the shipped fix) demands 3+ cards
/// in every unbid suit so off-shape one-suiters overcall (or wait for 17+) instead
/// of doubling and pulling to the 3-level.  [`TakeoutSupport::Off`] reproduces the
/// historical book; [`TakeoutSupport::Lenient`] tolerates one doubleton.  An A/B
/// knob (`bba-gen --ns-takeout-support off|lenient|strict`).
pub fn set_takeout_support(gate: TakeoutSupport) {
    TAKEOUT_SUPPORT.with(|cell| cell.set(gate));
}

/// The support gate currently authored for the 12+ takeout double
fn takeout_support() -> TakeoutSupport {
    TAKEOUT_SUPPORT.with(Cell::get)
}

/// Tighten the natural suit-overcall bands for books built *after* this call
/// (thread-local, read once at book-construction time)
///
/// `true` (the **default**, the shipped fix) raises the 1-level cap to 17 and the
/// 2-level band to `11..=17` (opening values before a below-their-suit 2-level
/// overcall, the standard discipline).  `false` reproduces the flat `points(8..=16)`
/// at both levels.  An A/B knob (`bba-gen --ns-overcall-discipline on|off`).
pub fn set_overcall_discipline(on: bool) {
    OVERCALL_DISCIPLINE.with(|cell| cell.set(on));
}

/// Allow a natural direct overcall on exactly four cards when the suit holds
/// at least five HCP (opt-in; the default `false` is byte-identical).
pub fn set_overcall_four_card(on: bool) {
    OVERCALL_FOUR_CARD.with(|cell| cell.set(on));
}

fn overcall_four_card() -> bool {
    OVERCALL_FOUR_CARD.with(Cell::get)
}

/// Let a passed hand overcall the disciplined 2-level a shade lighter (9+) for
/// books built *after* this call (thread-local, read once at book-construction
/// time)
///
/// `true` (the **default**, folded into base in the A5 pass) relaxes the floor to
/// 9+ for a passed hand only: it cannot hold opening values anyway, so the 11+
/// floor would all but forbid the safe, useful light overcall (partner is a
/// limited captain).  `false` applies the opening-values 11+ floor to every seat.
/// Only affects the disciplined 2-level overcall ([`set_overcall_discipline`] on);
/// the 1-level floor is untouched.  An A/B knob (`bba-gen --no-ns-passed-hand-overcall`
/// to disable).
pub fn set_passed_hand_overcall(on: bool) {
    PASSED_HAND_OVERCALL.with(|cell| cell.set(on));
}

/// Whether a passed hand's lighter 2-level overcall is currently authored
fn passed_hand_overcall() -> bool {
    PASSED_HAND_OVERCALL.with(Cell::get)
}

/// Demand 15+ for the 2-level **minor** overcall (`2♣`/`2♦` below their suit)
/// for books built *after* this call (thread-local, read once at construction)
///
/// `false` (the **default**) keeps the disciplined 11+ 2-level band for minors.
/// `true` raises it to `15..=17`, stranding the losing 11–14 single-suited minor
/// overcalls into Pass (partner reopens). The anchor bleeds on the 2-level minor
/// overcall across every points/shape/vul band and sd-lead confirms the loss is
/// real (not obstruction the blind lead recovers); majors and the 1-level are
/// untouched. An A/B knob (`bba-gen --ns-two-level-minor-overcall-tight`).
pub fn set_two_level_minor_overcall_tight(on: bool) {
    TWO_LEVEL_MINOR_OVERCALL_TIGHT.with(|cell| cell.set(on));
}

/// Whether the 2-level minor overcall demands 15+
fn two_level_minor_overcall_tight() -> bool {
    TWO_LEVEL_MINOR_OVERCALL_TIGHT.with(Cell::get)
}

/// Bar a five-card major from the natural 1NT overcall (overcall the major
/// instead) for books built *after* this call (thread-local, read at construction)
///
/// `false` (the **default**) lets a 15–18 balanced hand with a five-card major
/// overcall 1NT, burying the suit. `true` requires both majors ≤4 for the 1NT
/// overcall, so a five-card major overcalls naturally (`1♥`/`1♠`) and partner can
/// raise the fit — the anchor shows these buried majors miss the major game BBA
/// reaches. An A/B knob (`bba-gen --ns-nt-overcall-no-major`).
pub fn set_nt_overcall_no_major(on: bool) {
    NT_OVERCALL_NO_MAJOR.with(|cell| cell.set(on));
}

/// Whether a five-card major is barred from the 1NT overcall
fn nt_overcall_no_major() -> bool {
    NT_OVERCALL_NO_MAJOR.with(Cell::get)
}

/// Run systems-on (cue-Stayman) advances after our natural 1NT overcall, for
/// books built *after* this call (thread-local, read at construction)
///
/// `true` (the **default**) grafts the full opening-1NT response structure below
/// `[1t,1NT]`, so `1♦–1NT` equals `1♣–1NT` equals an opening 1NT — Stayman,
/// Jacoby/minor transfers, and Smolen, identical over both minors, with the same
/// structure over a major (one Stayman-found major is theirs). Transfers preserve
/// right-siding (the strong overcaller declares). `false` leaves the `[1t,1NT,P]`
/// advance to the instinct floor's naturals. Off flag: `bba-gen
/// --no-ns-nt-overcall-systems-on`.
pub fn set_nt_overcall_systems_on(on: bool) {
    NT_OVERCALL_SYSTEMS_ON.with(|cell| cell.set(on));
}

/// Whether systems-on advances of the 1NT overcall are authored
pub(crate) fn nt_overcall_systems_on() -> bool {
    NT_OVERCALL_SYSTEMS_ON.with(Cell::get)
}

/// Run **Gladiator** advances after our 1NT overcall of their **major**, for
/// books built *after* this call (thread-local, read at construction)
///
/// `false` (the **default**) keeps the systems-on opening-1NT graft over majors.
/// `true` replaces that graft (for major openings only — minors stay systems-on)
/// with Gladiator: `2♣` = weak relay (pass-or-correct to the best part-score),
/// the cue of their major = Stayman for the single unbid major, natural `2♦`/`2M`
/// = 5-card INV, `2NT` = NF INV clubs, plus splinter / Leaping-Michaels shape
/// actions. Independent of [`set_nt_overcall_systems_on`] (it only governs
/// the *major* branch — minors keep systems-on when that is set). Off flag:
/// `bba-gen --ns-nt-overcall-gladiator`.
pub fn set_nt_overcall_gladiator(on: bool) {
    NT_OVERCALL_GLADIATOR.with(|cell| cell.set(on));
}

/// Whether Gladiator advances replace the major-opening systems-on graft
pub(crate) fn nt_overcall_gladiator() -> bool {
    NT_OVERCALL_GLADIATOR.with(Cell::get)
}

/// Whether the disciplined overcall bands are currently authored
fn overcall_discipline() -> bool {
    OVERCALL_DISCIPLINE.with(Cell::get)
}

/// Set the HCP floor of the natural penalty double of their 1NT (default 15) for
/// books built *after* this call. An A/B knob (`bba-match --ns-double-floor`).
pub fn set_natural_double_floor(floor: u8) {
    NATURAL_DOUBLE_FLOOR.with(|cell| cell.set(floor));
}

pub(crate) fn natural_double_floor() -> u8 {
    NATURAL_DOUBLE_FLOOR.with(Cell::get)
}

/// Set the logit weight of the natural penalty double of their 1NT, in
/// centinats (default 130) for books built *after* this call. Below the 100
/// suit-overcall weight, a strong one-suiter overcalls instead of doubling. An
/// A/B knob (`bba-match --ns-double-weight`).
pub fn set_natural_double_weight(weight: i16) {
    NATURAL_DOUBLE_WEIGHT.with(|cell| cell.set(weight));
}

fn natural_double_weight() -> i16 {
    NATURAL_DOUBLE_WEIGHT.with(Cell::get)
}

/// Set the inclusive `points` range of the natural two-level suit overcall of
/// their 1NT (default 8–14) for books built *after* this call. Raising the
/// ceiling routes a strong shapely one-suiter into a suit overcall rather than
/// the penalty double. An A/B knob (`bba-match --ns-overcall LO:HI`).
pub fn set_natural_overcall_points(lo: u8, hi: u8) {
    NATURAL_OVERCALL_POINTS.with(|cell| cell.set((lo, hi)));
}

pub(crate) fn natural_overcall_points() -> (u8, u8) {
    NATURAL_OVERCALL_POINTS.with(Cell::get)
}

/// Gauge the "too strong to overcall" partition edge in raw HCP for books built
/// *after* this call: the strong-tier double of their suit opening becomes
/// `hcp(n..)` and every natural-overcall band trades its `points` top for
/// `hcp(..n)`, so no strength is orphaned between "overcall" and "double first,
/// then bid".
///
/// The strong tier exists to promise partner *defensive tricks* — a high-card
/// statement.  Its legacy `points(17..)` was HCP-flavored under the legacy
/// scale, but rule-of-N+8 reads a 5-4 fourteen-count 17+: the shaped 14–16 HCP
/// hands then double first (or overflow the overcall band top into the tier)
/// and lose to the natural overcall — the point-count remnant's X↔bid seam,
/// both mirror directions CI-clear.  `Some(18)` keeps 17-HCP shaped hands
/// overcalling, the forensic winners.  **Default `Some(18)`** (measured every
/// bracket; see the thread-local above); `None` restores the `points`
/// partition.
pub fn set_strong_double_hcp(n: Option<u8>) {
    STRONG_DOUBLE_HCP.with(|cell| cell.set(n));
}

pub(crate) fn strong_double_hcp() -> Option<u8> {
    STRONG_DOUBLE_HCP.with(Cell::get)
}

/// Gate the direct-seat pass over their weak two on the strong tier's
/// complement for books built *after* this call
///
/// On, `defense_to_weak_two`'s Pass rule reads `points(..17)` — the negative
/// inference of declining the shape-free `points(17..)` takeout double, exactly
/// as `defense_to_suit` already documents its own tier.  Off restores the
/// `hcp(0..)` catch-all, which projects ⊤ on all five axes the nets read.
///
/// Argmax-inert at the node itself (a 17+ hand already scored 1.2 for the
/// double against 0.0 for the pass), but the reading feeds
/// [`push_inference`][crate::bidding::features], so the neural floor sees
/// different inputs downstream.  The ceiling is sound only because that tier is
/// **shape-free**: it accepts every 17+ hand, so no hand that could have passed
/// is excluded.  A shaped tier would leave holes at every strength and no
/// ceiling would be authorable — which is why the analogous `1NT P` (90.7% ⊤ on
/// all five axes in the census) cannot be fixed this way.
///
/// **Default off — REFUTED** (204.8k bd/vul, `SEED_BASE` 1785083246; numbers on
/// the thread-local above).  Plain DD loses NV with a CI clear of zero and PD
/// washes: `loss | wash` never ships default-on.  The mechanism is the C1
/// encoding failure, not the bridge: capping the passer should make us *more*
/// cautious, yet every one of the five worst boards is the ON arm overbidding
/// into a double (6NT-X, 7♦-X, 5♦-X).  `push_inference` hands the net the raw
/// `{min, max}` pair, so `max/37` moves 1.00 → 0.43 on a seat it was trained to
/// see as ⊤ and it answers out of distribution.  Kept opt-in as a single-dummy
/// and post-retrain re-measure candidate: the reading itself is strictly sounder,
/// and an F2b-style evaluator twin selected on this knob would price it fairly.
pub fn set_weak_two_pass_gate(on: bool) {
    WEAK_TWO_PASS_GATE.with(|cell| cell.set(on));
}

fn weak_two_pass_gate() -> bool {
    WEAK_TWO_PASS_GATE.with(Cell::get)
}

/// Widen the 2NT overcall of their weak two from strict [`balanced`] shape to
/// `two_notrump_wide_shape` (2–4 majors, 2–6 minors) for books built *after* this call
///
/// `balanced()` in this crate is exactly 4333/4432/5332, so today a 6322 with a
/// solid six-card minor and their suit stopped has **no** 2NT — it doubles or
/// passes.  BBA's own 2NT bucket is 88–94% balanced with minors running to five,
/// so the rejected tail is real hands.
///
/// **Default off — WASH over two seeds** (numbers on the thread-local above).
/// Seed 1 came back positive in all four cells (+0.77 to +1.63 IMPs/fired) and
/// seed 2 did not replicate it (one cell mildly negative); pooled, every CI
/// still straddles zero.  The `wash | wash` tiebreak is naturalness, and it
/// argues the *other* way here: Cohen, kwbridge and the St Andrews notes all
/// specify **balanced** for this bid, so the narrow rule is the textbook one and
/// this widening is the trial.  Opt-in.
pub fn set_weak_two_notrump_shape(on: bool) {
    WEAK_TWO_NOTRUMP_SHAPE.with(|cell| cell.set(on));
}

fn weak_two_notrump_shape() -> bool {
    WEAK_TWO_NOTRUMP_SHAPE.with(Cell::get)
}

/// Author the jump in a new suit below 3NT over their weak two for books built
/// *after* this call
///
/// One trick higher than the cheapest overcall, so one trick more of hand:
/// **six-plus cards and three more points** than the natural band — natural,
/// non-forcing, strongly invitational.  Only three such calls exist below 3NT
/// (`3♥`/`3♠` over 2♦, `3♠` over 2♥); every other jump is at the four level.
/// BBA authors none of them, so this is an addition rather than a catch-up.
///
/// **Default off — LOST 4/4** (numbers on the thread-local above, −1.05 to
/// −1.61 IMPs/fired).  The trace is the classic case against *strong* jump
/// overcalls: the jump eats the room the strength wanted.
///
/// ```text
/// on:  2♦ 3♥ - 4♦ - 4♥ - - -     off: 2♦ 2♥ - 6♥ - - -
/// on:  2♦ 3♥ - 4♥ - - -          off: 2♦ 2♥ - 3♣ - 3♥ - 5♣ - - -
/// ```
///
/// The authoring makes it worse than it needs to be: `points(13..=19)` at weight
/// 1.1 **overlaps** the natural `points(10..=16)` at weight 1.0, so every 13–16
/// six-carder stops overcalling cheaply and jumps — precisely the hands that
/// wanted advancer to have room.  A retry should make the bands disjoint (jump
/// 17+, or cap the natural at 12 on six-card hands) before concluding anything
/// about jump overcalls as such.
pub fn set_weak_two_jump_overcall(on: bool) {
    WEAK_TWO_JUMP_OVERCALL.with(|cell| cell.set(on));
}

fn weak_two_jump_overcall() -> bool {
    WEAK_TWO_JUMP_OVERCALL.with(Cell::get)
}

/// Author the direct cue of their **major** weak two as Michaels for books built
/// *after* this call
///
/// `3♥` over 2♥ / `3♠` over 2♠ = the other major plus an unspecified minor, 5-5.
/// This is what BBA bids there (`probe-bba-constraints --mode def2-h`: ♠ 5–6,
/// longest minor 5–6, ♥ 0–2, 0% balanced) and what
/// [`set_cue_reading`][crate::bidding::set_cue_reading] already *reads* a direct
/// cue as — so knob-off the book authors a call the reader is waiting for.
///
/// Deliberately **not** extended to `3♦` over their 2♦: BBA never bids it (no
/// `3♦` bucket at all in `--mode def2-d`), the cheap 2♥/2♠ overcalls already
/// carry a major, and 4♦ Leaping Michaels covers the strong both-majors hand.
///
/// **Default off, and its A/B is VOID** — not a verdict on Michaels.  The
/// advancer has no node: `insert_all_seats` wires continuations for the
/// takeout double and Leaping Michaels only, so `[2♠, 3♠, P]` drops to the
/// floor, which *redoubles the cue* — the phantom-suit disaster in the flesh.
///
/// ```text
/// on:  - 2♠ 3♠ X XX - - -                                    (playing 3♠ redoubled — in their suit)
/// on:  2♠ 3♠ X 4♥ 4♠ - - X XX - - 4NT X 5♦ - 5♥ - 6♥ X - - -
/// ```
///
/// Measured −0.78 to −2.63 IMPs/fired, which is the missing continuation
/// talking.  Author advancer's structure (pick the major, relay for the minor,
/// and an SOS/pass-or-correct after their double) before re-measuring.
pub fn set_weak_two_cue(on: bool) {
    WEAK_TWO_CUE.with(|cell| cell.set(on));
}

fn weak_two_cue() -> bool {
    WEAK_TWO_CUE.with(Cell::get)
}

/// Demand more of the natural suit overcall of their weak two when **we** are
/// vulnerable (default **on**) for books built *after* this call
///
/// On, a vulnerable overcall needs 12–17 at the two level and 15–17 at the
/// three; non-vulnerable keeps the flat band
/// ([`set_weak_two_overcall_points`], default 10–16).  Off, the flat band
/// applies at every vulnerability.
///
/// Shipped on a `win | win`, 8/8 cells over two seeds (`SEED_BASE` 1785092622 /
/// 1785093604, 204.8k bd/arm/vul vs BBA 2/1), pooled:
///
/// | `-v` | fired | plain DD | PD |
/// | --- | --- | --- | --- |
/// | none | 0.00% | **0.0000 ± 0.0000** | 0.0000 ± 0.0000 |
/// | ns | 0.62% | **+0.0026 ± 0.0018** | +0.0136 ± 0.0022 |
/// | both | 0.67% | **+0.0029 ± 0.0020** | +0.0182 ± 0.0024 |
///
/// The `none` row is a free null control rather than a result: with nobody
/// vulnerable the rule reduces to the same `points(lo..=hi)` it replaced, so it
/// *must* read exactly zero on zero divergences, and a non-zero there would
/// have meant the vulnerability conjunct was miswired and the other two rows
/// meaningless.
///
/// The vulnerability conjunct is not a guess — it is what separated the earlier
/// exploratory measurement.  Run flat at 12:17:15:17 against the shipped band,
/// two seeds, 204.8k bd/arm/vul (`SEED_BASE` 1785088050 / 1785088953):
///
/// | `-v` | we vulnerable? | plain DD | PD |
/// | --- | --- | --- | --- |
/// | none | no | −0.0024 / −0.0029 | +0.0136 / +0.0132 |
/// | ns | **yes** | **+0.0048 / +0.0026** | +0.0165 / +0.0137 |
/// | both | **yes** | **+0.0063 / +0.0032** | +0.0223 / +0.0172 |
///
/// `none` and `both` are symmetric vulnerabilities and cannot tell our risk
/// from theirs; `ns` (we vulnerable, they not) is the cell that can, and plain
/// DD splits monotonically on **our** vulnerability with nothing left over —
/// so `vulnerable()` is the predicate and `they_vulnerable()` is refuted.
///
/// Note the PD column wins everywhere, including the cell plain DD loses.  That
/// is PD doing what PD does to a light overcall the field would never double,
/// and on its own it is the doubling artifact; the plain-DD half is the one
/// that flips, and it flips the way bridge says it should.
/// A/B knob (`bba-gen --ns-weak-two-overcall-discipline`).
pub fn set_weak_two_overcall_discipline(on: bool) {
    WEAK_TWO_OVERCALL_DISCIPLINE.with(|cell| cell.set(on));
}

fn weak_two_overcall_discipline() -> bool {
    WEAK_TWO_OVERCALL_DISCIPLINE.with(Cell::get)
}

/// Author advancer's Gladiator structure over our 2NT overcall of their weak
/// two in a **major** (default **off**) for books built *after* this call
///
/// Before this, the 2NT overcall had **no continuations at all** — the book
/// authors advances of the takeout double and of Leaping Michaels, but nothing
/// at `[2M, 2NT, P, ?]`, so advancer dropped to the instinct floor.  That is
/// the same structural hole that voided the `set_weak_two_cue` measurement,
/// except this call is a shipped default rather than an opt-in.
///
/// The scheme is Gladiator lifted one level, minus its invitational tier — at
/// 16–17 opposite there is no room to invite, so it is `3♣` or game:
///
/// ```text
/// 2♥ 2NT P  3♣    relay: weak, 5+ ♦, wants a 3-level partscore
///        P  3♦    game-forcing, 5+ ♦
///        P  3♥    cue = Stayman: exactly 4 ♠, game values, not flat
///        P  3♠    game-forcing, 5+ ♠
///        P 3NT    balanced game, to play
///
/// 2♥ 2NT P  3♣ P 3♦    forced, pass-or-correct, says nothing about diamonds
///                 P 3♥ cue = 6+ ♦, long enough that 4♦ is safe
///                 P  P play 3♦
/// ```
///
/// Two deliberate gaps, both `for now`.  Advancer's `3♠` and above in the relay
/// auction are unauthored, which means a *weak* hand with the other major has
/// no landing spot and passes 2NT — its correction would be exactly that `3♠`.
/// And over their `2♠` the delayed cue *is* `3♠`, so that whole rebid node is
/// omitted rather than half-authored.
///
/// A/B knob (`bba-gen --ns-weak-two-nt-advances`).
pub fn set_weak_two_notrump_advances(on: bool) {
    WEAK_TWO_NOTRUMP_ADVANCES.with(|cell| cell.set(on));
}

fn weak_two_notrump_advances_enabled() -> bool {
    WEAK_TWO_NOTRUMP_ADVANCES.with(Cell::get)
}

/// Set the inclusive `hcp` band of the 2NT overcall of their weak two (default
/// **16–17**) for books built *after* this call
///
/// The literature splits — Cohen and the Bridge Bulletin say 15–18, kwbridge
/// 14–18, the St Andrews notes 16–18 — and BBA's own direct-seat bucket is
/// **15–17, median 16** (`probe-bba-constraints --mode def2-h`).  Measurement
/// says both edges of the old 15–18 were wrong, and *independently* so.  The
/// two one-point trims act on disjoint hand classes, so each diverges from
/// 15–18 only at its own end — and a 15-count is some three times as common as
/// an 18-count, which is why trimming the floor moves twice the mass:
///
/// | band | trims | fired | plain NV/vul | PD NV/vul |
/// | --- | --- | --- | --- | --- |
/// | 15–17 | 18s → double | 0.06% | +0.0009 / +0.0004 | +0.0014 / +0.0007 |
/// | 16–18 | 15s → pass | 0.09% | +0.0006 / +0.0007 | +0.0024 / +0.0018 |
/// | **16–17** | both | 0.16% | **+0.0015 / +0.0011** | **+0.0037 / +0.0025** |
///
/// (IMPs/board, mean of seeds 1785088050 and 1785088953, 204.8k bd/arm/vul vs
/// BBA 2/1; pooled CI ±0.0008 plain, ±0.0009 PD.)  The 16–17 row is the sum of
/// the two above it to within noise on every cell, which is the tell that they
/// compose rather than compete.
///
/// The hands land where the system already wants them: an 18-count meets the
/// takeout double's `points(17..)`, and *that* is the classic double-then-
/// notrump auction.  A balanced 15 with a stopper has no home and passes —
/// facing a preempt with a partner who has not spoken, 2NT was buying a bad
/// 3NT.  A/B knob (`bba-gen --ns-weak-two-nt-points LO:HI`).
pub fn set_weak_two_notrump_points(lo: u8, hi: u8) {
    WEAK_TWO_NOTRUMP_POINTS.with(|cell| cell.set((lo, hi)));
}

fn weak_two_notrump_points() -> (u8, u8) {
    WEAK_TWO_NOTRUMP_POINTS.with(Cell::get)
}

/// Set the inclusive `points` bands of the natural suit overcall of their weak
/// two, separately for the calls that land at the two and three level (default
/// 10–16 at both — the shipped flat band) for books built *after* this call
///
/// A weak two leaves an overcall at either level depending on rank: over 2♥ a
/// spade overcall is `2♠` but a club overcall is `3♣`, and the flat band charges
/// both the same.  The one-opening defense already grades by level
/// ([`set_overcall_discipline`]: 1-level 8–17, 2-level 11–17), and the extra
/// trick has to be paid for somewhere.  BBA grades only slightly (10–16 at the
/// two level, 11–16 at the three).
/// A/B knob (`bba-gen --ns-weak-two-overcall LO2:HI2:LO3:HI3`).
pub fn set_weak_two_overcall_points(two_lo: u8, two_hi: u8, three_lo: u8, three_hi: u8) {
    WEAK_TWO_OVERCALL_POINTS.with(|cell| cell.set((two_lo, two_hi, three_lo, three_hi)));
}

fn weak_two_overcall_points() -> (u8, u8, u8, u8) {
    WEAK_TWO_OVERCALL_POINTS.with(Cell::get)
}

/// Require `hcp(n..)` on the Michaels cue-bid and the Unusual 2NT for books
/// built *after* this call, on top of the shipped `points(8..)`.
///
/// Both rules are documented "8+ HCP" but were gauged in `points`; rule-of-N+8
/// reads a 5-HCP 6-5 freak 8–9, and those garbage two-suiters cue at weight
/// 2.0 straight into −800 penalty doubles (the point-count remnant's Michaels
/// family, −17..−21 IMPs a board).  `Some(8)` restores the documented floor.
/// **Default `Some(8)`** (measured; see the thread-local above); `None`
/// restores the bare `points(8..)` gate.
pub fn set_two_suiter_hcp_floor(n: Option<u8>) {
    TWO_SUITER_HCP_FLOOR.with(|cell| cell.set(n));
}

pub(crate) fn two_suiter_hcp_floor() -> Option<u8> {
    TWO_SUITER_HCP_FLOOR.with(Cell::get)
}

/// Semi-balanced shape for the penalty double: balanced, or one of 5422/6322/7222
///
/// Authored as the exact 5-box union `{2..=5}⁴ ∪ four {suit 6..=7, rest
/// 2..=3}`: the cube is exactly "no singleton, no six-card suit" (balanced ∪
/// 5422, the 13-card sum excludes 5-5 and worse), and the four pan-handles
/// are the 6322/7222 patterns per long suit.  Eval-equivalence with the
/// closure this replaces is pinned exhaustively by
/// `semi_balanced_boxes_match_closure`.
fn semi_balanced() -> Cons<impl Constraint + Clone> {
    let mut boxes = vec![length_box([Range::new(2, 5); 4])];
    boxes.extend(Suit::ASC.map(|suit| long_suit_box(suit, Range::new(6, 7), Range::new(2, 3))));
    shapes("balanced or 5422/6322/7222", boxes)
}

thread_local! {
    /// Whether the responsive double after partner's **takeout double** + their
    /// raise (`[1t, X, raise]`) is authored; see [`set_responsive_takeout`].
    static RESPONSIVE_TAKEOUT: Cell<bool> = const { Cell::new(true) };
    /// Whether the responsive double after partner's **overcall** + their raise
    /// (`[1t, overcall, raise]`) is authored; see [`set_responsive_overcall`].
    static RESPONSIVE_OVERCALL: Cell<bool> = const { Cell::new(false) };
    /// Whether the *rich* advance of partner's takeout double of a one-opening
    /// (`[1t, X, P]`) is authored — the cue + notrump ladder that gives the
    /// advancer an invite/force channel; see [`set_rich_advance_double`].
    static RICH_ADVANCE_DOUBLE: Cell<bool> = const { Cell::new(true) };
    /// Whether the **jump-cue Rubens transfer** layer sits on top of the rich
    /// advance — a jump-cue transfer to a 5+ unbid major; see
    /// [`set_advance_rubens`].  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_RUBENS: Cell<bool> = const { Cell::new(false) };
    /// Whether the advance of partner's takeout double bids the **longest** suit
    /// (weight climbing with length) rather than the highest-ranking 4+ suit;
    /// see [`set_longest_first_advance`].
    static LONGEST_FIRST_ADVANCE: Cell<bool> = const { Cell::new(true) };
    /// Whether the advancer's three-level jump in a **minor** shows an
    /// invitational one-suiter (5+, 10–12, denying a 4-card unbid major); see
    /// [`set_advance_minor_jump`].  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_MINOR_JUMP: Cell<bool> = const { Cell::new(true) };
    /// Whether the advancer's **weak** penalty pass yields to a 4+ unbid major
    /// (below the cue band the hand bids the ladder instead of sitting); see
    /// [`set_advance_pass_yield_major`].
    static ADVANCE_PASS_YIELD_MAJOR: Cell<bool> = const { Cell::new(false) };
    /// The advancer's 4-card penalty-pass quality gate as a `suit_hcp` floor —
    /// `None` keeps the shipped `top_honors(t, 2..)`; see
    /// [`set_advance_sit_hcp_gate`].
    static ADVANCE_SIT_HCP_GATE: Cell<Option<u8>> = const { Cell::new(None) };
    /// Whether the doubler answers the advancer's invitational `2NT` with an
    /// authored accept/decline instead of falling to the instinct floor (which
    /// passes even game-going hands); see [`set_advance_2nt_continuation`].  **On
    /// by default** — a wash-positive A/B fix to a strict floor-pass in the
    /// default-on rich advance.  No effect unless [`RICH_ADVANCE_DOUBLE`] is on.
    static ADVANCE_2NT_CONTINUATION: Cell<bool> = const { Cell::new(true) };
}

/// Toggle the responsive double after partner's **takeout double** and their
/// raise (`(1t)–X–(2t)–?`) for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// **On by default** (the shipped behavior): advancer's double of the raise shows
/// the two unbid suits with 8+. Turn it off to drop the node to the instinct
/// floor — the A/B knob for `examples/responsive-ab --conv takeout`. This is the
/// canonical "responsive double" (BBA's single `Responsive double` toggle, on in
/// `21GF.bbsa`); see `docs/ai-bidder/21gf-ledger.md`.
pub fn set_responsive_takeout(on: bool) {
    RESPONSIVE_TAKEOUT.with(|cell| cell.set(on));
}

/// Whether the takeout-double responsive double is currently authored
pub(crate) fn responsive_takeout_enabled() -> bool {
    RESPONSIVE_TAKEOUT.with(Cell::get)
}

/// Toggle the responsive double after partner's **overcall** and their raise
/// (`(1t)–overcall–(2t)–?`) for books built *after* this call (thread-local, read
/// once at book-construction time)
///
/// **Off by default** (the auction falls to the instinct floor). When on, advancer's
/// double of the raise shows the two suits unbid by opener and partner with 8+ — a
/// non-standard extension of our own (BBA's `Responsive double` is only the takeout
/// version; the nearest overcall toggle, `Snapdragon Double`, is off in `21GF.bbsa`
/// and over a *new suit*, not a raise). The A/B knob for
/// `examples/responsive-ab --conv overcall`; see `docs/ai-bidder/21gf-ledger.md`.
pub fn set_responsive_overcall(on: bool) {
    RESPONSIVE_OVERCALL.with(|cell| cell.set(on));
}

/// Toggle the **rich advance** of partner's takeout double of a one-of-a-suit
/// opening (`(1t)–X–(P)–?`) for books built *after* this call (thread-local,
/// read once at book-construction time)
///
/// **On by default** (the shipped behavior); pass `false` (`bba-gen
/// --no-ns-rich-advance`) to drop back to the flat [`advance_double`] ladder.
/// The advancer gets a rich ladder: a cue of opener's suit asking for a 4-card
/// major (invitational 10–11 — the Stayman-ask; game hands blast 4M), a notrump
/// ladder (`1NT` 7–10 / `2NT` 11–12 / `3NT` 13+), weak shapely game jumps, and a
/// forced 3-card-suit response when broke — filling the invite/force gap the
/// flat floor leaves. Measured a constructive win vs the flat book (see
/// `docs/ai-bidder/21gf-ledger.md`).
pub fn set_rich_advance_double(on: bool) {
    RICH_ADVANCE_DOUBLE.with(|cell| cell.set(on));
}

/// Whether the rich advance of a takeout double is currently authored
fn rich_advance_double_enabled() -> bool {
    RICH_ADVANCE_DOUBLE.with(Cell::get)
}

/// Toggle the **jump-cue Rubens transfer** layer on top of the rich advance for
/// books built *after* this call (thread-local, read at book-construction time)
///
/// **Off by default**, and a no-op unless [`set_rich_advance_double`] is also on.
/// When on, the advancer's jump-cue (and, over `(1♠)`, a natural `3♥`) becomes a
/// **transfer to a 5+ unbid major** (invitational-or-better) — the doubler
/// completes and *declares*, right-siding the strong hand.  Right-siding is
/// invisible to double-dummy (the trick count is the same whoever declares), so
/// its value shows up under the single-dummy lead scorer, not the DD A/B; this
/// knob (`bba-gen --ns-advance-rubens`) exists to confirm no DD *regression* and
/// as an sd-lead re-measure candidate.  See `docs/ai-bidder/21gf-ledger.md`.
pub fn set_advance_rubens(on: bool) {
    ADVANCE_RUBENS.with(|cell| cell.set(on));
}

/// Whether the jump-cue Rubens transfer layer is currently authored
fn advance_rubens_enabled() -> bool {
    ADVANCE_RUBENS.with(Cell::get)
}

/// Toggle the **longest-first** suit discipline for the flat advance of partner's
/// takeout double of a one-of-a-suit opening (`(1t)–X–(P)–?`) for books built
/// *after* this call (thread-local, read at book-construction time)
///
/// **On by default** (the shipped behavior); pass `false` (`bba-gen
/// --no-ns-longest-advance`) to score every eligible 4+ suit alike, whereupon
/// the argmax tie-break bids the **highest-ranking** one regardless of length —
/// holding five clubs and four spades it advances `1♠`, not `2♣`. On, the
/// natural-advance weight climbs with suit length, so the advancer bids the
/// **longest** suit and breaks equal-length ties toward the higher-ranking suit
/// (a major over a minor, spades over hearts) — standard takeout-double
/// advancing.
pub fn set_longest_first_advance(on: bool) {
    LONGEST_FIRST_ADVANCE.with(|cell| cell.set(on));
}

/// Whether the longest-first advance discipline is currently authored
fn longest_first_advance_enabled() -> bool {
    LONGEST_FIRST_ADVANCE.with(Cell::get)
}

/// Toggle the weak advancer's **pass-yield to a 4-card major** over partner's
/// takeout double (`(1t)–X–(P)–?`) for books built *after* this call
/// (thread-local, read at book-construction time)
///
/// **Off by default.**  On (`bba-gen --ns-advance-pass-yield`), the penalty
/// pass's trump-stack legs yield when the hand is *below the cue band*
/// (`hcp ≤ 9`) **and** holds a 4+ unbid major: instead of converting the
/// double to penalty, the hand advances on the normal longest-first ladder
/// (which may still land in a longer minor).  Strong sits (10+) stand
/// regardless — restricting *them* is the refuted cap migration
/// (`ab-results/advance-penalty-pass/`, −2 IMPs/fired on both scorers).  The
/// A/B knob for `scripts/ab-advance-pass-yield.sh`.
pub fn set_advance_pass_yield_major(on: bool) {
    ADVANCE_PASS_YIELD_MAJOR.with(|cell| cell.set(on));
}

/// Whether the weak penalty pass yields to a 4-card unbid major
fn advance_pass_yield_major_enabled() -> bool {
    ADVANCE_PASS_YIELD_MAJOR.with(Cell::get)
}

/// Swap the advancer's **4-card penalty-pass quality gate** over partner's
/// takeout double (`(1t)–X–(P)–?`) to a per-suit HCP floor for books built
/// *after* this call (thread-local, read at book-construction time)
///
/// **`None` by default** (the shipped behavior): a 4-card trump stack sits
/// with two of the top three honors.  `Some(n)` (`bba-gen
/// --ns-advance-sit-hcp N`) replaces that gate with `suit_hcp(t, n..)` in the
/// **rich** advance only — the flat book, which is also the weak-two advance
/// node, keeps the honor gate.  The candidate gates nest,
/// {6+} ⊂ {top2} ⊂ {5+}:
/// - `Some(5)` admits exactly one new class, **AJxx** — KQ = 5 is the
///   cheapest two of A/K/Q, so nothing is removed (the same subset relation
///   probed for BBA's Ogust "good suit"; see [`suit_hcp`]);
/// - `Some(6)` instead drops exactly **bare KQxx** (no jack ⇒ 5) while
///   keeping KQJx/AKxx/AQxx; AJxx stays out (5 is the most a single top
///   honor can carry).
///
/// Composes with [`set_advance_pass_yield_major`]: the yield wraps whichever
/// sit gate is live (both default-off, so the default system is untouched).
/// The sweep knob for `scripts/ab-advance-sit-hcp.sh`.
pub fn set_advance_sit_hcp_gate(gate: Option<u8>) {
    ADVANCE_SIT_HCP_GATE.with(|cell| cell.set(gate));
}

/// The advancer's 4-card sit quality gate override, if any
fn advance_sit_hcp_gate() -> Option<u8> {
    ADVANCE_SIT_HCP_GATE.with(Cell::get)
}

/// Toggle the advancer's **invitational minor jump** on the rich advance of a
/// takeout double for books built *after* this call (thread-local, read at
/// book-construction time)
///
/// **On by default**, and a no-op unless [`set_rich_advance_double`] is on. When
/// on, a three-level jump in a *minor* (`(1♥)–X–(P)–3♣`, `(1♠)–X–(P)–3♦`, …)
/// shows an invitational one-suiter — a real 5-card suit, 10–12, **denying a
/// 4-card unbid major** (with one the advancer cues opener's suit to find the
/// 4-4 major fit).  It ranks *below* the notrump ladder, so a stopper still
/// prefers `1NT`/`2NT`/`3NT`; the jump is the residual for the no-stopper shapely
/// invite that would otherwise have to cue.  Game-forcing minors (13+) are capped
/// out and still cue or bid a stopped `3NT`.  The doubler, strong but stopperless,
/// re-asks for a stopper by cueing their suit (a Western cue); the advancer bids
/// the right-sided `3NT` with a stopper, else the minor game.  Two-seed A/B: SIG+
/// in all four cells (plain ≥ PD → constructive).  Turn off with
/// `bba-gen --no-ns-advance-minor-jump`.
pub fn set_advance_minor_jump(on: bool) {
    ADVANCE_MINOR_JUMP.with(|cell| cell.set(on));
}

/// Whether the invitational minor jump is currently authored
fn advance_minor_jump_enabled() -> bool {
    ADVANCE_MINOR_JUMP.with(Cell::get)
}

/// Toggle the doubler's **accept/decline of the advancer's invitational `2NT`**
/// on the rich advance of a takeout double for books built *after* this call
/// (thread-local, read at book-construction time)
///
/// **On by default**, and a no-op unless [`set_rich_advance_double`] is on. The
/// advancer's `2NT` (`(1t)–X–(P)–2NT`) is a limited balanced 11–12 invite with a
/// stopper, but with no authored continuation the doubler falls to the instinct
/// floor, which treats `2NT` as non-forcing and *passes it even holding a game*.
/// When on, the doubler answers the invite naturally: **Pass** declines with a
/// minimum, **`3NT`** accepts to play, and a **new 5-card major** accepts
/// game-forcing so the advancer can pick the 4-4/5-3 major game.  Fixing this
/// floor-pass measured wash-positive on all four cells (NV/vul × plain/PD),
/// which earns the default-on flip.  Off-switch `bba-gen
/// --no-ns-advance-2nt-continuation`.
pub fn set_advance_2nt_continuation(on: bool) {
    ADVANCE_2NT_CONTINUATION.with(|cell| cell.set(on));
}

/// Whether the doubler's answer to the advancer's `2NT` invite is authored
fn advance_2nt_continuation_enabled() -> bool {
    ADVANCE_2NT_CONTINUATION.with(Cell::get)
}

/// Whether the overcall responsive double is currently authored
fn responsive_overcall_enabled() -> bool {
    RESPONSIVE_OVERCALL.with(Cell::get)
}

// ---------------------------------------------------------------------------
// Direct overcalls and doubles
// ---------------------------------------------------------------------------

/// Our action over their one-of-a-suit opening
///
/// One decision: a natural overcall (five-card suit), a takeout double, a
/// 15–18 1NT overcall, or pass.  Strong hands (17+) double first regardless
/// of shape, planning to bid again — otherwise an opening-strength hand with
/// length in the opponents' suit would be stuck.
///
/// Two-suited overcalls are also available:
/// - **Michaels cue-bid** (2 of their suit, 8+ HCP, 5-5): over a minor,
///   both majors; over a major, the other major and an unspecified minor.
/// - **Unusual 2NT** (8+ HCP, 5-5 in the two lowest unbid suits): over 1♣
///   shows diamonds and hearts; over 1♦ shows clubs and hearts; over a major
///   shows both minors.
///
/// # Panics
///
/// Panics if `their_opening` is a notrump bid; pass a suit opening.
#[must_use]
pub fn defense_to_suit(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");

    let one_nt = Bid::new(1, Strain::Notrump);
    let nt_base = hcp(15..=18) & balanced() & stopper_in_their_suits();
    let mut rules = if nt_overcall_no_major() {
        Rules::new().rule(
            one_nt,
            150,
            nt_base & len(Suit::Hearts, ..=4) & len(Suit::Spades, ..=4),
        )
    } else {
        Rules::new().rule(one_nt, 150, nt_base)
    };

    // 12+ takeout double, optionally gated on support for the unbid suits so an
    // off-shape one-suiter overcalls (or waits for the 17+ tier) instead of
    // doubling and pulling to the 3-level.  See [`set_takeout_support`].
    rules = match takeout_support() {
        TakeoutSupport::Off => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & takeout_double_shape_ok(),
        ),
        TakeoutSupport::Lenient => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & unbid_support(1) & takeout_double_shape_ok(),
        ),
        TakeoutSupport::Strict => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & unbid_support(0) & takeout_double_shape_ok(),
        ),
    }
    .alert(TAKEOUT_DOUBLE);

    // The strong tier is a defensive-trick promise; when the partition is
    // HCP-gauged (`set_strong_double_hcp`) it reads `hcp(n..)` and the natural
    // overcall bands below trade their `points` top for `hcp(..n)`.  The pass
    // gate documents the tier's complement — "strong hands double first
    // regardless", so no hand above the tier's floor ever passes here.
    // Byte-identical to the old `hcp(0..)` catch-all: below the floor the gate
    // scores the same, above it the shape-free tier is finite at weight 1.2
    // and always outscores a weight-0 pass.  Authored so the pass reading
    // (`set_pass_reading`) can project the band a passed hand sits within.
    rules = match strong_double_hcp() {
        Some(n) => rules
            .rule(Call::Double, 120, hcp(n..))
            .alert(TAKEOUT_DOUBLE)
            .rule(Call::Pass, 0, hcp(..n)),
        None => rules
            .rule(Call::Double, 120, points(17..))
            .alert(TAKEOUT_DOUBLE)
            .rule(Call::Pass, 0, points(..17)),
    };

    // Natural overcalls: five-card suit.  Disciplined bands by default — 1-level
    // 8–17, 2-level 11–17 (opening values before a below-their-suit 2-level
    // overcall); `set_overcall_discipline(false)` reverts to the flat 8–16.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let level = if strain > theirs { 1 } else { 2 };
            let weight = if level == 1 { 140 } else { 100 };
            // A passed hand may take the disciplined 2-level overcall lighter (9+):
            // it cannot hold opening values, so the 11+ floor would all but forbid
            // the safe light overcall.  Off by default; see `set_passed_hand_overcall`.
            let tight_minor = level == 2
                && matches!(suit, Suit::Clubs | Suit::Diamonds)
                && two_level_minor_overcall_tight();
            let relax_passed =
                overcall_discipline() && level == 2 && passed_hand_overcall() && !tight_minor;
            let lo = if !overcall_discipline() || level == 1 {
                8
            } else if tight_minor {
                15
            } else if relax_passed {
                9
            } else {
                11
            };
            let hi = if overcall_discipline() { 17 } else { 16 };
            // The band top is the other face of the strong-tier floor: when the
            // partition is HCP-gauged it moves with it, so overflow lands in the
            // tier instead of a shape-blind double on a five-card suit.
            rules = match (strong_double_hcp(), relax_passed) {
                (Some(n), false) => rules.rule(
                    Bid::new(level, strain),
                    weight,
                    len(suit, 5..) & points(lo..) & hcp(..n),
                ),
                (Some(n), true) => rules.rule(
                    Bid::new(level, strain),
                    weight,
                    len(suit, 5..) & points(lo..) & hcp(..n) & (points(11..) | passed_hand()),
                ),
                (None, false) => rules.rule(
                    Bid::new(level, strain),
                    weight,
                    len(suit, 5..) & points(lo..=hi),
                ),
                (None, true) => rules.rule(
                    Bid::new(level, strain),
                    weight,
                    len(suit, 5..) & points(lo..=hi) & (points(11..) | passed_hand()),
                ),
            };
            if overcall_four_card() {
                rules = match (strong_double_hcp(), relax_passed) {
                    (Some(n), false) => rules.rule(
                        Bid::new(level, strain),
                        weight,
                        len(suit, 4..=4) & suit_hcp(suit, 5..) & points(lo..) & hcp(..n),
                    ),
                    (Some(n), true) => rules.rule(
                        Bid::new(level, strain),
                        weight,
                        len(suit, 4..=4)
                            & suit_hcp(suit, 5..)
                            & points(lo..)
                            & hcp(..n)
                            & (points(11..) | passed_hand()),
                    ),
                    (None, false) => rules.rule(
                        Bid::new(level, strain),
                        weight,
                        len(suit, 4..=4) & suit_hcp(suit, 5..) & points(lo..=hi),
                    ),
                    (None, true) => rules.rule(
                        Bid::new(level, strain),
                        weight,
                        len(suit, 4..=4)
                            & suit_hcp(suit, 5..)
                            & points(lo..=hi)
                            & (points(11..) | passed_hand()),
                    ),
                };
            }
        }
    }

    // Michaels cue-bid: 2 of their suit, 5-5, 8+ HCP — the HCP is real when
    // `set_two_suiter_hcp_floor` is armed; the shipped gate is `points(8..)`
    // alone, which rule-of-N+8 satisfies on 5-HCP 6-5 freaks.
    let (high, low) = match t {
        // t minor → both majors; t major → the other major (paired with a minor
        // below via `low`).
        Suit::Clubs | Suit::Diamonds => (Suit::Spades, Some(Suit::Hearts)),
        Suit::Hearts => (Suit::Spades, None),
        Suit::Spades => (Suit::Hearts, None),
    };
    rules = match (two_suiter_hcp_floor(), low) {
        (Some(f), Some(l)) => rules.rule(
            Bid::new(2, theirs),
            200,
            len(high, 5..) & len(l, 5..) & points(8..) & hcp(f..),
        ),
        (None, Some(l)) => rules.rule(
            Bid::new(2, theirs),
            200,
            len(high, 5..) & len(l, 5..) & points(8..),
        ),
        (Some(f), None) => rules.rule(
            Bid::new(2, theirs),
            200,
            len(high, 5..)
                & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                & points(8..)
                & hcp(f..),
        ),
        (None, None) => rules.rule(
            Bid::new(2, theirs),
            200,
            len(high, 5..) & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)) & points(8..),
        ),
    }
    .alert(MICHAELS);

    // Unusual 2NT: 5-5 in the two lowest unbid suits, 8+ HCP (same optional
    // floor as Michaels).
    let (a, b) = match t {
        Suit::Clubs => (Suit::Diamonds, Suit::Hearts),
        Suit::Diamonds => (Suit::Clubs, Suit::Hearts),
        Suit::Hearts | Suit::Spades => (Suit::Clubs, Suit::Diamonds),
    };
    match two_suiter_hcp_floor() {
        Some(f) => rules
            .rule(
                Bid::new(2, Strain::Notrump),
                190,
                len(a, 5..) & len(b, 5..) & points(8..) & hcp(f..),
            )
            .alert(UNUSUAL),
        None => rules
            .rule(
                Bid::new(2, Strain::Notrump),
                190,
                len(a, 5..) & len(b, 5..) & points(8..),
            )
            .alert(UNUSUAL),
    }
}

/// Our action over their weak-two opening
///
/// A weak two steals a level of room, so the toolkit is leaner than over a
/// one-bid: a takeout double (the workhorse), a natural 2NT overcall (15–18
/// with a stopper), and natural suit overcalls at the cheapest legal level.
/// Strong hands (17+) still double first, planning to bid again.
///
/// Overcall levels are derived from `their_opening`, so the suits higher than
/// theirs sit at the opening level and the lower ones one rung up — over 2♥, a
/// spade overcall is 2♠ but a club overcall is 3♣.
///
/// # Panics
///
/// Panics if `their_opening` is a notrump bid; pass a suit opening.
#[must_use]
pub fn defense_to_weak_two(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let (nt_lo, nt_hi) = weak_two_notrump_points();
    let mut rules = Rules::new();
    // The wide arm is `balanced() | (two_notrump_wide_shape() & two top honours in their
    // suit)`: the extra shapes are the 6322 and long-minor hands, which have a
    // trick source but one fewer stopper-guarded entry than a flat hand, so they
    // are asked for a *real* holding rather than the crisp Jxxx that
    // `stopper_in_their_suits` accepts.  Balanced hands keep today's gate, so the
    // knob only ever adds.
    rules = if weak_two_notrump_shape() {
        let theirs_suit = theirs.suit().expect("weak two is a suit bid");
        rules.rule(
            Bid::new(2, Strain::Notrump),
            150,
            hcp(nt_lo..=nt_hi)
                & stopper_in_their_suits()
                & (balanced() | (two_notrump_wide_shape() & top_honors(theirs_suit, 2..))),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            150,
            hcp(nt_lo..=nt_hi) & balanced() & stopper_in_their_suits(),
        )
    };

    // 12+ takeout double, optionally gated on unbid-suit support (see
    // [`set_takeout_support`]); the 17+ tier catches off-shape strong hands.
    rules = match takeout_support() {
        TakeoutSupport::Off => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & takeout_double_shape_ok(),
        ),
        TakeoutSupport::Lenient => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & unbid_support(1) & takeout_double_shape_ok(),
        ),
        TakeoutSupport::Strict => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & unbid_support(0) & takeout_double_shape_ok(),
        ),
    }
    .alert(TAKEOUT_DOUBLE);

    // The pass gate documents the 17+ tier's complement, exactly as
    // `defense_to_suit` does — "strong hands double first regardless".
    // Byte-identical to the old `hcp(0..)` catch-all: below the floor both score
    // 0.0, above it the shape-free tier is finite at weight 1.2 and always
    // outscores a weight-0 pass.  Authored so the pass reading
    // (`set_pass_reading`) has a band to project; the ⊤ census found the
    // direct-seat pass over their weak two reading *nothing* on all five axes.
    rules = rules
        .rule(Call::Double, 120, points(17..))
        .alert(TAKEOUT_DOUBLE);
    rules = if weak_two_pass_gate() {
        rules.rule(Call::Pass, 0, points(..17))
    } else {
        rules.rule(Call::Pass, 0, hcp(0..))
    };

    // Natural overcalls: five-card suit, 10–16 points, at the cheapest legal level.
    // The jump one rung above is the same call with a trick more of hand — six-plus
    // cards and three more points — but only where it still fits under 3NT; every
    // other jump is at the four level, where Leaping Michaels already lives.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let overcall_level = if strain > theirs { level } else { level + 1 };
            // The extra trick has to be paid for: the band is graded by the level
            // the overcall lands on (`set_weak_two_overcall_points`; default flat).
            let (two_lo, two_hi, three_lo, three_hi) = weak_two_overcall_points();
            let (lo, hi) = if overcall_level <= 2 {
                (two_lo, two_hi)
            } else {
                (three_lo, three_hi)
            };
            // Red-vs-white is where a light overcall gets punished, and the
            // measurement splits on *our* vulnerability alone — so the
            // discipline is authored as a vulnerability conjunct rather than a
            // flat band (`set_weak_two_overcall_discipline`).
            rules = if weak_two_overcall_discipline() {
                let (vul_lo, vul_hi) = if overcall_level <= 2 {
                    (12, 17)
                } else {
                    (15, 17)
                };
                rules.rule(
                    Bid::new(overcall_level, strain),
                    100,
                    len(suit, 5..) & points_by_vul(lo..=hi, vul_lo..=vul_hi),
                )
            } else {
                rules.rule(
                    Bid::new(overcall_level, strain),
                    100,
                    len(suit, 5..) & points(lo..=hi),
                )
            };
            if weak_two_jump_overcall() && overcall_level < 3 {
                rules = rules.rule(
                    Bid::new(overcall_level + 1, strain),
                    110,
                    len(suit, 6..) & points(13..=19),
                );
            }
        }
    }

    // The direct cue of their MAJOR weak two: the other major plus a minor, 5-5 —
    // what BBA bids and what `set_cue_reading` already reads.  Over 2♦ the cue is
    // deliberately absent (see `set_weak_two_cue`).
    //
    // Game-forcing, the same `points(14..)` Leaping Michaels demands, because the
    // cue *is* a game force by geometry: over 2♠ advancer cannot bid 3♥ under it,
    // so a heart preference costs 4♥ and every other answer is 3NT or four of a
    // minor.  A `points(8..)` Michaels here would commit an 8-count to the four
    // level.
    let cue_major = match theirs {
        Strain::Hearts => Some(Suit::Spades),
        Strain::Spades => Some(Suit::Hearts),
        _ => None,
    };
    if weak_two_cue()
        && let Some(other) = cue_major
    {
        rules = rules
            .rule(
                Bid::new(3, theirs),
                160,
                len(other, 5..) & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)) & points(14..),
            )
            .alert(MICHAELS);
    }

    // Leaping Michaels: a jump to 4♣/4♦ showing a 5-5 two-suiter with
    // game-forcing values.  These are all 4-level jumps, so they never collide
    // with the natural overcalls above (which sit at the 2/3 level), and 4♦ over
    // 2♦ is a cue the natural loop skips.
    if leaping_michaels_enabled() {
        let t = theirs.suit().expect("weak two is a suit bid");
        let gf = points(14..);
        match t {
            // Over a major: a minor plus the OTHER major.  Superseded by the cue
            // when that is on — the cue shows the same hand a level cheaper, and
            // at the same `points(14..)` every Leaping hand also satisfies it, so
            // leaving both would author a rung the weights can never reach.  BBA
            // makes the same choice: `--mode def2-h` shows a `3♥` cue bucket and
            // no `4♣`/`4♦` bucket at all.
            Suit::Hearts | Suit::Spades if !weak_two_cue() => {
                let other = if t == Suit::Hearts {
                    Suit::Spades
                } else {
                    Suit::Hearts
                };
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    rules = rules
                        .rule(
                            Bid::new(4, Strain::from(minor)),
                            200,
                            len(minor, 5..) & len(other, 5..) & gf.clone(),
                        )
                        .alert(LEAPING);
                }
            }
            // Over 2♦: 4♣ = clubs + a major; 4♦ (cue) = both majors.  Advancer's
            // continuation (incl. the 4♣ major-ask) is authored in
            // `leaping_michaels_advances`.
            Suit::Diamonds => {
                rules = rules
                    .rule(
                        Bid::new(4, Strain::Clubs),
                        200,
                        len(Suit::Clubs, 5..)
                            & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                            & gf.clone(),
                    )
                    .alert(LEAPING)
                    .rule(
                        Bid::new(4, Strain::Diamonds),
                        200,
                        len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & gf.clone(),
                    )
                    .alert(LEAPING);
            }
            Suit::Clubs => {} // no weak 2♣ in our system
            // Majors with the cue on: the guarded arm above declined them.
            Suit::Hearts | Suit::Spades => {}
        }
    }
    rules
}

/// [`defense_to_weak_two`] over each weak-two opening, as rows
///
/// The exact-node pilot of the row layer: `P* (2♦)` and friends lower to the
/// plain seat-fanned insert.  Clubs is omitted — a 2♣ opening is the strong
/// artificial bid, not a weak two.
fn weak_two_defense_package() -> Package {
    Package {
        name: "weak-two-defense",
        gate: || true,
        entries: || {
            [Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(2, Strain::from(suit));
                    rows_of(
                        Pattern::node(&format!("P* ({opening})")),
                        defense_to_weak_two(opening),
                    )
                })
                .collect()
        },
    }
}

/// Advancer's Gladiator structure over our `2NT` overcall of their weak two in
/// a **major** ([`set_weak_two_notrump_advances`])
///
/// The 1NT-level Gladiator ([`gladiator_advances`]) needs an invitational tier
/// and spends the two level on it.  Here the overcall is narrow — 16–17 by
/// default — so eight points opposite is already game values and the tier
/// vanishes: `3♣` is the weak relay, everything above it is game-forcing, and
/// the cue is Stayman.  The threshold tracks
/// [`set_weak_two_notrump_points`]'s floor, so a widened band raises it
/// instead of silently keeping a tierless structure calibrated for 16.
///
/// Every artificial call states its true meaning in its own rule text, so the
/// `.alert(...)` is the whole reading — `project_authored` decodes the box and
/// suppresses the phantom natural suit at the same index.  No bespoke
/// `Inferences` arm is needed (contrast `gladiator_reading`, whose relay has no
/// sound per-suit floor to project).
fn weak_two_notrump_advances(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    // Game values opposite the band's *minimum*: at the default 16–17 that is
    // eight (16 + 8 = 24).  Keyed to the band rather than frozen at 8, so
    // widening it with [`set_weak_two_notrump_points`] cannot leave advancer
    // driving to game on a 23-count — the bias would fall on exactly the hands
    // the widening adds.
    let game = 24u8.saturating_sub(weak_two_notrump_points().0);

    Rules::new()
        // Cue = Stayman for the *one* unbid major.  A flat (4333) is barred —
        // with no ruffing value a 4-4 fit does not beat 3NT (the 4333 curse).
        .rule(
            Bid::new(3, m),
            140,
            len(o, 4..=4) & points(game..) & !flat_4333(),
        )
        .alert(WEAK_TWO_NT_STAYMAN)
        // Game-forcing naturals: a real five-plus suit.
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) & points(game..),
        )
        .rule(Bid::new(3, os), 130, len(o, 5..) & points(game..))
        // Balanced game, to play — the overcaller holds the stopper, so the
        // notrump is right-sided as it stands.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            balanced() & points(game..),
        )
        // `3♣` = relay: a weak diamond hand looking for a 3-level partscore.
        // The forced `3♦` is the landing spot; a weak hand with the *other*
        // major has none (its correction would be `3♠`, unauthored) and passes.
        .rule(
            Bid::new(3, Strain::Clubs),
            50,
            points(..game) & len(Suit::Diamonds, 5..),
        )
        .alert(WEAK_TWO_NT_RELAY)
        .rule(Call::Pass, 30, hcp(0..))
}

/// Overcaller's forced `3♦` completion of the `3♣` relay
///
/// Pass-or-correct and utterly blind — it says nothing about diamonds, which is
/// why it is alerted: the alert is what stops the walk floring a phantom suit.
fn weak_two_notrump_relay_reply() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, hcp(0..))
        .alert(WEAK_TWO_NT_RELAY_PC)
}

/// Advancer's rebid over the forced `3♦`: pass to play it, or cue their major
/// to say the diamonds are long enough that `4♦` is safe
///
/// Only authored over their `2♥`, where the cue is `3♥`.  Over `2♠` the cue is
/// `3♠` itself, which is left unauthored for now — so the whole node is.
fn weak_two_notrump_relay_rebid(their_major: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(their_major)),
            100,
            len(Suit::Diamonds, 6..),
        )
        .alert(WEAK_TWO_NT_DIAMONDS)
        .rule(Call::Pass, 50, hcp(0..))
}

/// At least 5-4 (or 4-5) in the two named suits — the Landy two-suiter shape
fn five_four(a: Suit, b: Suit) -> Cons<impl Constraint + Clone> {
    (len(a, 5..) & len(b, 4..)) | (len(a, 4..) & len(b, 5..))
}

/// A *passed-hand* two-suiter in `a`+`b`: at least 5-4, but with neither suit
/// six-plus.  A passed hand holding a six-card suit would have opened a weak two
/// or a three-level preempt in first seat (see `openings.rs`), so those openable
/// shapes are excluded from the passed-hand 1NT defense — leaving the genuine
/// two-suiters that had no first-seat voice.  (A 5-4 two-suiter has at most four
/// cards in any third suit, so capping `a`/`b` at five bars every six-card suit.)
fn passed_two_suiter(a: Suit, b: Suit) -> Cons<impl Constraint + Clone> {
    five_four(a, b) & len(a, ..=5) & len(b, ..=5)
}

// ---------------------------------------------------------------------------
// Defense to their 1NT — per-call alerts
// ---------------------------------------------------------------------------
//
// A defensive "system" (Natural, Woolsey, DONT, …) is a *bundle* of per-call
// conventions: only the call carries a convention, not the system.  "Woolsey"
// is `X` = Woolsey + `2♣` = Landy + `2♦` = Multi + `2♥`/`2♠` = Muiderberg.  So
// each artificial `(call, convention)` is authored once as an alerted block, all
// of them are chained at the `[1NT]` node, and [`Rules::gated`] ships only the
// active system's calls at book-construction time (the same build-time gate the
// European 1NT minors use; see `notrump::notrump_responses`).
//
// **An [`Alert`] marks an artificial call: only artificial calls carry one.**  An
// unalerted call is *natural* and *floor-safe* — dropping its book node is at worst
// suboptimal, because the instinct floor bids it sensibly and reads it right.  An
// artificial call must be pinned by a book node (and an `Inferences::read`
// decoding), or the floor misreads the convention and raises a phantom suit into a
// doubled minus.  So the penalty `X`, the four natural suit overcalls, and `Pass`
// stay unalerted (authored where they are a measured DD win, via
// [`chain_natural_base`]); the conventions are the alerts — the same per-call
// [`Alert`] now carried by every artificial call system-wide (see [`Rules::alert`]).

/// Michaels cue-bid — 2 of their suit, 5-5, 8+ HCP (a two-suiter)
const MICHAELS: Alert = Alert("michaels");

/// Advancer's cue of opener's suit after partner's takeout double — general
/// invitational (10–11) with a 4-card unbid major, asking partner to raise it
const ADVANCE_CUE: Alert = Alert("advance-cue");

/// Advancer's jump-cue Rubens transfer — invitational-or-better with a 5+ unbid
/// major (the suit one rank above the bid), asking the doubler to declare it
const ADVANCE_TRANSFER: Alert = Alert("advance-transfer");

/// Unusual 2NT over a suit opening — 5-5 in the two lowest unbid suits
const UNUSUAL: Alert = Alert("unusual-2nt");

/// Leaping Michaels — a 4♣/4♦ jump over their weak two, a 5-5 game-forcing
/// two-suiter (distinct from the responder-side `comp:leaping-michaels`)
const LEAPING: Alert = Alert("leaping-michaels");

/// `3♣` relay over our 2NT overcall of their weak two — a weak *diamond* hand
/// looking for a 3-level partscore, routed through the forced `3♦`.  Says
/// nothing about clubs.
const WEAK_TWO_NT_RELAY: Alert = Alert("weak-two-nt:relay");
/// The forced `3♦` completion of that relay — pass-or-correct, and blind: it
/// says nothing about diamonds.
const WEAK_TWO_NT_RELAY_PC: Alert = Alert("weak-two-nt:relay-pc");
/// Cue of their weak two over our 2NT overcall = Stayman for the one unbid
/// major (exactly four, game values, not flat).
const WEAK_TWO_NT_STAYMAN: Alert = Alert("weak-two-nt:stayman");
/// Delayed cue (`3♣` relay → forced `3♦` → cue) — six-plus diamonds, long
/// enough that `4♦` is safe.  Says nothing about their major.
const WEAK_TWO_NT_DIAMONDS: Alert = Alert("weak-two-nt:diamonds");

/// Gladiator `2♣` relay over our 1NT overcall of their major — a weak takeout
/// (any suit) or an invitational hand routed through the forced `2♦`.
const GLADIATOR_RELAY: Alert = Alert("gladiator:relay");
/// Gladiator's forced `2♦` completion of the `2♣` relay — pass-or-correct, says
/// nothing about diamonds.
const GLADIATOR_RELAY_PC: Alert = Alert("gladiator:relay-pc");
/// Gladiator cue of their major — Stayman for the *one* unbid major (exactly 4,
/// invitational-or-better).
const GLADIATOR_STAYMAN: Alert = Alert("gladiator:stayman");
/// Gladiator `2NT` — a weak transfer to clubs (6+♣): a weak long-club hand
/// cannot sign off naturally below the cue, so it detours through `2NT` and the
/// overcaller completes to `3♣` (invitational clubs go `2♣`→`2♦`→`3♣` instead).
const GLADIATOR_CLUB_TRANSFER: Alert = Alert("gladiator:club-transfer");
/// Gladiator splinter (`3` of their major) — a game-forcing raise of the unbid
/// major with a singleton/void in their suit.
const GLADIATOR_SPLINTER: Alert = Alert("gladiator:splinter");
/// Gladiator **delayed cue** (`2♣` relay → forced `2♦` → cue of their major) —
/// exactly 3 in the unbid major, INV+, **not flat (4333)**.  The direct cue
/// promises 4; this delayed route promises exactly 3 with a doubleton (ruffing
/// value), checking for the 5-3 fit a balanced 5-card-major 1NT overcall can hold
/// (over `1♠`, a balanced 15–18 may hold 5 hearts).
const GLADIATOR_DELAYED_CUE: Alert = Alert("gladiator:delayed-cue");

/// Responsive double — partner doubled/overcalled, they raised, advancer's double
/// shows the two unbid suits (4-4, 8+).  A takeout call (asks partner to pick a
/// suit), not a desire to defend, so it is alerted rather than read structurally.
const RESPONSIVE: Alert = Alert("responsive-double");

/// Direct takeout double of their suit opening — 12+ (or the 17+ any-shape tier),
/// short in their suit, asking partner to pick an unbid suit.  Takeout by meaning,
/// so alerted even though its shape predicates project no length floor (the reading
/// is a sound points floor only — the 17+ tier admits any shape).
const TAKEOUT_DOUBLE: Alert = Alert("takeout-double");

/// Landy SOS redouble — after `[1NT, 2♣, X]`, equal majors asking the overcaller to
/// name the longer one.  A "pick a suit" call, not a desire to sit, so alerted.
const LANDY_SOS: Alert = Alert("landy:sos-redouble");

/// Answers to the Landy `2NT` game-force ask — conventional step responses, not
/// natural suits: `3♣`/`3♦` are the 5-4 strength steps and name no minor at all,
/// `3♥`/`3♠`/`3NT` are 5-5 minimum / medium / maximum.  The row-layer invariant
/// caught `3♥`: it floors *spades* at five while naming hearts.
const LANDY_2NT_ANSWER: Alert = Alert("landy:2nt-answer");

const WOOLSEY_X: Alert = Alert("1ntd:woolsey-x");
const LANDY_X: Alert = Alert("1ntd:landy-x");
const DONT_X: Alert = Alert("1ntd:dont-x");
const LANDY_2C: Alert = Alert("1ntd:landy-2c");
const WOOLSEY_2C: Alert = Alert("1ntd:woolsey-2c");
const DONT_2C: Alert = Alert("1ntd:dont-2c");
const MULTI_2D: Alert = Alert("1ntd:multi-2d");
const DONT_2D: Alert = Alert("1ntd:dont-2d");
const MUIDERBERG_2H: Alert = Alert("1ntd:muiderberg-2h");
const DONT_2H: Alert = Alert("1ntd:dont-2h");
const MUIDERBERG_2S: Alert = Alert("1ntd:muiderberg-2s");
const UNUSUAL_2NT: Alert = Alert("1ntd:unusual-2nt");
/// Meckwell two-way `X` — a single 6+ minor OR both majors.
const MECKWELL_X: Alert = Alert("1ntd:meckwell-x");
/// Meckwell `2♣` — clubs + a major (5-4+).
const MECKWELL_2C: Alert = Alert("1ntd:meckwell-2c");
/// Meckwell `2♦` — diamonds + a major (5-4+).
const MECKWELL_2D: Alert = Alert("1ntd:meckwell-2d");
/// Lead-directing double of the opponents' 2♣ Stayman — shows clubs (the bid
/// suit), not takeout.
const STAYMAN_DEFENSE_X: Alert = Alert("staydef:x-clubs");
/// Lead-directing double of the opponents' Jacoby transfer — shows the bid
/// (transfer) suit, not takeout.
const TRANSFER_DEFENSE_X: Alert = Alert("xferdef:x-bidsuit");
/// Cue of the suit the opponents showed via transfer — the other major + a minor
/// (Michaels).
const TRANSFER_DEFENSE_CUE: Alert = Alert("xferdef:cue-michaels");
/// Lead-directing double of the opponents' two-way 2♠ minor response — shows
/// spades (the bid suit), not takeout.
const MINOR_TRANSFER_DEFENSE_X: Alert = Alert("minorxferdef:x-spades");
/// `2NT` over their 2♠ — the two lowest unbid suits (diamonds + hearts, 5-5).
const MINOR_TRANSFER_DEFENSE_2NT: Alert = Alert("minorxferdef:2nt-reds");
/// Cue of their shown-clubs anchor (`3♣`) — the top-and-bottom two-suiter
/// (spades + diamonds, 5-5).
const MINOR_TRANSFER_DEFENSE_CUE: Alert = Alert("minorxferdef:cue-top-bottom");
/// Lead-directing double of the opponents' 2NT diamond transfer — shows diamonds
/// (the shown suit), not takeout.
const DIAMOND_TRANSFER_DEFENSE_X: Alert = Alert("diaxferdef:x-diamonds");
/// Cue of their shown-diamonds anchor (`3♦`) — both majors (5-5, Michaels).
const DIAMOND_TRANSFER_DEFENSE_CUE: Alert = Alert("diaxferdef:cue-majors");

// Each artificial block is a one-rule `Rules` lifting today's cascade verbatim
// (weight, shape, strength).  All twelve are chained unconditionally and then
// gated, so each reads its tuning knobs defensively (an `unwrap_or` placeholder
// band on a gated-out block never reaches the trie).

/// Woolsey takeout `X`: a 4-card major + a longer (5-6) minor, `points(floor..)`.
fn woolsey_x() -> Rules {
    Rules::new().rule(
        Call::Double,
        190,
        woolsey_double_shape() & points(woolsey_double_floor()..),
    )
}

/// Direct-Landy `X`: both majors (5-4, or flat 4-4 when configured), replacing the
/// 15+ penalty double; weight 1.9 beats the natural 2♥/2♠ so a both-majors hand
/// doubles rather than picking one major.
fn landy_x() -> Rules {
    let four_four = direct_landy_double().unwrap_or(false);
    Rules::new().rule(
        Call::Double,
        190,
        both_majors_shape(four_four) & points(direct_landy_double_floor()..),
    )
}

/// Defense to the opponents' 2♣ Stayman (`(1NT)-P-(2♣)`)
///
/// `X` = lead-directing clubs (5+ with values, the bid suit — not takeout);
/// `2♦/2♥/2♠` = a natural **6-card** suit; `3♣` = a **strong** natural club
/// one-suiter (declare, not preempt).  No Michaels cue (their 2♣ is artificial,
/// so a cue would be natural); an Unusual 2NT (both minors) was tried and
/// measured DD-negative (−4.9 IMPs/fired), so it was dropped.  An owning Pass
/// catches the ~80% that act on nothing, keeping the floor's undisciplined
/// balancing calls out.
///
/// The overcall length and points floor were **A/B-searched**, not copied from
/// BBA: a paired perfect-defense (PD) sweep ([`set_stayman_defense_overcall`])
/// settled on a six-card suit at `points(14..)`.  Over a *strong* 1NT the bidding
/// side holds the points, so a natural overcall into their auction is PD-negative
/// when light — the sweep is monotone in the floor (the 8–13 overcalls lose, 14
/// turns DD-harmless) and prefers length-6 over length-5 (the 5-card overcalls'
/// plain-DD edge is the light-sacrifice artifact PD prices away).  Routing the
/// weak long-club hand to `Pass` instead of a `3♣` preempt drops a DD-negative
/// obstruction bid; the strong `3♣` (tracking the same floor) is weighted above
/// the `X` so a real club hand declares rather than lead-directs.
fn defense_to_their_stayman() -> Rules {
    let (min_len, floor) = stayman_defense_overcall();
    Rules::new()
        .rule(
            Call::Double,
            190,
            len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..) & points(8..),
        )
        .alert(STAYMAN_DEFENSE_X)
        .rule(
            Bid::new(2, Strain::Diamonds),
            180,
            len(Suit::Diamonds, min_len..) & points(floor..),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            180,
            len(Suit::Hearts, min_len..) & points(floor..),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            180,
            len(Suit::Spades, min_len..) & points(floor..),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            200,
            len(Suit::Clubs, 6..) & points(floor..),
        )
        .rule(Call::Pass, 50, hcp(0..))
}

/// Defense to the opponents' Jacoby transfer (`(1NT)-P-(2♦→♥)` / `(2♥→♠)`)
///
/// `X` = lead-directing the `bid` (transfer) suit (5+ with values, not takeout);
/// a cue of the `shown_major` (the suit they transferred into) = the **other**
/// major + a minor (Michaels 5-5); natural one-suiter overcalls in every suit but
/// the one they showed (six-card, `points(14..)`, the A/B-searched Stayman-defense
/// floor — light overcalls into a strong-1NT auction are PD-negative), with the
/// transfer suit's own 3-level overcall weighted above the `X` so a real suit
/// declares rather than lead-directs.  An owning Pass catches the ~80% that act
/// on nothing.  Distilled from BBA (probe modes `xfer-h`/`xfer-s`).
fn defense_to_their_transfer(bid: Suit, shown_major: Suit) -> Rules {
    let (min_len, floor) = (6usize, 14u8);
    let other_major = if shown_major == Suit::Spades {
        Suit::Hearts
    } else {
        Suit::Spades
    };
    let mut rules = Rules::new()
        .rule(
            Call::Double,
            190,
            len(bid, 5..) & suit_hcp(bid, 5..) & points(8..),
        )
        .alert(TRANSFER_DEFENSE_X)
        .rule(
            Bid::new(2, Strain::from(shown_major)),
            170,
            len(other_major, 5..)
                & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                & points(8..),
        )
        .alert(TRANSFER_DEFENSE_CUE);
    // Natural one-suiter overcalls in every suit but the one they showed, each at
    // its cheapest legal level above their transfer; the transfer suit's own
    // overcall is the *strong* 3-level declare (weight 2.0) above the lead-direct X.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == shown_major {
            continue;
        }
        let strain = Strain::from(suit);
        let level = if strain > Strain::from(bid) { 2 } else { 3 };
        let weight = if suit == bid { 200 } else { 180 };
        rules = rules.rule(
            Bid::new(level, strain),
            weight,
            len(suit, min_len..) & points(floor..),
        );
    }
    rules.rule(Call::Pass, 50, hcp(0..))
}

/// Defense to the opponents' two-way 2♠ minor response (`(1NT)-P-(2♠)`)
///
/// Their 2♠ names spades (the bid) but means clubs (the anchor), so: `X` =
/// lead-directing spades (5+ with values, not takeout); `2NT` = the two lowest unbid
/// suits (diamonds + hearts, 5-5); `3♣` (cueing their clubs anchor) = the
/// top-and-bottom two-suiter (spades + diamonds, 5-5), weighted **above** the `X` so
/// a genuine two-suiter shows rather than lead-directs; natural `3♦`/`3♥` six-card
/// one-suiters (`points(14..)`, the A/B-searched Stayman-defense floor — light
/// overcalls into a strong-1NT auction are PD-negative).  An owning Pass catches the
/// ~80% that act on nothing.  Modeled on [`defense_to_their_transfer`].
fn defense_to_their_minor_transfer() -> Rules {
    Rules::new()
        // X = lead-directing spades (the bid suit), 5+ with values.
        .rule(
            Call::Double,
            190,
            len(Suit::Spades, 5..) & suit_hcp(Suit::Spades, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_X)
        // 2NT = the two lowest unbid suits (diamonds + hearts, 5-5) — naturally
        // disjoint from the spade-showing X.
        .rule(
            Bid::new(2, Strain::Notrump),
            170,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_2NT)
        // 3♣ cue of their clubs anchor = top-and-bottom (spades + diamonds, 5-5);
        // weight 2.0 beats the X so the two-suiter wins for a 5♠5♦ hand.
        .rule(
            Bid::new(3, Strain::Clubs),
            200,
            len(Suit::Spades, 5..) & len(Suit::Diamonds, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_CUE)
        // Natural six-card one-suiter overcalls in the unbid red suits.
        .rule(
            Bid::new(3, Strain::Diamonds),
            180,
            len(Suit::Diamonds, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            180,
            len(Suit::Hearts, 6..) & points(14..),
        )
        .rule(Call::Pass, 50, hcp(0..))
}

/// Our defense to the opponents' 2NT diamond transfer (`(1NT)-P-(2NT)-?`)
///
/// Their 2NT shows diamonds, so: `X` = lead-directing diamonds (5+ with values,
/// not takeout); `3♦` (cueing their diamond anchor) = both majors (5-5, Michaels),
/// weighted **above** the `X` so a genuine two-suiter shows rather than
/// lead-directs; natural `3♣`/`3♥`/`3♠` six-card one-suiters (`points(14..)`).  An
/// owning Pass catches the rest.  Modeled on [`defense_to_their_minor_transfer`].
fn defense_to_their_diamond_transfer() -> Rules {
    Rules::new()
        // X = lead-directing diamonds (the shown suit), 5+ with values.
        .rule(
            Call::Double,
            190,
            len(Suit::Diamonds, 5..) & suit_hcp(Suit::Diamonds, 5..) & points(8..),
        )
        .alert(DIAMOND_TRANSFER_DEFENSE_X)
        // 3♦ cue of their diamond anchor = both majors (5-5); weight 2.0 beats the
        // X so a 5♥-5♠ two-suiter shows rather than lead-directs.
        .rule(
            Bid::new(3, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
        )
        .alert(DIAMOND_TRANSFER_DEFENSE_CUE)
        // Natural six-card one-suiter overcalls in the unbid suits.
        .rule(
            Bid::new(3, Strain::Clubs),
            180,
            len(Suit::Clubs, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            180,
            len(Suit::Hearts, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            180,
            len(Suit::Spades, 6..) & points(14..),
        )
        .rule(Call::Pass, 50, hcp(0..))
}

/// DONT `X`: a one-suiter (♣/♦/♥), `points(direct-dont-x-floor..)`.
fn dont_x() -> Rules {
    let lo = direct_dont_x_floor();
    let one_min = direct_dont_one_suiter_min();
    Rules::new().rule(
        Call::Double,
        190,
        dont_one_suiter_direct(one_min) & points(lo..),
    )
}

/// Landy `2♣`: both majors, at least 5-4, on the shared two-suiter band
/// ([`woolsey_points`], coupled with Woolsey's identical `2♣`; see [`set_landy`]),
/// gauged as raw HCP or upgraded points per [`set_landy_hcp`].
fn landy_2c() -> Rules {
    let (lo, hi) = woolsey_points();
    let shape = five_four(Suit::Hearts, Suit::Spades);
    if landy_use_hcp() {
        Rules::new().rule(Bid::new(2, Strain::Clubs), 190, shape & hcp(lo..=hi))
    } else {
        Rules::new().rule(Bid::new(2, Strain::Clubs), 190, shape & points(lo..=hi))
    }
}

/// Woolsey `2♣` (Landy, inside the bundle): both majors, but neither major 6+
/// (`passed_two_suiter` caps each major at five, routing a 6-card major to the
/// Multi `2♦` and keeping the bundle's uniform 1.9 weights disjoint).  A distinct
/// block from [`landy_2c`] — same convention, load-bearing shape difference.
fn woolsey_2c() -> Rules {
    let (lo, hi) = woolsey_points();
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        190,
        passed_two_suiter(Suit::Hearts, Suit::Spades) & points(lo..=hi),
    )
}

/// DONT `2♣`: clubs + a higher major, 5-4 (or 4-4 when configured).
fn dont_2c() -> Rules {
    let lo = natural_overcall_points().0;
    let ff = direct_dont_four_four();
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        200,
        dont_minor_major(Suit::Clubs, ff) & points(lo..),
    )
}

/// Woolsey Multi `2♦`: a single 6+ major.
fn multi_2d() -> Rules {
    let (lo, hi) = woolsey_points();
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        190,
        woolsey_multi() & points(lo..=hi),
    )
}

/// DONT `2♦`: diamonds + a higher major, 5-4 (or 4-4 when configured).
fn dont_2d() -> Rules {
    let lo = natural_overcall_points().0;
    let ff = direct_dont_four_four();
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        200,
        dont_minor_major(Suit::Diamonds, ff) & points(lo..),
    )
}

/// Woolsey Muiderberg `2♥`/`2♠`: exactly 5 in `major` + a 4+ minor.
fn muiderberg(major: Suit) -> Rules {
    let (lo, hi) = woolsey_points();
    Rules::new().rule(
        Bid::new(2, Strain::from(major)),
        190,
        woolsey_muiderberg(major) & points(lo..=hi),
    )
}

/// DONT `2♥`: both majors, 5-4 (or 4-4 when configured).
fn dont_2h() -> Rules {
    let lo = natural_overcall_points().0;
    let ff = direct_dont_four_four();
    Rules::new().rule(
        Bid::new(2, Strain::Hearts),
        200,
        dont_both_majors(ff) & points(lo..),
    )
}

/// Meckwell two-way `X`: a single 6+ minor OR both majors,
/// `points(meckwell-x-floor..)`.  The both-majors shape is the probe knob
/// [`set_meckwell_x_four_four`], the floor is [`set_meckwell_x_floor`]; the
/// single-minor length is a fixed 6.
fn meckwell_x() -> Rules {
    let lo = meckwell_x_floor();
    Rules::new().rule(
        Call::Double,
        190,
        meckwell_double_shape(6, meckwell_x_four_four()) & points(lo..),
    )
}

/// Meckwell `2♣`: clubs + a major, 5-4 either way (or flat 4-4 per the probe knob
/// [`set_meckwell_minor_major_44`]).  Shares [`dont_minor_major`]'s shape on the
/// Meckwell knob so the two conventions can diverge.
fn meckwell_2c() -> Rules {
    let lo = natural_overcall_points().0;
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        200,
        dont_minor_major(Suit::Clubs, meckwell_minor_major_44()) & points(lo..),
    )
}

/// Meckwell `2♦`: diamonds + a major, 5-4 either way (or flat 4-4 per the probe knob).
fn meckwell_2d() -> Rules {
    let lo = natural_overcall_points().0;
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        200,
        dont_minor_major(Suit::Diamonds, meckwell_minor_major_44()) & points(lo..),
    )
}

/// Unusual `2NT`: both minors, 5-5, on its own range (raw HCP or points per
/// [`set_landy_hcp`]).  Additive — compatible with every system.
fn unusual_2nt() -> Rules {
    let (lo, hi) = unusual_notrump_range().unwrap_or((0, 37));
    let shape = len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..);
    if landy_use_hcp() {
        Rules::new().rule(Bid::new(2, Strain::Notrump), 180, shape & hcp(lo..=hi))
    } else {
        Rules::new().rule(Bid::new(2, Strain::Notrump), 180, shape & points(lo..=hi))
    }
}

/// The four natural two-level suit overcalls (five-card suit, `points(8..=14)`),
/// optionally skipping `2♣` when the Landy `2♣` overlay owns that slot.
fn chain_natural_overcalls(mut rules: Rules, skip_clubs: bool) -> Rules {
    let (oc_lo, oc_hi) = natural_overcall_points();
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == Suit::Clubs && skip_clubs {
            continue;
        }
        rules = rules.rule(
            Bid::new(2, Strain::from(suit)),
            100,
            len(suit, 5..) & points(oc_lo..=oc_hi),
        );
    }
    rules
}

/// Chain the untagged, floor-safe natural calls the active system uses: the owning
/// `Pass`, and — at the slots no artificial alert owns — the penalty `X` and the
/// natural suit overcalls.  Mirrors the pre-tag cascade's natural arms exactly; the
/// conventions are chained and gated separately in [`defense_to_notrump`].  A slot
/// no live system owns is simply not authored and falls to the instinct floor (the
/// natural-off baseline arm).
fn chain_natural_base(rules: Rules) -> Rules {
    // One active system — the enum makes the old "two families at once" state (which
    // the read-time cascade precedence Woolsey > DONT > Meckwell > direct-Landy >
    // natural used to arbitrate) unrepresentable.
    match notrump_defense() {
        // Woolsey owns X and every overcall; `Pass` is the only natural call.  Always-
        // pass is the same finite `Pass` logit — it shadows the floor so our side never
        // competes.
        NotrumpDefense::Woolsey | NotrumpDefense::AlwaysPass => rules.rule(Call::Pass, 0, hcp(0..)),
        NotrumpDefense::DirectDont => {
            // DONT keeps the natural `2♠` one-suiter (open-top, length-gated so the
            // one-suiter `X` can exclude spades) below its two-suiters, plus `Pass`.
            let lo = natural_overcall_points().0;
            let one_min = direct_dont_one_suiter_min();
            rules
                .rule(
                    Bid::new(2, Strain::Spades),
                    100,
                    len(Suit::Spades, one_min..) & points(lo..),
                )
                .rule(Call::Pass, 0, hcp(0..))
        }
        NotrumpDefense::Meckwell => {
            // Meckwell keeps the natural 5+ single-suited majors (2♥/2♠, disjoint from
            // its two-suiters) below the alerts, plus Pass.  The two-way X / minor+major
            // 2♣/2♦ / both-minors 2NT are the artificial calls.
            let lo = natural_overcall_points().0;
            rules
                .rule(
                    Bid::new(2, Strain::Hearts),
                    100,
                    meckwell_natural_major(Suit::Hearts) & points(lo..),
                )
                .rule(
                    Bid::new(2, Strain::Spades),
                    100,
                    meckwell_natural_major(Suit::Spades) & points(lo..),
                )
                .rule(Call::Pass, 0, hcp(0..))
        }
        NotrumpDefense::DirectLandy => {
            // The both-majors `X` is the alert; the four natural overcalls and `Pass`
            // are the floor-safe base (a 15+ balanced hand now passes or overcalls).
            chain_natural_overcalls(rules.rule(Call::Pass, 0, hcp(0..)), false)
        }
        NotrumpDefense::Natural => {
            // Penalty `X` (HCP floor fixed; shape gate per `set_natural_double_shape` —
            // each arm reissues `.rule()` so the differing constraint types unify), the
            // owning `Pass`, and the natural overcalls (ceding `2♣` to a Landy overlay).
            let floor = natural_double_floor();
            let w = natural_double_weight();
            let rules = match natural_double_shape() {
                DoubleShape::Balanced => rules.rule(Call::Double, w, hcp(floor..) & balanced()),
                DoubleShape::SemiBalanced => {
                    rules.rule(Call::Double, w, hcp(floor..) & semi_balanced())
                }
                DoubleShape::Any => rules.rule(Call::Double, w, hcp(floor..)),
            };
            chain_natural_overcalls(rules.rule(Call::Pass, 0, hcp(0..)), landy_range().is_some())
        }
        // No system: author nothing, fall to the instinct floor.
        NotrumpDefense::Off => rules,
    }
}

/// The artificial alerts live at the `[1NT]` node for the configured system, one
/// per [`NotrumpDefense`] plus the two independent overlays.  Read once at
/// book-construction time.
fn active_alerts() -> Vec<Alert> {
    let mut alerts = Vec::new();
    let system = notrump_defense();
    match system {
        // Always-pass authors only `Pass` — no alerts, no overlays.
        NotrumpDefense::AlwaysPass => return alerts,
        NotrumpDefense::Woolsey => {
            alerts.extend([
                WOOLSEY_X,
                WOOLSEY_2C,
                MULTI_2D,
                MUIDERBERG_2H,
                MUIDERBERG_2S,
            ]);
        }
        NotrumpDefense::DirectDont => alerts.extend([DONT_X, DONT_2C, DONT_2D, DONT_2H]),
        NotrumpDefense::Meckwell => alerts.extend([MECKWELL_X, MECKWELL_2C, MECKWELL_2D]),
        NotrumpDefense::DirectLandy => alerts.push(LANDY_X),
        // The natural penalty-X family and the bare floor add no alert of their own.
        NotrumpDefense::Natural | NotrumpDefense::Off => {}
    }
    // The Landy `2♣` overlay is the natural family's one convention, incompatible with
    // DONT / Meckwell / direct-Landy-X / Woolsey (each repurposes the `2♣` slot) — so
    // it rides only on the non-convention arms.
    if landy_range().is_some() && matches!(system, NotrumpDefense::Natural | NotrumpDefense::Off) {
        alerts.push(LANDY_2C);
    }
    // Unusual `2NT` is additive — every non-always-pass system.
    if unusual_notrump_range().is_some() {
        alerts.push(UNUSUAL_2NT);
    }
    alerts
}

/// Our defense to the opponents' 1NT opening, composed from per-call alert tags
///
/// The untagged natural base ([`chain_natural_base`]) and every artificial alert
/// are chained at the `[1NT]` node; [`Rules::gated`] then ships only the active
/// system's alerts (untagged natural rules always survive).  [`active_alerts`]
/// guarantees at most one convention per call, and the natural base skips any slot
/// an alert owns, so no two rules collide at a node.
pub fn defense_to_notrump() -> Rules {
    let alerts = active_alerts();
    chain_natural_base(Rules::new())
        .chain(woolsey_x().alert(WOOLSEY_X))
        .chain(landy_x().alert(LANDY_X))
        .chain(dont_x().alert(DONT_X))
        .chain(landy_2c().alert(LANDY_2C))
        .chain(woolsey_2c().alert(WOOLSEY_2C))
        .chain(dont_2c().alert(DONT_2C))
        .chain(multi_2d().alert(MULTI_2D))
        .chain(dont_2d().alert(DONT_2D))
        .chain(muiderberg(Suit::Hearts).alert(MUIDERBERG_2H))
        .chain(dont_2h().alert(DONT_2H))
        .chain(muiderberg(Suit::Spades).alert(MUIDERBERG_2S))
        .chain(meckwell_x().alert(MECKWELL_X))
        .chain(meckwell_2c().alert(MECKWELL_2C))
        .chain(meckwell_2d().alert(MECKWELL_2D))
        .chain(unusual_2nt().alert(UNUSUAL_2NT))
        .gated(move |t| alerts.contains(&t))
}

// Direct-seat DONT shapes.  Unlike the passed-hand twins these carry no six-card
// cap (an unpassed hand may hold a long suit), and they carve clubs+diamonds onto
// the `2NT` both-minors overlay so `2♣`/`2♦` mean a minor + a *major*.

/// Direct-seat DONT `X`: a one-suiter (a `min`+ suit, no second four-card suit) whose
/// long suit is a minor or hearts.  A spade one-suiter bids the natural `2♠`, so the
/// spade-long arm is omitted; each arm caps the other three suits at three, so exactly
/// one suit is long.  `min` (5 or 6) is [`set_direct_dont_one_suiter_min`].
fn dont_one_suiter_direct(min: usize) -> Cons<impl Constraint + Clone> {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    (len(Clubs, min..) & and([Diamonds, Hearts, Spades], ..=3))
        | (len(Diamonds, min..) & and([Clubs, Hearts, Spades], ..=3))
        | (len(Hearts, min..) & and([Clubs, Diamonds, Spades], ..=3))
}

/// Direct-seat DONT `2♣`/`2♦`: a minor + a *major*, 5-4 either way (or a flat 4-4
/// when `allow_44`).  The higher suit is ♥/♠ only — a minor + the other minor is
/// shown as `2NT` (both minors), not here.  `allow_44` is
/// [`set_direct_dont_four_four`].
fn dont_minor_major(minor: Suit, allow_44: bool) -> Cons<impl Constraint + Clone> {
    let longer = if allow_44 { 4 } else { 5 };
    // The minor (4+) plus a higher major (4+), one of the two at least `longer` — 5-4
    // either way, or a flat 4-4 when `allow_44` (then the third clause is redundant).
    len(minor, 4..)
        & or([Suit::Hearts, Suit::Spades], 4..)
        & (len(minor, longer..) | or([Suit::Hearts, Suit::Spades], longer..))
}

/// Direct-seat DONT `2♥`: both majors, 5-4 either way (or a flat 4-4 when `allow_44`).
/// A separate function from [`both_majors_shape`] (direct-Landy `X`) — identical shape
/// today, but on an independent flag, so the two conventions may diverge.
fn dont_both_majors(allow_44: bool) -> Cons<impl Constraint + Clone> {
    let longer = if allow_44 { 4 } else { 5 };
    and([Suit::Hearts, Suit::Spades], 4..) & or([Suit::Hearts, Suit::Spades], longer..)
}

/// Meckwell two-way `X`: a single `min`+ minor (♣ or ♦, the other three suits ≤3) OR
/// both majors (5-4, or flat 4-4 when `four_four`).  The signature two-way double —
/// the two arms are disjoint (the one-suiter caps its majors ≤3, the both-majors
/// floors them ≥4), so the reading can tell a single-minor from a both-majors hand by
/// the majors alone.  `min` is a fixed 6 (the DONT one-suiter parity length).
fn meckwell_double_shape(min: usize, four_four: bool) -> Cons<impl Constraint + Clone> {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    (len(Clubs, min..) & and([Diamonds, Hearts, Spades], ..=3))
        | (len(Diamonds, min..) & and([Clubs, Hearts, Spades], ..=3))
        | both_majors_shape(four_four)
}

/// Meckwell natural `2♥`/`2♠`: a 5+ single-suited major — the other major ≤3 (both
/// majors go through the `X`) and both minors ≤3 (a minor + this major goes through
/// `2♣`/`2♦`).  A pure one-suiter, disjoint from every Meckwell artificial call so a
/// 6-4 hand shows its two-suiter (`2♣`/`2♦`/`X`) rather than tying the natural rung.
fn meckwell_natural_major(major: Suit) -> Cons<impl Constraint + Clone> {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    len(major, 5..) & len(other, ..=3) & and([Suit::Clubs, Suit::Diamonds], ..=3)
}

/// M6.2d guard: every re-authored `or`/`and` defense shape accepts exactly the hands
/// its intended spec does, on every sampled hand — the proof the combinator forms say
/// what they should (and the only check that the simplified shapes match their gloss).
#[cfg(test)]
mod shape_guards;

/// Advancer's responses to partner's Landy `2♣` (both majors), per
/// [bridgebum](https://www.bridgebum.com/landy.php)
///
/// `2♦` = equal majors, weak (correct to the longer); `2♥`/`2♠` = preference
/// signoff; `2NT` = game-forcing ask; `3♥`/`3♠` = invitational with 4-card
/// support; `4♥`/`4♠` = to play game with a fit.  The invite/game point
/// thresholds track the `2♣` range — anchored so `lo = 10` reproduces bridgebum's
/// 10–12 invite / 12+ force — so a lighter overcall needs a stronger advancer to
/// reach the same game.
fn landy_advances(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);

    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    let equal_majors = equal_length("equal majors", Suit::Hearts, Suit::Spades);

    Rules::new()
        // Game with a known 4-card fit (preferred over the ask).
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        // Game-forcing ask without a clear 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 120, points(game..))
        // Invitational with 4-card support.
        .rule(
            Bid::new(3, Strain::Hearts),
            110,
            len(Suit::Hearts, 4..) & points(invite..game) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            len(Suit::Spades, 4..) & points(invite..game) & spades_longer.clone(),
        )
        // Weak: equal majors → 2♦ relay; else preference signoff.
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            equal_majors & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            90,
            hearts_longer & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            90,
            spades_longer & points(..invite),
        )
}

/// Advancer's response to a *doubled* Landy `2♣` (`[1NT, 2♣, X]`)
///
/// The opponents' Double is the stolen `2♣` Stayman, and their opener can sit for
/// `2♣` doubled with good clubs (the [`set_penalty_pass`] conversion) — a disaster
/// for us, since the Landy overcaller is both-majors / short-club.  The Double also
/// hands us an extra step (the Redouble), so we run a richer escape than over a pass:
///
/// - **Redouble** = equal majors, "you pick" — the relay the undoubled `2♦` was.
/// - **Pass** = a long club one-suiter: play `2♣` doubled (the doubler walked in).
/// - **`2♦`** = a long diamond one-suiter, natural and to play (the freed bid).
/// - **`2♥`/`2♠`** = the longer major (weak signoff), as over a pass.
/// - the strong arms (`4M` game, `2NT` game-ask, `3M` invite) are unchanged — the
///   Double buys no room above `2NT`.
///
/// A minor one-suiter (Pass / `2♦`) needs *both majors ≤2*: opposite the overcaller's
/// guaranteed 5-card major a 3-card major has an 8-card fit worth more than a doubled
/// minor, so those hands relay (Redouble) or sign off into the major instead.
///
/// [`set_penalty_pass`]: super::set_penalty_pass
fn landy_advances_over_double(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);

    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    let equal_majors = equal_length("equal majors", Suit::Hearts, Suit::Spades);
    // A long minor with both majors short (no 8-card fit opposite the overcaller's
    // 5-carder) outranks a major signoff. Gate A/B-tuned via set_doubled_landy_escape.
    let (min_minor, max_major) = doubled_landy_escape();
    let short_majors = len(Suit::Hearts, ..=max_major) & len(Suit::Spades, ..=max_major);

    Rules::new()
        // Strong arms — identical to the undoubled advance (no room gained above 2NT).
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        .rule(Bid::new(2, Strain::Notrump), 120, points(game..))
        .rule(
            Bid::new(3, Strain::Hearts),
            110,
            len(Suit::Hearts, 4..) & points(invite..game) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            110,
            len(Suit::Spades, 4..) & points(invite..game) & spades_longer.clone(),
        )
        // Long club one-suiter, no major fit: sit for 2♣ doubled.
        .rule(
            Call::Pass,
            105,
            len(Suit::Clubs, min_minor..) & short_majors.clone(),
        )
        // Long diamond one-suiter, no major fit: natural 2♦, to play.
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, min_minor..) & short_majors & points(..game),
        )
        // Equal majors: Redouble asks the overcaller to name the longer one.
        .rule(Call::Redouble, 95, equal_majors & points(..invite))
        .alert(LANDY_SOS)
        // Otherwise sign off in the longer major.
        .rule(
            Bid::new(2, Strain::Hearts),
            90,
            hearts_longer & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            90,
            spades_longer & points(..invite),
        )
}

/// Overcaller's rebid after advancer's *natural* `2♦` over the doubled Landy
/// (`[1NT, 2♣, X, 2♦, P]`): pass partner's diamonds, but with a singleton/void
/// diamond pull to the longer major (a 5-2 major fit beats a 6-1 diamond one).
fn landy_doubled_2d_rebid() -> Rules {
    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    Rules::new()
        .rule(
            Bid::new(2, Strain::Hearts),
            100,
            len(Suit::Diamonds, ..=1) & hearts_longer,
        )
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Diamonds, ..=1) & spades_longer,
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller's rebid after the `2♦` relay (`[1NT, 2♣, P, 2♦, P]`): name the
/// longer major, so the equal-majors advancer plays the right strain
fn landy_2d_rebid() -> Rules {
    let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, hearts_longer)
        .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
}

/// A Pass-only node: settle, play the contract on the table.  Authoring this where
/// the instinct floor would otherwise run keeps a finite logit on `Pass`, so the
/// floor's over-competition is shadowed (see `project_floor_shadowed_by_book_nodes`).
fn sit() -> Rules {
    Rules::new().rule(Call::Pass, 0, hcp(0..))
}

/// Advancer's runout after partner's both-majors `X` is **redoubled** (`[1NT, X, XX]`)
///
/// The redouble forces our side to act (sitting plays `1NTxx`), but it also frees a
/// clean structure: over the redoubled one-level `1NT` our `2♣` sits at the two level,
/// so the advancer has a *natural* rung for every suit.  **`Pass` = "ask back"** — no
/// suit of our own and no major preference, so the doubler names its longer (five-card)
/// major over the opponents' pass; **a bid (`2♣`/`2♦`/`2♥`/`2♠`, or `4♥`/`4♠`) = to
/// play** the natural suit.  No artificial `2♦` relay — that phantom diamond was what
/// let the floor run a doubled major into `3♦x` (the dominant DD leak); here the only
/// `2♦` is real diamonds, so a double of it is sat, not run from.
fn both_majors_x_runout(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    let hearts_longer = longer_suit(Suit::Hearts, Suit::Spades);
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    let short_majors = len(Suit::Hearts, ..=2) & len(Suit::Spades, ..=2);
    Rules::new()
        // To-play game with a big fit in the preferred major.
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            140,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        // Own long minor with no major fit → to play the minor.
        .rule(
            Bid::new(2, Strain::Clubs),
            110,
            len(Suit::Clubs, 5..) & short_majors.clone(),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            110,
            len(Suit::Diamonds, 5..) & short_majors,
        )
        // Major preference → to play.
        .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
        .rule(Bid::new(2, Strain::Hearts), 100, hearts_longer)
        // Equal majors / nothing to say → ask: the doubler names its five-card major.
        .rule(Call::Pass, 50, hcp(0..))
}

// ---------------------------------------------------------------------------
// Passed-hand DONT advances.  Both partners passed in [P,P,P,1NT,...], so the
// advancer is capped below opening too: every response is a pass-or-correct
// signoff at the two level — no invite/game/ask arms (they are unreachable).
// ---------------------------------------------------------------------------

/// Advancing partner's DONT one-suiter double (`[…,1NT,X,P]`): relay `2♣` to ask
/// which suit.  (A passed advancer is too weak to introduce its own suit, so the
/// single relay covers it.)
fn passed_dont_x_advance() -> Rules {
    Rules::new().rule(Bid::new(2, Strain::Clubs), 100, hcp(0..))
}

/// Doubler naming the one-suiter after the `2♣` relay (`[…,1NT,X,P,2♣,P]`): pass
/// with clubs, else bid the five-or-six-card suit.
fn passed_dont_x_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Diamonds, 5..))
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 5..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing partner's DONT `2♣` (clubs + a higher suit, `[…,1NT,2♣,P]`): pass
/// with club tolerance, else relay `2♦` ("name your higher suit").
fn passed_dont_2c_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Clubs, ..=2))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler naming the higher suit after the `2♦` relay (`[…,1NT,2♣,P,2♦,P]`):
/// pass with diamonds, else bid the major.
fn passed_dont_2c_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing partner's DONT `2♦` (diamonds + a major, `[…,1NT,2♦,P]`): pass with
/// diamond tolerance, else relay `2♥` ("name your major").
fn passed_dont_2d_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Diamonds, ..=2))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler naming the major after the `2♥` relay (`[…,1NT,2♦,P,2♥,P]`): pass with
/// hearts, correct to `2♠` with spades.
fn passed_dont_2d_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing partner's DONT `2♥` (both majors, `[…,1NT,2♥,P]`): pass with hearts,
/// correct to `2♠` with longer spades.
fn passed_dont_2h_advance() -> Rules {
    let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancing Meckwell's two-way `X` (`[…,1NT,X,P]`): relay `2♣` (pass-or-correct) —
/// the doubler then names its minor or shows both majors.  A single relay resolves the
/// two-way double's ambiguity; the advancer's own suits wait for the doubler's answer.
fn meckwell_x_advance() -> Rules {
    Rules::new().rule(Bid::new(2, Strain::Clubs), 100, hcp(0..))
}

/// The Meckwell doubler naming its hand after the `2♣` relay (`[…,1NT,X,P,2♣,P]`):
/// pass with a club one-suiter, `2♦` with a diamond one-suiter (real diamonds, short
/// majors), or `2♥` with both majors (4+ hearts — the advancer then passes or corrects
/// to `2♠` via [`passed_dont_2h_advance`]).  Names real suits throughout, so nothing
/// here is artificial (the both-majors hand under-describes as hearts, always sound).
fn meckwell_x_rebid() -> Rules {
    Rules::new()
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, ..=3) & len(Suit::Spades, ..=3),
        )
        .rule(Bid::new(2, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller's rebid after the game-forcing `2NT` ask (`[1NT, 2♣, P, 2NT, P]`)
///
/// The sourced min/med/max × 5-4/5-5 ladder, with the strength buckets tracking
/// the `2♣` range (partition `[lo, hi]` into thirds, `hi` capped at 16 when the
/// overcall is open-topped): a 5-5 hand shows `3♥`/`3♠`/`3NT` for min/medium/max;
/// a 5-4 hand shows `3♣` (min-or-medium) / `3♦` (max).
fn landy_2nt_rebid(lo: u8, hi: u8) -> Rules {
    let hi = hi.min(16);
    let step = hi.saturating_sub(lo) / 3;
    let med = lo + step;
    let max = lo + 2 * step;
    let five_five = len(Suit::Hearts, 5..) & len(Suit::Spades, 5..);

    Rules::new()
        // 5-5: 3♥ minimum, 3♠ medium, 3NT maximum.
        .rule(
            Bid::new(3, Strain::Hearts),
            130,
            five_five.clone() & points(lo..med),
        )
        .alert(LANDY_2NT_ANSWER)
        .rule(
            Bid::new(3, Strain::Spades),
            130,
            five_five.clone() & points(med..max),
        )
        .alert(LANDY_2NT_ANSWER)
        .rule(Bid::new(3, Strain::Notrump), 130, five_five & points(max..))
        .alert(LANDY_2NT_ANSWER)
        // 5-4 (the source omits a min-5-4 slot, so 3♣ folds min+medium together).
        .rule(Bid::new(3, Strain::Clubs), 120, points(lo..max))
        .alert(LANDY_2NT_ANSWER)
        .rule(Bid::new(3, Strain::Diamonds), 120, points(max..))
        .alert(LANDY_2NT_ANSWER)
}

// ---------------------------------------------------------------------------
// Woolsey "Multi-Landy" continuations.  Authored in full so the structure never
// bleeds to the instinct floor: the Multi 2♦ (BBA's two-strength pass-or-correct
// with the 2♠ → 2NT heart-relay, plus a game-force ask), the Muiderberg 2♥/2♠
// (raises + the 2NT minor-ask), and the takeout X (relay to the minor / own major
// / ask).  The both-majors 2♣ reuses the Landy advances above.  Every artificial
// call also has a doubled / redoubled escape (wired in `defensive`) so the
// opponents can never trap us in a doubled artificial contract.
// ---------------------------------------------------------------------------

/// Advancer over the Woolsey **Multi** `2♦` (`[1NT, 2♦, P]` or `[1NT, 2♦, X]`):
/// a major pass-or-correct in two strengths, plus a game-forcing ask.  Holds no
/// `Pass`, so over a double it always corrects rather than sitting in `2♦x` (the
/// overcaller has a major, never diamonds).  Thresholds track the overcall floor
/// `lo` (the `20-lo` / `22-lo` rule, as [`landy_advances`]).
fn multi_advances(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Game-force: ask the overcaller to name its 6-card major (it jumps to 4M).
        .rule(Bid::new(2, Strain::Notrump), 100, points(game..))
        // Constructive pass-or-correct: overcaller passes spades / 2NT-relays hearts.
        .rule(Bid::new(2, Strain::Spades), 95, points(invite..game))
        // Weak pass-or-correct: overcaller passes hearts / corrects 2♠ / jumps with 7+.
        .rule(Bid::new(2, Strain::Hearts), 90, points(..invite))
}

/// Overcaller over the weak `2♥` pass-or-correct (`[1NT, 2♦, P, 2♥, P]`): pass
/// with six hearts, correct to `2♠` with six spades, jump to `3♥`/`3♠` with seven
fn multi_2h_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 110, len(Suit::Hearts, 7..))
        .rule(Bid::new(3, Strain::Spades), 110, len(Suit::Spades, 7..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..=6))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the constructive `2♠` pass-or-correct (`[1NT, 2♦, P, 2♠, *]`):
/// pass with spades, bid `3♥` with hearts.  Bidding the major directly (rather than
/// a 2NT heart-relay) keeps the rebid identical whether the `2♠` was passed or
/// doubled — over a double we must not be left to the floor.
fn multi_2s_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the game-forcing `2NT` ask (`[1NT, 2♦, P, 2NT, P]`): jump to
/// game in the 6-card major
fn multi_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(4, Strain::Spades), 100, len(Suit::Spades, 6..))
}

/// Advancer over a **Muiderberg** `2M` (`[1NT, 2M, P]`): raise the known 5-card
/// major with support (`4M` game / `3M` invitational, or a `3M` preempt with
/// four-card support), or with no fit ask the 4+ minor via `2NT` (overcaller
/// answers `3♣`/`3♦`); a weak no-fit hand passes and plays `2M`.  `major` is the
/// overcaller's suit; thresholds track the overcall floor `lo`.
fn muiderberg_advances(major: Suit, lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, strain), 120, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            110,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No major fit, invitational+: ask the 4+ minor (then place 3NT / minor game).
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            len(major, ..=2) & points(invite..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancer over a **doubled** Muiderberg `2M` (`[1NT, 2M, X]`): with a fit sit
/// for `2Mx` (a known 8+ card trump fit) or raise; with no fit escape via the
/// `2NT` minor-ask rather than be trapped in a doubled 5-1 misfit
fn muiderberg_advances_doubled(major: Suit, lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, strain), 120, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            110,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No fit → escape to the 4+ minor (any strength); a fit sits 2Mx.
        .rule(Bid::new(2, Strain::Notrump), 50, len(major, ..=2))
        .rule(Call::Pass, 0, len(major, 3..))
}

/// Overcaller answering the Muiderberg `2NT` minor-ask (`[1NT, 2M, …, 2NT, P]`):
/// name the 4+ minor — `3♦` with diamonds (longer or equal), else `3♣`
fn muiderberg_2nt_rebid() -> Rules {
    let diamonds_longer = at_least_as_long(Suit::Diamonds, Suit::Clubs);
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, diamonds_longer)
        .rule(Bid::new(3, Strain::Clubs), 90, hcp(0..))
}

/// Advancer over the Woolsey takeout `X` (`[1NT, X, P]`): bid a 5+ major of your
/// own (to play), ask with a game-going hand, else relay `2♣` to the doubler's
/// long minor.  The catch-all `2♣` owns a finite logit so the floor never runs.
fn woolsey_x_advance(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Our own good major outranks the doubler's (its major may be the other one).
        .rule(Bid::new(2, Strain::Spades), 111, len(Suit::Spades, 5..))
        .rule(Bid::new(2, Strain::Hearts), 110, len(Suit::Hearts, 5..))
        // Game-going: ask the doubler to name its 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 100, points(game..))
        // Weak / no major of our own: name your minor, I pass or correct.
        .rule(Bid::new(2, Strain::Clubs), 90, hcp(0..))
}

/// Doubler over the `2♣` minor relay (`[1NT, X, P, 2♣, P]`): pass with the club
/// minor, correct to `2♦` with the diamond minor (advancer denied a major)
fn woolsey_x_minor_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler over the `2NT` game-ask (`[1NT, X, P, 2NT, P]`): name the 4-card major
/// (the `X` always holds exactly one), leaving the advancer to place the game
fn woolsey_x_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 4..))
}

// ---------------------------------------------------------------------------
// Advances
// ---------------------------------------------------------------------------

/// Advancer's action after partner's takeout double, RHO passing: `(opening) X (P)`
///
/// Partner doubled for takeout and asked us to pick.  In priority order:
///
/// - **pass for penalty** with a trump stack (four-plus of their suit, two top
///   honors) — converting the takeout double into penalties;
/// - **jump to a major-suit game** with four-plus cards and opening values;
/// - **bid 3NT** with a stopper in their suit and game-going values;
/// - **bid a new suit** at the cheapest legal level with four-plus cards;
/// - **escape to the cheapest notrump** as a weak catch-all — no fit, no
///   stopper, nothing better to say (lebensohl in spirit);
/// - **pass** as the final fallback.
///
/// Suit and notrump levels are derived from `their_opening`, so the one builder
/// answers over a one-bid (advances at the one and two levels) and over a weak
/// two (advances at the two and three levels) alike.
///
/// # Panics
///
/// Panics if `their_opening` is a notrump bid; pass a suit opening.
#[must_use]
pub fn advance_double(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();

    // Convert for penalty: a trump stack sits for the double — yielding, under
    // `set_advance_pass_yield_major`, to a weak hand's 4+ unbid major.
    let sit = len(t, 4..) & top_honors(t, 2..) & hcp(6..);
    let mut rules = if advance_pass_yield_major_enabled() {
        Rules::new().rule(Call::Pass, 150, sit & (hcp(10..) | no_unbid_major(t)))
    } else {
        Rules::new().rule(Call::Pass, 150, sit)
    };
    rules = rules
        // 3NT to play: a stopper in their suit and game values.
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            hcp(13..) & stopper_in_their_suits(),
        )
        // Weak escape to the cheapest notrump: no fit, no stopper, no stack.
        .rule(Bid::new(level, Strain::Notrump), 30, hcp(0..))
        // Final fallback.
        .rule(Call::Pass, 0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        // Natural advance at the cheapest legal level (longest-first under the knob).
        rules = natural_advance(rules, t, suit, bid_level, 100, 4);
        // Major-suit game jump with support and opening values.
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            rules = rules.rule(Bid::new(4, strain), 140, len(suit, 4..) & points(11..));
        }
    }
    rules
}

/// Append the natural-suit advance of a takeout double: `suit`, bid at
/// `bid_level`, with weight `base` and minimum length `min_len`.
///
/// Off the [`set_longest_first_advance`] knob this is a single flat rule, so the
/// classifier's argmax tie-break advances the highest-ranking eligible suit.  On
/// it, the rule gains the [`longest_unbid`] condition, so the **longest** unbid
/// suit advances, an equal-length tie going to the higher rank (5♦4♠ → `1♦`,
/// 4-4 majors → `1♠`) — the same choice the retired weight ladder
/// (`base + 0.001·held + 0.0001·rank`) made, said as a constraint instead of a
/// race among rules.
fn natural_advance(
    rules: Rules,
    theirs: Suit,
    suit: Suit,
    bid_level: u8,
    base: i16,
    min_len: usize,
) -> Rules {
    let bid = Bid::new(bid_level, Strain::from(suit));
    if longest_first_advance_enabled() {
        rules.rule(
            bid,
            base,
            len(suit, min_len..) & longest_unbid(suit, theirs),
        )
    } else {
        rules.rule(bid, base, len(suit, min_len..))
    }
}

/// `suit` is the cheapest-to-bid 3-card suit of a hand with no 4-card suit
/// outside `theirs` — the forced-advance rung's discipline
///
/// With a 4-card suit somewhere the longest-first rung takes over; stuck below
/// that, the priority flips from highest-ranking to **cheapest bid**, keeping
/// the forced auction as low as possible — `(1♥)`–X–(P) with 3=2=3=3 bids
/// `1♠`, but `(1♠)`–X–(P) with 2=3=3=3 bids `2♣`.  One exact box: `suit`
/// exactly three cards, every rival whose advance is cheaper capped at two
/// (it would be forced first), every dearer rival capped at three (a fourth
/// card there promotes the hand to the longest-first rung).  Knob-off the
/// reading stays ⊤, leaving the companion `len` floor as the whole legacy
/// reading.
fn cheapest_forced(suit: Suit, theirs: Suit, their_level: u8) -> Cons<impl Constraint + Clone> {
    let bid_of = |s: Suit| {
        (
            if s > theirs {
                their_level
            } else {
                their_level + 1
            },
            s,
        )
    };
    let mut lengths = [Range::FULL_LENGTH; 4];
    lengths[suit as usize] = Range::new(3, 3);
    for rival in Suit::ASC {
        if rival == suit || rival == theirs {
            continue;
        }
        let cap = if bid_of(rival) < bid_of(suit) { 2 } else { 3 };
        lengths[rival as usize] = Range::new(0, cap);
    }
    shapes(
        format!("{suit} the cheapest 3-card suit"),
        vec![length_box(lengths)],
    )
}

/// No 4-card major outside `theirs` — the weak sit's license to convert
///
/// The [`set_advance_pass_yield_major`] yield: a weak advancer holding a 4+
/// unbid major has a constructive home the penalty conversion would bury, so
/// the sit is reserved for hands with none (or with cue-band strength, where
/// the conversion is a choice, not a default).  One box capping each unbid
/// major at three; knob-off the reading stays ⊤.
fn no_unbid_major(theirs: Suit) -> Cons<impl Constraint + Clone> {
    let mut lengths = [Range::FULL_LENGTH; 4];
    for major in [Suit::Hearts, Suit::Spades] {
        if major != theirs {
            lengths[major as usize] = Range::new(0, 3);
        }
    }
    shapes(
        format!("no 4-card major outside {theirs}"),
        vec![length_box(lengths)],
    )
}

/// Rich advance of partner's takeout double of a one-of-a-suit `their_opening`
/// (`(1t)–X–(P)–?`), gated by [`set_rich_advance_double`]
///
/// The flat [`advance_double`] ladder gives the advancer only a cheapest natural
/// suit, a `3NT`, and a penalty pass — so the whole 10+ invitational-and-up
/// band collapses into "bid your cheapest suit," flat, with no way to invite or
/// force.  This adds the missing structure:
///
/// - **cue of opener's suit** (`2t`) — *invitational-or-better*, forcing one
///   round: the residual for any 10+ hand with no natural limited bid (a
///   4-card major seeking the fit, a stopperless hand, a slam try).  Advancer
///   then clarifies — simple rebid = invite, jump = game force
///   ([`advance_cue_rebid`]).  *Artificial* (`ADVANCE_CUE`); `hcp(10..)`.
/// - **natural notrump ladder** — `1NT` 8–10, `2NT` 11–12 balanced, `3NT`
///   limited 13–17, each with a stopper in their suit.
/// - **new-suit jumps** — a *major* two-level jump is *constructive* (8–10, 4+)
///   and a three-level jump is *invitational* (10–12, 5+); a *minor* three-level
///   jump (only under [`set_advance_minor_jump`]) is an invitational one-suiter
///   (10–12, 5+) ranked below the notrump ladder and denying a 4-card unbid
///   major.  The cheapest new suit is natural weak (0–7, 4+).
/// - **major game jump** (`4M`, 5+) — always *limited* (slam tries cue):
///   two-way (shapely-weak or minimum game force, 11–15 points) when no Rubens
///   transfer exists, or purely preemptive (0–10) when a transfer carries the
///   strong hands.
/// - **forced 3-card suit** when broke with no 4-card suit outside their suit —
///   a takeout double cannot be passed for want of a bid; the cheapest such
///   bid, keeping the forced auction low.
/// - **penalty pass** with a trump stack (5+ of their suit, or 4 with two top
///   honors — a swept `suit_hcp` floor under [`set_advance_sit_hcp_gate`]).
#[must_use]
fn advance_double_rich(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");
    let level = their_opening.level.get();
    let cue = Bid::new(level + 1, theirs);

    // Penalty pass: a trump stack sits for the double — 5+ of their suit
    // (length alone is enough to convert), or 4 with two top honors (under
    // `set_advance_sit_hcp_gate`, a swept `suit_hcp` floor instead).  A weak
    // 5-card holding in their suit passes rather than being forced into a
    // three-card minor that the field doubles at the game level.  Under
    // `set_advance_pass_yield_major`, a hand below the 10+ cue band holding a
    // 4+ unbid major bids the ladder instead of sitting.
    fn sit_pass(t: Suit, quality: Cons<impl Constraint + Clone + 'static>) -> Rules {
        let sit = len(t, 5..) | (len(t, 4..) & quality);
        if advance_pass_yield_major_enabled() {
            Rules::new().rule(Call::Pass, 160, sit & (hcp(10..) | no_unbid_major(t)))
        } else {
            Rules::new().rule(Call::Pass, 160, sit)
        }
    }
    let mut rules = match advance_sit_hcp_gate() {
        Some(gate) => sit_pass(t, suit_hcp(t, gate..)),
        None => sit_pass(t, top_honors(t, 2..)),
    };

    // Cue of opener's suit — *invitational-or-better*, forcing for one round
    // (the standard advancer force).  It is the residual for any 10+ hand with
    // no natural limited bid to name — a 4-card-major invite/force seeking the
    // fit, a stopperless hand, a two-suiter, or a slam try.  Deliberately the
    // *lowest-weighted* action above the weak natural suit (1.0): every specific
    // limited bid (a jump, a notrump, a game) outranks it, so only the genuinely
    // shapeless invite-or-better lands here.  The advancer then clarifies —
    // simple rebid = invite (partner may pass), jump = game force
    // ([`advance_cue_rebid`]).  One rule (M6.2d).  Artificial → `ADVANCE_CUE`.
    rules = rules.rule(cue, 105, hcp(10..)).alert(ADVANCE_CUE);

    rules = rules
        // 3NT to play: a *limited* balanced-ish game (13–17) with a stopper and
        // no five-card major.  Bigger hands cue (slam try); shapelier ones bid
        // the suit.  Weighted just over the cue so a clear 3NT is not diverted.
        .rule(
            Bid::new(3, Strain::Notrump),
            145,
            hcp(13..=17) & stopper_in_their_suits(),
        )
        // 2NT: invitational (11–12) balanced with a stopper — almost denies a
        // 4-card major, which would have cued.
        .rule(
            Bid::new(level + 1, Strain::Notrump),
            115,
            hcp(11..=12) & balanced() & stopper_in_their_suits(),
        )
        // Natural 1NT: 8–10 with a stopper — the same invitational band as the
        // two-level constructive suit jump, offered in notrump.
        .rule(
            Bid::new(level, Strain::Notrump),
            110,
            hcp(8..=10) & stopper_in_their_suits(),
        )
        // Final fallback.
        .rule(Call::Pass, 0, hcp(0..));

    // Majors that a Rubens transfer can reach (only when the transfer layer is
    // on).  For these the strong hands transfer, so the direct `4M` jump is
    // freed up to be purely preemptive; for the rest `4M` is the limited game
    // force.  Over `(1♠)` hearts is *not* here (it sits below the jump-cue), so
    // `1♠`–X–`4♥` stays the minimum game force.
    let transfer_majors: Vec<Suit> = if advance_rubens_enabled() {
        advance_major_transfers(theirs)
            .into_iter()
            .map(|(_, target)| target)
            .collect()
    } else {
        Vec::new()
    };

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        if longest_first_advance_enabled() {
            // Natural advance at the cheapest legal level (weak, 0–7): the
            // longest unbid suit, an equal-length tie to the higher rank.
            rules = natural_advance(rules, t, suit, bid_level, 100, 4);
            // Forced 3-card suit: a takeout double cannot be passed for want of
            // a bid — but with no 4-card suit outside their suit the priority
            // flips from highest-ranking to **cheapest bid**, keeping the
            // forced auction as low as possible.  No HCP cap — the
            // higher-weight cue, notrump, jump, and pass rules take every hand
            // with a better call, leaving only the genuinely stuck ones here.
            rules = rules.rule(
                Bid::new(bid_level, strain),
                30,
                len(suit, 3..) & cheapest_forced(suit, t, level),
            );
        } else {
            // Natural advance at the cheapest legal level (weak, 0–7).
            rules = natural_advance(rules, t, suit, bid_level, 100, 4);
            // Forced 3-card suit: a takeout double cannot be passed for want of
            // a bid, so any hand with no 4-card suit and no notrump/cue home
            // still introduces its highest-ranking 3-card suit (no HCP cap —
            // the higher-weight cue, notrump, and 4-card-suit rules take every
            // hand that has a better call, leaving only the genuinely stuck
            // ones here).
            rules = natural_advance(rules, t, suit, bid_level, 30, 3);
        }
        // Jump in a new *major*: a cheap two-level jump is *constructive*
        // (8–10, 4+); the more committal three-level jump is *invitational* and
        // wants a real 5-card suit.  (A game-forcing hand cues or blasts `4M` —
        // see below — so both are capped.)
        let jump = bid_level + 1;
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            if jump == 2 {
                rules = rules.rule(Bid::new(2, strain), 120, hcp(8..=10) & len(suit, 4..));
            } else if jump == 3 {
                rules = rules.rule(Bid::new(3, strain), 125, hcp(10..=12) & len(suit, 5..));
            }
        } else if jump == 3 && advance_minor_jump_enabled() {
            // Three-level jump in a *minor* — an invitational one-suiter (5+,
            // 10–12) that DENIES a 4-card unbid major: with one the advancer cues
            // opener's suit to find the 4-4 major fit rather than burying it under
            // the minor.  It does *not* deny a stopper — the rule carries no
            // stopper term; it is simply weighted *below* the notrump ladder, so
            // a hand that fits a natural notrump invite (balanced, in the
            // `1NT`/`2NT` band) prefers that, while a shapely hand outside the
            // notrump band (a 6-card minor, a stiff) still jumps.  The 10–12 cap
            // keeps game-forcing minors cueing or bidding `3NT`, so — unlike the
            // old high-weighted minor jump — it never abandons a makeable game.
            // The doubler then accepts or declines ([`answer_advance_minor_jump`]).
            // At most three in each *unbid* major (opener's own major is not
            // constrained — `..=13` is vacuously true).
            let no_unbid_major = len(
                Suit::Hearts,
                ..=if theirs == Strain::Hearts { 13 } else { 3 },
            ) & len(
                Suit::Spades,
                ..=if theirs == Strain::Spades { 13 } else { 3 },
            );
            rules = rules.rule(
                Bid::new(3, strain),
                108,
                hcp(10..=12) & len(suit, 5..) & no_unbid_major,
            );
        }
        // Major-suit game jump `4M` (5+ — a 4-card major cues to check the fit).
        // A game jump is always *limited*: slam tries cue.  When a Rubens
        // transfer carries the strong hands it is purely preemptive (weak, long
        // trumps).  Without a transfer there is nowhere else for a shapely weak
        // hand to compete to game, so `4M` stays two-way — shapely-weak *or*
        // minimum game force — capped at 15 (points, distribution-aware) so slam
        // hands still cue.  Measured: the pure-MIN-FG gate stranded weak 6-card
        // majors below a makeable game (advance-double-v5, −0.005/bd DD).
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            let bid = Bid::new(4, strain);
            rules = if transfer_majors.contains(&suit) {
                rules.rule(bid, 150, len(suit, 5..) & hcp(0..=10))
            } else {
                rules.rule(bid, 150, len(suit, 5..) & points(11..=15))
            };
        }
    }

    // Jump-cue Rubens transfers: a 5+ unbid major (invitational-or-better) shows
    // via a transfer one rank below it, so the doubler declares (right-siding).
    // Weighted above the cue and the game-blast so a 5+ major routes here.
    if advance_rubens_enabled() {
        for (bid, target) in advance_major_transfers(theirs) {
            rules = rules
                .rule(bid, 160, hcp(10..) & len(target, 5..))
                .alert(ADVANCE_TRANSFER);
        }
        // Over (1♠) the sole unbid major (hearts) sits *below* the jump-cue, so
        // there is nothing to transfer into: a 5-card heart hand is already shown
        // by the natural three-level `3♥` jump (invitational) in the suit loop
        // above, and a game-forcing one cues `2♠` or blasts `4♥`.
    }

    rules
}

/// The advancer's jump-cue major transfers over a one-of-`theirs` opening:
/// `(transfer bid, the 5+ unbid major it shows)`.  A transfer is the rank
/// immediately below its target major, at the three level.  Over `(1♠)` the sole
/// unbid major (hearts, `3♥`) is below the jump-cue (`3♠`), so it is shown by the
/// natural invitational `3♥` jump in [`advance_double_rich`] instead and is not
/// returned here.
fn advance_major_transfers(theirs: Strain) -> Vec<(Bid, Suit)> {
    if theirs == Strain::Spades {
        return Vec::new();
    }
    let mut out = Vec::new();
    for target in [Suit::Hearts, Suit::Spades] {
        if Strain::from(target) == theirs {
            continue;
        }
        let below = match target {
            Suit::Hearts => Suit::Diamonds,
            Suit::Spades => Suit::Hearts,
            _ => unreachable!("only hearts and spades are majors"),
        };
        out.push((Bid::new(3, Strain::from(below)), target));
    }
    out
}

/// Doubler's completion of the advancer's Rubens transfer
/// (`[1t, X, P, transfer, {P,X}, ?]`, gated by [`set_advance_rubens`])
///
/// The transfer promised a 5+ `target` major; the doubler bids it (declaring —
/// the right-siding point), jumping to game (`4M`) with a maximum and support.
/// The completion is a finite catch-all so the artificial transfer is never
/// passed out.  Both bids are natural (`target`), so neither is alerted.
fn complete_advance_transfer(target: Suit) -> Rules {
    let strain = Strain::from(target);
    Rules::new()
        // Super-accept: maximum with support jumps to game.
        .rule(Bid::new(4, strain), 130, len(target, 4..) & points(15..))
        // Complete the transfer (always) — never pass the artificial call.
        .rule(Bid::new(3, strain), 100, hcp(0..))
}

/// Advancer's rebid after the doubler completed the transfer
/// (`[1t, X, P, transfer, {P,X}, 3M, P, ?]`)
///
/// The transfer was invitational-or-better; opposite the doubler's minimum
/// completion a game-forcing advancer (12+) raises to game, an invitational one
/// (10–11) rests in the three-level partscore.
fn advance_transfer_rebid(target: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(target)), 100, hcp(12..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler's answer to the advancer's cue (`[1t, X, P, cue, P, ?]`, gated by
/// [`set_rich_advance_double`])
///
/// The cue ([`advance_double_rich`]) is invitational-or-better and forcing for
/// one round, asking the doubler to describe.  With a minimum the doubler bids
/// its cheapest 4-card unbid major (or the `2NT` catch-all); with extras (15+)
/// it jumps — `4M` with a major, `3NT` with a stopper.  The advancer then
/// clarifies invite-vs-force ([`advance_cue_rebid`]).  The `2NT` catch-all
/// guarantees a bid so the artificial cue is **never passed out**, which would
/// strand us declaring the opponents' suit (the M6.3 "passed-out cue" trap).
/// Every bid here is natural, so none is alerted.
fn answer_advance_cue(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let mut rules = Rules::new()
        // Extras and a stopper, no major to raise: 3NT to play.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(15..) & stopper_in_their_suits(),
        )
        // Always-legal non-pass catch-all: never leave the artificial cue in.
        .rule(Bid::new(level + 1, Strain::Notrump), 20, hcp(0..));

    for major in [Suit::Hearts, Suit::Spades] {
        let m = Strain::from(major);
        if m == theirs {
            continue;
        }
        // The cheapest legal bid of the unbid major, above the cue at (level+1, theirs).
        let cheap = if m > theirs { level + 1 } else { level + 2 };
        // Show the 4-4 major fit: cheapest with a minimum, game with extras.
        rules = rules.rule(Bid::new(cheap, m), 130, len(major, 4..));
        rules = rules.rule(Bid::new(4, m), 150, len(major, 4..) & points(15..));
    }
    rules
}

/// The doubler's *non-game* answers to the advancer's cue — the ones over which
/// the advancer still has to clarify invite-vs-force ([`advance_cue_rebid`]).
///
/// These are exactly the minimum descriptions from [`answer_advance_cue`]: the
/// cheapest bid of each unbid major and the `2NT` catch-all.  (A `3NT`/`4M`
/// answer is already game — the advancer passes it or moves toward slam, which
/// the floor handles.)
fn advance_cue_answers(their_opening: Bid) -> Vec<Bid> {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();
    let mut out = vec![Bid::new(level + 1, Strain::Notrump)];
    for major in [Suit::Hearts, Suit::Spades] {
        let m = Strain::from(major);
        if m == theirs {
            continue;
        }
        let cheap = if m > theirs { level + 1 } else { level + 2 };
        out.push(Bid::new(cheap, m));
    }
    out
}

/// Advancer's clarifying rebid after the cue and the doubler's minimum `answer`
/// (`[1t, X, P, cue, {P,X}, answer, {P,X}, ?]`, gated by
/// [`set_rich_advance_double`])
///
/// The cue was invitational-or-better ([`advance_double_rich`]); here the
/// advancer resolves it against the doubler's minimum.  A *game-forcing* advancer
/// (13+) must reach game: raise the doubler's shown major with support, else bid
/// `3NT` (a stopper preferred, but forced even without — the game is on).  An
/// *invitational* advancer (10–12) has heard a minimum and stops (`Pass`).  This
/// is the "simple rebid = invite, jump = force" split, authored so a game force
/// cannot stall below game (the cue projects only `hcp(10..)`, so the floor
/// alone could read it as a mere invite and pass out).
fn advance_cue_rebid(answer: Bid) -> Rules {
    let mut rules = Rules::new();
    // Game force with a fit: raise the doubler's suit to game.
    if let Some(s) = answer.strain.suit() {
        rules = rules.rule(Bid::new(4, answer.strain), 100, len(s, 3..) & hcp(13..));
    }
    rules
        // Game force, no raise: notrump game (stopper preferred, else a punt).
        .rule(
            Bid::new(3, Strain::Notrump),
            60,
            hcp(13..) & stopper_in_their_suits(),
        )
        .rule(Bid::new(3, Strain::Notrump), 20, hcp(13..))
        // Invitational: partner showed a minimum — stop.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler's accept-or-decline of the advancer's invitational minor jump
/// (`[1t, X, P, 3m, {P,X}, ?]`, gated by [`set_advance_minor_jump`])
///
/// The jump is a *limited* natural invite (10–12, 5+ `minor`, no 4-card unbid
/// major) that does **not** promise a stopper, so — unlike the forcing cue,
/// which the doubler may never pass — the continuation is a natural-invite
/// accept/decline: **Pass** declines (too weak for game), a **new 5+ suit**
/// accepts game-forcing (the advancer places it — [`advance_minor_jump_rebid`]),
/// and **`3NT`** accepts to play *with the doubler's own stopper*.  With game
/// values but **no** stopper and no biddable side suit the doubler instead
/// **cues their suit** — a Western stopper-ask; the advancer supplies the
/// notrump from its side ([`advance_minor_stopper_ask_answer`]), right-siding
/// `3NT` when it holds the stopper.  The cue is the only artificial call here
/// (`ADVANCE_CUE`); the rest are natural.
fn answer_advance_minor_jump(their_opening: Bid, minor: Suit) -> Rules {
    let theirs = their_opening.strain;
    let m = Strain::from(minor);
    let mut rules = Rules::new()
        // Accept to play: 3NT with values and a stopper.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            hcp(15..) & stopper_in_their_suits(),
        )
        // Too weak for game: decline (the invite is limited, so Pass is safe).
        .rule(Call::Pass, 0, hcp(0..));
    // Accept by showing a new 5+ suit (game-forcing) — any unbid suit above the
    // jump, biddable at the three level.
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let s = Strain::from(suit);
        if s == theirs || s <= m {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(15..) & len(suit, 5..));
    }
    // Game values but no stopper and no 5-card side suit: cue their suit to ask
    // the advancer for the stopper (a Western cue).  Lowest-weighted of the game
    // tries, so a hand with its own stopper (`3NT`) or a biddable side suit (a
    // new suit) is routed there first; only the shapeless stopperless 15+ lands
    // here.  Always legal — the minor jump exists only *below* their suit, so
    // 3-of-their-suit sits above `3m` and below `3NT`.  Artificial → `ADVANCE_CUE`.
    rules = rules
        .rule(Bid::new(3, theirs), 100, hcp(15..))
        .alert(ADVANCE_CUE);
    rules
}

/// Advancer's placement after the doubler accepts the minor jump with a forcing
/// new suit (`[1t, X, P, 3m, {P,X}, 3S, {P,X}, ?]`, gated by
/// [`set_advance_minor_jump`])
///
/// The doubler forced to game showing a 5+ `shown` suit; the advancer (already
/// limited to 10–12) places it: raise to game with three-card support, else
/// `3NT` (a stopper preferred, but the game is on either way).
fn advance_minor_jump_rebid(shown: Suit) -> Rules {
    let s = Strain::from(shown);
    let game = if matches!(shown, Suit::Hearts | Suit::Spades) {
        4
    } else {
        5
    };
    Rules::new()
        // Support: raise the doubler's suit to game.
        .rule(Bid::new(game, s), 100, len(shown, 3..))
        // No support: notrump game (stopper preferred, else forced — game is on).
        .rule(Bid::new(3, Strain::Notrump), 60, stopper_in_their_suits())
        .rule(Bid::new(3, Strain::Notrump), 20, hcp(0..))
}

/// Advancer's answer to the doubler's stopper-ask cue after the minor jump
/// (`[1t, X, P, 3m, {P,X}, 3t, {P,X}, ?]`, gated by [`set_advance_minor_jump`])
///
/// The doubler cued their suit holding game values but no stopper (and no 5-card
/// side suit); the advancer supplies the notrump decision.  With a stopper the
/// advancer bids **`3NT`** — right-siding it, so the opening lead runs up to the
/// advancer's tenace — otherwise no stopper sits on either side, so the advancer
/// signs off in the **minor game** (both hands have shown game values).  Natural;
/// nothing to alert.
fn advance_minor_stopper_ask_answer(minor: Suit) -> Rules {
    let m = Strain::from(minor);
    Rules::new()
        // Stopper: the right-sided notrump game (the lead comes up to us).
        .rule(Bid::new(3, Strain::Notrump), 130, stopper_in_their_suits())
        // No stopper anywhere: play the minor game (game values are established).
        .rule(Bid::new(5, m), 50, hcp(0..))
}

/// Doubler's accept-or-decline of the advancer's invitational `2NT`
/// (`[1t, X, P, 2NT, {P,X}, ?]`, gated by [`set_advance_2nt_continuation`])
///
/// The `2NT` invite is a limited balanced 11–12 with a stopper (the advancer
/// supplies the notrump stopper), so the doubler — sitting on the wide takeout
/// range — simply answers a natural invite: **Pass** declines with a minimum,
/// **`3NT`** accepts to play, and a **new 5-card major** accepts game-forcing so
/// the advancer can choose the 4-4/5-3 major game over `3NT` (the advancer places
/// it — [`advance_minor_jump_rebid`], the same accept-a-forcing-suit logic).  A
/// 5-card *minor* is not shown: with the advancer's stopper `3NT` is almost
/// always right, so only the fit-seeking majors are worth the detour.  All
/// natural; nothing artificial to alert.
fn answer_advance_2nt(their_opening: Bid) -> Rules {
    let theirs = their_opening.strain;
    let mut rules = Rules::new()
        // Accept to play: 3NT with a maximum (the advancer holds the stopper).
        .rule(Bid::new(3, Strain::Notrump), 120, hcp(14..))
        // Minimum: decline the invite, play 2NT.
        .rule(Call::Pass, 0, hcp(0..));
    // Accept game-forcing by showing a 5-card major to seek the fit.
    for major in [Suit::Hearts, Suit::Spades] {
        let s = Strain::from(major);
        if s == theirs {
            continue;
        }
        rules = rules.rule(Bid::new(3, s), 130, points(14..) & len(major, 5..));
    }
    rules
}

/// A Section-5 sohl structure for our side's advancer over a single
/// interfering suit `over`, hung off the auction-string `base` (a three-call
/// prefix ending at the advancer's first turn) — the advancer's responses, the
/// relay completion, and (for `Transfer`) the transfer / cue-Stayman answers
/// plus the `(2♦)` Smolen + Leaping-Michaels package.  Shared by
/// [`advance_of_double_package`] (`P* (2X) X (P)`) and
/// [`gladiator_sohl_package`] (`P* (1M) 1NT (2Y)`).
/// `gate_4333` gates the flat-4333 Stayman/cue carve; callers pass `false` when
/// partner is known short in `over`, `true` when partner is balanced (a 1NT).
fn sohl_rows_over(base: &str, over: Suit, style: LebensohlStyle, gate_4333: bool) -> Vec<Entry> {
    let mut entries = Vec::new();

    // Advancer's first action shadows the floor (the builders end in a 0.0 Pass,
    // which covers the weak and penalty-pass hands).
    let advancer = match style {
        LebensohlStyle::Transfer if over == Suit::Diamonds => {
            transfer_stayman_2d_responder(gate_4333)
        }
        LebensohlStyle::Transfer => transfer_lebensohl_responder(over, gate_4333),
        _ => lebensohl_responder(over),
    };
    entries.extend(rows_of(Pattern::node(base), advancer));

    // Partner completes the 2NT relay with a forced 3♣; advancer then signs off.
    let relay = format!("{base} 2NT (P)");
    entries.extend(rows_of(Pattern::node(&relay), complete_lebensohl_relay()));
    entries.extend(rows_of(
        Pattern::node(&format!("{relay} 3♣ (P)")),
        lebensohl_relay_rebid(over),
    ));

    // Transfer style: partner answers each 3-level transfer / cue. Over (2♦) the
    // Smolen block below owns the 3-level replies, so this covers (2♥)/(2♠).
    if style == LebensohlStyle::Transfer && over != Suit::Diamonds {
        // Over (2♥)/(2♠) the delayed cue (2NT relay, then their suit) is always
        // *recognized* — answered as Stayman with a stopper — even when the bot
        // never bids it itself, so a human partner who plays it gets a sensible
        // reply. `split` (the default-off `set_delayed_cue` toggle) additionally
        // makes the bot *bid* the convention and read the direct cue as denying a
        // stopper (so it is answered without a free 3NT).
        let recognize = matches!(over, Suit::Hearts | Suit::Spades);
        let split = delayed_cue() && recognize;
        for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let resp = call(3, Strain::from(bid_suit));
            let reply = if bid_suit == over {
                if split {
                    cue_stayman_answer_no_stopper(over)
                } else {
                    cue_stayman_answer(over)
                }
            } else if let Some(target) = transfer_target(bid_suit, over) {
                transfer_completion(target, over)
            } else {
                continue; // the lowest suit has no transfer target — floored
            };
            entries.extend(rows_of(Pattern::node(&format!("{base} {resp} (P)")), reply));
        }
        // Delayed cue: base–2NT–P–3♣–P–3X (their suit) — Stayman with a stopper,
        // answered exactly like the direct cue but with 3NT safe. Wired whenever
        // it could be bid (recognition), independent of whether the bot bids it.
        if recognize {
            let cue = call(3, Strain::from(over));
            entries.extend(rows_of(
                Pattern::node(&format!("{relay} 3♣ (P) {cue} (P)")),
                cue_stayman_answer(over),
            ));
        }
    }

    // Transfer over (2♦): 3♣-Stayman + Smolen, the Jacoby transfers
    // (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦ — the diamond-only package
    // ported from the 1NT-(2♦) context. (2♥/2♠ reuse the Transfer completions above.)
    if style == LebensohlStyle::Transfer && over == Suit::Diamonds {
        let nodes: Vec<(&str, Rules)> = vec![
            // 3♣ Stayman, partner's answer; then Smolen after the 3♦ denial.
            ("3♣ (P)", stayman_2d_answer()),
            ("3♣ (P) 3♦ (P)", smolen_at_three()),
            ("3♣ (P) 3♦ (P) 3♥ (P)", smolen_completion(Suit::Spades)),
            ("3♣ (P) 3♦ (P) 3♠ (P)", smolen_completion(Suit::Hearts)),
            // Partner showed a 4-card major over Stayman; advancer places.
            ("3♣ (P) 3♥ (P)", stayman_2d_fit_rebid(Suit::Hearts)),
            ("3♣ (P) 3♠ (P)", stayman_2d_fit_rebid(Suit::Spades)),
            // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
            ("3♦ (P)", transfer_completion(Suit::Hearts, over)),
            ("3♥ (P)", transfer_completion(Suit::Spades, over)),
            ("3♠ (P)", clubs_transfer_completion(over)),
            // Leaping Michaels: 4♦ both majors, 4♣ clubs + a major (ask).
            ("4♦ (P)", lm_2d_both_majors_advance()),
            ("4♣ (P)", lm_2d_clubs_ask()),
            ("4♣ (P) 4♦ (P)", lm_2d_clubs_major()),
        ];
        for (rest, rules) in nodes {
            entries.extend(rows_of(Pattern::node(&format!("{base} {rest}")), rules));
        }
    }
    entries
}

/// Advancing partner's takeout double of a weak two, honoring the selected
/// [`set_advance_sohl_style`]
///
/// `Off` keeps the flat [`advance_double`] ladder.  `Plain`/`Transfer` shadow it
/// with the reused Section-5 sohl builders under the `P* (2X) X (P)` prefix — the
/// `2NT` relay (and, for `Transfer`, the transfers + cue-Stayman) — plus the
/// doubler's continuations (relay completion, the rebid after `3♣`, and the
/// transfer / cue answers).  Over `(2♦)`, `Transfer` additionally plays
/// `3♣`-Stayman + Smolen + Leaping Michaels.  A forcing 3-level suit (`Plain`) or
/// a constructive advance is driven on by the instinct floor, which already
/// handles forced-to-game auctions.
fn advance_of_double_package() -> Package {
    Package {
        name: "advance-of-weak-two-double",
        gate: || true,
        entries: || {
            let style = advance_sohl_style();
            [Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(2, Strain::from(suit));
                    let base = format!("P* ({opening}) X (P)");
                    if style == LebensohlStyle::Off {
                        rows_of(Pattern::node(&base), advance_double(opening))
                    } else {
                        // gate_4333 = false: advancing partner's takeout double —
                        // partner is short in their suit, so the 4-4 fit keeps its
                        // ruffing value (the 4333 curse does not apply here, and
                        // that A/B was never run).
                        sohl_rows_over(&base, suit, style, false)
                    }
                })
                .collect()
        },
    }
}

/// Gladiator: the advances of our 1NT overcall of their major
/// ([`set_nt_overcall_gladiator`])
///
/// Over a MAJOR one Stayman-found major is theirs, so the systems-on graft of
/// the whole opening-1NT structure does not fit the geometry; Gladiator replaces
/// it with a weak `2♣` relay, a cue-Stayman for the one unbid major `O`, and
/// shape actions.  Authored in every seat the opening could have been made
/// (mirrors the overcall's fan).  Two entries are not rules: their `(2♣)` is
/// rebased away (it steals no room, so systems stay on and only the relay is
/// consumed, reappearing as `X`), and the advance behind it is a transplant that
/// moves the relay's logit onto `Double`.  Their 2-level suit action instead
/// goes to [`gladiator_sohl_package`].
fn gladiator_package() -> Package {
    Package {
        name: "gladiator",
        gate: nt_overcall_gladiator,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                let base = format!("P* ({opening}) 1NT (P)");
                let os = Strain::from(other_major(suit));
                let cue = call(2, theirs);
                let cheap = if os > theirs { 2 } else { 3 };
                entries.extend(rows_of(Pattern::node(&base), gladiator_advances(suit)));

                // Advancer places the contract from what the cue answer showed —
                // the same ladder after the direct and the delayed cue.  Over `1♠`
                // the jump is `4♥` and the `3NT` misfit is already game, so those
                // advancer bids are left to the floor to pass.
                let cue_placements = |prefix: &str| {
                    let mut rows = rows_of(
                        Pattern::node(&format!("{prefix} {} (P)", call(cheap, os))),
                        gladiator_cue_min_fit(suit),
                    );
                    rows.extend(rows_of(
                        Pattern::node(&format!("{prefix} 2NT (P)")),
                        gladiator_cue_min_misfit(),
                    ));
                    if cheap + 1 < 4 {
                        rows.extend(rows_of(
                            Pattern::node(&format!("{prefix} {} (P)", call(cheap + 1, os))),
                            gladiator_cue_max_fit_raise(suit),
                        ));
                    }
                    rows
                };

                // Cue (Stayman for the one unbid major): overcaller answers, then
                // advancer places.
                let after_cue = format!("{base} {cue} (P)");
                entries.extend(rows_of(
                    Pattern::node(&after_cue),
                    gladiator_cue_answer(suit),
                ));
                entries.extend(cue_placements(&after_cue));

                // Natural invitational 2♦/2O and the 2NT weak club transfer —
                // overcaller accepts, or completes 3♣ for advancer to pass.
                for (advance, answer) in [
                    (call(2, Strain::Diamonds), gladiator_inv_diamond_answer()),
                    (call(2, os), gladiator_inv_major_answer(suit)),
                    (call(2, Strain::Notrump), gladiator_club_transfer_rebid()),
                    // Game-forcing naturals 3♣/3♦/3O and the 3M splinter —
                    // overcaller drives to game.
                    (call(3, Strain::Clubs), gladiator_gf_minor_answer()),
                    (call(3, Strain::Diamonds), gladiator_gf_minor_answer()),
                    (call(3, os), gladiator_gf_major_answer(suit)),
                    (call(3, theirs), gladiator_gf_major_answer(suit)),
                    // Leaping Michaels — overcaller places the 5-5 game force.
                    (
                        call(4, Strain::Clubs),
                        gladiator_leaping_answer(suit, Some(Suit::Clubs)),
                    ),
                    (
                        call(4, Strain::Diamonds),
                        gladiator_leaping_answer(suit, Some(Suit::Diamonds)),
                    ),
                    (call(4, theirs), gladiator_leaping_answer(suit, None)),
                ] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} {advance} (P)")),
                        answer,
                    ));
                }

                // 2♣ relay → forced 2♦ → advancer's XYZ-style sort; overcaller
                // then accepts or declines each invitational rebid.
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 2♣ (P)")),
                    gladiator_relay_rebid(),
                ));
                let sorted = format!("{base} 2♣ (P) 2♦ (P)");
                entries.extend(rows_of(
                    Pattern::node(&sorted),
                    gladiator_relay_continuation(suit),
                ));
                for inv in ["2NT", "3♣", "3♦"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{sorted} {inv} (P)")),
                        gladiator_relay_inv_answer(),
                    ));
                }
                entries.extend(rows_of(
                    Pattern::node(&format!("{sorted} {} (P)", call(3, os))),
                    gladiator_relay_major_answer(suit),
                ));
                // The weak `2O` takeout is a signoff, not a free bid — overcaller
                // passes it unless a max with four trumps pushes once.
                entries.extend(rows_of(
                    Pattern::node(&format!("{sorted} {} (P)", call(2, os))),
                    gladiator_relay_signoff_answer(suit),
                ));
                // Delayed cue (relay → forced 2♦ → cue of their major = exactly 3
                // `O`, INV+, not flat): overcaller shows min/max × 5-`O`-fit/misfit,
                // then advancer places with the same logic as after the direct cue.
                let delayed = format!("{sorted} {cue} (P)");
                entries.extend(rows_of(
                    Pattern::node(&delayed),
                    gladiator_delayed_cue_answer(suit),
                ));
                entries.extend(cue_placements(&delayed));

                // --- RHO acts over our 1NT before advancer can bid Gladiator ---

                // (X): a doubled 1NT always wants a runout, and Gladiator cannot
                // borrow the graft's — turning off `systems_on_overcall_strip`
                // leaves the floor reading an auction it was never distilled on.
                // Author it (see `gladiator_doubled_runout`).
                entries.extend(rows_of(
                    Pattern::node(&format!("P* ({opening}) 1NT (X)")),
                    gladiator_doubled_runout(suit),
                ));

                // (2♣): systems on, but it is Gladiator.  2♣ steals no room — every
                // other advance still sits above it — so only the 2♣ relay is
                // consumed, reappearing as X.  Rebase (their 2♣ → pass, our X → the
                // relay) routes every continuation onto the uncontested Gladiator
                // tree above; the transplant hands X a finite logit to be chosen.
                let relay_call = call(2, Strain::Clubs);
                entries.push(rebase(
                    Pattern::first(&format!("P* ({opening}) 1NT"), "2♣"),
                    described_rewrite(
                        "systems on: their 2♣ is treated as a pass; X asks as the stolen Gladiator relay",
                        rewriter(move |auction: &[Call], depth: usize| {
                            if auction.get(depth) != Some(&relay_call) {
                                return None;
                            }
                            let mut rewritten = auction.to_vec();
                            rewritten[depth] = Call::Pass; // (2♣) steals no room → systems on
                            if auction.get(depth + 1) == Some(&Call::Double) {
                                rewritten[depth + 1] = relay_call; // stolen relay = Double
                            }
                            Some(rewritten)
                        }),
                    ),
                ));
                // The rebase routes continuations; hand advancer a finite logit on
                // Double so it can *choose* the stolen relay (2♣ is illegal here).
                let advances = gladiator_advances(suit);
                entries.push(classified(
                    Pattern::table(&format!("P* ({opening}) 1NT (2♣)")),
                    classifier(move |hand: Hand, context: &Context<'_>| {
                        let mut logits = advances.classify(hand, context);
                        let relay = *logits.0.get(relay_call);
                        *logits.0.get_mut(relay_call) = f32::NEG_INFINITY; // 2♣ is stolen
                        *logits.0.get_mut(Call::Double) = relay; // X inherits the relay
                        logits
                    }),
                ));
            }
            entries
        },
    }
}

/// Gladiator's contested advance: their 2-level action over our 1NT overcall
///
/// No room for the `2♣` relay tree, so the partnership plays its Transfer
/// Lebensohl as if partner had opened 1NT.  `gate_4333 = true`: the overcaller is
/// balanced like a 1NT opener.  Reading is free via the builders' alerts; RHO's
/// 3-level+ interference falls to the floor.
fn gladiator_sohl_package() -> Package {
    Package {
        name: "gladiator-sohl",
        gate: nt_overcall_gladiator,
        entries: || {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(1, Strain::from(major));
                for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    let overcall = Bid::new(2, Strain::from(over));
                    entries.extend(sohl_rows_over(
                        &format!("P* ({opening}) 1NT ({overcall})"),
                        over,
                        LebensohlStyle::Transfer,
                        true,
                    ));
                }
            }
            entries
        },
    }
}

/// Advancer's **Gladiator** actions after `[1M, 1NT, P]` (our 15–18 1NT overcall
/// of their major `M`); `O` is the one unbid major
///
/// `2♣` = weak relay (any suit) → forced `2♦`, pass-or-correct; cue of `M` =
/// Stayman for `O` (exactly 4, INV+); `2♦`/`2O` = natural 5-card INV; `2NT` = NF
/// INV clubs; `3♣`/`3♦`/`3O` = GF 5+; `3M` = splinter (0–1 M, 4 O, GF); `4O` = to
/// play; `4♣`/`4♦`/`4M` = Leaping Michaels (5-5 GF two-suiters).  Points are
/// advancer values opposite a strong NT: INV ≈ 8–9, GF ≈ 10+.
fn gladiator_advances(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    let inv = 8u8;
    let game = 10u8;

    Rules::new()
        // Leaping Michaels: 5-5 game-forcing two-suiters (O + a minor, or both
        // minors via the jump in their suit).
        .rule(
            Bid::new(4, Strain::Clubs),
            150,
            len(o, 5..) & len(Suit::Clubs, 5..) & points(game..),
        )
        .alert(LEAPING)
        .rule(
            Bid::new(4, Strain::Diamonds),
            150,
            len(o, 5..) & len(Suit::Diamonds, 5..) & points(game..),
        )
        .alert(LEAPING)
        .rule(
            Bid::new(4, m),
            150,
            len(Suit::Diamonds, 5..) & len(Suit::Clubs, 5..) & points(game..),
        )
        .alert(LEAPING)
        // Splinter: game-forcing raise of O with a singleton/void in their major.
        .rule(
            Bid::new(3, m),
            145,
            len(o, 4..) & len(their_major, ..=1) & points(game..),
        )
        .alert(GLADIATOR_SPLINTER)
        // To-play game with a long other major (6-card O invites route through
        // the relay, so this is a game-values jump).
        .rule(Bid::new(4, os), 135, len(o, 6..) & points(game..))
        // Cue = Stayman for the unbid major: exactly 4, invitational-or-better.
        // A flat (4333) is barred (the 4333 curse): with no doubleton it has no
        // ruffing value, so a 4-4 major fit does not beat 3NT — it invites in NT.
        .rule(
            Bid::new(2, m),
            140,
            len(o, 4..=4) & points(inv..) & !flat_4333(),
        )
        .alert(GLADIATOR_STAYMAN)
        // Game-forcing naturals: 3 of a real 5+ suit.
        .rule(
            Bid::new(3, Strain::Clubs),
            130,
            len(Suit::Clubs, 5..) & points(game..),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            130,
            len(Suit::Diamonds, 5..) & points(game..),
        )
        .rule(Bid::new(3, os), 130, len(o, 5..) & points(game..))
        // Balanced game, to play.
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            balanced() & points(game..),
        )
        // 2NT = weak transfer to clubs (6+♣): a weak long-club hand signs off in
        // 3♣ (invitational clubs go through the relay to 3♣ instead).
        .rule(
            Bid::new(2, Strain::Notrump),
            105,
            len(Suit::Clubs, 6..) & points(..inv),
        )
        .alert(GLADIATOR_CLUB_TRANSFER)
        // Natural invitational, exactly 5 (6-card invites route through the relay).
        .rule(
            Bid::new(2, Strain::Diamonds),
            100,
            len(Suit::Diamonds, 5..=5) & points(inv..game),
        )
        .rule(Bid::new(2, os), 100, len(o, 5..=5) & points(inv..game))
        // 2♣ = Gladiator relay (XYZ-style): a weak ♦/O takeout, any invitational
        // hand not shown directly, or a game-forcing non-flat hand with exactly 3
        // `O` that wants to check the 5-3 fit via the delayed cue — the forced 2♦
        // then sorts them.  A flat/short weak hand passes 1NT (the Pass catch-all).
        .rule(
            Bid::new(2, Strain::Clubs),
            50,
            (points(..inv) & (len(Suit::Diamonds, 5..) | len(o, 5..)))
                | points(inv..game)
                | (points(game..) & balanced() & len(o, 3..=3) & !flat_4333()),
        )
        .alert(GLADIATOR_RELAY)
        .rule(Call::Pass, 30, hcp(0..))
}

/// Overcaller's reply to the Gladiator cue (advancer showed exactly 4 `O`, INV+)
///
/// User-locked schema: cheapest `O` = MIN fit (15–16), jump `O` = MAX fit
/// (17–18); `2NT` = MIN misfit, `3NT` = MAX misfit.  Jumping to game opposite a
/// maximum fit is safe — the cue is INV+, so advancer is never broke.
fn gladiator_cue_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let m = Strain::from(their_major);
    let os = Strain::from(o);
    let cheap = if os > m { 2 } else { 3 };

    Rules::new()
        .rule(Bid::new(cheap, os), 140, len(o, 4..) & hcp(15..=16))
        .rule(Bid::new(cheap + 1, os), 140, len(o, 4..) & hcp(17..=18))
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            len(o, ..=3) & hcp(15..=16),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            len(o, ..=3) & hcp(17..=18),
        )
        // Finite catch-all (the overcall is a known 15–18, so the four above
        // already partition it).
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Overcaller's reply to the Gladiator **delayed** cue (advancer showed exactly 3
/// `O`, INV+, not flat — checking the 5-3 fit)
///
/// Same min/max × fit/misfit schema as [`gladiator_cue_answer`], but "fit" now
/// means a 5-card `O` (opposite advancer's exactly 3) rather than 4: cheapest `O`
/// = MIN fit (15–16 + 5 `O`), jump `O` = MAX fit (17–18 + 5 `O`); `2NT` = MIN
/// misfit, `3NT` = MAX misfit.  Advancer then places via the same
/// [`gladiator_cue_min_fit`] / [`gladiator_cue_min_misfit`] logic (GF→game,
/// INV→pass).
fn gladiator_delayed_cue_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    let m = Strain::from(their_major);
    let cheap = if os > m { 2 } else { 3 };

    Rules::new()
        .rule(Bid::new(cheap, os), 140, len(o, 5..) & hcp(15..=16))
        .rule(Bid::new(cheap + 1, os), 140, len(o, 5..) & hcp(17..=18))
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            len(o, ..=4) & hcp(15..=16),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            len(o, ..=4) & hcp(17..=18),
        )
        // Finite catch-all (the overcall is a known 15–18).
        .rule(Bid::new(3, Strain::Notrump), 100, hcp(0..))
}

/// Overcaller's forced completion of the Gladiator `2♣` relay
///
/// ponytail: pure `2♦` puppet; the max-break rebids (`2♥`/`2♠` showing a
/// maximum) are deferred — rare, and the advancer's own invitational
/// continuations carry the strength.
fn gladiator_relay_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, hcp(0..))
        .alert(GLADIATOR_RELAY_PC)
}

/// Advancer's continuation over the forced `2♦` (the XYZ-style sort)
///
/// Weak hands sign off (pass `2♦`, or `2O` with 5+ `O`); invitational hands show
/// a 6-card suit at the 3-level (`3♣`/`3♦`/`3O`) or bid `2NT` (balanced).  The
/// **delayed cue** (cue of their major) is exactly 3 `O`, INV+, not flat (4333) —
/// the 5-3-fit check that pairs with a 5-card-major overcall (see
/// [`GLADIATOR_DELAYED_CUE`]); a flat 4333 invites in notrump instead.
fn gladiator_relay_continuation(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    let m = Strain::from(their_major);
    let inv = 8u8;
    let game = 10u8;

    Rules::new()
        // Delayed cue = exactly 3 `O`, INV+, not flat (4333): checks the 5-3 major
        // fit that a 5-card-major 1NT overcall can hold (the direct cue promises 4;
        // a flat 4333 has no ruffing value and invites in notrump).
        .rule(
            Bid::new(2, m),
            100,
            len(o, 3..=3) & points(inv..) & !flat_4333(),
        )
        .alert(GLADIATOR_DELAYED_CUE)
        // Invitational, a 6-card suit.
        .rule(
            Bid::new(3, Strain::Clubs),
            95,
            len(Suit::Clubs, 6..) & points(inv..game),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            95,
            len(Suit::Diamonds, 6..) & points(inv..game),
        )
        .rule(Bid::new(3, os), 95, len(o, 6..) & points(inv..game))
        // Weak takeout: 5+ `O` to `2O`.
        .rule(Bid::new(2, os), 90, len(o, 5..) & points(..inv))
        // Invitational, balanced (no 6-card suit).
        .rule(Bid::new(2, Strain::Notrump), 85, points(inv..game))
        // Weak, diamond tolerance (or nothing better) — pass the puppet.
        .rule(Call::Pass, 50, hcp(0..))
}

/// Overcaller's reply to a natural invitational `2♦` (advancer 5+♦, INV ≈ 8–9)
///
/// A maximum accepts to `3NT` (diamonds a running source); a minimum passes the
/// diamond partscore.
fn gladiator_inv_diamond_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 130, hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to a natural invitational `2O` (advancer 5+ `O`, INV)
///
/// A three-card fit plus a maximum bids the `O` game; a maximum without a fit
/// tries `3NT`; a minimum passes the partscore.
fn gladiator_inv_major_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(4, os), 140, len(o, 3..) & hcp(17..))
        .rule(Bid::new(3, Strain::Notrump), 120, len(o, ..3) & hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller completes the `2NT` weak club transfer — forced `3♣`
fn gladiator_club_transfer_rebid() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Clubs), 100, hcp(0..))
}

/// Overcaller's reply to an invitational relay rebid (`2NT` balanced, or a
/// 6-card `3♣`/`3♦`): max accepts `3NT`, min passes the partscore.
fn gladiator_relay_inv_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 130, hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to an invitational 6-card-`O` relay rebid (`3O`): a fit
/// plus a max bids `4O`; a max without a fit tries `3NT`; a min passes.
fn gladiator_relay_major_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(4, os), 140, len(o, 3..) & hcp(17..))
        .rule(Bid::new(3, Strain::Notrump), 120, len(o, ..3) & hcp(17..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to a game-forcing `3O` or the `3M` splinter (advancer 4+
/// `O`, GF)
///
/// A three-card fit bids the `O` game; otherwise `3NT`.  The splinter shares this
/// — same raise, plus shortness in their major.
fn gladiator_gf_major_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new().rule(Bid::new(4, os), 140, len(o, 3..)).rule(
        Bid::new(3, Strain::Notrump),
        120,
        hcp(0..),
    )
}

/// Overcaller's reply to a game-forcing minor `3♣`/`3♦` — game-forced to `3NT`
fn gladiator_gf_minor_answer() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Notrump), 120, hcp(0..))
}

/// Overcaller's reply to the weak `2O` signoff off the relay
/// (`[2♣, P, 2♦, P, 2O]` — advancer 5+ `O`, under invitational)
///
/// Advancer took the relay to *run*, not to invite: it has denied invitational
/// values by not rebidding `2NT`/`3X`/the cue.  Pass, unless a maximum with real
/// support wants one more — `3O` on four trumps and 18, where nine trumps and
/// 22-plus points make the partscore push sound.  Unauthored, the floor read the
/// signoff as a free bid and raised on **three** trumps, or bid `3NT` opposite a
/// hand that had just denied 8 points.
fn gladiator_relay_signoff_answer(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(3, os), 120, len(o, 4..) & hcp(18..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Overcaller's reply to Leaping Michaels (`4♣`/`4♦` = 5-5 `O` + that minor;
/// `4M` = 5-5 both minors — both game-forcing)
///
/// `shown` is the minor the jump named, [`None`] for the both-minors `4M`.  The
/// auction is already past `3NT`, so there is no notrump landing and the only
/// question is which known fit to take: three-card support for `O` plays the
/// major game, otherwise five of the minor (the longer one when the jump showed
/// both).  Unauthored, the floor answered `4♣` with **`5NT`**.
fn gladiator_leaping_answer(their_major: Suit, shown: Option<Suit>) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    match shown {
        Some(minor) => Rules::new().rule(Bid::new(4, os), 140, len(o, 3..)).rule(
            Bid::new(5, Strain::from(minor)),
            120,
            hcp(0..),
        ),
        None => Rules::new()
            .rule(
                Bid::new(5, Strain::Diamonds),
                120,
                at_least_as_long(Suit::Diamonds, Suit::Clubs),
            )
            .rule(
                Bid::new(5, Strain::Clubs),
                120,
                longer_suit(Suit::Clubs, Suit::Diamonds),
            )
            // Finite catch-all: the two above already partition, but a table
            // that can reject a hand falls through to the floor.
            .rule(Bid::new(5, Strain::Clubs), 50, hcp(0..)),
    }
}

/// Advancer places the contract after the cue-answer showed a MIN fit (cheapest
/// `O`, 15–16 + 4 `O`): game-forcing values raise to `4O`, invitational pass.
fn gladiator_cue_min_fit(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new()
        .rule(Bid::new(4, os), 130, points(10..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Advancer after a MAX fit shown below game (jump `O` = `3O` over `1♥`): the max
/// fit forces game, so raise to `4O` with everything.
fn gladiator_cue_max_fit_raise(their_major: Suit) -> Rules {
    let o = other_major(their_major);
    let os = Strain::from(o);
    Rules::new().rule(Bid::new(4, os), 130, hcp(0..))
}

/// Advancer after a MIN misfit (`2NT`, 15–16 + ≤3 `O`): GF → `3NT`, INV → pass
fn gladiator_cue_min_misfit() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 130, points(10..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// Advancer's runout when RHO doubles our 1NT overcall (`[1M, 1NT, X]`)
///
/// A doubled 1NT always wants a runout.  The systems-on graft gets one for
/// free: [`systems_on_overcall_strip`][crate::bidding] deletes their opening, the
/// auction reads as an opening 1NT, and the deterministic floor's
/// `responder_one_nt_runout` rules fire on a well-formed picture.  Gladiator
/// turns that strip off — its advances differ, so the strip identity no longer
/// holds — and the distilled net, fed the unstripped auction, escaped to the
/// *three* level on a bust (`8732.932.J973.T4` bid `3♥` doubled).  A finite book
/// node shadows the floor, so author the house card here instead.
///
/// `XX` = values, play `1NT××`; otherwise run to a five-plus suit, the longer
/// the better.  **Never into their major** — our side bidding their suit reads
/// as a cue, and running into the suit they opened is the worst landing on the
/// board.  A bust with no other five-bagger sits.
fn gladiator_doubled_runout(their_major: Suit) -> Rules {
    // Matches the floor's `set_runout_xx_min` default: below it we run, at it
    // or above we sit for the redouble.
    let xx_min = 7;
    let mut rules = Rules::new().rule(Call::Redouble, 120, hcp(xx_min..));
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == their_major {
            continue;
        }
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            5
        } else {
            0
        };
        rules = rules
            .rule(
                Bid::new(2, strain),
                100 + major_bonus,
                len(suit, 5..) & hcp(..xx_min),
            )
            .rule(
                Bid::new(2, strain),
                110 + major_bonus,
                len(suit, 6..) & hcp(..xx_min),
            );
    }
    rules.rule(Call::Pass, 30, hcp(0..))
}

/// Advancer's response to partner's Michaels cue-bid over their opening `t`
fn michaels_advances(t: Suit) -> Rules {
    match t {
        // Partner shows both majors: prefer the longer one.
        Suit::Clubs | Suit::Diamonds => {
            let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
            let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Hearts),
                    130,
                    points(10..) & len(Suit::Hearts, 3..) & hearts_longer.clone(),
                )
                .rule(
                    Bid::new(4, Strain::Spades),
                    130,
                    points(10..) & len(Suit::Spades, 3..) & spades_longer.clone(),
                )
                .rule(Bid::new(2, Strain::Hearts), 100, hearts_longer)
                .rule(Bid::new(2, Strain::Spades), 100, spades_longer)
        }
        // Partner shows spades + a minor: bid spades.
        Suit::Hearts => Rules::new()
            .rule(
                Bid::new(4, Strain::Spades),
                130,
                points(10..) & len(Suit::Spades, 3..),
            )
            .rule(Bid::new(2, Strain::Spades), 50, hcp(0..)),
        // Partner shows hearts + a minor: bid hearts.
        Suit::Spades => Rules::new()
            .rule(
                Bid::new(4, Strain::Hearts),
                130,
                points(10..) & len(Suit::Hearts, 3..),
            )
            .rule(Bid::new(3, Strain::Hearts), 50, hcp(0..)),
    }
}

/// Advancer's response to partner's Leaping Michaels jump over their weak two
///
/// `theirs` is the suit they opened; `lm` is the suit of the jump (Clubs or
/// Diamonds).  The overcall is game-forcing, so every advance reaches game.
/// - Over a **major**, the jump names `lm` plus the *other* major: bid that
///   major game with a fit, else the `lm` minor game.
/// - Over **2♦**, the `4♦` *cue* shows both majors → pick the longer; the `4♣`
///   jump shows clubs + an unknown major → `5♣` with a club fit and no major,
///   else `4♥` pass-or-correct (see [`leaping_michaels_2d_4c_rebid`]).
fn leaping_michaels_advances(theirs: Suit, lm: Suit) -> Rules {
    match theirs {
        // Over a major: lm + the OTHER major, both known.
        Suit::Hearts | Suit::Spades => {
            let major = if theirs == Suit::Hearts {
                Suit::Spades
            } else {
                Suit::Hearts
            };
            // Prefer the major game even on a doubleton (a 7-card fit) — it
            // scores well and needs only ten tricks; retreat to the 5m game only
            // on a genuine major misfit (≤1), where DD has to make eleven.
            Rules::new()
                .rule(Bid::new(4, Strain::from(major)), 130, len(major, 2..))
                .rule(Bid::new(5, Strain::from(lm)), 120, len(major, 0..=1))
        }
        // Over 2♦.
        Suit::Diamonds => match lm {
            // 4♦ cue = both majors: pick the longer (both forced to game).
            Suit::Diamonds => {
                let hearts_longer = at_least_as_long(Suit::Hearts, Suit::Spades);
                let spades_longer = longer_suit(Suit::Spades, Suit::Hearts);
                Rules::new()
                    .rule(Bid::new(4, Strain::Hearts), 130, hearts_longer)
                    .rule(Bid::new(4, Strain::Spades), 130, spades_longer)
            }
            // 4♣ = clubs + a major: 5♣ with a club fit and no major, else 4♥
            // pass-or-correct (partner names their major).
            Suit::Clubs => Rules::new()
                .rule(
                    Bid::new(5, Strain::Clubs),
                    120,
                    len(Suit::Clubs, 3..) & len(Suit::Hearts, 0..=2) & len(Suit::Spades, 0..=2),
                )
                .rule(Bid::new(4, Strain::Hearts), 130, hcp(0..)),
            _ => unreachable!("a Leaping Michaels jump is clubs or diamonds"),
        },
        Suit::Clubs => unreachable!("there is no weak 2♣ opening"),
    }
}

/// Overcaller's rebid after `(2♦)–4♣–(P)–4♥–(P)`: pass-or-correct to their major
///
/// `4♣` over `2♦` showed clubs + a major; advancer's `4♥` is pass-or-correct, so
/// the overcaller passes with hearts or corrects to `4♠` with spades.
fn leaping_michaels_2d_4c_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 130, len(Suit::Spades, 5..))
        .rule(Call::Pass, 100, hcp(0..))
}

/// The two suits shown by an Unusual 2NT over their opening `t`
///
/// Returns `(a, b)` where `a < b` (lower suit first).
const fn unusual_suits(t: Suit) -> (Suit, Suit) {
    match t {
        Suit::Clubs => (Suit::Diamonds, Suit::Hearts),
        Suit::Diamonds => (Suit::Clubs, Suit::Hearts),
        Suit::Hearts | Suit::Spades => (Suit::Clubs, Suit::Diamonds),
    }
}

/// Advancer's response to partner's Unusual 2NT over their opening `t`
fn unusual_nt_advances(t: Suit) -> Rules {
    let (a, b) = unusual_suits(t);
    let a_longer = at_least_as_long(a, b);
    let b_longer = longer_suit(b, a);
    Rules::new()
        .rule(Bid::new(3, Strain::from(a)), 100, a_longer)
        .rule(Bid::new(3, Strain::from(b)), 100, b_longer)
}

// ---------------------------------------------------------------------------
// Responsive doubles
// ---------------------------------------------------------------------------

/// Advancer's action when partner made a takeout double and they raised `t` to `raise_lvl`
///
/// Responsive double: both suits of the rank opposite the opened suit (minor/major).
/// Natural bids at the minimum legal level (2–3) for suits other than `t`, 5-card, 8+ HCP.
fn responsive_doubles(t: Suit, _raise_lvl: u8) -> Rules {
    // Responsive double shows the two unbid suits of the same rank (minor or major).
    let mut rules = if matches!(t, Suit::Hearts | Suit::Spades) {
        // t major → both minors
        Rules::new()
            .rule(
                Call::Double,
                150,
                len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..) & points(8..),
            )
            .alert(RESPONSIVE)
    } else {
        // t minor → both majors
        Rules::new()
            .rule(
                Call::Double,
                150,
                len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & points(8..),
            )
            .alert(RESPONSIVE)
    };

    rules = rules.rule(Call::Pass, 0, hcp(0..));

    // Natural bids for suits ≠ t at levels 2 and 3.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t {
            continue;
        }
        let strain = Strain::from(suit);
        for bid_lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(bid_lvl, strain),
                100,
                min_level_is(bid_lvl, strain) & len(suit, 5..) & points(8..),
            );
        }
    }
    rules
}

/// Advancer's responsive double after partner *overcalled* `overcall` over their
/// `open`, and they raised (`(1t)–overcall–(2t)–?`)
///
/// A single-rule node: a `Call::Double` showing the two suits unbid by opener and
/// partner (all four minus `{open, overcall}`), 4+ in each, 8+ points.  By design it
/// has **no** catch-all — a hand that does not qualify gets all `-∞` logits and falls
/// through to the instinct floor's natural advances (mass-aware shadowing,
/// [`Trie::classify_floored`]), so this *layers* a responsive double onto the floor
/// rather than replacing it.  `Double` is always legal here (the opponents have a live
/// contract), so the lone rule cannot trip the silent-pass trap.
//
// ponytail: faithful reconstruction of the never-committed "8+ floor double" (ledger
// #100); off by default, the A/B knob for `examples/responsive-ab --conv overcall`.
fn responsive_overcall_doubles(open: Suit, overcall: Suit, _raise_lvl: u8) -> Rules {
    let mut unbid = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .filter(|&s| s != open && s != overcall);
    let s1 = unbid.next().expect("two suits remain unbid");
    let s2 = unbid.next().expect("two suits remain unbid");
    Rules::new()
        .rule(Call::Double, 150, len(s1, 4..) & len(s2, 4..) & points(8..))
        .alert(RESPONSIVE)
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Defense to their `1NT` opening, direct and balancing seat
///
/// The balancing entry reuses the direct ranges so `(1NT) P P ?` no longer
/// falls to the instinct floor's undisciplined balancing doubles; a lighter
/// balancing-specific range is a later refinement.  See
/// `set_notrump_balancing`.
fn notrump_defense_package() -> Package {
    Package {
        name: "notrump-defense",
        gate: || true,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* (1NT)"), defense_to_notrump());
            if notrump_balancing_enabled() {
                entries.extend(rows_of(
                    Pattern::node("P* (1NT) P (P)"),
                    defense_to_notrump(),
                ));
            }
            entries
        },
    }
}

/// Defense to their `2♣` Stayman: `X` = lead-directing clubs, natural
/// overcalls, Unusual `2NT`, natural `3♣` preempt (`set_stayman_defense`)
fn their_stayman_defense_package() -> Package {
    Package {
        name: "their-stayman-defense",
        gate: stayman_defense_enabled,
        entries: || rows_of(Pattern::node("P* (1NT) P (2♣)"), defense_to_their_stayman()),
    }
}

/// Defense to their Jacoby transfers: `X` = lead-directing the bid suit, the
/// cue is Michaels (the other major plus a minor), plus natural overcalls
/// (`set_transfer_defense`)
fn their_transfer_defense_package() -> Package {
    Package {
        name: "their-transfer-defense",
        gate: transfer_defense_enabled,
        entries: || {
            [(Suit::Diamonds, Suit::Hearts), (Suit::Hearts, Suit::Spades)]
                .into_iter()
                .flat_map(|(resp, shown)| {
                    let response = Bid::new(2, Strain::from(resp));
                    rows_of(
                        Pattern::node(&format!("P* (1NT) P ({response})")),
                        defense_to_their_transfer(resp, shown),
                    )
                })
                .collect()
        },
    }
}

/// Defense to their two-way `2♠` minor response: `X` = lead-directing spades,
/// `2NT` = the red two-suiter, `3♣` cue = top-and-bottom, natural `3♦`/`3♥`
/// overcalls (`set_minor_transfer_defense`)
fn their_minor_transfer_defense_package() -> Package {
    Package {
        name: "their-minor-transfer-defense",
        gate: minor_transfer_defense_enabled,
        entries: || {
            rows_of(
                Pattern::node("P* (1NT) P (2♠)"),
                defense_to_their_minor_transfer(),
            )
        },
    }
}

/// Defense to their `2NT` diamond transfer: `X` = lead-directing diamonds,
/// `3♦` cue = both majors, natural `3♣`/`3♥`/`3♠` overcalls
/// (`set_diamond_transfer_defense`)
fn their_diamond_transfer_defense_package() -> Package {
    Package {
        name: "their-diamond-transfer-defense",
        gate: diamond_transfer_defense_enabled,
        entries: || {
            rows_of(
                Pattern::node("P* (1NT) P (2NT)"),
                defense_to_their_diamond_transfer(),
            )
        },
    }
}

/// Advancing partner's both-minors `2NT` over their `1NT`
///
/// Doubled we never sit — sitting in `2NT`-doubled is a loser, the doubler has
/// values behind a 15-17 `1NT` — so both entries just pick the longer minor.
fn unusual_notrump_advance_package() -> Package {
    Package {
        name: "unusual-notrump-advance",
        gate: || unusual_notrump_range().is_some(),
        entries: || {
            let mut entries = rows_of(
                Pattern::node("P* (1NT) 2NT (P)"),
                unusual_nt_advances(Suit::Spades),
            );
            entries.extend(rows_of(
                Pattern::node("P* (1NT) 2NT (X)"),
                unusual_nt_advances(Suit::Spades),
            ));
            entries
        },
    }
}

/// Direct-seat DONT advances: the same pass-or-correct relays, keyed at
/// *every* seat (the `X`/`2♣`/`2♦`/`2♥` are direct-seat conventional calls)
///
/// Binding `(1NT) X (P)` is correct here — with DONT on, the direct `X` is a
/// one-suiter wanting the `2♣` relay, not a penalty.  Every artificial leg
/// carries a doubled/redoubled escape so we never sit in `1NT`-redoubled or a
/// doubled misfit `2♣`, the dominant DONT-`X` loss in the honest measure.
fn direct_dont_advance_package() -> Package {
    Package {
        name: "direct-dont-advance",
        gate: direct_dont_enabled,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* (1NT) X (P)"), passed_dont_x_advance());
            for (key, rules) in [
                ("P* (1NT) X (P) 2♣ (P)", passed_dont_x_rebid()),
                ("P* (1NT) 2♣ (P)", passed_dont_2c_advance()),
                ("P* (1NT) 2♣ (P) 2♦ (P)", passed_dont_2c_rebid()),
                ("P* (1NT) 2♦ (P)", passed_dont_2d_advance()),
                ("P* (1NT) 2♦ (P) 2♥ (P)", passed_dont_2d_rebid()),
                ("P* (1NT) 2♥ (P)", passed_dont_2h_advance()),
                // Their redouble of our one-suiter X: never sit in 1NTxx — relay
                // 2♣ just as over a pass, then the doubler names the suit.
                ("P* (1NT) X (XX)", passed_dont_x_advance()),
                ("P* (1NT) X (XX) 2♣ (P)", passed_dont_x_rebid()),
                // Their double of our artificial 2♣ relay (after our X, passed or
                // redoubled): the relay is NOT a club fit, so the doubler must
                // still name the real one-suiter (or pass with genuine clubs).
                ("P* (1NT) X (P) 2♣ (X)", passed_dont_x_rebid()),
                ("P* (1NT) X (XX) 2♣ (X)", passed_dont_x_rebid()),
            ] {
                entries.extend(rows_of(Pattern::node(key), rules));
            }
            entries
        },
    }
}

/// Direct-seat Meckwell advances: the `X` is a two-way "single 6+ minor OR
/// both majors" double
///
/// Advancer relays `2♣` (pass-or-correct); the doubler passes with clubs,
/// names `2♦` with diamonds, or bids `2♥` (4+ hearts ⇒ both majors here) and
/// the advancer passes or corrects to `2♠`.  The minor+major `2♣`/`2♦` reuse
/// the DONT pass-or-correct advances (the same "name your higher suit"
/// relay).  Every artificial leg has a doubled/redoubled escape.
fn meckwell_advance_package() -> Package {
    Package {
        name: "meckwell-advance",
        gate: meckwell_enabled,
        entries: || {
            let mut entries = rows_of(Pattern::node("P* (1NT) X (P)"), meckwell_x_advance());
            for (key, rules) in [
                ("P* (1NT) X (P) 2♣ (P)", meckwell_x_rebid()),
                ("P* (1NT) X (P) 2♣ (P) 2♥ (P)", passed_dont_2h_advance()),
                // 2♣/2♦ minor+major: reuse the DONT pass-or-correct advances.
                ("P* (1NT) 2♣ (P)", passed_dont_2c_advance()),
                ("P* (1NT) 2♣ (P) 2♦ (P)", passed_dont_2c_rebid()),
                ("P* (1NT) 2♦ (P)", passed_dont_2d_advance()),
                ("P* (1NT) 2♦ (P) 2♥ (P)", passed_dont_2d_rebid()),
                // Their redouble of our X: relay 2♣ anyway (never sit 1NTxx).
                ("P* (1NT) X (XX)", meckwell_x_advance()),
                ("P* (1NT) X (XX) 2♣ (P)", meckwell_x_rebid()),
                // Their double of our artificial 2♣ relay: the doubler still names
                // the real suit (pass only with genuine clubs), else runs.
                ("P* (1NT) X (P) 2♣ (X)", meckwell_x_rebid()),
                ("P* (1NT) X (XX) 2♣ (X)", meckwell_x_rebid()),
                // Their double of the doubler's both-majors 2♥ show: advancer
                // still picks a major.
                ("P* (1NT) X (P) 2♣ (P) 2♥ (X)", passed_dont_2h_advance()),
            ] {
                entries.extend(rows_of(Pattern::node(key), rules));
            }
            entries
        },
    }
}

/// Over each one-of-a-suit opening: our direct defense, and the advances of
/// partner's Michaels cue and Unusual `2NT`
///
/// All three key sets are disjoint from every other write in
/// [`defensive`] — `[1t]`, `[1t, 2t, P]` and `[1t, 2NT, P]` — so they lift out
/// of the per-suit loop without changing what lands where.
fn suit_defense_package() -> Package {
    Package {
        name: "suit-defense",
        gate: || true,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                let key = format!("P* ({opening})");
                entries.extend(rows_of(Pattern::node(&key), defense_to_suit(opening)));
                entries.extend(rows_of(
                    Pattern::node(&format!("{key} 2{theirs} (P)")),
                    michaels_advances(suit),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{key} 2NT (P)")),
                    unusual_nt_advances(suit),
                ));
            }
            entries
        },
    }
}

/// Advancing partner's Landy `2♣` (both majors) over their `1NT`
///
/// Woolsey's `2♣` is the identical both-majors call on the same shared band,
/// so it reuses this same advance wiring.  The undoubled branch runs the
/// `2♦` "pick a major" relay and the `2NT` game-ask; over their double the
/// advancer runs the richer escape (Redouble = equal-majors relay, Pass =
/// clubs, `2♦` = natural, `2♥`/`2♠` = the longer major).
fn landy_advance_package() -> Package {
    Package {
        name: "landy-advance",
        gate: || landy_range().is_some() || woolsey_enabled(),
        entries: || {
            let (lo, hi) = woolsey_points();
            [
                ("P* (1NT) 2♣ (P)", landy_advances(lo)),
                ("P* (1NT) 2♣ (P) 2♦ (P)", landy_2d_rebid()),
                ("P* (1NT) 2♣ (P) 2NT (P)", landy_2nt_rebid(lo, hi)),
                ("P* (1NT) 2♣ (X)", landy_advances_over_double(lo)),
                ("P* (1NT) 2♣ (X) XX (P)", landy_2d_rebid()),
                ("P* (1NT) 2♣ (X) 2♦ (P)", landy_doubled_2d_rebid()),
                ("P* (1NT) 2♣ (X) 2NT (P)", landy_2nt_rebid(lo, hi)),
            ]
            .into_iter()
            .flat_map(|(key, rules)| rows_of(Pattern::node(key), rules))
            .collect()
        },
    }
}

/// Woolsey "Multi-Landy" continuations ([`NotrumpDefense::Woolsey`])
///
/// Authored in full — the both-majors `2♣` reuses [`landy_advance_package`]'s
/// wiring.  Every artificial call carries its doubled / redoubled escape so the
/// opponents can never trap us in a doubled artificial contract.
fn woolsey_package() -> Package {
    Package {
        name: "woolsey",
        gate: woolsey_enabled,
        entries: || {
            let lo = woolsey_points().0;
            let mut entries = Vec::new();

            // Multi 2♦.  The advance is the same over a pass or a double (it never
            // sits 2♦x — the overcaller has a major, not diamonds).  `rho` is the
            // opponents' call over our 2♦; `after` is their call over our
            // pass-or-correct — the overcaller names its major regardless of a
            // double, so we are never left to the floor in a doubled 2♥x/2♠x (the
            // dominant 2♦ leak vs BBA).
            for rho in ["P", "X"] {
                let base = format!("P* (1NT) 2♦ ({rho})");
                entries.extend(rows_of(Pattern::node(&base), multi_advances(lo)));
                for after in ["P", "X"] {
                    for (bid, rebid) in [
                        // Weak 2♥ p/c → pass / correct 2♠ / jump 3M with seven.
                        ("2♥", multi_2h_rebid()),
                        // Constructive 2♠ p/c → pass spades / 3♥ with hearts.
                        ("2♠", multi_2s_rebid()),
                        // Game-force 2NT ask → overcaller jumps to game in its major.
                        ("2NT", multi_2nt_rebid()),
                    ] {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{base} {bid} ({after})")),
                            rebid,
                        ));
                    }
                }
            }

            // Muiderberg 2♥/2♠ — raises + the 2NT minor-ask (a doubled escape with
            // no fit).
            for (major, mbid) in [(Suit::Hearts, "2♥"), (Suit::Spades, "2♠")] {
                entries.extend(rows_of(
                    Pattern::node(&format!("P* (1NT) {mbid} (P)")),
                    muiderberg_advances(major, lo),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("P* (1NT) {mbid} (X)")),
                    muiderberg_advances_doubled(major, lo),
                ));
                // The 2NT minor-ask reaches the overcaller over either RHO action.
                for rho in ["P", "X"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) {mbid} ({rho}) 2NT (P)")),
                        muiderberg_2nt_rebid(),
                    ));
                }
            }

            // Takeout X — advancer relays to the minor / bids its own major / asks
            // 2NT.  A redouble forces us to run (never sit 1NTxx): the same advance
            // applies.
            let xfloor = woolsey_double_floor();
            for adv in ["P", "XX"] {
                let base = format!("P* (1NT) X ({adv})");
                entries.extend(rows_of(Pattern::node(&base), woolsey_x_advance(xfloor)));
                // The doubler names its 5-6 minor whether the 2♣ relay is passed or
                // doubled.
                for after in ["P", "X"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} 2♣ ({after})")),
                        woolsey_x_minor_rebid(),
                    ));
                }
                // The 2NT game-ask → the doubler names its 4-card major.
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 2NT (P)")),
                    woolsey_x_2nt_rebid(),
                ));
            }
            entries
        },
    }
}

/// Direct-seat both-majors `X` advances
/// ([`set_direct_landy_double`][super::set_direct_landy_double])
///
/// The `X` is a Landy-style both-majors takeout double at every seat, so the
/// advancer answers exactly as over a Landy `2♣` — binding `[1NT, X, P]` is
/// correct here, the direct `X` is both-majors, not penalty.  The `2♦` relay
/// and the `2NT` ask are artificial, so the doubler answers them whether they
/// are passed **or** doubled; over their redouble the relay is dropped
/// entirely (`Pass` = ask back), which kills the phantom-`3♦` run.
fn both_majors_double_package() -> Package {
    Package {
        name: "both-majors-double",
        gate: || direct_landy_double().is_some(),
        entries: || {
            // The advancer's invite/game thresholds track the X floor (a
            // stronger X asks less of the advancer), so read it here too.
            let (lo, hi) = (direct_landy_double_floor(), 37u8);
            let mut entries = Vec::new();
            for (key, rules) in [
                ("P* (1NT) X (P)", both_majors_x_advance(lo)),
                ("P* (1NT) X (P) 2♦ (P)", landy_2d_rebid()),
                ("P* (1NT) X (P) 2♦ (X)", landy_2d_rebid()),
                ("P* (1NT) X (P) 2NT (P)", landy_2nt_rebid(lo, hi)),
                ("P* (1NT) X (P) 2NT (X)", landy_2nt_rebid(lo, hi)),
                ("P* (1NT) X (XX)", both_majors_x_runout(lo)),
                ("P* (1NT) X (XX) P (P)", landy_2d_rebid()),
            ] {
                entries.extend(rows_of(Pattern::node(key), rules));
            }
            // …then the advancer SITS for that major whether it is passed or
            // doubled — play 2Mx (our real fit), never run.
            for m in ["2♥", "2♠"] {
                for after in ["(P)", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) X (XX) P (P) {m} {after}")),
                        sit(),
                    ));
                }
            }
            // The undoubled branch keeps the 2♦ relay (Pass there defends 1NT,
            // so it cannot be the ask).  Once the doubler names its major over
            // the (possibly doubled) relay, SIT when the opponents double it —
            // the doubler plays 2Mx instead of running to the phantom 3♦.
            for relay in ["(X)", "(P)"] {
                for m in ["2♥", "2♠"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) X (P) 2♦ {relay} {m} (X) P (P)")),
                        sit(),
                    ));
                }
            }
            entries
        },
    }
}

/// Advancing partner's takeout double of a one-of-a-suit opening
///
/// The rich cue + notrump ladder when [`set_rich_advance_double`] is on, else
/// the flat floor ladder; the continuations of the rich ladder's artificial
/// calls live in [`rich_advance_double_package`].
fn advance_double_package() -> Package {
    Package {
        name: "advance-of-double",
        gate: || true,
        entries: || {
            [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(1, Strain::from(suit));
                    let advances = if rich_advance_double_enabled() {
                        advance_double_rich(opening)
                    } else {
                        advance_double(opening)
                    };
                    rows_of(Pattern::node(&format!("P* ({opening}) X (P)")), advances)
                })
                .collect()
        },
    }
}

/// Continuations of the rich advance of partner's takeout double
///
/// Four sub-ladders, each authored for both RHO branches — RHO may pass *or*
/// double our artificial call, and the obligation to answer is the same either
/// way (leaving the doubled branch to the floor lets it pass out an artificial
/// cue):
///
/// * the doubler's answer to the advancer's cue, then the advancer's
///   invite-vs-force clarification over each minimum answer,
/// * the Rubens transfer completion and the advancer's rebid over it
///   ([`set_advance_rubens`]),
/// * the doubler's accept/decline of the invitational minor jump, the
///   advancer's placement over the forcing new suit, and the answer to the
///   stopper-ask cue ([`set_advance_minor_jump`]),
/// * the same accept/decline over the invitational `2NT`
///   ([`set_advance_2nt_continuation`]) — without it the doubler falls to the
///   floor, which passes `2NT` holding a game.
fn rich_advance_double_package() -> Package {
    Package {
        name: "rich-advance-of-double",
        gate: rich_advance_double_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                let base = format!("P* ({opening}) X (P)");

                let cue = Bid::new(2, theirs);
                for rho in ["(P)", "(X)"] {
                    let after_cue = format!("{base} {cue} {rho}");
                    entries.extend(rows_of(
                        Pattern::node(&after_cue),
                        answer_advance_cue(opening),
                    ));
                    for answer in advance_cue_answers(opening) {
                        for rho2 in ["(P)", "(X)"] {
                            entries.extend(rows_of(
                                Pattern::node(&format!("{after_cue} {answer} {rho2}")),
                                advance_cue_rebid(answer),
                            ));
                        }
                    }
                }

                // Rubens transfers: the doubler completes the transfer
                // (declaring), and the advancer raises to game or rests over the
                // completion — so the artificial transfer is never left in.
                if advance_rubens_enabled() {
                    for (bid, target) in advance_major_transfers(theirs) {
                        let completion = Bid::new(3, Strain::from(target));
                        for rho in ["(P)", "(X)"] {
                            let after_xfer = format!("{base} {bid} {rho}");
                            entries.extend(rows_of(
                                Pattern::node(&after_xfer),
                                complete_advance_transfer(target),
                            ));
                            entries.extend(rows_of(
                                Pattern::node(&format!("{after_xfer} {completion} (P)")),
                                advance_transfer_rebid(target),
                            ));
                        }
                    }
                }

                // The natural jump is limited, so — like a `2NT` invite — the
                // doubler passes to decline; only the accepting branches (and the
                // advancer's rebid over them) need authoring.
                if advance_minor_jump_enabled() {
                    for minor in [Suit::Clubs, Suit::Diamonds] {
                        let m = Strain::from(minor);
                        // A three-level minor jump exists only below their suit.
                        if m >= theirs {
                            continue;
                        }
                        let jump = Bid::new(3, m);
                        for rho in ["(P)", "(X)"] {
                            let after_jump = format!("{base} {jump} {rho}");
                            entries.extend(rows_of(
                                Pattern::node(&after_jump),
                                answer_advance_minor_jump(opening, minor),
                            ));
                            // The advancer places game over each forcing new suit
                            // the doubler can show (any unbid suit above the jump).
                            for shown in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                                let s = Strain::from(shown);
                                if s == theirs || s <= m {
                                    continue;
                                }
                                let bid = Bid::new(3, s);
                                for rho2 in ["(P)", "(X)"] {
                                    entries.extend(rows_of(
                                        Pattern::node(&format!("{after_jump} {bid} {rho2}")),
                                        advance_minor_jump_rebid(shown),
                                    ));
                                }
                            }
                            // The advancer answers the doubler's stopper-ask cue
                            // (3 of their suit): 3NT with a stopper (right-sided),
                            // else the minor game.
                            let ask = Bid::new(3, theirs);
                            for rho2 in ["(P)", "(X)"] {
                                entries.extend(rows_of(
                                    Pattern::node(&format!("{after_jump} {ask} {rho2}")),
                                    advance_minor_stopper_ask_answer(minor),
                                ));
                            }
                        }
                    }
                }

                if advance_2nt_continuation_enabled() {
                    for rho in ["(P)", "(X)"] {
                        let after_2nt = format!("{base} 2NT {rho}");
                        entries.extend(rows_of(
                            Pattern::node(&after_2nt),
                            answer_advance_2nt(opening),
                        ));
                        // The advancer places game over each forcing major the
                        // doubler can show (an unbid major at the three level).
                        for major in [Suit::Hearts, Suit::Spades] {
                            let s = Strain::from(major);
                            if s == theirs {
                                continue;
                            }
                            let bid = Bid::new(3, s);
                            for rho2 in ["(P)", "(X)"] {
                                entries.extend(rows_of(
                                    Pattern::node(&format!("{after_2nt} {bid} {rho2}")),
                                    advance_minor_jump_rebid(major),
                                ));
                            }
                        }
                    }
                }
            }
            entries
        },
    }
}

/// Responsive doubles: partner doubled for takeout, they raised
///
/// On by default; the A/B knob (`--conv takeout`) turns it off to compare the
/// shipped node against the bare floor.
fn responsive_double_package() -> Package {
    Package {
        name: "responsive-double",
        gate: responsive_takeout_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                for raise_lvl in [2u8, 3] {
                    let raise = Bid::new(raise_lvl, theirs);
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) X ({raise})")),
                        responsive_doubles(suit, raise_lvl),
                    ));
                }
            }
            entries
        },
    }
}

/// Responsive double after partner's *overcall* and their raise
///
/// Off by default: the auction is otherwise floored.  The A/B knob
/// (`--conv overcall`) turns it on; see [`set_responsive_overcall`].
fn responsive_overcall_package() -> Package {
    Package {
        name: "responsive-double-over-overcall",
        gate: responsive_overcall_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                for over in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if over == suit {
                        continue;
                    }
                    // Partner's natural overcall of `over` at its minimum level
                    // over 1t: the 1-level if it outranks their suit, else the 2.
                    let over_lvl = if over > suit { 1 } else { 2 };
                    let overcall = Bid::new(over_lvl, Strain::from(over));
                    for raise_lvl in [2u8, 3] {
                        let raise = Bid::new(raise_lvl, theirs);
                        entries.extend(rows_of(
                            Pattern::node(&format!("P* ({opening}) {overcall} ({raise})")),
                            responsive_overcall_doubles(suit, over, raise_lvl),
                        ));
                    }
                }
            }
            entries
        },
    }
}

/// Advancing our `2NT` overcall of their weak two ([`set_weak_two_notrump_advances`])
///
/// Majors only — over `2♦` both majors are unbid, so the cue has no Stayman to
/// be.
fn weak_two_notrump_advance_package() -> Package {
    Package {
        name: "weak-two-notrump-advance",
        gate: weak_two_notrump_advances_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(2, Strain::from(suit));
                let base = format!("P* ({opening}) 2NT (P)");
                entries.extend(rows_of(
                    Pattern::node(&base),
                    weak_two_notrump_advances(suit),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 3♣ (P)")),
                    weak_two_notrump_relay_reply(),
                ));
                // The delayed cue is 3♥ over their 2♥ but 3♠ over their 2♠, and
                // 3♠+ is unauthored — so over 2♠ the node would be Pass alone.
                if suit == Suit::Hearts {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} 3♣ (P) 3♦ (P)")),
                        weak_two_notrump_relay_rebid(suit),
                    ));
                }
            }
            entries
        },
    }
}

/// Advances of Leaping Michaels over their weak two
///
/// The jump is below game, so the advancer is forced on (a fit major game, else
/// the `5m` minor game — never a passed `4m` partscore).
fn leaping_michaels_package() -> Package {
    Package {
        name: "leaping-michaels-advance",
        gate: leaping_michaels_enabled,
        entries: || {
            let mut entries = Vec::new();
            for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(2, Strain::from(suit));
                for lm in [Suit::Clubs, Suit::Diamonds] {
                    let jump = Bid::new(4, Strain::from(lm));
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) {jump} (P)")),
                        leaping_michaels_advances(suit, lm),
                    ));
                }
                // Over 2♦, 4♣ shows clubs + an unknown major; advancer's 4♥ is
                // pass-or-correct, so the overcaller names their major in rebid.
                if suit == Suit::Diamonds {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) 4♣ (P) 4♥ (P)")),
                        leaping_michaels_2d_4c_rebid(),
                    ));
                }
            }
            entries
        },
    }
}

/// Build the defensive book: all our actions when the opponents open
///
/// Every package leads with `P*`, the seat fan, so every seat is covered.  A
/// defensive auction string starts from their opening, e.g. `P* (1♦) 2♦ (P)`
/// means they opened 1♦, we cue-bid 2♦ (Michaels), opener's side passed, and we
/// are the advancer.  The one hand-rolled site left is the systems-on graft of
/// the whole opening-1NT book below our 1NT overcall — `compile_into` writes
/// rows, not a subtree.
#[must_use]
pub fn defensive() -> Defensive {
    let mut d = Defensive::new();

    // Systems-on advances of our 1NT overcall: the whole 1NT-opening response
    // structure (Stayman, transfers, Smolen — reflecting the same knobs), built
    // once and grafted below each `[their-suit, 1NT]` so the advancer plays it
    // verbatim.  On by default; see `set_nt_overcall_systems_on`.
    let nt_overcall_book = nt_overcall_systems_on().then(|| {
        let mut nt = Trie::new();
        super::notrump::register_one_nt(&mut nt);
        nt
    });

    // Over each one-of-a-suit opening: our direct defense, and the advances of
    // partner's Michaels cue and Unusual 2NT.
    compile_into(&mut d, &[suit_defense_package()]);

    // Advancing partner's takeout double of a one-of-a-suit opening, and — when
    // the rich ladder is on — the continuations of its artificial calls.
    compile_into(
        &mut d,
        &[advance_double_package(), rich_advance_double_package()],
    );

    // Advances of our 1NT overcall ([1t, 1NT, P]).  Over a MINOR the advancer
    // plays the full opening-1NT structure (Stayman/transfers/Smolen) grafted
    // below `[1t, 1NT]` — `1♦–1NT` equals `1♣–1NT` equals an opening 1NT,
    // transfers preserving right-siding.  Grafted in every seat the opening could
    // have been made (mirrors the overcall's fan).  This is the one permanently
    // imperative site: `compile_into` writes rows, not a whole subtree.
    //
    // Advances of a *natural* overcall ([1t, overcall, Pass]) are left to the
    // instinct floor's Rubens transfers — the programmatic floor expresses the
    // transfer band for every (opening, overcall) pair in one place, where a
    // per-suit authored table cannot.
    if let Some(nt) = &nt_overcall_book {
        let one_nt = call(1, Strain::Notrump);
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            // Over a major the graft's Stayman would look for a major they own;
            // `gladiator_package` replaces it with the geometry that fits.
            if matches!(suit, Suit::Hearts | Suit::Spades) && nt_overcall_gladiator() {
                continue;
            }
            let opening = Bid::new(1, Strain::from(suit));
            for n in 0..=3 {
                let prefix: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
                    .chain([Call::Bid(opening), one_nt])
                    .collect();
                let collisions = d.graft(&prefix, nt, &[one_nt]);
                debug_assert!(
                    collisions.is_empty(),
                    "1NT-overcall systems-on graft collides at {prefix:?}: {collisions:?}"
                );
            }
        }
    }

    // Gladiator, when on: the advances of our 1NT overcall of their major, then
    // the tail for their 2-level action over it.
    compile_into(&mut d, &[gladiator_package(), gladiator_sohl_package()]);

    // Responsive doubles: partner acted (double or overcall) and they raised.
    compile_into(
        &mut d,
        &[responsive_double_package(), responsive_overcall_package()],
    );

    // Over each weak-two opening (the row packages): takeout double, natural
    // overcalls, 2NT; then the advances of our 2NT overcall and of Leaping
    // Michaels.
    compile_into(
        &mut d,
        &[
            weak_two_defense_package(),
            weak_two_notrump_advance_package(),
            leaping_michaels_package(),
        ],
    );
    // Advancing partner's takeout double: [2t, X, P] — advancer to act.
    compile_into(&mut d, &[advance_of_double_package()]);

    // Their 1NT opening and the three artificial responses we have a defense to
    // (Stayman, Jacoby, the two-way 2♠ and the 2NT diamond transfer); all three
    // response defenses are opt-in, default off.
    compile_into(
        &mut d,
        &[
            notrump_defense_package(),
            their_stayman_defense_package(),
            their_transfer_defense_package(),
            their_minor_transfer_defense_package(),
            their_diamond_transfer_defense_package(),
        ],
    );

    // Advancing partner's Landy 2♣ (both majors) over their 1NT, when on.  Woolsey's
    // 2♣ is the identical both-majors call on the same shared band, so it reuses this
    // same advance wiring.
    compile_into(&mut d, &[landy_advance_package()]);

    // Woolsey "Multi-Landy" continuations, when on.
    compile_into(&mut d, &[woolsey_package()]);

    // Advancing partner's both-minors 2NT over their 1NT, when on.
    compile_into(&mut d, &[unusual_notrump_advance_package()]);

    // Direct-seat DONT and Meckwell advances.  Both write `[1NT, X, P]` and
    // friends; with both knobs on, Meckwell wins the shared keys exactly as it
    // did when these were consecutive `insert_all_seats` blocks, so the package
    // order here is load-bearing.
    compile_into(
        &mut d,
        &[direct_dont_advance_package(), meckwell_advance_package()],
    );

    // Direct-seat both-majors X advances.
    compile_into(&mut d, &[both_majors_double_package()]);
    d
}

#[cfg(test)]
mod tests;
