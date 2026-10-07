//! Responder's weak answers to opener's reverse after a one-level response
//!
//! The four reverses of [`super::extras_ladder`] — `1♣ - 1♥ - 2♦`, `1♣ - 1♠ -
//! 2♦`, `1♣ - 1♠ - 2♥`, `1♦ - 1♠ - 2♥` (5+ in the minor, 4+ in the reverse
//! suit, 17+) — had no responder table, and the net floor **passes** the
//! forcing reverse on a weak hand.  Gated by
//! [`RebidKnobs::reverse_weak_responses`].
//!
//! The table claims only hands of at most 7 HCP **with somewhere to go** —
//! a six-card suit to rebid, four of the reverse suit, three of opener's
//! minor — and rejects the rest, which fall through to the floor.  Measured
//! against the floor (seed 1791363144, the fourth-suit structure of BBA's
//! book authored in full), its game and slam bids on 8+ HCP beat HCP-banded
//! rules — its `3NT` / `6NT` / `7NT` jumps make — and the fourth suit itself
//! priced at zero, while every gain was a weak hand with a fit the floor had
//! passed.  A natural weak `2NT` on the misfit (BBA's book: 6–8) read slightly
//! negative over four seeds against the floor's pass, which BBA's book also
//! lists at 6–8, so the misfit stays the floor's.  Over the weak raise of the
//! reverse suit opener passes below 19 HCP; its continuations over the other
//! weak calls are the floor's.
//!
//! The table is registered behind a guard that admits only an **undisturbed**
//! auction.  A partial table is unsound under a systems-on rebase (their
//! double stripped, the node found, the rejection answered as Pass — the
//! core's measured default, `docs/archive/next-steps-done.md` 2026-10-03):
//! on seed 1791364137 three such passes of a forcing reverse cost −59 IMPs,
//! the size of the whole gain.  Guarded, a contested reverse is the floor's.

use super::*;
use crate::bidding::Context;
use crate::bidding::fallback::{described_guard, guard};

/// The one reverse node: opener's `minor`, responder's one-level `response`
/// and the reverse suit `reverse`
struct Reverse {
    minor: Suit,
    response: Suit,
    reverse: Suit,
}

impl Reverse {
    /// The four reverses the extras ladder authors
    const ALL: [Self; 4] = [
        Self::new(Suit::Clubs, Suit::Hearts, Suit::Diamonds),
        Self::new(Suit::Clubs, Suit::Spades, Suit::Diamonds),
        Self::new(Suit::Clubs, Suit::Spades, Suit::Hearts),
        Self::new(Suit::Diamonds, Suit::Spades, Suit::Hearts),
    ];

    const fn new(minor: Suit, response: Suit, reverse: Suit) -> Self {
        Self {
            minor,
            response,
            reverse,
        }
    }

    /// The key, `P* 1m - 1x - 2y`; the node is its completion by RHO's pass
    fn key(&self) -> String {
        format!(
            "P* {} - {} - {}",
            call(1, Strain::from(self.minor)),
            call(1, Strain::from(self.response)),
            call(2, Strain::from(self.reverse)),
        )
    }

    /// Responder's node, admitted only when they have not acted
    fn node(&self) -> Pattern {
        Pattern::guarded(
            &self.key(),
            "-",
            described_guard(
                "- (undisturbed)",
                guard(|context: &Context<'_>, suffix: &[Call]| {
                    context.undisturbed() && suffix == [Call::Pass]
                }),
            ),
        )
    }
}

/// Responder's weak call over the reverse, at most 7 HCP with a place to go
///
/// Deliberately partial: an 8+ HCP hand, or a weak misfit, matches no rule
/// and is the floor's.
///
/// | Call | Wt   | Meaning |
/// |------|------|---------|
/// | 2x   | 1.1  | Weak rebid of responder's suit, 6+ cards |
/// | 3y   | 1.05 | Weak raise of the reverse suit, 4+ cards |
/// | 3m   | 1.0  | Weak preference to opener's minor, 3+ cards |
fn responder_over_reverse(r: &Reverse) -> Rules {
    // ponytail: HCP-only bands; the floor's shape evaluation owns 8+.
    Rules::new()
        .rule(
            Bid::new(2, Strain::from(r.response)),
            110,
            len(r.response, 6..) & hcp(..=7),
        )
        .rule(
            Bid::new(3, Strain::from(r.reverse)),
            105,
            len(r.reverse, 4..) & hcp(..=7),
        )
        .rule(
            Bid::new(3, Strain::from(r.minor)),
            100,
            len(r.minor, 3..) & hcp(..=7),
        )
}

/// Opener's call over the weak raise of the reverse suit
///
/// The floor bid `3NT` or `4♦` on 17–18 opposite the raise and went down;
/// BBA passes there below 19.
///
/// | Call | Wt  | Meaning |
/// |------|-----|---------|
/// | 4♥   | 1.1 | Game in the raised major, 19+ |
/// | Pass | 1.0 | At most 18 HCP |
/// | 3NT  | 0.9 | 19+, the reverse suit a minor |
fn opener_over_weak_raise(r: &Reverse) -> Rules {
    let mut rules = Rules::new();
    if r.reverse == Suit::Hearts {
        rules = rules.rule(Bid::new(4, Strain::Hearts), 110, hcp(19..));
    }
    rules
        .rule(Call::Pass, 100, hcp(..=18))
        .rule(Bid::new(3, Strain::Notrump), 90, hcp(19..))
}

/// Responder's weak answers over each reverse and opener's call over the raise
pub(crate) fn reverse_response_continuations() -> Package {
    Package {
        name: "reverse-weak-responses",
        gate: |a| a.rebid.reverse_weak_responses && a.decision.reading.opener_extras_ladder,
        entries: |agreements| {
            let mut entries = Vec::new();
            for r in &Reverse::ALL {
                // Under Odwrotka `1♣ - 1M - 2♦` is the artificial reverse.
                if agreements.rebid.odwrotka
                    && r.minor == Suit::Clubs
                    && r.reverse == Suit::Diamonds
                {
                    continue;
                }
                entries.extend(rows_of(r.node(), responder_over_reverse(r)));
                let raised = format!("{} - {} -", r.key(), call(3, Strain::from(r.reverse)));
                entries.extend(rows_of(Pattern::node(&raised), opener_over_weak_raise(r)));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
