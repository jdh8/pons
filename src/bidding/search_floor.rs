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

use super::american::{american_neural_search, american_search};
use super::array::Logits;
use super::context::Context;
use super::ev::ev_all;
use super::instinct::{forced, instinct};
use super::neural_floor::mask_illegal;
use super::trie::Classifier;
use super::{Family, Rules, Stance, System, features, neural};
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

        // Shortlist the net's top-k legal calls to actually simulate, then price
        // them by cardplay EV and re-seat the best above the prior tail.
        let shortlist = shortlist(&prior, self.shortlist);
        if shortlist.is_empty() {
            // Only -∞ logits (no legal call); leave the prior for the driver.
            return prior;
        }
        price_and_blend(&mut prior, &shortlist, hand, context, &feats, self);
        prior
    }
}

/// Price `candidates` by cardplay EV and re-seat them onto `base`
///
/// The reusable core shared by [`SearchFloor`] (prior = the net) and
/// [`SearchBook`] (prior = the authored book leaf ∪ the net): sample layouts,
/// solve once, [`ev_all`]-price every candidate over the shared solves under the
/// continuation [`POLICY`], then [`blend`] the EV-ranked band above the prior
/// tail.  `feats` is the decision's feature vector — passed in (both callers
/// already have it) so the RNG seed stays a pure function of the decision
/// (determinism, §0.5) without recomputing the features.  A no-op on an empty
/// candidate set; an all-`NaN` slate leaves `base` untouched (degrade to the bare
/// prior).
fn price_and_blend(
    base: &mut Logits,
    candidates: &[Call],
    hand: Hand,
    context: &Context<'_>,
    feats: &[f32],
    knobs: &SearchFloor,
) {
    if candidates.is_empty() {
        return;
    }
    let vul = absolute_vul(context.vul());
    let mut rng = StdRng::seed_from_u64(seed_from_features(feats));
    let evs = ev_all(
        hand,
        Seat::North,
        vul,
        context,
        candidates,
        &*POLICY,
        &mut rng,
        knobs.layouts,
    );
    blend(base, candidates, &evs, knobs.temperature);
}

/// The 2/1 search bidder that prices **authored book leaves** by DD too — M7.0
///
/// Where [`SearchFloor`] runs the double-dummy search only where the book is
/// silent (the contested fallback floor), `SearchBook` widens it to every
/// non-forced *book* leaf: the leaf's authored logits become the search *prior*
/// instead of the final word, so cardplay re-judges among the calls the rule
/// proposed — unioned with the net's natural alternatives — and bids the
/// highest-EV one.  "Rules propose, DD disposes", at every leaf (see
/// [`05-search-at-every-leaf.md`]).  The authored *constraints* (meaning) stay;
/// only the authored *weights* (judgement) are overridden.
///
/// It wraps a **bound** [`Stance`] (so build it with
/// [`american_search_book`], which seats the live-search floor underneath it),
/// and inherits every §0 safety invariant verbatim:
///
/// - **Forced rails first** — a `forced` auction delegates to the wrapped
///   stance (book leaf on-book, the [`SearchFloor`]'s `forced → instinct()`
///   off-book); the search never runs on the rails.
/// - **Floors already handled** — an auction that falls past the book to a
///   fallback floor (the [`SearchFloor`] on contested, the deterministic
///   [`instinct`][instinct()] ladder on constructive) is returned as that floor
///   gave it; only a real authored leaf (provenance `fallback: None`, with mass)
///   is re-priced.
/// - **Legality + determinism** — the same `mask_illegal` / `blend` masking and
///   RNG seeding as [`SearchFloor`].
///
/// This is the **treatment arm** of the M7 A/B against [`american_search`] (DD
/// off-book only); strong but even slower (it searches every non-forced on-book
/// decision).  Gated behind the `search` feature; [`american`][super::american::american]
/// and [`instinct`][instinct()] are untouched and stay the default.
///
/// [`05-search-at-every-leaf.md`]: https://github.com/Chen-Pang-He/pons/blob/main/docs/ai-bidder/05-search-at-every-leaf.md
/// [`american_search`]: super::american::american_search
// ponytail: american_search_book is the M7 treatment arm; collapse into
// american_search (default-on knob) on a measured win, or gate per book if it
// splits contested vs constructive.
#[derive(Clone, Debug)]
pub struct SearchBook {
    stance: Stance,
    knobs: SearchFloor,
}

impl SearchBook {
    /// Wrap a bound [`Stance`] with default search knobs
    #[must_use]
    pub fn new(stance: Stance) -> Self {
        Self::with(stance, SearchFloor::default())
    }

    /// Wrap a bound [`Stance`] with explicit search knobs
    ///
    /// The knobs (`layouts`/`shortlist`/`temperature`) are the same
    /// [`SearchFloor`] bundle, so data-generation and tuning runs can trade
    /// strength for speed without re-wiring.
    #[must_use]
    pub const fn with(stance: Stance, knobs: SearchFloor) -> Self {
        Self { stance, knobs }
    }
}

impl System for SearchBook {
    fn classify(&self, hand: Hand, vul: RelativeVulnerability, auction: &[Call]) -> Option<Logits> {
        let context = Context::new(vul, auction);
        if forced(&context) {
            // Rails: the wrapped stance's deterministic answer, never the search.
            return self.stance.classify(hand, vul, auction);
        }

        let (mut book, provenance) = self.stance.classify_with_provenance(hand, vul, auction)?;
        if provenance.fallback.is_some() {
            // Fell past the book to a fallback floor — the SearchFloor (contested)
            // or the instinct ladder (constructive) already produced this; leave it.
            return Some(book);
        }

        // A real authored book leaf with mass: price it by DD instead of trusting
        // its fixed weights.  Candidate set = the rule's own calls ∪ the net's
        // top-k natural alternatives, so DD can override a one-call rule.
        mask_illegal(&mut book, auction);
        let feats = features::features(hand, &context);
        let mut net = neural::classify(&feats);
        mask_illegal(&mut net, auction);
        let candidates = union_dedup(finite_calls(&book), shortlist(&net, self.knobs.shortlist));
        price_and_blend(&mut book, &candidates, hand, &context, &feats, &self.knobs);
        Some(book)
    }
}

/// The 2/1 pair as a leaf-pricing [`SearchBook`], bound against `them` — M7.0
///
/// Builds the M7 treatment arm: [`american_search`] (the live-search floor under
/// the bare authored books) bound against the opposing [`Family`], then wrapped
/// so every non-forced book leaf is DD-priced too.  The named handle the
/// `search-book` A/B uses; see [`SearchBook`] for the lifecycle of this name.
/// Gated behind the `search` feature.
#[must_use]
pub fn american_search_book(them: Family) -> SearchBook {
    SearchBook::new(american_search().against(them))
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

/// The legal (finite-logit) calls of a masked `logits`, in array order
///
/// For an authored book leaf this is the calls the rule actually proposes — the
/// conventional candidates [`SearchBook`] unions with the net's natural
/// alternatives before pricing them by DD.
fn finite_calls(logits: &Logits) -> Vec<Call> {
    logits
        .iter()
        .filter(|&(_, &logit)| logit.is_finite())
        .map(|(call, _)| call)
        .collect()
}

/// `a` followed by the entries of `b` not already present, order preserved
///
/// The candidate slates are a handful of calls each, so the linear membership
/// scan is free.
fn union_dedup(mut a: Vec<Call>, b: Vec<Call>) -> Vec<Call> {
    for call in b {
        if !a.contains(&call) {
            a.push(call);
        }
    }
    a
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
    fn advancing_a_double_delegates_to_instinct() {
        // The shell short-circuits to instinct, reproducing its calls — advance a
        // bust with an outside suit, defend with length behind their suit.
        let auction = [call(3, Strain::Clubs), Call::Double, Call::Pass];
        assert_eq!(best(&auction, "96432.J85.9742.2"), call(3, Strain::Spades));
        assert_eq!(best(&auction, "964.J85.974.9632"), Call::Pass);
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

    // M7.0 — the leaf-pricing bidder.  Small knobs keep the DD solves cheap; the
    // wrapped stance is the live-search pair bound against a natural opponent.

    /// A fast book-search bidder over the bound 2/1 stance.
    fn book() -> (Stance, SearchBook) {
        let stance = american_search().against(Family::NATURAL);
        let knobs = SearchFloor {
            layouts: 8,
            shortlist: 3,
            ..SearchFloor::default()
        };
        (stance.clone(), SearchBook::with(stance, knobs))
    }

    /// The book-search bidder's highest-logit call for a hand in an auction
    fn book_best(book: &SearchBook, auction: &[Call], hand: &str) -> Call {
        let hand: Hand = hand.parse().expect("valid test hand");
        let logits = book
            .classify(hand, RelativeVulnerability::NONE, auction)
            .expect("a covered auction");
        (&logits.0)
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).expect("logits are never NaN"))
            .map(|(call, _)| call)
            .expect("array is never empty")
    }

    /// Rails (§0.4): on a forced auction the wrapper delegates to the stance
    /// verbatim — the search never re-prices the rails, so the logits are
    /// identical.  (If `forced` failed to fire, the search would mutate them.)
    #[test]
    fn forced_leaf_delegates_to_the_stance() {
        let (stance, book) = book();
        // 2♣ opening, forced to game: a forced-to-game responder.
        let auction = [
            call(2, Strain::Clubs),
            Call::Pass,
            call(2, Strain::Diamonds),
            Call::Pass,
            call(2, Strain::Notrump),
            Call::Pass,
        ];
        let hand: Hand = "3.QJ9854.K32.J32".parse().expect("valid test hand");
        assert_eq!(
            book.classify(hand, RelativeVulnerability::NONE, &auction),
            stance.classify(hand, RelativeVulnerability::NONE, &auction),
        );
    }

    /// Legality (§0.4): a real, non-forced book leaf gets DD-priced and yields a
    /// finite, legal arg-max.  The authored leaf still owns the *meaning*: an
    /// opening forbids `Pass` (`-∞`), and the wrapper faithfully keeps it that way
    /// — DD re-judges only among the calls the rule (∪ the net) proposes, it never
    /// resurrects a call the agreement forbade.
    #[test]
    fn priced_leaf_arg_max_is_legal() {
        let (_, book) = book();
        // First seat, a clear opener: the constructive opening is a book leaf.
        let auction = [];
        let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
        let logits = book
            .classify(hand, RelativeVulnerability::NONE, &auction)
            .expect("the opening leaf answers");
        assert!(logits.has_mass(), "the leaf gives the hand a finite call");
        let chosen = book_best(&book, &auction, "AKQ2.KQ5.AQJ4.92");
        assert!(logits.0.get(chosen).is_finite());
        assert_ne!(chosen, Call::Pass, "a 21-count never passes the opening");
    }

    /// Determinism (§0.5): the wrapper seeds its rollout RNG from the decision,
    /// so the same hand and auction reproduce the same logits exactly.
    #[test]
    fn priced_leaf_is_deterministic() {
        let (_, book) = book();
        let auction = [];
        let hand: Hand = "AKQ2.KQ5.AQJ4.92".parse().expect("valid test hand");
        let a = book.classify(hand, RelativeVulnerability::NONE, &auction);
        let b = book.classify(hand, RelativeVulnerability::NONE, &auction);
        assert_eq!(a, b);
    }
}
