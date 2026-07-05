//! The competitive package over our openings
//!
//! This module builds the [`Competitive`] book that covers contested auctions
//! after our one-level openings: direct-seat responses to their overcall,
//! system-on over their double, support doubles and redoubles for minor
//! openings, and opener's answer to partner's negative double of a two-level
//! minor overcall.

use super::super::constraint::{
    Cons, Constraint, described, has_stopper, hcp, len, min_level_is, points, stopper_in,
    stopper_in_their_suits, suit_hcp, support, they_bid,
};
use super::super::context::Context;
use super::super::fallback::{Fallback, FirstIs, OvercallAtMost, ReplaceNext, guard, rewriter};
use super::super::trie::{Classifier, classifier};
use super::super::{Alert, Competitive, Rules};
use super::notrump::{
    PUPPET, complete_transfer, notrump_minors, notrump_responses, smolen_at_three,
    smolen_completion, stayman_answers, transfer_super_accept,
};
use super::{call, fallback_all_seats};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;
use std::sync::Arc;

// Per-call alerts for the competitive book's artificial calls.  An [`Alert`] marks
// a call as *conventional*: the inference reader decodes it as the convention
// rather than as a natural suit.  Natural raises, natural suit rebids, natural
// notrump, penalty passes, and the catch-all `Pass` stay unalerted.

/// Cue-bid raise — a cue of the opponents' suit as a limit-plus raise of partner's
/// opening (not natural).
const CUE_RAISE: Alert = Alert("comp:cue-raise");
/// Negative double — responder's takeout double showing the unbid suit(s) after
/// partner opens and RHO overcalls.
const NEGATIVE_DOUBLE: Alert = Alert("comp:negative-double");
/// Support double / redouble — opener's `X`/`XX` showing exactly three-card support.
const SUPPORT_DOUBLE: Alert = Alert("comp:support-double");
/// Lebensohl `2NT` — the weak relay to `3♣` over their overcall of our `1NT`.
const LEBENSOHL_RELAY: Alert = Alert("comp:lebensohl-relay");
/// Lebensohl cue — a cue of their suit as game-forcing Stayman.
const LEBENSOHL_CUE: Alert = Alert("comp:lebensohl-cue");
/// Transfer-Lebensohl 3-level transfer — bids the next suit up *through* the
/// adverse suit (INV+).
const LEBENSOHL_TRANSFER: Alert = Alert("comp:lebensohl-transfer");
/// Stayman over `(2♦)` — `3♣` as game-forcing Stayman (with Smolen after the
/// `3♦` denial).
const STAYMAN: Alert = Alert("comp:stayman");
/// Smolen — showing a 5-card major right-sided after the Stayman denial.
const SMOLEN: Alert = Alert("comp:smolen");
/// Leaping Michaels — `4♣`/`4♦` jumps naming a 5-5 game-forcing two-suiter.
const LEAPING_MICHAELS: Alert = Alert("comp:leaping-michaels");
/// Unusual-vs-Unusual cue — `3♣`/`3♦` cues finding a major fit over their
/// both-minors `2NT`.
const UVU_CUE: Alert = Alert("comp:uvu-cue");
/// Unusual-vs-Unusual splinter — `4♣`/`4♦` as a FG+ 5-5-majors splinter into the
/// short minor.
const UVU_SPLINTER: Alert = Alert("comp:uvu-splinter");
/// Stayman re-ask — responder's `XX` after the opponents doubled our 2♣ Stayman
/// and opener passed to deny a club stopper: re-asks the major (forcing).
const STAYMAN_REDOUBLE: Alert = Alert("comp:stayman-redouble");
/// Transfer re-ask — responder's `XX` after the opponents doubled our Jacoby
/// transfer and opener passed to decline: forces opener to complete (forcing).
const TRANSFER_REDOUBLE: Alert = Alert("comp:transfer-redouble");

/// Which Lebensohl package the competitive book carries over our overcalled
/// `1NT` (Section 5)
///
/// Terminology: *Rubensohl* proper makes `2NT` an artificial **club** transfer;
/// the transfer styles here keep the weak `2NT` **relay**, which makes them
/// *Transfer Lebensohl*.
///
/// - `Off` — no Lebensohl node; responder falls to the instinct floor.
/// - `Plain` — weak `2NT` relay / sign-off vs strong direct `3NT` / forcing
///   3-level; matches BBA's 21GF. The prior default (+0.26 IMPs/divergent vs the
///   floor, 200k boards).
/// - `Transfer` — **the default.** Larry Cohen's *Transfer Lebensohl*: 3-level
///   bids transfer up the line *through* the adverse suit, the cue is Stayman, and
///   a transfer to a suit above theirs is INV+ so opener is driven to game (the
///   anti-stranding fix for the earlier transfer-Lebensohl attempt that stranded
///   game hands in partscores). Over `(2♥)`/`(2♠)`/`(2♣)` that is the whole story;
///   it measures **+0.46/+1.24 IMPs/divergent (none/both) vs plain Lebensohl**
///   (`lebensohl-ab`, 200k boards each), and +0.35/+0.05 vs the bare floor. Over
///   `(2♦)` it additionally frees `3♣` for game-forcing Stayman (Smolen after
///   opener's `3♦` denial), reshuffles the 3-level transfers to direct Jacoby
///   (`3♦`→♥, `3♥`→♠, `3♠`→♣ — the `3♠`→♣ leg a forced game-force, its completion
///   `4♣`), and adds Leaping Michaels `4♦` (both majors) / `4♣` (clubs + a major).
///   That `(2♦)` Smolen package is worth **+0.020/+0.024 IMPs/board,
///   +2.286/+2.822 IMPs/divergent (none/both)** over Cohen-pure-over-`(2♦)`
///   (`lebensohl-ab`, 200k filtered each), and it also wins after a takeout double
///   of a weak `2♦` (**+0.014/+0.019 IMPs/board, +1.77/+2.52 IMPs/divergent**,
///   `sohl-after-double-ab`) — which is why the advancer carries it too.
///
/// (True Rubensohl — `2NT` an artificial **club** transfer, low transfers two-way —
/// was implemented and measured worse than `Transfer` (DD `−0.017/−0.046`,
/// perfect-defense `+0.001/−0.023 IMPs/board` none/both) and removed: its only edge
/// was DD-blind right-siding, and jdh8 prefers the Smolen+LM-over-minors /
/// Cohen-over-majors split that `Transfer` carries. See `docs/ai-bidder/21gf-ledger.md`.)
///
/// (An earlier standard-low-Stayman + Smolen hybrid over *both* `(2♦)` and `(2♥)`
/// — no Jacoby reshuffle, no Leaping Michaels — measured DD `−1.31/−1.76 IMPs/div`
/// and was reverted. The narrowed `(2♦)`-only package that `Transfer` now carries
/// *wins*: the Jacoby reshuffle plus Leaping Michaels add genuine fit-finding (5-3
/// major games through Stayman+Smolen, 5-5 major games through Leaping Michaels)
/// that the perfect-defense measure credits — unlike the reverted hybrid, whose
/// only gain was DD-blind right-siding. See `docs/ai-bidder/21gf-ledger.md`.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LebensohlStyle {
    /// Responder falls to the instinct floor (no Lebensohl node)
    Off,
    /// Plain Lebensohl (weak relay vs forcing 3-level) — the prior default
    Plain,
    /// Transfer Lebensohl (Larry Cohen's `2NT`-relay transfers) — the default;
    /// over `(2♦)` it adds `3♣`-Stayman + Smolen, Jacoby transfers
    /// (`3♦`→♥/`3♥`→♠/`3♠`→♣), and Leaping Michaels `4♣`/`4♦`
    Transfer,
}

thread_local! {
    /// Which Lebensohl package the competitive book carries (Section 5).
    static LEBENSOHL_STYLE: Cell<LebensohlStyle> = const { Cell::new(LebensohlStyle::Transfer) };
}

/// Select the Lebensohl package for books built *after* this call (thread-local,
/// read once at book-construction time)
pub fn set_lebensohl_style(style: LebensohlStyle) {
    LEBENSOHL_STYLE.with(|cell| cell.set(style));
}

/// Enable plain Lebensohl (`true` → [`LebensohlStyle::Plain`]) or disable it
/// (`false` → [`LebensohlStyle::Off`])
///
/// Back-compat shim over [`set_lebensohl_style`]; for Transfer Lebensohl call
/// that directly with [`LebensohlStyle::Transfer`].
pub fn set_lebensohl(on: bool) {
    set_lebensohl_style(if on {
        LebensohlStyle::Plain
    } else {
        LebensohlStyle::Off
    });
}

/// The currently selected Lebensohl package
fn lebensohl_style() -> LebensohlStyle {
    LEBENSOHL_STYLE.with(Cell::get)
}

/// The meaning of responder's double of the overcall in `1NT − (overcall) − X`.
///
/// All variants are *authored* in the book (a finite logit), so the instinct
/// floor's own takeout double — whose `hcp(12..)` threshold is too strong here —
/// is shadowed and we control the strength. Opener's continuation is authored to
/// match the style: penalty → `opener_leaves_in_penalty_double` sits; optional →
/// `opener_cooperates_optional` stands on a fit and runs with a doubleton.
/// Gated behind [`set_double_style`]; [`DoubleStyle::Optional`] (2-3/8+) is the
/// default.
///
/// A/B verdict (`ab-lebensohl`, NS vs EW with both pairs Transfer, 200k,
/// ~1500 divergent), once **both** the doubler's partner *and* the takeout
/// baseline are handled fairly: **Optional > Penalty > Takeout**. Optional beats
/// penalty by **+1.59** and takeout by **+2.14 IMPs/divergent**; penalty beats
/// takeout by **+0.51**. The earlier penalty-vs-takeout disagreement (plain DD
/// favored takeout, perfect-defense favored penalty) was an **artifact of opener
/// pulling responder's penalty double** — once opener sits, both measures favor
/// penalty over takeout; once opener also *cooperates* with a 2-3-card optional
/// double (stand on a fit, run with a doubleton) optional wins outright. The
/// ranking is robust to the responder's-double reading. `Takeout`/`Penalty` stay
/// selectable for A/B.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DoubleStyle {
    /// Classic takeout, `len(over, ..=3) & hcp(8..)` (former default; best plain-DD
    /// double only while penalty doubles were pulled — see [`DoubleStyle`]).
    Takeout,
    /// Penalty — length and values in their suit, `len(over, 4..) & hcp(9..)`;
    /// opener sits (see [`set_penalty_double_leave_in`]).
    Penalty,
    /// Penalty at a lower floor: `len(over, 4..) & hcp(7..)`
    PenaltyLight,
    /// Default: cooperative / optional takeout, never short: `len(over, 2..=3) &
    /// hcp(8..)`; opener stands on a fit and runs with a doubleton (see
    /// `opener_cooperates_optional`).
    #[default]
    Optional,
}

thread_local! {
    /// The meaning of responder's double of the overcall (see [`DoubleStyle`]).
    static DOUBLE_STYLE: Cell<DoubleStyle> = const { Cell::new(DoubleStyle::Optional) };
}

/// Select responder's double meaning for books built *after* this call
/// (thread-local, read once at book-construction time)
pub fn set_double_style(style: DoubleStyle) {
    DOUBLE_STYLE.with(|cell| cell.set(style));
}

/// The currently selected double meaning
fn double_style() -> DoubleStyle {
    DOUBLE_STYLE.with(Cell::get)
}

thread_local! {
    /// Whether opener leaves in responder's penalty double of a natural overcall of
    /// our 1NT (`[1NT,(2X),X,(P)]`) instead of letting the floor read `[…,X,P]` as a
    /// takeout advance and pull it. **On by default**; a no-op unless the active
    /// [`DoubleStyle`] is penalty. Read once at book construction. See
    /// [`set_penalty_double_leave_in`] — the A/B knob for the "opener pulls
    /// responder's penalty double" leak (the book dual of the penalty latch).
    static PENALTY_DOUBLE_LEAVE_IN: Cell<bool> = const { Cell::new(true) };
}

/// Toggle opener leaving in responder's penalty double of a natural overcall of our
/// 1NT, for books built *after* this call (thread-local; **on by default**)
///
/// Only matters when the active [`DoubleStyle`] is `Penalty`/`PenaltyLight`: opener
/// sits for `[1NT,(2X),X,(P)]` (defending the doubled overcall) rather than pulling
/// it, since responder's penalty double promised the trumps.  Off restores the bare
/// floor (which reads the double as takeout and advances).
pub fn set_penalty_double_leave_in(on: bool) {
    PENALTY_DOUBLE_LEAVE_IN.with(|cell| cell.set(on));
}

/// Whether opener's penalty-double leave-in is authored
fn penalty_double_leave_in() -> bool {
    PENALTY_DOUBLE_LEAVE_IN.with(Cell::get)
}

/// Opener's reply to responder's **penalty** double of their overcall of our 1NT
/// (`[1NT,(2X),X,(P)]`): always sit and defend, since responder promised length and
/// values in their suit
///
/// A 3NT escape (opener-max with their suit stopped) was A/B'd a clear *loss* vs
/// always sitting (+0.328 vs +0.507 IMPs/divergent on `ab-lebensohl`): defending the
/// doubled overcall beats a fragile notrump game, especially when opener also holds
/// length in their suit — so opener never pulls.
///
/// The book dual of the penalty latch's leave-in: without an authored node here the
/// floor reads `[…,X,P]` as a takeout advance and *pulls* the penalty double (opener
/// is usually short in their suit, so its own length-gated leave-in never fires).
fn opener_leaves_in_penalty_double() -> Rules {
    Rules::new().rule(Call::Pass, 1.5, hcp(0..))
}

/// Opener's reply to responder's **optional** (cooperative) double of their `over`
/// overcall of our 1NT (`[1NT,(2X),X,(P)]`): responder showed only 2-3 cards in
/// their suit, so opener *decides* — stand (defend) with a three-card-plus fit, but
/// **run with a doubleton** to a real five-card suit, escaping a thin defense
///
/// The floor would stand only with four-plus behind their suit and pull everything
/// else, so it runs the three-card fits opener should defend — the optional dual of
/// the penalty-double leak.  Without a five-card suit a short opener has nowhere to
/// run, so it sits (the catch-all `Pass`).
fn opener_cooperates_optional(over: Suit) -> Rules {
    // Stand by default: a fit defends, and a short hand with no suit has no better.
    let mut rules = Rules::new().rule(Call::Pass, 1.5, hcp(0..));
    // Run with a doubleton-or-less to a real five-card suit (cheapest legal level).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == over {
            continue;
        }
        let strain = Strain::from(suit);
        for level in 2..=3 {
            rules = rules.rule(
                Bid::new(level, strain),
                1.6,
                min_level_is(level, strain) & len(over, ..=2) & len(suit, 5..),
            );
        }
    }
    rules
}

thread_local! {
    /// Whether the competitive book carries the Unusual-vs-Unusual structure over
    /// our `1NT` when an opponent overcalls a both-minors `2NT` (Section 5d).
    /// Default on: the constructive cues are DD-robust (A/B +0.6–2.6 IMPs/board
    /// per call vs the passing floor) and the auction was previously unauthored.
    static UVU: Cell<bool> = const { Cell::new(true) };
    /// Responder's penalty-double HCP floor over `1NT − (2NT)`.
    static UVU_X_FLOOR: Cell<u8> = const { Cell::new(9) };
    /// Responder's INV+ cue-bid points floor over `1NT − (2NT)`.
    static UVU_CUE_FLOOR: Cell<u8> = const { Cell::new(8) };
    /// Length floor for responder's weak natural `3♥`/`3♠` escape over `1NT − (2NT)`
    /// (`6` = a clean six-bagger; `5` lets a five-card major escape when defending
    /// the both-minors overcall looks bad — the A/B sweep knob).
    static UVU_NATURAL_FLOOR: Cell<u8> = const { Cell::new(6) };
}

/// Enable the Unusual-vs-Unusual structure over `1NT − (2NT both minors)` for
/// books built *after* this call (thread-local, read once at construction).
///
/// Responder's `X` is penalty ("I can beat ≥1 of their suits"); the constructive
/// answers are cue-bids — `3♣` = INV+ Stayman or 5+♠, `3♦` = INV+ 5+♥, `4♣`/`4♦`
/// = FG+ 5-5-majors splinters — with symmetric Smolen after the `3♣`→`3♦` denial.
/// Default **on** ([`set_uvu_x_floor`] / [`set_uvu_cue_floor`] tune the strength
/// ranges; the A/B best is `9` HCP / `8` points).
pub fn set_uvu(on: bool) {
    UVU.with(|cell| cell.set(on));
}

/// Whether the Unusual-vs-Unusual `(2NT)` structure is enabled
fn uvu() -> bool {
    UVU.with(Cell::get)
}

/// Set responder's penalty-double HCP floor over `1NT − (2NT)` — the A/B sweep
/// knob for "I can penalize one of their suits" (default `9`)
pub fn set_uvu_x_floor(floor: u8) {
    UVU_X_FLOOR.with(|cell| cell.set(floor));
}

fn uvu_x_floor() -> u8 {
    UVU_X_FLOOR.with(Cell::get)
}

/// Set responder's INV+ cue-bid points floor over `1NT − (2NT)` (default `8`)
pub fn set_uvu_cue_floor(floor: u8) {
    UVU_CUE_FLOOR.with(|cell| cell.set(floor));
}

fn uvu_cue_floor() -> u8 {
    UVU_CUE_FLOOR.with(Cell::get)
}

/// Set the length floor for responder's weak natural `3♥`/`3♠` escape over
/// `1NT − (2NT)` (default `6`; `5` lets a five-card major escape a bad defence)
pub fn set_uvu_natural_floor(floor: u8) {
    UVU_NATURAL_FLOOR.with(|cell| cell.set(floor));
}

fn uvu_natural_floor() -> u8 {
    UVU_NATURAL_FLOOR.with(Cell::get)
}

thread_local! {
    /// Optional parametric override of responder's double as
    /// `(min_len, max_len, min_hcp)` in their suit, superseding [`DoubleStyle`]
    /// for A/B sweeps that tune the length/strength threshold directly. `None`
    /// (default) uses the named [`DoubleStyle`]. See [`set_double_override`].
    static DOUBLE_OVERRIDE: Cell<Option<(usize, usize, u8)>> = const { Cell::new(None) };
}

/// Override responder's double with an explicit `(min_len, max_len, min_hcp)` in
/// their suit (for books built *after* this call; thread-local). `None` restores
/// the named [`DoubleStyle`]. Lets an A/B sweep the penalty/takeout boundary as a
/// continuum instead of the four discrete styles.
pub fn set_double_override(spec: Option<(usize, usize, u8)>) {
    DOUBLE_OVERRIDE.with(|cell| cell.set(spec));
}

/// Author responder's double of their `over` overcall per the active
/// [`DoubleStyle`] (or the [`set_double_override`] spec). Shadows the instinct
/// floor's takeout double so the threshold is the one chosen here.
fn responder_double(rules: Rules, over: Suit) -> Rules {
    if let Some((lo, hi, floor)) = DOUBLE_OVERRIDE.with(Cell::get) {
        return rules.rule(Call::Double, 1.55, len(over, lo..=hi) & hcp(floor..));
    }
    // The `len` ranges have distinct types, so author inside each arm.
    match double_style() {
        DoubleStyle::Takeout => rules.rule(Call::Double, 1.55, len(over, ..=3) & hcp(8..)),
        DoubleStyle::Penalty => rules.rule(Call::Double, 1.55, len(over, 4..) & hcp(9..)),
        DoubleStyle::PenaltyLight => rules.rule(Call::Double, 1.55, len(over, 4..) & hcp(7..)),
        DoubleStyle::Optional => rules.rule(Call::Double, 1.55, len(over, 2..=3) & hcp(8..)),
    }
}

thread_local! {
    /// Opener's penalty-pass over a `(2♣)` overcall, as
    /// `(min_club_len, min_club_hcp, convert_over_major)`. After `1NT-(2♣)-X-(P)`
    /// — where the systems-on Double is the stolen `2♣` Stayman — opener with this
    /// club holding *passes* to defend `2♣` doubled instead of answering Stayman.
    /// `convert_over_major` decides whether good clubs outrank a `2♥`/`2♠` major
    /// fit (`true`) or yield to it (`false`).
    ///
    /// **Default `Some((4, 4, true))`:** 4+ clubs with 4+ club HCP (an ace or two
    /// honors sitting over the overcaller), converting even with a major fit. A/B'd
    /// a clear win at every gate tested (`landy-ab`, 2M, Landy off both arms):
    /// **+5.35/+7.28 IMPs/divergent (none/both) on plain DD, +5.32/+7.09 under
    /// perfect defense** — the conversion is a pure penalty decision, so the two
    /// scorers agree. `None` restores the prior flaw (opener could never convert).
    /// See [`set_penalty_pass`].
    static PENALTY_PASS: Cell<Option<(usize, u8, bool)>> =
        const { Cell::new(Some((4, 4, true))) };
}

/// Set opener's penalty-pass of the stolen-Stayman Double over a `(2♣)` overcall,
/// gated on `(min_club_len, min_club_hcp, convert_over_major)` (for books built
/// *after* this call; thread-local, read once at construction). `None` restores
/// the historic behaviour where opener can never convert. A looser gate captures
/// more total IMPs (every gate down to `(4, 0, true)` and even 3-card clubs stays
/// net positive on DD) at lower per-conversion quality; the default trades a
/// little frequency for a genuine "good clubs" holding. The A/B knob is
/// `landy-ab --ns-penalty-pass LEN:HCP[:major]`.
pub fn set_penalty_pass(spec: Option<(usize, u8, bool)>) {
    PENALTY_PASS.with(|cell| cell.set(spec));
}

/// Opener's currently selected penalty-pass gate over `(2♣)`
fn penalty_pass() -> Option<(usize, u8, bool)> {
    PENALTY_PASS.with(Cell::get)
}

thread_local! {
    /// Whether responder's *direct* `3NT` over the overcall requires its own
    /// stopper in their suit (the default, `true`) or may be bid on game values
    /// alone, trusting opener's `1NT` for the stop (`false`). See
    /// [`set_direct_3nt_stopper`].
    static DIRECT_3NT_STOPPER: Cell<bool> = const { Cell::new(true) };
}

/// Require (or drop) responder's own stopper for a direct `3NT` over the overcall
/// (for books built *after* this call; thread-local, read once at construction).
/// Default `true` (status quo). With `false`, a game-values hand bids `3NT`
/// without a guaranteed stopper, leaning on opener's `1NT` — the A/B knob for
/// "does direct 3NT really need a stopper, or does X show it?".
pub fn set_direct_3nt_stopper(on: bool) {
    DIRECT_3NT_STOPPER.with(|cell| cell.set(on));
}

/// Whether a direct `3NT` requires responder's own stopper in their suit
fn direct_3nt_stopper() -> bool {
    DIRECT_3NT_STOPPER.with(Cell::get)
}

thread_local! {
    /// Whether responder *traps* with a too-good stopper: a direct `3NT`
    /// additionally denies **5+ HCP in the overcall suit**, so a strong holding
    /// (AQ, KQ, AKJ…) passes instead — waiting for opener to reopen with a takeout
    /// double and converting it to penalty. On by default. See [`set_trap_pass`].
    static TRAP_PASS: Cell<bool> = const { Cell::new(true) };
}

/// Enable the trap pass: with a too-good stopper (5+ HCP in their suit) responder
/// passes rather than declaring `3NT` (for books built *after* this call;
/// thread-local). Strong honors in the overcaller's suit defend better than they
/// declare — sit, let opener reopen with a takeout double, and convert to penalty.
///
/// The `5`-HCP threshold is **distilled from a per-board double-dummy oracle**
/// (`lebensohl-ab --pd-3nt --log-relay`): comparing `3NT` against trapping over
/// sampled layouts, the trap rate rises monotonically with HCP *in their suit*
/// (hcp 4 → 53%, 5 → 77%, 6+ → ~100%) and is **independent of length** — a long
/// weak holding (e.g. ♠A9642, 4 HCP) is a running source that wants `3NT`, while a
/// short strong one (♥AQ, 6 HCP) defends. The earlier length-based gate (4+ cards)
/// got this backwards and lost; this honor gate is the fix. **On by default**
/// (A/B vs off, isolated, 200k plain DD: the 1NT-Lebensohl responder gains
/// `+172`/`+185` IMPs — the original `resp 3NT` losers, −22/−20, are erased — at a
/// near-wash in the shared advance-of-takeout-double context; net `+155`/`+230`).
pub fn set_trap_pass(on: bool) {
    TRAP_PASS.with(|cell| cell.set(on));
}

/// Whether responder traps (passes) with a too-good stopper instead of `3NT`
fn trap_pass() -> bool {
    TRAP_PASS.with(Cell::get)
}

thread_local! {
    /// Whether opener's answer to partner's cue-raise (`1M – (ovc) – cue – P`)
    /// is authored. Default on — without it the cue-raise falls through to the
    /// keyless floor, which cannot act on a bid whose *named* suit (the cue)
    /// differs from its *shown* suit (the major), so opener passes and the
    /// cuebid is left in as the contract.
    static CUE_RAISE_ANSWER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's answer to partner's cue-raise for books built *after* this
/// call (thread-local)
///
/// **Default on** (`--no-ns-cue-raise-answer` in `bba-gen` for the off arm).
pub fn set_cue_raise_answer(on: bool) {
    CUE_RAISE_ANSWER.with(|cell| cell.set(on));
}

/// Whether opener's answer to a cue-raise is currently authored
fn cue_raise_answer() -> bool {
    CUE_RAISE_ANSWER.with(Cell::get)
}

thread_local! {
    /// Whether opener's answer to a *minor*-opening cue-raise
    /// (`1m – (ovc) – cue – P`) is authored. The minor twin of
    /// [`CUE_RAISE_ANSWER`]; separate knob so the A/B can isolate the minor
    /// contribution over the already-shipped major answer. Default on.
    static CUE_MINOR_RAISE_ANSWER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's answer to a minor-opening cue-raise for books built *after*
/// this call (thread-local)
///
/// **Default on** (`--no-ns-cue-minor-raise-answer` in `bba-gen` for the off
/// arm). Independent of [`set_cue_raise_answer`], which governs the majors.
pub fn set_cue_minor_raise_answer(on: bool) {
    CUE_MINOR_RAISE_ANSWER.with(|cell| cell.set(on));
}

/// Whether opener's answer to a minor-opening cue-raise is currently authored
fn cue_minor_raise_answer() -> bool {
    CUE_MINOR_RAISE_ANSWER.with(Cell::get)
}

/// Author responder's direct `3NT` over the overcall at `weight`, honoring the
/// stopper ([`direct_3nt_stopper`]) and trap-pass ([`trap_pass`]) toggles. The
/// trap denies a too-good stopper (`suit_hcp(over, ..=4)`). The `&`-chained
/// constraints have distinct types, so each combination is authored in its own arm.
fn author_direct_3nt(rules: Rules, weight: f32, over: Suit) -> Rules {
    let nt = Bid::new(3, Strain::Notrump);
    match (direct_3nt_stopper(), trap_pass()) {
        (true, true) => rules.rule(
            nt,
            weight,
            points(10..) & stopper_in(over) & suit_hcp(over, ..=4),
        ),
        (true, false) => rules.rule(nt, weight, points(10..) & stopper_in(over)),
        (false, true) => rules.rule(nt, weight, points(10..) & suit_hcp(over, ..=4)),
        (false, false) => rules.rule(nt, weight, points(10..)),
    }
}

thread_local! {
    /// Whether the Transfer-Lebensohl cue is split by stopper (see
    /// [`set_delayed_cue`]).
    static DELAYED_CUE: Cell<bool> = const { Cell::new(false) };
}

/// Enable the stopper-split cue for books built *after* this call (thread-local,
/// read once at book-construction time)
///
/// Larry Cohen's fast-denies / slow-shows, adapted to our Transfer Lebensohl:
/// the *direct* cue of their suit denies a stopper, while a *delayed* cue (relay
/// through `2NT`, then their suit) is Stayman *with* a stopper. It also denies a
/// 5-card unbid major (Smolen / Leaping Michaels handle those). Only the
/// single-unbid-major contexts — over `(2♥)` and `(2♠)` — are affected. Off by
/// default; gated behind this toggle for A/B measurement.
pub fn set_delayed_cue(on: bool) {
    DELAYED_CUE.with(|cell| cell.set(on));
}

/// Whether the stopper-split cue is enabled
pub(super) fn delayed_cue() -> bool {
    DELAYED_CUE.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our 2♣
    /// Stayman (`1NT-(P)-2♣-(X)` and `-(2♦/2♥/2♠)`); **on by default**, with an
    /// off-switch for A/B measurement.  See [`set_competition_over_stayman`].
    static COMPETITION_OVER_STAYMAN: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our 2♣ Stayman,
/// for books built *after* this call (thread-local; **on by default**).
///
/// Over a `(X)` (lead-directing clubs) opener answers in the *pass-denies-stopper*
/// coded scheme: a major or `2♦` promises a club stopper, Pass denies one, `XX` is
/// business clubs; responder's `XX` after opener's pass re-asks Stayman (forcing).
/// Over a `(2♦/2♥/2♠)` overcall opener bids a 4-card major naturally if it
/// outranks their suit, doubles for cards, else passes.
pub fn set_competition_over_stayman(on: bool) {
    COMPETITION_OVER_STAYMAN.with(|cell| cell.set(on));
}

/// Whether competition over our 2♣ Stayman is currently authored
fn competition_over_stayman() -> bool {
    COMPETITION_OVER_STAYMAN.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our Jacoby
    /// transfer (`1NT-(P)-2♦/2♥-(X)` and `-(overcall)`); **off by default** (opt-in
    /// A/B).  See [`set_competition_over_transfer`].
    static COMPETITION_OVER_TRANSFER: Cell<bool> = const { Cell::new(false) };
}

/// Author opener's replies after the opponents double or overcall our Jacoby
/// transfer, for books built *after* this call (thread-local; **off by default**).
///
/// Over a `(X)` opener completes the transfer with three-card support, jump
/// super-accepts with four and a maximum, passes with a doubleton (declining —
/// responder's `XX` then re-asks, forcing), or redoubles with the doubled
/// transfer suit as its own.  Over an overcall opener super-accepts the major
/// with a fit, doubles for cards, else passes.  Opt-in: unlike the contested 2♣
/// Stayman (which won +3.5 IMPs/fired), a paired A/B vs BBA over 640 000 boards
/// found these continuations a DD **loss** (plain −0.94, PD −0.33 IMPs/board it
/// fires on) — the super-accept and forcing re-ask drive us into failing
/// contracts the floor's lower bids avoid — so it stays off by default.
pub fn set_competition_over_transfer(on: bool) {
    COMPETITION_OVER_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our Jacoby transfer is currently authored
fn competition_over_transfer() -> bool {
    COMPETITION_OVER_TRANSFER.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our two-way
    /// 2♠ minor response (`1NT-(P)-2♠-(X)` and `-(overcall)`); **on by default**,
    /// with an off-switch for A/B measurement.  See
    /// [`set_competition_over_minor_transfer`].
    static COMPETITION_OVER_MINOR_TRANSFER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our two-way 2♠
/// (clubs-or-balanced-invite) response, for books built *after* this call
/// (thread-local; **on by default**).
///
/// Only the PUPPET 2♠ (the default — a club one-suiter *or* the balanced
/// invite that asks opener's size) has a min/max answer to protect, so the block
/// no-ops under the EUROPEAN pure-transfer scheme.  Their `(X)` of 2♠ is
/// lead-directing spades, so opener re-encodes its size-ask answer *and* a spade
/// stopper across four calls: `2NT` = minimum **with** a stopper, `3♣` = maximum
/// **with** one, `Pass` = minimum **no** stopper, `XX` = maximum **no** stopper.
/// After a stopper-showing bid responder's rebids match the uncontested tree
/// (strip the `X` to a Pass); after a no-stopper reply responder signs off in `3♣`
/// with clubs.  A `(2NT)`/`(3♣)` overcall (which steals the size-ask steps) keeps
/// the signal alive — `3NT` = maximum + stopper, `X` = maximum no stopper, Pass =
/// minimum; any higher overcall is systems-off (a `X` showing their suit, else
/// Pass).  Like the contested 2♣ Stayman this is a **constructive** win: a paired
/// A/B vs BBA over 640 000 boards measured **+4.80 IMPs/board it fires on** on plain
/// double-dummy (+5.63 under perfect-defense — *higher*, so it is a sound
/// contract-finding gain, not a doubling artifact), CI excluding 0, so it ships on.
/// Rare (it fired on 0.03 %): BBA seldom contests our 2♠.
pub fn set_competition_over_minor_transfer(on: bool) {
    COMPETITION_OVER_MINOR_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our two-way 2♠ minor response is currently authored
fn competition_over_minor_transfer() -> bool {
    COMPETITION_OVER_MINOR_TRANSFER.with(Cell::get)
}

thread_local! {
    /// Whether opener authors continuations after the opponents contest our 2NT
    /// diamond transfer (`1NT-(P)-2NT-(X)` and `-(overcall)`); **on by default**,
    /// with an off-switch for A/B measurement.  See
    /// [`set_competition_over_diamond_transfer`].
    static COMPETITION_OVER_DIAMOND_TRANSFER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's replies after the opponents double or overcall our 2NT diamond
/// transfer (6+♦, or 5♦-4♣), for books built *after* this call (thread-local;
/// **on by default**).
///
/// Only the PUPPET scheme (the default) plays 2NT as the diamond transfer, so the
/// block no-ops under EUROPEAN (where 2NT is the balanced size-ask).  Their `(X)`
/// is lead-directing diamonds; the double frees `Pass` to be the catch-all
/// "no fit" call, which lets opener's `3♣` shed its uncontested
/// relay-denies-a-fit meaning and become **natural** (4+♣, finding responder's
/// 5♦-4♣ club fit): `3♦` = accept with a diamond fit (3+♦), `3♣` = no fit but
/// 4+♣, `XX` = maximum values without a fit (penalty-oriented), `Pass` = minimum
/// catch-all.  After a fit-showing `3♦`/`3♣` responder's rebids match the
/// uncontested tree (strip the `X` to a Pass); after `Pass`/`XX` (no fit)
/// responder always holds 5+♦ and signs off in `3♦`.  An overcall is handled
/// naturally: `3♣` leaves room to complete `3♦` with a fit (else `X` = penalty,
/// Pass = minimum); a higher overcall keeps `3NT` (max + stopper) / `X` (their
/// suit) / Pass.  **On by default** (off-switch `bba-gen
/// --no-ns-comp-over-diamond-transfer`): a paired A/B vs BBA over 1 000 000
/// `--filter-1nt` boards (410 fired, 0.04 %) measured a plain-DD **wash** (+0.24
/// IMPs/board it fires on, CI straddling 0) and a clear perfect-defense gain (+3.40
/// PD).  Unlike the 2♠ minor (which won on *both* scorers), the honest-DD signal is
/// a wash — but it never *loses* on plain DD, and the PD gain is real value the day
/// the opponents punish the floor's `X`-then-pull-to-`3NT` overreach, so it ships on.
pub fn set_competition_over_diamond_transfer(on: bool) {
    COMPETITION_OVER_DIAMOND_TRANSFER.with(|cell| cell.set(on));
}

/// Whether competition over our 2NT diamond transfer is currently authored
fn competition_over_diamond_transfer() -> bool {
    COMPETITION_OVER_DIAMOND_TRANSFER.with(Cell::get)
}

thread_local! {
    /// The weak natural `2♦/2♥/2♠` escape's strength floor as
    /// `(hcp_floor, points_floor)` — one is `0`; `(0, 0)` = no floor (see
    /// [`set_natural_floor`]). Defaults to a **`5`-HCP** floor (with opener's
    /// game-raise): a floor of any kind beats none by `+0.012`/`+0.016` IMPs/board
    /// (none/both), and — once `(2♣)` went systems-on, leaving the natural escape
    /// all *majors* (every one game-raisable, no raise-less minor) — `5` HCP beats
    /// the relay's `6` by `+2.5`/`+2.3` IMPs/divergent (none/both), all-positive.
    /// `4` HCP is too loose: the raises turn negative (overbidding). One lower than
    /// the relay's `6`, matching the 2X sitting one level lower.
    static NATURAL_FLOOR: Cell<(u8, u8)> = const { Cell::new((5, 0)) };
}

/// Floor responder's weak natural 2-level escape (for books built *after* this
/// call; thread-local, read once at book-construction time)
///
/// The direct natural `2♦/2♥/2♠` over the overcall is the same weak 5-card-suit
/// hand as the relay-then-correct sign-off (`2NT`→`3♣`→`3M`), one level lower —
/// but unlike that sign-off it currently has no strength floor and opener cannot
/// raise it. A non-zero floor makes the two symmetric: it adds the floor to the
/// natural (an HCP floor *or* a total-points floor — being a level lower than the
/// relay, the 2X floor can be lower or playing-strength oriented), and registers
/// opener's `lebensohl_signoff_raise` over a natural *major* sign-off so a
/// maximum with a fit stretches to game. Pass `(hcp, 0)` for an HCP floor,
/// `(0, points)` for a points floor, `(0, 0)` to disable. Off by default.
pub fn set_natural_floor(hcp_floor: u8, points_floor: u8) {
    NATURAL_FLOOR.with(|cell| cell.set((hcp_floor, points_floor)));
}

/// Whether the weak natural escape is floored (and opener may raise it)
fn natural_floor_on() -> bool {
    let (hcp, points) = NATURAL_FLOOR.with(Cell::get);
    hcp > 0 || points > 0
}

/// The HCP floor on the weak natural escape (`0` = none) — a bound, so the
/// constraint type stays stable whether or not the floor is engaged.
fn natural_floor_hcp() -> u8 {
    NATURAL_FLOOR.with(Cell::get).0
}

/// The total-points floor on the weak natural escape (`0` = none)
fn natural_floor_pts() -> u8 {
    NATURAL_FLOOR.with(Cell::get).1
}

thread_local! {
    /// Whether responder reads a `(2♦)` overcall of our `1NT` as a **Multi** (an
    /// unknown single-suited major) and answers with the Multi counter-defense
    /// ([`multi_responder`]) instead of the natural-diamond Transfer/Lebensohl
    /// package. Off by default — opt-in pending the A/B. It overrides only the
    /// `(2♦)` responder node; the shared `2NT` relay machinery is unchanged. See
    /// `docs/ai-bidder/bba-multi-2d.md`.
    static DEFENSE_2D_MULTI: Cell<bool> = const { Cell::new(false) };
}

/// Read a `(2♦)` overcall of our `1NT` as a **Multi** (an unknown single-suited
/// major) and answer with the Multi counter-defense, for books built *after*
/// this call (thread-local, read once at book-construction time)
///
/// Distilled from BBA's Multi-Landy counter (`docs/ai-bidder/bba-multi-2d.md`):
/// double = values, everything else natural. Off by default; faithful for the A/B
/// against BBA, whose `2♦` over our `1NT` is always a Multi.
pub fn set_defense_to_2d_multi(on: bool) {
    DEFENSE_2D_MULTI.with(|cell| cell.set(on));
}

/// Whether the `(2♦)`-as-Multi counter-defense is engaged
fn defense_2d_multi() -> bool {
    DEFENSE_2D_MULTI.with(Cell::get)
}

/// The single unbid major when `over` is itself a major (the other major)
///
/// `None` when `over` is a minor (then both majors are unbid) — the stopper-split
/// cue is only authored for the single-unbid-major contexts.
pub(super) fn unbid_major(over: Suit) -> Option<Suit> {
    match over {
        Suit::Hearts => Some(Suit::Spades),
        Suit::Spades => Some(Suit::Hearts),
        _ => None,
    }
}

/// The 2NT-relay shape over their `over` overcall: a 5+ suit (not their suit)
/// with 6+ HCP.
///
/// The 6-HCP floor is PD-distilled. A perfect-defense gate (relay only when
/// sampled double-dummy says our 3-level line out-scores defending) declines
/// nearly every sub-6 hand — pushing a near-bust to the 3 level loses on DD,
/// even with a 6-card suit — and this plain HCP floor recovers ~60–80% of that
/// gate's IMPs/board gain over relaying every 5-card suit (A/B, lebensohl-ab,
/// `--pd-relay`). Adverse-suit length/honors were *not* predictive; overall
/// weakness is the driver.
fn lebensohl_relay_shape(over: Suit) -> Cons<impl Constraint + Clone> {
    let five = |s: Suit| len(s, 5..);
    let any5 = match over {
        Suit::Clubs => five(Suit::Diamonds) | five(Suit::Hearts) | five(Suit::Spades),
        Suit::Diamonds => five(Suit::Clubs) | five(Suit::Hearts) | five(Suit::Spades),
        Suit::Hearts => five(Suit::Clubs) | five(Suit::Diamonds) | five(Suit::Spades),
        Suit::Spades => five(Suit::Clubs) | five(Suit::Diamonds) | five(Suit::Hearts),
    };
    any5 & hcp(6..)
}

// ---------------------------------------------------------------------------
// Section 1: direct-seat response to their overcall
// ---------------------------------------------------------------------------

/// Responder's action after our opening `o` and their overcall (≤ 2♠)
///
/// Covers cue-bid limit-plus raises, preemptive and competitive raises of
/// the opening suit, negative doubles, and weak jump shifts.
fn over_their_overcall(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);

    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };
    let jump_min: usize = if is_major { 4 } else { 5 };

    let other_major = match o {
        Suit::Hearts => Suit::Spades,
        // Spades → Hearts; for minors, Hearts is used only in the negative double
        _ => Suit::Hearts,
    };

    let mut rules = Rules::new();

    // Cue-bid raises: for each suit t ≠ o, levels 2 and 3
    for t in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if t == o {
            continue;
        }
        let t_strain = Strain::from(t);
        for lvl in 2u8..=3 {
            rules = rules
                .rule(
                    Bid::new(lvl, t_strain),
                    2.0,
                    they_bid(t_strain)
                        & min_level_is(lvl, t_strain)
                        & support(raise_min..)
                        & points(10..),
                )
                .alert(CUE_RAISE);
        }
    }

    // Jump raise: preemptive (min_level=2 means we could bid 2o, so 3o is a jump)
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.6,
        min_level_is(2, o_strain) & support(jump_min..) & points(..=9),
    );

    // Competitive raise: 3o when it's the minimum legal bid
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.3,
        min_level_is(3, o_strain) & support(raise_min..) & points(6..=9),
    );

    // Single raise
    rules = rules.rule(
        Bid::new(2, o_strain),
        1.5,
        min_level_is(2, o_strain) & support(raise_min..) & points(6..=9),
    );

    // Negative double
    rules = if is_major {
        // Other major, 4+ cards, 8+ HCP
        rules
            .rule(Call::Double, 1.0, len(other_major, 4..) & hcp(8..))
            .alert(NEGATIVE_DOUBLE)
    } else {
        // Both majors 4+, 8+ HCP
        rules
            .rule(
                Call::Double,
                1.0,
                len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
            )
            .alert(NEGATIVE_DOUBLE)
    };

    // Weak jump shifts: for each suit x ≠ o, levels 2 and 3
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let x_strain = Strain::from(x);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, x_strain),
                1.1,
                min_level_is(lvl - 1, x_strain) & len(x, 6..) & points(2..=5) & !they_bid(x_strain),
            );
        }
    }

    // Pass
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 3: support doubles and redoubles
// ---------------------------------------------------------------------------

/// Opener's support double/redouble rules showing three-card support for major M
///
/// `Call::Double` with exactly 3 (support double); `2M` with 4+ (natural raise);
/// Pass as the catch-all.
fn support_rules(major: Suit) -> Rules {
    let m = Strain::from(major);
    Rules::new()
        .rule(Call::Double, 1.5, support(3..=3))
        .alert(SUPPORT_DOUBLE)
        .rule(Bid::new(2, m), 1.4, support(4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 4: opener answers partner's negative double of a two-level minor
// ---------------------------------------------------------------------------

/// Opener's answer after `1M – (2m) – X – P` (partner doubled a minor overcall)
///
/// Shows four-card length in the other major or rebids the opening major on five.
/// No Pass rule — the double is forcing.
fn answer_neg_double_of_minor(opening_major: Suit) -> Rules {
    let m = Strain::from(opening_major);
    let other = if opening_major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let other_strain = Strain::from(other);
    Rules::new()
        .rule(Bid::new(2, other_strain), 1.0, len(other, 3..))
        .rule(Bid::new(2, m), 0.5, len(opening_major, 5..))
}

/// Opener's answer after `1M – (ovc) – cue – P` (partner cue-raised to a
/// limit-plus raise of the opening major): accept to game or decline
///
/// The contested twin of [`opener_after_limit_raise`][super::raises], minus the
/// keycard ask — offering `4NT` RKCB here would strand it, because the contested
/// node has no authored keycard *responses* (the uncontested tree installs them
/// via `slam::install_rkcb`; this one does not). So a strong opener blasts `4M`
/// game rather than pass out a 4NT nobody answers; slam exploration is a later
/// opt-in.
///
/// The one difference from the uncontested `1M – 3M` version is the **decline**:
/// there partner already *bid* the major, so opener passes to play it; after a
/// cuebid partner named the *opponents'* suit, so opener must actively **sign off
/// in 3M** — passing would leave the cuebid in as the contract (the very bug this
/// table fixes). The point-gate does the work: a minimum opener fails
/// `points(13..)` and takes the 3M catch-all.
fn answer_cue_raise(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        // 4M: accept → game.
        .rule(Bid::new(4, trump), 1.0, points(13..))
        // 3M: decline → sign off in the major (catch-all).
        //
        // ponytail: decline assumes 3M is legal, which holds for every cue below
        // 3M — all cues over 1♠, and cues over 1♥ except a 3♠ cue. A 3♠ cue over
        // 1♥ with a minimum opener has 3♥ illegal and falls back through to the
        // floor (Pass); rare, revisit if the A/B surfaces it.
        .rule(Bid::new(3, trump), 0.0, hcp(0..))
}

/// Opener's answer after `1m – (ovc) – cue – P` (partner cue-raised to a
/// limit-plus raise of the opening minor): bid the best game or sign off
///
/// The minor twin of [`answer_cue_raise`]. Two differences from the major
/// version, both because minor game (`5m`) is remote:
///
/// * **Accept is `3NT`, not `5m`** — gated on values *and* a stopper in their
///   suit (`stopper_in_their_suits`), so we don't get run in the overcall suit.
///   `3NT` outranks any in-scope cue (`≤ 3♠`), so it is always legal.
/// * **Decline is our minor, but its level floats.** After a 1-level overcall
///   the cue is at the 2 level and `3m` signs off; after a 2-level overcall the
///   cue is at the 3 level and `3m` sits *below* it for a club opening, so `4m`
///   is the only sign-off. The engine does **not** mask illegal calls, so each
///   decline rung is legality-anchored with `min_level_is`: exactly one of `3m`
///   / `4m` is the cheapest our-minor bid in any given auction, and only that
///   one fires.
fn answer_cue_minor_raise(minor: Suit) -> Rules {
    let trump = Strain::from(minor);
    Rules::new()
        // 3NT: accept to the best game — needs values and their suit stopped.
        //
        // ponytail: always 3NT, never 5m. A single stopper is thin against a
        // 6-card overcall suit, and a 10-card minor fit sometimes plays 5m when
        // 3NT gets run. The A/B win is net of that tail; splitting 3NT-vs-5m on
        // fit length is the upgrade path if a re-measure wants the last IMPs.
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            points(14..) & stopper_in_their_suits(),
        )
        // 3m: decline when our minor is still available at the 3 level.
        .rule(Bid::new(3, trump), 0.5, min_level_is(3, trump))
        // 4m: decline when 3m sits below the cue (club opening, 3-level cue).
        .rule(Bid::new(4, trump), 0.5, min_level_is(4, trump))
}

// ---------------------------------------------------------------------------
// Section 5: Lebensohl after our 1NT is overcalled
// ---------------------------------------------------------------------------

/// Responder's plain-Lebensohl actions after our `1NT` and a natural 2-level
/// overcall in `over`
///
/// A book node here *shadows* the instinct floor, so this table covers every
/// responder hand. The Lebensohl idea separates weak from strong: weak hands
/// relay through `2NT` to a `3♣` sign-off (or correct to a long suit), while game
/// hands bid a forcing 3-level suit or a to-play `3NT` directly — so a game is
/// never stranded in a partscore (the failure mode of the Rubensohl v1 attempt).
//
// ponytail: the cue (Stayman / stopper-ask, "slow shows / fast denies") is
// skipped — 4-4-major game hands bid 3NT. Author the cue + opener's reply if the
// A/B shows it matters.
//
// The Section-5 builders below are pure functions of `(over, hand)` — the auction
// prefix and the bidder's identity never enter — so `american/defense.rs` reuses
// them verbatim for "sohl after a takeout double" (advancing partner's takeout
// double of a weak two), where the opponents' suit is likewise at the two level.
pub(super) fn lebensohl_responder(over: Suit) -> Rules {
    let mut rules = Rules::new();

    // Forcing 3-level new suit: game-forcing, 5+ cards. A jump (when the 2-level
    // was available) or the cheapest 3-level bid (suit at/below the overcall) —
    // either way 3-of-a-suit over the interference is forcing. (All 3-level bids
    // clear a 2-level overcall, so no min-level gate is needed.)
    for s in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(Bid::new(3, strain), 1.8, len(s, 5..) & points(10..));
    }

    // Direct cue of their suit = Stayman: game-forcing with a 4-card unbid major
    // (no 5-card suit to bid forcingly — those use the 3-level above). Answered by
    // [`cue_stayman_answer`]. Stopper-agnostic, mirroring Transfer's default cue,
    // so a 4-4 major fit is found even with a stopper. Weight sits between the
    // natural forcing 3-level (1.8) and direct 3NT (1.7): a known 5-card suit is
    // bid naturally, a bare 4-card major cues, else 3NT.
    let cue = Bid::new(3, Strain::from(over));
    rules = match unbid_major(over) {
        Some(major) => rules
            .rule(cue, 1.75, len(major, 4..) & points(10..))
            .alert(LEBENSOHL_CUE),
        None => rules
            .rule(
                cue,
                1.75,
                (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & points(10..),
            )
            .alert(LEBENSOHL_CUE),
    };

    // Direct 3NT to play: game values with their suit stopped (toggles: drop the
    // stopper requirement, and/or trap-pass with 4+ in their suit).
    rules = author_direct_3nt(rules, 1.7, over);

    // Responder's double of their overcall (penalty by default; see [`DoubleStyle`]).
    rules = responder_double(rules, over);

    // Natural new suit at the 2 level (above the overcall, below 2NT): weak,
    // competitive, to play.
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.5,
            min_level_is(2, strain)
                & len(s, 5..)
                & points(..=9)
                & hcp(natural_floor_hcp()..)
                & points(natural_floor_pts()..),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak hand with a long suit not biddable
    // naturally at the 2 level (long clubs, or a suit below the overcall) — sign
    // off in 3♣ or correct (see [`lebensohl_relay_rebid`]). The natural 2-level
    // outranks this relay, so above-the-overcall suits are still bid naturally;
    // balanced weak hands pass. See [`lebensohl_relay_shape`] for the 6+/good-5
    // shape and the PD-distilled 6-HCP floor on the 5-card arm.
    let long_suit = lebensohl_relay_shape(over);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 1.4, points(..=9) & long_suit)
        .alert(LEBENSOHL_RELAY);

    // Pass — weak, nothing constructive to say.
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's counter-defense after `1NT − (2♦)` when the `2♦` is read as a
/// **Multi** (an unknown single-suited major), engaged by
/// [`set_defense_to_2d_multi`]
///
/// Distilled from BBA's Multi-Landy counter (`docs/ai-bidder/bba-multi-2d.md`):
/// **double = values / takeout** of the unknown major (BBA's 41% workhorse), and
/// everything else **natural**. Unlike the natural-diamond treatments, both
/// majors are biddable naturally at the 2 level and `2♦` steals no major room, so
/// there is no Stayman cue — the diamond bid that would be the cue is just natural
/// diamonds. The `2NT` relay and its `3♣` completion are the shared Lebensohl
/// machinery (registered for `(2♦)` regardless of this toggle), so weak
/// club/diamond one-suiters keep their sign-off.
fn multi_responder() -> Rules {
    let over = Suit::Diamonds; // the call we sit over; their real suit is a major
    let mut rules = Rules::new();

    // X = values / takeout of the unknown major — BBA's backbone (41%). Floored
    // at 8 (a touch above BBA's loose ~5) for doubled-contract discipline.
    rules = rules.rule(Call::Double, 1.55, points(8..));

    // Natural forcing 3-level single-suiter (incl. natural 3♦ — diamonds is not
    // their suit, so no cue).
    for s in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(Bid::new(3, strain), 1.8, len(s, 5..) & points(10..));
    }

    // Direct 3NT to play (default toggles → plain game values).
    rules = author_direct_3nt(rules, 1.7, over);

    // Natural weak 2-level major — both majors clear the `2♦` overcall.
    for s in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.5,
            len(s, 5..) & points(..=9) & hcp(natural_floor_hcp()..) & points(natural_floor_pts()..),
        );
    }

    // 2NT = Lebensohl relay to 3♣ (weak long minor / suit below the majors).
    let long_suit = lebensohl_relay_shape(over);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 1.4, points(..=9) & long_suit)
        .alert(LEBENSOHL_RELAY);

    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener completes responder's Lebensohl `2NT` relay with the forced `3♣`
pub(super) fn complete_lebensohl_relay() -> Rules {
    Rules::new().rule(Bid::new(3, Strain::Clubs), 1.0, hcp(0..))
}

/// Responder's rebid after the `2NT` relay is completed at `3♣`
///
/// Pass to play clubs, or correct to the six-card suit (still a weak sign-off).
pub(super) fn lebensohl_relay_rebid(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(3, strain),
            1.0,
            min_level_is(3, strain) & len(s, 5..),
        );
    }
    // Stopper-split on: the *delayed* cue of their suit — Stayman with a stopper,
    // game-forcing, exactly a 4-card unbid major (denies 5). Answered by
    // [`cue_stayman_answer`] (the stopper is guaranteed, so 3NT is safe).
    if let (true, Some(major)) = (delayed_cue(), unbid_major(over)) {
        rules = rules
            .rule(
                Bid::new(3, Strain::from(over)),
                1.5,
                points(10..) & stopper_in(over) & len(major, 4..) & len(major, ..5),
            )
            .alert(LEBENSOHL_CUE);
    }
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to responder's weak Lebensohl sign-off in a major
///
/// Responder's sign-off is a known weak hand with a 5+ suit, floored at
/// `resp_floor` points (the relay's PD-distilled 6, or the direct natural
/// escape's lower 5 — see [`lebensohl_relay_shape`] and [`set_natural_floor`]).
/// A *maximum* 1NT opener with a fit stretches to game: the combined floor is
/// then high enough to reach the 4M zone with a long-trump dummy.
///
/// The gauge is points *plus* trump length, not points alone — a
/// Law-of-Total-Tricks dummy adjustment that trades one point per extra trump.
/// The combined target is 23 (a 17-max opposite the relay's 6-floor with an
/// 8-card fit); a lower responder floor raises opener's bar by the same amount,
/// and each trump beyond three lowers it by one.  Anything short passes the
/// sign-off.  Only majors — a minor sign-off's game is the 5 level, out of
/// reach for a weak hand.
pub(super) fn lebensohl_signoff_raise(signoff: Suit, resp_floor: u8) -> Rules {
    let game = Bid::new(4, Strain::from(signoff));
    let base = 23u8.saturating_sub(resp_floor); // opener points with bare 3-card support
    Rules::new()
        .rule(
            game,
            1.0,
            (len(signoff, 3..=3) & points(base..))
                | (len(signoff, 4..=4) & points(base.saturating_sub(1)..))
                | (len(signoff, 5..) & points(base.saturating_sub(2)..)),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 5b: Transfer Lebensohl (Rubensohl) — Larry Cohen's version
// ---------------------------------------------------------------------------

/// How responder treats a flat 4-3-3-3 when our 1NT opening is overcalled.
///
/// The constructive 4333 rule (a flat hand plays 3NT, not the major fit, for want
/// of a ruffing value — see `notrump::flat_4333`) was unclear in competition: a
/// stopperless flat 4333 might *need* the 4-4 fit to escape a 3NT it cannot make.
/// A paired BBA A/B settled it — full [`Suppress`][Competitive4333::Suppress] of
/// the Transfer-Lebensohl cue-Stayman and the `3♣`-over-`(2♦)` Stayman beat both
/// `Allow` and the stopper-only middle on plain *and* PD double-dummy (960k boards
/// vul none, 63 fired: PD **+3.8 IMPs/fired**, +0.0002/board with the 95% CI
/// excluding 0; plain a wash-to-win at +1.3/fired).  Even the stopperless flat 4333
/// does better staying low than digging out a no-ruffing-value fit that gets
/// doubled.  **Default [`Suppress`][Competitive4333::Suppress]**; the other modes
/// stay for re-measurement (e.g. at vul both).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Competitive4333 {
    /// Cue-Stayman unchanged on a flat 4333 — the old behaviour / A/B baseline.
    Allow,
    /// Never cue-Stayman on a flat 4333; play 3NT (or a natural call) instead.
    Suppress,
    /// Suppress only a flat 4333 *with* a stopper in their suit (3NT is safe); a
    /// stopperless 4333 may still cue to dig out the 4-4 fit.
    SuppressWithStopper,
}

thread_local! {
    static COMPETITIVE_4333: Cell<Competitive4333> =
        const { Cell::new(Competitive4333::Suppress) };
}

/// Set how a flat 4-3-3-3 cue-Staymans when our 1NT is overcalled, for books
/// built *after* this call (thread-local; default [`Competitive4333::Suppress`]).
pub fn set_competitive_4333(mode: Competitive4333) {
    COMPETITIVE_4333.with(|cell| cell.set(mode));
}

/// The active [`Competitive4333`] mode
fn competitive_4333() -> Competitive4333 {
    COMPETITIVE_4333.with(Cell::get)
}

/// Gate ANDed into each competitive cue-Stayman rule: satisfied unless the active
/// [`Competitive4333`] mode diverts this flat 4-3-3-3 to 3NT.  Four suits all 3
/// or 4 cards long sum to 13 only as a 4-3-3-3, so that test *is* "flat 4333".
///
/// `gate` is true only in the 1NT-overcall context, where partner is a *balanced*
/// 1NT opener and a flat 4333 has no ruffing value anywhere.  When advancing a
/// takeout double (`gate = false`) partner is *short* in their suit, so the 4-4
/// fit keeps its ruffing value and the cue is never diverted — the curse does not
/// apply, and that A/B was never run.
fn competitive_4333_ok(over: Suit, gate: bool) -> Cons<impl Constraint + Clone> {
    let mode = if gate {
        competitive_4333()
    } else {
        Competitive4333::Allow
    };
    described(
        "not a flat 4-3-3-3 diverted to 3NT",
        move |hand: Hand, _: &Context<'_>| {
            let flat = Suit::ASC
                .into_iter()
                .all(|suit| (3..=4).contains(&hand[suit].len()));
            !match mode {
                Competitive4333::Allow => false,
                Competitive4333::Suppress => flat,
                Competitive4333::SuppressWithStopper => flat && has_stopper(hand[over]),
            }
        },
    )
}

/// The suit a 3-level Transfer-Lebensohl bid in `bid_suit` shows, given the
/// opponents' 2-level overcall in `over`
///
/// The cheapest suit strictly above `bid_suit` that is *not* their suit — a
/// transfer *through* the adverse suit. `None` when `bid_suit` is their suit
/// (that bid is the Stayman cue, not a transfer) or no higher suit remains
/// (the lowest target, clubs, has no dedicated transfer — those rare hands use
/// the `2NT` relay or `3NT`).
pub(super) fn transfer_target(bid_suit: Suit, over: Suit) -> Option<Suit> {
    if bid_suit == over {
        return None; // the cue = Stayman, not a transfer
    }
    [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
        .into_iter()
        .find(|&s| (s as u8) > (bid_suit as u8) && s != over)
}

/// Responder's Transfer-Lebensohl actions after our `1NT` and a natural 2-level
/// overcall in `over`
///
/// Weak hands keep the plain-Lebensohl outlets (natural 2-level, `2NT` relay to
/// `3♣`, penalty double). Invitational-or-better hands transfer at the 3 level:
/// each non-cue suit bid transfers to the next suit up *through* the adverse
/// suit, and the cue (their suit) is Stayman. Because a weak hand always has a
/// natural 2-level call, a 3-level transfer to a suit above theirs is INV+ — so
/// opener is driven to game (see [`transfer_completion`]) and a game is never
/// stranded in a partscore (the Rubensohl-v1 failure).
pub(super) fn transfer_lebensohl_responder(over: Suit, gate_4333: bool) -> Rules {
    let mut rules = Rules::new();

    // 3-level transfers (INV+, 5+ in the target) and the cue (Stayman, GF).
    for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(bid_suit);
        if bid_suit == over {
            // Cue = Stayman: game values with a 4-card unbid major. (The arms
            // differ in constraint type, so each returns the updated `Rules`.)
            // With the stopper-split on, the *direct* cue denies a stopper —
            // stopper hands relay through 2NT to the delayed cue (the broadened
            // 2NT below + [`lebensohl_relay_rebid`]).
            let cue = Bid::new(3, strain);
            let split = delayed_cue() && unbid_major(over).is_some();
            rules = match (over, split) {
                (Suit::Hearts, true) => rules
                    .rule(
                        cue,
                        1.7,
                        len(Suit::Spades, 4..)
                            & points(10..)
                            & !stopper_in(over)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Spades, true) => rules
                    .rule(
                        cue,
                        1.7,
                        len(Suit::Hearts, 4..)
                            & points(10..)
                            & !stopper_in(over)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Hearts, false) => rules
                    .rule(
                        cue,
                        1.7,
                        len(Suit::Spades, 4..)
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                (Suit::Spades, false) => rules
                    .rule(
                        cue,
                        1.7,
                        len(Suit::Hearts, 4..)
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
                _ => rules
                    .rule(
                        cue,
                        1.7,
                        (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..))
                            & points(10..)
                            & competitive_4333_ok(over, gate_4333),
                    )
                    .alert(LEBENSOHL_CUE),
            };
        } else if let Some(target) = transfer_target(bid_suit, over) {
            // Transfer: show 5+ in the target, invitational or better. A major
            // target outranks the cue so a 5-card major is shown by the
            // transfer, not Stayman; a minor target is rare (long minor, no
            // stopper) and yields to Stayman / 3NT.
            let weight = if matches!(target, Suit::Hearts | Suit::Spades) {
                1.8
            } else {
                1.45
            };
            rules = rules
                .rule(Bid::new(3, strain), weight, len(target, 5..) & points(9..))
                .alert(LEBENSOHL_TRANSFER);
        } else if over != Suit::Clubs {
            // Top step (no suit above to transfer into): a *forced* game-force
            // transfer to clubs, 6+♣. Its completion lands at game, so 3♣ can
            // never be the contract — the only forcing long-club route (the
            // 2NT→3♣ relay is the *weak* one). Weight below 3NT's 1.5 so a 6♣
            // hand *with* a stopper picks 3NT; only no-stopper hands transfer.
            // (Over (2♣) clubs is their suit — there is no top-step transfer.)
            rules = rules
                .rule(
                    Bid::new(3, strain),
                    1.45,
                    len(Suit::Clubs, 6..) & points(10..),
                )
                .alert(LEBENSOHL_TRANSFER);
        }
    }

    // Direct 3NT to play: game values with their suit stopped, no major to show
    // (toggles: drop the stopper requirement, and/or trap-pass with 4+ in their
    // suit — long-in-their-suit defends better than it declares).
    rules = author_direct_3nt(rules, 1.5, over);

    // Stopper-split on: a GF hand with a stopper *and* exactly a 4-card unbid
    // major relays through 2NT to bid the cue *slowly* (Stayman with a stopper,
    // see [`lebensohl_relay_rebid`]) — outweighing direct 3NT (1.5) so the 4-4
    // major fit is still found. Denies a 5-card major (Smolen / Leaping Michaels).
    if let (true, Some(major)) = (delayed_cue(), unbid_major(over)) {
        rules = rules
            .rule(
                Bid::new(2, Strain::Notrump),
                1.6,
                points(10..) & stopper_in(over) & len(major, 4..) & len(major, ..5),
            )
            .alert(LEBENSOHL_RELAY);
    }

    // Responder's double of their overcall (penalty by default; see
    // [`DoubleStyle`]). Authoring it is also what kept the floor's penalty
    // doubles — the Rubensohl-v1 attempt lost them by shadowing with no double.
    rules = responder_double(rules, over);

    // Natural new suit at the 2 level (above the overcall, below 2NT): weak.
    for s in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if s == over {
            continue;
        }
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.4,
            min_level_is(2, strain)
                & len(s, 5..)
                & points(..=8)
                & hcp(natural_floor_hcp()..)
                & points(natural_floor_pts()..),
        );
    }

    // 2NT = Lebensohl relay to 3♣: a weak long-suit hand (sign off or correct),
    // same shape as plain Lebensohl (see [`lebensohl_relay_shape`] — 6+ suit, or
    // a 5-carder with the PD-distilled 6-HCP floor, never their suit).
    let long_suit = lebensohl_relay_shape(over);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 1.35, points(..=8) & long_suit)
        .alert(LEBENSOHL_RELAY);

    // Pass — weak, nothing constructive to say.
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply after responder's Transfer-Lebensohl transfer to `target`
///
/// A transfer to a major is INV+, so opener is driven to **game**: `4M` with a
/// fit, else `3NT`. A transfer to a minor (rare — long minor, no stopper) is
/// completed at the 3 level, or `3NT` with a stopper; responder drives on.
pub(super) fn transfer_completion(target: Suit, over: Suit) -> Rules {
    let t = Strain::from(target);
    let mut rules = Rules::new();
    if matches!(target, Suit::Hearts | Suit::Spades) {
        rules = rules.rule(Bid::new(4, t), 1.6, len(target, 3..)).rule(
            Bid::new(3, Strain::Notrump),
            1.4,
            len(target, ..3),
        );
    } else {
        // ponytail: minor-target 5m / slam exploration is left to the floor;
        // 3NT-or-complete covers the common game. Author it if the A/B shows
        // minor transfers matter.
        rules = rules
            .rule(Bid::new(3, Strain::Notrump), 1.5, stopper_in(over))
            .rule(Bid::new(3, t), 1.3, len(target, 3..));
    }
    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's reply to responder's Transfer-Lebensohl cue (Stayman, game-forcing)
///
/// Shows a 4-card unbid major at its cheapest legal level, else `3NT`.
pub(super) fn cue_stayman_answer(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(3, m), 1.6, len(major, 4..) & min_level_is(3, m))
            .rule(Bid::new(4, m), 1.5, len(major, 4..) & min_level_is(4, m));
    }
    // No 4-card unbid major → 3NT (always legal above the 3-level cue).
    rules.rule(Bid::new(3, Strain::Notrump), 1.3, hcp(0..))
}

/// Answerer's reply to the *direct* (no-stopper) cue under the stopper-split
///
/// The cuer denied a stopper in their suit, so 3NT needs *our* own stopper;
/// without it (and without a 4-card-major fit) we run to a minor-suit game
/// rather than a stopperless 3NT. A 4-card unbid major is shown first (the fit).
/// The trailing low-weight 3NT is a guaranteed-finite catch-all (it never wins
/// against the minors, but keeps the node from silently passing the game force).
pub(super) fn cue_stayman_answer_no_stopper(over: Suit) -> Rules {
    let mut rules = Rules::new();
    for major in [Suit::Hearts, Suit::Spades] {
        if major == over {
            continue;
        }
        let m = Strain::from(major);
        rules = rules
            .rule(Bid::new(3, m), 1.6, len(major, 4..) & min_level_is(3, m))
            .rule(Bid::new(4, m), 1.5, len(major, 4..) & min_level_is(4, m));
    }
    // 3NT only with our own stopper (the cuer has none).
    rules = rules.rule(Bid::new(3, Strain::Notrump), 1.45, stopper_in(over));
    // No fit, no stopper → minor-suit game.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        let m = Strain::from(minor);
        rules = rules.rule(Bid::new(4, m), 1.2, len(minor, 4..) & min_level_is(4, m));
    }
    // Guaranteed-finite catch-all (rare: no major, no stopper, no 4-card minor).
    rules.rule(Bid::new(3, Strain::Notrump), 1.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 5c: Transfer over (2♦) — 3♣-Stayman + Smolen, Jacoby transfers
// (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦
// ---------------------------------------------------------------------------

/// Responder's action after our `1NT` and a `(2♦)` overcall, the `(2♦)`-only
/// Smolen leg of the [`LebensohlStyle::Transfer`] package
///
/// `2♦` leaves `3♣` free below the cue, so Stayman moves there (with Smolen after
/// opener's `3♦` denial) and the transfers shift down to direct Jacoby: `3♦`→♥,
/// `3♥`→♠, `3♠`→♣. The major transfers are INV+ and auto-driven to game by
/// [`transfer_completion`]; the `3♠`→♣ leg is a *forced* game-force (its completion
/// is `4♣`, so `3♣` is unplayable). Leaping Michaels `4♦` (both majors) and `4♣`
/// (clubs + a major) show 5-5 game-forcing two-suiters — partner opened `1NT`, so
/// `points(10..)` (≈ 8 HCP after the 5-5 upgrade) already forces game. The weak
/// outlets (natural 2-level, `2NT` relay, penalty double, direct `3NT`) match
/// `Transfer` so the A/B isolates the constructive change.
pub(super) fn transfer_stayman_2d_responder(gate_4333: bool) -> Rules {
    let mut rules = Rules::new();

    // 3♣ = Stayman: game-forcing with *exactly* a 4-card major. A single 5-card
    // major transfers instead; a 5-4 GF hand has its 4-card major here and so comes
    // to Stayman (for Smolen) — hence weight above the transfers, which it also fits.
    rules = rules
        .rule(
            Bid::new(3, Strain::Clubs),
            1.85,
            (len(Suit::Hearts, 4..=4) | len(Suit::Spades, 4..=4))
                & points(10..)
                & competitive_4333_ok(Suit::Diamonds, gate_4333),
        )
        .alert(STAYMAN);

    // Direct Jacoby transfers above their suit (INV+, auto-driven to game).
    rules = rules
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.8,
            len(Suit::Hearts, 5..) & points(9..),
        )
        .alert(LEBENSOHL_TRANSFER)
        .rule(
            Bid::new(3, Strain::Hearts),
            1.8,
            len(Suit::Spades, 5..) & points(9..),
        )
        .alert(LEBENSOHL_TRANSFER);

    // 3♠→clubs: a *forced* game-force with 6+ clubs (its completion is 4♣, so 3♣
    // can never be the contract). Weight below 3NT's, so a 6-club hand *with* a
    // diamond stopper picks 3NT; only the no-stopper hands transfer.
    rules = rules
        .rule(
            Bid::new(3, Strain::Spades),
            1.45,
            len(Suit::Clubs, 6..) & points(10..),
        )
        .alert(LEBENSOHL_TRANSFER);

    // Leaping Michaels: 5-5 game-forcing two-suiters.
    rules = rules
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.0,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(10..),
        )
        .alert(LEAPING_MICHAELS)
        .rule(
            Bid::new(4, Strain::Clubs),
            2.0,
            len(Suit::Clubs, 5..)
                & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                & points(10..),
        )
        .alert(LEAPING_MICHAELS);

    // Weak / to-play outlets — identical to `transfer_lebensohl_responder(Diamonds)`.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.5,
        points(10..) & stopper_in(Suit::Diamonds),
    );
    rules = responder_double(rules, Suit::Diamonds);
    for s in [Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(s);
        rules = rules.rule(
            Bid::new(2, strain),
            1.4,
            min_level_is(2, strain)
                & len(s, 5..)
                & points(..=8)
                & hcp(natural_floor_hcp()..)
                & points(natural_floor_pts()..),
        );
    }
    // Relay shape: 6+ suit, or a 5-carder with the PD-distilled 6-HCP floor,
    // never their diamonds (see [`lebensohl_relay_shape`]).
    let long_suit = lebensohl_relay_shape(Suit::Diamonds);
    rules = rules
        .rule(Bid::new(2, Strain::Notrump), 1.35, points(..=8) & long_suit)
        .alert(LEBENSOHL_RELAY);

    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer to `3♣` Stayman over `(2♦)`: a 4-card major, else `3♦`
///
/// `3♥`/`3♠` shows a 4-card major (hearts first when both); `3♦` denies one,
/// leaving `3♥`/`3♠` free for responder's Smolen. `3♦` is the finite catch-all.
pub(super) fn stayman_2d_answer() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.6, len(Suit::Hearts, 4..))
        .rule(
            Bid::new(3, Strain::Spades),
            1.55,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        )
        .rule(Bid::new(3, Strain::Diamonds), 0.5, hcp(0..))
}

/// Responder's rebid after opener shows a 4-card major over `3♣` Stayman
///
/// Game-forcing already: raise the shown major to game with 4-card support (an
/// eight-card fit), else settle in `3NT` (the finite catch-all).
pub(super) fn stayman_2d_fit_rebid(major: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::from(major)), 1.4, len(major, 4..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's completion of the top-step→clubs transfer (a forced game-force)
///
/// Responder has 6+ clubs, no stopper in `over`, game values. Opener bids `3NT`
/// with a stopper of its own, else raises to `5♣` — `3♣` is unplayable below the
/// top step, so the auction must reach game. (`5♣` is the finite catch-all.)
//
// ponytail: minor-suit slam exploration is left to the floor; 3NT-or-5♣ covers
// the common game. Author a keycard ladder here only if the A/B shows it matters.
pub(super) fn clubs_transfer_completion(over: Suit) -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Notrump), 1.4, stopper_in(over))
        .rule(Bid::new(5, Strain::Clubs), 0.5, hcp(0..))
}

/// Opener's reply to Leaping Michaels `4♦` (both majors, 5-5 game-forcing)
///
/// Bid game in the better major fit, preferring the nine-card fit (4-card
/// support) and breaking ties toward spades. `4♥` is the finite catch-all.
pub(super) fn lm_2d_both_majors_advance() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Spades), 1.6, len(Suit::Spades, 4..))
        .rule(Bid::new(4, Strain::Hearts), 1.55, len(Suit::Hearts, 4..))
        .rule(Bid::new(4, Strain::Spades), 1.5, len(Suit::Spades, 3..))
        .rule(Bid::new(4, Strain::Hearts), 1.0, hcp(0..))
}

/// Opener's reply to Leaping Michaels `4♣` (clubs + an unknown 5+ major)
///
/// `4♦` asks which major; responder names it in [`lm_2d_clubs_major`].
//
// ponytail: opener always relays — the major usually outplays 5♣, and opener's
// final placement (pass the major / correct to 5♣) is left to the floor. Add a
// direct 5♣ sign-off only if the A/B shows the relay costs.
pub(super) fn lm_2d_clubs_ask() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Diamonds), 1.4, hcp(0..))
}

/// Responder names the 5+ major behind a `4♣` Leaping Michaels, over the `4♦` ask
pub(super) fn lm_2d_clubs_major() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 1.5, len(Suit::Hearts, 5..))
        .rule(Bid::new(4, Strain::Spades), 1.5, len(Suit::Spades, 5..))
        .rule(Bid::new(5, Strain::Clubs), 0.5, hcp(0..))
}

/// Responder's first call over `1NT − (2NT both minors)` — Unusual vs Unusual
///
/// `X` is penalty ("I can beat ≥1 of their suits", `hcp(x_floor..)`); the cues
/// find a major fit — `3♣` = INV+ Stayman (a 4-card major) or 5+♠, `3♦` = INV+
/// 5+♥. `4♣`/`4♦` are FG+ 5-5-majors splinters into the short minor (every 5-5
/// hand is short in exactly one minor, so they cover all of them — 5-5 never goes
/// through Stayman). `3♥`/`3♠` are weak natural sign-offs (a clean 6-card major);
/// `3NT` is to play. The [`set_uvu_x_floor`] / [`set_uvu_cue_floor`]
/// knobs sweep the strength ranges. Pass is the finite catch-all.
pub(super) fn uvu_responder() -> Rules {
    let x_floor = uvu_x_floor();
    let cue_floor = uvu_cue_floor();
    let weak = cue_floor.saturating_sub(1); // points cap for the weak naturals
    let both_majors_55 = len(Suit::Spades, 5..) & len(Suit::Hearts, 5..);

    let mut rules = Rules::new();

    // FG+ 5-5-majors splinters: shortness in the named minor (always available —
    // a 5-5 hand holds ≤1 in exactly one minor).
    rules = rules
        .rule(
            Bid::new(4, Strain::Clubs),
            2.0,
            both_majors_55.clone() & len(Suit::Clubs, ..=1) & points(10..),
        )
        .alert(UVU_SPLINTER)
        .rule(
            Bid::new(4, Strain::Diamonds),
            2.0,
            both_majors_55 & len(Suit::Diamonds, ..=1) & points(10..),
        )
        .alert(UVU_SPLINTER);

    // 3♣ = INV+ Stayman (a 4-card major) or 5+♠ (not 5-5); 3♦ = INV+ 5+♥ (≤3♠, so
    // 5♥4♠ prefers 3♣ to hunt the spade fit, and 5-5 is excluded).
    rules = rules
        .rule(
            Bid::new(3, Strain::Clubs),
            1.85,
            ((len(Suit::Spades, 5..) & len(Suit::Hearts, ..=4))
                | len(Suit::Spades, 4..=4)
                | (len(Suit::Hearts, 4..=4) & len(Suit::Spades, ..=3)))
                & points(cue_floor..),
        )
        .alert(UVU_CUE)
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.8,
            len(Suit::Hearts, 5..) & len(Suit::Spades, ..=3) & points(cue_floor..),
        )
        .alert(UVU_CUE);

    // 3NT to play: game values, both minors stopped, no major to pursue.
    rules = rules.rule(
        Bid::new(3, Strain::Notrump),
        1.5,
        points(10..) & stopper_in(Suit::Clubs) & stopper_in(Suit::Diamonds),
    );

    // Penalty X: values *and* a minor we can punish — the (either-or) suit
    // penalty, "I can beat their clubs OR their diamonds". They are 5-5, so a
    // punishing holding is a trick sitting over a 5-card suit: 4+ length, or 4+
    // HCP (AJ/KJ/KQ/AQ) of honors. (Length alone is rare when they hold the
    // minors — honors carry it.) The A/B sweep knob; the suit-specific chase of
    // their actual runout is the encircling follow-up.
    rules = rules.rule(
        Call::Double,
        1.4,
        hcp(x_floor..)
            & (len(Suit::Clubs, 4..)
                | suit_hcp(Suit::Clubs, 4..)
                | len(Suit::Diamonds, 4..)
                | suit_hcp(Suit::Diamonds, 4..)),
    );

    // Weak natural sign-offs: a long major below INV values, to play. The length
    // floor (default 6) drops to 5 to let a five-card major escape when defending
    // the both-minors overcall looks bad (the A/B sweep knob).
    let nat = usize::from(uvu_natural_floor());
    rules = rules
        .rule(
            Bid::new(3, Strain::Hearts),
            1.3,
            len(Suit::Hearts, nat..) & points(..=weak),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            1.3,
            len(Suit::Spades, nat..) & points(..=weak),
        );

    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder's symmetric Smolen after `1NT − (2NT) − 3♣ − (P) − 3♦` — opener
/// denied a 4-card major, so show the 5-card major right-sided into opener
///
/// `3♥` = 5+♠, `3♠` = 5+♥; neither promises the *other* major (opener's `3♦`
/// already killed any 4-4 fit, leaving only the 5-3 hunt). `3NT` is the
/// no-five-card-major catch-all (the plain 4-4 Stayman hand).
pub(super) fn uvu_smolen() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 1.5, len(Suit::Spades, 5..))
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Spades), 1.5, len(Suit::Hearts, 5..))
        .alert(SMOLEN)
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Responder's rebid after `1NT − (2NT) − 3♣ − (P) − 3♥` (opener showed 4 hearts)
///
/// Raise to `4♥` with a fit; with 5+♠ and no heart fit show the spades (`3♠`,
/// opener places); else `3NT` (the finite catch-all).
pub(super) fn uvu_rebid_over_3h() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 1.5, len(Suit::Hearts, 4..))
        .rule(Bid::new(3, Strain::Spades), 1.4, len(Suit::Spades, 5..))
        .rule(Bid::new(3, Strain::Notrump), 0.5, hcp(0..))
}

/// Opener's coded reply after the opponents double our 2♣ Stayman
/// (`1NT-(P)-2♣-(X)`)
///
/// The `(X)` is lead-directing clubs, so the *pass-denies-stopper* scheme spends
/// the free pass on a club-stopper signal: a 4-card major (`2♥`/`2♠`) or `2♦`
/// (no major) promises a club stopper; **Pass denies one** (it may still hide a
/// major, shown after responder re-asks); `XX` is business clubs (offer to play
/// 2♣ doubled-redoubled).  Direct XX is business — distinct from responder's
/// SOS/re-ask XX below.
fn stayman_doubled_opener() -> Rules {
    Rules::new()
        .rule(
            Call::Redouble,
            1.0,
            len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..),
        )
        .rule(
            Bid::new(2, Strain::Hearts),
            1.0,
            len(Suit::Hearts, 4..) & stopper_in(Suit::Clubs),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4) & stopper_in(Suit::Clubs),
        )
        .rule(
            Bid::new(2, Strain::Diamonds),
            0.5,
            len(Suit::Hearts, ..4) & len(Suit::Spades, ..4) & stopper_in(Suit::Clubs),
        )
        .rule(Call::Pass, 0.25, !stopper_in(Suit::Clubs))
}

/// Responder's re-ask after opener passed our doubled Stayman to deny a club
/// stopper (`1NT-(P)-2♣-(X)-P-(P)`)
///
/// Balancing XX is SOS, not business: `XX` re-asks Stayman (forcing — responder
/// still holds the 4-card major), and opener must answer (`stayman_answers`, no
/// Pass).  An owning Pass is the always-mass catch-all.
fn stayman_redouble_reask() -> Rules {
    Rules::new()
        .rule(
            Call::Redouble,
            1.0,
            len(Suit::Hearts, 4..) | len(Suit::Spades, 4..),
        )
        .alert(STAYMAN_REDOUBLE)
        .rule(Call::Pass, 0.1, hcp(0..))
}

/// Opener's natural reply after the opponents overcall our 2♣ Stayman at the
/// 2-level (`1NT-(P)-2♣-(2♦/2♥/2♠)`)
///
/// Show the 4-card major if it outranks their suit; else `X` shows length in
/// their suit (cards/penalty — and, when they overcalled the very major opener
/// holds, the major opener could not bid); else Pass.  Responder stays captain.
fn stayman_overcalled_opener(over: Suit) -> Rules {
    let mut rules = Rules::new();
    if (Suit::Hearts as u8) > (over as u8) {
        rules = rules.rule(Bid::new(2, Strain::Hearts), 1.0, len(Suit::Hearts, 4..));
    }
    if (Suit::Spades as u8) > (over as u8) {
        rules = rules.rule(
            Bid::new(2, Strain::Spades),
            1.0,
            len(Suit::Spades, 4..) & len(Suit::Hearts, ..4),
        );
    }
    rules
        .rule(Call::Double, 0.6, len(over, 4..))
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// Opener's reply after the opponents double our Jacoby transfer
/// (`1NT-(P)-2♦/2♥-(X)`)
///
/// The transfer is still a command, but the `(X)` buys opener a meaningful pass:
/// **complete** (bid `major`) with three-card support, **jump super-accept**
/// (`3-major`) with four and a maximum, **Pass** with a doubleton (declines —
/// responder re-asks below), or `XX` when the doubled transfer suit (`bid`) is
/// opener's own and it wants to defend.
fn transfer_doubled_opener(major: Suit, bid: Suit) -> Rules {
    let strain = Strain::from(major);
    let mut rules = Rules::new();
    if transfer_super_accept() {
        rules = rules.rule(Bid::new(3, strain), 1.5, len(major, 4..) & hcp(17..));
    }
    rules
        .rule(Bid::new(2, strain), 1.0, len(major, 3..))
        .rule(Call::Redouble, 0.6, len(bid, 5..) & suit_hcp(bid, 5..))
        .rule(Call::Pass, 0.25, len(major, ..3))
}

/// Responder's re-ask after opener passed our doubled transfer
/// (`1NT-(P)-2♦/2♥-(X)-P-(P)`)
///
/// Opener's pass declined the transfer; responder still holds the five-card
/// major, so `XX` insists opener complete (forcing — opener answers with
/// [`complete_transfer`], no Pass).  An owning Pass is the catch-all.
fn transfer_pass_reask(major: Suit) -> Rules {
    Rules::new()
        .rule(Call::Redouble, 1.0, len(major, 5..))
        .alert(TRANSFER_REDOUBLE)
        .rule(Call::Pass, 0.1, hcp(0..))
}

/// Opener's reply after the opponents overcall our Jacoby transfer
/// (`1NT-(P)-2♦/2♥-(overcall)`)
///
/// Super-accept the `major` at the cheapest level above their `over_suit` with
/// four-card support; else `X` shows length in their suit (cards); else Pass.
/// Responder stays captain.
fn transfer_overcalled_opener(major: Suit, over_suit: Suit, over_level: u8) -> Rules {
    let strain = Strain::from(major);
    let lvl = if strain > Strain::from(over_suit) {
        over_level
    } else {
        over_level + 1
    };
    Rules::new()
        .rule(
            Bid::new(lvl, strain),
            1.0,
            min_level_is(lvl, strain) & len(major, 4..),
        )
        .rule(Call::Double, 0.6, len(over_suit, 4..))
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// Opener's coded reply after the opponents double our two-way 2♠
/// (`1NT-(P)-2♠-(X)`)
///
/// Their `X` is lead-directing spades, so opener answers the size-ask *and* shows
/// a spade stopper in one call: `2NT`/`3♣` keep their uncontested min/max meaning
/// and promise a stopper (responder then plays the rebased systems-on tree), while
/// `Pass`/`XX` deny a stopper for the minimum/maximum respectively (responder signs
/// off in clubs below).
fn minor_doubled_opener() -> Rules {
    Rules::new()
        // Maximum + spade stopper: the uncontested `3♣` max answer.
        .rule(
            Bid::new(3, Strain::Clubs),
            1.0,
            hcp(17..) & stopper_in(Suit::Spades),
        )
        // Minimum + spade stopper: the uncontested `2NT` min answer.
        .rule(Bid::new(2, Strain::Notrump), 0.9, stopper_in(Suit::Spades))
        // Maximum, no stopper: `XX`.
        .rule(Call::Redouble, 0.8, hcp(17..))
        // Minimum, no stopper: `Pass`.
        .rule(Call::Pass, 0.25, hcp(0..))
}

/// Responder's placement after opener denied a spade stopper over our doubled 2♠
/// (`1NT-(P)-2♠-(X)-P-(P)` minimum, or `…-XX-(P)` maximum)
///
/// Opener has shown min/max but no stopper, so notrump is off; the six-card club
/// hand signs off in `3♣`.  Pass is the catch-all — the balanced-invite hand has no
/// safe spot and defends the doubled 2♠ (rare; the convention is opt-in).
//
// ponytail: the invite hand passing 2♠-doubled is the known soft spot; refine only
// if an A/B says the no-stopper branch leaks.
fn minor_no_stopper_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Clubs), 0.8, len(Suit::Clubs, 6..))
        .rule(Call::Pass, 0.1, hcp(0..))
}

/// Opener's reply after the opponents overcall our two-way 2♠ at `2NT` or `3♣` —
/// the bids that steal opener's size-ask steps (`1NT-(P)-2♠-(2NT/3♣)`)
///
/// Keep the min/max + stopper signal alive in the room that remains: `3NT` =
/// maximum with a spade stopper (to play), `X` = maximum without one (penalty /
/// values), `Pass` = minimum.
fn minor_overcalled_high() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(17..) & stopper_in(Suit::Spades),
        )
        .rule(Call::Double, 0.7, hcp(17..))
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// Opener's systems-off reply after the opponents overcall our two-way 2♠ above
/// `3♣` (`1NT-(P)-2♠-(3♦/3♥/3♠)`)
///
/// Their suit is too high to keep the size-ask, so opener falls back to natural
/// competition: `X` shows length in their suit (cards), else Pass and leave
/// responder captain.
fn minor_overcalled_low(over: Suit) -> Rules {
    Rules::new()
        .rule(Call::Double, 0.6, len(over, 4..))
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// Opener's reply after the opponents double our 2NT diamond transfer
/// (`1NT-(P)-2NT-(X)`)
///
/// `Pass` now carries the "no diamond fit" message (the uncontested job of `3♣`),
/// so opener's `3♣` is freed to be natural 4+♣ (finding responder's 5♦-4♣ fit):
/// `3♦` = accept with 3+♦, `3♣` = no fit but 4+♣, `XX` = maximum values (no fit,
/// penalty-oriented), `Pass` = minimum catch-all.
fn diamond_doubled_opener() -> Rules {
    Rules::new()
        // Accept the transfer with a diamond fit — primary.
        .rule(Bid::new(3, Strain::Diamonds), 1.0, len(Suit::Diamonds, 3..))
        // No fit but real clubs: natural, lands responder's 5♦-4♣ in the club fit.
        .rule(
            Bid::new(3, Strain::Clubs),
            0.7,
            len(Suit::Diamonds, ..3) & len(Suit::Clubs, 4..),
        )
        // Maximum without a fit: redouble shows values (penalty-oriented).
        .rule(Call::Redouble, 0.6, hcp(17..))
        // Catch-all: minimum, no fit, no clubs.
        .rule(Call::Pass, 0.25, hcp(0..))
}

/// Responder's signoff after opener denied a diamond fit over our doubled 2NT
/// (`1NT-(P)-2NT-(X)-P-(P)` minimum, or `…-XX-(P)` maximum)
///
/// Responder always holds 5+♦ from the transfer, so pull to `3♦` rather than
/// languish in a doubled 2NT; Pass is a near-dead catch-all.
//
// ponytail: a strong responder bidding game over opener's XX is the rare soft
// spot left to the floor — refine only if an A/B says this branch leaks.
fn diamond_no_fit_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 0.8, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 0.1, hcp(0..))
}

/// Opener's reply after the opponents overcall our 2NT diamond transfer at `3♣`
/// (the one overcall that leaves the `3♦` completion legal)
fn diamond_overcalled_low() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 1.0, len(Suit::Diamonds, 3..))
        .rule(Call::Double, 0.6, len(Suit::Clubs, 4..))
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// Opener's reply after the opponents overcall our 2NT diamond transfer above `3♣`
/// (`3♦` cue / `3♥` / `3♠` — the `3♦` completion is gone)
///
/// `3NT` = maximum with a stopper in their suit (to play), `X` = length in their
/// suit (penalty), else Pass and leave responder captain.
fn diamond_overcalled_high(over: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Notrump),
            1.0,
            hcp(17..) & stopper_in(over),
        )
        .rule(Call::Double, 0.6, len(over, 4..))
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// The competitive package over our openings: cue-bid raises, preemptive raises,
/// negative doubles for all four openings, support doubles/redoubles, and
/// opener's answers to negative doubles of minor overcalls
///
/// Standalone, the system-on rebase has nothing to land on; bind through
/// [`Pair::against`][crate::bidding::Pair::against] (as [`american`][super::american] is meant to be
/// used) so it resolves into the uncontested core.
#[must_use]
pub fn competition() -> Competitive {
    let mut book = Competitive::new();

    // Section 1 & 2: over all four openings, attach direct-seat response rules
    // and system-on over their double.
    for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let opening_call = call(1, Strain::from(opening));
        fallback_all_seats(
            &mut book,
            &[opening_call],
            3,
            Arc::new(OvercallAtMost(Bid::new(2, Strain::Spades))),
            Fallback::classify(over_their_overcall(opening)),
        );
        fallback_all_seats(
            &mut book,
            &[opening_call],
            3,
            Arc::new(FirstIs(Call::Double)),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
    }

    // Section 3: support doubles and redoubles for each (minor, major) pair.
    for minor in [Suit::Clubs, Suit::Diamonds] {
        for major in [Suit::Hearts, Suit::Spades] {
            let suffix = [
                call(1, Strain::from(minor)),
                Call::Pass,
                call(1, Strain::from(major)),
            ];
            let just_below = if major == Suit::Hearts {
                Bid::new(2, Strain::Diamonds)
            } else {
                Bid::new(2, Strain::Hearts)
            };

            // Support double: they overcall at most `just_below`
            fallback_all_seats(
                &mut book,
                &suffix,
                3,
                Arc::new(OvercallAtMost(just_below)),
                Fallback::classify(support_rules(major)),
            );

            // Support redouble: they doubled
            fallback_all_seats(
                &mut book,
                &suffix,
                3,
                Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                    suffix == [Call::Double]
                })),
                Fallback::classify({
                    let m = Strain::from(major);
                    Rules::new()
                        .rule(Call::Redouble, 1.5, support(3..=3))
                        .alert(SUPPORT_DOUBLE)
                        .rule(Bid::new(2, m), 1.4, support(4..))
                        .rule(Call::Pass, 0.0, hcp(0..))
                }),
            );
        }
    }

    // Section 4: opener answers partner's negative double of a two-level minor.
    // Suffix is [1M]; guard checks that suffix is [2m, X, P].
    for major in [Suit::Hearts, Suit::Spades] {
        fallback_all_seats(
            &mut book,
            &[call(1, Strain::from(major))],
            3,
            Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                matches!(
                    suffix,
                    [Call::Bid(b), Call::Double, Call::Pass]
                        if b.level.get() == 2
                            && (b.strain == Strain::Clubs || b.strain == Strain::Diamonds)
                )
            })),
            Fallback::classify(answer_neg_double_of_minor(major)),
        );
    }

    // Section 4b: opener answers partner's cue-raise of the opening major. Suffix
    // is [1M]; the guard checks the calls after it are [ovc, cue, P] where the cue
    // bids the overcaller's suit (higher than the overcall) and the overcall is at
    // most 2♠ (matching the cue-raise's authored ceiling in `over_their_overcall`).
    // The `ovc.strain != trump` clause excludes the opponents cue-bidding *our*
    // major (e.g. a Michaels `1♠-(2♠)`): there responder's `3♠` is a natural raise,
    // not a cue-raise, and this table must not hijack it.  Majors only. Without
    // this the cue-raise falls through to the keyless floor — whose raise ladder
    // needs partner's *named* and *shown* suit to agree, which a cue decouples — so
    // opener passes and the cuebid is left in as the contract.
    if cue_raise_answer() {
        for major in [Suit::Hearts, Suit::Spades] {
            let trump = Strain::from(major);
            fallback_all_seats(
                &mut book,
                &[call(1, trump)],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    matches!(
                        suffix,
                        [Call::Bid(ovc), Call::Bid(cue), Call::Pass]
                            if cue.strain == ovc.strain
                                && cue > ovc
                                && *ovc <= Bid::new(2, Strain::Spades)
                                && ovc.strain != trump
                    )
                })),
                Fallback::classify(answer_cue_raise(major)),
            );
        }
    }

    // Section 4c: the minor twin of 4b. A minor-opening cue-raise passes out the
    // same way. The cue may sit as high as `3♠` (a 2-level overcall forces the
    // cue to the 3 level), so the ceiling is `cue <= 3♠` rather than `ovc <= 2♠`.
    // `ovc.strain != trump` again excludes a cue of our own minor (e.g. Michaels
    // `1♣-(2♣)` showing the majors — responder's `3♣` there is a raise).
    if cue_minor_raise_answer() {
        for minor in [Suit::Clubs, Suit::Diamonds] {
            let trump = Strain::from(minor);
            fallback_all_seats(
                &mut book,
                &[call(1, trump)],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    matches!(
                        suffix,
                        [Call::Bid(ovc), Call::Bid(cue), Call::Pass]
                            if cue.strain == ovc.strain
                                && cue > ovc
                                && *cue <= Bid::new(3, Strain::Spades)
                                && ovc.strain != trump
                    )
                })),
                Fallback::classify(answer_cue_minor_raise(minor)),
            );
        }
    }

    // Section 5: Lebensohl after our 1NT is overcalled at the 2 level. Purely
    // additive — nothing else lands at [1NT] in the competitive book. Plain or
    // Transfer Lebensohl per [`LebensohlStyle`]; both keep the weak 2NT relay.
    let style = lebensohl_style();
    if style != LebensohlStyle::Off {
        let one_nt = call(1, Strain::Notrump);
        let two_nt = call(2, Strain::Notrump);
        let three_clubs = call(3, Strain::Clubs);

        // Over a natural (2♣) overcall we play *systems on*, not Lebensohl: 2♣
        // steals no room (every transfer/relay still sits above it), so responder
        // keeps the uncontested 1NT structure (Jacoby transfers, minor transfers,
        // the 2NT invite, …) and shows the now-unbiddable 2♣ Stayman with a Double.
        // Rather than re-author all of that, rebase onto the uncontested tree: the
        // (2♣) overcall maps to the opponent's pass, and a Double directly over it
        // maps to the 2♣ Stayman it replaces. (So there is no natural 2♦/2♥/2♠
        // escape over 2♣ — those are transfers.)
        let two_clubs = call(2, Strain::Clubs);
        fallback_all_seats(
            &mut book,
            &[one_nt],
            3,
            Arc::new(FirstIs(two_clubs)),
            Fallback::rebase(rewriter(move |auction: &[Call], depth: usize| {
                if auction.get(depth) != Some(&two_clubs) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth] = Call::Pass; // (2♣) steals no room → systems on
                if auction.get(depth + 1) == Some(&Call::Double) {
                    rewritten[depth + 1] = two_clubs; // stolen 2♣ Stayman = Double
                }
                Some(rewritten)
            })),
        );

        // The rebase routes every *continuation*, but responder must be handed a
        // finite logit on Double to *choose* the stolen Stayman (the rebase only
        // offers the uncontested calls, where 2♣ is illegal here). So classify
        // responder's own call with the uncontested responses, moving the 2♣
        // Stayman logit onto Double: X *is* the stolen 2♣ — same weight, same
        // constraint, nothing to drift if Stayman is retuned. Empty-suffix guard →
        // only responder's first call; deeper calls fall through to the rebase.
        let responses = notrump_responses();
        fallback_all_seats(
            &mut book,
            &[one_nt, two_clubs],
            3,
            Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| suffix.is_empty())),
            Fallback::classify(classifier(move |hand: Hand, context: &Context<'_>| {
                let mut logits = responses.classify(hand, context);
                let stayman = *logits.0.get(two_clubs);
                *logits.0.get_mut(two_clubs) = f32::NEG_INFINITY; // 2♣ is stolen
                *logits.0.get_mut(Call::Double) = stayman; // X inherits 2♣ exactly
                logits
            })),
        );

        // Opener's penalty-pass of that Double: after [1NT, (2♣), X, (P)] opener
        // with good clubs sits to defend 2♣ doubled instead of answering the
        // stolen Stayman. Authored at the same [1NT, 2♣] node as the responder
        // classifier (depth 2), so `resolve_at` reaches it *before* the depth-1
        // systems-on rebase; the disjoint suffix guard ([X, P] vs the responder's
        // empty suffix) keeps the two from colliding. `stayman_answers()` rides
        // along as the always-mass catch-all, so a hand failing the club gate just
        // answers Stayman exactly as the rebase would (no silent pass).
        if let Some((min_len, min_hcp, over_major)) = penalty_pass() {
            let pass_logit = if over_major { 1.5 } else { 0.75 };
            let answers = stayman_answers().rule(
                Call::Pass,
                pass_logit,
                len(Suit::Clubs, min_len..) & suit_hcp(Suit::Clubs, min_hcp..),
            );
            fallback_all_seats(
                &mut book,
                &[one_nt, two_clubs],
                3,
                Arc::new(guard(|_: &Context<'_>, suffix: &[Call]| {
                    suffix == [Call::Double, Call::Pass]
                })),
                Fallback::classify(answers),
            );
        }

        // Lebensohl proper applies only over (2♦/2♥/2♠) — the overcalls that
        // actually steal room. (2♣) is the systems-on rebase above.
        for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let overcall = call(2, Strain::from(over));

            // Responder's first action: the uncovered suffix is exactly their overcall.
            let responder = match style {
                _ if over == Suit::Diamonds && defense_2d_multi() => multi_responder(),
                LebensohlStyle::Transfer if over == Suit::Diamonds => {
                    // gate_4333 = true: our 1NT overcalled, partner is balanced.
                    transfer_stayman_2d_responder(true)
                }
                LebensohlStyle::Transfer => transfer_lebensohl_responder(over, true),
                _ => lebensohl_responder(over),
            };
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    suffix == [overcall]
                })),
                Fallback::classify(responder),
            );

            // Opener's reply to responder's double of the overcall: suffix is
            // [overcall, X, P].  The penalty styles SIT (else the floor reads it as
            // a takeout advance and pulls — the documented leak); the optional style
            // cooperates (stand on a fit, run with a doubleton); takeout keeps the
            // floor's advance.  Gated on the leave-in knob.
            let opener_reply = match double_style() {
                DoubleStyle::Penalty | DoubleStyle::PenaltyLight => {
                    Some(opener_leaves_in_penalty_double())
                }
                DoubleStyle::Optional => Some(opener_cooperates_optional(over)),
                DoubleStyle::Takeout => None,
            };
            if let (true, Some(reply)) = (penalty_double_leave_in(), opener_reply) {
                fallback_all_seats(
                    &mut book,
                    &[one_nt],
                    3,
                    Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                        suffix == [overcall, Call::Double, Call::Pass]
                    })),
                    Fallback::classify(reply),
                );
            }

            // Opener completes the 2NT relay with 3♣: suffix is [overcall, 2NT, P].
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    suffix == [overcall, two_nt, Call::Pass]
                })),
                Fallback::classify(complete_lebensohl_relay()),
            );

            // Responder's rebid after 3♣ (the weak relay sign-off): suffix is
            // [overcall, 2NT, P, 3♣, P].
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                    suffix == [overcall, two_nt, Call::Pass, three_clubs, Call::Pass]
                })),
                Fallback::classify(lebensohl_relay_rebid(over)),
            );

            // Opener's reply to a weak major sign-off: pass, or stretch to game
            // with a maximum + fit (see [`lebensohl_signoff_raise`]). Suffix is
            // [overcall, 2NT, P, 3♣, P, 3M, P]. Only a major *below* the overcall
            // is reachable via the relay — a higher major is bid naturally at the
            // 2 level — so in practice this wires only (2♠)→3♥.
            for signoff in [Suit::Hearts, Suit::Spades] {
                if (signoff as u8) >= (over as u8) {
                    continue;
                }
                let three_m = call(3, Strain::from(signoff));
                fallback_all_seats(
                    &mut book,
                    &[one_nt],
                    3,
                    Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                        suffix
                            == [
                                overcall,
                                two_nt,
                                Call::Pass,
                                three_clubs,
                                Call::Pass,
                                three_m,
                                Call::Pass,
                            ]
                    })),
                    Fallback::classify(lebensohl_signoff_raise(signoff, 6)),
                );
            }

            // Floored natural escape (only under [`set_natural_floor`]): opener's
            // reply to a *direct* natural major sign-off — the one-level-lower
            // mirror of the relay sign-off raise above. Suffix is [overcall, 2M, P]
            // where 2M is a major *above* the overcall (a weak 5-card-suit hand
            // bids it naturally rather than relaying). Same
            // [`lebensohl_signoff_raise`], but fed the natural floor (5, not the
            // relay's 6) so opener's game bar is one point higher to compensate.
            if natural_floor_on() {
                for signoff in [Suit::Hearts, Suit::Spades] {
                    if (signoff as u8) <= (over as u8) {
                        continue; // not above the overcall — no 2-level natural
                    }
                    let two_m = call(2, Strain::from(signoff));
                    fallback_all_seats(
                        &mut book,
                        &[one_nt],
                        3,
                        Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                            suffix == [overcall, two_m, Call::Pass]
                        })),
                        Fallback::classify(lebensohl_signoff_raise(signoff, natural_floor_hcp())),
                    );
                }
            }

            // Plain style: opener's reply to the direct cue (Stayman). Suffix is
            // [overcall, 3X, P] where 3X is the cue of their suit. (Transfer wires
            // its cue reply in the block below.)
            if style == LebensohlStyle::Plain {
                let cue = call(3, Strain::from(over));
                fallback_all_seats(
                    &mut book,
                    &[one_nt],
                    3,
                    Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                        suffix == [overcall, cue, Call::Pass]
                    })),
                    Fallback::classify(cue_stayman_answer(over)),
                );
            }

            // Transfer style: opener's reply to each 3-level transfer / cue.
            // Suffix is [overcall, 3X, P] where 3X is responder's transfer or cue.
            // Over (2♦) the Smolen block below owns the 3-level replies, so this
            // covers (2♥)/(2♠)/(2♣) only.
            if style == LebensohlStyle::Transfer && over != Suit::Diamonds {
                for bid_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    let resp = call(3, Strain::from(bid_suit));
                    let reply = if bid_suit == over {
                        cue_stayman_answer(over)
                    } else if let Some(target) = transfer_target(bid_suit, over) {
                        transfer_completion(target, over)
                    } else if over != Suit::Clubs {
                        clubs_transfer_completion(over) // top step → clubs (forced GF)
                    } else {
                        continue; // over (2♣): clubs is their suit — floored
                    };
                    fallback_all_seats(
                        &mut book,
                        &[one_nt],
                        3,
                        Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                            suffix == [overcall, resp, Call::Pass]
                        })),
                        Fallback::classify(reply),
                    );
                }
            }

            // Recognize a delayed cue (2NT relay, then their suit) over (2♥)/(2♠):
            // Stayman with a stopper, answered like the direct cue but with 3NT
            // safe. Always wired so a human partner who plays it gets a sensible
            // reply, even though the bot only *bids* it under `set_delayed_cue`.
            if style == LebensohlStyle::Transfer && unbid_major(over).is_some() {
                let cue = call(3, Strain::from(over));
                fallback_all_seats(
                    &mut book,
                    &[one_nt],
                    3,
                    Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                        suffix
                            == [
                                overcall,
                                two_nt,
                                Call::Pass,
                                three_clubs,
                                Call::Pass,
                                cue,
                                Call::Pass,
                            ]
                    })),
                    Fallback::classify(cue_stayman_answer(over)),
                );
            }

            // Section 5c: Transfer over (2♦) — 3♣-Stayman + Smolen, the Jacoby
            // transfers (3♦→♥, 3♥→♠, 3♠→♣), and Leaping Michaels 4♣/4♦.
            // (The 2♥/2♠/2♣ branches reuse the Transfer completions above.)
            if style == LebensohlStyle::Transfer && over == Suit::Diamonds {
                let p = Call::Pass;
                let c3 = call(3, Strain::Clubs);
                let d3 = call(3, Strain::Diamonds);
                let h3 = call(3, Strain::Hearts);
                let s3 = call(3, Strain::Spades);
                let c4 = call(4, Strain::Clubs);
                let d4 = call(4, Strain::Diamonds);
                let nodes: Vec<(Vec<Call>, Rules)> = vec![
                    // 3♣ Stayman, opener's answer; then Smolen after the 3♦ denial.
                    (vec![overcall, c3, p], stayman_2d_answer()),
                    (vec![overcall, c3, p, d3, p], smolen_at_three()),
                    (
                        vec![overcall, c3, p, d3, p, h3, p],
                        smolen_completion(Suit::Spades),
                    ),
                    (
                        vec![overcall, c3, p, d3, p, s3, p],
                        smolen_completion(Suit::Hearts),
                    ),
                    // Opener showed a 4-card major over Stayman; responder places.
                    (
                        vec![overcall, c3, p, h3, p],
                        stayman_2d_fit_rebid(Suit::Hearts),
                    ),
                    (
                        vec![overcall, c3, p, s3, p],
                        stayman_2d_fit_rebid(Suit::Spades),
                    ),
                    // Jacoby transfers: 3♦→♥, 3♥→♠ (auto-driven), 3♠→♣ (forced GF).
                    (
                        vec![overcall, d3, p],
                        transfer_completion(Suit::Hearts, over),
                    ),
                    (
                        vec![overcall, h3, p],
                        transfer_completion(Suit::Spades, over),
                    ),
                    (vec![overcall, s3, p], clubs_transfer_completion(over)),
                    // Leaping Michaels: 4♦ both majors, 4♣ clubs + a major (ask).
                    (vec![overcall, d4, p], lm_2d_both_majors_advance()),
                    (vec![overcall, c4, p], lm_2d_clubs_ask()),
                    (vec![overcall, c4, p, d4, p], lm_2d_clubs_major()),
                ];
                for (suffix, rules) in nodes {
                    fallback_all_seats(
                        &mut book,
                        &[one_nt],
                        3,
                        Arc::new(guard(move |_: &Context<'_>, s: &[Call]| {
                            s == suffix.as_slice()
                        })),
                        Fallback::classify(rules),
                    );
                }
            }
        }
    }

    // Competition over our own 2♣ Stayman (`set_competition_over_stayman`,
    // default on): opener's replies after the opponents double `1NT-(P)-2♣-(X)`
    // or overcall it `-(2♦/2♥/2♠)`.  Keyed at the `[1NT, P, 2♣]` node — a distinct
    // trie path from the systems-on `[1NT, (2♣)]` block (their 2♣ at depth 1).
    if competition_over_stayman() {
        let stayman = [call(1, Strain::Notrump), Call::Pass, call(2, Strain::Clubs)];

        // A.1 — our Stayman doubled.  Opener's coded reply (suffix `[X]`).
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| s == [Call::Double])),
            Fallback::classify(stayman_doubled_opener()),
        );
        // After opener's *stopper-bid* (suffix `[X, <bid>, …]`) responder's rebids
        // are identical to the uncontested tree: rebase by stripping the X to a
        // Pass, re-keying onto `[1NT, P, 2♣, P, <bid>, …]`.
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
            })),
            Fallback::rebase(rewriter(move |auction: &[Call], depth: usize| {
                if auction.get(depth) != Some(&Call::Double) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth] = Call::Pass; // strip the X → systems on
                Some(rewritten)
            })),
        );
        // Opener passed to deny a stopper; responder re-asks (suffix `[X, P, P]`).
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                s == [Call::Double, Call::Pass, Call::Pass]
            })),
            Fallback::classify(stayman_redouble_reask()),
        );
        // Opener's forced re-answer to the re-ask (suffix `[X, P, P, XX, P]`):
        // reuse `stayman_answers()` — no Pass rule (opener cannot sit), and its 2♦
        // is exactly the artificial "no major" denial.
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                s == [
                    Call::Double,
                    Call::Pass,
                    Call::Pass,
                    Call::Redouble,
                    Call::Pass,
                ]
            })),
            Fallback::classify(stayman_answers()),
        );

        // A.1c — opener's 2-level answer (2♦/2♥/2♠) doubled.  The double of the
        // artificial answer steals no room (responder's escapes all sit above 2♦),
        // so responder's rebids are systems-on: strip the X to a Pass and re-key onto
        // the uncontested `[1NT, P, 2♣, P, <answer>, …]` tree.  This is the escape the
        // invitational-5-4 reroute needs — a 5♠4♥ that Staymaned bids its 2♠ instead
        // of sitting for a doubled 2♦ — and it also lets a 4-4 hand run to 2NT rather
        // than passing the double out.  Suffix `[P, <2-bid>, X, …]`.
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                s.first() == Some(&Call::Pass)
                    && matches!(
                        s.get(1),
                        Some(Call::Bid(b))
                            if b.level.get() == 2
                                && matches!(
                                    b.strain,
                                    Strain::Diamonds | Strain::Hearts | Strain::Spades
                                )
                    )
                    && s.get(2) == Some(&Call::Double)
            })),
            Fallback::rebase(rewriter(move |auction: &[Call], depth: usize| {
                if auction.get(depth + 2) != Some(&Call::Double) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth + 2] = Call::Pass; // strip the X → systems on
                Some(rewritten)
            })),
        );

        // A.2 — our Stayman overcalled at the 2-level.  Opener's natural reply.
        for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let overcall = call(2, Strain::from(over));
            fallback_all_seats(
                &mut book,
                &stayman,
                3,
                Arc::new(guard(move |_: &Context<'_>, s: &[Call]| s == [overcall])),
                Fallback::classify(stayman_overcalled_opener(over)),
            );
        }
    }

    // Competition over our own Jacoby transfers (`set_competition_over_transfer`,
    // default on): opener's replies after the opponents double `1NT-(P)-2♦/2♥-(X)`
    // or overcall it.  Keyed at the `[1NT, P, 2♦]` / `[1NT, P, 2♥]` nodes — distinct
    // trie paths from the Transfer-Lebensohl `[1NT, (2♦/2♥)]` block (theirs at depth 1).
    if competition_over_transfer() {
        for (resp, major) in [(Suit::Diamonds, Suit::Hearts), (Suit::Hearts, Suit::Spades)] {
            let transfer = [
                call(1, Strain::Notrump),
                Call::Pass,
                call(2, Strain::from(resp)),
            ];

            // Our transfer doubled.  Opener's reply (suffix `[X]`).
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(guard(|_: &Context<'_>, s: &[Call]| s == [Call::Double])),
                Fallback::classify(transfer_doubled_opener(major, resp)),
            );
            // After opener completes/super-accepts (suffix `[X, <bid>, …]`)
            // responder's rebids match the uncontested tree: strip the X to a Pass,
            // re-keying onto `[1NT, P, 2♦/2♥, P, <bid>, …]`.
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                    s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                })),
                Fallback::rebase(rewriter(move |auction: &[Call], depth: usize| {
                    if auction.get(depth) != Some(&Call::Double) {
                        return None;
                    }
                    let mut rewritten = auction.to_vec();
                    rewritten[depth] = Call::Pass; // strip the X → systems on
                    Some(rewritten)
                })),
            );
            // Opener passed to decline; responder re-asks (suffix `[X, P, P]`).
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                    s == [Call::Double, Call::Pass, Call::Pass]
                })),
                Fallback::classify(transfer_pass_reask(major)),
            );
            // Opener's forced completion after the re-ask (suffix `[X, P, P, XX, P]`):
            // reuse `complete_transfer` — no Pass rule, so opener cannot sit.
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                    s == [
                        Call::Double,
                        Call::Pass,
                        Call::Pass,
                        Call::Redouble,
                        Call::Pass,
                    ]
                })),
                Fallback::classify(complete_transfer(major)),
            );

            // Our transfer overcalled.  Opener's natural reply (suffix `[overcall]`).
            let overcalls: &[(Suit, u8)] = match resp {
                Suit::Diamonds => &[(Suit::Spades, 2), (Suit::Clubs, 3), (Suit::Diamonds, 3)],
                _ => &[(Suit::Clubs, 3), (Suit::Diamonds, 3)],
            };
            for &(over_suit, over_level) in overcalls {
                let overcall = call(over_level, Strain::from(over_suit));
                fallback_all_seats(
                    &mut book,
                    &transfer,
                    3,
                    Arc::new(guard(move |_: &Context<'_>, s: &[Call]| s == [overcall])),
                    Fallback::classify(transfer_overcalled_opener(major, over_suit, over_level)),
                );
            }
        }
    }

    // Competition over our own two-way 2♠ minor response (`set_competition_over_
    // minor_transfer`, default off): opener's replies after the opponents double
    // `1NT-(P)-2♠-(X)` or overcall it.  Keyed at `[1NT, P, 2♠]`.  Only the PUPPET
    // 2♠ (clubs *or* the balanced size-ask) has a min/max answer to protect, so the
    // block no-ops under the EUROPEAN pure-transfer scheme.
    if competition_over_minor_transfer() && notrump_minors() == PUPPET {
        let two_spade = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
        ];

        // A.1 — our 2♠ doubled.  Opener's coded min/max + stopper reply (suffix `[X]`).
        fallback_all_seats(
            &mut book,
            &two_spade,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| s == [Call::Double])),
            Fallback::classify(minor_doubled_opener()),
        );
        // After opener's stopper-bid (`2NT`/`3♣`, suffix `[X, <bid>, …]`) responder's
        // rebids match the uncontested tree: strip the `X` to a Pass, re-keying onto
        // `[1NT, P, 2♠, P, 2NT/3♣, …]` (the `two_spade_over_min`/`max` machinery).
        fallback_all_seats(
            &mut book,
            &two_spade,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
            })),
            Fallback::rebase(rewriter(move |auction: &[Call], depth: usize| {
                if auction.get(depth) != Some(&Call::Double) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth] = Call::Pass; // strip the X → systems on
                Some(rewritten)
            })),
        );
        // Opener denied a stopper (Pass = min, suffix `[X, P, P]`; or XX = max, suffix
        // `[X, XX, P]`).  Responder signs off in clubs.
        for deny in [
            [Call::Double, Call::Pass, Call::Pass],
            [Call::Double, Call::Redouble, Call::Pass],
        ] {
            fallback_all_seats(
                &mut book,
                &two_spade,
                3,
                Arc::new(guard(move |_: &Context<'_>, s: &[Call]| s == deny)),
                Fallback::classify(minor_no_stopper_rebid()),
            );
        }

        // A.2 — our 2♠ overcalled.  `2NT`/`3♣` steal the size-ask steps, so opener
        // keeps the min/max + stopper signal (`minor_overcalled_high`); a higher
        // overcall (`3♦/3♥/3♠`) is systems-off (`minor_overcalled_low`).
        let overcalls: [(Call, Rules); 5] = [
            (call(2, Strain::Notrump), minor_overcalled_high()),
            (call(3, Strain::Clubs), minor_overcalled_high()),
            (
                call(3, Strain::Diamonds),
                minor_overcalled_low(Suit::Diamonds),
            ),
            (call(3, Strain::Hearts), minor_overcalled_low(Suit::Hearts)),
            (call(3, Strain::Spades), minor_overcalled_low(Suit::Spades)),
        ];
        for (over, rules) in overcalls {
            fallback_all_seats(
                &mut book,
                &two_spade,
                3,
                Arc::new(guard(move |_: &Context<'_>, s: &[Call]| s == [over])),
                Fallback::classify(rules),
            );
        }
    }

    // Competition over our own 2NT diamond transfer (`set_competition_over_
    // diamond_transfer`, default off): opener's replies after the opponents double
    // `1NT-(P)-2NT-(X)` or overcall it.  Keyed at `[1NT, P, 2NT]`.  Only the PUPPET
    // scheme plays 2NT as the diamond transfer, so the block no-ops under EUROPEAN.
    if competition_over_diamond_transfer() && notrump_minors() == PUPPET {
        let two_nt = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
        ];

        // Our 2NT doubled.  Opener's 3♦-fit / 3♣-clubs / XX-values / Pass reply.
        fallback_all_seats(
            &mut book,
            &two_nt,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| s == [Call::Double])),
            Fallback::classify(diamond_doubled_opener()),
        );
        // After opener's fit-showing bid (`3♦`/`3♣`) responder's rebids match the
        // uncontested tree: strip the `X` to a Pass.
        fallback_all_seats(
            &mut book,
            &two_nt,
            3,
            Arc::new(guard(|_: &Context<'_>, s: &[Call]| {
                s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
            })),
            Fallback::rebase(rewriter(move |auction: &[Call], depth: usize| {
                if auction.get(depth) != Some(&Call::Double) {
                    return None;
                }
                let mut rewritten = auction.to_vec();
                rewritten[depth] = Call::Pass; // strip the X → systems on
                Some(rewritten)
            })),
        );
        // Opener denied a fit (Pass = min, suffix `[X, P, P]`; or XX = max values,
        // suffix `[X, XX, P]`).  Responder signs off in 3♦ (always 5+♦).
        for deny in [
            [Call::Double, Call::Pass, Call::Pass],
            [Call::Double, Call::Redouble, Call::Pass],
        ] {
            fallback_all_seats(
                &mut book,
                &two_nt,
                3,
                Arc::new(guard(move |_: &Context<'_>, s: &[Call]| s == deny)),
                Fallback::classify(diamond_no_fit_rebid()),
            );
        }

        // Our 2NT overcalled.  `3♣` leaves the `3♦` completion legal; a higher
        // overcall (`3♦` cue / `3♥` / `3♠`) keeps `3NT`/`X`/Pass natural.
        let overcalls: [(Call, Rules); 4] = [
            (call(3, Strain::Clubs), diamond_overcalled_low()),
            (
                call(3, Strain::Diamonds),
                diamond_overcalled_high(Suit::Diamonds),
            ),
            (
                call(3, Strain::Hearts),
                diamond_overcalled_high(Suit::Hearts),
            ),
            (
                call(3, Strain::Spades),
                diamond_overcalled_high(Suit::Spades),
            ),
        ];
        for (over, rules) in overcalls {
            fallback_all_seats(
                &mut book,
                &two_nt,
                3,
                Arc::new(guard(move |_: &Context<'_>, s: &[Call]| s == [over])),
                Fallback::classify(rules),
            );
        }
    }

    // Section 5d: Unusual vs Unusual over a both-minors (2NT) overcall of our 1NT
    // (`set_uvu`, default off). Responder's `X` is penalty; `3♣`/`3♦` are
    // INV+ cues (Stayman/5+♠, 5+♥); `4♣`/`4♦` are FG+ 5-5-majors splinters; the
    // `3♣`→`3♦` denial runs symmetric Smolen. Opener's `3♣` answer, the Smolen
    // completions, the splinter advance, and the `3♠` rebid are all shared with
    // the (2♦) Transfer machinery.
    if uvu() {
        let one_nt = call(1, Strain::Notrump);
        let p = Call::Pass;
        let overcall = call(2, Strain::Notrump);
        let c3 = call(3, Strain::Clubs);
        let d3 = call(3, Strain::Diamonds);
        let h3 = call(3, Strain::Hearts);
        let s3 = call(3, Strain::Spades);
        let c4 = call(4, Strain::Clubs);
        let d4 = call(4, Strain::Diamonds);

        // Responder's first action: the uncovered suffix is exactly their 2NT.
        fallback_all_seats(
            &mut book,
            &[one_nt],
            3,
            Arc::new(guard(move |_: &Context<'_>, suffix: &[Call]| {
                suffix == [overcall]
            })),
            Fallback::classify(uvu_responder()),
        );

        let nodes: Vec<(Vec<Call>, Rules)> = vec![
            // 3♣ Stayman/5+♠: opener answers, then symmetric Smolen / fit rebids.
            (vec![overcall, c3, p], stayman_2d_answer()),
            (vec![overcall, c3, p, d3, p], uvu_smolen()),
            (
                vec![overcall, c3, p, d3, p, h3, p],
                smolen_completion(Suit::Spades),
            ),
            (
                vec![overcall, c3, p, d3, p, s3, p],
                smolen_completion(Suit::Hearts),
            ),
            (vec![overcall, c3, p, h3, p], uvu_rebid_over_3h()),
            (
                vec![overcall, c3, p, s3, p],
                stayman_2d_fit_rebid(Suit::Spades),
            ),
            // 3♦ = 5+♥: opener raises with a fit, else 3NT.
            (vec![overcall, d3, p], smolen_completion(Suit::Hearts)),
            // 4♣/4♦ = FG+ 5-5-majors splinters: opener bids the better major game.
            (vec![overcall, c4, p], lm_2d_both_majors_advance()),
            (vec![overcall, d4, p], lm_2d_both_majors_advance()),
        ];
        for (suffix, rules) in nodes {
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(guard(move |_: &Context<'_>, s: &[Call]| {
                    s == suffix.as_slice()
                })),
                Fallback::classify(rules),
            );
        }
    }

    book
}

#[cfg(test)]
mod tests {
    use crate::bidding::Family;
    use crate::bidding::american::american;
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

    /// As [`best_call`], with plain Lebensohl forced on (independent of any other
    /// test on this thread having changed the style)
    fn bid(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_lebensohl_style(super::LebensohlStyle::Plain);
        best_call(auction, hand)
    }

    /// As [`best_call`], with Transfer Lebensohl forced on
    fn bid_transfer(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_lebensohl_style(super::LebensohlStyle::Transfer);
        best_call(auction, hand)
    }

    /// As [`best_call`], with the Unusual-vs-Unusual `(2NT)` structure forced on
    /// at the default A/B floors
    fn bid_uvu(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_uvu(true);
        super::set_uvu_x_floor(9);
        super::set_uvu_cue_floor(8);
        best_call(auction, hand)
    }

    /// As [`best_call`], with our Jacoby-transfer competition + jump super-accept
    /// enabled (both opt-in/default-off after the DD-negative A/B); restores the
    /// defaults so a thread reused by a later test sees them off again.
    fn bid_xfer(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_competition_over_transfer(true);
        crate::bidding::american::set_transfer_super_accept(true);
        let result = best_call(auction, hand);
        super::set_competition_over_transfer(false);
        crate::bidding::american::set_transfer_super_accept(false);
        result
    }

    /// As [`best_call`], with our 2♠ minor-transfer competition (Side A) forced on
    /// (it is also the default, but pin it so a thread that another test left off
    /// still sees it); restores the on default afterward.
    fn bid_minor(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_competition_over_minor_transfer(true);
        let result = best_call(auction, hand);
        super::set_competition_over_minor_transfer(true);
        result
    }

    /// As [`best_call`], with our 2NT diamond-transfer competition (Side A) forced
    /// on (it is also the default, but pin it so a thread that another test left off
    /// still sees it); restores the on default afterward.
    fn bid_diamond(auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_competition_over_diamond_transfer(true);
        let result = best_call(auction, hand);
        super::set_competition_over_diamond_transfer(true);
        result
    }

    // --- Competition over our 2♠ minor transfer (Side A) ---

    #[test]
    fn minor_doubled_opener_shows_min_with_stopper() {
        // 1NT-(P)-2♠-(X): minimum + spade stopper → 2NT.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Double,
        ];
        let (c, floored) = bid_minor(&auction, "KJ2.A32.K432.Q32");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the coded answer must come from the book");
    }

    #[test]
    fn minor_doubled_opener_jumps_max_with_stopper() {
        // 1NT-(P)-2♠-(X): maximum (17) + spade stopper → 3♣.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Double,
        ];
        let (c, floored) = bid_minor(&auction, "KQ2.AQ2.KJ32.A32");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the coded max answer must come from the book");
    }

    #[test]
    fn minor_doubled_opener_passes_min_no_stopper() {
        // 1NT-(P)-2♠-(X): minimum, NO spade stopper → Pass.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Double,
        ];
        let (c, floored) = bid_minor(&auction, "432.AQ2.KQ32.K32");
        assert_eq!(c, Call::Pass);
        assert!(!floored, "the no-stopper pass must come from the book");
    }

    #[test]
    fn minor_doubled_opener_redoubles_max_no_stopper() {
        // 1NT-(P)-2♠-(X): maximum (17), NO spade stopper → XX.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Double,
        ];
        let (c, _) = bid_minor(&auction, "432.AKQ.AQJ2.K32");
        assert_eq!(c, Call::Redouble);
    }

    #[test]
    fn minor_no_stopper_responder_signs_off_in_clubs() {
        // 1NT-(P)-2♠-(X)-P-(P): opener denied a stopper; 6 clubs → 3♣ sign-off.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        let (c, floored) = bid_minor(&auction, "32.43.32.KJ98765");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the club sign-off must come from the book");
    }

    #[test]
    fn minor_overcalled_high_bids_game_with_stopper() {
        // 1NT-(P)-2♠-(2NT): maximum + spade stopper → 3NT (to play).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            call(2, Strain::Notrump),
        ];
        let (c, floored) = bid_minor(&auction, "KQ2.AQ2.KJ32.A32");
        assert_eq!(c, call(3, Strain::Notrump));
        assert!(!floored, "the coded game must come from the book");
    }

    #[test]
    fn minor_overcalled_low_is_systems_off() {
        // 1NT-(P)-2♠-(3♦): systems-off, length in their suit → X (cards).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
            call(3, Strain::Diamonds),
        ];
        let (c, _) = bid_minor(&auction, "K32.K32.AQ32.A32");
        assert_eq!(c, Call::Double);
    }

    // --- Competition over our 2NT diamond transfer (Side A) ---

    #[test]
    fn diamond_doubled_opener_completes_with_a_fit() {
        // 1NT-(P)-2NT-(X): three diamonds → 3♦ (accept the transfer).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Double,
        ];
        let (c, floored) = bid_diamond(&auction, "Axx.Kxx.Qxx.AKxx");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the contested completion must come from the book");
    }

    #[test]
    fn diamond_doubled_opener_bids_natural_clubs() {
        // 1NT-(P)-2NT-(X): doubleton ♦ but 4 clubs → 3♣ (natural, Pass is the
        // catch-all, so 3♣ promises real clubs).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Double,
        ];
        let (c, floored) = bid_diamond(&auction, "AQx.Kxx.xx.AQxx");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the natural 3♣ must come from the book");
    }

    #[test]
    fn diamond_doubled_opener_redoubles_max_no_fit() {
        // 1NT-(P)-2NT-(X): maximum (18), no ♦ fit, no 4-card club → XX (values).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Double,
        ];
        let (c, floored) = bid_diamond(&auction, "AKxx.AQxx.Jx.Axx");
        assert_eq!(c, Call::Redouble);
        assert!(!floored, "the values redouble must come from the book");
    }

    #[test]
    fn diamond_no_fit_responder_signs_off_in_diamonds() {
        // 1NT-(P)-2NT-(X)-P-(P): opener denied a fit; responder pulls to 3♦.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        let (c, floored) = bid_diamond(&auction, "xx.xx.KJxxxx.xxx");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the signoff must come from the book");
    }

    #[test]
    fn diamond_overcalled_low_still_completes() {
        // 1NT-(P)-2NT-(3♣): 3♦ still legal, three diamonds → complete to 3♦.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            call(3, Strain::Clubs),
        ];
        let (c, floored) = bid_diamond(&auction, "Axx.Kxx.Qxx.AKxx");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the completion over 3♣ must come from the book");
    }

    #[test]
    fn diamond_overcalled_high_three_notrump_with_stopper() {
        // 1NT-(P)-2NT-(3♥): no 3♦ left; maximum (18) + heart stopper → 3NT.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            call(3, Strain::Hearts),
        ];
        let (c, floored) = bid_diamond(&auction, "AQx.KJx.Qx.AKxxx");
        assert_eq!(c, call(3, Strain::Notrump));
        assert!(!floored, "the 3NT must come from the book");
    }

    #[test]
    fn diamond_competition_disabled_falls_to_floor() {
        // Off-switch: with the toggle off, 1NT-(P)-2NT-(X) has no Side-A node.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Double,
        ];
        super::set_competition_over_diamond_transfer(false);
        let (_, floored) = best_call(&auction, "Axx.Kxx.Qxx.AKxx");
        super::set_competition_over_diamond_transfer(true); // restore the on default
        assert!(floored, "with the toggle off opener falls to the floor");
    }

    // --- Competition over our 2♣ Stayman (Side A) + defense to theirs (Side B) ---

    #[test]
    fn stayman_doubled_opener_bids_major_with_stopper() {
        // 1NT-(P)-2♣-(X): 4 hearts + a club stopper → 2♥ (the major + stopper).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Double,
        ];
        let (c, floored) = best_call(&auction, "A32.KQ32.A32.K32");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the coded answer must come from the book");
    }

    #[test]
    fn stayman_doubled_opener_passes_without_stopper() {
        // 1NT-(P)-2♣-(X): 4 hearts but NO club stopper → Pass (denies the stopper;
        // the major waits for responder's re-ask).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Double,
        ];
        let (c, floored) = best_call(&auction, "AQ2.KQ32.AQ32.32");
        assert_eq!(c, Call::Pass);
        assert!(!floored, "the stopper-denying pass must come from the book");
    }

    #[test]
    fn stayman_doubled_opener_redoubles_with_clubs() {
        // 1NT-(P)-2♣-(X): five good clubs → XX (business, play 2♣XX).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Double,
        ];
        let (c, _) = best_call(&auction, "A2.K32.A32.KQ876");
        assert_eq!(c, Call::Redouble);
    }

    #[test]
    fn stayman_doubled_reask_is_forcing() {
        // 1NT-(P)-2♣-(X)-P-(P): responder re-asks with XX (4 spades).
        let reask = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        let (c, floored) = best_call(&reask, "KQ32.A32.A32.432");
        assert_eq!(c, Call::Redouble);
        assert!(!floored, "the re-ask must come from the book");
        // …-XX-(P): opener is forced to answer (no Pass), 4 spades → 2♠.
        let answer = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Double,
            Call::Pass,
            Call::Pass,
            Call::Redouble,
            Call::Pass,
        ];
        let (c, floored) = best_call(&answer, "AQ32.K32.KQ2.432");
        assert_eq!(c, call(2, Strain::Spades));
        assert!(!floored, "the forced re-answer must come from the book");
    }

    #[test]
    fn stayman_overcalled_opener_bids_major() {
        // 1NT-(P)-2♣-(2♦): 4 hearts → 2♥ (natural, outranks diamonds).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            call(2, Strain::Diamonds),
        ];
        let (c, floored) = best_call(&auction, "A32.KQ32.K32.A32");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the natural major must come from the book");
    }

    #[test]
    fn stayman_overcalled_opener_doubles_their_suit() {
        // 1NT-(P)-2♣-(2♦): no biddable major, length in diamonds → X (cards).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Clubs),
            call(2, Strain::Diamonds),
        ];
        let (c, _) = best_call(&auction, "K32.K32.AQ32.A32");
        assert_eq!(c, Call::Double);
    }

    #[test]
    fn defense_to_their_stayman_doubles_clubs() {
        // (1NT)-P-(2♣ Stayman): our 4th-hand X = lead-directing clubs (5+ good).
        crate::bidding::american::set_stayman_defense(true);
        let auction = [call(1, Strain::Notrump), Call::Pass, call(2, Strain::Clubs)];
        let (c, floored) = best_call(&auction, "A2.K32.A32.KQ876");
        crate::bidding::american::set_stayman_defense(false); // restore default
        assert_eq!(c, Call::Double);
        assert!(
            !floored,
            "the lead-directing X must come from the defense book"
        );
    }

    // --- Competition over our Jacoby transfers (Side A) + defense to theirs (B) ---

    #[test]
    fn transfer_super_accept_uncontested() {
        // 1NT-P-2♦-P: four hearts + a maximum → 3♥ (jump super-accept).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = bid_xfer(&auction, "A2.KQ32.KQ32.K32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the super-accept must come from the book");
    }

    #[test]
    fn transfer_doubled_opener_completes_with_support() {
        // 1NT-(P)-2♦-(X): three hearts, not a maximum → 2♥ (complete the transfer).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Double,
        ];
        let (c, floored) = bid_xfer(&auction, "KQ2.K32.KQ32.Q32");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the completion must come from the book");
    }

    #[test]
    fn transfer_doubled_opener_super_accepts() {
        // 1NT-(P)-2♦-(X): four hearts + a maximum → 3♥ (the double does not suppress it).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Double,
        ];
        let (c, _) = bid_xfer(&auction, "A2.KQ32.KQ32.K32");
        assert_eq!(c, call(3, Strain::Hearts));
    }

    #[test]
    fn transfer_doubled_opener_passes_with_doubleton() {
        // 1NT-(P)-2♦-(X): only a doubleton heart → Pass (declines; responder re-asks).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Double,
        ];
        let (c, floored) = bid_xfer(&auction, "KQ32.K2.KQ32.Q32");
        assert_eq!(c, Call::Pass);
        assert!(!floored, "the declining pass must come from the book");
    }

    #[test]
    fn transfer_doubled_opener_redoubles_with_the_transfer_suit() {
        // 1NT-(P)-2♦-(X): the doubled diamonds are opener's own (5 to AKQ) → XX.
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Double,
        ];
        let (c, _) = bid_xfer(&auction, "Q43.K2.AKQ32.Q32");
        assert_eq!(c, Call::Redouble);
    }

    #[test]
    fn transfer_doubled_reask_is_forcing() {
        // 1NT-(P)-2♦-(X)-P-(P): responder re-asks with XX (still holds five hearts).
        let reask = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
            Call::Pass,
        ];
        let (c, floored) = bid_xfer(&reask, "K2.QJ432.K32.432");
        assert_eq!(c, Call::Redouble);
        assert!(!floored, "the re-ask must come from the book");
        // …-XX-(P): opener is forced to complete (no Pass) → 2♥.
        let answer = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
            Call::Pass,
            Call::Redouble,
            Call::Pass,
        ];
        let (c, floored) = bid_xfer(&answer, "AQ32.K32.KQ2.432");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the forced completion must come from the book");
    }

    #[test]
    fn transfer_overcalled_opener_super_accepts() {
        // 1NT-(P)-2♦-(2♠): four-card heart fit → 3♥ (cheapest level above their 2♠).
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            call(2, Strain::Spades),
        ];
        let (c, floored) = bid_xfer(&auction, "K2.KQ32.AQ32.K32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the natural super-accept must come from the book");
    }

    #[test]
    fn opener_answers_cue_raise_instead_of_passing() {
        // 1♠ – (2♣) – 3♣ (cue-raise = limit-plus spade raise) – P: opener must not
        // leave the cuebid in. The screenshot deal's East (♠QT743 ♥KQ7 ♦832 ♣A9,
        // 11 HCP — a minimum) declines by signing off in 3♠, from the book.
        let auction = [
            call(1, Strain::Spades),
            call(2, Strain::Clubs),
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (c, floored) = best_call(&auction, "QT743.KQ7.832.A9");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(
            !floored,
            "opener's answer must come from the cue-raise book"
        );
    }

    #[test]
    fn michaels_cue_of_our_major_is_not_a_cue_raise() {
        // 1♠ – (2♠ Michaels, a cue of OUR spades) – 3♠ (responder's NATURAL raise)
        // – P: the cue-raise answer table must not hijack this. A strong opener
        // (this hand tripped the old over-broad guard into a passed-out 4NT) must
        // NOT bid 4NT here.
        let auction = [
            call(1, Strain::Spades),
            call(2, Strain::Spades),
            call(3, Strain::Spades),
            Call::Pass,
        ];
        let (c, _) = best_call(&auction, "AKQT98.Q.AQT73.Q");
        assert_ne!(
            c,
            call(4, Strain::Notrump),
            "a natural spade raise must not be answered as a cue-raise"
        );
    }

    #[test]
    fn opener_answers_minor_cue_raise() {
        // 1♦ – (2♣) – 3♣ (cue-raise = limit-plus diamond raise) – P.
        // Minimum, no club stopper (12 HCP, ♣Q doubleton) → sign off 3♦.
        let auction = [
            call(1, Strain::Diamonds),
            call(2, Strain::Clubs),
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (c, floored) = best_call(&auction, "K43.Q43.AJ632.Q5");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the minor sign-off must come from the book");
        // Values + a club stopper (17 HCP, ♣Kx) → accept the best game, 3NT.
        let (c, floored) = best_call(&auction, "A54.Q43.AKJ32.K5");
        assert_eq!(c, call(3, Strain::Notrump));
        assert!(!floored, "the 3NT accept must come from the book");
    }

    #[test]
    fn minor_cue_raise_decline_jumps_when_3m_is_below_the_cue() {
        // 1♣ – (2♦) – 3♦ (cue-raise = limit-plus club raise) – P: 3♣ now sits
        // *below* the cue and is illegal, so a minimum opener must decline in 4♣,
        // not pass the cuebid out. Guards the 4m fallback rung.
        let auction = [
            call(1, Strain::Clubs),
            call(2, Strain::Diamonds),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = best_call(&auction, "A32.K43.43.KQ432");
        assert_eq!(c, call(4, Strain::Clubs));
        assert!(!floored, "the 4♣ decline must come from the book");
    }

    #[test]
    fn defense_to_their_transfer_doubles_the_bid_suit() {
        // (1NT)-P-(2♦ →♥): our 4th-hand X = lead-directing diamonds (the bid suit).
        crate::bidding::american::set_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
        ];
        let (c, floored) = best_call(&auction, "K2.A32.KQ1054.432");
        crate::bidding::american::set_transfer_defense(false); // restore default
        assert_eq!(c, Call::Double);
        assert!(
            !floored,
            "the lead-directing X must come from the defense book"
        );
    }

    #[test]
    fn defense_to_their_transfer_cues_michaels() {
        // (1NT)-P-(2♦ →♥): 5 spades + 5 diamonds → 2♥ cue (the other major + a minor).
        crate::bidding::american::set_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
        ];
        let (c, floored) = best_call(&auction, "AQ1054.3.KJ1054.32");
        crate::bidding::american::set_transfer_defense(false); // restore default
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the Michaels cue must come from the defense book");
    }

    // --- Defense to their 2♠ minor transfer (Side B) ---

    #[test]
    fn defense_to_their_minor_transfer_doubles_spades() {
        // (1NT)-P-(2♠ minor): our 4th-hand X = lead-directing spades (the bid suit).
        crate::bidding::american::set_minor_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
        ];
        let (c, floored) = best_call(&auction, "KQJ54.A32.432.32");
        crate::bidding::american::set_minor_transfer_defense(false); // restore default
        assert_eq!(c, Call::Double);
        assert!(
            !floored,
            "the lead-directing X must come from the defense book"
        );
    }

    #[test]
    fn defense_to_their_minor_transfer_cues_top_and_bottom() {
        // (1NT)-P-(2♠): 5 spades + 5 diamonds → 3♣ cue (top-and-bottom), beating the X.
        crate::bidding::american::set_minor_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
        ];
        let (c, floored) = best_call(&auction, "KQ1054.3.KJ1054.32");
        crate::bidding::american::set_minor_transfer_defense(false); // restore default
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the top-and-bottom cue must come from the book");
    }

    // --- Defense to their 2NT diamond transfer (Side B) ---

    #[test]
    fn defense_to_their_diamond_transfer_doubles_diamonds() {
        // (1NT)-P-(2NT →♦): our 4th-hand X = lead-directing diamonds (the shown suit).
        crate::bidding::american::set_diamond_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
        ];
        let (c, floored) = best_call(&auction, "A32.32.KQJ54.432");
        crate::bidding::american::set_diamond_transfer_defense(false); // restore default
        assert_eq!(c, Call::Double);
        assert!(
            !floored,
            "the lead-directing X must come from the defense book"
        );
    }

    #[test]
    fn defense_to_their_diamond_transfer_cues_both_majors() {
        // (1NT)-P-(2NT →♦): 5 spades + 5 hearts → 3♦ cue (both majors), beating the X.
        crate::bidding::american::set_diamond_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Notrump),
        ];
        let (c, floored) = best_call(&auction, "KQ1054.KJ1054.3.32");
        crate::bidding::american::set_diamond_transfer_defense(false); // restore default
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the both-majors cue must come from the book");
    }

    #[test]
    fn defense_to_their_minor_transfer_two_notrump_is_reds() {
        // (1NT)-P-(2♠): 5 diamonds + 5 hearts → 2NT (the two lowest unbid suits).
        crate::bidding::american::set_minor_transfer_defense(true);
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Spades),
        ];
        let (c, floored) = best_call(&auction, "3.KQ1054.KJ1054.32");
        crate::bidding::american::set_minor_transfer_defense(false); // restore default
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the red two-suiter must come from the book");
    }

    #[test]
    fn uvu_three_clubs_is_stayman() {
        // 1NT–(2NT both minors): a 4-4 majors hand bids 3♣ (Stayman), a book node.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
        let (c, floored) = bid_uvu(&auction, "AQ32.KJ32.A2.432");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "the cue must come from the book");
    }

    #[test]
    fn uvu_three_diamonds_shows_hearts() {
        // 1NT–(2NT): 5+♥ with ≤3♠ bids 3♦ (the heart cue).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
        let (c, _) = bid_uvu(&auction, "K3.KQ976.A32.432");
        assert_eq!(c, call(3, Strain::Diamonds));
    }

    #[test]
    fn uvu_splinter_with_five_five() {
        // 1NT–(2NT): 5-5 majors short a club → 4♣ splinter (FG+).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
        let (c, _) = bid_uvu(&auction, "AQ876.KJ987.32.A");
        assert_eq!(c, call(4, Strain::Clubs));
    }

    #[test]
    fn uvu_penalty_double_on_values() {
        // 1NT–(2NT): flat values, no 4-card major, no minor stopper → penalty X.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
        let (c, floored) = bid_uvu(&auction, "KJ2.AQ2.J532.532");
        assert_eq!(c, Call::Double);
        assert!(!floored, "the penalty X must come from the book");
    }

    #[test]
    fn uvu_smolen_shows_the_five_card_spade() {
        // 1NT–(2NT)–3♣–(P)–3♦ (denial): responder's 3♥ = Smolen 5+♠ (no ♥ promise).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Notrump),
            call(3, Strain::Clubs),
            Call::Pass,
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = bid_uvu(&auction, "AQ876.K32.A32.32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "Smolen must come from the book");
    }

    #[test]
    fn uvu_disabled_falls_to_floor() {
        // Disabled, 1NT–(2NT) has no book node → instinct floor (the toggle works).
        super::set_uvu(false);
        let auction = [call(1, Strain::Notrump), call(2, Strain::Notrump)];
        let (_, floored) = best_call(&auction, "AQ32.KJ32.A2.432");
        super::set_uvu(true); // restore the default for sibling tests on this thread
        assert!(floored, "without the toggle the auction is unauthored");
    }

    #[test]
    fn uvu_encircling_doubles_the_runout() {
        // 1NT-(2NT)-X, opponents run to 3♣: responder with a club stack doubles
        // (the UvU penalty chase), and partner would leave it in. The chase is
        // the instinct floor's, gated on set_uvu_encircle (read on this thread).
        super::set_uvu(true);
        crate::bidding::instinct::set_uvu_encircle(true);
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Notrump),
            Call::Double,
            call(3, Strain::Clubs),
            Call::Pass,
            Call::Pass,
        ];
        let (c, _) = best_call(&auction, "K54.84.732.KQJT9");
        crate::bidding::instinct::set_uvu_encircle(false); // restore for siblings
        assert_eq!(c, Call::Double, "encircle the 3♣ runout with a club stack");
    }

    #[test]
    fn transfer_smolen_three_clubs_is_stayman() {
        // 1NT–(2♦): a 4-4 majors game-force bids 3♣ Stayman (a book node).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "AQ32.KJ32.A2.432");
        assert_eq!(c, call(3, Strain::Clubs));
        assert!(!floored, "Stayman must come from the book");
    }

    #[test]
    fn transfer_smolen_opener_answers_stayman() {
        // 1NT–(2♦)–3♣: opener shows a 4-card major (3♥ here).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "K2.AQ54.A32.Q432");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn transfer_smolen_three_diamonds_is_the_heart_transfer() {
        // The reshuffle: 1NT–(2♦)–3♦ shows hearts (the freed cue slot), a book node.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "K3.KQ976.A32.432");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the heart transfer must come from the book");

        // Opener auto-drives the INV+ transfer to game with a fit.
        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = bid_transfer(&opener, "AQ5.A432.KQ4.J32");
        assert_eq!(c, call(4, Strain::Hearts));
    }

    #[test]
    fn transfer_smolen_routes_five_four_to_stayman_not_a_transfer() {
        // A 5♠4♥ game-force must bid 3♣ Stayman (1.85), not the 3♥ spade transfer
        // (1.8) — else Smolen could never show the 5-4.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, _) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
        assert_eq!(c, call(3, Strain::Clubs));
    }

    #[test]
    fn transfer_smolen_jumps_smolen_after_the_denial() {
        // 1NT–(2♦)–3♣–P–3♦(no major)–P: responder bids Smolen 3♥ to show 5 spades.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(3, Strain::Clubs),
            Call::Pass,
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "AKJ54.Q432.K2.32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "Smolen must come from the book");

        // Opener completes in the five-card spade game.
        let mut full = auction.to_vec();
        full.push(call(3, Strain::Hearts));
        full.push(Call::Pass);
        let (c, _) = bid_transfer(&full, "Q32.A65.AQ43.K32");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_smolen_leaping_michaels_both_majors() {
        // 1NT–(2♦)–4♦ = both majors 5-5, game-forcing.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "KQ954.AJ876.2.32");
        assert_eq!(c, call(4, Strain::Diamonds));
        assert!(!floored, "Leaping Michaels must come from the book");

        // Opener bids game in the better major (4♠ on three-card support).
        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(4, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = bid_transfer(&opener, "A32.K43.AQ32.Q42");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_smolen_keeps_cohen_over_a_major_overcall() {
        // Over (2♥), Transfer is plain Cohen: 5 spades transfers through
        // hearts — 3♦ shows spades (the Smolen reshuffle is (2♦)-only).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the Cohen transfer must come from the book");
    }

    #[test]
    fn lebensohl_forcing_three_level_is_a_book_node() {
        // 1NT–(2♦); responder 5 spades, game values, no diamond stopper →
        // forcing 3♠ (a jump), not a partscore.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid(&auction, "KQT95.A43.32.J32");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the forcing 3-level bid must come from the book");
    }

    #[test]
    fn lebensohl_weak_long_suit_relays_then_completes() {
        // Weak hand (6 HCP), 6 clubs, over 2♦ → 2NT relay; opener forced to 3♣.
        let responder = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid(&responder, "J2.43.32.KQ9876");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the Lebensohl relay must come from the book");

        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (completion, _) = bid(&opener, "AQ32.KQ5.AQ4.A32");
        assert_eq!(completion, call(3, Strain::Clubs));
    }

    #[test]
    fn lebensohl_weak_bids_natural_two_level() {
        // A weak hand with 5 hearts bids natural 2♥ (below 2NT), to play.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid(&auction, "K2.QJ976.432.432");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the natural 2-level bid must come from the book");
    }

    #[test]
    fn lebensohl_cue_is_stayman() {
        // 1NT–(2♥): a game-force with 4 spades and no 5-card suit cues 3♥ = Stayman
        // (it cannot bid a forcing 3-level suit, and the cue outranks direct 3NT).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (c, floored) = bid(&auction, "AQ32.K43.A32.K32");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the cue must come from the book");

        // Opener answers Stayman with the 4-card spade fit.
        let opener = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (a, floored) = bid(&opener, "KJ54.A32.K43.Q32");
        assert_eq!(a, call(3, Strain::Spades));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn lebensohl_five_card_suit_relays_then_signs_off_at_the_three_level() {
        // Weak hand, a 5-card heart suit it cannot show at the 2 level (below
        // their 2♠): relay 2NT, then correct 3♣→3♥ as a 3-level sign-off.
        let responder = [call(1, Strain::Notrump), call(2, Strain::Spades)];
        let (c, floored) = bid(&responder, "32.KQJ32.432.432");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the relay must come from the book");

        let after_3c = [
            call(1, Strain::Notrump),
            call(2, Strain::Spades),
            call(2, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (c, floored) = bid(&after_3c, "32.KQJ32.432.432");
        assert_eq!(c, call(3, Strain::Hearts));
        assert!(!floored, "the 3-level sign-off must come from the book");
    }

    #[test]
    fn lebensohl_maximum_raises_weak_signoff_to_game() {
        // 1NT–(2♠)–2NT–P–3♣–P–3♥–P: responder's weak heart sign-off. A maximum
        // (17) opener with three-card support stretches to 4♥; a minimum passes.
        let after_signoff = [
            call(1, Strain::Notrump),
            call(2, Strain::Spades),
            call(2, Strain::Notrump),
            Call::Pass,
            call(3, Strain::Clubs),
            Call::Pass,
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (c, floored) = bid(&after_signoff, "AK32.K43.A43.K32");
        assert_eq!(c, call(4, Strain::Hearts));
        assert!(!floored, "the game raise must come from the book");

        let (c, _) = bid(&after_signoff, "AK32.K43.KQ3.432");
        assert_eq!(c, Call::Pass, "a minimum passes the weak sign-off");
    }

    #[test]
    fn transfer_lebensohl_shows_spades_through_their_hearts() {
        // 1NT–(2♥); responder, 5 spades and game values, transfers *through*
        // hearts: 3♦ shows spades (not diamonds), a book node.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (c, floored) = bid_transfer(&auction, "AKQ65.43.K32.J32");
        assert_eq!(c, call(3, Strain::Diamonds));
        assert!(!floored, "the transfer must come from the book");
    }

    #[test]
    fn transfer_lebensohl_opener_bids_game_not_a_partscore() {
        // After 1NT–(2♥)–3♦ (transfer to spades), opener with a fit must bid
        // the spade *game*, never a 3♠ partscore (the Rubensohl-v1 failure).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (c, _) = bid_transfer(&auction, "AK5.KQ52.A43.432");
        assert_eq!(c, call(4, Strain::Spades));
    }

    #[test]
    fn transfer_lebensohl_cue_is_stayman() {
        // 1NT–(2♥)–3♥ is the cue = Stayman; opener answers a 4-card major.
        // (Over (2♦) the cue slot is freed for the Smolen 3♣-Stayman instead.)
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Hearts),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "AQ32.K43.A32.K32");
        assert_eq!(c, call(3, Strain::Spades));
        assert!(!floored, "the Stayman answer must come from the book");
    }

    #[test]
    fn transfer_lebensohl_keeps_the_penalty_double() {
        // Length and values in their suit, no game bid of our own: with the
        // `Penalty` style on, double from the book — Rubensohl v1 lost this by
        // shadowing the floor. (The default is now `Optional` (2-3 cards), which
        // would route this 4-card-diamond hand elsewhere; see
        // [`takeout_authored_double`].)
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) =
            bid_transfer_dbl(super::DoubleStyle::Penalty, &auction, "K2.K43.J932.Q432");
        assert_eq!(c, Call::Double);
        assert!(!floored, "the penalty double must come from the book");
    }

    /// As [`bid_transfer`], with the given double meaning forced on; resets the
    /// double style to the default afterward so it cannot leak across tests on
    /// the same thread.
    fn bid_transfer_dbl(style: super::DoubleStyle, auction: &[Call], hand: &str) -> (Call, bool) {
        super::set_lebensohl_style(super::LebensohlStyle::Transfer);
        super::set_double_style(style);
        let result = best_call(auction, hand);
        super::set_double_style(super::DoubleStyle::default());
        result
    }

    #[test]
    fn takeout_authored_double() {
        // Takeout: short in their suit (2♦) with values doubles from the book —
        // a hand the `Penalty` style (4+ ♦) would never double.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) =
            bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K432.K432.32.Q43");
        assert_eq!(c, Call::Double);
        assert!(
            !floored,
            "the authored takeout double must come from the book"
        );
    }

    #[test]
    fn optional_double_two_three_cards() {
        // Optional: exactly 3 cards in their suit (♦) with values doubles…
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) =
            bid_transfer_dbl(super::DoubleStyle::Optional, &auction, "K43.K43.432.Q43");
        assert_eq!(c, Call::Double);
        assert!(!floored, "the optional double must come from the book");

        // …but a singleton in their suit does NOT double (it routes elsewhere).
        let (c, _) = bid_transfer_dbl(super::DoubleStyle::Optional, &auction, "K432.K432.2.Q432");
        assert_ne!(
            c,
            Call::Double,
            "short-in-their-suit must not make an optional double"
        );
    }

    #[test]
    fn opener_pulls_a_takeout_double() {
        // After 1NT–(2♦)–X–(P), opener has no authored node and falls to the
        // floor: a maximum with a diamond stopper pulls to 3NT…
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        let (c, floored) =
            bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "AQ2.AQ2.A32.Q432");
        assert_eq!(c, call(3, Strain::Notrump));
        assert!(floored, "opener's pull comes from the instinct floor");

        // …while a diamond stack sits for penalty (passes the double).
        let (c, _) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K32.A32.AKQ2.J32");
        assert_eq!(c, Call::Pass, "a trump stack converts to penalty");
    }

    #[test]
    fn transfer_lebensohl_weak_bids_natural_two_level() {
        // Weak 5-card heart hand still bids natural 2♥ (transfers are INV+).
        let auction = [call(1, Strain::Notrump), call(2, Strain::Diamonds)];
        let (c, floored) = bid_transfer(&auction, "K2.QJ976.432.432");
        assert_eq!(c, call(2, Strain::Hearts));
        assert!(!floored, "the natural 2-level bid must come from the book");
    }

    #[test]
    fn transfer_lebensohl_top_step_is_a_clubs_transfer() {
        // The top step (no suit above to transfer into) is a forced game-force
        // transfer to clubs: 6+♣, game values, no stopper in their suit. The same
        // 10-HCP hand bids it over every overcall — 3♠ over (2♦)/(2♥), 3♥ over
        // (2♠) — a book node, never the natural floor. Tested under `Penalty`: the
        // default `Takeout` (≤3 in their suit, 1.55) outranks the clubs transfer
        // (1.45) and would hijack this short-suit hand into a takeout double — a
        // known weight interaction; the structural node is checked here in
        // isolation.
        let hand = "32.543.32.AKQJ86";
        for (over, top) in [
            (Strain::Diamonds, Strain::Spades),
            (Strain::Hearts, Strain::Spades),
            (Strain::Spades, Strain::Hearts),
        ] {
            let auction = [call(1, Strain::Notrump), call(2, over)];
            let (c, floored) = bid_transfer_dbl(super::DoubleStyle::Penalty, &auction, hand);
            assert_eq!(c, call(3, top), "top step → clubs over (2{over:?})");
            assert!(!floored, "the clubs transfer must come from the book");
        }
    }

    #[test]
    fn transfer_lebensohl_traps_a_too_good_stopper() {
        // Over 1NT–(2♥) with game values, a *too-good* heart stopper (♥AQ86, 6
        // HCP in their suit) traps: pass and wait for opener's reopening takeout
        // double, then convert. A merely *adequate* stopper (♥A964, 4 HCP) is a
        // source of tricks and still declares 3NT. (Trap pass on by default.)
        // The trap is a takeout-style mechanism — under the default Penalty style
        // this 4-card-heart hand doubles for penalty directly — so it is pinned to
        // Takeout here; the 3NT line (1.7) outranks any double, so it is style-free.
        let auction = [call(1, Strain::Notrump), call(2, Strain::Hearts)];
        let (trap, _) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K32.AQ86.KJ5.J32");
        assert_eq!(
            trap,
            Call::Pass,
            "a too-good stopper (6 HCP in hearts) traps"
        );
        // Also pinned to Takeout: under Penalty default this 4-card-heart hand
        // prefers the penalty double (1.55) to the relay's direct 3NT (1.5) — four
        // trumps behind declarer beat one fragile stopper, which is sound.
        let (bid, _) = bid_transfer_dbl(super::DoubleStyle::Takeout, &auction, "K32.A964.KJ5.Q32");
        assert_eq!(
            bid,
            call(3, Strain::Notrump),
            "an adequate stopper (4 HCP in hearts) still bids 3NT"
        );
    }

    #[test]
    fn transfer_lebensohl_top_step_opener_completes_at_game() {
        // After 1NT–(2♥)–3♠ (transfer to clubs, forced GF): opener bids 3NT with
        // a heart stopper, else raises to 5♣ — 3♣ is unplayable, so it reaches game.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            call(3, Strain::Spades),
            Call::Pass,
        ];
        let (c, floored) = bid_transfer(&auction, "A432.KQ5.A32.432");
        assert_eq!(c, call(3, Strain::Notrump), "stopper → 3NT");
        assert!(!floored, "the completion must come from the book");

        let (c, _) = bid_transfer(&auction, "A432.543.AKQ.432");
        assert_eq!(c, call(5, Strain::Clubs), "no stopper → 5♣");
    }

    #[test]
    fn opener_leaves_in_responder_penalty_double_when_penalty_style() {
        use super::{DoubleStyle, set_double_style, set_penalty_double_leave_in};
        // [1NT,(2♥),X,(P)] — responder penalty-doubled their heart overcall.
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
        ];
        super::set_lebensohl_style(super::LebensohlStyle::Plain);
        // Penalty style + leave-in on: opener SITS, and it is an authored node.
        set_double_style(DoubleStyle::Penalty);
        set_penalty_double_leave_in(true);
        let (c_on, floored_on) = best_call(&auction, "AQ5.J42.KQ3.K842"); // flat 15, no ♥ stop
        assert_eq!(c_on, Call::Pass, "penalty double left in");
        assert!(
            !floored_on,
            "the leave-in must be a book node, not the floor"
        );
        // Leave-in off: the floor reads the double as takeout and pulls — not a Pass.
        set_penalty_double_leave_in(false);
        let (c_off, floored_off) = best_call(&auction, "AQ5.J42.KQ3.K842");
        assert!(
            floored_off,
            "off → the node is gone, opener falls to the floor"
        );
        assert_ne!(
            c_off,
            Call::Pass,
            "the floor advances the double instead of sitting"
        );
        // Restore the defaults for other tests sharing this thread.
        set_penalty_double_leave_in(true);
        set_double_style(DoubleStyle::Penalty);
    }

    #[test]
    fn opener_cooperates_with_responder_optional_double() {
        use super::{DoubleStyle, set_double_style, set_penalty_double_leave_in};
        // [1NT,(2♥),X,(P)] — responder's OPTIONAL double (2-3 hearts + values).
        let auction = [
            call(1, Strain::Notrump),
            call(2, Strain::Hearts),
            Call::Double,
            Call::Pass,
        ];
        super::set_lebensohl_style(super::LebensohlStyle::Plain);
        set_double_style(DoubleStyle::Optional);
        set_penalty_double_leave_in(true);
        // Three-card fit (♥Q93): stand and defend the doubled overcall.
        let (fit, floored) = best_call(&auction, "AK5.Q93.KJ54.Q5");
        assert_eq!(fit, Call::Pass, "a three-card fit stands");
        assert!(!floored, "the cooperation must be an authored node");
        // Doubleton in their suit + a five-card suit (♣AKQ76): run with xx.
        let (run, _) = best_call(&auction, "A52.93.KJ5.AKQ76");
        assert_eq!(
            run,
            call(3, Strain::Clubs),
            "a doubleton runs to the five-card suit"
        );
        // Doubleton but no five-card suit: nowhere to run, so stand.
        let (stuck, _) = best_call(&auction, "A52.93.KJ54.AKQ6");
        assert_eq!(stuck, Call::Pass, "a doubleton with no suit stands");
        set_double_style(DoubleStyle::Penalty); // restore the default
    }
}
