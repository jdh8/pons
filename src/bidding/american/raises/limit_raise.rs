//! Limit-raise acceptance: `1M - 3M`
//!
//! Opener accepts, asks for keycards, or declines.  Gated by
//! [`ResponseKnobs::limit_raise_acceptance`], default on (+0.002/+0.002 IMPs/board
//! NV/vul — the whole win being the keycard ask at +4.4/+5.2 IMPs/divergent).

use super::*;

/// Opener's continuation after `1M - 3M -`: accept, ask, or
/// decline the limit raise
///
/// | Call | Meaning |
/// |---|---|
/// | 4NT | RKCB ask (19+) |
/// | 4M | Accept (13+) |
/// | Pass | Decline |
///
/// The accept sits at 13, not the textbook 14: the instinct floor's
/// raise-partner ladder already accepts at 13+ (`instinct.rs`, the
/// `(4, 13)` raise rung), and the 14/15-threshold experiments — which only
/// *under*-bid relative to that baseline — lost −4.6/−5.2 IMPs per divergent
/// board vulnerable (probe-limit-raise).  With a nine-card fit known, DD
/// prices the 23-combined game as a clear bid, so the authored value of this
/// node is the keycard ask (+5.2 IMPs/divergent), not the accept threshold.
#[must_use]
fn opener_after_limit_raise(major: Suit) -> Rules {
    let trump = Strain::from(major);
    // Opener's seat: the trump is the own five-card major, +5.
    Rules::new()
        // 4NT: RKCB ask.
        .rule(
            Bid::new(4, Strain::Notrump),
            150,
            support_points(major, 19..),
        )
        .alert(slam::RKCB)
        // 4M: accept.
        .rule(Bid::new(4, trump), 100, support_points(major, 13..))
        // Pass: decline.
        .rule(Call::Pass, 0, hcp(0..))
}

/// Limit-raise acceptance after `1M - 3M`, including its RKCB subtree
pub(crate) fn limit_raise_acceptance_continuations() -> Package {
    Package {
        name: "limit-raise-acceptance-continuations",
        gate: |a| a.response.limit_raise_acceptance,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Hearts, Suit::Spades] {
                let trump = Strain::from(major);
                let prefix = format!("P* {} - {} -", call(1, trump), call(3, trump));
                entries.extend(rows_of(
                    Pattern::node(&prefix),
                    opener_after_limit_raise(major),
                ));
                entries.extend(slam::rkcb_rows(&prefix, major));
            }
            entries
        },
    }
}

#[cfg(test)]
mod tests;
