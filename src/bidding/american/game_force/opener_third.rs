//! Opener's third call after responder sets trump at `1M - 2r - R - 3M`
//!
//! Gated by [`GameForceKnobs::opener_third`].  **On by
//! default** — but see the caveat, it is a deletion candidate blocked on a
//! floor capability, not a settled node.

use super::*;
use crate::bidding::american::slam;

/// Opener's third call after `1M - 2r - R - 3M`
///
/// Once trump has been set at three of the major, opener shows strength:
/// the 4NT key card ask on extras or a sign-off at four of the major.
///
/// No [`Pass`][Call::Pass] rule.
fn opener_third(major: Suit) -> Rules {
    let major_strain = Strain::from(major);
    Rules::new()
        .rule(call(4, Strain::Notrump), 100, points(15..))
        .alert(slam::RKCB)
        .rule(call(4, major_strain), 50, hcp(0..))
}

/// Opener's third-call table after responder agrees the opening major
pub(crate) fn opener_third_continuations() -> Package {
    Package {
        name: "two-over-one-opener-third",
        gate: |agreements| agreements.game_force.opener_third,
        entries: |_| {
            let mut entries = Vec::new();
            for major in [Suit::Spades, Suit::Hearts] {
                for resp in [Suit::Clubs, Suit::Diamonds, Suit::Hearts] {
                    if Strain::from(resp) >= Strain::from(major) {
                        continue;
                    }
                    let prefix = format!(
                        "P* {} - {} -",
                        call(1, Strain::from(major)),
                        call(2, Strain::from(resp)),
                    );
                    let three_major = Bid::new(3, Strain::from(major));
                    for rebid_call in distinct_calls(&opener_rebid(major, resp)) {
                        if let Call::Bid(rebid_bid) = rebid_call
                            && rebid_bid < three_major
                        {
                            entries.extend(rows_of(
                                Pattern::node(&format!("{prefix} {rebid_call} - {three_major} -")),
                                opener_third(major),
                            ));
                        }
                    }
                }
            }
            entries
        },
    }
}
