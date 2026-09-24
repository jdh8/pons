//! Odwrotka — `1♣ - 1M - 2♦!`, the artificial reverse, answered by reverse-445566 steps
//!
//! The Watermelon Dutch Doubleton plugin's second gadget
//! (<https://jdh8.github.io/watermelon-dutch/1C/1M.html>), gated on
//! [`RebidKnobs::odwrotka`][crate::bidding::agreements::RebidKnobs::odwrotka]
//! (**default off**) and independent of the wide `1♣`.  Opener's `2♦` over a
//! one-level major response stops being the natural reverse into diamonds and
//! becomes a game force, or an invitation with exactly three-card support
//! (`with_odwrotka`); responder's six steps pin the major's length and the
//! strength at once, the strong step *below* the weak at each length so the
//! weak hand never declares notrump — Ingberman's idea, taken one step further
//! so a seven-card fit can still play at the two level.  Opener's continuation
//! after a step is the floor's: the book does not specify it.

use super::*;

/// Opener's artificial reverse — game-forcing, or invitational with 3= support
const ODWROTKA: Alert = Alert("odwrotka");
/// One of responder's six steps — each names the major's exact length and a
/// strength band, so even the ones that rebid the major are conventional
const STEP: Alert = Alert("odwrotka:step");

/// Overlay opener's `2♦!` on the `1♣ - 1M` rebid table
///
/// Weight 200: above the balanced `2NT` (120) and every natural rebid, below
/// the four-card-support raises (`3M` 220, `4M` 260), so a hand with four
/// trumps still raises and the artificial reverse carries the rest of the
/// strong hands.  The natural `2♦` reverse is withheld from the extras ladder
/// under this knob (`extras_ladder.rs`); the `3♦` jump shift stays.
pub(super) fn with_odwrotka(rules: Rules, opener_minor: Suit, agreements: &Agreements) -> Rules {
    if opener_minor != Suit::Clubs || !agreements.rebid.odwrotka {
        return rules;
    }
    rules
        .rule(
            Bid::new(2, Strain::Diamonds),
            200,
            points(19..) | (support(3..=3) & points(16..=18)),
        )
        .alert(ODWROTKA)
}

/// Responder's steps over the artificial reverse
///
/// Game-forcing is 10+ opposite the invitational floor; each length takes two
/// consecutive steps, the game-forcing one first.
fn steps(major: Suit) -> Rules {
    let m = Strain::from(major);
    let om = Strain::from(other_major(major));
    Rules::new()
        // 2OM! game-forcing, exactly four; 2M! minimum, exactly four.
        .rule(Bid::new(2, om), 100, len(major, 4..=4) & points(10..))
        .alert(STEP)
        .rule(Bid::new(2, m), 100, len(major, 4..=4) & points(..=9))
        .alert(STEP)
        // 2NT! game-forcing, exactly five; 3♣! minimum, exactly five.
        .rule(
            Bid::new(2, Strain::Notrump),
            100,
            len(major, 5..=5) & points(10..),
        )
        .alert(STEP)
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            len(major, 5..=5) & points(..=9),
        )
        .alert(STEP)
        // 3♦! game-forcing, six-plus; 3M minimum, six-plus.
        .rule(
            Bid::new(3, Strain::Diamonds),
            100,
            len(major, 6..) & points(10..),
        )
        .alert(STEP)
        .rule(Bid::new(3, m), 100, len(major, 6..) & points(..=9))
        .alert(STEP)
}

/// Responder's step table at `1♣ - 1M - 2♦!`, as a gated package
pub(in crate::bidding::american) fn odwrotka_continuations() -> Package {
    Package {
        name: "odwrotka",
        gate: |agreements| agreements.rebid.odwrotka,
        entries: |_| expand("P* 1♣ - 1M - 2♦ -", |_| true, |b| steps(b.suit('M'))),
    }
}

#[cfg(test)]
mod tests;
