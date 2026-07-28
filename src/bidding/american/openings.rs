//! Uncontested openings for every seat

use super::insert_uncontested;
use crate::bidding::constraint::{
    Cons, Constraint, balanced, cccc, described, fifths, hcp, len, length_box, long_suit_box, nltc,
    nth_seat, points, shapes,
};
use crate::bidding::context::Context;
use crate::bidding::inference::Range;
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
        .rule(Bid::new(2, Strain::Clubs), 3.0, points(22..) | hcp(22..))
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
                2.0,
                fifths(14.5..17.5) & (notrump_shape(shape) | one_notrump_offshape_gate()),
            )
        } else if ONE_NOTRUMP_FIFTHS.with(Cell::get) {
            // 14.5..17.5 (centre 16), not 15..18 (centre 16.5): fifths sums to 40
            // over the deck like HCP, so an unbiased "15-17 HCP" gate shares the
            // plain-HCP band's centre — the old 15..18 was half a point too high.
            rules.rule(
                Bid::new(1, Strain::Notrump),
                2.0,
                fifths(14.5..17.5) & notrump_shape(shape),
            )
        } else if one_notrump_offshape() {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                2.0,
                hcp(15..=17) & (notrump_shape(shape) | one_notrump_offshape_gate()),
            )
        } else {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                2.0,
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
            2.0,
            fifths(20.0..22.0) & two_notrump_wide_shape(),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            2.0,
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
            1.6,
            points(12..=21) & hcp(10..) & len(Suit::Spades, 5..) & (nth_seat(1) | nth_seat(2)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            points(12..=21) & hcp(10..) & len(Suit::Hearts, 5..) & (nth_seat(1) | nth_seat(2)),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            2.6,
            points(11..=21) & hcp(8..) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            2.5,
            points(11..=21) & hcp(8..) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Better-minor openings (deny a five-card major).
        .rule(
            Bid::new(1, Strain::Diamonds),
            1.0,
            points(12..=21)
                & hcp(10..)
                & prefers_diamonds()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(1, Strain::Clubs),
            1.0,
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
            rules = rules.rule(bid, 1.0, len(suit, 5..=6) & points(3..=12) & !nth_seat(4));
            continue;
        }
        rules = match (weak_two_eval, weak_two_band) {
            (Some(WeakTwoEval::CcccBand(lo, hi)), _) => {
                rules.rule(bid, 1.0, six() & cccc(lo..hi) & !nth_seat(4))
            }
            (Some(WeakTwoEval::CcccFloor(x)), _) => {
                rules.rule(bid, 1.0, six() & points(5..=10) & cccc(x..) & !nth_seat(4))
            }
            (Some(WeakTwoEval::NltcBand(lo, hi)), _) => {
                rules.rule(bid, 1.0, six() & nltc(lo..=hi) & !nth_seat(4))
            }
            (Some(WeakTwoEval::NltcCeil(x)), _) => {
                rules.rule(bid, 1.0, six() & points(5..=10) & nltc(..=x) & !nth_seat(4))
            }
            (None, Some((lo, hi))) => rules.rule(bid, 1.0, six() & hcp(lo..=hi) & !nth_seat(4)),
            (None, None) => rules.rule(bid, 1.0, six() & points(5..=10) & !nth_seat(4)),
        };
    }
    // Three-level preempts (seven-card suit, not in fourth seat).
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        rules = rules.rule(
            Bid::new(3, Strain::from(suit)),
            0.9,
            len(suit, 7..) & points(..12) & !nth_seat(4),
        );
    }
    rules.rule(Call::Pass, 0.0, points(..12))
}

/// Register the opening table in the constructive book
pub(super) fn register(book: &mut Trie, shape: NotrumpShape) {
    insert_uncontested(book, &[], openings_with(shape));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::context::Context;
    use crate::bidding::trie::Classifier;
    use contract_bridge::auction::RelativeVulnerability;

    /// The highest-logit opening `rules` makes for a hand
    fn opens(rules: &Rules, hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let logits = rules.classify(hand, &context);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a): &(Call, &f32), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    #[test]
    fn sub_ten_hcp_freaks_open_only_in_third_seat() {
        use crate::bidding::constraint::{PointScale, set_point_scale};
        // Calibrated to the rule-of-N+8 opt-out — the scale these example
        // hands' points assume (6-5 reads 12, not the point-count cap of 10).
        set_point_scale(PointScale::RuleOfNFloored);
        // ♠5 ♥JT952 ♦J ♣AK7652 — 9 HCP, 12 points (6-5).  Rule-of-N+8 alone
        // would walk it in the sound-opening front door; `hcp(10..)` bars it in
        // first seat, and no other rule takes it (points 12 is past the weak
        // two's band, and clubs never weak-two), so every logit goes dead and
        // the full book's floor passes.  Third seat opens it: 12 points clears
        // `points(11..)` and 9 HCP clears the legal `hcp(8..)`.
        let freak: Hand = "5.JT952.J.AK7652".parse().expect("valid test hand");
        let table = openings();

        let first = Context::new(RelativeVulnerability::NONE, &[]);
        assert!(
            (&table.classify(freak, &first).0)
                .into_iter()
                .all(|(_, logit)| logit.is_infinite()),
            "first seat rejects the sub-10-HCP freak (falls through to Pass)"
        );

        let third = Context::new(RelativeVulnerability::NONE, &[Call::Pass, Call::Pass]);
        let best = (&table.classify(freak, &third).0)
            .into_iter()
            .max_by(|(_, a): &(Call, &f32), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty");
        assert_eq!(
            best,
            Call::Bid(Bid::new(1, Strain::Hearts)),
            "third seat opens it light"
        );

        assert_eq!(
            opens(&table, "AT86.975.QJ4.AJ3"),
            Call::Bid(Bid::new(1, Strain::Clubs)),
            "a flat 12 still opens its better minor in first seat"
        );
        set_point_scale(PointScale::PointCount);
    }

    #[test]
    fn wide_notrump_shape_gate() {
        let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
        let one_s = Call::Bid(Bid::new(1, Strain::Spades));
        let one_c = Call::Bid(Bid::new(1, Strain::Clubs));
        // 5422 / 6322, ~15–17 fifths, long suit a minor (joins the wide 1NT) or a
        // major (stays a suit); the long-minor 6322 also stays a suit.
        let five422_minor = "Q432.KQ.K2.AK432";
        let five422_major = "AK432.KQ.Q432.K2";
        let six322_minor = "Q2.K3.AQ4.KQ8765";
        let six322_major = "KQ8765.K3.AQ4.Q2";
        let balanced16 = "AQ32.K53.QJ4.A92";

        // Classic: only the balanced hand opens 1NT; the shapely ones open a suit.
        let narrow = openings_with(NotrumpShape::Balanced);
        assert_eq!(opens(&narrow, balanced16), one_nt);
        assert_eq!(opens(&narrow, five422_minor), one_c);
        assert_eq!(opens(&narrow, five422_major), one_s);
        assert_eq!(opens(&narrow, six322_minor), one_c);
        assert_eq!(opens(&narrow, six322_major), one_s);

        // Wide: the long-minor 5422 joins 1NT; majors and 6322 stay suits.
        let wide = openings_with(NotrumpShape::Wide);
        assert_eq!(opens(&wide, balanced16), one_nt);
        assert_eq!(opens(&wide, five422_minor), one_nt);
        assert_eq!(opens(&wide, five422_major), one_s);
        assert_eq!(opens(&wide, six322_minor), one_c);
        assert_eq!(opens(&wide, six322_major), one_s);

        // Wide6322 (default): the long-minor 6322 also joins 1NT; majors still stay suits.
        let wide6322 = openings_with(NotrumpShape::Wide6322);
        assert_eq!(opens(&wide6322, five422_minor), one_nt);
        assert_eq!(opens(&wide6322, five422_major), one_s);
        assert_eq!(opens(&wide6322, six322_minor), one_nt);
        assert_eq!(opens(&wide6322, six322_major), one_s);
    }

    #[test]
    fn suppress_one_notrump_opens_a_minor() {
        let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
        let one_c = Call::Bid(Bid::new(1, Strain::Clubs));
        let balanced16 = "AQ32.K53.QJ4.A92"; // 4333, 16 HCP — a textbook 1NT opener

        // Default: opens 1NT.
        assert_eq!(opens(&openings(), balanced16), one_nt);

        // Suppressed: the same hand opens its minor — never 1NT, never Pass.
        set_open_one_notrump(false);
        let call = opens(&openings(), balanced16);
        set_open_one_notrump(true);
        assert_eq!(call, one_c);
    }

    #[test]
    fn one_notrump_offshape_is_opt_in() {
        use contract_bridge::Seat;
        use contract_bridge::deck::full_deal;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let baseline = openings();
        set_one_notrump_offshape(false);
        let explicit_off = openings();
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let mut rng = StdRng::seed_from_u64(0x1A70_FF51);
        for _ in 0..64 {
            let deal = full_deal(&mut rng);
            for hand in Seat::ALL.map(|seat| deal[seat]) {
                assert_eq!(
                    baseline.classify(hand, &context).0,
                    explicit_off.classify(hand, &context).0
                );
            }
        }

        let offshape = "AQ43.KQJ2.K532.J";
        let one_nt = Call::Bid(Bid::new(1, Strain::Notrump));
        assert_ne!(opens(&explicit_off, offshape), one_nt);
        set_one_notrump_offshape(true);
        assert_eq!(opens(&openings(), offshape), one_nt);
        set_one_notrump_offshape(false);
    }

    #[test]
    fn weak_two_wild_is_opt_in() {
        use contract_bridge::Seat;
        use contract_bridge::deck::full_deal;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let baseline = openings();
        set_weak_two_wild(false);
        let explicit_off = openings();
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let mut rng = StdRng::seed_from_u64(0x2EA4_71D2);
        for _ in 0..64 {
            let deal = full_deal(&mut rng);
            for hand in Seat::ALL.map(|seat| deal[seat]) {
                assert_eq!(
                    baseline.classify(hand, &context).0,
                    explicit_off.classify(hand, &context).0
                );
            }
        }

        let five_card = "KQJ98.743.52.842";
        let two_s = Call::Bid(Bid::new(2, Strain::Spades));
        assert_ne!(opens(&explicit_off, five_card), two_s);
        set_weak_two_wild(true);
        assert_eq!(opens(&openings(), five_card), two_s);
        set_weak_two_wild(false);
    }

    #[test]
    fn sound_eleven_counts_open_one_of_a_suit() {
        use crate::bidding::constraint::{PointScale, set_point_scale};

        let one_s = Call::Bid(Bid::new(1, Strain::Spades));
        // 11 HCP, 5-2-4-2.  On the shipped raw-HCP+upgrade scale the wasted J9
        // doubleton voids the shape upgrade, leaving the hand at 11 points —
        // below the sole `points(12..=21)` opening — so it passes.
        let sound_11 = "AK986.J9.QJT6.64";
        assert_eq!(opens(&openings(), sound_11), Call::Pass);

        // On the rule-of-N+8 opt-out `points(12..)` *is* the Rule of 20 (11 + 9),
        // blind to the wasted J9, so the identity opens the same hand 1♠.
        set_point_scale(PointScale::RuleOfN);
        let call = opens(&openings(), sound_11);
        set_point_scale(PointScale::PointCount);
        assert_eq!(call, one_s);
    }

    #[test]
    fn weak_two_hcp_band_gauges_raw_hcp() {
        let two_s = Call::Bid(Bid::new(2, Strain::Spades));
        // 9 HCP, 6-4-2-1: on the floored scale `points` = 9 + (10−8) = 11, so
        // the default `points(5..=10)` excludes it and — too weak for a 1-opener
        // — it passes.  Raw HCP 9 is a sound weak two the HCP band admits.
        let sound_nine = "KQ9832.KJ85.74.4";
        set_weak_two_hcp(None);
        assert_eq!(opens(&openings(), sound_nine), Call::Pass);
        set_weak_two_hcp(Some((5, 10)));
        assert_eq!(opens(&openings(), sound_nine), two_s);

        // A junky shapely light hand the shape-crediting default over-admits:
        // 4 HCP, 6-4-2-1 reads `points` = 4 + 2 = 6, so the default opens a 2♠
        // the raw-HCP band (4 < 5) correctly declines.
        let junk_four = "QJ9832.T985.74.J";
        set_weak_two_hcp(None);
        assert_eq!(opens(&openings(), junk_four), two_s);
        set_weak_two_hcp(Some((5, 10)));
        assert_eq!(opens(&openings(), junk_four), Call::Pass);

        set_weak_two_hcp(None);
    }

    #[test]
    fn weak_two_eval_gauges_honor_location() {
        let two_s = Call::Bid(Bid::new(2, Strain::Spades));
        // Twin 6 HCP 6-3-2-2 hands: KQJ concentrated in the six-card suit
        // versus banished to the short suits.  Both read `points` 6, so the
        // shipped band opens both — the evaluator gauges tell them apart.
        let concentrated = "KQJ862.943.75.82";
        let scattered = "986432.94.KQ.J82";
        assert_eq!(opens(&openings(), concentrated), two_s);
        assert_eq!(opens(&openings(), scattered), two_s);

        // Discipline forms: prune the scattered hand, keep the concentrated
        // one (CCCC 8.10 vs 5.10; NLTC 9.5 vs 10.0 losers — probe-verified,
        // see `examples/probe-weak-two-eval.rs`).
        set_weak_two_eval(Some(WeakTwoEval::CcccFloor(7.0)));
        assert_eq!(opens(&openings(), concentrated), two_s);
        assert_eq!(opens(&openings(), scattered), Call::Pass);
        set_weak_two_eval(Some(WeakTwoEval::NltcCeil(9.5)));
        assert_eq!(opens(&openings(), concentrated), two_s);
        assert_eq!(opens(&openings(), scattered), Call::Pass);

        // Band (swap) forms replace `points` outright and win over the armed
        // HCP band.
        set_weak_two_hcp(Some((5, 10)));
        set_weak_two_eval(Some(WeakTwoEval::CcccBand(7.0, 13.0)));
        assert_eq!(opens(&openings(), concentrated), two_s);
        assert_eq!(opens(&openings(), scattered), Call::Pass);
        set_weak_two_eval(Some(WeakTwoEval::NltcBand(8.0, 9.5)));
        assert_eq!(opens(&openings(), concentrated), two_s);
        assert_eq!(opens(&openings(), scattered), Call::Pass);
        set_weak_two_hcp(None);

        // Byte-identical default restored.
        set_weak_two_eval(None);
        assert_eq!(opens(&openings(), scattered), two_s);
    }

    /// D1b: the box union behind each [`NotrumpShape`] variant accepts exactly
    /// the shapes the pre-DNF `balanced() | described(closure)` composite did,
    /// exhaustively over the 560-shape length lattice.
    #[test]
    fn notrump_shape_boxes_match_closure() {
        use crate::bidding::constraint::for_each_shape;
        use contract_bridge::auction::RelativeVulnerability;

        let ctx = Context::new(RelativeVulnerability::NONE, &[]);
        for shape in [
            NotrumpShape::Balanced,
            NotrumpShape::Wide,
            NotrumpShape::Wide6322,
        ] {
            let gate = notrump_shape(shape);
            for_each_shape(|lengths, hand| {
                // The replaced composite, restated on lengths: balanced, or
                // the wide closure (5m422 / Wide6322's 6m322, long suit a
                // minor — ♣ and ♦ are indexes 0 and 1 in ASC order).
                let mut sorted = lengths;
                sorted.sort_unstable();
                let is_balanced = matches!(sorted, [3, 3, 3, 4] | [2, 3, 4, 4] | [2, 3, 3, 5]);
                let long = match (shape, sorted) {
                    (NotrumpShape::Balanced, _) => 0,
                    (_, [2, 2, 4, 5]) => 5,
                    (NotrumpShape::Wide6322, [2, 2, 3, 6]) => 6,
                    _ => 0,
                };
                let wide = long != 0 && (lengths[0] == long || lengths[1] == long);
                assert_eq!(
                    gate.eval(hand, &ctx).is_finite(),
                    is_balanced || wide,
                    "{shape:?} disagrees at {lengths:?}",
                );
            });
        }
    }

    /// G0: the wide-minor 2NT shape is exactly `Wide6322` with the 5-card
    /// majors removed — it drops the 5M(332) pan-handles (a 5-card major opens
    /// one-of-a-major) and keeps the wide minors (5m422/6m322).
    #[test]
    fn two_notrump_wide_shape_drops_five_card_majors() {
        use crate::bidding::constraint::for_each_shape;
        use contract_bridge::auction::RelativeVulnerability;

        let ctx = Context::new(RelativeVulnerability::NONE, &[]);
        let wide6322 = notrump_shape(NotrumpShape::Wide6322);
        let g0 = two_notrump_wide_shape();
        for_each_shape(|lengths, hand| {
            // [C, D, H, S] in ASC order — majors are indexes 2 and 3.
            let majors_capped = lengths[2] <= 4 && lengths[3] <= 4;
            assert_eq!(
                g0.eval(hand, &ctx).is_finite(),
                wide6322.eval(hand, &ctx).is_finite() && majors_capped,
                "G0 shape disagrees at {lengths:?}",
            );
        });
    }
}
