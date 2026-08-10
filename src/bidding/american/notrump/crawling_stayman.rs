//! Crawling Stayman — `1NT - 2♣ - 2♦ - 2♥`, a weak two-suiter escape
//!
//! Responder's `2♥` over the diamond denial is a *pass-or-correct* signoff on
//! both majors rather than a natural heart bid.  Gated by
//! [`crawling_stayman`][crate::bidding::inference::ReadingProfile::crawling_stayman].

use super::*;

/// Crawling Stayman: a weak 2♣ on 4-4 majors *short in diamonds* (4414/4405)
///
/// The shapes garbage Stayman cannot escape — with ≤1 diamond, passing opener's
/// 2♦ would land in a singleton/void.  Crawling bids 2♣ anyway and crawls 2♦ to
/// 2♥ (see [`stayman_no_major_rebid`]).  4-4 majors with ≤1 diamond forces ≥4
/// clubs, so the 2♥ pass-or-correct (and opener's 3♣ flee) always finds a fit.
/// Weak only (`hcp(..8)`), disjoint from constructive 2♣ and the garbage tiers
/// (which need 3+ diamonds).  Same STAYMAN alert.  Empty when off.
pub(super) fn crawling_stayman_rule(agreements: &Agreements) -> Rules {
    if !agreements.decision.reading.crawling_stayman() {
        return Rules::new();
    }
    Rules::new()
        .rule(
            Bid::new(2, Strain::Clubs),
            150,
            len(Suit::Hearts, 4..=4)
                & len(Suit::Spades, 4..=4)
                & len(Suit::Diamonds, ..=1)
                & hcp(..8),
        )
        .alert(STAYMAN)
}

/// Opener's reply to the crawl (`1NT - 2♣ - 2♦ - 2♥`): drop-dead pass-or-correct
///
/// Opener denied both majors (≤3 each).  Pass the 4-3 heart fit; with only two
/// hearts correct to 2♠ (then ≥3 spades).  Short in *both* majors — only a
/// 5-card-minor 1NT can be 2-2 — flee to 3♣: responder is club-heavy (4414/4405),
/// so it is an 8-9 card fit, far better than a 4-2 major.
fn answer_crawling_stayman() -> Rules {
    Rules::new()
        .rule(
            Bid::new(3, Strain::Clubs),
            100,
            len(Suit::Hearts, ..3) & len(Suit::Spades, ..3),
        )
        .rule(
            Bid::new(2, Strain::Spades),
            100,
            len(Suit::Hearts, ..3) & len(Suit::Spades, 3..),
        )
        .rule(Call::Pass, 0, len(Suit::Hearts, 3..))
}

/// Crawling Stayman pass-or-correct continuation
pub(crate) fn crawling() -> Package {
    Package {
        name: "crawling-stayman",
        gate: |agreements| agreements.decision.reading.crawling_stayman(),
        entries: |_| {
            rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ - 2♥ -"),
                answer_crawling_stayman(),
            )
        },
    }
}

#[cfg(test)]
mod tests;
