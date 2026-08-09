//! Woolsey ("Multi-Landy") over their `1NT`
//!
//! The equilibrium defense: `X` = a one-suiter or a strong hand, `2♣` = both
//! majors (Landy's call), `2♦` = a Multi major, `2♥`/`2♠` = Muiderberg (that
//! major plus a minor).  Authored in full — every artificial call has a
//! doubled and redoubled escape — so the structure never bleeds to the floor.

use super::nt_defense::NotrumpDefense;
use super::*;

thread_local! {
    /// Inclusive `points` band for the Woolsey suit overcalls (`2♣`/`2♦`/`2♥`/`2♠`);
    /// **(8, 19) by default** — level with the natural overcall floor.  A 2026-06-26
    /// re-probe (continuations now fully authored) found honest plain-DD self-play
    /// *peaks at 8* and flattens below it (6/7 add no value), and the BBA head-to-head
    /// agrees; perfect-defense (PD) still mildly prefers 10, but PD over-deters by
    /// assuming a perfect doubler.  The conventions only rearrange *which* call shows a
    /// hand — the strength floor tracks natural's 8.  See `docs/` re-probe note.
    static WOOLSEY_POINTS: Cell<(u8, u8)> = const { Cell::new((8, 19)) };
    /// `points` floor for the Woolsey takeout `X` (4-card major + longer minor);
    /// **12 by default** — the X is the most constructive Woolsey action, so it
    /// floors above the preemptive suit overcalls.  See [`set_woolsey_double_floor`].
    static WOOLSEY_DOUBLE_FLOOR: Cell<u8> = const { Cell::new(12) };
}

/// Whether the Woolsey defense is the active system (read by the inference engine
/// to decode our artificial 2♣/2♦/2♥/2♠ overcalls; see `inference::multi_reading`)
pub(super) fn woolsey_enabled(agreements: &Agreements) -> bool {
    agreements.build.defense.notrump_defense == NotrumpDefense::Woolsey
}

/// Set the inclusive `points` band for the Woolsey suit overcalls (`2♣`/`2♦`/`2♥`/
/// `2♠`, default 8–19) for books built *after* this call.  No effect unless
/// [`NotrumpDefense::Woolsey`] is selected.  The A/B knob for
/// `examples/ab-landy --ns-woolsey-range`.
pub fn set_woolsey_points(lo: u8, hi: u8) {
    WOOLSEY_POINTS.with(|cell| cell.set((lo, hi)));
}

/// The configured Woolsey suit-overcall `points` band (also the points floor the
/// inference engine reads for our 2♣/2♦/2♥/2♠ overcalls)
pub(crate) fn woolsey_points() -> (u8, u8) {
    WOOLSEY_POINTS.with(Cell::get)
}

/// Set the `points` floor for the Woolsey takeout `X` (default 12) for books built
/// *after* this call.  No effect unless [`NotrumpDefense::Woolsey`] is selected.  The A/B knob for
/// `examples/ab-landy --ns-woolsey-x-floor`.
pub fn set_woolsey_double_floor(floor: u8) {
    WOOLSEY_DOUBLE_FLOOR.with(|cell| cell.set(floor));
}

/// The configured Woolsey takeout-`X` `points` floor
pub(crate) fn woolsey_double_floor() -> u8 {
    WOOLSEY_DOUBLE_FLOOR.with(Cell::get)
}

/// Woolsey **Multi** `2♦`: a single 6+ card major (unknown which), nothing else long —
/// both minors at most four.  M6.2d simplified the shape and states it with `or`/`and`
/// so it projects straight off the rule: the strictly-longer-major and no-6-6 guards
/// are dropped, so a 6-5 or 6-6 major hand now qualifies as Multi.
pub(super) fn woolsey_multi() -> Cons<impl Constraint + Clone> {
    or([Suit::Hearts, Suit::Spades], 6..) & and([Suit::Clubs, Suit::Diamonds], ..=4)
}

/// Woolsey **Muiderberg** `2M`: exactly 5 in `major`, at most 3 in the other major, and
/// a 4+ card minor.  M6.2d states the minor side with `or` so it projects; the shape is
/// otherwise unchanged — `5..=5` keeps it disjoint from the 6+ Multi `2♦`, and the
/// other-major ≤3 cap keeps it disjoint from the 2♣ both-majors (the Woolsey structure
/// relies on disjoint shapes so its uniform 1.9 weights never tie).
pub(super) fn woolsey_muiderberg(major: Suit) -> Cons<impl Constraint + Clone> {
    let other = if major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    len(major, 5..=5) & len(other, ..=3) & or([Suit::Clubs, Suit::Diamonds], 4..)
}

/// Woolsey takeout `X`: exactly 4 in one major, at most 3 in the other, and a longer
/// (5-6) minor (a 7+ minor one-suiter passes — no natural minor overcall).  A 4-card
/// major can co-exist with at most a 5-card minor here, so the `or([♣,♦],5..=6)` needs
/// no upper cap on the second minor — the major half bars a 7+ minor anyway.
pub(super) fn woolsey_double_shape() -> Cons<impl Constraint + Clone> {
    ((len(Suit::Hearts, 4..=4) & len(Suit::Spades, ..=3))
        | (len(Suit::Spades, 4..=4) & len(Suit::Hearts, ..=3)))
        & or([Suit::Clubs, Suit::Diamonds], 5..=6)
}

/// Woolsey takeout `X`: a 4-card major + a longer (5-6) minor, `points(floor..)`.
pub(super) fn woolsey_x(agreements: &Agreements) -> Rules {
    Rules::new().rule(
        Call::Double,
        190,
        woolsey_double_shape() & points(agreements.build.defense.woolsey_double_floor..),
    )
}

/// Woolsey `2♣` (Landy, inside the bundle): both majors, but neither major 6+
/// (`passed_two_suiter` caps each major at five, routing a 6-card major to the
/// Multi `2♦` and keeping the bundle's uniform 1.9 weights disjoint).  A distinct
/// block from [`landy_2c`] — same convention, load-bearing shape difference.
pub(super) fn woolsey_2c(agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.build.defense.woolsey_points;
    Rules::new().rule(
        Bid::new(2, Strain::Clubs),
        190,
        passed_two_suiter(Suit::Hearts, Suit::Spades) & points(lo..=hi),
    )
}

/// Woolsey Multi `2♦`: a single 6+ major.
pub(super) fn multi_2d(agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.build.defense.woolsey_points;
    Rules::new().rule(
        Bid::new(2, Strain::Diamonds),
        190,
        woolsey_multi() & points(lo..=hi),
    )
}

/// Woolsey Muiderberg `2♥`/`2♠`: exactly 5 in `major` + a 4+ minor.
pub(super) fn muiderberg(major: Suit, agreements: &Agreements) -> Rules {
    let (lo, hi) = agreements.build.defense.woolsey_points;
    Rules::new().rule(
        Bid::new(2, Strain::from(major)),
        190,
        woolsey_muiderberg(major) & points(lo..=hi),
    )
}

// ---------------------------------------------------------------------------
// Woolsey "Multi-Landy" continuations.  Authored in full so the structure never
// bleeds to the instinct floor: the Multi 2♦ (BBA's two-strength pass-or-correct
// with the 2♠ → 2NT heart-relay, plus a game-force ask), the Muiderberg 2♥/2♠
// (raises + the 2NT minor-ask), and the takeout X (relay to the minor / own major
// / ask).  The both-majors 2♣ reuses the Landy advances above.  Every artificial
// call also has a doubled / redoubled escape (wired in `defensive`) so the
// opponents can never trap us in a doubled artificial contract.
// ---------------------------------------------------------------------------

/// Advancer over the Woolsey **Multi** `2♦` (`(1NT) 2♦ -` or `(1NT) 2♦ (X)`):
/// a major pass-or-correct in two strengths, plus a game-forcing ask.  Holds no
/// `Pass`, so over a double it always corrects rather than sitting in `2♦x` (the
/// overcaller has a major, never diamonds).  Thresholds track the overcall floor
/// `lo` (the `20-lo` / `22-lo` rule, as [`landy_advances`]).
fn multi_advances(lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Game-force: ask the overcaller to name its 6-card major (it jumps to 4M).
        .rule(Bid::new(2, Strain::Notrump), 100, points(game..))
        // Constructive pass-or-correct: overcaller passes spades / 2NT-relays hearts.
        .rule(Bid::new(2, Strain::Spades), 95, points(invite..game))
        // Weak pass-or-correct: overcaller passes hearts / corrects 2♠ / jumps with 7+.
        .rule(Bid::new(2, Strain::Hearts), 90, points(..invite))
}

/// Overcaller over the weak `2♥` pass-or-correct (`(1NT) 2♦ - 2♥ -`): pass
/// with six hearts, correct to `2♠` with six spades, jump to `3♥`/`3♠` with seven
fn multi_2h_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 110, len(Suit::Hearts, 7..))
        .rule(Bid::new(3, Strain::Spades), 110, len(Suit::Spades, 7..))
        .rule(Bid::new(2, Strain::Spades), 100, len(Suit::Spades, 6..=6))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the constructive `2♠` pass-or-correct (`(1NT) 2♦ - 2♠ *`):
/// pass with spades, bid `3♥` with hearts.  Bidding the major directly (rather than
/// a 2NT heart-relay) keeps the rebid identical whether the `2♠` was passed or
/// doubled — over a double we must not be left to the floor.
fn multi_2s_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Overcaller over the game-forcing `2NT` ask (`(1NT) 2♦ - 2NT -`): jump to
/// game in the 6-card major
fn multi_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(4, Strain::Hearts), 100, len(Suit::Hearts, 6..))
        .rule(Bid::new(4, Strain::Spades), 100, len(Suit::Spades, 6..))
}

/// Advancer over a **Muiderberg** `2M` (`(1NT) 2M -`): raise the known 5-card
/// major with support (`4M` game / `3M` invitational, or a `3M` preempt with
/// four-card support), or with no fit ask the 4+ minor via `2NT` (overcaller
/// answers `3♣`/`3♦`); a weak no-fit hand passes and plays `2M`.  `major` is the
/// overcaller's suit; thresholds track the overcall floor `lo`.
fn muiderberg_advances(major: Suit, lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, strain), 120, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            110,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No major fit, invitational+: ask the 4+ minor (then place 3NT / minor game).
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            len(major, ..=2) & points(invite..),
        )
        .rule(Call::Pass, 0, hcp(0..))
}

/// Advancer over a **doubled** Muiderberg `2M` (`(1NT) 2M (X)`): with a fit sit
/// for `2Mx` (a known 8+ card trump fit) or raise; with no fit escape via the
/// `2NT` minor-ask rather than be trapped in a doubled 5-1 misfit
fn muiderberg_advances_doubled(major: Suit, lo: u8) -> Rules {
    let invite = 20u8.saturating_sub(lo);
    let game = 22u8.saturating_sub(lo);
    let strain = Strain::from(major);
    Rules::new()
        .rule(Bid::new(4, strain), 120, len(major, 3..) & points(game..))
        .rule(
            Bid::new(3, strain),
            110,
            (len(major, 4..) & points(..game)) | (len(major, 3..) & points(invite..game)),
        )
        // No fit → escape to the 4+ minor (any strength); a fit sits 2Mx.
        .rule(Bid::new(2, Strain::Notrump), 50, len(major, ..=2))
        .rule(Call::Pass, 0, len(major, 3..))
}

/// Overcaller answering the Muiderberg `2NT` minor-ask (`(1NT) 2M … 2NT -`):
/// name the 4+ minor — `3♦` with diamonds (longer or equal), else `3♣`
fn muiderberg_2nt_rebid() -> Rules {
    let diamonds_longer = at_least_as_long(Suit::Diamonds, Suit::Clubs);
    Rules::new()
        .rule(Bid::new(3, Strain::Diamonds), 100, diamonds_longer)
        .rule(Bid::new(3, Strain::Clubs), 90, hcp(0..))
}

/// Advancer over the Woolsey takeout `X` (`(1NT) X -`): bid a 5+ major of your
/// own (to play), ask with a game-going hand, else relay `2♣` to the doubler's
/// long minor.  The catch-all `2♣` owns a finite logit so the floor never runs.
fn woolsey_x_advance(lo: u8) -> Rules {
    let game = 22u8.saturating_sub(lo);
    Rules::new()
        // Our own good major outranks the doubler's (its major may be the other one).
        .rule(Bid::new(2, Strain::Spades), 111, len(Suit::Spades, 5..))
        .rule(Bid::new(2, Strain::Hearts), 110, len(Suit::Hearts, 5..))
        // Game-going: ask the doubler to name its 4-card major.
        .rule(Bid::new(2, Strain::Notrump), 100, points(game..))
        // Weak / no major of our own: name your minor, I pass or correct.
        .rule(Bid::new(2, Strain::Clubs), 90, hcp(0..))
}

/// Doubler over the `2♣` minor relay (`(1NT) X - 2♣ -`): pass with the club
/// minor, correct to `2♦` with the diamond minor (advancer denied a major)
fn woolsey_x_minor_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(2, Strain::Diamonds), 100, len(Suit::Diamonds, 5..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// Doubler over the `2NT` game-ask (`(1NT) X - 2NT -`): name the 4-card major
/// (the `X` always holds exactly one), leaving the advancer to place the game
fn woolsey_x_2nt_rebid() -> Rules {
    Rules::new()
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 4..))
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 4..))
}

/// Woolsey "Multi-Landy" continuations ([`NotrumpDefense::Woolsey`])
///
/// Authored in full — the both-majors `2♣` reuses [`landy_advance_package`]'s
/// wiring.  Every artificial call carries its doubled / redoubled escape so the
/// opponents can never trap us in a doubled artificial contract.
pub(super) fn woolsey_package() -> Package {
    Package {
        name: "woolsey",
        gate: |agreements| woolsey_enabled(agreements),
        entries: |agreements| {
            let lo = agreements.build.defense.woolsey_points.0;
            let mut entries = Vec::new();

            // Multi 2♦.  The advance is the same over a pass or a double (it never
            // sits 2♦x — the overcaller has a major, not diamonds).  `rho` is the
            // opponents' call over our 2♦; `after` is their call over our
            // pass-or-correct — the overcaller names its major regardless of a
            // double, so we are never left to the floor in a doubled 2♥x/2♠x (the
            // dominant 2♦ leak vs BBA).
            for rho in ["-", "(X)"] {
                let base = format!("P* (1NT) 2♦ {rho}");
                entries.extend(rows_of(Pattern::node(&base), multi_advances(lo)));
                for after in ["-", "(X)"] {
                    for (bid, rebid) in [
                        // Weak 2♥ p/c → pass / correct 2♠ / jump 3M with seven.
                        ("2♥", multi_2h_rebid()),
                        // Constructive 2♠ p/c → pass spades / 3♥ with hearts.
                        ("2♠", multi_2s_rebid()),
                        // Game-force 2NT ask → overcaller jumps to game in its major.
                        ("2NT", multi_2nt_rebid()),
                    ] {
                        entries.extend(rows_of(
                            Pattern::node(&format!("{base} {bid} {after}")),
                            rebid,
                        ));
                    }
                }
            }

            // Muiderberg 2♥/2♠ — raises + the 2NT minor-ask (a doubled escape with
            // no fit).
            for (major, mbid) in [(Suit::Hearts, "2♥"), (Suit::Spades, "2♠")] {
                entries.extend(rows_of(
                    Pattern::node(&format!("P* (1NT) {mbid} -")),
                    muiderberg_advances(major, lo),
                ));
                entries.extend(rows_of(
                    Pattern::node(&format!("P* (1NT) {mbid} (X)")),
                    muiderberg_advances_doubled(major, lo),
                ));
                // The 2NT minor-ask reaches the overcaller over either RHO action.
                for rho in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* (1NT) {mbid} {rho} 2NT -")),
                        muiderberg_2nt_rebid(),
                    ));
                }
            }

            // Takeout X — advancer relays to the minor / bids its own major / asks
            // 2NT.  A redouble forces us to run (never sit 1NTxx): the same advance
            // applies.
            let xfloor = agreements.build.defense.woolsey_double_floor;
            for adv in ["-", "(XX)"] {
                let base = format!("P* (1NT) X {adv}");
                entries.extend(rows_of(Pattern::node(&base), woolsey_x_advance(xfloor)));
                // The doubler names its 5-6 minor whether the 2♣ relay is passed or
                // doubled.
                for after in ["-", "(X)"] {
                    entries.extend(rows_of(
                        Pattern::node(&format!("{base} 2♣ {after}")),
                        woolsey_x_minor_rebid(),
                    ));
                }
                // The 2NT game-ask → the doubler names its 4-card major.
                entries.extend(rows_of(
                    Pattern::node(&format!("{base} 2NT -")),
                    woolsey_x_2nt_rebid(),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
