//! Defensive actions for the 2/1 system: overcalls, advances, and doubles
//!
//! This module covers everything our side does when the opponents open the
//! auction: simple overcalls, the 1NT overcall, takeout doubles, the
//! Michaels cue-bid, the Unusual 2NT, advances of all of these, advancing
//! partner's takeout double, responsive doubles when partner has made a
//! takeout double and they raise, and defense to a weak-two opening (takeout
//! double, a natural 2NT overcall, and natural suit overcalls).

use super::super::constraint::{
    Cons, Constraint, and, balanced, described, hcp, len, min_level_is, or, points,
    short_in_their_suits, stopper_in_their_suits, suit_hcp, top_honors, unbid_support,
};
use super::super::context::Context;
use super::super::{Alert, Defensive, Rules};
use super::competition::{
    LebensohlStyle, clubs_transfer_completion, complete_lebensohl_relay, cue_stayman_answer,
    cue_stayman_answer_no_stopper, delayed_cue, lebensohl_relay_rebid, lebensohl_responder,
    lm_2d_both_majors_advance, lm_2d_clubs_ask, lm_2d_clubs_major, stayman_2d_answer,
    stayman_2d_fit_rebid, transfer_completion, transfer_lebensohl_responder,
    transfer_stayman_2d_responder, transfer_target,
};
use super::notrump::{smolen_at_three, smolen_completion};
use super::{call, insert_all_seats};
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

thread_local! {
    /// Whether the natural one-suiter defense to their 1NT (penalty double + the
    /// four natural two-level overcalls + the owning `Pass` catch-all) is authored;
    /// **on by default**.  See [`set_natural_defense`].
    static NATURAL_DEFENSE: Cell<bool> = const { Cell::new(true) };
    /// Whether to also author the same defense in the *balancing* seat
    /// `(1NT) P P ?`; **off by default** (opt-in A/B). Off leaves the balancing
    /// seat to the instinct floor — the source of the toxic balancing doubles.
    static NOTRUMP_BALANCING: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the natural one-suiter defense to an opponent's 1NT for books built
/// *after* this call (thread-local, read once at book-construction time)
///
/// `true` (the **default**) authors the penalty double (15+ balanced), the four
/// natural two-level suit overcalls (five-card suit, 8–14), and the owning `Pass`
/// catch-all that lets the node keep a hand that qualifies for none of them.
/// `false` drops all of those, so when the two-suiter overlays ([`set_landy`],
/// [`set_unusual_notrump_defense`]) are also off the `[1NT]` node yields no finite
/// logit and the position falls through to the bare instinct floor — the baseline
/// arm of the standalone A/B (`examples/landy-ab --natural-measured`).
pub fn set_natural_defense(on: bool) {
    NATURAL_DEFENSE.with(|cell| cell.set(on));
}

/// Whether the natural one-suiter defense is currently authored
pub(crate) fn natural_defense_enabled() -> bool {
    NATURAL_DEFENSE.with(Cell::get)
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

thread_local! {
    /// Whether the **direct-seat DONT** defense replaces the natural penalty-X +
    /// overcalls over their 1NT; **off by default** (opt-in A/B).  On,
    /// `defense_to_notrump` authors the conventional DONT structure at every seat
    /// (one-suiter `X`, two-suiter `2♣`/`2♦`/`2♥`, natural `2♠`) and the
    /// passed-hand arm is suppressed (DONT already covers the passed seat).  See
    /// [`set_direct_dont`].
    static DIRECT_DONT: Cell<bool> = const { Cell::new(false) };
}

/// Replace the natural 1NT defense with conventional DONT at every seat, for books
/// built *after* this call (thread-local; **off by default**).
///
/// On: `X` = a one-suiter (♣/♦/♥, 5+, no second four-card suit — spade one-suiters
/// bid the natural `2♠`), `2♣` = clubs + a higher major, `2♦` = diamonds + a major,
/// `2♥` = both majors, `2♠` = natural spades, plus an owning `Pass` catch-all.
/// Pair with [`set_unusual_notrump_defense`] to add `2NT` = both minors.  Mutually
/// exclusive with the natural penalty-X arm ([`set_natural_defense`]).
pub fn set_direct_dont(on: bool) {
    DIRECT_DONT.with(|cell| cell.set(on));
}

/// Whether the direct-seat DONT defense is currently authored
pub(crate) fn direct_dont_enabled() -> bool {
    DIRECT_DONT.with(Cell::get)
}

thread_local! {
    /// Whether the direct-seat **Meckwell** defense replaces the natural defense of
    /// their 1NT; **off by default** (opt-in A/B).  Meckwell is DONT's cousin: the
    /// `X` is a two-way "single 6+ minor OR both majors" double, `2♣`/`2♦` are a
    /// minor + a major, `2♥`/`2♠` are natural single-suiters, and `2NT` is both
    /// minors (reusing [`set_unusual_notrump_defense`], on by default).  See
    /// [`set_meckwell`].
    static MECKWELL: Cell<bool> = const { Cell::new(false) };
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

/// Replace the natural 1NT defense with Meckwell at every seat, for books built
/// *after* this call (thread-local; **off by default**).
///
/// On: `X` = a single 6+ minor OR both majors (4-4+); `2♣` = clubs + a major, `2♦`
/// = diamonds + a major (5-4+ either way); `2♥`/`2♠` = a natural 5+ single-suited
/// major; `2NT` = both minors (pair with [`set_unusual_notrump_defense`], on by
/// default); an owning `Pass` catch-all.  Mutually exclusive with the natural
/// penalty-X arm and the DONT / direct-Landy / Woolsey conventions (each repurposes
/// the double).
pub fn set_meckwell(on: bool) {
    MECKWELL.with(|cell| cell.set(on));
}

/// Whether the direct-seat Meckwell defense is currently authored
pub(crate) fn meckwell_enabled() -> bool {
    MECKWELL.with(Cell::get)
}

/// Whether Meckwell's `2♣`/`2♦` accept a flat 4-4 (default `false` = 5-4+).  A
/// **probe** knob.  See [`set_meckwell`].
pub fn set_meckwell_minor_major_44(on: bool) {
    MECKWELL_MINOR_MAJOR_44.with(|cell| cell.set(on));
}

fn meckwell_minor_major_44() -> bool {
    MECKWELL_MINOR_MAJOR_44.with(Cell::get)
}

/// Whether Meckwell's both-majors `X` accepts a flat 4-4 (default `true` = 4-4).  A
/// **probe** knob.  See [`set_meckwell`].
pub fn set_meckwell_x_four_four(on: bool) {
    MECKWELL_X_FOUR_FOUR.with(|cell| cell.set(on));
}

fn meckwell_x_four_four() -> bool {
    MECKWELL_X_FOUR_FOUR.with(Cell::get)
}

/// Set the `points` floor for Meckwell's two-way `X` (default 0 = inherit the natural
/// overcall floor of 8; set e.g. 12 for a Woolsey-strength double).  A **probe** knob.
/// See [`set_meckwell`].
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
/// five-card one-suiters).  See [`set_direct_dont`].
pub fn set_direct_dont_one_suiter_min(min: u8) {
    DIRECT_DONT_ONE_SUITER_MIN.with(|cell| cell.set(min));
}

fn direct_dont_one_suiter_min() -> usize {
    DIRECT_DONT_ONE_SUITER_MIN.with(Cell::get) as usize
}

/// Whether DONT two-suiters accept a flat 4-4 (default true = traditional 4-4; false =
/// 5-4+).  See [`set_direct_dont`].
pub fn set_direct_dont_four_four(on: bool) {
    DIRECT_DONT_FOUR_FOUR.with(|cell| cell.set(on));
}

/// Set the `points` floor for the DONT one-suiter `X` (default 0 = inherit the natural
/// overcall floor of 8; raise it to double only with strong one-suiters).  See
/// [`set_direct_dont`].
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
    /// Whether the direct-seat **double** of their 1NT shows both majors (takeout)
    /// instead of the 15+ penalty double.  `None` = off (the **default**, natural
    /// penalty-X defense); `Some(four_four)` = on, with `false` = at least 5-4 in the
    /// majors and `true` = a flat 4-4 accepted.  See [`set_direct_landy_double`].
    static DIRECT_LANDY_DOUBLE: Cell<Option<bool>> = const { Cell::new(None) };
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
pub fn set_direct_landy_double(shape: Option<bool>) {
    DIRECT_LANDY_DOUBLE.with(|cell| cell.set(shape));
}

/// The configured direct-seat both-majors double shape, or `None` when off
pub(crate) fn direct_landy_double() -> Option<bool> {
    DIRECT_LANDY_DOUBLE.with(Cell::get)
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
    /// Whether our **Woolsey "Multi-Landy"** defense to their 1NT is authored at
    /// every seat, replacing the natural / Landy / penalty-X arms; **off by
    /// default** (opt-in A/B).  On, the direct seat over (1NT) is:
    ///
    /// - `X` = a 4-card major **+ a longer (5-6) minor** (takeout, never penalty),
    /// - `2♣` = both majors (5-4 / 5-5), advanced via the Landy machinery,
    /// - `2♦` = Multi, a single **6+ major**,
    /// - `2♥` / `2♠` = Muiderberg, **exactly 5** in the major **+ a 4+ minor**,
    /// - `Pass` everything else, including strong balanced (no penalty double).
    ///
    /// Distilled from BBA's compiled card (`docs/ai-bidder/bba-1nt-defense.md`);
    /// the strength bands are ours, not BBA's ([`set_woolsey_points`] /
    /// [`set_woolsey_double_floor`]).  See [`set_woolsey`].
    static WOOLSEY: Cell<bool> = const { Cell::new(false) };
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

/// Author the Woolsey "Multi-Landy" defense to their 1NT for books built *after*
/// this call (thread-local, read once at book-construction time)
///
/// **Off by default.**  On, the `[1NT]` node is the full Woolsey structure at every
/// seat — `X` = 4-card major + longer minor, `2♣` = both majors, `2♦` = Multi,
/// `2♥`/`2♠` = Muiderberg, `Pass` everything else — replacing the natural / Landy /
/// both-majors-X arms and superseding the passed-hand defense; the both-minors `2NT`
/// ([`set_unusual_notrump_defense`]) overlay stays compatible (it is outside the
/// Woolsey defense).  The A/B knob for `examples/ab-landy --ns-woolsey`.
pub fn set_woolsey(on: bool) {
    WOOLSEY.with(|cell| cell.set(on));
}

/// Whether the Woolsey defense is currently authored (read by the inference engine
/// to decode our artificial 2♣/2♦/2♥/2♠ overcalls; see `inference::multi_reading`)
pub(crate) fn woolsey_enabled() -> bool {
    WOOLSEY.with(Cell::get)
}

/// Set the inclusive `points` band for the Woolsey suit overcalls (`2♣`/`2♦`/`2♥`/
/// `2♠`, default 8–19) for books built *after* this call.  No effect unless
/// [`set_woolsey`] is on.  The A/B knob for `examples/ab-landy --ns-woolsey-range`.
pub fn set_woolsey_points(lo: u8, hi: u8) {
    WOOLSEY_POINTS.with(|cell| cell.set((lo, hi)));
}

/// The configured Woolsey suit-overcall `points` band (also the points floor the
/// inference engine reads for our 2♣/2♦/2♥/2♠ overcalls)
pub(crate) fn woolsey_points() -> (u8, u8) {
    WOOLSEY_POINTS.with(Cell::get)
}

/// Set the `points` floor for the Woolsey takeout `X` (default 12) for books built
/// *after* this call.  No effect unless [`set_woolsey`] is on.  The A/B knob for
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
            1.25,
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
    static NATURAL_DOUBLE_WEIGHT: Cell<f32> = const { Cell::new(1.3) };
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

/// Set the logit weight of the natural penalty double of their 1NT (default 1.3)
/// for books built *after* this call. Below the 1.0 suit-overcall weight, a strong
/// one-suiter overcalls instead of doubling. An A/B knob (`bba-match --ns-double-weight`).
pub fn set_natural_double_weight(weight: f32) {
    NATURAL_DOUBLE_WEIGHT.with(|cell| cell.set(weight));
}

fn natural_double_weight() -> f32 {
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

/// Semi-balanced shape for the penalty double: balanced, or one of 5422/6322/7222
fn semi_balanced() -> Cons<impl Constraint + Clone> {
    balanced()
        | described("5422/6322/7222", |hand: Hand, _: &Context<'_>| {
            let mut lengths = Suit::ASC.map(|suit| hand[suit].len());
            lengths.sort_unstable();
            matches!(lengths, [2, 2, 4, 5] | [2, 2, 3, 6] | [2, 2, 2, 7])
        })
}

thread_local! {
    /// The always-pass defense to their 1NT — a finite logit on `Pass`
    /// for every hand, which shadows the instinct floor at `[1NT]` so our side
    /// never competes. **Off by default.** See [`set_always_pass_defense`].
    static ALWAYS_PASS_DEFENSE: Cell<bool> = const { Cell::new(false) };
}

/// Toggle the always-pass defense to an opponent's 1NT for books built
/// *after* this call (thread-local, read once at book-construction time)
///
/// When on, the `[1NT]` node authors only `Pass` (for every hand), so our side
/// never acts over their 1NT — the truest "do nothing" baseline, distinct from
/// [`set_natural_defense`]`(false)` which drops to the instinct floor (and the
/// floor still competes a little). Overrides the natural and two-suiter arms.
/// The A/B baseline knob for `examples/landy-ab --ew-always-pass`.
pub fn set_always_pass_defense(on: bool) {
    ALWAYS_PASS_DEFENSE.with(|cell| cell.set(on));
}

/// Whether the always-pass defense is currently authored
fn always_pass_defense_enabled() -> bool {
    ALWAYS_PASS_DEFENSE.with(Cell::get)
}

thread_local! {
    /// Whether the responsive double after partner's **takeout double** + their
    /// raise (`[1t, X, raise]`) is authored; see [`set_responsive_takeout`].
    static RESPONSIVE_TAKEOUT: Cell<bool> = const { Cell::new(true) };
    /// Whether the responsive double after partner's **overcall** + their raise
    /// (`[1t, overcall, raise]`) is authored; see [`set_responsive_overcall`].
    static RESPONSIVE_OVERCALL: Cell<bool> = const { Cell::new(false) };
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
fn responsive_takeout_enabled() -> bool {
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

    let mut rules = Rules::new().rule(
        Bid::new(1, Strain::Notrump),
        1.5,
        hcp(15..=18) & balanced() & stopper_in_their_suits(),
    );

    // 12+ takeout double, optionally gated on support for the unbid suits so an
    // off-shape one-suiter overcalls (or waits for the 17+ tier) instead of
    // doubling and pulling to the 3-level.  See [`set_takeout_support`].
    rules = match takeout_support() {
        TakeoutSupport::Off => rules.rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits()),
        TakeoutSupport::Lenient => rules.rule(
            Call::Double,
            1.3,
            hcp(12..) & short_in_their_suits() & unbid_support(1),
        ),
        TakeoutSupport::Strict => rules.rule(
            Call::Double,
            1.3,
            hcp(12..) & short_in_their_suits() & unbid_support(0),
        ),
    };

    rules = rules
        .rule(Call::Double, 1.2, points(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls: five-card suit.  Disciplined bands by default — 1-level
    // 8–17, 2-level 11–17 (opening values before a below-their-suit 2-level
    // overcall); `set_overcall_discipline(false)` reverts to the flat 8–16.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let level = if strain > theirs { 1 } else { 2 };
            let weight = if level == 1 { 1.4 } else { 1.0 };
            let band = if !overcall_discipline() {
                8..=16
            } else if level == 1 {
                8..=17
            } else {
                11..=17
            };
            rules = rules.rule(
                Bid::new(level, strain),
                weight,
                len(suit, 5..) & points(band),
            );
        }
    }

    // Michaels cue-bid: 2 of their suit, 5-5, 8+ HCP.
    rules = match t {
        // t minor → both majors
        Suit::Clubs | Suit::Diamonds => rules
            .rule(
                Bid::new(2, theirs),
                2.0,
                len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
            )
            .alert(MICHAELS),
        // t = ♥ → spades + a minor
        Suit::Hearts => rules
            .rule(
                Bid::new(2, theirs),
                2.0,
                len(Suit::Spades, 5..)
                    & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                    & points(8..),
            )
            .alert(MICHAELS),
        // t = ♠ → hearts + a minor
        Suit::Spades => rules
            .rule(
                Bid::new(2, theirs),
                2.0,
                len(Suit::Hearts, 5..)
                    & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                    & points(8..),
            )
            .alert(MICHAELS),
    };

    // Unusual 2NT: 5-5 in the two lowest unbid suits, 8+ HCP.
    match t {
        Suit::Clubs => rules
            .rule(
                Bid::new(2, Strain::Notrump),
                1.9,
                len(Suit::Diamonds, 5..) & len(Suit::Hearts, 5..) & points(8..),
            )
            .alert(UNUSUAL),
        Suit::Diamonds => rules
            .rule(
                Bid::new(2, Strain::Notrump),
                1.9,
                len(Suit::Clubs, 5..) & len(Suit::Hearts, 5..) & points(8..),
            )
            .alert(UNUSUAL),
        Suit::Hearts | Suit::Spades => rules
            .rule(
                Bid::new(2, Strain::Notrump),
                1.9,
                len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..) & points(8..),
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

    let mut rules = Rules::new().rule(
        Bid::new(2, Strain::Notrump),
        1.5,
        hcp(15..=18) & balanced() & stopper_in_their_suits(),
    );

    // 12+ takeout double, optionally gated on unbid-suit support (see
    // [`set_takeout_support`]); the 17+ tier catches off-shape strong hands.
    rules = match takeout_support() {
        TakeoutSupport::Off => rules.rule(Call::Double, 1.3, hcp(12..) & short_in_their_suits()),
        TakeoutSupport::Lenient => rules.rule(
            Call::Double,
            1.3,
            hcp(12..) & short_in_their_suits() & unbid_support(1),
        ),
        TakeoutSupport::Strict => rules.rule(
            Call::Double,
            1.3,
            hcp(12..) & short_in_their_suits() & unbid_support(0),
        ),
    };

    rules = rules
        .rule(Call::Double, 1.2, points(17..))
        .rule(Call::Pass, 0.0, hcp(0..));

    // Natural overcalls: five-card suit, 10–16 points, at the cheapest legal level.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let overcall_level = if strain > theirs { level } else { level + 1 };
            rules = rules.rule(
                Bid::new(overcall_level, strain),
                1.0,
                len(suit, 5..) & points(10..=16),
            );
        }
    }

    // Leaping Michaels: a jump to 4♣/4♦ showing a 5-5 two-suiter with
    // game-forcing values.  These are all 4-level jumps, so they never collide
    // with the natural overcalls above (which sit at the 2/3 level), and 4♦ over
    // 2♦ is a cue the natural loop skips.
    if leaping_michaels_enabled() {
        let t = theirs.suit().expect("weak two is a suit bid");
        let gf = points(14..);
        match t {
            // Over a major: a minor plus the OTHER major.
            Suit::Hearts | Suit::Spades => {
                let other = if t == Suit::Hearts {
                    Suit::Spades
                } else {
                    Suit::Hearts
                };
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    rules = rules
                        .rule(
                            Bid::new(4, Strain::from(minor)),
                            2.0,
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
                        2.0,
                        len(Suit::Clubs, 5..)
                            & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                            & gf.clone(),
                    )
                    .alert(LEAPING)
                    .rule(
                        Bid::new(4, Strain::Diamonds),
                        2.0,
                        len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & gf.clone(),
                    )
                    .alert(LEAPING);
            }
            Suit::Clubs => {} // no weak 2♣ in our system
        }
    }
    rules
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

/// Unusual 2NT over a suit opening — 5-5 in the two lowest unbid suits
const UNUSUAL: Alert = Alert("unusual-2nt");

/// Leaping Michaels — a 4♣/4♦ jump over their weak two, a 5-5 game-forcing
/// two-suiter (distinct from the responder-side `comp:leaping-michaels`)
const LEAPING: Alert = Alert("leaping-michaels");

/// Responsive double — partner doubled/overcalled, they raised, advancer's double
/// shows the two unbid suits (4-4, 8+).  A takeout call (asks partner to pick a
/// suit), not a desire to defend, so it is alerted rather than read structurally.
const RESPONSIVE: Alert = Alert("responsive-double");

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
        1.9,
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
        1.9,
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
            1.9,
            len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..) & points(8..),
        )
        .alert(STAYMAN_DEFENSE_X)
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.8,
            len(Suit::Diamonds, min_len..) & points(floor..),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            1.8,
            len(Suit::Hearts, min_len..) & points(floor..),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.8,
            len(Suit::Spades, min_len..) & points(floor..),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            len(Suit::Clubs, 6..) & points(floor..),
        )
        .rule(Call::Pass, 0.5, hcp(0..))
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
            1.9,
            len(bid, 5..) & suit_hcp(bid, 5..) & points(8..),
        )
        .alert(TRANSFER_DEFENSE_X)
        .rule(
            Bid::new(2, Strain::from(shown_major)),
            1.7,
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
        let weight = if suit == bid { 2.0 } else { 1.8 };
        rules = rules.rule(
            Bid::new(level, strain),
            weight,
            len(suit, min_len..) & points(floor..),
        );
    }
    rules.rule(Call::Pass, 0.5, hcp(0..))
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
            1.9,
            len(Suit::Spades, 5..) & suit_hcp(Suit::Spades, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_X)
        // 2NT = the two lowest unbid suits (diamonds + hearts, 5-5) — naturally
        // disjoint from the spade-showing X.
        .rule(
            Bid::new(2, Strain::Notrump),
            1.7,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_2NT)
        // 3♣ cue of their clubs anchor = top-and-bottom (spades + diamonds, 5-5);
        // weight 2.0 beats the X so the two-suiter wins for a 5♠5♦ hand.
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            len(Suit::Spades, 5..) & len(Suit::Diamonds, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_CUE)
        // Natural six-card one-suiter overcalls in the unbid red suits.
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.8,
            len(Suit::Diamonds, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.8,
            len(Suit::Hearts, 6..) & points(14..),
        )
        .rule(Call::Pass, 0.5, hcp(0..))
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
            1.9,
            len(Suit::Diamonds, 5..) & suit_hcp(Suit::Diamonds, 5..) & points(8..),
        )
        .alert(DIAMOND_TRANSFER_DEFENSE_X)
        // 3♦ cue of their diamond anchor = both majors (5-5); weight 2.0 beats the
        // X so a 5♥-5♠ two-suiter shows rather than lead-directs.
        .rule(
            Bid::new(3, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
        )
        .alert(DIAMOND_TRANSFER_DEFENSE_CUE)
        // Natural six-card one-suiter overcalls in the unbid suits.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.8,
            len(Suit::Clubs, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            1.8,
            len(Suit::Hearts, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.8,
            len(Suit::Spades, 6..) & points(14..),
        )
        .rule(Call::Pass, 0.5, hcp(0..))
}

/// DONT `X`: a one-suiter (♣/♦/♥), `points(direct-dont-x-floor..)`.
fn dont_x() -> Rules {
    let lo = direct_dont_x_floor();
    let one_min = direct_dont_one_suiter_min();
    Rules::new().rule(
        Call::Double,
        1.9,
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
        Rules::new().rule(Bid::new(2, Strain::Clubs), 1.9, shape & hcp(lo..=hi))
    } else {
        Rules::new().rule(Bid::new(2, Strain::Clubs), 1.9, shape & points(lo..=hi))
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
        1.9,
        passed_two_suiter(Suit::Hearts, Suit::Spades) & points(lo..=hi),
    )
}

/// DONT `2♣`: clubs + a higher major, 5-4 (or 4-4 when configured).
fn dont_2c() -> Rules {
    let lo = natural_overcall_points().0;
    let ff = direct_dont_four_four();
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        2.0,
        dont_minor_major(Suit::Clubs, ff) & points(lo..),
    )
}

/// Woolsey Multi `2♦`: a single 6+ major.
fn multi_2d() -> Rules {
    let (lo, hi) = woolsey_points();
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        1.9,
        woolsey_multi() & points(lo..=hi),
    )
}

/// DONT `2♦`: diamonds + a higher major, 5-4 (or 4-4 when configured).
fn dont_2d() -> Rules {
    let lo = natural_overcall_points().0;
    let ff = direct_dont_four_four();
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        2.0,
        dont_minor_major(Suit::Diamonds, ff) & points(lo..),
    )
}

/// Woolsey Muiderberg `2♥`/`2♠`: exactly 5 in `major` + a 4+ minor.
fn muiderberg(major: Suit) -> Rules {
    let (lo, hi) = woolsey_points();
    Rules::new().rule(
        Bid::new(2, Strain::from(major)),
        1.9,
        woolsey_muiderberg(major) & points(lo..=hi),
    )
}

/// DONT `2♥`: both majors, 5-4 (or 4-4 when configured).
fn dont_2h() -> Rules {
    let lo = natural_overcall_points().0;
    let ff = direct_dont_four_four();
    Rules::new().rule(
        Bid::new(2, Strain::Hearts),
        2.0,
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
        1.9,
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
        2.0,
        dont_minor_major(Suit::Clubs, meckwell_minor_major_44()) & points(lo..),
    )
}

/// Meckwell `2♦`: diamonds + a major, 5-4 either way (or flat 4-4 per the probe knob).
fn meckwell_2d() -> Rules {
    let lo = natural_overcall_points().0;
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        2.0,
        dont_minor_major(Suit::Diamonds, meckwell_minor_major_44()) & points(lo..),
    )
}

/// Unusual `2NT`: both minors, 5-5, on its own range (raw HCP or points per
/// [`set_landy_hcp`]).  Additive — compatible with every system.
fn unusual_2nt() -> Rules {
    let (lo, hi) = unusual_notrump_range().unwrap_or((0, 37));
    let shape = len(Suit::Clubs, 5..) & len(Suit::Diamonds, 5..);
    if landy_use_hcp() {
        Rules::new().rule(Bid::new(2, Strain::Notrump), 1.8, shape & hcp(lo..=hi))
    } else {
        Rules::new().rule(Bid::new(2, Strain::Notrump), 1.8, shape & points(lo..=hi))
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
            1.0,
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
    // The always-pass baseline: a finite logit on `Pass` for every hand shadows the
    // floor here, so our side never competes (no overlays).
    if always_pass_defense_enabled() {
        return rules.rule(Call::Pass, 0.0, hcp(0..));
    }

    // Cascade precedence: Woolsey > DONT > direct-Landy-X > natural penalty-X.
    if woolsey_enabled() {
        // Woolsey owns X and every overcall; `Pass` is the only natural call.
        rules.rule(Call::Pass, 0.0, hcp(0..))
    } else if direct_dont_enabled() {
        // DONT keeps the natural `2♠` one-suiter (open-top, length-gated so the
        // one-suiter `X` can exclude spades) below its two-suiters, plus `Pass`.
        let lo = natural_overcall_points().0;
        let one_min = direct_dont_one_suiter_min();
        rules
            .rule(
                Bid::new(2, Strain::Spades),
                1.0,
                len(Suit::Spades, one_min..) & points(lo..),
            )
            .rule(Call::Pass, 0.0, hcp(0..))
    } else if meckwell_enabled() {
        // Meckwell keeps the natural 5+ single-suited majors (2♥/2♠, disjoint from its
        // two-suiters) below the alerts, plus Pass.  The two-way X / minor+major 2♣/2♦
        // / both-minors 2NT are the artificial calls.
        let lo = natural_overcall_points().0;
        rules
            .rule(
                Bid::new(2, Strain::Hearts),
                1.0,
                meckwell_natural_major(Suit::Hearts) & points(lo..),
            )
            .rule(
                Bid::new(2, Strain::Spades),
                1.0,
                meckwell_natural_major(Suit::Spades) & points(lo..),
            )
            .rule(Call::Pass, 0.0, hcp(0..))
    } else if direct_landy_double().is_some() {
        // The both-majors `X` is the alert; the four natural overcalls and `Pass`
        // are the floor-safe base (a 15+ balanced hand now passes or overcalls).
        chain_natural_overcalls(rules.rule(Call::Pass, 0.0, hcp(0..)), false)
    } else if natural_defense_enabled() {
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
        chain_natural_overcalls(
            rules.rule(Call::Pass, 0.0, hcp(0..)),
            landy_range().is_some(),
        )
    } else {
        // Natural off and no system: author nothing, fall to the instinct floor.
        rules
    }
}

/// The artificial alerts live at the `[1NT]` node for the configured system,
/// mirroring the cascade precedence (Woolsey > DONT > direct-Landy-X > natural)
/// plus the two independent overlays.  Read once at book-construction time.
fn active_alerts() -> Vec<Alert> {
    let mut alerts = Vec::new();
    if always_pass_defense_enabled() {
        return alerts;
    }
    if woolsey_enabled() {
        alerts.extend([
            WOOLSEY_X,
            WOOLSEY_2C,
            MULTI_2D,
            MUIDERBERG_2H,
            MUIDERBERG_2S,
        ]);
    } else if direct_dont_enabled() {
        alerts.extend([DONT_X, DONT_2C, DONT_2D, DONT_2H]);
    } else if meckwell_enabled() {
        alerts.extend([MECKWELL_X, MECKWELL_2C, MECKWELL_2D]);
    } else if direct_landy_double().is_some() {
        alerts.push(LANDY_X);
    }
    // The natural penalty-X family adds no alert of its own; the Landy `2♣` overlay
    // is its one convention, incompatible with DONT / Meckwell / direct-Landy-X /
    // Woolsey (each repurposes or replaces the `2♣` slot).
    if landy_range().is_some()
        && !direct_dont_enabled()
        && !meckwell_enabled()
        && direct_landy_double().is_none()
        && !woolsey_enabled()
    {
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
mod shape_guards {
    use super::*;
    use crate::bidding::verify::{accepts, compare, empty_context};
    use contract_bridge::Hand;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const N: usize = 8000;

    /// Sample `N` hands; assert `candidate` agrees with the intended `reference`
    /// boolean on all of them and that the reference is not vacuously empty.
    fn check(label: &str, candidate: impl Constraint, reference: impl Fn(Hand) -> bool) {
        let ctx = empty_context();
        let mut rng = StdRng::seed_from_u64(20_260_625);
        let report = compare(reference, |h| accepts(&candidate, h, &ctx), &mut rng, N);
        assert!(
            report.agrees(),
            "{label}: {} of {} hands disagree, e.g. {:?}",
            report.disagreements.len(),
            report.tested,
            report.disagreements.first(),
        );
        assert!(
            report.reference_accepts > 0,
            "{label}: reference accepts nothing — a vacuous guard",
        );
    }

    #[test]
    fn reauthored_shapes_match_intended_spec() {
        use Suit::{Clubs, Diamonds, Hearts, Spades};
        let ln = |h: Hand, s: Suit| h[s].len();

        // Multi 2♦ (simplified): a 6+ major, both minors ≤4 (now incl. 6-5 / 6-6).
        check("woolsey_multi", woolsey_multi(), |h| {
            (ln(h, Hearts) >= 6 || ln(h, Spades) >= 6) && ln(h, Clubs) <= 4 && ln(h, Diamonds) <= 4
        });

        // Muiderberg 2♥/2♠: exactly 5 in the major, ≤3 the other, a 4+ minor.  The
        // `== 5` pins disjointness from the 6+ Multi; the other-major ≤3 from 2♣.
        for major in [Hearts, Spades] {
            let other = if major == Hearts { Spades } else { Hearts };
            check("woolsey_muiderberg", woolsey_muiderberg(major), move |h| {
                ln(h, major) == 5
                    && ln(h, other) <= 3
                    && (ln(h, Clubs) >= 4 || ln(h, Diamonds) >= 4)
            });
        }

        // Woolsey X (unchanged): 4 in one major, ≤3 the other, a 5-6 minor.
        check("woolsey_double_shape", woolsey_double_shape(), |h| {
            let four_major = (ln(h, Hearts) == 4 && ln(h, Spades) <= 3)
                || (ln(h, Spades) == 4 && ln(h, Hearts) <= 3);
            four_major && ((5..=6).contains(&ln(h, Clubs)) || (5..=6).contains(&ln(h, Diamonds)))
        });

        // both_majors_shape: 5-4 (false) / flat 4-4 (true).
        check("both_majors_shape(false)", both_majors_shape(false), |h| {
            (ln(h, Hearts) >= 5 && ln(h, Spades) >= 4) || (ln(h, Hearts) >= 4 && ln(h, Spades) >= 5)
        });
        check("both_majors_shape(true)", both_majors_shape(true), |h| {
            ln(h, Hearts) >= 4 && ln(h, Spades) >= 4
        });

        // DONT one-suiter X: one of ♣/♦/♥ at least `min`, the other three ≤3.
        for min in [5usize, 6] {
            check(
                "dont_one_suiter_direct",
                dont_one_suiter_direct(min),
                move |h| {
                    let one = |long: Suit| {
                        ln(h, long) >= min
                            && [Clubs, Diamonds, Hearts, Spades]
                                .iter()
                                .all(|&s| s == long || ln(h, s) <= 3)
                    };
                    one(Clubs) || one(Diamonds) || one(Hearts)
                },
            );
        }

        // DONT minor+major: the minor 4+, a major 4+, one of them at least `longer`.
        for (minor, a44) in [
            (Clubs, true),
            (Clubs, false),
            (Diamonds, true),
            (Diamonds, false),
        ] {
            let longer = if a44 { 4 } else { 5 };
            check("dont_minor_major", dont_minor_major(minor, a44), move |h| {
                let hi = ln(h, Hearts).max(ln(h, Spades));
                ln(h, minor) >= 4 && hi >= 4 && (ln(h, minor) >= longer || hi >= longer)
            });
        }

        // DONT 2♥ both majors: flat 4-4 (true) / 5-4 (false).
        check("dont_both_majors(true)", dont_both_majors(true), |h| {
            ln(h, Hearts) >= 4 && ln(h, Spades) >= 4
        });
        check("dont_both_majors(false)", dont_both_majors(false), |h| {
            (ln(h, Hearts) >= 5 && ln(h, Spades) >= 4) || (ln(h, Hearts) >= 4 && ln(h, Spades) >= 5)
        });

        // Meckwell two-way X: a 6+ minor (other three ≤3) OR both majors (4-4 / 5-4).
        for a44 in [true, false] {
            let longer = if a44 { 4 } else { 5 };
            check(
                "meckwell_double_shape",
                meckwell_double_shape(6, a44),
                move |h| {
                    let one_minor = (ln(h, Clubs) >= 6
                        && ln(h, Diamonds) <= 3
                        && ln(h, Hearts) <= 3
                        && ln(h, Spades) <= 3)
                        || (ln(h, Diamonds) >= 6
                            && ln(h, Clubs) <= 3
                            && ln(h, Hearts) <= 3
                            && ln(h, Spades) <= 3);
                    let both_majors = ln(h, Hearts) >= 4
                        && ln(h, Spades) >= 4
                        && (ln(h, Hearts) >= longer || ln(h, Spades) >= longer);
                    one_minor || both_majors
                },
            );
        }

        // Meckwell natural 2M: 5+ in the major, ≤3 the other major, both minors ≤3.
        for major in [Hearts, Spades] {
            let other = if major == Hearts { Spades } else { Hearts };
            check(
                "meckwell_natural_major",
                meckwell_natural_major(major),
                move |h| {
                    ln(h, major) >= 5
                        && ln(h, other) <= 3
                        && ln(h, Clubs) <= 3
                        && ln(h, Diamonds) <= 3
                },
            );
        }
    }
}

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

    let hearts_longer = described("♥ at least as long as ♠", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() >= h[Suit::Spades].len()
    });
    let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
        h[Suit::Spades].len() > h[Suit::Hearts].len()
    });
    let equal_majors = described("equal majors", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() == h[Suit::Spades].len()
    });

    Rules::new()
        // Game with a known 4-card fit (preferred over the ask).
        .rule(
            Bid::new(4, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        // Game-forcing ask without a clear 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 1.2, points(game..))
        // Invitational with 4-card support.
        .rule(
            Bid::new(3, Strain::Hearts),
            1.1,
            len(Suit::Hearts, 4..) & points(invite..game) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.1,
            len(Suit::Spades, 4..) & points(invite..game) & spades_longer.clone(),
        )
        // Weak: equal majors → 2♦ relay; else preference signoff.
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.0,
            equal_majors & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            0.9,
            hearts_longer & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            0.9,
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

    let hearts_longer = described("♥ at least as long as ♠", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() >= h[Suit::Spades].len()
    });
    let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
        h[Suit::Spades].len() > h[Suit::Hearts].len()
    });
    let equal_majors = described("equal majors", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() == h[Suit::Spades].len()
    });
    // A long minor with both majors short (no 8-card fit opposite the overcaller's
    // 5-carder) outranks a major signoff. Gate A/B-tuned via set_doubled_landy_escape.
    let (min_minor, max_major) = doubled_landy_escape();
    let short_majors = len(Suit::Hearts, ..=max_major) & len(Suit::Spades, ..=max_major);

    Rules::new()
        // Strong arms — identical to the undoubled advance (no room gained above 2NT).
        .rule(
            Bid::new(4, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        .rule(Bid::new(2, Strain::Notrump), 1.2, points(game..))
        .rule(
            Bid::new(3, Strain::Hearts),
            1.1,
            len(Suit::Hearts, 4..) & points(invite..game) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.1,
            len(Suit::Spades, 4..) & points(invite..game) & spades_longer.clone(),
        )
        // Long club one-suiter, no major fit: sit for 2♣ doubled.
        .rule(
            Call::Pass,
            1.05,
            len(Suit::Clubs, min_minor..) & short_majors.clone(),
        )
        // Long diamond one-suiter, no major fit: natural 2♦, to play.
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.0,
            len(Suit::Diamonds, min_minor..) & short_majors & points(..game),
        )
        // Equal majors: Redouble asks the overcaller to name the longer one.
        .rule(Call::Redouble, 0.95, equal_majors & points(..invite))
        // Otherwise sign off in the longer major.
        .rule(
            Bid::new(2, Strain::Hearts),
            0.9,
            hearts_longer & points(..invite),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            0.9,
            spades_longer & points(..invite),
        )
}

/// Overcaller's rebid after advancer's *natural* `2♦` over the doubled Landy
/// (`[1NT, 2♣, X, 2♦, P]`): pass partner's diamonds, but with a singleton/void
/// diamond pull to the longer major (a 5-2 major fit beats a 6-1 diamond one).
fn landy_doubled_2d_rebid() -> Rules {
    let hearts_longer = described("♥ at least as long as ♠", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() >= h[Suit::Spades].len()
    });
    let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
        h[Suit::Spades].len() > h[Suit::Hearts].len()
    });
    Rules::new()
        .rule(
            Bid::new(2, Strain::Hearts),
            1.0,
            len(Suit::Diamonds, ..=1) & hearts_longer,
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Diamonds, ..=1) & spades_longer,
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Overcaller's rebid after the `2♦` relay (`[1NT, 2♣, P, 2♦, P]`): name the
/// longer major, so the equal-majors advancer plays the right strain
fn landy_2d_rebid() -> Rules {
    let hearts_longer = described("♥ at least as long as ♠", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() >= h[Suit::Spades].len()
    });
    let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
        h[Suit::Spades].len() > h[Suit::Hearts].len()
    });
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, hearts_longer)
        .rule(Bid::new(2, Strain::Spades), 1.0, spades_longer)
}

/// A Pass-only node: settle, play the contract on the table.  Authoring this where
/// the instinct floor would otherwise run keeps a finite logit on `Pass`, so the
/// floor's over-competition is shadowed (see `project_floor_shadowed_by_book_nodes`).
fn sit() -> Rules {
    Rules::new().rule(Call::Pass, 0.0, hcp(0..))
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
    let hearts_longer = described("♥ longer than ♠", |h: Hand, _: &Context<'_>| {
        h[Suit::Hearts].len() > h[Suit::Spades].len()
    });
    let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
        h[Suit::Spades].len() > h[Suit::Hearts].len()
    });
    let short_majors = len(Suit::Hearts, ..=2) & len(Suit::Spades, ..=2);
    Rules::new()
        // To-play game with a big fit in the preferred major.
        .rule(
            Bid::new(4, Strain::Hearts),
            1.4,
            len(Suit::Hearts, 4..) & points(game..) & hearts_longer.clone(),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            1.4,
            len(Suit::Spades, 4..) & points(game..) & spades_longer.clone(),
        )
        // Own long minor with no major fit → to play the minor.
        .rule(
            Bid::new(2, Strain::Clubs),
            1.1,
            len(Suit::Clubs, 5..) & short_majors.clone(),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.1,
            len(Suit::Diamonds, 5..) & short_majors,
        )
        // Major preference → to play.
        .rule(Bid::new(2, Strain::Spades), 1.0, spades_longer)
        .rule(Bid::new(2, Strain::Hearts), 1.0, hearts_longer)
        // Equal majors / nothing to say → ask: the doubler names its five-card major.
        .rule(Call::Pass, 0.5, hcp(0..))
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
    Rules::new().rule(Bid::new(2, Strain::Clubs), 1.0, hcp(0..))
}

/// Doubler naming the one-suiter after the `2♣` relay (`[…,1NT,X,P,2♣,P]`): pass
/// with clubs, else bid the five-or-six-card suit.
fn passed_dont_x_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 1.0, len(Suit::Diamonds, 5..))
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 5..))
        .rule(Bid::new(2, Strain::Spades), 1.0, len(Suit::Spades, 5..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Advancing partner's DONT `2♣` (clubs + a higher suit, `[…,1NT,2♣,P]`): pass
/// with club tolerance, else relay `2♦` ("name your higher suit").
fn passed_dont_2c_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 1.0, len(Suit::Clubs, ..=2))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Doubler naming the higher suit after the `2♦` relay (`[…,1NT,2♣,P,2♦,P]`):
/// pass with diamonds, else bid the major.
fn passed_dont_2c_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(Bid::new(2, Strain::Spades), 1.0, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Advancing partner's DONT `2♦` (diamonds + a major, `[…,1NT,2♦,P]`): pass with
/// diamond tolerance, else relay `2♥` ("name your major").
fn passed_dont_2d_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Diamonds, ..=2))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Doubler naming the major after the `2♥` relay (`[…,1NT,2♦,P,2♥,P]`): pass with
/// hearts, correct to `2♠` with spades.
fn passed_dont_2d_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 1.0, len(Suit::Spades, 4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Advancing partner's DONT `2♥` (both majors, `[…,1NT,2♥,P]`): pass with hearts,
/// correct to `2♠` with longer spades.
fn passed_dont_2h_advance() -> Rules {
    let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
        h[Suit::Spades].len() > h[Suit::Hearts].len()
    });
    Rules::new()
        .rule(Bid::new(2, Strain::Spades), 1.0, spades_longer)
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Advancing Meckwell's two-way `X` (`[…,1NT,X,P]`): relay `2♣` (pass-or-correct) —
/// the doubler then names its minor or shows both majors.  A single relay resolves the
/// two-way double's ambiguity; the advancer's own suits wait for the doubler's answer.
fn meckwell_x_advance() -> Rules {
    Rules::new().rule(Bid::new(2, Strain::Clubs), 1.0, hcp(0..))
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
            1.0,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, ..=3) & len(Suit::Spades, ..=3),
        )
        .rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(Call::Pass, 0.0, hcp(0..))
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
            1.3,
            five_five.clone() & points(lo..med),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.3,
            five_five.clone() & points(med..max),
        )
        .rule(Bid::new(3, Strain::Notrump), 1.3, five_five & points(max..))
        // 5-4 (the source omits a min-5-4 slot, so 3♣ folds min+medium together).
        .rule(Bid::new(3, Strain::Clubs), 1.2, points(lo..max))
        .rule(Bid::new(3, Strain::Diamonds), 1.2, points(max..))
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
        .rule(Bid::new(2, Strain::Notrump), 1.0, points(game..))
        // Constructive pass-or-correct: overcaller passes spades / 2NT-relays hearts.
        .rule(Bid::new(2, Strain::Spades), 0.95, points(invite..game))
        // Weak pass-or-correct: overcaller passes hearts / corrects 2♠ / jumps with 7+.
        .rule(Bid::new(2, Strain::Hearts), 0.9, points(..invite))
}

/// Overcaller over the weak `2♥` pass-or-correct (`[1NT, 2♦, P, 2♥, P]`): pass
/// with six hearts, correct to `2♠` with six spades, jump to `3♥`/`3♠` with seven
fn multi_2h_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.1, len(Suit::Hearts, 7..))
        .rule(Bid::new(3, Strain::Spades), 1.1, len(Suit::Spades, 7..))
        .rule(Bid::new(2, Strain::Spades), 1.0, len(Suit::Spades, 6..=6))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Overcaller over the constructive `2♠` pass-or-correct (`[1NT, 2♦, P, 2♠, *]`):
/// pass with spades, bid `3♥` with hearts.  Bidding the major directly (rather than
/// a 2NT heart-relay) keeps the rebid identical whether the `2♠` was passed or
/// doubled — over a double we must not be left to the floor.
fn multi_2s_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 6..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Overcaller over the game-forcing `2NT` ask (`[1NT, 2♦, P, 2NT, P]`): jump to
/// game in the 6-card major
fn multi_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 1.0, len(Suit::Hearts, 6..))
        .rule(Bid::new(4, Strain::Spades), 1.0, len(Suit::Spades, 6..))
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
        .rule(Bid::new(4, strain), 1.2, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            1.1,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No major fit, invitational+: ask the 4+ minor (then place 3NT / minor game).
        .rule(
            Bid::new(2, Strain::Notrump),
            1.0,
            len(major, ..=2) & points(invite..),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Advancer over a **doubled** Muiderberg `2M` (`[1NT, 2M, X]`): with a fit sit
/// for `2Mx` (a known 8+ card trump fit) or raise; with no fit escape via the
/// `2NT` minor-ask rather than be trapped in a doubled 5-1 misfit
fn muiderberg_advances_doubled(major: Suit, lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, strain), 1.2, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            1.1,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No fit → escape to the 4+ minor (any strength); a fit sits 2Mx.
        .rule(Bid::new(2, Strain::Notrump), 0.5, len(major, ..=2))
        .rule(Call::Pass, 0.0, len(major, 3..))
}

/// Overcaller answering the Muiderberg `2NT` minor-ask (`[1NT, 2M, …, 2NT, P]`):
/// name the 4+ minor — `3♦` with diamonds (longer or equal), else `3♣`
fn muiderberg_2nt_rebid() -> Rules {
    let diamonds_longer = described("♦ at least as long as ♣", |h: Hand, _: &Context<'_>| {
        h[Suit::Diamonds].len() >= h[Suit::Clubs].len()
    });
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 1.0, diamonds_longer)
        .rule(Bid::new(3, Strain::Clubs), 0.9, hcp(0..))
}

/// Advancer over the Woolsey takeout `X` (`[1NT, X, P]`): bid a 5+ major of your
/// own (to play), ask with a game-going hand, else relay `2♣` to the doubler's
/// long minor.  The catch-all `2♣` owns a finite logit so the floor never runs.
fn woolsey_x_advance(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Our own good major outranks the doubler's (its major may be the other one).
        .rule(Bid::new(2, Strain::Spades), 1.11, len(Suit::Spades, 5..))
        .rule(Bid::new(2, Strain::Hearts), 1.1, len(Suit::Hearts, 5..))
        // Game-going: ask the doubler to name its 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 1.0, points(game..))
        // Weak / no major of our own: name your minor, I pass or correct.
        .rule(Bid::new(2, Strain::Clubs), 0.9, hcp(0..))
}

/// Doubler over the `2♣` minor relay (`[1NT, X, P, 2♣, P]`): pass with the club
/// minor, correct to `2♦` with the diamond minor (advancer denied a major)
fn woolsey_x_minor_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 1.0, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Doubler over the `2NT` game-ask (`[1NT, X, P, 2NT, P]`): name the 4-card major
/// (the `X` always holds exactly one), leaving the advancer to place the game
fn woolsey_x_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.0, len(Suit::Hearts, 4..))
        .rule(Bid::new(3, Strain::Spades), 1.0, len(Suit::Spades, 4..))
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

    let mut rules = Rules::new()
        // Convert for penalty: a trump stack sits for the double.
        .rule(Call::Pass, 1.5, len(t, 4..) & top_honors(t, 2..) & hcp(6..))
        // 3NT to play: a stopper in their suit and game values.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            hcp(13..) & stopper_in_their_suits(),
        )
        // Weak escape to the cheapest notrump: no fit, no stopper, no stack.
        .rule(Bid::new(level, Strain::Notrump), 0.3, hcp(0..))
        // Final fallback.
        .rule(Call::Pass, 0.0, hcp(0..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain == theirs {
            continue;
        }
        let bid_level = if strain > theirs { level } else { level + 1 };
        // Natural advance at the cheapest legal level.
        rules = rules.rule(Bid::new(bid_level, strain), 1.0, len(suit, 4..));
        // Major-suit game jump with support and opening values.
        if matches!(suit, Suit::Hearts | Suit::Spades) {
            rules = rules.rule(Bid::new(4, strain), 1.4, len(suit, 4..) & points(11..));
        }
    }
    rules
}

/// Insert the advancer's actions after partner's takeout double of weak-two
/// `opening` (in `suit`), honoring the selected [`set_advance_sohl_style`]
///
/// `Off` keeps the flat [`advance_double`] ladder.  `Plain`/`Transfer`
/// shadow it with the reused Section-5 sohl builders under the `[2X, X, P]`
/// prefix — the `2NT` relay (and, for `Transfer`, the transfers + cue-Stayman) —
/// plus the doubler's continuations (relay completion, the rebid after `3♣`, and
/// the transfer / cue answers).  Over `(2♦)`, `Transfer` additionally
/// plays `3♣`-Stayman + Smolen + Leaping Michaels.  A forcing 3-level suit (`Plain`) or a
/// constructive advance is driven on by the instinct floor, which already
/// handles forced-to-game auctions.
fn insert_advance_of_double(d: &mut Defensive, suit: Suit, opening: Bid, style: LebensohlStyle) {
    let dbl_p = [Call::Bid(opening), Call::Double, Call::Pass];
    if style == LebensohlStyle::Off {
        insert_all_seats(d, &dbl_p, 3, advance_double(opening));
        return;
    }

    // Advancer's first action shadows the floor (the builders end in a 0.0 Pass,
    // which covers the weak and penalty-pass hands).
    let advancer = match style {
        // gate_4333 = false: advancing partner's takeout double — partner is short
        // in their suit, so the 4-4 fit keeps its ruffing value (the 4333 curse does
        // not apply here, and that A/B was never run).
        LebensohlStyle::Transfer if suit == Suit::Diamonds => transfer_stayman_2d_responder(false),
        LebensohlStyle::Transfer => transfer_lebensohl_responder(suit, false),
        _ => lebensohl_responder(suit),
    };
    insert_all_seats(d, &dbl_p, 3, advancer);

    // Doubler completes the 2NT relay with a forced 3♣; advancer then signs off.
    let two_nt = call(2, Strain::Notrump);
    let three_clubs = call(3, Strain::Clubs);
    insert_all_seats(
        d,
        &[
            Call::Bid(opening),
            Call::Double,
            Call::Pass,
            two_nt,
            Call::Pass,
        ],
        3,
        complete_lebensohl_relay(),
    );
    insert_all_seats(
        d,
        &[
            Call::Bid(opening),
            Call::Double,
            Call::Pass,
            two_nt,
            Call::Pass,
            three_clubs,
            Call::Pass,
        ],
        3,
        lebensohl_relay_rebid(suit),
    );

    // Transfer style: the doubler answers each 3-level transfer / cue. Over (2♦)
    // the Smolen block below owns the 3-level replies, so this covers (2♥)/(2♠).
    if style == LebensohlStyle::Transfer && suit != Suit::Diamonds {
        // Over (2♥)/(2♠) the delayed cue (2NT relay, then their suit) is always
        // *recognized* — answered as Stayman with a stopper — even when the bot
        // never bids it itself, so a human partner who plays it gets a sensible
        // reply. `split` (the default-off `set_delayed_cue` toggle) additionally
        // makes the bot *bid* the convention and read the direct cue as denying a
        // stopper (so it is answered without a free 3NT).
        let recognize = matches!(suit, Suit::Hearts | Suit::Spades);
        let split = delayed_cue() && recognize;
        for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let resp = call(3, Strain::from(bid_suit));
            let reply = if bid_suit == suit {
                if split {
                    cue_stayman_answer_no_stopper(suit)
                } else {
                    cue_stayman_answer(suit)
                }
            } else if let Some(target) = transfer_target(bid_suit, suit) {
                transfer_completion(target, suit)
            } else {
                continue; // the lowest suit has no transfer target — floored
            };
            insert_all_seats(
                d,
                &[
                    Call::Bid(opening),
                    Call::Double,
                    Call::Pass,
                    resp,
                    Call::Pass,
                ],
                3,
                reply,
            );
        }
        // Delayed cue: (2X)–X–P–2NT–P–3X (their suit) — Stayman with a stopper,
        // answered exactly like the direct cue but with 3NT safe. Wired whenever
        // it could be bid (recognition), independent of whether the bot bids it.
        if recognize {
            let cue = call(3, Strain::from(suit));
            insert_all_seats(
                d,
                &[
                    Call::Bid(opening),
                    Call::Double,
                    Call::Pass,
                    call(2, Strain::Notrump),
                    Call::Pass,
                    call(3, Strain::Clubs),
                    Call::Pass,
                    cue,
                    Call::Pass,
                ],
                3,
                cue_stayman_answer(suit),
            );
        }
    }

    // Transfer over (2♦): 3♣-Stayman + Smolen, the Jacoby transfers
    // (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦ — the diamond-only package
    // ported from the 1NT-(2♦) context. (2♥/2♠ reuse the Transfer completions above.)
    if style == LebensohlStyle::Transfer && suit == Suit::Diamonds {
        let p = Call::Pass;
        let c3 = call(3, Strain::Clubs);
        let d3 = call(3, Strain::Diamonds);
        let h3 = call(3, Strain::Hearts);
        let s3 = call(3, Strain::Spades);
        let c4 = call(4, Strain::Clubs);
        let d4 = call(4, Strain::Diamonds);
        let nodes: Vec<(Vec<Call>, Rules)> = vec![
            // 3♣ Stayman, doubler's answer; then Smolen after the 3♦ denial.
            (vec![c3, p], stayman_2d_answer()),
            (vec![c3, p, d3, p], smolen_at_three()),
            (vec![c3, p, d3, p, h3, p], smolen_completion(Suit::Spades)),
            (vec![c3, p, d3, p, s3, p], smolen_completion(Suit::Hearts)),
            // Doubler showed a 4-card major over Stayman; advancer places.
            (vec![c3, p, h3, p], stayman_2d_fit_rebid(Suit::Hearts)),
            (vec![c3, p, s3, p], stayman_2d_fit_rebid(Suit::Spades)),
            // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
            (vec![d3, p], transfer_completion(Suit::Hearts, suit)),
            (vec![h3, p], transfer_completion(Suit::Spades, suit)),
            (vec![s3, p], clubs_transfer_completion(suit)),
            // Leaping Michaels: 4♦ both majors, 4♣ clubs + a major (ask).
            (vec![d4, p], lm_2d_both_majors_advance()),
            (vec![c4, p], lm_2d_clubs_ask()),
            (vec![c4, p, d4, p], lm_2d_clubs_major()),
        ];
        for (rest, rules) in nodes {
            let prefix: Vec<Call> = dbl_p.iter().copied().chain(rest).collect();
            insert_all_seats(d, &prefix, 3, rules);
        }
    }
}

/// Advancer's response to partner's Michaels cue-bid over their opening `t`
fn michaels_advances(t: Suit) -> Rules {
    match t {
        // Partner shows both majors: prefer the longer one.
        Suit::Clubs | Suit::Diamonds => {
            let hearts_longer = described(
                "♥ at least as long as ♠",
                |hand: Hand, _: &Context<'_>| hand[Suit::Hearts].len() >= hand[Suit::Spades].len(),
            );
            let spades_longer = described("♠ longer than ♥", |hand: Hand, _: &Context<'_>| {
                hand[Suit::Spades].len() > hand[Suit::Hearts].len()
            });
            Rules::new()
                .rule(
                    Bid::new(4, Strain::Hearts),
                    1.3,
                    points(10..) & len(Suit::Hearts, 3..) & hearts_longer.clone(),
                )
                .rule(
                    Bid::new(4, Strain::Spades),
                    1.3,
                    points(10..) & len(Suit::Spades, 3..) & spades_longer.clone(),
                )
                .rule(Bid::new(2, Strain::Hearts), 1.0, hearts_longer)
                .rule(Bid::new(2, Strain::Spades), 1.0, spades_longer)
        }
        // Partner shows spades + a minor: bid spades.
        Suit::Hearts => Rules::new()
            .rule(
                Bid::new(4, Strain::Spades),
                1.3,
                points(10..) & len(Suit::Spades, 3..),
            )
            .rule(Bid::new(2, Strain::Spades), 0.5, hcp(0..)),
        // Partner shows hearts + a minor: bid hearts.
        Suit::Spades => Rules::new()
            .rule(
                Bid::new(4, Strain::Hearts),
                1.3,
                points(10..) & len(Suit::Hearts, 3..),
            )
            .rule(Bid::new(3, Strain::Hearts), 0.5, hcp(0..)),
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
                .rule(Bid::new(4, Strain::from(major)), 1.3, len(major, 2..))
                .rule(Bid::new(5, Strain::from(lm)), 1.2, len(major, 0..=1))
        }
        // Over 2♦.
        Suit::Diamonds => match lm {
            // 4♦ cue = both majors: pick the longer (both forced to game).
            Suit::Diamonds => {
                let hearts_longer =
                    described("♥ at least as long as ♠", |h: Hand, _: &Context<'_>| {
                        h[Suit::Hearts].len() >= h[Suit::Spades].len()
                    });
                let spades_longer = described("♠ longer than ♥", |h: Hand, _: &Context<'_>| {
                    h[Suit::Spades].len() > h[Suit::Hearts].len()
                });
                Rules::new()
                    .rule(Bid::new(4, Strain::Hearts), 1.3, hearts_longer)
                    .rule(Bid::new(4, Strain::Spades), 1.3, spades_longer)
            }
            // 4♣ = clubs + a major: 5♣ with a club fit and no major, else 4♥
            // pass-or-correct (partner names their major).
            Suit::Clubs => Rules::new()
                .rule(
                    Bid::new(5, Strain::Clubs),
                    1.2,
                    len(Suit::Clubs, 3..) & len(Suit::Hearts, 0..=2) & len(Suit::Spades, 0..=2),
                )
                .rule(Bid::new(4, Strain::Hearts), 1.3, hcp(0..)),
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
        .rule(Bid::new(4, Strain::Spades), 1.3, len(Suit::Spades, 5..))
        .rule(Call::Pass, 1.0, hcp(0..))
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
    let a_longer = described(
        format!("{a} at least as long as {b}"),
        move |hand: Hand, _: &Context<'_>| hand[a].len() >= hand[b].len(),
    );
    let b_longer = described(
        format!("{b} longer than {a}"),
        move |hand: Hand, _: &Context<'_>| hand[b].len() > hand[a].len(),
    );
    Rules::new()
        .rule(Bid::new(3, Strain::from(a)), 1.0, a_longer)
        .rule(Bid::new(3, Strain::from(b)), 1.0, b_longer)
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
                1.5,
                len(Suit::Clubs, 4..) & len(Suit::Diamonds, 4..) & points(8..),
            )
            .alert(RESPONSIVE)
    } else {
        // t minor → both majors
        Rules::new()
            .rule(
                Call::Double,
                1.5,
                len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & points(8..),
            )
            .alert(RESPONSIVE)
    };

    rules = rules.rule(Call::Pass, 0.0, hcp(0..));

    // Natural bids for suits ≠ t at levels 2 and 3.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t {
            continue;
        }
        let strain = Strain::from(suit);
        for bid_lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(bid_lvl, strain),
                1.0,
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
        .rule(Call::Double, 1.5, len(s1, 4..) & len(s2, 4..) & points(8..))
        .alert(RESPONSIVE)
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the defensive book: all our actions when the opponents open
///
/// Seat-fanned with `insert_all_seats(…, 3, …)` so every seat is covered.
/// Keys for a defensive auction are the raw table auction starting from their
/// opening, e.g. `[1♦, 2♦, Pass]` means they opened 1♦, we cue-bid 2♦
/// (Michaels), opener's side passed, and we are the advancer.
#[must_use]
pub fn defensive() -> Defensive {
    let mut d = Defensive::new();
    let advance_sohl = advance_sohl_style();

    // Over each one-of-a-suit opening: overcalls, double, 1NT, Michaels, Unusual.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(1, theirs);
        insert_all_seats(&mut d, &[Call::Bid(opening)], 3, defense_to_suit(opening));

        // Advancing partner's takeout double: [1t, X, P] — advancer to act.
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), Call::Double, Call::Pass],
            3,
            advance_double(opening),
        );

        // Advances of a natural overcall ([1t, overcall, Pass]) are left to the
        // instinct floor's Rubens transfers — the programmatic floor expresses
        // the transfer band for every (opening, overcall) pair in one place,
        // where a per-suit authored table cannot.

        // Advances of Michaels: [1t, 2t, Pass] — advancer to act.
        let michaels_bid = call(2, theirs);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), michaels_bid, Call::Pass],
            3,
            michaels_advances(suit),
        );

        // Advances of Unusual 2NT: [1t, 2NT, Pass] — advancer to act.
        let unusual_bid = call(2, Strain::Notrump);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening), unusual_bid, Call::Pass],
            3,
            unusual_nt_advances(suit),
        );

        // Responsive doubles: partner doubled for takeout, they raised to lvl.
        // On by default; the A/B knob (`--conv takeout`) turns it off to compare the
        // shipped node against the bare floor.
        if responsive_takeout_enabled() {
            for raise_lvl in [2u8, 3] {
                let raise = call(raise_lvl, theirs);
                insert_all_seats(
                    &mut d,
                    &[Call::Bid(opening), Call::Double, raise],
                    3,
                    responsive_doubles(suit, raise_lvl),
                );
            }
        }

        // Responsive double after partner's *overcall* + their raise
        // ([1t, overcall, raise]): off by default (the auction is otherwise floored).
        // The A/B knob (`--conv overcall`) turns it on; see set_responsive_overcall.
        if responsive_overcall_enabled() {
            for over in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                if over == suit {
                    continue;
                }
                // Partner's natural overcall of `over` at its minimum level over 1t:
                // the 1-level if it outranks their suit, else the 2-level.
                let over_lvl = if over > suit { 1 } else { 2 };
                let overcall = call(over_lvl, Strain::from(over));
                for raise_lvl in [2u8, 3] {
                    let raise = call(raise_lvl, theirs);
                    insert_all_seats(
                        &mut d,
                        &[Call::Bid(opening), overcall, raise],
                        3,
                        responsive_overcall_doubles(suit, over, raise_lvl),
                    );
                }
            }
        }
    }

    // Over each weak-two opening: takeout double, natural overcalls, 2NT, and
    // advancing partner's takeout double.  Clubs is omitted — a 2♣ opening is
    // the strong artificial bid, not a weak two.
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let theirs = Strain::from(suit);
        let opening = Bid::new(2, theirs);
        insert_all_seats(
            &mut d,
            &[Call::Bid(opening)],
            3,
            defense_to_weak_two(opening),
        );

        // Advancing partner's takeout double: [2t, X, P] — advancer to act.
        // Plain/Transfer sohl per `set_advance_sohl_style` (default Off keeps the
        // flat `advance_double` ladder).
        insert_advance_of_double(&mut d, suit, opening, advance_sohl);

        // Advances of Leaping Michaels: [2t, 4m, P] — advancer to act.  The jump
        // is below game, so the advancer is forced on (a fit major game, else the
        // 5m minor game — never a passed 4m partscore).
        if leaping_michaels_enabled() {
            for lm in [Suit::Clubs, Suit::Diamonds] {
                insert_all_seats(
                    &mut d,
                    &[Call::Bid(opening), call(4, Strain::from(lm)), Call::Pass],
                    3,
                    leaping_michaels_advances(suit, lm),
                );
            }
            // Over 2♦, 4♣ shows clubs + an unknown major; advancer's 4♥ is
            // pass-or-correct, so the overcaller names their major in rebid.
            if suit == Suit::Diamonds {
                insert_all_seats(
                    &mut d,
                    &[
                        Call::Bid(opening),
                        call(4, Strain::Clubs),
                        Call::Pass,
                        call(4, Strain::Hearts),
                        Call::Pass,
                    ],
                    3,
                    leaping_michaels_2d_4c_rebid(),
                );
            }
        }
    }

    let notrump = call(1, Strain::Notrump);
    insert_all_seats(&mut d, &[notrump], 3, defense_to_notrump());
    // Balancing seat (1NT) P P ?: reuse the same defense so we no longer fall to
    // the instinct floor's undisciplined balancing doubles.  First cut reuses the
    // direct ranges; a lighter balancing-specific range is a later refinement.
    if notrump_balancing_enabled() {
        insert_all_seats(
            &mut d,
            &[notrump, Call::Pass, Call::Pass],
            3,
            defense_to_notrump(),
        );
    }

    // Defense to the opponents' 2♣ Stayman: (1NT) P (2♣) ?  Opt-in (default off).
    // X = lead-directing clubs, natural overcalls, Unusual 2NT, natural 3♣ preempt.
    if stayman_defense_enabled() {
        insert_all_seats(
            &mut d,
            &[notrump, Call::Pass, call(2, Strain::Clubs)],
            3,
            defense_to_their_stayman(),
        );
    }

    // Defense to the opponents' Jacoby transfers: (1NT) P (2♦→♥) / (2♥→♠) ?
    // Opt-in (default off).  X = lead-directing the bid suit, cue = Michaels (the
    // other major + a minor), natural overcalls.
    if transfer_defense_enabled() {
        for (resp, shown) in [(Suit::Diamonds, Suit::Hearts), (Suit::Hearts, Suit::Spades)] {
            insert_all_seats(
                &mut d,
                &[notrump, Call::Pass, call(2, Strain::from(resp))],
                3,
                defense_to_their_transfer(resp, shown),
            );
        }
    }

    // Defense to the opponents' two-way 2♠ minor response: (1NT) P (2♠) ?  Opt-in
    // (default off).  X = lead-directing spades, 2NT = the red two-suiter, 3♣ cue =
    // top-and-bottom, natural 3♦/3♥ overcalls.
    if minor_transfer_defense_enabled() {
        insert_all_seats(
            &mut d,
            &[notrump, Call::Pass, call(2, Strain::Spades)],
            3,
            defense_to_their_minor_transfer(),
        );
    }

    // Defense to the opponents' 2NT diamond transfer: (1NT) P (2NT) ?  Opt-in
    // (default off).  X = lead-directing diamonds, 3♦ cue = both majors, natural
    // 3♣/3♥/3♠ overcalls.
    if diamond_transfer_defense_enabled() {
        insert_all_seats(
            &mut d,
            &[notrump, Call::Pass, call(2, Strain::Notrump)],
            3,
            defense_to_their_diamond_transfer(),
        );
    }

    // Advancing partner's Landy 2♣ (both majors) over their 1NT, when on.  Woolsey's
    // 2♣ is the identical both-majors call on the same shared band, so it reuses this
    // same advance wiring.
    if landy_range().is_some() || woolsey_enabled() {
        let (lo, hi) = woolsey_points();
        let landy_2c = call(2, Strain::Clubs);

        // [1NT, 2♣, P] — advancer picks a major / asks via the 2♦ / 2NT routes.
        insert_all_seats(
            &mut d,
            &[notrump, landy_2c, Call::Pass],
            3,
            landy_advances(lo),
        );
        // [1NT, 2♣, P, 2♦, P] — overcaller corrects to the longer major.
        insert_all_seats(
            &mut d,
            &[
                notrump,
                landy_2c,
                Call::Pass,
                call(2, Strain::Diamonds),
                Call::Pass,
            ],
            3,
            landy_2d_rebid(),
        );
        // [1NT, 2♣, P, 2NT, P] — overcaller answers the game-forcing ask.
        insert_all_seats(
            &mut d,
            &[
                notrump,
                landy_2c,
                Call::Pass,
                call(2, Strain::Notrump),
                Call::Pass,
            ],
            3,
            landy_2nt_rebid(lo, hi),
        );

        // [1NT, 2♣, X] — opponents doubled (stolen Stayman); advancer runs the
        // richer escape (Redouble = equal-majors relay, Pass = clubs, 2♦ = natural
        // diamonds, 2♥/2♠ = longer major). The Double frees the Redouble step.
        insert_all_seats(
            &mut d,
            &[notrump, landy_2c, Call::Double],
            3,
            landy_advances_over_double(lo),
        );
        // [1NT, 2♣, X, XX, P] — Redouble was the equal-majors relay; name the major.
        insert_all_seats(
            &mut d,
            &[notrump, landy_2c, Call::Double, Call::Redouble, Call::Pass],
            3,
            landy_2d_rebid(),
        );
        // [1NT, 2♣, X, 2♦, P] — advancer's 2♦ is natural; pass it or pull to a major.
        insert_all_seats(
            &mut d,
            &[
                notrump,
                landy_2c,
                Call::Double,
                call(2, Strain::Diamonds),
                Call::Pass,
            ],
            3,
            landy_doubled_2d_rebid(),
        );
        // [1NT, 2♣, X, 2NT, P] — overcaller answers the game-forcing ask.
        insert_all_seats(
            &mut d,
            &[
                notrump,
                landy_2c,
                Call::Double,
                call(2, Strain::Notrump),
                Call::Pass,
            ],
            3,
            landy_2nt_rebid(lo, hi),
        );
    }

    // Woolsey "Multi-Landy" continuations, when on — authored in full (the
    // both-majors 2♣ reuses the Landy advance wiring above).  Every artificial call
    // carries its doubled / redoubled escape so the opponents can never trap us in a
    // doubled artificial contract.
    if woolsey_enabled() {
        let lo = woolsey_points().0;
        let x = Call::Double;
        let multi = call(2, Strain::Diamonds);
        let hearts = call(2, Strain::Hearts);
        let spades = call(2, Strain::Spades);
        let clubs = call(2, Strain::Clubs);
        let nt2 = call(2, Strain::Notrump);

        // Multi 2♦.  The advance is the same over a pass or a double (it never sits
        // 2♦x — the overcaller has a major, not diamonds).  `rho` is the opponents'
        // call over our 2♦; `after` is their call over our pass-or-correct — the
        // overcaller names its major regardless of a double, so we are never left to
        // the floor in a doubled 2♥x/2♠x (the dominant 2♦ leak vs BBA).
        for rho in [Call::Pass, x] {
            insert_all_seats(&mut d, &[notrump, multi, rho], 3, multi_advances(lo));
            for after in [Call::Pass, x] {
                // Weak 2♥ p/c → pass / correct 2♠ / jump 3M with seven.
                insert_all_seats(
                    &mut d,
                    &[notrump, multi, rho, hearts, after],
                    3,
                    multi_2h_rebid(),
                );
                // Constructive 2♠ p/c → pass spades / 3♥ with hearts.
                insert_all_seats(
                    &mut d,
                    &[notrump, multi, rho, spades, after],
                    3,
                    multi_2s_rebid(),
                );
                // Game-force 2NT ask → overcaller jumps to game in its major.
                insert_all_seats(
                    &mut d,
                    &[notrump, multi, rho, nt2, after],
                    3,
                    multi_2nt_rebid(),
                );
            }
        }

        // Muiderberg 2♥/2♠ — raises + the 2NT minor-ask (a doubled escape with no fit).
        for (major, mbid) in [(Suit::Hearts, hearts), (Suit::Spades, spades)] {
            insert_all_seats(
                &mut d,
                &[notrump, mbid, Call::Pass],
                3,
                muiderberg_advances(major, lo),
            );
            insert_all_seats(
                &mut d,
                &[notrump, mbid, x],
                3,
                muiderberg_advances_doubled(major, lo),
            );
            // The 2NT minor-ask reaches the overcaller over either RHO action.
            for rho in [Call::Pass, x] {
                insert_all_seats(
                    &mut d,
                    &[notrump, mbid, rho, nt2, Call::Pass],
                    3,
                    muiderberg_2nt_rebid(),
                );
            }
        }

        // Takeout X — advancer relays to the minor / bids its own major / asks 2NT.
        // A redouble forces us to run (never sit 1NTxx): the same advance applies.
        let xfloor = woolsey_double_floor();
        for adv in [Call::Pass, Call::Redouble] {
            insert_all_seats(&mut d, &[notrump, x, adv], 3, woolsey_x_advance(xfloor));
            // The doubler names its 5-6 minor whether the 2♣ relay is passed or doubled.
            for after in [Call::Pass, x] {
                insert_all_seats(
                    &mut d,
                    &[notrump, x, adv, clubs, after],
                    3,
                    woolsey_x_minor_rebid(),
                );
            }
            // The 2NT game-ask → the doubler names its 4-card major.
            insert_all_seats(
                &mut d,
                &[notrump, x, adv, nt2, Call::Pass],
                3,
                woolsey_x_2nt_rebid(),
            );
        }
    }

    // Advancing partner's both-minors 2NT over their 1NT, when on.
    if unusual_notrump_range().is_some() {
        // [1NT, 2NT, P] — pick the longer minor (reuse the Unusual 2NT advance).
        insert_all_seats(
            &mut d,
            &[notrump, call(2, Strain::Notrump), Call::Pass],
            3,
            unusual_nt_advances(Suit::Spades),
        );
        // [1NT, 2NT, X] — doubled: never sit, just run to the longer minor (sitting
        // in 2NT-X is a loser — the doubler has values behind a 15-17 1NT).
        insert_all_seats(
            &mut d,
            &[notrump, call(2, Strain::Notrump), Call::Double],
            3,
            unusual_nt_advances(Suit::Spades),
        );
    }

    let p = Call::Pass;
    let x = Call::Double;
    let xx = Call::Redouble;

    // Direct-seat DONT advances: the same pass-or-correct relays, but keyed at
    // *every* seat via insert_all_seats (the X/2♣/2♦/2♥ are now direct-seat
    // conventional calls).  Binding [1NT,X,P] etc. is correct here — with DONT on
    // the direct `X` is a one-suiter wanting the 2♣ relay, not a penalty, so this
    // is exactly the case the passed-hand note above warned against doing when off.
    if direct_dont_enabled() {
        let c2 = call(2, Strain::Clubs);
        let d2 = call(2, Strain::Diamonds);
        let h2 = call(2, Strain::Hearts);
        insert_all_seats(&mut d, &[notrump, x, p], 3, passed_dont_x_advance());
        insert_all_seats(&mut d, &[notrump, x, p, c2, p], 3, passed_dont_x_rebid());
        insert_all_seats(&mut d, &[notrump, c2, p], 3, passed_dont_2c_advance());
        insert_all_seats(&mut d, &[notrump, c2, p, d2, p], 3, passed_dont_2c_rebid());
        insert_all_seats(&mut d, &[notrump, d2, p], 3, passed_dont_2d_advance());
        insert_all_seats(&mut d, &[notrump, d2, p, h2, p], 3, passed_dont_2d_rebid());
        insert_all_seats(&mut d, &[notrump, h2, p], 3, passed_dont_2h_advance());
        // Their redouble of our one-suiter X: never sit in 1NTxx — relay 2♣ just as
        // over a pass, then the doubler names the suit (mirrors the passed-hand
        // NaturalLandyDouble redouble escape).
        insert_all_seats(&mut d, &[notrump, x, xx], 3, passed_dont_x_advance());
        insert_all_seats(&mut d, &[notrump, x, xx, c2, p], 3, passed_dont_x_rebid());
        // Their double of our artificial 2♣ relay (after our X, passed or redoubled):
        // the relay is NOT a club fit, so the doubler must still name the real
        // one-suiter (2♦/2♥/2♠, or pass with genuine clubs) — else we sit in a
        // doubled misfit 2♣x, the dominant DONT-X loss in the honest measure.
        insert_all_seats(&mut d, &[notrump, x, p, c2, x], 3, passed_dont_x_rebid());
        insert_all_seats(&mut d, &[notrump, x, xx, c2, x], 3, passed_dont_x_rebid());
    }

    // Direct-seat Meckwell advances: the X is a two-way "single 6+ minor OR both
    // majors" double.  Advancer relays 2♣ (pass-or-correct); the doubler passes with
    // clubs, names 2♦ with diamonds, or bids 2♥ (4+ hearts ⇒ both majors here) and the
    // advancer passes / corrects to 2♠.  The minor+major 2♣/2♦ reuse the DONT
    // pass-or-correct advances (same "name your higher suit" relay).  Every artificial
    // leg has a doubled/redoubled escape so we never sit in 1NTxx or a doubled misfit.
    if meckwell_enabled() {
        let c2 = call(2, Strain::Clubs);
        let d2 = call(2, Strain::Diamonds);
        let h2 = call(2, Strain::Hearts);
        // X = two-way: relay 2♣, doubler names its minor / shows both majors (2♥).
        insert_all_seats(&mut d, &[notrump, x, p], 3, meckwell_x_advance());
        insert_all_seats(&mut d, &[notrump, x, p, c2, p], 3, meckwell_x_rebid());
        insert_all_seats(
            &mut d,
            &[notrump, x, p, c2, p, h2, p],
            3,
            passed_dont_2h_advance(),
        );
        // 2♣/2♦ minor+major: reuse the DONT pass-or-correct advances.
        insert_all_seats(&mut d, &[notrump, c2, p], 3, passed_dont_2c_advance());
        insert_all_seats(&mut d, &[notrump, c2, p, d2, p], 3, passed_dont_2c_rebid());
        insert_all_seats(&mut d, &[notrump, d2, p], 3, passed_dont_2d_advance());
        insert_all_seats(&mut d, &[notrump, d2, p, h2, p], 3, passed_dont_2d_rebid());
        // Their redouble of our X: relay 2♣ anyway (never sit 1NTxx), doubler names.
        insert_all_seats(&mut d, &[notrump, x, xx], 3, meckwell_x_advance());
        insert_all_seats(&mut d, &[notrump, x, xx, c2, p], 3, meckwell_x_rebid());
        // Their double of our artificial 2♣ relay: the doubler still names the real
        // suit (pass only with genuine clubs), else runs — never a doubled misfit 2♣x.
        insert_all_seats(&mut d, &[notrump, x, p, c2, x], 3, meckwell_x_rebid());
        insert_all_seats(&mut d, &[notrump, x, xx, c2, x], 3, meckwell_x_rebid());
        // Their double of the doubler's both-majors 2♥ show: advancer still picks a major.
        insert_all_seats(
            &mut d,
            &[notrump, x, p, c2, p, h2, x],
            3,
            passed_dont_2h_advance(),
        );
    }

    // Direct-seat both-majors X advances: the X is a Landy-style both-majors takeout
    // double at every seat, so the advancer answers exactly as over a Landy 2♣ (pick
    // a major / 2♦ relay / 2NT game-ask), keyed at [1NT,X,…] via insert_all_seats.
    // Binding [1NT,X,P] is correct here — the direct X is both-majors, not penalty.
    if direct_landy_double().is_some() {
        // The advancer's invite/game thresholds track the X floor (a stronger X asks
        // less of the advancer), so read it here too.
        let (lo, hi) = (direct_landy_double_floor(), 37u8);
        let d2 = call(2, Strain::Diamonds);
        let nt2 = call(2, Strain::Notrump);
        // [1NT,X,P] — advancer picks a major / relays 2♦ / asks 2NT, or (with the
        // penalty-pass knob on) passes to defend 1NTx with no fit and enough defense.
        insert_all_seats(&mut d, &[notrump, x, p], 3, both_majors_x_advance(lo));
        // [1NT,X,P,2♦,*] — the 2♦ relay is artificial (equal-majors "pick a major"),
        // so the doubler names the longer major whether the relay is passed OR
        // doubled — never left to sit in a short-diamond 2♦x misfit (the DONT bug).
        insert_all_seats(&mut d, &[notrump, x, p, d2, p], 3, landy_2d_rebid());
        insert_all_seats(&mut d, &[notrump, x, p, d2, x], 3, landy_2d_rebid());
        // [1NT,X,P,2NT,*] — the game-ask is artificial too; the doubler answers it
        // regardless of a double (landy_2nt_rebid has no Pass, so it always pulls).
        insert_all_seats(&mut d, &[notrump, x, p, nt2, p], 3, landy_2nt_rebid(lo, hi));
        insert_all_seats(&mut d, &[notrump, x, p, nt2, x], 3, landy_2nt_rebid(lo, hi));
        // [1NT,X,XX] — their redouble.  A *clean* runout (no artificial 2♦ relay):
        // Pass = ask back (doubler names its five-card major), a bid = to play the
        // natural suit (2♣ now sits at the two level over the redoubled 1NT, so a
        // club one-suiter has a home).  Killing the relay kills the phantom-3♦ run.
        insert_all_seats(&mut d, &[notrump, x, xx], 3, both_majors_x_runout(lo));
        // [1NT,X,XX,P,P] — advancer asked; the doubler names its longer major.
        insert_all_seats(&mut d, &[notrump, x, xx, p, p], 3, landy_2d_rebid());
        // …then the advancer SITS for that major whether it is passed or doubled —
        // play 2Mx (our real fit), never run.
        for m in [call(2, Strain::Hearts), call(2, Strain::Spades)] {
            insert_all_seats(&mut d, &[notrump, x, xx, p, p, m, p], 3, sit());
            insert_all_seats(&mut d, &[notrump, x, xx, p, p, m, x], 3, sit());
        }
        // The undoubled branch keeps the 2♦ relay (Pass there defends 1NT, so it
        // cannot be the ask).  Once the doubler names its major over the (possibly
        // doubled) relay, SIT when the opponents double it: `[1NT,X,P,2♦,{X|P},2M,X,P,P]`
        // round-trips to the doubler, who plays 2Mx instead of running to the phantom
        // 3♦.  (The dominant DD leak was this `… 2♦ X 2M X … 3♦` run from a making
        // doubled major; the redoubled branch above now avoids the relay entirely.)
        for relay in [x, p] {
            for m in [call(2, Strain::Hearts), call(2, Strain::Spades)] {
                insert_all_seats(&mut d, &[notrump, x, p, d2, relay, m, x, p, p], 3, sit());
            }
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use crate::bidding::Family;
    use crate::bidding::american::{
        LebensohlStyle, american, set_advance_sohl_style, set_always_pass_defense, set_direct_dont,
        set_direct_landy_double, set_leaping_michaels, set_meckwell, set_unusual_notrump_defense,
        set_woolsey, set_woolsey_double_floor, set_woolsey_points,
    };
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Hand, Strain};

    const fn call(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// `american()`'s best call for a hand in an auction, and whether the instinct
    /// floor (not a book node) produced it
    fn best_call(auction: &[Call], hand: &str) -> (Call, bool) {
        let hand: Hand = hand.parse().expect("valid test hand");
        let (logits, prov) = american()
            .against(Family::NATURAL)
            .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
            .expect("a legal auction classifies");
        let best = (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty");
        (best, prov.depth == 0 && prov.fallback.is_some())
    }

    /// Coupling: a Landy range feeds the one shared two-suiter band, so Landy's and
    /// Woolsey's identical both-majors `2♣` can never carry divergent strengths.
    #[test]
    fn landy_range_feeds_the_shared_woolsey_band() {
        super::set_landy(Some((9, 16)));
        assert_eq!(
            super::woolsey_points(),
            (9, 16),
            "a Landy range sets the shared band"
        );
        // Turning Landy off must not clobber an explicit Woolsey band.
        set_woolsey_points(7, 18);
        super::set_landy(None);
        assert_eq!(
            super::woolsey_points(),
            (7, 18),
            "set_landy(None) leaves the band alone"
        );
        // Restore the default for any sibling test sharing this thread.
        set_woolsey_points(8, 19);
    }

    /// Per-call exclusivity: in every named 1NT-defense config the `[1NT]` node
    /// authors at most one rule per call.  This is the invariant the alert-tag gate
    /// must preserve — two rules on one call would let a hand fire the wrong
    /// convention and a reading mis-decode it (e.g. a natural overcall leaking onto
    /// a slot an artificial alert owns).  Pass is always authored (these configs all
    /// own the auction).
    #[test]
    fn defense_to_notrump_authors_one_rule_per_call() {
        fn reset() {
            super::set_woolsey(false);
            super::set_direct_dont(false);
            super::set_meckwell(false);
            super::set_direct_landy_double(None);
            super::set_landy(None);
            super::set_natural_defense(true);
            super::set_always_pass_defense(false);
            super::set_unusual_notrump_defense(Some((8, 13)));
        }

        let configs: [(&str, fn()); 7] = [
            ("natural+unusual2nt", || {}),
            ("natural+landy", || super::set_landy(Some((8, 15)))),
            ("woolsey", || super::set_woolsey(true)),
            ("dont", || super::set_direct_dont(true)),
            ("meckwell", || super::set_meckwell(true)),
            ("direct-landy-x", || {
                super::set_direct_landy_double(Some(false))
            }),
            ("always-pass", || super::set_always_pass_defense(true)),
        ];

        for (label, setup) in configs {
            reset();
            setup();
            let calls: Vec<Call> = super::defense_to_notrump()
                .rules()
                .iter()
                .map(|r| r.call())
                .collect();
            reset();
            assert!(
                calls.contains(&Call::Pass),
                "{label}: the owning Pass is missing",
            );
            for i in 0..calls.len() {
                for j in (i + 1)..calls.len() {
                    assert!(
                        calls[i] != calls[j],
                        "{label}: call {:?} authored by two rules at the [1NT] node",
                        calls[i],
                    );
                }
            }
        }
    }

    /// Best call with the advance-of-double sohl forced to `style` (independent of
    /// any other test on this thread having changed it)
    fn advance(style: LebensohlStyle, auction: &[Call], hand: &str) -> (Call, bool) {
        set_advance_sohl_style(style);
        best_call(auction, hand)
    }

    /// `(2♦)–X–(P)` — partner doubled their weak two, advancer to act
    fn over_2d() -> [Call; 3] {
        [call(2, Strain::Diamonds), Call::Double, Call::Pass]
    }

    #[test]
    fn off_keeps_the_flat_advance_no_relay() {
        // Default Off: a weak six-club hand bids the natural 3♣ (advance_double),
        // not the 2NT relay — the toggle gates the new structure.
        let (c, _) = advance(LebensohlStyle::Off, &over_2d(), "32.43.32.KQ9876");
        assert_eq!(c, call(3, Strain::Clubs));
    }

    #[test]
    fn plain_weak_long_suit_relays_then_completes() {
        // Plain: weak hand (6 HCP), six clubs → 2NT relay; doubler forced to 3♣.
        let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "J2.43.32.KQ9876");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the relay must come from the book");

        let relayed = [
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (completion, _) = advance(LebensohlStyle::Plain, &relayed, "AKJ2.KQ52.4.A532");
        assert_eq!(completion, call(3, Strain::Clubs));
    }

    #[test]
    fn plain_forcing_three_level_is_a_book_node() {
        // Plain: five spades and game values → forcing 3♠ (a jump over 2♦),
        // never a weak partscore.
        let (c, floored) = advance(LebensohlStyle::Plain, &over_2d(), "KQT95.A43.32.J32");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the forcing 3-level bid must come from the book");
    }

    #[test]
    fn transfer_shows_spades_through_their_hearts() {
        // Transfer: over (2♥), five spades and game values transfer *through*
        // hearts — 3♦ shows spades (not diamonds), a book node.
        let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
        let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the transfer must come from the book");
    }

    #[test]
    fn transfer_doubler_bids_game_not_partscore() {
        // After (2♥)–X–(P)–3♦ (transfer to spades), the doubler with a fit bids
        // the spade *game*, never a 3♠ partscore.
        let auction = [
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = advance(LebensohlStyle::Transfer, &auction, "AK52.4.A432.K432");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_cue_is_stayman() {
        // (2♥)–X–(P)–3♥ is the cue = Stayman; the doubler shows a 4-card major.
        // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
        let auction = [
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (c, floored) = advance(LebensohlStyle::Transfer, &auction, "AQ32.K32.4.KJ432");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn penalty_pass_sits_for_the_double() {
        // A trump stack in their suit (five spades over 2♠) has no constructive
        // call — the book's terminal Pass leaves the takeout double in for
        // penalty, exactly as the flat ladder would.
        let over_2s = [call(2, Strain::Spades), Call::Double, Call::Pass];
        let (c, floored) = advance(LebensohlStyle::Plain, &over_2s, "KQJ95.J32.432.32");
        assert_eq!(c, Call::Pass);
        assert!(!floored, "the sign-off Pass must come from the book node");
    }

    #[test]
    fn transfer_over_2d_is_three_club_stayman() {
        // (2♦)–X–(P): Transfer's (2♦)-only Smolen leg bids 3♣-Stayman for a 4-4
        // majors GF advancer, a book node (over (2♥)/(2♠) it is plain Cohen, whose
        // 3♣ is not Stayman).
        let (c, floored) = advance(LebensohlStyle::Transfer, &over_2d(), "AQ32.KJ32.A2.432");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the Stayman bid must come from the book");
    }

    #[test]
    fn always_pass_defense_passes_over_1nt() {
        // The always-pass baseline: a 15-count balanced hand that would normally make a
        // penalty double passes instead, and the Pass is a book node (not the floor)
        // so it shadows whatever the floor would have done over their 1NT.
        let over_1nt = [call(1, Strain::Notrump)];
        set_always_pass_defense(true);
        let (c, floored) = best_call(&over_1nt, "AQ32.KQ3.K32.Q32");
        set_always_pass_defense(false);
        assert_eq!(c, Call::Pass);
        assert!(!floored, "the always-pass must come from the book node");
    }

    /// Best call with Woolsey forced on (default ranges) and the conflicting
    /// overlays reset, independent of any other test on this thread.  Resets the
    /// toggle afterward so it cannot leak into a non-Woolsey test.
    fn woolsey(auction: &[Call], hand: &str) -> (Call, bool) {
        set_always_pass_defense(false);
        set_unusual_notrump_defense(None);
        set_woolsey_points(9, 19);
        set_woolsey_double_floor(11);
        set_woolsey(true);
        let result = best_call(auction, hand);
        set_woolsey(false);
        result
    }

    #[test]
    fn woolsey_direct_seat_routes_every_shape() {
        let over_1nt = [call(1, Strain::Notrump)];
        // 2♦ Multi: a single 6-card heart suit (other major short).
        let (multi, floored) = woolsey(&over_1nt, "32.KQJ987.A32.32");
        assert_eq!(multi, call(2, Strain::Diamonds));
        assert!(
            !floored,
            "the Woolsey overcall must come from the book node"
        );
        // 2♣ both majors: 5-4.
        assert_eq!(
            woolsey(&over_1nt, "AJ987.KQ32.32.32").0,
            call(2, Strain::Clubs)
        );
        // 2♥ Muiderberg: exactly 5 hearts + a 4-card minor, short spades.
        assert_eq!(
            woolsey(&over_1nt, "32.AQJ98.K987.2").0,
            call(2, Strain::Hearts)
        );
        // X: a 4-card major + a longer (5-card) minor, 11+.
        assert_eq!(woolsey(&over_1nt, "AKQ8.32.KJ987.32").0, Call::Double);
    }

    #[test]
    fn woolsey_has_no_penalty_double() {
        let over_1nt = [call(1, Strain::Notrump)];
        // A flat 22-count has no Woolsey bid — it passes, exactly as in BBA's read
        // (there is no penalty double in this structure).
        let (strong, floored) = woolsey(&over_1nt, "AQ32.KQ3.KQ3.AQ2");
        assert_eq!(strong, Call::Pass);
        assert!(!floored, "the settling Pass must come from the book node");
        // A bare 5332 with a five-card major (no 4-card minor) also passes.
        assert_eq!(woolsey(&over_1nt, "AKJ32.K32.Q32.32").0, Call::Pass);
    }

    #[test]
    fn woolsey_multi_advance_pass_or_corrects() {
        // [1NT, 2♦, P] — a weak advancer bids the 2♥ pass-or-correct.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = woolsey(&auction, "32.K32.J32.J5432");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the Multi advance must come from the book node");
    }

    #[test]
    fn woolsey_x_advance_never_sits_for_penalty() {
        // [1NT, X, P] — the X is takeout, so a weak no-major advancer relays 2♣
        // (names the doubler's minor), never passing to defend a phantom 1NTx.
        let auction = [call(1, Strain::Notrump), Call::Double, Call::Pass];
        let (relay, floored) = woolsey(&auction, "432.432.432.5432");
        assert_eq!(relay, call(2, Strain::Clubs));
        assert!(!floored, "the X advance must come from the book node");
        // With a good 5-card major of its own, the advancer bids it to play.
        assert_eq!(
            woolsey(&auction, "KQ982.32.432.432").0,
            call(2, Strain::Spades)
        );
    }

    #[test]
    fn woolsey_muiderberg_advance_raises_and_asks() {
        // [1NT, 2♥, P] — a known 5-card heart suit.  With support + game values the
        // advancer raises to 4♥; with no fit it asks the minor via 2NT (a book node).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            Call::Pass,
        ];
        let (raise, floored) = woolsey(&auction, "32.K54.AK32.AQ32");
        assert_eq!(raise, call(4, Strain::Hearts));
        assert!(
            !floored,
            "the Muiderberg advance must come from the book node"
        );
        // No heart fit (singleton), invitational+ → 2NT minor-ask, never a floored guess.
        assert_eq!(
            woolsey(&auction, "KQJ2.2.K432.Q432").0,
            call(2, Strain::Notrump)
        );
    }

    #[test]
    fn woolsey_muiderberg_doubled_escapes_a_misfit() {
        // [1NT, 2♥, X] — a weak hand short in hearts escapes the doubled misfit via
        // the 2NT minor-ask rather than sitting in a doubled 5-1 fit.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            Call::Double,
        ];
        let (escape, floored) = woolsey(&auction, "Q432.2.J432.J432");
        assert_eq!(escape, call(2, Strain::Notrump));
        assert!(!floored, "the doubled escape must come from the book node");
        // With a genuine fit it sits for 2♥x (a known 8-card trump fit).
        assert_eq!(woolsey(&auction, "Q43.K52.J432.432").0, Call::Pass);
    }

    #[test]
    fn woolsey_muiderberg_2nt_names_the_minor() {
        // [1NT, 2♥, P, 2NT, P] — the overcaller answers the minor-ask: 3♦ with
        // diamonds, 3♣ with clubs (it always holds a 4+ minor).
        let asked = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(
            woolsey(&asked, "2.AKJ32.Q432.32").0,
            call(3, Strain::Diamonds)
        );
        assert_eq!(woolsey(&asked, "2.AKJ32.32.Q432").0, call(3, Strain::Clubs));
    }

    #[test]
    fn transfer_over_2h_is_plain_cohen() {
        // Over (2♥) Transfer is plain Cohen: a 5-spade GF transfers *through*
        // hearts — 3♦ shows spades, a book node (the diamond Smolen leg only
        // fires over (2♦)).
        let over_2h = [call(2, Strain::Hearts), Call::Double, Call::Pass];
        let (c, floored) = advance(LebensohlStyle::Transfer, &over_2h, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the transfer must come from the book");
    }

    /// Best call with Leaping Michaels forced to `on` (and the sohl toggles reset,
    /// independent of any other test on this thread)
    fn leaping(on: bool, auction: &[Call], hand: &str) -> (Call, bool) {
        set_advance_sohl_style(LebensohlStyle::Off);
        set_leaping_michaels(on);
        best_call(auction, hand)
    }

    #[test]
    fn leaping_michaels_minor_plus_other_major_over_a_major() {
        // Over (2♥): 5-5 clubs+spades, game values → 4♣; 5-5 diamonds+spades → 4♦.
        let over_2h = [call(2, Strain::Hearts)];
        let (c, floored) = leaping(true, &over_2h, "AKQ65.4.32.KQJ76");
        assert_eq!(c, call(4, Strain::Clubs));
        assert!(!floored, "Leaping Michaels must come from the book node");

        let (d, _) = leaping(true, &over_2h, "AKQ65.4.KQJ76.32");
        assert_eq!(d, call(4, Strain::Diamonds));
    }

    #[test]
    fn leaping_michaels_cue_shows_both_majors_over_2d() {
        // Over (2♦): 5-5 in the majors → 4♦ (the cue), both majors.
        let over_2d = [call(2, Strain::Diamonds)];
        let (c, floored) = leaping(true, &over_2d, "AKQ65.KQJ76.4.32");
        assert_eq!(c, call(4, Strain::Diamonds));
        assert!(!floored, "Leaping Michaels must come from the book node");
    }

    #[test]
    fn leaping_michaels_advancer_picks_the_major_game() {
        // (2♥)–4♣–(P): partner shows clubs + spades. With spade support the
        // advancer bids the 4♠ game; with none, the 5♣ minor game (never pass 4♣).
        let auction = [call(2, Strain::Hearts), call(4, Strain::Clubs), Call::Pass];
        let (fit, floored) = leaping(true, &auction, "KQ7.32.J865.A432");
        assert_eq!(fit, call(4, Strain::Spades));
        assert!(!floored, "the advance must come from the book node");

        // A doubleton (7-card fit) still takes the 4♠ game — it scores well and
        // needs only ten tricks.
        let (thin, _) = leaping(true, &auction, "K7.QJ32.8654.A32");
        assert_eq!(thin, call(4, Strain::Spades));

        // A genuine major misfit (≤1) retreats to the 5♣ game, not a passed 4♣.
        let (no_fit, _) = leaping(true, &auction, "2.QJ32.J8654.KQ4");
        assert_eq!(no_fit, call(5, Strain::Clubs));
    }

    #[test]
    fn leaping_michaels_advancer_picks_longer_major_over_2d_cue() {
        // (2♦)–4♦–(P): the cue shows both majors; advancer picks the longer.
        let auction = [
            call(2, Strain::Diamonds),
            call(4, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = leaping(true, &auction, "AQ32.K8.654.9432");
        assert_eq!(c, call(4, Strain::Spades));
        assert!(!floored, "the advance must come from the book node");
    }

    #[test]
    fn leaping_michaels_2d_4c_pass_or_correct() {
        // (2♦)–4♣–(P): clubs + an unknown major → 4♥ pass-or-correct, then the
        // overcaller with spades corrects to 4♠.
        let advance = [
            call(2, Strain::Diamonds),
            call(4, Strain::Clubs),
            Call::Pass,
        ];
        let (relay, _) = leaping(true, &advance, "K32.A87.9654.J32");
        assert_eq!(relay, call(4, Strain::Hearts));

        let rebid = [
            call(2, Strain::Diamonds),
            call(4, Strain::Clubs),
            Call::Pass,
            call(4, Strain::Hearts),
            Call::Pass,
        ];
        let (correct, _) = leaping(true, &rebid, "AKQ65.4.32.KQJ76");
        assert_eq!(correct, call(4, Strain::Spades));
    }

    #[test]
    fn leaping_michaels_silent_when_disabled() {
        // Turned off: the same club-spade two-suiter never jumps to 4♣ (the
        // escape hatch back to the pre-Leaping-Michaels weak-two defense).
        let over_2h = [call(2, Strain::Hearts)];
        let (c, _) = leaping(false, &over_2h, "AKQ65.4.32.KQJ76");
        assert_ne!(c, call(4, Strain::Clubs));
    }

    /// Best call with direct-seat DONT forced on, restored after so it never leaks
    /// into a sibling test on this thread.
    fn direct_dont(auction: &[Call], hand: &str) -> (Call, bool) {
        let prev = super::direct_dont_enabled();
        set_direct_dont(true);
        let result = best_call(auction, hand);
        set_direct_dont(prev);
        result
    }

    #[test]
    fn direct_dont_replaces_the_penalty_double() {
        // Direct seat over (1NT) with DONT on: the conventional structure, not the
        // natural penalty-X + overcalls.
        let over_1nt = [call(1, Strain::Notrump)];

        // Clubs + a higher major (5♣-4♠) → 2♣  (♣+♦ would be 2NT, not authored here).
        let (c, floored) = direct_dont(&over_1nt, "KJ32.32.4.AQ876");
        assert_eq!(c, call(2, Strain::Clubs));
        assert!(!floored, "DONT 2♣ must come from the book node");

        // Diamonds + a major (5♦-4♥) → 2♦.
        let (c, _) = direct_dont(&over_1nt, "32.KJ32.AQ876.4");
        assert_eq!(c, call(2, Strain::Diamonds));

        // Both majors (5♠-4♥) → 2♥.
        let (c, _) = direct_dont(&over_1nt, "AJ932.K842.32.32");
        assert_eq!(c, call(2, Strain::Hearts));

        // A spade one-suiter bids the natural 2♠ directly (not the X relay).
        let (c, _) = direct_dont(&over_1nt, "AKJ87.432.32.432");
        assert_eq!(c, call(2, Strain::Spades));

        // A non-spade (heart) one-suiter → X, the one-suiter relay double.
        let (c, _) = direct_dont(&over_1nt, "432.AKJ87.32.432");
        assert_eq!(c, Call::Double);

        // 15+ balanced has no DONT bid → Pass; the penalty double is gone.
        let (c, _) = direct_dont(&over_1nt, "AKQ2.KQ2.KJ2.432");
        assert_eq!(c, Call::Pass);
    }

    #[test]
    fn direct_dont_one_suiter_double_relays_then_names() {
        // [1NT,X,P]: with DONT on the direct-seat X is a one-suiter, so the advancer
        // relays 2♣ (a book node now keyed at the direct seat, not floored)...
        let nt = call(1, Strain::Notrump);
        let p = Call::Pass;
        let prev = super::direct_dont_enabled();
        set_direct_dont(true);
        let (relay, floored) = best_call(&[nt, Call::Double, p], "Q32.Q32.Q432.432");
        // ...and the doubler with a long heart suit names it.
        let after_relay = [nt, Call::Double, p, call(2, Strain::Clubs), p];
        let (name, _) = best_call(&after_relay, "432.AKJ87.32.432");
        // And if they redouble the one-suiter X, the advancer still relays 2♣ —
        // never sits in 1NTxx.
        let (escape, esc_floored) =
            best_call(&[nt, Call::Double, Call::Redouble], "Q32.Q32.Q432.432");
        // And if they double our artificial 2♣ relay, the doubler still names the
        // real suit (2♥ here) rather than sitting in the 2♣x misfit.
        let relay_doubled = [
            nt,
            Call::Double,
            Call::Redouble,
            call(2, Strain::Clubs),
            Call::Double,
        ];
        let (named, nd_floored) = best_call(&relay_doubled, "432.AKJ87.32.432");
        set_direct_dont(prev);
        assert_eq!(relay, call(2, Strain::Clubs));
        assert!(!floored, "the direct-seat relay must come from the book");
        assert_eq!(name, call(2, Strain::Hearts));
        assert_eq!(escape, call(2, Strain::Clubs), "must escape 1NTxx, not sit");
        assert!(!esc_floored, "the redouble escape must come from the book");
        assert_eq!(
            named,
            call(2, Strain::Hearts),
            "must escape 2♣x to the real suit"
        );
        assert!(
            !nd_floored,
            "the doubled-relay escape must come from the book"
        );
    }

    /// Best call with Meckwell forced on, restored after so it never leaks to a
    /// sibling test on this thread.
    fn meckwell(auction: &[Call], hand: &str) -> (Call, bool) {
        let prev = super::meckwell_enabled();
        set_meckwell(true);
        let result = best_call(auction, hand);
        set_meckwell(prev);
        result
    }

    #[test]
    fn meckwell_overcalls_replace_the_penalty_double() {
        let over_1nt = [call(1, Strain::Notrump)];

        // A single 6+ minor (long clubs, short elsewhere) → the two-way X, from the book.
        let (c, floored) = meckwell(&over_1nt, "32.32.432.AKQ876");
        assert_eq!(c, Call::Double);
        assert!(!floored, "Meckwell X must come from the book node");

        // Both majors (5-4) → the two-way X too (default four-four accepts it).
        let (c, _) = meckwell(&over_1nt, "AJ32.KQ876.32.32");
        assert_eq!(c, Call::Double);

        // Clubs + a major (5♣-4♠) → 2♣.
        let (c, floored) = meckwell(&over_1nt, "KJ32.32.4.AQ876");
        assert_eq!(c, call(2, Strain::Clubs));
        assert!(!floored, "Meckwell 2♣ must come from the book node");

        // Diamonds + a major (5♦-4♥) → 2♦.
        let (c, _) = meckwell(&over_1nt, "32.KJ32.AQ876.4");
        assert_eq!(c, call(2, Strain::Diamonds));

        // A natural single-suited 6-card heart hand → 2♥ (not the both-majors X).
        let (c, floored) = meckwell(&over_1nt, "32.AKJ876.432.32");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "natural 2♥ must come from the book node");

        // A natural single-suited spade hand → 2♠.
        let (c, _) = meckwell(&over_1nt, "AKJ876.32.432.32");
        assert_eq!(c, call(2, Strain::Spades));

        // Both minors (5-5) → 2NT (the Unusual overlay, on by default).
        let (c, _) = meckwell(&over_1nt, "3.3.AJ876.KQ876");
        assert_eq!(c, call(2, Strain::Notrump));

        // 15+ balanced has no Meckwell bid → Pass; the penalty double is gone.
        let (c, _) = meckwell(&over_1nt, "AKQ2.KQ2.KJ2.432");
        assert_eq!(c, Call::Pass);
    }

    #[test]
    fn meckwell_two_way_double_relays_then_names() {
        let nt = call(1, Strain::Notrump);
        let p = Call::Pass;
        let c2 = call(2, Strain::Clubs);
        let prev = super::meckwell_enabled();
        set_meckwell(true);

        // [1NT,X,P]: advancer relays 2♣ (pass-or-correct), from the book.
        let (relay, relay_floored) = best_call(&[nt, Call::Double, p], "Q32.Q32.Q432.432");
        // [1NT,X,P,2♣,P]: a diamond one-suiter doubler names 2♦ (real diamonds).
        let (diamonds, _) = best_call(&[nt, Call::Double, p, c2, p], "32.32.AKQ876.432");
        // …a both-majors doubler bids 2♥ (4+ hearts here ⇒ both majors).
        let (majors, majors_floored) = best_call(&[nt, Call::Double, p, c2, p], "AJ32.KQ87.32.32");
        // …a club one-suiter doubler passes (plays 2♣).
        let (clubs, _) = best_call(&[nt, Call::Double, p, c2, p], "32.32.432.AKQ876");
        // [1NT,X,XX]: their redouble — the advancer still relays 2♣, never sits 1NTxx.
        let (escape, esc_floored) =
            best_call(&[nt, Call::Double, Call::Redouble], "Q32.Q32.Q432.432");
        // [1NT,X,P,2♣,X]: they double our relay — the diamond doubler still names 2♦,
        // never sits in the doubled 2♣x misfit.
        let (named, nd_floored) =
            best_call(&[nt, Call::Double, p, c2, Call::Double], "32.32.AKQ876.432");
        set_meckwell(prev);

        assert_eq!(relay, c2, "advancer relays 2♣ over the two-way X");
        assert!(!relay_floored, "the relay must come from the book");
        assert_eq!(
            diamonds,
            call(2, Strain::Diamonds),
            "diamond one-suiter names 2♦"
        );
        assert_eq!(majors, call(2, Strain::Hearts), "both majors shown as 2♥");
        assert!(
            !majors_floored,
            "the both-majors show must come from the book"
        );
        assert_eq!(clubs, Call::Pass, "club one-suiter passes to play 2♣");
        assert_eq!(escape, c2, "must escape 1NTxx with the relay, not sit");
        assert!(!esc_floored, "the redouble escape must come from the book");
        assert_eq!(
            named,
            call(2, Strain::Diamonds),
            "must escape 2♣x to real diamonds"
        );
        assert!(
            !nd_floored,
            "the doubled-relay escape must come from the book"
        );
    }

    #[test]
    fn direct_landy_double_shows_both_majors_and_runs_clean() {
        let nt = call(1, Strain::Notrump);
        let p = Call::Pass;
        let x = Call::Double;
        let xx = Call::Redouble;
        let d2 = call(2, Strain::Diamonds);
        let prev = super::direct_landy_double();
        let prev_floor = super::direct_landy_double_floor();
        set_direct_landy_double(Some(false)); // 5-4
        super::set_direct_landy_double_floor(8); // low floor so these 10-14 hands fire the X

        // Both majors 5-4 → X (the both-majors takeout double), from the book.
        let (dbl, floored) = best_call(&[nt], "AJ32.KQ876.32.32");
        // 15+ balanced has no penalty double now → Pass.
        let (pass, _) = best_call(&[nt], "AKQ2.KQ2.KJ2.432");
        // Advancer, equal majors and weak → 2♦ relay ("pick a major").
        let (relay, relay_floored) = best_call(&[nt, x, p], "Q32.Q43.J432.432");
        // They double the artificial relay → doubler still names the longer major
        // (5-4 hearts → 2♥), never sits in the short-diamond 2♦x misfit.
        let (named, named_floored) = best_call(&[nt, x, p, d2, x], "AJ32.KQ876.32.32");
        // They redouble our X.  Clean runout: equal majors / no suit → Pass = ask back
        // (the doubler will name its major), never the phantom 2♦ relay.
        let (ask, ask_floored) = best_call(&[nt, x, xx], "Q32.Q43.J432.432");
        // …and a long-club, short-major advancer escapes to its own 2♣ (to play) —
        // the club rung the two-level 2♣ over the redoubled 1NT gives us.
        let (clubs, _) = best_call(&[nt, x, xx], "32.43.432.AKQ876");
        // After the ask, the doubler names its five-card major.
        let (named_xx, named_xx_floored) = best_call(&[nt, x, xx, p, p], "AJ32.KQ876.32.32");
        // After we name our major (via the undoubled relay) and they double it, SIT —
        // play 2♥x (our 5-4+ fit), never run to 3♦.  `[1NT,X,P,2♦,X,2♥,X,P,P]`.
        let sit_auction = [nt, x, p, d2, x, call(2, Strain::Hearts), x, p, p];
        let (settle, settle_floored) = best_call(&sit_auction, "AJ32.KQ876.32.32");

        set_direct_landy_double(prev);
        super::set_direct_landy_double_floor(prev_floor);
        assert_eq!(ask, Call::Pass, "equal majors over XX → Pass = ask back");
        assert!(!ask_floored, "the ask-Pass must come from the book");
        assert_eq!(
            clubs,
            call(2, Strain::Clubs),
            "long clubs over XX → 2♣ to play"
        );
        assert_eq!(
            named_xx,
            call(2, Strain::Hearts),
            "doubler names its major after the ask"
        );
        assert!(!named_xx_floored, "the named major must come from the book");
        assert_eq!(
            settle,
            Call::Pass,
            "must sit in our doubled major, not run to 3♦"
        );
        assert!(!settle_floored, "the settle-Pass must come from the book");
        assert_eq!(dbl, Call::Double);
        assert!(!floored, "the both-majors X must come from the book node");
        assert_eq!(pass, Call::Pass, "no penalty double when it is replaced");
        assert_eq!(relay, d2, "weak equal majors relays 2♦");
        assert!(!relay_floored, "the relay must come from the book");
        assert_eq!(
            named,
            call(2, Strain::Hearts),
            "must pull from the doubled 2♦ relay"
        );
        assert!(
            !named_floored,
            "the doubled-relay escape must come from the book"
        );
    }

    #[test]
    fn direct_landy_penalty_pass_defends_1ntx() {
        let nt = call(1, Strain::Notrump);
        let p = Call::Pass;
        let x = Call::Double;
        let prev = super::direct_landy_double();
        let prev_pen = super::direct_landy_penalty_pass();
        let prev_floor = super::direct_landy_double_floor();
        set_direct_landy_double(Some(false)); // 5-4
        super::set_direct_landy_double_floor(8); // floor 8 → penalty needs 22-8 = 14+

        // No major fit (2-2) + defensive values: with the knob OFF the advancer is
        // forced to bid (no Pass rule); with it ON it passes to defend 1NTx.
        let defensive = "AQ.KQ.QJ876.K432"; // 14 HCP, 2♠-2♥
        super::set_direct_landy_penalty_pass(false);
        let (forced, _) = best_call(&[nt, x, p], defensive);
        super::set_direct_landy_penalty_pass(true);
        let (penalty, pen_floored) = best_call(&[nt, x, p], defensive);
        // A hand WITH a major fit still bids even with the knob on (not a penalty pass).
        let (with_fit, _) = best_call(&[nt, x, p], "QJ32.K.QJ876.K43"); // 4 spades

        set_direct_landy_double(prev);
        super::set_direct_landy_penalty_pass(prev_pen);
        super::set_direct_landy_double_floor(prev_floor);
        assert_ne!(forced, Call::Pass, "knob off: advancer is forced to bid");
        assert_eq!(
            penalty,
            Call::Pass,
            "knob on, no fit + values → pass for penalty"
        );
        assert!(!pen_floored, "the penalty pass must come from the book");
        assert_ne!(
            with_fit,
            Call::Pass,
            "a major fit still bids, never penalty-passes"
        );
    }

    #[test]
    fn doubled_unusual_2nt_runs_never_sits() {
        // Their 1NT, our both-minors 2NT (on by default), their penalty X — the
        // advancer must run to the longer minor, never sit in the doomed 2NT-X.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Notrump),
            Call::Double,
        ];
        // Clubs longer → 3♣ (a book node, not a floored pass).
        let (c, floored) = best_call(&auction, "432.32.QJ8.T9876");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the runout must come from the book");
        // Diamonds longer → 3♦.
        let (d, _) = best_call(&auction, "432.32.QJ876.T98");
        assert_eq!(d, call(3, Strain::Diamonds));
    }
}
