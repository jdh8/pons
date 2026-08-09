//! Direct overcalls and takeout doubles over their opening
//!
//! The base defensive table: natural suit overcalls, the `1NT` overcall, and
//! the takeout double.  [`DoubleShape`] and [`TakeoutSupport`] tune what the
//! double promises; the point bands and disciplines are knobs
//! ([`set_natural_overcall_points`], `agreements.defense.overcall_discipline`,
//! `agreements.defense.passed_hand_overcall`,
//! `agreements.defense.strong_double_hcp`).

use super::michaels::{michaels_advances, unusual_nt_advances};
use super::*;

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

thread_local! {
    /// HCP floor for the natural penalty double of their 1NT; **15 by default**.
    static NATURAL_DOUBLE_FLOOR: Cell<u8> = const { Cell::new(15) };
    /// Inclusive `points` range for the natural two-level suit overcall of their
    /// 1NT; **(8, 14) by default**. Lifting the ceiling lets a strong one-suiter
    /// overcall its suit instead of falling through to the penalty double.
    static NATURAL_OVERCALL_POINTS: Cell<(u8, u8)> = const { Cell::new((8, 14)) };
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

/// Set the HCP floor of the natural penalty double of their 1NT (default 15) for
/// books built *after* this call. An A/B knob (`bba-match --ns-double-floor`).
pub fn set_natural_double_floor(floor: u8) {
    NATURAL_DOUBLE_FLOOR.with(|cell| cell.set(floor));
}

pub(crate) fn natural_double_floor() -> u8 {
    NATURAL_DOUBLE_FLOOR.with(Cell::get)
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

    let one_nt = Bid::new(1, Strain::Notrump);
    let nt_base = hcp(15..=18) & balanced() & stopper_in_their_suits();
    let mut rules = if agreements.defense.nt_overcall_no_major {
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
    // doubling and pulling to the 3-level.  See `agreements.defense.takeout_support`.
    rules = match agreements.defense.takeout_support {
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
            let mut entries = Vec::new();
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let theirs = Strain::from(suit);
                let opening = Bid::new(1, theirs);
                let key = format!("P* ({opening})");
                entries.extend(rows_of(
                    Pattern::node(&key),
                    defense_to_suit(opening, agreements),
                ));
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
