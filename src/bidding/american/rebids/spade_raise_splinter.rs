//! Opener's splinter raise after `1x - 1♠` and responder's answers
//!
//! Gated by [`RebidKnobs::spade_raise_splinter`] (default off; *Open work*
//! item 9 (b) of `docs/bba-gap-campaign.md`).  Opener's double jump in a new
//! suit — `4♦` / `4♥` after `1♣`, `4♣` / `4♥` after `1♦`, `4♣` / `4♦` after
//! `1♥` — shows four-card spade support, the `4♠` raise's points (17+ under
//! the ladder) and a singleton or void in the bid suit.  The knob's value is
//! the most HCP the short suit may hold: `Some(1)` is BBA's rule (a small
//! singleton or a void; it never splinters on a singleton K and rarely on an
//! A or Q), `Some(4)` any singleton.
//!
//! | Call | Wt | Meaning |
//! |------|----|---------|
//! | opener `4x` | 2.7 | 4+ ♠, the `4♠` range, 0–1 in `x` with at most the knob's HCP there |
//! | responder `4NT` | 1.5 | RKCB on points − ½ × K/Q/J HCP in `x` ≥ 10 (the census's slam gate; the ace is not waste, round 1 lost on counting it) |
//! | responder `4♠` | 1.0 | sign-off, everything else |
//! | opener over `4♠` | 0.0 | pass |
//!
//! Their double of the splinter takes the same table; their overcall of a
//! `4♣` / `4♦` splinter up to `4♥` leaves responder bidding `4♠`.
//!
//! The double-dummy census over 5.4M `1x - 1♠` raises (2026-11-08,
//! `ab-results/item9-research-2026-11-08/r_census.md`): a responder bidding
//! slam facing a 17–21 splinter on points − ½ × wasted ≥ 10 is worth about
//! +1.6 / +1.9 IMPs per splinter hand (none / vul), and separating the
//! shortness hands from the blind `4♠` about +0.02 IMPs per raise board.

use super::*;
use crate::bidding::american::slam;
use crate::bidding::constraint::{has_ace, suit_hcp};

/// Splinter — opener's double jump in a new suit over `1x - 1♠`, showing a
/// singleton or void there with four-card spade support
const SPLINTER: Alert = Alert("splinter");

/// The two suits opener can splinter in after opening `opener_suit` and
/// hearing `1♠`
fn splinter_suits(opener_suit: Suit) -> [Suit; 2] {
    match opener_suit {
        Suit::Clubs => [Suit::Diamonds, Suit::Hearts],
        Suit::Diamonds => [Suit::Clubs, Suit::Hearts],
        _ => [Suit::Clubs, Suit::Diamonds],
    }
}

/// Append opener's splinters to a `1x - 1♠` rebid table
///
/// `four` is the `4♠` raise's point range; weight 270 sits above that raise
/// (260) so a short-suited hand splinters instead.
pub(super) fn with_spade_raise_splinter(
    rules: Rules,
    opener_suit: Suit,
    four: core::ops::RangeFrom<u8>,
    agreements: &Agreements,
) -> Rules {
    let Some(short_hcp) = agreements.rebid.spade_raise_splinter else {
        return rules;
    };
    splinter_suits(opener_suit)
        .into_iter()
        .fold(rules, |rules, x| {
            rules
                .rule(
                    Bid::new(4, Strain::from(x)),
                    270,
                    support(4..) & points(four.clone()) & len(x, ..=1) & suit_hcp(x, ..=short_hcp),
                )
                .alert(SPLINTER)
        })
}

/// At most `w` HCP wasted in `x`: K, Q and J there count, the ace facing
/// shortness is a working card (round 1 lost on responder signing off with
/// it, see the module doc)
fn waste(x: Suit, w: u8) -> Cons<impl Constraint + Clone> {
    (!has_ace(x) & suit_hcp(x, ..=w)) | (has_ace(x) & suit_hcp(x, ..=w + 4))
}

/// Responder facing the splinter in `x`: RKCB when points less half the
/// honours wasted in `x` reach 10, else the `4♠` sign-off
///
/// The half-waste discount is spelled as a ladder of (points, wasted) bands
/// so it projects like any other constraint.
fn responder_after_splinter(x: Suit) -> Rules {
    let slam_try = (points(10..) & waste(x, 1))
        | (points(11..) & waste(x, 3))
        | (points(12..) & waste(x, 5))
        | (points(13..) & waste(x, 7))
        | points(14..);
    Rules::new()
        .rule(Bid::new(4, Strain::Notrump), 150, slam_try)
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::Spades), 100, hcp(0..))
}

/// Responder over their overcall of the splinter: the game, nothing more
fn responder_after_splinter_overcalled() -> Rules {
    Rules::new().rule(Bid::new(4, Strain::Spades), 100, hcp(0..))
}

/// Responder's answers after every `1x - 1♠ - 4y` splinter, both tails, the
/// RKCB trees, and opener's pass of the sign-off
pub(crate) fn spade_raise_splinter_continuations() -> Package {
    Package {
        name: "spade-raise-splinter",
        gate: |a| a.rebid.spade_raise_splinter.is_some(),
        entries: |_| {
            let mut entries = Vec::new();
            for opener_suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                let opening = call(1, Strain::from(opener_suit));
                for x in splinter_suits(opener_suit) {
                    let splinter = format!("P* {opening} - 1♠ - {}", call(4, Strain::from(x)));
                    for tail in ["-", "(X)"] {
                        let node = format!("{splinter} {tail}");
                        entries.extend(rows_of(Pattern::node(&node), responder_after_splinter(x)));
                        entries.extend(slam::rkcb_rows(&node, Suit::Spades));
                        // Opener showed the hand; responder's `4♠` declines.
                        entries.extend(rows_of(
                            Pattern::node(&format!("{node} 4♠ -")),
                            Rules::new().rule(Call::Pass, 0, hcp(0..)),
                        ));
                    }
                    if x != Suit::Hearts {
                        entries.extend(rows_of(
                            Pattern::up_to(&splinter, "4♥"),
                            responder_after_splinter_overcalled(),
                        ));
                    }
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
