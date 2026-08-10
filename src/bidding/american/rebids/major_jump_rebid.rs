//! Opener's jump-rebid of a six-card major with extras
//!
//! The deferred major-opening half of the [extras ladder](super::extras_ladder):
//! `1♥ - 1♠ - 3♥` and `1M - 1NT - 3M` on a 6+ suit with 16+ points, plus
//! responder's continuation over it. Gated by
//! [`opener_major_jump_rebid`][field@crate::bidding::inference::ReadingProfile::opener_major_jump_rebid].

use super::*;

/// Append opener's jump-rebid of a six-card major with extras
///
/// `major` is opener's opened suit and `highest` responder's call.  The jump
/// `3M` sits above the `2M` minimum by weight, so only a 16+ hand takes it.
/// Gated on
/// [`opener_major_jump_rebid`][field@crate::bidding::inference::ReadingProfile::opener_major_jump_rebid].
pub(super) fn with_major_jump_rebid(
    rules: Rules,
    major: Suit,
    highest: Bid,
    agreements: &Agreements,
) -> Rules {
    if !agreements.decision.reading.opener_major_jump_rebid {
        return rules;
    }
    let trump = Strain::from(major);
    let level = cheapest_level_over(highest, trump) + 1;
    rules.rule(Bid::new(level, trump), 150, len(major, 6..) & points(16..))
}

/// Responder's call over opener's invitational `3M` jump-rebid
///
/// Opener has shown 6+ of the major and 16+ points.  A forcing-1NT responder
/// is usually short in the major (3+ support would have raised), so the
/// notrump game is the common accept; a doubleton is already an eight-card fit
/// opposite six, so the major-game raise needs only `len(major, 2..)`.  Used at
/// `1M - 1NT - 3M` and `1♥ - 1♠ - 3♥`.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4M   | 1.4 | Accept: major game on an 8+ card fit (2+ support, 8+ points) |
/// | 3NT  | 1.2 | Accept: notrump game, no major fit (9+ points) |
/// | Pass | 0.0 | Decline: minimum — play `3M` |
fn responder_after_major_jump_rebid(major: Suit) -> Rules {
    let trump = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, trump), 140, len(major, 2..) & points(8..))
        .rule(Bid::new(3, Strain::Notrump), 120, points(9..))
        .rule(Call::Pass, 0, points(0..))
}

/// Responder's call over opener's `3M` jump-rebid
///
/// Covers `1M - 1NT - 3M` and `1♥ - 1♠ - 3♥`.  This package follows the
/// generic forcing-1NT package so its specialized `3M` table keeps winning the
/// same exact-node overwrite.
pub(crate) fn major_jump_rebid_continuations() -> Package {
    Package {
        name: "major-jump-rebid-continuations",
        gate: |a| a.decision.reading.opener_major_jump_rebid,
        entries: |_| {
            let mut entries = expand(
                "P* 1M - 1NT - 3M -",
                |_| true,
                |b| responder_after_major_jump_rebid(b.suit('M')),
            );
            // 1♥ - 1♠ - 3♥: opener's major is hearts, responder has shown 4+
            // spades.
            entries.extend(rows_of(
                Pattern::node("P* 1♥ - 1♠ - 3♥ -"),
                responder_after_major_jump_rebid(Suit::Hearts),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
