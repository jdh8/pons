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

mod openings;
mod responses;

use super::Pair;
use super::agreements::Agreements;
use super::american::american_book;
use super::card::dutch_card;
use super::common::{with_floor, with_floor_v5, with_instinct_floor};
use super::features::{CompactConfig, Config, ConventionCard};
use super::rows::compile_into;

/// Build the Dutch system as one side's [`Pair`]
///
/// Bind it with [`Pair::against`] and seat two pairs with
/// [`Table::of_pairs`][super::Table::of_pairs], exactly like `american()`.
///
/// The contested books stand on
/// [`ConfiguredFloorBba`][super::neural_floor::ConfiguredFloorBba] under
/// [`dutch_card`], so the net is told it is bidding
/// Dutch rather than 2/1 — the v4 corpus covers both base systems.  Before the
/// configured net this was `NeuralFloorBba`, which had never seen a WJ card and
/// invented a diamond suit over the `1♦` relay.
///
/// ```
/// use pons::dutch;
/// use pons::bidding::System;
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = dutch().against();
/// let hand = "AQ32.K53.QJ4.A92".parse().unwrap(); // 16 HCP, balanced
/// let logits = stance
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
pub fn dutch() -> Pair {
    dutch_with_config(Config::symmetric(&dutch_card()))
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
pub fn dutch_with_config(config: Config) -> Pair {
    with_floor(dutch_book(), config)
}

/// [`dutch`] on the **compact-config (v5)** floor — opt-in, ungated
///
/// The Dutch twin of [`american_v5`][super::american::american_v5]; the
/// `dutch` dim is live in the v5 corpus (the `DEFAULT_CELLS` rotation), so
/// this cell is in distribution.  Unlike the American twin, this cell has
/// **no gate A/B of its own** — [`dutch`] stays on the v4 floor until one
/// runs (clone `scripts/ab-v5-floor.sh` with the dutch pair).
#[must_use]
pub fn dutch_v5() -> Pair {
    with_floor_v5(
        dutch_book(),
        CompactConfig::symmetric(&ConventionCard::capture(true)),
    )
}

/// The Dutch pair with the deterministic **instinct** floor (the pre-swap default)
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
pub fn dutch_instinct() -> Pair {
    with_instinct_floor(dutch_book())
}

/// Build the Dutch pair as the authored books alone, with no floor
///
/// Takes a full [`american_book`] pair and compiles two ungated row packages
/// onto its constructive trie. `dutch-openings` replaces the opening table;
/// `dutch-wide-one-club` carries the wide-1♣ responses, relay continuations,
/// and both natural minor-response structures. Across their 17 exact patterns,
/// eight replace inherited American classifiers and nine add Dutch-only nodes;
/// every other American continuation is reused verbatim. The rare 18–20 `1NT`
/// / 21–23 `2♦!` continuations stay American's — projection discloses their
/// strength; see `docs/dutch-system.md`.
#[must_use]
pub fn dutch_book() -> Pair {
    let agreements = Agreements::current();
    let mut pair = american_book();
    // Compile after American: these packages intentionally replace eight
    // inherited exact nodes and add nine Dutch-only continuations.
    compile_into(
        &mut pair.constructive.0,
        &agreements,
        &[openings::package(), responses::package()],
    );
    pair
}

#[cfg(test)]
mod tests;
