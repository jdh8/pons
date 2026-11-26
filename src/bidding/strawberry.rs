//! Strawberry Polish Club (<https://polish.club/>)
//!
//! [`strawberry()`] assembles a [`System`] for jdh8's Polish Club: a forcing,
//! three-way `1♣`, a wide natural `1♦` (11–20), limited five-card majors
//! (11–16) and a strong notrump.  It is a system of its own, not a variant of
//! [`american()`][crate::bidding::american::american()]: the opening table and the `1♣`
//! structure are authored here from the book, one module per chapter.
//!
//! # What is authored
//!
//! | Chapter | Module | State |
//! | --- | --- | --- |
//! | Openings | [`openings`][crate::bidding::strawberry::openings] | the one level, `1NT` and the three-level preempts; the two-level openings, the gambling `3NT` and Namyats are owed |
//! | Polish `1♣` | `one_club` | responses, opener's rebids, part of round two; Checkback Gladiator and the contested tables are owed |
//! | `1♦`, `1♥`, `1♠` | — | **borrowed** from the house book (see below) |
//! | Strong `1NT` | — | the house structure, which the book shares |
//!
//! `docs/strawberry.md` is the ledger: the census against the book, the holes
//! ranked, and the measurement.
//!
//! # Borrowed chapters
//!
//! A chapter not yet authored is grafted from the house book (`BORROWED`):
//! the responses to that opening, everything below them, and the competitive
//! tables over it.  That keeps the system playable and makes a comparison
//! with [`american()`][crate::bidding::american::american()] price exactly the authored chapters.
//! It is not the book's meaning — `1M - 2♣` is a natural game force there,
//! where Strawberry's is a relay — so each chapter leaves the list in the
//! commit that authors it.  Nothing is grafted under `1♣`: the house tables
//! there read the opening as natural clubs, so the Polish `1♣` stands on its
//! own rows and the floor.
//!
//! # The other two books
//!
//! The defensive book is the house's, taken whole: what we play when they
//! open does not depend on what we open.  The mirror book — how we read
//! *their* auctions — is the house book too, since an undeclared opponent
//! does not play a Polish club.

use super::agreements::Agreements;
use super::american;
use super::common::{mirror_agreements, with_floor_v6, with_instinct_floor};
use super::features::{CompactConfig, ConventionCard};
use super::rows::compile_into;
use super::{Competitive, Constructive, System};
use contract_bridge::auction::Call;
use contract_bridge::{Bid, Strain};

mod one_club;
pub mod openings;

/// The openings whose chapters are still the house book's
///
/// Each is grafted in every seat, constructive and competitive.  `1NT` is on
/// the list for good: the book's notrump structure is the house's.
const BORROWED: [Strain; 4] = [
    Strain::Diamonds,
    Strain::Hearts,
    Strain::Spades,
    Strain::Notrump,
];

/// Build Strawberry Polish Club as one side's [`System`]
///
/// The contested books stand on the same floor [`american()`][crate::bidding::american::american()]
/// uses, under the same convention card: no slot of that card expresses a
/// Polish club, so the net is told nothing about it and reads our `1♣` from
/// the opening rule's own projection.  A board they open is therefore bid
/// exactly as [`american()`][crate::bidding::american::american()] bids it.
///
/// ```
/// use pons::bidding::Bidder;
/// use pons::bidding::agreements::Agreements;
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let partnership = pons::strawberry(&Agreements::default()).bind();
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
pub fn strawberry(agreements: &Agreements) -> System {
    with_floor_v6(
        book(agreements),
        CompactConfig::symmetric(&ConventionCard::capture(agreements)),
        agreements,
    )
}

/// [`strawberry`] on the deterministic [`instinct`][crate::bidding::instinct()] floor
///
/// The fully disclosable twin, and the arm that says what the learned floor
/// is worth under a book it was never trained on.
#[must_use]
pub fn strawberry_instinct(agreements: &Agreements) -> System {
    with_instinct_floor(book(agreements), agreements)
}

/// [`strawberry`] as the authored books alone, with no floor
///
/// A driver seating this passes whenever the books run out; it is what
/// `render-book` prints.
#[must_use]
pub fn strawberry_book(agreements: &Agreements) -> System {
    book(agreements)
}

fn book(agreements: &Agreements) -> System {
    let house = american::book(agreements);
    let mut constructive = Constructive::new();
    let mut competitive = Competitive::new();
    compile_into(
        &mut constructive,
        agreements,
        &[openings::package(), one_club::package()],
    );
    for strain in BORROWED {
        for passes in 0..=3 {
            let prefix: Vec<Call> = core::iter::repeat_n(Call::Pass, passes)
                .chain([Call::Bid(Bid::new(1, strain))])
                .collect();
            let collisions = constructive.graft(&prefix, &house.constructive, &prefix);
            debug_assert!(
                collisions.is_empty(),
                "borrowed chapter collides at {prefix:?}: {collisions:?}"
            );
            competitive.graft(&prefix, &house.competitive, &prefix);
        }
    }
    let mirror = match mirror_agreements(agreements) {
        Some(mirror) => american::book(&mirror),
        None => american::book(agreements),
    };
    System::new(constructive, competitive, house.defensive, *agreements).with_mirror(mirror)
}

#[cfg(test)]
mod tests;
