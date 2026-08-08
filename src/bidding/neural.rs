//! In-crate forward pass for the distilled neural floor — AI-bidder M1.2.
//!
//! An `f32` matmul + ReLU evaluation of the MLP that `trainer/` fits off-crate.
//! There is no ML runtime: the weights are embedded with [`include_bytes!`] and
//! the arithmetic is three `nalgebra` gemvs. The configured BBA-distilled net
//! ([`classify_bba_v4`][crate::bidding::neural::classify_bba_v4]) backs the
//! default [`american`][crate::american()]
//! floor, and runs on the contested off-book decisions — the hand-rolled scalar
//! loops this replaced were ~60% of the crate's bidding time on their own.
//!
//! The forward pass mirrors `candle_nn::Linear` (weights are `(out, in)`
//! row-major, `y = x·Wᵀ + b`). The parity test below asserts it reproduces the
//! trainer's candle logits on an exported fixture within a tight tolerance, and
//! that the arg-max (the chosen call) matches exactly.

use super::array::Logits;
use super::features::{FEATURES_LEN_V4, FEATURES_LEN_V5};
use nalgebra::{SMatrixView, SVector, SVectorView};
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
pub(super) fn decode(raw: &[u8]) -> Vec<f32> {
    raw.as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect()
}

/// `W·x + b` for `weight` in `(R, C)` row-major layout — i.e.
/// `candle_nn::Linear`'s `x·Wᵀ + b`.
///
/// Read column-major, a row-major `(R, C)` buffer *is* `Wᵀ`, so `tr_mul` applies
/// `W` without materialising a transpose or copying the blob: the view borrows
/// the decoded weights in place. Each output is then a dot product down one
/// contiguous column, which is both cache-optimal and what nalgebra vectorises.
///
/// The hand-rolled scalar loop this replaces summed with `Iterator::sum`, whose
/// loop-carried dependency LLVM may not reassociate — it ran at ~2 GMAC/s
/// against ~40 here.
pub(super) fn affine<const R: usize, const C: usize>(
    weight: &[f32],
    bias: &[f32],
    x: &SVector<f32, C>,
) -> SVector<f32, R> {
    SMatrixView::<f32, C, R>::from_slice(weight).tr_mul(x) + SVectorView::<f32, R>::from_slice(bias)
}

pub(super) fn relu<const R: usize>(v: &mut SVector<f32, R>) {
    v.apply(|x| *x = x.max(0.0));
}

/// Run the MLP: `IN` features → 38 logits in `Call`-index (`encode_call`)
/// order. `weights` is the layer-ordered blob for an `IN`-input net.
fn forward<const IN: usize>(weights: &[f32], x: &[f32]) -> Logits {
    let (w1, rest) = weights.split_at(HID * IN);
    let (b1, rest) = rest.split_at(HID);
    let (w2, rest) = rest.split_at(N_W2);
    let (b2, rest) = rest.split_at(HID);
    let (w3, b3) = rest.split_at(N_W3);

    let x = SVectorView::<f32, IN>::from_slice(x).into_owned();

    let mut h1 = affine::<HID, IN>(w1, b1, &x);
    relu(&mut h1);

    let mut h2 = affine::<HID, HID>(w2, b2, &h1);
    relu(&mut h2);

    let z = affine::<OUT, HID>(w3, b3, &h2);

    // The net's output dim `i` is the logit for `decode_call(i)`, and
    // `iter_mut()` visits slots in that same index order — so a positional zip
    // places each logit on its call.
    let mut logits = Logits::new();
    for ((_call, slot), &value) in logits.iter_mut().zip(&z) {
        *slot = value;
    }
    logits
}

// ── The configured floor: v4 features, one net for every regime ──────────────
// `docs/ai-bidder/configured-net.md`.  The convention card is an *input* here,
// so a regime needs no weights artifact of its own.  That is what retired the v3
// twin pair — one net per kickback stance, selected per decision by the knob —
// on gate 1's verdict (configured-net.md phase 6).

/// Input width of `american_bba_v4`, pinned to the artifact (= [`FEATURES_LEN_V4`]).
const IN_V4: usize = FEATURES_LEN_V4;

/// Embedded configured weights: v4 layout (88 disclosable inputs + both cards),
/// EPBot 2/1 teacher, corpus rotating through six `{American, Dutch} ×
/// {kickback off, on}` table configurations plus an enriched slam slice.
static RAW_BBA_V4: &[u8] = include_bytes!("weights/american_bba_v4.f32");
const _: () = assert!(
    RAW_BBA_V4.len() == total(IN_V4) * 4,
    "configured BBA weights artifact size mismatch"
);

/// [`RAW_BBA_V4`] decoded to `f32` once, on first use.
static WEIGHTS_BBA_V4: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_BBA_V4));

/// Evaluate the **configured** BBA-distilled floor: 368 features → 38 logits.
///
/// The shipped floor's net, and the only one left: it retired the v3 twin pair
/// (`american_bba` + `american_bba_kickback`, selected per call by the kickback
/// knob) on gate 1's verdict.  It reads no
/// ambient knob state at all: the regime arrives in the feature vector as both
/// partnerships' convention cards ([`features_v4`][super::features::features_v4]),
/// so one artifact serves every cell and an A/B arm differs by a card row
/// rather than by a separately trained net.  Deterministic — fixed weights, no
/// RNG.
///
/// This is the raw net output; legality masking and the forced-situation
/// overrides belong to the shell
/// ([`ConfiguredFloorBba`][super::neural_floor::ConfiguredFloorBba]).
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned v4 [`FEATURES_LEN_V4`] (368).
#[must_use]
pub fn classify_bba_v4(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V4, "expected {IN_V4} features");
    forward::<IN_V4>(&WEIGHTS_BBA_V4, features)
}

// ── The compact-config floor: v5 features ────────────────────────────────────
// `docs/ai-bidder/card-manifold.md` §"The retrain".  The 280-float card blocks
// gave way to two 28-slot `Agreements` vectors: every input is an axis the
// corpus can vary, so the frozen-coordinate disease that motivated the fold is
// structurally impossible here.  Disclosure (`.bbsa`) is untouched — only the
// net's input stops being the card.

/// Input width of `american_bba_v5`, pinned to the artifact (= [`FEATURES_LEN_V5`]).
const IN_V5: usize = FEATURES_LEN_V5;

/// Embedded compact-config weights: v5 layout (88 disclosable inputs + both
/// sides' [`Agreements`][super::features::Agreements]), EPBot 2/1 teacher,
/// corpus = the six `{American, Dutch} × {kickback}` cells plus an enriched
/// slam slice and eight single-axis flip shards (22.pdd rows 3.25M–4.16M).
static RAW_BBA_V5: &[u8] = include_bytes!("weights/american_bba_v5.f32");
const _: () = assert!(
    RAW_BBA_V5.len() == total(IN_V5) * 4,
    "compact-config BBA weights artifact size mismatch"
);

/// [`RAW_BBA_V5`] decoded to `f32` once, on first use.
static WEIGHTS_BBA_V5: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_BBA_V5));

/// Evaluate the **compact-config** BBA-distilled floor: 144 features → 38 logits.
///
/// The v5 sibling of [`classify_bba_v4`], gated behind
/// [`american_v5`][super::american::american_v5] until the v5-vs-v4 A/B
/// verdict.  Deterministic — fixed weights, no RNG.  Legality masking and the
/// forced rails belong to the shell
/// ([`ConfiguredFloorV5`][super::neural_floor::ConfiguredFloorV5]).
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned v5 [`FEATURES_LEN_V5`] (144).
#[must_use]
pub fn classify_bba_v5(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V5, "expected {IN_V5} features");
    forward::<IN_V5>(&WEIGHTS_BBA_V5, features)
}

#[cfg(test)]
mod tests;
