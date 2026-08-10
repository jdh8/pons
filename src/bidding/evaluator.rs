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
//! [`bilans_floor`][crate::bidding::instinct::InstinctProfile::bilans_floor]
//! (bilans session D,
//! default off pending its A/B); the module itself is ungated and always
//! builds.

use super::context::DecisionProfile;
use super::features::{
    FEATURES_LEN_EVAL, FEATURES_LEN_EVAL_V3, FEATURES_LEN_EVAL_V4, features_eval, features_eval_on,
    features_eval_v3_on, features_eval_v4_on,
};
use super::inference::{Inferences, ReadingProfile, Relative};
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
/// recipe, corpus regenerated with
/// [`envelope_union`][field@crate::bidding::ReadingProfile::envelope_union] enabled so the range
/// blocks come from the tightened prefixed readings that regime serves.
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
/// corpus regenerated with [`pass_exclusion`][field@crate::bidding::ReadingProfile::pass_exclusion]
/// enabled on top of the
/// envelope-union regime (val NLL −1.55010 vs the union-reading twin's −1.54872 on its own
/// regime).  The explicit-profile serving path selects it only when that field
/// is enabled.
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
/// On, [`trick_estimates_with_auction`] feeds [`features_eval_v3`][super::features::features_eval_v3] — the hull
/// vector plus the last four call identities — to the v3 artifact, which the
/// 2026-07-27 NLL ablation priced at 0.038 over the hull-only vector (bare
/// calls; docs/ai-bidder/evaluator-net.md §auction-input ablation).  The A/B
/// shipped it default-on with a `win | win` verdict: plain DD +0.0180 ± 0.0042
/// (none) / +0.0284 ± 0.0056 (both), PD +0.0222 / +0.0360, on 204,800
/// boards/arm/vul at `SEED_BASE` 1785138816 — fired 1.3–1.6%, +1.3 to +2.3
/// IMPs per fired board at the bilans game/slam gates.  The v3 twin was
/// trained on the [`envelope_union`][field@crate::bidding::inference::ReadingProfile::envelope_union] regime
/// only, so the setting is only honoured
/// there; anywhere else the v2 path serves as before.
///
/// Per-thread, like every reading knob, and captured into a [`Stance`] when
/// it is built: set it *before* the build, not inside a worker closure.
///
/// [`Stance`]: crate::bidding::Stance
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
/// On, [`trick_estimates_with_auction`] feeds [`features_eval_v4`][super::features::features_eval_v4] to the v4
/// artifact: v3's vector with each hidden seat's four length `{min, max}` pairs
/// replaced by its **shape distribution** — `E[len]` and `sd[len]` per suit over
/// the 560-shape lattice, plus one column for how much the reading pins the seat
/// down.  Three columns wider than v3, and worth nothing in NLL: the round-two
/// ablation scored the encoding at +0.00004 against a matched control on 8.15M
/// rows, inside a 0.0006 seed spread.
///
/// The prize is **invariance**, not accuracy.  A hull is not a well-defined
/// function of a reading — `♥5..13` and `♥5..8` are the same claim yet differ by
/// a third of the column's range — so
/// [`sum_closure`][field@crate::bidding::ReadingProfile::sum_closure], which provably rejects
/// no hand, still displaces the endpoint columns at 81% of nodes by up to 4.19σ
/// and has to buy a retrain before it can be judged on merit.  The shape columns
/// move at 0.11% of nodes by up to 0.07σ, and that 0.11% is where the reading
/// genuinely changed.  Under this knob the reading-fidelity chops become
/// measurable on their own terms.
///
/// Supersedes [`set_eval_auction`] when both are on — v4 carries the calls tail
/// verbatim.  Like the v3 twin it was trained on the
/// [`envelope_union`][field@crate::bidding::inference::ReadingProfile::envelope_union] regime
/// only, and its shape block reads the *union* of announced boxes, so it is
/// honoured only there.
///
/// Per-thread, like every reading knob, and captured into a [`Stance`] when
/// it is built: set it *before* the build, not inside a worker closure.
///
/// [`Stance`]: crate::bidding::Stance
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
    /// Exact floating-point representation for cache-parity tests
    #[cfg(test)]
    pub(crate) fn bit_pattern(&self) -> [[[u32; 2]; 4]; STRAIN_ROWS] {
        self.0
            .map(|row| row.map(|estimate| [estimate.mean.to_bits(), estimate.sd.to_bits()]))
    }

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
/// Deterministic — fixed weights, no RNG, no solver.  This public convenience
/// serves the shipped [`ReadingProfile::default`] envelope-union regime;
/// classify-time callers use an explicit profile through the private
/// `trick_estimates_on` twin.
#[must_use]
pub fn trick_estimates(hand: Hand, inferences: &Inferences) -> TrickEstimates {
    let x = features_eval(hand, inferences);
    debug_assert_eq!(x.len(), IN);
    reshape(forward(&x))
}

/// [`trick_estimates`] on an explicit decision profile — the classify-time
/// entry point, so a stance scores a hand the same way on any thread.
#[must_use]
fn trick_estimates_on(
    profile: &DecisionProfile,
    hand: Hand,
    inferences: &Inferences,
) -> TrickEstimates {
    let x = features_eval_on(profile.blind_inference, hand, inferences);
    debug_assert_eq!(x.len(), IN);
    reshape(forward_on(profile.reading.envelope_union(), &x))
}

/// [`trick_estimates`], with the raw auction available for the v3 calls-tail
/// artifact.
///
/// Under [`set_eval_auction`] **and** the
/// [`envelope_union`][field@crate::bidding::inference::ReadingProfile::envelope_union] regime the v3 twin
/// was trained on, this serves [`features_eval_v3`][super::features::features_eval_v3] — the same vector plus the
/// last four call identities — from the weight set matching the explicit
/// profile's [`pass_exclusion`][field@crate::bidding::ReadingProfile::pass_exclusion] regime.  Under [`set_eval_shape`] it
/// serves [`features_eval_v4`][super::features::features_eval_v4] instead, which carries that tail and reads each
/// hidden seat as a shape distribution rather than a bounding box.  Anywhere
/// else it is exactly [`trick_estimates`], byte for byte, so call sites can
/// migrate to this signature unconditionally.
#[must_use]
pub fn trick_estimates_with_auction(
    hand: Hand,
    inferences: &Inferences,
    calls: &[Call],
) -> TrickEstimates {
    trick_estimates_with_auction_on(&DecisionProfile::current(), hand, inferences, calls)
}

/// [`trick_estimates_with_auction`] on an explicit decision profile — what
/// [`Context::trick_estimates`][super::Context::trick_estimates] serves, so the
/// twin and weight set a stance selects are the ones pinned into it at build.
#[must_use]
pub(crate) fn trick_estimates_with_auction_on(
    profile: &DecisionProfile,
    hand: Hand,
    inferences: &Inferences,
    calls: &[Call],
) -> TrickEstimates {
    // Both twins were fit on the tightened prefixed readings a knob-on bidder
    // serves, and v4's shape block conditions on the box union itself.
    if !profile.reading.envelope_union() {
        return trick_estimates_on(profile, hand, inferences);
    }
    if profile.eval_shape {
        let x = features_eval_v4_on(profile.blind_inference, hand, inferences, calls);
        debug_assert_eq!(x.len(), IN_V4);
        return reshape(forward_with::<IN_V4>(&WEIGHTS_V4_UNION_READING, &x));
    }
    if !profile.eval_auction {
        return trick_estimates_on(profile, hand, inferences);
    }
    let x = features_eval_v3_on(profile.blind_inference, hand, inferences, calls);
    debug_assert_eq!(x.len(), IN_V3);
    // The exclusion twin was fit on readings carrying the pass-exclusion caps;
    // serving it only under its knob keeps knob-off byte-identical.
    let weights = if profile.reading.pass_exclusion_reading() {
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
/// fit on the shipped [`ReadingProfile::default`] envelope-union regime.
/// Classify-time callers select the profile's regime through [`forward_on`].
pub(super) fn forward(x: &[f32]) -> [f32; OUT] {
    forward_on(ReadingProfile::default().envelope_union, x)
}

/// [`forward`] on an explicit reading regime (see [`trick_estimates_on`])
fn forward_on(union_reading: bool, x: &[f32]) -> [f32; OUT] {
    let weights = if union_reading {
        WEIGHTS_UNION_READING.as_slice()
    } else {
        WEIGHTS.as_slice()
    };
    forward_with::<IN>(weights, x)
}

/// Benchmark-only forward half of the v3 calls-tail evaluator, serving the
/// shipped [`ReadingProfile::default`] pass-exclusion regime.
#[cfg(feature = "bench-internals")]
pub(super) fn forward_v3(x: &[f32]) -> [f32; OUT] {
    assert_eq!(x.len(), IN_V3, "v3 evaluator feature width");
    let weights = if ReadingProfile::default().pass_exclusion {
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
mod tests;
