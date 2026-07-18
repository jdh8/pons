//! The Dutch system — a natural 2/1 built around a wide, non-forcing 1♣
//!
//! Dutch naturalises the Polish 1♣: a "lawyer's Polish Club" that keeps Polish
//! constructiveness while staying natural and less restricted.  The 1♣ opening
//! is non-forcing, 2+♣, 11–23 HCP, and hosts every strong hand that lacks the
//! strong-2♣ shape (the `1♣–1♦` relay sorts them out).  Otherwise it mirrors
//! `american()`: five-card majors, a 15–17 1NT, 2/1 game-forcing continuations.
//!
//! This is a **champion candidate**, built by copying `american()` and applying
//! the Dutch diff one measurable phase at a time.  Until it measures stronger,
//! it lives here as a sibling factory under the standard A/B discipline; see
//! `docs/dutch-system.md` for the campaign ledger.

mod openings;

use super::Pair;
use super::american::{bare_american, insert_uncontested, with_instinct_floor};

/// Build the Dutch system as one side's [`Pair`]
///
/// Bind it against the opponents' [`Family`][super::Family] with
/// [`Pair::against`] and seat two pairs with [`Table::of_pairs`][super::Table::of_pairs],
/// exactly like `american()`.
///
/// ```
/// use pons::dutch;
/// use pons::bidding::{Family, System};
/// use contract_bridge::auction::{Call, RelativeVulnerability};
/// use contract_bridge::{Bid, Strain};
///
/// let stance = dutch().against(Family::NATURAL);
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
    with_instinct_floor(bare_dutch())
}

/// The Dutch pair without the instinct floor — the authored books
///
/// Phase 1 reuses every american book wholesale and overrides only the
/// **opening table**: a full [`bare_american`] pair with its opening node
/// replaced by the Dutch wide-1♣ structure ([`openings::dutch_openings`]).
/// Every american continuation (responses, 1NT structure, rebids, competitive,
/// defensive) is reused as-is, so the A/B measures the opening diff alone.  The
/// response structure diverges in Phase 2; see `docs/dutch-system.md`.
fn bare_dutch() -> Pair {
    let mut pair = bare_american();
    // `insert_uncontested` re-keys at the opening auction (empty our-calls) for
    // every seat, and `Trie::insert_arc` replaces the classifier there — a clean
    // overwrite of american's opening table with the Dutch one.
    insert_uncontested(&mut pair.constructive.0, &[], openings::dutch_openings());
    pair
}

#[cfg(test)]
mod tests {
    use super::dutch;
    use crate::bidding::{Family, System};
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::{Bid, Strain};

    /// The Dutch opening for a first-seat hand.
    fn opens(hand: &str) -> Call {
        let stance = dutch().against(Family::NATURAL);
        let hand = hand.parse().unwrap();
        let logits = stance
            .classify(hand, RelativeVulnerability::NONE, &[])
            .expect("an opening decision");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(call, _)| call)
            .unwrap()
    }

    fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// The wide-1♣ opening partition (Phase 1): the load-bearing cases.
    #[test]
    fn opening_partition() {
        // The wide 1♣ hosts a strong balanced 23-count (american opens it 2♣).
        assert_eq!(opens("AKQ2.KQ3.KQ3.A32"), bid(1, Strain::Clubs));
        // Four-diamond hands open 1♣ — every one but the 4=4=4=1.
        assert_eq!(opens("KQ32.K32.KJ32.32"), bid(1, Strain::Clubs));
        // The singleton-club 4=4=4=1 is the one four-diamond hand that opens 1♦.
        assert_eq!(opens("KQ32.KQ32.Q432.2"), bid(1, Strain::Diamonds));
        // A real five-card diamond suit opens 1♦.
        assert_eq!(opens("A32.3.KQ432.K432"), bid(1, Strain::Diamonds));
        // 21–23 with a five-card major is the strong, artificial 2♣.
        assert_eq!(opens("AKQ32.AK3.AQ2.32"), bid(2, Strain::Clubs));
        // Rule of 20 gates the light end: a flat 12-count passes.
        assert_eq!(opens("KJ32.K32.K32.Q32"), Call::Pass);
    }
}
