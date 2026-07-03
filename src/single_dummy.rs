//! Single-dummy trick estimation by Monte-Carlo double-dummy
//!
//! Double-dummy scoring sees all 52 cards; a declarer sees only 26 — their own
//! hand and dummy.  This module answers the question that view actually poses:
//! *how many tricks will this strain make?*  The standard, universally used
//! estimator (GIB, Q-plus, …) is Monte-Carlo double-dummy — deal the two hidden
//! defender hands at random, solve each layout double-dummy, and aggregate the
//! trick counts into a distribution.  A *true* single-dummy solver (each player
//! acting on only what it sees) is an imperfect-information search orders of
//! magnitude more expensive, and is intentionally out of scope: the DD-averaging
//! proxy is what the field uses.
//!
//! The build reuses the crate's existing pieces: the actor's two known hands are
//! pinned into a partial deal and the rest dealt by
//! [`fill_deals`][contract_bridge::deck::fill_deals] (as in
//! [`sampler`][crate::bidding::sample_layouts]); each layout is solved by
//! `Solver::solve_deals`; and the results fold straight into a
//! [`HistogramTable`][crate::stats::HistogramTable], whose
//! [`expected_tricks`][crate::stats::HistogramTable::expected_tricks] and
//! [`make_probability`][crate::stats::HistogramTable::make_probability] read out
//! the answer.

use crate::stats::HistogramTable;
use contract_bridge::deck::fill_deals;
use contract_bridge::{Builder, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use rand::Rng;

/// Monte-Carlo single-dummy trick distribution for a declaring side
///
/// Pins `declarer` at `seat` and `dummy` at `seat.partner()`, deals the two
/// hidden defender hands uniformly at random `n` times, solves each layout
/// double-dummy, and returns the per-(seat × strain) trick [`HistogramTable`].
/// The meaningful rows are the declaring side's (`seat` and `seat.partner()`);
/// the defenders' rows are solved too but describe *them* as declarer, which is
/// rarely what a single-dummy caller wants.
///
/// Read the result with [`HistogramTable::expected_tricks`] (mean tricks in a
/// strain) and [`HistogramTable::make_probability`] (fraction of layouts a
/// contract makes).  With `n == 0` the histogram is empty and those readers
/// return [`f64::NAN`].
///
/// # Panics
///
/// Panics if `declarer` and `dummy` are not disjoint thirteen-card hands (they
/// together form one side's 26 cards, so an overlap is a caller error).
#[must_use]
pub fn single_dummy(
    declarer: Hand,
    dummy: Hand,
    seat: Seat,
    rng: &mut impl Rng,
    n: usize,
) -> HistogramTable {
    let mut builder = Builder::new();
    builder[seat] = declarer;
    builder[seat.partner()] = dummy;
    let partial = builder
        .build_partial()
        .expect("declarer and dummy must be disjoint thirteen-card hands");

    // `fill_deals` is infinite, so `take(n)` always yields exactly `n` layouts —
    // no rejection, no starvation.  One solve per layout; the fold into a
    // `HistogramTable` is `FromIterator<TrickCountTable>` (free aggregation).
    let deals: Vec<_> = fill_deals(rng, partial).take(n).collect();
    Solver::lock()
        .solve_deals(&deals, NonEmptyStrainFlags::ALL)
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::{Bid, Level, Strain};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const fn four_spades() -> Bid {
        Bid {
            level: Level::new(4),
            strain: Strain::Spades,
        }
    }

    /// North + South hold every spade, ace, king, and top honor: no defender can
    /// ever win a trick, so North takes all thirteen in spades in *every* layout.
    /// The spade game is therefore a double-dummy lock and the defender who holds
    /// no trump can never make it.
    fn unbeatable_spade_fit() -> (Hand, Hand) {
        // North: all top spades + AK of every side suit.
        let north: Hand = "AKQJT98.AK.AK.AK".parse().expect("valid test hand");
        // South: the rest of the spades + QJ(T) of the side suits.
        let south: Hand = "765432.QJT.QJ.QJ".parse().expect("valid test hand");
        (north, south)
    }

    /// The lock makes on every deal: `make_probability` is exactly 1 and the mean
    /// trick count is a full thirteen, regardless of how the defenders' cards lie.
    #[test]
    fn unbeatable_fit_always_makes() {
        let (north, south) = unbeatable_spade_fit();
        let mut rng = StdRng::seed_from_u64(1);
        let hist = single_dummy(north, south, Seat::North, &mut rng, 16);

        assert_eq!(hist.expected_tricks(Seat::North, Strain::Spades), 13.0);
        assert_eq!(hist.make_probability(Seat::North, four_spades()), 1.0);
        // A defender holding no trump can never bring home a spade game.
        assert_eq!(hist.make_probability(Seat::East, four_spades()), 0.0);
    }

    /// Same seed and inputs reproduce the same histogram exactly (the solver never
    /// samples its own RNG).
    #[test]
    fn deterministic_given_a_seed() {
        let (north, south) = unbeatable_spade_fit();
        let mut rng_a = StdRng::seed_from_u64(7);
        let a = single_dummy(north, south, Seat::North, &mut rng_a, 12);
        let mut rng_b = StdRng::seed_from_u64(7);
        let b = single_dummy(north, south, Seat::North, &mut rng_b, 12);
        assert_eq!(a, b);
    }

    /// With no layouts the histogram is empty and the readers report `NaN`.
    #[test]
    fn empty_is_no_signal() {
        let (north, south) = unbeatable_spade_fit();
        let mut rng = StdRng::seed_from_u64(0);
        let hist = single_dummy(north, south, Seat::North, &mut rng, 0);
        assert!(hist.expected_tricks(Seat::North, Strain::Spades).is_nan());
        assert!(hist.make_probability(Seat::North, four_spades()).is_nan());
    }
}
