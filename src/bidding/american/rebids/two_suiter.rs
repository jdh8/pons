//! Opener's invitational major two-suiter rebids after the forcing `1NT`
//!
//! `1♥ - 1NT - 2♠` (the reverse, 5+ hearts and 4+ spades) and `1♠ - 1NT - 3♥`
//! (the 5-5 jump), both 15–17 — the seam between the minimum natural rebids and
//! the 18+ game force.  Gated by [`RebidKnobs::forcing_nt_two_suiter`].

use super::*;

// ponytail: same construction-time toggle as the Meckstroth adjunct above.
/// Whether a rebid is opener's invitational major two-suiter (`forcing_nt_two_suiter`)
///
/// `1♥ - 1NT - 2♠` (the reverse) or `1♠ - 1NT - 3♥` (the 5-5 jump); the other
/// major has no such call.
pub(super) fn is_forcing_nt_two_suiter(major: Suit, rebid: Call) -> bool {
    match major {
        Suit::Hearts => rebid == call(2, Strain::Spades),
        Suit::Spades => rebid == call(3, Strain::Hearts),
        _ => false,
    }
}

/// Append opener's invitational (15–17) major two-suiter rebid when enabled
///
/// Fills the seam between the minimum natural rebids and the 18+ game force:
/// over `1♥` the `2♠` reverse (5+ hearts, 4+ spades, forcing one round), over
/// `1♠` the `3♥` jump (5-5 majors, invitational).  Both floor opener's first
/// suit, so both are alerted (reused reverse/jump-shift tags) and decoded by
/// rule projection.  Weights sit above the natural minimum rebids (0.9/1.0) but
/// below the `3M` major jump-rebid (1.5) and the 18+ `2NT` (1.6), so the crisp
/// `points(15..=17)` band keeps 18+ hands in the game force.
pub(super) fn with_forcing_nt_two_suiter(rules: Rules, major: Suit, knobs: &RebidKnobs) -> Rules {
    if !knobs.forcing_nt_two_suiter {
        return rules;
    }
    // Under the natural jump shifts `1♠ - 1NT - 3♥` is the 18+ jump shift;
    // only the `1♥ - 1NT - 2♠` reverse survives.
    if major == Suit::Spades && forcing_nt_jump_shifts_on(knobs) {
        return rules;
    }
    match major {
        Suit::Hearts => with_forcing_nt_reverse(rules, knobs).alert(OPENER_REVERSE),
        Suit::Spades => rules
            .rule(
                Bid::new(3, Strain::Hearts),
                115,
                len(Suit::Spades, 5..) & len(Suit::Hearts, 5..) & points(15..=17),
            )
            .alert(OPENER_JUMP_SHIFT),
        _ => rules,
    }
}

/// Opener's `1♥ - 1NT - 2♠` reverse: 5+ hearts, 4+ spades, 15–17 points
///
/// [`RebidKnobs::forcing_nt_reverse_five_hearts`] caps the hearts at five and
/// [`RebidKnobs::forcing_nt_reverse_hcp_floor`] adds a 15 HCP floor; with both
/// off the rule is the shipped one, unchanged.
fn with_forcing_nt_reverse(rules: Rules, knobs: &RebidKnobs) -> Rules {
    let bid = Bid::new(2, Strain::Spades);
    let spades = || len(Suit::Spades, 4..);
    match (
        knobs.forcing_nt_reverse_five_hearts,
        knobs.forcing_nt_reverse_hcp_floor,
    ) {
        (false, false) => rules.rule(
            bid,
            110,
            len(Suit::Hearts, 5..) & spades() & points(15..=17),
        ),
        (true, false) => rules.rule(
            bid,
            110,
            len(Suit::Hearts, 5..=5) & spades() & points(15..=17),
        ),
        (false, true) => rules.rule(
            bid,
            110,
            len(Suit::Hearts, 5..) & spades() & points(15..=17) & hcp(15..),
        ),
        (true, true) => rules.rule(
            bid,
            110,
            len(Suit::Hearts, 5..=5) & spades() & points(15..=17) & hcp(15..),
        ),
    }
}

/// Responder's call over opener's `1♥ - 1NT - 2♠` reverse (5+ hearts, 4+ spades)
///
/// Opener has 15–17 and a real spade suit; responder holds ≤ 3 spades (the
/// forcing 1NT denied four).  Forcing one round — the `2NT` fallback is the
/// finite catch-all, so there is no `Pass`.  Opener's acceptance of a below-game
/// signoff (`3♥`/`2NT`) is left to the deterministic floor (a natural invite).
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♥   | 1.5 | 5-3 heart game (3+ hearts, values) |
/// | 4♠   | 1.3 | 4-3 spade game (exactly three spades, values) |
/// | 3NT  | 1.2 | No eight-card fit, values — to play |
/// | 3♥   | 1.0 | Heart preference, minimum |
/// | 2NT  | 0.0 | Guaranteed-legal minimum catch-all |
///
/// Under [`RebidKnobs::forcing_nt_reverse_heart_game`] `4♥` takes a doubleton
/// too: the 5-2 heart fit outplays the 4-3 spade fit.
fn responder_over_forcing_nt_reverse(knobs: &RebidKnobs) -> Rules {
    let hearts = if knobs.forcing_nt_reverse_heart_game {
        2
    } else {
        3
    };
    Rules::new()
        .rule(
            Bid::new(4, Strain::Hearts),
            150,
            len(Suit::Hearts, hearts..) & points(8..),
        )
        .rule(
            Bid::new(4, Strain::Spades),
            130,
            len(Suit::Spades, 3..=3) & points(8..),
        )
        .rule(Bid::new(3, Strain::Notrump), 120, points(8..))
        .rule(Bid::new(3, Strain::Hearts), 100, len(Suit::Hearts, 2..))
        .rule(Bid::new(2, Strain::Notrump), 0, points(0..))
}

/// Responder's call over opener's `1♠ - 1NT - 3♥` jump (5-5 majors, invitational)
///
/// Opener has 15–17 and 5-5 in the majors; responder accepts to game with a fit
/// or values, else declines.  Non-forcing — `Pass` (heart tolerance) is the
/// finite catch-all.  Opener's acceptance of a `3♠` decline is left to the floor.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♠   | 1.5 | Spade fit game (3+ spades, values) |
/// | 4♥   | 1.4 | Heart fit game (3+ hearts, values) |
/// | 3NT  | 1.2 | Values, no three-card fit — to play |
/// | 3♠   | 1.0 | Spade preference, decline (minimum) |
/// | Pass | 0.0 | Heart tolerance, decline — play `3♥` |
pub(super) fn responder_over_forcing_nt_5_5() -> Rules {
    Rules::new()
        .rule(
            Bid::new(4, Strain::Spades),
            150,
            len(Suit::Spades, 3..) & points(8..),
        )
        .rule(
            Bid::new(4, Strain::Hearts),
            140,
            len(Suit::Hearts, 3..) & points(8..),
        )
        .rule(Bid::new(3, Strain::Notrump), 120, points(8..))
        .rule(Bid::new(3, Strain::Spades), 100, len(Suit::Spades, 2..))
        .rule(Call::Pass, 0, points(0..))
}

/// Responder's continuations over opener's invitational major two-suiter rebids
pub(crate) fn forcing_nt_two_suiter_continuations() -> Package {
    Package {
        name: "forcing-nt-two-suiter-continuations",
        gate: |a| a.rebid.forcing_nt_two_suiter,
        entries: |agreements| {
            let mut entries = rows_of(
                Pattern::node("P* 1♥ - 1NT - 2♠ -"),
                responder_over_forcing_nt_reverse(&agreements.rebid),
            );
            if !forcing_nt_jump_shifts_on(&agreements.rebid) {
                entries.extend(rows_of(
                    Pattern::node("P* 1♠ - 1NT - 3♥ -"),
                    responder_over_forcing_nt_5_5(),
                ));
            }
            entries
        },
    }
}
