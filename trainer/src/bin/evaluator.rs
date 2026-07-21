//! Fit the trick evaluator (bilans session C): ranges → double-dummy trick mean
//! and spread.
//!
//! Reads the corpus from `examples/dump-evaluator` — rows of
//! `[features][20 dd_tricks]`, where the features are own-hand summary plus the
//! three hidden seats' shown ranges and **no auction** — and fits a
//! **heteroscedastic Gaussian** per target: two heads, `mu` and `s = ln σ`,
//! trained by negative log-likelihood (dropping the constant `½·ln 2π`):
//!
//! ```text
//! L(t; mu, s) = s + ½·(t − mu)²·exp(−2s)
//! ```
//!
//! Each row is *one* real deal consistent with its ranges, i.e. one unbiased
//! draw from the posterior over hidden hands. Minimising this over the
//! population drives `mu` to the true conditional mean of tricks given the
//! information state and `σ` to its conditional standard deviation — the spread
//! emerges from the population without ever sampling N completions per state.
//!
//! What the Gaussian costs: it *does* assume symmetry and unbounded support,
//! and trick counts are neither — they are discrete, left-skewed on a good fit,
//! and hard-bounded at 13. The `below_mean` metric (fraction of labels strictly
//! under `mu`, nominally 50%) is the diagnostic that measures that cost.
//!
//! ```text
//! cargo run --release --bin evaluator -- --data ../target/eval-train \
//!     --test ../target/eval-test --hidden 64
//! ```
//!
//! `--hidden 0` is the honest linear baseline (~2.5k coefficients); the default
//! `64` is a two-hidden-layer MLP an order of magnitude smaller than the
//! distilled policy net. Deliberately self-contained: the policy trainer's
//! loader hard-asserts a 38-wide softmax, which this corpus does not have.

use anyhow::{Context as _, Result, bail};
use candle_core::{DType, Device, Tensor};
use candle_nn::{AdamW, Linear, Module, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use serde::Deserialize;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Targets per row: 5 strains × 4 declarers.
const DD_LEN: usize = 20;
/// Output heads per target: `mu` and `ln σ`.
const HEADS: usize = 2;
/// Lower clamp on the `ln σ` head, paired with `LN_SD_MAX`. In the corpus's
/// `tricks / 13` units the pair is σ ∈ [0.087, 13] tricks — generous both ways.
/// The clamp exists to stop the classic heteroscedastic collapse, where the net
/// drives σ → 0 on the rows it finds easy and the loss runs to −∞.
// ponytail: a hard clamp zeroes the gradient at the boundary, so a head parked
// there cannot climb back off it. If that ever bites, the upgrade is a softplus
// parameterisation (σ = softplus(raw)), smooth everywhere and needing no clamp.
const LN_SD_MIN: f64 = -5.0;
/// Upper clamp on the `ln σ` head; see `LN_SD_MIN`.
const LN_SD_MAX: f64 = 0.0;
/// Feature floats the three hidden seats' range blocks occupy (the tail).
const LEN_RANGES: usize = 30;
/// One seat's unknown-range encoding: `[min, max]` pairs of `[0, 1]`.
const UNKNOWN_PAIR: [f32; 2] = [0.0, 1.0];

/// splitmix64: advance `state` and return the next word.
///
/// A few lines beat a new dependency, and reproducibility is the only thing
/// asked of it. Shared by [`seed_params`] and the per-epoch shuffle.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Deep-copy every parameter, for the best-val checkpoint.
///
/// `Var::set` writes through the existing storage, so a cloned `Tensor` would
/// alias the live parameter and be clobbered by the next optimiser step —
/// [`Tensor::copy`] is what makes the snapshot a snapshot.
fn snapshot(varmap: &VarMap) -> Result<Vec<(String, Tensor)>> {
    let data = varmap.data().lock().expect("varmap poisoned");
    data.iter()
        .map(|(name, var)| Ok((name.clone(), var.as_tensor().copy()?)))
        .collect()
}

/// Deterministically re-initialise every parameter from `seed`.
///
/// candle's CPU device rejects `set_seed`, so `VarBuilder`'s init is drawn from
/// an unseeded thread-local RNG. Overwriting the parameters afterwards is the
/// cheapest way to make a run reproducible — and it has to be reproducible,
/// because the `ln σ` head is init-sensitive (see [`Args::seed`]).
///
/// Weights are `U(-k, k)` with `k = 1/√fan_in` (PyTorch's `Linear` default);
/// biases start at zero.
fn seed_params(varmap: &VarMap, seed: u64, device: &Device) -> Result<()> {
    let mut state = seed;
    let mut unit = move || (splitmix64(&mut state) >> 11) as f64 / (1u64 << 53) as f64;
    let data = varmap.data().lock().expect("varmap poisoned");
    // Sorted, so the draw order does not depend on HashMap iteration order.
    let mut names: Vec<_> = data.keys().collect();
    names.sort();
    for name in names {
        let var = &data[name];
        let tensor = match *var.dims() {
            [out, fan_in] => {
                let k = 1.0 / (fan_in as f64).sqrt();
                let v: Vec<f32> = (0..out * fan_in)
                    .map(|_| ((2.0 * unit() - 1.0) * k) as f32)
                    .collect();
                Tensor::from_vec(v, (out, fan_in), device)?
            }
            [out] => Tensor::zeros(out, DType::F32, device)?,
            ref other => bail!("unexpected parameter shape {other:?} for {name}"),
        };
        var.set(&tensor)?;
    }
    Ok(())
}

#[derive(Parser)]
#[command(about = "Fit the DD trick mean/spread evaluator (bilans session C)")]
struct Args {
    /// Corpus stem; reads `<stem>.f32`, `<stem>.json`, `<stem>.tags`
    #[arg(long, default_value = "../target/eval-train")]
    data: String,
    /// Held-out corpus stem (deal-disjoint by construction — a different
    /// database slice). Without it, a contiguous `--val-frac` tail is used;
    /// the dump is deal-major, so that tail is still deal-disjoint.
    #[arg(long)]
    test: Option<String>,
    /// Output stem: `<stem>.f32` + `<stem>.json` + `<stem>.fixture.json`
    #[arg(long, default_value = "../src/bidding/weights/evaluator_v1")]
    weights_out: String,
    /// Hidden width of both hidden layers; `0` fits a single linear layer
    #[arg(long, default_value_t = 64)]
    hidden: usize,
    /// Training epochs
    #[arg(long, default_value_t = 60)]
    epochs: usize,
    /// AdamW learning rate
    #[arg(long, default_value_t = 1e-3)]
    lr: f64,
    /// AdamW weight decay
    #[arg(long, default_value_t = 0.0)]
    wd: f64,
    /// Minibatch size
    #[arg(long, default_value_t = 4096)]
    batch: usize,
    /// Seed for weight initialisation. Left unseeded, the `μ` head still lands
    /// within ~0.004 tricks of MAE run to run, but `ln σ` does not: two runs at
    /// identical settings came in 0.075 apart in NLL and 2.7 points apart in
    /// coverage. Anything comparing those two numbers across runs must fix this.
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Validation fraction, taken contiguously from the end (ignored with `--test`)
    #[arg(long, default_value_t = 0.10)]
    val_frac: f64,
    /// Rows to dump as the in-crate parity fixture
    #[arg(long, default_value_t = 8)]
    fixture: usize,
    /// Ablation: overwrite every range block with the *unknown* pattern, so the
    /// net sees only its own hand. The delta against a normal run is what the
    /// auction (compressed to ranges) is worth in tricks.
    #[arg(long)]
    blank_ranges: bool,
    /// Ablation: fold the 20 per-declarer targets to 10 per-side ones, taking
    /// the better declarer of each side.
    #[arg(long)]
    collapse_side: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let device = Device::Cpu;

    let mut train = Dataset::load(&args.data)?;
    let mut val = match &args.test {
        Some(stem) => {
            let ds = Dataset::load(stem)?;
            if ds.features_len != train.features_len {
                bail!(
                    "feature width mismatch: train {} vs test {}",
                    train.features_len,
                    ds.features_len
                );
            }
            ds
        }
        None => {
            let nval = (((train.rows as f64) * args.val_frac).round() as usize)
                .clamp(1, train.rows.saturating_sub(1));
            train.split_off(train.rows - nval)
        }
    };
    if args.blank_ranges {
        train.blank_ranges();
        val.blank_ranges();
    }
    if args.collapse_side {
        train.collapse_side();
        val.collapse_side();
    }

    let in_dim = train.features_len;
    let targets = train.target_len;
    let out_dim = targets * HEADS;
    eprintln!(
        "train {} rows / val {} rows; {in_dim} features → {out_dim} outputs \
         ({targets} targets × {HEADS} heads){}",
        train.rows,
        val.rows,
        if args.blank_ranges {
            " [ranges blanked]"
        } else {
            ""
        },
    );

    let xtrain = train.features_tensor(&device)?;
    let ytrain = train.labels_tensor(&device)?;
    let xval = val.features_tensor(&device)?;
    let yval = val.labels_tensor(&device)?;

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = Net::new(in_dim, args.hidden, out_dim, vb)?;
    seed_params(&varmap, args.seed, &device)?;
    let mut opt = AdamW::new(
        varmap.all_vars(),
        ParamsAdamW {
            lr: args.lr,
            weight_decay: args.wd,
            ..Default::default()
        },
    )?;

    // The corpus is deal-major with ~20 rows per deal, and all rows of a deal
    // share one DD label vector. Walked in order, a nominal 4096-row batch holds
    // only ~200 distinct labels replicated ~20× — and the same ~200 every epoch.
    // The σ head is fit from the spread of residuals within a batch, so that is
    // exactly the head a fixed, label-degenerate batching destabilises. Shuffle.
    let mut perm: Vec<u32> = (0..train.rows as u32).collect();
    let mut rng = args.seed ^ 0x5EED_5EED_5EED_5EED;
    let mut best: Option<(f32, Vec<(String, Tensor)>)> = None;

    for epoch in 1..=args.epochs {
        // Fisher–Yates.
        for i in (1..perm.len()).rev() {
            perm.swap(i, (splitmix64(&mut rng) % (i as u64 + 1)) as usize);
        }
        // Cosine decay to ~0; the decayed tail is what lets the σ head settle.
        let progress = (epoch - 1) as f64 / args.epochs.max(1) as f64;
        opt.set_learning_rate(args.lr * 0.5 * (1.0 + (std::f64::consts::PI * progress).cos()));

        let (mut start, mut running, mut steps) = (0usize, 0f32, 0usize);
        while start < train.rows {
            let len = args.batch.min(train.rows - start);
            let idx = Tensor::from_slice(&perm[start..start + len], len, &device)?;
            let pred = model.forward(&xtrain.index_select(&idx, 0)?)?;
            let loss = gaussian_nll(&pred, &ytrain.index_select(&idx, 0)?, targets)?;
            opt.backward_step(&loss)?;
            running += loss.to_scalar::<f32>()?;
            steps += 1;
            start += len;
        }

        // Every epoch, so the checkpoint is the true best and not the best of a
        // 5-epoch stride; a val forward pass is cheap beside the training epoch.
        let e = evaluate(&model, &xval, &yval, targets, &val.tags)?;
        let nll = e.overall.mean_nll();
        if best.as_ref().is_none_or(|(b, _)| nll < *b) {
            best = Some((nll, snapshot(&varmap)?));
        }
        if epoch == 1 || epoch % 5 == 0 || epoch == args.epochs {
            eprintln!(
                "epoch {epoch:>4}: train {:.5}  val nll {:.5}  MAE {:.3}  RMSE {:.3} tricks  \
                 coverage {:.1}% (constructive {:.1}% / contested {:.1}%)  \
                 below-mu {:.1}%",
                running / steps as f32,
                nll,
                e.overall.mae_tricks(),
                e.overall.rmse_tricks(),
                100.0 * e.overall.coverage(),
                100.0 * e.phase[0].coverage(),
                100.0 * e.phase[1].coverage(),
                100.0 * e.overall.below_mean(),
            );
        }
    }

    // Ship the best epoch, not whatever the last one happened to land on.
    if let Some((nll, params)) = best {
        let data = varmap.data().lock().expect("varmap poisoned");
        for (name, tensor) in &params {
            data[name].set(tensor)?;
        }
        eprintln!("restored best-val checkpoint: nll {nll:.5}");
    }
    let final_eval = evaluate(&model, &xval, &yval, targets, &val.tags)?;
    export(&args, &varmap, &model, &xval, &train, targets, &final_eval)?;
    Ok(())
}

// ── Model ─────────────────────────────────────────────────────────────────────

/// The evaluator: one linear layer, or two ReLU hidden layers. The export order
/// (`l1.weight, l1.bias, …`) is what the in-crate hand-rolled forward pass reads.
struct Net {
    layers: Vec<Linear>,
}

impl Net {
    fn new(in_dim: usize, hidden: usize, out_dim: usize, vb: VarBuilder) -> Result<Self> {
        let layers = if hidden == 0 {
            vec![candle_nn::linear(in_dim, out_dim, vb.pp("l1"))?]
        } else {
            vec![
                candle_nn::linear(in_dim, hidden, vb.pp("l1"))?,
                candle_nn::linear(hidden, hidden, vb.pp("l2"))?,
                candle_nn::linear(hidden, out_dim, vb.pp("l3"))?,
            ]
        };
        Ok(Self { layers })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let mut h = x.clone();
        for (i, layer) in self.layers.iter().enumerate() {
            h = layer.forward(&h)?;
            if i + 1 < self.layers.len() {
                h = h.relu()?;
            }
        }
        Ok(h)
    }

    /// Parameter names in export order.
    fn param_names(&self) -> Vec<String> {
        (1..=self.layers.len())
            .flat_map(|i| [format!("l{i}.weight"), format!("l{i}.bias")])
            .collect()
    }
}

/// Mean Gaussian negative log-likelihood, dropping the constant `½·ln 2π`:
///
/// ```text
/// L(t; mu, s) = s + ½·(t − mu)²·exp(−2s)
/// ```
///
/// Its minimiser is exactly what the evaluator wants: `mu` converges to the
/// conditional **mean** of double-dummy tricks given the information state, and
/// σ to the conditional **standard deviation** — both from single-deal
/// supervision, because each row is one unbiased draw from the posterior over
/// hidden hands, so the population supplies the spread no single row carries.
///
/// `pred` is `(batch, targets × 2)` laid out head-major — all `mu`s, then all
/// `ln σ`s — so each head is one contiguous `narrow`.
fn gaussian_nll(pred: &Tensor, target: &Tensor, targets: usize) -> Result<Tensor> {
    let mu = pred.narrow(1, 0, targets)?;
    let ln_sd = pred
        .narrow(1, targets, targets)?
        .clamp(LN_SD_MIN, LN_SD_MAX)?;
    // ½·(t − mu)²·exp(−2s): squared error weighted by the predicted precision.
    // The net can only buy a cheaper residual by paying the `s` term for it,
    // which is what stops σ from collapsing everywhere it is inconvenient.
    let quad = ((target - &mu)?.sqr()?.affine(0.5, 0.0)? * ln_sd.affine(-2.0, 0.0)?.exp()?)?;
    Ok((ln_sd + quad)?.mean_all()?)
}

/// The standard normal's upper quartile, `Φ⁻¹(0.75)`. The band `mu ± Z75·σ` is
/// the model's central 50%, so its coverage stays directly comparable to the
/// interquartile coverage the old quantile heads reported.
const Z75: f64 = 0.674_490;

/// Metrics over one slice of the held-out set.
#[derive(Default, Clone, Copy)]
struct Slice {
    nll: f64,
    abs: f64,
    sq: f64,
    inside: u64,
    below: u64,
    n: u64,
}

impl Slice {
    fn push(&mut self, nll: f64, abs: f64, inside: bool, below: bool) {
        self.nll += nll;
        self.abs += abs;
        self.sq += abs * abs;
        self.inside += u64::from(inside);
        self.below += u64::from(below);
        self.n += 1;
    }

    fn mean_nll(self) -> f32 {
        (self.nll / self.n.max(1) as f64) as f32
    }

    /// Mean-head absolute error, rescaled from the corpus's `tricks / 13`.
    fn mae_tricks(self) -> f32 {
        (13.0 * self.abs / self.n.max(1) as f64) as f32
    }

    /// Mean-head root-mean-square error, rescaled from the corpus's `tricks / 13`.
    ///
    /// This — not [`Self::mae_tricks`] — is the metric the `μ` head optimises:
    /// squared error is minimised by the conditional *mean*, absolute error by
    /// the conditional *median*. Scoring a mean head on MAE hands a systematic
    /// advantage to any predictor that aims at the median instead, so the two
    /// are reported side by side.
    fn rmse_tricks(self) -> f32 {
        (13.0 * (self.sq / self.n.max(1) as f64).sqrt()) as f32
    }

    /// Fraction of labels inside the central-50% band `mu ± Z75·σ` — nominally
    /// 50%.
    fn coverage(self) -> f32 {
        self.inside as f32 / self.n.max(1) as f32
    }

    /// Fraction of labels strictly below `mu` — the skew diagnostic. For a
    /// symmetric distribution this is 50%; double-dummy trick counts on a good
    /// fit are left-skewed and hard-bounded at 13, so a systematic departure
    /// from 50% is the Gaussian's shape assumption failing to fit the data, not
    /// a bug.
    fn below_mean(self) -> f32 {
        self.below as f32 / self.n.max(1) as f32
    }
}

/// Render a pair of named slices for the sidecar.
fn slices(names: [&str; 2], slices: &[Slice; 2]) -> serde_json::Value {
    let rows: Vec<_> = names
        .iter()
        .zip(slices)
        .map(|(name, s)| {
            serde_json::json!({
                "slice": name,
                "nll": s.mean_nll(),
                "mae_tricks": s.mae_tricks(),
                "rmse_tricks": s.rmse_tricks(),
                "coverage": s.coverage(),
                "below_mean": s.below_mean(),
                "targets": s.n,
            })
        })
        .collect();
    serde_json::Value::Array(rows)
}

/// Held-out metrics, overall and sliced by the two tag bits.
struct Eval {
    overall: Slice,
    /// Tag bit 0: `[constructive, contested]`.
    phase: [Slice; 2],
    /// Tag bit 1: the corpus's system slot. Comparable numbers here are the
    /// evidence for the system-agnostic claim — the net reads ranges, never
    /// calls, so neither book should be systematically easier.
    system: [Slice; 2],
}

fn evaluate(model: &Net, x: &Tensor, y: &Tensor, targets: usize, tags: &[u8]) -> Result<Eval> {
    let p = model.forward(x)?.to_vec2::<f32>()?;
    let t = y.to_vec2::<f32>()?;
    let mut eval = Eval {
        overall: Slice::default(),
        phase: [Slice::default(); 2],
        system: [Slice::default(); 2],
    };

    for (row, (pr, tr)) in p.iter().zip(&t).enumerate() {
        let tag = tags.get(row).copied().unwrap_or(0);
        for j in 0..targets {
            // Score the clamped `ln σ`, which is what training optimised and
            // what serving will read.
            let mu = f64::from(pr[j]);
            let s = f64::from(pr[targets + j]).clamp(LN_SD_MIN, LN_SD_MAX);
            let sigma = s.exp();
            let truth = f64::from(tr[j]);
            let d = truth - mu;
            let nll = s + 0.5 * d * d * (-2.0 * s).exp();
            let abs = d.abs();
            let inside = abs <= Z75 * sigma;
            let below = truth < mu;
            eval.overall.push(nll, abs, inside, below);
            eval.phase[usize::from(tag & 1)].push(nll, abs, inside, below);
            eval.system[usize::from(tag >> 1 & 1)].push(nll, abs, inside, below);
        }
    }
    Ok(eval)
}

// ── Corpus ────────────────────────────────────────────────────────────────────

/// The `dump-evaluator` sidecar fields we depend on (serde ignores the rest).
#[derive(Debug, Deserialize)]
struct Meta {
    feature_version: u32,
    features_len: usize,
    dd_len: usize,
    row_len: usize,
    rows: u64,
    seed: u64,
    #[serde(default)]
    encoding: String,
    #[serde(default)]
    git_sha: String,
    #[serde(default)]
    systems: Vec<String>,
    #[serde(default)]
    deals: String,
}

struct Dataset {
    features: Vec<f32>,
    labels: Vec<f32>,
    tags: Vec<u8>,
    rows: usize,
    features_len: usize,
    target_len: usize,
    meta: Meta,
}

impl Dataset {
    fn load(stem: &str) -> Result<Self> {
        let json_path = format!("{stem}.json");
        let f32_path = format!("{stem}.f32");
        let meta: Meta = serde_json::from_slice(
            &std::fs::read(&json_path).with_context(|| format!("reading sidecar {json_path}"))?,
        )
        .with_context(|| format!("parsing sidecar {json_path}"))?;
        if meta.feature_version != 1 {
            bail!(
                "evaluator feature_version {} unsupported (this trainer knows 1)",
                meta.feature_version
            );
        }
        if meta.dd_len != DD_LEN || meta.row_len != meta.features_len + DD_LEN {
            bail!(
                "row layout mismatch: features_len {} + dd_len {} != row_len {}",
                meta.features_len,
                meta.dd_len,
                meta.row_len
            );
        }

        let bytes = std::fs::read(&f32_path).with_context(|| format!("reading {f32_path}"))?;
        let row_bytes = meta.row_len * 4;
        if bytes.len() % row_bytes != 0 {
            bail!("{f32_path} is not a whole number of {row_bytes}-byte rows");
        }
        let rows = bytes.len() / row_bytes;
        if rows as u64 != meta.rows {
            bail!("{f32_path} has {rows} rows, sidecar says {}", meta.rows);
        }

        let features_len = meta.features_len;
        let mut features = Vec::with_capacity(rows * features_len);
        let mut labels = Vec::with_capacity(rows * DD_LEN);
        for row in bytes.chunks_exact(row_bytes) {
            let mut floats = row
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]));
            features.extend((&mut floats).take(features_len));
            labels.extend(floats);
        }

        let tags_path = format!("{stem}.tags");
        let tags = std::fs::read(&tags_path).with_context(|| format!("reading {tags_path}"))?;
        if tags.len() != rows {
            bail!("{tags_path} has {} tags but {rows} rows", tags.len());
        }

        Ok(Self {
            features,
            labels,
            tags,
            rows,
            features_len,
            target_len: DD_LEN,
            meta,
        })
    }

    /// Split the trailing `rows - at` rows off into a second dataset. The dump
    /// is deal-major, so a contiguous tail never shares a board — and hence
    /// never shares a DD label — with what stays behind.
    fn split_off(&mut self, at: usize) -> Self {
        let tail = Self {
            features: self.features.split_off(at * self.features_len),
            labels: self.labels.split_off(at * self.target_len),
            tags: self.tags.split_off(at),
            rows: self.rows - at,
            features_len: self.features_len,
            target_len: self.target_len,
            meta: Meta {
                feature_version: self.meta.feature_version,
                features_len: self.features_len,
                dd_len: DD_LEN,
                row_len: self.meta.row_len,
                rows: (self.rows - at) as u64,
                seed: self.meta.seed,
                encoding: self.meta.encoding.clone(),
                git_sha: self.meta.git_sha.clone(),
                systems: self.meta.systems.clone(),
                deals: self.meta.deals.clone(),
            },
        };
        self.rows = at;
        tail
    }

    /// Ablation: pretend nothing was ever bid. The unknown *encoding* is
    /// `[0, 1]` per bound pair, not zeros — zeros would be a hand with no cards.
    fn blank_ranges(&mut self) {
        let start = self.features_len - LEN_RANGES;
        for row in self.features.chunks_exact_mut(self.features_len) {
            for pair in row[start..].chunks_exact_mut(2) {
                pair.copy_from_slice(&UNKNOWN_PAIR);
            }
        }
    }

    /// Ablation: 20 per-declarer targets → 10 per-side ones, keeping the better
    /// declarer of each side (right-siding stops being visible).
    fn collapse_side(&mut self) {
        let mut folded = Vec::with_capacity(self.rows * DD_LEN / 2);
        for row in self.labels.chunks_exact(DD_LEN) {
            // Each strain contributes [me, lho, partner, rho].
            for strain in row.chunks_exact(4) {
                folded.push(strain[0].max(strain[2]));
                folded.push(strain[1].max(strain[3]));
            }
        }
        self.labels = folded;
        self.target_len = DD_LEN / 2;
    }

    fn features_tensor(&self, device: &Device) -> Result<Tensor> {
        Ok(Tensor::from_slice(
            &self.features,
            (self.rows, self.features_len),
            device,
        )?)
    }

    fn labels_tensor(&self, device: &Device) -> Result<Tensor> {
        Ok(Tensor::from_slice(
            &self.labels,
            (self.rows, self.target_len),
            device,
        )?)
    }
}

// ── Export ────────────────────────────────────────────────────────────────────

fn export(
    args: &Args,
    varmap: &VarMap,
    model: &Net,
    xval: &Tensor,
    ds: &Dataset,
    targets: usize,
    eval: &Eval,
) -> Result<()> {
    let stem = &args.weights_out;
    if let Some(parent) = Path::new(stem).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let names = model.param_names();
    let f32_path = format!("{stem}.f32");
    let mut w = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut shapes = serde_json::Map::new();
    let mut total = 0usize;
    {
        let data = varmap.data().lock().expect("varmap mutex poisoned");
        for name in &names {
            let var = data
                .get(name)
                .with_context(|| format!("missing param {name}"))?;
            shapes.insert(name.clone(), serde_json::json!(var.dims()));
            for x in var.flatten_all()?.to_vec1::<f32>()? {
                w.write_all(&x.to_le_bytes())?;
                total += 1;
            }
        }
    }
    w.flush()?;

    let out_dim = targets * HEADS;
    let sidecar = serde_json::json!({
        "trainer": "pons-trainer evaluator",
        "feature_version": ds.meta.feature_version,
        "features_len": ds.features_len,
        "encoding": ds.meta.encoding,
        "targets": targets,
        "heads": ["mu", "ln_sd"],
        "ln_sd_clamp": [LN_SD_MIN, LN_SD_MAX],
        "out_dim": out_dim,
        "out_layout": format!("[{targets} × mu][{targets} × ln_sd], tricks / 13"),
        "label_order": "strain-major NT,S,H,D,C × declarer [me,lho,partner,rho]",
        "hidden": args.hidden,
        "arch": if args.hidden == 0 {
            format!("x -> Linear({}, {out_dim})", ds.features_len)
        } else {
            format!(
                "x -> Linear({}, {h}) -> relu -> Linear({h}, {h}) -> relu -> Linear({h}, {out_dim})",
                ds.features_len,
                h = args.hidden
            )
        },
        "param_order": names,
        "param_shapes": shapes,
        "param_floats": total,
        "dtype": "f32-le",
        "blank_ranges": args.blank_ranges,
        "collapse_side": args.collapse_side,
        "data_deals": ds.meta.deals,
        "data_systems": ds.meta.systems,
        "data_git_sha": ds.meta.git_sha,
        "data_seed": ds.meta.seed,
        "train_rows": ds.rows,
        "test": args.test,
        "epochs": args.epochs,
        "lr": args.lr,
        "wd": args.wd,
        "batch": args.batch,
        "seed": args.seed,
        "git_sha": git_sha(),
        "val_nll": eval.overall.mean_nll(),
        "val_mae_tricks": eval.overall.mae_tricks(),
        "val_rmse_tricks": eval.overall.rmse_tricks(),
        "val_coverage": eval.overall.coverage(),
        "val_below_mean": eval.overall.below_mean(),
        "val_by_phase": slices(["constructive", "contested"], &eval.phase),
        "val_by_system": slices(
            [
                ds.meta.systems.first().map_or("system 0", String::as_str),
                ds.meta.systems.get(1).map_or("system 1", String::as_str),
            ],
            &eval.system,
        ),
    });
    std::fs::write(format!("{stem}.json"), format!("{sidecar:#}\n"))?;

    let k = args.fixture.min(xval.dim(0)?);
    if k > 0 {
        let xf = xval.narrow(0, 0, k)?;
        let fixture = serde_json::json!({
            "note": "Parity: the in-crate hand-rolled forward pass must reproduce \
                     these outputs from these features (within tolerance).",
            "feature_version": ds.meta.feature_version,
            "rows": k,
            "features": xf.to_vec2::<f32>()?,
            "outputs": model.forward(&xf)?.to_vec2::<f32>()?,
        });
        std::fs::write(format!("{stem}.fixture.json"), format!("{fixture:#}\n"))?;
    }

    eprintln!("exported {total} floats -> {f32_path} (+ .json, .fixture.json)");
    eprintln!(
        "final val: nll {:.5}  MAE {:.3}  RMSE {:.3} tricks  coverage {:.1}%  below-mu {:.1}%",
        eval.overall.mean_nll(),
        eval.overall.mae_tricks(),
        eval.overall.rmse_tricks(),
        100.0 * eval.overall.coverage(),
        100.0 * eval.overall.below_mean(),
    );
    let row = |name: &str, s: Slice| {
        eprintln!(
            "  {name:<14} nll {:.5}  MAE {:.3}  RMSE {:.3}  coverage {:.1}%  \
             below-mu {:.1}%  ({} targets)",
            s.mean_nll(),
            s.mae_tricks(),
            s.rmse_tricks(),
            100.0 * s.coverage(),
            100.0 * s.below_mean(),
            s.n,
        );
    };
    row("constructive", eval.phase[0]);
    row("contested", eval.phase[1]);
    for (i, s) in eval.system.iter().enumerate() {
        if s.n > 0 {
            row(ds.meta.systems.get(i).map_or("system?", String::as_str), *s);
        }
    }
    Ok(())
}

/// Best-effort current commit for the sidecar; `"unknown"` on failure.
fn git_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string())
}
