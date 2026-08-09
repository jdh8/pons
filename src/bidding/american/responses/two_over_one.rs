use super::GAME_FORCE;
use crate::bidding::Rules;
use crate::bidding::agreements::ResponseKnobs;
use crate::bidding::constraint::{
    Cons, Constraint, envelope_union_upgrade, hcp, len, points, support, support_points,
};
use crate::bidding::inference::{Envelope, EnvelopeUnion, Range, Strength};
use contract_bridge::{Bid, Strain, Suit};
use std::cell::Cell;

std::thread_local! {
    /// Whether the major 2/1 game-force entry gains the **fit leg**: with
    /// exactly three-card support the 2/1 is a preparation for `4M`, so the
    /// hand is gauged in `support_points` (the fit is privately known —
    /// opener promised five).  Default `true` — **shipped default-on
    /// 2026-07-15** jointly with the `Hcp13` gate (alone a vul-only plain
    /// win; the pair plain +0.0033/+0.0048, PD +0.0070/+0.0087 NV/vul —
    /// the fit leg re-admits with support what the hcp gate demotes).
    static TWO_OVER_ONE_FIT: Cell<bool> = const { Cell::new(true) };
}

std::thread_local! {
    /// The gauge for the **no-fit** leg of the major 2/1 game-force entry.
    /// Default [`TwoOverOneGate::Points13`] — **shipped 2026-07-25** under the
    /// PointCount scale (277059f): `points(13..)` re-admits the shapely
    /// 11-12 HCP hands that the raw-HCP `Hcp13` gate demoted to a forcing 1NT.
    /// `Hcp13` (shipped 2026-07-15) is the shape-indifferent opt-out.
    static TWO_OVER_ONE_GATE: Cell<TwoOverOneGate> =
        const { Cell::new(TwoOverOneGate::Points13) };
}

std::thread_local! {
    /// Whether the major 2/1 game force names **natural per-call suit lengths**
    /// instead of a uniform four: `1♠ - 2♥` promises five (a 2/1 into a major is
    /// a real five-card suit), `1♠ - 2♣` allows three (the cheapest 2/1 is the
    /// catch-all), and the rest keep four.  Default `false` (uniform four,
    /// book byte-identical); A/B pending.
    static TWO_OVER_ONE_NATURAL_LENGTHS: Cell<bool> = const { Cell::new(false) };
}

std::thread_local! {
    /// Whether `1♠ - 2♥` (the five-card-major 2/1) forces game a shade light: its
    /// no-fit `Hcp*` floor drops by one — `hcp(12..)` at the default `Hcp13`
    /// gate — serving both 3NT and `4♥`.  Default `false` (book byte-identical);
    /// A/B pending.
    static TWO_OVER_ONE_MAJOR_DISCOUNT: Cell<bool> = const { Cell::new(false) };
}

std::thread_local! {
    /// Whether `1♠ - 2♥` forces game on a flat twelve, banking its **ensured
    /// five-card heart suit**: the no-fit leg becomes `len(♥,5..) & hcp(12..)`
    /// (from the default `points(13..)` at `min_len` four), admitting the flat
    /// 5=3=3=2 twelve-counts that the `points` scale leaves at a forcing 1NT
    /// (they carry no `upgrade`).  The bet: unlike the minor 2/1s' thin 3NT, a
    /// five-card major finds a `4♥` landing whenever opener holds three — the
    /// strain-location fix, not the upgrade.  The fit leg (exactly-three-card
    /// spade support on `support_points(13..)`) is unchanged.  Default `false`
    /// (book byte-identical); A/B pending.
    static TWO_OVER_ONE_HEART_LIGHT: Cell<bool> = const { Cell::new(false) };
}

/// The gauge for the no-fit leg of the major 2/1 game force
/// (`set_two_over_one_gate`)
///
/// Under the default PointCount scale (277059f: raw HCP + a linearised
/// `upgrade`), the default is [`Points13`][Self::Points13] — `points(13..)`
/// admits the shapely 11-12 HCP hands whose upgrade lifts them to 13, on the
/// bet (SD-PD-confirmed, 2026-07-25) that a shaped 12-count out-tricks a flat
/// 13.  `Hcp13` (the 2026-07-15 shipped gate) is the shape-indifferent
/// alternative — it demotes those same hands to a forcing 1NT — and `Hcp14`
/// its stricter counterpart.  `Points12`/`Hcp12` lower the floor a further
/// point; both lost at PD (the thin-game doubling signature — the perfect
/// defender doubles the balanced-12 3NT), so they stay opt-in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TwoOverOneGate {
    /// `points(13..)` on the global scale — the shipped default: under
    /// PointCount, shape (`upgrade`) lets a strong 11-12 force game
    #[default]
    Points13,
    /// `points(12..)` on the global scale — one point lighter than the
    /// default; on the rule-of-N+8 scale this is exactly the Rule of 20 (raw
    /// HCP plus the two longest suits, floored at 8)
    Points12,
    /// Raw `hcp(13..)` — shape-indifferent, demotes shaped 11-12s to 1NT;
    /// the 2026-07-15 shipped gate, now the opt-out
    Hcp13,
    /// Raw `hcp(12..)` — one lighter, admits every 12-HCP hand
    Hcp12,
    /// Raw `hcp(14..)` — one *stricter* than `Hcp13`, the tightening
    /// counterpart to `Hcp12`: is 13 itself too light, or does tightening
    /// give back more than it costs?
    Hcp14,
}

impl TwoOverOneGate {
    /// The raw-HCP floor of an `Hcp*` gate (unused by the `Points*` gates,
    /// which are matched separately in [`super::major_responses`])
    pub(super) const fn hcp_floor(self) -> u8 {
        match self {
            Self::Points13 | Self::Points12 | Self::Hcp13 => 13,
            Self::Hcp12 => 12,
            Self::Hcp14 => 14,
        }
    }
}

/// Author the fit leg of the major 2/1 game force for books built after this
/// call (default `true`; off-switch `--no-ns-two-over-one-fit` in `bba-gen`)
///
/// On: a hand with exactly three-card support and a biddable side suit enters
/// the 2/1 on `support_points(13..)` — the 2/1 is a preparation for `4M`, and
/// the fit is privately known (opener promised five), so shortness counts.
/// Off: every 2/1 is gauged by the no-fit gate alone.
pub fn set_two_over_one_fit(on: bool) {
    TWO_OVER_ONE_FIT.with(|cell| cell.set(on));
}

/// Whether the 2/1 fit leg is currently authored
pub(super) fn two_over_one_fit() -> bool {
    TWO_OVER_ONE_FIT.with(Cell::get)
}

/// Set the no-fit gauge of the major 2/1 game force for books built after
/// this call (default [`TwoOverOneGate::Points13`];
/// `--ns-two-over-one-gate` in `bba-gen`)
pub fn set_two_over_one_gate(gate: TwoOverOneGate) {
    TWO_OVER_ONE_GATE.with(|cell| cell.set(gate));
}

/// The currently authored no-fit 2/1 gauge
pub(super) fn two_over_one_gate() -> TwoOverOneGate {
    TWO_OVER_ONE_GATE.with(Cell::get)
}

/// Author natural per-call suit lengths for the major 2/1 game force for books
/// built after this call (default `false`;
/// `--ns-two-over-one-natural-lengths` in `bba-gen`)
///
/// On: `1♠ - 2♥` promises 5+ hearts and `1♠ - 2♣` allows 3+ clubs (the cheapest
/// 2/1 is the catch-all); every other 2/1 keeps its 4+ floor.  Off: a uniform
/// 4+ in every 2/1 suit.
pub fn set_two_over_one_natural_lengths(on: bool) {
    TWO_OVER_ONE_NATURAL_LENGTHS.with(|cell| cell.set(on));
}

/// Whether natural per-call 2/1 suit lengths are currently authored
pub(super) fn two_over_one_natural_lengths() -> bool {
    TWO_OVER_ONE_NATURAL_LENGTHS.with(Cell::get)
}

/// Lighten the `1♠ - 2♥` game force by one HCP for books built after this call
/// (default `false`; `--ns-two-over-one-major-discount` in `bba-gen`)
///
/// On: the no-fit leg of `1♠ - 2♥` drops its `Hcp*` floor by one — `hcp(12..)`
/// at the default `Hcp13` gate — because the five-card major is worth a game
/// force a shade light.  Off: the full gate floor.  No effect on the `Points*`
/// gates or on any other 2/1.
pub fn set_two_over_one_major_discount(on: bool) {
    TWO_OVER_ONE_MAJOR_DISCOUNT.with(|cell| cell.set(on));
}

/// Whether the `1♠ - 2♥` HCP discount is currently authored
pub(super) fn two_over_one_major_discount() -> bool {
    TWO_OVER_ONE_MAJOR_DISCOUNT.with(Cell::get)
}

/// Force `1♠ - 2♥` game on a flat twelve with five hearts for books built after
/// this call (default `false`; measured via `ab-point-count --fix
/// two-over-one-heart-light`)
///
/// On: the no-fit leg of `1♠ - 2♥` becomes `len(♥,5..) & hcp(12..)` — the ensured
/// five-card major forces game a full HCP light, admitting the flat 5=3=3=2
/// twelve-counts the `points` scale leaves at a forcing 1NT.  Off: the shipped
/// `points(13..)` no-fit gate at `min_len` four.  No effect on any other 2/1 or
/// on the fit leg.  Unlike [`set_two_over_one_major_discount`] (which threads
/// only the `Hcp*` gates), this overrides the `Points*` default directly.
///
/// **Refuted 2026-07-25** (default stays off): the admitted flat twelves do not
/// settle in the intended `4♥` on the 5-3 fit — the floor's slam machinery
/// overshoots to `6♥`/`7♥` because the 2/1 response reads `0..=37` (the deferred
/// fit-split `Or` erasure; see `docs/ai-bidder/sampled-projection.md`), so opener
/// cannot see responder is a minimum.  A/B `ab-point-count --fix`: plain
/// −0.0007/−0.0005, PD −0.0010/−0.0009 IMPs/board NV/vul.  A **reading-cap**
/// re-measure candidate — capping the 2/1 reading (a ceiling, not just
/// `set_two_over_one_slam_strength`'s floor) is the prerequisite.
pub fn set_two_over_one_heart_light(on: bool) {
    TWO_OVER_ONE_HEART_LIGHT.with(|cell| cell.set(on));
}

/// Whether the `1♠ - 2♥` flat-twelve heart-light gate is currently authored
pub(super) fn two_over_one_heart_light() -> bool {
    TWO_OVER_ONE_HEART_LIGHT.with(Cell::get)
}

pub(super) fn with_two_over_one(rules: Rules, major: Suit, knobs: &ResponseKnobs) -> Rules {
    let mut rules = rules;
    let trump = Strain::from(major);
    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.  The
    // entry gate splits per the knobs: the no-fit gauge is `points` or raw
    // `hcp` (`set_two_over_one_gate`), and the fit leg
    // (`set_two_over_one_fit`) admits exactly-three-card support on
    // `support_points` — fit-known, so shortness counts.  The default is
    // `(true, Points13)` (shipped 2026-07-25); the `(off, Points13)` arm
    // reproduces the pre-knob legacy book byte-identically.
    let mut weight = 110;
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
        if Strain::from(suit) < trump {
            let bid = Bid::new(2, Strain::from(suit));
            // Suit-length floor: a 2/1 into a major promises five (2♥ over 1♠),
            // and the cheapest 2/1 (2♣ over 1♠) is the catch-all and can be
            // three; every other 2/1 stays four.  Hearts only reaches this loop
            // over 1♠ (`Strain < trump` bars it over 1♥), so it needs no guard.
            let min_len = if knobs.two_over_one_natural_lengths {
                match suit {
                    Suit::Hearts => 5,
                    Suit::Clubs if major == Suit::Spades => 3,
                    _ => 4,
                }
            } else {
                4
            };
            // 2♥ over 1♠ (the five-card major) may force game one HCP light.
            let discount = u8::from(knobs.two_over_one_major_discount && suit == Suit::Hearts);
            // Heart-light overrides the gate on `1♠ - 2♥` alone: the ensured
            // five-card suit forces game on a flat twelve (`len(♥,5..) &
            // hcp(12..)`), reaching `4♥` on the 5-3 fit — the strain-location
            // bet.  Fit leg (exactly-three-card spade support) unchanged; an
            // early-out keeps the gate match below byte-identical.  ponytail:
            // pairs with the shipped fit-split, never `set_two_over_one_fit(false)`.
            if knobs.two_over_one_heart_light && suit == Suit::Hearts {
                rules = rules
                    .rule(
                        bid,
                        weight,
                        fit_split_gate(suit, 5, major, hcp(12..), gauge_floor(|s| &mut s.hcp, 12)),
                    )
                    .alert(GAME_FORCE);
                weight -= 5;
                continue;
            }
            rules = match (knobs.two_over_one_fit, knobs.two_over_one_gate) {
                (false, TwoOverOneGate::Points13) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..) & points(13..) & !support(4..),
                ),
                (false, TwoOverOneGate::Points12) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..) & points(12..) & !support(4..),
                ),
                (false, gate) => rules.rule(
                    bid,
                    weight,
                    len(suit, min_len..) & hcp((gate.hcp_floor() - discount)..) & !support(4..),
                ),
                (true, TwoOverOneGate::Points13) => rules.rule(
                    bid,
                    weight,
                    fit_split_gate(
                        suit,
                        min_len,
                        major,
                        points(13..),
                        gauge_floor(|s| &mut s.points, 13),
                    ),
                ),
                (true, TwoOverOneGate::Points12) => rules.rule(
                    bid,
                    weight,
                    fit_split_gate(
                        suit,
                        min_len,
                        major,
                        points(12..),
                        gauge_floor(|s| &mut s.points, 12),
                    ),
                ),
                (true, gate) => rules.rule(
                    bid,
                    weight,
                    fit_split_gate(
                        suit,
                        min_len,
                        major,
                        hcp((gate.hcp_floor() - discount)..),
                        gauge_floor(|s| &mut s.hcp, gate.hcp_floor() - discount),
                    ),
                ),
            }
            .alert(GAME_FORCE);
            weight -= 5;
        }
    }
    rules
}

/// The 2/1 fit-split gate as a native [`EnvelopeUnion`] — the union of the two hands a
/// game-forcing 2/1 with the fit leg admits
///
/// Replaces the composite `len(suit, min_len..) & !support(4..) & (no_fit |
/// (support(3..) & support_points(13..)))`: opener's `major` is statically
/// known here, so the `support` legs are plain length pins on it, and
/// `support(3..) & !support(4..)` pins the major to **exactly three**.
///
/// | box | `suit` | `major` | strength |
/// | --- | --- | --- | --- |
/// | no-fit | `min_len..` | `..=3` | the arm's gauge floor (`no_fit_floor`) |
/// | fit | `min_len..` | exactly 3 | 13+ support points |
///
/// The knob-on boxes are pinned eval-equivalent to the legacy composite by
/// `fit_split_union_matches_composite`, and the legacy gate carries everything
/// shipped — eval, describe, and the knob-off reading with all its
/// context-sensitivity (its `support` legs replay under the *reader's* seat,
/// which is how every 2/1 came to read `0..=37`,
/// docs/ai-bidder/sampled-projection.md).  Knob-on, [`envelope_union_upgrade`] swaps in
/// the exact two-box reading — the fit-split bug's cure.
fn fit_split_gate(
    suit: Suit,
    min_len: usize,
    major: Suit,
    no_fit: Cons<impl Constraint + Clone + 'static>,
    no_fit_floor: impl FnOnce(&mut Strength),
) -> Cons<impl Constraint + Clone> {
    let legacy = len(suit, min_len..)
        & !support(4..)
        & (no_fit | (support(3..) & support_points(major, 13..)));
    envelope_union_upgrade(legacy, fit_split_boxes(suit, min_len, major, no_fit_floor))
}

/// The exact two-box knob-on reading of [`fit_split_gate`]
fn fit_split_boxes(
    suit: Suit,
    min_len: usize,
    major: Suit,
    no_fit_floor: impl FnOnce(&mut Strength),
) -> EnvelopeUnion {
    // A 2/1 length floor is 3..=5, so the cast cannot truncate.
    let min_len = u8::try_from(min_len).unwrap_or_else(|_| unreachable!());
    let mut base = Envelope::unknown();
    base.lengths[suit as usize] = Range::new(min_len, Range::FULL_LENGTH.max);

    let mut no_fit = base;
    no_fit.lengths[major as usize] = Range::new(0, 3);
    no_fit_floor(&mut no_fit.strength);

    let mut fit = base;
    fit.lengths[major as usize] = Range::new(3, 3);
    fit.narrow_support_points(major, Range::new(13, Range::FULL_POINTS.max));

    EnvelopeUnion::from(no_fit).union(EnvelopeUnion::from(fit))
}

/// A gauge floor for [`fit_split_boxes`]'s no-fit box: `floor..` on the field
/// `pick` selects
fn gauge_floor(
    pick: impl FnOnce(&mut Strength) -> &mut Range,
    floor: u8,
) -> impl FnOnce(&mut Strength) {
    move |strength| *pick(strength) = Range::new(floor, Range::FULL_POINTS.max)
}

#[cfg(test)]
mod tests;
