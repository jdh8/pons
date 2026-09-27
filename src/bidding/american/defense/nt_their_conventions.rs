//! Defending *their* notrump conventions — Stayman, transfers, and the minors
//!
//! Once they open `1NT` and their partner bids a convention, our double and
//! cue change meaning: the double shows the suit their call is *really* about.
//! Four independent agreements, one per convention
//! (`stayman_defense_enabled`, `transfer_defense_enabled`,
//! `minor_transfer_defense_enabled`, `diamond_transfer_defense_enabled`, all on
//! `agreements.defense`).
use super::*;

/// Defense to the opponents' 2♣ Stayman (`(1NT) - (2♣)`)
///
/// Only the `X` is authored: lead-directing clubs (5+ with values, the bid
/// suit — not takeout).  Every other hand is **rejected**, so the floor owns
/// it: the natural overcalls and the pass are the floor's, read by the natural
/// walk.  The v6 floor's own `X` here is already a club double (5+ clubs on
/// 99% of them), so the rule's projection reads it truthfully too.
///
/// This replaced an owning table (6-card `points(14..)` overcalls, a strong
/// `3♣` and a `Pass` catch-all) whose `Pass` shadowed the floor's natural
/// overcalls: SD-PD −0.0052/−0.0043 IMPs/board NV/vul (2026-09-27), while its
/// `X` alone was SD-PD +0.44/+0.47 per fired.  An Unusual `2NT` was dropped
/// earlier (−4.9 IMPs/fired).
fn defense_to_their_stayman() -> Rules {
    Rules::new()
        .rule(
            Call::Double,
            190,
            len(Suit::Clubs, 5..) & suit_hcp(Suit::Clubs, 5..) & points(8..),
        )
        .alert(STAYMAN_DEFENSE_X)
}

/// Defense to the opponents' Jacoby transfer after `(1NT) - (2♦)` or
/// `(1NT) - (2♥)`; their response transfers to hearts or spades, respectively
///
/// `X` = lead-directing the `bid` (transfer) suit (5+ with values, not takeout);
/// a cue of the `shown_major` (the suit they transferred into) = the **other**
/// major + a minor (Michaels 5-5); natural one-suiter overcalls in every suit but
/// the one they showed (six-card, `points(14..)`, the A/B-searched Stayman-defense
/// floor — light overcalls into a strong-1NT auction are PD-negative), with the
/// transfer suit's own 3-level overcall weighted above the `X` so a real suit
/// declares rather than lead-directs.  An owning Pass catches the ~80% that act
/// on nothing.  Distilled from BBA (probe modes `xfer-h`/`xfer-s`).
fn defense_to_their_transfer(bid: Suit, shown_major: Suit) -> Rules {
    let (min_len, floor) = (6usize, 14u8);
    let other_major = if shown_major == Suit::Spades {
        Suit::Hearts
    } else {
        Suit::Spades
    };
    let mut rules = Rules::new()
        .rule(
            Call::Double,
            190,
            len(bid, 5..) & suit_hcp(bid, 5..) & points(8..),
        )
        .alert(TRANSFER_DEFENSE_X)
        .rule(
            Bid::new(2, Strain::from(shown_major)),
            170,
            len(other_major, 5..)
                & (len(Suit::Clubs, 5..) | len(Suit::Diamonds, 5..))
                & points(8..),
        )
        .alert(TRANSFER_DEFENSE_CUE);
    // Natural one-suiter overcalls in every suit but the one they showed, each at
    // its cheapest legal level above their transfer; the transfer suit's own
    // overcall is the *strong* 3-level declare (weight 2.0) above the lead-direct X.
    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == shown_major {
            continue;
        }
        let strain = Strain::from(suit);
        let level = if strain > Strain::from(bid) { 2 } else { 3 };
        let weight = if suit == bid { 200 } else { 180 };
        rules = rules.rule(
            Bid::new(level, strain),
            weight,
            len(suit, min_len..) & points(floor..),
        );
    }
    rules.rule(Call::Pass, 50, hcp(0..))
}

/// Defense to the opponents' two-way 2♠ minor response (`(1NT) - (2♠)`)
///
/// Their 2♠ names spades (the bid) but means clubs (the anchor), so: `X` =
/// lead-directing spades (5+ with values, not takeout); `2NT` = the two lowest unbid
/// suits (diamonds + hearts, 5-5); `3♣` (cueing their clubs anchor) = the
/// top-and-bottom two-suiter (spades + diamonds, 5-5), weighted **above** the `X` so
/// a genuine two-suiter shows rather than lead-directs; natural `3♦`/`3♥` six-card
/// one-suiters (`points(14..)`, the A/B-searched Stayman-defense floor — light
/// overcalls into a strong-1NT auction are PD-negative).  An owning Pass catches the
/// ~80% that act on nothing.  Modeled on [`defense_to_their_transfer`].
fn defense_to_their_minor_transfer() -> Rules {
    Rules::new()
        // X = lead-directing spades (the bid suit), 5+ with values.
        .rule(
            Call::Double,
            190,
            len(Suit::Spades, 5..) & suit_hcp(Suit::Spades, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_X)
        // 2NT = the two lowest unbid suits (diamonds + hearts, 5-5) — naturally
        // disjoint from the spade-showing X.
        .rule(
            Bid::new(2, Strain::Notrump),
            170,
            len(Suit::Diamonds, 5..) & len(Suit::Hearts, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_2NT)
        // 3♣ cue of their clubs anchor = top-and-bottom (spades + diamonds, 5-5);
        // weight 2.0 beats the X so the two-suiter wins for a 5♠5♦ hand.
        .rule(
            Bid::new(3, Strain::Clubs),
            200,
            len(Suit::Spades, 5..) & len(Suit::Diamonds, 5..) & points(8..),
        )
        .alert(MINOR_TRANSFER_DEFENSE_CUE)
        // Natural six-card one-suiter overcalls in the unbid red suits.
        .rule(
            Bid::new(3, Strain::Diamonds),
            180,
            len(Suit::Diamonds, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            180,
            len(Suit::Hearts, 6..) & points(14..),
        )
        .rule(Call::Pass, 50, hcp(0..))
}

/// Our defense after the opponents' 2NT diamond transfer (`(1NT) - (2NT)`)
///
/// Their 2NT shows diamonds, so: `X` = lead-directing diamonds (5+ with values,
/// not takeout); `3♦` (cueing their diamond anchor) = both majors (5-5, Michaels),
/// weighted **above** the `X` so a genuine two-suiter shows rather than
/// lead-directs; natural `3♣`/`3♥`/`3♠` six-card one-suiters (`points(14..)`).  An
/// owning Pass catches the rest.  Modeled on [`defense_to_their_minor_transfer`].
fn defense_to_their_diamond_transfer() -> Rules {
    Rules::new()
        // X = lead-directing diamonds (the shown suit), 5+ with values.
        .rule(
            Call::Double,
            190,
            len(Suit::Diamonds, 5..) & suit_hcp(Suit::Diamonds, 5..) & points(8..),
        )
        .alert(DIAMOND_TRANSFER_DEFENSE_X)
        // 3♦ cue of their diamond anchor = both majors (5-5); weight 2.0 beats the
        // X so a 5♥-5♠ two-suiter shows rather than lead-directs.
        .rule(
            Bid::new(3, Strain::Diamonds),
            200,
            len(Suit::Hearts, 5..) & len(Suit::Spades, 5..) & points(8..),
        )
        .alert(DIAMOND_TRANSFER_DEFENSE_CUE)
        // Natural six-card one-suiter overcalls in the unbid suits.
        .rule(
            Bid::new(3, Strain::Clubs),
            180,
            len(Suit::Clubs, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Hearts),
            180,
            len(Suit::Hearts, 6..) & points(14..),
        )
        .rule(
            Bid::new(3, Strain::Spades),
            180,
            len(Suit::Spades, 6..) & points(14..),
        )
        .rule(Call::Pass, 50, hcp(0..))
}

/// Defense to their `2♣` Stayman: `X` = lead-directing clubs, everything
/// else to the floor (`set_stayman_defense`)
pub(super) fn their_stayman_defense_package() -> Package {
    Package {
        name: "their-stayman-defense",
        gate: |agreements| agreements.defense.stayman_defense_enabled,
        entries: |_| rows_of(Pattern::node("P* (1NT) - (2♣)"), defense_to_their_stayman()),
    }
}

/// Defense to their Jacoby transfers: `X` = lead-directing the bid suit, the
/// cue is Michaels (the other major plus a minor), plus natural overcalls
/// (`set_transfer_defense`)
pub(super) fn their_transfer_defense_package() -> Package {
    Package {
        name: "their-transfer-defense",
        gate: |agreements| agreements.defense.transfer_defense_enabled,
        entries: |_| {
            [(Suit::Diamonds, Suit::Hearts), (Suit::Hearts, Suit::Spades)]
                .into_iter()
                .flat_map(|(resp, shown)| {
                    let response = Bid::new(2, Strain::from(resp));
                    rows_of(
                        Pattern::node(&format!("P* (1NT) - ({response})")),
                        defense_to_their_transfer(resp, shown),
                    )
                })
                .collect()
        },
    }
}

/// Defense to their two-way `2♠` minor response: `X` = lead-directing spades,
/// `2NT` = the red two-suiter, `3♣` cue = top-and-bottom, natural `3♦`/`3♥`
/// overcalls (`set_minor_transfer_defense`)
pub(super) fn their_minor_transfer_defense_package() -> Package {
    Package {
        name: "their-minor-transfer-defense",
        gate: |agreements| agreements.defense.minor_transfer_defense_enabled,
        entries: |_| {
            rows_of(
                Pattern::node("P* (1NT) - (2♠)"),
                defense_to_their_minor_transfer(),
            )
        },
    }
}

/// Defense to their `2NT` diamond transfer: `X` = lead-directing diamonds,
/// `3♦` cue = both majors, natural `3♣`/`3♥`/`3♠` overcalls
/// (`set_diamond_transfer_defense`)
pub(super) fn their_diamond_transfer_defense_package() -> Package {
    Package {
        name: "their-diamond-transfer-defense",
        gate: |agreements| agreements.defense.diamond_transfer_defense_enabled,
        entries: |_| {
            rows_of(
                Pattern::node("P* (1NT) - (2NT)"),
                defense_to_their_diamond_transfer(),
            )
        },
    }
}
