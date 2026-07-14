//! Uncontested openings for every seat

use super::insert_uncontested;
use crate::bidding::constraint::{
    Cons, Constraint, balanced, described, fifths, hcp, len, nth_seat, points, rule_of_20,
};
use crate::bidding::context::Context;
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
    /// Whether we open sound 10-11 counts that satisfy the Rule of 20 with one
    /// of a suit instead of passing.  Default `true` (shipped default-on).
    static RULE_OF_20: Cell<bool> = const { Cell::new(true) };
    /// Which shape policy the 1NT opening admits when `american()` rebuilds.
    /// Default [`NotrumpShape::Wide6322`] (the shipped default).
    static NOTRUMP_SHAPE: Cell<NotrumpShape> = const { Cell::new(NotrumpShape::Wide6322) };
    /// The weak-two opening's strength band, gauged in raw HCP when `Some`.
    /// Default `None`: byte-identical `points(5..=10)`.  See [`set_weak_two_hcp`].
    static WEAK_TWO_HCP: Cell<Option<(u8, u8)>> = const { Cell::new(None) };
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

/// Open sound 10-11 counts satisfying the Rule of 20 (raw HCP + two longest
/// suits ≥ 20) with one of a suit, instead of passing (`true`, the shipped
/// default; `false` restores the 12+-only opener).  Natural: strain priority
/// mirrors the 12+ openings (five-card major first, else the better minor).
/// Shipped default-on after the anchor's Constructive/book/opening bucket
/// traced to sound 11-counts we passed and BBA opened — a plain-DD and
/// sd-lead win both vulnerabilities (the pd loss is the perfect-doubler
/// bracket; see [docs/bba-gap-campaign.md]).
///
/// Natural and folded into base per [docs/bidding-options.md]; retained only
/// as a measurement off-switch, not a user-facing toggle (dropped from the
/// `web` settings registry).
pub fn set_rule_of_20(on: bool) {
    RULE_OF_20.with(|cell| cell.set(on));
}

/// Whether Rule-of-20 light openings are on (read by the opening inference,
/// which drops its one-level suit point floor 12→10 to match).
pub(crate) fn rule_of_20_enabled() -> bool {
    RULE_OF_20.with(Cell::get)
}

/// Select the 1NT opening [`NotrumpShape`] for the next rebuild of
/// [`american()`][crate::american()] — the web Settings shape radio.  Default
/// [`NotrumpShape::Wide6322`].  The baked ablation handles
/// ([`american_wide`][crate::bidding::american::american_wide],
/// [`american_classic`][crate::bidding::american::american_classic]) ignore this
/// knob; only [`bare_american`][crate::bidding::american::bare_american] reads it.
pub fn set_notrump_shape(shape: NotrumpShape) {
    NOTRUMP_SHAPE.with(|cell| cell.set(shape));
}

/// The 1NT opening shape currently selected by [`set_notrump_shape`].
pub(super) fn notrump_shape_setting() -> NotrumpShape {
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
fn notrump_shape(shape: NotrumpShape) -> Cons<impl Constraint + Clone> {
    balanced()
        | described("wide 1NT shape", move |hand: Hand, _: &Context<'_>| {
            let mut lengths = Suit::ASC.map(|suit| hand[suit].len());
            lengths.sort_unstable();
            let long = match (shape, lengths) {
                (NotrumpShape::Balanced, _) => return false,
                (_, [2, 2, 4, 5]) => 5,
                (NotrumpShape::Wide6322, [2, 2, 3, 6]) => 6,
                _ => return false,
            };
            hand[Suit::Clubs].len() == long || hand[Suit::Diamonds].len() == long
        })
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
        rules = if ONE_NOTRUMP_FIFTHS.with(Cell::get) {
            // 14.5..17.5 (centre 16), not 15..18 (centre 16.5): fifths sums to 40
            // over the deck like HCP, so an unbiased "15-17 HCP" gate shares the
            // plain-HCP band's centre — the old 15..18 was half a point too high.
            rules.rule(
                Bid::new(1, Strain::Notrump),
                2.0,
                fifths(14.5..17.5) & notrump_shape(shape),
            )
        } else {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                2.0,
                hcp(15..=17) & notrump_shape(shape),
            )
        };
    }
    rules = rules
        // Strong 2NT.
        .rule(
            Bid::new(2, Strain::Notrump),
            2.0,
            fifths(20.0..22.0) & balanced(),
        )
        // Five-card majors; 1♠ ranks just above 1♥ so 5-5 opens the higher.
        .rule(
            Bid::new(1, Strain::Spades),
            1.6,
            points(12..=21) & len(Suit::Spades, 5..),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            1.5,
            points(12..=21) & len(Suit::Hearts, 5..),
        )
        // Lighter five-card majors in third/fourth seat.
        .rule(
            Bid::new(1, Strain::Spades),
            2.6,
            points(9..=11) & len(Suit::Spades, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        .rule(
            Bid::new(1, Strain::Hearts),
            2.5,
            points(9..=11) & len(Suit::Hearts, 5..) & (nth_seat(3) | nth_seat(4)),
        )
        // Better-minor openings (deny a five-card major).
        .rule(
            Bid::new(1, Strain::Diamonds),
            1.0,
            points(12..=21) & prefers_diamonds() & len(Suit::Hearts, ..5) & len(Suit::Spades, ..5),
        )
        .rule(
            Bid::new(1, Strain::Clubs),
            1.0,
            points(12..=21)
                & len(Suit::Clubs, 3..)
                & !prefers_diamonds()
                & len(Suit::Hearts, ..5)
                & len(Suit::Spades, ..5),
        );

    // Weak twos (six-card suit, not in fourth seat).  Strength gauged in raw HCP
    // when `set_weak_two_hcp` is armed (the Root-A preempt-discipline fix — sound
    // bridge, but it measured a wash on the honest sd-lead scorer, so it stays
    // opt-in), else the default rule-of-N+8 `points(5..=10)`.
    let weak_two_band = WEAK_TWO_HCP.with(Cell::get);
    for suit in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let bid = Bid::new(2, Strain::from(suit));
        rules = match weak_two_band {
            Some((lo, hi)) => rules.rule(bid, 1.0, len(suit, 6..=6) & hcp(lo..=hi) & !nth_seat(4)),
            None => rules.rule(bid, 1.0, len(suit, 6..=6) & points(5..=10) & !nth_seat(4)),
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
    // Rule-of-20 light openers (sound 10-11 counts) — behind `set_rule_of_20`.
    // Natural; same weights and strain priority as the 12+ suit openings, so a
    // five-card major opens ahead of the better minor and these outrank the
    // weak two / preempt a shapely light hand would otherwise reach.
    if RULE_OF_20.with(Cell::get) {
        rules = rules
            .rule(
                Bid::new(1, Strain::Spades),
                1.6,
                hcp(10..=11) & rule_of_20() & len(Suit::Spades, 5..),
            )
            .rule(
                Bid::new(1, Strain::Hearts),
                1.5,
                hcp(10..=11) & rule_of_20() & len(Suit::Hearts, 5..),
            )
            .rule(
                Bid::new(1, Strain::Diamonds),
                1.0,
                hcp(10..=11)
                    & rule_of_20()
                    & prefers_diamonds()
                    & len(Suit::Hearts, ..5)
                    & len(Suit::Spades, ..5),
            )
            .rule(
                Bid::new(1, Strain::Clubs),
                1.0,
                hcp(10..=11)
                    & rule_of_20()
                    & len(Suit::Clubs, 3..)
                    & !prefers_diamonds()
                    & len(Suit::Hearts, ..5)
                    & len(Suit::Spades, ..5),
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
    fn rule_of_20_opens_sound_eleven_counts() {
        use crate::bidding::constraint::{PointScale, set_point_scale};

        let one_s = Call::Bid(Bid::new(1, Strain::Spades));
        // 11 HCP, 5-2-4-2, Rule of 20 (11 + 9).  The wasted J9 voids the legacy
        // points upgrade; by default we open the five-card major.
        let sound_11 = "AK986.J9.QJT6.64";
        assert_eq!(opens(&openings(), sound_11), one_s);

        // The shipped rule-of-N+8 scale absorbs the knob: Rule of 20 is
        // exactly `points(12..)` there, so the hand opens even with the light
        // rules off.
        set_rule_of_20(false);
        assert_eq!(opens(&openings(), sound_11), one_s);

        // The knob's off arm only bites on the legacy opt-out scale, where the
        // voided upgrade leaves this hand at 11 — the 12+ opener passes.
        set_point_scale(PointScale::PointCount);
        let call = opens(&openings(), sound_11);
        set_point_scale(PointScale::RuleOfNFloored);
        set_rule_of_20(true);
        assert_eq!(call, Call::Pass);
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
}
