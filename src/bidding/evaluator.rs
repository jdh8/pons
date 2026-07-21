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

use super::features::{FEATURES_LEN_EVAL, features_eval};
use super::inference::{Inferences, Relative};
use super::neural::{affine, decode, relu};
use contract_bridge::{Hand, Strain};
use nalgebra::SVectorView;
use std::sync::LazyLock;

/// Input width, pinned to the artifact.
const IN: usize = FEATURES_LEN_EVAL;
/// Hidden width of both hidden layers.
const HID: usize = 64;
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

static RAW: &[u8] = include_bytes!("weights/evaluator_v1.f32");
const _: () = assert!(
    RAW.len() == TOTAL * 4,
    "evaluator weights artifact size mismatch"
);

/// Weights decoded to `f32` once, on first use.
static WEIGHTS: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW));

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

    let z = forward(&x);

    // Head-major: all 20 means, then all 20 log deviations — and in units of
    // tricks / 13, the scale `gib::relativized_tricks` labels in.
    let mut out = [[Gaussian { mean: 0.0, sd: 0.0 }; 4]; STRAIN_ROWS];
    for (i, slot) in out.iter_mut().flatten().enumerate() {
        *slot = Gaussian {
            mean: 13.0 * z[i],
            sd: 13.0 * z[TARGETS + i].clamp(LN_SD_MIN, LN_SD_MAX).exp(),
        };
    }
    TrickEstimates(out)
}

/// The raw `OUT` outputs, before reshaping and rescaling.
fn forward(x: &[f32]) -> [f32; OUT] {
    let weights = WEIGHTS.as_slice();
    let (w1, rest) = weights.split_at(HID * IN);
    let (b1, rest) = rest.split_at(HID);
    let (w2, rest) = rest.split_at(HID * HID);
    let (b2, rest) = rest.split_at(HID);
    let (w3, b3) = rest.split_at(OUT * HID);

    let x = SVectorView::<f32, IN>::from_slice(x).into_owned();

    let mut h1 = affine::<HID, IN>(w1, b1, &x);
    relu(&mut h1);

    let mut h2 = affine::<HID, HID>(w2, b2, &h1);
    relu(&mut h2);

    affine::<OUT, HID>(w3, b3, &h2).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bidding::Context;
    use contract_bridge::auction::RelativeVulnerability;

    fn hand(s: &str) -> Hand {
        s.parse().expect("valid test hand")
    }

    /// The hand-rolled forward pass must reproduce the trainer's candle outputs
    /// on the exported fixture.
    #[test]
    fn matches_candle_fixture() {
        let fx: serde_json::Value =
            serde_json::from_str(include_str!("weights/evaluator_v1.fixture.json")).unwrap();
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
            assert_eq!(x.len(), IN);
            assert_eq!(gold.len(), OUT);
            for (pred, g) in forward(&x).iter().zip(&gold) {
                max_abs = max_abs.max((pred - g).abs());
            }
        }
        assert!(max_abs < 1.0e-3, "max abs diff {max_abs} exceeds tolerance");
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
