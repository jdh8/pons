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
//! - [`single_dummy_playout`] / [`single_dummy_declarer_tricks`] —
//!   **declarer's view during play**: after the lead, declarer chooses every
//!   card by the same Monte-Carlo double-dummy averaging over worlds
//!   consistent with the auction and the cards seen, while the defenders
//!   play on double-dummy.  This prices the *other* seam — declarer's
//!   misguesses, which dominate at the slam level where the lead gap tapers
//!   to zero and plain DD is systematically optimistic for the side bidding
//!   more slams.
//!
//! A *true* single-dummy solver (each player acting on only what it sees at
//! every trick, with defenders also fallible and signalling) is an
//! imperfect-information search orders of magnitude more expensive, and is
//! intentionally out of scope: the DD-averaging proxy is what the field uses
//! (GIB, Q-plus, …).
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

use crate::bidding::{Inferences, sample_defender_remnants, sample_layouts};
use crate::stats::HistogramTable;
use contract_bridge::deal::PartialDeal;
use contract_bridge::deck::fill_deals;
use contract_bridge::hand::{Holding, Rank};
use contract_bridge::{Builder, Card, FullDeal, Hand, Seat, Strain, Suit};
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
/// still perfect on both sides, so declarer misguesses (the slam-level bias —
/// [`single_dummy_playout`] models exactly that seam) and later defensive
/// signalling are not modelled.
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

/// Solved score of `card` among `plays`, matching its sequence equals
///
/// A [`Target::Legal`] solve lists one [`Play`] per sequence; a candidate
/// matches the play whose `card` or `equals` holds its rank.  An absent card
/// scores zero (it was not legal in that position).
fn score_of(plays: &[Play], card: Card) -> u64 {
    plays
        .iter()
        .find(|play| {
            card.suit == play.card.suit
                && (card.rank == play.card.rank || play.equals.contains(card.rank))
        })
        .map_or(0, |play| u64::from(u8::from(play.score)))
}

/// Legal plays collapsed to one representative per sequence
///
/// Two legal cards are equivalent iff they share a suit and every rank
/// between them is in the mover's own remaining `hand` or already `seen`
/// (played to this or an earlier trick) — DDS's "equals" rule.  Each
/// sequence's highest card represents it.  `led` is the suit led to the
/// current trick, if any; a hand with that suit must follow.
fn distinct_plays(hand: Hand, led: Option<Suit>, seen: Hand) -> Vec<Card> {
    let mut out = Vec::new();
    let mut push_suit = |suit: Suit| {
        let own = hand[suit];
        let covered = own | seen[suit];
        let mut in_sequence = false;
        for rank in (2..=14).rev().map(Rank::new) {
            if own.contains(rank) {
                if !in_sequence {
                    out.push(Card { suit, rank });
                }
                in_sequence = true;
            } else if !covered.contains(rank) {
                in_sequence = false;
            }
        }
    };
    match led.filter(|&suit| !hand[suit].is_empty()) {
        Some(suit) => push_suit(suit),
        None => Suit::ASC.into_iter().for_each(push_suit),
    }
    out
}

/// Winner of a completed trick led by `leader`
fn trick_winner(leader: Seat, trick: &[Card], trump: Option<Suit>) -> Seat {
    let mut winner = leader;
    let mut best = trick[0];
    let mut seat = leader;
    for &card in &trick[1..] {
        seat = seat.lho();
        let beats = if card.suit == best.suit {
            card.rank > best.rank
        } else {
            Some(card.suit) == trump
        };
        if beats {
            winner = seat;
            best = card;
        }
    }
    winner
}

/// Card-by-card playout state for [`single_dummy_playout`]
struct Playout<'a> {
    solver: Solver,
    strain: Strain,
    trump: Option<Suit>,
    declarer: Seat,
    /// The auction read from **declarer's** perspective
    inferences: &'a Inferences,
    /// Unplayed cards per seat
    remaining: Builder,
    /// Played cards per seat (including the current trick)
    played: Builder,
    /// Cards each defender may still hold (show-outs remembered)
    may: Builder,
    /// All played cards, for sequence collapsing
    seen: Hand,
    leader: Seat,
    /// Cards played to the current trick, in playing order
    trick: Vec<Card>,
    declarer_tricks: u8,
}

impl Playout<'_> {
    /// Seat to play the next card
    fn mover(&self) -> Seat {
        self.trick.iter().fold(self.leader, |seat, _| seat.lho())
    }

    /// The current trick as the solver sees it
    fn current_trick(&self) -> CurrentTrick {
        CurrentTrick::from_slice(self.strain, self.leader, &self.trick)
            .expect("a tracked trick holds at most three distinct cards")
    }

    /// Choose the mover's card: forced plays free, declarer's side over `k`
    /// sampled worlds, defenders double-dummy on the actual position
    fn choose(&self, rng: &mut impl Rng, k: usize) -> Card {
        let mover = self.mover();
        let led = self.trick.first().map(|card| card.suit);
        let candidates = distinct_plays(self.remaining[mover], led, self.seen);
        match candidates[..] {
            // Forced (a single sequence): play it without solving.  Also
            // load-bearing: DDS mode-0 answers a single-choice position with
            // the sentinel score −2, which ddss rejects as invalid.
            [only] => only,
            _ if mover == self.declarer || mover == self.declarer.partner() => {
                self.declarer_choice(&candidates, rng, k)
            }
            _ => self.defender_choice(),
        }
    }

    /// Declarer's single-dummy pick: best mean outcome over `k` worlds
    /// consistent with declarer's view
    fn declarer_choice(&self, candidates: &[Card], rng: &mut impl Rng, k: usize) -> Card {
        let (lho, rho) = (self.declarer.lho(), self.declarer.rho());
        let pool = self.remaining[lho] | self.remaining[rho];
        let worlds = sample_defender_remnants(
            pool,
            self.played[lho],
            self.played[rho],
            self.may[lho],
            self.may[rho],
            self.inferences,
            rng,
            k,
        );
        let trick = self.current_trick();
        let objectives: Vec<Objective> = worlds
            .into_iter()
            .map(|(left, right)| {
                let mut builder = Builder::default();
                builder[self.declarer] = self.remaining[self.declarer];
                builder[self.declarer.partner()] = self.remaining[self.declarer.partner()];
                builder[lho] = left;
                builder[rho] = right;
                Objective {
                    board: Board::try_new(
                        builder
                            .build_partial()
                            .expect("a world splits the unseen pool disjointly"),
                        trick.clone(),
                    )
                    .expect("a sampled world respects hand sizes and show-outs"),
                    target: Target::Legal,
                }
            })
            .collect();

        // Total declarer-side tricks per candidate across the worlds;
        // `max_by_key` keeps a deterministic tie-break (the last maximum in
        // the candidates' fixed order), as in the lead scorer.
        let mut totals: Vec<(Card, u64)> = candidates.iter().map(|&card| (card, 0)).collect();
        for world in self.solver.solve_boards(&objectives) {
            for (card, total) in &mut totals {
                *total += score_of(&world.plays, *card);
            }
        }
        totals
            .into_iter()
            .max_by_key(|&(_, total)| total)
            .expect("a non-forced turn has candidates")
            .0
    }

    /// A defender's double-dummy best card on the actual position
    fn defender_choice(&self) -> Card {
        let board = Board::try_new(
            self.remaining
                .build_partial()
                .expect("the tracked position is a valid partial deal"),
            self.current_trick(),
        )
        .expect("the tracked position is a valid board");
        let found = self.solver.solve_board(&Objective {
            board,
            target: Target::Legal,
        });
        // Target::Legal sorts plays by score descending: the first is the
        // double-dummy best for the side to move.
        found
            .plays
            .first()
            .expect("a position with a choice has legal plays")
            .card
    }

    /// Play `card` for the mover: record show-outs, advance the trick, and
    /// score it when it completes
    fn apply(&mut self, card: Card) {
        let mover = self.mover();
        if let Some(led) = self.trick.first().map(|card| card.suit)
            && card.suit != led
        {
            // Shown out: everyone saw this seat holds no more of the suit.
            let mut gone = Hand::EMPTY;
            gone[led] = Holding::ALL;
            self.may[mover] -= gone;
        }
        assert!(
            self.remaining[mover].remove(card),
            "a chosen card must come from the mover's hand"
        );
        self.played[mover].insert(card);
        self.seen.insert(card);
        self.trick.push(card);
        if self.trick.len() == 4 {
            let winner = trick_winner(self.leader, &self.trick, self.trump);
            self.declarer_tricks +=
                u8::from(winner == self.declarer || winner == self.declarer.partner());
            self.leader = winner;
            self.trick.clear();
        }
    }
}

/// Single-dummy declarer playout: a fallible declarer against perfect defense
///
/// The dual of [`single_dummy_leads`]: where that scorer prices the
/// *defenders'* information asymmetry (the blind opening lead), this one
/// prices *declarer's* — the misguesses double-dummy scoring erases, which
/// dominate real-vs-DD results at the slam level where a DD declarer picks
/// every two-way queen, drops every offside singleton king, and finds every
/// squeeze.  Given the reached contract and the opening `lead` (choose it
/// with [`single_dummy_lead_tricks`] to compose both seams — or take the
/// actual deal's double-dummy lead to isolate this one), the deal is played
/// out card by card:
///
/// - **Declarer and dummy turns** choose single-dummy: `k` worlds consistent
///   with declarer's view — the defenders' unseen cards split at their
///   remaining sizes, hard-constrained by remembered show-outs and
///   soft-constrained by the auction reading
///   ([`sample_defender_remnants`]) — are solved double-dummy, and the
///   candidate with the best mean declarer outcome is played on the
///   **actual** deal.
/// - **Defender turns** play the double-dummy best card of the actual
///   position (perfect defense — the conservative side kept from plain DD).
/// - **Forced turns** (one legal sequence) are played without solving.
///
/// `inferences` must be read from **declarer's** perspective (e.g. via
/// [`Stance::infer`][crate::bidding::Stance::infer] on an auction prefix that
/// puts declarer to act), so the sampled worlds respect both defenders'
/// disclosures.
///
/// Returns declarer's tricks on `deal`.  Ceiling, stated: the defenders are
/// omniscient — they defend double-dummy against the actual layout and are
/// never fooled by declarer's line — so this bracket is *pessimistic* for
/// declarer everywhere (real defenders err after trick one too, which is not
/// modelled).  At the slam boundary, where the lead seam has tapered out,
/// that makes plain DD the optimist end and this the pessimist, with the
/// table result between (Pavlicek: actual − DD ≈ −1.3pp of make-rate at
/// small slams, −6.0pp at grands).
///
/// # Panics
///
/// Panics if `k == 0` (a line cannot be chosen over zero worlds), if `lead`
/// is not in the opening leader's hand, or if `deal` is not a valid full
/// deal.
#[must_use]
pub fn single_dummy_playout(
    deal: &FullDeal,
    strain: Strain,
    declarer: Seat,
    lead: Card,
    inferences: &Inferences,
    rng: &mut impl Rng,
    k: usize,
) -> TrickCount {
    assert!(k > 0, "a declarer line needs at least one sampled world");
    assert!(
        deal[declarer.lho()].contains(lead),
        "the opening lead must come from the leader's hand"
    );
    let mut may = Builder::default();
    for seat in [Seat::North, Seat::East, Seat::South, Seat::West] {
        may[seat] = Hand::ALL;
    }
    let mut playout = Playout {
        solver: Solver::lock(),
        strain,
        trump: Suit::try_from(strain).ok(),
        declarer,
        inferences,
        remaining: Builder::from(PartialDeal::from(*deal)),
        played: Builder::default(),
        may,
        seen: Hand::EMPTY,
        leader: declarer.lho(),
        trick: Vec::with_capacity(4),
        declarer_tricks: 0,
    };
    playout.apply(lead);
    for _ in 1..52 {
        let card = playout.choose(rng, k);
        playout.apply(card);
    }
    TrickCount::try_new(playout.declarer_tricks).expect("at most thirteen tricks")
}

/// Blind lead, then single-dummy declarer play: the table-proxy bracket
///
/// Composes the two modelled information seams: the opening leader chooses
/// blind over `n` auction-consistent worlds ([`single_dummy_lead_tricks`]),
/// then declarer plays the deal out single-dummy over `k` worlds per decision
/// ([`single_dummy_playout`]).  `leader_inferences` reads the auction from
/// the leader's seat and `declarer_inferences` from declarer's — the two
/// views differ (each knows their own hand and reads the other side's
/// calls).
///
/// Returns the chosen lead and declarer's tricks on `deal`.
///
/// # Panics
///
/// Panics if `n == 0` or `k == 0`, or if `deal` is not a valid full deal.
#[must_use]
// Each argument is a distinct fact of the position, as in `sample_layouts_replay`.
#[allow(clippy::too_many_arguments)]
pub fn single_dummy_declarer_tricks(
    deal: &FullDeal,
    strain: Strain,
    declarer: Seat,
    leader_inferences: &Inferences,
    declarer_inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
    k: usize,
) -> (Card, TrickCount) {
    let (lead, _) = single_dummy_lead_tricks(deal, strain, declarer, leader_inferences, rng, n);
    let tricks = single_dummy_playout(deal, strain, declarer, lead, declarer_inferences, rng, k);
    (lead, tricks)
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

    /// Against the lock even a fallible declarer takes all thirteen: every
    /// line wins in every world, so the playout cannot lose a trick.
    #[test]
    fn playout_cannot_misplay_the_lock() {
        let deal = unbeatable_deal();
        let inferences = no_inferences();
        let mut rng = StdRng::seed_from_u64(5);
        let (lead, tricks) = single_dummy_declarer_tricks(
            &deal,
            Strain::Spades,
            Seat::North,
            &inferences,
            &inferences,
            &mut rng,
            8,
            8,
        );
        assert!(deal[Seat::East][lead.suit].contains(lead.rank));
        assert_eq!(u8::from(tricks), 13);
    }

    /// A grand slam hinging on a two-way trump-queen guess: North-South hold
    /// every side winner, and the spade suit (AJT9 opposite K876, missing
    /// Q5432) picks up West's ♠Q54 double-dummy by finessing through West —
    /// but a declarer who cannot see the queen must guess.
    fn two_way_guess_deal() -> FullDeal {
        let mut builder = Builder::new();
        builder[Seat::North] = "AJT9.AKQ.AKQ2.AK".parse().expect("valid test hand");
        builder[Seat::South] = "K876.JT9.JT9.QJ4".parse().expect("valid test hand");
        builder[Seat::West] = "Q54.8765.876.T98".parse().expect("valid test hand");
        builder[Seat::East] = "32.432.543.76532".parse().expect("valid test hand");
        builder.build_full().expect("52 disjoint cards")
    }

    /// Double-dummy the guess deal is a cold grand (the finesse is always
    /// "found"), but the single-dummy playout must guess blind: over many
    /// seeds it sometimes misguesses (no peeking at the actual layout) and
    /// sometimes guesses right — and never loses more than the guess.
    #[test]
    fn playout_guesses_where_double_dummy_peeks() {
        let deal = two_way_guess_deal();
        // Fixture validity: DD says North makes 7♠ on the actual layout.
        let table = Solver::lock().solve_deal(deal);
        assert_eq!(u8::from(table[Strain::Spades].get(Seat::North)), 13);

        let inferences = no_inferences();
        let results: Vec<u8> = (0..12)
            .map(|seed| {
                let mut rng = StdRng::seed_from_u64(seed);
                let (_, tricks) = single_dummy_declarer_tricks(
                    &deal,
                    Strain::Spades,
                    Seat::North,
                    &inferences,
                    &inferences,
                    &mut rng,
                    8,
                    8,
                );
                u8::from(tricks)
            })
            .collect();
        assert!(
            results.iter().all(|&tricks| (11..=13).contains(&tricks)),
            "only the guess (and rare mean-max risk) may cost tricks: {results:?}"
        );
        assert!(
            results.iter().any(|&tricks| tricks < 13),
            "a blind declarer must sometimes misguess: {results:?}"
        );
        assert!(
            results.contains(&13),
            "a blind declarer must sometimes guess right: {results:?}"
        );
    }

    /// Same seed and inputs reproduce the same playout exactly.
    #[test]
    fn playout_is_deterministic() {
        let deal = two_way_guess_deal();
        let inferences = no_inferences();
        let mut rng_a = StdRng::seed_from_u64(17);
        let a = single_dummy_declarer_tricks(
            &deal,
            Strain::Spades,
            Seat::North,
            &inferences,
            &inferences,
            &mut rng_a,
            6,
            6,
        );
        let mut rng_b = StdRng::seed_from_u64(17);
        let b = single_dummy_declarer_tricks(
            &deal,
            Strain::Spades,
            Seat::North,
            &inferences,
            &inferences,
            &mut rng_b,
            6,
            6,
        );
        assert_eq!(a, b);
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
