//! In-crate forward pass for the distilled neural floor — AI-bidder M1.2.
//!
//! An `f32` matmul + ReLU evaluation of the MLP that `trainer/` fits off-crate.
//! There is no ML runtime: the weights are embedded with [`include_bytes!`] and
//! the arithmetic is three `nalgebra` gemvs. The configured BBA-distilled net
//! ([`classify_bba_v6`][crate::bidding::neural::classify_bba_v6]) backs the
//! default [`american`][crate::american()]
//! floor, and runs on the contested off-book decisions — the hand-rolled scalar
//! loops this replaced were ~60% of the crate's bidding time on their own.
//!
//! The forward pass mirrors `candle_nn::Linear` (weights are `(out, in)`
//! row-major, `y = x·Wᵀ + b`). The parity test below asserts it reproduces the
//! trainer's candle logits on an exported fixture within a tight tolerance, and
//! that the arg-max (the chosen call) matches exactly.

use super::array::Logits;
use super::features::{FEATURES_LEN_V4, FEATURES_LEN_V6, TOKEN_LEN_V7};
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
// twin pair — one net per kickback partnership, selected per decision by the knob —
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

// ── The honest-reading compact floor: v6 features ────────────────────────────

/// Input width of `american_bba_v6`, pinned to the artifact.
const IN_V6: usize = FEATURES_LEN_V6;

/// Compact-config weights retrained on the live authored reading.  Whole-hand
/// points and the four fit-specific support ranges are separate inputs.
static RAW_BBA_V6: &[u8] = include_bytes!("weights/american_bba_v6.f32");
const _: () = assert!(
    RAW_BBA_V6.len() == total(IN_V6) * 4,
    "honest-reading BBA weights artifact size mismatch"
);

static WEIGHTS_BBA_V6: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_BBA_V6));

/// Evaluate the v6 BBA-distilled floor: 176 features → 38 logits.
#[must_use]
pub fn classify_bba_v6(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V6, "expected {IN_V6} features");
    forward::<IN_V6>(&WEIGHTS_BBA_V6, features)
}

/// Evaluate the v6 twin retrained on BBA's disclosed Multi-Landy readings.
static RAW_BBA_V6_THEIR: &[u8] = include_bytes!("weights/american_bba_v6_their.f32");
const _: () = assert!(
    RAW_BBA_V6_THEIR.len() == total(IN_V6) * 4,
    "BBA-reading twin weights artifact size mismatch"
);
static WEIGHTS_BBA_V6_THEIR: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_BBA_V6_THEIR));

/// Evaluate the BBA-reading v6 twin: 176 features → 38 logits.
#[must_use]
pub fn classify_bba_v6_their(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V6, "expected {IN_V6} features");
    forward::<IN_V6>(&WEIGHTS_BBA_V6_THEIR, features)
}

// ── The v7 sequence floor: an LSTM auction encoder feeding the v6 head ───────
//
// `features_v6` flattens the auction into strain bitmasks plus cumulative
// hulls, so a transfer `2♠` and a natural `2♠` reach the net as the same float.
// v7 keeps that vector as a *static* block and prepends a recurrence over one
// token per prior call, each carrying the call plus the box union its authoring
// rule projected (`features::call_tokens_v7`).  The LSTM is a pure auction
// encoder: its final hidden state is concatenated with the static block and the
// pair is run through the very same two-hidden-layer MLP as v6, so `forward`
// below is reused verbatim for the head.
//
// See `docs/ai-bidder/plan.md` M5.2.

/// Hidden width of the v7 LSTM (`h` and `c` are each this wide).
const HL: usize = 128;
/// One token's expanded width — what the recurrence reads per step.
const TOK: usize = TOKEN_LEN_V7;
/// The four gate blocks (`i`, `f`, `g̃`, `o`) share one affine map.
const G: usize = 4 * HL;
/// The head reads the final hidden state concatenated with the v6 vector.
const IN_HEAD: usize = HL + IN_V6;

/// Float count of a v7 net: the LSTM's two weight matrices and two biases, then
/// the head MLP.  Mirrors `PARAM_NAMES_LSTM`'s order in `trainer/`.
#[must_use]
pub const fn total_lstm() -> usize {
    G * TOK + G * HL + 2 * G + total(IN_HEAD)
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Evaluate a v7 sequence net: `tokens` (oldest first, from
/// [`call_tokens_v7`][super::features::call_tokens_v7]) plus the v6 static
/// block → 38 logits.
///
/// One step is candle's `LSTM::step` exactly: `g = x·W_ihᵀ + b_ih + h·W_hhᵀ +
/// b_hh`, chunked into four 128-wide blocks in candle's order — **`i`, `f`,
/// `g̃`, `o`** — then `c ← σ(f)∘c + σ(i)∘tanh(g̃)` and `h ← σ(o)∘tanh(c)`.  An
/// empty `tokens` leaves the zero state, which is exactly what the trainer's
/// `gather` at index `len` returns for a row with no prior calls.
///
/// Weights arrive as an argument rather than an embedded artifact, so a floor
/// can be pointed at any trained blob — including before one is embedded, which
/// is what lets this be tested against a candle fixture with no shipped net.
///
/// The state is rebuilt from scratch on every decision — no incremental cache.
/// At ten steps that is ~1.3M MACs, roughly 11× v6 and still tens of
/// microseconds.
///
/// # Panics
///
/// Panics if `weights` is not [`total_lstm`] floats, or `features` is not the
/// pinned v6 width.
// ponytail: scalar `exp`/`tanh` and a from-scratch state per decision; the
// upgrade path if this ever shows on a profile is to cache `(h, c)` per auction
// prefix in `Context`, which is a `docs/bidding-performance-handoff.md` change,
// not a numerics one.
#[must_use]
pub fn classify_lstm(weights: &[f32], features: &[f32], tokens: &[[f32; TOKEN_LEN_V7]]) -> Logits {
    assert_eq!(weights.len(), total_lstm(), "expected a v7 weights blob");
    assert_eq!(features.len(), IN_V6, "expected {IN_V6} features");
    let x = features;
    let (w_ih, rest) = weights.split_at(G * TOK);
    let (w_hh, rest) = rest.split_at(G * HL);
    let (b_ih, rest) = rest.split_at(G);
    let (b_hh, head) = rest.split_at(G);

    let mut h = SVector::<f32, HL>::zeros();
    let mut c = SVector::<f32, HL>::zeros();
    for token in tokens {
        let input = SVectorView::<f32, TOK>::from_slice(token).into_owned();
        // The two biases are summed, not applied once: candle adds `b_ih` to the
        // input branch and `b_hh` to the hidden branch, and the branches are
        // then added — so the gate pre-activation carries both.
        let gates = affine::<G, TOK>(w_ih, b_ih, &input) + affine::<G, HL>(w_hh, b_hh, &h);
        for i in 0..HL {
            let input_gate = sigmoid(gates[i]);
            let forget = sigmoid(gates[HL + i]);
            let cell = gates[2 * HL + i].tanh();
            let output = sigmoid(gates[3 * HL + i]);
            c[i] = forget * c[i] + input_gate * cell;
            h[i] = output * c[i].tanh();
        }
    }

    let mut joined = [0.0; IN_HEAD];
    joined[..HL].copy_from_slice(h.as_slice());
    joined[HL..].copy_from_slice(x);
    forward::<IN_HEAD>(head, &joined)
}

#[cfg(test)]
mod tests;
