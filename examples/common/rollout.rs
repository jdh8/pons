//! The rollout pricer shared by `probe-rollout-label` and `dump-teacher
//! --relabel`: the restricted proposal, the replay draw, and the per-layout
//! swing of every candidate against the own call.
//!
//! `docs/ai-bidder/logit-calibration.md` §4 designed the measurement and §4d
//! priced it; the relabel pass inside `dump-teacher` is that same pricing run
//! over the corpus, so the two binaries must agree byte for byte on what a
//! swing is.  Both call these.

use contract_bridge::auction::{Auction, Call};
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use ddss::TrickCountTable;
use pons::bidding::array::Logits;
use pons::bidding::context::relative;
use pons::bidding::sampler::sample_layouts_replay;
use pons::bidding::{Bidder, Partnership, Table};
use pons::scoring::{final_contract, imps, ns_score_contract, ns_score_pd};
use rand::SeedableRng;
use rand::rngs::StdRng;

/// The proposal hook: `softmax(z / t)` restricted to `admissible` and
/// renormalised over it, paired with the share of the *unrestricted* mass that
/// set holds (what the epsilon rung thresholds).  Sorted by odds, descending.
pub fn restricted(logits: &Logits, admissible: &[Call], t: f32) -> (Vec<(Call, f32)>, f32) {
    let beta = 1.0 / t;
    let max = logits
        .iter()
        .map(|(_, &z)| z)
        .fold(f32::NEG_INFINITY, f32::max);
    let (mut inside, mut total) = (0.0, 0.0);
    let mut odds = Vec::with_capacity(admissible.len());
    for (call, &z) in logits.iter() {
        let w = (beta * (z - max)).exp();
        total += w;
        if admissible.contains(&call) {
            inside += w;
            odds.push((call, w));
        }
    }
    for (_, w) in &mut odds {
        *w /= inside;
    }
    odds.sort_by(|a, b| b.1.total_cmp(&a.1));
    (odds, inside / total)
}

/// The candidate set of one decision: the own call first, then the
/// proposal's top-`k` minus it.  Empty when the proposal put under `epsilon`
/// of its mass on `admissible` (the hook declined) or offered no alternative.
pub fn candidates(
    proposal: &Logits,
    admissible: &[Call],
    own: Call,
    top_k: usize,
    epsilon: f32,
    temperature: f32,
) -> Vec<Call> {
    let (odds, mass) = restricted(proposal, admissible, temperature);
    if mass < epsilon {
        return Vec::new();
    }
    let mut out = vec![own];
    out.extend(
        odds.iter()
            .take(top_k)
            .map(|&(call, _)| call)
            .filter(|&call| call != own),
    );
    if out.len() < 2 {
        out.clear();
    }
    out
}

/// Draw `n` layouts for a decision from the replay sampler, seeded by `seed`.
///
/// The accepted sequence is a deterministic function of the stream, and the
/// sampler's two budgets (a total draw cap and a dry-run limit) do not scale
/// with `n`, so the first `n` layouts of a longer draw under the same seed are
/// this draw — which is what lets a stored draw be **extended** without
/// re-solving its prefix.
#[allow(clippy::too_many_arguments)]
pub fn sample_for(
    hand: Hand,
    seat: Seat,
    policy: &Partnership,
    vul: AbsoluteVulnerability,
    prefix: &[Call],
    n: usize,
    seed: u64,
) -> Vec<FullDeal> {
    let rel = relative(vul, seat);
    let inferences = policy.infer(rel, prefix);
    let mut rng = StdRng::seed_from_u64(seed);
    sample_layouts_replay(hand, seat, policy, rel, prefix, &inferences, &mut rng, n)
}

/// Every candidate's swing over the own call, in IMPs, per layout and per
/// scorer — `[candidate][layout] = [plain DD, perfect defense]`.
///
/// `candidates[0]` is the own call, so its row is all zeros by construction;
/// one double-dummy solve per layout is shared across every candidate, each
/// seeded onto the real `prefix` and bid out at a table of `ours` (the actor's
/// side) against `theirs`.
#[allow(clippy::too_many_arguments)]
pub fn swings(
    candidates: &[Call],
    prefix: &[Call],
    dealer: Seat,
    seat: Seat,
    layouts: &[FullDeal],
    tables: &[TrickCountTable],
    ours: &dyn Bidder,
    theirs: &dyn Bidder,
    vul: AbsoluteVulnerability,
) -> Vec<Vec<[i64; 2]>> {
    let actor_is_ns = matches!(seat, Seat::North | Seat::South);
    let (ns, ew): (&dyn Bidder, &dyn Bidder) = if actor_is_ns {
        (ours, theirs)
    } else {
        (theirs, ours)
    };
    let table = Table::new(ns, ew, dealer, vul);
    let sign = if actor_is_ns { 1 } else { -1 };

    let priced: Vec<Vec<[i64; 2]>> = candidates
        .iter()
        .map(|&call| {
            layouts
                .iter()
                .zip(tables)
                .map(|(layout, tricks)| {
                    let mut seed = Auction::new();
                    seed.try_extend(prefix.iter().copied())
                        .expect("a prior table auction is legal");
                    // Every candidate came from `admissible`, which already
                    // tested `can_push` against this same prefix.
                    seed.try_push(call).expect("a candidate is legal here");
                    let reached = final_contract(&table.bid_out_from(layout, seed), dealer);
                    [
                        sign * ns_score_contract(reached, tricks, vul),
                        sign * ns_score_pd(reached, tricks, vul),
                    ]
                })
                .collect()
        })
        .collect();
    let base = priced[0].clone();
    priced
        .into_iter()
        .map(|candidate| {
            candidate
                .into_iter()
                .zip(&base)
                .map(|(got, want)| [imps(got[0] - want[0]), imps(got[1] - want[1])])
                .collect()
        })
        .collect()
}
