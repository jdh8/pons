use super::*;

/// The retired guarded form of [`over_their_overcall`], kept verbatim as
/// the eval-equivalence oracle for `per_overcall_tables_match_legacy`;
/// its legality-anchored constraints (`min_level_is`, `they_bid`) resolve
/// per context exactly as the guard consulted it
#[cfg(test)]
pub(crate) fn over_their_overcall_legacy(opening: Suit) -> Rules {
    let o = opening;
    let o_strain = Strain::from(o);

    let is_major = matches!(o, Suit::Hearts | Suit::Spades);
    let raise_min: usize = if is_major { 3 } else { 5 };
    let jump_min: usize = if is_major { 4 } else { 5 };

    let other_major = match o {
        Suit::Hearts => Suit::Spades,
        // Spades → Hearts; for minors, Hearts is used only in the negative double
        _ => Suit::Hearts,
    };

    let mut rules = Rules::new();

    // Cue-bid raises: for each suit t ≠ o, levels 2 and 3
    for t in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if t == o {
            continue;
        }
        let t_strain = Strain::from(t);
        for lvl in 2u8..=3 {
            rules = rules
                .rule(
                    Bid::new(lvl, t_strain),
                    200,
                    they_bid(t_strain)
                        & min_level_is(lvl, t_strain)
                        & support(raise_min..)
                        & points(10..),
                )
                .alert(CUE_RAISE);
        }
    }

    // Jump raise: preemptive (min_level=2 means we could bid 2o, so 3o is a jump)
    rules = rules.rule(
        Bid::new(3, o_strain),
        160,
        min_level_is(2, o_strain) & support(jump_min..) & points(..=9),
    );

    // Competitive raise: 3o when it's the minimum legal bid
    rules = rules.rule(
        Bid::new(3, o_strain),
        130,
        min_level_is(3, o_strain) & support(raise_min..) & points(6..=9),
    );

    // Single raise
    rules = rules.rule(
        Bid::new(2, o_strain),
        150,
        min_level_is(2, o_strain) & support(raise_min..) & points(6..=9),
    );

    // Negative double. The major-opening double is common to every school;
    // the minor-opening shape follows [`NegativeDoubleShape`]. The dynamic
    // "which overcall" conditions are legality-anchored: `min_level_is(1, ♥)`
    // holds exactly over a (1♦) overcall, `they_bid(♥) & min_level_is(1, ♠)`
    // exactly over (1♥).
    let shape = negative_double_shape();
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
            NegativeDoubleShape::Modern => rules
                // Over (1♦): both majors, floor 6.
                .rule(
                    Call::Double,
                    100,
                    min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(6..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over (1♥): exactly four spades (five-plus bids the free 1♠).
                .rule(
                    Call::Double,
                    100,
                    they_bid(Strain::Hearts)
                        & min_level_is(1, Strain::Spades)
                        & len(Suit::Spades, 4..=4)
                        & hcp(6..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over (1♠)/(2♠): 4+ hearts, floor 8 (the reply starts at the
                // 2 level).
                .rule(
                    Call::Double,
                    100,
                    they_bid(Strain::Spades) & len(Suit::Hearts, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over a 2-level minor: both majors, floor 8.
                .rule(
                    Call::Double,
                    100,
                    (they_bid(Strain::Clubs) | they_bid(Strain::Diamonds))
                        & !min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
            NegativeDoubleShape::Cachalot => rules
                // Over (1♦): X transfers — 4+ hearts (may hold spades too).
                .rule(
                    Call::Double,
                    100,
                    min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & points(free_bid_floor()..),
                )
                .alert(CACHALOT_X)
                // Over (1♥): X transfers — 4+ spades.
                .rule(
                    Call::Double,
                    100,
                    they_bid(Strain::Hearts)
                        & min_level_is(1, Strain::Spades)
                        & len(Suit::Spades, 4..)
                        & points(free_bid_floor()..),
                )
                .alert(CACHALOT_X)
                // Natural from (1♠) up: the Modern rules apply.
                .rule(
                    Call::Double,
                    100,
                    they_bid(Strain::Spades) & len(Suit::Hearts, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE)
                .rule(
                    Call::Double,
                    100,
                    (they_bid(Strain::Clubs) | they_bid(Strain::Diamonds))
                        & !min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
            NegativeDoubleShape::Sputnik => rules
                // Over (1♦): the residual — ≤3 in both majors, 7+ (4+ in
                // either bids the natural free 1-level suit below).
                .rule(
                    Call::Double,
                    100,
                    min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, ..=3)
                        & len(Suit::Spades, ..=3)
                        & hcp(7..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over (1♥): the residual — ≤3 spades, 7+ (4+ bids the free 1♠).
                .rule(
                    Call::Double,
                    100,
                    they_bid(Strain::Hearts)
                        & min_level_is(1, Strain::Spades)
                        & len(Suit::Spades, ..=3)
                        & hcp(7..),
                )
                .alert(NEGATIVE_DOUBLE)
                // From (1♠) up: 4+ hearts, floor 8 — no 1-level major to deny
                // (the Modern rule).
                .rule(
                    Call::Double,
                    100,
                    they_bid(Strain::Spades) & len(Suit::Hearts, 4..) & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE)
                // Over a 2-level minor: both majors, floor 8 (the Modern rule).
                .rule(
                    Call::Double,
                    100,
                    (they_bid(Strain::Clubs) | they_bid(Strain::Diamonds))
                        & !min_level_is(1, Strain::Hearts)
                        & len(Suit::Hearts, 4..)
                        & len(Suit::Spades, 4..)
                        & hcp(8..),
                )
                .alert(NEGATIVE_DOUBLE),
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
        rules = rules
            // Over (1♦): 1♥ = 4+ spades without 4 hearts (4+ hearts doubles).
            .rule(
                Bid::new(1, Strain::Hearts),
                145,
                min_level_is(1, Strain::Hearts)
                    & len(Suit::Spades, 4..)
                    & len(Suit::Hearts, ..=3)
                    & points(free_bid_floor()..),
            )
            .alert(CACHALOT_TRANSFER)
            // Over (1♦): 1♠ = the takeout hand, ≤3 in both majors. Sits below
            // the notrump rules so a stopper hand prefers 1NT/2NT.
            .rule(
                Bid::new(1, Strain::Spades),
                85,
                min_level_is(1, Strain::Hearts)
                    & len(Suit::Hearts, ..=3)
                    & len(Suit::Spades, ..=3)
                    & hcp(7..),
            )
            .alert(CACHALOT_TAKEOUT)
            // Over (1♥): 1♠ = the takeout hand, ≤3 spades (4+ doubles).
            .rule(
                Bid::new(1, Strain::Spades),
                85,
                they_bid(Strain::Hearts)
                    & min_level_is(1, Strain::Spades)
                    & len(Suit::Spades, ..=3)
                    & hcp(7..),
            )
            .alert(CACHALOT_TAKEOUT);
    }

    // Sputnik's natural 1-level majors show 4+ (not the shared block's 5+) —
    // the free bid its residual double leans on. Only minor openings; the
    // `min_level_is` guards keep them to (1♦) [1♥/1♠] and (1♥) [1♠].
    if !is_major && shape == NegativeDoubleShape::Sputnik {
        rules = rules
            .rule(
                Bid::new(1, Strain::Hearts),
                145,
                min_level_is(1, Strain::Hearts)
                    & len(Suit::Hearts, 4..)
                    & points(free_bid_floor()..),
            )
            .rule(
                Bid::new(1, Strain::Spades),
                145,
                min_level_is(1, Strain::Spades)
                    & len(Suit::Spades, 4..)
                    & points(free_bid_floor()..),
            );
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
            if !(rotate && matches!(x, Suit::Hearts | Suit::Spades)) {
                let one_level =
                    min_level_is(1, xs) & len(x, 5..) & points(free_bid_floor()..) & !they_bid(xs);
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
            match free_bid_style() {
                // Forcing one round (the shipped default), answered by 4d.
                FreeBidStyle::Forcing => {
                    rules = rules.rule(
                        Bid::new(2, xs),
                        145,
                        min_level_is(2, xs) & len(x, 5..) & points(10..) & !they_bid(xs),
                    );
                }
                // Classic negative free bid: non-forcing 5–11 with a
                // six-carder or a strong five-carder — stronger long-suit
                // hands start with the widened double below.
                FreeBidStyle::Negative => {
                    rules = rules.rule(
                        Bid::new(2, xs),
                        145,
                        min_level_is(2, xs)
                            & (len(x, 6..) | (len(x, 5..) & top_honors(x, 2..)))
                            & points(5..=11)
                            & !they_bid(xs),
                    );
                }
                // The transfer rotation is authored per suit *pair* after
                // this loop — it needs both slots in one constraint.
                FreeBidStyle::Transfer => {}
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
                min_level_is(2, ss) & !they_bid(ss)
            };
            for i in 0..3 {
                for j in (i + 1)..3 {
                    let (x, y) = (others[i], others[j]);
                    let w = others[3 - i - j];
                    // The lower slot shows the higher suit (true transfer)…
                    rules = rules
                        .rule(
                            Bid::new(2, Strain::from(x)),
                            145,
                            slot(x) & slot(y) & !slot(w) & len(y, 5..) & points(6..),
                        )
                        .alert(FREE_TRANSFER)
                        // …and the higher slot wraps around to show the lower.
                        .rule(
                            Bid::new(2, Strain::from(y)),
                            145,
                            slot(x) & slot(y) & !slot(w) & len(x, 5..) & points(6..),
                        )
                        .alert(FREE_TRANSFER);
                }
            }
            for i in 0..3 {
                let x = others[i];
                let (y, z) = (others[(i + 1) % 3], others[(i + 2) % 3]);
                // No swap partner (or two of them): natural and forcing, as
                // in the default style.
                rules = rules.rule(
                    Bid::new(2, Strain::from(x)),
                    145,
                    slot(x)
                        & ((slot(y) & slot(z)) | (!slot(y) & !slot(z)))
                        & len(x, 5..)
                        & points(10..),
                );
            }
        }
        let one_notrump = min_level_is(1, Strain::Notrump)
            & hcp(free_1nt_floor()..=10)
            & stopper_in_their_suits();
        rules = if free_bid_quality() {
            rules.rule(
                Bid::new(1, Strain::Notrump),
                90,
                one_notrump & !vulnerable(),
            )
        } else {
            rules.rule(Bid::new(1, Strain::Notrump), 90, one_notrump)
        };
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            95,
            min_level_is(2, Strain::Notrump) & hcp(11..=12) & stopper_in_their_suits(),
        );
        // The natural invitational 2NT *jump* over a 1-level overcall: 11–12
        // with a stopper, the invite the ordinary 2NT rule (min-level, i.e. a
        // 2-level overcall) leaves stranded. `min_level_is(1, Notrump)` means
        // 1NT is still the cheapest notrump, so this 2NT is a jump.
        rules = rules.rule(
            Bid::new(2, Strain::Notrump),
            95,
            min_level_is(1, Strain::Notrump) & hcp(11..=12) & stopper_in_their_suits(),
        );
    }

    // Weak jump shifts: for each suit x ≠ o, levels 2 and 3
    for x in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if x == o {
            continue;
        }
        let x_strain = Strain::from(x);
        for lvl in 2u8..=3 {
            rules = rules.rule(
                Bid::new(lvl, x_strain),
                110,
                min_level_is(lvl - 1, x_strain) & len(x, 6..) & points(2..=5) & !they_bid(x_strain),
            );
        }
    }

    // Pass
    rules.rule(Call::Pass, 0, hcp(0..))
}
