//! Crawling Stayman — `1NT - 2♣ - 2♦ - 2♥`, a weak two-suiter escape
//!
//! Responder's `2♥` over the diamond denial is a *pass-or-correct* signoff on
//! both majors rather than a natural heart bid.  Gated by
//! [`set_crawling_stayman`].

use super::*;

/// Crawling Stayman: a weak 2♣ on 4-4 majors *short in diamonds* (4414/4405)
///
/// The shapes garbage Stayman cannot escape — with ≤1 diamond, passing opener's
/// 2♦ would land in a singleton/void.  Crawling bids 2♣ anyway and crawls 2♦ to
/// 2♥ (see [`stayman_no_major_rebid`]).  4-4 majors with ≤1 diamond forces ≥4
/// clubs, so the 2♥ pass-or-correct (and opener's 3♣ flee) always finds a fit.
/// Weak only (`hcp(..8)`), disjoint from constructive 2♣ and the garbage tiers
/// (which need 3+ diamonds).  Same STAYMAN alert.  Empty when off.
pub(super) fn crawling_stayman_rule() -> Rules {
    if !crawling_stayman() {
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

thread_local! {
    /// Crawling Stayman: the superset of garbage Stayman for 4-4 majors *short in
    /// diamonds* (4414/4405).  Garbage needs a safe 2♦ landing (3+ diamonds), so it
    /// cannot escape with a singleton/void diamond; crawling bids 2♣ anyway and, if
    /// opener denies a major (2♦), *crawls* to 2♥ — both majors, pass-or-correct —
    /// rather than passing a doomed diamond partscore.  **On by default.**  See
    /// [`set_crawling_stayman`].
    static CRAWLING_STAYMAN: Cell<bool> = const { Cell::new(true) };
}

/// Author Crawling Stayman for books built *after* this call (thread-local; **on
/// by default**).
///
/// A weak 4-4-majors hand short in diamonds (4414/4405) bids 2♣ and, over opener's
/// 2♦ denial, crawls to 2♥ (pass-or-correct between the majors).  The strict
/// superset of garbage Stayman, which cannot escape such hands (it passes 2♦, a
/// singleton/void diamond "fit").
pub fn set_crawling_stayman(on: bool) {
    CRAWLING_STAYMAN.with(|cell| cell.set(on));
}

/// Whether Crawling Stayman is currently authored (read by the inference engine
/// too, to widen the 2♣ point range it reads)
pub(crate) fn crawling_stayman() -> bool {
    CRAWLING_STAYMAN.with(Cell::get)
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
        gate: crawling_stayman,
        entries: || {
            rows_of(
                Pattern::node("P* 1NT - 2♣ - 2♦ - 2♥ -"),
                answer_crawling_stayman(),
            )
        },
    }
}

#[cfg(test)]
mod tests;
