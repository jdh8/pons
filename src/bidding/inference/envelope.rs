//! The box types: what a call showed, as ranges, and their DNF union
//!
//! An [`Envelope`] is one axis-aligned box — four per-suit length [`Range`]s and
//! a [`Strength`].  An [`EnvelopeUnion`] is a disjunction of such boxes, the
//! reading a rule's `project` fold produces when its meaning is not a single box.

use super::knobs::ReadingProfile;
use super::{LENGTH_CAP, POINTS_CAP, SUIT_HCP_CAP};
use crate::bidding::constraint::PointScale;
use contract_bridge::{Hand, Suit};

impl ReadingProfile {
    /// Whether envelope-union projection is enabled in this pinned profile.
    pub(crate) const fn envelope_union(self) -> bool {
        self.envelope_union
    }

    /// Whether the forward strength folds keep their ceilings in this profile.
    pub(crate) const fn strength_ceilings(self) -> bool {
        self.strength_ceilings
    }
}

/// An inclusive `[min, max]` range of a shown quantity — a length or points
///
/// A plain `Copy` pair rather than [`core::ops::RangeInclusive`], so it can be
/// stored, compared, and (de)serialized, and carries [`intersect`][Self::intersect].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Range {
    /// The least the quantity can be
    pub min: u8,
    /// The most the quantity can be
    pub max: u8,
}

impl Range {
    /// Nothing known about a suit length yet: `0..=13`
    pub const FULL_LENGTH: Self = Self {
        min: 0,
        max: LENGTH_CAP,
    };
    /// Nothing known about points yet: `0..=37`
    pub const FULL_POINTS: Self = Self {
        min: 0,
        max: POINTS_CAP,
    };
    /// Nothing known about a suit's HCP yet: `0..=10`
    pub const FULL_SUIT_HCP: Self = Self {
        min: 0,
        max: SUIT_HCP_CAP,
    };

    /// An inclusive `[min, max]` range
    #[must_use]
    pub const fn new(min: u8, max: u8) -> Self {
        Self { min, max }
    }

    /// `min..=cap` — at least `min`, up to the quantity's natural ceiling
    #[must_use]
    pub(super) const fn at_least(min: u8, cap: u8) -> Self {
        Self { min, max: cap }
    }

    /// Whether `n` falls within the range
    #[must_use]
    pub const fn contains(self, n: u8) -> bool {
        self.min <= n && n <= self.max
    }

    /// The conjunction of two ranges — the tighter bounds of each
    ///
    /// Two independently sound inferences about the same quantity both hold, so
    /// the truth lies in their intersection.  If the bounds cross (an empty
    /// intersection), some inference was unsound for this auction; rather than
    /// drop the truth, widen to their *span* — soundness over tightness.
    #[must_use]
    pub fn intersect(self, other: Self) -> Self {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        if min <= max {
            Self { min, max }
        } else {
            Self {
                min: self.min.min(other.min),
                max: self.max.max(other.max),
            }
        }
    }

    /// The disjunction of two ranges — the loosest bounds spanning both
    ///
    /// A hand satisfying *either* of two alternatives (an `Or` projection of a
    /// [`Constraint`][crate::bidding::constraint::Constraint]) has its quantity in one
    /// range or the other, so the sound envelope is their span.  The dual of
    /// [`intersect`][Self::intersect], which keeps the tighter bounds.
    #[must_use]
    pub fn span(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    /// The overlap, or `None` when the ranges are disjoint (an empty product)
    ///
    /// Unlike [`intersect`][Self::intersect], which widens a crossed range to
    /// preserve soundness within a single box, this reports the empty product so
    /// an [`EnvelopeUnion`] can **drop** the contradictory term.
    #[must_use]
    fn intersect_nonempty(self, other: Self) -> Option<Self> {
        let (min, max) = (self.min.max(other.min), self.max.min(other.max));
        (min <= max).then(|| Self::new(min, max))
    }

    /// Whether `other` lies entirely within this range
    const fn encloses(self, other: Self) -> bool {
        self.min <= other.min && other.max <= self.max
    }
}

/// Shown strength, gauged on the several scales bridge counts on
///
/// The scales are **not mutually ordered**: raw [`hcp`][crate::bidding::constraint::hcp],
/// the length-upgraded [`point_count`][crate::bidding::constraint::point_count] (`points`),
/// and the fit-known shortness-upgraded
/// [`support_point_count`][crate::bidding::constraint::support_point_count]
/// (`support_points`).  A reader promise is an axis-aligned interval on *one*
/// scale, so each scale is its own [`Range`] and the combinators fold
/// field-by-field.  The gauges are **marginals, never a joint**: no cross-gauge
/// relation (`points == hcp`, i.e. "balanced") fits a box — that is a shape fact,
/// and lives in [`Envelope::lengths`].  The one exception is the monotone floor
/// `support_points >= hcp`, which *is* box-representable and `canonicalize`
/// restores after every narrow.  (`points >= hcp` holds too, but is written at
/// the source by [`hcp`][crate::bidding::constraint::hcp]'s projection rather than
/// restored here — see `canonicalize`.)  The shape fact *is* recoverable per
/// box, just not in the `(hcp, points)` plane: the lengths sit in the same box,
/// so `Envelope::narrow_to_upgrade` closes the two gauges against each other
/// under
/// [`upgrade_closure`][field@crate::bidding::ReadingProfile::upgrade_closure].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Strength {
    /// Raw HCP, crisp (no upgrade slack) — the notrump-valuation gauge
    pub hcp: Range,
    /// HCP + long-suit upgrade, on the [`points`][crate::bidding::constraint::points] scale
    /// the suit-oriented rules gauge (raw HCP for the balanced openings) — the
    /// legacy single axis
    pub points: Range,
    /// HCP + shortness, on the fit-known suit-indexed
    /// [`support_point_count_in`][crate::bidding::constraint::support_point_count_in]
    /// scale — one slot per candidate trump suit, indexed by `suit as usize`
    /// like [`Envelope::lengths`], each gauged with its own suit as trump (no
    /// shortness value in the trump suit itself).
    pub support_points: [Range; 4],
    /// Raw HCP held in each suit, indexed by `suit as usize` like
    /// [`Envelope::lengths`]; cap 10 (AKQJ).  The honor-*location* gauge the
    /// quality gates (`suit_hcp`, `top_honors`) read — deliberately uncoupled
    /// from the whole-hand gauges in `canonicalize`: every candidate coupling
    /// either writes an old axis (a shipped-reading change) or manufactures
    /// the containment that lets `EnvelopeUnion::tidy`'s correct dedup swallow the arm
    /// holding the suit knowledge.
    pub suit_hcp: [Range; 4],
}

impl Strength {
    /// Nothing shown yet: every whole-hand gauge `0..=37`, every suit `0..=10`
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            hcp: Range::FULL_POINTS,
            points: Range::FULL_POINTS,
            support_points: [Range::FULL_POINTS; 4],
            suit_hcp: [Range::FULL_SUIT_HCP; 4],
        }
    }

    /// Restore the sound cross-gauge floor `support_points >= hcp`
    ///
    /// `hcp` bounds every scale from below (`point_count` and
    /// `support_point_count` both add a non-negative upgrade to raw HCP, bar the
    /// rule-of-N flat-4-3-3-3 read the `points` scale already slacks by
    /// [`flat_hcp_slack`][crate::bidding::constraint::flat_hcp_slack]).  So a pure-HCP
    /// promise (a 15–17 1NT) floors the support gauge for free, without a
    /// fit-showing raise having fired.  Monotone (only raises a `.min`), so it
    /// never narrows past the truth.
    ///
    /// [`points`][Self::points] is **not** floored here even though the same
    /// implication holds: its floor is written at the source by
    /// [`Hcp::project`][crate::bidding::constraint::hcp], and adding it here would be a
    /// shipped-reading change (a bare `narrow_hcp` currently leaves `points`
    /// alone).  [`Envelope::narrow_to_upgrade`] is where that coupling lands,
    /// knob-gated and two-sided.
    ///
    /// [`suit_hcp`][Self::suit_hcp] is deliberately **not** coupled here: the
    /// sound whole-hand implications (`hcp.min >= Σ suit mins`, a length-capped
    /// suit ceiling) all write an *old* axis — a shipped-reading change — and
    /// the floor coupling manufactures exactly the containment that lets
    /// [`EnvelopeUnion::tidy`]'s correct dedup swallow the arm carrying the suit
    /// knowledge (the `points(22..) | hcp(22..)` lesson, in reverse).
    fn canonicalize(&mut self, scale: PointScale) {
        let floor = self
            .hcp
            .min
            .saturating_sub(crate::bidding::constraint::flat_hcp_slack(scale));
        for slot in &mut self.support_points {
            slot.min = slot.min.max(floor);
        }
    }

    /// Field-by-field [`Range::intersect`], then [`canonicalize`][Self::canonicalize]
    #[must_use]
    fn intersect(mut self, other: Self, scale: PointScale) -> Self {
        self.hcp = self.hcp.intersect(other.hcp);
        self.points = self.points.intersect(other.points);
        self.support_points =
            core::array::from_fn(|i| self.support_points[i].intersect(other.support_points[i]));
        self.suit_hcp = core::array::from_fn(|i| self.suit_hcp[i].intersect(other.suit_hcp[i]));
        self.canonicalize(scale);
        self
    }

    /// Field-by-field [`Range::span`] — the `|` dual, soundness over tightness
    #[must_use]
    fn span(self, other: Self) -> Self {
        Self {
            hcp: self.hcp.span(other.hcp),
            points: self.points.span(other.points),
            support_points: core::array::from_fn(|i| {
                self.support_points[i].span(other.support_points[i])
            }),
            suit_hcp: core::array::from_fn(|i| self.suit_hcp[i].span(other.suit_hcp[i])),
        }
    }

    /// Bounded intersection; `None` only when the `points` gauge is disjoint
    ///
    /// **Only `points` gates box-emptiness**, exactly as the pre-`Strength` box
    /// algebra did.  The new gauges combine by the widening [`Range::intersect`]
    /// so they never drop a box a `points`/length reading would have kept — they
    /// are inert until Edits 1/2, and must not perturb the [`EnvelopeUnion`] the sampler
    /// reads through `admits` (which reads `points` only).
    fn intersect_nonempty(self, other: Self, scale: PointScale) -> Option<Self> {
        let mut out = Self {
            hcp: self.hcp.intersect(other.hcp),
            points: self.points.intersect_nonempty(other.points)?,
            support_points: core::array::from_fn(|i| {
                self.support_points[i].intersect(other.support_points[i])
            }),
            suit_hcp: core::array::from_fn(|i| self.suit_hcp[i].intersect(other.suit_hcp[i])),
        };
        out.canonicalize(scale);
        Some(out)
    }

    /// The support-points floor for `suit` as trump if that slot was narrowed
    /// from unknown, else `None` — a consumer reads this dedicated axis only
    /// when populated, and otherwise falls back to the length-scale
    /// [`points`][Self::points].  Suit-blind (the default) every slot holds
    /// the same band; suit-indexed, the slot is on the same units minus the
    /// trump suit's phantom shortness — never any length term.
    #[must_use]
    pub fn support_floor(&self, suit: Suit) -> Option<u8> {
        let slot = self.support_points[suit as usize];
        (slot.max < Range::FULL_POINTS.max).then_some(slot.min)
    }

    /// The raw-HCP floor if the [`hcp`][Self::hcp] gauge was narrowed, else `None`.
    #[must_use]
    pub fn hcp_floor(&self) -> Option<u8> {
        (self.hcp.max < Range::FULL_POINTS.max).then_some(self.hcp.min)
    }

    /// The sharpest sound scalar floor of the shown strength — the legacy
    /// [`points`][Self::points] floor lifted by every per-suit support promise
    ///
    /// The raise readers once wrote their support-scale bands verbatim onto
    /// the legacy axis, and every floor gate summing "own count + partner's
    /// shown floor" calibrated on that number.  Now that the legacy axis
    /// keeps only a band's sound image (`support_band_to_points`), this max
    /// hands those gates the same figure from its correct home.  Sound
    /// either way: each slot floor is a true claim about the hand's value in
    /// play with that trump, and `canonicalize` seeds slots only from the
    /// raw-HCP floor, itself a lower bound of every scale.
    #[must_use]
    pub fn shown_floor(&self) -> u8 {
        self.support_points
            .iter()
            .map(|slot| slot.min)
            .fold(self.points.min, u8::max)
    }
}

/// What the calls have shown about one player, hand-independently
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Envelope {
    /// Shown length range per suit, indexed by `suit as usize` (the ascending
    /// [`Suit::ASC`] order: clubs, diamonds, hearts, spades)
    pub lengths: [Range; 4],
    /// Shown strength, gauged on every scale (see [`Strength`])
    pub strength: Strength,
}

impl Envelope {
    /// Nothing shown yet: every suit `0..=13`, every strength gauge `0..=37`
    #[must_use]
    pub const fn unknown() -> Self {
        Self {
            lengths: [Range::FULL_LENGTH; 4],
            strength: Strength::unknown(),
        }
    }

    /// The shown length range of a suit
    #[must_use]
    pub const fn length(&self, suit: Suit) -> Range {
        self.lengths[suit as usize]
    }

    /// Narrow a suit's shown length by intersecting in `range`
    pub(super) fn narrow_length(&mut self, suit: Suit, range: Range) {
        let slot = &mut self.lengths[suit as usize];
        *slot = slot.intersect(range);
    }

    /// Narrow the shown points (length scale) by intersecting in `range`
    pub(super) fn narrow_points(&mut self, range: Range) {
        self.strength.points = self.strength.points.intersect(range);
    }

    /// Narrow the shown support points (fit-known scale) by intersecting in
    /// `range`
    ///
    /// Only fit-showing raises call this: a raise's point promise is valued on
    /// the support scale once the fit is agreed.  `suit` is the agreed trump —
    /// the promise narrows that suit's slot alone.
    pub(in crate::bidding) fn narrow_support_points(&mut self, suit: Suit, range: Range) {
        let slot = &mut self.strength.support_points[suit as usize];
        *slot = slot.intersect(range);
    }

    /// Narrow the shown raw HCP by intersecting in `range`, then propagate
    pub(super) fn narrow_hcp(&mut self, range: Range, scale: PointScale) {
        self.strength.hcp = self.strength.hcp.intersect(range);
        self.strength.canonicalize(scale);
    }

    /// Pointwise intersection — the `&` projection (both sets of bounds hold)
    ///
    /// The forward dual of a constraint conjunction: a hand accepted by `a & b`
    /// lies within both envelopes, so each quantity takes the tighter bounds
    /// ([`Range::intersect`]).
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        self.intersect_on(other, ReadingProfile::default().point_scale)
    }

    /// [`intersect`][Self::intersect] on an explicit point scale — what every
    /// classify-time caller uses, so the fold runs on the scale pinned into the
    /// partnership rather than the classifying thread's.
    #[must_use]
    pub(crate) fn intersect_on(&self, other: &Self, scale: PointScale) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.narrow_length(suit, other.length(suit));
        }
        out.strength = out.strength.intersect(other.strength, scale);
        out
    }

    /// Pointwise span — the bounding box covering either set of bounds
    ///
    /// The forward dual of a constraint disjunction: a hand accepted by `a | b`
    /// lies within one envelope or the other, so each quantity spans both
    /// ([`Range::span`]) — soundness over tightness.
    #[must_use]
    pub fn span(&self, other: &Self) -> Self {
        let mut out = *self;
        for suit in Suit::ASC {
            out.lengths[suit as usize] = out.length(suit).span(other.length(suit));
        }
        out.strength = out.strength.span(other.strength);
        out
    }

    /// The intersection box, or `None` when any axis is disjoint (empty product)
    ///
    /// Unlike [`intersect`][Self::intersect], which widens a crossed range to
    /// preserve soundness *within* a single box, this reports the empty product
    /// so an [`EnvelopeUnion`] can **drop** the contradictory term — the surviving terms of
    /// a union still cover every hand, so the union stays sound while getting
    /// tighter (e.g. `1NT ∩ 4-5♥` drops the balanced-diamond box whose hearts
    /// cannot reach four).
    fn intersect_nonempty(&self, other: &Self, scale: PointScale) -> Option<Self> {
        let mut lengths = [Range::FULL_LENGTH; 4];
        for suit in Suit::ASC {
            let (a, b) = (self.length(suit), other.length(suit));
            let (min, max) = (a.min.max(b.min), a.max.min(b.max));
            if min > max {
                return None;
            }
            lengths[suit as usize] = Range::new(min, max);
        }
        let strength = self.strength.intersect_nonempty(other.strength, scale)?;
        Some(Self { lengths, strength })
    }

    /// Whether a hand's suit lengths and point count all fall within this box
    ///
    /// The per-box membership test the sampler and [`EnvelopeUnion::contains`] share.
    /// Reads the `points` (length) gauge only — until
    /// [`gauge_membership`][field@crate::bidding::inference::ReadingProfile::gauge_membership]
    /// (chop E, default off) also gives the raw-HCP
    /// and support-points bands membership teeth.
    #[must_use]
    pub fn admits(&self, hand: Hand) -> bool {
        self.admits_on(hand, ReadingProfile::default())
    }

    /// [`admits`][Self::admits] on an explicit reading profile — what the
    /// sampler tests through, so acceptance runs on the settings pinned into
    /// the partnership rather than the sampling thread's.
    #[must_use]
    pub(crate) fn admits_on(&self, hand: Hand, profile: ReadingProfile) -> bool {
        Suit::ASC.into_iter().all(|suit| {
            // SAFETY: a suit length is at most 13, so the cast cannot truncate.
            #[allow(clippy::cast_possible_truncation)]
            let length = hand[suit].len() as u8;
            self.length(suit).contains(length)
        }) && self
            .strength
            .points
            .contains(crate::bidding::constraint::point_count_on(
                profile.point_scale,
                hand,
            ))
            && (!profile.gauge_membership
                || (self
                    .strength
                    .hcp
                    .contains(crate::bidding::constraint::raw_hcp(hand))
                    && self.supports(hand, profile)
                    && self.suit_hcps(hand)))
    }

    /// Whether the hand's support points fit every suit's slot, each gauged by
    /// [`support_point_count_in`][crate::bidding::constraint::support_point_count_in]
    /// with that suit as trump (clamped at the scale cap, whose ceiling means
    /// "unbounded")
    fn supports(&self, hand: Hand, profile: ReadingProfile) -> bool {
        Suit::ASC.into_iter().all(|suit| {
            let value = crate::bidding::constraint::support_point_count_in_on(
                profile.support_points,
                profile.point_scale,
                hand,
                suit,
            );
            self.strength.support_points[suit as usize].contains(value.min(Range::FULL_POINTS.max))
        })
    }

    /// Whether the hand's raw HCP in each suit fits that suit's
    /// [`suit_hcp`][Strength::suit_hcp] slot (no clamp — a suit holds at most
    /// 10 HCP physically)
    fn suit_hcps(&self, hand: Hand) -> bool {
        Suit::ASC.into_iter().all(|suit| {
            self.strength.suit_hcp[suit as usize]
                .contains(crate::bidding::constraint::suit_raw_hcp(hand, suit))
        })
    }

    /// Whether some 13-card hand can realize this box's suit lengths
    ///
    /// A box born of an intersection can be range-nonempty on every axis yet
    /// sum-infeasible — `balanced ∩ {3..}⁴` leaves a 5-3-3-3 product summing
    /// 14 — and such a **ghost box** admits nothing.  Dropping it from a union
    /// is exact.
    fn sum_feasible(&self) -> bool {
        let min: u8 = self.lengths.iter().map(|range| range.min).sum();
        let max: u8 = self.lengths.iter().map(|range| range.max).sum();
        min <= 13 && 13 <= max
    }

    /// Narrow the suit lengths to the tightest bounds `Σ len = 13` implies
    ///
    /// [`sum_feasible`][Self::sum_feasible] only *tests* the sum; a box born of
    /// an `&` keeps whatever ⊤ its arms left, so a both-majors reading stores
    /// `{♠ 5..13, ♥ 5..13, ♦ 0..13, ♣ 0..13}` when eight of those thirteen
    /// cards are already spoken for.  One sweep achieves bounds consistency for
    /// a single linear equality, and it is **exact**: the largest realizable
    /// `len_i` is `min(hi_i, 13 − Σ_{j≠i} lo_j)`, attained by giving `i` that
    /// value and distributing the rest, which is feasible for any
    /// `sum_feasible` box.  Symmetric for the floor.
    ///
    /// **Membership-inert** — every 13-card hand satisfies the sum, so
    /// [`admits`][Self::admits] is unchanged.  Only [`EnvelopeUnion::hull`] (tighter) and
    /// [`subset_of`][Self::subset_of] (more containments) move.  Idempotent.
    fn narrow_to_sum(&mut self) {
        let total_min: u8 = self.lengths.iter().map(|range| range.min).sum();
        let total_max: u8 = self.lengths.iter().map(|range| range.max).sum();
        // Every suit reads the *original* totals — that is what makes the one
        // sweep bounds-consistent (and idempotent) rather than order-dependent.
        let before = self.lengths;
        for (range, was) in self.lengths.iter_mut().zip(before) {
            range.max = was.max.min(13_u8.saturating_sub(total_min - was.min));
            range.min = was.min.max(13_u8.saturating_sub(total_max - was.max));
        }
    }

    /// Close `hcp` and `points` against each other through the shape upgrade
    ///
    /// `points = hcp + upgrade` on the shipped scale, and the upgrade is a
    /// function of shape and honor placement — so the box's own `lengths` bound
    /// it.  The case that pays: **balanced hands never upgrade**
    /// ([`upgrade`][crate::bidding::constraint::upgrade]; every balanced shape has its
    /// two longest suits at 9 cards or fewer), so a box whose lengths force
    /// balanced has `points == hcp` exactly, where
    /// [`Points::project`][crate::bidding::constraint::points] slacked `hcp` by the
    /// scale's global worst case in both directions.
    ///
    /// Exact — it drops no hand the box claims — hence sound for
    /// [`subset_of`][Self::subset_of] dedup.  But **not membership-inert**,
    /// unlike [`narrow_to_sum`][Self::narrow_to_sum]: it bounds `points`,
    /// which [`admits`][Self::admits] tests, using `hcp`, which `admits`
    /// ignores until
    /// [`gauge_membership`][field@crate::bidding::ReadingProfile::gauge_membership].
    /// So it gives an unenforced HCP
    /// claim teeth through `points`, and the sampler *does* move
    /// (`upgrade_closure_gives_hcp_teeth`).  A no-op on scales whose upgrade a
    /// length box cannot bound (see `crate::bidding::constraint::upgrade_ceiling`).
    fn narrow_to_upgrade(&mut self, scale: PointScale) {
        let Some(ceiling) = crate::bidding::constraint::upgrade_ceiling(scale, &self.lengths)
        else {
            return;
        };
        // The floor of `points − hcp` is 0 on every closable scale (a wasted
        // honor can eat the whole upgrade), so only the ceiling is two-sided.
        let strength = &mut self.strength;
        strength.points.min = strength.points.min.max(strength.hcp.min);
        strength.hcp.max = strength.hcp.max.min(strength.points.max);
        strength.points.max = strength
            .points
            .max
            .min(strength.hcp.max.saturating_add(ceiling));
        strength.hcp.min = strength
            .hcp
            .min
            .max(strength.points.min.saturating_sub(ceiling));
        // A raised `hcp` floor owes the support gauge its floor back.
        strength.canonicalize(scale);
    }

    /// Whether every hand in this box also lies in `other` — axis-wise
    /// enclosure over the lengths and every gauge
    fn subset_of(&self, other: &Self) -> bool {
        Suit::ASC
            .into_iter()
            .all(|suit| other.length(suit).encloses(self.length(suit)))
            && other.strength.hcp.encloses(self.strength.hcp)
            && other.strength.points.encloses(self.strength.points)
            && (self.strength.support_points.iter())
                .zip(other.strength.support_points)
                .all(|(inner, outer)| outer.encloses(*inner))
            && (self.strength.suit_hcp.iter())
                .zip(other.strength.suit_hcp)
                .all(|(inner, outer)| outer.encloses(*inner))
    }

    /// Whether a hand lies within this box on **every** gauge — the strict,
    /// gate-side membership test
    ///
    /// [`admits`][Self::admits] plus the raw-HCP and support-points gauges,
    /// each scored on its own scale (raw HCP, per-suit
    /// [`support_point_count_in`][crate::bidding::constraint::support_point_count_in]).
    /// This is what a natively authored [`Envelope`] / [`EnvelopeUnion`] **gate** evaluates
    /// through `Constraint::eval`: the box is the whole rule, so every stored
    /// bound — ceilings included — is enforced.  The reading-side
    /// [`admits`][Self::admits] stays lenient (lengths + `points` only) for
    /// sampler compatibility with
    /// projected readings.  Strict ⟹ lenient, so a native gate's accepted
    /// hands always satisfy the eval-within-projection soundness sweep.
    #[must_use]
    pub fn accepts(&self, hand: Hand) -> bool {
        let profile = ReadingProfile::default();
        self.admits_on(hand, profile)
            && self
                .strength
                .hcp
                .contains(crate::bidding::constraint::raw_hcp(hand))
            && self.supports(hand, profile)
            && self.suit_hcps(hand)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EnvelopeBoxes {
    One(Envelope),
    Many(Vec<Envelope>),
}

/// A forward reading as a nonempty union of envelope boxes
///
/// One [`Envelope`] is a single axis-aligned box; a disjunction (`Multi`, a
/// two-suiter, a `!`-shape) needs a *union* of boxes, which a single box cannot
/// hold without widening to the bounding box (the "`Or` wall").  An
/// [`EnvelopeUnion`] keeps the terms of that disjunctive normal form: a hand is
/// consistent with the call iff it lies in **some** box.
///
/// Sound by construction — every operation is *exact or widening, never
/// narrowing*.  [`intersect`][Self::intersect] distributes (Cartesian product of
/// box-intersects, dropping empty products); [`union`][Self::union] concatenates.
/// [`hull`][Self::hull] collapses the union back to the single bounding box, the
/// migration escape hatch that reproduces the legacy single-box reading.
#[derive(Clone, PartialEq, Eq)]
pub struct EnvelopeUnion(EnvelopeBoxes);

impl core::fmt::Debug for EnvelopeUnion {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("EnvelopeUnion")
            .field(&self.boxes())
            .finish()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for EnvelopeUnion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(self.boxes(), serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for EnvelopeUnion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let boxes = <Vec<Envelope> as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self::from_boxes(boxes))
    }
}

impl EnvelopeUnion {
    /// Nothing shown yet: a single [`Envelope::unknown`] box
    #[must_use]
    pub fn unknown() -> Self {
        Self(EnvelopeBoxes::One(Envelope::unknown()))
    }

    fn from_boxes(mut boxes: Vec<Envelope>) -> Self {
        if boxes.len() == 1 {
            Self(EnvelopeBoxes::One(boxes.pop().expect("one envelope")))
        } else {
            // Preserve serde's historical acceptance of an empty sequence even
            // though all public constructors maintain the non-empty invariant.
            Self(EnvelopeBoxes::Many(boxes))
        }
    }

    /// The bounding box of the union — fold [`Envelope::span`] over the terms
    ///
    /// Today's single-box behaviour: every consumer that still wants one
    /// [`Envelope`] hulls here.  Never narrows (a hand in some term is in the
    /// hull), so hulling a sound `EnvelopeUnion` stays sound.
    #[must_use]
    pub fn hull(&self) -> Envelope {
        self.boxes()
            .iter()
            .copied()
            .reduce(|a, b| a.span(&b))
            .unwrap_or_else(Envelope::unknown)
    }

    /// Whether **some** box admits the hand — tighter than `hull().admits()`
    #[must_use]
    pub fn contains(&self, hand: Hand) -> bool {
        self.contains_on(hand, ReadingProfile::default())
    }

    /// [`contains`][Self::contains] on an explicit reading profile (see
    /// [`Envelope::admits_on`])
    #[must_use]
    pub(crate) fn contains_on(&self, hand: Hand, profile: ReadingProfile) -> bool {
        self.boxes().iter().any(|b| b.admits_on(hand, profile))
    }

    /// The disjoined boxes — non-empty (`len >= 1`) by invariant
    #[must_use]
    pub fn boxes(&self) -> &[Envelope] {
        match &self.0 {
            EnvelopeBoxes::One(box_) => core::slice::from_ref(box_),
            EnvelopeBoxes::Many(boxes) => boxes,
        }
    }

    /// Concatenate the terms — the `|` projection (either box may hold)
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        match (self.0, other.0) {
            (EnvelopeBoxes::One(one), EnvelopeBoxes::One(two)) => {
                Self(EnvelopeBoxes::Many(vec![one, two]))
            }
            (EnvelopeBoxes::One(one), EnvelopeBoxes::Many(mut many)) => {
                many.insert(0, one);
                Self(EnvelopeBoxes::Many(many))
            }
            (EnvelopeBoxes::Many(mut many), EnvelopeBoxes::One(one)) => {
                many.push(one);
                Self(EnvelopeBoxes::Many(many))
            }
            (EnvelopeBoxes::Many(mut one), EnvelopeBoxes::Many(mut two)) => {
                one.append(&mut two);
                Self(EnvelopeBoxes::Many(one))
            }
        }
    }

    /// The same projection fold under the shipped decision defaults.
    #[must_use]
    pub fn disjoin(self, other: Self) -> Self {
        self.disjoin_with(other, ReadingProfile::default())
    }

    /// The `|` combine the projection fold uses: separate boxes under
    /// [`envelope_union`][field@crate::bidding::ReadingProfile::envelope_union],
    /// else the single bounding-box hull
    ///
    /// Off, reproduces [`Envelope::span`] exactly, so the hull
    /// path stays byte-identical; on, keeps the arms so an enclosing `&`
    /// distributes and the sampler pins the disjunction.
    #[must_use]
    pub(crate) fn disjoin_with(self, other: Self, profile: ReadingProfile) -> Self {
        if profile.envelope_union {
            self.union(other).tidy(profile)
        } else {
            Self::from(self.hull().span(&other.hull()))
        }
    }

    /// Cartesian product of pairwise box-intersects — the `&` projection
    ///
    /// `(A ∪ B) ∩ (C ∪ D) = (A∩C) ∪ (A∩D) ∪ (B∩C) ∪ (B∩D)`, dropping empty
    /// products (`Envelope::intersect_nonempty`).  The common case is one box
    /// each → one box out (`and` is a cheap box-shrink); growth needs *both*
    /// sides to be genuine disjunctions.  If every product is empty the whole
    /// conjunction is unsatisfiable, so fall back to the widened hull-intersect
    /// — sound and loose, never an empty (unsound) `EnvelopeUnion`.
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        self.clone()
            .intersect_owned(other, ReadingProfile::default())
    }

    /// Consuming intersection used by append-only projection accumulators.
    pub(crate) fn intersect_owned(self, other: &Self, profile: ReadingProfile) -> Self {
        let scale = profile.point_scale;
        let fallback = self.hull().intersect_on(&other.hull(), scale);
        match (self.0, &other.0) {
            (EnvelopeBoxes::One(one), EnvelopeBoxes::One(two)) => Self(EnvelopeBoxes::One(
                one.intersect_nonempty(two, scale).unwrap_or(fallback),
            ))
            .tidy(profile),
            (EnvelopeBoxes::Many(mut boxes), EnvelopeBoxes::One(one)) => {
                boxes.retain_mut(|box_| {
                    let Some(product) = box_.intersect_nonempty(one, scale) else {
                        return false;
                    };
                    *box_ = product;
                    true
                });
                if boxes.is_empty() {
                    Self(EnvelopeBoxes::One(fallback)).tidy(profile)
                } else {
                    Self::from_boxes(boxes).tidy(profile)
                }
            }
            (left, _) => {
                let left = Self(left);
                let mut out = Vec::new();
                for a in left.boxes() {
                    for b in other.boxes() {
                        if let Some(product) = a.intersect_nonempty(b, scale) {
                            out.push(product);
                        }
                    }
                }
                if out.is_empty() {
                    out.push(fallback);
                }
                // ponytail: no cap — `and`-of-two-`or`s is the only multiplier and it is
                // rare, so the Vec stays short on the real book.  The assert fires
                // loudly if some auction blows up; add sound exact-merge (containment +
                // axis-adjacency) only then.
                let out = Self::from_boxes(out).tidy(profile);
                debug_assert!(
                    out.boxes().len() < 64,
                    "envelope union term explosion: {} boxes",
                    out.boxes().len()
                );
                out
            }
        }
    }

    pub(super) fn intersect_assign(&mut self, other: &Self, profile: ReadingProfile) {
        let owned = core::mem::replace(self, Self::unknown());
        *self = owned.intersect_owned(other, profile);
    }

    /// Knob-on box hygiene — drop what changes nothing, keep the union exact
    ///
    /// Two prunes, both union-preserving: **ghost boxes** whose suit lengths
    /// are sum-infeasible ([`Envelope::sum_feasible`]) admit no hand, and a
    /// box **contained** in another ([`Envelope::subset_of`]) adds no hands
    /// (equal boxes keep their first copy).  Between them, under
    /// [`sum_closure`][field@crate::bidding::ReadingProfile::sum_closure] /
    /// [`upgrade_closure`][field@crate::bidding::ReadingProfile::upgrade_closure],
    /// each surviving box is
    /// narrowed to the bounds its own contents imply
    /// (`Envelope::narrow_to_sum`, `Envelope::narrow_to_upgrade`) — exact, so
    /// the extra containments the dedup then finds are real.  Runs only under
    /// [`envelope_union`][field@crate::bidding::ReadingProfile::envelope_union] —
    /// the knob-off hull path must stay byte-identical — and restores the
    /// non-empty invariant with ⊤ if every box was a ghost (an unsatisfiable
    /// conjunction; sound, loose, rare).
    pub(super) fn tidy(self, profile: ReadingProfile) -> Self {
        if !profile.envelope_union {
            return self;
        }
        let mut boxes = match self.0 {
            EnvelopeBoxes::One(mut box_) => {
                if !box_.sum_feasible() {
                    return Self::unknown();
                }
                if profile.sum_closure {
                    box_.narrow_to_sum();
                }
                if profile.upgrade_closure {
                    box_.narrow_to_upgrade(profile.point_scale);
                }
                return Self(EnvelopeBoxes::One(box_));
            }
            EnvelopeBoxes::Many(boxes) => boxes,
        };
        boxes.retain(Envelope::sum_feasible);
        if profile.sum_closure || profile.upgrade_closure {
            // Exact and membership-inert, so running it *before* the dedup is
            // safe: every containment it exposes is a real one.  Sum first —
            // it can force a box balanced, which is what the upgrade closure
            // reads.
            for box_ in &mut boxes {
                if profile.sum_closure {
                    box_.narrow_to_sum();
                }
                if profile.upgrade_closure {
                    box_.narrow_to_upgrade(profile.point_scale);
                }
            }
        }
        let mut i = 0;
        while i < boxes.len() {
            let mut redundant = false;
            for (j, b) in boxes.iter().enumerate() {
                let a = &boxes[i];
                if i != j && a.subset_of(b) && (!b.subset_of(a) || j < i) {
                    redundant = true;
                    break;
                }
            }
            if redundant {
                boxes.remove(i);
            } else {
                i += 1;
            }
        }
        if boxes.is_empty() {
            return Self::unknown();
        }
        Self::from_boxes(boxes)
    }
}

impl From<Envelope> for EnvelopeUnion {
    fn from(box_: Envelope) -> Self {
        Self(EnvelopeBoxes::One(box_))
    }
}

/// A seat relative to the player about to act, clockwise
///
/// The discriminant is the seating order from the actor — the natural index
/// for a sampler walking the four hands — *not* the auction's parity, so map
/// an auction position through `relative_of` rather than casting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Relative {
    /// The player about to act
    Me = 0,
    /// Left-hand opponent (the next to act)
    Lho = 1,
    /// Partner
    Partner = 2,
    /// Right-hand opponent (the previous to act)
    Rho = 3,
}

/// The relative seat of the call at `index` in an auction of length `len`
///
/// Mirrors [`Context`]'s parity: the call before the actor's (`len - index ==
/// 1`) is RHO, two before is partner, three before is LHO, four before is the
/// actor again.
pub(crate) const fn relative_of(len: usize, index: usize) -> Relative {
    match (len - index) % 4 {
        0 => Relative::Me,
        1 => Relative::Rho,
        2 => Relative::Partner,
        _ => Relative::Lho,
    }
}

#[cfg(test)]
mod tests;
