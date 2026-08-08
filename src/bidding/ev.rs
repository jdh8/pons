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
use super::sampler::{sample_layouts, sample_layouts_replay};
use super::table::Table;
use crate::scoring::{final_contract, ns_score_bid};
use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, Hand, Seat};
use ddss::{NonEmptyStrainFlags, Solver};
use rand::Rng;

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
/// - `context` carries the prior auction; its
///   [`Inferences`][super::inference::Inferences] are read here to
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

    let inferences = context.inferences();
    let deals = if context.reading_profile().rule_accept() {
        // Read each authored prior bid by replaying the rule that authored it
        // (frozen at its node); unauthored nodes fall back to the range reading.
        let mut deals = sample_layouts_replay(
            hand,
            seat,
            policy,
            context.vul(),
            context.auction(),
            &inferences,
            rng,
            n,
        );
        if deals.len() < n {
            // Replay can still starve on a tight authored auction.  Top up with
            // the range reader alone so the rollout keeps a usable layout count.
            // ponytail: pays the full replay budget first; add a probe-budget
            // early-abort if the wasted draws on starved auctions bite.
            let more = sample_layouts(hand, seat, &inferences, rng, n - deals.len());
            deals.extend(more);
        }
        deals
    } else {
        sample_layouts(hand, seat, &inferences, rng, n)
    };
    if deals.is_empty() {
        return vec![f32::NAN; calls.len()];
    }

    // One solve per layout, shared across every candidate call (the cost note).
    let tables = Solver::lock(None).solve_deals(&deals, NonEmptyStrainFlags::ALL);
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
mod tests;
