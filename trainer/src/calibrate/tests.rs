use super::*;

/// Softmax of `z / t`, the exact teacher a fit at `t` should recover.
fn soft(z: &[f32], t: f32) -> Vec<f32> {
    let m = z[argmax(z)] / t;
    let e: Vec<f32> = z.iter().map(|v| (v / t - m).exp()).collect();
    let s: f32 = e.iter().sum();
    e.into_iter().map(|v| v / s).collect()
}

/// Labels drawn as `softmax(z / T0)` are minimized by exactly `T = T0`: the
/// per-row cross-entropy `H(y, p_beta)` bottoms out where `p_beta == y`, and
/// every row agrees on that beta, so their mean does too.
#[test]
fn recovers_the_temperature_that_generated_the_labels() {
    const CLASSES: usize = 7;
    let mut logits = Vec::new();
    let mut seed = 12_345u32;
    for _ in 0..200 * CLASSES {
        // xorshift: a deterministic spread, no rand dependency.
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        logits.push(f32::from(seed as u16) / 8192.0 - 4.0);
    }
    for t0 in [0.4, 1.0, 2.5] {
        let labels: Vec<f32> = logits
            .as_chunks::<CLASSES>()
            .0
            .iter()
            .flat_map(|z| soft(z, t0))
            .collect();
        let cal = fit(&logits, &labels, CLASSES);
        assert!(
            (cal.temperature - t0).abs() < 1e-2,
            "T0 {t0} -> fitted {}",
            cal.temperature
        );
        // The bracket contains 1, so the fit can never be beaten by raw logits.
        assert!(cal.nll_after <= cal.nll_before + 1e-6);
        assert_eq!(cal.rows, 200);
    }
}
