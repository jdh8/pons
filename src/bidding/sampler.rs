//! Constrained layout sampling — the inverse of [`Inferences`]
//!
//! [`Inferences`] reads an auction *forward* into per-player shown ranges (suit
//! lengths and points).  This module runs that backward: given the player to
//! act, their actual hand, and those ranges, it deals the *other three* hands at
//! random so each falls within everything the calls have shown.
//!
//! Because every range starts at [`Inference::unknown`] and only ever narrows
//! soundly (`Range::intersect`'s soundness-over-tightness), a hand that truly
//! made these calls always lands inside its range.  The sampled layouts are
//! therefore a sound population of "full deals this auction could have come
//! from" — the substrate a double-dummy search scores each candidate call over
//! (AI-bidder M2.1, the prerequisite for M2.2's call-EV evaluator).
//!
//! # Method
//!
//! Rejection sampling on top of [`fill_deals`][contract_bridge::deck::fill_deals]:
//! the actor's known thirteen cards are pinned into a partial deal, so every
//! draw deals only the other thirty-nine; a draw is kept iff LHO, partner, and
//! RHO each land within their shown ranges, and discarded otherwise.  This is
//! correct by construction — an accepted layout satisfies every range by the
//! acceptance test itself — and reuses the battle-tested dealer rather than
//! reinventing constrained shuffling.
//!
//! The shown ranges are deliberately loose, so acceptance is workable; a tight
//! or jointly-infeasible auction is bounded by an attempt cap (see
//! [`sample_layouts`]) and may return fewer layouts than requested rather than
//! loop forever.  A smarter importance sampler can replace the rejection loop
//! later if EV throughput demands it; the signature would not change.

use super::constraint::point_count;
use super::inference::{Inference, Inferences};
use contract_bridge::deck::fill_deals;
use contract_bridge::{Builder, FullDeal, Hand, Seat, Suit};
use rand::Rng;

/// Random deals tried per requested layout before giving up
///
/// Rejection sampling needs roughly `1 / acceptance` draws per kept layout, so
/// the total budget is `n * MAX_ATTEMPTS_PER_LAYOUT`.  The cap exists only to
/// terminate auctions whose ranges no hand can satisfy; for the loose ranges
/// [`Inferences`] actually produces it is rarely approached.
const MAX_ATTEMPTS_PER_LAYOUT: usize = 256;

/// Deal up to `n` full layouts consistent with what an auction has shown
///
/// `hand` is the actor's own thirteen cards and `seat` their absolute seat;
/// both are held fixed while the other three hands are dealt at random so that
/// LHO, partner, and RHO each fall within their [`Inferences`] ranges (which are
/// relative to `seat`, the side to act).  `rng` is the caller's — the model
/// never samples, so the learned floor stays deterministic (invariant §0.5).
///
/// Returns at most `n` layouts.  Fewer (possibly none) means the attempt budget
/// of `n * 256` draws ran out first, which happens only when the shown ranges
/// are tight or jointly infeasible given `hand`; a caller should read a short
/// result as a weak or absent signal, not an error.
// ponytail: the `build_partial` expect cannot fire — one hand placed in an
// otherwise empty builder is always a valid partial deal.
#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn sample_layouts(
    hand: Hand,
    seat: Seat,
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<FullDeal> {
    let mut out = Vec::with_capacity(n);
    if n == 0 {
        return out;
    }

    let mut builder = Builder::new();
    builder[seat] = hand;
    let partial = builder
        .build_partial()
        .expect("one thirteen-card hand is a valid partial deal");

    let budget = n.saturating_mul(MAX_ATTEMPTS_PER_LAYOUT);
    for deal in fill_deals(rng, partial).take(budget) {
        if within_ranges(&deal, seat, inferences) {
            out.push(deal);
            if out.len() == n {
                break;
            }
        }
    }
    out
}

/// Whether LHO, partner, and RHO in `deal` each fall within their shown ranges
///
/// The actor's own hand was pinned, so it is consistent by construction and is
/// not re-checked.
fn within_ranges(deal: &FullDeal, seat: Seat, inferences: &Inferences) -> bool {
    [
        (seat.lho(), inferences.lho()),
        (seat.partner(), inferences.partner()),
        (seat.rho(), inferences.rho()),
    ]
    .into_iter()
    .all(|(other, shown)| hand_within(deal[other], shown))
}

/// Whether a hand falls within one player's shown length and point ranges
fn hand_within(hand: Hand, shown: &Inference) -> bool {
    let lengths_fit = Suit::ASC.into_iter().all(|suit| {
        // SAFETY: a suit length is at most 13, so the cast cannot truncate.
        #[allow(clippy::cast_possible_truncation)]
        let length = hand[suit].len() as u8;
        shown.length(suit).contains(length)
    });
    lengths_fit && shown.points.contains(point_count(hand))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::context::Context;
    use contract_bridge::auction::{Call, RelativeVulnerability};
    use contract_bridge::deck::full_deal;
    use contract_bridge::{Bid, Level, Strain};
    use proptest::prelude::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    /// Inferences relative to the side to act, read from an auction
    fn inferences(auction: &[Call]) -> Inferences {
        Inferences::read(&Context::new(RelativeVulnerability::NONE, auction))
    }

    /// A fixed hand short in hearts, so an RHO who must hold 5+ hearts is easy
    /// to satisfy and the sampler reaches its requested count quickly.
    fn short_heart_actor() -> Hand {
        "AKQ32.32.AKQ2.32".parse().expect("valid test hand")
    }

    /// Soundness: every sampled layout keeps the actor's hand fixed and places
    /// the other three within their shown ranges.  Holds vacuously when the
    /// draw is infeasible, so it is robust to a hand that crowds out a range.
    #[test]
    fn sampled_layouts_respect_ranges() {
        let actor = Seat::North;
        // RHO opened 1H (5+ hearts, 12-21); LHO and partner are unconstrained.
        let inf = inferences(&[bid(1, Strain::Hearts)]);

        proptest!(|(seed in any::<u64>())| {
            let mut rng = StdRng::seed_from_u64(seed);
            let hand = full_deal(&mut rng)[actor];
            for deal in sample_layouts(hand, actor, &inf, &mut rng, 4) {
                prop_assert_eq!(deal[actor], hand);
                for (other, shown) in [
                    (actor.lho(), inf.lho()),
                    (actor.partner(), inf.partner()),
                    (actor.rho(), inf.rho()),
                ] {
                    for suit in Suit::ASC {
                        #[allow(clippy::cast_possible_truncation)]
                        let length = deal[other][suit].len() as u8;
                        prop_assert!(shown.length(suit).contains(length));
                    }
                    prop_assert!(shown.points.contains(point_count(deal[other])));
                }
            }
        });
    }

    /// A richer auction whose constraints land on more than one player.
    #[test]
    fn respects_a_developed_auction() {
        let actor = Seat::North;
        // Partner opened 1H, then RHO overcalled 1S (5+ spades, 8+).  Inferences
        // reads partner's opening and RHO's overcall; we sample around them.
        let auction = [bid(1, Strain::Hearts), bid(1, Strain::Spades)];
        let inf = inferences(&auction);
        let mut rng = StdRng::seed_from_u64(7);
        let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 16);

        assert!(!layouts.is_empty(), "the auction is feasible");
        for deal in &layouts {
            let partner = deal[actor.partner()];
            assert!(partner[Suit::Hearts].len() >= 5);
            assert!(inf.partner().points.contains(point_count(partner)));
            let rho = deal[actor.rho()];
            assert!(rho[Suit::Spades].len() >= 5);
            assert!(inf.rho().points.contains(point_count(rho)));
        }
    }

    /// The opener's shown shape and strength are honored on every layout.
    #[test]
    fn opener_constraint_is_enforced() {
        let actor = Seat::North;
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let mut rng = StdRng::seed_from_u64(1);
        let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 32);

        assert_eq!(layouts.len(), 32, "a 1H opening is easy to satisfy");
        for deal in &layouts {
            let opener = deal[actor.rho()];
            assert!(opener[Suit::Hearts].len() >= 5);
            assert!((12..=21).contains(&point_count(opener)));
        }
    }

    /// Coverage: the dealt population is not degenerate — both a constrained and
    /// an unconstrained seat take a spread of shapes across samples.
    #[test]
    fn coverage_is_not_degenerate() {
        let actor = Seat::North;
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let mut rng = StdRng::seed_from_u64(99);
        let layouts = sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 40);

        // RHO's heart length (constrained to 5+) still varies; LHO is free.
        let rho_hearts: std::collections::HashSet<usize> = layouts
            .iter()
            .map(|deal| deal[actor.rho()][Suit::Hearts].len())
            .collect();
        let lho_spades: std::collections::HashSet<usize> = layouts
            .iter()
            .map(|deal| deal[actor.lho()][Suit::Spades].len())
            .collect();
        assert!(rho_hearts.len() >= 2, "constrained seat should still vary");
        assert!(lho_spades.len() >= 3, "free seat should vary widely");
    }

    /// An infeasible auction terminates within the budget and returns nothing,
    /// rather than looping forever.
    #[test]
    fn infeasible_auction_returns_empty() {
        let actor = Seat::North;
        // RHO opened 1H, demanding 5+ hearts, but the actor holds nine of them,
        // leaving only four in the deck — no layout can satisfy the opening.
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let hoard: Hand = "32.AKQJT9876.2.2".parse().expect("valid test hand");
        assert_eq!(hoard[Suit::Hearts].len(), 9);
        let mut rng = StdRng::seed_from_u64(5);

        let layouts = sample_layouts(hoard, actor, &inf, &mut rng, 5);
        assert!(layouts.is_empty());
    }

    /// Requesting zero layouts samples nothing.
    #[test]
    fn zero_request_is_empty() {
        let actor = Seat::North;
        let inf = inferences(&[bid(1, Strain::Hearts)]);
        let mut rng = StdRng::seed_from_u64(0);
        assert!(sample_layouts(short_heart_actor(), actor, &inf, &mut rng, 0).is_empty());
    }
}
