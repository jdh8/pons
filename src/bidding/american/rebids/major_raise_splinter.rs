//! Opener's splinter raise after `1x - 1M` and responder's answers
//!
//! Spades: gated by [`RebidKnobs::spade_raise_splinter`] (default `Some(4)`
//! since round 3, 2026-11-10; *Open work* item 9 (b) of
//! `docs/bba-gap-campaign.md`).  Opener's double jump in a new suit — `4♦` /
//! `4♥` after `1♣`, `4♣` / `4♥` after `1♦`, `4♣` / `4♦` after `1♥` — shows
//! four-card spade support, the `4♠` raise's points (17+ under the ladder)
//! and a singleton or void in the bid suit.  The knob's value is
//! the most HCP the short suit may hold: `Some(1)` is BBA's rule (a small
//! singleton or a void; it never splinters on a singleton K and rarely on an
//! A or Q), `Some(4)` any singleton.
//!
//! Hearts: the mirror over `1m - 1♥`, gated by
//! [`RebidKnobs::heart_raise_splinter`] (default `Some(4)` since 2026-11-11,
//! won 4/4 vs the blind `4♥`) — `3♠` /
//! `4♦` after `1♣`, `3♠` / `4♣` after `1♦` (BBA's book, 16–17), on the
//! `4♥` raise's points.  Everything below holds with `M` for `♠`.
//!
//! | Call | Wt | Meaning |
//! |------|----|---------|
//! | opener `4x` | 2.7 | 4+ ♠, the `4♠` range, 0–1 in `x` with at most the knob's HCP there |
//! | responder `4NT` | 1.5 | RKCB on the gate below |
//! | responder `4♠` | 1.0 | sign-off, everything else |
//! | opener over `4♠` | 0.0 | pass |
//!
//! Their double of the splinter takes the same table; their overcall of the
//! splinter below `4M` leaves responder bidding `4M`.
//!
//! # Responder's gate
//!
//! Points against the K/Q/J HCP wasted in `x` (the ace there is a working
//! card: round 1 lost on counting it), a fifth spade worth two points:
//!
//! | K/Q/J in `x` | four spades | five or more |
//! |--------------|-------------|--------------|
//! | 0 | 10+ | 8+ |
//! | 1–2 | 11+ | 9+ |
//! | 3 | 12+ | 10+ |
//! | 4–6 | 13+ | 11+ |
//!
//! # The grand rung
//!
//! The keycard tree is [`slam::grand_rkcb_rows_on`] with no strength gate of
//! the asker's own: opener has shown 17+, so all five keycards and a settled
//! trump queen bid `7M` on two side kings between the hands, or on one with
//! 13+ HCP in the asker's hand.
//!
//! # Census
//!
//! Spades only; the heart mirror reuses its gate and rung, measured as a
//! package rather than censused.
//! Double-dummy over 100M deals, 249,601 splinter auctions (2026-11-10,
//! `ab-results/spade-raise-splinter-census-2026-11-10/`).  Each cell of
//! the gate is where RKCB (five on three keycards, else six) outscores the
//! `4♠` sign-off at both vulnerabilities; the table is worth +1.87 / +2.21
//! IMPs per splinter auction (none / vul) over signing off always, against
//! +1.64 / +1.94 for the fit-blind points − ½ × waste ≥ 10 it replaced.  With
//! all five keycards and the queen held or made moot by a ninth trump, `7♠`
//! makes on 35% with no side king, 58% with one, 79% with two and 86% with
//! three, against a break-even near 56%; one king makes on 68% once the asker
//! holds 13+ HCP, 54% below.  The rung is worth +0.28 / +0.32 IMPs per splinter auction.

use super::*;
use crate::bidding::american::slam;
use crate::bidding::constraint::{has_ace, suit_hcp};
use crate::bidding::rows::Entry;

/// Splinter — opener's double jump in a new suit over `1x - 1M`, showing a
/// singleton or void there with four-card support
const SPLINTER: Alert = Alert("splinter");

/// The knob gating the splinter into `major`
fn knob(agreements: &Agreements, major: Suit) -> Option<u8> {
    match major {
        Suit::Spades => agreements.rebid.spade_raise_splinter,
        _ => agreements.rebid.heart_raise_splinter,
    }
}

/// Opener's splinters after opening `opener_suit` and hearing `1M`: each new
/// suit at its double jump (two above its cheapest level, which is 1 above
/// `M` and 2 below it)
fn splinters(opener_suit: Suit, major: Suit) -> impl Iterator<Item = (Suit, Bid)> {
    Suit::ASC
        .into_iter()
        .filter(move |&x| x != opener_suit && x != major)
        .map(move |x| (x, Bid::new(if x > major { 3 } else { 4 }, Strain::from(x))))
}

/// Append opener's splinters to a `1x - 1M` rebid table
///
/// `four` is the `4M` raise's point range; weight 270 sits above that raise
/// (260) so a short-suited hand splinters instead.
pub(super) fn with_major_raise_splinter(
    rules: Rules,
    opener_suit: Suit,
    major: Suit,
    four: core::ops::RangeFrom<u8>,
    agreements: &Agreements,
) -> Rules {
    let Some(short_hcp) = knob(agreements, major) else {
        return rules;
    };
    splinters(opener_suit, major).fold(rules, |rules, (x, bid)| {
        rules
            .rule(
                bid,
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

/// Responder facing the splinter in `x`: RKCB on the module doc's gate, else
/// the `4M` sign-off
///
/// The gate is spelled as a ladder of (points, wasted) bands so it projects
/// like any other constraint.
fn responder_after_splinter(x: Suit, major: Suit) -> Rules {
    let four = (points(10..) & waste(x, 0))
        | (points(11..) & waste(x, 2))
        | (points(12..) & waste(x, 3))
        | points(13..);
    let five = (points(8..) & waste(x, 0))
        | (points(9..) & waste(x, 2))
        | (points(10..) & waste(x, 3))
        | points(11..);
    Rules::new()
        .rule(
            Bid::new(4, Strain::Notrump),
            150,
            four | (len(major, 5..) & five),
        )
        .alert(slam::RKCB)
        .rule(Bid::new(4, Strain::from(major)), 100, hcp(0..))
}

/// The grand rung facing the splinter's 17+: no strength gate of the asker's
/// own, and one side king is enough from 13 HCP (see the module doc)
const GRAND: slam::Grand = slam::Grand {
    explore: 0,
    one_king: Some(13),
};

/// Responder's answers after every `1x - 1M - splinter`, both tails, their
/// overcall below `4M`, the RKCB trees, and opener's pass of the sign-off
fn continuations(major: Suit) -> Vec<Entry> {
    let response = call(1, Strain::from(major));
    let game = Bid::new(4, Strain::from(major));
    // Their overcall up to the strain below `4M`; `4♥` over `1♠`, `4♦` over `1♥`.
    let below_game = Bid::new(
        4,
        Strain::from(match major {
            Suit::Spades => Suit::Hearts,
            _ => Suit::Diamonds,
        }),
    );
    let openers = Suit::ASC.into_iter().filter(|&s| s < major);
    let mut entries = Vec::new();
    for opener_suit in openers {
        let opening = call(1, Strain::from(opener_suit));
        for (x, bid) in splinters(opener_suit, major) {
            let splinter = format!("P* {opening} - {response} - {}", Call::Bid(bid));
            for tail in ["-", "(X)"] {
                let node = format!("{splinter} {tail}");
                entries.extend(rows_of(
                    Pattern::node(&node),
                    responder_after_splinter(x, major),
                ));
                entries.extend(slam::grand_rkcb_rows_on(&node, major, GRAND));
                // Opener showed the hand; responder's `4M` declines.
                entries.extend(rows_of(
                    Pattern::node(&format!("{node} {} -", Call::Bid(game))),
                    Rules::new().rule(Call::Pass, 0, hcp(0..)),
                ));
            }
            if bid < below_game {
                entries.extend(rows_of(
                    Pattern::up_to(&splinter, &Call::Bid(below_game).to_string()),
                    Rules::new().rule(Call::Bid(game), 100, hcp(0..)),
                ));
            }
        }
    }
    entries
}

/// The spade splinter's package (`1x - 1♠ - 4y`)
pub(crate) fn spade_raise_splinter_continuations() -> Package {
    Package {
        name: "spade-raise-splinter",
        gate: |a| a.rebid.spade_raise_splinter.is_some(),
        entries: |_| continuations(Suit::Spades),
    }
}

/// The heart splinter's package (`1m - 1♥ - 3♠` / `4m`)
pub(crate) fn heart_raise_splinter_continuations() -> Package {
    Package {
        name: "heart-raise-splinter",
        gate: |a| a.rebid.heart_raise_splinter.is_some(),
        entries: |_| continuations(Suit::Hearts),
    }
}

#[cfg(test)]
mod tests;
