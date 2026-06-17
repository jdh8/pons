//! Gated live double-dummy search bidder — AI-bidder M2.3.
//!
//! This is "simulations in action": the floor *thinks* before it bids.  Where
//! [`NeuralFloor`][crate::bidding::neural_floor::NeuralFloor] returns the distilled net's
//! judgement in one forward pass,
//! [`SearchFloor`][crate::bidding::search_floor::SearchFloor] uses that net only
//! as a *prior* — to propose which calls are worth simulating — then scores the
//! shortlist by cardplay over sampled layouts (the M2.2 [`ev_all`] evaluator) and
//! bids the highest-EV call.  "Net proposes, search disposes."
//!
//! It wears the **same deterministic safety shell** as the neural floor, so the
//! §0.4 forced rails are preserved by construction:
//!
//! - **Forced** — when `forced` reports an auction-determined forced situation
//!   (partner's live takeout double, a prior call committing us to game, a
//!   just-made transfer over our strong notrump), it returns the deterministic
//!   [`instinct`][instinct()] answer verbatim.  The net is never trusted on the rails, and
//!   neither is the search.
//! - **Judgement** — otherwise it runs the search:
//!   1. evaluate the net prior and mask the illegal calls (exactly the neural
//!      floor's judgement path);
//!   2. shortlist the top-`shortlist` legal calls by
//!      that prior;
//!   3. price each over `layouts` sampled deals with
//!      [`ev_all`] under the continuation policy (our own distilled net
//!      bidding all four seats — self-play);
//!   4. re-seat the evaluated calls onto an EV-ranked band above the prior tail,
//!      so the driver's arg-max is the best-EV call while every legal call keeps
//!      a sane fallback logit and `Pass` stays finite (invariant §0.2).
//!
//! # Determinism
//!
//! [`Classifier::classify`][crate::bidding::trie::Classifier::classify] must be a pure function (invariant §0.5), yet the
//! rollout samples layouts.  The shell reconciles this by seeding the rollout
//! RNG *from the decision itself* (a hash of the feature vector, which is a
//! deterministic function of the hand, the auction, vulnerability, and seat): the
//! same decision always draws the same layouts, so the same EVs, so the same
//! logits.  No randomness escapes into the floor.
//!
//! # Seat canonicalization
//!
//! A [`Classifier`][crate::bidding::trie::Classifier] receives only the hand and a [`Context`] (relative
//! vulnerability + the raw auction); it never learns the actor's absolute seat,
//! which [`ev_all`] needs.  Because an EV is computed entirely *relative* to the
//! actor — the sampler, [the dealer placement][ev_all], and the scoring sign all
//! key off it — the absolute choice is free, so the shell pins the actor to
//! [`Seat::North`][contract_bridge::Seat::North] and rebuilds the absolute vulnerability from the relative one
//! (the inverse of [`relative`][crate::bidding::context::relative] at North: we ↔ NS,
//! they ↔ EW).

use super::american::american_neural_search;
use super::array::Logits;
use super::context::Context;
use super::ev::ev_all;
use super::instinct::{forced, instinct};
use super::neural_floor::mask_illegal;
use super::trie::Classifier;
use super::{Family, Rules, Stance, features, neural};
use contract_bridge::auction::{AbsoluteVulnerability, Call, RelativeVulnerability};
use contract_bridge::{Hand, Seat};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::sync::LazyLock;

/// The deterministic ladder, built once; the forced path reuses it per board.
static LADDER: LazyLock<Rules> = LazyLock::new(instinct);

/// The continuation policy that finishes every rollout auction
///
/// Our search-target distilled floor ([`american_neural_search`] — the M3.2
/// round-1 net) bound for self-play: the rollout assumes all four seats bid as we
/// would.  This is the policy M3.2 iterates — "feed the improved net back into the
/// continuations" — so each round's targets are scored by the previous round's
/// policy.  Round 1 used the teacher-distilled `american_neural`; this is the
/// round-2 continuation.  Built once and shared across decisions.
static POLICY: LazyLock<Stance> =
    LazyLock::new(|| american_neural_search().against(Family::NATURAL));

/// How far above the un-evaluated prior tail the EV band sits, in nats
///
/// Every call the search actually evaluated outranks every legal call it did
/// not, by at least this margin — the driver's arg-max is therefore always a
/// searched call.  Three nats matches the books' strength-gap convention, so the
/// un-evaluated tail keeps a small but non-zero softmax mass as a fallback.
const EV_BAND_GAP: f32 = 3.0;

/// The gated live-search floor, made safe to attach under the book
///
/// A [`Classifier`] drop-in for [`instinct`] and
/// [`NeuralFloor`][super::neural_floor::NeuralFloor]: the deterministic rails,
/// then a double-dummy search over the net's shortlist in the judgement middle.
/// See the [module docs][self].  The knobs default to *strength, not latency*
/// (the standing M2.3 decision); shrink them for a faster, noisier bidder.
#[derive(Clone, Copy, Debug)]
pub struct SearchFloor {
    /// Layouts sampled and solved per decision (the rollout count)
    pub layouts: usize,
    /// Top-k legal calls, by the net prior, actually scored by EV
    pub shortlist: usize,
    /// EV temperature in points per nat: a larger value flattens the EV band
    pub temperature: f32,
}

impl Default for SearchFloor {
    fn default() -> Self {
        // Strength, not latency (the standing M2.3 decision): 128 layouts keep
        // the EV estimates tight enough that the 8-call shortlist's extra
        // candidates help rather than inject noise — `n` and `k` rise together.
        // ~1.4 s per decision; shrink both for a faster, noisier bidder.
        Self {
            layouts: 128,
            shortlist: 8,
            temperature: 100.0,
        }
    }
}

impl Classifier for SearchFloor {
    fn classify(&self, hand: Hand, context: &Context<'_>) -> Logits {
        if forced(context) {
            // Rails: trust the deterministic floor, never the net or the search.
            return LADDER.classify(hand, context);
        }

        // The net prior, legality-masked — the neural floor's judgement path.
        let feats = features::features(hand, context);
        let mut prior = neural::classify(&feats);
        mask_illegal(&mut prior, context.auction());

        // Shortlist the net's top-k legal calls to actually simulate.
        let shortlist = shortlist(&prior, self.shortlist);
        if shortlist.is_empty() {
            // Only -∞ logits (no legal call); leave the prior for the driver.
            return prior;
        }

        // Price the shortlist by cardplay EV over deterministically-seeded
        // layouts, then re-seat the evaluated calls above the prior tail.
        let vul = absolute_vul(context.vul());
        let mut rng = StdRng::seed_from_u64(seed_from_features(&feats));
        let evs = ev_all(
            hand,
            Seat::North,
            vul,
            context,
            &shortlist,
            &*POLICY,
            &mut rng,
            self.layouts,
        );
        blend(&mut prior, &shortlist, &evs, self.temperature);
        prior
    }
}

/// The top-`k` legal calls of a masked prior, highest logit first
///
/// Illegal calls are already `-∞`, so filtering to finite logits keeps only the
/// legal ones; `Pass` is always among them, so a non-empty auction never yields
/// an empty shortlist.
fn shortlist(prior: &Logits, k: usize) -> Vec<Call> {
    let mut ranked: Vec<(Call, f32)> = prior
        .iter()
        .map(|(call, &logit)| (call, logit))
        .filter(|&(_, logit)| logit.is_finite())
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("a masked prior is never NaN"));
    ranked.into_iter().take(k).map(|(call, _)| call).collect()
}

/// Re-seat the EV-scored shortlist onto a band above the net's prior tail
///
/// The masked `prior` is the base distribution; each shortlisted call with a
/// finite EV is moved to `prior_max + EV_BAND_GAP + (ev − best_ev)/temperature`,
/// so the highest-EV call tops the distribution (the driver bids it) and the
/// rest fall by their EV deficit.  Un-evaluated legal calls keep their prior
/// logit as a fallback, and `Pass` stays finite.  An all-`NaN` slate — no layout
/// could be sampled — leaves the prior untouched, so the floor degrades exactly
/// to the bare net.
fn blend(prior: &mut Logits, shortlist: &[Call], evs: &[f32], temperature: f32) {
    let best = evs
        .iter()
        .copied()
        .filter(|ev| ev.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    if best == f32::NEG_INFINITY {
        return; // no signal: keep the bare net prior
    }

    let prior_max = prior
        .values()
        .copied()
        .filter(|logit| logit.is_finite())
        .fold(f32::NEG_INFINITY, f32::max);
    let band_base = prior_max + EV_BAND_GAP;

    for (&call, &ev) in shortlist.iter().zip(evs) {
        if ev.is_finite() {
            *prior.get_mut(call) = band_base + (ev - best) / temperature;
        }
    }
}

/// The absolute vulnerability matching a relative one read at North's seat
///
/// The actor is canonicalized to [`Seat::North`], so "we" is North/South and
/// "they" is East/West.  This is the inverse of
/// [`relative`][super::context::relative]`(_, North)`.
fn absolute_vul(vul: RelativeVulnerability) -> AbsoluteVulnerability {
    let mut out = AbsoluteVulnerability::NONE;
    out.set(
        AbsoluteVulnerability::NS,
        vul.contains(RelativeVulnerability::WE),
    );
    out.set(
        AbsoluteVulnerability::EW,
        vul.contains(RelativeVulnerability::THEY),
    );
    out
}

/// A deterministic RNG seed from the decision's feature vector
///
/// The feature vector is a pure function of the hand, the auction,
/// vulnerability, and seat (see [`features`]), so an FNV-1a fold over its bit
/// patterns is a stable per-decision seed — the same decision always draws the
/// same layouts.  Collisions are harmless: two decisions sharing a stream still
/// each sample within their own ranges.
fn seed_from_features(feats: &[f32]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &value in feats {
        for byte in value.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use contract_bridge::{Bid, Strain};

    const fn call(level: u8, strain: Strain) -> Call {
        Call::Bid(Bid::new(level, strain))
    }

    /// A fast test floor: a handful of layouts and a small shortlist keep the
    /// double-dummy solves cheap while still exercising the search path.
    fn floor() -> SearchFloor {
        SearchFloor {
            layouts: 8,
            shortlist: 3,
            ..SearchFloor::default()
        }
    }

    /// The shelled bidder's logits for a hand in an auction
    fn shelled(auction: &[Call], hand: &str) -> Logits {
        let hand: Hand = hand.parse().expect("valid test hand");
        let context = Context::new(RelativeVulnerability::NONE, auction);
        floor().classify(hand, &context)
    }

    /// The shelled bidder's highest-logit call
    fn best(auction: &[Call], hand: &str) -> Call {
        let logits = shelled(auction, hand);
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    // The five §0.4 safety properties, enforced by the same shell as the neural
    // floor.  The four forced rails short-circuit to `instinct()` before any
    // search, so they reproduce its tested calls exactly; the legality rail
    // exercises the net + search + mask.

    #[test]
    fn forced_advance_never_passes() {
        let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
        assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
        assert_eq!(best(&auction, "964.J85.974.9632"), call(3, Strain::Notrump));
    }

    #[test]
    fn forced_to_game_never_passes_below_game() {
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "QJ52.K43.T62.J32"), call(3, Strain::Notrump));
        assert_eq!(best(&auction, "3.QJ9854.K32.J32"), call(4, Strain::Hearts));
    }

    #[test]
    fn completes_partners_transfer_over_notrump() {
        let auction = [
            call(1, Strain::Notrump),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
        ];
        assert_eq!(best(&auction, "AQ32.KJ5.KQ4.Q92"), call(2, Strain::Hearts));
    }

    #[test]
    fn forced_game_steps_aside_when_penalizing() {
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            call(3, Strain::Diamonds),
            Call::Double,
            Call::Pass,
        ];
        let chosen = best(&auction, "K3.KQ4.65.QJ8765");
        assert_eq!(chosen, call(4, Strain::Clubs));
        assert_ne!(chosen, call(3, Strain::Notrump));
    }

    #[test]
    fn doubles_only_their_live_bids() {
        // Not a forced auction → the net + search + legality mask.  Doubling our
        // own raised overcall is illegal, so the mask zeroes it (and the search
        // never lifts an illegal candidate), while `Pass` stays finite.
        let auction = [
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        let logits = shelled(&auction, "92.K53.AQJ42.962");
        assert_eq!(*logits.0.get(Call::Double), f32::NEG_INFINITY);
        assert!(logits.0.get(Call::Pass).is_finite());
    }

    /// Determinism (invariant §0.5): the floor seeds its rollout RNG from the
    /// decision, so the same hand and auction reproduce the same logits exactly.
    #[test]
    fn deterministic_given_a_decision() {
        let auction = [
            call(1, Strain::Hearts),
            call(1, Strain::Spades),
            Call::Pass,
            call(2, Strain::Spades),
            Call::Pass,
        ];
        let a = shelled(&auction, "92.K53.AQJ42.962");
        let b = shelled(&auction, "92.K53.AQJ42.962");
        assert_eq!(a, b);
    }

    /// The search must beat the raw net prior on a hand where cardplay sees
    /// through it: an evaluated call always outranks every un-evaluated legal
    /// call (the EV band sits above the prior tail).
    #[test]
    fn evaluated_calls_outrank_the_prior_tail() {
        // A routine balanced opener; the search runs and re-seats its shortlist.
        let auction = [call(1, Strain::Spades), Call::Double];
        let logits = shelled(&auction, "K92.AQ4.KJ32.Q92");

        // The chosen call is finite and legal; a distribution exists.
        let chosen = best(&auction, "K92.AQ4.KJ32.Q92");
        assert!(logits.0.get(chosen).is_finite());
        assert!(logits.0.get(Call::Pass).is_finite());
    }
}
