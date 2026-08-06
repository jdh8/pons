//! Uncontested openings for every seat

use crate::bidding::constraint::{
    Cons, Constraint, balanced, cccc, described, fifths, hcp, len, length_box, long_suit_box, nltc,
    nth_seat, points, shapes,
};
use crate::bidding::context::Context;
use crate::bidding::inference::Range;
use crate::bidding::rows::{Package, Pattern, compile_into, rows_of};
use crate::bidding::{Alert, Rules, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Hand, Strain, Suit};
use std::cell::Cell;

/// The strong, artificial `2♣` opening (22+) — the only artificial opening
const STRONG_2C: Alert = Alert("strong-2c");

thread_local! {
    /// Whether our side opens a strong balanced 15-17 with 1NT.  Default `true`.
    static OPEN_ONE_NOTRUMP: Cell<bool> = const { Cell::new(true) };
    /// Restore the fifths gauge (`fifths(14.5..17.5)`, centre-matched to plain HCP
    /// 15-17) for the 1NT opening.  Default `false` — the opening gauges plain HCP.
    static ONE_NOTRUMP_FIFTHS: Cell<bool> = const { Cell::new(false) };
    /// Which shape policy the 1NT opening admits when `american()` rebuilds.
    /// Default [`NotrumpShape::Wide6322`] (the shipped default).
    static NOTRUMP_SHAPE: Cell<NotrumpShape> = const { Cell::new(NotrumpShape::Wide6322) };
    /// The weak-two opening's strength band, gauged in raw HCP when `Some`.
    /// Default `None`: byte-identical `points(5..=10)`.  See [`set_weak_two_hcp`].
    static WEAK_TWO_HCP: Cell<Option<(u8, u8)>> = const { Cell::new(None) };
    /// The weak-two opening's honor-location evaluator gauge when `Some`.
    /// Default `None`: byte-identical.  Wins over [`WEAK_TWO_HCP`] if both are
    /// armed.  See [`set_weak_two_eval`].
    static WEAK_TWO_EVAL: Cell<Option<WeakTwoEval>> = const { Cell::new(None) };
    /// Whether the strong 2NT (20-21) opening admits the wide-minor shape
    /// instead of plain `balanced()`.  Default `false` (byte-identical).  See
    /// [`set_two_notrump_wide`].
    static TWO_NOTRUMP_WIDE: Cell<bool> = const { Cell::new(false) };
    /// Whether the 1NT opening admits human-style off-shape hands. Default
    /// `false` (byte-identical). See [`set_one_notrump_offshape`].
    static ONE_NOTRUMP_OFFSHAPE: Cell<bool> = const { Cell::new(false) };
    /// Whether weak twos admit five-card suits and a wider strength band.
    /// Default `false` (byte-identical). See [`set_weak_two_wild`].
    static WEAK_TWO_WILD: Cell<bool> = const { Cell::new(false) };
}

/// Suppress (`false`) or restore (`true`, the default) our own 1NT opening.
///
/// With it off, a strong balanced 15-17 opens a minor instead of 1NT, so a
/// diagnostic can isolate our *defense* to an opponent's 1NT without our own 1NT
/// openings polluting the duplicate (see `bba-match --no-our-1nt`).
pub fn set_open_one_notrump(on: bool) {
    OPEN_ONE_NOTRUMP.with(|cell| cell.set(on));
}

/// Restore the legacy fifths strength gauge for the 1NT opening (`true`); the
/// default (`false`) gauges plain HCP 15-17, which opens 1NT a touch more often.
pub fn set_one_notrump_fifths(on: bool) {
    ONE_NOTRUMP_FIFTHS.with(|cell| cell.set(on));
}

/// Select the 1NT opening [`NotrumpShape`] for the next rebuild of
/// [`american()`][crate::american()] — the web Settings shape radio.  Default
/// [`NotrumpShape::Wide6322`].  Read by
/// [`american_book`][crate::bidding::american::american_book], so every
/// constructor built on it picks up the setting.
pub fn set_notrump_shape(shape: NotrumpShape) {
    NOTRUMP_SHAPE.with(|cell| cell.set(shape));
}

/// The 1NT opening shape currently selected by [`set_notrump_shape`].
pub(crate) fn notrump_shape_setting() -> NotrumpShape {
    NOTRUMP_SHAPE.with(Cell::get)
}

/// Gauge the weak-two openings in raw HCP over `lo..=hi` instead of the default
/// rule-of-N+8 `points(5..=10)` (opt-in; the default is byte-identical).
///
/// The opening is *fit-unknown*, so a preempt's length is already pinned by the
/// six-card requirement and gauging its *strength* in shape-crediting `points`
/// double-counts that length: a six-card suit reads `+max(0, L2−8)`, i.e. +0 on
/// 6-2-2-3 up to +2 on 6-4-2-1, so no single `points` shift restores a clean
/// cutoff — the shapely hands slip in one-to-two HCP light while the top edge
/// blurs.  Raw HCP is the disciplined, disclosable gauge: partner can trust the
/// count for games, sacrifices, and leads.
///
/// Only the fit-unknown *opening* moves.  The Ogust min/max answers stay on
/// `points`, deliberately: responder's 2NT promises support, so those are
/// *fit-known* and re-credit shape (the split mirrors the 2/1 gate's
/// hcp/support-points fit-split).
///
/// **Rejected default-on** (opt-in only): fix-vs-shipped `hcp(5..=10)` measured a
/// wash on the honest sd-lead scorer (−0.0045 NV / −0.0018 vul, CIs span 0) — a
/// weak two is a preempt, and the plain-DD "remnant" the point-count campaign
/// priced on this family is the obstruction/disclosure wall, not a fixable gauge
/// (the marginal weak twos over-disclose to the opponents' blind leads).  A
/// major-only carve measured strictly worse (sd-vul −0.0113).  Retained as a
/// single-dummy re-measure candidate (docs/point-count-threshold-campaign.md).
pub fn set_weak_two_hcp(band: Option<(u8, u8)>) {
    WEAK_TWO_HCP.with(|cell| cell.set(band));
}

/// An honor-location evaluator gauge for the weak-two openings
/// ([`set_weak_two_eval`])
///
/// Both evaluators reward honors sitting *in the long suit* — the weak twos
/// whose offense is real and whose disclosure to the opponents' blind leads
/// costs least (the sd-lead bias that rejected the raw-HCP re-gauge).  The
/// `*Band` forms replace the strength leg outright (evaluator-as-gauge); the
/// `CcccFloor`/`NltcCeil` forms AND onto the shipped `points(5..=10)` band
/// (evaluator-as-discipline — a strict subset, so the opening's `points
/// 5..=10` inference reading stays exactly sound).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WeakTwoEval {
    /// Kaplan–Rubens CCCC in `lo..hi` replaces `points(5..=10)`.
    CcccBand(f64, f64),
    /// `points(5..=10)` plus a CCCC floor pruning scattered-honor hands.
    CcccFloor(f64),
    /// NLTC in `lo..=hi` replaces `points(5..=10)`; *fewer* losers = stronger,
    /// so `lo` is the strong edge and `hi` the junk edge.
    NltcBand(f64, f64),
    /// `points(5..=10)` plus an NLTC ceiling: at most this many losers.
    NltcCeil(f64),
}

/// Gauge the weak-two openings with an honor-location evaluator
/// ([`WeakTwoEval`]) instead of the default rule-of-N+8 `points(5..=10)`
/// (opt-in; the default `None` is byte-identical; wins over
/// [`set_weak_two_hcp`] if both are armed).
///
/// The raw-HCP re-gauge ([`set_weak_two_hcp`]) was rejected on the sd-lead
/// scorer: the marginal weak twos it admits over-disclose to the opponents'
/// blind leads.  CCCC and NLTC test the follow-up hypothesis that *where the
/// honors sit* — concentrated in the six-card suit versus scattered through
/// the short suits — separates the weak twos worth their disclosure from the
/// rest.  Like the HCP knob, only the fit-unknown opening moves; the Ogust
/// min/max answers stay on fit-known `points`.
pub fn set_weak_two_eval(gauge: Option<WeakTwoEval>) {
    WEAK_TWO_EVAL.with(|cell| cell.set(gauge));
}

/// Open the strong 2NT (20-21) on the wide-minor shape `{M 2..=4, m 2..=6}`
/// instead of plain `balanced()` (opt-in; the default `false` is byte-identical)
///
/// This is DNF-ledger chop G0 (docs/dnf-migration.md): the 1NT `Wide6322`
/// treatment carried up to the 20-21 opening.  It **drops the 5M(332)** balanced
/// hands (a 5-card major now opens one-of-a-major and jump-rebuilds) and **adds
/// the wide minors** (5m422/6m322), mirroring [`NotrumpShape::Wide6322`] minus
/// its two major pan-handles.  The reading in
/// [`apply_opening`][crate::bidding::inference] widens opener's minors to six
/// under the same knob.
pub fn set_two_notrump_wide(on: bool) {
    TWO_NOTRUMP_WIDE.with(|cell| cell.set(on));
}

/// Admit 5422 and mild singleton-honour shapes to the 15–17 1NT opening
/// (opt-in; the default `false` is byte-identical).
pub fn set_one_notrump_offshape(on: bool) {
    ONE_NOTRUMP_OFFSHAPE.with(|cell| cell.set(on));
}

fn one_notrump_offshape() -> bool {
    ONE_NOTRUMP_OFFSHAPE.with(Cell::get)
}

/// Admit five-card suits and `points(3..=12)` to weak-two openings (opt-in;
/// the default `false` is byte-identical).
pub fn set_weak_two_wild(on: bool) {
    WEAK_TWO_WILD.with(|cell| cell.set(on));
}

fn weak_two_wild() -> bool {
    WEAK_TWO_WILD.with(Cell::get)
}

/// Whether the strong 2NT opening admits the wide-minor shape (chop G0).  Read
/// by both the opening table and the inference reading so they stay in step.
pub(crate) fn two_notrump_wide() -> bool {
    TWO_NOTRUMP_WIDE.with(Cell::get)
}

/// Which hand shapes the strong 1NT opening admits ([`openings_with`])
///
/// Every variant opens the balanced patterns (4333/4432/5332).  A long *major*
/// always prefers a one-of-a-major opening it can rebid, so the shapely
/// additions are minor-only.  Strength ([`fifths`] 15–17) and the inference side
/// are untouched; this is the shape-only knob for the deferred redesign (see the
/// `nt-shape-abc` and `nt-shape-contested` examples).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotrumpShape {
    /// Balanced only — the classic baseline.
    Balanced,
    /// Balanced plus a 5422 with a five-card minor — the pre-6322 baseline.
    Wide,
    /// [`Wide`][NotrumpShape::Wide] plus a 6322 with a six-card minor — the
    /// shipped default (adopted after a two-seed A/B win vs the reference
    /// opponent, +0.004…0.006 IMPs/board plain, sd-confirmed).
    Wide6322,
}

/// Shapes eligible for a 1NT opening, per the [`NotrumpShape`] policy
///
/// Authored as an exact union of length boxes ([`shapes`]).  The wide
/// variants are a bigger cube plus two pan-handles: majors capped at four and
/// minors at five (`Wide`) or six (`Wide6322`) — the 13-card sum excludes
/// 5-5, 6-4, 7222, and singletons from the cube, so it is exactly the
/// balanced-with-≤4-card-majors patterns plus 5m(422) (and 6m(322) for
/// `Wide6322`) — plus the two 5M(332) boxes restoring the balanced five-card
/// majors.  `Balanced` is the plain 4333/4432/5332 union.  Eval-equivalence
/// with the closure this replaces is pinned exhaustively by
/// `notrump_shape_boxes_match_closure`.
pub(crate) fn notrump_shape(shape: NotrumpShape) -> Cons<impl Constraint + Clone> {
    let major_cube = Range::new(2, 4);
    let (label, minor_cube) = match shape {
        NotrumpShape::Balanced => ("balanced 1NT shape", Range::new(2, 4)),
        NotrumpShape::Wide => ("balanced or 5m(422) 1NT shape", Range::new(2, 5)),
        NotrumpShape::Wide6322 => ("balanced or 5m(422)/6m(322) 1NT shape", Range::new(2, 6)),
    };
    let mut boxes = vec![length_box([minor_cube, minor_cube, major_cube, major_cube])];
    for suit in Suit::ASC {
        // The pan-handles: 5(332) for every suit under `Balanced` (the plain
        // balanced union), majors only for the wide cubes (whose minor range
        // already covers the five-card-minor patterns).
        if shape == NotrumpShape::Balanced || matches!(suit, Suit::Hearts | Suit::Spades) {
            boxes.push(long_suit_box(suit, Range::new(5, 5), Range::new(2, 3)));
        }
    }
    shapes(label, boxes)
}

/// The wide-minor 2NT shape (chop G0): the `Wide6322` cube `{M 2..=4, m 2..=6}`
/// with no pan-handles — balanced/semi-balanced with the longest suit a minor.
///
/// The 13-card sum does the excluding: majors capped at four drop every 5-card
/// major (so 5M(332) opens one-of-a-major instead), and minors run to six for
/// the 5m(422)/6m(322) hands.  `{4432, 4333, 5m332, 5m422, 6m322}`.
pub(crate) fn two_notrump_wide_shape() -> Cons<impl Constraint + Clone> {
    shapes(
        "balanced or wide-minor 2NT shape",
        vec![length_box([
            Range::new(2, 6), // clubs
            Range::new(2, 6), // diamonds
            Range::new(2, 4), // hearts
            Range::new(2, 4), // spades
        ])],
    )
}

/// Human-style off-shape 1NT hands: any 5422, or 4441/5431 with a singleton Q/J.
fn one_notrump_offshape_gate() -> Cons<impl Constraint + Clone> {
    let mut boxes = Vec::new();
    for suit in Suit::ASC {
        boxes.push(long_suit_box(suit, Range::new(5, 5), Range::new(2, 2)));
    }
    let shape = shapes("5422 1NT shape", boxes);
    shape
        | described(
            "4441/5431 with a singleton queen or jack",
            |hand: Hand, _: &Context<'_>| {
                let mut lengths = Suit::ASC.map(|suit| hand[suit].len());
                lengths.sort_unstable();
                matches!(lengths, [1, 3, 4, 5] | [1, 4, 4, 4])
                    && Suit::ASC.into_iter().any(|suit| {
                        hand[suit].len() == 1
                            && (hand[suit].contains(contract_bridge::Rank::Q)
                                || hand[suit].contains(contract_bridge::Rank::J))
                    })
            },
        )
}

/// Better-minor selector: open 1♦ rather than 1♣
///
/// Open the longer minor; with equal length open 1♦ on four-or-more (the
/// standard 4-4 → 1♦, 3-3 → 1♣ split).
fn prefers_diamonds() -> Cons<impl Constraint + Clone> {
    described("prefers diamonds", |hand: Hand, _: &Context<'_>| {
        let clubs = hand[Suit::Clubs].len();
        let diamonds = hand[Suit::Diamonds].len();
        diamonds > clubs || (diamonds == clubs && diamonds >= 4)
    })
}

/// The opening table, shared by every seat
///
/// Strong notrumps (15–17 / 20–21), the artificial 2♣ (22+), five-card majors,
/// better-minor one-of-a-minor openings, weak twos, and three-level preempts.
/// A lighter five-card major is allowed in third and fourth seat.  The 1NT also
/// opens a 5422 or 6322 with a long minor (`wide6322`, the shipped default; see
/// [`openings_with`]).
///
/// Sharp on shape, fuzzy on strength: suit openings gauge upgraded
/// [`points`], notrump ranges gauge [`fifths`].  A clean shapely maximum
/// upgrades out of a weak two — it is too good for one.
#[must_use]
pub fn openings() -> Rules {
    openings_with(NotrumpShape::Wide6322)
}

/// [`openings`] with the 1NT [`NotrumpShape`] policy selectable
///
/// `openings()` ships [`NotrumpShape::Wide`] (a 5422 with a five-card minor also
/// opens 1NT); [`NotrumpShape::Balanced`] is the classic baseline and
/// [`NotrumpShape::Wide6322`] the experimental superset.
#[must_use]
pub fn openings_with(shape: NotrumpShape) -> Rules {
    let mut rules = Rules::new()
        // Strong, artificial 2♣ — top priority.  The `hcp` leg is exact cover
        // for the plain rule-of-N+8 opt-in scale's flat hole: a 4-3-3-3
        // 22-count reads 21 points there and would otherwise demote a game
        // force to a passable 1♣ (the shipped floored scale reads it 22, and
        // unbalanced 22-HCP hands read 22+ points on every scale, so the
        // union adds nothing else — it's redundant-but-exact by default).
        .rule(Bid::new(2, Strain::Clubs), 300, points(22..) | hcp(22..))
        .alert(STRONG_2C);
    // Strong 1NT — gated so a diagnostic can suppress our own 1NT opening
    // (`set_open_one_notrump`); the 15-17 balanced hands then open a minor.
    if OPEN_ONE_NOTRUMP.with(Cell::get) {
        // Strength gauged by plain HCP 15-17 by default; `set_one_notrump_fifths`
        // restores the legacy Andrews' fifths gauge.  Each arm reissues `.rule()`
        // so the differing constraint types unify to `Rules`.
        rules = if (ONE_NOTRUMP_FIFTHS.with(Cell::get), one_notrump_offshape()) == (true, true) {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                fifths(14.5..17.5) & (notrump_shape(shape) | one_notrump_offshape_gate()),
            )
        } else if ONE_NOTRUMP_FIFTHS.with(Cell::get) {
            // 14.5..17.5 (centre 16), not 15..18 (centre 16.5): fifths sums to 40
            // over the deck like HCP, so an unbiased "15-17 HCP" gate shares the
            // plain-HCP band's centre — the old 15..18 was half a point too high.
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                fifths(14.5..17.5) & notrump_shape(shape),
            )
        } else if one_notrump_offshape() {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                hcp(15..=17) & (notrump_shape(shape) | one_notrump_offshape_gate()),
            )
        } else {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                200,
                hcp(15..=17) & notrump_shape(shape),
            )
        };
    }
    // Strong 2NT.  `balanced()` by default; the G0 opt-in
    // (`set_two_notrump_wide`) swaps in the wide-minor shape.  Each arm reissues
    // `.rule()` so the differing constraint types unify to `Rules`.
    rules = if TWO_NOTRUMP_WIDE.with(Cell::get) {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            200,
            fifths(20.0..22.0) & two_notrump_wide_shape(),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            200,
            fifths(20.0..22.0) & balanced(),
        )
    };
    // One-level suit openings.  Every band carries an explicit `hcp` floor.
    // On the default PointCount scale the shape [`upgrade`] caps at 2, so
    // `points(N..)` already implies `hcp(N−2..)` and the floor is redundant —
    // but it is retained as the belt for the rule-of-N+8 opt-out, where a
    // count is `hcp + max(0, L1+L2 − 8)`, `L1+L2` reaches 13 on a 7-6-0-0, and
    // `points(N..)` bottoms out at `N − 5` HCP (`points(12..)` would admit a
    // 7-count).  WBF Systems Policy 2024 §2.3.1(c) makes a system a HUM if a
    // one-level opening in first or second seat "may be made with 7 high card
    // points or less", so an envelope that *touches* 0..=7 is caught however
    // rare the hand.  EBU Blue Book §7A3/§8A4 impose the same absolute 8 in
    // every seat, third and fourth included, where the WBF does not reach.
    //
    // First/second seat takes `hcp(10..)` rather than the bare legal minimum —
    // standard practice, since partner must be able to aim at 3NT.  Third and
    // fourth seat open lighter on `points(11..)`, exactly the Rule of 19 (=
    // ACBL "Average Strength", which those charts require in all four seats),
    // floored at the legal 8.
    //
    // Five-card majors; 1♠ ranks just above 1♥ so 5-5 opens the higher.
    rules = rules
        .rule(
            Bid::new(1, Strain::Spades),
            160,
            points(12..=21) & hcp(10..) & len(Suit::Spades, 5..) & (nth_seat(1) | nth_seat(2)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            150,
            points(12..=21) & hcp(10..) & len(Suit::Hearts, 5..) & (nth_seat(1) | nth_seat(2)),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            260,
            points(11..=21) & hcp(8..) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            250,
            points(11..=21) & hcp(8..) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Better-minor openings (deny a five-card major).
        .rule(
            Bid::new(1, Strain::Diamonds),
            100,
            points(12..=21)
                & hcp(10..)
                & prefers_diamonds()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(1, Strain::Clubs),
            100,
            points(12..=21)
                & hcp(10..)
                & len(Suit::Clubs, 3..)
                & !prefers_diamonds()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        );

    // Weak twos (six-card suit, not in fourth seat).  Strength gauged by an
    // honor-location evaluator when `set_weak_two_eval` is armed, in raw HCP
    // when `set_weak_two_hcp` is armed (the Root-A preempt-discipline fix —
    // sound bridge, but it measured a wash on the honest sd-lead scorer, so it
    // stays opt-in), else the default rule-of-N+8 `points(5..=10)`.
    let weak_two_eval = WEAK_TWO_EVAL.with(Cell::get);
    let weak_two_band = WEAK_TWO_HCP.with(Cell::get);
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let bid = Bid::new(2, Strain::from(suit));
        let six = move || len(suit, 6..=6);
        if weak_two_wild() {
            rules = rules.rule(bid, 100, len(suit, 5..=6) & points(3..=12) & !nth_seat(4));
            continue;
        }
        rules = match (weak_two_eval, weak_two_band) {
            (Some(WeakTwoEval::CcccBand(lo, hi)), _) => {
                rules.rule(bid, 100, six() & cccc(lo..hi) & !nth_seat(4))
            }
            (Some(WeakTwoEval::CcccFloor(x)), _) => {
                rules.rule(bid, 100, six() & points(5..=10) & cccc(x..) & !nth_seat(4))
            }
            (Some(WeakTwoEval::NltcBand(lo, hi)), _) => {
                rules.rule(bid, 100, six() & nltc(lo..=hi) & !nth_seat(4))
            }
            (Some(WeakTwoEval::NltcCeil(x)), _) => {
                rules.rule(bid, 100, six() & points(5..=10) & nltc(..=x) & !nth_seat(4))
            }
            (None, Some((lo, hi))) => rules.rule(bid, 100, six() & hcp(lo..=hi) & !nth_seat(4)),
            (None, None) => rules.rule(bid, 100, six() & points(5..=10) & !nth_seat(4)),
        };
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            90,
            len(suit, 7..) & points(..12) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0, points(..12))
}

/// The opening table as a row package
///
/// The whole file is one node — the empty auction, fanned over the four seats
/// — so the package is a single row group.  The 1NT shape policy is read from
/// [`notrump_shape_setting`] here rather than passed in: [`Package::entries`]
/// is a bare `fn` and cannot capture, and the one caller passed exactly that.
pub(super) fn package() -> Package {
    Package {
        name: "openings",
        gate: || true,
        entries: || rows_of(Pattern::node("P*"), openings_with(notrump_shape_setting())),
    }
}

/// Register the opening table in the constructive book
pub(super) fn register(book: &mut Trie) {
    compile_into(book, &[package()]);
}

#[cfg(test)]
mod tests;
