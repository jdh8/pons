//! System-independent build helpers shared across bidding systems
//!
//! Call/suit helpers, guarded seat fan-out, and floor-attachment wiring that
//! have nothing to do with any one system.
//! [`american`][super::american] and [`dutch`][super::dutch] — and any future
//! system — import these from here rather than from each other.

use super::agreements::Agreements;
use super::fallback::{Always, Fallback, Guard};
use super::features::{CompactConfig, Config};
use super::instinct::instinct;
use super::neural_floor::{ConfiguredFloorBba, ConfiguredFloorV6};
use super::{Rules, System, Trie};
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

/// Attach one ladder and one contested floor to a system's three books
///
/// A root `Always` fallback on both contested books, shared through the
/// `Fallback`'s `Arc`.  Resolution reaches the root last, so the floor never
/// overrides an authored rule — it only catches the auctions that fall past all
/// of them.
///
/// Uncontested auctions never reach the contested floor, so an off-book
/// constructive sequence would pass out below a cold game (e.g. `1♦ - 1♥ - 1NT`
/// passed out on a balanced 16 opposite the 12–14 rebid).  The constructive
/// book therefore gets `ladder` — the natural milestone bidder reaches game or
/// slam on those sequences.
///
/// `ladder` is a parameter rather than a fresh [`instinct()`] because the
/// contested floor needs the *same* one: [`ConfiguredFloorBba`] delegates forced
/// situations to it, and [`instinct()`] itself reads the pinned profile at
/// build time (its RKCB fields pick the kickback ladder over the plain one).
/// Two independent builds could disagree; one `Arc` cannot.
fn with_floors(mut system: System, ladder: &Arc<Rules>, contested: Fallback) -> System {
    system
        .competitive
        .fallback_at(&[], Always, contested.clone());
    system.defensive.fallback_at(&[], Always, contested);
    system
        .constructive
        .fallback_at(&[], Always, Fallback::Classify(ladder.clone()));
    system
}

/// Attach the card-input (v4) BBA-distilled floor to a system's contested books
///
/// This remains the explicit card-input v4 entry point behind
/// [`american_with_config`][super::american::american_with_config] and
/// [`dutch_with_config`][super::dutch::dutch_with_config].
pub(in crate::bidding) fn with_floor(
    system: System,
    config: Config,
    agreements: &Agreements,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorBba::new(config, Arc::clone(&ladder)));
    with_floors(system, &ladder, contested)
}

/// Attach the shipped compact-config v6 floor and honest evaluator twin.
pub(in crate::bidding) fn with_floor_v6(
    system: System,
    compact: CompactConfig,
    agreements: &Agreements,
) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::classify(ConfiguredFloorV6::new(compact, Arc::clone(&ladder)));
    with_floors(system, &ladder, contested)
}

/// Attach the deterministic instinct floor to a system's contested books
///
/// The fully-disclosable reference wiring: one ladder on all three books.
pub(in crate::bidding) fn with_instinct_floor(system: System, agreements: &Agreements) -> System {
    let ladder = Arc::new(instinct(agreements));
    let contested = Fallback::Classify(ladder.clone());
    with_floors(system, &ladder, contested)
}
