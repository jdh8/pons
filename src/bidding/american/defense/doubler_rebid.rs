//! The doubler's rebid over advancer's minimum response
//!
//! Partner doubled their one-of-a-suit opening and we answered at the cheapest
//! level; that answer is forced, so it says nothing.  This module gives the
//! doubler the five calls that say something back, which is what makes the
//! double's *strength* tiers disclosable at all: the double itself carries one
//! `TAKEOUT_DOUBLE` alert whatever its tier, so a hand too strong for a natural
//! overcall — the reason [`super::overcall`]'s seam split exists — is
//! indistinguishable from a bare 12-count until the doubler bids again.
//!
//! Only the **minimum** advance is authored: one level up when advancer's suit
//! ranks above theirs, two levels when it does not.  A jump advance is
//! invitational and keeps the floor, as does a notrump advance
//! ([`super::advance_2nt`]).
//!
//! The rungs overlap in strength and separate by shape, ordered by weight —
//! exact-and-disjoint is a constraint on the *direct* tiers, whose complements
//! the pass reading rides, not on rebids.  There is deliberately no `hcp(0..)`
//! catch-all: these are exact `Pattern::node` rows, so a hand no rung accepts
//! falls through to the floor rather than being forced onto a wrong rung.

use super::*;

/// The doubler's rebid table after `(1t) X - {advance} -`
///
/// Five rungs, cheapest-first in bridge terms and highest-weight-first in
/// resolution terms:
///
/// | call | shows |
/// | --- | --- |
/// | cheapest `NT` | 19–21 balanced with a stopper — too strong for the direct `1NT` overcall |
/// | new suit | a five-card suit and 15+ — the strong one-suiter that had to double |
/// | raise | four-card support and 15+ |
/// | cue of their suit | 17+, no clear direction — game-forcing, artificial |
/// | `Pass` | a minimum: the double promised nothing beyond 12 |
///
/// When advancer was forced to the two level the whole ladder costs a level of
/// room, so every floor rises by two HCP.
///
/// # Panics
///
/// Panics if `their_opening` or `advance` is a notrump bid; pass suit bids.
#[must_use]
fn doubler_rebid(their_opening: Bid, advance: Bid) -> Rules {
    let t = their_opening
        .strain
        .suit()
        .expect("their opening is always a suit bid");
    let a = advance
        .strain
        .suit()
        .expect("the authored advance is always a suit bid");
    // A two-level advance burns a level of room before the doubler speaks, so
    // every band's floor rises with it.
    let step = 2 * u8::from(advance.level.get() > their_opening.level.get());
    // The cheapest legal level for a strain over the advance.
    let level_of = |strain: Strain| {
        if strain > advance.strain {
            advance.level.get()
        } else {
            advance.level.get() + 1
        }
    };

    let mut rules = Rules::new()
        // Too strong for the direct 1NT overcall (15–18): that hand bids 1NT at
        // weight 150 and never reaches this node, so the cheapest notrump here
        // starts where the overcall stops.  The ceiling is spelled as the
        // complement of the next floor — forward projection carries point
        // floors but drops a plain range top.
        .rule(
            Bid::new(level_of(Strain::Notrump), Strain::Notrump),
            150,
            hcp((19 + step)..) & !hcp((22 + step)..) & balanced() & stopper_in_their_suits(),
        )
        // Four-card support for the suit partner was forced to name.
        .rule(
            Bid::new(level_of(advance.strain), advance.strain),
            130,
            support(4..) & hcp((15 + step)..),
        )
        // Game-forcing with no clear direction — artificial, so alerted.
        .rule(
            Bid::new(level_of(their_opening.strain), their_opening.strain),
            120,
            hcp((17 + step)..),
        )
        .alert(DOUBLER_CUE)
        // A minimum double has nothing to add.  Banded, not a catch-all: the
        // reading has to say *how* minimum, and hands above the band that fit
        // no rung stay with the floor.
        .rule(Call::Pass, 0, !hcp((15 + step)..));

    for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
        if suit == t || suit == a {
            continue;
        }
        let strain = Strain::from(suit);
        // A five-card side suit and extras: the one-suiter that was too strong
        // to overcall, now showing what it holds.  15+, not 17+ — a 14–16 with
        // a five-card minor doubles legitimately (the shape bar removes only
        // five-card majors and six-card minors) and would otherwise be stranded.
        rules = rules.rule(
            Bid::new(level_of(strain), strain),
            140,
            len(suit, 5..) & hcp((15 + step)..),
        );
    }
    rules
}

/// The doubler's rebids over every minimum advance of a one-of-a-suit takeout
/// double
///
/// Twelve nodes: four openings × three unbid suits, each at advancer's cheapest
/// legal level.  Gated with the seam split it completes — measuring the seam
/// against floor rebids would be measuring an incomplete convention.
pub(super) fn doubler_rebid_package() -> Package {
    Package {
        name: "doubler-rebid",
        gate: |agreements| agreements.defense.defensive_seam_split,
        entries: |_| {
            let mut entries = Vec::new();
            for t in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let opening = Bid::new(1, Strain::from(t));
                for a in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if a == t {
                        continue;
                    }
                    let strain = Strain::from(a);
                    let level = u8::from(strain < opening.strain) + 1;
                    let advance = Bid::new(level, strain);
                    entries.extend(rows_of(
                        Pattern::node(&format!("P* ({opening}) X - {advance} -")),
                        doubler_rebid(opening, advance),
                    ));
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
