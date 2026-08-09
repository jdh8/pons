//! Opener's jump-rebid of a six-card major with extras
//!
//! The deferred major-opening half of the [extras ladder](super::extras_ladder):
//! `1♥ - 1♠ - 3♥` and `1M - 1NT - 3M` on a 6+ suit with 16+ points, plus
//! responder's continuation over it.  Gated by [`set_opener_major_jump_rebid`].

use super::*;

// ponytail: same construction-time toggle idiom as the extras ladder above.
std::thread_local! {
    /// Whether opener's major-opening rebid nodes carry the jump-rebid rung of
    /// a six-card major with extras (`1♥ - 1♠ - 3♥`, `1M - 1NT - 3M`) and
    /// responder's continuation over it.  Shipped **on** (BBA-gap bucket #3
    /// residual); see [`set_opener_major_jump_rebid`].
    static OPENER_MAJOR_JUMP_REBID: Cell<bool> = const { Cell::new(true) };
}

/// Enable opener's major jump-rebid rung in books built after this call
///
/// The [extras ladder](super::set_opener_extras_ladder) covers only the two
/// minor-opening rebid nodes; the major-opening nodes (`1♥ - 1♠` and the
/// forcing-`1NT` rebid) still cap opener's own-major rebid at a minimum `2M`
/// with no upper bound, so a 16+ hand with a strong six-card major underbids
/// and misses the game BBA reaches (the `6+ ♥`/`6+ ♠` residual in the
/// Constructive/book/round-2 anchor bucket — `3♥ → 4♥`, `2♥ → 3♥`, `3♠ → 4♠`).
///
/// This adds the single jump-rebid `3M` (6+ suit, 16+ points), disjoint from
/// the `2M` minimum by a crisp point band, **plus responder's continuation**
/// (`responder_after_major_jump_rebid`: raise `4M` on an 8-card fit, `3NT` with
/// no fit, pass with a minimum).  It is the deferred major-opening half of the
/// extras ladder, scoped to opener's *own* suit to avoid the Meckstroth `3m`
/// collision on the jump-shift-into-a-minor rung.  Natural (names opener's own
/// suit), so unalerted and floor-safe; the matching
/// [`Inferences`](crate::bidding::inference) reading gates on the same toggle.
///
/// Read at book-construction time; shipped default-on (+0.0059/+0.0125 plain,
/// +0.0046/+0.0104 PD IMPs/board vs BBA, NV/vul, all CIs>0).  The bare rung
/// *without* the continuation measured a loss (−0.005/−0.009 plain: responder
/// passed the invitational `3M` and stranded below game) — authoring both sides
/// flipped it to a win.
pub fn set_opener_major_jump_rebid(on: bool) {
    OPENER_MAJOR_JUMP_REBID.with(|cell| cell.set(on));
}

/// Whether opener's major jump-rebid rung is currently enabled
///
/// Read at book-construction time by `register`, and at classify time by the
/// matching `Inferences` reading.
pub(crate) fn opener_major_jump_rebid() -> bool {
    OPENER_MAJOR_JUMP_REBID.with(Cell::get)
}

/// Append opener's jump-rebid of a six-card major with extras
///
/// `major` is opener's opened suit and `highest` responder's call.  The jump
/// `3M` sits above the `2M` minimum by weight, so only a 16+ hand takes it.
/// Gated on [`set_opener_major_jump_rebid`].
pub(super) fn with_major_jump_rebid(rules: Rules, major: Suit, highest: Bid) -> Rules {
    if !opener_major_jump_rebid() {
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
        gate: |_| opener_major_jump_rebid(),
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
