//! Direct overcalls and takeout doubles over their opening
//!
//! The base defensive table: natural suit overcalls, the `1NT` overcall, and
//! the takeout double.  [`DoubleShape`] and [`TakeoutSupport`] tune what the
//! double promises; the point bands and disciplines are knobs
//! ([`natural_overcall_points`][field@crate::bidding::inference::ReadingProfile::natural_overcall_points],
//! `agreements.defense.overcall_discipline`,
//! `agreements.defense.passed_hand_overcall`,
//! `agreements.defense.strong_double_hcp`).

use super::michaels::{michaels_advances, unusual_nt_advances};
use super::*;
use crate::bidding::rows::expand;

/// Exact disclosure for the opt-in weak direct jump: the generic natural walk
/// can only infer "six-plus" because other jump overcalls may be strong.  This
/// tag lets the authored projection carry the disjoint six-card/weak band to
/// partner and the opponents without changing those other treatments.
const WEAK_JUMP_OVERCALL: Alert = Alert("weak-jump-overcall");

/// Which shapes qualify for the natural penalty double of their 1NT (the 15+ HCP
/// floor is fixed; this only widens the *shape* gate).  Selected by
/// `agreements.defense.natural_double_shape`.
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

/// Support gate added to the 12+ takeout double of a suit / weak-two opening.
///
/// The 12+ tier of the takeout double only checks shortness in *their* suit(s),
/// so an off-shape one-suiter short in an unbid suit doubles at 12 and — when its
/// suit ranks below theirs — outranks the 2-level overcall (weight 1.3 > 1.0),
/// gets pulled to the 3-level, and lands doubled.  This gate demands genuine
/// support for the unbid suits on the 12+ tier, forcing off-shape hands down to
/// an overcall or up to the 17+ any-shape tier (matching BBA's two-regime X:
/// 12+ with 3-suit support, else 17+).  Selected by
/// `agreements.defense.takeout_support`.
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

/// Author the natural `1NT` rule with the configured major-suit preference
fn one_notrump_overcall<C: Constraint + 'static>(
    their_suit: Suit,
    agreements: &Agreements,
    when: Cons<C>,
) -> Rules {
    let strict = agreements.defense.nt_overcall_no_major;
    let cheap = agreements.defense.nt_overcall_prefer_one_level_major;
    // Strict always wins.  Otherwise the lighter arm denies a major only when
    // it ranks above their opening and is therefore available at level one.
    let deny = |major: Suit| major != their_suit && (strict || (cheap && major > their_suit));
    let one_nt = Bid::new(1, Strain::Notrump);
    match (deny(Suit::Hearts), deny(Suit::Spades)) {
        (true, true) => Rules::new().rule(
            one_nt,
            150,
            when & len(Suit::Hearts, ..=4) & len(Suit::Spades, ..=4),
        ),
        (true, false) => Rules::new().rule(one_nt, 150, when & len(Suit::Hearts, ..=4)),
        (false, true) => Rules::new().rule(one_nt, 150, when & len(Suit::Spades, ..=4)),
        (false, false) => Rules::new().rule(one_nt, 150, when),
    }
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
pub fn defense_to_suit(their_opening: Bid, agreements: &Agreements) -> Rules {
    let theirs = their_opening.strain;
    let t = theirs.suit().expect("their opening is always a suit bid");

    let mut rules = if agreements.defense.nt_overcall_without_stopper {
        one_notrump_overcall(t, agreements, hcp(15..=18) & balanced())
    } else {
        one_notrump_overcall(
            t,
            agreements,
            hcp(15..=18) & balanced() & stopper_in_their_suits(),
        )
    };

    // 12+ takeout double, optionally gated on support for the unbid suits so an
    // off-shape one-suiter overcalls (or waits for the 17+ tier) instead of
    // doubling and pulling to the 3-level.  See `agreements.defense.takeout_support`.
    rules = match agreements.defense.takeout_support {
        TakeoutSupport::Off => rules.rule(
            Call::Double,
            130,
            hcp(12..) & short_in_their_suits() & takeout_double_shape_ok(agreements.defense),
        ),
        TakeoutSupport::Lenient => rules.rule(
            Call::Double,
            130,
            hcp(12..)
                & short_in_their_suits()
                & unbid_support(1)
                & takeout_double_shape_ok(agreements.defense),
        ),
        TakeoutSupport::Strict => rules.rule(
            Call::Double,
            130,
            hcp(12..)
                & short_in_their_suits()
                & unbid_support(0)
                & takeout_double_shape_ok(agreements.defense),
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
    // (`ReadingProfile::pass`) can project the band a passed hand sits within.
    rules = match agreements.defense.strong_double_hcp {
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
    // overcall); `agreements.defense.overcall_discipline = false` reverts to the
    // flat 8–16.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let level = if strain > theirs { 1 } else { 2 };
            let weight = if level == 1 { 140 } else { 100 };
            // The exact shared BEN/BBA residual: with a weak six-card suit,
            // both references preempt while we make the cheapest overcall.
            // The shipped major family and opt-in `(1♣) 2♦` arm share the
            // same disjoint box; seven-card preempts remain with the floor.
            let weak_major = agreements.defense.direct_weak_jump_overcall
                && level == 1
                && matches!(suit, Suit::Hearts | Suit::Spades);
            let weak_diamond = agreements.defense.direct_minor_weak_jump_overcall
                && level == 1
                && t == Suit::Clubs
                && suit == Suit::Diamonds;
            if weak_major || weak_diamond {
                rules = rules
                    .rule(
                        Bid::new(2, strain),
                        150,
                        // Spell the HCP ceiling as the complement of the
                        // 12+ tier: forward projection normally carries only
                        // point floors, while a complemented lower bound
                        // carries the exact ceiling the alert must disclose.
                        len(suit, 6..=6) & points(8..) & !hcp(12..),
                    )
                    .alert(WEAK_JUMP_OVERCALL);
            }
            // A passed hand may take the disciplined 2-level overcall lighter (9+):
            // it cannot hold opening values, so the 11+ floor would all but forbid
            // the safe light overcall.  Off by default; see `set_passed_hand_overcall`.
            let tight_minor = level == 2
                && matches!(suit, Suit::Clubs | Suit::Diamonds)
                && agreements.defense.two_level_minor_overcall_tight;
            let relax_passed = agreements.defense.overcall_discipline
                && level == 2
                && agreements.defense.passed_hand_overcall
                && !tight_minor;
            let lo = if !agreements.defense.overcall_discipline || level == 1 {
                8
            } else if tight_minor {
                15
            } else if relax_passed {
                9
            } else {
                11
            };
            let hi = if agreements.defense.overcall_discipline {
                17
            } else {
                16
            };
            // The band top is the other face of the strong-tier floor: when the
            // partition is HCP-gauged it moves with it, so overflow lands in the
            // tier instead of a shape-blind double on a five-card suit.
            rules = match (agreements.defense.strong_double_hcp, relax_passed) {
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
            if agreements.defense.overcall_four_card {
                rules = match (agreements.defense.strong_double_hcp, relax_passed) {
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

    // Michaels cue-bid: 2 of their suit, 5-5, 8+ HCP — the HCP leg is armed by
    // `defense.two_suiter_hcp_floor`, shipped `Some(8)`: the bare `points(8..)`
    // gate let rule-of-N+8 cue 5-HCP 6-5 freaks into penalty doubles.
    let (high, low) = match t {
        // t minor → both majors; t major → the other major (paired with a minor
        // below via `low`).
        Suit::Clubs | Suit::Diamonds => (Suit::Spades, Some(Suit::Hearts)),
        Suit::Hearts => (Suit::Spades, None),
        Suit::Spades => (Suit::Hearts, None),
    };
    rules = match (agreements.defense.two_suiter_hcp_floor, low) {
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
    match agreements.defense.two_suiter_hcp_floor {
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
        gate: |_| true,
        entries: |agreements| {
            let mut entries = expand(
                "P* (1x)",
                |_| true,
                |b| defense_to_suit(Bid::new(1, b.suit('x').into()), agreements),
            );
            entries.extend(expand(
                "P* (1x) 2x -",
                |_| true,
                |b| michaels_advances(b.suit('x')),
            ));
            entries.extend(expand(
                "P* (1x) 2NT -",
                |_| true,
                |b| unusual_nt_advances(b.suit('x')),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
