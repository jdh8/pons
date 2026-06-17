//! The Strawberry Polish Club bidding system (AI-bidder M4.3)
//!
//! [`polish_club()`] assembles a [`Pair`] for the author's *Strawberry Polish
//! Club* (<https://polish.club>, [source](https://github.com/jdh8/polish.club)),
//! a BTU-flavored Polish Club: an artificial forcing 1♣, five-card majors, a
//! strong 15–17 notrump, and a preemptive two-level (Ekren 2♣, Multi 2♦,
//! Muiderberg 2♥/2♠, unusual 2NT).  It is the **second** authored system in the
//! crate, ported from its written notes with the authoring toolchain
//! (`docs/ai-bidder/dsl-spec.md` and
//! [`verify`][mod@crate::bidding::verify]), and the source of the second description corpus.
//!
//! # A genuine Polish Club, not natural 2/1
//!
//! Unlike [`american`][super::american::american] — a
//! `NATURAL`-family 2/1 with a natural 1♦ and a strong artificial 2♣ — here 1♣
//! is the artificial small-club itself and the family is
//! [`Family::POLISH_CLUB`].
//!
//! # Scope (M4.3, first pass)
//!
//! The **Constructive** book is authored as a backbone — the openings and the
//! principal first responses — with the deep artificial relay tails (Checkback
//! Gladiator, Odwrotka, the strong-club relays) left to the
//! [`instinct`][super::instinct()] floor, which is attached to *all three* books
//! (including the constructive one, as in
//! [`american`][super::american::american]) so no uncontested
//! auction strands.  The Competitive and Defensive books are empty for now; the
//! floor answers those auctions until a later pass.
//!
//! [`instinct`][super::instinct()] stays the baseline and the floor; this is an
//! added system, never a removal.

use super::fallback::{Always, Fallback};
use super::instinct::instinct;
use super::trie::Classifier;
use super::{Competitive, Constructive, Family, Pair, Trie};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain};
use std::sync::Arc;

mod defense;
mod openings;
mod responses;

pub use openings::openings;

/// A bid as a [`Call`], for trie keys
const fn call(level: u8, strain: Strain) -> Call {
    Call::Bid(Bid::new(level, strain))
}

// ---------------------------------------------------------------------------
// Seat-fan helpers (mirrors `american`'s private plumbing)
// ---------------------------------------------------------------------------

/// Insert one classifier at `suffix` under every leading-pass prefix
fn insert_all_seats(
    book: &mut Trie,
    suffix: &[Call],
    max_passes: usize,
    rules: impl Classifier + 'static,
) {
    let shared: Arc<dyn Classifier> = Arc::new(rules);
    for n in 0..=max_passes {
        let key: Vec<Call> = core::iter::repeat_n(Call::Pass, n)
            .chain(suffix.iter().copied())
            .collect();
        book.insert_arc(&key, Arc::clone(&shared));
    }
}

/// Interleave one opposing pass after each of our calls (the constructive key)
fn uncontested(our_calls: &[Call]) -> Vec<Call> {
    our_calls
        .iter()
        .flat_map(|&call| [call, Call::Pass])
        .collect()
}

/// Insert a continuation table after our undisturbed `our_calls`, every seat
///
/// An empty `our_calls` registers an opening table.
fn insert_uncontested(book: &mut Trie, our_calls: &[Call], rules: impl Classifier + 'static) {
    insert_all_seats(book, &uncontested(our_calls), 3, rules);
}

// ---------------------------------------------------------------------------
// Assembly
// ---------------------------------------------------------------------------

/// Build the Strawberry Polish Club system as one side's [`Pair`]
///
/// Bind it against the opponents' [`Family`] with [`Pair::against`] for a
/// playable system, exactly like [`american`][super::american()].
///
/// ```
/// use pons::bidding::polish_club::polish_club;
/// use pons::bidding::{Family, System};
/// use contract_bridge::auction::RelativeVulnerability;
/// use contract_bridge::{Bid, Strain};
///
/// let stance = polish_club().against(Family::NATURAL);
/// let hand = "AQ5.KJ4.KQ72.K43".parse().unwrap(); // 18 HCP balanced
/// let logits = stance
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .filter(|(_, l)| l.is_finite())
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, pons::bidding::polish_club::polish_club_one_club());
/// ```
#[must_use]
pub fn polish_club() -> Pair {
    let mut pair = with_instinct_floor(bare_polish_club());
    // The deep relay continuations are not exhaustively authored, so — like the
    // strawberry 2/1 variant — also floor the constructive book.  Instinct's
    // unforced default is Pass, so weak uncontested auctions are unaffected.
    let floor = Fallback::classify(instinct());
    pair.constructive.fallback_at(&[], Always, floor);
    pair
}

/// The 1♣ opening call (a small spelling helper for doctests and tests)
#[must_use]
pub fn polish_club_one_club() -> Call {
    call(1, Strain::Clubs)
}

/// Build the Polish Club pair *without* the instinct floor: the bare books
///
/// The ablation handle for measuring the floor and the input to corpus export,
/// mirroring [`bare_american`][super::american::bare_american].  The
/// Competitive and Defensive books are empty in this first pass.
#[must_use]
pub fn bare_polish_club() -> Pair {
    let mut c = Constructive::new();

    openings::register(&mut c);
    responses::register(&mut c);

    Pair::new(
        Family::POLISH_CLUB,
        c,
        Competitive::new(),
        defense::defensive(),
    )
}

/// Attach the deterministic instinct floor to a pair's contested books
fn with_instinct_floor(mut pair: Pair) -> Pair {
    let floor = Fallback::classify(instinct());
    pair.competitive.fallback_at(&[], Always, floor.clone());
    pair.defensive.fallback_at(&[], Always, floor);
    pair
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::{Family, System};

    // The textbook-opening fixtures live in `tests/polish_club.rs`
    // (`textbook_openings_are_correct`); this is just the assembly smoke test.
    #[test]
    fn assembles_without_panic() {
        // `Pair::against` debug-asserts no constructive/competitive collision.
        let _ = polish_club().against(Family::NATURAL);
        let _ = bare_polish_club().against(Family::NATURAL);
    }
}
