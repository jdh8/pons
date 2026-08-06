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
//! Partner's live takeout double — the auction ends `… (bid) X -` with
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
    support, support_point_count, support_point_count_in, takeout_double_shape_ok, they_bid,
    top_honors,
};
use super::context::Context;
use super::inference::{Inferences, Relative, relative_of};
use super::rules::{Alert, FaceId};
use contract_bridge::auction::{Call, RelativeVulnerability};
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
/// 1NT — the `(1NT) X (2Y) X` second double (A/B knob, see [`set_latch_style`])
///
/// The mirror of [`DoubleStyle`][super::american::DoubleStyle] on the defensive
/// side: the same penalty-vs-optional question the we-open `1NT (2X) X` faced.
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
    /// `1NT (2NT) X` — the Unusual-vs-Unusual penalty chase. Default on (it only
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
    /// and the doubler defends by passing or latch-doubles their escape.
    static PENALTY_NO_PULL: Cell<bool> = const { Cell::new(true) };

    /// Whether a weak advancer runs from their *redoubled* penalty double
    /// (`(1NT) X (XX)`, **on by default** — see [`set_advancer_xx_runout`]).  Their
    /// XX is business (BBA and our own system both: "we make 1NT redoubled"), so a
    /// broke advancer escapes to its long suit rather than sit for the doom.
    static ADVANCER_XX_RUNOUT: Cell<bool> = const { Cell::new(true) };

    /// Whether the *doubler* runs after `(1NT) X (XX) - -` comes back around
    /// (**on by default** — see [`set_doubler_xx_runout`]).  Construction-gated:
    /// read once in [`instinct`] so the escape rule lands only in the on book.
    static DOUBLER_XX_RUNOUT: Cell<bool> = const { Cell::new(true) };

    /// Whether advances of partner's simple overcall are Rubens transfers /
    /// the cue-raise (**off by default since 2026-07-31** — the layer was
    /// measured against the natural ladder it replaced and lost; see
    /// [`set_rubens_advances`]).  On restores the transfer structure: raises go
    /// through a relay and the cue-raise means a limit-plus raise.
    static RUBENS_ADVANCES: Cell<bool> = const { Cell::new(false) };

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

    /// A hand that already bid a suit rebids it in competition rather than being
    /// forced to a takeout double (**shipped default-on**; see
    /// [`set_competitive_rebid`]).
    static COMPETITIVE_REBID: Cell<bool> = const { Cell::new(true) };

    /// Opener's balanced-18-19 notrump actions in a `1X (1Y) …` auction the
    /// floor otherwise passes out — reopening 1NT, 3NT over responder's free
    /// 1NT, and responder's raise (**default-on**; see [`set_reopening_notrump`]).
    static REOPENING_NOTRUMP: Cell<bool> = const { Cell::new(true) };

    /// A minimum takeout doubler stops raising partner's *forced* advance on its
    /// own points (the double already showed them) rather than driving to a
    /// doubled game (**default-on**; see [`set_rein_advance_raise`]).
    static REIN_ADVANCE_RAISE: Cell<bool> = const { Cell::new(true) };

    /// Combined-points floor at which the floor's RKCB *ask* (4NT) fires on a
    /// known five-plus-card major fit (A/B knob; see [`set_floor_slam_entry`]).
    /// Default **29** — lowered from the 33 notrump small-slam yardstick to enter
    /// keycarding on the shape-slam band a population probe found (5-3/5-4 fits at
    /// ~29 combined points make a small slam >50% double-dummy within genuine 8+
    /// fits).  The ask's own five-plus decodability gate keeps it off bare 4-4
    /// fits (which would blast the uncontrolled direct milestone), so the lower
    /// floor only ever routes through RKCB's keycard check.  A/B'd a plain-DD win
    /// at both vulnerabilities (~+0.005 IMPs/board, PD in lockstep); `33` restores
    /// the pre-knob gate.  29 beat 28 (28's marginal fires overreach and dilute).
    static FLOOR_SLAM_ENTRY: Cell<u8> = const { Cell::new(29) };

    /// Combined-points floor at which the floor bids a major game on a known
    /// eight-plus fit, *counting the trump length as points* — the total-tricks
    /// yardstick where a ninth trump ≈ a point (threshold knob; see
    /// [`set_fit_sum_game`]).  Game once
    /// `own_points + partner_shown_floor + (own_len + partner_shown_len) >= t`, so
    /// an eight-card fit games at `t - 8` combined, a nine-card fit at `t - 9`, a
    /// ten-card fit at `t - 10` — strictly lighter as the fit lengthens.  Default
    /// `31` is the dual-metric peak of a swept boundary (34→31 each a CI-clean
    /// plain-DD gain with perfect defense tracking; at 30 the NV perfect-defense
    /// line turns negative — a doubling artifact).  Proven default-on, so there is
    /// no off-state — the threshold is always armed.
    ///
    /// `31` holds under the default-on `support_point_count` scale: re-probed
    /// 2026-07-14 (`ab-fit-sum-game --support-points`, 200k×2vul), 32-vs-31 is NV
    /// PD +0.004 but **vul PD −0.004 (parity/behind)** — not a bump.  The gate
    /// re-adds `own_len`, which the scale's own long-suit term already counts, so
    /// the hotness self-cancels and the peak stays at 31 (an earlier, broader
    /// re-probe under the since-deleted global scale had suggested 32; the
    /// fit-known-only scale that shipped is narrower and refuted it).
    static FIT_SUM_GAME: Cell<u8> = const { Cell::new(31) };

    /// The *bilans* floor: game/slam boundary gates priced by the learned trick
    /// evaluator instead of point arithmetic (see [`set_bilans_floor`]).
    static BILANS_FLOOR: Cell<bool> = const { Cell::new(true) };

    /// Collar the bilans net's licence instead of letting it replace the point
    /// arithmetic: the net accelerates at game and only vetoes at slam (see
    /// [`set_net_collar`]).  Default off — byte-identical to the shipped mask.
    static NET_COLLAR: Cell<bool> = const { Cell::new(false) };

    /// Edit 1: read partner's fit-known strength off the dedicated
    /// `support_points` gauge in [`fit_sum_game`], falling back to the
    /// length-scale `points` when it is unpopulated (see
    /// [`set_fit_sum_support_read`]).  Default off — byte-identical to reading
    /// `points`.
    static FIT_SUM_SUPPORT_READ: Cell<bool> = const { Cell::new(false) };

    /// Edit 2: value the notrump game/slam milestones on raw HCP — own hand and
    /// partner's crisp `hcp` gauge — instead of the length-upgraded `point_count`
    /// (see [`set_nt_hcp_read`]).  Default off — [`combined_hcp`] is then
    /// [`combined_points`] verbatim.
    static NT_HCP_READ: Cell<bool> = const { Cell::new(false) };
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

/// Enable the *bilans* floor on the current thread: the game/slam boundary
/// gates price their calls with the learned trick evaluator instead of the
/// point-count arithmetic
///
/// **Shipped default-on** (2026-07-21).  The `ab-bilans-floor` duplicate match
/// over 200 000 boards per arm put it ahead on *both* scorers at *both*
/// vulnerabilities — non-vul +0.036 IMPs/board plain DD (95% CI [+0.030,
/// +0.042]) and +0.009 PD ([+0.002, +0.016]); vulnerable +0.065 plain DD
/// ([+0.057, +0.073]) and +0.013 PD ([+0.003, +0.022]).  Both arms clear the
/// decision table's win/win row, and the gain is larger vulnerable than
/// non-vul (as is the divergence rate, 4.29% vs 3.52%) — the vulnerability
/// axis the point gates were blind to, moving the direction the design
/// predicts.  Pass `false` to recover the point-gate arithmetic.
///
/// With it on, each converted gate asks
/// [`trick_estimates`][super::evaluator::trick_estimates] for the contract's make
/// probability and compares it against the IMP break-even for that decision at
/// the live vulnerability ([`break_even`]) — partscore→game at even money
/// non-vul / 44.4% vul (our failing branch priced *doubled*, per the
/// bid-scoring split), small slam at even money, grand at ~56–58% — the
/// vulnerability-awareness the point gates never had.  The RKCB ask enters at
/// [`SLAM_ENTRY_P`] instead of the [`set_floor_slam_entry`] point floor.
///
/// The name tips the hat to Edward Piwowar: *bilans* (Polish for "balance") is
/// his term for the always-running deal evaluator inside EPBot, the engine
/// behind BBA, whose reverse-engineering (`docs/ai-bidder/bba-floor.md` §5)
/// inspired this floor.  His is analytic winner/loser arithmetic on
/// reconstructed hands; ours is the session-C learned net over the same
/// question.
///
/// Flip it only on threads that classify through a [`Stance`][super::Stance]
/// (every real harness does): the net was fit on trie-prefixed readings, and a
/// bare [`Context`] hands it the looser projection-less ranges it was not
/// trained on.  Read at classification time, per-thread.
#[doc(hidden)]
pub fn set_bilans_floor(enabled: bool) {
    BILANS_FLOOR.with(|flag| flag.set(enabled));
}

/// The bilans floor is enabled (see [`set_bilans_floor`])
fn bilans_floor() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| bilans_enabled())
}

/// Plain-bool read of the bilans knob, for gates that must short-circuit ahead
/// of a net forward pass (see [`net_break_even_gate`])
fn bilans_enabled() -> bool {
    BILANS_FLOOR.with(Cell::get)
}

/// Collar the bilans net instead of letting it replace the point arithmetic
///
/// [`set_bilans_floor`] as shipped *masks the authored arithmetic off* and hands
/// the net the whole criterion — an unbounded veto over hands the point sums
/// accept, and an unbounded reach below them.  With this on, the arithmetic is
/// the criterion again and the net rules on it in one direction only, chosen by
/// the decision's own IMP economics ([`break_even`]):
///
/// | decision | `tricks` | break-even, both vuls | net's licence |
/// | --- | --- | --- | --- |
/// | game | ≤ 11 | never *above* even money | accelerate — [`points_or_net`] |
/// | slam | ≥ 12 | never *below* even money | veto — [`points_and_net`] |
///
/// A game is taken at or below even money, so the cheap direction is to *add*
/// hands the point sums decline, bounded by a collar ([`COLLAR_SLACK`] below the
/// authored threshold).  A slam needs at or above even money, so the net may only
/// *decline*.  The two boundary rows sit exactly on 0.5 (non-vul game under the
/// bid-scoring doubling premium, and the small slam on every convention — its
/// bonus is symmetric), where the economics give no direction; the tie-break
/// there is structural, since a veto keeps the authored reading and an
/// accelerator does not (`docs/ai-bidder/evaluator-net.md`, "The reach ceiling").
///
/// Default off — byte-identical to the shipped mask in every
/// [`set_bilans_floor`] state.  Read at classification time, per-thread.
#[doc(hidden)]
pub fn set_net_collar(enabled: bool) {
    NET_COLLAR.with(|flag| flag.set(enabled));
}

/// The net collar is enabled (see [`set_net_collar`])
fn net_collar() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, _: &Context<'_>| NET_COLLAR.with(Cell::get))
}

/// How far below an authored game threshold the collared net may reach — the
/// invitational band, where a human already calls it judgment
// ponytail: slack fixed at 2; a live NET_COLLAR_SLACK thread-local is the
// upgrade if the A/B lands close enough to want it swept.
const COLLAR_SLACK: u8 = 2;

/// Enable opener's/overcaller's competitive rebid of a long suit (**shipped default-on**)
///
/// Once our side has bid, the deterministic floor's only competitive actions are
/// raising partner and the takeout double — so a self-sufficient one-suiter
/// (e.g. `1♦ (1♥) - (2♥)` holding `AKJT984`) can only double, misdescribing a
/// takeout shape it does not have.  With this on, a suit we already bid and hold
/// six-plus in is rebid at the cheapest legal level, outranking that double; the
/// existing raise ladder then carries responder to game.  The two-level rebid is
/// unconditional; the more committal three-level rebid demands a real source of
/// tricks (seven cards, or a good six — two of the top three honors).
///
/// A/B (SEED_BASE 1783316036, 102.4k bd/arm/vul): plain **+0.047/+0.037** IMPs/bd
/// NV/vul, PD **+0.040/+0.023**, all four cells' CIs exclude 0.  Disable with
/// `bba-gen --no-ns-competitive-rebid`.  Read at book construction.
#[doc(hidden)]
pub fn set_competitive_rebid(enabled: bool) {
    COMPETITIVE_REBID.with(|flag| flag.set(enabled));
}

/// The competitive long-suit rebid is enabled (see [`set_competitive_rebid`])
fn competitive_rebid_enabled() -> bool {
    COMPETITIVE_REBID.with(Cell::get)
}

/// Author opener's balanced-18-19 notrump actions in a `1X (1Y) …` auction the
/// floor otherwise passes out: the reopening 1NT (`1X (1Y) - -` back to
/// opener with their suit stopped), 3NT over responder's free 1NT, and
/// responder's raise of the reopening 1NT.  Default on; the off state restores
/// the lone-takeout-double floor for the A/B (`bba-gen --no-ns-reopening-notrump`).
/// Read at book construction.
#[doc(hidden)]
pub fn set_reopening_notrump(enabled: bool) {
    REOPENING_NOTRUMP.with(|flag| flag.set(enabled));
}

/// Opener's contested notrump actions are enabled (see [`set_reopening_notrump`])
fn reopening_notrump_enabled() -> bool {
    REOPENING_NOTRUMP.with(Cell::get)
}

/// Rein in a minimum takeout doubler that over-raises partner's forced advance
///
/// After we double an opponent's suit for takeout and partner names a suit — a
/// forced, possibly bust advance — a minimum doubler (< 17 total points) that
/// re-doubles or raises the advance to the three level double-counts its values
/// and drives to a doubled game (`1X (1Y) - (1Z) X - 2m (2Z) 3m (X) …`).  Default on;
/// the off state restores the blind raise ladder for the A/B
/// (`bba-gen --no-ns-rein-advance-raise`).  Read at book construction.
#[doc(hidden)]
pub fn set_rein_advance_raise(enabled: bool) {
    REIN_ADVANCE_RAISE.with(|flag| flag.set(enabled));
}

/// The advance-raise rein is enabled (see [`set_rein_advance_raise`])
fn rein_advance_raise_enabled() -> bool {
    REIN_ADVANCE_RAISE.with(Cell::get)
}

/// Enable or disable Rubens advances of partner's simple overcall
///
/// **Off by default since 2026-07-31 — a reversal on re-measure.**  The layer
/// won its M6.3 A/B (2026-07-02, plain +0.0016 ±0.0015 with the CI excluding
/// zero, PD −0.0009 wash, 1144 fired) once both sides' continuations were
/// authored, and shipped default-on on that.  Re-measured on the current
/// system (`scripts/ab-rubens.sh`, 204,800 bd/arm/vul, SEED_BASE 1785426828,
/// sha 4485555) it **loses in all four cells** — plain −0.0009 ±0.0009 NV / −0.0008 ±0.0011 vul, PD
/// −0.0014 ±0.0011 / −0.0014 ±0.0013 (both PD CIs clear of zero), fired
/// 0.11%/0.09%, −0.83/−0.85 plain and −1.29/−1.51 PD per fired board.  The
/// default is now the natural ladder: raises stay the natural ladder (the
/// limit distinction is lost, the honest natural price) and a natural
/// two-level new-suit advance covers the hands the transfers covered.
///
/// On, the calls from the cue up to just below partner's suit are transfers
/// over a one-level overcall, and the cue is the limit-plus raise over a
/// two-level one.  The firing rate fell with the verdict — 1144 fired at M6.3
/// against 218/193 now on the same 204,800 boards, so the floor around it
/// moved and the transfers are reached far less often than when they won.
/// Kept as a knob for re-measure — the losing tail is
/// over-reach (`1♦ (1♠) - (2♥)` climbing to a failing `4♠` where the natural arm
/// stops), not one unauthored continuation, and the transfers were mostly not
/// reached in the first place: over `1♣ (1♥) -` the bidder took the `2♦`
/// transfer 0.4% of the time, holding six-plus *diamonds*
/// (`probe-bba-constraints --mode rub-ch --ours`,
/// docs/reader-retirement.md §The Rubens layer).
///
/// Read at classification time, per-thread.  The [`Inferences`] reading shares
/// the knob: off, an advance in the band is a genuine suit — so this default
/// also silences `rubens_reading`.
///
/// [`Inferences`]: super::inference::Inferences
#[doc(hidden)]
pub fn set_rubens_advances(enabled: bool) {
    RUBENS_ADVANCES.with(|flag| flag.set(enabled));
}

/// Rubens advances are enabled (see [`set_rubens_advances`]); shared with the
/// [`Inferences`](super::inference::Inferences) reading so the bidder and the
/// reader flip together
pub(super) fn rubens_advances_enabled() -> bool {
    RUBENS_ADVANCES.with(Cell::get)
}

/// The floor's 4NT keycard ask and its 1430 answers are artificial (M6.4);
/// the alert suppresses their natural reading — without it partner would read
/// a 5♦ answer as a diamond suit and the sampler would deal the phantom.
const RKCB_FLOOR: Alert = Alert("floor:rkcb");
const FACE_RKCB_ANSWER: FaceId = FaceId::new("rkcb:answer-window", 0);
const FACE_RKCB_ROPI: FaceId = FaceId::new("rkcb:ropi-window", 0);
const FACE_RKCB_DOPI: FaceId = FaceId::new("rkcb:dopi-window", 0);

const fn rkcb_relay_face(back: u8) -> FaceId {
    FaceId::new("rkcb:relay-window", back)
}

const fn rkcb_relocated_ask_face(target: Suit) -> FaceId {
    FaceId::new("rkcb:relocated-ask", target as u8)
}

std::thread_local! {
    /// Whether the floor asks and answers RKCB 1430 once a fit and small-slam
    /// values are known (**on by default**, M6.4).  See [`set_floor_rkcb`].
    static FLOOR_RKCB: Cell<bool> = const { Cell::new(true) };

    /// Whether an uncontested 2/1 marks the auction forced to game.  **On by
    /// default** since 2026-07-20; see [`set_two_over_one_force`].
    static TWO_OVER_ONE_FORCE: Cell<bool> = const { Cell::new(true) };

    /// Whether a live 2/1 floors partner's shown strength for the slam-entry
    /// gate.  **On by default** since 2026-07-20; see
    /// [`set_two_over_one_slam_strength`].
    static TWO_OVER_ONE_SLAM_STRENGTH: Cell<bool> = const { Cell::new(true) };

    /// Whether the floor's keycard ask reaches agreed **minors** as well as
    /// majors.  **On by default** since 2026-08-01; see [`set_rkcb_minors`].
    static KEYCARD_MINORS: Cell<bool> = const { Cell::new(true) };

    /// Which relocation stance the keycard ask plays.  **[`Plain`] by
    /// default** — Kickback was default-on 2026-08-02 to 2026-08-03 only; see
    /// [`set_rkcb_variant`].
    ///
    /// [`Plain`]: RkcbVariant::Plain
    static RKCB_VARIANT: Cell<RkcbVariant> = const { Cell::new(RkcbVariant::Plain) };

}

/// Enable or disable the floor's two-over-one game force (**on by default**)
///
/// The authored book has always held this invariant by *omission* — no table in
/// the 2/1 game-force book carries a `Pass` rule, so pass
/// scores −∞ and a 2/1 auction cannot die below game.  The floor never learned
/// it, which did not matter while the game backstop covered every uncovered
/// continuation.  Deleting that node
/// ([`set_game_backstop`][super::american::set_game_backstop], now the default)
/// exposed the gap: against BBA, 24% of the affected boards had our side
/// settling below game in an established 2/1 — opener passing responder's 3♣ out
/// in a partscore.
///
/// On, an uncontested 2/1 sets `Interpretation::forced_to_game`, so the floor
/// takes the cheapest game milestone instead of passing.  Measured on top of the
/// deletion: **+0.0067/+0.0102 plain, +0.0060/+0.0094 PD** IMPs/board NV/vul vs
/// BBA (409,600×2, all CI>0), firing on 606/622 boards — exactly the set that
/// abandoned the force — at +4.5/+6.7 IMPs each.  It costs routing those nodes
/// through the deterministic ladder rather than the learned net, since the
/// [shell][super::neural_floor] delegates wholesale on a forced auction; that
/// price is inside the measurement.
///
/// Uncontested only, matching the `Undisturbed` guard the deleted node carried:
/// over interference a two-level new suit is a free bid, not a game force.
pub fn set_two_over_one_force(on: bool) {
    TWO_OVER_ONE_FORCE.with(|cell| cell.set(on));
}

pub(super) fn two_over_one_force() -> bool {
    TWO_OVER_ONE_FORCE.with(Cell::get)
}

/// Enable or disable the two-over-one strength floor on the slam-entry gate
///
/// **On by default.**  The 2/1 response is alerted
/// (`GAME_FORCE`), so the inference walk skips its natural reading and takes the
/// rule's projection instead — and the rule gates on `points(13..)`, which on
/// the rule-of-N+8 scale soundly projects to *no* high-card floor at all (a
/// 13-point hand can be an eight-count with a six-card suit).  Partner therefore
/// reads as **zero** through an established game force, and
/// the slam-entry gate never fires on these auctions: opener holding a
/// 26-count opposite the force counts `26 + 0 < 29` and signs off in game.
///
/// On, partner's shown minimum is floored at the 13 points the two-over-one
/// promised — the same scale our own [`support_point_count`] term already uses,
/// so the sum stays consistent.  Only when *partner* made the two-over-one:
/// opener's one-level opening is read naturally and needs no floor.
///
/// Measured vs BBA (409,600×2, both scorers): **+0.0032/+0.0042 plain,
/// +0.0031/+0.0041 PD** IMPs/board NV/vul, all CI>0, firing on 0.08%/0.09% of
/// boards at +3.8/+4.8 IMPs each.  The same run priced deleting
/// [`opener_third`][super::american::set_opener_third] *on top of* this floor at
/// +0.0003/+0.0004 with the CI straddling zero, so that node stays: the
/// constructive re-audit's candidate #2 was starved of a reading, not shadowing
/// a better call.
///
/// Read at classification time, per-thread.
pub fn set_two_over_one_slam_strength(on: bool) {
    TWO_OVER_ONE_SLAM_STRENGTH.with(|cell| cell.set(on));
}

/// Partner's shown minimum points, floored by a live 2/1 (see
/// [`set_two_over_one_slam_strength`])
fn partner_slam_strength(context: &Context<'_>) -> u8 {
    let shown = context.inferences().partner().strength.shown_floor();
    if !TWO_OVER_ONE_SLAM_STRENGTH.with(Cell::get) || !two_over_one_game_force(context) {
        return shown;
    }
    // Partner made the two-over-one exactly when we are the opener: the opening
    // and the call to make share a lane.
    let auction = context.auction();
    match opening_bid(auction) {
        Some((index, _)) if index % 4 == auction.len() % 4 => shown.max(13),
        _ => shown,
    }
}

/// Enable or disable the floor's RKCB 1430 (M6.4)
///
/// **On by default**: with a known eight-card fit and combined small-slam
/// values the floor asks 4NT before committing — the milestone 6-of-the-fit
/// only fires directly at the grand-zone 37 or when the ask has no room.  The
/// answers reuse the book's 1430 ladder ([`american`](super::american)'s
/// keycard counting), so instinct decodes instinct on both sides.  Disable to
/// recover the direct-milestone floor (the A/B off arm, `bba-gen
/// --no-ns-floor-rkcb`); read at classification time, per-thread.
///
/// **The outer gate of the whole keycard package.**  Off, there is no ask to
/// relocate, so [`relocating_now`] is false whatever [`set_rkcb_variant`] says
/// and [`set_rkcb_minors`] has nothing to widen — the card discloses plain 4NT
/// and the plain twin serves the floor.
#[doc(hidden)]
pub fn set_floor_rkcb(enabled: bool) {
    FLOOR_RKCB.with(|flag| flag.set(enabled));
}

/// The floor RKCB is enabled (see [`set_floor_rkcb`])
pub(in crate::bidding) fn floor_rkcb_now() -> bool {
    FLOOR_RKCB.with(Cell::get)
}

/// Let the floor's keycard ask reach agreed **minors** (**on by default**)
///
/// [`keycard_trump`] was majors-only by measured decision — round 4 of the M6.4
/// A/B lost to the milestone 6NT power-blast on minor and thin 6-2 asks — and
/// **that verdict expired on the 2026-08 system**.  Arm B of the three-arm
/// kickback A/B (`ab-kickback`, 1M boards a cell, seed 1785546026) re-priced it
/// and the ask now beats the blast on every scoring row at both vulnerabilities:
/// **+0.00394 PD / +0.00375 plain DD** per board vul none (1840 divergent) and
/// **+0.00502 / +0.00471** vul both (1753), every 95% CI clear of zero.  Half
/// the gain is the ask *declining* a slam — `5♦ vs 6♦` ran +44 IMPs over four
/// audited boards where the majors-only arm blasted six without the keycards.
///
/// Off, the ask reverts to majors-only — the plain-4NT carve this knob owns.
/// A live relocation implies the minors' reach regardless
/// (`minor_asks_now`): [`RkcbVariant::Redwood`] and [`RkcbVariant::Kickback`]
/// bring their own minor lanes.  The tie-break still prefers the higher
/// suit either way, so a major fit of equal length keeps winning.  Read at
/// classification time, per-thread.
#[doc(hidden)]
pub fn set_rkcb_minors(enabled: bool) {
    KEYCARD_MINORS.with(|flag| flag.set(enabled));
}

/// The keycard ask reaches minors (see [`set_rkcb_minors`])
pub(in crate::bidding) fn rkcb_minors_now() -> bool {
    KEYCARD_MINORS.with(Cell::get)
}

/// Where the keycard ask lives — the relocation stance of the 1430 machinery
///
/// The variants are the *playable* cells of what used to be two independent
/// bools (`set_kickback` + `set_redwood`, folded 2026-08-03): the full ladder
/// implies the Redwood scope — a ladder that relocated only hearts would
/// relocate exactly one call, since spades asks at 4NT either way ("there is
/// no point to kickback only hearts", jdh8 2026-08-03) — so a hearts-only
/// ladder was unrepresentable and `(kickback, redwood)` had four cells for
/// three stances.  One `Cell` makes the honest domain the type, the
/// [`NotrumpDefense`][super::american::NotrumpDefense] precedent.  Selected
/// by [`set_rkcb_variant`]; either relocation implies the minors' reach
/// whatever [`set_rkcb_minors`] says (`minor_asks_now`), because a ladder
/// whose payoff is the minor lanes needs a minor to ask in.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum RkcbVariant {
    /// The ask is plain 4NT for every trump — the shipped default.
    #[default]
    Plain,

    /// The minor half of the ladder alone — 4♦ asks in clubs and 4♥ in
    /// diamonds, the majors keep plain 4NT: the stance partnerships actually
    /// play under the name Redwood.  The minors are where relocation pays: a
    /// plain 4NT ask over an agreed minor blows past five of trump on half
    /// its answers, while the majors barely move.
    ///
    /// **Unmeasured as its own arm.**  The full-kickback A/B lost (see
    /// [`Kickback`][Self::Kickback]) and its per-lane cut charged the minor
    /// lanes too, but a per-lane cut of one arm prices no stance — the
    /// routing moves with the ladder.  Opt-in until an arm of its own says
    /// otherwise.
    ///
    /// The floor's weights approximate under this stance: Redwood has no
    /// distilled twin, so the v3 floor serves the kickback twin — which reads
    /// the relocated minor lanes correctly and is merely trained to expect a
    /// 4♠ ask the hearts lane no longer makes — and the generated card
    /// discloses "Kickback 1430", the nearest row BBA's card offers (it
    /// over-claims the hearts lane the same way).  Reading and weights stay
    /// consistent with each other either way; the approximation is to the
    /// *stance*, and a twin of its own is owed before this could ever ship
    /// default-on.
    Redwood,

    /// The full ladder: the Redwood scope plus 4♠ asking in hearts — the
    /// cheapest unguarded suit above each trump, so every 1430 answer lands
    /// at or below five of trump instead of blowing past it.  The ladder is
    /// `kickback_ladder`, face-only, so both members derive the same asking
    /// call with no reading at all.  4NT keeps its meaning throughout:
    /// kickback *adds* asks, it never removes one.
    ///
    /// **Measured a loss, and the loss survived the defence.**  It shipped
    /// default-on 2026-08-02 on a PD win under the *twin* nets, where the
    /// arms differed by a two-week-newer artifact as well as by the
    /// convention.  The configured net (`features_v4`) made the fair
    /// comparison possible — one net, arms one card row apart — and gate 2
    /// read plain DD −0.0105/−0.0092 (NV/vul, 2M boards a cell, seed
    /// 1785708870), PD parity, **sd-declarer −0.0088/−0.0073** with both
    /// intervals clear of zero.  Every relocated lane loses: ♥ −1.09
    /// PD/board over 391 boards, ♦ −3.76 over 230, ♣ −1.28 over 144.
    ///
    /// The ladder's *arithmetic* is not in doubt — a relocated ask genuinely
    /// brings every 1430 answer to at or below five of trump.  What it costs
    /// is the faces: 4♦/4♥/4♠ are among the most common natural calls in
    /// bridge, and the room the relay buys does not pay for them back.  The
    /// obvious objection — double dummy never lets a thin slam fail, so it
    /// structurally charges the arm that *stops* — was tested with the
    /// sd-declarer row and failed.  Do not re-raise it without a scorer that
    /// fights DD's slam optimism the way sd fights its defensive optimism.
    /// Full ledger: `docs/ai-bidder/bba-kickback.md` §7.13.
    Kickback,
}

/// Select the keycard ask's relocation stance (**[`Plain`] by default**)
///
/// The per-suit scope lives in `kickback_ladder`'s claim loop, so every
/// recognizer and rule downstream inherits one claim table instead of
/// re-deriving the stance.
///
/// **Read in two regimes, and a harness must arm both.** Rule *presence* is
/// gated at [`instinct`] build time, because the reading's `alerted` test is
/// structural — it asks whether any rule on the made call carries an alert,
/// and never evaluates the constraint — so an always-present alerted rule on
/// 4♠ would suppress the natural reading of *every* floor-classified 4♠ even
/// in the plain stance.  The recognizer (`keycard_ask_bid`, and
/// `keycard_conversation_now` outside the rules table) is read at
/// classification time.  Build one stance per arm **and** set the variant per
/// call by side; arming only one gives dead alert sites (rules present,
/// recognizer off) or a phantom ask (recognizer on, rules absent).
///
/// **The stance also selects the floor's weights.**  A relocation is not a
/// rule the reader can hold alone: a net distilled from a kickback-blind
/// teacher keeps bidding a natural 4♥ into the relocated ask, so
/// `classify_bba` serves the kickback twin whenever a relocation is live.
/// [`Plain`] restores both halves.
///
/// [`Plain`]: RkcbVariant::Plain
pub fn set_rkcb_variant(variant: RkcbVariant) {
    RKCB_VARIANT.with(|cell| cell.set(variant));
}

/// The keycard ask's relocation stance (see [`set_rkcb_variant`])
pub(in crate::bidding) fn rkcb_variant_now() -> RkcbVariant {
    RKCB_VARIANT.with(Cell::get)
}

/// Some relocation is live — the full ladder or its minor half
/// ([`RkcbVariant`]), *and* the floor still has a keycard ask to relocate.
/// The build-time gate for the relocated answer set and the relocated-ask
/// rules; the *per-suit* scope is [`kickback_ladder`]'s claim loop, so every
/// recognizer downstream of the ladder inherits it.
///
/// The [`set_floor_rkcb`] conjunct is what keeps the stance *consistent* across
/// its three consumers.  The relocated rules' `face` gate always carried it, but
/// the convention card ([`card.rs`](super::card)) and the distilled-net
/// selection ([`classify_bba`](super::neural::classify_bba)) read the variant
/// alone — so `(floor_rkcb = off, variant = Kickback)` published `Kickback 1430`
/// on the generated card and served the kickback twin while the floor made no
/// relocated ask at all.  A knob cross-product has to name a stance a
/// partnership could play; that one disclosed a convention we did not.
pub(in crate::bidding) fn relocating_now() -> bool {
    floor_rkcb_now() && rkcb_variant_now() != RkcbVariant::Plain
}

/// The keycard ask reaches agreed minors — carved in by [`set_rkcb_minors`],
/// or implied by a live relocation ([`relocating_now`]): a ladder whose payoff
/// is the minor lanes with no minor to ask in would be "kickback only hearts",
/// a stance nobody plays.
pub(in crate::bidding) fn minor_asks_now() -> bool {
    rkcb_minors_now() || relocating_now()
}

/// The combined trump length at which the fit itself stands in for the trump
/// queen, matching BBA's `posiadane_karty >= 10`
/// (`docs/ai-bidder/bba-kickback.md` §3)
///
/// Ten trumps run the suit; nine do not, and a grand breaks even near 56–58%,
/// so letting nine answer "queen" would make the reply mean *honour or length*
/// — ambiguous exactly where a wrong reading is most expensive.  Nine takes the
/// other road out, [`QUEEN_BUFF_FIT`].  Every reply on this rung is therefore a
/// real queen or a suit that needs none.
///
/// Was a knob (`set_queen_fit`) while the relay was being tuned; the tuning is
/// done, and a threshold nobody varies is a constant.
pub(in crate::bidding) const QUEEN_FIT: u8 = 10;

/// The combined trump length that is a *buff* rather than a queen — the
/// answerer jumps to six of trumps instead of denying
///
/// The queen ask has three answers, not two, and this is the threshold for the
/// third: no queen, but something RKCB has no rung for and partner cannot see.
/// An asker who hears a denial holds four keycards and no queen and *will* pass
/// five — right on an eight-card fit (six makes 45.8% double-dummy, 51.9%
/// de-biased) and wrong on a nine-card fit (**76.0%** / **82.9%**,
/// `probe-trump-queen` over 120k deals at 30-plus combined points).  Six is the
/// honest bid: it never claims the honour and it never gets passed.  A
/// side-suit void rides the same rung, being worth a trick the arithmetic
/// cannot see and the ladder cannot show.
///
/// Was a knob (`set_queen_buff_fit`), retired with [`QUEEN_FIT`].
pub(in crate::bidding) const QUEEN_BUFF_FIT: u8 = 9;

/// The combined trump length that stands in for the queen (see [`QUEEN_FIT`])
pub(in crate::bidding) const fn queen_fit() -> u8 {
    QUEEN_FIT
}

/// The combined trump length that is a buff, not a queen (see
/// [`QUEEN_BUFF_FIT`])
pub(in crate::bidding) const fn queen_buff_fit() -> u8 {
    QUEEN_BUFF_FIT
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

/// Combined-points floor at which the floor's RKCB ask fires on a known
/// five-plus major fit (see [`FLOOR_SLAM_ENTRY`]).  For A/B measurement:
/// default 33; ~28–29 enters keycarding on shape-slam values.
#[doc(hidden)]
pub fn set_floor_slam_entry(threshold: u8) {
    FLOOR_SLAM_ENTRY.with(|cell| cell.set(threshold));
}

/// The combined-points floor at which the floor bids a major game on a known
/// eight-plus fit, counting the trump length as points (see [`FIT_SUM_GAME`]).
/// For A/B measurement of the threshold itself — the shipped default is `31`;
/// the `support_points` flip re-probes `32`.
#[doc(hidden)]
pub fn set_fit_sum_game(threshold: u8) {
    FIT_SUM_GAME.with(|cell| cell.set(threshold));
}

/// Edit 1 — read partner's fit-known strength off the dedicated `support_points`
/// gauge in [`fit_sum_game`] instead of the length-scale `points` (default off).
/// For A/B measurement (`ab-fit-sum-game`).
#[doc(hidden)]
pub fn set_fit_sum_support_read(on: bool) {
    FIT_SUM_SUPPORT_READ.with(|cell| cell.set(on));
}

/// Edit 2 — value the notrump game/slam milestones ([`combined_hcp`]) on raw HCP
/// instead of the length-upgraded `point_count` (default off).  For A/B measurement.
#[doc(hidden)]
pub fn set_nt_hcp_read(on: bool) {
    NT_HCP_READ.with(|cell| cell.set(on));
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

/// Enable or disable the Unusual-vs-Unusual penalty chase after `1NT (2NT) X`
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

/// The index and bid of our side's natural 1NT that anchors a runout — either
/// the auction's opening 1NT, or a 1NT overcall made immediately over the
/// opponents' 1-level suit opening.  Ignores whose turn it is; callers layer
/// their own seat check (see [`our_one_nt_runout_seat`]).
fn one_nt_anchor(auction: &[Call]) -> Option<(usize, Bid)> {
    let (open_index, opening) = opening_bid(auction)?;
    let nt = Bid::new(1, Strain::Notrump);
    if opening == nt {
        Some((open_index, nt))
    } else if opening.level.get() == 1
        && opening.strain.suit().is_some()
        && auction.get(open_index + 1) == Some(&Call::Bid(nt))
    {
        Some((open_index + 1, nt))
    } else {
        None
    }
}

/// Our side bid a natural 1NT (opening *or* overcall) that anchors a runout,
/// and the player to act relates to it as `partner` (`false` = the 1NT bidder
/// acting again, `true` = its partner).  The overcall-aware generalization of
/// [`our_strong_notrump`]`(context, 1, partner)`; returns the anchor's index.
fn our_one_nt_runout_seat(context: &Context<'_>, partner: bool) -> Option<usize> {
    let auction = context.auction();
    let (anchor, _) = one_nt_anchor(auction)?;
    // Our side owns the indices sharing the player-to-act's parity.
    if anchor % 2 != auction.len() % 2 {
        return None;
    }
    // Seats four apart are the same player; two apart are partners.
    match (auction.len() - anchor) % 4 {
        0 if !partner => Some(anchor),
        2 if partner => Some(anchor),
        _ => None,
    }
}

/// Partner bid a strong 1NT (opening or overcall), RHO doubled it, and it is
/// responder/advancer's first turn — the runout situation.  The double need not
/// be penalty: left in, any double of 1NT plays for the penalty, so a weak
/// partner escapes regardless.
fn responder_one_nt_runout_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let n = auction.len();
    n >= 2
        && auction.last() == Some(&Call::Double)
        && our_one_nt_runout_seat(context, true) == Some(n - 2)
}

/// [`responder_one_nt_runout_now`] as a hand-ignoring [`Constraint`]
fn responder_one_nt_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| responder_one_nt_runout_now(context))
}

/// We opened a strong 1NT, LHO doubled, partner ran out to a suit, and it is
/// our turn again.  Responder ran because it is weak, so it captains the
/// auction: opener passes rather than read the escape as a natural new suit.
fn opener_after_one_nt_runout_now(context: &Context<'_>) -> bool {
    let Some(anchor) = our_one_nt_runout_seat(context, false) else {
        return false;
    };
    let auction = context.auction();
    auction.get(anchor + 1) == Some(&Call::Double)
        && matches!(
            auction.get(anchor + 2),
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
    let Some(anchor) = our_one_nt_runout_seat(context, false) else {
        return false;
    };
    let auction = context.auction();
    auction.get(anchor + 1) == Some(&Call::Double)
        && auction.get(anchor + 2) == Some(&Call::Bid(Bid::new(2, Strain::Notrump)))
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
    let Some(anchor) = our_one_nt_runout_seat(context, false) else {
        return false;
    };
    let auction = context.auction();
    auction.len() == anchor + 4
        && auction.get(anchor + 1) == Some(&Call::Double)
        && auction[anchor + 2] == Call::Pass
        && auction[anchor + 3] == Call::Pass
}

/// [`opener_balancing_runout_now`] as a hand-ignoring [`Constraint`]
fn opener_balancing_runout() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_balancing_runout_now(context))
}

/// We opened one of a suit, LHO overcalled a suit, partner passed, and it is
/// back to us (RHO passed or raised) — opener's reopening seat.  Partner may be
/// sitting on a trap pass or values short of a free bid, so opener protects.
fn opener_reopening_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let Some((index, opening)) = opening_bid(auction) else {
        return false;
    };
    opening.strain.suit().is_some()
        && auction.len() == index + 4
        && matches!(auction.get(index + 1), Some(&Call::Bid(bid)) if bid.strain.suit().is_some())
        && auction[index + 2] == Call::Pass
}

/// [`opener_reopening_now`] as a hand-ignoring [`Constraint`]
fn opener_reopening() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_reopening_now(context))
}

/// Responder bid a natural 1NT over the overcall (`1X (1Y) 1NT`) and it is back
/// to opener.  A balanced 15-17 would have opened 1NT, so a suit opener's
/// balanced hands are bimodal (12-14 / 18-19); only the top range wants game
/// opposite the 6-10 response.
fn opener_over_free_1nt_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let Some((index, opening)) = opening_bid(auction) else {
        return false;
    };
    opening.strain.suit().is_some()
        && auction.len() == index + 4
        && matches!(auction.get(index + 1), Some(&Call::Bid(bid)) if bid.strain.suit().is_some())
        && auction.get(index + 2) == Some(&Call::Bid(Bid::new(1, Strain::Notrump)))
        && auction[index + 3] == Call::Pass
}

/// [`opener_over_free_1nt_now`] as a hand-ignoring [`Constraint`]
fn opener_over_free_1nt() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_over_free_1nt_now(context))
}

/// Opener reopened with a natural 1NT (`1X (1Y) - - 1NT`, our 18-19 balanced)
/// and it is back to responder — the seat to raise to game with the values that
/// could not make a free bid the first time.
fn responder_over_reopening_1nt_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let Some((index, opening)) = opening_bid(auction) else {
        return false;
    };
    opening.strain.suit().is_some()
        && auction.len() == index + 6
        && matches!(auction.get(index + 1), Some(&Call::Bid(bid)) if bid.strain.suit().is_some())
        && auction[index + 2] == Call::Pass
        && auction[index + 3] == Call::Pass
        && auction.get(index + 4) == Some(&Call::Bid(Bid::new(1, Strain::Notrump)))
        && auction[index + 5] == Call::Pass
}

/// [`responder_over_reopening_1nt_now`] as a hand-ignoring [`Constraint`]
fn responder_over_reopening_1nt() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| responder_over_reopening_1nt_now(context))
}

/// Opener SOS-redoubled (the balancing redouble) and it is back to responder:
/// pick a suit, four-card suits included — opener has none of its own.
fn responder_after_opener_sos_now(context: &Context<'_>) -> bool {
    let Some(anchor) = our_one_nt_runout_seat(context, true) else {
        return false;
    };
    let auction = context.auction();
    auction.len() >= anchor + 6
        && auction.get(anchor + 1) == Some(&Call::Double)
        && auction[anchor + 2] == Call::Pass
        && auction[anchor + 3] == Call::Pass
        && auction[anchor + 4] == Call::Redouble
        && auction[anchor + 5..].iter().all(|&call| call == Call::Pass)
}

/// [`responder_after_opener_sos_now`] as a hand-ignoring [`Constraint`]
fn responder_after_opener_sos() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| responder_after_opener_sos_now(context))
}

/// Responder answered our SOS redouble with a suit; pass it (responder captains
/// the rescue) rather than read it as a natural new suit and raise.
fn opener_after_responder_sos_now(context: &Context<'_>) -> bool {
    let Some(anchor) = our_one_nt_runout_seat(context, false) else {
        return false;
    };
    let auction = context.auction();
    auction.len() >= anchor + 8
        && auction.get(anchor + 1) == Some(&Call::Double)
        && auction[anchor + 2] == Call::Pass
        && auction[anchor + 3] == Call::Pass
        && auction[anchor + 4] == Call::Redouble
        && auction[anchor + 5] == Call::Pass
        && matches!(auction[anchor + 6], Call::Bid(bid) if bid.strain.suit().is_some())
        && auction[anchor + 7..].iter().all(|&call| call == Call::Pass)
}

/// [`opener_after_responder_sos_now`] as a hand-ignoring [`Constraint`]
fn opener_after_responder_sos() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| opener_after_responder_sos_now(context))
}

/// The opponents have escaped our doubled (or redoubled) 1NT and it is our turn
///
/// Our side bid 1NT (opening or overcall), LHO doubled, and since then we have
/// only passed or (re)doubled — never made a contract bid — so the live suit
/// contract is *theirs* (their escape), not a suit of ours.  Returns the index
/// of our 1NT when the pattern holds.  Counting our own doubles as "no contract
/// bid" is what lets the penalty chase recurse as they keep running.
fn our_doubled_one_nt_escape(context: &Context<'_>) -> Option<usize> {
    let auction = context.auction();
    let (index, _) = one_nt_anchor(auction)?;
    // Our side is to act, and bid a 1NT (opening or overcall) that LHO doubled.
    if index % 2 != auction.len() % 2 {
        return None;
    }
    if auction.get(index + 1) != Some(&Call::Double) {
        return None;
    }
    // We made no contract bid since the 1NT: the live suit contract is the
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

/// Their escape is undoubled *and* responder's business redouble (`1NT (X) XX`) has
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

/// The opponents have escaped our `1NT (2NT) X` penalty double and it is our turn
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

/// Partner's takeout double is live: the auction ends `… (bid) X -`
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

/// Partner is advancing a takeout double *we* made — a forced, weak (≈0-8)
/// response, not a constructive suit of their own.
///
/// The mirror of [`advancing_a_double_now`] from the doubler's chair: earlier we
/// (the seat to act) doubled an opponent's suit bid for takeout, and partner has
/// since named a suit — a bid they were forced to make and could hold a bust for.
/// The competitive raise/rebid ladder consults this so a *minimum* doubler does
/// not raise partner's forced advance on its own points (the double already
/// showed them) and drive to a doubled game.  A genuine maximum (17+) still acts.
fn partner_advanced_our_double_now(context: &Context<'_>) -> bool {
    let auction = context.auction();
    let len = auction.len();
    // Our own most recent double (the actor's seat: `(len - i) % 4 == 0`).
    let Some(dbl) = (0..len)
        .rev()
        .find(|&i| (len - i).is_multiple_of(4) && auction[i] == Call::Double)
    else {
        return false;
    };
    // Takeout, not penalty/SOS: the call it doubled — the last non-pass before it
    // — is an opponent's suit bid.
    let takeout = auction[..dbl]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, call)| **call != Call::Pass)
        .is_some_and(|(k, call)| {
            (len - k) % 2 == 1 && matches!(call, Call::Bid(bid) if bid.strain.suit().is_some())
        });
    // Partner named a suit after the double (their forced advance).
    takeout
        && (dbl + 1..len).any(|j| {
            (len - j) % 4 == 2
                && matches!(auction[j], Call::Bid(bid) if bid.strain.suit().is_some())
        })
}

/// [`partner_advanced_our_double_now`] as a hand-ignoring [`Constraint`]
fn partner_advanced_our_double() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| partner_advanced_our_double_now(context))
}

/// The rein (`set_rein_advance_raise`) is silencing a minimum's *second* action
/// over partner's forced advance of our double — a constraint to `&`-negate into
/// the takeout-double rule (the separate 17+ rule keeps the maximum doubling).
fn minimum_reraise_blocked() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| {
        rein_advance_raise_enabled() && partner_advanced_our_double_now(context)
    })
}

/// We already hold an eight-card fit in some suit: our length there plus the
/// minimum partner has shown ([`Inferences`]) reaches eight.
///
/// Reads the shown *minimum* (length partner cannot lack), so it fires only on a
/// fit the calls have promised.  Used by the free-bid gate to stop inventing a
/// new suit once a trump suit is already found.
fn has_fit(hand: Hand, context: &Context<'_>) -> bool {
    let inferences = context.inferences();
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
        // No fit here (`!has_fit`), so raw HCP — not `point_count`, whose
        // distributional upgrade is a ruffing value that only counts opposite a
        // trump fit (the `support_points` rule of thumb, applied to the legacy
        // upgrade too).
        let raw_hcp: u8 = Suit::ASC
            .iter()
            .map(|&suit| holding_hcp::<u8>(hand[suit]))
            .sum();
        level < 4 || !SETTLE_FLOOR.with(Cell::get) || (raw_hcp >= 11 && !has_fit(hand, context))
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

/// The player to act has *personally* bid `suit` — the anchor for rebidding our
/// own long suit in competition (see [`set_competitive_rebid`]).
///
/// Seat-scoped, not side-scoped (`context.we_bid` is the union of both seats):
/// partner's artificial bid — a Jacoby transfer names our short major — must not
/// license a phantom natural "rebid" of a suit we never held.  Our own past
/// turns sit at the indices congruent to the acting seat, `(len - index) % 4 == 0`
/// (the same arithmetic `Context` uses for partner at `== 2`).
fn i_bid_suit(suit: Suit) -> Cons<impl Constraint + Clone> {
    let strain = Strain::from(suit);
    pred(move |_: Hand, context: &Context<'_>| {
        let auction = context.auction();
        let len = auction.len();
        auction.iter().enumerate().any(|(index, &call)| {
            (len - index).is_multiple_of(4)
                && matches!(call, Call::Bid(bid) if bid.strain == strain)
        })
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
/// the `3NT`, so it fires in contested auctions too (`1NT (2♦) … 3NT`).
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

// ---------------------------------------------------------------------------
// M6.4: slam machinery on the floor — RKCB 1430 and control-bid signoffs
// ---------------------------------------------------------------------------

/// The agreed keycard trump: the suit maximizing our actual length plus
/// partner's shown floor, provided the total reaches an eight-card fit and we
/// hold three-plus ourselves — BWS's "an agreed suit makes 4NT keycard",
/// derived instead of installed.  Ties prefer the higher suit.
///
/// This was majors-only until 2026-08-01, on round 4 of the M6.4 A/B: at
/// combined 33 the milestone 6NT power-blast out-scored minor and thin 6-2 suit
/// slams on double-dummy.  Re-priced on the 2026-08 system, **the ask beats the
/// blast** — see [`set_rkcb_minors`], which now carves *back* to majors
/// rather than lifting a carve; a live relocation lifts it too
/// ([`minor_asks_now`]).
fn keycard_trump(hand: Hand, context: &Context<'_>) -> Option<Suit> {
    let inferences = context.inferences();
    let partner = inferences.partner();
    let candidates: &[Suit] = if minor_asks_now() {
        &Suit::ASC
    } else {
        &[Suit::Hearts, Suit::Spades]
    };
    #[allow(clippy::cast_possible_truncation)]
    candidates
        .iter()
        .copied()
        .map(|suit| (hand[suit].len() as u8 + partner.length(suit).min, suit))
        .filter(|&(total, suit)| total >= 8 && hand[suit].len() >= 3)
        .max_by_key(|&(total, suit)| (total, suit as u8))
        .map(|(_, suit)| suit)
}

/// The trump a 4NT at `ask` keys on, read from the auction's face alone —
/// no hand, no readings, so every seat (and the [`forced`] rail) provably
/// lands on the one trump by keying on the same physical ask index
///
/// The dichotomy (jdh8): when 4NT is ambiguous, it is RKCB if the side's
/// last non-cue bid below the ask is a *suit*, quantitative if *notrump*.
/// Two rules, in order:
///
/// 1. **The known fit** — a suit *both* members of the asking side bid
///    below the ask, suits the opponents had named excluded (both members
///    bidding *their* suit is cue territory, not agreement), most recent
///    agreement wins.  Fit precedence is what keeps a cue or control bid
///    on the way to 4NT from masquerading as trumps (`1♠ - 3♠ - 4♣ - 4NT`
///    asks in spades, not clubs).  One carve: an agreed **minor** yields
///    when the side's last non-cue bid is notrump — that 3NT was a
///    *sign-off* re-opening the strain (`1♦ - 3♦ - 3NT - 4NT` is
///    quantitative), where over an agreed **major** the same 3NT is
///    non-serious and the fit stands.  BBA agrees the minor cell is no
///    keycard ask: probed, its own slam move there is 4♣ Gerber (ace
///    steps), and a forced 4NT draws an unconditional 6♦ — but Gerber is
///    rejected for pons (ambiguous when clubs are the strain); the
///    dichotomy already gives minors their RKCB route through a suit-last
///    auction.
/// 2. **The side's last non-cue bid is a suit** — then that suit is the
///    trump: `(2♥) X (3♥) X - 4♠ - 4NT` asks in spades, a completed transfer
///    or Stayman answer asks in the found major, `1♦ - 4NT` asks in
///    diamonds, and a cue of their suit is transparent — `1♥ (3♦) 4♦ -
///    4NT` steps back past the cue and asks in hearts.  A last bid in
///    *notrump* vetoes the rule — that 4NT is quantitative.
///
/// The final rung of [`answer_trump`]'s derivation ladder (the hand-seen
/// fit and the provable-eight readings sit above it and see through
/// transfers, where the face mislabels the artificial call).  `None` (no bid below
/// the ask, a notrump last bid, a bare cue) leaves the 4NT unrecognizable
/// from the face alone.  All four suits qualify — the asker's continuation
/// rungs are built per suit, and 1430 arithmetic is trump-agnostic.
fn face_trump(auction: &[Call], ask: usize) -> Option<Suit> {
    let mut ours = [[false; 2]; 4]; // [suit][member]
    let mut theirs = [false; 4];
    let mut agreed = None;
    let mut last = None; // the side's most recent non-cue bid below the ask
    for (index, call) in auction.iter().enumerate().take(ask) {
        let Call::Bid(bid) = call else { continue };
        if index % 2 != ask % 2 {
            if let Some(suit) = bid.strain.suit() {
                theirs[suit as usize] = true;
            }
            continue;
        }
        let Some(suit) = bid.strain.suit() else {
            last = Some(bid.strain);
            continue;
        };
        if theirs[suit as usize] {
            continue; // a cue is transparent to the face: it never becomes `last`
        }
        last = Some(bid.strain);
        let member = usize::from(index % 4 == ask % 4);
        if ours[suit as usize][1 - member] {
            agreed = Some(suit);
        }
        ours[suit as usize][member] = true;
    }
    agreed
        .filter(|&suit| {
            matches!(suit, Suit::Hearts | Suit::Spades) || last != Some(Strain::Notrump)
        })
        .or_else(|| last?.suit().filter(|&suit| !theirs[suit as usize]))
}

/// The kickback ladder for the side acting at `ask`: for each suit, the trump
/// a four-of-that-suit bid asks keycards in, indexed by `Suit` in ascending
/// order.  Face-only like [`face_trump`] — no hand, no readings — so both
/// members provably build the same table, the same guarantee that lets the
/// 4NT ask be answered at all.
///
/// Three notions, all read off the face below `ask`:
///
/// - **guarded** — a suit either member of our side named naturally, or the
///   opponents named at all.  A guarded suit keeps its natural meaning at the
///   four level (after `1♦ - 1♥ - 3♦`, responder's 4♥ is *hearts*), and their
///   suit there is a cue.  Hearts is guarded by a **spade bid** too, unless the
///   auction disproves five of them: longest first, ties to the higher rank,
///   so 5-5 majors bid spades and the spade bid alone never denies hearts —
///   after `1♦ - 1♠ - 2♦`, 4♥ is plausibly natural and the ladder must not
///   claim it (the collision that made the phase-5 re-measure a wash: the
///   natural walk bids 4♥ *meaning hearts* and both seats sign off in 5♦).
///   A member who named a **second** suit has shown 5+4 = 9 cards and can no
///   longer hold five hearts, so `1♠ - 2♦ - 3♦` keeps its relocation.
/// - **set** — a suit our side named **twice**: both members (a formal raise)
///   or one member twice (`1♦ - 1♥ - 3♦`).  One bid is no agreement, or
///   `1♦ - 4♥` would ask.
/// - the [`face_trump`] **veto** — when the face names no trump at all (the
///   notrump dichotomy: `1♦ - 3♦ - 3NT -` is quantitative), nothing relocates.
///
/// Each set suit claims **four of the next suit up, and nothing else**.  If
/// that one call is guarded or already claimed, the suit does not relocate at
/// all and asks at 4NT, whose meaning is unchanged: kickback *adds* asks, it
/// never removes one, so no auction pons already bids changes.  Two fits can
/// still carry two relocated asks — after `1♣ - 2♣ - 2♥ - 3♥ -`, 4♦ asks in
/// clubs and 4♠ in hearts.
///
/// This is BBA's rule (`docs/ai-bidder/bba-kickback.md` §1.1), adopted
/// deliberately in place of jdh8's earlier **walk-up** ladder, which kept
/// walking to the cheapest unguarded suit above the trump — so 4♠ could ask in
/// diamonds after `1♦ - 1♥ - 3♦`, where BBA reverts to 4NT.  The walk-up is
/// strictly cheaper when both sides read it, and that is the whole problem: a
/// relocated ask two suits above the trump is unrecognisable to anything that
/// has not built the same table, and one seat mistaking it for a natural bid or
/// a cue costs a slam — while the prize for being right is one or two steps of
/// room.  **The saving is always stormed by the misunderstanding** (jdh8,
/// 2026-08-02).  Falling back to 4NT is never ambiguous, because 4NT asked
/// keycards before kickback existed.
///
/// Legality is the caller's business — the table says what a bid *would* mean,
/// not that it is available over the auction so far.
///
/// Stance-scoped since Redwood: the claim loop consults [`RkcbVariant`],
/// claiming minor-trump lanes under either relocation and the hearts lane
/// under [`RkcbVariant::Kickback`] alone.  Still no hand and no readings —
/// the stance is part of the system, set per side by any harness (see
/// [`set_rkcb_variant`]'s two-regimes note) — and scoping *here* means every
/// recognizer and rule downstream inherits one claim table instead of
/// re-deriving the stance.
fn kickback_ladder(auction: &[Call], ask: usize) -> [Option<Suit>; 4] {
    let mut ladder = [None; 4];
    if face_trump(auction, ask).is_none() {
        return ladder;
    }
    let mut ours = [[0u8; 2]; 4]; // our side's natural bids, per suit and member
    let mut theirs = [false; 4];
    for (index, call) in auction.iter().enumerate().take(ask) {
        let Call::Bid(bid) = call else { continue };
        let Some(suit) = bid.strain.suit() else {
            continue;
        };
        if index % 2 != ask % 2 {
            theirs[suit as usize] = true;
        } else if !theirs[suit as usize] {
            // A cue of a suit they have already named shows no length, exactly
            // as it stays transparent to [`face_trump`].
            ours[suit as usize][usize::from(index % 4 == ask % 4)] += 1;
        }
    }
    let bids = |suit: Suit| ours[suit as usize][0] + ours[suit as usize][1];
    // A member whose only named suit is spades can still hold five hearts, so
    // hearts is not disprovable and a later 4♥ stays plausibly natural.  A
    // second named suit is 5+4 = 9 cards and closes the hole.
    let lone_spades = |member: usize| {
        ours[Suit::Spades as usize][member] > 0
            && Suit::ASC
                .into_iter()
                .all(|suit| suit == Suit::Spades || ours[suit as usize][member] == 0)
    };
    let undisprovable_hearts = lone_spades(0) || lone_spades(1);
    let guarded = |suit: Suit| {
        bids(suit) > 0 || theirs[suit as usize] || (suit == Suit::Hearts && undisprovable_hearts)
    };
    for trump in Suit::ASC {
        if bids(trump) < 2 {
            continue;
        }
        // The stance scopes the claim per trump: the full ladder relocates
        // everything, Redwood only the minors, and no stance is a hearts-only
        // ladder.
        let claims = match rkcb_variant_now() {
            RkcbVariant::Kickback => true,
            RkcbVariant::Redwood => trump < Suit::Hearts,
            RkcbVariant::Plain => false,
        };
        if !claims {
            continue;
        }
        // Four of the *next* suit up, or nothing: an occupied rung falls back
        // to 4NT rather than walking on.  Walking was cheaper and unreadable.
        let Some(claim) = Suit::ASC.into_iter().find(|&suit| suit > trump) else {
            continue;
        };
        if !guarded(claim) && ladder[claim as usize].is_none() {
            ladder[claim as usize] = Some(trump);
        }
    }
    ladder
}

/// The trump the 4NT *answerer* counts against: the known fit if our hand
/// sees one, else a still-provable eight (partner's shown floor completing
/// our shorter holding — the 6-2 and 7-1 fits the first rung's three-card
/// bar refuses) or our own self-sufficient seven, else the auction's face
/// ([`face_trump`]: the raised suit, or the side's last bid a natural suit).
/// One hand's shown five alone never synthesizes a fit — an asker keen on
/// our suit asks *over* it, so the face carries that intent, and a shown
/// five the face cannot see was never agreed (the old either-seat shown-five
/// rung let the asker's own suit become trump with no fit evidence, and a
/// DOPI step answer got sat in a 5-2).
///
/// Once the 1430 answer is on the table (the asker decoding at `ask + 4`),
/// the natural walk reads that artificial five-of-a-major as a genuine long
/// suit — partner's heart floor jumps past five and the reading rungs flip
/// to a phantom trump the answerer never counted against, so the asker sits
/// the answer as if it were trumps (`1♦ - 1♥ (1♠) - (3♠) 4♦ - 4NT - 5♥ (X)`
/// passed out on a 4-1 "fit", −20).  The reading rungs therefore accept the
/// *answer's own suit* only when the reading as the **answerer** saw it —
/// the auction through the opponent's call over the ask — already justified
/// it by the same provable-eight-or-own-seven bar the rungs themselves
/// hold.  The prefix is read on a bare context, which under-reads authored
/// calls (a keyed prefix needs the stance, unreachable from a rule); the
/// lanes that lean on them (transfers, Jacoby) recover through the face
/// rung, whose completion or agreement evidence survives truncation.
fn answer_trump(hand: Hand, context: &Context<'_>, ask: usize) -> Option<Suit> {
    let auction = context.auction();
    // A relocated ask carries its own trump on the face ([`kickback_trump`]),
    // so none of the derivation below runs: the ladder already pinned the suit
    // for both members, and re-deriving it from a hand could only disagree.
    if let Some(trump) = kickback_trump(auction, ask) {
        return Some(trump);
    }
    // The in-window answer, present once the auction extends past the
    // answerer's turn (the asker decoding it, or the answerer back on the
    // respect path).
    let ask_bid = keycard_ask_bid(auction, ask);
    let answer = auction.get(ask + 2).and_then(|call| match *call {
        Call::Bid(bid) if answer_step(ask_bid?, bid).is_some() => bid.strain.suit(),
        _ => None,
    });
    let pre = answer.map(|_| Inferences::read(&Context::new(context.vul(), &auction[..ask + 2])));
    // At the prefix's end the *answerer* is to act, so the caller's partner
    // is the prefix's `me` when the asker calls (decoding at `ask + 4`) and
    // its `partner` when the answerer calls back (the respect path at
    // `ask + 6`).  A suit passes if the pre-answer evidence justified it by
    // the same bar the derivation rungs hold (the provable eight, or our own
    // self-sufficient seven, which needs no reading at all).
    let corroborated = |suit: Suit| {
        answer != Some(suit)
            || hand[suit].len() >= 7
            || pre.as_ref().is_some_and(|pre| {
                let partner = if (auction.len() - ask).is_multiple_of(4) {
                    pre.me()
                } else {
                    pre.partner()
                };
                hand[suit].len() + usize::from(partner.length(suit).min) >= 8
            })
    };
    keycard_trump(hand, context)
        .filter(|&suit| corroborated(suit))
        .or_else(|| {
            let inferences = context.inferences();
            let total =
                |suit: Suit| hand[suit].len() + usize::from(inferences.partner().length(suit).min);
            [Suit::Hearts, Suit::Spades]
                .into_iter()
                .filter(|&suit| (total(suit) >= 8 || hand[suit].len() >= 7) && corroborated(suit))
                .max_by_key(|&suit| (total(suit), suit as u8))
        })
        .or_else(|| face_trump(context.auction(), ask))
}

/// The opponents' calls since `index` (exclusive) are all passes or doubles
/// — the keycard-window discipline shared by the whole M6.4 machinery and
/// the [`forced`] rail predicate, so the two can never disagree on when a
/// window is live.  A contested auction *before* the 4NT is no obstacle
/// (4NT in competition with an agreed suit is keycard, not quantitative),
/// and their double of the ask or of an answer changes nothing the 1430
/// arithmetic depends on; but their *bid* inside the window takes the
/// answer rooms away, so the machinery stands down and judgement resumes.
///
/// The anchors are always an even distance from the end (partner's ask at
/// `n − 2`, our ask at `n − 4`, partner's ask at `n − 6`), so the
/// opponents' calls are exactly the odd-from-end slots the reverse walk
/// visits.
fn opponents_quiet_since(auction: &[Call], index: usize) -> bool {
    auction[index + 1..]
        .iter()
        .rev()
        .step_by(2)
        .all(|call| matches!(call, Call::Pass | Call::Double))
}

/// Partner's off-book ask — 4NT, or the kickback ladder's relocated call
/// ([`keycard_ask_bid`]) — asks keycards: a quiet window (opponents at most
/// doubling since the ask), not an opening, not over our own notrump bid
/// (that 4NT is quantitative — BWS reads keycard only with an agreed suit),
/// and a trump derivable ([`answer_trump`], whose final rung reads the
/// auction's face).  Returns the agreed trump the 1430 answer counts against
/// and the asking bid its steps are measured from.
fn keycard_asked(hand: Hand, context: &Context<'_>) -> Option<(Suit, Bid)> {
    let ask = keycard_asked_face(context)?;
    Some((
        answer_trump(hand, context, context.auction().len() - 2)?,
        ask,
    ))
}

/// The face half of [`keycard_asked`]: every gate that reads the auction alone,
/// no hand and no reading.  The [`Rules::face`] gate the kickback arm attaches
/// to its answer rules, so bidder (`Rule::eval` consults the gate) and reader
/// (the inference consult sites skip face-dead rules) share one predicate and
/// cannot drift — the phase-5 fix for the §7.3.1 union poison.
fn keycard_asked_face(context: &Context<'_>) -> Option<Bid> {
    if !floor_rkcb_now() {
        return None;
    }
    let auction = context.auction();
    let n = auction.len();
    let ask = keycard_ask_bid(auction, n.checked_sub(2)?)?;
    if !opponents_quiet_since(auction, n - 2) {
        return None;
    }
    let (opening_index, _) = opening_bid(auction)?;
    if opening_index == n - 2 {
        return None;
    }
    if n >= 4 && matches!(auction[n - 4], Call::Bid(bid) if bid.strain == Strain::Notrump) {
        return None;
    }
    Some(ask)
}

/// The 1430 answer that lands on `bid`: the rung `bid` occupies above the ask
/// ([`answer_step`]) matches our keycard count, and the queen-splitting rungs
/// 3 and 4 match our trump-queen holding
///
/// One function for the whole ladder, because the rungs are the *steps* — over
/// a 4NT ask those steps are 5♣/5♦/5♥/5♠, exactly the four rules the floor has
/// always carried, so kickback-off behaviour is unchanged.  Step 1 also covers
/// all five keycards (a 2♣ rock answering its raiser's ask; the book ladder's
/// `{1,4}` left that hand with *no* answer and round 3 passed the ask out).
fn keycard_answer(bid: Bid) -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    described(
        "the matching 1430 keycard answer",
        move |hand: Hand, context: &Context<'_>| {
            let Some((trump, ask)) = keycard_asked(hand, context) else {
                return false;
            };
            let keycards = count_keycards(hand, trump);
            match answer_step(ask, bid) {
                Some(1) => [1, 4, 5].contains(&keycards),
                Some(2) => [0, 3].contains(&keycards),
                Some(3) => keycards == 2 && !holds_queen(hand, context, trump),
                Some(4) => keycards == 2 && holds_queen(hand, context, trump),
                _ => false,
            }
        },
    )
}

/// The next bid up the auction's ladder — the "cheapest step" the DOPI
/// answers count in
fn bid_successor(bid: Bid) -> Option<Bid> {
    let level = bid.level.get();
    Some(match bid.strain {
        Strain::Clubs => Bid::new(level, Strain::Diamonds),
        Strain::Diamonds => Bid::new(level, Strain::Hearts),
        Strain::Hearts => Bid::new(level, Strain::Spades),
        Strain::Spades => Bid::new(level, Strain::Notrump),
        Strain::Notrump => {
            if level >= 7 {
                return None;
            }
            Bid::new(level + 1, Strain::Clubs)
        }
    })
}

/// The trump a relocated four-of-a-suit keycard ask at `ask` pins, if a
/// relocation is live ([`RkcbVariant`]) and [`kickback_ladder`] claims that
/// suit
///
/// Face-only by construction: the ladder is what made the call an ask, so the
/// ask's own trump comes from the ladder and never from [`answer_trump`]'s
/// hand-and-reading derivation.  A hand-derived trump for a hand-derived ask
/// site is a phantom-trump generator — both members must land on one suit or
/// the answer counts against a different one than the ask meant.
///
/// The emission gate is `context.undisturbed()`, so the recognizer holds the
/// same bar: a four-level suit bid in a contested auction is a cue or a
/// contract, and reading it as an ask would deal exactly the phantom the
/// alert exists to prevent.  (4NT's own recognizer is looser — a contested 4NT
/// with an agreed suit is still keycard — but it has no natural meaning to
/// lose.)
fn kickback_trump(auction: &[Call], ask: usize) -> Option<Suit> {
    if !relocating_now() {
        return None;
    }
    let Some(&Call::Bid(bid)) = auction.get(ask) else {
        return None;
    };
    if bid.level.get() != 4 {
        return None;
    }
    if auction
        .iter()
        .take(ask)
        .enumerate()
        .any(|(index, call)| index % 2 != ask % 2 && *call != Call::Pass)
    {
        return None;
    }
    kickback_ladder(auction, ask)[bid.strain.suit()? as usize]
}

/// The keycard ask made at `ask`, if any: 4NT always, plus the kickback
/// ladder's relocated four-of-a-suit call.  The whole M6.4 machinery anchors on
/// this one predicate so its five ask positions can never disagree.
///
/// One exception, and it is the whole reason a relocated ask needs its own
/// recognizer: **an answer is an answer, never a re-ask** — and neither is any
/// later rung.  [`conversation_rung`] owns that judgement whole, so the ask
/// recognizer and the conversation walker cannot disagree about whether RKCB is
/// already talking.
///
/// It matters because the relocated ladders **overlap**.  With diamonds agreed
/// 4♥ asks and 4♠ is its step 1 — but 4♠ is itself the ask bid one lane over,
/// so without the guard the *asker* sees a live ask on partner's answer and
/// answers its own question: the 1.9-weighted answer rung outbids its own 1.82
/// signoff and the auction walks into a phantom suit (`1♦ - 3♦ - 4♥ - 4♠ - 5♣`
/// doubled, singleton ♣A opposite ♣987, −1100).  The same shape one rung higher
/// is 4NT over a 4♠ ask, §7.4's −15 IMP smoke-run failure.  A plain 4NT ask can
/// never collide this way: all four of its rungs are five-level, which is why
/// the kickback-off system is untouched by any of this.
fn keycard_ask_bid(auction: &[Call], ask: usize) -> Option<Bid> {
    let &Call::Bid(bid) = auction.get(ask)? else {
        return None;
    };
    // An answer is an answer, never a re-ask — and neither is any later rung.
    // [`conversation_rung`] owns that judgement whole.
    if conversation_rung(auction, ask) {
        return None;
    }
    if kickback_trump(auction, ask).is_some() {
        return Some(bid);
    }
    (bid == Bid::new(4, Strain::Notrump)).then_some(bid)
}

/// Whether the kickback ladder was offering a relocated ask **in `trump`** at
/// `index`
///
/// The companion to [`keycard_ask_at`] for measurement.  An ask that came out
/// as 4NT while the ladder was offering a relocation *in that very trump* is a
/// position where the convention was available and something else took the
/// seat — the authored book, which installs RKCB at absolute 4NT and has not
/// been relocated.  That difference is the size of the prize for relocating the
/// book; without it a 4NT ask is indistinguishable from a lane where the ladder
/// simply claims nothing, which is the common and entirely correct case.
///
/// Keyed to the ask's own trump on purpose.  Asking merely whether the ladder
/// claimed *something* conflates the shadowed lanes with the ones where 4NT is
/// already right: a spade ask belongs at 4NT, so a spade-trump 4NT alongside a
/// club claim elsewhere on the face is not a missed relocation at all.  The
/// same conflation — bucketing by a label that does not identify the lane — is
/// what invalidated the per-trump-by-contract-strain cut (§7.9).
///
/// **Reads the knob**, like [`keycard_ask_at`].
#[doc(hidden)]
#[must_use]
pub fn kickback_offered_at(auction: &[Call], index: usize, trump: Suit) -> bool {
    relocating_now() && kickback_ladder(auction, index)[trump as usize].is_some()
}

/// The keycard ask made at `index`, if the call there is one: the asking bid
/// and the trump it asks in, plus whether the ask was **relocated** onto the
/// kickback ladder rather than made at 4NT
///
/// For measurement harnesses.  An A/B that buckets its divergent boards by the
/// strain of the *final contract* conflates two different things — the lane a
/// keycard ask was made in, and where the auction happened to land — and it
/// cannot see the boards where no ask was made at all.  That last bucket is
/// the one that matters when the floor's weights move with the knob: it is the
/// residue attributable to the net alone (`docs/ai-bidder/bba-kickback.md`
/// §7.8).
///
/// **Reads the knob**, through [`kickback_trump`]: call it with the same arm
/// armed that produced the auction, or a relocated ask reads as no ask.
#[doc(hidden)]
#[must_use]
pub fn keycard_ask_at(auction: &[Call], index: usize) -> Option<(Bid, Suit, bool)> {
    let ask = keycard_ask_bid(auction, index)?;
    match kickback_trump(auction, index) {
        Some(trump) => Some((ask, trump, true)),
        None => Some((ask, face_trump(auction, index)?, false)),
    }
}

/// Which 1430 rung `answer` sits on above `ask` — steps 1..=4 up the auction's
/// own ladder, [`bid_successor`] applied that many times
///
/// Over a 4NT ask the steps *are* the absolute rungs the floor has always used
/// (1 = 5♣, 2 = 5♦, 3 = 5♥, 4 = 5♠), so making the machinery step-relative
/// leaves the kickback-off system byte-identical.
fn answer_step(ask: Bid, answer: Bid) -> Option<usize> {
    let mut rung = ask;
    for step in 1..=4 {
        rung = bid_successor(rung)?;
        if rung == answer {
            return Some(step);
        }
    }
    None
}

/// What partner's single reply to the queen ask can say
///
/// The merged answer (`docs/ai-bidder/bba-kickback.md` §7.6): one round carries
/// the queen *and* the kings, because the space between the ask and six of
/// trump is exactly big enough to hold both.
///
/// - `weak` — five of trump: no queen, and not worth six anyway.  Always
///   present, because the ask has to sit strictly below it: the answerer reads
///   the *face*, so an ask that landed on five of trump would be indis-
///   tinguishable from the signoff and partner would raise a signoff to six.
/// - `deny` — six of trump: no queen.  With `weak` present it means the
///   stronger half, "no queen but bid it anyway" (the ninth trump, a void).
/// - `kings` — the three side suits, cheapest first: the queen, plus the king
///   of that suit.  **Skipping a step denies that king**, so the reply reads
///   "my cheapest king is this one and I hold none below it" — BBA's ladder,
///   and worth strictly more than naming one king out of a count.
/// - `no_king` — 5NT: the queen, and no side king at all.
#[derive(Clone, Copy)]
pub(in crate::bidding) struct RelayMap {
    /// The queen ask itself
    pub ask: Bid,
    /// Five of trump — no queen, no buff, and the contract
    pub weak: Bid,
    /// Six of trump — no queen
    pub deny: Bid,
    /// The three side suits, cheapest first, with the call showing that king
    pub kings: [(Suit, Bid); 3],
    /// 5NT — the queen, no side king
    pub no_king: Bid,
}

/// The queen ask over `answer`, when the merged reply fits under six of trump
///
/// The ask is one step up the auction's own ladder, and it exists exactly when
/// that step still lands **below** five of trump — 11 of the 16 ask/answer
/// lanes, every one of them a relocated ask plus the two plain-4NT spade lanes
/// and hearts after a one-or-four.  Room is the binding constraint, which is
/// the whole argument for relocating the ask in the first place.
pub(in crate::bidding) fn queen_ask_room(answer: Bid, trump: Suit) -> Option<Bid> {
    Some(relay_map(answer, trump)?.ask)
}

/// Assign every message of the merged reply to a call, or fail
///
/// Fails when a side suit's cheapest bid above the ask climbs past six of
/// trump — the cramped plain-4NT minor lanes, which the relay has never
/// served.  Every assignment is checked for collision, because that is where
/// both earlier cuts of this ladder broke.
pub(in crate::bidding) fn relay_map(answer: Bid, trump: Suit) -> Option<RelayMap> {
    let ask = bid_successor(answer)?;
    let strain = Strain::from(trump);
    let (five, six) = (Bid::new(5, strain), Bid::new(6, strain));
    let mut taken = Vec::with_capacity(6);
    let mut take = |call: Bid| -> Option<Bid> {
        (call > ask && call <= six && !taken.contains(&call)).then(|| {
            taken.push(call);
            call
        })
    };
    // Strictly below the signoff, or the answerer cannot tell the ask from it.
    let weak = take(five)?;
    let deny = take(six)?;
    let mut kings = [(trump, six); 3];
    for (slot, side) in kings
        .iter_mut()
        .zip(Suit::ASC.into_iter().filter(|&suit| suit != trump))
    {
        let call = (5..=6)
            .map(|level| Bid::new(level, Strain::from(side)))
            .find(|&call| call > ask)?;
        *slot = (side, take(call)?);
    }
    kings.sort_by_key(|&(_, call)| call);
    let no_king = take(Bid::new(5, Strain::Notrump))?;
    Some(RelayMap {
        ask,
        weak,
        deny,
        kings,
        no_king,
    })
}

/// What partner's merged reply said
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::bidding) enum Reply {
    /// Five of trump — no queen, and nothing to make up for it
    Weak,
    /// Six of trump — no queen, but a fit or a void partner cannot see
    Buff,
    /// The queen, plus the king of [`RelayMap::kings`]`[i]`, and no side king
    /// on a cheaper rung
    King(usize),
    /// The queen, and no side king at all
    NoKing,
}

/// Decode partner's reply to the queen ask
pub(in crate::bidding) fn read_reply(map: &RelayMap, call: Call) -> Option<Reply> {
    let Call::Bid(bid) = call else {
        return None;
    };
    if bid == map.weak {
        return Some(Reply::Weak);
    }
    if bid == map.deny {
        return Some(Reply::Buff);
    }
    if bid == map.no_king {
        return Some(Reply::NoKing);
    }
    map.kings
        .iter()
        .position(|&(_, rung)| rung == bid)
        .map(Reply::King)
}

/// The second relay: over a king-showing reply, ask whether partner holds one
/// of the kings *dearer* than the one already named
///
/// Two rungs are all the grand gate needs — it counts kings, it does not name
/// them — so the answer is "one more" on the cheap step and six of trump
/// otherwise, which is a contract rather than a code.  The ask is the plain
/// successor of the reply, so **kickback lowers this ask exactly as it lowers
/// the first one**: relocating the keycard ask buys room twice, once for the
/// queen and once again here.
#[derive(Clone, Copy)]
pub(in crate::bidding) struct KingRelay {
    /// The relay itself
    pub ask: Bid,
    /// At least one more side king, dearer than the one already shown
    pub more: Bid,
    /// Six of trump — no more kings, and the contract with it
    pub none: Bid,
}

/// The second relay over `reply`, when both its rungs fit under six of trump
pub(in crate::bidding) fn king_relay(reply: Bid, trump: Suit) -> Option<KingRelay> {
    let none = Bid::new(6, Strain::from(trump));
    let ask = bid_successor(reply)?;
    let more = bid_successor(ask)?;
    (more < none).then_some(KingRelay { ask, more, none })
}

/// A call that could anchor a keycard conversation on its face alone: 4NT, or
/// a kickback-relocated four-of-a-suit bid
///
/// Deliberately looser than [`keycard_ask_bid`] — it exists to *veto* reading a
/// relay rung as a fresh ask, so over-matching costs nothing while
/// under-matching would let the phantom through.
fn plausible_ask(auction: &[Call], index: usize) -> Option<Bid> {
    let &Call::Bid(bid) = auction.get(index)? else {
        return None;
    };
    (bid == Bid::new(4, Strain::Notrump) || kickback_trump(auction, index).is_some()).then_some(bid)
}

/// The call at `index` belongs to a keycard conversation already in motion —
/// the 1430 answer itself, or a queen ask, queen reply, king ask or king reply
/// above it — and so is **never an ask of its own**
///
/// The single source of truth for "is RKCB already talking here".
/// [`keycard_ask_bid`] consults it before reading any call as a fresh ask, and
/// nothing else decides the question, so the two halves cannot drift apart.
///
/// Needed because every rung lands on a call something else wants to read as an
/// ask, a cue or a contract — and worse, because the relocated ladders
/// **overlap**: a diamond ask at 4♥ is answered on 4♠, which is itself an ask
/// bid.  Reading a rung as a fresh ask sends the partnership off counting
/// keycards in a phantom suit, §7.4's −15 IMP failure and the −2.50 IMPs/board
/// the club lane bled before the answer arm below existed.
///
/// Two arms, and they are gated differently on purpose:
///
/// - **The answer**, two calls back.  Always live: the 1430 answer exists
///   whenever the ask does, with or without the relay.  Positional and
///   trump-free — the rungs are mechanical, [`answer_step`] applied to partner's
///   ask, and carry no suit of their own; only the queen and king asks *above*
///   them take our trump into account.  It does have to be one of those four
///   rungs: vetoing every call after an ask, rather than the ladder's own, turns
///   off answer windows that are legitimately live and shifts the readings that
///   depend on them (four suite failures, 2026-08-02).
/// - **The relay's rungs**, four or more calls back.  Deliberately coarse: anything above the answer
///   and at or below six of spades, the highest six of trump the relay reaches.
///   It cannot be sharper — the trump is not derivable from the face alone —
///   and need not be, since over-matching only refuses to read a call as a
///   *fresh* ask, which nothing up there wanted anyway.
///
/// Face-only and non-recursive: each arm walks *forward* from a candidate
/// anchor and consults [`plausible_ask`] rather than [`keycard_ask_bid`].
fn conversation_rung(auction: &[Call], index: usize) -> bool {
    let Some(&Call::Bid(made)) = auction.get(index) else {
        return false;
    };
    let answers_partners_ask = index
        .checked_sub(2)
        .and_then(|anchor| plausible_ask(auction, anchor))
        .is_some_and(|ask| answer_step(ask, made).is_some());
    if answers_partners_ask {
        return true;
    }
    [4usize, 6, 8, 10].into_iter().any(|back| {
        index
            .checked_sub(back)
            .and_then(|anchor| {
                let ask = plausible_ask(auction, anchor)?;
                opponents_quiet_since(auction, anchor).then_some(())?;
                let Call::Bid(answer) = *auction.get(anchor + 2)? else {
                    return None;
                };
                answer_step(ask, answer)?;
                Some(made > answer && made <= Bid::new(6, Strain::Spades))
            })
            .unwrap_or(false)
    })
}

/// The seven calls a 1430 answer can land on, over any ask the ladder can
/// relocate to (4♦ → 4♥…5♣, 4♥ → 4♠…5♦, 4♠ → 4NT…5♥, 4NT → 5♣…5♠)
const KICKBACK_ANSWERS: [Bid; 7] = [
    Bid::new(4, Strain::Hearts),
    Bid::new(4, Strain::Spades),
    Bid::new(4, Strain::Notrump),
    Bid::new(5, Strain::Clubs),
    Bid::new(5, Strain::Diamonds),
    Bid::new(5, Strain::Hearts),
    Bid::new(5, Strain::Spades),
];

/// Every call a relay rung can land on, over any ask the ladder can relocate to
///
/// The union of [`relay_map`] over the answers that leave a relay room
/// ([`queen_ask_room`] rejects 5♥ and 5♠ answers outright — their no-queen rung
/// would already be past five of trump).  Each rule's own constraint rejects
/// the landings that are not its rung, exactly as the 1430 answers do, and
/// [`relay_lanes`] narrows each rule *class* to the rungs it can reach.
const RELAY_RUNGS: [Bid; 11] = [
    Bid::new(4, Strain::Spades),
    Bid::new(4, Strain::Notrump),
    Bid::new(5, Strain::Clubs),
    Bid::new(5, Strain::Diamonds),
    Bid::new(5, Strain::Hearts),
    Bid::new(5, Strain::Spades),
    Bid::new(5, Strain::Notrump),
    Bid::new(6, Strain::Clubs),
    Bid::new(6, Strain::Diamonds),
    Bid::new(6, Strain::Hearts),
    Bid::new(6, Strain::Spades),
];

/// The four calls a 1430 answer to a plain 4NT ask can land on — the
/// kickback-off answer set, and exactly the rules the floor has always carried
const PLAIN_ANSWERS: [Bid; 4] = [
    Bid::new(5, Strain::Clubs),
    Bid::new(5, Strain::Diamonds),
    Bid::new(5, Strain::Hearts),
    Bid::new(5, Strain::Spades),
];

/// Every relay lane the shared geometry can build — each possible 1430 answer
/// bid crossed with each trump — as `(trump, map)`
///
/// The install-time source of truth for which [`RELAY_RUNGS`] a rule class can
/// actually land on.  An alerted rule installed on a rung no lane reaches is
/// constraint-dead but face-live, and the reading's structural `alerted` bit
/// would erase the natural walk for five and six of trump at every in-window
/// placement seat — the §7.3.1 union poison, fired from *inside* the window.
/// Each rule class below therefore installs only where some lane can put it.
fn relay_lanes() -> impl Iterator<Item = (Suit, RelayMap)> {
    KICKBACK_ANSWERS.into_iter().flat_map(|answer| {
        Suit::ASC
            .into_iter()
            .filter_map(move |trump| relay_map(answer, trump).map(|map| (trump, map)))
    })
}

/// Some lane's queen ask lands on `bid`
fn queen_ask_can_land(bid: Bid) -> bool {
    relay_lanes().any(|(_, map)| map.ask == bid)
}

/// Some lane's artificial queen reply — a king rung or 5NT — lands on `bid`
fn artificial_reply_can_land(bid: Bid) -> bool {
    relay_lanes()
        .any(|(_, map)| map.no_king == bid || map.kings.iter().any(|&(_, rung)| rung == bid))
}

/// Some lane's natural queen denial — five or six of the agreed trump — lands
/// on `bid`
fn denial_can_land(bid: Bid) -> bool {
    relay_lanes().any(|(_, map)| map.weak == bid || map.deny == bid)
}

/// Some lane's second relay (the king ask above a king-showing reply) lands on
/// `bid`
fn king_ask_can_land(bid: Bid) -> bool {
    relay_lanes().any(|(trump, map)| {
        map.kings
            .iter()
            .any(|&(_, rung)| king_relay(rung, trump).is_some_and(|relay| relay.ask == bid))
    })
}

/// Some lane's second-relay reply lands on `bid`; `more` picks the rung
fn king_reply_can_land(bid: Bid, more: bool) -> bool {
    relay_lanes().any(|(trump, map)| {
        map.kings.iter().any(|&(_, rung)| {
            king_relay(rung, trump)
                .is_some_and(|relay| bid == if more { relay.more } else { relay.none })
        })
    })
}

/// Partner's off-book ask was overcalled by RHO — the DOPI/DEPO window.
/// Same recognizability gates as [`keycard_asked`]; returns the trump and
/// their bid.  (Their *double* of the ask keeps the quiet window alive and
/// is answered by the ROPI rungs through [`keycard_asked`] itself.)
fn keycard_asked_over_bid(hand: Hand, context: &Context<'_>) -> Option<(Suit, Bid)> {
    let their = keycard_asked_over_bid_face(context)?;
    let trump = answer_trump(hand, context, context.auction().len() - 2)?;
    Some((trump, their))
}

/// The face gate confining the always-present 1430 answer rules to a live ask
/// window
///
/// The 1430 answers (5♣–5♠ over a plain 4NT) and the ROPI/DOPI/DEPO rules on
/// X/XX/Pass are installed in **every** stance and every one is alerted, so
/// without this an ordinary 5♦ with no ask anywhere on the auction reads as a
/// keycard answer: the union over rules sharing the call picks up the answer
/// rule's ⊤ projection and collapses partner's diamond floor, and the
/// structural `alerted` bit suppresses the natural walk on top.
///
/// Was a knob (`set_keycard_answer_gates`) and is now unconditional.  It
/// measured **zero divergent boards** over 1M × 2 vulnerabilities when it
/// shipped, and three boards in 200k when re-checked on 2026-08-02: the
/// off-position never changed a contract, it only broke readings.  A knob whose
/// off-position is a bug rather than an agreement is a foot-gun, and this one
/// cost an invalid A/B arm pairing before it was retired.
fn answer_window_face(context: &Context<'_>) -> bool {
    keycard_asked_face(context).is_some()
}

/// [`answer_window_face`] for the ROPI rungs — their double of the ask
fn ropi_window_face(context: &Context<'_>) -> bool {
    context.auction().last() == Some(&Call::Double) && keycard_asked_face(context).is_some()
}

/// [`answer_window_face`] for the DOPI/DEPO rungs — their bid over the ask
fn dopi_window_face(context: &Context<'_>) -> bool {
    keycard_asked_over_bid_face(context).is_some()
}

/// The face half of [`keycard_asked_over_bid`] — see [`keycard_asked_face`]
fn keycard_asked_over_bid_face(context: &Context<'_>) -> Option<Bid> {
    if !floor_rkcb_now() {
        return None;
    }
    let auction = context.auction();
    let n = auction.len();
    keycard_ask_bid(auction, n.checked_sub(2)?)?;
    let Call::Bid(their) = auction[n - 1] else {
        return None;
    };
    let (opening_index, _) = opening_bid(auction)?;
    if opening_index == n - 2 {
        return None;
    }
    if n >= 4 && matches!(auction[n - 4], Call::Bid(bid) if bid.strain == Strain::Notrump) {
        return None;
    }
    Some(their)
}

/// The ROPI answer over their double of partner's ask — classic R0P1:
/// redouble 0, pass 1, the cheapest bid (step 1) 2, each with the 1430-style
/// wraparound the asker resolves arithmetically.  The queen dimension is
/// traded away, the classic price of the convention.
fn ropi_answer(counts: &'static [usize]) -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    described(
        "the ROPI answer over their double of the ask",
        move |hand: Hand, context: &Context<'_>| {
            context.auction().last() == Some(&Call::Double)
                && keycard_asked(hand, context)
                    .is_some_and(|(trump, _)| counts.contains(&count_keycards(hand, trump)))
        },
    )
}

/// The ROPI two-keycard step: the cheapest bid over their double of the ask —
/// step 1 up the ladder, `5♣` over a plain 4NT
fn ropi_step(bid: Bid) -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    described(
        "the ROPI step answer over their double of the ask",
        move |hand: Hand, context: &Context<'_>| {
            context.auction().last() == Some(&Call::Double)
                && keycard_asked(hand, context).is_some_and(|(trump, ask)| {
                    answer_step(ask, bid) == Some(1)
                        && [2, 5].contains(&count_keycards(hand, trump))
                })
        },
    )
}

/// The DOPI answer over their bid below five of trump — classic D0P1:
/// double 0, pass 1 (both with the wraparound), the cheapest step 2
fn dopi_answer(counts: &'static [usize]) -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    described(
        "the DOPI answer over their bid below five of trump",
        move |hand: Hand, context: &Context<'_>| {
            keycard_asked_over_bid(hand, context).is_some_and(|(trump, their)| {
                their < Bid::new(5, Strain::from(trump))
                    && counts.contains(&count_keycards(hand, trump))
            })
        },
    )
}

/// The DOPI two-keycard step: the cheapest bid over their interference is
/// exactly `bid`
fn dopi_step(bid: Bid) -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    described(
        "the DOPI step answer over their bid below five of trump",
        move |hand: Hand, context: &Context<'_>| {
            keycard_asked_over_bid(hand, context).is_some_and(|(trump, their)| {
                their < Bid::new(5, Strain::from(trump))
                    && bid_successor(their) == Some(bid)
                    && [2, 5].contains(&count_keycards(hand, trump))
            })
        },
    )
}

/// The DEPO answer over their bid at or above five of trump — no room for
/// steps, so parity alone: double even, pass odd
fn depo_answer(even: bool) -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    described(
        "the DEPO answer over their bid at or above five of trump",
        move |hand: Hand, context: &Context<'_>| {
            keycard_asked_over_bid(hand, context).is_some_and(|(trump, their)| {
                their >= Bid::new(5, Strain::from(trump))
                    && count_keycards(hand, trump).is_multiple_of(2) == even
            })
        },
    )
}

/// We asked two calls ago — 4NT, or the ladder's relocated call — and partner
/// answered: decode the answer, returning the trump and the partnership's
/// combined keycard count
///
/// Their call over the *ask* picks the answering scheme — quiet keeps the
/// 1430 ladder, their double answers in ROPI, their bid in DOPI (below five
/// of trump) or DEPO (at or above).  Their bid over the *answer* stands the
/// machinery down and judgement resumes.
///
/// The ambiguous answers resolve arithmetically, and the arithmetic is
/// exact by doctrine (jdh8): a partnership that cannot assume **three
/// combined keycards should not be seeking slam at all** — the ask's
/// `combined_points(29)` floor is what buys the assumption — and inside
/// combined 3..=5 the two readings of any step differ by three, so at most
/// one fits the window.  A high reading past five means the low one; a low
/// reading under three means the high one; both at once is impossible.
/// The round-1 ask-gate A/B measured the alternative: sub-29 asks whose
/// decode had to guess, driving six off two keycards on every high guess.
fn keycard_answered(hand: Hand, context: &Context<'_>) -> Option<(Suit, usize)> {
    use super::american::slam::count_keycards;
    if !floor_rkcb_now() {
        return None;
    }
    let auction = context.auction();
    let n = auction.len();
    let ask = keycard_ask_bid(auction, n.checked_sub(4)?)?;
    if matches!(auction[n - 1], Call::Bid(_)) {
        return None;
    }
    // The same derivation ladder as the answerer's, its face rung keyed on
    // the same physical ask index — sharing the function is the guarantee
    // both seats land on the one trump.  The ladder's pre-answer discipline
    // (see [`answer_trump`]) keeps that guarantee across time: the answer we
    // are decoding must not mint the trump it is counted against.
    let trump = answer_trump(hand, context, n - 4)?;
    let mine = count_keycards(hand, trump);
    let answer = auction[n - 2];
    let (low, high) = match auction[n - 3] {
        interference @ (Call::Pass | Call::Double) => {
            let doubled = interference == Call::Double;
            match answer {
                // ROPI over their double of the ask: redouble 0, pass 1,
                // the cheapest bid 2 — each with the 1430-style wraparound.
                Call::Redouble if doubled => (0, 3),
                Call::Pass if doubled => (1, 4),
                Call::Bid(bid) if doubled && answer_step(ask, bid) == Some(1) => (2, 5),
                // The 1430 ladder, counted in *steps above the ask* (an
                // off-scheme emission — the net's — decodes on the same
                // table).  Over a plain 4NT the steps are 5♣/5♦/5♥/5♠, the
                // absolute rungs this table has always held.
                Call::Bid(bid) => answer_band(answer_step(ask, bid)?),
                _ => return None,
            }
        }
        // DOPI below five of trump: double 0, pass 1, the cheapest step 2.
        Call::Bid(their) if their < Bid::new(5, Strain::from(trump)) => match answer {
            Call::Double => (0, 3),
            Call::Pass => (1, 4),
            Call::Bid(bid) if bid_successor(their) == Some(bid) => (2, 5),
            _ => return None,
        },
        // DEPO at or above: parity alone — the largest count our own hand
        // leaves possible.
        Call::Bid(_) => {
            let parities = match answer {
                Call::Double => [4, 2, 0],
                Call::Pass => [5, 3, 1],
                _ => return None,
            };
            let partners = parities.into_iter().find(|p| mine + p <= 5)?;
            return Some((trump, mine + partners));
        }
        Call::Redouble => return None,
    };
    Some((trump, resolve_total(mine, low, high)))
}

/// The two counts a 1430 rung leaves open — step 1 is "one or four", step 2
/// "none or three", and the two-keycard rungs are exact
fn answer_band(step: usize) -> (usize, usize) {
    match step {
        1 => (1, 4),
        2 => (0, 3),
        _ => (2, 2),
    }
}

/// The combined count `mine` and an ambiguous `(low, high)` answer resolve to
///
/// Exact by doctrine ([`keycard_answered`]): inside combined 3..=5 the two
/// readings differ by three, so a high reading past five means the low one was
/// meant.  Shared by every position that decodes an answer, so the relay's
/// deeper rungs can never drift from the direct one.
fn resolve_total(mine: usize, low: usize, high: usize) -> usize {
    mine + if mine + high > 5 { low } else { high }
}

/// A long enough proven trump fit stands in for the trump queen
///
/// Counted as our own length plus the *sound floor* of partner's shown length,
/// the same bound [`known_eight_card_fit`] uses, so neither seat can claim a fit
/// the auction has not shown.  The threshold is [`QUEEN_FIT`].
///
/// Why a threshold and not a constant: seeing four keycards already puts one
/// loser on the table, so the slam turns on there not being a *second* one in
/// trumps, and how likely that is depends entirely on the fit.  Measured over
/// random deals at four keycards (`probe-trump-queen`), the queen is worth
/// **+13.9pp** to a six with an eight-card fit, **+7.6pp** at nine and
/// **+3.5pp** at ten: an eight-card fit needs the honour to have a finesse to
/// take, a ten-card fit draws trumps in two rounds without it, and nine is in
/// between — which is exactly where a sweep, not a constant, belongs.
fn long_fit_for_queen(hand: Hand, context: &Context<'_>, trump: Suit) -> bool {
    let shown = hand[trump].len() + usize::from(context.inferences().partner().length(trump).min);
    shown >= usize::from(QUEEN_FIT)
}

/// We hold the trump queen, or the side has shown the ten-card fit that stands
/// in for it — the answerer's queen bit, and the asker's when it can settle the
/// question from its own hand
fn holds_queen(hand: Hand, context: &Context<'_>, trump: Suit) -> bool {
    hand[trump].contains(Rank::Q) || long_fit_for_queen(hand, context, trump)
}

/// The keycard conversation whose ask sits `back` calls behind the end, with
/// its answer on a 1430 rung and a relay that had room to exist
///
/// The shared spine of every relay position: one place derives the trump (from
/// the **original** ask's index, so all six rungs count against the one suit),
/// one place holds the recognizability gates [`keycard_asked_face`] holds, and
/// one place decides the relay existed at all.  Returns the trump and the 1430
/// answer the relay hangs off.
fn relay_window(hand: Hand, context: &Context<'_>, back: usize) -> Option<(Suit, Bid, usize)> {
    if !relay_window_face(context, back) {
        return None;
    }
    let auction = context.auction();
    let anchor = auction.len() - back;
    let ask = keycard_ask_bid(auction, anchor)?;
    let Call::Bid(answer) = auction[anchor + 2] else {
        return None;
    };
    let step = answer_step(ask, answer)?;
    let trump = answer_trump(hand, context, anchor)?;
    queen_ask_room(answer, trump)?;
    Some((trump, answer, step))
}

/// The face half of [`relay_window`]: every gate that reads the auction alone,
/// no hand and no reading
///
/// The [`Rules::face`] gate the relay's artificial rungs attach to, so bidder
/// and reader share one predicate and cannot drift — the §7.3.1 discipline,
/// and it matters more here than anywhere: the relay's rungs land on
/// 4♠/4NT/5♣–5NT/6♣–6♥, so an ungated alerted rule would erase the natural
/// reading of those calls on every face in the game.  Necessarily looser than
/// the constraint (it cannot derive the trump), which is the right direction:
/// the gate must be *implied by* the constraint for exclusion to stay sound.
fn relay_window_face(context: &Context<'_>, back: usize) -> bool {
    if !floor_rkcb_now() {
        return false;
    }
    let auction = context.auction();
    let Some(anchor) = auction.len().checked_sub(back) else {
        return false;
    };
    let Some(ask) = keycard_ask_bid(auction, anchor) else {
        return false;
    };
    if !opponents_quiet_since(auction, anchor) {
        return false;
    }
    if opening_bid(auction).is_none_or(|(index, _)| index == anchor) {
        return false;
    }
    if anchor >= 2 && matches!(auction[anchor - 2], Call::Bid(bid) if bid.strain == Strain::Notrump)
    {
        return false;
    }
    matches!(auction.get(anchor + 2), Some(&Call::Bid(answer)) if answer_step(ask, answer).is_some())
}

/// The side kings we hold ourselves
fn side_kings(hand: Hand, trump: Suit) -> usize {
    Suit::ASC
        .into_iter()
        .filter(|&suit| suit != trump && hand[suit].contains(Rank::K))
        .count()
}

/// Partner's queen ask awaits our reply — the relay one round on from the 1430
/// answer we just gave.  Returns the trump and the whole reply map.
fn queen_asked(hand: Hand, context: &Context<'_>) -> Option<(Suit, RelayMap)> {
    let (trump, answer, _) = relay_window(hand, context, 6)?;
    let auction = context.auction();
    let map = relay_map(answer, trump)?;
    (auction[auction.len() - 2] == Call::Bid(map.ask)).then_some((trump, map))
}

/// Partner answered our queen ask: the trump, the combined keycard count, the
/// map the reply was read off, and the reply itself
fn queen_answered(hand: Hand, context: &Context<'_>) -> Option<(Suit, usize, RelayMap, Reply)> {
    use super::american::slam::count_keycards;
    let (trump, answer, step) = relay_window(hand, context, 8)?;
    let auction = context.auction();
    let n = auction.len();
    if matches!(auction[n - 1], Call::Bid(_)) {
        return None;
    }
    let map = relay_map(answer, trump)?;
    if auction[n - 4] != Call::Bid(map.ask) {
        return None;
    }
    let reply = read_reply(&map, auction[n - 2])?;
    let (low, high) = answer_band(step);
    Some((
        trump,
        resolve_total(count_keycards(hand, trump), low, high),
        map,
        reply,
    ))
}

/// Partner's second relay awaits our reply — we showed the queen and a king,
/// partner has the values for seven and wants one more king.  Returns the
/// trump, the index of the king we already named, and the two rungs.
fn king_asked(hand: Hand, context: &Context<'_>) -> Option<(Suit, KingRelay)> {
    let (trump, answer, _) = relay_window(hand, context, 10)?;
    let auction = context.auction();
    let n = auction.len();
    let map = relay_map(answer, trump)?;
    if auction[n - 6] != Call::Bid(map.ask) {
        return None;
    }
    let Reply::King(shown) = read_reply(&map, auction[n - 4])? else {
        return None;
    };
    let relay = king_relay(map.kings[shown].1, trump)?;
    (auction[n - 2] == Call::Bid(relay.ask)).then_some((trump, relay))
}

/// Partner answered our second relay: the trump and the side kings the
/// partnership holds (partner's cheap rung is "one more", so the total is a
/// sound floor)
fn king_answered(hand: Hand, context: &Context<'_>) -> Option<(Suit, usize)> {
    let (trump, answer, _) = relay_window(hand, context, 12)?;
    let auction = context.auction();
    let n = auction.len();
    if matches!(auction[n - 1], Call::Bid(_)) {
        return None;
    }
    let map = relay_map(answer, trump)?;
    if auction[n - 8] != Call::Bid(map.ask) {
        return None;
    }
    let Reply::King(shown) = read_reply(&map, auction[n - 6])? else {
        return None;
    };
    let relay = king_relay(map.kings[shown].1, trump)?;
    if auction[n - 4] != Call::Bid(relay.ask) {
        return None;
    }
    let partners = 1 + match auction[n - 2] {
        call if call == Call::Bid(relay.more) => 1,
        call if call == Call::Bid(relay.none) => 0,
        _ => return None,
    };
    Some((trump, side_kings(hand, trump) + partners))
}

/// The combined keycard count after partner's 1430 answer is within `range`,
/// with `trump` the agreed suit
fn keycard_total(
    trump: Suit,
    range: impl core::ops::RangeBounds<usize> + Clone + Send + Sync + 'static,
) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        keycard_answered(hand, context)
            .is_some_and(|(t, total)| t == trump && range.contains(&total))
    })
}

/// The queen question is already settled at the direct-answer position: we
/// hold it (or the ten-card fit stands in), or partner's rung was one of the
/// two-keycard ones that discloses it.  `None` when only the relay can answer.
fn queen_settled(hand: Hand, context: &Context<'_>, trump: Suit) -> Option<bool> {
    if holds_queen(hand, context, trump) {
        return Some(true);
    }
    let auction = context.auction();
    let n = auction.len();
    let ask = keycard_ask_bid(auction, n.checked_sub(4)?)?;
    let Call::Bid(answer) = auction[n - 2] else {
        return None;
    };
    match answer_step(ask, answer)? {
        3 => Some(false),
        4 => Some(true),
        _ => None,
    }
}

/// The queen cannot change what the asker bids
///
/// Two different questions share the trump suit and must not share a threshold:
///
/// - the **answerer** asks "is my suit as good as the queen?", and that has to
///   hold up for a grand, so it takes ten ([`QUEEN_FIT`]);
/// - the **asker** asks "can the reply change my call?", and at nine it cannot
///   — six is bid over a denial anyway (`QUEEN_BUFF_FIT`), so the round
///   is spent for nothing.
///
/// Both roads end at six; only the asker's saves a round of bidding.
fn queen_moot(hand: Hand, context: &Context<'_>, trump: Suit) -> bool {
    holds_queen(hand, context, trump) || {
        let shown =
            hand[trump].len() + usize::from(context.inferences().partner().length(trump).min);
        shown >= usize::from(QUEEN_BUFF_FIT)
    }
}

/// The relay is worth making at the direct-answer position: the queen is still
/// open and the ladder has room for the ask.  Its negation is the "no space to
/// ask" case, where the asker bets the small slam on four keycards.
fn relay_available(hand: Hand, context: &Context<'_>, trump: Suit) -> bool {
    if queen_settled(hand, context, trump).is_some() || queen_moot(hand, context, trump) {
        return false;
    }
    let auction = context.auction();
    let n = auction.len();
    // A quiet window only.  Under interference the 1430 ladder already trades
    // the queen away (ROPI/DOPI/DEPO), and the relay's rungs are exactly the
    // room their bid took.
    n >= 3
        && auction[n - 3] == Call::Pass
        && matches!(auction[n - 2], Call::Bid(answer) if queen_ask_room(answer, trump).is_some())
}

/// Our queen ask at the direct-answer position lands on `bid`
///
/// The *geometry* only — whether the queen is still open, whether the ladder has
/// room, and whether `bid` is the rung.  Whether the answer is worth a round is
/// the rule site's business ([`one_keycard_missing`] for the five-versus-six
/// lane, [`grand_zone`] for the seven lane).
///
/// Trump-free: [`keycard_answered`] derives it, so one rule per landing call
/// serves every trump — the same economy the 1430 rungs get from
/// [`answer_step`].
fn queen_ask_here(bid: Bid) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        keycard_answered(hand, context).is_some_and(|(trump, _)| {
            let auction = context.auction();
            relay_available(hand, context, trump)
                && matches!(auction[auction.len() - 2], Call::Bid(answer)
                    if queen_ask_room(answer, trump) == Some(bid))
        })
    })
}

/// Exactly one keycard missing — the count where the trump queen decides
/// between five and six, and so the count where asking pays
///
/// **Ask only when the answer changes the call.**  With all five keycards six
/// is bid whatever comes back, so unless the partnership is exploring seven the
/// relay would spend a round to learn something it will not act on — and spend
/// it at the five level, where the room it costs is the room the signoff needs.
/// The grand lane gets its own rule, conjoined with [`grand_zone`].
fn one_keycard_missing() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        keycard_answered(hand, context).is_some_and(|(_, total)| total == 4)
    })
}

/// The queen is known good, or the position never got to ask — the conjunct
/// that turns the asker's six into "four keycards **and** the queen"
///
/// Five keycards bid six whatever the queen does (all four aces and the trump
/// king are on the table), and where the relay had no room the asker bets it
/// on four, exactly as the floor does today.
fn queen_ok(trump: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        keycard_answered(hand, context).is_some_and(|(_, total)| {
            total >= 5
                || queen_settled(hand, context, trump) == Some(true)
                || !relay_available(hand, context, trump)
        })
    })
}

/// The relay is available, so the direct grand blast stands down and lets the
/// conversation find out — RKCB is a slam veto, not a slam seeker
fn relay_pending(trump: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| relay_available(hand, context, trump))
}

/// The one reply our hand makes to partner's queen ask
///
/// Total by construction — every hand lands on exactly one message, which is
/// what lets a single rule per landing call carry the whole ladder and what
/// makes the "skipped steps deny" reading true rather than merely intended.
fn our_reply(hand: Hand, context: &Context<'_>, trump: Suit, map: &RelayMap) -> Reply {
    if !holds_queen(hand, context, trump) {
        return if queen_buff(hand, context, trump) {
            Reply::Buff
        } else {
            Reply::Weak
        };
    }
    map.kings
        .iter()
        .position(|&(suit, _)| hand[suit].contains(Rank::K))
        .map_or(Reply::NoKing, Reply::King)
}

/// Where a reply lands, or `None` for a message this lane cannot carry
fn reply_call(map: &RelayMap, reply: Reply) -> Option<Bid> {
    Some(match reply {
        Reply::Weak => map.weak,
        Reply::Buff => map.deny,
        Reply::King(index) => map.kings.get(index)?.1,
        Reply::NoKing => map.no_king,
    })
}

/// Our reply to partner's queen ask lands on `bid`
///
/// Split in two by `artificial` so the alert can be: the two denials are five
/// and six of the agreed trump — contracts, not codes — while the king rungs
/// and 5NT are artificial and must be alerted and read.  One rule cannot decide
/// that from its landing call alone, because 6♠ is the denial with spades
/// agreed and a king rung with hearts.
fn queen_reply(bid: Bid, artificial: bool) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        queen_asked(hand, context).is_some_and(|(trump, map)| {
            let reply = our_reply(hand, context, trump, &map);
            matches!(reply, Reply::King(_) | Reply::NoKing) == artificial
                && reply_call(&map, reply) == Some(bid)
        })
    })
}

/// No queen, but something the ladder has no rung for and partner cannot see:
/// the fit itself at `QUEEN_BUFF_FIT`, or a side-suit void
fn queen_buff(hand: Hand, context: &Context<'_>, trump: Suit) -> bool {
    let shown = hand[trump].len() + usize::from(context.inferences().partner().length(trump).min);
    shown >= usize::from(QUEEN_BUFF_FIT)
        || Suit::ASC
            .into_iter()
            .any(|suit| suit != trump && hand[suit].is_empty())
}

/// After the relay: the combined keycard count is in `range`, the trump is
/// `trump`, and the queen came back as `want` (or the count made it moot)
fn relay_verdict(
    trump: Suit,
    range: impl core::ops::RangeBounds<usize> + Clone + Send + Sync + 'static,
    want: bool,
) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        queen_answered(hand, context).is_some_and(|(t, total, _, reply)| {
            let queen = matches!(reply, Reply::King(_) | Reply::NoKing);
            t == trump && range.contains(&total) && (queen || total >= 5) == want
        })
    })
}

/// The side kings the partnership has shown by the queen reply alone are at
/// least `want` — our own, plus the one partner named if it named one
fn kings_so_far(trump: Suit, want: usize) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        queen_answered(hand, context).is_some_and(|(t, _, _, reply)| {
            t == trump
                && side_kings(hand, trump) + usize::from(matches!(reply, Reply::King(_))) >= want
        })
    })
}

/// Our second relay over partner's queen-and-king reply lands on `bid`
///
/// Fires only when it can change the call: partner named its cheapest king, so
/// one king of our own already makes the two the grand gate wants.  With none,
/// the second king is the whole question — and with the reply already at six of
/// trump there is nowhere to put it, so [`king_relay`] declines and the asker
/// places the small slam.  The *strength* gate rides the rule site.
fn king_ask_here(bid: Bid) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        queen_answered(hand, context).is_some_and(|(trump, total, map, reply)| {
            let Reply::King(shown) = reply else {
                return false;
            };
            total >= 5
                && side_kings(hand, trump) == 0
                && king_relay(map.kings[shown].1, trump).map(|relay| relay.ask) == Some(bid)
        })
    })
}

/// Our reply to partner's second relay lands on `bid`; `more` is whether we hold
/// a second side king (the first was named by the queen reply)
fn king_reply(bid: Bid, more: bool) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        king_asked(hand, context).is_some_and(|(trump, relay)| {
            (side_kings(hand, trump) >= 2) == more
                && bid == if more { relay.more } else { relay.none }
        })
    })
}

/// Grand-zone values: the authored point floor **and** the net's verdict
///
/// [`points_and_net`] alone is not that.  With [`set_bilans_floor`] on — the
/// default — its authored arm is dead and the net's break-even test decides by
/// itself, and the net calls a grand plausible on hands the point count puts
/// nowhere near one: probing the `1♠ - 3♠ - 4NT - 5♣` asker, *every* hand strong
/// enough to reach six also cleared it.  A gate that never says no cannot veto
/// anything, and RKCB is a slam veto.
///
/// Requiring both is what makes the relay's grand lane mean what it says: the
/// points have to be there before a round is spent looking at seven, and the
/// net still holds its veto over them.
fn grand_zone(strain: Strain) -> Cons<impl Constraint + Clone> {
    combined_points(37) & points_and_net(combined_points(37), strain, 13)
}

/// The side kings the partnership showed over our king ask are in `range`
fn king_total(
    trump: Suit,
    range: impl core::ops::RangeBounds<usize> + Clone + Send + Sync + 'static,
) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        king_answered(hand, context).is_some_and(|(t, kings)| t == trump && range.contains(&kings))
    })
}

/// Partner's 1430 answer was five of the agreed trump itself, so passing
/// plays it — the cramped-minor signoff
fn answer_is_five_of(trump: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        context.last_bid() == Some(Bid::new(5, Strain::from(trump)))
    })
}

/// Their double sits directly on partner's 1430 answer — the asker is
/// looking at a doubled artificial contract, not a place to play.  Combined
/// with [`keycard_total`] (which pins the ask at `n - 4` and the answer at
/// `n - 2` inside a quiet window), a trailing double can only be theirs and
/// can only double the answer.
fn answer_doubled() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.auction().last() == Some(&Call::Double))
}

/// We answered partner's keycard ask and partner has since placed the
/// contract — respect the placement (the asker holds the count); never
/// milestone past it
///
/// The exception: holding **two or more** keycards ourselves, the milestone
/// drive stays live — the asker may have taken an ambiguous answer's low
/// reading, and the book's asker tables sign off pessimistically (after a 5♥
/// answer with two own keycards the total is four, one missing, yet they
/// stop); the answerer corrects with a maximum, the standard agreement.
/// Rounds 2–3 of the A/B lost 11 IMPs a board suppressing exactly those
/// corrections.  With at most one keycard the total genuinely cannot be
/// slam-safe, so the placement stands.
fn respect_keycard_signoff() -> Cons<impl Constraint + Clone> {
    use super::american::slam::count_keycards;
    pred(|hand: Hand, context: &Context<'_>| {
        if !floor_rkcb_now() {
            return false;
        }
        let auction = context.auction();
        let n = auction.len();
        let Some(ask) = n.checked_sub(6).and_then(|i| keycard_ask_bid(auction, i)) else {
            return false;
        };
        if !opponents_quiet_since(auction, n - 6) {
            return false;
        }
        let (Call::Bid(answer), Call::Bid(signoff)) = (auction[n - 4], auction[n - 2]) else {
            return false;
        };
        // Landing on a rung is the whole test: over a plain 4NT ask the rungs
        // are the four five-level suits, so the old `level 5 && is_suit` guard
        // is exactly this one — but a relocated ask's step 1 can be 4NT itself
        // (4♠ asking in hearts), which that guard would have thrown away.
        if answer_step(ask, answer).is_none() {
            return false;
        }
        let trump = match signoff.strain.suit() {
            Some(trump) => trump,
            // The doubled-answer escape can land in 5NT (the asker's escape
            // rungs): respect it like any placement, deriving the trump
            // through the shared ladder.  Only when their X sits on our
            // answer — an undisturbed 5NT stays the king ask, not ours to
            // pass.
            None => {
                if signoff.level.get() != 5 || auction[n - 3] != Call::Double {
                    return false;
                }
                match answer_trump(hand, context, n - 6) {
                    Some(trump) => trump,
                    None => return false,
                }
            }
        };
        count_keycards(hand, trump) <= 1
    })
}

/// Partner's last call reads as a control bid agreeing `trump` — the M6.4
/// classifier's own witness carried on [`Inferences`] (a to-play four-level
/// bid is also unread, so "the named suit floors nothing" cannot tell them
/// apart).  A slam try agreeing a suit is never a place to play: it must not
/// be passed out.
///
/// [`Inferences`]: super::inference::Inferences
fn partner_control_bid(trump: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        if !super::inference::control_bid_reading() || !context.undisturbed() {
            return false;
        }
        let n = context.auction().len();
        n >= 2
            && context
                .inferences()
                .control_bid()
                .is_some_and(|(index, agreed)| usize::from(index) == n - 2 && agreed == trump)
    })
}

/// A known eight-card fit in `suit`: our exact length plus partner's shown floor
/// reaches eight — the bridge-true test, sound because it counts partner's
/// *guaranteed* minimum ([`Inferences`]), never an overbid
///
/// This replaces a hand-rolled enumeration of length pairs (`(5,3)|(3,5)|(2,6)`)
/// that only recognised a fit when *one* hand showed five-plus, so a bare 4-4
/// (opener's jump-shift/reverse names a four-card suit; responder holds four) was
/// invisible and a known eight-card major slipped into `3NT` instead of `4M`.
/// The sole exception is a bare 4-4 opposite our own flat `4-3-3-3`: no ruffing
/// value, so notrump's nine-trick game outscores the suit's ten and it is not a
/// playing fit (a shapely partner is not 4333 and shows the fit from its own
/// seat).  A/B'd a plain-DD win at both vulnerabilities.
///
/// [`Inferences`]: super::inference::Inferences
fn known_eight_card_fit(suit: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        let mine = hand[suit].len();
        let partner = usize::from(context.inferences().partner().length(suit).min);
        mine + partner >= 8 && !bare_four_four_own_flat(hand, suit, partner)
    })
}

/// A bare 4-4 (neither hand five-plus) opposite our own flat 4-3-3-3 has no
/// ruffing value — sorted lengths [4,3,3,3] is exactly that shape.  The
/// measured carve of [`known_eight_card_fit`], shared with the RKCB ask gate
/// so a fit the machinery refuses to play never becomes its trump.
fn bare_four_four_own_flat(hand: Hand, suit: Suit, partner_min: usize) -> bool {
    hand[suit].len() == 4 && partner_min == 4 && {
        let mut lens = Suit::ASC.map(|s| hand[s].len());
        lens.sort_unstable_by(|a, b| b.cmp(a));
        lens == [4, 3, 3, 3]
    }
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
        let partner_min = context.inferences().partner().strength.shown_floor();
        u16::from(point_count(hand)) + u16::from(partner_min) >= u16::from(threshold)
    })
}

/// Raw HCP of a whole hand — the notrump valuation, no distributional upgrade
fn raw_hcp(hand: Hand) -> u8 {
    Suit::ASC
        .iter()
        .map(|&suit| holding_hcp::<u8>(hand[suit]))
        .sum()
}

/// [`combined_points`] for notrump: both hands on raw HCP (length and shortness
/// are worthless in notrump), reading partner's crisp `hcp` gauge when populated
///
/// Edit 2 — with [`NT_HCP_READ`] off this is [`combined_points`] verbatim
/// (`point_count` own, partner's length-scale `points` floor), so the notrump
/// milestones stay byte-identical until the knob flips.
fn combined_hcp(threshold: u8) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        let inferences = context.inferences();
        let partner = inferences.partner();
        let (own, partner_min) = if NT_HCP_READ.with(Cell::get) {
            (
                raw_hcp(hand),
                partner
                    .strength
                    .hcp_floor()
                    .unwrap_or_else(|| partner.strength.shown_floor()),
            )
        } else {
            (point_count(hand), partner.strength.shown_floor())
        };
        u16::from(own) + u16::from(partner_min) >= u16::from(threshold)
    })
}

/// [`combined_points`] against the live [`FLOOR_SLAM_ENTRY`] floor — read at
/// classify time (not baked at construction) so an A/B harness can flip the
/// RKCB-ask threshold per call, matching the other floor knobs.
fn slam_entry_reached() -> Cons<impl Constraint + Clone> {
    pred(|hand: Hand, context: &Context<'_>| {
        // Bilans mode: enter keycarding once the small slam clears the entry
        // probability — the net analogue of the support-point floor below.
        if BILANS_FLOOR.with(Cell::get) {
            return keycard_trump(hand, context).is_some_and(|trump| {
                let strain = Strain::from(trump);
                bilans_accepts(
                    hand,
                    context,
                    strain,
                    12,
                    BilansThreshold::Inclusive(SLAM_ENTRY_P),
                )
            });
        }
        // Fit-known: the RKCB ask only fires on a shown trump, so count
        // shortness as support value.  Still the suit-blind scalar — the
        // trump here is dynamic (`keycard_trump`), and the migration to
        // `support_point_count_in` is a ledger follow-up with its own
        // FLOOR_SLAM_ENTRY resweep.
        let partner_min = partner_slam_strength(context);
        u16::from(support_point_count(hand)) + u16::from(partner_min)
            >= u16::from(FLOOR_SLAM_ENTRY.with(Cell::get))
    })
}

/// The major-game gate that counts trump length as points (see [`FIT_SUM_GAME`]).
///
/// Folds the known combined trump length — our holding in `suit` plus partner's
/// shown floor, the same sum [`known_eight_card_fit`] gates on — into the point
/// total: game once `own_points + partner_shown_floor + fit >= t` (default `t =
/// 31`; the floor is `Strength::shown_floor`, the legacy `points` floor
/// lifted by any populated support promise).  A ninth trump then buys game a
/// point cheaper, a tenth two.  Partner's
/// *minimum* length and points keep it a sound floor, never an overbid.  The
/// eight-card-fit gate still lives on the rule's [`known_eight_card_fit`]; this
/// only moves the point boundary.
///
/// `slack` lowers the threshold, so `fit_sum_game(suit, COLLAR_SLACK)` is the
/// collar arm of [`points_or_net`] at the fitted-major game.  It subtracts from
/// the *live* [`FIT_SUM_GAME`], so the collar keeps tracking
/// [`set_fit_sum_game`] instead of pinning a second constant.
fn fit_sum_game(suit: Suit, slack: u8) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        // Fit-known (the rule pairs this with `known_eight_card_fit`), so count
        // shortness as support value — side suits only, never the own trump
        // holding; the fit length term is explicit.
        let inferences = context.inferences();
        let partner = inferences.partner();
        // Edit 1: partner's raise valued on the support scale when that gauge is
        // populated (a fit-showing raise fired), else the length-scale floor.
        let partner_pts = FIT_SUM_SUPPORT_READ
            .with(Cell::get)
            .then(|| partner.strength.support_floor(suit))
            .flatten()
            .unwrap_or_else(|| partner.strength.shown_floor());
        let own = support_point_count_in(hand, suit);
        let fit = hand[suit].len() as u16 + u16::from(partner.length(suit).min);
        u16::from(own) + u16::from(partner_pts) + fit
            >= u16::from(FIT_SUM_GAME.with(Cell::get).saturating_sub(slack))
    })
}

/// Make probability of the small slam at which the bilans floor's RKCB ask
/// fires — deliberately below [`break_even`]'s even-money decision line,
/// because the ask buys information: inside the band the keycard answer
/// converts the guess (two keycards missing → sign off at five, at most one →
/// bid the slam).  The one bilans constant with no derivation behind it; sweep
/// it if the A/B lands close.
const SLAM_ENTRY_P: f32 = 0.35;

/// The make probability at which bidding on breaks even in IMPs against
/// stopping in the cold alternative — the economics half of the bilans floor
///
/// A gate prices a *call*, and the house scoring split prices calls under
/// perfect defense (`ns_score_bid` is the call scorer; the M3.1 7NT flood is
/// why), so the failing branch of the contemplated game is **down one
/// doubled**: non-vul risks 6 IMPs (−100 against the +140 partscore) to gain
/// 6, vul risks 8 (−200) to gain 10.  The undoubled derivation in
/// `docs/ai-bidder/evaluator-net.md` gave 5/11 and 6/16; doubling our own
/// failure is also the adverse-selection premium (the layouts where a marginal
/// game fails are the ones where the double gets found) and a hedge against
/// the estimator's winner's curse at the firing margin.  The slam and grand
/// rows keep the plain-derived values: their risk side is dominated by the
/// lost game bonus, and the doubled undertrick moves them at most one IMP
/// bucket — inside the derivation's own assumption noise.  `tricks` keys the
/// decision: ≤ 11 is a game, 12 the small slam, 13 the grand.
// ponytail: the endpoints (plain 5/11–6/16, doubled 6/12–8/18) bracket the
// truth; a q = P(doubled | we fail) dial interpolating them is the upgrade if
// neither endpoint A/Bs clean.
fn break_even(tricks: u8, strain: Strain, vul_we: bool) -> f32 {
    match (tricks, strain, vul_we) {
        (..=11, _, false) => 6.0 / 12.0,
        (..=11, _, true) => 8.0 / 18.0,
        (12, _, _) => 0.5,
        (_, Strain::Hearts | Strain::Spades, false) => 14.0 / 24.0,
        (_, Strain::Hearts | Strain::Spades, true) => 17.0 / 30.0,
        (_, _, false) => 14.0 / 25.0,
        (_, _, true) => 16.0 / 29.0,
    }
}

/// Who would declare a contract of ours in `strain`: the first of our side to
/// have named it in the auction, else the actor (bidding it now would name it
/// first).  The evaluator's trick columns are per-declarer — double-dummy
/// tricks depend on who is on lead — so the bilans gates price the seat that
/// would actually play the hand.
fn our_declarer(context: &Context<'_>, strain: Strain) -> Relative {
    let auction = context.auction();
    auction
        .iter()
        .enumerate()
        .find_map(
            |(index, &call)| match (call, relative_of(auction.len(), index)) {
                (Call::Bid(bid), who @ (Relative::Me | Relative::Partner))
                    if bid.strain == strain =>
                {
                    Some(who)
                }
                _ => None,
            },
        )
        .unwrap_or(Relative::Me)
}

/// The comparison made by a bilans trick gate
///
/// Keep strict and inclusive thresholds distinct: several established rules
/// intentionally disagree at exactly 0.5, and cache plumbing must not move
/// that boundary.
#[derive(Clone, Copy)]
enum BilansThreshold {
    Strict(f32),
    Inclusive(f32),
    BreakEven,
}

/// Evaluate one named bilans boundary from the decision-scoped forward pass
fn bilans_accepts(
    hand: Hand,
    context: &Context<'_>,
    strain: Strain,
    tricks: u8,
    threshold: BilansThreshold,
) -> bool {
    let probability =
        context
            .trick_estimates(hand)
            .p_at_least(strain, our_declarer(context, strain), tricks);
    match threshold {
        BilansThreshold::Strict(value) => probability > value,
        BilansThreshold::Inclusive(value) => probability >= value,
        BilansThreshold::BreakEven => {
            probability
                >= break_even(
                    tricks,
                    strain,
                    context.vul().contains(RelativeVulnerability::WE),
                )
        }
    }
}

/// A classification-time bilans constraint with one explicit boundary
///
/// The knob test remains ahead of the evaluator so disabled arms retain their
/// historical short-circuit behavior even though surrounding `And`/`Or`
/// constraints are eager.
fn bilans_trick_gate(
    knob: fn() -> bool,
    want: bool,
    strain: Strain,
    tricks: u8,
    threshold: BilansThreshold,
) -> Cons<impl Constraint + Clone> {
    pred(move |hand: Hand, context: &Context<'_>| {
        knob() && want == bilans_accepts(hand, context, strain, tricks, threshold)
    })
}

/// With [`set_bilans_floor`] on, the net likes `tricks` of ours in `strain` at
/// better than even money
///
/// Unlike [`points_or_net`] this converts no authored arithmetic — it is a gate
/// that exists *only* knob-on, for a decision the point sums never priced.
fn net_makes(strain: Strain, tricks: u8) -> Cons<impl Constraint + Clone> {
    bilans_trick_gate(
        bilans_enabled,
        true,
        strain,
        tricks,
        BilansThreshold::Strict(0.5),
    )
}

/// A **game** milestone's authored point arithmetic, which the evaluator net may
/// *accelerate* — reach below, no further than `collar`
///
/// Three states, and the first two are the shipped ones:
///
/// - Both knobs off: exactly `authored`.  The net arm is `-∞` and falls out of
///   the [`Or`][super::constraint] max without touching the net (its predicate
///   short-circuits on the thread-local); the masking arms contribute `0.0`.
/// - [`set_bilans_floor`] on, [`set_net_collar`] off: exactly the net.  The
///   authored arm is masked to `-∞` and the gate is
///   `P(≥ tricks by our declarer in strain) ≥ break_even` — an unbounded reach
///   *and* an unbounded veto over hands the point sums accept.
/// - Both on: `authored | (collar & net)`.  The arithmetic decides on its own
///   above the threshold, the net gets a vote only inside the collar, and it can
///   no longer veto a hand the arithmetic accepts.
///
/// Games break even at or *below* even money in IMPs ([`break_even`]), which is
/// why the collared licence here is to add rather than to decline; the slam
/// milestones take the mirror shape, [`points_and_net`].
// The decision-scoped Context memoizes the evaluator, so every eager rule in
// this ladder observes the same one forward pass.
fn points_or_net(
    authored: Cons<impl Constraint + Clone>,
    collar: Cons<impl Constraint + Clone>,
    strain: Strain,
    tricks: u8,
) -> Cons<impl Constraint + Clone> {
    debug_assert!(tricks <= 11, "a game milestone, per break_even's own key");
    // ponytail: the `!net_collar()` legs are the legacy mask, byte-identical to
    // the shipped wiring; they die with the knob flip whichever way it lands.
    (authored & (net_collar() | !bilans_floor()))
        | ((!net_collar() | collar) & net_break_even_gate(bilans_enabled, true, strain, tricks))
}

/// A **slam** milestone's authored point arithmetic, which the evaluator net may
/// only *veto* — decline, never reach below
///
/// The mirror of [`points_or_net`]: slams break even at or *above* even money, so
/// the cheap direction is to decline rather than to add.  Knob states, as there:
/// both off is exactly `authored`; [`set_bilans_floor`] alone is exactly the net
/// (the legacy mask); both on is `authored & net`, which needs no collar because
/// it only ever shrinks the accepted set — and therefore keeps `authored`'s own
/// reading, loose in the safe direction.
fn points_and_net(
    authored: Cons<impl Constraint + Clone>,
    strain: Strain,
    tricks: u8,
) -> Cons<impl Constraint + Clone> {
    debug_assert!(tricks >= 12, "a slam milestone, per break_even's own key");
    // ponytail: as in `points_or_net`, the first two arms are the legacy mask.
    (!net_collar() & !bilans_floor() & authored.clone())
        | (!net_collar() & net_break_even_gate(bilans_enabled, true, strain, tricks))
        | (net_collar()
            & authored
            & (!bilans_floor() | net_break_even_gate(bilans_enabled, true, strain, tricks)))
}

/// The evaluator net's verdict on `tricks` of ours in `strain`, as a
/// classification-time constraint arm for a converted seam: finite only when
/// `knob()` is on **and** the break-even comparison equals `want`
///
/// The generalisation of [`points_or_net`]'s net arm over which knob gates it
/// and over the verdict's sign — the invite side of a converted force/invite
/// seam needs the *declined* half (`want == false`) so the pair stays a
/// partition.  The knob check short-circuits ahead of the forward pass (And/Or
/// constraints do not short-circuit), so knob-off never touches the net.
pub(crate) fn net_break_even_gate(
    knob: fn() -> bool,
    want: bool,
    strain: Strain,
    tricks: u8,
) -> Cons<impl Constraint + Clone> {
    bilans_trick_gate(knob, want, strain, tricks, BilansThreshold::BreakEven)
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
/// Each flag is recovered by a short walk of the auction and memoized by a
/// classification-scoped [`Context`]. Every flag here is *hand-independent* —
/// it follows from the calls alone — so hand-conditioned forces (a
/// strong-notrump responder who holds game values) stay as ordinary
/// [`Constraint`]s rather than living here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Interpretation {
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
    pub(crate) fn read(context: &Context<'_>) -> Self {
        Self {
            forced_to_game: forcing_two_clubs_response(context)
                || opener_forced_past_invitation(context)
                || two_over_one_game_force(context),
            penalizing: penalizing(context),
        }
    }
}

/// Partner answered our one-of-a-suit opening with a game-forcing two-over-one
///
/// Exactly the auctions the 2/1 game-force book registers
/// its tables over: `1♥`/`1♠` and a cheaper two-level suit, or `1♦ - 2♣`.  Both
/// partners hold the force, so the flag is read from either seat.
///
/// Uncontested only — over interference a two-level new suit is a free bid, not
/// a game force — which also matches the `Undisturbed` guard the deleted game
/// backstop carried.  Without this the floor has no idea it is forced and
/// happily passes partner's 2/1 in a partscore ([`set_two_over_one_force`]).
fn two_over_one_game_force(context: &Context<'_>) -> bool {
    let auction = context.auction();
    if !two_over_one_force() || !context.undisturbed() {
        return false;
    }
    let Some((index, opening)) = opening_bid(auction) else {
        return false;
    };
    // The player to act must be on the opening side — opener or responder.
    if index % 2 != auction.len() % 2 || opening.level.get() != 1 {
        return false;
    }
    let Some(&Call::Bid(response)) = auction.get(index + 2) else {
        return false;
    };
    response.level.get() == 2
        && match opening.strain {
            // A 2/1 sits *below* the opening; higher is a jump shift.
            Strain::Hearts | Strain::Spades => response.strain < opening.strain,
            Strain::Diamonds => response.strain == Strain::Clubs,
            _ => false,
        }
}

/// A prior call has committed our side to game (see [`Interpretation`])
fn auction_forces_game() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| context.interpretation().forced_to_game)
}

/// We are not sitting for a penalty double of our own (see [`Interpretation`])
///
/// A game force forbids passing below game — *unless* we are penalizing the
/// opponents, where passing their doubled contract out is the whole point.
/// There the forced-to-game rules step aside and let the natural defense —
/// including the [advance][advancing_a_double] of partner's penalty double —
/// govern.
fn not_penalizing() -> Cons<impl Constraint + Clone> {
    pred(|_: Hand, context: &Context<'_>| !context.interpretation().penalizing)
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
/// business redouble (`(1NT) X (XX)`) marks their side with the values, so a weak
/// advancer escapes to its long suit rather than sit for a making `1NTxx`.  The
/// mirror of the [responder runout][`set_one_nt_runout`] on the defensive side.
/// Disable for the off arm of the A/B; read at classification time, per-thread.
#[doc(hidden)]
pub fn set_advancer_xx_runout(enabled: bool) {
    ADVANCER_XX_RUNOUT.with(|flag| flag.set(enabled));
}

/// Their redoubled penalty double is back to a weak advancer (`(1NT) X (XX)`) and
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
/// **On by default.**  After `(1NT) X (XX)` the opponents' business redouble runs
/// back around — advancer passes, opener passes (`(1NT) X (XX) - -`) — to the 15+
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

/// Their redoubled penalty double has run back to the doubler (`(1NT) X (XX) - -`)
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
/// prior call committing our side to game, partner's just-made transfer over
/// our strong notrump, or a live keycard conversation
/// ([`keycard_conversation_now`])
///
/// Hand-independent — it follows from the calls alone.  The neural safety shell
/// consults it to decide when to delegate to the deterministic [`instinct()`]
/// ladder instead of trusting the learned net: the net handles the judgement
/// middle, but never these forced rails.  Hand-conditioned forces (a
/// strong-notrump responder who holds game values) are deliberately excluded —
/// they are judgement the net is trusted with, measured on the harness.
pub(crate) fn forced(context: &Context<'_>) -> bool {
    advancing_a_double_now(context)
        || context.interpretation().forced_to_game
        || TRANSFERS
            .iter()
            .any(|&(nt_level, from, _)| partner_transferred_now(context, from, nt_level))
        || keycard_conversation_now(context)
}

/// A live keycard conversation, judged from the auction alone: partner's
/// ask awaits our answer, partner's 1430 answer awaits our placement, or
/// partner has placed the contract over our answer.  Auction-determined like
/// the other [`forced`] arms — the neural shell delegates these to the
/// deterministic ladder, because a keycard window is a convention in motion,
/// not judgement: the reading-drift A/B's worst boards were the net
/// freewheeling inside one (a 4NT passed out, a 5♦ answer played, a doubled
/// 5♣ answer redoubled and left in).  Over-matching is NOT safe: every
/// shape requires the recognizable-ask gate, because a hijacked window with
/// no decodable trump has no continuation rung and strands the auction in
/// the 1430 answer — the round-2 A/B's worst boards were making minor slams
/// passed out in 5♥.  Shares [`opponents_quiet_since`] with the machinery's
/// own gates so the two never disagree.
pub(crate) fn keycard_conversation_now(context: &Context<'_>) -> bool {
    if !floor_rkcb_now() {
        return false;
    }
    let auction = context.auction();
    let n = auction.len();
    // The ask `k` calls back, and whether a call sits on one of its 1430 rungs.
    // Over a plain 4NT ask the rungs are exactly the four five-level suits this
    // rail has always matched, so kickback-off behaviour is unchanged.
    let ask_at = |k: usize| {
        n.checked_sub(k)
            .and_then(|index| keycard_ask_bid(auction, index))
    };
    let rung =
        |ask: Bid, call: Call| matches!(call, Call::Bid(bid) if answer_step(ask, bid).is_some());
    // The window is only ours to police when the ask itself is recognizable:
    // not an opening, not over the asker's own side's notrump bid
    // (quantitative), and trump-decodable — [`face_trump`] reads one off the
    // auction alone (the known fit, or the side's last bid a natural suit),
    // or a five-card major is genuinely shown (the readings branch that sees
    // through transfers where the face mislabels).  Every shape needs this,
    // not just `asked`: an unrecognizable 4NT (quantitative, or a minor-slam
    // probe) is judgement all the way down — hijacking its five-level answer
    // leaves the ladder with no continuation rung, and the round-2 A/B's
    // worst boards were making minor slams passed out in 5♥.
    let recognizable = |ask: usize| {
        opening_bid(auction).is_some_and(|(index, _)| index != ask)
            && !(ask >= 2
                && matches!(auction[ask - 2], Call::Bid(bid) if bid.strain == Strain::Notrump))
            && (face_trump(auction, ask).is_some() || {
                let inferences = context.inferences();
                [Suit::Hearts, Suit::Spades].into_iter().any(|major| {
                    inferences
                        .me()
                        .length(major)
                        .min
                        .max(inferences.partner().length(major).min)
                        >= 5
                })
            })
    };
    // Partner just asked: answer.
    let asked = ask_at(2).is_some() && opponents_quiet_since(auction, n - 2) && recognizable(n - 2);
    // Their bid directly over partner's ask: the DOPI/DEPO window (their
    // double keeps `asked` itself alive — the ROPI rungs answer it).
    let asked_over_bid =
        ask_at(2).is_some() && matches!(auction[n - 1], Call::Bid(_)) && recognizable(n - 2);
    // We asked and partner answered on a 1430 rung: place the contract.
    let answered = ask_at(4).is_some_and(|ask| {
        rung(ask, auction[n - 2]) && opponents_quiet_since(auction, n - 4) && recognizable(n - 4)
    });
    // We asked over their double and partner answered in ROPI's non-bid
    // messages (the step-1 bid already matches `answered`): place.
    let ropi_answered = ask_at(4).is_some_and(|_| {
        auction[n - 3] == Call::Double
            && matches!(auction[n - 2], Call::Redouble | Call::Pass)
            && opponents_quiet_since(auction, n - 4)
            && recognizable(n - 4)
    });
    // We asked over their bid and partner answered in DOPI/DEPO: place.
    // Their bid over the answer stands the machinery down; the third round
    // after a non-bid answer is left to judgement — filed.  The DOPI step is
    // measured from *their* bid, not from the ask, so it needs its own arm
    // once a relocated ask lets their interference sit at the four level.
    let dopi_answered = ask_at(4).is_some_and(|ask| {
        let dopi_step_answer = matches!(auction[n - 2], Call::Bid(bid)
            if matches!(auction[n - 3], Call::Bid(their) if bid_successor(their) == Some(bid)));
        matches!(auction[n - 3], Call::Bid(_))
            && (matches!(auction[n - 2], Call::Double | Call::Pass)
                || rung(ask, auction[n - 2])
                || dopi_step_answer)
            && !matches!(auction[n - 1], Call::Bid(_))
            && recognizable(n - 4)
    });
    // We answered and partner placed: respect (or correct) the placement.
    let placed = ask_at(6).is_some_and(|ask| {
        rung(ask, auction[n - 4])
            && matches!(auction[n - 2], Call::Bid(_))
            && opponents_quiet_since(auction, n - 6)
            && recognizable(n - 6)
    });
    // The relay's own rounds: partner's queen or king ask
    // awaiting our reply, and our own ask awaiting partner's placement.  The
    // rail has to reach them for the same reason it reaches the 1430 rungs — a
    // relay is a convention in motion, and the net freewheeling inside one
    // passes the ask out or plays the artificial reply.  Every arm is
    // `relay_window_face`, so it is inert off a live relay.
    let relaying = [4usize, 6, 8, 10, 12]
        .into_iter()
        .any(|back| relay_window_face(context, back));
    asked || asked_over_bid || answered || ropi_answered || dopi_answered || placed || relaying
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
    // ... and the side's FIRST action: a double before the bid makes this an
    // advance-of-double structure, not an overcall advance — the doubler's
    // later cue of the opening is a strong raise, never a Rubens transfer
    // (A/B'd: transfer-detecting these tails died in unauthored passouts).
    if auction[open_index + 1..overcall_index].contains(&Call::Double) {
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
/// `(1X) Y -` to us
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
        rubens_advances_enabled()
            && advance_of_overcall(context).is_some_and(|(x, y, level)| {
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
        rubens_advances_enabled()
            && advance_of_overcall(context)
                .is_some_and(|(x, _, level)| level == 2 && x as u8 == cue as u8)
    })
}

/// Partner answered our simple one-level overcall with a Rubens transfer, RHO
/// passing — the cue to complete it
///
/// Returns the suit to complete into: the suit just above partner's transfer.
/// Mechanical (hand-independent), like completing a transfer over our own
/// notrump — see [`TRANSFERS`].
fn rubens_completion(context: &Context<'_>) -> Option<Suit> {
    if !rubens_advances_enabled() {
        return None;
    }
    let auction = context.auction();
    let len = auction.len();
    let (x, y, overcall_index, level) = overcall_shape(auction)?;
    // Only a one-level overcall carries the transfer ladder; the sequence is
    // `overcall - transfer { - | (X) }`, then us.
    // Completing through the double matters: without it the relay dies and the
    // advancer plays the phantom suit doubled (A/B'd −14 IMPs a board).
    if level != 1
        || overcall_index + 4 != len
        || auction[overcall_index + 1] != Call::Pass
        || !matches!(auction[len - 1], Call::Pass | Call::Double)
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

/// Partner answered our simple two-level overcall with the Rubens cue-raise,
/// opener passing or doubling — we must place the contract
///
/// The cue names the *opponents'* suit, so passing it out plays their suit at
/// the two level (A/B'd the dominant Rubens leak: a quarter of the divergent
/// boards died in this passout).  Returns `(X = their suit, Y = our suit)`.
fn rubens_cue_answer(context: &Context<'_>) -> Option<(Suit, Suit)> {
    if !rubens_advances_enabled() {
        return None;
    }
    let auction = context.auction();
    let len = auction.len();
    let (x, y, overcall_index, level) = overcall_shape(auction)?;
    // The sequence is `overcall - cue { - | (X) }`, then us.
    if level != 2
        || overcall_index + 4 != len
        || auction[overcall_index + 1] != Call::Pass
        || !matches!(auction[len - 1], Call::Pass | Call::Double)
    {
        return None;
    }
    (auction[overcall_index + 2] == Call::Bid(Bid::new(2, Strain::from(x)))).then_some((x, y))
}

/// [`rubens_cue_answer`] as a [`Constraint`]: our overcall suit is `y`
fn rubens_cue_answers(y: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        rubens_cue_answer(context).is_some_and(|(_, own)| own == y)
    })
}

/// Partner's transfer lands in our own overcall suit `y` — the limit-plus
/// raise — and we are on completion duty ([`rubens_completion`])
///
/// The seat that grades the overcall against the shown ten-plus: complete
/// `2Y` with a minimum, super-accept `3Y` in between, break to game with a
/// maximum — the separation Rubens buys over a flat natural raise is cashed
/// here or nowhere.
fn rubens_into_partner(y: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        rubens_completion(context) == Some(y)
            && overcall_shape(context.auction()).is_some_and(|(_, own, _, _)| own == y)
    })
}

/// Partner mechanically completed our transfer into its own suit `y` — the
/// limit-plus raise rests at `2Y` unless we hold extras
///
/// The auction is `(1X) Y - 2S (-|X) 2Y (-|X)` back to us: the completion
/// limited partner to no super-accept, so only extras beyond our shown
/// ten-plus move again.
fn rubens_raiser_rebid(context: &Context<'_>) -> Option<Suit> {
    if !rubens_advances_enabled() {
        return None;
    }
    let auction = context.auction();
    let len = auction.len();
    let (x, y, overcall_index, level) = overcall_shape(auction)?;
    if level != 1 || overcall_index + 6 != len || auction[overcall_index + 1] != Call::Pass {
        return None;
    }
    // Our transfer into partner's suit…
    let Call::Bid(transfer) = auction[overcall_index + 2] else {
        return None;
    };
    let source = transfer.strain.suit()?;
    if transfer.level.get() != 2 || (source as u8) < (x as u8) || source as u8 + 1 != y as u8 {
        return None;
    }
    // …mechanically completed (opener may have doubled either turn).
    (matches!(auction[overcall_index + 3], Call::Pass | Call::Double)
        && auction[overcall_index + 4] == Call::Bid(Bid::new(2, Strain::from(y)))
        && matches!(auction[len - 1], Call::Pass | Call::Double))
    .then_some(y)
}

/// [`rubens_raiser_rebid`] as a [`Constraint`]: partner's suit is `y`
fn rubens_raiser_rebids(y: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| rubens_raiser_rebid(context) == Some(y))
}

/// Partner mechanically completed our *new-suit* transfer into `target` — the
/// wide-yet-unlimited hand clarifies now
///
/// The auction is `(1X) Y - 2S (-|X) 2(S+1) (-|X)` back to us with
/// `S+1 ≠ Y`: the transfer showed the suit cheaply without settling forcing
/// questions, so a mild hand passes the completion and extras move again.
fn rubens_transferee_rebid(context: &Context<'_>) -> Option<Suit> {
    if !rubens_advances_enabled() {
        return None;
    }
    let auction = context.auction();
    let len = auction.len();
    let (x, y, overcall_index, level) = overcall_shape(auction)?;
    if level != 1 || overcall_index + 6 != len || auction[overcall_index + 1] != Call::Pass {
        return None;
    }
    let Call::Bid(transfer) = auction[overcall_index + 2] else {
        return None;
    };
    let source = transfer.strain.suit()?;
    // Band first: `source < y` also keeps the `source + 1` index in range.
    if transfer.level.get() != 2 || (source as u8) < (x as u8) || (source as u8) >= (y as u8) {
        return None;
    }
    let target = Suit::ASC[(source as u8 + 1) as usize];
    if target == y {
        return None;
    }
    (matches!(auction[overcall_index + 3], Call::Pass | Call::Double)
        && auction[overcall_index + 4] == Call::Bid(Bid::new(2, Strain::from(target)))
        && matches!(auction[len - 1], Call::Pass | Call::Double))
    .then_some(target)
}

/// [`rubens_transferee_rebid`] as a [`Constraint`]: our shown suit is `target`
fn rubens_transferee_rebids(target: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| rubens_transferee_rebid(context) == Some(target))
}

/// Partner's transfer names a *new* suit `target` and we are on completion
/// duty ([`rubens_completion`])
///
/// The mechanical completion covers exactly the hands that would have passed a
/// natural non-forcing `2 target`; a hand good enough to bid over that makes
/// the same descriptive bid here instead of completing.
fn rubens_new_suit_completion(target: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        rubens_completion(context) == Some(target)
            && overcall_shape(context.auction()).is_some_and(|(_, own, _, _)| own != target)
    })
}

/// `2 target` is a *natural* new-suit advance of partner's one-level overcall,
/// live only while Rubens advances are off ([`set_rubens_advances`])
///
/// The knob-off baseline: the natural five-card-suit overcall rule is anchored
/// on [`we_have_not_bid`], dead for the advancer, so without this the band
/// hands (own five-card suit between their suit and partner's) would have *no*
/// call and the A/B would measure Rubens against a pass, not against natural
/// advances.  Covers exactly the hand class of the new-suit
/// [`rubens_transfer`], at the same weight.
fn natural_new_suit_advance(target: Suit) -> Cons<impl Constraint + Clone> {
    pred(move |_: Hand, context: &Context<'_>| {
        !rubens_advances_enabled()
            && advance_of_overcall(context).is_some_and(|(x, y, level)| {
                level == 1 && (x as u8) < (target as u8) && (target as u8) < (y as u8)
            })
    })
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
        .rule(Call::Pass, 150, advancing_a_double() & doubled_suit_stack())
        // Settle floor (opt-in): a takeout double is not 100% forcing.  With four
        // cards behind their doubled suit, *defend* — pass plays their doubled
        // contract.  Above the advance ladder (new suit ~1.0, raises 1.2) and the
        // 0.3 notrump escape, below the trump stack 1.5 and the game jumps 1.45.
        .rule(
            Call::Pass,
            135,
            settle_floor() & advancing_a_double() & doubled_suit_length(),
        )
        // Forced: 3NT to play with game values and their suits stopped.
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            advancing_a_double()
                & hcp(13..)
                & stopper_in_their_suits()
                & level_available(3, Strain::Notrump),
        )
        // Default unforced pass.  Under the settle floor it is also available in a
        // advance of partner's double (pass plays the top bid) — it still loses to every advance
        // rule, so a bust with no penalty advances as before.
        .rule(Call::Pass, 0, !advancing_a_double() | settle_floor())
        // The absolute last resort, keeping logits finite when all else is illegal.
        .rule(Call::Pass, -500, hcp(0..));

    // Forced: jump to a major-suit game with four-plus cards and values —
    // in an unbid major, never in the suit partner asked us to take out of.
    for major in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(major);
        rules = rules.rule(
            Bid::new(4, strain),
            145,
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
            5
        } else {
            0
        };

        // Forced: a new suit at the cheapest level; longer suits and majors
        // are preferred.  Bidding their suit would be a cue-bid — excluded.  The
        // settle floor's `free_bid_gate` (off by default) makes the four-level new
        // suit a free bid — values and no existing fit.
        for level in 1u8..=4 {
            rules = rules
                .rule(
                    Bid::new(level, strain),
                    100 + major_bonus,
                    advancing_a_double()
                        & min_level_is(level, strain)
                        & len(suit, 4..)
                        & !they_bid(strain)
                        & free_bid_gate(level),
                )
                .rule(
                    Bid::new(level, strain),
                    110 + major_bonus,
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
        //
        // The rein (`set_rein_advance_raise`, default on) blocks the three-level+
        // rung when partner's suit was a forced advance of *our* takeout double:
        // the double already showed our values, so a minimum re-raise double-counts
        // them into a doubled game.  A genuine maximum (17+ points) still competes.
        for (level, threshold) in [(2u8, 6u8), (3, 10), (4, 13)] {
            let raise = partner_suit_is(suit)
                & partner_shown_len(suit, 3..)
                & min_level_is(level, strain)
                & support(3..)
                & points(threshold..);
            rules = if rein_advance_raise_enabled() && level >= 3 {
                rules.rule(
                    Bid::new(level, strain),
                    120,
                    raise & (!partner_advanced_our_double() | points(17..)),
                )
            } else {
                rules.rule(Bid::new(level, strain), 120, raise)
            };
        }

        // Preemptive jump to game: five-card support but too weak to invite —
        // the weak distributional raise, distinct from the point-showing raises
        // above.  Now that the floor owns advances of an overcall, this is the
        // weak end the book's `advances` used to cover.
        rules = rules.rule(
            Bid::new(4, strain),
            130,
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
                100 + major_bonus,
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
            5
        } else {
            0
        };
        rules = rules
            .rule(
                Bid::new(2, strain),
                100 + major_bonus,
                one_nt_runout_enabled()
                    & responder_one_nt_runout()
                    & len(suit, 5..)
                    & hcp(..RUNOUT_MAX_HCP),
            )
            .rule(
                Bid::new(2, strain),
                110 + major_bonus,
                one_nt_runout_enabled()
                    & responder_one_nt_runout()
                    & len(suit, 6..)
                    & hcp(..RUNOUT_MAX_HCP),
            );
    }

    // Advancer's runout from their redoubled penalty double (`(1NT) X (XX)`,
    // default on; `set_advancer_xx_runout`).  Their XX is business, so a weak
    // advancer escapes to its longest five-plus-card suit instead of sitting for a
    // making `1NTxx` — the defensive mirror of the responder runout above.  A
    // values advancer (>= `RUNOUT_MAX_HCP`) passes to defend `1NTxx` instead.
    // ponytail: five-plus suits only; a 4-4 bust still sits — add the both-minors
    // escape (cf. the `2NT` rule below) if the A/B asks for it.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
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
                advancer_xx_runout() & len(suit, 5..) & hcp(..RUNOUT_MAX_HCP),
            )
            .rule(
                Bid::new(2, strain),
                110 + major_bonus,
                advancer_xx_runout() & len(suit, 6..) & hcp(..RUNOUT_MAX_HCP),
            );
    }

    // Doubler's runout once the redouble runs back around (`(1NT) X (XX) - -`,
    // on by default; `set_doubler_xx_runout`).  Unlike the advancer, the doubler is
    // the 15+ penalty hand, so there is *no* HCP cap — a doubler holding a five-plus
    // suit (a 5332 under the default balanced gate) escapes the redoubled `1NTxx`
    // rather than defend it; a 4-3-3-3/4-4-3-2 bust has nowhere to run and sits.
    // Construction-gated so the off arm of a duplicate A/B never carries the rule.
    if doubler_xx_runout_enabled() {
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
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
                    doubler_xx_runout() & len(suit, 5..),
                )
                .rule(
                    Bid::new(2, strain),
                    110 + major_bonus,
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
        120,
        one_nt_runout_enabled() & responder_one_nt_runout() & responder_has_xx_values(),
    );

    // Runout, the bilans end: 1NT redoubled is a *contract*, and a making one
    // outscores the slam the milestones would otherwise reach — 1NT×× +5 is
    // 160 + 100 insult + 300 game + five redoubled overtricks ≈ 2560 non-vul,
    // against 990 for 6NT=.  So when the net likes our seven tricks at better
    // than even money the business redouble outranks every game and slam
    // milestone (max 1.75) instead of sitting below them at 1.2 — but stays
    // under the forcing keycard answers at 1.80+, which are not ours to pull.
    // Knob-off `net_makes` is `-∞` and this rule never fires.
    //
    // The `hcp(15..)` is what keeps this narrow.  "1NT rates to make" is nearly
    // always true opposite a 15–17 opener, so `net_makes` alone would promote
    // the redouble over the *runout* rules too — and an eight-count with a
    // six-card major wants to play 4M, not 1NT××.  The only hands that ever
    // needed rescuing are the ones a milestone was about to pull to slam, and
    // those take 15+ opposite the opening to fire at all.
    rules = rules.rule(
        Call::Redouble,
        178,
        one_nt_runout_enabled()
            & responder_one_nt_runout()
            & responder_has_xx_values()
            & hcp(15..)
            & net_makes(Strain::Notrump, 7),
    );

    // Runout, the both-minors end: 2NT = unusual, four-four in the minors with
    // no five-card suit to run to (a 5+ suit prefers the natural escape, which
    // outweighs this; a five-card major is excluded).  Opener picks the better
    // minor.  Weight below the suit escape, above Pass — a 4-4 minor bust escapes
    // 1NT-X to a known eight-card fit rather than sit.  Opt-in only: the default
    // `Direct` mode runs straight to a minor (below); A/B'd the relay a loser.
    rules = rules.rule(
        Bid::new(2, Strain::Notrump),
        50,
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
        115,
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
            100,
            direct_bust.clone() & longer_diamonds(),
        )
        .rule(
            Bid::new(2, Strain::Clubs),
            100,
            direct_bust & !longer_diamonds(),
        );

    // Opener passes partner's runout: responder ran because it is weak, so it
    // captains the auction.  Weight outranks the natural raise *and* the 1.5
    // transfer completion — without it a 2♦/2♥ escape is misread as a Jacoby
    // transfer and opener "completes" it into responder's short suit.
    // ponytail: always pass; pulling to a better suit on a misfit is deferred.
    rules = rules.rule(
        Call::Pass,
        155,
        one_nt_runout_enabled() & opener_after_one_nt_runout(),
    );

    // Opener answers partner's 2NT minors-scramble with the better minor (longer,
    // ties to diamonds).  Weight outranks the 1.5 transfer completion — the floor
    // reads 2NT as a diamond transfer, which would force a club-longer hand to 3♦.
    rules = rules
        .rule(
            Bid::new(3, Strain::Diamonds),
            160,
            one_nt_runout_enabled() & opener_after_one_nt_minors() & longer_diamonds(),
        )
        .rule(
            Bid::new(3, Strain::Clubs),
            160,
            one_nt_runout_enabled() & opener_after_one_nt_minors() & !longer_diamonds(),
        );

    // Universal runout, opener's balancing seat (`set_one_nt_runout_universal`).
    // The double came back to opener with a weak partner (it had no escape), so
    // 1NT-X rates to fail: opener runs its own five-plus-card suit rather than
    // sit — but only minimum-ish, since a maximum still rates to make 1NT-X.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
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
                one_nt_runout_enabled()
                    & one_nt_runout_universal()
                    & opener_balancing_runout()
                    & len(suit, 5..)
                    & hcp(..17),
            )
            .rule(
                Bid::new(2, strain),
                110 + major_bonus,
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
        100,
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
            5
        } else {
            0
        };
        for (length, weight) in [(4usize, 100i16), (5, 110), (6, 120)] {
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
        155,
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
            160,
            one_nt_runout_enabled()
                & penalize_escape_stack_enabled()
                & opp_escaped_our_nt_undoubled()
                & doubled_suit_stack(),
        )
        .rule(
            Call::Double,
            160,
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
        155,
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
        155,
        penalty_latched_c() & latch_penalty_c() & advancing_a_double(),
    );

    // UvU encircling: the opponents ran from our 1NT (2NT) X.  Double their
    // escape with a trump stack — and keep doubling as they keep running — by
    // agreement; partner leaves in.  Mirrors the doubled-1NT escape chase above,
    // gated on its own A/B knob ([`set_uvu_encircle`]), independent of the runout.
    rules = rules
        .rule(
            Call::Double,
            160,
            uvu_encircle_enabled() & opp_escaped_our_uvu_undoubled() & doubled_suit_stack(),
        )
        .rule(
            Call::Pass,
            155,
            uvu_encircle_enabled() & leave_in_uvu_penalty(),
        );

    for level in 1u8..=4 {
        // Forced: the notrump escape guarantees an action — no fit, no
        // stopper, no four-card suit outside theirs still has a call.
        rules = rules.rule(
            Bid::new(level, Strain::Notrump),
            30,
            advancing_a_double() & min_level_is(level, Strain::Notrump),
        );
    }

    for level in 1u8..=3 {
        // Notrump overcall: 15–18 balanced with their suits stopped.
        rules = rules.rule(
            Bid::new(level, Strain::Notrump),
            105,
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
            150,
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
    // The strength-independent game forces (a strong-notrump responder past
    // invitation, a strong 2♣, an auction already forced to game).  Split out so
    // each game rule can price its own strand — the fitted major-game rule swaps
    // the plain `combined_points(25)` for the fit-length-adjusted
    // [`fit_sum_game`], and every milestone wraps its points in [`points_or_net`]
    // (the bilans knob prices that contract's own strain and trick target) —
    // while these forces still apply untouched.
    let game_forces = (partner_strong_notrump(1)
        & (hcp(10..) | (hcp(nt_responder_game_floor()..) & undisturbed())))
        | (partner_strong_notrump(2) & hcp(5..))
        | auction_forces_game();
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        140,
        (game_forces.clone()
            | points_or_net(
                combined_hcp(25),
                combined_hcp(25 - COLLAR_SLACK),
                Strain::Notrump,
                9,
            ))
            & not_penalizing()
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
                145,
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
        let known_minor_fit = known_eight_card_fit(minor);
        rules = rules.rule(
            Bid::new(5, strain),
            142,
            (game_forces.clone()
                | points_or_net(
                    combined_points(25),
                    combined_points(25 - COLLAR_SLACK),
                    strain,
                    11,
                ))
                & not_penalizing()
                & below_game()
                & inference_aware()
                & known_minor_fit
                & !stopper_in_their_suits()
                & level_available(5, strain),
        );
    }
    for major in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(major);
        // A *known* eight-card major fit outranks 3NT: our length plus partner's
        // shown floor reaches eight (a transferred suit jumped or raised to game,
        // a jump-shift's four opposite our four — see [`known_eight_card_fit`]).
        // The shown lengths come from the auction interpretation, so this fires
        // only on a fit the calls have promised.
        let known_major_fit = known_eight_card_fit(major);
        rules = rules.rule(
            Bid::new(4, strain),
            145,
            (game_forces.clone()
                | points_or_net(
                    combined_points(25),
                    combined_points(25 - COLLAR_SLACK),
                    strain,
                    10,
                ))
                & not_penalizing()
                & below_game()
                & len(major, 6..)
                & level_available(4, strain),
        );
        // Preemptive 4M over a double of our 1NT (opt-in; `set_preempt_4m_over_double`).
        // The major's mirror of the gambling 3NT: a *quality* long (6+) major —
        // semi-solid and headed by the trump ace (a sure trump trick that buffs total
        // tricks) — on a modest hand, partly preemptive and partly to make opposite the
        // strong notrump.  Natural (the bid major reads as 6+), so unalerted; the
        // game-values arm above still governs undisturbed and over an overcall.
        rules = rules.rule(
            Bid::new(4, strain),
            145,
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
        // A ninth or tenth trump counts as points here: `fit_sum_game` reaches
        // game once own points + partner's shown floor + the combined trump length
        // clear the [`FIT_SUM_GAME`] threshold (default 31).
        rules = rules.rule(
            Bid::new(4, strain),
            150,
            (game_forces.clone()
                | points_or_net(
                    fit_sum_game(major, 0),
                    fit_sum_game(major, COLLAR_SLACK),
                    strain,
                    10,
                ))
                & not_penalizing()
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
            150,
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
            165,
            points_and_net(combined_points(33), strain, 12)
                & not_penalizing()
                & below_slam()
                & inference_aware()
                & known_major_fit.clone()
                & level_available(6, strain),
        );
        rules = rules.rule(
            Bid::new(7, strain),
            175,
            points_and_net(combined_points(37), strain, 13)
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
            160,
            points_and_net(combined_hcp(33), Strain::Notrump, 12)
                & not_penalizing()
                & below_slam()
                & stopper_in_their_suits()
                & level_available(6, Strain::Notrump),
        )
        .rule(
            Bid::new(7, Strain::Notrump),
            170,
            points_and_net(combined_hcp(37), Strain::Notrump, 13)
                & not_penalizing()
                & below_slam()
                & stopper_in_their_suits()
                & level_available(7, Strain::Notrump),
        );

    // ------------------------------------------------------------------
    // M6.4: slam machinery on the floor — RKCB 1430 (instinct decoding
    // instinct on both sides) and control-bid signoffs.
    // ------------------------------------------------------------------
    // The ask: with a known eight-card fit and combined small-slam values,
    // ask for keycards before committing — outweighing the direct milestone
    // slams (1.65), because RKCB's point is staying *out* of the
    // two-keycards-missing slam.  Not over partner's notrump bid (partner
    // would read that 4NT quantitative), mirroring the answerer's gate; the
    // grand-zone 37 keeps bidding sevens directly (1.75 outweighs the ask).
    // The ask must also be *decodable*: partner proves the eight from their
    // hand plus our shown floor, and their hand is at least their own shown
    // floor — so an eight provable on the table (the two floors sum to
    // eight) is decodable by construction; otherwise the hand-independent
    // face must key the same trump, the fiat every seat derives identically
    // (an 8-count fit against a four-card Puppet answer used to be known
    // only to us, and round 2 lost 11 IMPs a board on the passed-out ask —
    // the face rung reads that lane now, and the book's quantitative 4NT
    // shadows the 1NT lanes anyway).  Only a beyond-doubt trump converts
    // 4NT from the dual-exit quant — six-of-major or the misfit 6NT, both
    // still open — into an ask that forfeits the notrump exit.
    rules = rules
        .rule(
            Bid::new(4, Strain::Notrump),
            168,
            pred(|hand: Hand, context: &Context<'_>| {
                floor_rkcb_now()
                    && context.undisturbed()
                    && keycard_trump(hand, context).is_some_and(|trump| {
                        let inferences = context.inferences();
                        let partner = inferences.partner().length(trump).min;
                        let on_table = inferences.me().length(trump).min + partner;
                        // A fit the fit-sum machinery refuses to play is no
                        // RKCB trump either (the measured flat-4333 carve of
                        // `known_eight_card_fit`) — the initiation site only;
                        // an answerer's flatness is irrelevant once asked.
                        !bare_four_four_own_flat(hand, trump, usize::from(partner))
                            && (on_table >= 8
                                || face_trump(context.auction(), context.auction().len())
                                    == Some(trump))
                    })
                    && partner_last_call(context.auction())
                        .is_none_or(|bid| bid.strain != Strain::Notrump)
            }) & inference_aware()
                // The strength floor that makes the decode exact (jdh8's
                // doctrine): a side without slam-seeking values cannot
                // assume three combined keycards, and outside combined
                // 3..=5 the 1430 steps are guesses — the round-1 A/B's
                // worst boards were ~26-27-combined asks over limited
                // raises, decoded high and six off two.  The bilans entry
                // prices tricks; this floor prices the *conversation*.
                & combined_points(29)
                & slam_entry_reached()
                & not_penalizing()
                & below_slam()
                & level_available(4, Strain::Notrump),
        )
        .alert(RKCB_FLOOR);
    // The 1430 answers — forcing, so they outweigh every milestone.  The rung
    // is the *step above the ask*, so one rule per landing call serves every
    // ask: over a plain 4NT the four steps are 5♣/5♦/5♥/5♠, exactly the rules
    // the floor has always carried, in the order it carried them.  Step 1 also
    // covers all five keycards (a 2♣ rock answering its raiser's ask; the book
    // ladder's {1,4} left that hand with *no* answer and round 3 passed the
    // ask out).
    //
    // The knob gates rule *presence*, not just the constraint: the reading's
    // `alerted` test is structural — it asks whether any rule on the made call
    // carries an alert (evaluating only the face gate) — so an always-present
    // alerted rule on 4♥/4♠ would suppress the natural reading of every
    // floor-classified 4♥/4♠ even with kickback off.  Build one stance per arm.
    //
    // ROPI's two-keycard step and DOPI's ride the same landing set: both are
    // "the cheapest bid", one counted from the ask and one from their
    // interference, and their constraints reject every landing that is not the
    // right one.  Off the knob that leaves 5♣ ROPI and 5♦/5♥/5♠ DOPI — the
    // rules the floor has always carried, plus two that can never fire (a 5♣
    // DOPI step would need their bid to *be* the 4NT ask).
    if relocating_now() {
        // The relocated arms' landings include the 4-level, where the alert
        // collides with natural games — so each rule is face-gated on its
        // recognizer's face half: on faces where no ask window is live the
        // rule is as-if-absent and the natural reading of 4♥/4♠/4NT stands
        // (the §7.3.1 union poison).  Redwood rides the same answer set; the
        // ladder's claim scope is what narrows its lanes.  The plain arm
        // below stays ungated — byte-identical to the shipped default.
        for &landing in &KICKBACK_ANSWERS {
            rules = rules
                .rule(landing, 190, keycard_answer(landing))
                .alert(RKCB_FLOOR)
                .shared_face(FACE_RKCB_ANSWER, |context: &Context<'_>| {
                    keycard_asked_face(context).is_some()
                })
                .rule(landing, 192, ropi_step(landing))
                .alert(RKCB_FLOOR)
                .shared_face(FACE_RKCB_ROPI, |context: &Context<'_>| {
                    context.auction().last() == Some(&Call::Double)
                        && keycard_asked_face(context).is_some()
                })
                .rule(landing, 190, dopi_step(landing))
                .alert(RKCB_FLOOR)
                .shared_face(FACE_RKCB_DOPI, |context: &Context<'_>| {
                    keycard_asked_over_bid_face(context).is_some()
                });
        }
    } else {
        // The plain arm's gates: the same §7.3.1 cure reaches the default
        // system's five-level answers, confining them to a live ask window.
        for &landing in &PLAIN_ANSWERS {
            rules = rules
                .rule(landing, 190, keycard_answer(landing))
                .alert(RKCB_FLOOR)
                .shared_face(FACE_RKCB_ANSWER, answer_window_face)
                .rule(landing, 192, ropi_step(landing))
                .alert(RKCB_FLOOR)
                .shared_face(FACE_RKCB_ROPI, ropi_window_face)
                .rule(landing, 190, dopi_step(landing))
                .alert(RKCB_FLOOR)
                .shared_face(FACE_RKCB_DOPI, dopi_window_face);
        }
    }
    // The relay: the queen ask one step above partner's
    // 1430 answer, its merged reply, then the second relay and its two rungs —
    // all derived from the answer by [`relay_map`] and [`king_relay`], so one
    // rule per landing call serves every trump and every ask position.
    //
    // These landings are the ordinary contracts of the game and the `alerted`
    // bit is structural, so every rule here carries its window's face gate: on
    // faces with no relay in motion the rule is as-if-absent and 5♥ reads as
    // hearts again.  (Was `if queen_ask_now()`; the relay is unconditional now
    // — BBA relays for the queen with no toggle, so a floor distilled from it
    // asks whatever we do, and an off-arm would bid what it cannot read.)
    {
        for &landing in &RELAY_RUNGS {
            // The artificial half of the merged reply — the king rungs and
            // 5NT.  The denials are five and six of the agreed trump:
            // contracts, not codes, so they carry no alert.  Every class is
            // installed only where some lane's geometry can land it — a
            // constraint-dead alerted rule would still be face-live, and its
            // structural `alerted` bit would erase the natural reading of the
            // most common placements in the window ([`relay_lanes`]).
            if artificial_reply_can_land(landing) {
                rules = rules
                    .rule(landing, 190, queen_reply(landing, true))
                    .alert(RKCB_FLOOR)
                    .shared_face(rkcb_relay_face(6), |context: &Context<'_>| {
                        relay_window_face(context, 6)
                    });
            }
            if denial_can_land(landing) {
                rules = rules
                    .rule(landing, 190, queen_reply(landing, false))
                    .shared_face(rkcb_relay_face(6), |context: &Context<'_>| {
                        relay_window_face(context, 6)
                    });
            }
            // The second relay's rungs: "one more king" is a code, "none"
            // is six of the agreed trump and places the contract.
            if king_reply_can_land(landing, true) {
                rules = rules
                    .rule(landing, 190, king_reply(landing, true))
                    .alert(RKCB_FLOOR)
                    .shared_face(rkcb_relay_face(10), |context: &Context<'_>| {
                        relay_window_face(context, 10)
                    });
            }
            if king_reply_can_land(landing, false) {
                rules = rules
                    .rule(landing, 190, king_reply(landing, false))
                    .shared_face(rkcb_relay_face(10), |context: &Context<'_>| {
                        relay_window_face(context, 10)
                    });
            }
            if queen_ask_can_land(landing) {
                rules = rules
                    .rule(
                        landing,
                        185,
                        queen_ask_here(landing) & one_keycard_missing() & slam_entry_reached(),
                    )
                    .alert(RKCB_FLOOR)
                    .shared_face(rkcb_relay_face(4), |context: &Context<'_>| {
                        relay_window_face(context, 4)
                    });
            }
        }
    }
    rules = rules
        // ROPI over their double of the ask — classic R0P1, outweighing the
        // 1430 answers (whose quiet window tolerates the double) so the
        // doubled ask answers in scheme: redouble 0, pass 1, step 1 is 2.
        .rule(Call::Redouble, 192, ropi_answer(&[0, 3]))
        .alert(RKCB_FLOOR)
        .shared_face(FACE_RKCB_ROPI, ropi_window_face)
        .rule(Call::Pass, 192, ropi_answer(&[1, 4]))
        .alert(RKCB_FLOOR)
        .shared_face(FACE_RKCB_ROPI, ropi_window_face)
        // DOPI over their bid below five of trump — classic D0P1: double 0,
        // pass 1, the cheapest step 2.  The machinery used to stand down on
        // their bid over the ask (the card declared DOPI with nothing
        // behind it); these rungs are that window's authored floor.
        .rule(Call::Double, 190, dopi_answer(&[0, 3]))
        .alert(RKCB_FLOOR)
        .shared_face(FACE_RKCB_DOPI, dopi_window_face)
        .rule(Call::Pass, 190, dopi_answer(&[1, 4]))
        .alert(RKCB_FLOOR)
        .shared_face(FACE_RKCB_DOPI, dopi_window_face)
        // DEPO at or above five of trump: no room for steps — double even,
        // pass odd.
        .rule(Call::Double, 190, depo_answer(true))
        .alert(RKCB_FLOOR)
        .shared_face(FACE_RKCB_DOPI, dopi_window_face)
        .rule(Call::Pass, 190, depo_answer(false))
        .alert(RKCB_FLOOR)
        .shared_face(FACE_RKCB_DOPI, dopi_window_face)
        // After our answer the asker holds the count: respect the placement.
        .rule(Call::Pass, 188, respect_keycard_signoff());
    // The relocated ask (`set_rkcb_variant`): the cheapest unguarded suit above
    // the trump, so every 1430 answer lands at or below five of trump instead
    // of blowing past it — 4♦ and 4♥ over the minors are Redwood, 4♠ over
    // hearts is the Kickback proper.  Same gate as the 4NT ask, keyed on the
    // face-only [`kickback_ladder`] instead of a derived trump, and a notch
    // heavier so the asker takes the cheaper ask where the ladder offers one.
    // 4NT keeps its own meaning: kickback *adds* asks, it never removes one.
    //
    // Rule *presence* is what the knob gates, not just the constraint: the
    // reading's `alerted` test is structural, so an always-present alerted
    // rule on 4♠ would suppress the natural reading of every floor-classified
    // 4♠ even with the knob off.  Build one stance per arm.  Within the arm,
    // the face-only conjuncts live in the `Rules::face` gate — `Rule::eval`
    // consults it, so the bidder is unchanged, and the reader skips the rule
    // on faces where the ladder claims nothing, so a natural 4♠ keeps its
    // natural reading (the §7.3.1 union poison).
    if relocating_now() {
        for target in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let strain = Strain::from(target);
            rules = rules
                .rule(
                    Bid::new(4, strain),
                    169,
                    pred(move |hand: Hand, context: &Context<'_>| {
                        let auction = context.auction();
                        keycard_trump(hand, context).is_some_and(|trump| {
                            let inferences = context.inferences();
                            let partner = inferences.partner().length(trump).min;
                            let on_table = inferences.me().length(trump).min + partner;
                            kickback_ladder(auction, auction.len())[target as usize] == Some(trump)
                                && !bare_four_four_own_flat(hand, trump, usize::from(partner))
                                && (on_table >= 8
                                    || face_trump(auction, auction.len()) == Some(trump))
                        })
                    }) & inference_aware()
                        & combined_points(29)
                        & slam_entry_reached()
                        & not_penalizing()
                        & below_slam()
                        & level_available(4, strain),
                )
                .alert(RKCB_FLOOR)
                .shared_face(
                    rkcb_relocated_ask_face(target),
                    move |context: &Context<'_>| {
                        let auction = context.auction();
                        relocating_now()
                            && context.undisturbed()
                            && kickback_ladder(auction, auction.len())[target as usize].is_some()
                            && partner_last_call(auction)
                                .is_none_or(|bid| bid.strain != Strain::Notrump)
                    },
                );
        }
    }
    // The asker's continuations, per possible trump.
    for trump in Suit::ASC {
        let strain = Strain::from(trump);
        rules = rules
            // All five keycards and grand-zone strength: bid seven — unless
            // the relay is live, in which case the blast stands down and lets
            // the conversation find out.  Seven off a missing trump queen is
            // the loss the relay exists to prevent, and a round of bidding is
            // cheap against it.  [`relay_pending`] is false wherever the
            // relay has no room, so the blast is unchanged in those lanes.
            .rule(
                Bid::new(7, strain),
                186,
                keycard_total(trump, 5..)
                    & points_and_net(combined_points(37), strain, 13)
                    & !relay_pending(trump)
                    & level_available(7, strain),
            )
            // At most one keycard missing AND the values that justify an
            // ask: small slam.  The entry gate re-checks what the ladder's
            // own ask verified before bidding 4NT — but the *net* also asks
            // (contested lanes, where its 4NT is judgement), and decoding an
            // unvetted ask into an automatic six converts every frivolous
            // 4NT into a slam; without the entry, the signoff below wins.
            //
            // [`queen_ok`] is what makes this "four keycards **and** the
            // queen": five keycards bid six whatever the queen does, a settled
            // queen bids it, and where the relay had no room the asker bets it
            // on four exactly as before.
            .rule(
                Bid::new(6, strain),
                184,
                keycard_total(trump, 4..)
                    & slam_entry_reached()
                    & queen_ok(trump)
                    & level_available(6, strain),
            )
            // Sign off at five while it is still available — the catch-all
            // placement (weighted below the vetted six, so "one missing and
            // entry values" still bids the slam)...
            .rule(
                Bid::new(5, strain),
                182,
                keycard_total(trump, ..) & level_available(5, strain),
            )
            // ...pass when partner's answer already is five of the trump...
            .rule(
                Call::Pass,
                180,
                keycard_total(trump, ..) & answer_is_five_of(trump),
            )
            // ...and with no room below slam (a cramped minor, or a 5♠ answer
            // over hearts) accept six rather than strand the phantom answer —
            // the book's `no_room_six` policy.
            .rule(
                Bid::new(6, strain),
                30,
                keycard_total(trump, ..) & level_available(6, strain),
            )
            // The cramped doubled answer: their X sits on partner's answer
            // past five of trump — we never play a suit we have no fit in,
            // so the doubled artificial suit is always escaped (`… 4♦ - 4NT
            // - 5♥ (X)` passed out on a 4-1, −20).  Below the true signoffs
            // (five of trump still available bids it at 1.82; the answer
            // itself our trump sits at 1.80), the rungs order the landing
            // spots by trust: the hand-seen fit, a stopped 5NT a level
            // cheaper, another seen fit (below, outside the loop), and the
            // derived trump as the last resort.
            .rule(
                Bid::new(6, strain),
                173,
                keycard_total(trump, ..)
                    & answer_doubled()
                    & known_eight_card_fit(trump)
                    & level_available(6, strain),
            )
            .rule(
                Bid::new(5, Strain::Notrump),
                172,
                keycard_total(trump, ..)
                    & answer_doubled()
                    & stopper_in_their_suits()
                    & level_available(5, Strain::Notrump),
            )
            .rule(
                Bid::new(6, strain),
                170,
                keycard_total(trump, ..) & answer_doubled() & level_available(6, strain),
            );
        // The asker's continuations one and two rounds further on — after the
        // queen reply, and after the king reply.  Same shape and same weights
        // as the direct placements above, because the decision is the same
        // decision with one more fact in it.  All natural contracts, so no
        // alert and no face gate; the artificial rungs they answer carry both.
        {
            for &landing in &RELAY_RUNGS {
                // All five keycards: six is already decided, so the queen
                // is only worth a round when seven is live — and seven is
                // live only on grand-zone values.  Without them this rule
                // is dead and the asker simply bids the slam, which is the
                // whole point of asking only what you will act on.  Both
                // classes install only on the rungs some lane can reach —
                // a dead alerted rule erases readings ([`relay_lanes`]).
                if queen_ask_can_land(landing) {
                    rules = rules
                        .rule(
                            landing,
                            185,
                            queen_ask_here(landing)
                                & keycard_total(trump, 5..)
                                & grand_zone(strain),
                        )
                        .alert(RKCB_FLOOR)
                        .shared_face(rkcb_relay_face(4), |context: &Context<'_>| {
                            relay_window_face(context, 4)
                        });
                }
                // Explore seven only when the values are already there:
                // RKCB is a slam veto, not a slam seeker, so a partnership
                // short of the grand zone never spends the round.
                if king_ask_can_land(landing) {
                    rules = rules
                        .rule(
                            landing,
                            185,
                            king_ask_here(landing)
                                & relay_verdict(trump, 5.., true)
                                & grand_zone(strain),
                        )
                        .alert(RKCB_FLOOR)
                        .shared_face(rkcb_relay_face(8), |context: &Context<'_>| {
                            relay_window_face(context, 8)
                        });
                }
            }
            rules = rules
                // Two of the three side kings on top of all five keycards and
                // the queen: bid seven.  The merged reply can show them by
                // itself — partner named its cheapest king and we hold one —
                // which is the round the second relay no longer has to spend.
                .rule(
                    Bid::new(7, strain),
                    186,
                    relay_verdict(trump, 5.., true)
                        & kings_so_far(trump, 2)
                        & grand_zone(strain)
                        & level_available(7, strain),
                )
                // ...or the second relay found the second one.
                .rule(
                    Bid::new(7, strain),
                    186,
                    king_total(trump, 2..) & level_available(7, strain),
                )
                // The queen came back (or five keycards made it moot): six.
                .rule(
                    Bid::new(6, strain),
                    184,
                    relay_verdict(trump, 4.., true) & level_available(6, strain),
                )
                // The second relay answered short of two kings — six.
                .rule(
                    Bid::new(6, strain),
                    184,
                    king_total(trump, ..) & level_available(6, strain),
                )
                // The queen came back denied on four keycards — one keycard
                // *and* the queen missing.  Stop at five while we still can...
                .rule(
                    Bid::new(5, strain),
                    182,
                    relay_verdict(trump, .., false) & level_available(5, strain),
                )
                // ...and pass when partner's reply already **is** the contract.
                // The merged ladder puts both denials on the agreed trump — five
                // of it flat, six of it with the fit or the void we could not see
                // — and the second relay's "no more kings" on six as well, so
                // every one of them is a place to play rather than a rung to
                // rescue.
                .rule(Call::Pass, 180, relay_verdict(trump, .., false))
                .rule(Call::Pass, 180, king_total(trump, ..));
        }
        // Fleeing the face-derived trump to a fit we can actually see: the
        // face's agreement rule is both-bid-it, not eight cards, so a seen
        // eight-card fit elsewhere outranks an unseen trump (1.71 sits
        // between the seen-fit six above and the last-resort six).  Never
        // into the answer's own suit: its floor in the live reading is the
        // answer's natural mis-read (the very pollution [`answer_trump`]
        // corroborates away), not a fit.
        for other in Suit::ASC {
            if other == trump {
                continue;
            }
            let other_strain = Strain::from(other);
            rules = rules.rule(
                Bid::new(6, other_strain),
                171,
                keycard_total(trump, ..)
                    & answer_doubled()
                    & !answer_is_five_of(other)
                    & known_eight_card_fit(other)
                    & !known_eight_card_fit(trump)
                    & level_available(6, other_strain),
            );
        }
    }
    // Never pass out partner's control bid — it agrees a suit and forces.
    // Return to the agreed suit at the cheapest level; with slam-zone values
    // the ask above (or the direct milestones) outweighs this signoff, and
    // the control bidder keeps captaining over it.
    for trump in Suit::ASC {
        let strain = Strain::from(trump);
        for level in 4..=6 {
            rules = rules.rule(
                Bid::new(level, strain),
                155,
                partner_control_bid(trump)
                    & inference_aware()
                    & known_eight_card_fit(trump)
                    & min_level_is(level, strain),
            );
        }
    }

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
                135,
                rubens_transfer(source, false)
                    & len(target, 5..)
                    & points(10..)
                    & min_level_is(2, source_strain),
            )
            .rule(
                Bid::new(2, source_strain),
                145,
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
            145,
            rubens_cue_raise(cue) & support(3..) & points(10..) & min_level_is(2, cue_strain),
        );
    }
    // Complete partner's transfer into the suit just above it — mechanical, like
    // completing a transfer over our own notrump.
    for target in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(2, Strain::from(target)),
            155,
            rubens_completes(target),
        );
    }
    // Grade the into-partner completion: the transfer showed a limit-plus
    // raise (10+ with support), so the completion `2Y` denies extras, a 13–14
    // hand super-accepts `3Y` (an invite the raiser carries on with 13+, via
    // the raise ladder), and a 15+ maximum places the game outright — the
    // acceptance that cashes the transfer's separation over a flat natural
    // raise (cf. the 1NT invite-acceptance win).  The minor maximum is `3NT`
    // behind a stopper; only diamonds can be an into-partner target at the one
    // level (nothing transfers into clubs).
    // ponytail: help-suit trials, a minor super-accept, and rebids after a
    // NEW-SUIT transfer are the named ceiling — author them if the A/B still trails.
    for y in [Suit::Hearts, Suit::Spades] {
        rules = rules
            .rule(
                Bid::new(4, Strain::from(y)),
                160,
                rubens_into_partner(y) & points(15..),
            )
            .rule(
                Bid::new(3, Strain::from(y)),
                158,
                rubens_into_partner(y) & points(13..15),
            );
    }
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        160,
        rubens_into_partner(Suit::Diamonds) & points(15..) & stopper_in_their_suits(),
    );
    // The overcaller breaks a NEW-SUIT completion, too, exactly when it would
    // have bid over a natural non-forcing `2 target`: the fit raise with
    // values, graded like the into-partner break (invite 13–14, game 15+; the
    // diamond maximum is `3NT` behind a stopper).
    for target in [Suit::Diamonds, Suit::Hearts] {
        rules = rules.rule(
            Bid::new(3, Strain::from(target)),
            158,
            rubens_new_suit_completion(target) & len(target, 3..) & points(13..15),
        );
    }
    rules = rules
        .rule(
            Bid::new(4, Strain::from(Suit::Hearts)),
            160,
            rubens_new_suit_completion(Suit::Hearts) & len(Suit::Hearts, 3..) & points(15..),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            160,
            rubens_new_suit_completion(Suit::Diamonds) & points(15..) & stopper_in_their_suits(),
        );

    // The raiser's rebid over the minimum completion: extras beyond the shown
    // ten drive to game opposite even a minimum overcall.
    for y in [Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(4, Strain::from(y)),
            150,
            rubens_raiser_rebids(y) & points(14..),
        );
    }
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        150,
        rubens_raiser_rebids(Suit::Diamonds) & points(14..) & stopper_in_their_suits(),
    );
    // The new-suit transferee's rebid: the transfer was wide yet unlimited —
    // it subsumes both the non-forcing and the forcing natural treatments,
    // because the rest/continue split happens *after* the cheap completion.  A
    // mild hand has already rested; 12–13 re-raises the suit as the invite;
    // 14+ clarifies to game — the six-card major, or `3NT` behind a stopper.
    rules = rules.rule(
        Bid::new(4, Strain::from(Suit::Hearts)),
        152,
        rubens_transferee_rebids(Suit::Hearts) & len(Suit::Hearts, 6..) & points(14..),
    );
    for target in [Suit::Diamonds, Suit::Hearts] {
        rules = rules
            .rule(
                Bid::new(3, Strain::Notrump),
                150,
                rubens_transferee_rebids(target) & points(14..) & stopper_in_their_suits(),
            )
            .rule(
                Bid::new(3, Strain::from(target)),
                150,
                rubens_transferee_rebids(target) & len(target, 6..) & points(12..14),
            );
    }

    // Answer partner's two-level cue-raise — the cue must never play their
    // suit.  Retreat to our overcall suit as the guaranteed action; with a
    // maximum (14+, opposite the cue's 10+) place the game instead: `4♥` on
    // the heart fit, `3NT` over a minor with their suit stopped.
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        rules = rules.rule(Bid::new(3, Strain::from(y)), 150, rubens_cue_answers(y));
    }
    rules = rules.rule(
        Bid::new(4, Strain::from(Suit::Hearts)),
        155,
        rubens_cue_answers(Suit::Hearts) & points(14..),
    );
    for y in [Suit::Clubs, Suit::Diamonds] {
        rules = rules.rule(
            Bid::new(3, Strain::Notrump),
            155,
            rubens_cue_answers(y) & points(14..) & stopper_in_their_suits(),
        );
    }
    // Knob-off natural new-suit advance — the fair baseline for the Rubens A/B
    // (see `natural_new_suit_advance`); dormant while the transfers are on.
    for target in [Suit::Diamonds, Suit::Hearts] {
        let target_strain = Strain::from(target);
        rules = rules.rule(
            Bid::new(2, target_strain),
            135,
            natural_new_suit_advance(target)
                & len(target, 5..)
                & points(10..)
                & min_level_is(2, target_strain),
        );
    }

    // Competitive long-suit rebid (opt-in; see `set_competitive_rebid`).  Once
    // `we_have_not_bid` is false the floor competes only by raising partner or
    // doubling, so a hand with a suit of its own — the opener's rebiddable
    // six-bagger, an overcaller's — is stuck doubling.  Rebid that suit at the
    // cheapest legal level, outranking the 0.9 takeout double below, and the
    // existing raise ladder carries responder to game.  The natural reading
    // already reads a repeated suit as 6+ (`inference.rs`).
    //
    // Capped at the three level: over their three-level bid the cheapest rebid of
    // a lower-ranking suit is game (4♣ over 3♦, 4♥ over 3♠), and this rule carries
    // no strength — a minimum must not blast game unilaterally.
    //
    // The A/B split the two live levels sharply: the 2-level rebid (balancing
    // seat, low overcaller rebids) is a clean win at both vulnerabilities on both
    // scorers, so it stays unconditional; the blanket 3-level rebid lost —
    // marginal non-vul, clearly negative vulnerable under perfect defense — so it
    // now demands a genuine source of tricks: seven cards, or a good six (two of
    // the top three honors).  A ragged six-bagger competing to the three level
    // stays home (double or pass).
    if competitive_rebid_enabled() {
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let strain = Strain::from(suit);
            rules = rules.rule(
                Bid::new(2, strain),
                100,
                their_live_bid_at_most(3)
                    & i_bid_suit(suit)
                    & min_level_is(2, strain)
                    & len(suit, 6..)
                    & !they_bid(strain)
                    & not_penalty_latched(),
            );
            rules = rules.rule(
                Bid::new(3, strain),
                100,
                their_live_bid_at_most(3)
                    & i_bid_suit(suit)
                    & min_level_is(3, strain)
                    & len(suit, 6..)
                    & (len(suit, 7..) | top_honors(suit, 2..))
                    & !they_bid(strain)
                    & not_penalty_latched(),
            );
        }
    }

    // Opener's notrump actions in a contested auction the floor otherwise
    // passes out (author both sides).  A suit opener who is balanced is bimodal
    // — 15-17 opens 1NT, 20-21 opens 2NT — so the balanced 18-19 surfaces here
    // wanting game and, until now, could only make a lone takeout double.
    //
    //  * Reopening 1NT (`1X (1Y) - -`): partner passed the overcall, back to
    //    us — natural, their suit stopped, the invite-to-game a takeout double
    //    of a balanced hand cannot make.  Outranks the 0.9 double below.
    //  * 3NT over responder's free 1NT (`1X (1Y) 1NT -`): responder already
    //    promised 6-10 with a stopper, so a balanced 18-19 raises to game.
    //  * Responder raises the reopening 1NT to game with the trapped values.
    if reopening_notrump_enabled() {
        rules = rules
            .rule(
                Bid::new(1, Strain::Notrump),
                95,
                opener_reopening()
                    & min_level_is(1, Strain::Notrump)
                    & balanced()
                    & stopper_in_their_suits()
                    & hcp(18..=19)
                    & not_penalty_latched(),
            )
            .rule(
                Bid::new(3, Strain::Notrump),
                100,
                opener_over_free_1nt() & balanced() & hcp(18..=19) & not_penalty_latched(),
            )
            .rule(
                Bid::new(3, Strain::Notrump),
                100,
                responder_over_reopening_1nt() & hcp(6..),
            );
    }

    // Takeout double of their low suit bid: shape with opening values, or
    // any strong hand planning to bid again.  The penalty latch steps these
    // aside — once we own the auction for penalty, a double is not takeout.
    rules
        .rule(
            Call::Double,
            90,
            their_live_bid_at_most(3)
                & short_in_their_suits()
                & hcp(12..)
                & not_penalty_latched()
                & takeout_double_shape_ok()
                & !minimum_reraise_blocked(),
        )
        .rule(
            Call::Double,
            80,
            their_live_bid_at_most(3) & points(17..) & not_penalty_latched(),
        )
        // Penalty latch: double their runout for penalty on a trump stack instead
        // of takeout on shortness.  Weight matches the runout penalty doubles.
        .rule(
            Call::Double,
            160,
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
            160,
            their_live_bid_at_most(3)
                & penalty_latched_c()
                & latch_optional_c()
                & doubled_suit_len(2..=3)
                & hcp(6..),
        )
}

#[cfg(test)]
mod tests;
