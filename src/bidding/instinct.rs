//! The instinct bidder: a keyless floor for off-book auctions
//!
//! Competitive auctions cannot be enumerated — interference multiplies
//! sequences combinatorially, and a book that stops mid-auction leaves the
//! driver to pass by default.  The worst of those defaults is passing
//! partner's takeout double on a worthless hand, turning a routine advance
//! into a doubled partscore for the opponents.
//!
//! [`instinct()`] is the floor under the book: one context-driven [`Rules`]
//! ladder that answers *every* auction with a sane natural action.  Attach it
//! as a root [`Always`][super::fallback::Always] fallback — as
//! [`american()`][crate::bidding::american::american] does for its
//! competitive and defensive books — and the system never falls off the book.
//! By [`Trie::resolve`][super::Trie::resolve] precedence the root is reached
//! last, so instinct can never override an authored rule, only catch what
//! falls past all of them.
//!
//! # Everything is natural
//!
//! Instinct fires precisely where the book has no agreement, so partner's
//! continuation is usually off-book too — decoded by *partner's* instinct.
//! The two halves stay coherent because every instinct call is natural:
//! bids show the bid suit, raises show support, doubles are takeout.  No
//! conventional calls (in particular no strength-showing cue-bids) belong
//! here until both sides of the convention are authored.
//!
//! # Advancing partner's double
//!
//! Partner's live takeout double — the auction ends `… (bid) X (Pass)` with
//! their suit bid at the three level or below doubled by partner — calls for an
//! advance, but a takeout double is *not 100% forcing*.  Pass means *play the
//! top bid*: with length behind their doubled suit the better action is to
//! **defend** (pass plays their doubled contract), so the floor passes; only a
//! hand that cannot beat their contract advances — a penalty pass on a trump
//! stack, a major-suit game jump or 3NT with values, the longest unbid suit at
//! the cheapest level, and a notrump escape so *some* action is always
//! available.  A four-level new suit is a *free bid* (you could defend instead),
//! so it shows values.  The interpretation of the double is deliberately
//! mechanical: a classifier may know its system, and instinct's system is plain
//! standard.  (The defend-or-advance reading is the "settle floor", default on;
//! [`set_settle_floor`] recovers the old always-advance behavior.)
//!
//! # Observability
//!
//! Instinct activations are visible in the
//! [`Provenance`][super::trie::Provenance] returned by
//! [`Trie::resolve`][super::Trie::resolve]: `depth == 0` with
//! `fallback == Some(_)` is the floor firing.  In simulation, count these —
//! the most-hit auctions are the next nodes worth authoring properly.

use super::Rules;
use super::constraint::{
    Cons, Constraint, balanced, described, hcp, len, min_level_is, partner_shown_len,
    partner_suit_is, point_count, points, pred, short_in_their_suits, stopper_in_their_suits,
    support, they_bid,
};
use super::context::Context;
use super::inference::Inferences;
use super::rules::Alert;
use contract_bridge::auction::Call;
use contract_bridge::eval::hcp as holding_hcp;
use contract_bridge::{Bid, Hand, Penalty, Rank, Strain, Suit};
use core::cell::Cell;

/// The per-call alert for responder's gambling 3NT over a double of our 1NT: a
/// long minor run, *not* a natural balanced 3NT.  Marks the call artificial so
/// the inference reader suppresses the natural notrump reading — without it the
/// sampler would deal responder balanced and mis-score the gamble.
const GAMBLING_3NT: Alert = Alert("1ntx:gambling-3nt");

/// What responder's `2NT` shows in the doubled-1NT runout (A/B knob)
///
/// This governs only the weak, no-five-card-suit responder's both-minor action;
/// a hand with a five-card suit always escapes naturally, in every mode.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Unusual2nt {
    /// `2NT` = both minors, four-four (the scramble); opener picks the better
    /// minor.  The historic behavior, now an opt-in.
    FourFour,
    /// `FourFour` plus a five-five-minors hand bids `2NT` too (above the natural
    /// escape) so opener picks the better fit rather than guess a minor.  A/B'd a
    /// *loss* vs the default, so opt-in only.
    FiveFiveAdd,
    /// No `2NT` relay: a four-four bust bids its longer minor directly at the two
    /// level — one double-exposure instead of the relay's two.  The default: A/B'd
    /// a win over the relay (the extra exposure and higher landing level cost more
    /// than the better minor the relay finds).
    #[default]
    Direct,
}

/// What a *latched* later double means after our natural penalty double of their
/// 1NT — the `(1NT)−X−(2Y)−X` second double (A/B knob, see [`set_latch_style`])
///
/// The mirror of [`DoubleStyle`][super::american::DoubleStyle] on the defensive
/// side: the same penalty-vs-optional question the we-open `1NT−(2X)−X` faced.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LatchStyle {
    /// Pure penalty: the latched double needs a trump stack (4+ with two top
    /// honors) and partner *sits*.  The default — the human "once penalty, always
    /// penalty" rule.
    #[default]
    Penalty,
    /// Cooperative / optional: the latched double shows only 2-3 cards in their
    /// suit with values, and partner *cooperates* (sit on a fit, run when short)
    /// via the general advance-a-double machinery instead of being forced to sit.
    Optional,
}

std::thread_local! {
    /// Whether the floor consults the auction interpretation for known fits
    static INFERENCE_AWARE: Cell<bool> = const { Cell::new(true) };

    /// Whether a weak responder runs from our doubled 1NT (default on)
    static ONE_NT_RUNOUT: Cell<bool> = const { Cell::new(true) };

    /// HCP floor at which responder redoubles a doubled 1NT to play (A/B knob)
    static RUNOUT_XX_MIN: Cell<u8> = const { Cell::new(7) };

    /// Whether the runout is universal: opener also escapes / SOS-redoubles in
    /// the balancing seat, not just the weak responder direct (default on)
    static ONE_NT_RUNOUT_UNIVERSAL: Cell<bool> = const { Cell::new(true) };

    /// What responder's `2NT` shows in the runout (see [`set_unusual_2nt`]);
    /// `Direct` (no relay) by default, A/B'd a win over the `FourFour` relay
    static UNUSUAL_2NT: Cell<Unusual2nt> = const { Cell::new(Unusual2nt::Direct) };

    /// Whether we double the opponents' escape from our doubled 1NT on a trump
    /// stack in their suit (default on; A/B'd +5..+7 IMPs/divergent)
    static PENALIZE_ESCAPE_STACK: Cell<bool> = const { Cell::new(true) };

    /// Whether we double their escape from our 1NT-XX on values, once
    /// responder's business redouble has shown them (default on; A/B'd a win)
    static PENALIZE_ESCAPE_VALUES: Cell<bool> = const { Cell::new(true) };

    /// Whether we encircle (penalty-double) the opponents' escape from our
    /// `1NT-(2NT)-X` — the Unusual-vs-Unusual penalty chase. Default on (it only
    /// fires after our own UvU `X`, so it is dormant unless [`set_uvu`] is on).
    static UVU_ENCIRCLE: Cell<bool> = const { Cell::new(true) };

    /// Whether the "settle" view of Pass is in force (**default on** — see
    /// [`set_settle_floor`]): partner's takeout double is not 100% forcing, so a
    /// hand may pass to *play the top bid* (defend) instead of always advancing
    static SETTLE_FLOOR: Cell<bool> = const { Cell::new(true) };

    /// Whether the "once penalty, always penalty" latch is in force (**on by
    /// default** — DD-measured a penalty-X-bucket win with no regression, see
    /// [`set_penalty_latch`]): after our natural penalty double of their 1NT, our
    /// later doubles read as penalty (sit / leave in) rather than the takeout default
    static PENALTY_LATCH: Cell<bool> = const { Cell::new(true) };

    /// What a latched later double means: [`Penalty`] (stack + sit, **the
    /// default**) or [`Optional`] (2-3 + cooperate). See [`set_latch_style`].
    ///
    /// [`Penalty`]: LatchStyle::Penalty
    /// [`Optional`]: LatchStyle::Optional
    static LATCH_STYLE: Cell<LatchStyle> = const { Cell::new(LatchStyle::Penalty) };

    /// Whether to suppress the doubler's *constructive pulls* of its own penalty
    /// double of their 1NT (**on by default** — DD-measured a clear penalty-X-bucket
    /// win; see [`set_penalty_no_pull`]).  While [`penalty_latched`], the natural
    /// suit and notrump overcall rules still fire for the doubler (a double is not a
    /// bid), so a 15+ balanced doubler "competes" to 2NT/3NT/a major opposite a
    /// likely-broke partner — the dominant defense leak.  On, those pulls step aside
    /// and the doubler defends (Pass) or latch-doubles their escape.
    static PENALTY_NO_PULL: Cell<bool> = const { Cell::new(true) };

    /// Whether a weak advancer runs from their *redoubled* penalty double
    /// (`[1NT, X, XX]`, **on by default** — see [`set_advancer_xx_runout`]).  Their
    /// XX is business (BBA and our own system both: "we make 1NT redoubled"), so a
    /// broke advancer escapes to its long suit rather than sit for the doom.
    static ADVANCER_XX_RUNOUT: Cell<bool> = const { Cell::new(true) };

    /// Whether the *doubler* runs after `[1NT, X, XX, P, P]` comes back around
    /// (**on by default** — see [`set_doubler_xx_runout`]).  Construction-gated:
    /// read once in [`instinct`] so the escape rule lands only in the on book.
    static DOUBLER_XX_RUNOUT: Cell<bool> = const { Cell::new(true) };

    /// HCP floor at which a strong-1NT responder forces game off the floor *in an
    /// undisturbed auction* (A/B knob; see [`set_nt_responder_game_floor`]).  The
    /// authored direct-3NT game force is already 9, but a 9-count *five-card-major*
    /// hand can't bid it (it must transfer) and matches no authored game-forcing
    /// transfer rebid, so it lands here; default **9** (an A/B win: plain +0.0048
    /// IMPs/board vs BBA, PD wash).  Only undisturbed: forcing a thin 9 over a suit
    /// overcall measured a DD loss (the enemy lead/shape beats the thin 3NT), and
    /// over a double the business XX governs ([`SUPPRESS_NT_GF_OVER_DOUBLE`]).
    static NT_RESPONDER_GAME_FLOOR: Cell<u8> = const { Cell::new(9) };

    /// Whether to suppress the strong-1NT responder's natural-3NT game force at
    /// responder's first turn over a *double* of our 1NT (**on by default**; see
    /// [`set_suppress_nt_game_force_over_double`]).  The business redouble is
    /// unlimited — over the double we defend `1NT` redoubled (or escape a long
    /// suit) rather than pull to 3NT.  Isolated A/B win +5.6 IMPs/fired in both
    /// plain and PD (rare, ~0.03%).
    static SUPPRESS_NT_GF_OVER_DOUBLE: Cell<bool> = const { Cell::new(true) };

    /// Whether opener corrects partner's choice-of-games `3NT` to `4M` holding a
    /// *known* eight-card major fit — **undisturbed and with a ruffing doubleton**
    /// (see [`set_correct_3nt_to_major`]).  The 5-3 ruffing edge is single-dummy lore
    /// that double-dummy shares only when the trump-short hand can ruff: a flat
    /// 4-3-3-3 has no ruff (`3NT` keeps its ninth trick against `4M`'s tenth), and a
    /// contested pull walks into a penalty double.  Ungated the correction lost
    /// −0.037 IMPs/board; gated on both (`undisturbed`, `has_ruffing_shortness`) it
    /// wins **+0.0062 plain / +0.0068 PD** (CI ±0.0005, two seeds).  Default **on**.
    static CORRECT_3NT_TO_MAJOR: Cell<bool> = const { Cell::new(true) };

    /// Whether responder's 3NT over a *double* of our 1NT is the **gambling**
    /// long-minor game — six-plus clubs or diamonds, semi-solid, optionally an
    /// outside ace — instead of the suppressed game-force / business-XX baseline.
    /// Off by default (opt-in A/B knob; see [`set_gambling_3nt_over_double`]).  The
    /// minor length floor is fixed at six (it must be a build-time `len` to project
    /// the suit for the reader); the quality and ace gates are runtime knobs.
    static GAMBLING_3NT_OVER_DOUBLE: Cell<bool> = const { Cell::new(false) };

    /// Top-honor floor (count of A/K/Q) the gambling 3NT's long minor must hold —
    /// the "semi-solid" gate.  `0` disables it (length only).  Default `2`.
    static GAMBLING_3NT_TOP_HONORS: Cell<u8> = const { Cell::new(2) };

    /// Whether the gambling 3NT requires the *suit* ace — the ace of the long
    /// minor itself, so the suit runs from the top and buffs total tricks.  On by
    /// default when the package is armed.
    static GAMBLING_3NT_REQUIRE_ACE: Cell<bool> = const { Cell::new(true) };

    /// Whether responder's 4M over a *double* of our 1NT is loosened to a
    /// **preemptive** long-major game — six-plus major plus a modest HCP floor —
    /// instead of needing full game values.  Off by default (opt-in A/B knob; see
    /// [`set_preempt_4m_over_double`]).  The undisturbed / over-an-overcall 4M is
    /// unchanged: this only adds a rule in the doubled-1NT runout.
    static PREEMPT_4M_OVER_DOUBLE: Cell<bool> = const { Cell::new(false) };

    /// The HCP floor for the preemptive 4M long-major game (see
    /// [`PREEMPT_4M_OVER_DOUBLE`]).  Default `5` — a source of tricks, not a bust.
    static PREEMPT_4M_FLOOR: Cell<u8> = const { Cell::new(5) };

    /// Top-honor floor (count of A/K/Q) the preemptive 4M's long major must hold —
    /// the same "semi-solid" gate the gambling 3NT uses, so 4M is a *quality* long
    /// major, not any six-bagger (`0` = length only).  Default `2`: a ragged six-card
    /// major jumping to game fails double-dummy exactly as a ragged minor 3NT does.
    static PREEMPT_4M_TOP_HONORS: Cell<u8> = const { Cell::new(2) };

    /// Whether the preemptive 4M requires the *trump* ace — the ace of the long
    /// major, a sure trump trick and control that buffs total tricks.  On by default
    /// when the package is armed.
    static PREEMPT_4M_REQUIRE_ACE: Cell<bool> = const { Cell::new(true) };
}

/// Responder runs from a doubled 1NT below this many HCP; with more, 1NT-X
/// rates to make opposite a 15–17 opener, so sit (or redouble — see
/// [`set_runout_xx_min`]).  A named knob for A/B tuning.
const RUNOUT_MAX_HCP: u8 = 8;

/// Enable or disable inference-aware instinct rules on the current thread
///
/// For A/B measurement only (see the `inference-floor` example): with it
/// disabled the floor ignores partner's shown shape, falling back to the
/// shape-blind 3NT / six-card-major game selection.  The flag is read at
/// classification time and is per-thread; classify on the thread that set it.
#[doc(hidden)]
pub fn set_inference_aware(enabled: bool) {
    INFERENCE_AWARE.with(|flag| flag.set(enabled));
}

/// The floor is consulting the auction interpretation (see [`set_inference_aware`])
fn inference_aware() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| INFERENCE_AWARE.with(Cell::get))
}

/// Enable or disable the doubled-1NT runout on the current thread
///
/// On by default: when our 1NT is doubled, a weak responder escapes to its
/// longest five-plus-card suit instead of sitting for the penalty (and opener
/// passes that escape).  Disable to fall back to the natural floor — Pass.  For
/// A/B measurement (see the `ab-one-nt-runout` example); read at classification
/// time, per-thread.
#[doc(hidden)]
pub fn set_one_nt_runout(enabled: bool) {
    ONE_NT_RUNOUT.with(|flag| flag.set(enabled));
}

/// The doubled-1NT runout is enabled (see [`set_one_nt_runout`])
fn one_nt_runout_enabled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| ONE_NT_RUNOUT.with(Cell::get))
}

/// Enable or disable the "settle" view of Pass on the current thread
///
/// **On by default** (A/B'd a clear win — +0.26 IMPs/board vul none, +0.37 vul
/// both, on `ab-settle-floor`'s perfect-defense measure).  The floor treats Pass
/// as *playing the top bid*: partner's takeout double is not 100% forcing, so a
/// hand with a good penalty (length behind their doubled suit) defends instead of
/// advancing, and a four-level advance becomes a *free bid* requiring values.
/// Disable to recover the old always-advance floor.  For A/B measurement (see the
/// `ab-settle-floor` example); read at classification time, per-thread.
#[doc(hidden)]
pub fn set_settle_floor(enabled: bool) {
    SETTLE_FLOOR.with(|flag| flag.set(enabled));
}

/// The "settle" view of Pass is enabled (see [`set_settle_floor`])
fn settle_floor() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| SETTLE_FLOOR.with(Cell::get))
}

/// Set the HCP floor at which responder redoubles a doubled 1NT to play
///
/// For A/B measurement (see `ab-one-nt-runout --xx-min`).  XX shows values and
/// suggests defending 1NT redoubled; keyed on raw HCP (defensive strength), not
/// the shape-upgraded point count — a shapely weak hand should run, not sit.
#[doc(hidden)]
pub fn set_runout_xx_min(floor: u8) {
    RUNOUT_XX_MIN.with(|cell| cell.set(floor));
}

/// Set the HCP floor at which a strong-1NT responder forces game off the floor
///
/// For A/B measurement.  Default 10; lowering to 9 closes the post-transfer seam
/// where a 9-count five-card-major hand transfers, finds no authored game-forcing
/// rebid, and stalls below the floor's trigger.  The authored direct-3NT force is
/// already 9, so 9 here is symmetric.
#[doc(hidden)]
pub fn set_nt_responder_game_floor(floor: u8) {
    NT_RESPONDER_GAME_FLOOR.with(|cell| cell.set(floor));
}

/// The current strong-1NT responder game-force floor (see
/// [`set_nt_responder_game_floor`])
fn nt_responder_game_floor() -> u8 {
    NT_RESPONDER_GAME_FLOOR.with(Cell::get)
}

/// Suppress (or not) the strong-1NT responder's 3NT game force over a double of
/// our 1NT — see [`SUPPRESS_NT_GF_OVER_DOUBLE`].  For A/B measurement.
#[doc(hidden)]
pub fn set_suppress_nt_game_force_over_double(suppress: bool) {
    SUPPRESS_NT_GF_OVER_DOUBLE.with(|cell| cell.set(suppress));
}

/// Author whether opener corrects partner's choice-of-games `3NT` to `4M` with a
/// known eight-card major fit, undisturbed and holding a ruffing doubleton (see
/// [`CORRECT_3NT_TO_MAJOR`]).  Default on; disable for the off arm of an A/B.
#[doc(hidden)]
pub fn set_correct_3nt_to_major(correct: bool) {
    CORRECT_3NT_TO_MAJOR.with(|cell| cell.set(correct));
}

/// Whether the strong-1NT responder's 3NT game force is allowed in the current
/// auction.  It steps aside only at responder's first turn over a double of our
/// 1NT (when [`SUPPRESS_NT_GF_OVER_DOUBLE`] is set) — the business-XX / escape
/// runout governs instead.  Over a suit overcall it bids as usual (no XX there,
/// the opponents are not penalizing).
fn nt_game_force_3nt_allowed() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        !(SUPPRESS_NT_GF_OVER_DOUBLE.with(Cell::get) && responder_one_nt_runout_now(context))
    })
}

/// Responder holds redouble values: raw HCP at or above the [`RUNOUT_XX_MIN`]
/// floor (see [`set_runout_xx_min`])
fn responder_has_xx_values() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, _: &Context<'_>| {
        let hcp: u8 = Suit::ASC
            .iter()
            .map(|&suit| holding_hcp::<u8>(hand[suit]))
            .sum();
        hcp >= RUNOUT_XX_MIN.with(Cell::get)
    })
}

/// Author whether responder's 3NT over a double of our 1NT is the gambling
/// long-minor game (see [`GAMBLING_3NT_OVER_DOUBLE`]).  For A/B measurement.
#[doc(hidden)]
pub fn set_gambling_3nt_over_double(on: bool) {
    GAMBLING_3NT_OVER_DOUBLE.with(|cell| cell.set(on));
}

/// Set the gambling 3NT's "semi-solid" top-honor floor (see
/// [`GAMBLING_3NT_TOP_HONORS`]; `0` = length only).  For A/B measurement.
#[doc(hidden)]
pub fn set_gambling_3nt_top_honors(floor: u8) {
    GAMBLING_3NT_TOP_HONORS.with(|cell| cell.set(floor));
}

/// Author whether the gambling 3NT requires an outside ace (see
/// [`GAMBLING_3NT_REQUIRE_ACE`]).  For A/B measurement.
#[doc(hidden)]
pub fn set_gambling_3nt_require_ace(on: bool) {
    GAMBLING_3NT_REQUIRE_ACE.with(|cell| cell.set(on));
}

/// Author whether responder's 4M over a double of our 1NT is the preemptive
/// long-major game (see [`PREEMPT_4M_OVER_DOUBLE`]).  For A/B measurement.
#[doc(hidden)]
pub fn set_preempt_4m_over_double(on: bool) {
    PREEMPT_4M_OVER_DOUBLE.with(|cell| cell.set(on));
}

/// Set the HCP floor for the preemptive 4M long-major game (see
/// [`PREEMPT_4M_FLOOR`]).  For A/B measurement.
#[doc(hidden)]
pub fn set_preempt_4m_floor(floor: u8) {
    PREEMPT_4M_FLOOR.with(|cell| cell.set(floor));
}

/// Set the preemptive 4M's "semi-solid" top-honor floor (see
/// [`PREEMPT_4M_TOP_HONORS`]; `0` = length only).  For A/B measurement.
#[doc(hidden)]
pub fn set_preempt_4m_top_honors(floor: u8) {
    PREEMPT_4M_TOP_HONORS.with(|cell| cell.set(floor));
}

/// Author whether the preemptive 4M requires the trump ace (see
/// [`PREEMPT_4M_REQUIRE_ACE`]).  For A/B measurement.
#[doc(hidden)]
pub fn set_preempt_4m_require_ace(on: bool) {
    PREEMPT_4M_REQUIRE_ACE.with(|cell| cell.set(on));
}

/// The gambling long-minor 3NT is armed (see [`set_gambling_3nt_over_double`])
fn gambling_3nt_authored() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| GAMBLING_3NT_OVER_DOUBLE.with(Cell::get))
}

/// The gambling 3NT's long minor is semi-solid: it holds at least
/// [`GAMBLING_3NT_TOP_HONORS`] of the top three honors (A/K/Q).  An eval-time
/// knob (not the build-time [`top_honors`][super::constraint::top_honors]) so the
/// A/B can flip length-only vs semi-solid per board without rebuilding.
fn gambling_3nt_semisolid(minor: Suit) -> Cons<impl Constraint + Clone> {
    described("a semi-solid suit", move |hand: Hand, _: &Context<'_>| {
        let top = [Rank::A, Rank::K, Rank::Q]
            .into_iter()
            .filter(|&rank| hand[minor].contains(rank))
            .count() as u8;
        top >= GAMBLING_3NT_TOP_HONORS.with(Cell::get)
    })
}

/// The gambling 3NT's long minor is headed by its own ace — the suit ace cashes
/// and buffs total tricks (the running suit loses no top trick to a missing ace).
/// Vacuously satisfied when the ace requirement is off.
fn gambling_3nt_suit_ace(minor: Suit) -> Cons<impl Constraint + Clone> {
    described("the suit ace", move |hand: Hand, _: &Context<'_>| {
        !GAMBLING_3NT_REQUIRE_ACE.with(Cell::get) || hand[minor].contains(Rank::A)
    })
}

/// The preemptive long-major 4M is armed (see [`set_preempt_4m_over_double`])
fn preempt_4m_authored() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| PREEMPT_4M_OVER_DOUBLE.with(Cell::get))
}

/// Responder holds at least the preemptive-4M HCP floor (see [`PREEMPT_4M_FLOOR`])
fn preempt_4m_values() -> Cons<impl Constraint + Clone> {
    described("a modest opening", |hand: Hand, _: &Context<'_>| {
        let hcp: u8 = Suit::ASC
            .iter()
            .map(|&suit| holding_hcp::<u8>(hand[suit]))
            .sum();
        hcp >= PREEMPT_4M_FLOOR.with(Cell::get)
    })
}

/// The preemptive 4M's long major is semi-solid: it holds at least
/// [`PREEMPT_4M_TOP_HONORS`] of the top three honors (A/K/Q).  The major's mirror
/// of [`gambling_3nt_semisolid`].
fn preempt_4m_semisolid(major: Suit) -> Cons<impl Constraint + Clone> {
    described("a semi-solid major", move |hand: Hand, _: &Context<'_>| {
        let top = [Rank::A, Rank::K, Rank::Q]
            .into_iter()
            .filter(|&rank| hand[major].contains(rank))
            .count() as u8;
        top >= PREEMPT_4M_TOP_HONORS.with(Cell::get)
    })
}

/// The preemptive 4M's long major is headed by the trump ace — a sure trump trick
/// and control that buffs total tricks.  Vacuously satisfied when off.
fn preempt_4m_trump_ace(major: Suit) -> Cons<impl Constraint + Clone> {
    described("the trump ace", move |hand: Hand, _: &Context<'_>| {
        !PREEMPT_4M_REQUIRE_ACE.with(Cell::get) || hand[major].contains(Rank::A)
    })
}

/// Enable or disable the *universal* doubled-1NT runout on the current thread
///
/// On by default: opener too escapes its own five-plus-card suit, and SOS-
/// redoubles (the balancing redouble) when it has none, in the seat where the
/// double comes back to it with a weak partner.  Off restricts the runout to the
/// weak responder's direct seat.  For A/B measurement (see
/// `ab-one-nt-runout --universal`); read at classification time, per-thread.
#[doc(hidden)]
pub fn set_one_nt_runout_universal(enabled: bool) {
    ONE_NT_RUNOUT_UNIVERSAL.with(|flag| flag.set(enabled));
}

/// The universal runout is enabled (see [`set_one_nt_runout_universal`])
fn one_nt_runout_universal() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| ONE_NT_RUNOUT_UNIVERSAL.with(Cell::get))
}

/// Set what responder's `2NT` shows in the doubled-1NT runout
///
/// For A/B measurement (see `ab-one-nt-runout --compare minors5|direct`); read
/// at classification time, per-thread.  [`Unusual2nt::Direct`] is the default.
#[doc(hidden)]
pub fn set_unusual_2nt(mode: Unusual2nt) {
    UNUSUAL_2NT.with(|cell| cell.set(mode));
}

/// Responder's `2NT` is configured to `mode` (see [`set_unusual_2nt`])
fn unusual_2nt_is(mode: Unusual2nt) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, _: &Context<'_>| UNUSUAL_2NT.with(Cell::get) == mode)
}

/// Enable or disable the trump-stack penalty double of the opponents' escape
///
/// For A/B measurement (see `ab-one-nt-runout --compare escape-stack`); read at
/// classification time, per-thread.  On by default.
#[doc(hidden)]
pub fn set_penalize_escape_stack(enabled: bool) {
    PENALIZE_ESCAPE_STACK.with(|flag| flag.set(enabled));
}

/// The trump-stack escape penalty is enabled (see [`set_penalize_escape_stack`])
fn penalize_escape_stack_enabled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| PENALIZE_ESCAPE_STACK.with(Cell::get))
}

/// Enable or disable the values penalty double of their escape from our 1NT-XX
///
/// For A/B measurement (see `ab-one-nt-runout --compare escape-values`); read at
/// classification time, per-thread.  On by default.
#[doc(hidden)]
pub fn set_penalize_escape_values(enabled: bool) {
    PENALIZE_ESCAPE_VALUES.with(|flag| flag.set(enabled));
}

/// The values escape penalty is enabled (see [`set_penalize_escape_values`])
fn penalize_escape_values_enabled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| PENALIZE_ESCAPE_VALUES.with(Cell::get))
}

/// Enable or disable the Unusual-vs-Unusual penalty chase after `1NT-(2NT)-X`
///
/// "All our doubles are penalty from the first X on"; a pass conveys inability
/// to punish *this* contract.  The responder `X` itself lives in the american
/// book ([`set_uvu`][crate::bidding::american::set_uvu]); this only adds the
/// follow-up chase of the opponents' escape.  Read at classification time,
/// per-thread.  On by default — but dormant unless our UvU `X` was bid.
#[doc(hidden)]
pub fn set_uvu_encircle(enabled: bool) {
    UVU_ENCIRCLE.with(|flag| flag.set(enabled));
}

/// The UvU penalty chase is enabled (see [`set_uvu_encircle`])
fn uvu_encircle_enabled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| UVU_ENCIRCLE.with(Cell::get))
}

/// Partner opened a strong 1NT, RHO doubled it, and it is responder's first
/// turn — the runout situation.  The double need not be penalty: left in, any
/// double of 1NT plays for the penalty, so a weak responder escapes regardless.
fn responder_one_nt_runout_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let n = auction.len();
    our_strong_notrump(context, 1, true)
        && auction.last() == Some(&Call::Double)
        && n >= 2
        && matches!(auction[n - 2], Call::Bid(bid) if bid == Bid::new(1, Strain::Notrump))
}

/// [`responder_one_nt_runout_now`] as a hand-ignoring [`Constraint`]
fn responder_one_nt_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| responder_one_nt_runout_now(context))
}

/// We opened a strong 1NT, LHO doubled, partner ran out to a suit, and it is
/// our turn again.  Responder ran because it is weak, so it captains the
/// auction: opener passes rather than read the escape as a natural new suit.
fn opener_after_one_nt_runout_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    if !our_strong_notrump(context, 1, false) {
        return false;
    }
    let Some((index, _)) = opening_bid(auction) else {
        return false;
    };
    auction.get(index + 1) == Some(&Call::Double)
        && matches!(
            auction.get(index + 2),
            Some(&Call::Bid(bid)) if bid.strain.suit().is_some()
        )
}

/// [`opener_after_one_nt_runout_now`] as a hand-ignoring [`Constraint`]
fn opener_after_one_nt_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_after_one_nt_runout_now(context))
}

/// We opened a strong 1NT, LHO doubled, partner scrambled `2NT` (both minors),
/// and it is our turn — name the better minor at the three level.
fn opener_after_one_nt_minors_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    if !our_strong_notrump(context, 1, false) {
        return false;
    }
    let Some((index, _)) = opening_bid(auction) else {
        return false;
    };
    auction.get(index + 1) == Some(&Call::Double)
        && auction.get(index + 2) == Some(&Call::Bid(Bid::new(2, Strain::Notrump)))
}

/// [`opener_after_one_nt_minors_now`] as a hand-ignoring [`Constraint`]
fn opener_after_one_nt_minors() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_after_one_nt_minors_now(context))
}

/// Diamonds are at least as long as clubs (the minor to name over a scramble)
fn longer_diamonds() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, _: &Context<'_>| hand[Suit::Diamonds].len() >= hand[Suit::Clubs].len())
}

/// We opened a strong 1NT, LHO doubled, partner and the doubler's partner both
/// passed, and it is our turn — the balancing seat.  Partner had no escape, so
/// it is weak: opener may run its own suit or SOS-redouble rather than sit.
fn opener_balancing_runout_now(context: &Context<'_>) -> bool {
    if !our_strong_notrump(context, 1, false) {
        return false;
    }
    let auction = context.auction();
    let Some((index, _)) = opening_bid(auction) else {
        return false;
    };
    auction.len() == index + 4
        && auction.get(index + 1) == Some(&Call::Double)
        && auction[index + 2] == Call::Pass
        && auction[index + 3] == Call::Pass
}

/// [`opener_balancing_runout_now`] as a hand-ignoring [`Constraint`]
fn opener_balancing_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_balancing_runout_now(context))
}

/// Opener SOS-redoubled (the balancing redouble) and it is back to responder:
/// pick a suit, four-card suits included — opener has none of its own.
fn responder_after_opener_sos_now(context: &Context<'_>) -> bool {
    if !our_strong_notrump(context, 1, true) {
        return false;
    }
    let auction = context.auction();
    let Some((index, _)) = opening_bid(auction) else {
        return false;
    };
    auction.len() >= index + 6
        && auction.get(index + 1) == Some(&Call::Double)
        && auction[index + 2] == Call::Pass
        && auction[index + 3] == Call::Pass
        && auction[index + 4] == Call::Redouble
        && auction[index + 5..].iter().all(|&call| call == Call::Pass)
}

/// [`responder_after_opener_sos_now`] as a hand-ignoring [`Constraint`]
fn responder_after_opener_sos() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| responder_after_opener_sos_now(context))
}

/// Responder answered our SOS redouble with a suit; pass it (responder captains
/// the rescue) rather than read it as a natural new suit and raise.
fn opener_after_responder_sos_now(context: &Context<'_>) -> bool {
    if !our_strong_notrump(context, 1, false) {
        return false;
    }
    let auction = context.auction();
    let Some((index, _)) = opening_bid(auction) else {
        return false;
    };
    auction.len() >= index + 8
        && auction.get(index + 1) == Some(&Call::Double)
        && auction[index + 2] == Call::Pass
        && auction[index + 3] == Call::Pass
        && auction[index + 4] == Call::Redouble
        && auction[index + 5] == Call::Pass
        && matches!(auction[index + 6], Call::Bid(bid) if bid.strain.suit().is_some())
        && auction[index + 7..].iter().all(|&call| call == Call::Pass)
}

/// [`opener_after_responder_sos_now`] as a hand-ignoring [`Constraint`]
fn opener_after_responder_sos() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_after_responder_sos_now(context))
}

/// The opponents have escaped our doubled (or redoubled) 1NT and it is our turn
///
/// Our side opened 1NT, LHO doubled, and since then we have only passed or
/// (re)doubled — never made a contract bid — so the live suit contract is
/// *theirs* (their escape), not a suit of ours.  Returns the index of our
/// opening 1NT when the pattern holds.  Counting our own doubles as "no contract
/// bid" is what lets the penalty chase recurse as they keep running.
fn our_doubled_one_nt_escape(context: &Context<'_>) -> Option<usize> {
    let auction = context.auction();
    let (index, bid) = opening_bid(auction)?;
    // Our side is to act, and opened a 1NT that LHO doubled.
    if index % 2 != auction.len() % 2 || bid != Bid::new(1, Strain::Notrump) {
        return None;
    }
    if auction.get(index + 1) != Some(&Call::Double) {
        return None;
    }
    // We made no contract bid since the opening: the live suit contract is the
    // opponents' escape, not a suit of ours that they doubled.
    let we_only_doubled = auction
        .iter()
        .enumerate()
        .skip(index + 1)
        .filter(|(i, _)| i % 2 == index % 2)
        .all(|(_, &call)| !matches!(call, Call::Bid(_)));
    if !we_only_doubled {
        return None;
    }
    // The live contract is a suit at the three level or below.
    context
        .last_bid()
        .filter(|bid| bid.strain.suit().is_some() && bid.level.get() <= 3)?;
    Some(index)
}

/// Their escape from our (re)doubled 1NT is live and undoubled — we may double
/// it for penalty (see [`our_doubled_one_nt_escape`])
fn opp_escaped_our_nt_undoubled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        our_doubled_one_nt_escape(context).is_some() && context.penalty() == Penalty::Undoubled
    })
}

/// Their escape is undoubled *and* responder's business redouble (1NT-X-XX) has
/// already shown the values — combined we hold the balance, so a values double
/// is sound without a personal stack (see [`our_doubled_one_nt_escape`])
fn opp_escaped_our_business_xx() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        our_doubled_one_nt_escape(context).is_some_and(|index| {
            context.penalty() == Penalty::Undoubled
                && context.auction().get(index + 2) == Some(&Call::Redouble)
        })
    })
}

/// We doubled their escape for penalty; partner leaves it in rather than read it
/// as the takeout the `advancing_a_double` default would advance (see
/// [`our_doubled_one_nt_escape`])
fn leave_in_escape_penalty() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        our_doubled_one_nt_escape(context).is_some() && context.penalty() == Penalty::Doubled
    })
}

/// The opponents have escaped our `1NT-(2NT)-X` penalty double and it is our turn
///
/// Our side opened 1NT, RHO overcalled a (both-minors) 2NT, our side doubled it
/// for penalty, and since then we have only passed or doubled — so the live suit
/// contract is *theirs* (their escape from the X).  Returns the index of our
/// opening 1NT when the pattern holds; mirrors [`our_doubled_one_nt_escape`] for
/// the Unusual-vs-Unusual chase ([`set_uvu_encircle`]).
fn our_uvu_penalty_escape(context: &Context<'_>) -> Option<usize> {
    let auction = context.auction();
    let (index, bid) = opening_bid(auction)?;
    // Our side is to act, opened 1NT, RHO overcalled 2NT, our side doubled it.
    if index % 2 != auction.len() % 2 || bid != Bid::new(1, Strain::Notrump) {
        return None;
    }
    if auction.get(index + 1) != Some(&Call::Bid(Bid::new(2, Strain::Notrump)))
        || auction.get(index + 2) != Some(&Call::Double)
    {
        return None;
    }
    // We made no contract bid since the opening: the live suit is their escape.
    let we_only_doubled = auction
        .iter()
        .enumerate()
        .skip(index + 1)
        .filter(|(i, _)| i % 2 == index % 2)
        .all(|(_, &call)| !matches!(call, Call::Bid(_)));
    if !we_only_doubled {
        return None;
    }
    // The live contract is a suit at the three level or below.
    context
        .last_bid()
        .filter(|bid| bid.strain.suit().is_some() && bid.level.get() <= 3)?;
    Some(index)
}

/// Their escape from our UvU penalty `X` is live and undoubled — we may double
/// it for penalty (see [`our_uvu_penalty_escape`])
fn opp_escaped_our_uvu_undoubled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        our_uvu_penalty_escape(context).is_some() && context.penalty() == Penalty::Undoubled
    })
}

/// We doubled their UvU escape for penalty; partner leaves it in (see
/// [`our_uvu_penalty_escape`])
fn leave_in_uvu_penalty() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        our_uvu_penalty_escape(context).is_some() && context.penalty() == Penalty::Doubled
    })
}

/// Partner's takeout double is live: the auction ends `… (bid) X (Pass)`
///
/// Mechanically: the last two calls are partner's double and RHO's pass, and
/// the doubled contract is their suit bid at the three level or below —
/// doubles of notrump or of game-level contracts read as penalty, not as a
/// request to act.
fn advancing_a_double_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let n = auction.len();
    n >= 2
        && auction[n - 1] == Call::Pass
        && auction[n - 2] == Call::Double
        && context
            .last_bid()
            .is_some_and(|bid| bid.strain.suit().is_some() && bid.level.get() <= 3)
}

/// [`advancing_a_double_now`] as a hand-ignoring [`Constraint`] for the ladder
fn advancing_a_double() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| advancing_a_double_now(context))
}

/// We already hold an eight-card fit in some suit: our length there plus the
/// minimum partner has shown ([`Inferences`]) reaches eight.
///
/// Reads the shown *minimum* (length partner cannot lack), so it fires only on a
/// fit the calls have promised.  Used by the free-bid gate to stop inventing a
/// new suit once a trump suit is already found.
fn has_fit(hand: Hand, context: &Context<'_>) -> bool {
    let inferences = Inferences::read(context);
    let partner = inferences.partner();
    Suit::ASC
        .iter()
        .any(|&suit| hand[suit].len() + usize::from(partner.length(suit).min) >= 8)
}

/// The free-bid gate on an advance of partner's double into a new suit at `level` (see
/// [`set_settle_floor`])
///
/// A no-op unless the settle floor is on, and only ever gates the **four level**:
/// a new suit there is a *free bid* — partner's takeout double does not force us to
/// the four level, so voluntarily climbing must show values (~11+ points) and not
/// invent a suit once we already [`has_fit`].  Without them the hand stays lower (a
/// three-level take-out, the notrump escape) or defends (see [`doubled_suit_length`]).
/// One- to three-level advances are untouched: the leak was the captive `4♣x`, not
/// the cheap take-out a bust owes partner's double.
fn free_bid_gate(level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        level < 4
            || !SETTLE_FLOOR.with(Cell::get)
            || (point_count(hand) >= 11 && !has_fit(hand, context))
    })
}

/// Four-plus cards in the doubled suit: a *good penalty* behind their suit
///
/// The milder sibling of [`doubled_suit_stack`] — length without the two top
/// honors.  Opposite partner's takeout double (partner is short their suit, with
/// values), sitting with four trumps behind declarer beats taking out: pass and
/// play their doubled contract.  Drives the settle floor's defend pass.
fn doubled_suit_length() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        context
            .last_bid()
            .and_then(|bid| bid.strain.suit())
            .is_some_and(|suit| hand[suit].len() >= 4)
    })
}

/// The doubled suit's length is within `range` — the cooperative-double gate
///
/// The 2-3-card holding behind their suit that makes the latched double *optional*
/// (see [`LatchStyle::Optional`]): some length and values, but partner decides.
fn doubled_suit_len(range: core::ops::RangeInclusive<usize>) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        context
            .last_bid()
            .and_then(|bid| bid.strain.suit())
            .is_some_and(|suit| range.contains(&hand[suit].len()))
    })
}

/// A trump stack in the doubled suit: four-plus cards with two top honors
///
/// The one holding that converts partner's takeout double into penalties.
fn doubled_suit_stack() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        context
            .last_bid()
            .and_then(|bid| bid.strain.suit())
            .is_some_and(|suit| {
                let holding = hand[suit];
                let honors = [Rank::A, Rank::K, Rank::Q]
                    .into_iter()
                    .filter(|&rank| holding.contains(rank))
                    .count();
                holding.len() >= 4 && honors >= 2
            })
    })
}

/// Our side has not bid yet (doubles and passes do not count)
///
/// The anchor for overcall-shaped actions: once we have shown a suit or
/// notrump, instinct competes by raising or doubling instead.
fn we_have_not_bid() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        !Suit::ASC
            .into_iter()
            .map(Strain::from)
            .chain([Strain::Notrump])
            .any(|strain| context.we_bid(strain))
    })
}

/// The opponents' undoubled suit bid at most `level` is the call to beat
///
/// This is the legality *and* sanity anchor for instinct doubles: the last
/// non-pass call is an opposing suit bid, not yet doubled, low enough that a
/// double still reads as takeout.
fn their_live_bid_at_most(level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        context.penalty() == Penalty::Undoubled
            && context
                .last_bid()
                .is_some_and(|bid| bid.strain.suit().is_some() && bid.level.get() <= level)
            && context
                .auction()
                .iter()
                .rposition(|&call| call != Call::Pass)
                .is_some_and(|index| (context.auction().len() - index) % 2 == 1)
    })
}

/// The strain is still biddable at or below the given level
fn level_available(level: u8, strain: Strain) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        context
            .min_level(strain)
            .is_some_and(|min| min.get() <= level)
    })
}

/// The opening bid (first non-pass call) and its index, if it is a bid
fn opening_bid(auction: &[Call]) -> Option<(usize, Bid)> {
    let index = auction.iter().position(|&call| call != Call::Pass)?;
    match auction[index] {
        Call::Bid(bid) => Some((index, bid)),
        _ => None,
    }
}

/// Our side opened a strong notrump of `level`, and the player to act is its
/// opener (`partner == false`) or its responder (`partner == true`)
///
/// This is one of the two conventions instinct reads (the other is the strong
/// 2♣ — see [`forcing_two_clubs_response`]): a strong notrump opening is the
/// anchor for completing transfers and refusing to pass below a forced game,
/// the deep conventional structures the book may not author.
fn our_strong_notrump(context: &Context<'_>, level: u8, partner: bool) -> bool {
    let auction = context.auction();
    let Some((index, bid)) = opening_bid(auction) else {
        return false;
    };
    // Our side owns the indices sharing the player-to-act's parity.
    if index % 2 != auction.len() % 2 {
        return false;
    }
    if bid.strain != Strain::Notrump || bid.level.get() != level {
        return false;
    }
    // Seats four apart are the same player; two apart are partners.
    match (auction.len() - index) % 4 {
        0 => !partner,
        2 => partner,
        _ => false,
    }
}

/// Partner's call immediately before ours, if it was a bid
fn partner_last_call(auction: &[Call]) -> Option<Bid> {
    match auction.len().checked_sub(2).map(|i| auction[i]) {
        Some(Call::Bid(bid)) => Some(bid),
        _ => None,
    }
}

/// The current contract is below game: no bid, or a partscore-level suit bid
fn below_game() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| below_game_now(context))
}

/// Partner's last call was a choice-of-games `3NT` we may correct to `4M`, and
/// the correction is enabled (see [`CORRECT_3NT_TO_MAJOR`])
///
/// Pair with a known eight-card major fit: a responder who transferred (showing
/// five) then bid `3NT` offers the choice, and opposite three-card support the
/// 5-3 fit out-scores notrump (`answer_transfer_spade_single`).  Keyed only on
/// the `3NT`, so it fires in contested auctions too (`1NT–(2♦)–…–3NT`).
fn correct_3nt_to_major_now() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        CORRECT_3NT_TO_MAJOR.with(Cell::get)
            && context.last_bid() == Some(Bid::new(3, Strain::Notrump))
    })
}

/// A ruffing doubleton — any suit of two cards or fewer.  For the balanced 1NT
/// opener this is exactly *not* a flat 4-3-3-3, the one shape with no ruffing
/// value: the 3NT→4M correction gains its extra trick only when the trump-short
/// hand can ruff, so opposite responder's balanced transferred five it stands
/// down on the flat hand and leaves the better game (3NT) in place.
fn has_ruffing_shortness() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, _: &Context<'_>| Suit::ASC.iter().any(|&suit| hand[suit].len() <= 2))
}

/// The current contract is below game (the predicate body of [`below_game`])
fn below_game_now(context: &Context<'_>) -> bool {
    context.last_bid().is_none_or(|bid| {
        let level = bid.level.get();
        level <= 2 || (level == 3 && bid.strain != Strain::Notrump)
    })
}

/// The current contract is below slam: nothing above the five level yet
fn below_slam() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.last_bid().is_none_or(|bid| bid.level.get() <= 5))
}

/// Our side holds at least `threshold` combined points: our exact count plus the
/// *sound floor* of partner's shown points ([`Inferences`]), so the true total
/// is never less than the test admits
///
/// This is the general game/slam trigger.  Where the special-cased forces (a
/// strong-notrump responder, a strong 2♣) encode a single auction, this fires on
/// *any* auction whose shown strength reaches a milestone — the inference floor
/// makes it sound, never an overbid on a hand that could be weaker than counted.
///
/// [`Inferences`]: super::inference::Inferences
fn combined_points(threshold: u8) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        let partner_min = Inferences::read(context).partner().points.min;
        u16::from(point_count(hand)) + u16::from(partner_min) >= u16::from(threshold)
    })
}

/// Partner opened a strong notrump of `level` (we are the responder)
fn partner_strong_notrump(level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| our_strong_notrump(context, level, true))
}

/// We opened a strong notrump and partner forced past invitation with a
/// three-level suit bid — so passing below game is wrong, whatever our hand
fn opener_forced_past_invitation(context: &Context<'_>) -> bool {
    (our_strong_notrump(context, 1, false) || our_strong_notrump(context, 2, false))
        && partner_last_call(context.auction())
            .is_some_and(|bid| bid.level.get() == 3 && bid.strain != Strain::Notrump)
}

/// Our side opened a strong 2♣ and responder answered past the double negative
///
/// The artificial `2♣` promises 22+ and is forcing — but for one round only.
/// Responder's *answer* settles the game force: the 0–3 HCP double negative
/// (`2♥`) keeps open the option to stop short, while every other response — the
/// waiting `2♦` or a natural positive — commits *both* partners to at least
/// game.  So the force is read off responder's call, not off the 2♣ opening.
/// (Interference, where responder's seat holds a pass or double rather than a
/// response, is out of scope and reads as not forced.)
fn forcing_two_clubs_response(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let Some((index, bid)) = opening_bid(auction) else {
        return false;
    };
    // The player to act must be on the opening side — opener or responder.
    if index % 2 != auction.len() % 2 {
        return false;
    }
    if bid != Bid::new(2, Strain::Clubs) {
        return false;
    }
    // Responder sits two seats past the opening; the force is on once that
    // answer is in and is any bid other than the double-negative 2♥.
    matches!(
        auction.get(index + 2),
        Some(&Call::Bid(response)) if response != Bid::new(2, Strain::Hearts)
    )
}

/// We are sitting for a penalty: the live contract is the opponents' bid
/// doubled (or redoubled) by our side
///
/// Since a side may only double the other, a doubled contract whose last bid is
/// theirs was doubled by us — passing it out is the intended penalty action.
fn penalizing(context: &Context<'_>) -> bool {
    let auction = context.auction();
    context.penalty() != Penalty::Undoubled
        && auction
            .iter()
            .rposition(|call| matches!(call, Call::Bid(_)))
            .is_some_and(|index| (auction.len() - index) % 2 == 1)
}

/// Instinct's reading of an auction: the system intent the laws-only [`Context`]
/// deliberately omits, reconstructed from the immutable auction on demand
///
/// There is no per-classification scratchpad to cache this in, so each flag is
/// recovered by a short walk of the auction whenever the floor consults it.
/// Every flag here is *hand-independent* — it follows from the calls alone — so
/// hand-conditioned forces (a strong-notrump responder who holds game values)
/// stay as ordinary [`Constraint`]s rather than living here.
#[derive(Clone, Copy, Debug)]
struct Interpretation {
    /// Our side is committed to at least game by a prior call: a strong 2♣
    /// whose response cleared the double negative, or an opener forced past
    /// invitation opposite our strong notrump.
    forced_to_game: bool,
    /// We are sitting for our own penalty double, so passing below game is the
    /// intended action rather than a missed game.
    penalizing: bool,
}

impl Interpretation {
    /// Read the auction's intent from its [`Context`]
    fn read(context: &Context<'_>) -> Self {
        Self {
            forced_to_game: forcing_two_clubs_response(context)
                || opener_forced_past_invitation(context),
            penalizing: penalizing(context),
        }
    }
}

/// A prior call has committed our side to game (see [`Interpretation`])
fn auction_forces_game() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| Interpretation::read(context).forced_to_game)
}

/// We are not sitting for a penalty double of our own (see [`Interpretation`])
///
/// A game force forbids passing below game — *unless* we are penalizing the
/// opponents, where passing their doubled contract out is the whole point.
/// There the forced-to-game rules step aside and let the natural defense —
/// including the [advance][advancing_a_double] of partner's penalty double —
/// govern.
fn not_penalizing() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| !Interpretation::read(context).penalizing)
}

/// The opponents have made nothing but passes (see [`Context::undisturbed`])
fn undisturbed() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.undisturbed())
}

/// Enable or disable the penalty-double latch on the current thread
///
/// **On by default** (DD-measured a clear penalty-X-bucket win with no regression:
/// self-play X bucket −0.621 → −0.464 IMPs/action-board, vs BBA −2.716 → −2.329
/// IMPs/X-board).  The human "once penalty, always penalty" rule: after our side's
/// natural penalty double of their 1NT ([`penalty_x_reading`]), our later doubles
/// read as **penalty** — we double their runout on a trump stack rather than for
/// takeout on shortness, and partner leaves our double in rather than advancing it.
/// Keyed off the one penalty double the floor classifies today, so it is a no-op
/// unless the natural defense is on.  Disable for the off arm of the A/B.  Read at
/// classification time, per-thread.
///
/// [`penalty_x_reading`]: super::inference::penalty_x_reading
#[doc(hidden)]
pub fn set_penalty_latch(enabled: bool) {
    PENALTY_LATCH.with(|flag| flag.set(enabled));
}

/// Our side has latched into a penalty stance: we made the natural penalty double
/// of their 1NT earlier this auction and have bid no contract since
///
/// Hand-independent — it follows from the calls alone.  Same-side only (the
/// opponents' penalty doubles do not latch us).  Once we penalty-double their 1NT
/// the penalty stance holds for the rest of the auction — "once penalty, always
/// penalty" — even after our side bids a suit of its own.  Gated on
/// [`set_penalty_latch`], so it is dormant by default.
fn penalty_latched(context: &Context<'_>) -> bool {
    if !PENALTY_LATCH.with(Cell::get) {
        return false;
    }
    let auction = context.auction();
    let Some(double_index) = super::inference::penalty_x_reading(auction) else {
        return false;
    };
    // The doubler shares the player-to-act's parity (our side).
    double_index % 2 == auction.len() % 2
}

/// Whether the penalty-double latch is enabled (see [`set_penalty_latch`])
///
/// Exposed for the inference walk's matching reading
/// ([`penalty_latch_double_reading`][super::inference]), which must agree with the
/// floor on when a later double is penalty rather than takeout.
pub(super) fn penalty_latch_enabled() -> bool {
    PENALTY_LATCH.with(Cell::get)
}

/// [`penalty_latched`] as a hand-ignoring [`Constraint`] for the ladder
fn penalty_latched_c() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| penalty_latched(context))
}

/// Suppress the doubler's constructive pulls of its own penalty double of their
/// 1NT (**on by default**)
///
/// Independent of the latch's double-handling: this only stops the *bids* (the
/// natural suit / notrump overcalls), so the doubler defends or latch-doubles
/// instead of "competing" to 2NT/3NT/a major opposite a likely-broke partner.
/// A no-op unless the latch is on (the penalty stance it keys off, see
/// [`penalty_latched`]).  Read at classification time, per-thread.
///
/// DD-measured against BBA's 2/1 on the isolated 1NT-defense match (8000 we-defend
/// boards/seed): the penalty-X bucket goes −2.312 → −1.013 IMPs/X-board vulnerable
/// (paired +0.058 IMPs/board overall, 95% CI [+0.030, +0.085]) and is neutral
/// non-vulnerable (+0.007, CI straddles 0); the swing is isolated to the X bucket.
/// Disable for the off arm of the A/B.
#[doc(hidden)]
pub fn set_penalty_no_pull(enabled: bool) {
    PENALTY_NO_PULL.with(|flag| flag.set(enabled));
}

/// The doubler may make a constructive overcall: either the no-pull knob is off,
/// or we are not in the penalty stance ([`penalty_latched`]).  Gates the
/// overcall-shaped rules that fire off [`we_have_not_bid`] (a double is not a bid).
fn may_pull_penalty() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        !(PENALTY_NO_PULL.with(Cell::get) && penalty_latched(context))
    })
}

/// The penalty latch is *not* in force (the takeout-double default applies)
fn not_penalty_latched() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| !penalty_latched(context))
}

/// Select what a latched later double means for the current thread (live-read)
///
/// [`LatchStyle::Penalty`] (the **default**) is the stack-and-sit penalty double;
/// [`LatchStyle::Optional`] is the 2-3-card cooperative double.  A live-read
/// instinct flag (like [`set_penalty_latch`]), so the A/B harness sets it per
/// worker thread.
#[doc(hidden)]
pub fn set_latch_style(style: LatchStyle) {
    LATCH_STYLE.with(|cell| cell.set(style));
}

/// The latched double is the cooperative *optional* style (see [`LatchStyle`])
fn latch_optional_c() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| LATCH_STYLE.with(Cell::get) == LatchStyle::Optional)
}

/// The latched double is the pure *penalty* style (the default; see [`LatchStyle`])
fn latch_penalty_c() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| LATCH_STYLE.with(Cell::get) == LatchStyle::Penalty)
}

/// Enable or disable the advancer's runout from their redoubled penalty double
///
/// **On by default.**  After our natural penalty double of their 1NT, their
/// business redouble (`[1NT, X, XX]`) marks their side with the values, so a weak
/// advancer escapes to its long suit rather than sit for a making `1NTxx`.  The
/// mirror of the [responder runout][`set_one_nt_runout`] on the defensive side.
/// Disable for the off arm of the A/B; read at classification time, per-thread.
#[doc(hidden)]
pub fn set_advancer_xx_runout(enabled: bool) {
    ADVANCER_XX_RUNOUT.with(|flag| flag.set(enabled));
}

/// Their redoubled penalty double is back to a weak advancer (`[1NT, X, XX]`) and
/// the runout is enabled — the defensive mirror of [`responder_one_nt_runout_now`]
///
/// Keyed off [`penalty_x_reading`][super::inference::penalty_x_reading]: our side
/// penalty-doubled their 1NT, their next call was the redouble, and it is now the
/// doubler's partner (the advancer) to act for the first time.
fn advancer_xx_runout_now(context: &Context<'_>) -> bool {
    if !ADVANCER_XX_RUNOUT.with(Cell::get) {
        return false;
    }
    let auction = context.auction();
    let Some(x_index) = super::inference::penalty_x_reading(auction) else {
        return false;
    };
    auction.len() == x_index + 2
        && auction.last() == Some(&Call::Redouble)
        && x_index % 2 == auction.len() % 2
}

/// [`advancer_xx_runout_now`] as a hand-ignoring [`Constraint`] for the ladder
fn advancer_xx_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| advancer_xx_runout_now(context))
}

/// Enable or disable the *doubler's* runout from their redoubled penalty double
///
/// **On by default.**  After `[1NT, X, XX]` the opponents' business redouble runs
/// back around — advancer passes, opener passes (`[1NT, X, XX, P, P]`) — to the 15+
/// doubler.  On, a doubler holding a five-plus-card suit escapes to it rather than
/// defend a likely-making `1NTxx`; off, it sits.  Read once at book construction
/// (the escape rule is added only when on) so a duplicate A/B isolates cleanly.
#[doc(hidden)]
pub fn set_doubler_xx_runout(enabled: bool) {
    DOUBLER_XX_RUNOUT.with(|flag| flag.set(enabled));
}

/// Whether the doubler's runout rule is authored into the current book
fn doubler_xx_runout_enabled() -> bool {
    DOUBLER_XX_RUNOUT.with(Cell::get)
}

/// Their redoubled penalty double has run back to the doubler (`[1NT, X, XX, P, P]`)
///
/// Keyed off [`penalty_x_reading`][super::inference::penalty_x_reading] like
/// [`advancer_xx_runout_now`], but two calls later: the business redouble, then the
/// advancer's and opener's passes, leaving the doubler to act for the first time
/// since the double.  Pure on the auction (the flag gates the rule at construction).
fn doubler_xx_runout_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let Some(x_index) = super::inference::penalty_x_reading(auction) else {
        return false;
    };
    auction.len() == x_index + 4
        && auction[x_index + 1] == Call::Redouble
        && auction[x_index + 2] == Call::Pass
        && auction[x_index + 3] == Call::Pass
}

/// [`doubler_xx_runout_now`] as a hand-ignoring [`Constraint`] for the ladder
fn doubler_xx_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| doubler_xx_runout_now(context))
}

/// We opened the strong notrump of `nt_level` and partner just transferred with
/// the call `from` — the cue to complete the transfer
fn partner_transferred_now(context: &Context<'_>, from: Bid, nt_level: u8) -> bool {
    our_strong_notrump(context, nt_level, false)
        && partner_last_call(context.auction()) == Some(from)
}

/// [`partner_transferred_now`] as a hand-ignoring [`Constraint`] for the ladder
fn partner_transferred(from: Bid, nt_level: u8) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| partner_transferred_now(context, from, nt_level))
}

/// The transfers instinct completes opposite our own strong notrump, each
/// `(nt_level, partner's artificial call, completion)`
///
/// Standard Jacoby (2♦/2♥ over 1NT, 3♦/3♥ over 2NT) and South African Texas
/// (4♣/4♦).  Shared by the ladder's completion rules and the [`forced`] rail
/// predicate so the two never disagree on which transfers are in force.
const TRANSFERS: [(u8, Bid, Bid); 6] = [
    (
        1,
        Bid::new(2, Strain::Diamonds),
        Bid::new(2, Strain::Hearts),
    ),
    (1, Bid::new(2, Strain::Hearts), Bid::new(2, Strain::Spades)),
    (1, Bid::new(4, Strain::Clubs), Bid::new(4, Strain::Hearts)),
    (
        1,
        Bid::new(4, Strain::Diamonds),
        Bid::new(4, Strain::Spades),
    ),
    (
        2,
        Bid::new(3, Strain::Diamonds),
        Bid::new(3, Strain::Hearts),
    ),
    (2, Bid::new(3, Strain::Hearts), Bid::new(3, Strain::Spades)),
];

/// An auction-determined forced situation: partner's live takeout double, a
/// prior call committing our side to game, or partner's just-made transfer over
/// our strong notrump
///
/// Hand-independent — it follows from the calls alone.  The neural safety shell
/// consults it to decide when to delegate to the deterministic [`instinct()`]
/// ladder instead of trusting the learned net: the net handles the judgement
/// middle, but never these forced rails.  Hand-conditioned forces (a
/// strong-notrump responder who holds game values) are deliberately excluded —
/// they are judgement the net is trusted with, measured on the harness.
#[cfg(feature = "neural-floor")]
pub(crate) fn forced(context: &Context<'_>) -> bool {
    advancing_a_double_now(context)
        || Interpretation::read(context).forced_to_game
        || TRANSFERS
            .iter()
            .any(|&(nt_level, from, _)| partner_transferred_now(context, from, nt_level))
}

/// The opponents opened a one-level suit `X`, and our side answered with a
/// *simple* (non-jump) suit overcall `Y` — the setting for Rubens advances
///
/// Returns `(X, Y, overcall index, overcall level)`.  Only a one-level opening
/// and a non-jump overcall qualify: a jump overcall is preemptive (advance it
/// naturally, like over a preempt), and a preemptive opening leaves no room.  A
/// cue-bid (`Y == X`) is not a natural overcall.  Shared with [`Inferences`] so
/// the bidding and the reading agree on which calls are Rubens transfers.
///
/// [`Inferences`]: super::inference::Inferences
pub(crate) fn overcall_shape(auction: &[Call]) -> Option<(Suit, Suit, usize, u8)> {
    let (open_index, opening) = opening_bid(auction)?;
    let x = opening.strain.suit()?;
    if opening.level.get() != 1 {
        return None;
    }
    let opening_side = open_index % 2;
    let overcall_index =
        (open_index + 1..auction.len()).find(|&i| matches!(auction[i], Call::Bid(_)))?;
    // The first bid after the opening must be the *other* side's — an overcall,
    // not the opening side bidding on.
    if overcall_index % 2 == opening_side {
        return None;
    }
    let Call::Bid(overcall) = auction[overcall_index] else {
        return None;
    };
    let y = overcall.strain.suit()?;
    if y == x {
        return None;
    }
    // A simple overcall sits at the cheapest level: one when above the opening,
    // two when below it.  Anything higher is a (preemptive) jump.
    let simple = if (y as u8) > (x as u8) { 1 } else { 2 };
    (overcall.level.get() == simple).then_some((x, y, overcall_index, simple))
}

/// We are advancing partner's simple overcall, RHO having passed: the auction is
/// `(1X) Y (Pass)` to us
///
/// Returns `(X = the cue suit, Y = partner's overcall, overcall level)`.
fn advance_of_overcall(context: &Context<'_>) -> Option<(Suit, Suit, u8)> {
    let auction = context.auction();
    let (x, y, overcall_index, level) = overcall_shape(auction)?;
    (overcall_index + 2 == auction.len() && auction[auction.len() - 1] == Call::Pass)
        .then_some((x, y, level))
}

/// `2 source` is a Rubens transfer over partner's one-level overcall: the band
/// `X ≤ source < Y`, transferring to the next suit up
///
/// `into_partner` selects the transfer that lands in partner's suit `Y` (a
/// limit-plus raise) over a new-suit transfer (advancer's own five-card suit).
fn rubens_transfer(source: Suit, into_partner: bool) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        advance_of_overcall(context).is_some_and(|(x, y, level)| {
            level == 1
                && (x as u8) <= (source as u8)
                && (source as u8) < (y as u8)
                && (source as u8 + 1 == y as u8) == into_partner
        })
    })
}

/// `2 cue` is the Rubens cue-raise over partner's simple *two-level* overcall —
/// a limit-plus raise, the cue being the opponents' suit `X`
fn rubens_cue_raise(cue: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        advance_of_overcall(context).is_some_and(|(x, _, level)| level == 2 && x as u8 == cue as u8)
    })
}

/// Partner answered our simple one-level overcall with a Rubens transfer, RHO
/// passing — the cue to complete it
///
/// Returns the suit to complete into: the suit just above partner's transfer.
/// Mechanical (hand-independent), like completing a transfer over our own
/// notrump — see [`TRANSFERS`].
fn rubens_completion(context: &Context<'_>) -> Option<Suit> {
    let auction = context.auction();
    let len = auction.len();
    let (x, y, overcall_index, level) = overcall_shape(auction)?;
    // Only a one-level overcall carries the transfer ladder; the sequence is
    // overcall, (pass), transfer, (pass), us.
    if level != 1
        || overcall_index + 4 != len
        || auction[overcall_index + 1] != Call::Pass
        || auction[len - 1] != Call::Pass
    {
        return None;
    }
    let Call::Bid(transfer) = auction[overcall_index + 2] else {
        return None;
    };
    let source = transfer.strain.suit()?;
    (transfer.level.get() == 2 && (x as u8) <= (source as u8) && (source as u8) < (y as u8))
        .then(|| Suit::ASC[(source as u8 + 1) as usize])
}

/// [`rubens_completion`] as a [`Constraint`]: complete into `target`
fn rubens_completes(target: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| rubens_completion(context) == Some(target))
}

/// Build the instinct ladder: a sane natural action for any auction
///
/// Forced (partner's live takeout double — see the [module docs][self]):
/// penalty pass on a trump stack, a major-suit game jump or 3NT with values,
/// the longest unbid suit at the cheapest level (majors and five-card suits
/// preferred), and a cheapest-notrump escape as the guaranteed action.
///
/// Otherwise: raise partner's suit with three-card support and rising
/// strength per level, overcall notrump (15–18 balanced with stoppers) or a
/// five-card suit if we have not bid, double their low suit bid for takeout
/// on shape (or any 17+), and pass.
///
/// The unconditioned pass at weight `-5` is the absolute last resort: it
/// keeps the logits finite when every action is illegal, while sitting far
/// enough below every forced action (≥ 3 nats) that sampling drivers never
/// pass a forced auction by accident.
#[must_use]
pub fn instinct() -> Rules {
    let mut rules = Rules::new()
        // Forced: a trump stack sits for partner's takeout double.
        .rule(Call::Pass, 1.5, advancing_a_double() & doubled_suit_stack())
        // Settle floor (opt-in): a takeout double is not 100% forcing.  With four
        // cards behind their doubled suit, *defend* — pass plays their doubled
        // contract.  Above the advance ladder (new suit ~1.0, raises 1.2) and the
        // 0.3 notrump escape, below the trump stack 1.5 and the game jumps 1.45.
        .rule(
            Call::Pass,
            1.35,
            settle_floor() & advancing_a_double() & doubled_suit_length(),
        )
        // Forced: 3NT to play with game values and their suits stopped.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            advancing_a_double()
                & hcp(13..)
                & stopper_in_their_suits()
                & level_available(3, Strain::Notrump),
        )
        // Default unforced pass.  Under the settle floor it is also available in a
        // advance of partner's double (pass plays the top bid) — it still loses to every advance
        // rule, so a bust with no penalty advances as before.
        .rule(Call::Pass, 0.0, !advancing_a_double() | settle_floor())
        // The absolute last resort, keeping logits finite when all else is illegal.
        .rule(Call::Pass, -5.0, hcp(0..));

    // Forced: jump to a major-suit game with four-plus cards and values —
    // in an unbid major, never in the suit partner asked us to take out of.
    for major in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(major);
        rules = rules.rule(
            Bid::new(4, strain),
            1.45,
            advancing_a_double()
                & len(major, 4..)
                & points(11..)
                & level_available(4, strain)
                & !they_bid(strain),
        );
    }

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            0.05
        } else {
            0.0
        };

        // Forced: a new suit at the cheapest level; longer suits and majors
        // are preferred.  Bidding their suit would be a cue-bid — excluded.  The
        // settle floor's `free_bid_gate` (off by default) makes the four-level new
        // suit a free bid — values and no existing fit.
        for level in 1u8..=4 {
            rules = rules
                .rule(
                    Bid::new(level, strain),
                    1.0 + major_bonus,
                    advancing_a_double()
                        & min_level_is(level, strain)
                        & len(suit, 4..)
                        & !they_bid(strain)
                        & free_bid_gate(level),
                )
                .rule(
                    Bid::new(level, strain),
                    1.1 + major_bonus,
                    advancing_a_double()
                        & min_level_is(level, strain)
                        & len(suit, 5..)
                        & !they_bid(strain)
                        & free_bid_gate(level),
                );
        }

        // Raise partner's suit with three-card support; each level up asks
        // for more strength, so competitive raises terminate by themselves.
        // `partner_shown_len(suit, 3..)` makes the raise trust the *reading*, not the
        // bid suit: an artificial overcall (Woolsey 2♣ = both majors, 2♦ = a major)
        // shows its named minor short, so the floor never raises the phantom suit
        // into a doubled disaster.  A natural overcall is shown 5+, so it is
        // unaffected.  See `inference::multi_reading`.
        for (level, threshold) in [(2u8, 6u8), (3, 10), (4, 13)] {
            rules = rules.rule(
                Bid::new(level, strain),
                1.2,
                partner_suit_is(suit)
                    & partner_shown_len(suit, 3..)
                    & min_level_is(level, strain)
                    & support(3..)
                    & points(threshold..),
            );
        }

        // Preemptive jump to game: five-card support but too weak to invite —
        // the weak distributional raise, distinct from the point-showing raises
        // above.  Now that the floor owns advances of an overcall, this is the
        // weak end the book's `advances` used to cover.
        rules = rules.rule(
            Bid::new(4, strain),
            1.3,
            partner_suit_is(suit)
                & partner_shown_len(suit, 3..)
                & support(5..)
                & hcp(..6)
                & level_available(4, strain),
        );

        // Overcall a five-card suit if we have not bid; the strength floor
        // rises with the level and stronger hands double first.
        for (level, floor) in [(1u8, 8u8), (2, 10), (3, 13)] {
            rules = rules.rule(
                Bid::new(level, strain),
                1.0 + major_bonus,
                we_have_not_bid()
                    & may_pull_penalty()
                    & min_level_is(level, strain)
                    & len(suit, 5..)
                    & points(floor..=16)
                    & !they_bid(strain),
            );
        }
    }

    // Runout after our 1NT is doubled (default on; `set_one_nt_runout`).  A weak
    // responder escapes to its longest five-plus-card suit rather than sit for
    // the (effectively penalty) double; the values end redoubles and opener
    // passes the escape — both rules below.  The run/XX boundary is the
    // `set_runout_xx_min` knob (raw HCP), measured best near 7.
    //
    // The both-minor 2NT action (`set_unusual_2nt`) and the penalty double of
    // the opponents' escape (`set_penalize_escape_stack` / `_values`) are
    // authored below as A/B knobs; see the `ab-one-nt-runout --compare` axes.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            0.05
        } else {
            0.0
        };
        rules = rules
            .rule(
                Bid::new(2, strain),
                1.0 + major_bonus,
                one_nt_runout_enabled()
                    & responder_one_nt_runout()
                    & len(suit, 5..)
                    & hcp(..RUNOUT_MAX_HCP),
            )
            .rule(
                Bid::new(2, strain),
                1.1 + major_bonus,
                one_nt_runout_enabled()
                    & responder_one_nt_runout()
                    & len(suit, 6..)
                    & hcp(..RUNOUT_MAX_HCP),
            );
    }

    // Advancer's runout from their redoubled penalty double (`[1NT, X, XX]`,
    // default on; `set_advancer_xx_runout`).  Their XX is business, so a weak
    // advancer escapes to its longest five-plus-card suit instead of sitting for a
    // making `1NTxx` — the defensive mirror of the responder runout above.  A
    // values advancer (>= `RUNOUT_MAX_HCP`) passes to defend `1NTxx` instead.
    // ponytail: five-plus suits only; a 4-4 bust still sits — add the both-minors
    // escape (cf. the `2NT` rule below) if the A/B asks for it.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            0.05
        } else {
            0.0
        };
        rules = rules
            .rule(
                Bid::new(2, strain),
                1.0 + major_bonus,
                advancer_xx_runout() & len(suit, 5..) & hcp(..RUNOUT_MAX_HCP),
            )
            .rule(
                Bid::new(2, strain),
                1.1 + major_bonus,
                advancer_xx_runout() & len(suit, 6..) & hcp(..RUNOUT_MAX_HCP),
            );
    }

    // Doubler's runout once the redouble runs back around (`[1NT, X, XX, P, P]`,
    // on by default; `set_doubler_xx_runout`).  Unlike the advancer, the doubler is
    // the 15+ penalty hand, so there is *no* HCP cap — a doubler holding a five-plus
    // suit (a 5332 under the default balanced gate) escapes the redoubled `1NTxx`
    // rather than defend it; a 4-3-3-3/4-4-3-2 bust has nowhere to run and sits.
    // Construction-gated so the off arm of a duplicate A/B never carries the rule.
    if doubler_xx_runout_enabled() {
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let strain = Strain::from(suit);
            let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
                0.05
            } else {
                0.0
            };
            rules = rules
                .rule(
                    Bid::new(2, strain),
                    1.0 + major_bonus,
                    doubler_xx_runout() & len(suit, 5..),
                )
                .rule(
                    Bid::new(2, strain),
                    1.1 + major_bonus,
                    doubler_xx_runout() & len(suit, 6..),
                );
        }
    }

    // Runout, the values end: responder redoubles to play 1NT-XX rather than
    // run.  Outranks the escape so a values hand with a long suit still sits for
    // the (re)double; stays below the 1.40 game milestone so a game-going hand
    // bids it instead.  Opener then passes (1NT-XX) or bids game off the floor.
    rules = rules.rule(
        Call::Redouble,
        1.2,
        one_nt_runout_enabled() & responder_one_nt_runout() & responder_has_xx_values(),
    );

    // Runout, the both-minors end: 2NT = unusual, four-four in the minors with
    // no five-card suit to run to (a 5+ suit prefers the natural escape, which
    // outweighs this; a five-card major is excluded).  Opener picks the better
    // minor.  Weight below the suit escape, above Pass — a 4-4 minor bust escapes
    // 1NT-X to a known eight-card fit rather than sit.  Opt-in only: the default
    // `Direct` mode runs straight to a minor (below); A/B'd the relay a loser.
    rules = rules.rule(
        Bid::new(2, Strain::Notrump),
        0.5,
        one_nt_runout_enabled()
            & responder_one_nt_runout()
            & !unusual_2nt_is(Unusual2nt::Direct)
            & hcp(..RUNOUT_MAX_HCP)
            & len(Suit::Clubs, 4..)
            & len(Suit::Diamonds, 4..)
            & len(Suit::Hearts, ..5)
            & len(Suit::Spades, ..5),
    );

    // Runout, 2NT extended (`set_unusual_2nt(FiveFiveAdd)`): a five-five-minors
    // hand bids 2NT too, above the natural minor escape (1.0/1.1), so opener
    // picks the better fit instead of responder guessing a minor.  A five-five
    // hand cannot hold a five-card major, so no major guard is needed.
    rules = rules.rule(
        Bid::new(2, Strain::Notrump),
        1.15,
        one_nt_runout_enabled()
            & responder_one_nt_runout()
            & unusual_2nt_is(Unusual2nt::FiveFiveAdd)
            & hcp(..RUNOUT_MAX_HCP)
            & len(Suit::Clubs, 5..)
            & len(Suit::Diamonds, 5..),
    );

    // Runout, the direct escape (`set_unusual_2nt(Direct)`, the default): no 2NT
    // relay — a weak four-four-minors bust bids its longer minor (ties to
    // diamonds) at the two level, one double-exposure instead of the relay's two.
    // Opener passes it like any escape (`opener_after_one_nt_runout`, above).
    let direct_bust = one_nt_runout_enabled()
        & responder_one_nt_runout()
        & unusual_2nt_is(Unusual2nt::Direct)
        & hcp(..RUNOUT_MAX_HCP)
        & len(Suit::Clubs, 4..)
        & len(Suit::Diamonds, 4..)
        & len(Suit::Hearts, ..5)
        & len(Suit::Spades, ..5);
    rules = rules
        .rule(
            Bid::new(2, Strain::Diamonds),
            1.0,
            direct_bust.clone() & longer_diamonds(),
        )
        .rule(
            Bid::new(2, Strain::Clubs),
            1.0,
            direct_bust & !longer_diamonds(),
        );

    // Opener passes partner's runout: responder ran because it is weak, so it
    // captains the auction.  Weight outranks the natural raise *and* the 1.5
    // transfer completion — without it a 2♦/2♥ escape is misread as a Jacoby
    // transfer and opener "completes" it into responder's short suit.
    // ponytail: always pass; pulling to a better suit on a misfit is deferred.
    rules = rules.rule(
        Call::Pass,
        1.55,
        one_nt_runout_enabled() & opener_after_one_nt_runout(),
    );

    // Opener answers partner's 2NT minors-scramble with the better minor (longer,
    // ties to diamonds).  Weight outranks the 1.5 transfer completion — the floor
    // reads 2NT as a diamond transfer, which would force a club-longer hand to 3♦.
    rules = rules
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.6,
            one_nt_runout_enabled() & opener_after_one_nt_minors() & longer_diamonds(),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            1.6,
            one_nt_runout_enabled() & opener_after_one_nt_minors() & !longer_diamonds(),
        );

    // Universal runout, opener's balancing seat (`set_one_nt_runout_universal`).
    // The double came back to opener with a weak partner (it had no escape), so
    // 1NT-X rates to fail: opener runs its own five-plus-card suit rather than
    // sit — but only minimum-ish, since a maximum still rates to make 1NT-X.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            0.05
        } else {
            0.0
        };
        rules = rules
            .rule(
                Bid::new(2, strain),
                1.0 + major_bonus,
                one_nt_runout_enabled()
                    & one_nt_runout_universal()
                    & opener_balancing_runout()
                    & len(suit, 5..)
                    & hcp(..17),
            )
            .rule(
                Bid::new(2, strain),
                1.1 + major_bonus,
                one_nt_runout_enabled()
                    & one_nt_runout_universal()
                    & opener_balancing_runout()
                    & len(suit, 6..)
                    & hcp(..17),
            );
    }

    // Balancing redouble = SOS: no five-card suit to run to and not a maximum —
    // ask partner to pick a suit, four-card suits included.
    rules = rules.rule(
        Call::Redouble,
        1.0,
        one_nt_runout_enabled()
            & one_nt_runout_universal()
            & opener_balancing_runout()
            & hcp(..17)
            & len(Suit::Clubs, ..5)
            & len(Suit::Diamonds, ..5)
            & len(Suit::Hearts, ..5)
            & len(Suit::Spades, ..5),
    );

    // Responder answers the SOS redouble with its longest suit (four-card OK).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        let major_bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            0.05
        } else {
            0.0
        };
        for (length, weight) in [(4usize, 1.0f32), (5, 1.1), (6, 1.2)] {
            rules = rules.rule(
                Bid::new(2, strain),
                weight + major_bonus,
                one_nt_runout_enabled()
                    & one_nt_runout_universal()
                    & responder_after_opener_sos()
                    & len(suit, length..),
            );
        }
    }

    // Opener passes responder's SOS answer — responder captains the rescue.
    // Outranks the natural raise and the transfer completion, as elsewhere.
    rules = rules.rule(
        Call::Pass,
        1.55,
        one_nt_runout_enabled() & one_nt_runout_universal() & opener_after_responder_sos(),
    );

    // Encircling: the opponents ran from our doubled (or redoubled) 1NT.  We
    // hold the balance, so double their escape for penalty — and keep doubling
    // as they keep running — rather than let them buy it cheaply.  Two arms,
    // each an A/B knob: a trump stack in their suit (sound in any seat), or
    // general values once responder's business redouble has shown them.  Weight
    // outranks the floor's takeout double of the same suit (<=0.9).
    rules = rules
        .rule(
            Call::Double,
            1.6,
            one_nt_runout_enabled()
                & penalize_escape_stack_enabled()
                & opp_escaped_our_nt_undoubled()
                & doubled_suit_stack(),
        )
        .rule(
            Call::Double,
            1.6,
            one_nt_runout_enabled()
                & penalize_escape_values_enabled()
                & opp_escaped_our_business_xx()
                & hcp(7..),
        );

    // Partner leaves in our penalty double of their escape: it is penalty by
    // agreement, not the takeout the `advancing_a_double` default would advance.
    // Outranks every advance action (<=1.5).
    rules = rules.rule(
        Call::Pass,
        1.55,
        one_nt_runout_enabled() & leave_in_escape_penalty(),
    );

    // Penalty latch: once our side has penalty-doubled their 1NT, leave in any
    // later double of ours rather than advance it — "once penalty, always
    // penalty".  Outranks every advance action (<=1.5); the mirror of the runout
    // leave-in above, gated on its own A/B knob ([`set_penalty_latch`]).  The
    // optional latch style suppresses this forced sit: partner instead cooperates
    // (sit on a fit, run when short) via the general advance-a-double machinery.
    rules = rules.rule(
        Call::Pass,
        1.55,
        penalty_latched_c() & latch_penalty_c() & advancing_a_double(),
    );

    // UvU encircling: the opponents ran from our 1NT-(2NT)-X.  Double their
    // escape with a trump stack — and keep doubling as they keep running — by
    // agreement; partner leaves in.  Mirrors the doubled-1NT escape chase above,
    // gated on its own A/B knob ([`set_uvu_encircle`]), independent of the runout.
    rules = rules
        .rule(
            Call::Double,
            1.6,
            uvu_encircle_enabled() & opp_escaped_our_uvu_undoubled() & doubled_suit_stack(),
        )
        .rule(
            Call::Pass,
            1.55,
            uvu_encircle_enabled() & leave_in_uvu_penalty(),
        );

    for level in 1u8..=4 {
        // Forced: the notrump escape guarantees an action — no fit, no
        // stopper, no four-card suit outside theirs still has a call.
        rules = rules.rule(
            Bid::new(level, Strain::Notrump),
            0.3,
            advancing_a_double() & min_level_is(level, Strain::Notrump),
        );
    }

    for level in 1u8..=3 {
        // Notrump overcall: 15–18 balanced with their suits stopped.
        rules = rules.rule(
            Bid::new(level, Strain::Notrump),
            1.05,
            we_have_not_bid()
                & may_pull_penalty()
                & min_level_is(level, Strain::Notrump)
                & balanced()
                & hcp(15..=18)
                & stopper_in_their_suits(),
        );
    }

    // Opposite our own strong notrump: complete partner's transfer.  Standard
    // Jacoby (2♦/2♥, 3♦/3♥ over 2NT) and South African Texas (4♣/4♦); the book
    // authors these where it can, so this only catches off-book and competitive
    // continuations.  Bid the suit just above partner's artificial call.
    for (nt_level, from, to) in TRANSFERS {
        rules = rules.rule(
            to,
            1.5,
            partner_transferred(from, nt_level) & level_available(to.level.get(), to.strain),
        );
    }

    // Game values.  Three strands force game regardless of the point estimate:
    // the hand-conditioned strong-notrump responder forces (10+ opposite a 15–17
    // 1NT, 5+ opposite a 20–21 2NT), and the hand-independent forces from the
    // auction interpretation — a strong 2♣ past the double negative, or an opener
    // forced past invitation.  A fourth strand is *general*: our own count plus
    // the sound floor of partner's shown points reaching 25 (the inference makes
    // it sound, never an overbid).  Below game we take the cheapest milestone — a
    // known major fit, else 3NT with their suits stopped, dropping to the minor
    // game only when their suit is unstopped — but step aside when penalizing the
    // opponents.  The 3NT stopper guard is vacuous uncontested (no suit of theirs
    // to stop), so it changes only competitive auctions: never a notrump game bid
    // into an unstopped enemy suit.
    let game_values = ((partner_strong_notrump(1)
        & (hcp(10..) | (hcp(nt_responder_game_floor()..) & undisturbed())))
        | (partner_strong_notrump(2) & hcp(5..))
        | auction_forces_game()
        | combined_points(25))
        & not_penalizing();
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.40,
        game_values.clone()
            & below_game()
            & stopper_in_their_suits()
            & nt_game_force_3nt_allowed()
            & level_available(3, Strain::Notrump),
    );
    // Gambling 3NT over a double of our 1NT (opt-in; `set_gambling_3nt_over_double`).
    // A long (6+) minor, semi-solid, with an outside ace by default — responder runs
    // its suit opposite the 15–17 opener rather than defend the redouble or escape.
    // Split per minor so the build-time `len(minor, 6..)` floors the *named* suit in
    // the projection; `.alert(GAMBLING_3NT)` marks the call artificial so the reader
    // suppresses the natural balanced-3NT reading and the sampler stops dealing
    // responder flat.  Weight 1.45 outranks the business XX (1.2) and the escapes
    // (≤1.1); a balanced strong hand holds no 6-card minor and still redoubles.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        rules = rules
            .rule(
                Bid::new(3, Strain::Notrump),
                1.45,
                one_nt_runout_enabled()
                    & responder_one_nt_runout()
                    & gambling_3nt_authored()
                    & len(minor, 6..)
                    & gambling_3nt_semisolid(minor)
                    & gambling_3nt_suit_ace(minor)
                    & level_available(3, Strain::Notrump),
            )
            .alert(GAMBLING_3NT);
    }
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let strain = Strain::from(minor);
        // 3NT is the milestone of choice; reach for the minor game only when
        // notrump is unsafe (a suit they bid is unstopped) and we hold a known
        // eight-card fit.  Uncontested, their suits are vacuously stopped, so
        // this never fires and 3NT plays.
        let known_minor_fit = (len(minor, 5..) & partner_shown_len(minor, 3..))
            | (len(minor, 3..) & partner_shown_len(minor, 5..));
        rules = rules.rule(
            Bid::new(5, strain),
            1.42,
            game_values.clone()
                & below_game()
                & inference_aware()
                & known_minor_fit
                & !stopper_in_their_suits()
                & level_available(5, strain),
        );
    }
    for major in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(major);
        // A *known* eight-card major fit outranks 3NT: our five-card suit meets
        // partner's shown three-card support, our three meet partner's shown
        // five, or our doubleton meets partner's shown six (a transferred suit
        // jumped or raised to game — see [`Inferences`]).  The shown lengths come
        // from the auction interpretation, so this fires only on a fit the calls
        // have promised.
        //
        // [`Inferences`]: super::inference::Inferences
        let known_major_fit = (len(major, 5..) & partner_shown_len(major, 3..))
            | (len(major, 3..) & partner_shown_len(major, 5..))
            | (len(major, 2..) & partner_shown_len(major, 6..));
        rules = rules.rule(
            Bid::new(4, strain),
            1.45,
            game_values.clone() & below_game() & len(major, 6..) & level_available(4, strain),
        );
        // Preemptive 4M over a double of our 1NT (opt-in; `set_preempt_4m_over_double`).
        // The major's mirror of the gambling 3NT: a *quality* long (6+) major —
        // semi-solid and headed by the trump ace (a sure trump trick that buffs total
        // tricks) — on a modest hand, partly preemptive and partly to make opposite the
        // strong notrump.  Natural (the bid major reads as 6+), so unalerted; the
        // game-values arm above still governs undisturbed and over an overcall.
        rules = rules.rule(
            Bid::new(4, strain),
            1.45,
            one_nt_runout_enabled()
                & responder_one_nt_runout()
                & preempt_4m_authored()
                & len(major, 6..)
                & preempt_4m_semisolid(major)
                & preempt_4m_trump_ace(major)
                & preempt_4m_values()
                & below_game()
                & level_available(4, strain),
        );
        rules = rules.rule(
            Bid::new(4, strain),
            1.50,
            game_values.clone()
                & below_game()
                & inference_aware()
                & known_major_fit.clone()
                & level_available(4, strain),
        );
        // Correct partner's choice-of-games 3NT to a known eight-card major fit —
        // but only undisturbed and with a ruffing doubleton.  Game is already
        // agreed, so this is a pure strain choice (no strength gate); opposite
        // responder's *balanced* transferred five the 5-3 fit out-scores notrump
        // only when the trump-short hand can ruff, so a flat 4-3-3-3 opener leaves
        // it in 3NT (`has_ruffing_shortness`).  `undisturbed` keeps it off contested
        // auctions, where the pull to the four level walks into a penalty double.
        rules = rules.rule(
            Bid::new(4, strain),
            1.50,
            correct_3nt_to_major_now()
                & undisturbed()
                & inference_aware()
                & known_major_fit.clone()
                & has_ruffing_shortness()
                & level_available(4, strain),
        );
        // Slam is a milestone too: with a known major fit and the combined
        // minimum in the small- (33) or grand- (37) slam zone, bid it.
        rules = rules.rule(
            Bid::new(6, strain),
            1.65,
            combined_points(33)
                & not_penalizing()
                & below_slam()
                & inference_aware()
                & known_major_fit.clone()
                & level_available(6, strain),
        );
        rules = rules.rule(
            Bid::new(7, strain),
            1.75,
            combined_points(37)
                & not_penalizing()
                & below_slam()
                & inference_aware()
                & known_major_fit
                & level_available(7, strain),
        );
    }
    // Notrump slam when no major fit is known: small at 33, grand at 37, with
    // their suits stopped (vacuous when uncontested).
    rules = rules
        .rule(
            Bid::new(6, Strain::Notrump),
            1.60,
            combined_points(33)
                & not_penalizing()
                & below_slam()
                & stopper_in_their_suits()
                & level_available(6, Strain::Notrump),
        )
        .rule(
            Bid::new(7, Strain::Notrump),
            1.70,
            combined_points(37)
                & not_penalizing()
                & below_slam()
                & stopper_in_their_suits()
                & level_available(7, Strain::Notrump),
        );

    // Rubens advances of partner's simple overcall.  Over a one-level overcall
    // the calls from the cue up to just below a two-level raise are transfers to
    // the next suit: a new-suit transfer shows a five-card suit and 10+ upgraded
    // points — a *good* 9 and all 10+, since the transfer commits partner to the
    // two-level — and the transfer into partner's suit is a limit-plus raise.
    // Over a two-level overcall the cue itself is the limit-plus raise.  Both halves are read in [`Inferences`]
    // (the transfer/cue suit is a relay, not a holding), so partner's instinct
    // completes the transfer and the milestone never misreads it as natural.
    //
    // [`Inferences`]: super::inference::Inferences
    for source in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        let source_strain = Strain::from(source);
        let target = Suit::ASC[(source as u8 + 1) as usize];
        rules = rules
            .rule(
                Bid::new(2, source_strain),
                1.35,
                rubens_transfer(source, false)
                    & len(target, 5..)
                    & points(10..)
                    & min_level_is(2, source_strain),
            )
            .rule(
                Bid::new(2, source_strain),
                1.45,
                rubens_transfer(source, true)
                    & support(3..)
                    & points(10..)
                    & min_level_is(2, source_strain),
            );
    }
    for cue in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let cue_strain = Strain::from(cue);
        rules = rules.rule(
            Bid::new(2, cue_strain),
            1.45,
            rubens_cue_raise(cue) & support(3..) & points(10..) & min_level_is(2, cue_strain),
        );
    }
    // Complete partner's transfer into the suit just above it — mechanical, like
    // completing a transfer over our own notrump.
    for target in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(target)),
            1.55,
            rubens_completes(target),
        );
    }

    // Takeout double of their low suit bid: shape with opening values, or
    // any strong hand planning to bid again.  The penalty latch steps these
    // aside — once we own the auction for penalty, a double is not takeout.
    rules
        .rule(
            Call::Double,
            0.9,
            their_live_bid_at_most(3) & short_in_their_suits() & hcp(12..) & not_penalty_latched(),
        )
        .rule(
            Call::Double,
            0.8,
            their_live_bid_at_most(3) & points(17..) & not_penalty_latched(),
        )
        // Penalty latch: double their runout for penalty on a trump stack instead
        // of takeout on shortness.  Weight matches the runout penalty doubles.
        .rule(
            Call::Double,
            1.6,
            their_live_bid_at_most(3)
                & penalty_latched_c()
                & latch_penalty_c()
                & doubled_suit_stack(),
        )
        // Optional latch: double their runout cooperatively on 2-3 cards and
        // values — partner decides (sit on a fit, run when short).  The defensive
        // mirror of the we-open optional double; same weight as the penalty stack.
        .rule(
            Call::Double,
            1.6,
            their_live_bid_at_most(3)
                & penalty_latched_c()
                & latch_optional_c()
                & doubled_suit_len(2..=3)
                & hcp(6..),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    const fn call(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The highest-logit instinct call for a hand in an auction
    fn best(auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        let logits = instinct().classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    /// The full-`american()` call for a hand and whether the floor produced it
    ///
    /// `depth == 0` with `fallback == Some(_)` is the instinct floor firing — so
    /// the second tuple field tells a test the node is off-book (floor territory),
    /// guarding against a floor rule that is silently shadowed by a book node.
    fn american_floored(auction: &[Call], hand: &str) -> (Call, bool) {
        use crate::bidding::Family;
        use crate::bidding::american::american;
        let hand: Hand = hand.parse().expect("valid test hand");
        let (logits, provenance) = american()
            .against(Family::NATURAL)
            .classify_with_provenance(hand, RelativeVulnerability::NONE, auction)
            .expect("a legal auction classifies");
        let call = (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty");
        (call, provenance.depth == 0 && provenance.fallback.is_some())
    }

    #[test]
    fn advancing_a_double_advances_a_bust_but_defends_with_length() {
        // Partner doubled their 3♣ for takeout, RHO passed.
        let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
        // A worthless hand with a five-card suit outside theirs still advances —
        // it cannot beat 3♣ doubled, so it bids rather than pass into it.
        assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
        // But four cards sitting behind their suit defend: pass plays 3♣ doubled,
        // a better penalty than escaping (the settle floor, default on).
        assert_eq!(best(&auction, "964.J85.974.9632"), Call::Pass);
    }

    #[test]
    fn trump_stack_converts_to_penalties() {
        // KQ92 behind the 2♠ bidder sits for partner's takeout double.
        let auction = [call(2, Strain::Spades), Call::Double, Call::Pass];
        assert_eq!(best(&auction, "KQ92.A532.J42.96"), Call::Pass);
    }

    #[test]
    fn penalty_latch_doubles_the_runout_for_penalty() {
        // (1NT) X — our penalty double — (2♦) runout; we hold a diamond stack.
        let auction = [
            call(1, Strain::Notrump),
            Call::Double,
            call(2, Strain::Diamonds),
        ];
        // A pure diamond stack (9 HCP, all in their suit): combined with partner's
        // shown 15+ this is below game, so the floor neither bids nor advances.
        // Latch off — defend by passing, no penalty double offered.
        set_penalty_latch(false);
        assert_eq!(best(&auction, "T98.964.AKQ7.853"), Call::Pass);
        // Latch on (the default): "once penalty, always penalty" — double for penalty.
        set_penalty_latch(true);
        assert_eq!(best(&auction, "T98.964.AKQ7.853"), Call::Double);
        // The latch keys off the 1NT penalty double only: a plain takeout auction
        // is untouched — short in clubs with opening values still doubles 2♣ takeout.
        let takeout = [call(2, Strain::Clubs)];
        assert_eq!(best(&takeout, "AQ95.KJ73.K842.6"), Call::Double);
    }

    #[test]
    fn penalty_latch_leaves_partner_s_double_in() {
        // (1NT) X (2♦) X (Pass): partner doubled the runout for penalty, back to us.
        let auction = [
            call(1, Strain::Notrump),
            Call::Double,
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        // A flat 16-count with no diamond stopper: latch off, the takeout-advance
        // jumps to a dubious 4♠ on a four-card suit.
        set_penalty_latch(false);
        assert_eq!(best(&auction, "AQ74.AQ5.82.A632"), call(4, Strain::Spades));
        // Latched (the default), partner's double is penalty — leave it in (defend 2♦x).
        set_penalty_latch(true);
        assert_eq!(best(&auction, "AQ74.AQ5.82.A632"), Call::Pass);
    }

    #[test]
    fn advancer_runs_from_redoubled_penalty_double() {
        // (1NT) X (XX): their business redouble, back to the broke advancer.
        let auction = [call(1, Strain::Notrump), Call::Double, Call::Redouble];
        // Weak with a five-card major: escape to it rather than sit for 1NTxx.
        assert_eq!(best(&auction, "J9763.852.764.43"), call(2, Strain::Spades));
        // Weak with a six-card minor: run to it.
        assert_eq!(best(&auction, "82.43.765.QJ8765"), call(2, Strain::Clubs));
        // Values (9 HCP): sit and defend 1NTxx — our side beats it.
        assert_eq!(best(&auction, "KQ7.K83.J642.643"), Call::Pass);
        // Off-switch: the runout disabled, the broke hand sits.
        set_advancer_xx_runout(false);
        assert_eq!(best(&auction, "J9763.852.764.43"), Call::Pass);
        set_advancer_xx_runout(true);
    }

    #[test]
    fn doubler_runs_from_redoubled_penalty_double() {
        // (1NT) X (XX) P P: the redouble ran back to the 15+ doubler.
        let auction = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Redouble,
            Call::Pass,
            Call::Pass,
        ];
        // On by default: a 15+ 5332 escapes to its five-card suit rather than defend
        // the redouble.
        assert_eq!(best(&auction, "AQ765.KQ4.A82.K3"), call(2, Strain::Spades));
        assert_eq!(best(&auction, "AQ4.K82.K3.AQ765"), call(2, Strain::Clubs));
        // No five-card suit (4-4-3-2): nowhere to run, so sit.
        assert_eq!(best(&auction, "AQ74.KQ32.A82.K3"), Call::Pass);
        // Off-switch: the strong doubler sits and defends 1NTxx.
        set_doubler_xx_runout(false);
        assert_eq!(best(&auction, "AQ765.KQ4.A82.K3"), Call::Pass);
        set_doubler_xx_runout(true);
    }

    #[test]
    fn optional_latch_doubles_short_and_partner_cooperates() {
        // (1NT) X (2♦): our latched double of their runout.
        let runout = [
            call(1, Strain::Notrump),
            Call::Double,
            call(2, Strain::Diamonds),
        ];
        // Three small diamonds (no stack), 13 HCP, no four-card suit worth bidding:
        // the PENALTY latch needs a stack, so it does not double…
        let cooperative = "KQ5.KQ5.642.QJ93";
        set_latch_style(LatchStyle::Penalty);
        assert_ne!(best(&runout, cooperative), Call::Double);
        // …but the OPTIONAL latch doubles on the 2-3 holding and values.
        set_latch_style(LatchStyle::Optional);
        assert_eq!(best(&runout, cooperative), Call::Double);

        // (1NT) X (2♦) X (Pass): partner's latched double, back to the 15+ doubler.
        let advance = [
            call(1, Strain::Notrump),
            Call::Double,
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        // Penalty: forced sit — leave the penalty double in (defend 2♦x).
        set_latch_style(LatchStyle::Penalty);
        assert_eq!(best(&advance, "AQ74.AQ5.82.A632"), Call::Pass);
        // Optional: cooperate (not forced to sit) — short in their suit with a
        // four-card major and values, run to the major game.
        set_latch_style(LatchStyle::Optional);
        assert_eq!(best(&advance, "AQ74.AQ5.82.A632"), call(4, Strain::Spades));

        set_latch_style(LatchStyle::default());
    }

    #[test]
    fn advancing_a_double_bids_game_with_values() {
        let auction = [call(2, Strain::Spades), Call::Double, Call::Pass];
        // 13 HCP with their suit stopped and no length behind it: 3NT to play.
        assert_eq!(best(&auction, "AQ3.K65.J64.QJ96"), call(3, Strain::Notrump));
        // 11 HCP with four hearts: jump to the major-suit game.
        assert_eq!(best(&auction, "92.AQ53.KQ42.962"), call(4, Strain::Hearts));
    }

    #[test]
    fn unforced_raise_with_fit() {
        // Partner opened 1♠ and they overcalled 2♥: raise with three-card
        // support and 8 HCP.
        let auction = [call(1, Strain::Spades), call(2, Strain::Hearts)];
        assert_eq!(best(&auction, "Q32.953.A964.Q92"), call(2, Strain::Spades));
    }

    #[test]
    fn unforced_takeout_double_on_shape() {
        // Their 3♦ preempt: 13 HCP, short in diamonds, no five-card suit.
        let auction = [call(3, Strain::Diamonds)];
        assert_eq!(best(&auction, "KQ32.AJ53.2.A942"), Call::Double);
    }

    #[test]
    fn unforced_pass_without_values() {
        // Nothing to say over their 3♦: too weak to act at the three level.
        let auction = [call(3, Strain::Diamonds)];
        assert_eq!(best(&auction, "Q5432.J53.942.92"), Call::Pass);
    }

    #[test]
    fn doubles_only_their_live_bids() {
        // The call to beat is our own 2♠ (partner raised our overcall):
        // doubling our side is never on the table.
        let auction = [
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        let hand: Hand = "92.K53.AQJ42.962".parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let logits = instinct().classify(hand, &context);
        assert_eq!(*logits.0.get(Call::Double), f32::NEG_INFINITY);
    }

    #[test]
    fn settle_floor_defends_with_length_behind_their_suit() {
        // Their 3♠, partner doubles (takeout), RHO passes → advancing a double.
        let auction = [call(3, Strain::Spades), Call::Double, Call::Pass];
        // 6 HCP, five clubs but four cards sitting behind their spades.
        let weak_with_defense = "9543.74.K2.QJ876";

        // Off (default): the floor over-advances to the captive 4♣.
        set_settle_floor(false);
        assert_eq!(best(&auction, weak_with_defense), call(4, Strain::Clubs));

        // On: the four-level new suit is a free bid we lack the values for, and we
        // hold four behind their suit, so we defend — pass plays 3♠ doubled.
        set_settle_floor(true);
        assert_eq!(best(&auction, weak_with_defense), Call::Pass);

        // On, with real values: the free bid is earned — we still advance to 4♣
        // (a hand short in their suit cannot defend anyway).
        let strong = "2.853.K42.AKQ876";
        assert_eq!(best(&auction, strong), call(4, Strain::Clubs));

        set_settle_floor(true); // restore the default (on) for the rest of the suite
    }

    #[test]
    fn completes_partners_transfer_over_notrump() {
        // We opened 1NT and partner transferred 2♦ (hearts): complete with 2♥,
        // even off-book, rather than passing or raising the artificial diamonds.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "AQ32.KJ5.KQ4.Q92"), call(2, Strain::Hearts));
    }

    #[test]
    fn forced_to_game_opposite_strong_notrump() {
        // Partner opened 1NT; after an artificial 2NT super-accept of our heart
        // transfer a game-forced 12-count bids 3NT, never passing below game.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "KQ52.AQ984.J6.32"), call(3, Strain::Notrump));
    }

    #[test]
    fn forced_to_game_picks_the_known_major_fit() {
        // We opened 1NT; partner's off-book, forcing 3♥ shows five-plus hearts.
        // With three-card support that is a known eight-card fit, so bid 4♥
        // rather than the stopperless-agnostic 3NT.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "AQ52.K53.KQ4.32"), call(4, Strain::Hearts));
    }

    #[test]
    fn transfer_invite_reaches_the_floor_over_a_possible_five_two() {
        // 1NT–2♦–2♥–3♥: partner transferred to hearts and raised.  With the six-card
        // invite on (the default) this node is authored — so turn it off to exercise
        // the floor path this test guards: the projection reads the 2♦ transfer's
        // five-card floor (M6.1's core), but M6.2c dropped the old reader's six-card
        // upgrade off the 3♥ raise (soundness over tightness — projecting a
        // natural-suit raise is out of the overlay's artificial-only scope).  With
        // only a five-card major shown and our own doubleton, the floor prefers 3NT
        // over a possible 5-2 game.
        crate::bidding::american::set_sixcard_invite_floor(14);
        let invite = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (bid, from_floor) = american_floored(&invite, "AKQ2.J5.AQ52.K42");
        assert!(
            from_floor,
            "the transfer invite is off-book, the floor decides"
        );
        assert_eq!(bid, call(3, Strain::Notrump));
        crate::bidding::american::set_sixcard_invite_floor(13); // restore default
    }

    #[test]
    fn transfer_jump_to_game_reaches_the_floor_and_passes() {
        // 1NT–2♦–2♥–4♥: the jump past 3NT is off-book too.  Game is already
        // reached and the floor has no slam machinery yet (M6.2), so it passes —
        // M6.1 derives the six-card major (length only) without over-reaching.
        let game = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
            call(4, Strain::Hearts),
            Call::Pass,
        ];
        let (bid, from_floor) = american_floored(&game, "AKQ2.J5.AQ52.K42");
        assert!(from_floor, "the 4♥ jump is off-book, the floor decides");
        assert_eq!(bid, Call::Pass);
    }

    #[test]
    fn nine_count_five_card_major_forces_game_after_a_transfer() {
        // 1NT–2♥–2♠: a 9-count with a single five-card spade suit transferred (it
        // cannot bid the direct 3NT, which denies a five-card major) and now forces
        // game off the floor — the authored rebid table stops at the exactly-8
        // invite, so the floor (default 9) carries the 9.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        let (bid, from_floor) = american_floored(&auction, "AK543.82.Q76.542");
        assert!(from_floor, "the game force is off-book, the floor decides");
        assert_eq!(bid, call(3, Strain::Notrump));
    }

    #[test]
    fn opener_corrects_choice_of_games_3nt_to_the_known_major_fit() {
        // 1NT–2♥–2♠–3NT: responder transferred (showing five spades) then offered
        // the choice with 3NT.  Opposite three-card support the 5-3 fit out-scores
        // notrump single-dummy *only with a ruffing doubleton*, so opener corrects
        // to 4♠ on a doubleton, but a flat 4-3-3-3 (no ruff) leaves the better game
        // in 3NT.  Default on; the guard sets it explicitly to stay isolated.
        set_correct_3nt_to_major(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
            call(3, Strain::Notrump),
            Call::Pass,
        ];
        // Three-card support with a ruffing doubleton (3-2-4-4): correct to 4♠.
        let (fit, _) = american_floored(&auction, "AQ4.K8.KJ72.Q832");
        assert_eq!(
            fit,
            call(4, Strain::Spades),
            "3-card support with a doubleton corrects to 4♠"
        );
        // Three-card support but a flat 4-3-3-3 (no ruffing value): stay in 3NT.
        let (flat, _) = american_floored(&auction, "AQ4.K83.KJ72.Q83");
        assert_eq!(
            flat,
            Call::Pass,
            "flat 4333 has no ruff — 3NT is the better game"
        );
        // Only a doubleton spade — no eight-card fit — also stays in 3NT.
        let (two, _) = american_floored(&auction, "AQ.K842.KJ73.Q82");
        assert_eq!(two, Call::Pass, "no eight-card fit leaves it in 3NT");
        set_correct_3nt_to_major(true); // restore the default
    }

    #[test]
    fn strong_balanced_redoubles_a_double_of_our_1nt_not_3nt() {
        // 1NT–(X): a strong balanced responder defends the unlimited business
        // redouble rather than pulling to 3NT (the floor suppresses the game-force
        // 3NT over a double of our 1NT).
        let auction = [call(1, Strain::Notrump), Call::Double];
        let (bid, from_floor) = american_floored(&auction, "KQ4.KJ43.AQ62.Q5");
        assert!(from_floor, "the response is off-book, the floor decides");
        assert_eq!(bid, Call::Redouble);
    }

    #[test]
    fn keeps_passing_with_a_weak_responder() {
        // Partner opened 1NT but we are too weak to force game: still pass when
        // off-book (the forced-to-game floor must not fire on invitational-or-less).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "8632.J9842.96.42"), Call::Pass);
    }

    #[test]
    fn forced_to_game_after_strong_two_clubs() {
        // 2♣ (strong) – 2♦ (game-forcing waiting) – 2NT (22–24 balanced): the
        // auction is game forcing, so a flat 7-count bids 3NT, never passing.
        // 2♣–2♥ is the double negative, so 2♦ commits the partnership to game.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "QJ52.K43.T62.J32"), call(3, Strain::Notrump));
    }

    #[test]
    fn forced_two_clubs_bids_major_game() {
        // The same forcing 2♣–2♦–2NT auction, but holding six hearts: jump to
        // the major-suit game in preference to 3NT.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "3.QJ9854.K32.J32"), call(4, Strain::Hearts));
    }

    #[test]
    fn double_negative_two_clubs_may_pass() {
        // 2♣ – 2♥ is the double negative (0–3 HCP); after opener's 2NT the
        // partnership may still stop, so a yarborough passes off-book — the
        // forcing-2♣ floor must not fire once responder has shown the bust.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "8632.J9842.96.42"), Call::Pass);
    }

    #[test]
    fn forced_game_steps_aside_when_penalizing() {
        // 2♣ – 2♦ (game forcing) – 2NT, then they sacrifice in 3♦ and partner
        // doubles for penalty.  Passing the double out is the game-forcing
        // action, so the floor must not pull it to a stopperless 3NT; with six
        // clubs and no diamond guard, show the suit instead.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            call(3, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        assert_eq!(best(&auction, "K3.KQ4.65.QJ8765"), call(4, Strain::Clubs));
    }

    #[test]
    fn milestone_game_opposite_a_limited_rebid() {
        // 1♦–1♥–1NT: opposite the 12–16 rebid a balanced 16 has 28+ combined,
        // a cold 3NT the constructive book never reached (the board that started
        // this).  The floor reads the rebid's strength and bids the game.
        let auction = [
            call(1, Strain::Diamonds),
            Call::Pass,
            call(1, Strain::Hearts),
            Call::Pass,
            call(1, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "J9.AKJ7.K94.A852"), call(3, Strain::Notrump));
        // A 10-count is only invitational (22–24 combined): the floor uses the
        // *guaranteed* minimum, so it stays sound and passes rather than overbid.
        assert_eq!(best(&auction, "KJ9.QJ73.K94.852"), Call::Pass);
    }

    #[test]
    fn milestone_slam_opposite_a_strong_rebid() {
        // 1♦–1♥–2NT is the 18–19 jump rebid; a balanced 16 lifts the combined
        // minimum to 34, the small-slam zone, so bid 6NT instead of stranding in
        // game.  No known major fit, so notrump is the strain.
        let auction = [
            call(1, Strain::Diamonds),
            Call::Pass,
            call(1, Strain::Hearts),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "KQ.AKJ7.K94.8542"), call(6, Strain::Notrump));
    }

    #[test]
    fn milestone_game_opposite_a_competitive_overcall() {
        // LHO opened 3♦, partner overcalled 3♠ (the overcall reading: 5+ ♠,
        // 8+ points), RHO passed.  A 21-count with three-card support lifts the
        // combined minimum to 29 with a known eight-card spade fit, so the floor
        // bids the game it would otherwise miss off-book.
        let auction = [
            call(3, Strain::Diamonds),
            call(3, Strain::Spades),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "K32.AKJ.AQ4.KJ32"), call(4, Strain::Spades));
        // A flat 12-count is only 20 combined: below the milestone, and no raise
        // fits below game, so the floor stays sound and passes rather than overbid.
        assert_eq!(best(&auction, "K32.KJ4.KQ4.5432"), Call::Pass);
    }

    #[test]
    fn milestone_notrump_game_needs_a_stopper_in_competition() {
        // LHO opened 3♣, partner overcalled 3♦, RHO passed.  Game values opposite
        // the overcall, but no major fit and no diamond fit — the strain is 3NT,
        // and the floor must hold a club guard to bid it.
        let auction = [
            call(3, Strain::Clubs),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        // A club stopper (K432): 3NT is the milestone game.
        assert_eq!(best(&auction, "AKQ.AQJ.32.K432"), call(3, Strain::Notrump));
        // No club guard and no fit: pass rather than bid into an unstopped suit.
        assert_eq!(best(&auction, "AKQ4.AKQ4.32.432"), Call::Pass);
    }

    #[test]
    fn rubens_new_suit_transfer() {
        // (1♣) 1♠ (P): advancing partner's spade overcall with our own five-card
        // diamond suit, we transfer — 2♣ shows diamonds (the next suit up).  The
        // floor is 10 upgraded points (a *good* 9 and all 10+), since the
        // transfer commits partner to the two-level.
        let auction = [call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass];
        // A good 9: working K/KQ in a five-card suit upgrades over the floor.
        assert_eq!(best(&auction, "2.K32.KQT54.J432"), call(2, Strain::Clubs));
        // A bare 8 does not reach it: too weak to introduce the suit, pass.
        assert_eq!(best(&auction, "2.Q32.KQT54.J432"), Call::Pass);
    }

    #[test]
    fn rubens_limit_raise_transfer() {
        // (1♣) 1♠ (P): a limit raise of partner's spades goes through the
        // transfer that lands in their suit — 2♥ (the bid just below 2♠).
        let auction = [call(1, Strain::Clubs), call(1, Strain::Spades), Call::Pass];
        assert_eq!(best(&auction, "K54.K32.K43.Q432"), call(2, Strain::Hearts));
    }

    #[test]
    fn rubens_completion_is_mechanical() {
        // (1♣) 1♠ (P) 2♣ (P): partner transferred to diamonds; the overcaller
        // completes into 2♦ regardless of hand.
        let auction = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
        ];
        assert_eq!(
            best(&auction, "AKJ52.K3.952.J32"),
            call(2, Strain::Diamonds)
        );
    }

    #[test]
    fn rubens_two_level_cue_raise() {
        // (1♠) 2♣ (P): partner overcalled at the two level, so the cue (2♠) is
        // the limit-plus raise of clubs — no transfer ladder where there is no room.
        let auction = [call(1, Strain::Spades), call(2, Strain::Clubs), Call::Pass];
        assert_eq!(best(&auction, "432.K32.K2.KQJ54"), call(2, Strain::Spades));
    }

    #[test]
    fn rubens_skips_jump_overcalls() {
        // (1♣) 2♠ (P): partner's 2♠ is a jump (1♠ was available), a preemptive
        // weak jump overcall — not a simple overcall, so no Rubens.  A limit hand
        // with support raises spades naturally rather than transferring.
        let auction = [call(1, Strain::Clubs), call(2, Strain::Spades), Call::Pass];
        assert_eq!(best(&auction, "K54.K32.K43.Q432"), call(3, Strain::Spades));
    }

    #[test]
    fn one_nt_runout_disabled_passes() {
        // Disabled, responder has no runout and falls to the natural floor —
        // Pass — even broke with a five-card suit.
        set_one_nt_runout(false);
        let doubled = [call(1, Strain::Notrump), Call::Double];
        assert_eq!(best(&doubled, "32.QJ763.9742.83"), Call::Pass);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_escapes_to_the_long_suit() {
        set_one_nt_runout(true);
        let doubled = [call(1, Strain::Notrump), Call::Double];
        // A broke hand with five hearts runs to 2♥ rather than sit for it.
        assert_eq!(best(&doubled, "32.QJ763.9742.83"), call(2, Strain::Hearts));
        // Length beats the major preference: six clubs over five spades.
        assert_eq!(best(&doubled, "T9842.3.7.QJ9632"), call(2, Strain::Clubs));
        // A balanced bust has nowhere to run: it sits.
        assert_eq!(best(&doubled, "432.J85.K74.9632"), Call::Pass);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_redoubles_with_values() {
        set_one_nt_runout(true);
        set_runout_xx_min(8);
        let doubled = [call(1, Strain::Notrump), Call::Double];
        // 8 balanced HCP — too good to run, not enough to force game opposite a
        // 15–17 opener (23 combined): redouble to play 1NT-XX.
        assert_eq!(best(&doubled, "K43.KQ5.8642.972"), Call::Redouble);
        // A shapely bust at the same boundary still runs, never redoubles.
        assert_eq!(best(&doubled, "3.QJ763.97642.83"), call(2, Strain::Hearts));
        set_runout_xx_min(7);
        set_one_nt_runout(true);
    }

    #[test]
    fn gambling_3nt_over_double_routes_long_minors() {
        set_one_nt_runout(true);
        set_gambling_3nt_over_double(true);
        set_gambling_3nt_top_honors(2);
        set_gambling_3nt_require_ace(true);
        let doubled = [call(1, Strain::Notrump), Call::Double];

        // A six-card minor headed by its own ace (semi-solid, suit ace) runs to the
        // gambling 3NT — opposite the 15–17 opener the suit cashes — not XX, not an
        // escape.
        assert_eq!(best(&doubled, "32.43.654.AKJ987"), call(3, Strain::Notrump));
        assert_eq!(best(&doubled, "32.43.AKJ987.654"), call(3, Strain::Notrump));

        // A strong balanced hand holds no six-card minor, so the gamble can never
        // steal it: it still defends the business redouble.
        assert_eq!(best(&doubled, "KQ4.KJ43.AQ62.Q5"), Call::Redouble);

        // The suit-ace gate (default on): a semi-solid six-bagger missing its own ace
        // cannot gamble — it escapes.  Drop the requirement and it gambles.
        assert_eq!(best(&doubled, "32.43.654.KQJ987"), call(2, Strain::Clubs));
        set_gambling_3nt_require_ace(false);
        assert_eq!(best(&doubled, "32.43.654.KQJ987"), call(3, Strain::Notrump));
        set_gambling_3nt_require_ace(true);

        // The semi-solid gate: an ace-headed but ragged six-bagger (one top honor)
        // escapes; length-only (top-honors 0) lets it gamble instead.
        assert_eq!(best(&doubled, "32.43.654.AJ9876"), call(2, Strain::Clubs));
        set_gambling_3nt_top_honors(0);
        assert_eq!(best(&doubled, "32.43.654.AJ9876"), call(3, Strain::Notrump));

        set_gambling_3nt_top_honors(2);
        set_gambling_3nt_over_double(false);
        set_one_nt_runout(true);
    }

    #[test]
    fn preempt_4m_over_double_jumps_the_long_major() {
        set_one_nt_runout(true);
        set_preempt_4m_over_double(true);
        set_preempt_4m_top_honors(2);
        set_preempt_4m_require_ace(true);
        let doubled = [call(1, Strain::Notrump), Call::Double];

        // A semi-solid six-card major headed by the trump ace jumps to its game.
        assert_eq!(best(&doubled, "432.AKJ987.65.32"), call(4, Strain::Hearts));
        assert_eq!(best(&doubled, "AKJ987.432.65.32"), call(4, Strain::Spades));

        // The trump-ace gate (default on): a KQ-headed six-bagger lacking the trump
        // ace does not preempt to game (a 6-count escapes); drop it and it jumps.
        assert_eq!(best(&doubled, "432.KQJ987.65.32"), call(2, Strain::Hearts));
        set_preempt_4m_require_ace(false);
        assert_eq!(best(&doubled, "432.KQJ987.65.32"), call(4, Strain::Hearts));
        set_preempt_4m_require_ace(true);

        // The semi-solid gate: an ace-headed but ragged six-bagger escapes;
        // length-only (top-honors 0) lets it preempt.
        assert_eq!(best(&doubled, "432.AJ9876.65.32"), call(2, Strain::Hearts));
        set_preempt_4m_top_honors(0);
        assert_eq!(best(&doubled, "432.AJ9876.65.32"), call(4, Strain::Hearts));

        set_preempt_4m_top_honors(2);
        set_preempt_4m_over_double(false);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_2nt_scrambles_the_minors() {
        set_one_nt_runout(true);
        // The 2NT relay is the opt-in `FourFour` mode (the default is `Direct`).
        set_unusual_2nt(Unusual2nt::FourFour);
        // 4-4 in the minors, no five-card suit, broke: 2NT asks opener to pick.
        let doubled = [call(1, Strain::Notrump), Call::Double];
        assert_eq!(best(&doubled, "K3.842.Q642.J642"), call(2, Strain::Notrump));
        // Opener names the longer minor: clubs here, diamonds when reversed —
        // never blindly "completing" 2NT as a diamond transfer.
        let after = [
            call(1, Strain::Notrump),
            Call::Double,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&after, "AQ5.KQ4.32.AK842"), call(3, Strain::Clubs));
        assert_eq!(best(&after, "AQ5.KQ4.AK842.32"), call(3, Strain::Diamonds));
        set_unusual_2nt(Unusual2nt::Direct);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_2nt_shape_modes() {
        set_one_nt_runout(true);
        let doubled = [call(1, Strain::Notrump), Call::Double];
        // A weak 5-5 in the minors.  In `FourFour` it escapes naturally to a
        // five-card minor; the 2NT scramble is only the no-five-card-suit action.
        set_unusual_2nt(Unusual2nt::FourFour);
        assert_ne!(best(&doubled, "3.42.KQ876.J8765"), call(2, Strain::Notrump));
        // FiveFiveAdd routes the 5-5 hand through 2NT so opener picks the better
        // minor instead of responder guessing.
        set_unusual_2nt(Unusual2nt::FiveFiveAdd);
        assert_eq!(best(&doubled, "3.42.KQ876.J8765"), call(2, Strain::Notrump));
        // Direct suppresses 2NT: the 4-4 bust runs straight to its longer minor
        // (ties to diamonds) at the two level.
        set_unusual_2nt(Unusual2nt::Direct);
        assert_eq!(
            best(&doubled, "K3.842.Q642.J642"),
            call(2, Strain::Diamonds)
        );
        set_unusual_2nt(Unusual2nt::Direct);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_penalizes_escape_on_stack() {
        set_one_nt_runout(true);
        set_penalize_escape_values(false);
        // 1NT-(X)-XX (business redouble); RHO runs to 2♣.  A club stack (and not
        // short in their suit, so the floor would not take out) doubles the run
        // for penalty.  Toggling the arm off withdraws the double.
        let run = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Redouble,
            call(2, Strain::Clubs),
        ];
        set_penalize_escape_stack(true);
        assert_eq!(best(&run, "Q52.K43.Q43.AKJ4"), Call::Double);
        set_penalize_escape_stack(false);
        assert_ne!(best(&run, "Q52.K43.Q43.AKJ4"), Call::Double);
        set_penalize_escape_stack(true);
        set_penalize_escape_values(true);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_leaves_in_escape_penalty() {
        set_one_nt_runout(true);
        set_penalize_escape_stack(true);
        // 1NT-(X)-XX-(2♣)-X-(P): partner doubled their run for penalty.  We pass
        // to leave it in, never advancing it as if it were a takeout double.
        let doubled_run = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Redouble,
            call(2, Strain::Clubs),
            Call::Double,
            Call::Pass,
        ];
        assert_eq!(best(&doubled_run, "KQ3.K54.J632.987"), Call::Pass);
        set_penalize_escape_stack(true);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_penalizes_escape_on_values() {
        set_one_nt_runout(true);
        set_penalize_escape_stack(false);
        set_penalize_escape_values(true);
        // After responder's business redouble shows values, opener doubles their
        // run on general strength — no personal trump stack, and not short in
        // their suit, so the double is ours and not the floor's takeout.
        let run = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Redouble,
            call(2, Strain::Clubs),
        ];
        assert_eq!(best(&run, "AQ5.KQ43.K3.6432"), Call::Double);
        // The chase recurses: they run on to 2♦, the values hand doubles again.
        let again = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Redouble,
            call(2, Strain::Clubs),
            Call::Double,
            call(2, Strain::Diamonds),
        ];
        assert_eq!(best(&again, "KQ3.K54.J632.987"), Call::Double);
        // But opener's *SOS* redouble shows no values, so the values arm stays
        // silent there: 1NT-(X)-P-P-XX(SOS)-(2♣) is not a values double.
        let sos = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            Call::Pass,
            Call::Redouble,
            call(2, Strain::Clubs),
        ];
        assert_ne!(best(&sos, "J32.Q54.J632.987"), Call::Double);
        set_penalize_escape_stack(true);
        set_penalize_escape_values(true);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_universal_opener_escapes_and_sos() {
        set_one_nt_runout(true);
        set_one_nt_runout_universal(true);
        // Balancing seat (1NT-X-P-P): partner is broke, opener acts rather than
        // sit 1NT-X.  A minimum with five spades runs to 2♠.
        let balancing = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        assert_eq!(
            best(&balancing, "AQ542.KJ.K43.Q32"),
            call(2, Strain::Spades)
        );
        // A minimum with no five-card suit SOS-redoubles instead.
        assert_eq!(best(&balancing, "AQ4.KJ2.K432.Q32"), Call::Redouble);
        // Responder answers the SOS with its longest suit, a four-carder.
        let after_sos = [
            call(1, Strain::Notrump),
            Call::Double,
            Call::Pass,
            Call::Pass,
            Call::Redouble,
            Call::Pass,
        ];
        assert_eq!(
            best(&after_sos, "QJ32.842.642.J32"),
            call(2, Strain::Spades)
        );
        set_one_nt_runout_universal(true);
        set_one_nt_runout(true);
    }

    #[test]
    fn one_nt_runout_opener_passes_not_completes_phantom_transfer() {
        set_one_nt_runout(true);
        // 1NT–(X)–2♥ is partner's *runout*, not a Jacoby transfer: opener passes
        // rather than "complete" it to 2♠ (responder's short suit).
        let after_runout = [
            call(1, Strain::Notrump),
            Call::Double,
            call(2, Strain::Hearts),
            Call::Pass,
        ];
        assert_eq!(best(&after_runout, "AQ4.KJ3.KQ52.432"), Call::Pass);
        set_one_nt_runout(true);
    }
}
