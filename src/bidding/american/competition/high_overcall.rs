//! Their jump and three-level overcalls
//!
//! The shipped exact tables stop at `2♠`; this guarded agreement covers
//! everything above, where responder has one round and no room.  Opt-in via
//! [`set_high_overcall_responses`].

use super::lebensohl::unbid_major;
use super::*;

thread_local! {
    /// Whether responder's structure over their jump / 3-level overcalls
    /// (`2NT < bid ≤ 3♠`) is authored — the shipped direct-seat package stops
    /// at 2♠ (one exact table per overcall through there, plus their 1NT) and
    /// everything higher falls to the floor.  Default off while the A/B runs.
    static HIGH_OVERCALL_RESPONSES: Cell<bool> = const { Cell::new(false) };
}

/// Author responder's structure over their 3-level overcalls for books built
/// *after* this call (thread-local)
///
/// Default off (`--ns-high-overcall` in `bba-gen` for the on arm).
pub fn set_high_overcall_responses(on: bool) {
    HIGH_OVERCALL_RESPONSES.with(|cell| cell.set(on));
}

/// Whether the 3-level-overcall package is engaged
fn high_overcall_responses() -> bool {
    HIGH_OVERCALL_RESPONSES.with(Cell::get)
}

/// Responder after our opening `o` and their suit overcall in `2NT < bid ≤ 3♠`
///
/// The 4-level cue is deliberately dropped (a game-forcing raise doubles
/// first or blasts game), which keeps the Section-4b/4c cue-raise guards
/// untouched. Their `2NT` is excluded by the guard — in the NATURAL family a
/// `(2NT)` jump over our opening is a two-suiter, never natural.
fn over_their_high_overcall(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);
    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };

    let mut rules = Rules::new().rule(
        Bid::new(3, Strain::Notrump),
        170,
        points(13..) & stopper_in_their_suits(),
    );

    // Forcing 3-level new suits — strength does the forcing.
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let xs = Strain::from(x);
        rules = rules.rule(
            Bid::new(3, xs),
            145,
            min_level_is(3, xs) & !they_bid(xs) & len(x, 5..) & points(13..),
        );
    }

    // The negative double, strength scaled to the level.
    rules = if is_major {
        let om = unbid_major(o).expect("a major opening has an unbid major");
        let om_strain = Strain::from(om);
        rules
            .rule(
                Call::Double,
                100,
                len(om, 4..) & hcp(10..) & !they_bid(om_strain),
            )
            .alert(NEGATIVE_DOUBLE)
    } else {
        rules
            .rule(
                Call::Double,
                100,
                (len(Suit::Hearts, 4..) | len(Suit::Spades, 4..)) & hcp(10..),
            )
            .alert(NEGATIVE_DOUBLE)
    };

    // Raises: competitive at the 3 level, game with real extras (majors) or
    // preemptive on shape (minors).
    rules = rules.rule(
        Bid::new(3, o_strain),
        130,
        min_level_is(3, o_strain) & len(o, raise_min..) & points(6..),
    );
    rules = if is_major {
        rules.rule(Bid::new(4, o_strain), 125, len(o, 4..) & points(11..))
    } else {
        rules.rule(Bid::new(4, o_strain), 125, len(o, 5..) & points(..=9))
    };

    rules.rule(Call::Pass, 0, hcp(0..))
}

/// Opener's forced answer to partner's negative double of a 3-level overcall
///
/// Bid an unbid major at the cheapest legal level with four; `3NT` with their
/// suit stopped; rebid a 6-card opening suit; else the 3-card major
/// tolerance, with a low-weight `3NT` as the finite catch-all (the double is
/// forcing — `3NT` is always legal under the ≤3♠ guard).
fn answer_high_neg_double(opening: Suit) -> Rules {
    let o_strain = Strain::from(opening);
    let mut rules = Rules::new();

    for m in [Suit::Hearts, Suit::Spades] {
        if m == opening {
            continue;
        }
        let ms = Strain::from(m);
        rules = rules
            .rule(
                Bid::new(3, ms),
                120,
                min_level_is(3, ms) & len(m, 4..) & !they_bid(ms),
            )
            .rule(
                Bid::new(4, ms),
                110,
                min_level_is(4, ms) & len(m, 4..) & !they_bid(ms),
            );
    }
    rules = rules
        .rule(Bid::new(3, Strain::Notrump), 100, stopper_in_their_suits())
        .rule(
            Bid::new(3, o_strain),
            90,
            min_level_is(3, o_strain) & len(opening, 6..),
        )
        .rule(
            Bid::new(4, o_strain),
            85,
            min_level_is(4, o_strain) & len(opening, 6..),
        );
    for m in [Suit::Hearts, Suit::Spades] {
        if m == opening {
            continue;
        }
        let ms = Strain::from(m);
        rules = rules
            .rule(
                Bid::new(3, ms),
                30,
                min_level_is(3, ms) & len(m, 3..) & !they_bid(ms),
            )
            .rule(
                Bid::new(4, ms),
                25,
                min_level_is(4, ms) & len(m, 3..) & !they_bid(ms),
            );
    }
    rules.rule(Bid::new(3, Strain::Notrump), 15, hcp(0..))
}

/// Section 10 as a row package: their jump / 3-level suit overcalls
/// ([`set_high_overcall_responses`][super::set_high_overcall_responses],
/// default off)
///
/// A guarded entry at `[1x]` — its bid range `(2NT, 3♠]` sits above the
/// shipped per-overcall exact nodes (which stop at 2♠), so nothing races it.
/// Their `(2NT)` and their 3-level cue of our own suit are
/// excluded (the first is a two-suiter, the second is rare enough for the
/// floor).
pub(super) fn high_overcall_package() -> Package {
    Package {
        name: "high-overcall-responses",
        gate: high_overcall_responses,
        entries: || {
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let key = format!("P* 1{}", Strain::from(opening));
                // `(2NT, 3♠]` in suits is exactly the four 3-level suit bids;
                // their 3-level cue of our own suit stays excluded.
                let not_ours = move |bindings: &Bindings| bindings.suit('x') != opening;
                entries.extend(expand(&format!("{key} (3x)"), not_ours, move |_| {
                    over_their_high_overcall(opening)
                }));
                entries.extend(expand(&format!("{key} (3x) X -"), not_ours, move |_| {
                    answer_high_neg_double(opening)
                }));
            }
            entries
        },
    }
}

/// The retired guarded wiring of [`high_overcall_package`], kept as the
/// resolution-equivalence oracle for `converted_packages_match_legacy`
#[cfg(test)]
pub(super) fn high_overcall_package_legacy() -> Package {
    Package {
        name: "high-overcall-responses",
        gate: high_overcall_responses,
        entries: || {
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let o_strain = Strain::from(opening);
                let key = format!("P* 1{o_strain}");
                // The sample overcall must be a suit other than our own.
                let over = if opening == Suit::Diamonds {
                    "(3♥)"
                } else {
                    "(3♦)"
                };
                let in_range = move |bid: &Bid| {
                    *bid > Bid::new(2, Strain::Notrump)
                        && *bid <= Bid::new(3, Strain::Spades)
                        && bid.strain.is_suit()
                        && bid.strain != o_strain
                };
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        over,
                        described_guard(
                            "(2NT < overcall ≤3♠)",
                            guard(move |_: &Context<'_>, s: &[Call]| {
                                matches!(s, [Call::Bid(b)] if in_range(b))
                            }),
                        ),
                    ),
                    over_their_high_overcall(opening),
                ));
                entries.extend(rows_of(
                    Pattern::guarded(
                        &key,
                        &format!("{over} X (P)"),
                        described_guard(
                            "(2NT < overcall ≤3♠) X -",
                            guard(move |_: &Context<'_>, s: &[Call]| {
                                matches!(s, [Call::Bid(b), Call::Double, Call::Pass] if in_range(b))
                            }),
                        ),
                    ),
                    answer_high_neg_double(opening),
                ));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
