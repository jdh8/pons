//! Defending their weak two
//!
//! Takeout double, natural `2NT`, natural suit overcalls, and the cue — each
//! with its own point band, because a weak two both steals room and bounds
//! their hand.

use super::overcall::TakeoutSupport;
use super::*;

/// Our action over their weak-two opening
///
/// A weak two steals a level of room, so the toolkit is leaner than over a
/// one-bid: a takeout double (the workhorse), a natural 2NT overcall (15–18
/// with a stopper), and natural suit overcalls at the cheapest legal level.
/// Strong hands (17+) still double first, planning to bid again.
///
/// Overcall levels are derived from `their_opening`, so the suits higher than
/// theirs sit at the opening level and the lower ones one rung up — over 2♥, a
/// spade overcall is 2♠ but a club overcall is 3♣.
///
/// # Panics
///
/// Panics if `their_opening` is a notrump bid; pass a suit opening.
#[must_use]
pub fn defense_to_weak_two(their_opening: Bid, agreements: &Agreements) -> Rules {
    let theirs = their_opening.strain;
    let level = their_opening.level.get();

    let (nt_lo, nt_hi) = agreements.defense.weak_two_notrump_points;
    let mut rules = Rules::new();
    // The wide arm is `balanced() | (two_notrump_wide_shape() & two top honours in their
    // suit)`: the extra shapes are the 6322 and long-minor hands, which have a
    // trick source but one fewer stopper-guarded entry than a flat hand, so they
    // are asked for a *real* holding rather than the crisp Jxxx that
    // `stopper_in_their_suits` accepts.  Balanced hands keep today's gate, so the
    // knob only ever adds.
    rules = if agreements.defense.weak_two_notrump_shape {
        let theirs_suit = theirs.suit().expect("weak two is a suit bid");
        rules.rule(
            Bid::new(2, Strain::Notrump),
            150,
            hcp(nt_lo..=nt_hi)
                & stopper_in_their_suits()
                & (balanced() | (two_notrump_wide_shape() & top_honors(theirs_suit, 2..))),
        )
    } else {
        rules.rule(
            Bid::new(2, Strain::Notrump),
            150,
            hcp(nt_lo..=nt_hi) & balanced() & stopper_in_their_suits(),
        )
    };

    // 12+ takeout double, optionally gated on unbid-suit support (see
    // `agreements.defense.takeout_support`); the 17+ tier catches off-shape strong hands.
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

    // The pass gate documents the 17+ tier's complement, exactly as
    // `defense_to_suit` does — "strong hands double first regardless".
    // Byte-identical to the old `hcp(0..)` catch-all: below the floor both score
    // 0.0, above it the shape-free tier is finite at weight 1.2 and always
    // outscores a weight-0 pass.  Authored so the pass reading
    // (`set_pass_reading`) has a band to project; the ⊤ census found the
    // direct-seat pass over their weak two reading *nothing* on all five axes.
    rules = rules
        .rule(Call::Double, 120, points(17..))
        .alert(TAKEOUT_DOUBLE);
    rules = if agreements.defense.weak_two_pass_gate {
        rules.rule(Call::Pass, 0, points(..17))
    } else {
        rules.rule(Call::Pass, 0, hcp(0..))
    };

    // Natural overcalls: five-card suit, 10–16 points, at the cheapest legal level.
    // The jump one rung above is the same call with a trick more of hand — six-plus
    // cards and three more points — but only where it still fits under 3NT; every
    // other jump is at the four level, where Leaping Michaels already lives.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        let strain = Strain::from(suit);
        if strain != theirs {
            let overcall_level = if strain > theirs { level } else { level + 1 };
            // The extra trick has to be paid for: the band is graded by the level
            // the overcall lands on (`set_weak_two_overcall_points`; default flat).
            let (two_lo, two_hi, three_lo, three_hi) = agreements.defense.weak_two_overcall_points;
            let (lo, hi) = if overcall_level <= 2 {
                (two_lo, two_hi)
            } else {
                (three_lo, three_hi)
            };
            // Red-vs-white is where a light overcall gets punished, and the
            // measurement splits on *our* vulnerability alone — so the
            // discipline is authored as a vulnerability conjunct rather than a
            // flat band (`set_weak_two_overcall_discipline`).
            rules = if agreements.defense.weak_two_overcall_discipline {
                let (vul_lo, vul_hi) = if overcall_level <= 2 {
                    (12, 17)
                } else {
                    (15, 17)
                };
                rules.rule(
                    Bid::new(overcall_level, strain),
                    100,
                    len(suit, 5..) & points_by_vul(lo..=hi, vul_lo..=vul_hi),
                )
            } else {
                rules.rule(
                    Bid::new(overcall_level, strain),
                    100,
                    len(suit, 5..) & points(lo..=hi),
                )
            };
            if agreements.defense.weak_two_jump_overcall && overcall_level < 3 {
                rules = rules.rule(
                    Bid::new(overcall_level + 1, strain),
                    110,
                    len(suit, 6..) & points(13..=19),
                );
            }
        }
    }

    // The direct cue of their MAJOR weak two: the other major plus a minor, 5-5 —
    // what BBA bids and what `set_cue_reading` already reads.  Over 2♦ the cue is
    // deliberately absent (see `set_weak_two_cue`).
    //
    // Game-forcing, the same `points(14..)` Leaping Michaels demands, because the
    // cue *is* a game force by geometry: over 2♠ advancer cannot bid 3♥ under it,
    // so a heart preference costs 4♥ and every other answer is 3NT or four of a
    // minor.  A `points(8..)` Michaels here would commit an 8-count to the four
    // level.
    let cue_major = match theirs {
        Strain::Hearts => Some(Suit::Spades),
        Strain::Spades => Some(Suit::Hearts),
        _ => None,
    };
    if agreements.defense.weak_two_cue
        && let Some(other) = cue_major
    {
        rules = rules
            .rule(
                Bid::new(3, theirs),
                160,
                len(other, 5..) & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..)) & points(14..),
            )
            .alert(MICHAELS);
    }

    // Leaping Michaels: a jump to 4♣/4♦ showing a 5-5 two-suiter with
    // game-forcing values.  These are all 4-level jumps, so they never collide
    // with the natural overcalls above (which sit at the 2/3 level), and 4♦ over
    // 2♦ is a cue the natural loop skips.
    if agreements.defense.leaping_michaels_enabled {
        let t = theirs.suit().expect("weak two is a suit bid");
        let gf = points(14..);
        match t {
            // Over a major: a minor plus the OTHER major.  Superseded by the cue
            // when that is on — the cue shows the same hand a level cheaper, and
            // at the same `points(14..)` every Leaping hand also satisfies it, so
            // leaving both would author a rung the weights can never reach.  BBA
            // makes the same choice: `--mode def2-h` shows a `3♥` cue bucket and
            // no `4♣`/`4♦` bucket at all.
            Suit::Hearts | Suit::Spades if !agreements.defense.weak_two_cue => {
                let other = if t == Suit::Hearts {
                    Suit::Spades
                } else {
                    Suit::Hearts
                };
                for minor in [Suit::Clubs, Suit::Diamonds] {
                    rules = rules
                        .rule(
                            Bid::new(4, Strain::from(minor)),
                            200,
                            len(minor, 5..) & len(other, 5..) & gf.clone(),
                        )
                        .alert(LEAPING);
                }
            }
            // Over 2♦: 4♣ = clubs + a major; 4♦ (cue) = both majors.  Advancer's
            // continuation (incl. the 4♣ major-ask) is authored in
            // `leaping_michaels_advances`.
            Suit::Diamonds => {
                rules = rules
                    .rule(
                        Bid::new(4, Strain::Clubs),
                        200,
                        len(Suit::Clubs, 5..)
                            & (len(Suit::Hearts, 5..) | len(Suit::Spades, 5..))
                            & gf.clone(),
                    )
                    .alert(LEAPING)
                    .rule(
                        Bid::new(4, Strain::Diamonds),
                        200,
                        len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & gf.clone(),
                    )
                    .alert(LEAPING);
            }
            Suit::Clubs => {} // no weak 2♣ in our system
            // Majors with the cue on: the guarded arm above declined them.
            Suit::Hearts | Suit::Spades => {}
        }
    }
    rules
}

/// [`defense_to_weak_two`] over each weak-two opening, as rows
///
/// The exact-node pilot of the row layer: `P* (2♦)` and friends lower to the
/// plain seat-fanned insert.  Clubs is omitted — a 2♣ opening is the strong
/// artificial bid, not a weak two.
pub(super) fn weak_two_defense_package() -> Package {
    Package {
        name: "weak-two-defense",
        gate: |_| true,
        entries: |agreements| {
            [Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .flat_map(|suit| {
                    let opening = Bid::new(2, Strain::from(suit));
                    rows_of(
                        Pattern::node(&format!("P* ({opening})")),
                        defense_to_weak_two(opening, agreements),
                    )
                })
                .collect()
        },
    }
}

#[cfg(test)]
mod tests;
