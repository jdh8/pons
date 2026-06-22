//! Call-EV evaluator — AI-bidder M2.2.
//!
//! The books say *which* call a system makes; they never said what a call is
//! *worth*.  This module answers that with a Monte-Carlo rollout grounded in
//! cardplay:
//!
//! 1. **Deal the unknowns.**  [`sample_layouts`] deals the other three hands
//!    consistent with everything the auction has shown (the actor's own hand is
//!    pinned, so every layout is a full deal this auction could have come from).
//! 2. **Finish the auction.**  Seed the candidate call onto the prior auction
//!    and let a *continuation policy* bid it out — all four seats bid the same
//!    policy (a self-play assumption: "what happens if everyone plays like us").
//! 3. **Score double-dummy.**  Solve each sampled layout once and price the
//!    contract each candidate reached, signed to the **actor's** favour, under
//!    **perfect-defense doubling** ([`ns_score_bid`][crate::scoring::ns_score_bid]): a contract
//!    that fails double-dummy is scored *doubled*.  The cardplay already assumes
//!    optimal defense, so the penalty must too — otherwise the rollout's weak
//!    doubling lets failing sacrifices price far too cheaply and the search
//!    chases phantom saves into runaway competitive auctions.
//! 4. **Average** over layouts.  That average is the call's EV.
//!
//! The continuation policy is a [`System`] *parameter*, not hardwired.  M2.2
//! defaults callers to the deterministic [`american`][crate::american()]
//! (debuggable, and ≈ the distilled net at bootstrap); the M3 search-improvement
//! loop swaps in successive nets without touching this code.
//!
//! The double-dummy solve is the cost, so it is **shared across candidates**:
//! [`ev_all`] solves each layout once with [`NonEmptyStrainFlags::ALL`][ddss::NonEmptyStrainFlags::ALL] and
//! prices every candidate contract from that one [`TrickCountTable`][ddss::TrickCountTable].  Cost is
//! `n` solves, not `k · n`.  This batch form is also what the M2.3 live search
//! bidder wants — score the net-shortlisted top-`k` at once.

use super::System;
use super::context::Context;
use super::inference::Inferences;
use super::sampler::sample_layouts;
use super::table::Table;
use crate::scoring::{final_contract, ns_score_bid};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use rand::Rng;

/// Cardplay-grounded value of one candidate `call`, in the actor's favour
///
/// A thin wrapper over [`ev_all`]; prefer that when scoring several candidates
/// on the same decision, so the double-dummy solves are shared.  See the
/// [module docs][self] for the rollout and the meaning of the returned number
/// (average score in points, positive good for the actor; [`f32::NAN`] when no
/// layout could be sampled — read it as *no signal*, per [`sample_layouts`]).
#[must_use]
// Each argument is a distinct fact of the decision; a struct would be ceremony.
#[allow(clippy::too_many_arguments)]
pub fn ev(
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    context: &Context<'_>,
    call: Call,
    policy: &impl System,
    rng: &mut impl Rng,
    n: usize,
) -> f32 {
    ev_all(hand, seat, vul, context, &[call], policy, rng, n)[0]
}

/// Cardplay-grounded value of each candidate `call`, in the actor's favour
///
/// Returns one EV per entry of `calls`, aligned by index.  All candidates are
/// scored over the **same** `n` sampled layouts and the **same** double-dummy
/// solves, so their EVs are directly comparable and the solve cost is paid once.
///
/// - `hand`/`seat` are the actor's own thirteen cards and absolute seat (as in
///   [`sample_layouts`] — [`Context`] carries neither).
/// - `vul` is the absolute table vulnerability, used to score and to drive the
///   continuation policy (which converts it per seat itself).
/// - `context` carries the prior auction; its [`Inferences`] are read here to
///   sample the layouts the rollout continues.
/// - `policy` bids every seat during the rollout (the self-play assumption).
///
/// An entry is [`f32::NAN`] when its call is illegal in the prior auction, and
/// every entry is `NAN` when no layout could be sampled (a tight or infeasible
/// auction); callers should treat `NAN` as no signal, not an error.
///
/// # Panics
///
/// Panics if `context`'s prior auction is not a legal sequence of calls (it
/// always is when the context comes from a real table).
#[must_use]
#[allow(clippy::cast_precision_loss)] // averaging i64 points into an f32 EV
#[allow(clippy::too_many_arguments)] // each argument is a distinct decision fact
pub fn ev_all(
    hand: Hand,
    seat: Seat,
    vul: AbsoluteVulnerability,
    context: &Context<'_>,
    calls: &[Call],
    policy: &impl System,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<f32> {
    if calls.is_empty() {
        return Vec::new();
    }

    let inferences = Inferences::read(context);
    let deals = sample_layouts(hand, seat, &inferences, rng, n);
    if deals.is_empty() {
        return vec![f32::NAN; calls.len()];
    }

    // One solve per layout, shared across every candidate call (the cost note).
    let tables = Solver::lock().solve_deals(&deals, NonEmptyStrainFlags::ALL);
    let dealer = dealer_of(seat, context.auction().len());
    let table = Table::new(policy, policy, dealer, vul);
    let actor_is_ns = matches!(seat, Seat::North | Seat::South);

    calls
        .iter()
        .map(|&call| {
            // Seed the prior auction, then the candidate; an illegal candidate
            // has no rollout, so it carries no signal.
            let mut seed = Auction::new();
            seed.try_extend(context.auction().iter().copied())
                .expect("a prior table auction is legal");
            if seed.can_push(call).is_err() {
                return f32::NAN;
            }
            seed.push(call);

            let total: i64 = deals
                .iter()
                .zip(tables.iter())
                .map(|(deal, tricks)| {
                    let auction = table.bid_out_from(deal, seed.clone());
                    let result = final_contract(&auction, dealer).map(|(c, s)| (c.bid, s));
                    let score = ns_score_bid(result, tricks, vul);
                    if actor_is_ns { score } else { -score }
                })
                .sum();
            total as f32 / deals.len() as f32
        })
        .collect()
}

/// The dealer such that the seat acting after the prior auction is `seat`
///
/// [`Table`] positions a seeded auction from the dealer, so for the rollout's
/// continuation to attribute calls to the right players, the dealer must place
/// the actor on move after `prior_len` calls:
/// `seat_to_act(dealer, prior_len) == seat`.
fn dealer_of(seat: Seat, prior_len: usize) -> Seat {
    let actor = Seat::ALL
        .iter()
        .position(|&s| s == seat)
        .expect("every seat is in Seat::ALL");
    Seat::ALL[(actor + 4 - prior_len % 4) % 4]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::american;
    use crate::bidding::Family;
    use contract_bridge::auction::RelativeVulnerability;
    use contract_bridge::{Bid, Level, Strain};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    const fn bid(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid {
            level: Level::new(level),
            strain,
        })
    }

    /// A balanced 20-count (4-3-3-3, AKQ2/KQ2/KJ2/Q32): strong enough that 3NT
    /// is a sound game and 7NT is a hopeless grand, so the EV ranking between
    /// them is unambiguous.
    fn balanced_twenty() -> Hand {
        "AKQ2.KQ2.KJ2.Q32".parse().expect("valid test hand")
    }

    /// The deterministic continuation policy used throughout these tests.
    fn deterministic() -> impl System {
        american().against(Family::NATURAL)
    }

    /// Sanity: the evaluator prefers the obviously-right call.  As dealer with a
    /// flat 20-count, a sound game (3NT) must out-value a hopeless grand (7NT),
    /// and the grand must price out clearly negative (it goes down off the top).
    #[test]
    fn prefers_game_over_hopeless_grand() {
        let policy = deterministic();
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let mut rng = StdRng::seed_from_u64(20);
        let evs = ev_all(
            balanced_twenty(),
            Seat::North,
            AbsoluteVulnerability::NONE,
            &context,
            &[bid(3, Strain::Notrump), bid(7, Strain::Notrump)],
            &policy,
            &mut rng,
            48,
        );

        assert!(
            evs[0] > evs[1],
            "3NT ({}) should beat 7NT ({})",
            evs[0],
            evs[1]
        );
        assert!(
            evs[1] < 0.0,
            "7NT off the top should be negative, got {}",
            evs[1]
        );
    }

    /// Determinism: the model never samples its own RNG, so the same seed and
    /// inputs reproduce the same EVs exactly (invariant §0.5).
    #[test]
    fn deterministic_given_a_seed() {
        let policy = deterministic();
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let calls = [bid(3, Strain::Notrump), Call::Pass];

        let mut rng_a = StdRng::seed_from_u64(7);
        let a = ev_all(
            balanced_twenty(),
            Seat::North,
            AbsoluteVulnerability::NONE,
            &context,
            &calls,
            &policy,
            &mut rng_a,
            24,
        );
        let mut rng_b = StdRng::seed_from_u64(7);
        let b = ev_all(
            balanced_twenty(),
            Seat::North,
            AbsoluteVulnerability::NONE,
            &context,
            &calls,
            &policy,
            &mut rng_b,
            24,
        );
        assert_eq!(a, b);
    }

    /// An infeasible auction samples no layout, so every EV is `NaN` — the
    /// "no signal" contract, not a panic.  North hoards nine hearts while RHO's
    /// 1H opening demands five, leaving only four in the deck.
    #[test]
    fn infeasible_auction_is_no_signal() {
        let policy = deterministic();
        // dealer_of(North, 1) == West, so the lone prior call (1H) is West's,
        // and West is North's RHO — exactly the seat the opening constrains.
        let auction = [bid(1, Strain::Hearts)];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let hoard: Hand = "32.AKQJT9876.2.2".parse().expect("valid test hand");
        let mut rng = StdRng::seed_from_u64(5);

        let evs = ev_all(
            hoard,
            Seat::North,
            AbsoluteVulnerability::NONE,
            &context,
            &[Call::Pass, bid(2, Strain::Hearts)],
            &policy,
            &mut rng,
            8,
        );
        assert!(
            evs.iter().all(|ev| ev.is_nan()),
            "no layout means no signal"
        );
    }

    /// An illegal candidate carries no signal even when other candidates do.
    #[test]
    fn illegal_candidate_is_nan() {
        let policy = deterministic();
        // RHO (West) opened 1H; North is to act.  1C is below 1H — illegal.
        let auction = [bid(1, Strain::Hearts)];
        let context = Context::new(RelativeVulnerability::NONE, &auction);
        let mut rng = StdRng::seed_from_u64(3);

        let evs = ev_all(
            balanced_twenty(),
            Seat::North,
            AbsoluteVulnerability::NONE,
            &context,
            &[bid(1, Strain::Clubs), Call::Pass],
            &policy,
            &mut rng,
            8,
        );
        assert!(evs[0].is_nan(), "1C over 1H is illegal");
        assert!(evs[1].is_finite(), "Pass is legal and should score");
    }

    /// Requesting no candidates returns nothing.
    #[test]
    fn empty_candidates_is_empty() {
        let policy = deterministic();
        let context = Context::new(RelativeVulnerability::NONE, &[]);
        let mut rng = StdRng::seed_from_u64(0);
        assert!(
            ev_all(
                balanced_twenty(),
                Seat::North,
                AbsoluteVulnerability::NONE,
                &context,
                &[],
                &policy,
                &mut rng,
                8,
            )
            .is_empty()
        );
    }
}
