use super::GAME_FORCE;
use crate::bidding::Rules;
use crate::bidding::agreements::ResponseKnobs;
use crate::bidding::constraint::{
    Cons, Constraint, envelope_union_upgrade, hcp, len, points, support, support_points,
};
use crate::bidding::inference::{Envelope, EnvelopeUnion, Range, Strength};
use contract_bridge::{Bid, Strain, Suit};

/// The gauge for the no-fit leg of the major 2/1 game force
/// ([`ResponseKnobs::two_over_one_gate`])
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

pub(super) fn with_two_over_one(rules: Rules, major: Suit, knobs: &ResponseKnobs) -> Rules {
    let mut rules = rules;
    let trump = Strain::from(major);
    // 2/1 game-forcing new suits: cheaper suits, ranked up the line.  The
    // entry gate splits per the knobs: the no-fit gauge is `points` or raw
    // `hcp` (`two_over_one_gate`), and the fit leg
    // (`two_over_one_fit`) admits exactly-three-card support on
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
            // pairs with the shipped fit-split, never `two_over_one_fit: false`.
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
