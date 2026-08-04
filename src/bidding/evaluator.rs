//! The learned trick evaluator — bilans session C.
//!
//! One forward pass answers the question BBA's *bilans* engine answers by
//! reconstructing hands and counting winners and losers: **how many
//! double-dummy tricks does each declarer take in each strain**, given my own
//! cards and the range envelopes the auction has put on the other three hands?
//!
//! It is an amortization of [`sample_layouts`][crate::bidding::sampler::sample_layouts] +
//! `solve_deals` — the sample-and-solve loop that costs ~1.4 s per decision,
//! learned offline into a few thousand multiply-adds. Its input
//! ([`features_eval`]) carries **no auction, no seat and no vulnerability**: the
//! auction enters only through the [`Inferences`] the book distilled from it, so
//! the same weights serve any bidding system. Score, vulnerability and doubling
//! are economics and belong to the caller; this module is physics.
//!
//! Uncertainty comes back as a **Gaussian per contract**, not a point estimate:
//! two heads per target, mean and `ln σ`, fit by negative log-likelihood on
//! single deals. Each training row is one real deal consistent with its ranges —
//! one unbiased draw from the posterior over hidden hands — so minimising NLL
//! over the population drives `μ` to the conditional mean and `σ` to the
//! conditional spread without ever sampling a state twice. The spread costs one
//! extra output column and no extra labels: the net simply has to explain the
//! size of its own residual.
//!
//! `(μ, σ)` is a sufficient statistic, so every threshold the floor asks about
//! is a closed-form `Φ` away — no knots, no interpolation, and a CDF that stays
//! smooth out into the tails where an interpolated one would have to clamp.
//!
//! Consumed by the instinct floor's game/slam boundary gates behind
//! [`set_bilans_floor`][super::instinct::set_bilans_floor] (bilans session D,
//! default off pending its A/B); the module itself is ungated and always
//! builds.

use super::features::{
    FEATURES_LEN_EVAL, FEATURES_LEN_EVAL_V3, FEATURES_LEN_EVAL_V4, features_eval, features_eval_v3,
    features_eval_v4,
};
use super::inference::{Inferences, Relative, envelope_union_reading, pass_exclusion_reading};
use super::neural::{affine, decode, relu};
use contract_bridge::auction::Call;
use contract_bridge::{Hand, Strain};
use nalgebra::SVectorView;
use std::cell::Cell;
use std::sync::LazyLock;

/// Input width, pinned to the artifact.
const IN: usize = FEATURES_LEN_EVAL;
/// Hidden width of both hidden layers.
const HID: usize = 256;
/// Trick targets: 5 strains × 4 declarers.
const TARGETS: usize = 20;
/// Heads per target: the mean and the log standard deviation.
const HEADS: usize = 2;
/// Output width.
const OUT: usize = HEADS * TARGETS;

/// Float count of the MLP (`W1,b1,W2,b2,W3,b3`).
const TOTAL: usize = HID * IN + HID + HID * HID + HID + OUT * HID + OUT;

/// Bounds on the `ln σ` head, matching the trainer's clamp exactly — in the
/// corpus's `tricks / 13` units this is σ ∈ [0.087, 13] tricks. Serving must
/// clamp identically to training or the two disagree on the same weights.
const LN_SD_MIN: f32 = -5.0;
/// Upper bound on `ln σ`; see [`LN_SD_MIN`].
const LN_SD_MAX: f32 = 0.0;

static RAW: &[u8] = include_bytes!("weights/evaluator_v2.f32");
const _: () = assert!(
    RAW.len() == TOTAL * 4,
    "evaluator weights artifact size mismatch"
);

/// The knob-matched twin (DNF chop F2b): same architecture and training
/// recipe, corpus regenerated with `set_envelope_union_reading(true)` so the range
/// blocks come from the tightened prefixed readings a knob-on bidder serves.
/// Selected per call by [`envelope_union_reading`]; knob-off never touches it.
static RAW_UNION_READING: &[u8] = include_bytes!("weights/evaluator_v2_dnf.f32");
const _: () = assert!(
    RAW_UNION_READING.len() == TOTAL * 4,
    "envelope-union evaluator weights artifact size mismatch"
);

/// Weights decoded to `f32` once, on first use.
static WEIGHTS: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW));

/// [`RAW_UNION_READING`] decoded once, on first use.
static WEIGHTS_UNION_READING: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_UNION_READING));

/// Input width of the v3 (calls-tail) artifact.
const IN_V3: usize = FEATURES_LEN_EVAL_V3;

/// Float count of the v3 MLP — same architecture, wider first layer.
const TOTAL_V3: usize = HID * IN_V3 + HID + HID * HID + HID + OUT * HID + OUT;

/// The calls-tail evaluator (`features_eval_v3`), trained on the envelope-union
/// reading regime — the only regime it serves; see
/// [`trick_estimates_with_auction`].
static RAW_V3_UNION_READING: &[u8] = include_bytes!("weights/evaluator_v3_dnf.f32");
const _: () = assert!(
    RAW_V3_UNION_READING.len() == TOTAL_V3 * 4,
    "v3 evaluator weights artifact size mismatch"
);

/// [`RAW_V3_UNION_READING`] decoded once, on first use.
static WEIGHTS_V3_UNION_READING: LazyLock<Vec<f32>> =
    LazyLock::new(|| decode(RAW_V3_UNION_READING));

/// The pass-exclusion twin of the v3 artifact — same architecture and recipe,
/// corpus regenerated with `set_pass_exclusion_reading(true)` on top of the
/// envelope-union regime (val NLL −1.55010 vs the union-reading twin's −1.54872 on its own
/// regime).  Selected per call by [`pass_exclusion_reading`] inside the v3
/// path; knob-off never touches it.
static RAW_V3_EXCLUSION: &[u8] = include_bytes!("weights/evaluator_v3_exclusion.f32");
const _: () = assert!(
    RAW_V3_EXCLUSION.len() == TOTAL_V3 * 4,
    "exclusion evaluator weights artifact size mismatch"
);

/// [`RAW_V3_EXCLUSION`] decoded once, on first use.
static WEIGHTS_V3_EXCLUSION: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_V3_EXCLUSION));

/// Input width of the v4 (shape-reading) artifact.
const IN_V4: usize = FEATURES_LEN_EVAL_V4;

/// Float count of the v4 MLP — same architecture, three columns wider.
const TOTAL_V4: usize = HID * IN_V4 + HID + HID * HID + HID + OUT * HID + OUT;

/// The shape-reading evaluator (`features_eval_v4`), trained on the envelope-union
/// regime whose union readings the shape block conditions on; see
/// [`set_eval_shape`].
static RAW_V4_UNION_READING: &[u8] = include_bytes!("weights/evaluator_v4_dnf.f32");
const _: () = assert!(
    RAW_V4_UNION_READING.len() == TOTAL_V4 * 4,
    "v4 evaluator weights artifact size mismatch"
);

/// [`RAW_V4_UNION_READING`] decoded once, on first use.
static WEIGHTS_V4_UNION_READING: LazyLock<Vec<f32>> =
    LazyLock::new(|| decode(RAW_V4_UNION_READING));

std::thread_local! {
    /// Whether [`trick_estimates_with_auction`] serves the v3 calls-tail
    /// artifact (see [`set_eval_auction`]).  On by default.
    static EVAL_AUCTION: Cell<bool> = const { Cell::new(true) };

    /// Whether [`trick_estimates_with_auction`] serves the v4 shape-reading
    /// artifact (see [`set_eval_shape`]).
    static EVAL_SHAPE: Cell<bool> = const { Cell::new(false) };
}

/// Serve the v3 calls-tail evaluator (default **on**, shipped 2026-07-27)
///
/// On, [`trick_estimates_with_auction`] feeds [`features_eval_v3`] — the hull
/// vector plus the last four call identities — to the v3 artifact, which the
/// 2026-07-27 NLL ablation priced at 0.038 over the hull-only vector (bare
/// calls; docs/ai-bidder/evaluator-net.md §auction-input ablation).  The A/B
/// shipped it default-on with a `win | win` verdict: plain DD +0.0180 ± 0.0042
/// (none) / +0.0284 ± 0.0056 (both), PD +0.0222 / +0.0360, on 204,800
/// boards/arm/vul at `SEED_BASE` 1785138816 — fired 1.3–1.6%, +1.3 to +2.3
/// IMPs per fired board at the bilans game/slam gates.  The v3 twin was
/// trained on the [`envelope_union_reading`] regime only, so the knob is only honoured
/// there; anywhere else the v2 path serves as before.
///
/// Per-thread, like every reading knob; set it inside worker closures.
pub fn set_eval_auction(on: bool) {
    EVAL_AUCTION.with(|cell| cell.set(on));
}

/// Whether the v3 calls-tail evaluator is enabled (default on)
#[must_use]
pub fn eval_auction() -> bool {
    EVAL_AUCTION.with(Cell::get)
}

/// Serve the v4 shape-reading evaluator (default **off**, pending its A/B)
///
/// On, [`trick_estimates_with_auction`] feeds [`features_eval_v4`] to the v4
/// artifact: v3's vector with each hidden seat's four length `{min, max}` pairs
/// replaced by its **shape distribution** — `E[len]` and `sd[len]` per suit over
/// the 560-shape lattice, plus one column for how much the reading pins the seat
/// down.  Three columns wider than v3, and worth nothing in NLL: the round-two
/// ablation scored the encoding at +0.00004 against a matched control on 8.15M
/// rows, inside a 0.0006 seed spread.
///
/// The prize is **invariance**, not accuracy.  A hull is not a well-defined
/// function of a reading — `♥5..13` and `♥5..8` are the same claim yet differ by
/// a third of the column's range — so `set_sum_closure`, which provably rejects
/// no hand, still displaces the endpoint columns at 81% of nodes by up to 4.19σ
/// and has to buy a retrain before it can be judged on merit.  The shape columns
/// move at 0.11% of nodes by up to 0.07σ, and that 0.11% is where the reading
/// genuinely changed.  Under this knob the reading-fidelity chops become
/// measurable on their own terms.
///
/// Supersedes [`set_eval_auction`] when both are on — v4 carries the calls tail
/// verbatim.  Like the v3 twin it was trained on the [`envelope_union_reading`] regime
/// only, and its shape block reads the *union* of announced boxes, so it is
/// honoured only there.
///
/// Per-thread, like every reading knob; set it inside worker closures.
pub fn set_eval_shape(on: bool) {
    EVAL_SHAPE.with(|cell| cell.set(on));
}

/// Whether the v4 shape-reading evaluator is enabled (default off)
#[must_use]
pub fn eval_shape() -> bool {
    EVAL_SHAPE.with(Cell::get)
}

/// The strain order the training label uses (`gib::relativized_tricks`, itself
/// the GIB tail order). [`Strain`]'s own discriminants ascend ♣♦♥♠NT, so this
/// is exactly the reverse — see [`TrickEstimates::get`].
const STRAIN_ROWS: usize = 5;

/// A trick count's estimated distribution: Gaussian, in tricks.
///
/// ponytail: a Gaussian is symmetric and unbounded, and double-dummy trick
/// counts are neither — on a good fit they are left-skewed and pile up against
/// the hard ceiling of 13. The fit absorbs that as extra spread, which shades
/// `p_at_least` toward the mean wherever the truth is skewed. Harmless if the
/// consumer only compares candidate contracts; the upgrade path if it ever
/// costs IMPs is categorical per-trick heads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaussian {
    /// Expected double-dummy tricks
    pub mean: f32,
    /// Standard deviation of the trick count over consistent deals
    pub sd: f32,
}

impl Gaussian {
    /// The fitted CDF at `x`.
    #[must_use]
    pub fn cdf(self, x: f32) -> f32 {
        standard_normal_cdf((x - self.mean) / self.sd)
    }

    /// Estimated probability of taking at least `tricks` tricks.
    ///
    /// The half-trick continuity correction bridges a discrete trick count and
    /// a continuous fitted CDF: `P(T ≥ k) = 1 − F(k − ½)`.
    #[must_use]
    pub fn p_at_least(self, tricks: u8) -> f32 {
        1.0 - self.cdf(f32::from(tricks) - 0.5)
    }
}

/// Standard normal CDF, Abramowitz & Stegun 26.2.17 — max abs error 7.5e-8,
/// itself below `f32` resolution. Evaluated in `f64` so the published
/// coefficients keep their full precision.
fn standard_normal_cdf(z: f32) -> f32 {
    /// Horner coefficients, ascending in `t`.
    const B: [f64; 5] = [
        0.319_381_53,
        -0.356_563_782,
        1.781_477_937,
        -1.821_255_978,
        1.330_274_429,
    ];
    /// 1/√(2π)
    const INV_SQRT_TAU: f64 = 0.398_942_280_401_432_7;

    let x = f64::from(z).abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * x);
    // b1·t + b2·t² + … + b5·t⁵
    let poly = B.iter().rev().fold(0.0, |acc, b| (acc + b) * t);
    // The upper tail 1 − Φ(|z|); by symmetry that is Φ(z) itself when z < 0.
    let upper = INV_SQRT_TAU * (-0.5 * x * x).exp() * poly;
    (if z < 0.0 { upper } else { 1.0 - upper }) as f32
}

/// Estimated double-dummy tricks for all 20 (strain, declarer) pairs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrickEstimates([[Gaussian; 4]; STRAIN_ROWS]);

impl TrickEstimates {
    /// The estimate for one contract's strain and declarer, the declarer named
    /// relative to the player whose hand was evaluated.
    #[must_use]
    pub fn get(&self, strain: Strain, declarer: Relative) -> Gaussian {
        // `Strain::ASC` runs ♣♦♥♠NT and the label rows run NT♠♥♦♣.
        self.0[STRAIN_ROWS - 1 - strain as usize][declarer as usize]
    }

    /// Estimated probability that `declarer` takes at least `tricks` tricks in
    /// `strain` — the make probability of a contract at that level.
    #[must_use]
    pub fn p_at_least(&self, strain: Strain, declarer: Relative, tricks: u8) -> f32 {
        self.get(strain, declarer).p_at_least(tricks)
    }
}

/// Evaluate a hand against what the auction has shown about the other three.
///
/// Take `inferences` from [`Stance::infer`][super::Stance::infer], which routes
/// the auction through the book's trie so conventional calls decode off their
/// authoring rules; a bare [`Context`][super::Context] reading is looser and is
/// not the distribution this net was fit on.
///
/// Deterministic — fixed weights, no RNG, no solver.
#[must_use]
pub fn trick_estimates(hand: Hand, inferences: &Inferences) -> TrickEstimates {
    let x = features_eval(hand, inferences);
    debug_assert_eq!(x.len(), IN);
    reshape(forward(&x))
}

/// [`trick_estimates`], with the raw auction available for the v3 calls-tail
/// artifact.
///
/// Under [`set_eval_auction`] **and** the [`envelope_union_reading`] regime the v3 twin
/// was trained on, this serves [`features_eval_v3`] — the same vector plus the
/// last four call identities — from the weight set matching the calling
/// thread's [`pass_exclusion_reading`] regime.  Under [`set_eval_shape`] it
/// serves [`features_eval_v4`] instead, which carries that tail and reads each
/// hidden seat as a shape distribution rather than a bounding box.  Anywhere
/// else it is exactly [`trick_estimates`], byte for byte, so call sites can
/// migrate to this signature unconditionally.
#[must_use]
pub fn trick_estimates_with_auction(
    hand: Hand,
    inferences: &Inferences,
    calls: &[Call],
) -> TrickEstimates {
    // Both twins were fit on the tightened prefixed readings a knob-on bidder
    // serves, and v4's shape block conditions on the box union itself.
    if !envelope_union_reading() {
        return trick_estimates(hand, inferences);
    }
    if eval_shape() {
        let x = features_eval_v4(hand, inferences, calls);
        debug_assert_eq!(x.len(), IN_V4);
        return reshape(forward_with::<IN_V4>(&WEIGHTS_V4_UNION_READING, &x));
    }
    if !eval_auction() {
        return trick_estimates(hand, inferences);
    }
    let x = features_eval_v3(hand, inferences, calls);
    debug_assert_eq!(x.len(), IN_V3);
    // The exclusion twin was fit on readings carrying the pass-exclusion caps;
    // serving it only under its knob keeps knob-off byte-identical.
    let weights = if pass_exclusion_reading() {
        &WEIGHTS_V3_EXCLUSION
    } else {
        &WEIGHTS_V3_UNION_READING
    };
    reshape(forward_with::<IN_V3>(weights, &x))
}

/// Reshape the raw head-major outputs — all 20 means, then all 20 log
/// deviations, in units of tricks / 13 (the scale `gib::relativized_tricks`
/// labels in) — into [`TrickEstimates`].
fn reshape(z: [f32; OUT]) -> TrickEstimates {
    let mut out = [[Gaussian { mean: 0.0, sd: 0.0 }; 4]; STRAIN_ROWS];
    for (i, slot) in out.iter_mut().flatten().enumerate() {
        *slot = Gaussian {
            mean: 13.0 * z[i],
            sd: 13.0 * z[TARGETS + i].clamp(LN_SD_MIN, LN_SD_MAX).exp(),
        };
    }
    TrickEstimates(out)
}

/// The raw `OUT` outputs, before reshaping and rescaling. Serves the weights
/// fit on the reading regime the calling thread is actually in: the knob-on
/// twin under [`envelope_union_reading`], the shipped artifact otherwise.
pub(super) fn forward(x: &[f32]) -> [f32; OUT] {
    let weights = if envelope_union_reading() {
        WEIGHTS_UNION_READING.as_slice()
    } else {
        WEIGHTS.as_slice()
    };
    forward_with::<IN>(weights, x)
}

/// Benchmark-only forward half of the active v3 calls-tail evaluator.
#[cfg(feature = "bench-internals")]
pub(super) fn forward_v3(x: &[f32]) -> [f32; OUT] {
    assert_eq!(x.len(), IN_V3, "v3 evaluator feature width");
    let weights = if pass_exclusion_reading() {
        &WEIGHTS_V3_EXCLUSION
    } else {
        &WEIGHTS_V3_UNION_READING
    };
    forward_with::<IN_V3>(weights, x)
}

/// Benchmark-only forward half of the active v4 shape evaluator.
#[cfg(feature = "bench-internals")]
pub(super) fn forward_v4(x: &[f32]) -> [f32; OUT] {
    assert_eq!(x.len(), IN_V4, "v4 evaluator feature width");
    forward_with::<IN_V4>(&WEIGHTS_V4_UNION_READING, x)
}

/// One forward pass of the shared architecture at input width `IN_DIM`, over
/// an explicit weight blob (`W1,b1,W2,b2,W3,b3`).
fn forward_with<const IN_DIM: usize>(weights: &[f32], x: &[f32]) -> [f32; OUT] {
    let (w1, rest) = weights.split_at(HID * IN_DIM);
    let (b1, rest) = rest.split_at(HID);
    let (w2, rest) = rest.split_at(HID * HID);
    let (b2, rest) = rest.split_at(HID);
    let (w3, b3) = rest.split_at(OUT * HID);

    let x = SVectorView::<f32, IN_DIM>::from_slice(x).into_owned();

    let mut h1 = affine::<HID, IN_DIM>(w1, b1, &x);
    relu(&mut h1);

    let mut h2 = affine::<HID, HID>(w2, b2, &h1);
    relu(&mut h2);

    affine::<OUT, HID>(w3, b3, &h2).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Context;
    use crate::bidding::features::FEATURES_VERSION_EVAL;
    use contract_bridge::auction::RelativeVulnerability;

    fn hand(s: &str) -> Hand {
        s.parse().expect("valid test hand")
    }

    /// The hand-rolled forward pass must reproduce the trainer's candle
    /// outputs on the exported fixture — the legacy (knob-off) weights.
    /// Thread-local knob, restored to the crate default afterwards.
    #[test]
    fn matches_candle_fixture() {
        crate::bidding::set_envelope_union_reading(false);
        check_candle_fixture(include_str!("weights/evaluator_v2.fixture.json"));
        crate::bidding::set_envelope_union_reading(true);
    }

    /// The knob-on twin (the shipped default) against its own fixture.
    #[test]
    fn union_reading_matches_candle_fixture() {
        crate::bidding::set_envelope_union_reading(true);
        check_candle_fixture(include_str!("weights/evaluator_v2_dnf.fixture.json"));
    }

    /// The v3 calls-tail artifact against its own fixture — served directly,
    /// no knobs, since the weight set is explicit.
    #[test]
    fn v3_matches_candle_fixture() {
        check_fixture(
            include_str!("weights/evaluator_v3_dnf.fixture.json"),
            3,
            IN_V3,
            |x| forward_with::<IN_V3>(&WEIGHTS_V3_UNION_READING, x),
        );
    }

    /// The pass-exclusion twin of the v3 artifact against its own fixture.
    #[test]
    fn exclusion_matches_candle_fixture() {
        check_fixture(
            include_str!("weights/evaluator_v3_exclusion.fixture.json"),
            3,
            IN_V3,
            |x| forward_with::<IN_V3>(&WEIGHTS_V3_EXCLUSION, x),
        );
    }

    /// The exclusion knob's serving contract: with the reading held fixed,
    /// knob on swaps the v3 path onto the exclusion twin and knob off is
    /// byte-identical to the union-reading twin. Restores the crate default (off).
    #[test]
    fn exclusion_knob_swaps_v3_weights() {
        use contract_bridge::{Bid, Level};
        let auction = [
            Call::Bid(Bid {
                level: Level::new(1),
                strain: Strain::Spades,
            }),
            Call::Pass,
        ];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let inf = Inferences::read(&ctx);
        let h = hand("AQ32.K53.QJ4.A92");

        crate::bidding::set_envelope_union_reading(true);
        crate::bidding::set_pass_exclusion_reading(false);
        let union_reading = trick_estimates_with_auction(h, &inf, &auction);

        crate::bidding::set_pass_exclusion_reading(true);
        let exclusion = trick_estimates_with_auction(h, &inf, &auction);
        crate::bidding::set_pass_exclusion_reading(false);

        assert_ne!(
            exclusion, union_reading,
            "the twin should not shadow the envelope-union weights"
        );
        assert_eq!(
            trick_estimates_with_auction(h, &inf, &auction),
            union_reading,
            "knob off must be byte-identical"
        );
    }

    /// The v4 shape-reading artifact against its own fixture.
    #[test]
    fn v4_matches_candle_fixture() {
        check_fixture(
            include_str!("weights/evaluator_v4_dnf.fixture.json"),
            u64::from(crate::bidding::features::FEATURES_VERSION_EVAL_V4),
            IN_V4,
            |x| forward_with::<IN_V4>(&WEIGHTS_V4_UNION_READING, x),
        );
    }

    /// The v4 knob's contract: off it changes nothing, on it supersedes v3 and
    /// still lands in the plausible band.  Restores the crate defaults.
    #[test]
    fn eval_shape_knob_contract() {
        use contract_bridge::{Bid, Level};
        let auction = [
            Call::Bid(Bid {
                level: Level::new(1),
                strain: Strain::Spades,
            }),
            Call::Pass,
        ];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let inf = Inferences::read(&ctx);
        let h = hand("AQ32.K53.QJ4.A92");

        crate::bidding::set_envelope_union_reading(true);
        set_eval_auction(true);
        let v3 = trick_estimates_with_auction(h, &inf, &auction);
        assert!(!eval_shape(), "the v4 knob ships off");

        set_eval_shape(true);
        let v4 = trick_estimates_with_auction(h, &inf, &auction);
        set_eval_shape(false);
        assert_eq!(
            trick_estimates_with_auction(h, &inf, &auction),
            v3,
            "knob off must be byte-identical"
        );

        assert_ne!(v4, v3, "the v4 artifact should not shadow v3 exactly");
        for strain in Strain::ASC {
            for who in [
                Relative::Me,
                Relative::Lho,
                Relative::Partner,
                Relative::Rho,
            ] {
                let g = v4.get(strain, who);
                assert!(
                    (0.0..=13.0).contains(&g.mean) && g.sd > 0.0 && g.sd < 13.0,
                    "{strain:?} {who:?}: {g:?}"
                );
            }
        }
    }

    fn check_candle_fixture(fixture: &str) {
        check_fixture(fixture, u64::from(FEATURES_VERSION_EVAL), IN, forward);
    }

    fn check_fixture(
        fixture: &str,
        version: u64,
        in_dim: usize,
        fwd: impl Fn(&[f32]) -> [f32; OUT],
    ) {
        let fx: serde_json::Value = serde_json::from_str(fixture).unwrap();

        // The blob's own guard is a byte count, and a byte count cannot tell a
        // 54-wide v2 artifact from any other blob of the same size — so pin the
        // layout tag too, or a stale fixture would sail through on width alone.
        assert_eq!(
            fx["feature_version"].as_u64(),
            Some(version),
            "fixture layout tag disagrees with the crate's"
        );

        let rows = fx["features"].as_array().unwrap();
        let golds = fx["outputs"].as_array().unwrap();
        assert!(!rows.is_empty(), "fixture has no rows");

        let to_vec = |v: &serde_json::Value| -> Vec<f32> {
            v.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap() as f32)
                .collect()
        };

        let mut max_abs = 0f32;
        for (frow, grow) in rows.iter().zip(golds) {
            let x = to_vec(frow);
            let gold = to_vec(grow);
            assert_eq!(x.len(), in_dim);
            assert_eq!(gold.len(), OUT);
            for (pred, g) in fwd(&x).iter().zip(&gold) {
                max_abs = max_abs.max((pred - g).abs());
            }
        }
        assert!(max_abs < 1.0e-3, "max abs diff {max_abs} exceeds tolerance");
    }

    /// Knob off, `trick_estimates_with_auction` is exactly `trick_estimates`
    /// — the byte-identity half of the knob contract.  Knob on (the shipped
    /// default) in the envelope-union regime, the v3 artifact serves: same plausibility
    /// bounds, and the auction tail visibly moves the estimate.
    #[test]
    fn with_auction_knob_contract() {
        use contract_bridge::{Bid, Level};
        let auction = [
            Call::Bid(Bid {
                level: Level::new(1),
                strain: Strain::Spades,
            }),
            Call::Pass,
        ];
        let ctx = Context::new(RelativeVulnerability::NONE, &auction);
        let inf = Inferences::read(&ctx);
        let h = hand("AQ32.K53.QJ4.A92");

        set_eval_auction(false);
        let v2 = trick_estimates(h, &inf);
        assert_eq!(trick_estimates_with_auction(h, &inf, &auction), v2);

        crate::bidding::set_envelope_union_reading(true);
        set_eval_auction(true);
        let v3 = trick_estimates_with_auction(h, &inf, &auction);

        assert_ne!(v3, v2, "v3 artifact should not shadow v2 exactly");
        for strain in Strain::ASC {
            for who in [
                Relative::Me,
                Relative::Lho,
                Relative::Partner,
                Relative::Rho,
            ] {
                let g = v3.get(strain, who);
                assert!((-1.0..=14.0).contains(&g.mean), "{strain:?} {who:?} {g:?}");
                assert!((0.1..=5.0).contains(&g.sd), "{strain:?} {who:?} {g:?}");
            }
        }
    }

    #[test]
    fn estimates_are_positive_and_plausible() {
        let ctx = Context::new(RelativeVulnerability::NONE, &[]);
        let e = trick_estimates(hand("AKQ32.K532.QJ4.9"), &Inferences::read(&ctx));
        for strain in Strain::ASC {
            for who in [
                Relative::Me,
                Relative::Lho,
                Relative::Partner,
                Relative::Rho,
            ] {
                let g = e.get(strain, who);
                assert!(g.sd > 0.0, "{strain:?} {who:?} non-positive sd: {g:?}");
                assert!(
                    (-1.0..=14.0).contains(&g.mean),
                    "{strain:?} {who:?} mean off-scale: {g:?}"
                );
                // Nobody is ever sure to within a tenth of a trick, and nobody
                // is ever clueless to within half a deal.
                assert!(
                    (0.1..=5.0).contains(&g.sd),
                    "{strain:?} {who:?} implausible sd: {g:?}"
                );
            }
        }
    }

    /// A strong hand should not read the same as a bust one — the net must be
    /// looking at the hand block, not just the (identical) ranges.
    #[test]
    fn strength_moves_the_estimate() {
        let ctx = Context::new(RelativeVulnerability::NONE, &[]);
        let inf = Inferences::read(&ctx);
        let strong = trick_estimates(hand("AKQJ.AKQ.AKQ.AKQ"), &inf);
        let weak = trick_estimates(hand("8432.7532.652.32"), &inf);
        let notrump = |e: &TrickEstimates| e.get(Strain::Notrump, Relative::Me).mean;
        assert!(
            notrump(&strong) > notrump(&weak) + 3.0,
            "strong {} vs weak {}",
            notrump(&strong),
            notrump(&weak)
        );
    }

    /// Φ against textbook values, and the CDF's contract at the mean.
    #[test]
    fn normal_cdf_is_accurate() {
        for (z, want) in [
            (-3.0, 0.001_350),
            (-1.96, 0.025_000),
            (-1.0, 0.158_655),
            (0.0, 0.500_000),
            (1.0, 0.841_345),
            (1.96, 0.975_000),
            (3.0, 0.998_650),
        ] {
            let got = standard_normal_cdf(z);
            assert!((got - want).abs() < 1e-5, "Φ({z}) = {got}, want {want}");
        }
    }

    #[test]
    fn p_at_least_reads_off_the_gaussian() {
        let g = Gaussian {
            mean: 10.0,
            sd: 1.5,
        };
        // Exactly at the half-trick correction: μ − 0.5 is a third of a σ below
        // the mean, so making ten is a shade better than even money.
        assert!((g.p_at_least(10) - 0.630_559).abs() < 1e-5);
        assert!((g.cdf(10.0) - 0.5).abs() < 1e-6);
        // Monotone, and saturating in both directions.
        assert!(g.p_at_least(7) > g.p_at_least(10));
        assert!(g.p_at_least(10) > g.p_at_least(13));
        assert!(g.p_at_least(0) > 0.999);
    }
}
