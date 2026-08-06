//! System-independent build helpers shared across bidding systems
//!
//! Call/suit helpers, guarded seat fan-out, and floor-attachment wiring that
//! have nothing to do with any one system.
//! [`american`][super::american] and [`dutch`][super::dutch] — and any future
//! system — import these from here rather than from each other.

use super::fallback::{Always, Fallback, Guard};
use super::instinct::instinct;
use super::trie::Classifier;
use super::{Pair, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain, Suit};
use std::sync::Arc;

/// A bid as a [`Call`], for trie keys
pub(in crate::bidding) const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

/// The other major
pub(in crate::bidding) const fn other_major(major: Suit) -> Suit {
    match major {
        Suit::Hearts => Suit::Spades,
        _ => Suit::Hearts,
    }
}

/// The other minor
pub(in crate::bidding) const fn other_minor(minor: Suit) -> Suit {
    match minor {
        Suit::Clubs => Suit::Diamonds,
        _ => Suit::Clubs,
    }
}

/// Attach a guarded fallback at `suffix` under every leading-pass prefix
// ponytail: `guard`/`fallback` stay by-value — callers pass a freshly built
// `Arc::new(ConcreteGuard)`, which unsize-coerces to `Arc<dyn Guard>` only on
// the move; a `&Arc<dyn Guard>` param would force a `let` binding at all ~20
// call sites for no real gain.
#[allow(clippy::needless_pass_by_value)]
pub(in crate::bidding) fn fallback_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    guard: Arc<dyn Guard>,
    fallback: Fallback,
) {
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.fallback_arc_at(&key, Arc::clone(&guard), fallback.clone());
    }
}

// ---------------------------------------------------------------------------
// Floor attachment
// ---------------------------------------------------------------------------

/// Attach any classifier as the floor on a pair's contested books
///
/// A root `Always` fallback on both contested books, shared through the
/// `Fallback`'s `Arc`.  Resolution reaches the root last, so the floor never
/// overrides an authored rule — it only catches the auctions that fall past all
/// of them.  Generic over the floor so [`american`][super::american::american]
/// (the BBA-distilled net) and
/// [`american_instinct`][super::american::american_instinct] (the deterministic
/// [`instinct`][crate::bidding::instinct()] ladder) share one wiring.
pub(in crate::bidding) fn with_floor<C: Classifier + 'static>(mut pair: Pair, floor: C) -> Pair {
    let contested = Fallback::classify(floor);
    pair.competitive.fallback_at(&[], Always, contested.clone());
    pair.defensive.fallback_at(&[], Always, contested);

    // Uncontested auctions never reach the contested floor, so an off-book
    // constructive sequence would pass out below a cold game (e.g. `1♦–1♥–1NT`
    // passed out on a balanced 16 opposite the 12–14 rebid).  Floor the
    // constructive book with the deterministic instinct ladder — the natural
    // milestone bidder reaches game or slam on those sequences.
    pair.constructive
        .fallback_at(&[], Always, Fallback::classify(instinct()));
    pair
}

/// Attach the deterministic instinct floor to a pair's contested books
pub(in crate::bidding) fn with_instinct_floor(pair: Pair) -> Pair {
    with_floor(pair, instinct())
}
