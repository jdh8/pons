//! The competitive package over our openings
//!
//! This module builds the [`Competitive`] book that covers contested auctions
//! after our one-level openings: direct-seat responses to their overcall,
//! system-on over their double, support doubles and redoubles for minor
//! openings, and opener's answer to partner's negative double of a two-level
//! minor overcall.

use super::super::constraint::{
    Cons, Constraint, balanced, described, has_stopper, hcp, len, min_level_is, partner_suit_is,
    points, stopper_in, stopper_in_their_suits, suit_hcp, support, they_bid, top_honors,
    vulnerable,
};
use super::super::context::Context;
use super::super::fallback::{
    Fallback, FirstIs, OvercallAtMost, ReplaceNext, SuffixIs, described_guard, described_rewrite,
    guard, rewriter,
};
use super::super::trie::{Classifier, classifier};
use super::super::{Alert, Competitive, Rules};
use super::notrump::{
    PUPPET, complete_transfer, notrump_minors, notrump_responses, smolen_at_three,
    smolen_completion, stayman_answers, transfer_super_accept,
};
use super::weak_twos;
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
/// Multi takeout double — `X` of their `(2♦)` Multi, values/takeout of the unknown
/// major (8+).  Takeout by meaning, so alerted (the reading is a sound 8+ points
/// floor — their suit is unknown, so no single side suit to floor).
const MULTI_TAKEOUT: Alert = Alert("comp:multi-takeout");
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
/// Unusual-vs-unusual over our 1M — the cheaper cue of the two-suiter's suits
/// (`3♣` over their both-minors `(2NT)`, the other-major cue over their
/// Michaels) as a limit-plus raise of our major.
const UVU_MAJOR_RAISE: Alert = Alert("comp:uvu-major-raise");
/// The second cue over their both-minors `(2NT)` — `3♦` as a game force with
/// 5+ cards in the other major.
const UVU_MAJOR_FOURTH: Alert = Alert("comp:uvu-major-fourth");
/// Business redouble of their takeout double of our weak two — 13+ values
/// (redoubles are natural-by-default; the alert buys the points-floor decode).
const WEAK_TWO_XX: Alert = Alert("comp:weak-two-xx");
/// Ogust survives their overcall of our weak two — the contested `2NT` still
/// asks (2+ card support, 14+), alerted so the fit and strength project.
const CONTESTED_OGUST: Alert = Alert("comp:ogust");
/// Cachalot rotated double — 4+ cards in the *adjacent* major (hearts over
/// `(1♦)`, spades over `(1♥)`), not a classic unbid-majors negative double.
const CACHALOT_X: Alert = Alert("comp:cachalot-x");
/// Cachalot transfer — `1♥` over `(1♦)` showing 4+ **spades**.
const CACHALOT_TRANSFER: Alert = Alert("comp:cachalot-transfer");
/// Cachalot residual — `1♠` over `(1♦)`/`(1♥)` as the takeout hand, ≤3 in
/// each major the rotation could have shown.
const CACHALOT_TAKEOUT: Alert = Alert("comp:cachalot-takeout");
/// 2-level free-bid transfer (`FreeBidStyle::Transfer`) — a non-jump 2-level
/// new suit over their overcall showing the *other* unbid suit when exactly
/// two unbid suits sit at the two level; opener completes and declares.
const FREE_TRANSFER: Alert = Alert("comp:free-transfer");
/// Cachalot completion — opener's 1-level completion of the transfer shows
/// **exactly three** trumps (forcing one round; the raise shows four).
const CACHALOT_THREE: Alert = Alert("comp:cachalot-three");
/// Jordan/Truscott `2NT` over their takeout double — a limit-plus raise of
/// the opening (4+ support for a major, 5+ for a minor), not natural.
const JORDAN: Alert = Alert("comp:jordan");
/// Value redouble over their takeout double — 10+ without the Jordan fit
/// (redoubles are natural-by-default; the alert buys the points-floor decode).
const VALUE_REDOUBLE: Alert = Alert("comp:value-redouble");

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

thread_local! {
    /// Whether responder's structure over the opponents' two-suiters over our
    /// 1♥/1♠ opening — their both-minors `(2NT)` and their Michaels cue of our
    /// own major — is authored, and whether the inference walk reads those
    /// calls as two-suiters instead of natural overcalls. **Default on** —
    /// measured vs BBA 2/1 (204.8k boards/arm/vul): plain DD +0.0019/+0.0018
    /// IMPs/board NV/vul (both CIs exclude 0; +1.43/+1.58 IMPs/fired, ~0.12%
    /// fired), perfect-defense the same sign.
    static UVU_OVER_MAJORS: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's structure over their two-suiters over our 1M for books
/// built *after* this call, and read their direct cue / `(2NT)` as a
/// two-suiter (thread-local)
///
/// The book half is read at construction; the inference half at classify time
/// — parallel harnesses must set this inside their worker closures too.
/// **Default on** (`--no-ns-uvu-over-majors` in `bba-gen` for the off arm).
pub fn set_uvu_over_majors(on: bool) {
    UVU_OVER_MAJORS.with(|cell| cell.set(on));
}

/// Whether the two-suiters-over-our-1M package is engaged (book construction
/// *and* the [`inference`][super::super::inference] walk's two-suiter reading)
pub(crate) fn uvu_over_majors() -> bool {
    UVU_OVER_MAJORS.with(Cell::get)
}

thread_local! {
    /// Whether our contested weak twos are authored: responder over their
    /// takeout double (business `XX`, systems-on Ogust) and over their
    /// overcall (Ogust-when-legal, values `X`, preemptive raises). Default
    /// off while the A/B runs.
    static WEAK_TWO_COMPETITION: Cell<bool> = const { Cell::new(false) };

    /// Whether our contested strong 2♣ is authored: systems-on over their
    /// double, and over their overcall a natural-GF / values-`X` / waiting-
    /// pass structure backed by opener's forced reopening. Without it
    /// responder's `X` falls to the floor's *takeout* reading — with a 22+
    /// opener behind it. **Default on** — measured vs BBA 2/1 (204.8k
    /// boards/arm/vul): plain DD +1.86/+2.79 IMPs/fired NV/vul,
    /// perfect-defense +2.00/+2.93; all four cells' CIs exclude 0 (~0.05%
    /// fired).
    static STRONG_TWO_COMPETITION: Cell<bool> = const { Cell::new(true) };
}

/// Author our contested weak twos for books built *after* this call
/// (thread-local)
///
/// Default off (`--ns-weak-two-comp` in `bba-gen` for the on arm).
pub fn set_weak_two_competition(on: bool) {
    WEAK_TWO_COMPETITION.with(|cell| cell.set(on));
}

/// Whether the contested weak-two package is engaged
fn weak_two_competition() -> bool {
    WEAK_TWO_COMPETITION.with(Cell::get)
}

/// Author our contested strong 2♣ for books built *after* this call
/// (thread-local)
///
/// **Default on** (`--no-ns-strong-two-comp` in `bba-gen` for the off arm).
pub fn set_strong_two_competition(on: bool) {
    STRONG_TWO_COMPETITION.with(|cell| cell.set(on));
}

/// Whether the contested strong-2♣ package is engaged
fn strong_two_competition() -> bool {
    STRONG_TWO_COMPETITION.with(Cell::get)
}

thread_local! {
    /// Whether opener's support double/redouble extends to the major-major
    /// auction `1♥ – (P) – 1♠ – (X / overcall below 2♠)`. The minor-opening
    /// pairs are always on (shipped). **Default on** — measured vs BBA 2/1
    /// (204.8k boards/arm/vul): plain DD wash (−0.0004/+0.0004, CIs straddle
    /// 0), perfect-defense +0.97/+1.69 IMPs/fired NV/vul (vul CI excludes 0)
    /// — the plain-wash + PD-gain ship row (~0.10% fired).
    static MAJOR_SUPPORT_DOUBLE: Cell<bool> = const { Cell::new(true) };
}

/// Extend support doubles to `1♥ – (P) – 1♠` for books built *after* this
/// call (thread-local)
///
/// **Default on** (`--no-ns-major-support-double` in `bba-gen` for the off
/// arm).
pub fn set_major_support_double(on: bool) {
    MAJOR_SUPPORT_DOUBLE.with(|cell| cell.set(on));
}

/// Whether the major-major support double is engaged
fn major_support_double() -> bool {
    MAJOR_SUPPORT_DOUBLE.with(Cell::get)
}

/// The negative-double school over our **minor** openings
/// ([`set_negative_double_shape`]; the major-opening double — 4+ in the other
/// major, 8+ — is common to all three)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NegativeDoubleShape {
    /// Both majors 4-4+ at 8+ regardless of the overcall — the shipped rule
    BothMajors,
    /// Modern standard (BWS/Cohen): over `(1♦)` both majors 4-4+ at 6+; over
    /// `(1♥)` **exactly** four spades at 6+ (with 5+ bid the free `1♠`); over
    /// `(1♠)` 4+ hearts at 8+; over a 2-level minor both majors at 8+.
    /// Implies the free bids (the exactly-4 double is unsound without the
    /// 5-card outlet).
    Modern,
    /// Cachalot — transfer Walsh in competition (Lebel–Soulet lineage): over
    /// `(1♦)`/`(1♥)` the 1-level calls rotate — `X` = 4+ in the adjacent
    /// major, `1♥` = 4+ spades, `1♠` = the residual takeout hand (≤3 in each
    /// shown-able major). Opener's 1-level completion shows **exactly three**
    /// trumps, forcing; the raise shows four. Natural from `(1♠)` up (the
    /// Modern rules apply there). Implies the free bids.
    Cachalot,
    /// Sputnik (Roth–Stone original): the double is the **residual** — it
    /// *denies* a 4-card major biddable at the 1-level, showing 7+ with the
    /// biddable majors held to ≤3; the free 1-level major shows a natural 4+
    /// (not Modern's 5+, since the double no longer carries the exactly-four
    /// hand). Over `(1♦)`: `X` = ≤3 in both majors, `1♥`/`1♠` = 4+ natural;
    /// over `(1♥)`: `X` = ≤3 spades, `1♠` = 4+. From `(1♠)` up and over a
    /// 2-level minor the Modern rules apply (no 1-level major to deny). Implies
    /// the free bids.
    Sputnik,
}

/// The meaning of responder's non-jump 2-level new suit over their overcall
/// (`set_free_bid_style`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeBidStyle {
    /// Forcing one round — the shipped default (the Fix 1 ruling: 1-level
    /// frees unconditionally forcing, 2-level forcing one round), answered by
    /// the Section-4d `answer_free_bid`.
    Forcing,
    /// Classic negative free bids: 2-level new suits are **non-forcing**,
    /// 5–11 points with a six-card suit or a strong five-carder (two of the
    /// top three honors); every stronger long-suit hand starts with the
    /// widened negative double, and double-then-new-suit is forcing to game.
    Negative,
    /// Cachalot-style 2-level transfers: when exactly two unbid suits sit at
    /// the two level the slots swap — each shows the other suit — and opener
    /// completes (declaring the concealed hand); the wrap slot completes a
    /// level higher. A lone (or three-way, over 1NT) 2-level slot stays
    /// natural-forcing.
    Transfer,
}

thread_local! {
    /// Which negative-double school the minor openings play. Default
    /// `Modern` — **shipped default-on 2026-07-10** with the forcing free-bid
    /// answers: plain +0.0213 NV / +0.0074 vul (CI>0), sd arbiter +0.42/+0.29
    /// per divergent board (CI>0, sd>plain, disclosure-corrected); the vul-PD
    /// −0.026 is the perfect-defense doubling artifact on thin vul games.
    static NEGATIVE_DOUBLE_SHAPE: Cell<NegativeDoubleShape> =
        const { Cell::new(NegativeDoubleShape::Modern) };

    /// Whether responder's natural free bids over an overcall are authored
    /// (1-level new suit 5+ & 6+, 2-level non-jump 5+ & 10+, 1NT 6–10 / 2NT
    /// 11–12 with a stopper). Default off as a *direct* toggle, but the
    /// shipped `Modern` shape implies them (with opener's forcing answers) —
    /// the default system plays free bids.
    static FREE_BIDS: Cell<bool> = const { Cell::new(false) };

    /// Minimum points/HCP for the 1-level free *suit* bids (new-suit 5+, plus
    /// the Sputnik natural 4+ majors). Default 6 — the shipped floor. The vul-PD
    /// leak of the whole free-bid family lives here; sweep to 8+ and re-measure.
    /// The free 1NT has its own floor (`FREE_1NT_FLOOR`): a forcing suit bid
    /// finds a fit cheaply and is safe light, a limited non-forcing 1NT is not.
    static FREE_BID_FLOOR: Cell<u8> = const { Cell::new(6) };

    /// Minimum HCP for the free 1NT (`1X (1Y) 1NT`), decoupled from the suit
    /// floor above. Default 6 — byte-identical to the historical shared value.
    static FREE_1NT_FLOOR: Cell<u8> = const { Cell::new(6) };

    /// Whether the vulnerable free bids demand quality: a vulnerable 1-level
    /// new suit needs two of the top three honors, and the free 1NT is not
    /// authored vulnerable. The P3b′ floor sweep named the family's vulnerable
    /// leak as plain-DD-visible and strength-independent — a suit-quality
    /// gate, not a floor. Default off while the A/B runs.
    static FREE_BID_QUALITY: Cell<bool> = const { Cell::new(false) };

    /// The 2-level free-bid style — forcing (shipped default), classic
    /// negative free bids, or Cachalot-style transfers. The 1-level free
    /// bids stay forcing in every style.
    static FREE_BID_STYLE: Cell<FreeBidStyle> = const { Cell::new(FreeBidStyle::Forcing) };
}

/// Choose the 2-level free-bid style for books built *after* this call
/// (thread-local)
///
/// Default [`FreeBidStyle::Forcing`] (`--ns-free-bid-style` in `bba-gen` for
/// the other arms).
pub fn set_free_bid_style(style: FreeBidStyle) {
    FREE_BID_STYLE.with(|cell| cell.set(style));
}

/// The 2-level free-bid style in effect
fn free_bid_style() -> FreeBidStyle {
    FREE_BID_STYLE.with(Cell::get)
}

/// Choose the negative-double school for books built *after* this call
/// (thread-local)
///
/// Default [`NegativeDoubleShape::Modern`] — shipped default-on; pass
/// `--ns-negative-double-shape both-majors` in `bba-gen` for the old rule.
pub fn set_negative_double_shape(shape: NegativeDoubleShape) {
    NEGATIVE_DOUBLE_SHAPE.with(|cell| cell.set(shape));
}

/// The negative-double school in effect
fn negative_double_shape() -> NegativeDoubleShape {
    NEGATIVE_DOUBLE_SHAPE.with(Cell::get)
}

thread_local! {
    /// Whether opener's contested-X answer is authored (Cachalot only). Default
    /// on; the off state restores the floored continuation for the A/B.
    static CACHALOT_CONTESTED_X: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's raise of a Cachalot `X` transfer when LHO competes over it
/// (thread-local, Cachalot only)
///
/// Default on — `--no-ns-cachalot-contested-x` in `bba-gen` restores the old
/// floored continuation.
pub fn set_cachalot_contested_x(on: bool) {
    CACHALOT_CONTESTED_X.with(|cell| cell.set(on));
}

/// Whether opener's contested-X answer is engaged
fn cachalot_contested_x() -> bool {
    CACHALOT_CONTESTED_X.with(Cell::get)
}

/// Author responder's natural free bids over an overcall for books built
/// *after* this call (thread-local)
///
/// Default off (`--ns-free-bids` in `bba-gen` for the on arm).
pub fn set_free_bids(on: bool) {
    FREE_BIDS.with(|cell| cell.set(on));
}

/// Whether the free bids are authored — directly, or implied by a
/// negative-double shape whose tighter double needs the natural outlet
fn free_bids_engaged() -> bool {
    FREE_BIDS.with(Cell::get) || negative_double_shape() != NegativeDoubleShape::BothMajors
}

/// Set the minimum points/HCP for the 1-level free bids (thread-local)
///
/// Default 6 (`--ns-free-bid-floor` in `bba-gen`). Raising it trims the
/// vulnerable-PD leak the free-bid family inherits.
pub fn set_free_bid_floor(min: u8) {
    FREE_BID_FLOOR.with(|cell| cell.set(min));
}

/// The minimum points/HCP for the 1-level free bids
fn free_bid_floor() -> u8 {
    FREE_BID_FLOOR.with(Cell::get)
}

/// Set the minimum HCP for the free 1NT (`1X (1Y) 1NT`), decoupled from the
/// suit floor (thread-local)
///
/// Default 6 (`--ns-free-1nt-floor` in `bba-gen`). The free 1NT is a limited,
/// non-forcing commitment to notrump values; raising this trims light 1NTs
/// without touching the forcing 1-level suit bids.
pub fn set_free_1nt_floor(min: u8) {
    FREE_1NT_FLOOR.with(|cell| cell.set(min));
}

/// The minimum HCP for the free 1NT
fn free_1nt_floor() -> u8 {
    FREE_1NT_FLOOR.with(Cell::get)
}

/// Gate the vulnerable free bids on suit quality for books built *after* this
/// call (thread-local)
///
/// Default off (`--ns-free-bid-quality` in `bba-gen` for the on arm). When
/// on, a vulnerable 1-level free bid demands two of the top three honors in
/// the bid suit and the free 1NT is not authored vulnerable; non-vulnerable
/// rules and the 2-level/2NT free bids are unchanged.
pub fn set_free_bid_quality(on: bool) {
    FREE_BID_QUALITY.with(|cell| cell.set(on));
}

/// Whether the vulnerable free-bid quality gate is on
fn free_bid_quality() -> bool {
    FREE_BID_QUALITY.with(Cell::get)
}

thread_local! {
    /// Whether responder's structure over their jump / 3-level overcalls
    /// (`2NT < bid ≤ 3♠`) is authored — the shipped direct-seat package stops
    /// at `OvercallAtMost(2♠)` and everything higher falls to the floor.
    /// Default off while the A/B runs.
    static HIGH_OVERCALL_RESPONSES: Cell<bool> = const { Cell::new(false) };
}

/// Author responder's structure over their 3-level overcalls for books built
/// *after* this call (thread-local)
///
/// Default off (`--ns-high-overcall` in `bba-gen` for the on arm).
pub fn set_high_overcall_responses(on: bool) {
    HIGH_OVERCALL_RESPONSES.with(|cell| cell.set(on));
}

/// Whether the 3-level-overcall package is engaged
fn high_overcall_responses() -> bool {
    HIGH_OVERCALL_RESPONSES.with(Cell::get)
}

thread_local! {
    /// Whether responder's structure over their takeout double of our 1-suit
    /// opening is authored: Jordan/Truscott `2NT`, the value redouble, the
    /// preemptive jump-raise flip, and weak non-forcing 2-level suits — with
    /// the shipped systems-on rebase surviving below it as the catch-all for
    /// every deeper continuation. **Default on** — the campaign's largest
    /// per-board win vs BBA 2/1 (204.8k boards/arm/vul): plain DD
    /// +0.0041/+0.0067 IMPs/board NV/vul, perfect-defense +0.0049/+0.0065;
    /// all four cells' CIs exclude 0 (+0.5…+0.8 IMPs/fired, ~0.8% fired).
    static JORDAN_TRUSCOTT: Cell<bool> = const { Cell::new(true) };
}

/// Author responder's structure over their takeout double for books built
/// *after* this call (thread-local)
///
/// **Default on** (`--no-ns-jordan-truscott` in `bba-gen` for the off arm).
pub fn set_jordan_truscott(on: bool) {
    JORDAN_TRUSCOTT.with(|cell| cell.set(on));
}

/// Whether the over-their-double package is engaged
fn jordan_truscott() -> bool {
    JORDAN_TRUSCOTT.with(Cell::get)
}

thread_local! {
    /// Whether opener's rebid over the value redouble (`1x – (X) – XX – (P)`)
    /// is authored.  **Default on** (fix-vs-shipped, 1M boards/vul, 24.pdd
    /// 16.3M–18.3M: plain DD +0.0056 ± 0.0005 NV / +0.0078 ± 0.0007 vul, PD
    /// +0.0058/+0.0080, ≈ +11..+14 IMPs per divergent board).  Off, the
    /// systems-on rebase strips both the double and the redouble, so opener
    /// replays onto the uncontested tree with responder's shown 10+ unseen,
    /// and the floor blasts stopperless 3NTs / thin games off shaped minimums
    /// — the point-count remnant's single worst per-board family
    /// (−16..−17 IMPs/board vulnerable).  See [`set_redouble_answer`].
    static REDOUBLE_ANSWER: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's rebid over the value redouble (`1x – (X) – XX – (P)`) for
/// books built *after* this call (thread-local); requires
/// [`set_jordan_truscott`] on (the redouble itself)
///
/// **Default on** (measured; see the thread-local above).  The authored node
/// is pass-only — a long-suit minimum sits for the redoubled make, and a 2M
/// escape rung measured −11 IMPs/fired before deletion.  `false` restores the
/// shipped floor for the off arm.
pub fn set_redouble_answer(on: bool) {
    REDOUBLE_ANSWER.with(|cell| cell.set(on));
}

/// Whether opener's answer over the value redouble is authored
fn redouble_answer() -> bool {
    REDOUBLE_ANSWER.with(Cell::get)
}

thread_local! {
    /// Whether a double of our splinter runs systems-on (see
    /// [`set_splinter_doubled`]).
    static SPLINTER_DOUBLED: Cell<bool> = const { Cell::new(true) };
}

/// Play systems-on over their double of our splinter for books built *after*
/// this call (thread-local)
///
/// A splinter (`1M – (P) – double-jump`) is game-forcing, but the double
/// reroutes opener's rebid to the competitive book, where — unauthored — it
/// fell to the floor and *passed*, leaving the game force doubled at the four
/// level (the anchor's Constructive/book/round-1 bucket #4 tail: our monster
/// opener passing a doubled `4♣` splinter while the field bids `7♠`). This
/// rebases the double back onto the undisturbed splinter continuation (4M
/// sign-off floor, RKCB with slam values). **Default on** — measured vs BBA
/// 2/1 (204.8k bd/arm/vul, SEED_BASE 1783439089): plain DD +0.0059/+0.0079
/// IMPs/board NV/vul, perfect-defense +0.0059/+0.0079, all four CIs exclude 0,
/// +15.4/+17.6 IMPs/fired (0.04% fired). Off-switch `--no-ns-splinter-doubled`.
pub fn set_splinter_doubled(on: bool) {
    SPLINTER_DOUBLED.with(|cell| cell.set(on));
}

/// Whether the doubled-splinter systems-on rebase is engaged
fn splinter_doubled() -> bool {
    SPLINTER_DOUBLED.with(Cell::get)
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

    // Negative double. The major-opening double is common to every school;
    // the minor-opening shape follows [`NegativeDoubleShape`]. The dynamic
    // "which overcall" conditions are legality-anchored: `min_level_is(1, ♥)`
    // holds exactly over a (1♦) overcall, `they_bid(♥) & min_level_is(1, ♠)`
    // exactly over (1♥).
    let shape = negative_double_shape();
    rules = if is_major {
        // Other major, 4+ cards, 8+ HCP
        rules
            .rule(Call::Double, 1.0, len(other_major, 4..) & hcp(8..))
            .alert(NEGATIVE_DOUBLE)
    } else {
        match shape {
            // Both majors 4+, 8+ HCP — the shipped rule.
            NegativeDoubleShape::BothMajors => rules
                .rule(
                    Call::Double,
                    1.0,
                    len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
            NegativeDoubleShape::Modern => rules
                // Over (1♦): both majors, floor 6.
                .rule(
                    Call::Double,
                    1.0,
                    min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(6..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over (1♥): exactly four spades (five-plus bids the free 1♠).
                .rule(
                    Call::Double,
                    1.0,
                    they_bid(Strain::Hearts)
                        & min_level_is(1, Strain::Spades)
                        & len(Suit::Spades, 4..=4)
                        & hcp(6..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over (1♠)/(2♠): 4+ hearts, floor 8 (the reply starts at the
                // 2 level).
                .rule(
                    Call::Double,
                    1.0,
                    they_bid(Strain::Spades) & len(Suit::Hearts, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over a 2-level minor: both majors, floor 8.
                .rule(
                    Call::Double,
                    1.0,
                    (they_bid(Strain::Clubs) | they_bid(Strain::Diamonds))
                        & !min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
            NegativeDoubleShape::Cachalot => rules
                // Over (1♦): X transfers — 4+ hearts (may hold spades too).
                .rule(
                    Call::Double,
                    1.0,
                    min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & points(free_bid_floor()..),
                )
                .alert(CACHALOT_X)
                // Over (1♥): X transfers — 4+ spades.
                .rule(
                    Call::Double,
                    1.0,
                    they_bid(Strain::Hearts)
                        & min_level_is(1, Strain::Spades)
                        & len(Suit::Spades, 4..)
                        & points(free_bid_floor()..),
                )
                .alert(CACHALOT_X)
                // Natural from (1♠) up: the Modern rules apply.
                .rule(
                    Call::Double,
                    1.0,
                    they_bid(Strain::Spades) & len(Suit::Hearts, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE)
                .rule(
                    Call::Double,
                    1.0,
                    (they_bid(Strain::Clubs) | they_bid(Strain::Diamonds))
                        & !min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
            NegativeDoubleShape::Sputnik => rules
                // Over (1♦): the residual — ≤3 in both majors, 7+ (4+ in
                // either bids the natural free 1-level suit below).
                .rule(
                    Call::Double,
                    1.0,
                    min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, ..=3)
                        & len(Suit::Spades, ..=3)
                        & hcp(7..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over (1♥): the residual — ≤3 spades, 7+ (4+ bids the free 1♠).
                .rule(
                    Call::Double,
                    1.0,
                    they_bid(Strain::Hearts)
                        & min_level_is(1, Strain::Spades)
                        & len(Suit::Spades, ..=3)
                        & hcp(7..),
                )
                .alert(NEGATIVE_DOUBLE)
                // From (1♠) up: 4+ hearts, floor 8 — no 1-level major to deny
                // (the Modern rule).
                .rule(
                    Call::Double,
                    1.0,
                    they_bid(Strain::Spades) & len(Suit::Hearts, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over a 2-level minor: both majors, floor 8 (the Modern rule).
                .rule(
                    Call::Double,
                    1.0,
                    (they_bid(Strain::Clubs) | they_bid(Strain::Diamonds))
                        & !min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
        }
    };

    // Classic NFB widens the double: the 2-level new suits are capped at 11,
    // so every stronger long-suit hand starts here and clarifies with the
    // forcing-to-game new suit next round (Section 4d″). The second `X` rule
    // ORs into the projection — the points floor survives (every school's
    // double floors at or below 12) but the suit floors collapse to zero:
    // the named OR-projection wall, priced by the Stage-B A/B. Weight below
    // the cue (2.0) and the free bids (1.45) so a biddable hand still bids.
    if free_bid_style() == FreeBidStyle::Negative {
        rules = rules
            .rule(Call::Double, 0.9, points(12..))
            .alert(NEGATIVE_DOUBLE);
    }

    // Cachalot's rotated 1-level calls over (1♦)/(1♥): 1♥ shows spades, 1♠
    // is the residual takeout hand. Only minor openings rotate. Cachalot is
    // rotated Sputnik, so the floors match Sputnik's — the major-showing
    // calls take the free-bid `points` floor (hcp(6..) orphaned the light
    // shapely hands Modern frees, the Stage-A named leak) and the residual
    // takeout matches the residual double's hcp(7..).
    if !is_major && shape == NegativeDoubleShape::Cachalot {
        rules = rules
            // Over (1♦): 1♥ = 4+ spades without 4 hearts (4+ hearts doubles).
            .rule(
                Bid::new(1, Strain::Hearts),
                1.45,
                min_level_is(1, Strain::Hearts)
                    & len(Suit::Spades, 4..)
                    & len(Suit::Hearts, ..=3)
                    & points(free_bid_floor()..),
            )
            .alert(CACHALOT_TRANSFER)
            // Over (1♦): 1♠ = the takeout hand, ≤3 in both majors. Sits below
            // the notrump rules so a stopper hand prefers 1NT/2NT.
            .rule(
                Bid::new(1, Strain::Spades),
                0.85,
                min_level_is(1, Strain::Hearts)
                    & len(Suit::Hearts, ..=3)
                    & len(Suit::Spades, ..=3)
                    & hcp(7..),
            )
            .alert(CACHALOT_TAKEOUT)
            // Over (1♥): 1♠ = the takeout hand, ≤3 spades (4+ doubles).
            .rule(
                Bid::new(1, Strain::Spades),
                0.85,
                they_bid(Strain::Hearts)
                    & min_level_is(1, Strain::Spades)
                    & len(Suit::Spades, ..=3)
                    & hcp(7..),
            )
            .alert(CACHALOT_TAKEOUT);
    }

    // Sputnik's natural 1-level majors show 4+ (not the shared block's 5+) —
    // the free bid its residual double leans on. Only minor openings; the
    // `min_level_is` guards keep them to (1♦) [1♥/1♠] and (1♥) [1♠].
    if !is_major && shape == NegativeDoubleShape::Sputnik {
        rules = rules
            .rule(
                Bid::new(1, Strain::Hearts),
                1.45,
                min_level_is(1, Strain::Hearts)
                    & len(Suit::Hearts, 4..)
                    & points(free_bid_floor()..),
            )
            .rule(
                Bid::new(1, Strain::Spades),
                1.45,
                min_level_is(1, Strain::Spades)
                    & len(Suit::Spades, 4..)
                    & points(free_bid_floor()..),
            );
    }

    // Natural free bids (`set_free_bids`; implied by the Modern/Cachalot
    // shapes, whose tighter doubles need the natural outlet). A free bid of
    // their suit is the cue above; the 1-level majors stay out of the
    // Cachalot rotation's way (a 5-card major routes through its transfer).
    if free_bids_engaged() {
        // Cachalot and Sputnik both author their own 1-level majors above, so
        // skip the shared 5+ rule for them (Cachalot rotates, Sputnik lowers to
        // 4+).
        let rotate = !is_major
            && matches!(
                shape,
                NegativeDoubleShape::Cachalot | NegativeDoubleShape::Sputnik
            );
        for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            if x == o {
                continue;
            }
            let xs = Strain::from(x);
            if !(rotate && matches!(x, Suit::Hearts | Suit::Spades)) {
                let one_level =
                    min_level_is(1, xs) & len(x, 5..) & points(free_bid_floor()..) & !they_bid(xs);
                rules = if free_bid_quality() {
                    rules.rule(
                        Bid::new(1, xs),
                        1.45,
                        one_level & (top_honors(x, 2..) | !vulnerable()),
                    )
                } else {
                    rules.rule(Bid::new(1, xs), 1.45, one_level)
                };
            }
            match free_bid_style() {
                // Forcing one round (the shipped default), answered by 4d.
                FreeBidStyle::Forcing => {
                    rules = rules.rule(
                        Bid::new(2, xs),
                        1.45,
                        min_level_is(2, xs) & len(x, 5..) & points(10..) & !they_bid(xs),
                    );
                }
                // Classic negative free bid: non-forcing 5–11 with a
                // six-carder or a strong five-carder — stronger long-suit
                // hands start with the widened double below.
                FreeBidStyle::Negative => {
                    rules = rules.rule(
                        Bid::new(2, xs),
                        1.45,
                        min_level_is(2, xs)
                            & (len(x, 6..) | (len(x, 5..) & top_honors(x, 2..)))
                            & points(5..=11)
                            & !they_bid(xs),
                    );
                }
                // The transfer rotation is authored per suit *pair* after
                // this loop — it needs both slots in one constraint.
                FreeBidStyle::Transfer => {}
            }
        }
        // Cachalot-style 2-level transfers: when exactly two unbid suits sit
        // at the two level the slots swap, so opener completes and declares
        // the concealed hand; the wrap (higher) slot completes a level
        // higher. A lone slot — or all three over a (1NT) overcall — stays
        // natural-forcing. Unlimited at 6+: the weak hand passes the
        // completion, strength clarifies a round later.
        if free_bid_style() == FreeBidStyle::Transfer {
            let others: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .filter(|&x| x != o)
                .collect();
            let slot = |s: Suit| {
                let ss = Strain::from(s);
                min_level_is(2, ss) & !they_bid(ss)
            };
            for i in 0..3 {
                for j in (i + 1)..3 {
                    let (x, y) = (others[i], others[j]);
                    let w = others[3 - i - j];
                    // The lower slot shows the higher suit (true transfer)…
                    rules = rules
                        .rule(
                            Bid::new(2, Strain::from(x)),
                            1.45,
                            slot(x) & slot(y) & !slot(w) & len(y, 5..) & points(6..),
                        )
                        .alert(FREE_TRANSFER)
                        // …and the higher slot wraps around to show the lower.
                        .rule(
                            Bid::new(2, Strain::from(y)),
                            1.45,
                            slot(x) & slot(y) & !slot(w) & len(x, 5..) & points(6..),
                        )
                        .alert(FREE_TRANSFER);
                }
            }
            for i in 0..3 {
                let x = others[i];
                let (y, z) = (others[(i + 1) % 3], others[(i + 2) % 3]);
                // No swap partner (or two of them): natural and forcing, as
                // in the default style.
                rules = rules.rule(
                    Bid::new(2, Strain::from(x)),
                    1.45,
                    slot(x)
                        & ((slot(y) & slot(z)) | (!slot(y) & !slot(z)))
                        & len(x, 5..)
                        & points(10..),
                );
            }
        }
        let one_notrump = min_level_is(1, Strain::Notrump)
            & hcp(free_1nt_floor()..=10)
            & stopper_in_their_suits();
        rules = if free_bid_quality() {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                0.9,
                one_notrump & !vulnerable(),
            )
        } else {
            rules.rule(Bid::new(1, Strain::Notrump), 0.9, one_notrump)
        };
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            0.95,
            min_level_is(2, Strain::Notrump) & hcp(11..=12) & stopper_in_their_suits(),
        );
        // The natural invitational 2NT *jump* over a 1-level overcall: 11–12
        // with a stopper, the invite the ordinary 2NT rule (min-level, i.e. a
        // 2-level overcall) leaves stranded. `min_level_is(1, Notrump)` means
        // 1NT is still the cheapest notrump, so this 2NT is a jump.
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            0.95,
            min_level_is(1, Strain::Notrump) & hcp(11..=12) & stopper_in_their_suits(),
        );
    }

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

/// Opener's answer to responder's natural free bid — a new suit over their
/// overcall, **forcing one round** at both levels (the free-bid-quality A/B's
/// worst vulnerable boards were opener passing a game-going `2♦` out)
///
/// Raise partner's suit with 3-card support, bid notrump with a stopper in
/// their suit, show a natural second suit (reverses and 3-level suits need
/// 16+), else rebid the opening suit as the catch-all. No `Pass` rule — the
/// free bid forces by omission; the table is total via the rebid.
fn answer_free_bid(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);
    let mut rules = Rules::new();

    // Raise partner's freely bid suit with 3-card support (the free bid
    // promises five). `min_level_is` picks the cheapest legal raise. A raise
    // to *two* answers a 1-level free bid (the only auction whose cheapest
    // raise sits there), and Sputnik's natural 1-level majors promise only
    // four — raising on three would be a Moysian at the two level, so that
    // rung demands four; 2-level frees promise five in every school.
    let two_level_support: usize = if negative_double_shape() == NegativeDoubleShape::Sputnik {
        4
    } else {
        3
    };
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if y == o {
            continue;
        }
        let y_strain = Strain::from(y);
        for lvl in 2u8..=3 {
            let min_support = if lvl == 2 { two_level_support } else { 3 };
            rules = rules.rule(
                Bid::new(lvl, y_strain),
                1.5,
                partner_suit_is(y) & min_level_is(lvl, y_strain) & support(min_support..),
            );
        }
    }

    // Cheapest notrump with a stopper in their suit, minimum balanced range.
    for lvl in 1u8..=2 {
        rules = rules.rule(
            Bid::new(lvl, Strain::Notrump),
            1.2,
            min_level_is(lvl, Strain::Notrump) & stopper_in_their_suits() & hcp(12..=14),
        );
    }

    // A natural second suit: cheap non-reverse freely, a reverse or 3-level
    // suit shows 16+.
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let x_strain = Strain::from(x);
        for lvl in 1u8..=3 {
            let strong = lvl >= 3 || (lvl == 2 && x > o);
            let shape = min_level_is(lvl, x_strain)
                & !partner_suit_is(x)
                & !they_bid(x_strain)
                & len(x, 4..);
            rules = if strong {
                rules.rule(Bid::new(lvl, x_strain), 1.1, shape & hcp(16..))
            } else {
                rules.rule(Bid::new(lvl, x_strain), 1.1, shape)
            };
        }
    }

    // Catch-all: rebid the opening suit at the cheapest level (weakest action).
    for lvl in 2u8..=3 {
        rules = rules.rule(
            Bid::new(lvl, o_strain),
            0.0,
            min_level_is(lvl, o_strain) & hcp(0..),
        );
    }
    rules
}

/// Opener's answer to a *negative* (non-forcing) free bid — 5–11 with a
/// six-carder or a strong five-carder (`FreeBidStyle::Negative`)
///
/// `Pass` is the treatment's whole point: the catch-all drops the capped
/// hand at the two level (mirroring `answer_weak_new_suit`). Raising to
/// three needs a fit and real extras; `2NT` shows a stopper-backed maximum.
fn answer_negative_free_bid(opening: Suit) -> Rules {
    let mut rules = Rules::new();
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if y == opening {
            continue;
        }
        let ys = Strain::from(y);
        rules = rules.rule(
            Bid::new(3, ys),
            0.9,
            partner_suit_is(y) & min_level_is(3, ys) & len(y, 3..) & points(15..),
        );
    }
    rules
        .rule(
            Bid::new(2, Strain::Notrump),
            0.8,
            min_level_is(2, Strain::Notrump) & stopper_in_their_suits() & hcp(13..=14),
        )
        .rule(Call::Pass, 0.3, hcp(0..))
}

/// The negative doubler's rebid after opener answers (`FreeBidStyle::
/// Negative`): a new suit is the strong hand the capped free bid could not
/// carry — **forcing to game**
///
/// Also claims the *ordinary* doubler's second turn (this node cannot tell
/// the two apart): raise opener's answer with a real fit, `2NT` with a
/// stopper and 10–12, else the `Pass` catch-all drops the minimum answers.
fn negative_doubler_rebid(opening: Suit) -> Rules {
    let o = opening;
    let mut rules = Rules::new();
    // The FG clarification: the long suit the double concealed.
    for z in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if z == o {
            continue;
        }
        let zs = Strain::from(z);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, zs),
                1.3,
                min_level_is(lvl, zs)
                    & !partner_suit_is(z)
                    & !they_bid(zs)
                    & len(z, 5..)
                    & points(12..),
            );
        }
    }
    // Raise opener's answer with four trumps and invitational values.
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let ys = Strain::from(y);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, ys),
                1.0,
                partner_suit_is(y) & min_level_is(lvl, ys) & support(4..) & points(8..),
            );
        }
    }
    rules
        .rule(
            Bid::new(2, Strain::Notrump),
            0.9,
            min_level_is(2, Strain::Notrump) & stopper_in_their_suits() & hcp(10..=12),
        )
        .rule(Call::Pass, 0.2, hcp(0..))
}

/// Opener's completion of a 2-level free-bid transfer (`FreeBidStyle::
/// Transfer`) — `shown` is responder's real suit, `comp_lvl` where the
/// completion sits (3 on the wrap slot)
///
/// The duty completion is non-forcing and puts opener on play (the
/// right-siding payoff); four trumps with extras super-accept. No notrump
/// option — declining the transfer into notrump re-sides the hand the
/// treatment exists to conceal.
fn free_transfer_completion(shown: Suit, comp_lvl: u8) -> Rules {
    let m = Strain::from(shown);
    Rules::new()
        .rule(
            Bid::new(comp_lvl + 1, m),
            1.3,
            len(shown, 4..) & points(15..),
        )
        .rule(Bid::new(comp_lvl, m), 1.2, hcp(0..))
}

/// Responder's clarification after opener completes the 2-level transfer:
/// `Pass` = the weak hand (the NFB equivalent), raise = invitational, the
/// cue of their suit = game force
fn free_transfer_clarify(shown: Suit, comp_lvl: u8, cue: Bid) -> Rules {
    let m = Strain::from(shown);
    Rules::new()
        .rule(cue, 1.1, points(13..))
        .rule(Bid::new(comp_lvl + 1, m), 1.0, points(10..=12))
        .rule(Call::Pass, 0.3, hcp(0..))
}

/// How many unbid suits sit at the two level over their `ovc` after our
/// `o_strain` opening — the `FreeBidStyle::Transfer` swap fires on exactly
/// two (the same cheapest-level arithmetic as the Section-4d guard)
fn two_level_slots(o_strain: Strain, ovc: Bid) -> usize {
    [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ]
    .into_iter()
    .filter(|&s| s != o_strain && s != ovc.strain)
    .filter(|&s| ovc.level.get() + u8::from(s < ovc.strain) == 2)
    .count()
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
// Section 6: their two-suiters over our 1M (`set_uvu_over_majors`)
// ---------------------------------------------------------------------------

/// Responder after our 1M and their both-minors `(2NT)` — unusual vs unusual
///
/// The two cues split by strength and direction: `3♣` (their lower suit) is
/// the limit-plus raise of our major, `3♦` a game force with 5+ in the other
/// major. `3NT` is to play with both minors stopped; `X` shows values and a
/// minor we can punish (the shape [`uvu_responder`] measured over our
/// overcalled 1NT); the direct raises stay natural — `3M` competitive, `4M`
/// preemptive. Written with `len` rather than `support` so the alerted cues
/// project (the opening major is known here).
fn uvu_major_responder(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om = unbid_major(major).expect("a major opening has an unbid major");

    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            2.0,
            len(major, 3..) & points(10..),
        )
        .alert(UVU_MAJOR_RAISE)
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.9,
            len(om, 5..) & points(13..),
        )
        .alert(UVU_MAJOR_FOURTH)
        .rule(
            Bid::new(3, Strain::Notrump),
            1.5,
            points(13..) & stopper_in(Suit::Clubs) & stopper_in(Suit::Diamonds),
        )
        .rule(
            Call::Double,
            1.4,
            hcp(10..)
                & (len(Suit::Clubs, 4..)
                    | suit_hcp(Suit::Clubs, 4..)
                    | len(Suit::Diamonds, 4..)
                    | suit_hcp(Suit::Diamonds, 4..)),
        )
        .rule(Bid::new(3, m), 1.3, len(major, 3..) & points(6..=9))
        .rule(Bid::new(4, m), 1.25, len(major, 4..) & points(..=9))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Responder after our 1M and their Michaels cue of our own major (`1♥-(2♥)`
/// / `1♠-(2♠)` — 5+ in the other major and 5+ in an unknown minor)
///
/// The cue of their *known* suit (`2♠` over `1♥-(2♥)`, `3♥` over `1♠-(2♠)`)
/// is the limit-plus raise; `X` shows values (their runout has nowhere quiet
/// to land); the direct raises keep their natural meaning — the guard in
/// Section 4b always excluded their cue of our own major precisely because
/// `3M` here is a raise, not a cue-raise. `3♣`/`3♦` are natural weak escapes
/// (their minor is unknown, so both are biddable).
fn michaels_cue_responder(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om_cue = if major == Suit::Hearts {
        Bid::new(2, Strain::Spades)
    } else {
        Bid::new(3, Strain::Hearts)
    };

    Rules::new()
        .rule(om_cue, 2.0, len(major, 3..) & points(10..))
        .alert(UVU_MAJOR_RAISE)
        .rule(Call::Double, 1.6, hcp(10..))
        .rule(Bid::new(3, m), 1.3, len(major, 3..) & points(6..=9))
        .rule(Bid::new(4, m), 1.25, len(major, 4..) & points(..=9))
        .rule(
            Bid::new(3, Strain::Clubs),
            1.1,
            len(Suit::Clubs, 6..) & points(2..=9),
        )
        .rule(
            Bid::new(3, Strain::Diamonds),
            1.1,
            len(Suit::Diamonds, 6..) & points(2..=9),
        )
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer after `1M – (2NT) – 3♦ – (P)` — partner's game force with
/// 5+ in the other major
///
/// Raise the shown major to game with 3+, else `3NT` with both minors
/// stopped, else rebid a 6-card opening major; the low-weight `3NT` is the
/// finite catch-all (the node is forced — partner's `3♦` is unbounded).
/// A slow forcing `3OM` probe is a deferral; opposite 13+ the blast is sound.
fn uvu_fourth_suit_answer(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om = unbid_major(major).expect("a major opening has an unbid major");
    let om_strain = Strain::from(om);

    Rules::new()
        .rule(Bid::new(4, om_strain), 1.5, len(om, 3..))
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            stopper_in(Suit::Clubs) & stopper_in(Suit::Diamonds),
        )
        .rule(Bid::new(4, m), 1.0, len(major, 6..))
        .rule(Bid::new(3, Strain::Notrump), 0.2, hcp(0..))
}

/// Opener's answer to a Cachalot rotation showing 4+ in `shown` after our
/// minor opening and their `over` overcall
///
/// The memo's ladder: raise to two with four trumps; **complete at the one
/// level with exactly three** (forcing one round — the convention's payoff);
/// name the fourth suit naturally; `1NT` with their suit stopped (it does not
/// deny three trumps — the completion simply outweighs it); rebid a 5-card
/// opening minor; the low-weight `1NT` is the finite catch-all (the rotation
/// is forcing).
fn cachalot_answer(opening: Suit, over: Suit, shown: Suit) -> Rules {
    let m = Strain::from(shown);
    let mut rules = Rules::new()
        .rule(Bid::new(2, m), 1.3, len(shown, 4..))
        .rule(Bid::new(1, m), 1.2, len(shown, 3..=3))
        .alert(CACHALOT_THREE);
    if shown == Suit::Hearts {
        // The fourth suit at the one level (spades, when hearts were shown
        // over their (1♦)).
        rules = rules.rule(Bid::new(1, Strain::Spades), 1.1, len(Suit::Spades, 4..));
    }
    rules
        .rule(Bid::new(1, Strain::Notrump), 1.0, stopper_in(over))
        .rule(Bid::new(2, Strain::from(opening)), 0.9, len(opening, 5..))
        .rule(Bid::new(1, Strain::Notrump), 0.2, hcp(0..))
}

/// Opener's answer to the Cachalot takeout `1♠` — as over a Sputnik double
///
/// Partner denied four cards in every rotation major, so there is no fit to
/// hunt: `1NT` with their suit stopped, a natural 5-card rebid, else the
/// cheapest rebid of the opening minor as the finite catch-all.
fn cachalot_takeout_answer(opening: Suit, over: Suit) -> Rules {
    let o = Strain::from(opening);
    Rules::new()
        .rule(Bid::new(1, Strain::Notrump), 1.0, stopper_in(over))
        .rule(Bid::new(2, o), 0.9, len(opening, 5..))
        .rule(Bid::new(2, o), 0.2, hcp(0..))
}

/// Opener's answer to a Cachalot `X` transfer once LHO has competed — hearts
/// over `(1♦)`, spades over `(1♥)`.
///
/// The pass-out completion is authored separately (and stays right-sided). This
/// is the *contested* branch, which the floor otherwise misjudges (the measured
/// `X·wrapped` leak): a rebase to the natural auction can't help because the
/// natural continuation is itself floored, so opener's raise is authored
/// directly here.  Opener knows partner holds 4+ `shown`, so it raises the fit
/// at the level the competition forces — `last_bid` fixes the cheapest legal
/// naming of the major, four-card support jumps a level — else passes to defend.
/// The gain is reaching the major games Modern's natural response finds and the
/// bare double misses.
fn cachalot_x_contested_answer(shown: Suit) -> impl Classifier {
    let m = Strain::from(shown);
    classifier(move |hand, context| {
        let mut rules = Rules::new();
        if let Some(bid) = context.last_bid() {
            // The cheapest legal level to name our major above their last bid;
            // when they bid our major the `+1` raises past it, still gated on
            // real support by `len` below.
            let level = if m > bid.strain {
                bid.level.get()
            } else {
                bid.level.get() + 1
            };
            if level < 7 {
                rules = rules.rule(Bid::new(level + 1, m), 1.3, len(shown, 4..));
            }
            if level <= 7 {
                rules = rules.rule(Bid::new(level, m), 1.2, len(shown, 3..));
            }
        }
        rules
            .rule(Call::Pass, 0.2, hcp(0..))
            .classify(hand, context)
    })
}

// ---------------------------------------------------------------------------
// Section 11: over their takeout double (`set_jordan_truscott`)
// ---------------------------------------------------------------------------

/// Responder's first call after our 1-suit opening and their takeout double
///
/// Over a double the meanings genuinely change, so the whole first call is
/// re-authored (total table); every *deeper* continuation still rides the
/// shipped systems-on rebase below this node. Jordan/Truscott `2NT` = limit+
/// raise (4+ support majors, 5+ minors); `XX` = 10+ without that fit; the
/// jump raise **flips preemptive**; 1-level suits stay forcing-as-uncontested
/// (their continuations rebase onto the uncontested tree); 2-level new suits
/// are weak and non-forcing (2/1 is off over the double); `1NT` natural 6–9.
fn doubled_opening_responder(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);
    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let jordan_min: usize = if is_major { 4 } else { 5 };
    let raise_min: usize = if is_major { 3 } else { 5 };
    let xx_max: usize = if is_major { 3 } else { 4 };

    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            len(o, jordan_min..) & points(10..),
        )
        .alert(JORDAN)
        .rule(Call::Redouble, 1.6, hcp(10..) & len(o, ..=xx_max))
        .alert(VALUE_REDOUBLE)
        .rule(
            Bid::new(3, o_strain),
            1.5,
            len(o, jordan_min..) & points(..=9),
        )
        .rule(
            Bid::new(2, o_strain),
            1.4,
            len(o, raise_min..) & points(6..=9),
        );
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let xs = Strain::from(x);
        rules = rules
            .rule(
                Bid::new(1, xs),
                1.3,
                min_level_is(1, xs) & len(x, 4..) & points(6..),
            )
            .rule(
                Bid::new(2, xs),
                1.2,
                min_level_is(2, xs) & len(x, 5..) & points(6..=9),
            );
    }
    rules
        .rule(Bid::new(1, Strain::Notrump), 1.1, hcp(6..=9))
        .rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's rebid over responder's value redouble and their pass
/// (`1x – (X) – XX – (P)`, behind [`set_redouble_answer`])
///
/// Partner holds 10+ HCP with at most three (majors; four minors) of our suit:
/// the deal belongs to us, and the partnership's plan is to penalize their
/// runout or buy the redoubled contract.  The rebase would strip both the
/// double and the redouble and replay opener uncontested, where partner's
/// shown strength reads as silence — the floor then re-prices a shaped minimum
/// as game-going and blasts a stopperless 3NT.  Sound bridge is **pass**,
/// full stop: even (especially) a long-suit minimum — one-of-a-suit
/// redoubled with six-plus trumps makes with overtricks, while any pull
/// forfeits the redoubled bonus and reopens the auction for their runout (a
/// 2M-escape rung measured −11 IMPs/fired in the smoke A/B before it was
/// deleted).  Extras act naturally on the next round once they run.
fn answer_value_redouble() -> Rules {
    Rules::new().rule(Call::Pass, 0.6, hcp(0..))
}

/// Opener's answer to the flipped preemptive jump raise (`1x – (X) – 3x`)
///
/// The rebase would misread it as the uncontested limit raise, so this node
/// shadows it: game only with genuine extras, else pass the preempt out.
fn answer_preemptive_raise(opening: Suit) -> Rules {
    let o_strain = Strain::from(opening);
    let game = if matches!(opening, Suit::Hearts | Suit::Spades) {
        Rules::new().rule(Bid::new(4, o_strain), 0.9, points(17..))
    } else {
        Rules::new().rule(Bid::new(5, o_strain), 0.9, points(19..))
    };
    game.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's answer to a weak non-forcing 2-level new suit (`1x – (X) – 2y`)
///
/// The rebase would misread it as a 2/1 game force, so this node shadows it:
/// raise with a fit and real extras, else pass.
fn answer_weak_new_suit(x: Suit) -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::from(x)),
            0.9,
            len(x, 4..) & points(15..),
        )
        .rule(Call::Pass, 0.3, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 10: their jump / 3-level overcalls (`set_high_overcall_responses`)
// ---------------------------------------------------------------------------

/// Responder after our opening `o` and their suit overcall in `2NT < bid ≤ 3♠`
///
/// The 4-level cue is deliberately dropped (a game-forcing raise doubles
/// first or blasts game), which keeps the Section-4b/4c cue-raise guards
/// untouched. Their `2NT` is excluded by the guard — in the NATURAL family a
/// `(2NT)` jump over our opening is a two-suiter, never natural.
fn over_their_high_overcall(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);
    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };

    let mut rules = Rules::new().rule(
        Bid::new(3, Strain::Notrump),
        1.7,
        points(13..) & stopper_in_their_suits(),
    );

    // Forcing 3-level new suits — strength does the forcing.
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let xs = Strain::from(x);
        rules = rules.rule(
            Bid::new(3, xs),
            1.45,
            min_level_is(3, xs) & !they_bid(xs) & len(x, 5..) & points(13..),
        );
    }

    // The negative double, strength scaled to the level.
    rules = if is_major {
        let om = unbid_major(o).expect("a major opening has an unbid major");
        let om_strain = Strain::from(om);
        rules
            .rule(
                Call::Double,
                1.0,
                len(om, 4..) & hcp(10..) & !they_bid(om_strain),
            )
            .alert(NEGATIVE_DOUBLE)
    } else {
        rules
            .rule(
                Call::Double,
                1.0,
                (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & hcp(10..),
            )
            .alert(NEGATIVE_DOUBLE)
    };

    // Raises: competitive at the 3 level, game with real extras (majors) or
    // preemptive on shape (minors).
    rules = rules.rule(
        Bid::new(3, o_strain),
        1.3,
        min_level_is(3, o_strain) & len(o, raise_min..) & points(6..),
    );
    rules = if is_major {
        rules.rule(Bid::new(4, o_strain), 1.25, len(o, 4..) & points(11..))
    } else {
        rules.rule(Bid::new(4, o_strain), 1.25, len(o, 5..) & points(..=9))
    };

    rules.rule(Call::Pass, 0.0, hcp(0..))
}

/// Opener's forced answer to partner's negative double of a 3-level overcall
///
/// Bid an unbid major at the cheapest legal level with four; `3NT` with their
/// suit stopped; rebid a 6-card opening suit; else the 3-card major
/// tolerance, with a low-weight `3NT` as the finite catch-all (the double is
/// forcing — `3NT` is always legal under the ≤3♠ guard).
fn answer_high_neg_double(opening: Suit) -> Rules {
    let o_strain = Strain::from(opening);
    let mut rules = Rules::new();

    for m in [Suit::Hearts, Suit::Spades] {
        if m == opening {
            continue;
        }
        let ms = Strain::from(m);
        rules = rules
            .rule(
                Bid::new(3, ms),
                1.2,
                min_level_is(3, ms) & len(m, 4..) & !they_bid(ms),
            )
            .rule(
                Bid::new(4, ms),
                1.1,
                min_level_is(4, ms) & len(m, 4..) & !they_bid(ms),
            );
    }
    rules = rules
        .rule(Bid::new(3, Strain::Notrump), 1.0, stopper_in_their_suits())
        .rule(
            Bid::new(3, o_strain),
            0.9,
            min_level_is(3, o_strain) & len(opening, 6..),
        )
        .rule(
            Bid::new(4, o_strain),
            0.85,
            min_level_is(4, o_strain) & len(opening, 6..),
        );
    for m in [Suit::Hearts, Suit::Spades] {
        if m == opening {
            continue;
        }
        let ms = Strain::from(m);
        rules = rules
            .rule(
                Bid::new(3, ms),
                0.3,
                min_level_is(3, ms) & len(m, 3..) & !they_bid(ms),
            )
            .rule(
                Bid::new(4, ms),
                0.25,
                min_level_is(4, ms) & len(m, 3..) & !they_bid(ms),
            );
    }
    rules.rule(Bid::new(3, Strain::Notrump), 0.15, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 7: our contested weak twos (`set_weak_two_competition`)
// ---------------------------------------------------------------------------

/// Responder after our weak two in `our` and their takeout double
///
/// The uncontested responses ride unchanged — Ogust `2NT` still asks, raises
/// stay preemptive (RONF), the forcing new suits survive — plus a business
/// redouble: 13+ values without the 2-card fit Ogust wants (a fit-and-values
/// hand still prefers the ask, whose weight sits above).
fn weak_two_doubled_responder(our: Suit) -> Rules {
    weak_twos::responses(our)
        .rule(Call::Redouble, 1.8, hcp(13..))
        .alert(WEAK_TWO_XX)
}

/// Responder after our weak two in `our` and their overcall (≤ 3♠)
///
/// Ogust survives when `2NT` is still available (their overcall ≤ 2♠); `X` is
/// a penalty-leaning values double (the floor's settle machinery answers it —
/// sit on a stack, pull with shape); the raises stay preemptive at *any*
/// strength — blocking, not inviting (RONF).
fn weak_two_overcalled_responder(our: Suit) -> Rules {
    let trump = Strain::from(our);
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            min_level_is(2, Strain::Notrump) & len(our, 2..) & points(14..),
        )
        .alert(CONTESTED_OGUST)
        .rule(Call::Double, 1.6, hcp(11..))
        .rule(
            Bid::new(3, trump),
            1.3,
            min_level_is(3, trump) & len(our, 3..),
        )
        .rule(Bid::new(4, trump), 1.25, len(our, 4..))
        .rule(Call::Pass, 0.0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 8: our contested strong 2♣ (`set_strong_two_competition`)
// ---------------------------------------------------------------------------

/// Responder after our strong 2♣ and their overcall
///
/// Natural game-forcing new suits keep the uncontested positive shape (5+
/// suit to two top honors, 8+), legality-anchored so exactly one rung fires;
/// `2NT`/`3NT` is the balanced positive with their suit stopped; `X` shows
/// "cards" (6+ HCP, penalty-leaning opposite 22+ — shadowing the floor's
/// *takeout* reading, the bug this table fixes); **Pass is waiting**, safe
/// because opener's reopening node below never sells out.
fn strong_two_overcalled_responder() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.3,
            min_level_is(2, Strain::Notrump) & hcp(8..) & balanced() & stopper_in_their_suits(),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.3,
            min_level_is(3, Strain::Notrump) & hcp(8..) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 1.2, hcp(6..))
        .rule(Call::Pass, 0.5, hcp(0..));
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(x);
        for level in 2..=3u8 {
            rules = rules.rule(
                Bid::new(level, strain),
                1.5,
                min_level_is(level, strain) & len(x, 5..) & top_honors(x, 2..) & points(8..),
            );
        }
    }
    rules
}

/// Opener's forced reopening after `2♣ – (overcall) – P – (P)`
///
/// A 22+ hand never sells out to an overcall: natural 5+ suit rebids
/// (legality-anchored rungs), notrump with their suit stopped, and a "cards"
/// double as the finite catch-all — partner decides whether to defend.
fn strong_two_reopening() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            1.2,
            min_level_is(2, Strain::Notrump) & balanced() & stopper_in_their_suits(),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            1.2,
            min_level_is(3, Strain::Notrump) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 0.4, hcp(0..));
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(x);
        for level in 2..=3u8 {
            rules = rules.rule(
                Bid::new(level, strain),
                1.0,
                min_level_is(level, strain) & len(x, 5..),
            );
        }
    }
    rules
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
    rules = rules
        .rule(Call::Double, 1.55, points(8..))
        .alert(MULTI_TAKEOUT);

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

    // Section 2b: systems-on over their double of our splinter. A splinter is
    // game-forcing, but the double reroutes opener into this book, where —
    // unauthored — it fell to the floor and passed out the doubled game force.
    // The `FirstIs(Double)` rebase strips the double off the whole subtree, so
    // opener (and responder's keycard answers) resolve on the undisturbed
    // splinter continuation. See `set_splinter_doubled`.
    if splinter_doubled() {
        for major in [Suit::Hearts, Suit::Spades] {
            let m_strain = Strain::from(major);
            let splinter_suits: &[Suit] = if major == Suit::Hearts {
                &[Suit::Spades, Suit::Clubs, Suit::Diamonds]
            } else {
                &[Suit::Clubs, Suit::Diamonds, Suit::Hearts]
            };
            for &x in splinter_suits {
                let (level, strain) = super::responses::splinter_bid(major, x);
                let suffix = [call(1, m_strain), Call::Pass, call(level, strain)];
                fallback_all_seats(
                    &mut book,
                    &suffix,
                    3,
                    Arc::new(FirstIs(Call::Double)),
                    Fallback::rebase(ReplaceNext(Call::Pass)),
                );
            }
        }
    }

    // Section 3: support doubles and redoubles for each (opening, major)
    // pair. The four minor-major pairs always; `1♥ – (P) – 1♠` behind
    // `set_major_support_double` (default on).
    let mut support_pairs = vec![
        (Suit::Clubs, Suit::Hearts),
        (Suit::Clubs, Suit::Spades),
        (Suit::Diamonds, Suit::Hearts),
        (Suit::Diamonds, Suit::Spades),
    ];
    if major_support_double() {
        support_pairs.push((Suit::Hearts, Suit::Spades));
    }
    for (opening, major) in support_pairs {
        let suffix = [
            call(1, Strain::from(opening)),
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
            Arc::new(SuffixIs(vec![Call::Double])),
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

    // Section 4: opener answers partner's negative double of a two-level minor.
    // Suffix is [1M]; guard checks that suffix is [2m, X, P].
    for major in [Suit::Hearts, Suit::Spades] {
        fallback_all_seats(
            &mut book,
            &[call(1, Strain::from(major))],
            3,
            Arc::new(described_guard(
                "2♣/2♦ X -",
                guard(|_: &Context<'_>, suffix: &[Call]| {
                    matches!(
                        suffix,
                        [Call::Bid(b), Call::Double, Call::Pass]
                            if b.level.get() == 2
                                && (b.strain == Strain::Clubs || b.strain == Strain::Diamonds)
                    )
                }),
            )),
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
                Arc::new(described_guard(
                    "(overcall ≤2♠) cue -",
                    guard(move |_: &Context<'_>, suffix: &[Call]| {
                        matches!(
                            suffix,
                            [Call::Bid(ovc), Call::Bid(cue), Call::Pass]
                                if cue.strain == ovc.strain
                                    && cue > ovc
                                    && *ovc <= Bid::new(2, Strain::Spades)
                                    && ovc.strain != trump
                        )
                    }),
                )),
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
                Arc::new(described_guard(
                    "(overcall) (cue ≤3♠) -",
                    guard(move |_: &Context<'_>, suffix: &[Call]| {
                        matches!(
                            suffix,
                            [Call::Bid(ovc), Call::Bid(cue), Call::Pass]
                                if cue.strain == ovc.strain
                                    && cue > ovc
                                    && *cue <= Bid::new(3, Strain::Spades)
                                    && ovc.strain != trump
                        )
                    }),
                )),
                Fallback::classify(answer_cue_minor_raise(minor)),
            );
        }
    }

    // Section 4d: opener answers responder's natural free bid (a non-jump new
    // suit over their overcall ≤2♠), forcing one round at both levels — the
    // free-bid-quality A/B's worst vulnerable-PD boards were opener *passing*
    // a game-going free bid out. Suffix guard mirrors the free-bid authoring
    // in `over_their_overcall`: overcall ≤2♠ and not a cue of our suit (4b/4c
    // own the cue-raises), the free bid a cheapest-level new suit that is
    // neither their suit nor ours nor notrump (the free 1NT/2NT are
    // non-forcing). Cachalot rotates the 1-level calls over its minor
    // openings, so those stay with the Section-9 completions (whose deeper
    // keys shadow this entry anyway — the `rotated` conjunct is
    // defense-in-depth and honest rendering); its natural 2-level frees get
    // the forcing answers like every other school's. The 2-level free-bid
    // *style* carves this node further: `Negative` sends the level-2 frees
    // to 4d′ (non-forcing answers, with Pass), `Transfer` sends the swapped
    // slots to the Section-4f completions (a lone or three-way slot stays
    // natural-forcing and keeps the 4d answers).
    if free_bids_engaged() {
        let cachalot = negative_double_shape() == NegativeDoubleShape::Cachalot;
        let negative = free_bid_style() == FreeBidStyle::Negative;
        let transfer = free_bid_style() == FreeBidStyle::Transfer;
        for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let o_strain = Strain::from(opening);
            let rotated = cachalot && matches!(opening, Suit::Clubs | Suit::Diamonds);
            fallback_all_seats(
                &mut book,
                &[call(1, o_strain)],
                3,
                Arc::new(described_guard(
                    if rotated {
                        "(overcall ≤2♠) 2-level free-suit -"
                    } else {
                        "(overcall ≤2♠) free-suit -"
                    },
                    guard(move |_: &Context<'_>, suffix: &[Call]| {
                        matches!(
                            suffix,
                            [Call::Bid(ovc), Call::Bid(free), Call::Pass]
                                if *ovc <= Bid::new(2, Strain::Spades)
                                    && ovc.strain != o_strain
                                    && free.strain != Strain::Notrump
                                    && free.strain != ovc.strain
                                    && free.strain != o_strain
                                    && free.level.get()
                                        == ovc.level.get() + u8::from(free.strain < ovc.strain)
                                    && !(rotated && free.level.get() == 1)
                                    && !(negative && free.level.get() == 2)
                                    && !(transfer
                                        && free.level.get() == 2
                                        && two_level_slots(o_strain, *ovc) == 2)
                        )
                    }),
                )),
                Fallback::classify(answer_free_bid(opening)),
            );

            // Section 4d′ (`FreeBidStyle::Negative`): the capped, non-forcing
            // level-2 frees get answers WITH a Pass catch-all.
            if negative {
                fallback_all_seats(
                    &mut book,
                    &[call(1, o_strain)],
                    3,
                    Arc::new(described_guard(
                        "(overcall ≤2♠) negative free-suit -",
                        guard(move |_: &Context<'_>, suffix: &[Call]| {
                            matches!(
                                suffix,
                                [Call::Bid(ovc), Call::Bid(free), Call::Pass]
                                    if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                                        && free.strain != Strain::Notrump
                                        && free.strain != ovc.strain
                                        && free.strain != o_strain
                                        && free.level.get() == 2
                                        && free.level.get()
                                            == ovc.level.get()
                                                + u8::from(free.strain < ovc.strain)
                            )
                        }),
                    )),
                    Fallback::classify(answer_negative_free_bid(opening)),
                );

                // Section 4d″: the doubler's rebid over opener's answer — a
                // new suit is the strong hand the capped free bid could not
                // carry, forcing to game. This node also claims the ordinary
                // doubler's second turn (previously floored — bucket
                // X-then-Pass vs X-then-suit in the forensics).
                fallback_all_seats(
                    &mut book,
                    &[call(1, o_strain)],
                    3,
                    Arc::new(described_guard(
                        "(overcall ≤2♠) X - answer -",
                        guard(move |_: &Context<'_>, suffix: &[Call]| {
                            matches!(
                                suffix,
                                [Call::Bid(ovc), Call::Double, Call::Pass, Call::Bid(_), Call::Pass]
                                    if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                            )
                        }),
                    )),
                    Fallback::classify(negative_doubler_rebid(opening)),
                );

                // Section 4d‴: opener answers the game-forcing rebid with the
                // ordinary forcing-answer table; the guard's `< 3 of the
                // opening suit` scope keeps that table's catch-all legal.
                fallback_all_seats(
                    &mut book,
                    &[call(1, o_strain)],
                    3,
                    Arc::new(described_guard(
                        "(overcall ≤2♠) X - answer - FG-suit -",
                        guard(move |_: &Context<'_>, suffix: &[Call]| {
                            matches!(
                                suffix,
                                [Call::Bid(ovc), Call::Double, Call::Pass, Call::Bid(ans), Call::Pass, Call::Bid(new), Call::Pass]
                                    if *ovc <= Bid::new(2, Strain::Spades)
                                        && ovc.strain != o_strain
                                        && new.strain != Strain::Notrump
                                        && new.strain != ovc.strain
                                        && new.strain != o_strain
                                        && new.strain != ans.strain
                                        && *new < Bid::new(3, o_strain)
                            )
                        }),
                    )),
                    Fallback::classify(answer_free_bid(opening)),
                );
            }
        }
    }

    // Section 4f (`FreeBidStyle::Transfer`): opener completes the 2-level
    // free-bid transfer and responder clarifies. The swap contexts are a
    // closed enumeration — (opening, their overcall, lower slot → shown,
    // wrap slot → shown, completing a level higher on the wrap):
    if free_bids_engaged() && free_bid_style() == FreeBidStyle::Transfer {
        #[allow(clippy::type_complexity)]
        #[rustfmt::skip]
        let swaps: [(Strain, u8, Strain, [(Strain, Suit); 2]); 7] = [
            (Strain::Clubs, 1, Strain::Spades, [(Strain::Diamonds, Suit::Hearts), (Strain::Hearts, Suit::Diamonds)]),
            (Strain::Clubs, 2, Strain::Diamonds, [(Strain::Hearts, Suit::Spades), (Strain::Spades, Suit::Hearts)]),
            (Strain::Diamonds, 1, Strain::Spades, [(Strain::Clubs, Suit::Hearts), (Strain::Hearts, Suit::Clubs)]),
            (Strain::Diamonds, 2, Strain::Clubs, [(Strain::Hearts, Suit::Spades), (Strain::Spades, Suit::Hearts)]),
            (Strain::Hearts, 1, Strain::Spades, [(Strain::Clubs, Suit::Diamonds), (Strain::Diamonds, Suit::Clubs)]),
            (Strain::Hearts, 2, Strain::Clubs, [(Strain::Diamonds, Suit::Spades), (Strain::Spades, Suit::Diamonds)]),
            (Strain::Spades, 2, Strain::Clubs, [(Strain::Diamonds, Suit::Hearts), (Strain::Hearts, Suit::Diamonds)]),
        ];
        for (o_strain, ovc_level, ovc_strain, slots) in swaps {
            for (slot, shown) in slots {
                let shown_strain = Strain::from(shown);
                let comp_lvl = if shown_strain > slot { 2 } else { 3 };
                let cue_lvl = comp_lvl + u8::from(ovc_strain < shown_strain);
                fallback_all_seats(
                    &mut book,
                    &[call(1, o_strain), call(ovc_level, ovc_strain)],
                    3,
                    Arc::new(SuffixIs(vec![call(2, slot), Call::Pass])),
                    Fallback::classify(free_transfer_completion(shown, comp_lvl)),
                );
                fallback_all_seats(
                    &mut book,
                    &[call(1, o_strain), call(ovc_level, ovc_strain)],
                    3,
                    Arc::new(SuffixIs(vec![
                        call(2, slot),
                        Call::Pass,
                        call(comp_lvl, shown_strain),
                        Call::Pass,
                    ])),
                    Fallback::classify(free_transfer_clarify(
                        shown,
                        comp_lvl,
                        Bid::new(cue_lvl, ovc_strain),
                    )),
                );
            }
        }
    }

    // Section 6: their two-suiters over our 1M (`set_uvu_over_majors`, default
    // on): unusual-vs-unusual over their both-minors (2NT), and a raise
    // structure over their Michaels cue of our own major. Keyed at the deeper
    // [1M, <their call>] nodes — their cue and their 2NT are single concrete
    // calls — so these shadow the [1M] direct-seat package (whose negative
    // double misfires over a Michaels cue) with no declaration-order race.
    if uvu_over_majors() {
        for major in [Suit::Hearts, Suit::Spades] {
            let trump = Strain::from(major);
            let open = call(1, trump);
            let unusual = call(2, Strain::Notrump);
            let michaels = call(2, trump);
            let om_cue = if major == Suit::Hearts {
                call(2, Strain::Spades)
            } else {
                call(3, Strain::Hearts)
            };

            // Their (2NT): responder, then opener's answers to the two cues.
            // The limit-plus 3♣ cue reuses the shipped cue-raise answer table.
            fallback_all_seats(
                &mut book,
                &[open, unusual],
                3,
                Arc::new(SuffixIs(vec![])),
                Fallback::classify(uvu_major_responder(major)),
            );
            fallback_all_seats(
                &mut book,
                &[open, unusual],
                3,
                Arc::new(SuffixIs(vec![call(3, Strain::Clubs), Call::Pass])),
                Fallback::classify(answer_cue_raise(major)),
            );
            fallback_all_seats(
                &mut book,
                &[open, unusual],
                3,
                Arc::new(SuffixIs(vec![call(3, Strain::Diamonds), Call::Pass])),
                Fallback::classify(uvu_fourth_suit_answer(major)),
            );

            // Their Michaels cue of our major: responder, then opener's answer
            // to the other-major cue (again the shipped cue-raise table — its
            // accept/decline shape is cue-agnostic).
            fallback_all_seats(
                &mut book,
                &[open, michaels],
                3,
                Arc::new(SuffixIs(vec![])),
                Fallback::classify(michaels_cue_responder(major)),
            );
            fallback_all_seats(
                &mut book,
                &[open, michaels],
                3,
                Arc::new(SuffixIs(vec![om_cue, Call::Pass])),
                Fallback::classify(answer_cue_raise(major)),
            );
        }
    }

    // Section 11: over their takeout double (`set_jordan_truscott`, default
    // on). Responder's first call at the deeper [1x, X] key — it wins over
    // the [1x] FirstIs(X) systems-on rebase structurally, and the rebase
    // survives untouched below it for every deeper suffix these exact-suffix
    // guards don't claim. Opener nodes shadow exactly the three rebase
    // misreads: Jordan-onto-Jacoby-2NT, preemptive-3x-onto-limit-raise, and
    // weak-2y-onto-2/1.
    if jordan_truscott() {
        for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let o_strain = Strain::from(opening);
            let key = [call(1, o_strain), Call::Double];

            fallback_all_seats(
                &mut book,
                &key,
                3,
                Arc::new(SuffixIs(vec![])),
                Fallback::classify(doubled_opening_responder(opening)),
            );
            fallback_all_seats(
                &mut book,
                &key,
                3,
                Arc::new(SuffixIs(vec![call(2, Strain::Notrump), Call::Pass])),
                Fallback::classify(match opening {
                    Suit::Hearts | Suit::Spades => answer_cue_raise(opening),
                    minor => answer_cue_minor_raise(minor),
                }),
            );
            fallback_all_seats(
                &mut book,
                &key,
                3,
                Arc::new(SuffixIs(vec![call(3, o_strain), Call::Pass])),
                Fallback::classify(answer_preemptive_raise(opening)),
            );
            // Opener over the value redouble: the rebase replays it as an
            // uncontested rebid with responder's 10+ unseen, so the floor
            // blasts; shadow it with the pass-only answer.
            if redouble_answer() {
                fallback_all_seats(
                    &mut book,
                    &key,
                    3,
                    Arc::new(SuffixIs(vec![Call::Redouble, Call::Pass])),
                    Fallback::classify(answer_value_redouble()),
                );
            }
            for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                if Strain::from(x) >= o_strain {
                    continue;
                }
                fallback_all_seats(
                    &mut book,
                    &key,
                    3,
                    Arc::new(SuffixIs(vec![call(2, Strain::from(x)), Call::Pass])),
                    Fallback::classify(answer_weak_new_suit(x)),
                );
            }
        }
    }

    // Section 10: their jump / 3-level suit overcalls
    // (`set_high_overcall_responses`, default off). A second guarded entry at
    // [1x] — its bid range (2NT, 3♠] is disjoint from the shipped
    // OvercallAtMost(2♠) entry, so declaration order is irrelevant. Their
    // (2NT) and their 3-level cue of our own suit are excluded (the first is
    // a two-suiter, the second is rare enough for the floor).
    if high_overcall_responses() {
        for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let o_strain = Strain::from(opening);
            let open = call(1, o_strain);
            fallback_all_seats(
                &mut book,
                &[open],
                3,
                Arc::new(described_guard(
                    "(2NT < overcall ≤3♠)",
                    guard(move |_: &Context<'_>, s: &[Call]| {
                        matches!(s, [Call::Bid(b)]
                            if *b > Bid::new(2, Strain::Notrump)
                                && *b <= Bid::new(3, Strain::Spades)
                                && b.strain.is_suit()
                                && b.strain != o_strain)
                    }),
                )),
                Fallback::classify(over_their_high_overcall(opening)),
            );
            fallback_all_seats(
                &mut book,
                &[open],
                3,
                Arc::new(described_guard(
                    "(2NT < overcall ≤3♠) X -",
                    guard(move |_: &Context<'_>, s: &[Call]| {
                        matches!(s, [Call::Bid(b), Call::Double, Call::Pass]
                            if *b > Bid::new(2, Strain::Notrump)
                                && *b <= Bid::new(3, Strain::Spades)
                                && b.strain.is_suit()
                                && b.strain != o_strain)
                    }),
                )),
                Fallback::classify(answer_high_neg_double(opening)),
            );
        }
    }

    // Section 9: opener's Cachalot answers (`NegativeDoubleShape::Cachalot`
    // only). The rotated calls are forcing; each gets its completion table at
    // the deeper [1m, <their 1-level overcall>] key.
    if negative_double_shape() == NegativeDoubleShape::Cachalot {
        let one_heart = call(1, Strain::Hearts);
        let one_spade = call(1, Strain::Spades);

        // (1♦) over 1♣: X shows hearts, 1♥ shows spades, 1♠ is the takeout.
        let clubs = call(1, Strain::Clubs);
        let d_ovc = call(1, Strain::Diamonds);
        fallback_all_seats(
            &mut book,
            &[clubs, d_ovc],
            3,
            Arc::new(SuffixIs(vec![Call::Double, Call::Pass])),
            Fallback::classify(cachalot_answer(Suit::Clubs, Suit::Diamonds, Suit::Hearts)),
        );
        fallback_all_seats(
            &mut book,
            &[clubs, d_ovc],
            3,
            Arc::new(SuffixIs(vec![one_heart, Call::Pass])),
            Fallback::classify(cachalot_answer(Suit::Clubs, Suit::Diamonds, Suit::Spades)),
        );
        fallback_all_seats(
            &mut book,
            &[clubs, d_ovc],
            3,
            Arc::new(SuffixIs(vec![one_spade, Call::Pass])),
            Fallback::classify(cachalot_takeout_answer(Suit::Clubs, Suit::Diamonds)),
        );

        // (1♥) over 1♣/1♦: X shows spades, 1♠ is the takeout.
        for opening in [Suit::Clubs, Suit::Diamonds] {
            let open = call(1, Strain::from(opening));
            fallback_all_seats(
                &mut book,
                &[open, one_heart],
                3,
                Arc::new(SuffixIs(vec![Call::Double, Call::Pass])),
                Fallback::classify(cachalot_answer(opening, Suit::Hearts, Suit::Spades)),
            );
            fallback_all_seats(
                &mut book,
                &[open, one_heart],
                3,
                Arc::new(SuffixIs(vec![one_spade, Call::Pass])),
                Fallback::classify(cachalot_takeout_answer(opening, Suit::Hearts)),
            );
        }

        // Contested X: LHO competed over the transfer, so the pass-out
        // completions above don't fire and opener would fall to the floor (the
        // measured X·wrapped leak).  Author opener's raise of the shown major —
        // hearts over (1♦), spades over (1♥).  The guard admits exactly opener's
        // immediate answer, [X, <their non-pass intervention>]: the [X, P]
        // pass-out is shadowed above, and deeper continuations fall to the floor
        // as before.
        if cachalot_contested_x() {
            let x_intervention = || {
                described_guard(
                    "X (their intervention) -",
                    guard(
                        |_: &Context<'_>, s: &[Call]| matches!(s, [Call::Double, c] if !matches!(c, Call::Pass)),
                    ),
                )
            };
            fallback_all_seats(
                &mut book,
                &[clubs, d_ovc],
                3,
                Arc::new(x_intervention()),
                Fallback::classify(cachalot_x_contested_answer(Suit::Hearts)),
            );
            for opening in [Suit::Clubs, Suit::Diamonds] {
                fallback_all_seats(
                    &mut book,
                    &[call(1, Strain::from(opening)), one_heart],
                    3,
                    Arc::new(x_intervention()),
                    Fallback::classify(cachalot_x_contested_answer(Suit::Spades)),
                );
            }
        }
    }

    // Section 9b: opener's answers to the Sputnik residual double
    // (`NegativeDoubleShape::Sputnik`). The double *denies* a biddable major,
    // so — unlike a classic negative double — opener must NOT raise a major:
    // the floor's "negative double = the unbid major" instinct is exactly
    // inverted here and would jump the phantom denied suit into a doubled game
    // (the measured leak). `cachalot_takeout_answer` bids NT/opening-minor
    // naturally instead. Over (1♠)/a 2-minor Sputnik's double is Modern's
    // major-showing one, which the floor reads correctly — left to it.
    if negative_double_shape() == NegativeDoubleShape::Sputnik {
        let one_heart = call(1, Strain::Hearts);
        // (1♦) over 1♣: X = ≤3 in both majors — no fit to hunt.
        fallback_all_seats(
            &mut book,
            &[call(1, Strain::Clubs), call(1, Strain::Diamonds)],
            3,
            Arc::new(SuffixIs(vec![Call::Double, Call::Pass])),
            Fallback::classify(cachalot_takeout_answer(Suit::Clubs, Suit::Diamonds)),
        );
        // (1♥) over 1♣/1♦: X = ≤3 spades.
        for opening in [Suit::Clubs, Suit::Diamonds] {
            fallback_all_seats(
                &mut book,
                &[call(1, Strain::from(opening)), one_heart],
                3,
                Arc::new(SuffixIs(vec![Call::Double, Call::Pass])),
                Fallback::classify(cachalot_takeout_answer(opening, Suit::Hearts)),
            );
        }
    }

    // Section 7: our contested weak twos (`set_weak_two_competition`, default
    // off). Their double: responder's first call at the deeper [2M, X] node
    // (business XX riding on the uncontested responses), everything deeper
    // systems-on. Their overcall (≤ 3♠): responder's direct action, and a
    // targeted rebase so an Ogust 2NT bid over the overcall still gets
    // opener's undisturbed five-rung answer.
    if weak_two_competition() {
        for our in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let trump = Strain::from(our);
            let open = call(2, trump);
            let two_nt = call(2, Strain::Notrump);

            fallback_all_seats(
                &mut book,
                &[open, Call::Double],
                3,
                Arc::new(SuffixIs(vec![])),
                Fallback::classify(weak_two_doubled_responder(our)),
            );
            fallback_all_seats(
                &mut book,
                &[open],
                3,
                Arc::new(FirstIs(Call::Double)),
                Fallback::rebase(ReplaceNext(Call::Pass)),
            );

            fallback_all_seats(
                &mut book,
                &[open],
                3,
                Arc::new(OvercallAtMost(Bid::new(3, Strain::Spades))),
                Fallback::classify(weak_two_overcalled_responder(our)),
            );
            fallback_all_seats(
                &mut book,
                &[open],
                3,
                Arc::new(described_guard(
                    "(overcall <2NT) 2NT …",
                    guard(move |_: &Context<'_>, s: &[Call]| {
                        matches!(s.first(), Some(&Call::Bid(b)) if b < Bid::new(2, Strain::Notrump))
                            && s.get(1) == Some(&two_nt)
                    }),
                )),
                Fallback::rebase(ReplaceNext(Call::Pass)),
            );
        }
    }

    // Section 8: our contested strong 2♣ (`set_strong_two_competition`,
    // default on). Their double steals no room → systems on wholesale; their
    // overcall gets responder's natural-GF / values-X / waiting-pass table,
    // backed by opener's forced reopening in the pass-out seat.
    if strong_two_competition() {
        let open = call(2, Strain::Clubs);

        fallback_all_seats(
            &mut book,
            &[open],
            3,
            Arc::new(FirstIs(Call::Double)),
            Fallback::rebase(ReplaceNext(Call::Pass)),
        );
        fallback_all_seats(
            &mut book,
            &[open],
            3,
            Arc::new(described_guard(
                "(overcall)",
                guard(|_: &Context<'_>, s: &[Call]| matches!(s, [Call::Bid(_)])),
            )),
            Fallback::classify(strong_two_overcalled_responder()),
        );
        fallback_all_seats(
            &mut book,
            &[open],
            3,
            Arc::new(described_guard(
                "(overcall) - -",
                guard(|_: &Context<'_>, s: &[Call]| {
                    matches!(s, [Call::Bid(_), Call::Pass, Call::Pass])
                }),
            )),
            Fallback::classify(strong_two_reopening()),
        );
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
            Fallback::rebase(described_rewrite(
                "systems on: their 2♣ is treated as a pass; X asks as the stolen 2♣ Stayman",
                rewriter(move |auction: &[Call], depth: usize| {
                    if auction.get(depth) != Some(&two_clubs) {
                        return None;
                    }
                    let mut rewritten = auction.to_vec();
                    rewritten[depth] = Call::Pass; // (2♣) steals no room → systems on
                    if auction.get(depth + 1) == Some(&Call::Double) {
                        rewritten[depth + 1] = two_clubs; // stolen 2♣ Stayman = Double
                    }
                    Some(rewritten)
                }),
            )),
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
            Arc::new(SuffixIs(vec![])),
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
                Arc::new(SuffixIs(vec![Call::Double, Call::Pass])),
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
                Arc::new(SuffixIs(vec![overcall])),
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
                    Arc::new(SuffixIs(vec![overcall, Call::Double, Call::Pass])),
                    Fallback::classify(reply),
                );
            }

            // Opener completes the 2NT relay with 3♣: suffix is [overcall, 2NT, P].
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(SuffixIs(vec![overcall, two_nt, Call::Pass])),
                Fallback::classify(complete_lebensohl_relay()),
            );

            // Responder's rebid after 3♣ (the weak relay sign-off): suffix is
            // [overcall, 2NT, P, 3♣, P].
            fallback_all_seats(
                &mut book,
                &[one_nt],
                3,
                Arc::new(SuffixIs(vec![
                    overcall,
                    two_nt,
                    Call::Pass,
                    three_clubs,
                    Call::Pass,
                ])),
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
                    Arc::new(SuffixIs(vec![
                        overcall,
                        two_nt,
                        Call::Pass,
                        three_clubs,
                        Call::Pass,
                        three_m,
                        Call::Pass,
                    ])),
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
                        Arc::new(SuffixIs(vec![overcall, two_m, Call::Pass])),
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
                    Arc::new(SuffixIs(vec![overcall, cue, Call::Pass])),
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
                        Arc::new(SuffixIs(vec![overcall, resp, Call::Pass])),
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
                    Arc::new(SuffixIs(vec![
                        overcall,
                        two_nt,
                        Call::Pass,
                        three_clubs,
                        Call::Pass,
                        cue,
                        Call::Pass,
                    ])),
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
                        Arc::new(SuffixIs(suffix)),
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
            Arc::new(SuffixIs(vec![Call::Double])),
            Fallback::classify(stayman_doubled_opener()),
        );
        // After opener's *stopper-bid* (suffix `[X, <bid>, …]`) responder's rebids
        // are identical to the uncontested tree: rebase by stripping the X to a
        // Pass, re-keying onto `[1NT, P, 2♣, P, <bid>, …]`.
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(described_guard(
                "X (bid) …",
                guard(|_: &Context<'_>, s: &[Call]| {
                    s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                }),
            )),
            Fallback::rebase(described_rewrite(
                "systems on: their X is stripped to a pass",
                rewriter(move |auction: &[Call], depth: usize| {
                    if auction.get(depth) != Some(&Call::Double) {
                        return None;
                    }
                    let mut rewritten = auction.to_vec();
                    rewritten[depth] = Call::Pass; // strip the X → systems on
                    Some(rewritten)
                }),
            )),
        );
        // Opener passed to deny a stopper; responder re-asks (suffix `[X, P, P]`).
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(SuffixIs(vec![Call::Double, Call::Pass, Call::Pass])),
            Fallback::classify(stayman_redouble_reask()),
        );
        // Opener's forced re-answer to the re-ask (suffix `[X, P, P, XX, P]`):
        // reuse `stayman_answers()` — no Pass rule (opener cannot sit), and its 2♦
        // is exactly the artificial "no major" denial.
        fallback_all_seats(
            &mut book,
            &stayman,
            3,
            Arc::new(SuffixIs(vec![
                Call::Double,
                Call::Pass,
                Call::Pass,
                Call::Redouble,
                Call::Pass,
            ])),
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
            Arc::new(described_guard(
                "- 2♦/2♥/2♠ X …",
                guard(|_: &Context<'_>, s: &[Call]| {
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
                }),
            )),
            Fallback::rebase(described_rewrite(
                "systems on: their X is stripped to a pass",
                rewriter(move |auction: &[Call], depth: usize| {
                    if auction.get(depth + 2) != Some(&Call::Double) {
                        return None;
                    }
                    let mut rewritten = auction.to_vec();
                    rewritten[depth + 2] = Call::Pass; // strip the X → systems on
                    Some(rewritten)
                }),
            )),
        );

        // A.2 — our Stayman overcalled at the 2-level.  Opener's natural reply.
        for over in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let overcall = call(2, Strain::from(over));
            fallback_all_seats(
                &mut book,
                &stayman,
                3,
                Arc::new(SuffixIs(vec![overcall])),
                Fallback::classify(stayman_overcalled_opener(over)),
            );
        }
    }

    // Competition over our own Jacoby transfers (`set_competition_over_transfer`,
    // default off): opener's replies after the opponents double `1NT-(P)-2♦/2♥-(X)`
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
                Arc::new(SuffixIs(vec![Call::Double])),
                Fallback::classify(transfer_doubled_opener(major, resp)),
            );
            // After opener completes/super-accepts (suffix `[X, <bid>, …]`)
            // responder's rebids match the uncontested tree: strip the X to a Pass,
            // re-keying onto `[1NT, P, 2♦/2♥, P, <bid>, …]`.
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(described_guard(
                    "X (bid) …",
                    guard(|_: &Context<'_>, s: &[Call]| {
                        s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                    }),
                )),
                Fallback::rebase(described_rewrite(
                    "systems on: their X is stripped to a pass",
                    rewriter(move |auction: &[Call], depth: usize| {
                        if auction.get(depth) != Some(&Call::Double) {
                            return None;
                        }
                        let mut rewritten = auction.to_vec();
                        rewritten[depth] = Call::Pass; // strip the X → systems on
                        Some(rewritten)
                    }),
                )),
            );
            // Opener passed to decline; responder re-asks (suffix `[X, P, P]`).
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(SuffixIs(vec![Call::Double, Call::Pass, Call::Pass])),
                Fallback::classify(transfer_pass_reask(major)),
            );
            // Opener's forced completion after the re-ask (suffix `[X, P, P, XX, P]`):
            // reuse `complete_transfer` — no Pass rule, so opener cannot sit.
            fallback_all_seats(
                &mut book,
                &transfer,
                3,
                Arc::new(SuffixIs(vec![
                    Call::Double,
                    Call::Pass,
                    Call::Pass,
                    Call::Redouble,
                    Call::Pass,
                ])),
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
                    Arc::new(SuffixIs(vec![overcall])),
                    Fallback::classify(transfer_overcalled_opener(major, over_suit, over_level)),
                );
            }
        }
    }

    // Competition over our own two-way 2♠ minor response (`set_competition_over_
    // minor_transfer`, default on): opener's replies after the opponents double
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
            Arc::new(SuffixIs(vec![Call::Double])),
            Fallback::classify(minor_doubled_opener()),
        );
        // After opener's stopper-bid (`2NT`/`3♣`, suffix `[X, <bid>, …]`) responder's
        // rebids match the uncontested tree: strip the `X` to a Pass, re-keying onto
        // `[1NT, P, 2♠, P, 2NT/3♣, …]` (the `two_spade_over_min`/`max` machinery).
        fallback_all_seats(
            &mut book,
            &two_spade,
            3,
            Arc::new(described_guard(
                "X (bid) …",
                guard(|_: &Context<'_>, s: &[Call]| {
                    s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                }),
            )),
            Fallback::rebase(described_rewrite(
                "systems on: their X is stripped to a pass",
                rewriter(move |auction: &[Call], depth: usize| {
                    if auction.get(depth) != Some(&Call::Double) {
                        return None;
                    }
                    let mut rewritten = auction.to_vec();
                    rewritten[depth] = Call::Pass; // strip the X → systems on
                    Some(rewritten)
                }),
            )),
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
                Arc::new(SuffixIs(deny.to_vec())),
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
                Arc::new(SuffixIs(vec![over])),
                Fallback::classify(rules),
            );
        }
    }

    // Competition over our own 2NT diamond transfer (`set_competition_over_
    // diamond_transfer`, default on): opener's replies after the opponents double
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
            Arc::new(SuffixIs(vec![Call::Double])),
            Fallback::classify(diamond_doubled_opener()),
        );
        // After opener's fit-showing bid (`3♦`/`3♣`) responder's rebids match the
        // uncontested tree: strip the `X` to a Pass.
        fallback_all_seats(
            &mut book,
            &two_nt,
            3,
            Arc::new(described_guard(
                "X (bid) …",
                guard(|_: &Context<'_>, s: &[Call]| {
                    s.first() == Some(&Call::Double) && matches!(s.get(1), Some(Call::Bid(_)))
                }),
            )),
            Fallback::rebase(described_rewrite(
                "systems on: their X is stripped to a pass",
                rewriter(move |auction: &[Call], depth: usize| {
                    if auction.get(depth) != Some(&Call::Double) {
                        return None;
                    }
                    let mut rewritten = auction.to_vec();
                    rewritten[depth] = Call::Pass; // strip the X → systems on
                    Some(rewritten)
                }),
            )),
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
                Arc::new(SuffixIs(deny.to_vec())),
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
                Arc::new(SuffixIs(vec![over])),
                Fallback::classify(rules),
            );
        }
    }

    // Section 5d: Unusual vs Unusual over a both-minors (2NT) overcall of our 1NT
    // (`set_uvu`, default on). Responder's `X` is penalty; `3♣`/`3♦` are
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
            Arc::new(SuffixIs(vec![overcall])),
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
                Arc::new(SuffixIs(suffix)),
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
        // 4-3-4-2, 17: a flat 4-3-3-3 17-count would read 16 on the shipped
        // rule-of-N+8 scale and rightly decline the stretch.
        let (c, floored) = bid(&after_signoff, "AK32.K43.A432.K3");
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

    #[test]
    fn uvu_major_cues_split_raise_and_fourth_suit() {
        super::set_uvu_over_majors(true);
        // [1♥, (2NT both minors)]: 12-count with 3 hearts → 3♣ = limit+ raise.
        let auction = [call(1, Strain::Hearts), call(2, Strain::Notrump)];
        let (raise, floored) = best_call(&auction, "K52.QJ5.A964.Q32");
        assert_eq!(raise, call(3, Strain::Clubs), "the cheap cue raises");
        assert!(!floored, "an authored node, not the floor");
        // 14-count, 5 spades, 2 hearts → 3♦ = game force in the other major.
        let (fourth, _) = best_call(&auction, "AQJ54.K5.965.A43");
        assert_eq!(fourth, call(3, Strain::Diamonds), "the second cue forces");
        super::set_uvu_over_majors(true);
    }

    #[test]
    fn michaels_cue_of_our_major_gets_a_structure() {
        super::set_uvu_over_majors(true);
        // [1♠, (2♠ Michaels)]: a limit raise cues their known major (3♥)...
        let auction = [call(1, Strain::Spades), call(2, Strain::Spades)];
        let (cue, floored) = best_call(&auction, "KQ5.A54.96432.Q2");
        assert_eq!(cue, call(3, Strain::Hearts), "the known-suit cue raises");
        assert!(!floored, "an authored node, not the floor");
        // ...while a competitive 7-count raises 3♠ naturally — the raise
        // keeps its meaning over their cue of our own suit.
        let (raise, _) = best_call(&auction, "Q542.95.9643.KQ3");
        assert_eq!(raise, call(3, Strain::Spades), "the natural raise survives");
        super::set_uvu_over_majors(true);
    }

    #[test]
    fn opener_answers_the_uvu_major_cue() {
        super::set_uvu_over_majors(true);
        // [1♥, (2NT), 3♣ = limit+ raise, (P)]: a minimum declines in 3♥, a
        // maximum accepts to game — the shipped cue-raise answer, rewired.
        let auction = [
            call(1, Strain::Hearts),
            call(2, Strain::Notrump),
            call(3, Strain::Clubs),
            Call::Pass,
        ];
        let (decline, floored) = best_call(&auction, "965.AQJ54.K54.32");
        assert_eq!(decline, call(3, Strain::Hearts), "a minimum signs off");
        assert!(!floored, "an authored node, not the floor");
        let (accept, _) = best_call(&auction, "65.AKQ54.KJ54.A2");
        assert_eq!(accept, call(4, Strain::Hearts), "a maximum accepts");
        super::set_uvu_over_majors(true);
    }

    #[test]
    fn opener_answers_the_uvu_fourth_suit_force() {
        super::set_uvu_over_majors(true);
        // [1♥, (2NT), 3♦ = GF 5+ spades, (P)]: three-card support raises the
        // shown major to game.
        let auction = [
            call(1, Strain::Hearts),
            call(2, Strain::Notrump),
            call(3, Strain::Diamonds),
            Call::Pass,
        ];
        let (game, floored) = best_call(&auction, "K65.AQJ54.K54.32");
        assert_eq!(game, call(4, Strain::Spades), "raise the game force");
        assert!(!floored, "an authored node, not the floor");
        super::set_uvu_over_majors(true);
    }

    #[test]
    fn weak_two_doubled_gets_business_redouble_and_systems_on() {
        super::set_weak_two_competition(true);
        // [2♠, (X)]: 17-count with a singleton spade — no Ogust fit — redoubles.
        let auction = [call(2, Strain::Spades), Call::Double];
        let (xx, floored) = best_call(&auction, "A.K654.A964.KQ32");
        assert_eq!(xx, Call::Redouble, "business redouble on values");
        assert!(!floored, "an authored node, not the floor");
        // A 3-card raise stays preemptive (RONF rides through unchanged).
        let (raise, _) = best_call(&auction, "954.Q542.964.432");
        assert_eq!(raise, call(3, Strain::Spades), "the raise stays preemptive");
        // Deeper continuations are systems-on: opener answers Ogust through
        // the rebase exactly as if undisturbed (min points, good suit → 3♦).
        let ogust = [
            call(2, Strain::Hearts),
            Call::Double,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (answer, _) = best_call(&ogust, "54.KQ9654.96.432");
        assert_eq!(answer, call(3, Strain::Diamonds), "Ogust survives their X");
        super::set_weak_two_competition(false);
    }

    #[test]
    fn weak_two_overcalled_double_is_values_and_ogust_survives() {
        super::set_weak_two_competition(true);
        // [2♥, (2♠)]: a 12-count doubles for penalty-leaning values.
        let auction = [call(2, Strain::Hearts), call(2, Strain::Spades)];
        let (double, floored) = best_call(&auction, "KJ54.Q5.A964.Q32");
        assert_eq!(double, Call::Double, "values double");
        assert!(!floored, "an authored node, not the floor");
        // A 16-count with a doubleton heart still asks Ogust...
        let (ask, _) = best_call(&auction, "AK54.Q5.A964.K32");
        assert_eq!(ask, call(2, Strain::Notrump), "Ogust survives the overcall");
        // ...and opener's five-rung answer arrives through the targeted rebase.
        let answered = [
            call(2, Strain::Hearts),
            call(2, Strain::Spades),
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (answer, _) = best_call(&answered, "54.KQ9654.96.432");
        assert_eq!(answer, call(3, Strain::Diamonds), "min points, good suit");
        super::set_weak_two_competition(false);
    }

    #[test]
    fn strong_two_contested_stays_strong() {
        super::set_strong_two_competition(true);
        // [2♣, (X)]: systems on — a bust still gives the 2♥ double negative.
        let doubled = [call(2, Strain::Clubs), Call::Double];
        let (negative, floored) = best_call(&doubled, "9542.Q54.964.432");
        assert_eq!(negative, call(2, Strain::Hearts), "systems on over their X");
        assert!(!floored, "the rebase resolves to the authored tree");
        // [2♣, (2♠)]: a positive with good hearts bids them naturally (3♥ —
        // the 2-level is gone); a values hand without a suit doubles; a bust
        // passes and waits.
        let overcalled = [call(2, Strain::Clubs), call(2, Strain::Spades)];
        let (positive, _) = best_call(&overcalled, "54.AQ542.964.Q32");
        assert_eq!(positive, call(3, Strain::Hearts), "natural positive");
        let (waiting, _) = best_call(&overcalled, "954.Q542.964.432");
        assert_eq!(waiting, Call::Pass, "the waiting pass");
        // ...backed by opener's forced reopening: 24 balanced with a spade
        // stopper rebids 2NT rather than selling out.
        let reopen = [
            call(2, Strain::Clubs),
            call(2, Strain::Spades),
            Call::Pass,
            Call::Pass,
        ];
        let (rebid, _) = best_call(&reopen, "AQ2.AKQ5.KQ54.A2");
        assert_eq!(rebid, call(2, Strain::Notrump), "opener never sells out");
        super::set_strong_two_competition(true);
    }

    #[test]
    fn major_support_double_shows_three_spades() {
        super::set_major_support_double(true);
        // [1♥, (P), 1♠, (2♣)]: opener with exactly three spades doubles.
        let auction = [
            call(1, Strain::Hearts),
            Call::Pass,
            call(1, Strain::Spades),
            call(2, Strain::Clubs),
        ];
        let (support, floored) = best_call(&auction, "K32.AQ542.A95.32");
        assert_eq!(support, Call::Double, "exactly three = support double");
        assert!(!floored, "an authored node, not the floor");
        super::set_major_support_double(true);
    }

    #[test]
    fn modern_negative_double_is_exactly_four_over_one_heart() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
        // [1♦, (1♥)]: five spades bid the free 1♠; exactly four double.
        let auction = [call(1, Strain::Diamonds), call(1, Strain::Hearts)];
        let (free, floored) = best_call(&auction, "AQ542.95.964.Q32");
        assert_eq!(free, call(1, Strain::Spades), "five spades bid the suit");
        assert!(!floored, "an authored node, not the floor");
        let (neg, _) = best_call(&auction, "AQ54.95.9642.Q32");
        assert_eq!(neg, Call::Double, "exactly four doubles");
        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn free_bids_fill_the_natural_gaps() {
        super::set_free_bids(true);
        // [1♠, (2♦)]: an 11-count with five hearts bids the 2/1-ish 2♥.
        let auction = [call(1, Strain::Spades), call(2, Strain::Diamonds)];
        let (two_hearts, floored) = best_call(&auction, "K5.AQ542.964.Q32");
        assert_eq!(two_hearts, call(2, Strain::Hearts), "the 2-level free bid");
        assert!(!floored, "an authored node, not the floor");
        // [1♥, (1♠)]: a balanced 10 with a spade stopper bids 1NT.
        let one_nt_auction = [call(1, Strain::Hearts), call(1, Strain::Spades)];
        let (one_nt, _) = best_call(&one_nt_auction, "K52.95.KJ64.QJ32");
        assert_eq!(one_nt, call(1, Strain::Notrump), "the natural 1NT");
        super::set_free_bids(false);
    }

    #[test]
    fn free_bid_floor_gates_the_marginal_hand() {
        super::set_free_bids(true);
        // [1♣, (1♦)]: a 6-ish balanced hand with five hearts. At the default
        // floor of 6 it makes the 1♥ free bid; raise the floor to 8 and it no
        // longer qualifies (falls through to the floor's pass).
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        let hand = "T32.KJ542.94.Q32";
        let (bid_at_6, _) = best_call(&auction, hand);
        assert_eq!(
            bid_at_6,
            call(1, Strain::Hearts),
            "the 1♥ free bid at floor 6"
        );
        super::set_free_bid_floor(8);
        let (bid_at_8, _) = best_call(&auction, hand);
        assert_ne!(
            bid_at_8,
            call(1, Strain::Hearts),
            "floor 8 rejects the 6-count"
        );
        super::set_free_bid_floor(6);
        super::set_free_bids(false);
    }

    #[test]
    fn cachalot_rotates_the_one_level() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        // X = 4+ hearts; 1♥ = 4+ spades; 1♠ = the residual takeout hand.
        let (x, floored) = best_call(&auction, "K52.QJ54.964.Q32");
        assert_eq!(x, Call::Double, "X shows the adjacent major");
        assert!(!floored, "an authored node, not the floor");
        let (transfer, _) = best_call(&auction, "QJ54.K52.964.Q32");
        assert_eq!(transfer, call(1, Strain::Hearts), "1♥ shows spades");
        let (takeout, _) = best_call(&auction, "K52.Q54.964.QJ32");
        assert_eq!(takeout, call(1, Strain::Spades), "1♠ is the takeout hand");
        // Opener completes the heart transfer with exactly three, raises four.
        let complete = [
            call(1, Strain::Clubs),
            call(1, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        let (three, _) = best_call(&complete, "AQ54.K52.96.QJ32");
        assert_eq!(three, call(1, Strain::Hearts), "exactly three completes");
        let (four, _) = best_call(&complete, "AQ5.K542.96.QJ32");
        assert_eq!(four, call(2, Strain::Hearts), "four raises");
        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn cachalot_probe_spades3() {
        use contract_bridge::Strain::*;
        super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
        let h = "A2.KJ54.KQ543.A2"; // opener-ish, 4 spades
        let cases = vec![
            (
                "1D(1H)X (1S)",
                vec![
                    call(1, Diamonds),
                    call(1, Hearts),
                    Call::Double,
                    call(1, Spades),
                ],
            ),
            (
                "1D(1H)X (1NT)",
                vec![
                    call(1, Diamonds),
                    call(1, Hearts),
                    Call::Double,
                    call(1, Notrump),
                ],
            ),
            (
                "1D(1H)X (2C)",
                vec![
                    call(1, Diamonds),
                    call(1, Hearts),
                    Call::Double,
                    call(2, Clubs),
                ],
            ),
            (
                "1C(1H)X (2C)",
                vec![
                    call(1, Clubs),
                    call(1, Hearts),
                    Call::Double,
                    call(2, Clubs),
                ],
            ),
            (
                "nat 1D(1H)1S(2C)",
                vec![
                    call(1, Diamonds),
                    call(1, Hearts),
                    call(1, Spades),
                    call(2, Clubs),
                ],
            ),
            (
                "1C(1D)X (2C) [hearts fam]",
                vec![
                    call(1, Clubs),
                    call(1, Diamonds),
                    Call::Double,
                    call(2, Clubs),
                ],
            ),
            (
                "nat 1C(1D)1H(2C)",
                vec![
                    call(1, Clubs),
                    call(1, Diamonds),
                    call(1, Hearts),
                    call(2, Clubs),
                ],
            ),
        ];
        for (t, a) in cases {
            eprintln!("{t:28}: {:?}", best_call(&a, h));
        }
        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn cachalot_probe_spades2() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
        // spades-family PASS-OUT: does the authored completion even fire over (1♥)?
        let passout = [
            call(1, Strain::Diamonds),
            call(1, Strain::Hearts),
            Call::Double,
            Call::Pass,
        ];
        eprintln!(
            "1D(1H)X P  opener 4sp: {:?}",
            best_call(&passout, "AQ54.K2.KQ543.A2")
        );
        eprintln!(
            "1D(1H)X P  opener 3sp: {:?}",
            best_call(&passout, "AQ5.K42.KQ543.A2")
        );
        // and the (1D) hearts pass-out for contrast:
        let ph = [
            call(1, Strain::Clubs),
            call(1, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        eprintln!(
            "1C(1D)X P  opener 4he: {:?}",
            best_call(&ph, "A2.KQ54.A3.KJ654")
        );
        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn cachalot_probe_spades() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
        // Is 1♦(1♥)X even the spade transfer? And does reveal fire?
        let respond = [call(1, Strain::Diamonds), call(1, Strain::Hearts)];
        eprintln!(
            "responder 4=spades: {:?}",
            best_call(&respond, "KJ54.952.A64.Q32")
        );
        for (a, tag) in [
            (
                vec![
                    call(1, Strain::Diamonds),
                    call(1, Strain::Hearts),
                    Call::Double,
                    call(2, Strain::Clubs),
                ],
                "X 2C",
            ),
            (
                vec![
                    call(1, Strain::Diamonds),
                    call(1, Strain::Hearts),
                    call(1, Strain::Spades),
                    call(2, Strain::Clubs),
                ],
                "1S 2C",
            ),
            (
                vec![
                    call(1, Strain::Clubs),
                    call(1, Strain::Hearts),
                    Call::Double,
                    call(2, Strain::Clubs),
                ],
                "1C:X 2C",
            ),
        ] {
            eprintln!("{tag}: {:?}", best_call(&a, "A2.KJ54.KQ543.A2"));
        }
        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn cachalot_x_contested_answer_raises_the_shown_major() {
        // Under competition the pass-out completion doesn't fire; opener's
        // authored contested answer raises the major the X showed at the level
        // the intervention forces — a fit the floor would otherwise leave for a
        // bare double. Hearts over (1♦), spades over (1♥).
        super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
        // [1♣, (1♦), X(=4+♥), (2♦)]: opener with four hearts jumps to 3♥.
        let x_hearts = [
            call(1, Strain::Clubs),
            call(1, Strain::Diamonds),
            Call::Double,
            call(2, Strain::Diamonds),
        ];
        let (raise, _) = best_call(&x_hearts, "A2.KQ42.A3.KJ654");
        assert_eq!(raise, call(3, Strain::Hearts), "four-card support jumps");
        // Three-card support makes the simple raise to 2♥ (2♥ is above 2♦).
        let (simple, _) = best_call(&x_hearts, "A32.KQ4.A63.KJ54");
        assert_eq!(simple, call(2, Strain::Hearts), "three-card simple raise");
        // No support ⇒ opener passes to defend, not a phantom bid.
        let (defend, _) = best_call(&x_hearts, "AQ32.J.KQ632.A54");
        assert_eq!(defend, Call::Pass, "no fit defends");

        // [1♦, (1♥), X(=4+♠), (2♣)]: over (1♥) the X shows spades — opener raises.
        let x_spades = [
            call(1, Strain::Diamonds),
            call(1, Strain::Hearts),
            Call::Double,
            call(2, Strain::Clubs),
        ];
        let (raise, _) = best_call(&x_spades, "KQ54.A2.KJ543.A2");
        assert_eq!(raise, call(3, Strain::Spades), "over (1♥) four spades jump");

        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn sputnik_negative_double_is_the_residual() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Sputnik);
        // [1♣, (1♦)]: a 4-card major is bid naturally at the 1-level...
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        // 7 HCP: a flat 6-count reads 5 on the rule-of-N+8 scale and passes.
        let (spades, floored) = best_call(&auction, "KJ54.952.964.QJ2");
        assert_eq!(spades, call(1, Strain::Spades), "four spades bid the suit");
        assert!(!floored, "an authored node, not the floor");
        // ...while X denies a biddable major — the residual, ≤3 in each.
        let (neg, _) = best_call(&auction, "K52.Q54.J964.Q32");
        assert_eq!(
            neg,
            Call::Double,
            "≤3 in both majors is the residual double"
        );
        super::set_negative_double_shape(super::NegativeDoubleShape::BothMajors);
    }

    #[test]
    fn cachalot_natural_free_bids_get_the_forcing_answers() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Cachalot);
        // A natural 2-level free bid reaches Section 4d's forcing answers:
        // opener raises partner's freely bid diamonds with three.
        let answer = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        let (raise, floored) = best_call(&answer, "A5.K52.Q64.KJ632");
        assert_eq!(raise, call(3, Strain::Diamonds), "the free bid is raised");
        assert!(!floored, "an authored node, not the floor");
        // The rotated 1-level call stays with its Section-9 completion —
        // 1♥ over (1♦) shows spades; exactly three completes 1♠.
        let complete = [
            call(1, Strain::Clubs),
            call(1, Strain::Diamonds),
            call(1, Strain::Hearts),
            Call::Pass,
        ];
        let (three, _) = best_call(&complete, "AQ5.K52.964.QJ32");
        assert_eq!(
            three,
            call(1, Strain::Spades),
            "the rotation completes, not answer_free_bid"
        );
        super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
    }

    #[test]
    fn sputnik_free_major_raise_needs_four() {
        super::set_negative_double_shape(super::NegativeDoubleShape::Sputnik);
        // Sputnik's natural 1-level major promises only four, so opener's
        // two-level raise demands four trumps — three would be a Moysian.
        let answer = [
            call(1, Strain::Clubs),
            call(1, Strain::Diamonds),
            call(1, Strain::Hearts),
            Call::Pass,
        ];
        let (three, floored) = best_call(&answer, "K52.Q52.K94.AJ63");
        assert_eq!(
            three,
            call(1, Strain::Notrump),
            "three trumps bid 1NT, not the Moysian raise"
        );
        assert!(!floored, "an authored node, not the floor");
        let (four, _) = best_call(&answer, "K5.Q542.K94.AJ63");
        assert_eq!(four, call(2, Strain::Hearts), "four trumps raise");
        super::set_negative_double_shape(super::NegativeDoubleShape::Modern);
    }

    #[test]
    fn negative_free_bid_is_weak_and_capped() {
        super::set_free_bid_style(super::FreeBidStyle::Negative);
        let auction = [call(1, Strain::Clubs), call(1, Strain::Spades)];
        // 8 points with a six-card suit: the classic NFB.
        let (weak, floored) = best_call(&auction, "52.Q4.KJ8642.T53");
        assert_eq!(weak, call(2, Strain::Diamonds), "the negative free bid");
        assert!(!floored, "an authored node, not the floor");
        // The same suit with game values starts with the widened double.
        let (strong, _) = best_call(&auction, "52.A4.AKJ642.Q53");
        assert_eq!(strong, Call::Double, "12+ doubles first");
        // Opener drops the capped free bid with a minimum (the Pass answer).
        let answer = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        let (drop, drop_floored) = best_call(&answer, "A5.Q52.Q64.KJ632");
        assert_eq!(drop, Call::Pass, "the NFB is non-forcing");
        assert!(!drop_floored, "an authored node, not the floor");
        super::set_free_bid_style(super::FreeBidStyle::Forcing);
    }

    #[test]
    fn negative_double_then_suit_is_game_forcing() {
        super::set_free_bid_style(super::FreeBidStyle::Negative);
        // The doubler clarifies with the concealed long suit — forcing to
        // game — and opener answers it with the forcing-answer table.
        let rebid = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            Call::Double,
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
        ];
        let (fg, floored) = best_call(&rebid, "52.A4.AKJ642.Q53");
        assert_eq!(fg, call(2, Strain::Diamonds), "X then the suit is FG");
        assert!(!floored, "an authored node, not the floor");
        let answer = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            Call::Double,
            Call::Pass,
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        let (raise, raise_floored) = best_call(&answer, "A5.K52.Q64.KJ632");
        assert_eq!(
            raise,
            call(3, Strain::Diamonds),
            "opener answers the FG suit"
        );
        assert!(!raise_floored, "an authored node, not the floor");
        super::set_free_bid_style(super::FreeBidStyle::Forcing);
    }

    #[test]
    fn free_bid_transfers_swap_the_two_level() {
        super::set_free_bid_style(super::FreeBidStyle::Transfer);
        // [1♣, (1♠)]: both red suits sit at the two level, so the slots swap
        // — 2♦ shows hearts, 2♥ shows diamonds (the wrap).
        let auction = [call(1, Strain::Clubs), call(1, Strain::Spades)];
        let (hearts, floored) = best_call(&auction, "52.KJ864.Q42.T53");
        assert_eq!(hearts, call(2, Strain::Diamonds), "2♦ transfers to hearts");
        assert!(!floored, "an authored node, not the floor");
        let (diamonds, _) = best_call(&auction, "52.Q42.KJ864.T53");
        assert_eq!(diamonds, call(2, Strain::Hearts), "2♥ wraps to diamonds");
        // Opener completes (2♥ on the true transfer, 3♦ on the wrap)…
        let complete = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        let (comp, comp_floored) = best_call(&complete, "A53.Q52.64.AJ632");
        assert_eq!(
            comp,
            call(2, Strain::Hearts),
            "opener completes and declares"
        );
        assert!(!comp_floored, "an authored node, not the floor");
        let wrap = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            call(2, Strain::Hearts),
            Call::Pass,
        ];
        let (wrap_comp, _) = best_call(&wrap, "A53.Q52.64.AJ632");
        assert_eq!(
            wrap_comp,
            call(3, Strain::Diamonds),
            "the wrap completes a level higher"
        );
        // …and the weak transferor passes the completion out.
        let clarify = [
            call(1, Strain::Clubs),
            call(1, Strain::Spades),
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Hearts),
            Call::Pass,
        ];
        let (weak, weak_floored) = best_call(&clarify, "52.KJ864.Q42.T53");
        assert_eq!(weak, Call::Pass, "the weak hand passes the completion");
        assert!(!weak_floored, "an authored node, not the floor");
        // A lone two-level slot stays natural and forcing: over (1♥) only
        // diamonds sit at the two level.
        let lone = [call(1, Strain::Clubs), call(1, Strain::Hearts)];
        let (natural, _) = best_call(&lone, "K52.4.AQJ86.T532");
        assert_eq!(natural, call(2, Strain::Diamonds), "a lone slot is natural");
        super::set_free_bid_style(super::FreeBidStyle::Forcing);
    }

    #[test]
    fn high_overcalls_get_a_structure() {
        super::set_high_overcall_responses(true);
        // [1♠, (3♦)]: 4 hearts + 12 HCP make the 3-level negative double; a
        // diamond stopper + 16 bids 3NT instead.
        let auction = [call(1, Strain::Spades), call(3, Strain::Diamonds)];
        let (neg, floored) = best_call(&auction, "K5.KQ54.965.A432");
        assert_eq!(neg, Call::Double, "the 3-level negative double");
        assert!(!floored, "an authored node, not the floor");
        let (game, _) = best_call(&auction, "K5.KQ54.A65.A432");
        assert_eq!(game, call(3, Strain::Notrump), "3NT with a stopper");
        // Opener answers the forcing double with the unbid major.
        let answer = [
            call(1, Strain::Spades),
            call(3, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        let (major, _) = best_call(&answer, "AQ542.KJ54.96.32");
        assert_eq!(major, call(3, Strain::Hearts), "four hearts answer 3♥");
        super::set_high_overcall_responses(false);
    }

    #[test]
    fn jordan_truscott_over_their_double() {
        super::set_jordan_truscott(true);
        let auction = [call(1, Strain::Spades), Call::Double];
        // Jordan 2NT: 4 trumps, limit+.
        let (jordan, floored) = best_call(&auction, "Q542.A5.K964.Q32");
        assert_eq!(jordan, call(2, Strain::Notrump), "Jordan/Truscott");
        assert!(!floored, "an authored node, not the floor");
        // Value redouble: 10+ without the fit.
        let (xx, _) = best_call(&auction, "K2.A54.K964.Q532");
        assert_eq!(xx, Call::Redouble, "the value redouble");
        // The jump raise flips preemptive.
        let (preempt, _) = best_call(&auction, "Q542.9.96432.Q32");
        assert_eq!(preempt, call(3, Strain::Spades), "preemptive jump raise");
        // A weak 2-level new suit is non-forcing — opener passes a minimum.
        let weak = [
            call(1, Strain::Spades),
            Call::Double,
            call(2, Strain::Clubs),
            Call::Pass,
        ];
        let (pass, weak_floored) = best_call(&weak, "AQ542.K54.96.432");
        assert_eq!(pass, Call::Pass, "the weak new suit is dropped");
        assert!(!weak_floored, "an authored node, not the floor");
        // Opener answers Jordan with the cue-raise ladder (not Jacoby 2NT,
        // which the systems-on rebase would have reached).
        let answer = [
            call(1, Strain::Spades),
            Call::Double,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let (accept, _) = best_call(&answer, "AKQ54.K54.96.A32");
        assert_eq!(accept, call(4, Strain::Spades), "a maximum accepts");
        let (decline, _) = best_call(&answer, "AQ542.954.96.A32");
        assert_eq!(decline, call(3, Strain::Spades), "a minimum declines");
        super::set_jordan_truscott(true);
    }

    #[test]
    fn redouble_answer_shadows_the_rebase_blast() {
        // [1♠ (X) XX (P)]: opener's rebid.  The systems-on rebase strips the
        // double and the redouble, so opener replays uncontested with
        // responder's shown 10+ unseen, and the floor re-prices this shaped
        // minimum (12 HCP, 15 points) as game-going — the remnant report's
        // worst per-board family (−16..−17 IMPs/board vulnerable).  The
        // authored answer passes — even with a long suit (one-of-a-suit
        // redoubled makes with overtricks; a 2M escape rung measured
        // −11 IMPs/fired and was deleted) — and shadows the floor.
        let auction = [
            call(1, Strain::Spades),
            Call::Double,
            Call::Redouble,
            Call::Pass,
        ];
        let opener = "KQ652..AKT764.85"; // 12 HCP 5=0=6=2, opened 1♠
        let (default_call, default_floored) = best_call(&auction, opener);
        assert_eq!(default_call, Call::Pass, "the authored answer passes");
        assert!(!default_floored, "the node shadows the floor");
        let (long, long_floored) = best_call(&auction, "KQJT65.2.KJ85.T4"); // 10 HCP, 6 spades
        assert_eq!(
            long,
            Call::Pass,
            "a long-suit minimum sits for the redoubled make"
        );
        assert!(!long_floored, "the sit is authored too");

        super::set_redouble_answer(false);
        let (off_call, _) = best_call(&auction, opener);
        super::set_redouble_answer(true);
        assert_ne!(
            off_call,
            Call::Pass,
            "the off arm: the rebase + floor bids on blindly"
        );
    }

    /// Renderability invariant: every guarded fallback in the competitive book
    /// describes itself — the guard names its condition and a rebase names its
    /// rewrite — so `render-book` and the web book show the whole book.  A new
    /// bare `guard(closure)` fails here; wrap it in `described_guard`.
    #[test]
    fn competitive_fallbacks_are_renderable() {
        use crate::bidding::fallback::Fallback;

        let book = super::competition();
        let all = book.0.fallbacks();
        assert!(
            all.len() > 30,
            "the competitive book has {} guarded entries — the walk is broken",
            all.len()
        );

        for (auction, guard, fallback) in &all {
            let key = contract_bridge::auction::display_calls(auction).to_string();
            assert!(
                guard.describe().is_some(),
                "unlabeled guard at [{key}] — wrap it in described_guard"
            );
            if let Fallback::Rebase(rewrite) = fallback {
                assert!(
                    rewrite.describe().is_some(),
                    "opaque rebase at [{key}] — wrap it in described_rewrite"
                );
            }
        }

        // One concrete probe: the [1♠] direct-seat package renders with its
        // overcall ceiling and carries the negative double.
        let (_, guard, fallback) = all
            .iter()
            .find(|(auction, ..)| auction.as_ref() == [Call::Bid(Bid::new(1, Strain::Spades))])
            .expect("a guarded entry at [1♠]");
        assert_eq!(guard.describe().as_deref(), Some("(overcall ≤2♠)"));
        let Fallback::Classify(classifier) = fallback else {
            panic!("the direct-seat package is a classifier");
        };
        let rules = classifier.as_rules().expect("an authored Rules table");
        assert!(
            rules.rules().iter().any(|rule| rule.call() == Call::Double),
            "the negative double renders"
        );
    }

    // --- Free 1NT floor + the natural 2NT jump over a 1-level overcall ---

    /// `1♣ (1♦) 1NT`: a balanced 7-count with a diamond stopper takes the free
    /// 1NT at the default floor of 6.
    #[test]
    fn free_1nt_fires_at_default_floor() {
        super::set_free_1nt_floor(6);
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        let (c, floored) = best_call(&auction, "Q54.J54.KJ32.543");
        assert_eq!(c, call(1, Strain::Notrump));
        assert!(!floored, "the free 1NT is a book node");
    }

    /// Raising the isolated 1NT floor to 8 drops the 7-count from 1NT — and,
    /// being decoupled, leaves the forcing 1-level suit bids untouched.
    #[test]
    fn free_1nt_dropped_above_raised_floor() {
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        super::set_free_1nt_floor(8);
        let (c, _) = best_call(&auction, "Q54.J54.KJ32.543");
        super::set_free_1nt_floor(6);
        assert_ne!(
            c,
            call(1, Strain::Notrump),
            "7 HCP is below the raised floor"
        );
    }

    /// `1♣ (1♦) 2NT`: a balanced 12-count with a diamond stopper — too strong
    /// for the capped 1NT, no fit to cue — invites at 2NT (default-on).
    #[test]
    fn free_2nt_jump_fires_by_default() {
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        let (c, floored) = best_call(&auction, "K54.K54.KJ32.Q54");
        assert_eq!(c, call(2, Strain::Notrump));
        assert!(!floored, "the 2NT jump is a book node");
    }

    /// The ladder boundary: a balanced 10-count with a stopper is still 1NT,
    /// not the 2NT jump (which starts at 11).
    #[test]
    fn free_1nt_caps_below_the_jump() {
        let auction = [call(1, Strain::Clubs), call(1, Strain::Diamonds)];
        let (c, _) = best_call(&auction, "K54.Q54.KJ32.J54");
        assert_eq!(c, call(1, Strain::Notrump), "10 HCP caps at 1NT");
    }
}
