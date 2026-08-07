//! Direct overcalls and takeout doubles over their opening
//!
//! The base defensive table: natural suit overcalls, the `1NT` overcall, and
//! the takeout double.  [`DoubleShape`] and [`TakeoutSupport`] tune what the
//! double promises; the point bands and disciplines are knobs
//! ([`set_natural_overcall_points`], [`set_overcall_discipline`],
//! [`set_passed_hand_overcall`], [`set_strong_double_hcp`]).

use super::michaels::{michaels_advances, two_suiter_hcp_floor, unusual_nt_advances};
use super::*;

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
pub(super) fn natural_double_shape() -> DoubleShape {
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
pub(super) fn takeout_support() -> TakeoutSupport {
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
pub fn passed_hand_overcall() -> bool {
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

pub(super) fn natural_double_weight() -> i16 {
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

/// Over each one-of-a-suit opening: our direct defense, and the advances of
/// partner's Michaels cue and Unusual `2NT`
///
/// All three key sets are disjoint from every other write in
/// [`defensive`] — `(1t)`, `(1t) 2t -` and `(1t) 2NT -` — so they lift out
/// of the per-suit loop without changing what lands where.
pub(super) fn suit_defense_package() -> Package {
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
                    Pattern::node(&format!("{key} 2{theirs} -")),
                    michaels_advances(suit),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("{key} 2NT -")),
                    unusual_nt_advances(suit),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
