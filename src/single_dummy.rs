//! Single-dummy trick estimation by Monte-Carlo double-dummy
//!
//! Double-dummy scoring sees all 52 cards; a real player sees only their own.
//! This module prices the two views' gap at the two seats where it is known to
//! matter, both by the same Monte-Carlo double-dummy technique (sample hidden
//! layouts, solve each double-dummy, aggregate):
//!
//! - [`single_dummy()`] — **declarer's view before play**: how many tricks will
//!   this strain make, over random hidden defender hands?  The trick histogram
//!   answers contract-choice questions.
//! - [`single_dummy_leads`] / [`single_dummy_lead_tricks`] — **the opening
//!   leader's view**: pick the blind lead that maximizes expected defensive
//!   tricks over auction-consistent worlds, then play the actual deal
//!   double-dummy from that card.  This scores a *reached contract* under the
//!   one information asymmetry that dominates real-vs-DD results at partscore
//!   level (Pavlicek: 1NT makes 67.7% at the table vs 60.5% double-dummy).
//!
//! A *true* single-dummy solver (each player acting on only what it sees at
//! every trick) is an imperfect-information search orders of magnitude more
//! expensive, and is intentionally out of scope: the DD-averaging proxy is
//! what the field uses (GIB, Q-plus, …).
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

use crate::bidding::{Inferences, sample_layouts};
use crate::stats::HistogramTable;
use contract_bridge::deck::fill_deals;
use contract_bridge::{Builder, Card, FullDeal, Hand, Seat, Strain};
use ddss::{Board, CurrentTrick, NonEmptyStrainFlags, Objective, Play, Solver, Target, TrickCount};
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

/// Single-dummy opening lead and the double-dummy tricks it concedes
///
/// The information asymmetry that dominates real-vs-DD trick results at the
/// partscore level is the **opening lead**: a double-dummy defender always
/// finds the killing start, while a real leader leads blind (Pavlicek's actual-
/// vs-DD study: 1NT makes 67.7% at the table against 60.5% double-dummy, and
/// the gap tapers to zero as the level rises).  This scorer models exactly that
/// seam and nothing else: the leader — `declarer`'s left-hand opponent —
/// chooses a card *single-dummy*, maximizing mean defensive tricks over `n`
/// layouts consistent with what the auction showed, and the play thereafter is
/// double-dummy on the **actual** `deal`.
///
/// `inferences` must be read from the **leader's** perspective (e.g. via
/// [`Stance::infer`][crate::bidding::Stance::infer] on an auction prefix that
/// puts the leader on lead), so the sampled worlds respect both sides'
/// disclosures — an overcall directs partner's lead here in precisely the way
/// plain DD scoring erases.  If the reading is too tight to sample from, the
/// worlds are topped up with unconstrained layouts (a weak signal, not an
/// error).  One [`Target::Legal`] solve per world prices every candidate lead
/// at once; sequence equals share their listed score.
///
/// Returns the chosen lead and the declaring side's double-dummy tricks on
/// `deal` after it.  Ceiling, stated: play *after* trick one's first card is
/// still perfect on both sides, so declarer misguesses (the slam-level bias)
/// and later defensive signalling are not modelled.
///
/// # Panics
///
/// Panics if `n == 0` (a lead cannot be chosen over zero worlds) or if `deal`
/// is not a valid full deal containing the leader's thirteen cards.
#[must_use]
pub fn single_dummy_lead_tricks(
    deal: &FullDeal,
    strain: Strain,
    declarer: Seat,
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> (Card, TrickCount) {
    let question = LeadQuestion {
        deal: *deal,
        strain,
        declarer,
        inferences: *inferences,
    };
    single_dummy_leads(&[question], rng, n)
        .pop()
        .expect("one question yields one answer")
}

/// One opening-lead position for [`single_dummy_leads`]: the actual deal, the
/// contract's strain and declarer, and the auction reading from the leader's
/// perspective.
#[derive(Clone, Debug)]
pub struct LeadQuestion {
    /// The actual full deal the contract is played on
    pub deal: FullDeal,
    /// Trump strain of the contract
    pub strain: Strain,
    /// Declarer's absolute seat; the leader is `declarer.lho()`
    pub declarer: Seat,
    /// What the auction showed, read from the **leader's** perspective
    pub inferences: Inferences,
}

/// Batched [`single_dummy_lead_tricks`]: answer many opening-lead positions
/// with one pooled double-dummy call
///
/// Semantically identical to mapping [`single_dummy_lead_tricks`] over
/// `questions` with the same `rng` — same worlds, same leads, same tricks —
/// but every position's solves go into a **single** batched
/// [`Solver::solve_boards`], so one slow board no longer stalls a tiny batch:
/// wall-clock approaches total work over the pool width instead of the sum of
/// per-position maxima.  Prefer this whenever positions are known in advance
/// (a tournament scorer pricing thousands of auctions).
#[must_use]
pub fn single_dummy_leads(
    questions: &[LeadQuestion],
    rng: &mut impl Rng,
    n: usize,
) -> Vec<(Card, TrickCount)> {
    assert!(n > 0, "a lead needs at least one sampled world");
    // Per question: `n` sampled worlds then the actual deal, all as trick-one
    // [`Target::Legal`] solves.  A legal-solve score is the tricks the
    // *defense* takes after that lead with perfect play thereafter; the lead
    // is chosen from the sampled worlds only (no peeking at the real layout),
    // and the actual deal's entry converts the chosen lead into declarer
    // tricks as `13 − defensive tricks`.
    let mut objectives = Vec::with_capacity(questions.len() * (n + 1));
    for question in questions {
        let leader = question.declarer.lho();
        let hand = question.deal[leader];
        let mut worlds = sample_layouts(hand, leader, &question.inferences, rng, n);
        if worlds.len() < n {
            // Tight or jointly-infeasible reading: top up unconstrained so
            // the lead is still chosen over `n` worlds.
            let mut builder = Builder::new();
            builder[leader] = hand;
            let partial = builder
                .build_partial()
                .expect("one thirteen-card hand is a valid partial deal");
            worlds.extend(fill_deals(rng, partial).take(n - worlds.len()));
        }
        worlds.push(question.deal);
        objectives.extend(worlds.into_iter().map(|world| {
            Objective {
                board: Board::try_new(world.into(), CurrentTrick::new(question.strain, leader))
                    .expect("a full deal at trick one is a valid board"),
                target: Target::Legal,
            }
        }));
    }
    let found = Solver::lock().solve_boards(&objectives);

    let score_of = |plays: &[Play], card: Card| {
        plays
            .iter()
            .find(|play| {
                card.suit == play.card.suit
                    && (card.rank == play.card.rank || play.equals.contains(card.rank))
            })
            .map_or(0, |play| u64::from(u8::from(play.score)))
    };
    questions
        .iter()
        .zip(found.chunks_exact(n + 1))
        .map(|(question, chunk)| {
            let (actual, worlds) = chunk.split_last().expect("chunks are n + 1 long");
            // Total defensive tricks per candidate lead across the sampled
            // worlds.  Every card of the leader's hand is legal at trick one,
            // so each solve scores all of them; `max_by_key` keeps a
            // deterministic tie-break (the last maximum in the hand's fixed
            // iteration order).
            let hand = question.deal[question.declarer.lho()];
            let mut totals: Vec<(Card, u64)> = hand.into_iter().map(|card| (card, 0)).collect();
            for world in worlds {
                for (card, total) in &mut totals {
                    *total += score_of(&world.plays, *card);
                }
            }
            let (lead, _) = totals
                .into_iter()
                .max_by_key(|&(_, total)| total)
                .expect("the leader holds thirteen cards");

            // SAFETY: a defensive trick count is at most 13, so the
            // subtraction and the conversion back cannot fail.
            #[allow(clippy::cast_possible_truncation)]
            let tricks = TrickCount::try_new(13 - score_of(&actual.plays, lead) as u8)
                .expect("13 minus a trick count is a trick count");
            (lead, tricks)
        })
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

    /// The unbeatable-fit deal with the remaining cards split between the
    /// defenders — the actual layout the lead scorer plays out.
    fn unbeatable_deal() -> FullDeal {
        let (north, south) = unbeatable_spade_fit();
        let east: Hand = ".987654.T9876.T9".parse().expect("valid test hand");
        let west: Hand = ".32.5432.8765432".parse().expect("valid test hand");
        let mut builder = Builder::new();
        builder[Seat::North] = north;
        builder[Seat::South] = south;
        builder[Seat::East] = east;
        builder[Seat::West] = west;
        builder.build_full().expect("52 disjoint cards")
    }

    /// A silent reading — nothing shown by anyone.
    fn no_inferences() -> Inferences {
        use crate::bidding::Context;
        use contract_bridge::auction::RelativeVulnerability;
        Inferences::read(&Context::new(RelativeVulnerability::NONE, &[]))
    }

    /// Against the lock no lead matters: declarer takes all thirteen whatever
    /// East chooses, and the chosen card really is East's.
    #[test]
    fn lead_cannot_beat_the_lock() {
        let deal = unbeatable_deal();
        let mut rng = StdRng::seed_from_u64(3);
        let (lead, tricks) = single_dummy_lead_tricks(
            &deal,
            Strain::Spades,
            Seat::North,
            &no_inferences(),
            &mut rng,
            8,
        );
        assert!(deal[Seat::East][lead.suit].contains(lead.rank));
        assert_eq!(u8::from(tricks), 13);
    }

    /// Same seed and inputs reproduce the same lead and trick count.
    #[test]
    fn lead_choice_is_deterministic() {
        let deal = unbeatable_deal();
        let inferences = no_inferences();
        let mut rng_a = StdRng::seed_from_u64(11);
        let a = single_dummy_lead_tricks(
            &deal,
            Strain::Spades,
            Seat::North,
            &inferences,
            &mut rng_a,
            6,
        );
        let mut rng_b = StdRng::seed_from_u64(11);
        let b = single_dummy_lead_tricks(
            &deal,
            Strain::Spades,
            Seat::North,
            &inferences,
            &mut rng_b,
            6,
        );
        assert_eq!(a, b);
    }
}
