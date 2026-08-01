//! In-crate forward pass for the distilled neural floor — AI-bidder M1.2.
//!
//! An `f32` matmul + ReLU evaluation of the MLP that `trainer/` fits off-crate.
//! There is no ML runtime: the weights are embedded with [`include_bytes!`] and
//! the arithmetic is three `nalgebra` gemvs. The BBA-distilled net
//! (`classify_bba`) backs the default [`american`][crate::american()] floor, and
//! runs on roughly half of all bidding decisions — the hand-rolled scalar loops
//! this replaced were ~60% of the crate's bidding time on their own.
//!
//! The forward pass mirrors `candle_nn::Linear` (weights are `(out, in)`
//! row-major, `y = x·Wᵀ + b`). The parity test below asserts it reproduces the
//! trainer's candle logits on an exported fixture within a tight tolerance, and
//! that the arg-max (the chosen call) matches exactly.

use super::array::Logits;
use super::features::FEATURES_LEN_V3;
use super::instinct::kickback_now;
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

// ── BBA-distilled floor: the disclosable v3 features, EPBot 2/1 teacher ───────
// The net sees only the *disclosable* hand summary — no card-specific values
// (see `features::features_v3`) — and its teacher is the vendored EPBot 2/1
// oracle, a behavioral clone of BBA's chosen call. A stronger prior for the
// floor to stand on than the deterministic ladder it replaces.

/// Input width of `american_bba`, pinned to the artifact (= [`FEATURES_LEN_V3`]).
const IN_V3: usize = FEATURES_LEN_V3;

/// Embedded BBA-distilled weights: v3 layout (88 disclosable inputs), EPBot 2/1 teacher.
static RAW_BBA: &[u8] = include_bytes!("weights/american_bba.f32");
const _: () = assert!(
    RAW_BBA.len() == total(IN_V3) * 4,
    "BBA weights artifact size mismatch"
);

/// BBA weights decoded to `f32` once, on first use.
static WEIGHTS_BBA: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_BBA));

/// The kickback twin of the BBA artifact — same architecture, same recipe, same
/// `data_seed`/`init_seed`, corpus regenerated from a teacher that *plays*
/// kickback (`dump-teacher --conv "Kickback 1430=1" --kickback`).  Selected per
/// call by [`kickback_now`]; knob-off never touches it.
///
/// It exists because kickback is not a rule the reader can hold on its own.  A
/// ladder decides what a bid *means*; the net decides what gets *bid*, and a net
/// distilled from a kickback-blind teacher keeps jumping to a natural 4♥ in the
/// very auction where the ladder has claimed 4♥ as the diamond ask — the
/// answerer reads a question, the natural continuation is face-gated off, and
/// the auction is passed out in a 4-1 fit.  Retuning the ladder cannot reach
/// that; only the net that chooses the call can
/// (`docs/ai-bidder/bba-kickback.md` §7.7).
///
/// Both halves must move together, which is why one knob drives both: a
/// convention this widespread redefines the system, so the net that serves it is
/// part of the convention, not a variable beside it.
static RAW_BBA_KICKBACK: &[u8] = include_bytes!("weights/american_bba_kickback.f32");
const _: () = assert!(
    RAW_BBA_KICKBACK.len() == total(IN_V3) * 4,
    "kickback BBA weights artifact size mismatch"
);

/// [`RAW_BBA_KICKBACK`] decoded to `f32` once, on first use.
static WEIGHTS_BBA_KICKBACK: LazyLock<Vec<f32>> = LazyLock::new(|| decode(RAW_BBA_KICKBACK));

/// Evaluate the BBA-distilled floor: 88 disclosable features → 38 logits, in
/// `Call`-index order. Distilled from the vendored EPBot 2/1 oracle (a hard
/// clone of BBA's argmax call). Deterministic — fixed weights, no RNG.
///
/// This is the raw net output; legality masking and the forced-situation
/// overrides are the job of the safety shell (M1.3), not of this function.
///
/// # Panics
///
/// Panics if `features.len()` is not the pinned v3 [`FEATURES_LEN_V3`] (88).
#[must_use]
pub fn classify_bba(features: &[f32]) -> Logits {
    assert_eq!(features.len(), IN_V3, "expected {IN_V3} features");
    let weights = if kickback_now() {
        WEIGHTS_BBA_KICKBACK.as_slice()
    } else {
        WEIGHTS_BBA.as_slice()
    };
    forward::<IN_V3>(weights, features)
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
    fn matches_candle_fixture_bba() {
        check_fixture(include_str!("weights/american_bba.fixture.json"), |x| {
            classify_bba(x).iter().map(|(_, l)| *l).collect()
        });
    }

    /// The kickback twin clears the same parity bar — and because the twin is
    /// reachable only through the knob, this pins the *selection* too: knob-off
    /// serves the plain net, whose logits miss this fixture.  The knob is a
    /// thread-local and the harness gives each test its own thread, so setting
    /// it here cannot leak into a sibling.
    #[test]
    fn matches_candle_fixture_bba_kickback() {
        crate::bidding::instinct::set_kickback(true);
        check_fixture(
            include_str!("weights/american_bba_kickback.fixture.json"),
            |x| classify_bba(x).iter().map(|(_, l)| *l).collect(),
        );
        crate::bidding::instinct::set_kickback(false);
    }
}
