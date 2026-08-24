//! The Dutch system — a natural 2/1 built around a wide, non-forcing 1♣
//!
//! Dutch naturalises the Polish 1♣: a "lawyer's Polish Club" that keeps Polish
//! constructiveness while staying natural and less restricted.  The 1♣ opening
//! is non-forcing, 2+♣, 11–23 HCP, and hosts every strong hand that lacks the
//! strong-2♣ shape (the `1♣ - 1♦` relay sorts them out).  Otherwise it mirrors
//! `american()`: five-card majors, a 15–17 1NT, 2/1 game-forcing continuations.
//!
//! This is a **champion candidate**, built by copying `american()` and applying
//! the Dutch diff one measurable phase at a time.  Until it measures stronger,
//! it lives here as a sibling factory under the standard A/B discipline; see
//! `docs/dutch-system.md` for the campaign ledger.

mod multi;
mod openings;
mod responses;

use super::System;
use super::agreements::Agreements;
use super::common::{with_floor, with_floor_v6, with_instinct_floor};
use super::features::{CompactConfig, Config, ConventionCard};
use super::rows::compile_into;

/// Build the Dutch system as one side's [`System`]
///
/// Bind it with [`System::bind`] and seat two systems with
/// [`Table::of_systems`][super::Table::of_systems], exactly like `american()`.
///
/// The contested books stand on the compact v6 floor retrained on the live
/// authored reading. Its regime input identifies Dutch rather than 2/1.
///
/// ```
/// use pons::dutch_default;
/// use pons::bidding::Bidder;
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let partnership = dutch_default().bind();
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = partnership
///     .classify(hand, RelativeVulnerability::NONE, &[])
///     .expect("an opening decision");
/// let best = (&logits.0)
///     .into_iter()
///     .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
///     .map(|(call, _)| call)
///     .unwrap();
/// assert_eq!(best, Call::Bid(Bid::new(1, Strain::Notrump)));
/// ```
#[must_use]
pub fn dutch(agreements: &Agreements) -> System {
    with_floor_v6(
        book(agreements),
        CompactConfig::symmetric(&ConventionCard::capture(agreements, true)),
        agreements,
    )
}

/// [`dutch`] on the shipped agreements — see
/// [`american_default`][super::american::american_default]
#[must_use]
pub fn dutch_default() -> System {
    dutch(&Agreements::default())
}

/// [`dutch`] against a **declared** opponent — the mixed table
///
/// The Dutch twin of
/// [`american_with_config`][super::american::american_with_config], and it
/// carries the same caveat: `config` is taken verbatim while the **book still
/// comes from the live knobs**, so a card claiming an agreement the rules do not
/// play is a misdisclosure to the net that nothing checks.
///
/// This is what an american-vs-dutch table needs.  Seating bare [`dutch`]
/// against bare `american()` declares *both* sides symmetric, so each net is
/// told the opposition plays its own system — false at every seat.
#[must_use]
pub fn dutch_with_config(agreements: &Agreements, config: Config) -> System {
    with_floor(book(agreements), config, agreements)
}

/// [`dutch`] against a declared opponent, on the shipped v6 floor.
#[must_use]
pub fn dutch_with_card(agreements: &Agreements, theirs: &ConventionCard) -> System {
    with_floor_v6(
        book(agreements),
        CompactConfig::new(&ConventionCard::capture(agreements, true), theirs),
        agreements,
    )
}

/// Alias of [`dutch`], whose v6 floor shipped on the Phase-5 gate.
#[must_use]
pub fn dutch_v6(agreements: &Agreements) -> System {
    dutch(agreements)
}

/// The Dutch system with the deterministic **instinct** floor (the pre-swap default)
///
/// Exactly [`dutch`] but for the floor: the BBA-distilled
/// [`ConfiguredFloorBba`][super::neural_floor::ConfiguredFloorBba] gives way to the
/// deterministic
/// [`instinct`][crate::bidding::instinct()] ladder.  Mirrors
/// [`american_instinct`][crate::american_instinct] — the fully-disclosable
/// reference, and the fixed baseline the Dutch campaign's floor A/Bs anchor on.
///
/// The floor is the *only* difference; both share the same authored books.
#[must_use]
pub fn dutch_instinct(agreements: &Agreements) -> System {
    with_instinct_floor(book(agreements), agreements)
}

/// [`dutch_instinct`] on the shipped agreements — see
/// [`american_default`][super::american::american_default]
#[must_use]
pub fn dutch_instinct_default() -> System {
    dutch_instinct(&Agreements::default())
}

/// Build the Dutch system as the authored books alone, with no floor
///
/// Takes a full [`american_book`][super::american::american_book] system and compiles two ungated row packages
/// onto its constructive trie. `dutch-openings` replaces the opening table;
/// `dutch-wide-one-club` carries the wide-1♣ responses, relay continuations,
/// and both natural minor-response structures. Across their 17 exact patterns,
/// eight replace inherited American classifiers and nine add Dutch-only nodes;
/// every other American continuation is reused verbatim. The rare 18–20 `1NT`
/// / 21–23 `2♦!` continuations stay American's — projection discloses their
/// strength; see `docs/dutch-system.md`.
#[must_use]
pub fn dutch_book(agreements: &Agreements) -> System {
    book(agreements)
}

/// [`dutch_book`] on the shipped agreements — see
/// [`american_default`][super::american::american_default]
#[must_use]
pub fn dutch_book_default() -> System {
    dutch_book(&Agreements::default())
}

/// [`dutch_book`] on an explicit capture — see [`american::book`][super::american::book]
pub(in crate::bidding) fn book(agreements: &Agreements) -> System {
    let agreements = *agreements;
    let mut system = super::american::book(&agreements);
    // Compile after American: these packages intentionally replace eight
    // inherited exact nodes and add nine Dutch-only continuations.
    compile_into(
        &mut system.constructive.0,
        &agreements,
        &[openings::package(), responses::package(), multi::package()],
    );
    // The mirror book American attached is American; Dutch's must carry the
    // Dutch packages, so re-attach ours over it.  Both are `None` whenever
    // nothing is declared, which is every default build.
    match super::common::mirror_agreements(&agreements) {
        Some(mirror) => system.with_mirror(book(&mirror)),
        None => system,
    }
}

#[cfg(test)]
mod tests;
