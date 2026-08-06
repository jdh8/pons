//! Constrained layout sampling — the inverse of [`Inferences`]
//!
//! [`Inferences`] reads an auction *forward* into per-player shown ranges (suit
//! lengths and points).  This module runs that backward: given the player to
//! act, their actual hand, and those ranges, it deals the *other three* hands at
//! random so each falls within everything the calls have shown.
//!
//! Because every range starts at [`Envelope::unknown`] and only ever narrows
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

use super::System;
use super::inference::{Inferences, Relative, relative_of};
use contract_bridge::auction::{Auction, Call, RelativeVulnerability};
use contract_bridge::deck::fill_deals;
use contract_bridge::{Builder, Card, FullDeal, Hand, Seat};
use rand::Rng;
use rand::seq::SliceRandom;

/// Random deals tried per requested layout before giving up
///
/// Rejection sampling needs roughly `1 / acceptance` draws per kept layout, so
/// the total budget is `n * MAX_ATTEMPTS_PER_LAYOUT`.  The cap exists only to
/// terminate auctions whose ranges no hand can satisfy.
///
/// It was 256, which the ranges *do* approach: `probe-replay-yield` measured
/// 59–93 % fills on ordinary auctions (`(1NT) X`, `1H (2C)`, a 2/1 sequence),
/// each exhausting the whole budget — the shortfall was the cap, not
/// infeasibility.  A short fill is the expensive failure: `ev_all` then averages
/// over fewer, edge-biased worlds.  A draw is ~0.24 µs (same probe), so even a
/// fully-spent 128-layout budget costs ~125 ms against the double-dummy solve
/// each kept layout pays — look harder rather than loosen the envelope
/// ([`Inferences`] soundness).
const MAX_ATTEMPTS_PER_LAYOUT: usize = 4096;

/// Random splits tried per requested defender world, before topping up hard-only
///
/// The mid-play sampler ([`sample_defender_remnants`]) keeps the old, tighter
/// budget: it runs at every declarer turn of every single-dummy playout, and a
/// starved draw there degrades gracefully (the hard masks still hold) rather
/// than shortening the population.
const DEFENDER_ATTEMPTS_PER_WORLD: usize = 256;

/// Total random deals the *replay* sampler will draw for one request — a generous
/// ceiling (~10-20 s, in tempo for a human bid), since a deal is a ~0.3 µs shuffle
/// and the accept test a few classifies, both far below the double-dummy solve
/// each *accepted* layout then pays.  Look as hard as it takes rather than fall
/// back to the unfaithful ranges.
///
/// This is only a backstop: [`REPLAY_DRY_LIMIT`] governs termination in practice,
/// so a feasible auction stops when it fills and an infeasible one bails early.
const REPLAY_DRAW_CAP: usize = 50_000_000;

/// Consecutive rejected draws after which the replay sampler gives up on the
/// current request — the auction is *feasibility*-limited, not budget-limited
/// (e.g. a penalty double needs the doubler to hold 15+, impossible when the
/// actor is strong), so more draws will not help and the caller tops up with the
/// ranges.  Resets on every accept, so it never cuts short an auction yielding
/// more than roughly `1 / REPLAY_DRY_LIMIT`.
const REPLAY_DRY_LIMIT: usize = 1 << 20;

/// How far below its best legal call the policy may rank a player's actual call
/// and still accept the hand, in nats (the replay sampler's relaxation knob).
///
/// Strict argmax (`0.0`) over-tightens — every committal call becomes an
/// independent hurdle and the rejection loop starves.  This margin accepts
/// near-ties, the population the loose range readers approximated.  Tuned for
/// sampler yield; see the plan.
const MARGIN: f32 = 3.0;

/// Deal up to `n` full layouts consistent with what an auction has shown
///
/// `hand` is the actor's own thirteen cards and `seat` their absolute seat;
/// both are held fixed while the other three hands are dealt at random so that
/// LHO, partner, and RHO each fall within their [`Inferences`] ranges (which are
/// relative to `seat`, the side to act).  `rng` is the caller's — the model
/// never samples, so the learned floor stays deterministic (invariant §0.5).
///
/// Returns at most `n` layouts.  Fewer (possibly none) means the attempt budget
/// of `n * 4096` draws ran out first, which happens only when the shown ranges
/// are tight or jointly infeasible given `hand`; a caller should read a short
/// result as a weak or absent signal, not an error.
#[must_use]
pub fn sample_layouts(
    hand: Hand,
    seat: Seat,
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<FullDeal> {
    let budget = n.saturating_mul(MAX_ATTEMPTS_PER_LAYOUT);
    sample_with(hand, seat, rng, n, budget, usize::MAX, |deal| {
        within_ranges(deal, seat, inferences)
    })
}

/// Deal up to `n` layouts, accepting each by *replaying the rule* on top of the
/// [`Inferences`] ranges (gated by
/// [`set_rule_accept`][super::inference::set_rule_accept]).
///
/// A hand is kept iff it (a) falls within `inferences` — the old range reading,
/// which covers every call — *and* (b) at every **authored** node a non-actor
/// player bid ([`System::authored_at`]), `policy` re-run on the candidate ranks
/// the made call within a margin of its best legal call.  Replay only tightens
/// where a rule answers; a bid the keyless floor handled (a competitive
/// raise/rebid with no authored node) is left to the range reading alone.  `vul`
/// is relative to `seat` (the actor): partner shares it, the opponents see it
/// side-swapped.
///
/// Short-result semantics match [`sample_layouts`], but with a far larger draw
/// budget: replay is tight, and looking harder is cheap next to the double-dummy
/// solve each accepted layout pays.
#[must_use]
// Each argument is a distinct fact of the decision, as in [`ev_all`].
#[allow(clippy::too_many_arguments)]
pub fn sample_layouts_replay(
    hand: Hand,
    seat: Seat,
    policy: &dyn System,
    vul: RelativeVulnerability,
    auction: &[Call],
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<FullDeal> {
    sample_with(
        hand,
        seat,
        rng,
        n,
        REPLAY_DRAW_CAP,
        REPLAY_DRY_LIMIT,
        |deal| {
            within_ranges(deal, seat, inferences) && rules_accept(deal, seat, policy, vul, auction)
        },
    )
}

/// Rejection-sample up to `n` layouts whose other three hands pass `accept`,
/// drawing at most `budget` random deals and giving up early after `dry_limit`
/// consecutive rejects (pass `usize::MAX` to disable the early-out).
///
/// The actor's thirteen cards are pinned, so each draw deals only the other
/// thirty-nine.
// ponytail: the `build_partial` expect cannot fire — one hand placed in an
// otherwise empty builder is always a valid partial deal.
fn sample_with(
    hand: Hand,
    seat: Seat,
    rng: &mut impl Rng,
    n: usize,
    budget: usize,
    dry_limit: usize,
    accept: impl Fn(&FullDeal) -> bool,
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

    let mut dry = 0usize;
    for deal in fill_deals(rng, partial).take(budget) {
        if accept(&deal) {
            out.push(deal);
            if out.len() == n {
                break;
            }
            dry = 0;
        } else {
            dry += 1;
            if dry >= dry_limit {
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
        (seat.lho(), Relative::Lho),
        (seat.partner(), Relative::Partner),
        (seat.rho(), Relative::Rho),
    ]
    .into_iter()
    .all(|(other, who)| inferences.admits(who, deal[other]))
}

/// Whether LHO, partner, and RHO in `deal` could each have made their actual
/// calls under `policy` (the rule-replay accept test; see
/// [`sample_layouts_replay`]).
fn rules_accept(
    deal: &FullDeal,
    seat: Seat,
    policy: &dyn System,
    vul: RelativeVulnerability,
    auction: &[Call],
) -> bool {
    let len = auction.len();
    let theirs = swap_sides(vul);
    [
        (seat.lho(), Relative::Lho, theirs),
        (seat.partner(), Relative::Partner, vul),
        (seat.rho(), Relative::Rho, theirs),
    ]
    .into_iter()
    .all(|(other, who, pvul)| {
        let hand = deal[other];
        // This player's own call indices, deepest first — the tightest node
        // rejects fastest, short-circuiting the rest.
        (0..len)
            .rev()
            .filter(|&i| relative_of(len, i) == who)
            .all(|i| made_plausibly(hand, policy, pvul, &auction[..i], auction[i]))
    })
}

/// Whether `policy`, classifying `hand` at `prefix`, ranks the `made` call
/// within [`MARGIN`] of its best legal call.  A call no rule authors (nothing
/// to replay) abstains so the range reader handles it, and an off-book node
/// has no opinion; both accept.  A **pass** replays like any call — the
/// negative inference the interval ranges cannot express: a candidate whose
/// best alternative beats the pass is rejected (hard where the pass gate is
/// `-∞`, e.g. a 12-count at the opening node; soft within [`MARGIN`] of
/// weight-close alternatives such as a preempt).  A candidate the node
/// rejects wholesale accepts (`-∞ ≥ -∞ − MARGIN`) — the floor-pass worlds
/// stay in.
fn made_plausibly(
    hand: Hand,
    policy: &dyn System,
    vul: RelativeVulnerability,
    prefix: &[Call],
    made: Call,
) -> bool {
    if !policy.authored_at(vul, prefix) {
        return true;
    }
    let Some(logits) = policy.classify(hand, vul, prefix) else {
        return true;
    };
    // Best over *legal* calls only — a fallback book may offer a call now illegal
    // at this node, which must not inflate the argmax the made call is judged
    // against (the made call is legal by construction).
    let mut played = Auction::new();
    played
        .try_extend(prefix.iter().copied())
        .expect("a prior table auction is legal");
    let best = logits
        .0
        .iter()
        .filter(|(call, _)| played.can_push(*call).is_ok())
        .fold(f32::NEG_INFINITY, |best, (_, &logit)| best.max(logit));
    *logits.0.get(made) >= best - MARGIN
}

/// Deal exactly `n` mid-play defender worlds consistent with declarer's view
///
/// The single-dummy playout ([`single_dummy_playout`][crate::single_dummy_playout])
/// asks, at each of declarer's turns: how might the cards declarer *cannot*
/// see — `pool`, both defenders' unplayed cards — lie?  Each world splits
/// `pool` between the two defenders at their remaining hand sizes (derived
/// from `lho_played`/`rho_played`: a defender's remnant is thirteen minus
/// what they have played), subject to two layers of constraint:
///
/// - **Hard** — `lho_may`/`rho_may`, the cards each defender can still hold
///   (a defender who showed out of a suit holds none of it; declarer saw
///   that).  Satisfied *constructively*: cards only one defender may hold are
///   forced there and the rest split at random, so every world respects the
///   masks — a violating world would be an impossible layout (and a revoke
///   waiting to happen at the solver's door).
/// - **Soft** — the shown ranges of `inferences`, read from **declarer's**
///   perspective and applied to each defender's reconstructed *original*
///   hand (remnant ∪ played).  Rejection-sampled; when the reading is too
///   tight to fill within the attempt budget, the remainder is topped up
///   hard-only (a weak signal, not an error — the playout must price its
///   candidates over *some* population, and the lead scorer tops up the same
///   way), so this function always returns exactly `n` worlds.
///
/// # Panics
///
/// Panics if the inputs do not reconstruct two thirteen-card defenders
/// (`pool.len() + lho_played.len() + rho_played.len() == 26` with `pool`
/// disjoint from both), or if the masks make the true layout impossible
/// (a pool card neither defender may hold, or more forced cards than a
/// remnant has room for) — the caller tracks the play, so either is a
/// bookkeeping bug.
#[must_use]
// Each argument is a distinct fact of the position, as in `sample_layouts_replay`.
#[allow(clippy::too_many_arguments)]
pub fn sample_defender_remnants(
    pool: Hand,
    lho_played: Hand,
    rho_played: Hand,
    lho_may: Hand,
    rho_may: Hand,
    inferences: &Inferences,
    rng: &mut impl Rng,
    n: usize,
) -> Vec<(Hand, Hand)> {
    assert!(
        pool & (lho_played | rho_played) == Hand::EMPTY
            && pool.len() + lho_played.len() + rho_played.len() == 26,
        "pool and played cards must reconstruct two thirteen-card defenders"
    );
    // Cards only one defender may hold are forced; the rest split at random.
    let lho_forced = pool - rho_may;
    let rho_forced = pool - lho_may;
    let lho_len = 13 - lho_played.len();
    assert!(
        lho_forced & rho_forced == Hand::EMPTY
            && lho_forced.len() <= lho_len
            && rho_forced.len() <= pool.len() - lho_len,
        "hard masks must admit the true layout"
    );
    let mut free: Vec<Card> = (pool - lho_forced - rho_forced).into_iter().collect();
    let lho_free = lho_len - lho_forced.len();
    let mut split = move |rng: &mut _| {
        let (drawn, _) = free.partial_shuffle(rng, lho_free);
        let lho = lho_forced | drawn.iter().copied().collect();
        (lho, pool - lho)
    };

    let mut out = Vec::with_capacity(n);
    for _ in 0..n.saturating_mul(DEFENDER_ATTEMPTS_PER_WORLD) {
        if out.len() == n {
            break;
        }
        let (lho, rho) = split(rng);
        if inferences.admits(Relative::Lho, lho | lho_played)
            && inferences.admits(Relative::Rho, rho | rho_played)
        {
            out.push((lho, rho));
        }
    }
    // Tight or jointly-infeasible reading: top up hard-only.
    while out.len() < n {
        out.push(split(rng));
    }
    out
}

/// Vulnerability seen from the opposing side: swap the WE and THEY bits.
fn swap_sides(vul: RelativeVulnerability) -> RelativeVulnerability {
    let mut out = RelativeVulnerability::NONE;
    out.set(
        RelativeVulnerability::WE,
        vul.contains(RelativeVulnerability::THEY),
    );
    out.set(
        RelativeVulnerability::THEY,
        vul.contains(RelativeVulnerability::WE),
    );
    out
}

#[cfg(test)]
mod tests;
