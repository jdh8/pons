//! Distill `american()` into a policy net — AI-bidder M1.1 (MLP) and M5.2 (LSTM).
//!
//! Reads the teacher dump (`examples/teacher-dump`), fits a `160 -> H -> H -> 38`
//! MLP to the teacher's softmax by soft-target cross-entropy
//! (`-Σ teacher · log_softmax(student)`), and exports the weights + a sidecar +
//! a parity fixture into the crate's `src/bidding/weights/` for M1.2 to embed.
//!
//! `--arch lstm` swaps the flat net for [`model::LstmPolicy`]: a single-layer
//! LSTM over the auction's call tokens, whose final hidden state is
//! concatenated with the same flat feature vector before the same head. It
//! needs a dump with a `.seq` sibling (a `seq` block in the sidecar); `--arch
//! mlp` stays the default and keeps the old path — it never reads `.seq`, never
//! builds the token tables, and hands `Mlp::new` the root `VarBuilder`, so the
//! weights blob, the six parameter names and their order, and the parity
//! fixture are byte-identical to before. (Held-out *metrics* can move in the
//! last ulp: `evaluate` now accumulates over minibatches, because the LSTM's
//! per-step states will not fit on the device in one go.)
//!
//! This crate is its own cargo workspace (see `Cargo.toml`); it is built and run
//! only from inside `trainer/` and never compiled by the pons build.

mod calibrate;
mod data;
mod model;

use anyhow::{Context as _, Result, bail};
use candle_core::{D, DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::{Parser, ValueEnum};
use data::{SOFTMAX_LEN, SeqMeta};
use model::{LstmPolicy, Mlp, Policy};
use std::io::{BufWriter, Write};
use std::path::Path;

/// Endpoint bytes per reading block: `[len min,max] x 4`, `[points min,max]`,
/// `[support min,max] x 4` — `pons::bidding::features::LEN_INFERENCE_V6`.
const ENDPOINTS_PER_BLOCK: usize = 18;
/// Of those, the leading eight are suit lengths (`/13`); the other ten are
/// point counts (`/37`). The same two divisors `push_inference_v6` uses, which
/// is what makes a token bit-identical to the static block's encoding.
const LENGTH_SLOTS: usize = 8;
/// Width a flag byte expands to: seat one-hot 4, authored, artificial.
const FLAG_LEN: usize = 6;

/// Which policy to fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Arch {
    /// The shipped flat net over the v6 feature vector.
    Mlp,
    /// LSTM auction encoder + the same head (M5.2); needs a `.seq` sibling.
    Lstm,
}

#[derive(Parser)]
#[command(about = "Distill american() into a policy net (AI-bidder M1.1 / M5.2)")]
struct Args {
    /// Teacher-dump path stem; reads `<stem>.f32`, `<stem>.json`, `<stem>.tags`.
    /// Repeatable: several dumps train as one mixture corpus, each contributing
    /// its own board-disjoint validation tail (see [`data::load_mixture`]).
    #[arg(long, default_values_t = ["../target/teacher-data".to_string()])]
    data: Vec<String>,
    /// Output stem for the artifact: `<stem>.f32` + `<stem>.json` + `<stem>.fixture.json`
    #[arg(long, default_value = "../src/bidding/weights/american_v1")]
    weights_out: String,
    /// Which policy to fit: the flat `mlp` (default, the shipped path) or the
    /// `lstm` auction encoder, which requires a dump with a `.seq` sibling.
    #[arg(long, value_enum, default_value_t = Arch::Mlp)]
    arch: Arch,
    /// Hidden width of both hidden layers
    #[arg(long, default_value_t = 256)]
    hidden: usize,
    /// Hidden width of the LSTM state (`--arch lstm` only).
    #[arg(long, default_value_t = 128)]
    lstm_hidden: usize,
    /// Training epochs
    #[arg(long, default_value_t = 300)]
    epochs: usize,
    /// AdamW learning rate
    #[arg(long, default_value_t = 1e-3)]
    lr: f64,
    /// AdamW weight decay (L2 regularization; counters overfitting)
    #[arg(long, default_value_t = 0.0)]
    wd: f64,
    /// Minibatch size
    #[arg(long, default_value_t = 4096)]
    batch: usize,
    /// Validation fraction, taken contiguously from the end (board-disjoint)
    #[arg(long, default_value_t = 0.10)]
    val_frac: f64,
    /// Number of (features, logits) rows to dump as the M1.2 parity fixture
    #[arg(long, default_value_t = 8)]
    fixture: usize,
    /// Weight on the value-head DD-regression loss (only when the dump carries a
    /// DD target); the policy cross-entropy always has weight 1.
    #[arg(long, default_value_t = 1.0)]
    dd_weight: f64,
    /// Train on a CUDA GPU (requires the binary built with `--features cuda`);
    /// otherwise CPU. The policy MLP is tiny, so this pays off mainly for a
    /// multi-draw variance sweep — many independent inits over the same corpus.
    #[arg(long)]
    cuda: bool,
    /// Which CUDA device to use (with `--cuda`), e.g. 0 or 1 to spread a sweep
    /// across both GPUs.
    #[arg(long, default_value_t = 0)]
    device_index: usize,
    /// Seed the weight-init RNG for a reproducible draw. Omitted, the init is
    /// entropy-seeded (the historical unseeded behavior) and every run differs —
    /// which is *why* the retrain variance exists. Set distinct seeds to make a
    /// variance sweep reproducible.
    #[arg(long)]
    init_seed: Option<u64>,
    /// Load `<stem>.f32` and skip training: evaluate, fit `T`, and export.
    ///
    /// The calibration fitter runs at the end of a training job, so a *shipped*
    /// artifact has no temperature until something loads it back. This is that
    /// mode — `--weights-in ../src/bidding/weights/american_bba_v6` over the
    /// same dumps and the same `--val-frac` reads `T` off the same held-out
    /// tail v6 was gated on. Point `--weights-out` at a scratch stem: the
    /// export is a full artifact write, and the fitted `T` belongs in the
    /// shipped sidecar only by a deliberate edit.
    ///
    /// Only the six `PARAM_NAMES` are in the blob, so the auxiliary DD value
    /// head loads at its random init and `val_dd_mse` is reported as null. The
    /// policy head — everything the calibration fit and `val_top1` read — is
    /// exact: re-exporting a loaded artifact is byte-identical.
    #[arg(long)]
    weights_in: Option<String>,
}

/// The constant device tables that turn a packed `.seq` row into the 98-wide
/// token sequence the LSTM eats, plus the row geometry read from the dump
/// sidecar (nothing here is hard-coded to 20/56/1121).
///
/// Expansion is *not* precomputed: the packed corpus is 1121 B/row and the
/// expanded form is 20 x 98 f32 = 7840 B/row, so the bytes stay on the host and
/// one minibatch at a time is uploaded and expanded on the device.
struct Seq {
    /// Bytes per packed row, `1 + max_steps * token_bytes`.
    row: usize,
    max_steps: usize,
    token_bytes: usize,
    /// Endpoint bytes per token, `(1 + boxes) * ENDPOINTS_PER_BLOCK`.
    endpoints: usize,
    /// Expanded token width: `[seat 4][call 38][authored 1][artificial 1][endpoints]`.
    token_len: usize,
    /// `(SOFTMAX_LEN, SOFTMAX_LEN)` identity — the call one-hot, gathered by id.
    eye: Tensor,
    /// `(256, FLAG_LEN)` flag expansion `[seat one-hot 4, authored, artificial]`.
    ///
    /// Only the low nibble carries meaning, so rows repeat every 16 and the raw
    /// byte indexes the table directly — the same thing as masking with `0x0F`,
    /// one kernel instead of four, and out-of-range is impossible by
    /// construction.
    flags: Tensor,
    /// Per-endpoint-byte divisor, `([13] * 8 ++ [37] * 10)` per block.
    scale: Tensor,
    device: Device,
}

impl Seq {
    fn new(device: &Device, meta: &SeqMeta) -> Result<Self> {
        let blocks = 1 + meta.boxes;
        let endpoints = blocks * ENDPOINTS_PER_BLOCK;
        if meta.token_bytes != 2 + endpoints {
            bail!(
                "sidecar seq block: token_bytes {} but 2 + (1 + boxes {}) * {ENDPOINTS_PER_BLOCK} = {}",
                meta.token_bytes,
                meta.boxes,
                2 + endpoints
            );
        }
        let mut eye = vec![0f32; SOFTMAX_LEN * SOFTMAX_LEN];
        for i in 0..SOFTMAX_LEN {
            eye[i * SOFTMAX_LEN + i] = 1.0;
        }
        let mut flags = vec![0f32; 256 * FLAG_LEN];
        for byte in 0..256usize {
            let f = byte & 0x0F;
            flags[byte * FLAG_LEN + (f & 3)] = 1.0;
            flags[byte * FLAG_LEN + 4] = f32::from((f >> 2) & 1 == 1);
            flags[byte * FLAG_LEN + 5] = f32::from((f >> 3) & 1 == 1);
        }
        let mut scale = Vec::with_capacity(endpoints);
        for _ in 0..blocks {
            scale.extend(std::iter::repeat_n(13.0f32, LENGTH_SLOTS));
            scale.extend(std::iter::repeat_n(
                37.0f32,
                ENDPOINTS_PER_BLOCK - LENGTH_SLOTS,
            ));
        }
        Ok(Self {
            row: 1 + meta.max_steps * meta.token_bytes,
            max_steps: meta.max_steps,
            token_bytes: meta.token_bytes,
            endpoints,
            token_len: 4 + SOFTMAX_LEN + 2 + endpoints,
            eye: Tensor::from_slice(&eye, (SOFTMAX_LEN, SOFTMAX_LEN), device)?,
            flags: Tensor::from_slice(&flags, (256, FLAG_LEN), device)?,
            scale: Tensor::from_slice(&scale, endpoints, device)?,
            device: device.clone(),
        })
    }

    /// Expand packed rows `[from, from + n)` of `bytes` into
    /// `(n, max_steps, token_len)` f32 tokens and the `(n,)` u32 step counts.
    fn batch(&self, bytes: &[u8], from: usize, n: usize) -> Result<(Tensor, Tensor)> {
        let raw = Tensor::from_slice(
            &bytes[from * self.row..(from + n) * self.row],
            (n, self.row),
            &self.device,
        )?;
        let len = raw.narrow(1, 0, 1)?.squeeze(1)?.to_dtype(DType::U32)?;
        let steps = self.max_steps;
        let tok =
            raw.narrow(1, 1, steps * self.token_bytes)?
                .reshape((n, steps, self.token_bytes))?;
        // `index_select` insists on 1-D ids, so flatten and reshape back. U32
        // rather than the native U8: candle reads an id equal to the dtype's
        // max as "write zeros", which would silently mangle flag byte 0xFF.
        let ids = |at: usize| -> Result<Tensor> {
            Ok(tok.narrow(2, at, 1)?.flatten_all()?.to_dtype(DType::U32)?)
        };
        let call = self
            .eye
            .index_select(&ids(0)?, 0)?
            .reshape((n, steps, SOFTMAX_LEN))?;
        let flag = self
            .flags
            .index_select(&ids(1)?, 0)?
            .reshape((n, steps, FLAG_LEN))?;
        let ends = tok
            .narrow(2, 2, self.endpoints)?
            .to_dtype(DType::F32)?
            .broadcast_div(&self.scale)?;
        // Canonical order: [seat 4][call 38][authored][artificial][hull|box1|box2].
        let seq = Tensor::cat(
            &[&flag.narrow(2, 0, 4)?, &call, &flag.narrow(2, 4, 2)?, &ends],
            2,
        )?;
        Ok((seq, len))
    }
}

/// Held-out metrics, split by the constructive/contested tag.
struct Eval {
    loss: f32,
    overall: f32,
    constructive: f32,
    contested: f32,
    n_constructive: usize,
    n_contested: usize,
    /// Value-head DD mean-squared error, if the head is present.
    dd_mse: Option<f32>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let device = if args.cuda {
        Device::new_cuda(args.device_index)?
    } else {
        Device::Cpu
    };
    // Seed the init RNG *before* the VarMap draws its weights, so a set seed
    // makes the draw reproducible; without one, entropy seeds it (unseeded).
    if let Some(seed) = args.init_seed {
        device.set_seed(seed)?;
    }
    eprintln!("device: {device:?}  init_seed: {:?}", args.init_seed);

    let (ds, ntrain) = data::load_mixture(
        &args.data,
        args.val_frac,
        args.batch,
        args.arch == Arch::Lstm,
    )?;
    let features_len = ds.features_len;
    let nval = ds.rows - ntrain;
    eprintln!(
        "loaded {} rows from {} dump(s) (feature v{}, {features_len} features, seed {}, \
         teacher {:?}); train {ntrain} / val {nval}",
        ds.rows,
        args.data.len(),
        ds.meta.feature_version,
        ds.meta.seed,
        ds.meta.teacher
    );

    let slice = |v: &[f32], from: usize, n: usize, w: usize| -> Result<Tensor> {
        Ok(Tensor::from_slice(
            &v[from * w..(from + n) * w],
            (n, w),
            &device,
        )?)
    };
    let xtrain = slice(&ds.features, 0, ntrain, features_len)?;
    let ytrain = slice(&ds.targets, 0, ntrain, SOFTMAX_LEN)?;
    let xval = slice(&ds.features, ntrain, nval, features_len)?;
    let yval = slice(&ds.targets, ntrain, nval, SOFTMAX_LEN)?;
    let val_tags = &ds.tags[ntrain..];

    // Optional DD regression target (present iff the dump was fed a GIB file).
    let dd_dim = ds.dd_len;
    let ddtrain = (dd_dim > 0)
        .then(|| slice(&ds.dd, 0, ntrain, dd_dim))
        .transpose()?;
    let ddval = (dd_dim > 0)
        .then(|| slice(&ds.dd, ntrain, nval, dd_dim))
        .transpose()?;

    // Auction tokens: host-resident bytes plus the constant expansion tables.
    // Built only for `--arch lstm`, so the MLP path allocates nothing new and
    // its weight draw stays exactly where it was in the RNG stream.
    let seq = match args.arch {
        Arch::Mlp => None,
        Arch::Lstm => {
            let Some(meta) = ds.meta.seq.clone() else {
                bail!(
                    "--arch lstm needs a dump with a .seq sibling (a \"seq\" block in the \
                     sidecar); {:?} has none",
                    args.data
                );
            };
            let seq = Seq::new(&device, &meta)?;
            if seq.row != ds.seq_row {
                bail!(
                    "seq geometry mismatch: tables say {} B/row, loader read {}",
                    seq.row,
                    ds.seq_row
                );
            }
            eprintln!(
                "auction tokens: v{}, {} steps x {} B -> {} f32/token ({} MB of .seq on the \
                 host); layout {:?}",
                meta.version,
                seq.max_steps,
                seq.token_bytes,
                seq.token_len,
                ds.seq.len() / (1 << 20),
                meta.layout
            );
            Some(seq)
        }
    };
    let train_seq = seq.as_ref().map(|s| (s, &ds.seq[..ntrain * s.row]));
    let val_seq = seq.as_ref().map(|s| (s, &ds.seq[ntrain * s.row..]));

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = match &seq {
        None => Policy::Mlp(Mlp::new(
            features_len,
            args.hidden,
            SOFTMAX_LEN,
            dd_dim,
            vb,
        )?),
        Some(seq) => Policy::Lstm(LstmPolicy::new(
            seq.token_len,
            args.lstm_hidden,
            features_len,
            args.hidden,
            SOFTMAX_LEN,
            dd_dim,
            vb,
        )?),
    };
    let mut opt = AdamW::new(
        varmap.all_vars(),
        ParamsAdamW {
            lr: args.lr,
            weight_decay: args.wd,
            ..Default::default()
        },
    )?;

    // `--weights-in` swaps the fit for a load: same model, same held-out tail,
    // no optimizer steps. Everything downstream — evaluate, calibrate, export —
    // is untouched, which is the point: the temperature is fitted by exactly the
    // code that would have fitted it at the end of the original run.
    let epochs = if let Some(stem) = &args.weights_in {
        load_weights(stem, &varmap, &model, &device)?;
        eprintln!("loaded {stem}.f32; training skipped");
        0
    } else {
        args.epochs
    };
    for epoch in 1..=epochs {
        let (mut start, mut running, mut steps) = (0usize, 0f32, 0usize);
        while start < ntrain {
            let len = args.batch.min(ntrain - start);
            let xb = xtrain.narrow(0, start, len)?;
            let yb = ytrain.narrow(0, start, len)?;
            let sb = train_seq
                .map(|(seq, bytes)| seq.batch(bytes, start, len))
                .transpose()?;
            let (logits, value) = model.forward(&xb, sb.as_ref().map(|(t, l)| (t, l)))?;
            let logp = candle_nn::ops::log_softmax(&logits, D::Minus1)?;
            // Soft-target cross-entropy: -mean_b Σ_c teacher · log_softmax(student).
            let mut loss = yb.mul(&logp)?.sum(D::Minus1)?.mean(0)?.neg()?;
            // Auxiliary value head: MSE to the cached DD table.
            if let (Some(value), Some(ddtrain)) = (value, &ddtrain) {
                let ddb = ddtrain.narrow(0, start, len)?;
                let mse = value.sub(&ddb)?.sqr()?.mean_all()?;
                loss = (loss + (mse * args.dd_weight)?)?;
            }
            opt.backward_step(&loss)?;
            running += loss.to_scalar::<f32>()?;
            steps += 1;
            start += len;
        }
        if epoch == 1 || epoch % 10 == 0 || epoch == args.epochs {
            let e = evaluate(
                &model,
                &xval,
                &yval,
                ddval.as_ref(),
                val_tags,
                val_seq,
                args.batch,
            )?;
            let dd = e
                .dd_mse
                .map_or(String::new(), |m| format!("  val_dd_mse {m:.4}"));
            eprintln!(
                "epoch {epoch:>4}/{epochs}: train_loss {:.4}  val_ce {:.4}  top1 {:.1}%{dd}  \
                 (constructive {:.1}% / {}, contested {:.1}% / {})",
                running / steps as f32,
                e.loss,
                100.0 * e.overall,
                100.0 * e.constructive,
                e.n_constructive,
                100.0 * e.contested,
                e.n_contested,
            );
        }
    }

    let final_eval = evaluate(
        &model,
        &xval,
        &yval,
        ddval.as_ref(),
        val_tags,
        val_seq,
        args.batch,
    )?;
    // Post-hoc temperature: fitted on the held-out split, reported, and written
    // to the sidecar. Serving reads the raw logits — argmax is scale-invariant,
    // so the shipped floor is byte-identical. See `calibrate`'s module doc.
    let cal = calibrate::fit(
        &val_logits(&model, &xval, val_seq, args.batch)?,
        &yval.flatten_all()?.to_vec1::<f32>()?,
        SOFTMAX_LEN,
    );
    eprintln!(
        "calibration: T {:.4} on {} held-out rows  val_nll {:.4} -> {:.4}  ECE {:.4} -> {:.4}  \
         (argmax unchanged; serving reads raw logits)",
        cal.temperature, cal.rows, cal.nll_before, cal.nll_after, cal.ece_before, cal.ece_after,
    );
    export(
        &args,
        &varmap,
        &model,
        &xval,
        &ds,
        ntrain,
        nval,
        &final_eval,
        &cal,
        val_seq,
    )?;
    Ok(())
}

/// Forward over the whole validation set; report soft-CE and top-1 agreement
/// with the teacher, split by the constructive/contested tag.
///
/// Chunked by `batch` and accumulated as a row-weighted mean. The LSTM's
/// per-step states are `(rows, steps, hidden)`, so a whole-validation forward
/// would stack ~40 GB on the device; the MLP is chunked through the same path
/// for one code path, which perturbs `val_ce` only in the summation order.
fn evaluate(
    model: &Policy,
    x: &Tensor,
    y: &Tensor,
    dd: Option<&Tensor>,
    tags: &[u8],
    seq: Option<(&Seq, &[u8])>,
    batch: usize,
) -> Result<Eval> {
    let rows = x.dim(0)?;
    let (mut loss_sum, mut dd_sum, mut dd_elems) = (0f64, 0f64, 0usize);
    let (mut hit, mut hit0, mut hit1, mut n0, mut n1) = (0usize, 0usize, 0usize, 0usize, 0usize);
    let mut start = 0usize;
    while start < rows {
        let len = batch.max(1).min(rows - start);
        let xb = x.narrow(0, start, len)?;
        let yb = y.narrow(0, start, len)?;
        let sb = seq
            .map(|(s, bytes)| s.batch(bytes, start, len))
            .transpose()?;
        let (logits, value) = model.forward(&xb, sb.as_ref().map(|(t, l)| (t, l)))?;
        let logp = candle_nn::ops::log_softmax(&logits, D::Minus1)?;
        loss_sum += f64::from(
            yb.mul(&logp)?
                .sum(D::Minus1)?
                .sum(0)?
                .neg()?
                .to_scalar::<f32>()?,
        );
        if let (Some(value), Some(dd)) = (value, dd) {
            let ddb = dd.narrow(0, start, len)?;
            dd_sum += f64::from(value.sub(&ddb)?.sqr()?.sum_all()?.to_scalar::<f32>()?);
            dd_elems += ddb.elem_count();
        }
        let pred = logits.argmax(D::Minus1)?.to_vec1::<u32>()?;
        let gold = yb.argmax(D::Minus1)?.to_vec1::<u32>()?;
        for i in 0..len {
            let ok = usize::from(pred[i] == gold[i]);
            hit += ok;
            if tags[start + i] == 0 {
                n0 += 1;
                hit0 += ok;
            } else {
                n1 += 1;
                hit1 += ok;
            }
        }
        start += len;
    }
    let frac = |h: usize, n: usize| if n == 0 { 0.0 } else { h as f32 / n as f32 };
    Ok(Eval {
        loss: (loss_sum / rows.max(1) as f64) as f32,
        overall: frac(hit, rows),
        constructive: frac(hit0, n0),
        contested: frac(hit1, n1),
        n_constructive: n0,
        n_contested: n1,
        dd_mse: (dd_elems > 0).then(|| (dd_sum / dd_elems as f64) as f32),
    })
}

/// Forward the whole validation set once and return the raw logits, row-major
/// `(rows, SOFTMAX_LEN)` — what [`calibrate::fit`] needs and [`evaluate`]
/// reduces away. Chunked by `batch` for the same reason `evaluate` is.
fn val_logits(
    model: &Policy,
    x: &Tensor,
    seq: Option<(&Seq, &[u8])>,
    batch: usize,
) -> Result<Vec<f32>> {
    let rows = x.dim(0)?;
    let mut out = Vec::with_capacity(rows * SOFTMAX_LEN);
    let mut start = 0usize;
    while start < rows {
        let len = batch.max(1).min(rows - start);
        let sb = seq
            .map(|(s, bytes)| s.batch(bytes, start, len))
            .transpose()?;
        let (logits, _) =
            model.forward(&x.narrow(0, start, len)?, sb.as_ref().map(|(t, l)| (t, l)))?;
        out.extend(logits.flatten_all()?.to_vec1::<f32>()?);
        start += len;
    }
    Ok(out)
}

/// Read a flat `<stem>.f32` blob back into `varmap`, in `param_names()` order
///
/// The exact inverse of [`export`]'s weight write — same order, same
/// little-endian f32 layout — so a shipped artifact round-trips. The float
/// count is the only cheap consistency check there is, and it is precisely what
/// a wrong `--hidden`, `--arch`, or a dump with a different `dd_len` looks
/// like, so it is reported with both numbers rather than read as a truncation.
fn load_weights(stem: &str, varmap: &VarMap, model: &Policy, device: &Device) -> Result<()> {
    let path = format!("{stem}.f32");
    let bytes = std::fs::read(&path).with_context(|| format!("reading {path}"))?;
    let (words, tail) = bytes.as_chunks::<4>();
    if !tail.is_empty() {
        bail!("{path}: {} bytes is not a whole number of f32", bytes.len());
    }
    let floats: Vec<f32> = words.iter().copied().map(f32::from_le_bytes).collect();
    let data = varmap.data().lock().expect("varmap mutex poisoned");
    let mut vars = Vec::with_capacity(model.param_names().len());
    for &name in model.param_names() {
        vars.push(
            data.get(name)
                .with_context(|| format!("missing param {name}"))?,
        );
    }
    let want: usize = vars.iter().map(|var| var.elem_count()).sum();
    if floats.len() != want {
        bail!(
            "{path} holds {} floats but this model wants {want}; check --arch, --hidden \
             and the dump's dd_len against the artifact's sidecar",
            floats.len(),
        );
    }
    let mut at = 0;
    for var in vars {
        let n = var.elem_count();
        var.set(&Tensor::from_slice(
            &floats[at..at + n],
            var.dims().to_vec(),
            device,
        )?)?;
        at += n;
    }
    Ok(())
}

/// Write the weights (`<stem>.f32`, layer order `PARAM_NAMES`), the versioned
/// sidecar, and a small (features, logits) parity fixture for M1.2.
#[allow(clippy::too_many_arguments)]
fn export(
    args: &Args,
    varmap: &VarMap,
    model: &Policy,
    xval: &Tensor,
    ds: &data::Dataset,
    ntrain: usize,
    nval: usize,
    eval: &Eval,
    cal: &calibrate::Calibration,
    seq: Option<(&Seq, &[u8])>,
) -> Result<()> {
    let stem = &args.weights_out;
    if let Some(parent) = Path::new(stem).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let f32_path = format!("{stem}.f32");
    let mut w = BufWriter::new(std::fs::File::create(&f32_path)?);
    let mut shapes = serde_json::Map::new();
    let mut total = 0usize;
    {
        let data = varmap.data().lock().expect("varmap mutex poisoned");
        for &name in model.param_names() {
            let var = data
                .get(name)
                .with_context(|| format!("missing param {name}"))?;
            shapes.insert(name.to_string(), serde_json::json!(var.dims()));
            for x in var.flatten_all()?.to_vec1::<f32>()? {
                w.write_all(&x.to_le_bytes())?;
                total += 1;
            }
        }
    }
    w.flush()?;

    let mut sidecar = serde_json::json!({
        "trainer": "pons-trainer 0.1.0",
        "feature_version": ds.meta.feature_version,
        "features_len": ds.features_len,
        "softmax_len": SOFTMAX_LEN,
        "hidden": args.hidden,
        "arch": match model {
            Policy::Mlp(_) => format!(
                "x -> Linear({},H) -> relu -> Linear(H,H) -> relu -> Linear(H,{SOFTMAX_LEN})",
                ds.features_len
            ),
            Policy::Lstm(_) => format!(
                "[tokens -> LSTM({},{})].h_at_len | x -> Linear({},H) -> relu -> Linear(H,H) \
                 -> relu -> Linear(H,{SOFTMAX_LEN})",
                seq.map_or(0, |(s, _)| s.token_len),
                args.lstm_hidden,
                args.lstm_hidden + ds.features_len
            ),
        },
        "param_order": model.param_names(),
        "param_shapes": shapes,
        "param_floats": total,
        "dtype": "f32-le",
        "teacher": ds.meta.teacher,
        "card": ds.meta.card,
        "conv": ds.meta.conv,
        "our_kickback": ds.meta.our_kickback,
        "mix_kickback": ds.meta.mix_kickback,
        "data_git_sha": ds.meta.git_sha,
        "data_seed": ds.meta.seed,
        "data_stems": &args.data,
        "data_rows": ds.rows,
        "data_contested_rows": ds.meta.contested_rows,
        "train_rows": ntrain,
        "val_rows": nval,
        "epochs": if args.weights_in.is_some() { 0 } else { args.epochs },
        "lr": args.lr,
        "wd": args.wd,
        "batch": args.batch,
        "init_seed": args.init_seed,
        "device": if args.cuda { format!("cuda:{}", args.device_index) } else { "cpu".into() },
        "git_sha": git_sha(),
        "val_ce": eval.loss,
        "val_top1_overall": eval.overall,
        "val_top1_constructive": eval.constructive,
        "val_top1_contested": eval.contested,
        "dd_len": ds.dd_len,
        "dd_weight": args.dd_weight,
        // The value head is auxiliary and `PARAM_NAMES` does not export it, so a
        // `--weights-in` model runs it at its random init: the metric would be
        // noise, not a measurement of the artifact.
        "val_dd_mse": if args.weights_in.is_some() { None } else { eval.dd_mse },
        "temperature": cal.temperature,
        "val_nll_raw": cal.nll_before,
        "val_nll_calibrated": cal.nll_after,
        "val_ece_raw": cal.ece_before,
        "val_ece_calibrated": cal.ece_after,
    });
    // Both blocks add keys only on the path that needs them, so an ordinary
    // `--arch mlp` training run's sidecar keeps the exact key set it had.
    if let (Some(stem), Some(obj)) = (&args.weights_in, sidecar.as_object_mut()) {
        obj.insert("weights_in".into(), serde_json::json!(stem));
    }
    // Only the LSTM artifact carries these, so an `--arch mlp` sidecar keeps the
    // exact key set it had before the sequence channel existed.
    if let (Some((seq, _)), Some(obj)) = (seq, sidecar.as_object_mut()) {
        obj.insert("lstm_hidden".into(), serde_json::json!(args.lstm_hidden));
        obj.insert("max_steps".into(), serde_json::json!(seq.max_steps));
        obj.insert("token_len".into(), serde_json::json!(seq.token_len));
    }
    std::fs::write(format!("{stem}.json"), format!("{sidecar:#}\n"))?;

    let k = args.fixture.min(nval);
    if k > 0 {
        let xf = xval.narrow(0, 0, k)?;
        let sf = seq.map(|(s, bytes)| s.batch(bytes, 0, k)).transpose()?;
        let logits = model.forward(&xf, sf.as_ref().map(|(t, l)| (t, l)))?.0;
        let mut fixture = serde_json::json!({
            "note": "M1.2 parity: the in-crate hand-rolled forward pass must reproduce \
                     these logits from these features (within tolerance).",
            "feature_version": ds.meta.feature_version,
            "rows": k,
            "features": xf.to_vec2::<f32>()?,
            "logits": logits.to_vec2::<f32>()?,
        });
        if let (Some((seq, bytes)), Some((tokens, len)), Some(obj)) =
            (seq, &sf, fixture.as_object_mut())
        {
            obj.insert(
                "note".into(),
                serde_json::json!(
                    "M5.2 parity: the in-crate hand-rolled LSTM + head must reproduce these \
                     logits from these features and tokens (within tolerance); `tokens` is the \
                     expansion of `seq_u8` and must match the crate's own, bit for bit."
                ),
            );
            obj.insert("len".into(), serde_json::json!(len.to_vec1::<u32>()?));
            obj.insert(
                "seq_u8".into(),
                serde_json::json!(
                    bytes[..k * seq.row]
                        .chunks(seq.row)
                        .map(<[u8]>::to_vec)
                        .collect::<Vec<_>>()
                ),
            );
            obj.insert("tokens".into(), serde_json::json!(tokens.to_vec3::<f32>()?));
        }
        std::fs::write(format!("{stem}.fixture.json"), format!("{fixture:#}\n"))?;
    }

    eprintln!("exported {total} floats -> {f32_path} (+ .json, .fixture.json)");
    eprintln!(
        "final val top1: overall {:.1}%  constructive {:.1}%  contested {:.1}%",
        100.0 * eval.overall,
        100.0 * eval.constructive,
        100.0 * eval.contested,
    );
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
