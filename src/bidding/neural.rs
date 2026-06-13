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
use std::sync::LazyLock;

/// Architecture of `two_over_one_v1`, pinned to the artifact (bump together).
const IN: usize = 160;
const HID: usize = 256;
const OUT: usize = 38;

const N_W1: usize = HID * IN;
const N_W2: usize = HID * HID;
const N_W3: usize = OUT * HID;
/// Total `f32` count across `W1,b1,W2,b2,W3,b3`.
const TOTAL: usize = N_W1 + HID + N_W2 + HID + N_W3 + OUT;

/// Embedded weights: little-endian `f32`, layer order `l1.w,l1.b,l2.w,l2.b,l3.w,l3.b`.
static RAW: &[u8] = include_bytes!("weights/two_over_one_v1.f32");
const _: () = assert!(RAW.len() == TOTAL * 4, "weights artifact size mismatch");

/// Weights decoded to `f32` once, on first use.
static WEIGHTS: LazyLock<Vec<f32>> = LazyLock::new(|| {
    RAW.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
});

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

/// Evaluate the distilled floor: 160 features → 38 logits, in `Call`-index
/// (`encode_call`) order. Deterministic — fixed weights, no RNG.
///
/// This is the raw net output; legality masking and the forced-situation
/// overrides are the job of the safety shell (M1.3), not of this function.
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned `FEATURES_LEN` (160).
#[must_use]
pub fn classify(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN, "expected {IN} features");
    let w = WEIGHTS.as_slice();
    let (w1, rest) = w.split_at(N_W1);
    let (b1, rest) = rest.split_at(HID);
    let (w2, rest) = rest.split_at(N_W2);
    let (b2, rest) = rest.split_at(HID);
    let (w3, b3) = rest.split_at(N_W3);

    let mut h1 = [0f32; HID];
    affine(w1, b1, features, &mut h1);
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
    #[test]
    fn matches_candle_fixture() {
        let fx: serde_json::Value =
            serde_json::from_str(include_str!("weights/two_over_one_v1.fixture.json")).unwrap();
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
            let pred: Vec<f32> = classify(&x).iter().map(|(_, l)| *l).collect();
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
}
