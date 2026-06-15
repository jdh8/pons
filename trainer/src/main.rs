//! Distill `two_over_one()` into an MLP — AI-bidder M1.1.
//!
//! Reads the teacher dump (`examples/teacher-dump`), fits a `160 -> H -> H -> 38`
//! MLP to the teacher's softmax by soft-target cross-entropy
//! (`-Σ teacher · log_softmax(student)`), and exports the weights + a sidecar +
//! a parity fixture into the crate's `src/bidding/weights/` for M1.2 to embed.
//!
//! This crate is its own cargo workspace (see `Cargo.toml`); it is built and run
//! only from inside `trainer/` and never compiled by the pons build.

mod data;
mod model;

use anyhow::{Context as _, Result};
use candle_core::{D, DType, Device, Tensor};
use candle_nn::{AdamW, Optimizer, ParamsAdamW, VarBuilder, VarMap};
use clap::Parser;
use data::SOFTMAX_LEN;
use model::{Mlp, PARAM_NAMES};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Parser)]
#[command(about = "Distill two_over_one() into an MLP (AI-bidder M1.1)")]
struct Args {
    /// Teacher-dump path stem; reads `<stem>.f32`, `<stem>.json`, `<stem>.tags`
    #[arg(long, default_value = "../target/teacher-data")]
    data: String,
    /// Output stem for the artifact: `<stem>.f32` + `<stem>.json` + `<stem>.fixture.json`
    #[arg(long, default_value = "../src/bidding/weights/two_over_one_v1")]
    weights_out: String,
    /// Hidden width of both hidden layers
    #[arg(long, default_value_t = 256)]
    hidden: usize,
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
}

/// Held-out metrics, split by the constructive/contested tag.
struct Eval {
    loss: f32,
    overall: f32,
    constructive: f32,
    contested: f32,
    n_constructive: usize,
    n_contested: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let device = Device::Cpu;

    let ds = data::Dataset::load(&args.data)?;
    let features_len = ds.features_len;
    let nval =
        (((ds.rows as f64) * args.val_frac).round() as usize).clamp(1, ds.rows.saturating_sub(1));
    let ntrain = ds.rows - nval;
    eprintln!(
        "loaded {} rows (feature v{}, {features_len} features, seed {}, teacher {:?}); \
         train {ntrain} / val {nval}",
        ds.rows, ds.meta.feature_version, ds.meta.seed, ds.meta.teacher
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

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let model = Mlp::new(features_len, args.hidden, SOFTMAX_LEN, vb)?;
    let mut opt = AdamW::new(
        varmap.all_vars(),
        ParamsAdamW {
            lr: args.lr,
            weight_decay: args.wd,
            ..Default::default()
        },
    )?;

    for epoch in 1..=args.epochs {
        let (mut start, mut running, mut steps) = (0usize, 0f32, 0usize);
        while start < ntrain {
            let len = args.batch.min(ntrain - start);
            let xb = xtrain.narrow(0, start, len)?;
            let yb = ytrain.narrow(0, start, len)?;
            let logits = model.forward(&xb)?;
            let logp = candle_nn::ops::log_softmax(&logits, D::Minus1)?;
            // Soft-target cross-entropy: -mean_b Σ_c teacher · log_softmax(student).
            let loss = yb.mul(&logp)?.sum(D::Minus1)?.mean(0)?.neg()?;
            opt.backward_step(&loss)?;
            running += loss.to_scalar::<f32>()?;
            steps += 1;
            start += len;
        }
        if epoch == 1 || epoch % 10 == 0 || epoch == args.epochs {
            let e = evaluate(&model, &xval, &yval, val_tags)?;
            eprintln!(
                "epoch {epoch:>4}: train_ce {:.4}  val_ce {:.4}  top1 {:.1}%  \
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

    let final_eval = evaluate(&model, &xval, &yval, val_tags)?;
    export(
        &args,
        &varmap,
        &model,
        &xval,
        &ds,
        ntrain,
        nval,
        &final_eval,
    )?;
    Ok(())
}

/// Forward over the whole validation set; report soft-CE and top-1 agreement
/// with the teacher, split by the constructive/contested tag.
fn evaluate(model: &Mlp, x: &Tensor, y: &Tensor, tags: &[u8]) -> Result<Eval> {
    let logits = model.forward(x)?;
    let logp = candle_nn::ops::log_softmax(&logits, D::Minus1)?;
    let loss = y
        .mul(&logp)?
        .sum(D::Minus1)?
        .mean(0)?
        .neg()?
        .to_scalar::<f32>()?;
    let pred = logits.argmax(D::Minus1)?.to_vec1::<u32>()?;
    let gold = y.argmax(D::Minus1)?.to_vec1::<u32>()?;

    let (mut hit, mut hit0, mut hit1, mut n0, mut n1) = (0usize, 0usize, 0usize, 0usize, 0usize);
    for i in 0..pred.len() {
        let ok = usize::from(pred[i] == gold[i]);
        hit += ok;
        if tags[i] == 0 {
            n0 += 1;
            hit0 += ok;
        } else {
            n1 += 1;
            hit1 += ok;
        }
    }
    let frac = |h: usize, n: usize| if n == 0 { 0.0 } else { h as f32 / n as f32 };
    Ok(Eval {
        loss,
        overall: frac(hit, pred.len()),
        constructive: frac(hit0, n0),
        contested: frac(hit1, n1),
        n_constructive: n0,
        n_contested: n1,
    })
}

/// Write the weights (`<stem>.f32`, layer order `PARAM_NAMES`), the versioned
/// sidecar, and a small (features, logits) parity fixture for M1.2.
#[allow(clippy::too_many_arguments)]
fn export(
    args: &Args,
    varmap: &VarMap,
    model: &Mlp,
    xval: &Tensor,
    ds: &data::Dataset,
    ntrain: usize,
    nval: usize,
    eval: &Eval,
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
        for name in PARAM_NAMES {
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

    let sidecar = serde_json::json!({
        "trainer": "pons-trainer 0.1.0",
        "feature_version": ds.meta.feature_version,
        "features_len": ds.features_len,
        "softmax_len": SOFTMAX_LEN,
        "hidden": args.hidden,
        "arch": format!(
            "x -> Linear({},H) -> relu -> Linear(H,H) -> relu -> Linear(H,{SOFTMAX_LEN})",
            ds.features_len
        ),
        "param_order": PARAM_NAMES,
        "param_shapes": shapes,
        "param_floats": total,
        "dtype": "f32-le",
        "teacher": ds.meta.teacher,
        "data_git_sha": ds.meta.git_sha,
        "data_seed": ds.meta.seed,
        "data_rows": ds.rows,
        "data_contested_rows": ds.meta.contested_rows,
        "train_rows": ntrain,
        "val_rows": nval,
        "epochs": args.epochs,
        "lr": args.lr,
        "wd": args.wd,
        "batch": args.batch,
        "git_sha": git_sha(),
        "val_ce": eval.loss,
        "val_top1_overall": eval.overall,
        "val_top1_constructive": eval.constructive,
        "val_top1_contested": eval.contested,
    });
    std::fs::write(format!("{stem}.json"), format!("{sidecar:#}\n"))?;

    let k = args.fixture.min(nval);
    if k > 0 {
        let xf = xval.narrow(0, 0, k)?;
        let fixture = serde_json::json!({
            "note": "M1.2 parity: the in-crate hand-rolled forward pass must reproduce \
                     these logits from these features (within tolerance).",
            "feature_version": ds.meta.feature_version,
            "rows": k,
            "features": xf.to_vec2::<f32>()?,
            "logits": model.forward(&xf)?.to_vec2::<f32>()?,
        });
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
