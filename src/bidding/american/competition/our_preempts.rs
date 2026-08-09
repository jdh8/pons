//! Our contested weak twos and our contested strong `2♣`
//!
//! Two agreements sharing a shape: they interfere over *our* preemptive or
//! strong opening.  [`set_weak_two_competition`] covers the weak two (business
//! redouble, contested Ogust); [`set_strong_two_competition`] the `2♣` (systems
//! on over their double, opener's forced reopening over their overcall).
use super::*;

thread_local! {
    /// Whether our contested weak twos are authored: responder over their
    /// takeout double (business `XX`, systems-on Ogust) and over their
    /// overcall (Ogust-when-legal, values `X`, preemptive raises). Default
    /// off while the A/B runs.
    static WEAK_TWO_COMPETITION: Cell<bool> = const { Cell::new(false) };

    /// Whether our contested strong 2♣ is authored: systems-on over their
    /// double, and over their overcall a natural-GF / values-`X` / waiting-
    /// pass structure backed by opener's forced reopening. Without it
    /// responder's `X` falls to the floor's *takeout* reading — with a 22+
    /// opener behind it. **Default on** — measured vs BBA 2/1 (204.8k
    /// boards/arm/vul): plain DD +1.86/+2.79 IMPs/fired NV/vul,
    /// perfect-defense +2.00/+2.93; all four cells' CIs exclude 0 (~0.05%
    /// fired).
    static STRONG_TWO_COMPETITION: Cell<bool> = const { Cell::new(true) };
}

/// Author our contested weak twos for books built *after* this call
/// (thread-local)
///
/// Default off (`--ns-weak-two-comp` in `bba-gen` for the on arm).
pub fn set_weak_two_competition(on: bool) {
    WEAK_TWO_COMPETITION.with(|cell| cell.set(on));
}

/// Whether the contested weak-two package is engaged
fn weak_two_competition() -> bool {
    WEAK_TWO_COMPETITION.with(Cell::get)
}

/// Author our contested strong 2♣ for books built *after* this call
/// (thread-local)
///
/// **Default on** (`--no-ns-strong-two-comp` in `bba-gen` for the off arm).
pub fn set_strong_two_competition(on: bool) {
    STRONG_TWO_COMPETITION.with(|cell| cell.set(on));
}

/// Whether the contested strong-2♣ package is engaged
fn strong_two_competition() -> bool {
    STRONG_TWO_COMPETITION.with(Cell::get)
}

/// Responder after our weak two in `our` and their takeout double
///
/// The uncontested responses ride unchanged — Ogust `2NT` still asks, raises
/// stay preemptive (RONF), the forcing new suits survive — plus a business
/// redouble: 13+ values without the 2-card fit Ogust wants (a fit-and-values
/// hand still prefers the ask, whose weight sits above).
fn weak_two_doubled_responder(our: Suit) -> Rules {
    weak_twos::responses(our)
        .rule(Call::Redouble, 180, hcp(13..))
        .alert(WEAK_TWO_XX)
}

/// Responder after our weak two in `our` and their overcall (≤ 3♠)
///
/// Ogust survives when `2NT` is still available (their overcall ≤ 2♠); `X` is
/// a penalty-leaning values double (the floor's settle machinery answers it —
/// sit on a stack, pull with shape); the raises stay preemptive at *any*
/// strength — blocking, not inviting (RONF).
fn weak_two_overcalled_responder(our: Suit) -> Rules {
    let trump = Strain::from(our);
    Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            200,
            min_level_is(2, Strain::Notrump) & len(our, 2..) & points(14..),
        )
        .alert(CONTESTED_OGUST)
        .rule(Call::Double, 160, hcp(11..))
        .rule(
            Bid::new(3, trump),
            130,
            min_level_is(3, trump) & len(our, 3..),
        )
        .rule(Bid::new(4, trump), 125, len(our, 4..))
        .rule(Call::Pass, 0, hcp(0..))
}

// ---------------------------------------------------------------------------
// Section 8: our contested strong 2♣ (`set_strong_two_competition`)
// ---------------------------------------------------------------------------

/// Responder after our strong 2♣ and their overcall
///
/// Natural game-forcing new suits keep the uncontested positive shape (5+
/// suit to two top honors, 8+), legality-anchored so exactly one rung fires;
/// `2NT`/`3NT` is the balanced positive with their suit stopped; `X` shows
/// "cards" (6+ HCP, penalty-leaning opposite 22+ — shadowing the floor's
/// *takeout* reading, the bug this table fixes); **Pass is waiting**, safe
/// because opener's reopening node below never sells out.
fn strong_two_overcalled_responder() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            130,
            min_level_is(2, Strain::Notrump) & hcp(8..) & balanced() & stopper_in_their_suits(),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            130,
            min_level_is(3, Strain::Notrump) & hcp(8..) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 120, hcp(6..))
        .rule(Call::Pass, 50, hcp(0..));
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(x);
        for level in 2..=3u8 {
            rules = rules.rule(
                Bid::new(level, strain),
                150,
                min_level_is(level, strain) & len(x, 5..) & top_honors(x, 2..) & points(8..),
            );
        }
    }
    rules
}

/// Opener's forced reopening after `2♣ (overcall) - -`
///
/// A 22+ hand never sells out to an overcall: natural 5+ suit rebids
/// (legality-anchored rungs), notrump with their suit stopped, and a "cards"
/// double as the finite catch-all — partner decides whether to defend.
fn strong_two_reopening() -> Rules {
    let mut rules = Rules::new()
        .rule(
            Bid::new(2, Strain::Notrump),
            120,
            min_level_is(2, Strain::Notrump) & balanced() & stopper_in_their_suits(),
        )
        .rule(
            Bid::new(3, Strain::Notrump),
            120,
            min_level_is(3, Strain::Notrump) & balanced() & stopper_in_their_suits(),
        )
        .rule(Call::Double, 40, hcp(0..));
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(x);
        for level in 2..=3u8 {
            rules = rules.rule(
                Bid::new(level, strain),
                100,
                min_level_is(level, strain) & len(x, 5..),
            );
        }
    }
    rules
}

/// Section 7 as a row package: our contested weak twos
/// ([`set_weak_two_competition`][super::set_weak_two_competition], default off)
///
/// Their double: responder's first call at the deeper `2M (X)` node (business
/// `XX` riding on the uncontested responses), everything deeper systems-on.
/// Their overcall (≤ `3♠`): responder's direct action, and a targeted rebase so
/// an Ogust `2NT` bid over the overcall still gets opener's undisturbed
/// five-rung answer.
pub(super) fn weak_two_competition_package() -> Package {
    Package {
        name: "weak-two-competition",
        gate: |_| weak_two_competition(),
        entries: |_| {
            let two_nt = call(2, Strain::Notrump);
            let mut entries = Vec::new();
            for our in [Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let trump = Strain::from(our);
                let key = format!("P* 2{trump}");

                entries.extend(rows_of(
                    Pattern::table(&format!("{key} (X)")),
                    weak_two_doubled_responder(our),
                ));
                entries.push(rebase(Pattern::first(&key, "X"), ReplaceNext(Call::Pass)));

                entries.extend(rows_of(
                    Pattern::up_to(&key, "3♠"),
                    weak_two_overcalled_responder(our),
                ));
                // The guard admits any overcall below `2NT`, of which a `2♠`
                // opening has none — the sample stays inside the guard (which
                // never checks legality) rather than inside the auction.
                let sample = if our == Suit::Hearts {
                    "(2♠) 2NT"
                } else {
                    "(2♥) 2NT"
                };
                entries.push(rebase(
                    Pattern::guarded(
                        &key,
                        sample,
                        described_guard(
                            "(overcall <2NT) 2NT …",
                            guard(move |_: &Context<'_>, s: &[Call]| {
                                matches!(s.first(), Some(&Call::Bid(b)) if b < Bid::new(2, Strain::Notrump))
                                    && s.get(1) == Some(&two_nt)
                            }),
                        ),
                    ),
                    ReplaceNext(Call::Pass),
                ));
            }
            entries
        },
    }
}

/// Section 8 as a row package: our contested strong `2♣`
/// ([`set_strong_two_competition`][super::set_strong_two_competition], default
/// on)
///
/// Their double steals no room → systems on wholesale; their overcall gets
/// responder's natural-GF / values-`X` / waiting-pass table, backed by opener's
/// forced reopening in the pass-out seat.
pub(super) fn strong_two_competition_package() -> Package {
    Package {
        name: "strong-two-competition",
        gate: |_| strong_two_competition(),
        entries: |_| {
            const OPEN: &str = "P* 2♣";
            let mut entries = vec![rebase(Pattern::first(OPEN, "X"), ReplaceNext(Call::Pass))];
            // Their overcall — any bid over 2♣, the suit columns and the
            // notrump columns each spelled once; ascension does the rest.
            for column in ["(jx)", "(jN)"] {
                entries.extend(expand(
                    &format!("{OPEN} {column}"),
                    |_| true,
                    |_| strong_two_overcalled_responder(),
                ));
            }
            // The pass-out seat: opener's forced reopening.
            for column in ["(jx) - -", "(jN) - -"] {
                entries.extend(expand(
                    &format!("{OPEN} {column}"),
                    |_| true,
                    |_| strong_two_reopening(),
                ));
            }
            entries
        },
    }
}

/// The retired guarded wiring of [`strong_two_competition_package`], kept as
/// the resolution-equivalence oracle for `converted_packages_match_legacy`
#[cfg(test)]
pub(super) fn strong_two_competition_package_legacy() -> Package {
    Package {
        name: "strong-two-competition",
        gate: |_| strong_two_competition(),
        entries: |_| {
            const OPEN: &str = "P* 2♣";
            let mut entries = vec![rebase(Pattern::first(OPEN, "X"), ReplaceNext(Call::Pass))];
            entries.extend(rows_of(
                Pattern::guarded(
                    OPEN,
                    "(2♦)",
                    described_guard(
                        "(overcall)",
                        guard(|_: &Context<'_>, s: &[Call]| matches!(s, [Call::Bid(_)])),
                    ),
                ),
                strong_two_overcalled_responder(),
            ));
            entries.extend(rows_of(
                Pattern::guarded(
                    OPEN,
                    "(2♦) - -",
                    described_guard(
                        "(overcall) - -",
                        guard(|_: &Context<'_>, s: &[Call]| {
                            matches!(s, [Call::Bid(_), Call::Pass, Call::Pass])
                        }),
                    ),
                ),
                strong_two_reopening(),
            ));
            entries
        },
    }
}

#[cfg(test)]
mod tests;
