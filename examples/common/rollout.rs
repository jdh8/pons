//! The rollout pricer shared by `probe-rollout-label` and `dump-teacher
//! --relabel`: the restricted proposal and the replay draw.  The per-layout
//! swing of every candidate against the own call is `pons::bidding::ev::swings`,
//! shared with the live `Lookahead`.
//!
//! `docs/ai-bidder/logit-calibration.md` §4 designed the measurement and §4d
//! priced it; the relabel pass inside `dump-teacher` is that same pricing run
//! over the corpus, so the two binaries must agree byte for byte on what a
//! swing is.  Both call these.

use contract_bridge::auction::Call;
use contract_bridge::{AbsoluteVulnerability, FullDeal, Hand, Seat};
use pons::bidding::Partnership;
use pons::bidding::array::Logits;
use pons::bidding::context::relative;
use pons::bidding::sampler::sample_layouts_replay;
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
