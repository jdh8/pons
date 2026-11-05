//! The natural runout after they double our `1NT` overcall
//!
//! `(1t) 1NT (X)`, every seat, behind `agreements.defense.nt_overcall_doubled_runout`
//! (default on since 2026-11-05: won every cell plain and PD vs BBA, see
//! `docs/defensive-overcalls.md` § "The natural runout after they double our
//! `1NT` overcall").  The mirror of the instinct floor's runout of our
//! doubled `1NT` *opening* (`instinct.rs`, `one_nt_runout`), which the
//! defensive lane never reaches: the net floors it, and the BBA-relabel net
//! bids a phantom `3♦` there on five-card heart hands
//! (`docs/reseed-gap-handoff.md` §5 "Owed from the ship").  BBA itself plays
//! full systems on over the double (`2♦` transfer, `2♠`/`3♣` minor
//! transfers, `XX` 6+); this package keeps the house doctrine instead.
//!
//! | Advancer at `(1t) 1NT (X)` | Shows |
//! | --- | --- |
//! | `2x`, x ≠ t | five-plus x, under 8 HCP (the longer suit first, six-plus outranks five) |
//! | `2♣` / `2♦` | a 4-4 minor bust with no five-card major: the longer minor, ties to diamonds |
//! | `XX` | 7+ HCP, to play `1NTxx` or penalize their runout |
//! | `-` | the rest: weak with nothing to run to |
//!
//! The overcaller passes the escape, sits for their double of it, and in the
//! balancing seat (`(X) - -`) runs to its own five-plus suit.  Every node
//! after `XX`, and their bids over an escape, stay with the floor, which
//! reads the `XX` as 7+ off this package's projection.

use super::nt_landy::sit;
use super::*;

/// A weak advancer: under this many HCP it runs rather than sits
const RUNOUT_MAX_HCP: u8 = 8;

/// The 4-4 minor bust's escape: it floors the *other* minor at four, so it
/// is artificial to the alert invariant and is disclosed ("may be four-four
/// in the minors")
const RUNOUT_MINORS: Alert = Alert("nt-overcall:runout-minors");

/// From this many HCP the advancer redoubles (the instinct `runout_xx_min`)
const XX_MIN_HCP: u8 = 7;

/// A natural escape to each suit but `theirs` on a `weak` hand: five-plus at
/// 100, six-plus at 110, majors five above minors (the instinct runout's
/// weights)
fn escapes(theirs: Suit, mut rules: Rules, weak: Cons<impl Constraint + Clone + 'static>) -> Rules {
    for suit in Suit::ASC.into_iter().filter(|&suit| suit != theirs) {
        let bonus = if matches!(suit, Suit::Hearts | Suit::Spades) {
            5
        } else {
            0
        };
        let escape = Bid::new(2, Strain::from(suit));
        rules = rules
            .rule(escape, 100 + bonus, len(suit, 5..) & weak.clone())
            .rule(escape, 110 + bonus, len(suit, 6..) & weak.clone());
    }
    rules
}

/// The advancer at `(1t) 1NT (X)`: a total table
fn advancer_runout(theirs: Suit) -> Rules {
    use Suit::{Clubs, Diamonds, Hearts, Spades};
    let weak = hcp(..RUNOUT_MAX_HCP);
    let bust =
        weak.clone() & and([Clubs, Diamonds], 4..) & and([Hearts, Spades], ..5) & !len(theirs, 5..);
    let diamonds = Bid::new(2, Strain::Diamonds);
    let clubs = Bid::new(2, Strain::Clubs);
    let rules = escapes(theirs, Rules::new(), weak);
    let rules = match theirs {
        Diamonds => rules,
        Clubs => rules.rule(diamonds, 100, bust.clone()).alert(RUNOUT_MINORS),
        _ => rules
            .rule(
                diamonds,
                100,
                bust.clone() & at_least_as_long(Diamonds, Clubs),
            )
            .alert(RUNOUT_MINORS),
    };
    let rules = match theirs {
        Clubs => rules,
        Diamonds => rules.rule(clubs, 100, bust).alert(RUNOUT_MINORS),
        _ => rules
            .rule(clubs, 100, bust & !at_least_as_long(Diamonds, Clubs))
            .alert(RUNOUT_MINORS),
    };
    rules
        .rule(Call::Redouble, 120, hcp(XX_MIN_HCP..))
        .rule(Call::Pass, 0, hcp(0..))
}

/// The overcaller in the balancing seat (`(1t) 1NT (X) - -`): partner is weak
/// with nothing to run to, so run to our own five-plus suit, else sit
fn balancing_runout(theirs: Suit) -> Rules {
    escapes(theirs, Rules::new(), hcp(0..)).rule(Call::Pass, 0, hcp(0..))
}

/// The natural runout after they double our `1NT` overcall, when
/// `agreements.defense.nt_overcall_doubled_runout` is on
pub(super) fn nt_doubled_runout_package() -> Package {
    Package {
        name: "nt-doubled-runout",
        gate: |agreements| agreements.defense.nt_overcall_doubled_runout,
        entries: |_| {
            let mut entries = Vec::new();
            for theirs in Suit::ASC {
                let prefix = format!("P* ({}) 1NT (X)", Bid::new(1, Strain::from(theirs)));
                let node = |suffix: &str| Pattern::node(&format!("{prefix} {suffix}"));
                entries.extend(rows_of(Pattern::node(&prefix), advancer_runout(theirs)));
                entries.extend(rows_of(node("- -"), balancing_runout(theirs)));
                for suit in Suit::ASC.into_iter().filter(|&suit| suit != theirs) {
                    let escape = Bid::new(2, Strain::from(suit));
                    // Partner sits whether they pass or double the escape, from
                    // either seat, after the advancer's or the overcaller's run.
                    for seat in ["", "- - "] {
                        for tail in ["-", "(X)", "(X) - -", "- - (X)", "- - (X) - -"] {
                            let key = format!("{seat}{escape} {tail}");
                            entries.extend(rows_of(node(&key), sit()));
                        }
                    }
                }
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
