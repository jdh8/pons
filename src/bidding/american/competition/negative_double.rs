//! The negative double — Sputnik, Cachalot, and opener's answers
//!
//! [`NegativeDoubleShape`] picks what the double promises.  Cachalot adds the
//! transfer answers and the contested `X` ([`set_cachalot_contested_x`]); the
//! Sputnik residual is the leftover shape opener must still answer.

use super::*;

/// The negative-double school over our **minor** openings
/// ([`set_negative_double_shape`]; the major-opening double — 4+ in the other
/// major, 8+ — is common to all three)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NegativeDoubleShape {
    /// Both majors 4-4+ at 8+ regardless of the overcall — the shipped rule
    BothMajors,
    /// Modern standard (BWS/Cohen): over `(1♦)` both majors 4-4+ at 6+; over
    /// `(1♥)` **exactly** four spades at 6+ (with 5+ bid the free `1♠`); over
    /// `(1♠)` 4+ hearts at 8+; over a 2-level minor both majors at 8+.
    /// Implies the free bids (the exactly-4 double is unsound without the
    /// 5-card outlet).
    Modern,
    /// Cachalot — transfer Walsh in competition (Lebel–Soulet lineage): over
    /// `(1♦)`/`(1♥)` the 1-level calls rotate — `X` = 4+ in the adjacent
    /// major, `1♥` = 4+ spades, `1♠` = the residual takeout hand (≤3 in each
    /// shown-able major). Opener's 1-level completion shows **exactly three**
    /// trumps, forcing; the raise shows four. Natural from `(1♠)` up (the
    /// Modern rules apply there). Implies the free bids.
    Cachalot,
    /// Sputnik (Roth–Stone original): the double is the **residual** — it
    /// *denies* a 4-card major biddable at the 1-level, showing 7+ with the
    /// biddable majors held to ≤3; the free 1-level major shows a natural 4+
    /// (not Modern's 5+, since the double no longer carries the exactly-four
    /// hand). Over `(1♦)`: `X` = ≤3 in both majors, `1♥`/`1♠` = 4+ natural;
    /// over `(1♥)`: `X` = ≤3 spades, `1♠` = 4+. From `(1♠)` up and over a
    /// 2-level minor the Modern rules apply (no 1-level major to deny). Implies
    /// the free bids.
    Sputnik,
}

thread_local! {
    /// Which negative-double school the minor openings play. Default
    /// `Modern` — **shipped default-on 2026-07-10** with the forcing free-bid
    /// answers: plain +0.0213 NV / +0.0074 vul (CI>0), sd arbiter +0.42/+0.29
    /// per divergent board (CI>0, sd>plain, disclosure-corrected); the vul-PD
    /// −0.026 is the perfect-defense doubling artifact on thin vul games.
    static NEGATIVE_DOUBLE_SHAPE: Cell<NegativeDoubleShape> =
        const { Cell::new(NegativeDoubleShape::Modern) };
}

/// Choose the negative-double school for books built *after* this call
/// (thread-local)
///
/// Default [`NegativeDoubleShape::Modern`] — shipped default-on; pass
/// `--ns-negative-double-shape both-majors` in `bba-gen` for the old rule.
pub fn set_negative_double_shape(shape: NegativeDoubleShape) {
    NEGATIVE_DOUBLE_SHAPE.with(|cell| cell.set(shape));
}

/// The negative-double school in effect
pub(super) fn negative_double_shape() -> NegativeDoubleShape {
    NEGATIVE_DOUBLE_SHAPE.with(Cell::get)
}

thread_local! {
    /// Whether opener's contested-X answer is authored (Cachalot only). Default
    /// on; the off state restores the floored continuation for the A/B.
    static CACHALOT_CONTESTED_X: Cell<bool> = const { Cell::new(true) };
}

/// Author opener's raise of a Cachalot `X` transfer when LHO competes over it
/// (thread-local, Cachalot only)
///
/// Default on — `--no-ns-cachalot-contested-x` in `bba-gen` restores the old
/// floored continuation.
pub fn set_cachalot_contested_x(on: bool) {
    CACHALOT_CONTESTED_X.with(|cell| cell.set(on));
}

/// Whether opener's contested-X answer is engaged
fn cachalot_contested_x() -> bool {
    CACHALOT_CONTESTED_X.with(Cell::get)
}

/// The negative doubler's rebid after opener answers (`FreeBidStyle::
/// Negative`): a new suit is the strong hand the capped free bid could not
/// carry — **forcing to game**
///
/// Also claims the *ordinary* doubler's second turn (this node cannot tell
/// the two apart): raise opener's answer with a real fit, `2NT` with a
/// stopper and 10–12, else the `Pass` catch-all drops the minimum answers.
pub(super) fn negative_doubler_rebid(opening: Suit) -> Rules {
    let o = opening;
    let mut rules = Rules::new();
    // The FG clarification: the long suit the double concealed.
    for z in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if z == o {
            continue;
        }
        let zs = Strain::from(z);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, zs),
                130,
                min_level_is(lvl, zs)
                    & !partner_suit_is(z)
                    & !they_bid(zs)
                    & len(z, 5..)
                    & points(12..),
            );
        }
    }
    // Raise opener's answer with four trumps and invitational values.
    for y in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let ys = Strain::from(y);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, ys),
                100,
                partner_suit_is(y) & min_level_is(lvl, ys) & support(4..) & points(8..),
            );
        }
    }
    rules
        .rule(
            Bid::new(2, Strain::Notrump),
            90,
            min_level_is(2, Strain::Notrump) & stopper_in_their_suits() & hcp(10..=12),
        )
        .rule(Call::Pass, 20, hcp(0..))
}

/// Opener's answer after `1M – (2m) – X – P` (partner doubled a minor overcall)
///
/// Shows four-card length in the other major or rebids the opening major on five.
/// No Pass rule — the double is forcing.
fn answer_neg_double_of_minor(opening_major: Suit) -> Rules {
    let m = Strain::from(opening_major);
    let other = if opening_major == Suit::Hearts {
        Suit::Spades
    } else {
        Suit::Hearts
    };
    let other_strain = Strain::from(other);
    Rules::new()
        .rule(Bid::new(2, other_strain), 100, len(other, 3..))
        .rule(Bid::new(2, m), 50, len(opening_major, 5..))
}

/// Opener's answer to a Cachalot rotation showing 4+ in `shown` after our
/// minor opening and their `over` overcall
///
/// The memo's ladder: raise to two with four trumps; **complete at the one
/// level with exactly three** (forcing one round — the convention's payoff);
/// name the fourth suit naturally; `1NT` with their suit stopped (it does not
/// deny three trumps — the completion simply outweighs it); rebid a 5-card
/// opening minor; the low-weight `1NT` is the finite catch-all (the rotation
/// is forcing).
fn cachalot_answer(opening: Suit, over: Suit, shown: Suit) -> Rules {
    let m = Strain::from(shown);
    let mut rules = Rules::new()
        .rule(Bid::new(2, m), 130, len(shown, 4..))
        .rule(Bid::new(1, m), 120, len(shown, 3..=3))
        .alert(CACHALOT_THREE);
    if shown == Suit::Hearts {
        // The fourth suit at the one level (spades, when hearts were shown
        // over their (1♦)).
        rules = rules.rule(Bid::new(1, Strain::Spades), 110, len(Suit::Spades, 4..));
    }
    rules
        .rule(Bid::new(1, Strain::Notrump), 100, stopper_in(over))
        .rule(Bid::new(2, Strain::from(opening)), 90, len(opening, 5..))
        .rule(Bid::new(1, Strain::Notrump), 20, hcp(0..))
}

/// Opener's answer to the Cachalot takeout `1♠` — as over a Sputnik double
///
/// Partner denied four cards in every rotation major, so there is no fit to
/// hunt: `1NT` with their suit stopped, a natural 5-card rebid, else the
/// cheapest rebid of the opening minor as the finite catch-all.
fn cachalot_takeout_answer(opening: Suit, over: Suit) -> Rules {
    let o = Strain::from(opening);
    Rules::new()
        .rule(Bid::new(1, Strain::Notrump), 100, stopper_in(over))
        .rule(Bid::new(2, o), 90, len(opening, 5..))
        .rule(Bid::new(2, o), 20, hcp(0..))
}

/// Opener's answer to a Cachalot `X` transfer once LHO has competed — hearts
/// over `(1♦)`, spades over `(1♥)`.
///
/// The pass-out completion is authored separately (and stays right-sided). This
/// is the *contested* branch, which the floor otherwise misjudges (the measured
/// `X·wrapped` leak): a rebase to the natural auction can't help because the
/// natural continuation is itself floored, so opener's raise is authored
/// directly here.  Opener knows partner holds 4+ `shown`, so it raises the fit
/// at the level the competition forces — `last` fixes the cheapest legal naming
/// of the major, four-card support jumps a level — else passes to defend.  The
/// gain is reaching the major games Modern's natural response finds and the
/// bare double misses.
///
/// Their intervention is a row column, so `last` is known when the book is
/// built; a redouble leaves their overcall as the last bid.  These rules
/// deliberately carry no `.alert(…)`: the raise is natural on real support, and
/// `authored_effect` publishes a reading only for an *alerted* rule under the
/// shipped [`ReadingScope`][super::super::inference::ReadingScope].  That is
/// what let this table stop being a `classified(` closure without opening a
/// reading channel — adding an alert here is a reading change, so measure it.
fn cachalot_x_contested_answer(shown: Suit, last: Bid) -> Rules {
    let m = Strain::from(shown);
    // The cheapest legal level to name our major above their last bid; when
    // they bid our major the `+1` raises past it, still gated on real support
    // by `len` below.
    let level = if m > last.strain {
        last.level.get()
    } else {
        last.level.get() + 1
    };
    let mut rules = Rules::new();
    if level < 7 {
        rules = rules.rule(Bid::new(level + 1, m), 130, len(shown, 4..));
    }
    if level <= 7 {
        rules = rules.rule(Bid::new(level, m), 120, len(shown, 3..));
    }
    rules.rule(Call::Pass, 20, hcp(0..))
}

/// Section 9 as a row package: opener's Cachalot answers
///
/// The rotated calls are forcing; each gets its completion table at the deeper
/// `[1m, <their 1-level overcall>]` key.
pub(super) fn cachalot_package() -> Package {
    Package {
        name: "cachalot-answer",
        gate: || negative_double_shape() == NegativeDoubleShape::Cachalot,
        entries: || {
            // (1♦) over 1♣: X shows hearts, 1♥ shows spades, 1♠ is the takeout.
            let over_diamond = "P* 1♣ (1♦)";
            let mut entries = rows_of(
                Pattern::after(over_diamond, "X (P)"),
                cachalot_answer(Suit::Clubs, Suit::Diamonds, Suit::Hearts),
            );
            entries.extend(rows_of(
                Pattern::after(over_diamond, "1♥ (P)"),
                cachalot_answer(Suit::Clubs, Suit::Diamonds, Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::after(over_diamond, "1♠ (P)"),
                cachalot_takeout_answer(Suit::Clubs, Suit::Diamonds),
            ));

            // (1♥) over 1♣/1♦: X shows spades, 1♠ is the takeout.
            for opening in [Suit::Clubs, Suit::Diamonds] {
                let key = format!("P* 1{} (1♥)", Strain::from(opening));
                entries.extend(rows_of(
                    Pattern::after(&key, "X (P)"),
                    cachalot_answer(opening, Suit::Hearts, Suit::Spades),
                ));
                entries.extend(rows_of(
                    Pattern::after(&key, "1♠ (P)"),
                    cachalot_takeout_answer(opening, Suit::Hearts),
                ));
            }

            // Contested X: LHO competed over the transfer, so the pass-out
            // completions above don't fire and opener would fall to the floor
            // (the measured X·wrapped leak).  Author opener's raise of the
            // shown major — hearts over (1♦), spades over (1♥) — one column
            // per intervention: their suit bid, their notrump bid, their
            // redouble.  The [X, P] pass-out is shadowed by the completions
            // above, and deeper continuations fall to the floor as before.
            if cachalot_contested_x() {
                let contested = |key: &str, shown: Suit, overcall: Bid| {
                    let mut rows = expand(
                        &format!("{key} X (jz)"),
                        |_: &Bindings| true,
                        move |b: &Bindings| {
                            cachalot_x_contested_answer(
                                shown,
                                Bid::new(b.level('j').get(), b.suit('z').into()),
                            )
                        },
                    );
                    rows.extend(expand(
                        &format!("{key} X (jN)"),
                        |_: &Bindings| true,
                        move |b: &Bindings| {
                            cachalot_x_contested_answer(
                                shown,
                                Bid::new(b.level('j').get(), Strain::Notrump),
                            )
                        },
                    ));
                    rows.extend(rows_of(
                        Pattern::node(&format!("{key} X (XX)")),
                        cachalot_x_contested_answer(shown, overcall),
                    ));
                    rows
                };
                entries.extend(contested(
                    over_diamond,
                    Suit::Hearts,
                    Bid::new(1, Strain::Diamonds),
                ));
                for opening in [Suit::Clubs, Suit::Diamonds] {
                    entries.extend(contested(
                        &format!("P* 1{} (1♥)", Strain::from(opening)),
                        Suit::Spades,
                        Bid::new(1, Strain::Hearts),
                    ));
                }
            }
            entries
        },
    }
}

/// The retired guarded wiring of [`cachalot_package`]'s contested-`X` answers,
/// kept as the resolution-equivalence oracle for `converted_packages_match_legacy`
///
/// The pass-out completions ride along verbatim so the two packages are
/// comparable as wholes.
#[cfg(test)]
pub(super) fn cachalot_package_legacy() -> Package {
    /// The context-reading twin of [`cachalot_x_contested_answer`], which used
    /// to compute the raise level from `last_bid` at classification time
    fn contested_answer(shown: Suit) -> impl Classifier {
        classifier(move |hand, context| {
            let rules = match context.last_bid() {
                Some(bid) => cachalot_x_contested_answer(shown, bid),
                None => Rules::new(),
            };
            rules.classify(hand, context)
        })
    }

    Package {
        name: "cachalot-answer",
        gate: || negative_double_shape() == NegativeDoubleShape::Cachalot,
        entries: || {
            let over_diamond = "P* 1♣ (1♦)";
            let mut entries = rows_of(
                Pattern::after(over_diamond, "X (P)"),
                cachalot_answer(Suit::Clubs, Suit::Diamonds, Suit::Hearts),
            );
            entries.extend(rows_of(
                Pattern::after(over_diamond, "1♥ (P)"),
                cachalot_answer(Suit::Clubs, Suit::Diamonds, Suit::Spades),
            ));
            entries.extend(rows_of(
                Pattern::after(over_diamond, "1♠ (P)"),
                cachalot_takeout_answer(Suit::Clubs, Suit::Diamonds),
            ));
            for opening in [Suit::Clubs, Suit::Diamonds] {
                let key = format!("P* 1{} (1♥)", Strain::from(opening));
                entries.extend(rows_of(
                    Pattern::after(&key, "X (P)"),
                    cachalot_answer(opening, Suit::Hearts, Suit::Spades),
                ));
                entries.extend(rows_of(
                    Pattern::after(&key, "1♠ (P)"),
                    cachalot_takeout_answer(opening, Suit::Hearts),
                ));
            }
            if cachalot_contested_x() {
                let x_intervention = || {
                    described_guard(
                        "X (their intervention) -",
                        guard(
                            |_: &Context<'_>, s: &[Call]| matches!(s, [Call::Double, c] if !matches!(c, Call::Pass)),
                        ),
                    )
                };
                entries.push(classified(
                    Pattern::guarded(over_diamond, "X (2♦)", x_intervention()),
                    contested_answer(Suit::Hearts),
                ));
                for opening in [Suit::Clubs, Suit::Diamonds] {
                    entries.push(classified(
                        Pattern::guarded(
                            &format!("P* 1{} (1♥)", Strain::from(opening)),
                            "X (2♦)",
                            x_intervention(),
                        ),
                        contested_answer(Suit::Spades),
                    ));
                }
            }
            entries
        },
    }
}

/// Section 9b as a row package: opener's answers to the Sputnik residual double
///
/// The double *denies* a biddable major, so — unlike a classic negative double
/// — opener must NOT raise a major: the floor's "negative double = the unbid
/// major" instinct is exactly inverted here and would jump the phantom denied
/// suit into a doubled game (the measured leak).  [`cachalot_takeout_answer`]
/// bids NT or the opening minor naturally instead.  Over `(1♠)` or a 2-minor,
/// Sputnik's double is Modern's major-showing one, which the floor reads
/// correctly — left to it.
pub(super) fn sputnik_residual_answer_package() -> Package {
    Package {
        name: "sputnik-residual-answer",
        gate: || negative_double_shape() == NegativeDoubleShape::Sputnik,
        entries: || {
            // (1♦) over 1♣: X = ≤3 in both majors — no fit to hunt.
            let mut entries = rows_of(
                Pattern::after("P* 1♣ (1♦)", "X (P)"),
                cachalot_takeout_answer(Suit::Clubs, Suit::Diamonds),
            );
            // (1♥) over 1♣/1♦: X = ≤3 spades.
            for opening in [Suit::Clubs, Suit::Diamonds] {
                entries.extend(rows_of(
                    Pattern::after(&format!("P* 1{} (1♥)", Strain::from(opening)), "X (P)"),
                    cachalot_takeout_answer(opening, Suit::Hearts),
                ));
            }
            entries
        },
    }
}

/// Section 4 as a row package: opener answers partner's negative double of a
/// two-level minor overcall — one exact node per (major, minor) column
pub(super) fn answer_negative_double_package() -> Package {
    Package {
        name: "answer-negative-double-of-minor",
        gate: || true,
        entries: || {
            expand(
                "P* 1M (2m) X -",
                |_| true,
                |bindings| answer_neg_double_of_minor(bindings.suit('M')),
            )
        },
    }
}

/// The retired guarded wiring of [`answer_negative_double_package`], kept as
/// the resolution-equivalence oracle for `converted_packages_match_legacy`
#[cfg(test)]
pub(super) fn answer_negative_double_package_legacy() -> Package {
    Package {
        name: "answer-negative-double-of-minor",
        gate: || true,
        entries: || {
            [Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|major| {
                    rows_of(
                        Pattern::guarded(
                            &format!("P* 1{}", Strain::from(major)),
                            "(2♦) X (P)",
                            described_guard(
                                "2♣/2♦ X -",
                                guard(|_: &Context<'_>, suffix: &[Call]| {
                                    matches!(
                                        suffix,
                                        [Call::Bid(b), Call::Double, Call::Pass]
                                            if b.level.get() == 2
                                                && (b.strain == Strain::Clubs
                                                    || b.strain == Strain::Diamonds)
                                    )
                                }),
                            ),
                        ),
                        answer_neg_double_of_minor(major),
                    )
                })
                .collect()
        },
    }
}

#[cfg(test)]
mod tests;
