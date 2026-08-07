//! Responder's direct-seat action over their overcall of our one-level opening
//!
//! The base contested table every other agreement in this directory hangs off:
//! the raises, the natural bids and their floor ([`super::lebensohl::set_natural_floor`]), the
//! stopper-gated direct `3NT`, and the slot arithmetic the transfer free bids
//! use.  Responder's `X`/`Pass` options are [`super::penalty_double`]; the
//! free-bid and negative-double *answers* are [`super::free_bids`] and
//! [`super::negative_double`].
//!
//! Roughly a third of this module is [`legacy::over_their_overcall_legacy`], the retired
//! imperative oracle the rows port is pinned against — dead in production,
//! reachable only from `per_overcall_tables_match_legacy`.

use super::free_bids::{
    FreeBidStyle, free_1nt_floor, free_bid_floor, free_bid_quality, free_bid_style,
    free_bids_engaged,
};
use super::negative_double::{NegativeDoubleShape, negative_double_shape};
use super::two_suiters::uvu_over_majors;
use super::*;

/// Responder's action after our opening `opening` and their `overcall` — any
/// bid through 2♠, or their 1NT
///
/// Covers cue-bid limit-plus raises, preemptive and competitive raises of
/// the opening suit, negative doubles, and weak jump shifts.  One exact
/// table per overcall: the legality-anchored conditions the guarded form
/// carried (`min_level_is`, `they_bid`) are decided at build time by
/// `cheapest`, so each surviving rule's projection reads only the hands that
/// can actually make its call — the arms for *other* overcalls no longer
/// leak into the reading.  Eval-equivalence with the retired guarded form is
/// pinned by `per_overcall_tables_match_legacy`.
pub(super) fn over_their_overcall(opening: Suit, overcall: Bid) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);
    // The cheapest legal level for a strain over `overcall` — the build-time
    // mirror of `min_level_is`.
    let cheapest = |strain: Strain| overcall.level.get() + u8::from(strain <= overcall.strain);

    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };
    let jump_min: usize = if is_major { 4 } else { 5 };

    let other_major = match o {
        Suit::Hearts => Suit::Spades,
        // Spades → Hearts; for minors, Hearts is used only in the negative double
        _ => Suit::Hearts,
    };

    let mut rules = Rules::new();

    // Cue-bid raise of their suit at the cheapest cue level: limit-plus
    if let Ok(t) = Suit::try_from(overcall.strain)
        && t != o
    {
        rules = rules
            .rule(
                Bid::new(cheapest(overcall.strain), overcall.strain),
                200,
                support(raise_min..) & points(10..),
            )
            .alert(CUE_RAISE);
    }

    // Raises of the opening suit: which rungs exist depends only on where
    // the overcall pushed the auction — room below 3o leaves the preemptive
    // jump and the single raise; 3o cheapest leaves the competitive raise.
    if cheapest(o_strain) == 2 {
        rules = rules
            .rule(
                Bid::new(3, o_strain),
                160,
                support(jump_min..) & points(..=9),
            )
            .rule(
                Bid::new(2, o_strain),
                150,
                support(raise_min..) & points(6..=9),
            );
    } else {
        rules = rules.rule(
            Bid::new(3, o_strain),
            130,
            support(raise_min..) & points(6..=9),
        );
    }

    // Negative double. The major-opening double is common to every school;
    // the minor-opening shape follows [`NegativeDoubleShape`], each column
    // keeping only the arm its overcall selects.
    let shape = negative_double_shape();
    let over_one_diamond = cheapest(Strain::Hearts) == 1;
    let over_one_heart = overcall == Bid::new(1, Strain::Hearts);
    let over_spades = overcall.strain == Strain::Spades;
    let over_two_minor =
        matches!(overcall.strain, Strain::Clubs | Strain::Diamonds) && !over_one_diamond;
    rules = if is_major {
        // Other major, 4+ cards, 8+ HCP
        rules
            .rule(Call::Double, 100, len(other_major, 4..) & hcp(8..))
            .alert(NEGATIVE_DOUBLE)
    } else {
        match shape {
            // Both majors 4+, 8+ HCP — the shipped rule.
            NegativeDoubleShape::BothMajors => rules
                .rule(
                    Call::Double,
                    100,
                    len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
            NegativeDoubleShape::Modern => {
                if over_one_diamond {
                    // Over (1♦): both majors, floor 6.
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(6..),
                        )
                        .alert(NEGATIVE_DOUBLE)
                } else if over_one_heart {
                    // Over (1♥): exactly four spades (five-plus bids the free 1♠).
                    rules
                        .rule(Call::Double, 100, len(Suit::Spades, 4..=4) & hcp(6..))
                        .alert(NEGATIVE_DOUBLE)
                } else if over_spades {
                    // Over (1♠)/(2♠): 4+ hearts, floor 8 (the reply starts at
                    // the 2 level).
                    rules
                        .rule(Call::Double, 100, len(Suit::Hearts, 4..) & hcp(8..))
                        .alert(NEGATIVE_DOUBLE)
                } else if over_two_minor {
                    // Over a 2-level minor: both majors, floor 8.
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
                        )
                        .alert(NEGATIVE_DOUBLE)
                } else {
                    rules
                }
            }
            NegativeDoubleShape::Cachalot => {
                if over_one_diamond {
                    // Over (1♦): X transfers — 4+ hearts (may hold spades too).
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Hearts, 4..) & points(free_bid_floor()..),
                        )
                        .alert(CACHALOT_X)
                } else if over_one_heart {
                    // Over (1♥): X transfers — 4+ spades.
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Spades, 4..) & points(free_bid_floor()..),
                        )
                        .alert(CACHALOT_X)
                } else if over_spades {
                    // Natural from (1♠) up: the Modern rules apply.
                    rules
                        .rule(Call::Double, 100, len(Suit::Hearts, 4..) & hcp(8..))
                        .alert(NEGATIVE_DOUBLE)
                } else if over_two_minor {
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
                        )
                        .alert(NEGATIVE_DOUBLE)
                } else {
                    rules
                }
            }
            NegativeDoubleShape::Sputnik => {
                if over_one_diamond {
                    // Over (1♦): the residual — ≤3 in both majors, 7+ (4+ in
                    // either bids the natural free 1-level suit below).
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Hearts, ..=3) & len(Suit::Spades, ..=3) & hcp(7..),
                        )
                        .alert(NEGATIVE_DOUBLE)
                } else if over_one_heart {
                    // Over (1♥): the residual — ≤3 spades, 7+ (4+ bids the free 1♠).
                    rules
                        .rule(Call::Double, 100, len(Suit::Spades, ..=3) & hcp(7..))
                        .alert(NEGATIVE_DOUBLE)
                } else if over_spades {
                    // From (1♠) up: 4+ hearts, floor 8 — no 1-level major to
                    // deny (the Modern rule).
                    rules
                        .rule(Call::Double, 100, len(Suit::Hearts, 4..) & hcp(8..))
                        .alert(NEGATIVE_DOUBLE)
                } else if over_two_minor {
                    // Over a 2-level minor: both majors, floor 8 (the Modern rule).
                    rules
                        .rule(
                            Call::Double,
                            100,
                            len(Suit::Hearts, 4..) & len(Suit::Spades, 4..) & hcp(8..),
                        )
                        .alert(NEGATIVE_DOUBLE)
                } else {
                    rules
                }
            }
        }
    };

    // Classic NFB widens the double: the 2-level new suits are capped at 11,
    // so every stronger long-suit hand starts here and clarifies with the
    // forcing-to-game new suit next round (Section 4d″). The second `X` rule
    // ORs into the projection — the points floor survives (every school's
    // double floors at or below 12) but the suit floors collapse to zero:
    // the named OR-projection wall, priced by the Stage-B A/B. Weight below
    // the cue (2.0) and the free bids (1.45) so a biddable hand still bids.
    if free_bid_style() == FreeBidStyle::Negative {
        rules = rules
            .rule(Call::Double, 90, points(12..))
            .alert(NEGATIVE_DOUBLE);
    }

    // Cachalot's rotated 1-level calls over (1♦)/(1♥): 1♥ shows spades, 1♠
    // is the residual takeout hand. Only minor openings rotate. Cachalot is
    // rotated Sputnik, so the floors match Sputnik's — the major-showing
    // calls take the free-bid `points` floor (hcp(6..) orphaned the light
    // shapely hands Modern frees, the Stage-A named leak) and the residual
    // takeout matches the residual double's hcp(7..).
    if !is_major && shape == NegativeDoubleShape::Cachalot {
        if over_one_diamond {
            rules = rules
                // Over (1♦): 1♥ = 4+ spades without 4 hearts (4+ hearts doubles).
                .rule(
                    Bid::new(1, Strain::Hearts),
                    145,
                    len(Suit::Spades, 4..) & len(Suit::Hearts, ..=3) & points(free_bid_floor()..),
                )
                .alert(CACHALOT_TRANSFER)
                // Over (1♦): 1♠ = the takeout hand, ≤3 in both majors. Sits
                // below the notrump rules so a stopper hand prefers 1NT/2NT.
                .rule(
                    Bid::new(1, Strain::Spades),
                    85,
                    len(Suit::Hearts, ..=3) & len(Suit::Spades, ..=3) & hcp(7..),
                )
                .alert(CACHALOT_TAKEOUT);
        } else if over_one_heart {
            // Over (1♥): 1♠ = the takeout hand, ≤3 spades (4+ doubles).
            rules = rules
                .rule(
                    Bid::new(1, Strain::Spades),
                    85,
                    len(Suit::Spades, ..=3) & hcp(7..),
                )
                .alert(CACHALOT_TAKEOUT);
        }
    }

    // Sputnik's natural 1-level majors show 4+ (not the shared block's 5+) —
    // the free bid its residual double leans on. Only minor openings; the
    // `cheapest` gates keep them to (1♦) [1♥/1♠] and (1♥) [1♠].
    if !is_major && shape == NegativeDoubleShape::Sputnik {
        if cheapest(Strain::Hearts) == 1 {
            rules = rules.rule(
                Bid::new(1, Strain::Hearts),
                145,
                len(Suit::Hearts, 4..) & points(free_bid_floor()..),
            );
        }
        if cheapest(Strain::Spades) == 1 {
            rules = rules.rule(
                Bid::new(1, Strain::Spades),
                145,
                len(Suit::Spades, 4..) & points(free_bid_floor()..),
            );
        }
    }

    // Natural free bids (`set_free_bids`; implied by the Modern/Cachalot
    // shapes, whose tighter doubles need the natural outlet). A free bid of
    // their suit is the cue above; the 1-level majors stay out of the
    // Cachalot rotation's way (a 5-card major routes through its transfer).
    if free_bids_engaged() {
        // Cachalot and Sputnik both author their own 1-level majors above, so
        // skip the shared 5+ rule for them (Cachalot rotates, Sputnik lowers to
        // 4+).
        let rotate = !is_major
            && matches!(
                shape,
                NegativeDoubleShape::Cachalot | NegativeDoubleShape::Sputnik
            );
        for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            if x == o {
                continue;
            }
            let xs = Strain::from(x);
            if cheapest(xs) == 1 && !(rotate && matches!(x, Suit::Hearts | Suit::Spades)) {
                let one_level = len(x, 5..) & points(free_bid_floor()..);
                rules = if free_bid_quality() {
                    rules.rule(
                        Bid::new(1, xs),
                        145,
                        one_level & (top_honors(x, 2..) | !vulnerable()),
                    )
                } else {
                    rules.rule(Bid::new(1, xs), 145, one_level)
                };
            }
            if cheapest(xs) == 2 && xs != overcall.strain {
                match free_bid_style() {
                    // Forcing one round (the shipped default), answered by 4d.
                    FreeBidStyle::Forcing => {
                        rules = rules.rule(Bid::new(2, xs), 145, len(x, 5..) & points(10..));
                    }
                    // Classic negative free bid: non-forcing 5–11 with a
                    // six-carder or a strong five-carder — stronger long-suit
                    // hands start with the widened double below.
                    FreeBidStyle::Negative => {
                        rules = rules.rule(
                            Bid::new(2, xs),
                            145,
                            (len(x, 6..) | (len(x, 5..) & top_honors(x, 2..))) & points(5..=11),
                        );
                    }
                    // The transfer rotation is authored per suit *pair* after
                    // this loop — it needs both slots in one constraint.
                    FreeBidStyle::Transfer => {}
                }
            }
        }
        // Cachalot-style 2-level transfers: when exactly two unbid suits sit
        // at the two level the slots swap, so opener completes and declares
        // the concealed hand; the wrap (higher) slot completes a level
        // higher. A lone slot — or all three over a (1NT) overcall — stays
        // natural-forcing. Unlimited at 6+: the weak hand passes the
        // completion, strength clarifies a round later.
        if free_bid_style() == FreeBidStyle::Transfer {
            let others: Vec<Suit> = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .filter(|&x| x != o)
                .collect();
            let slot = |s: Suit| {
                let ss = Strain::from(s);
                cheapest(ss) == 2 && ss != overcall.strain
            };
            for i in 0..3 {
                for j in (i + 1)..3 {
                    let (x, y) = (others[i], others[j]);
                    let w = others[3 - i - j];
                    if slot(x) && slot(y) && !slot(w) {
                        // The lower slot shows the higher suit (true transfer)…
                        rules = rules
                            .rule(Bid::new(2, Strain::from(x)), 145, len(y, 5..) & points(6..))
                            .alert(FREE_TRANSFER)
                            // …and the higher slot wraps around to show the lower.
                            .rule(Bid::new(2, Strain::from(y)), 145, len(x, 5..) & points(6..))
                            .alert(FREE_TRANSFER);
                    }
                }
            }
            for i in 0..3 {
                let x = others[i];
                let (y, z) = (others[(i + 1) % 3], others[(i + 2) % 3]);
                // No swap partner (or two of them): natural and forcing, as
                // in the default style.
                if slot(x) && ((slot(y) && slot(z)) || (!slot(y) && !slot(z))) {
                    rules = rules.rule(
                        Bid::new(2, Strain::from(x)),
                        145,
                        len(x, 5..) & points(10..),
                    );
                }
            }
        }
        if cheapest(Strain::Notrump) == 1 {
            let one_notrump = hcp(free_1nt_floor()..=10) & stopper_in_their_suits();
            rules = if free_bid_quality() {
                rules.rule(
                    Bid::new(1, Strain::Notrump),
                    90,
                    one_notrump & !vulnerable(),
                )
            } else {
                rules.rule(Bid::new(1, Strain::Notrump), 90, one_notrump)
            };
        }
        // The invitational 2NT: 11–12 with a stopper — the cheapest notrump
        // over a 2-level overcall (or their 1NT), a jump over a 1-level suit
        // overcall.  The guarded form needed one arm per case; per column
        // they collapse into this single rung.
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            95,
            hcp(11..=12) & stopper_in_their_suits(),
        );
    }

    // Weak jump shifts: one level above each unbid suit's cheapest bid,
    // through the 3 level
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let xs = Strain::from(x);
        if cheapest(xs) <= 2 && xs != overcall.strain {
            rules = rules.rule(
                Bid::new(cheapest(xs) + 1, xs),
                110,
                len(x, 6..) & points(2..=5),
            );
        }
    }

    // Pass
    rules.rule(Call::Pass, 0, hcp(0..))
}

#[cfg(test)]
mod legacy;
#[cfg(test)]
pub(super) use legacy::over_their_overcall_legacy;

/// How many unbid suits sit at the two level over their `ovc` after our
/// `o_strain` opening — the `FreeBidStyle::Transfer` swap fires on exactly
/// two (the same cheapest-level arithmetic as the Section-4d guard)
pub(super) fn two_level_slots(o_strain: Strain, ovc: Bid) -> usize {
    [
        Strain::Clubs,
        Strain::Diamonds,
        Strain::Hearts,
        Strain::Spades,
    ]
    .into_iter()
    .filter(|&s| s != o_strain && s != ovc.strain)
    .filter(|&s| ovc.level.get() + u8::from(s < ovc.strain) == 2)
    .count()
}

/// Section 1 & 2 as a row package: over each one-suit opening, the direct-seat
/// responder tables per overcall through 2♠ plus their 1NT
/// ([`over_their_overcall`]), and the systems-on rebase over their takeout
/// double
///
/// One exact node per (opening, overcall) — the expansion of the retired
/// `(≤2♠)` guard.  Their Michaels cue of our major belongs to the
/// uvu-over-majors package, which answers the whole call at the same key; the
/// domain yields that column whenever that package is engaged (the guard used
/// to lose the same resolution race structurally).
pub(super) fn direct_seat_package() -> Package {
    Package {
        name: "direct-seat",
        gate: || true,
        entries: || {
            let mut entries = Vec::new();
            for opening in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let key = format!("P* 1{}", Strain::from(opening));
                entries.extend(expand(
                    &format!("{key} (.x)"),
                    move |bindings| {
                        let overcall = bindings.bid('x');
                        overcall <= Bid::new(2, Strain::Spades)
                            && !(matches!(opening, Suit::Hearts | Suit::Spades)
                                && overcall == Bid::new(2, Strain::from(opening))
                                && uvu_over_majors())
                    },
                    move |bindings| over_their_overcall(opening, bindings.bid('x')),
                ));
                // The guard admitted their 1NT overcall too (1NT < 2♠).
                entries.extend(rows_of(
                    Pattern::node(&format!("{key} (1NT)")),
                    over_their_overcall(opening, Bid::new(1, Strain::Notrump)),
                ));
                entries.push(rebase(Pattern::first(&key, "X"), ReplaceNext(Call::Pass)));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
