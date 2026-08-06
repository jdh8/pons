//! `eval-arithmetic` — what does *auditable* arithmetic cost, in tricks?
//!
//! The trick evaluator ([`pons::bidding::evaluator`]) is a 54→256→256→40 MLP.
//! It works — 0.1060 P(make) error in the decision band — but it is a black box,
//! and DNF chop C1 showed it degrading when a *true* narrowing tightened its
//! inputs (`docs/dnf-migration.md`, 47–64 % of that loss attributed to the net).
//! This harness prices the alternative: BBA-shaped arithmetic over the same
//! inputs, fitted by least squares, served as a handful of adds.
//!
//! BBA's own engine is now read rather than inferred (`docs/ai-bidder/bba-floor.md`
//! §5.5). Its level is `(total_points + 1) / 3 − 6` floored by
//! `max(level, winning_tricks − 6)`, and its per-suit trick count consumes
//! exactly `(combined honour mask, our two lengths, their two lengths)`. The
//! rungs below walk from "combined strength alone" out toward that shape, and
//! each is a strict superset of the last, so **every rung is a leading principal
//! submatrix of one Gram matrix** — the whole ladder costs one pass to fit.
//!
//! Scored against the shipped net's held-out numbers with the trainer's exact
//! loss, `L(t; μ, s) = s + ½·(t − μ)²·exp(−2s)` in `tricks/13` units with
//! `s = ln σ` clamped to `[-5, 0]` (`trainer/src/bin/evaluator/main.rs`). The
//! reference to beat, or to fall short of by a known margin, is **NLL −1.5118,
//! MAE 1.441 tricks** (`evaluator_v2_dnf`, `docs/dnf-migration.md` row F2b).
//!
//! No solver, no sampler, no bidding: the corpus is already
//! `(own hand + hidden-seat envelope-union hulls, double-dummy tricks)`.
//!
//! ```text
//! cargo run --release --example eval-arithmetic -- \
//!     --train target/eval-train-1m-dnf.f32 --test target/eval-test-bits-dnf.f32
//! ```
//!
//! ## What this harness cannot reach
//!
//! BBA's per-suit count needs the *partnership* honour mask, and the corpus's
//! range blocks carry lengths and `points` only — `Envelope::strength.suit_hcp`
//! is never encoded (`features::push_inference`). So the R3 rung below uses our
//! own honour texture and partner's strength as a scalar, not the combined
//! per-suit mask. Reaching the real thing needs `dump-evaluator` to emit the
//! `suit_hcp` range columns; that is the follow-up, not a limit of the method.
use anyhow::{Context as _, Result, bail};
use clap::Parser;
use nalgebra::{DMatrix, DVector};
use std::fs::File;
use std::io::{BufReader, Read};

/// `--encoding bits --oracle` row width, from the corpus sidecar's `row_len`.
const ROW_LEN: usize = 107;
/// Feature floats before the 20 double-dummy labels.
const FEATURES: usize = 87;
/// `(strain, declarer)` label columns: 5 strains × 4 declarers.
const TARGETS: usize = 20;
/// Own-hand block: 4 suits × `[len/13, spots/8, suit_hcp/10, A, K, Q, J, T]`.
const SUIT_BITS: usize = 8;
/// First range block (LHO); then partner, then RHO, 15 floats each.
const RANGES: usize = 34;
/// One seat's range block: 4 suits × `(min, max, width)` then points ditto.
const SEAT_RANGE: usize = 15;

/// `ln σ` clamps, matched to the trainer so the losses are comparable.
const LN_SD_MIN: f64 = -5.0;
/// See [`LN_SD_MIN`].
const LN_SD_MAX: f64 = 0.0;

/// Widest arithmetic rung's column count, including the leading constant.
const K: usize = 13;
/// Raw corpus features excluding the `--oracle` truth columns at `[79, 87)`.
const RAW: usize = 79;
/// The wide diagnostic rung: every raw feature plus a constant.
const WIDE: usize = RAW + 1;

/// Bracket on the fitted variance, as a multiple of the rung's own mean squared
/// residual. Without a floor a linear σ² extrapolates through zero on held-out
/// rows and the loss explodes; without a ceiling it does the mirror image.
const VAR_FLOOR: f64 = 0.25;
/// See [`VAR_FLOOR`].
const VAR_CEIL: f64 = 4.0;

/// The ladder: `(label, columns used)`. Nested — each rung is a prefix.
const RUNGS: [(&str, usize); 6] = [
    ("R0 const", 1),
    ("R1 +strength", 2),
    ("R2 +fit", 3),
    ("R3 +texture", 6),
    ("R4 +defence", 8),
    ("R5 +shape/width", K),
];

#[derive(Parser)]
struct Args {
    /// Training corpus (flat little-endian f32, `--encoding bits`)
    #[arg(long, default_value = "target/eval-train-1m-dnf.f32")]
    train: String,
    /// Held-out corpus, from a different `.pdd` shard
    #[arg(long, default_value = "target/eval-test-bits-dnf.f32")]
    test: String,
    /// Cap on training rows. Five coefficients do not need 20 M of them.
    #[arg(long, default_value_t = 2_000_000)]
    train_rows: usize,
    /// Rows for the wide diagnostic rung, whose Gram is 100× the ladder's.
    #[arg(long, default_value_t = 200_000)]
    wide_rows: usize,
}

/// Stream a corpus file row by row, calling `f` with each `ROW_LEN` slice.
fn for_each_row(path: &str, cap: usize, mut f: impl FnMut(&[f32])) -> Result<usize> {
    let file = File::open(path).with_context(|| format!("opening {path}"))?;
    let mut reader = BufReader::with_capacity(1 << 22, file);
    let mut buf = vec![0u8; ROW_LEN * 4 * 4096];
    let mut row = [0f32; ROW_LEN];
    let mut seen = 0usize;
    loop {
        let mut filled = 0;
        while filled < buf.len() {
            match reader.read(&mut buf[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        if filled == 0 {
            break;
        }
        if filled % (ROW_LEN * 4) != 0 {
            bail!("{path}: {filled} bytes is not a whole number of {ROW_LEN}-float rows");
        }
        for chunk in buf[..filled].as_chunks::<{ ROW_LEN * 4 }>().0 {
            for (slot, bytes) in row.iter_mut().zip(chunk.as_chunks::<4>().0) {
                *slot = f32::from_le_bytes(*bytes);
            }
            f(&row);
            seen += 1;
            if seen >= cap {
                return Ok(seen);
            }
        }
        if filled < buf.len() {
            break;
        }
    }
    Ok(seen)
}

/// Midpoint of a seat's `points` range, in natural points (the corpus stores
/// `min/37, max/37, width/37`).
fn points_mid(row: &[f32], seat: usize) -> f64 {
    let base = RANGES + seat * SEAT_RANGE + 12;
    37.0 * f64::from(row[base] + row[base + 1]) / 2.0
}

/// Midpoint of a seat's length range in `suit`, in natural cards.
fn length_mid(row: &[f32], seat: usize, suit: usize) -> f64 {
    let base = RANGES + seat * SEAT_RANGE + suit * 3;
    13.0 * f64::from(row[base] + row[base + 1]) / 2.0
}

/// Own hand: length in `suit`, in natural cards.
fn own_length(row: &[f32], suit: usize) -> f64 {
    13.0 * f64::from(row[suit * SUIT_BITS])
}

/// Own hand: high-card points in `suit` (`4A + 3K + 2Q + J`).
fn own_suit_hcp(row: &[f32], suit: usize) -> f64 {
    10.0 * f64::from(row[suit * SUIT_BITS + 2])
}

/// Own hand: count of a given honour across the four suits. Offsets 3..8 in a
/// suit block are the A, K, Q, J, T bits.
fn own_honours(row: &[f32], offset: usize) -> f64 {
    (0..4)
        .map(|s| f64::from(row[s * SUIT_BITS + 3 + offset]))
        .sum()
}

/// The design row for one `(row, target)`, in natural units so the fitted
/// coefficients read as bridge quantities.
///
/// `target = strain * 4 + declarer`, strains in GIB order `NT ♠ ♥ ♦ ♣` and
/// declarers `[me, lho, partner, rho]`. Range blocks are `[lho, partner, rho]`,
/// so seat indices here are 0, 1, 2 in that order.
fn design(row: &[f32], target: usize) -> [f64; K] {
    let strain = target / 4;
    let declarer = target % 4;
    // Our side declares from `me` (0) and `partner` (2).
    let ours = declarer.is_multiple_of(2);
    // GIB strain order is NT ♠ ♥ ♦ ♣; `Suit::ASC` is ♣ ♦ ♥ ♠, so `4 - strain`.
    let trump = 4usize.saturating_sub(strain);

    let own_hcp = 40.0 * f64::from(row[32]);
    let (partner, lho, rho) = (1usize, 0usize, 2usize);

    // Strength of the declaring pair, and of the defending pair.
    let (pair_hcp, defence_hcp) = if ours {
        (
            own_hcp + points_mid(row, partner),
            points_mid(row, lho) + points_mid(row, rho),
        )
    } else {
        (
            points_mid(row, lho) + points_mid(row, rho),
            own_hcp + points_mid(row, partner),
        )
    };

    // Combined trump length. At notrump there is no trump, so the pair's best
    // combined fit stands in — the same quantity a notrump hand cares about.
    let combined = |suit: usize| {
        if ours {
            own_length(row, suit) + length_mid(row, partner, suit)
        } else {
            length_mid(row, lho, suit) + length_mid(row, rho, suit)
        }
    };
    let fit = if strain == 0 {
        (0..4).map(combined).fold(0.0, f64::max)
    } else {
        combined(trump)
    };

    // Our own honour texture. For a defending target this is the defence's,
    // which is equally informative — the per-target fit carries the sign.
    let trump_hcp = if strain == 0 {
        (0..4).map(|s| own_suit_hcp(row, s)).fold(0.0, f64::max)
    } else {
        own_suit_hcp(row, trump)
    };

    // Shortness in a side suit: the ruffing term, and zero at notrump where
    // ruffing does not exist.
    let shortness = if strain == 0 {
        0.0
    } else {
        (0..4)
            .filter(|&s| s != trump)
            .map(combined)
            .fold(f64::INFINITY, f64::min)
    };

    // R5. The wide diagnostic says ~0.3 tricks are still on the table in
    // features the corpus already carries, so spend a few more auditable terms
    // on them: the rest of the honour ladder, the second fit (a source of
    // discards), our spot cards, and — the term the plan asked for by name —
    // partner's `points` *width*, the envelope-uncertainty column that the net
    // folds invisibly into `ln σ`.
    let mut fits: Vec<f64> = (0..4).map(combined).collect();
    fits.sort_by(|a, b| b.total_cmp(a));
    let second_fit = fits.get(1).copied().unwrap_or(0.0);
    let width_base = RANGES + if ours { partner } else { lho } * SEAT_RANGE + 14;
    let points_width = 37.0 * f64::from(row[width_base]);
    let spots: f64 = (0..4)
        .map(|s| 8.0 * f64::from(row[s * SUIT_BITS + 1]))
        .sum();

    [
        1.0,
        pair_hcp,
        fit,
        own_honours(row, 0), // aces
        own_honours(row, 1), // kings
        trump_hcp,
        defence_hcp,
        shortness,
        own_honours(row, 2), // queens
        own_honours(row, 3), // jacks
        second_fit,
        spots,
        points_width,
    ]
}

/// Normal-equation accumulator: `A = Σ v vᵀ`, `b = Σ v·y`, for one target.
#[derive(Clone)]
struct Gram<const N: usize> {
    a: Vec<f64>,
    b: [f64; N],
    n: u64,
}

impl<const N: usize> Gram<N> {
    fn new() -> Self {
        Self {
            a: vec![0.0; N * N],
            b: [0.0; N],
            n: 0,
        }
    }

    fn push(&mut self, v: &[f64; N], y: f64) {
        for (i, &vi) in v.iter().enumerate() {
            self.b[i] += vi * y;
            let row = &mut self.a[i * N..(i + 1) * N];
            for (slot, &vj) in row.iter_mut().zip(v) {
                *slot += vi * vj;
            }
        }
        self.n += 1;
    }

    /// Mean of the accumulated `y`, which for a residual-squared Gram is the
    /// rung's mean squared error — the base of the variance budget.
    fn mean_y(&self) -> f64 {
        self.b[0] / self.n.max(1) as f64
    }

    /// Solve the leading `k × k` system. A ridge of `1e-6` keeps a rung whose
    /// extra column is constant within a target (notrump `shortness`, say) from
    /// producing a singular matrix rather than a zero coefficient.
    fn solve(&self, k: usize) -> Vec<f64> {
        let mut a = DMatrix::from_fn(k, k, |i, j| self.a[i * N + j]);
        for i in 0..k {
            a[(i, i)] += 1e-6;
        }
        let b = DVector::from_fn(k, |i, _| self.b[i]);
        a.lu()
            .solve(&b)
            .map_or_else(|| vec![0.0; k], |x| x.iter().copied().collect())
    }
}

fn dot<const N: usize>(beta: &[f64], v: &[f64; N]) -> f64 {
    beta.iter().zip(v).map(|(c, x)| c * x).sum()
}

/// Held-out scores for one rung, in the trainer's units.
#[derive(Default, Clone, Copy)]
struct Score {
    nll: f64,
    abs: f64,
    n: u64,
}

impl Score {
    fn push(&mut self, mu: f64, ln_sd: f64, t: f64) {
        let s = ln_sd.clamp(LN_SD_MIN, LN_SD_MAX);
        self.nll += s + 0.5 * (t - mu).powi(2) * (-2.0 * s).exp();
        self.abs += (t - mu).abs();
        self.n += 1;
    }

    fn nll(self) -> f64 {
        self.nll / self.n.max(1) as f64
    }

    /// Rescaled from the corpus's `tricks / 13`, matching `mae_tricks`.
    fn mae_tricks(self) -> f64 {
        13.0 * self.abs / self.n.max(1) as f64
    }
}

/// Every raw corpus feature the net can see, as one design row — the diagnostic
/// that splits "the gap is our feature set" from "the gap is linearity". The
/// oracle columns at `[79, 87)` are partner's *true* honours and are excluded:
/// they are truth, not something a bidder knows.
fn design_wide(row: &[f32]) -> [f64; WIDE] {
    let mut v = [0.0; WIDE];
    v[0] = 1.0;
    for (slot, &x) in v[1..].iter_mut().zip(&row[..RAW]) {
        *slot = f64::from(x);
    }
    v
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Pass 1 — the mean head. One Gram per target; every rung is a prefix.
    // The wide diagnostic rides along on a capped prefix of the same pass: 79
    // coefficients converge long before 2 M rows, and its Gram is 100× wider.
    let mut mean = vec![Gram::<K>::new(); TARGETS];
    let mut wide = vec![Gram::<WIDE>::new(); TARGETS];
    let mut seen = 0usize;
    let train_rows = for_each_row(&args.train, args.train_rows, |row| {
        for (t, gram) in mean.iter_mut().enumerate() {
            gram.push(&design(row, t), f64::from(row[FEATURES + t]));
        }
        if seen < args.wide_rows {
            let v = design_wide(row);
            for (t, gram) in wide.iter_mut().enumerate() {
                gram.push(&v, f64::from(row[FEATURES + t]));
            }
        }
        seen += 1;
    })?;
    eprintln!(
        "fitted the mean head on {train_rows} rows of {} ({} for the wide rung)",
        args.train,
        args.wide_rows.min(train_rows),
    );
    let wide_betas: Vec<Vec<f64>> = wide.iter().map(|g| g.solve(WIDE)).collect();

    // Solve every rung up front so pass 2 can fit each one's own residual.
    let betas: Vec<Vec<Vec<f64>>> = mean
        .iter()
        .map(|g| RUNGS.iter().map(|&(_, k)| g.solve(k)).collect())
        .collect();

    // Pass 2 — the variance head, one Gram per (target, rung), fitted to the
    // squared residual. That estimates `Var(T | features) + model bias²`, which
    // is exactly the predictive variance a `P(make)` gate should integrate; see
    // `docs/binky-points.md` on the two variance columns.
    let mut var = vec![vec![Gram::<K>::new(); RUNGS.len()]; TARGETS];
    for_each_row(&args.train, args.train_rows, |row| {
        for (t, per_rung) in var.iter_mut().enumerate() {
            let v = design(row, t);
            let y = f64::from(row[FEATURES + t]);
            for (r, gram) in per_rung.iter_mut().enumerate() {
                gram.push(&v, (y - dot(&betas[t][r], &v)).powi(2));
            }
        }
    })?;
    let gammas: Vec<Vec<Vec<f64>>> = var
        .iter()
        .map(|per_rung| {
            per_rung
                .iter()
                .zip(RUNGS)
                .map(|(g, (_, k))| g.solve(k))
                .collect()
        })
        .collect();

    // The variance budget's base term: each rung's own mean squared residual.
    // A *linear* σ² is free to extrapolate to zero on held-out rows, and the
    // loss punishes that catastrophically — a 2-trick miss priced at the σ
    // floor costs ~260 nats. So bracket the fitted variance around its base,
    // which is the cap the plan asked for and the knob the net does not have.
    let base: Vec<Vec<f64>> = var
        .iter()
        .map(|per_rung| per_rung.iter().map(Gram::mean_y).collect())
        .collect();

    // Pass 3 — score every rung on the held-out shard.
    let mut overall = vec![Score::default(); RUNGS.len()];
    let mut slam = vec![Score::default(); RUNGS.len()];
    let mut wide_score = Score::default();
    let test_rows = for_each_row(&args.test, usize::MAX, |row| {
        let vw = design_wide(row);
        for t in 0..TARGETS {
            let v = design(row, t);
            let y = f64::from(row[FEATURES + t]);
            for r in 0..RUNGS.len() {
                let mu = dot(&betas[t][r], &v);
                let b = base[t][r];
                let sd2 = dot(&gammas[t][r], &v).clamp(VAR_FLOOR * b, VAR_CEIL * b);
                let ln_sd = 0.5 * sd2.ln();
                overall[r].push(mu, ln_sd, y);
                if y * 13.0 >= 12.0 {
                    slam[r].push(mu, ln_sd, y);
                }
            }
            // μ-only: the wide rung answers "is the gap features or linearity",
            // and MAE answers that. It carries no variance head.
            wide_score.push(dot(&wide_betas[t], &vw), 0.0, y);
        }
    })?;
    eprintln!("scored on {test_rows} held-out rows of {}\n", args.test);

    println!("| rung | cols | NLL | MAE tricks | slam MAE |");
    println!("|---|---:|---:|---:|---:|");
    for (r, (label, k)) in RUNGS.iter().enumerate() {
        println!(
            "| {label} | {k} | {:.5} | {:.3} | {:.3} |",
            overall[r].nll(),
            overall[r].mae_tricks(),
            slam[r].mae_tricks(),
        );
    }
    println!(
        "| *linear on all {RAW} raw* | {WIDE} | — | *{:.3}* | — |",
        wide_score.mae_tricks(),
    );
    println!("| **evaluator_v2_dnf** | 54→256→256 | **−1.51183** | **1.441** | **2.646** |");

    // The coefficients are the deliverable as much as the scores — an auditable
    // backend is one whose numbers can be read. Print the widest rung for the
    // four our-side game targets.
    const TERMS: [&str; K] = [
        "const",
        "pair HCP",
        "fit",
        "aces",
        "kings",
        "trump HCP",
        "their HCP",
        "shortness",
        "queens",
        "jacks",
        "fit2",
        "spots",
        "pts width",
    ];
    println!("\nR5 coefficients, tricks per unit (our side declaring):");
    println!("| strain | {} |", TERMS.join(" | "));
    println!("|---{}|", "|---:".repeat(K));
    for (strain, name) in ["NT", "♠", "♥", "♦", "♣"].iter().enumerate() {
        let beta = &betas[strain * 4][RUNGS.len() - 1];
        print!("| {name} |");
        for c in beta {
            print!(" {:.3} |", c * 13.0);
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests;
