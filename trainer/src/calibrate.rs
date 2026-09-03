//! Post-hoc temperature calibration of the policy head — session 2 of
//! [`docs/ai-bidder/logit-calibration.md`](../../docs/ai-bidder/logit-calibration.md).
//!
//! One scalar `T`, fitted on the held-out split by the same soft-target
//! cross-entropy the trainer optimizes, but read over `z / T`. With
//! `beta = 1 / T` and the teacher row `y`,
//!
//! ```text
//! NLL(beta) = mean_r [ logsumexp(beta * z_r) - beta * <y_r, z_r> ]
//! ```
//!
//! `logsumexp` is the softmax's log-partition, which is convex, and the second
//! term is linear — so `NLL` is convex in `beta` and a single 1-D unimodal
//! search finds the optimum with no gradient of the net involved. Scaling every
//! logit of a row by the same `beta` cannot reorder them, so the argmax — the
//! only thing serving reads — is untouched: `T` moves the *spread*, which is
//! what the odds consumers in §4 of the doc want and what the raw logits get
//! wrong.
//!
//! ECE (expected calibration error) is the companion report: bin the held-out
//! rows by the top-1 probability, and in each bin compare how often the argmax
//! was right with how confident it claimed to be. Zero means "when it says 70%,
//! it is right 70% of the time". It scores only the top label, so the fitted
//! `T` is chosen on NLL — which scores every entry, and is what the sampler's
//! weights read — and ECE is reported beside it.

#[cfg(test)]
mod tests;

/// Equal-width bins over `[0, 1]` for the ECE histogram.
const ECE_BINS: usize = 15;
/// Search bracket for `T`. Wide enough that landing on an endpoint means the
/// fit failed rather than the corpus wanting a 20x rescale.
const T_LO: f32 = 0.05;
const T_HI: f32 = 20.0;
/// `1 / golden ratio`, the golden-section shrink factor.
const INV_PHI: f32 = 0.618_034;

/// What one fit found. `nll_before` / `ece_before` are at `T = 1` (the raw
/// logits, what serving reads); the `after` pair is at [`Self::temperature`].
pub struct Calibration {
    pub temperature: f32,
    pub nll_before: f32,
    pub nll_after: f32,
    pub ece_before: f32,
    pub ece_after: f32,
    /// Held-out rows the fit saw.
    pub rows: usize,
}

/// First (not last) argmax, matching production's `select_with_legal_state`.
fn argmax(row: &[f32]) -> usize {
    let mut best = 0;
    for (i, v) in row.iter().enumerate() {
        if *v > row[best] {
            best = i;
        }
    }
    best
}

/// Fit `T` on `rows x classes` held-out logits against the teacher rows.
///
/// `labels` are the teacher's distribution (one-hot for a BBA dump), row-major
/// and the same shape as `logits`.
pub fn fit(logits: &[f32], labels: &[f32], classes: usize) -> Calibration {
    assert_eq!(logits.len(), labels.len(), "logits/labels shape mismatch");
    assert!(
        classes > 0 && logits.len().is_multiple_of(classes),
        "ragged rows"
    );
    let rows = logits.len() / classes;
    assert!(rows > 0, "no held-out rows to calibrate on");

    // `<y, z>` is the only part of the loss that does not move with `beta`.
    let dots: Vec<f32> = logits
        .chunks_exact(classes)
        .zip(labels.chunks_exact(classes))
        .map(|(z, y)| z.iter().zip(y).map(|(a, b)| a * b).sum())
        .collect();
    let nll = |t: f32| {
        let beta = 1.0 / t;
        let sum: f64 = logits
            .chunks_exact(classes)
            .zip(&dots)
            .map(|(z, dot)| {
                let m = beta * z[argmax(z)];
                let lse = m + z.iter().map(|v| (beta * v - m).exp()).sum::<f32>().ln();
                f64::from(lse - beta * dot)
            })
            .sum();
        (sum / rows as f64) as f32
    };

    // Golden section over `ln T`: convex in `beta`, hence unimodal in `ln T`,
    // and the log scale makes the bracket symmetric about "no change".
    let (mut a, mut b) = (T_LO.ln(), T_HI.ln());
    let (mut c, mut d) = (b - (b - a) * INV_PHI, a + (b - a) * INV_PHI);
    let (mut fc, mut fd) = (nll(c.exp()), nll(d.exp()));
    for _ in 0..40 {
        if fc < fd {
            (b, d, fd) = (d, c, fc);
            c = b - (b - a) * INV_PHI;
            fc = nll(c.exp());
        } else {
            (a, c, fc) = (c, d, fd);
            d = a + (b - a) * INV_PHI;
            fd = nll(d.exp());
        }
    }
    let temperature = (0.5 * (a + b)).exp();
    Calibration {
        temperature,
        nll_before: nll(1.0),
        nll_after: nll(temperature),
        ece_before: ece(logits, labels, classes, 1.0),
        ece_after: ece(logits, labels, classes, temperature),
        rows,
    }
}

/// Expected calibration error over [`ECE_BINS`] equal-width confidence bins.
///
/// "Right" is the top-1 agreement `evaluate` reports: the net's argmax equals
/// the teacher's. The bin is chosen by the top-1 probability at `t`, which is
/// the only thing `t` moves — the argmax itself is scale-invariant.
fn ece(logits: &[f32], labels: &[f32], classes: usize, t: f32) -> f32 {
    let (mut n, mut conf, mut acc) = ([0usize; ECE_BINS], [0f64; ECE_BINS], [0f64; ECE_BINS]);
    let beta = 1.0 / t;
    for (z, y) in logits
        .chunks_exact(classes)
        .zip(labels.chunks_exact(classes))
    {
        let top = argmax(z);
        let m = z[top];
        let p = 1.0 / z.iter().map(|v| (beta * (v - m)).exp()).sum::<f32>();
        let bin = ((p * ECE_BINS as f32) as usize).min(ECE_BINS - 1);
        n[bin] += 1;
        conf[bin] += f64::from(p);
        acc[bin] += f64::from(u8::from(top == argmax(y)));
    }
    let rows = logits.len() / classes;
    let gap: f64 = (0..ECE_BINS)
        .filter(|&b| n[b] > 0)
        .map(|b| (acc[b] - conf[b]).abs() / rows as f64)
        .sum();
    gap as f32
}
