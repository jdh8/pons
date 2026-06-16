//! In-crate forward pass for the distilled neural floor — AI-bidder M1.2.
//!
//! A hand-rolled `f32` matmul + ReLU evaluation of the MLP that `trainer/` fits
//! off-crate. There is no ML dependency: the weights are embedded with
//! [`include_bytes!`] and the arithmetic is a few loops, so the default build
//! stays exactly as lean as before — this module compiles only under the
//! `neural-floor` feature.
//!
//! The forward pass mirrors `candle_nn::Linear` (weights are `(out, in)`
//! row-major, `y = x·Wᵀ + b`). The parity test below asserts it reproduces the
//! trainer's candle logits on an exported fixture within a tight tolerance, and
//! that the arg-max (the chosen call) matches exactly.

use super::array::Logits;
use super::features::{FEATURES_LEN, FEATURES_LEN_V2};
use std::sync::LazyLock;

/// Shape shared by every distilled floor: hidden width and output (call) width.
/// Only the input width changes between feature versions.
const HID: usize = 256;
const OUT: usize = 38;
const N_W2: usize = HID * HID;
const N_W3: usize = OUT * HID;

/// Float count of an MLP with `in_dim` inputs (`W1,b1,W2,b2,W3,b3`).
const fn total(in_dim: usize) -> usize {
    HID * in_dim + HID + N_W2 + HID + N_W3 + OUT
}

/// Decode a little-endian `f32` weights blob.
fn decode(raw: &[u8]) -> Vec<f32> {
    raw.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

// ── version 1: the 160-input distilled floor ────────────────────────────────

/// Input width of `two_over_one_v1`, pinned to the artifact (= [`FEATURES_LEN`]).
const IN_V1: usize = FEATURES_LEN;

/// Embedded v1 weights: little-endian `f32`, layer order `l1.w,l1.b,…,l3.b`.
static RAW_V1: &[u8] = include_bytes!("weights/two_over_one_v1.f32");
const _: () = assert!(
    RAW_V1.len() == total(IN_V1) * 4,
    "v1 weights artifact size mismatch"
);

/// v1 weights decoded to `f32` once, on first use.
static WEIGHTS_V1: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_V1));

/// `out[o] = bias[o] + Σ_i weight[o·in + i] · x[i]`, with `weight` `(out, in)`
/// row-major — i.e. `candle_nn::Linear`'s `x·Wᵀ + b`.
fn affine(weight: &[f32], bias: &[f32], x: &[f32], out: &mut [f32]) {
    let n = x.len();
    for (o, slot) in out.iter_mut().enumerate() {
        let row = &weight[o * n..o * n + n];
        *slot = bias[o] + row.iter().zip(x).map(|(w, xi)| w * xi).sum::<f32>();
    }
}

fn relu(v: &mut [f32]) {
    for x in v {
        *x = x.max(0.0);
    }
}

/// Run the MLP: `in_dim` features → 38 logits in `Call`-index (`encode_call`)
/// order. `weights` is the layer-ordered blob for an `in_dim`-input net.
fn forward(weights: &[f32], x: &[f32], in_dim: usize) -> Logits {
    let (w1, rest) = weights.split_at(HID * in_dim);
    let (b1, rest) = rest.split_at(HID);
    let (w2, rest) = rest.split_at(N_W2);
    let (b2, rest) = rest.split_at(HID);
    let (w3, b3) = rest.split_at(N_W3);

    let mut h1 = [0f32; HID];
    affine(w1, b1, x, &mut h1);
    relu(&mut h1);

    let mut h2 = [0f32; HID];
    affine(w2, b2, &h1, &mut h2);
    relu(&mut h2);

    let mut z = [0f32; OUT];
    affine(w3, b3, &h2, &mut z);

    // The net's output dim `i` is the logit for `decode_call(i)`, and
    // `iter_mut()` visits slots in that same index order — so a positional zip
    // places each logit on its call.
    let mut logits = Logits::new();
    for ((_call, slot), &value) in logits.iter_mut().zip(&z) {
        *slot = value;
    }
    logits
}

/// Evaluate the v1 distilled floor: 160 features → 38 logits, in `Call`-index
/// (`encode_call`) order. Deterministic — fixed weights, no RNG.
///
/// This is the raw net output; legality masking and the forced-situation
/// overrides are the job of the safety shell (M1.3), not of this function.
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned v1 [`FEATURES_LEN`] (160).
#[must_use]
pub fn classify(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V1, "expected {IN_V1} features");
    forward(WEIGHTS_V1.as_slice(), features, IN_V1)
}

// ── version 2: the tag-augmented distilled floor (AI-bidder M5.1) ────────────

/// Input width of `two_over_one_v2` (= [`FEATURES_LEN_V2`]).
const IN_V2: usize = FEATURES_LEN_V2;

/// Embedded v2 weights: same layer order as v1, wider first layer.
static RAW_V2: &[u8] = include_bytes!("weights/two_over_one_v2.f32");
const _: () = assert!(
    RAW_V2.len() == total(IN_V2) * 4,
    "v2 weights artifact size mismatch"
);

/// v2 weights decoded to `f32` once, on first use.
static WEIGHTS_V2: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_V2));

/// Evaluate the v2 distilled floor: tag-augmented features → 38 logits, in
/// `Call`-index order. Deterministic — fixed weights, no RNG.
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned v2 [`FEATURES_LEN_V2`].
#[must_use]
pub fn classify_v2(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V2, "expected {IN_V2} features");
    forward(WEIGHTS_V2.as_slice(), features, IN_V2)
}

// ── search-target: v1-featured, distilled from the live-search teacher ───────
// AI-bidder M3.2. Same 160-input shape and forward pass as v1; only the training
// *target* differs (the M3.1 search softmax instead of the deterministic teacher
// softmax). Not the live search bidder — a fast net that learned its judgement.

/// Embedded search-target weights: v1 layout (160 inputs), search-distilled.
static RAW_SEARCH_V1: &[u8] = include_bytes!("weights/two_over_one_v1_search.f32");
const _: () = assert!(
    RAW_SEARCH_V1.len() == total(IN_V1) * 4,
    "search-target weights artifact size mismatch"
);

/// Search-target weights decoded to `f32` once, on first use.
static WEIGHTS_SEARCH_V1: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_SEARCH_V1));

/// Evaluate the search-target distilled floor: 160 features → 38 logits, in
/// `Call`-index order. Same shape and forward pass as [`classify`]; only the
/// trained weights differ — distilled from the M2.3 live-search teacher's
/// EV-grounded targets (AI-bidder M3.2). Deterministic — fixed weights, no RNG.
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned v1 [`FEATURES_LEN`] (160).
#[must_use]
pub fn classify_search(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V1, "expected {IN_V1} features");
    forward(WEIGHTS_SEARCH_V1.as_slice(), features, IN_V1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argmax(v: &[f32]) -> usize {
        v.iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap()
    }

    /// The hand-rolled forward pass must reproduce the trainer's candle logits
    /// on the exported fixture (the M1.2 bit-match check), with an identical
    /// arg-max on every row.
    fn check_fixture(fixture: &str, classify: impl Fn(&[f32]) -> Vec<f32>) {
        let fx: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let rows = fx["features"].as_array().unwrap();
        let golds = fx["logits"].as_array().unwrap();
        assert_eq!(rows.len(), golds.len());
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
            let pred = classify(&x);
            assert_eq!(pred.len(), gold.len());
            for (p, g) in pred.iter().zip(&gold) {
                max_abs = max_abs.max((p - g).abs());
            }
            assert_eq!(
                argmax(&pred),
                argmax(&gold),
                "arg-max (chosen call) differs"
            );
        }
        assert!(
            max_abs < 1.0e-3,
            "max abs logit diff {max_abs} exceeds tolerance"
        );
    }

    #[test]
    fn matches_candle_fixture() {
        check_fixture(include_str!("weights/two_over_one_v1.fixture.json"), |x| {
            classify(x).iter().map(|(_, l)| *l).collect()
        });
    }

    #[test]
    fn matches_candle_fixture_v2() {
        check_fixture(include_str!("weights/two_over_one_v2.fixture.json"), |x| {
            classify_v2(x).iter().map(|(_, l)| *l).collect()
        });
    }

    #[test]
    fn matches_candle_fixture_search() {
        check_fixture(
            include_str!("weights/two_over_one_v1_search.fixture.json"),
            |x| classify_search(x).iter().map(|(_, l)| *l).collect(),
        );
    }
}
