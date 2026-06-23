//! The distilled policy net: a plain MLP `160 -> H -> H -> 38`.
//!
//! This is the exact arithmetic M1.2 reimplements by hand in the crate:
//! `z = affine(W3, relu(affine(W2, relu(affine(W1, x)))))`. `candle_nn::linear`
//! stores each weight as `(out, in)` row-major and computes `x · Wᵀ + b`, so the
//! exported layer order `W1,b1,W2,b2,W3,b3` maps directly onto the hand-rolled
//! `affine` in `src/bidding/neural.rs`.
//!
//! An optional **value head** (`vh`, `H -> dd_len`) branches off the second
//! hidden activation to regress the deal's double-dummy table. It is a
//! train-only auxiliary: its gradients shape the shared trunk, but it is **not**
//! in `PARAM_NAMES`, so the exported policy weights and the M1.2 parity are
//! byte-identical whether or not it is present.

use candle_core::{Result, Tensor};
use candle_nn::{Linear, Module, VarBuilder};

/// Ordered parameter names as registered in the `VarMap` — also the export
/// order written to the `.f32` weights artifact.
pub const PARAM_NAMES: [&str; 6] = [
    "l1.weight",
    "l1.bias",
    "l2.weight",
    "l2.bias",
    "l3.weight",
    "l3.bias",
];

pub struct Mlp {
    l1: Linear,
    l2: Linear,
    l3: Linear,
    /// Train-only value head (`H -> dd_len`); `None` when `dd_len == 0`.
    vh: Option<Linear>,
}

impl Mlp {
    /// Build the net, registering trainable variables under `vb`. `dd_dim > 0`
    /// adds the value head.
    pub fn new(
        in_dim: usize,
        hidden: usize,
        out_dim: usize,
        dd_dim: usize,
        vb: VarBuilder,
    ) -> Result<Self> {
        Ok(Self {
            l1: candle_nn::linear(in_dim, hidden, vb.pp("l1"))?,
            l2: candle_nn::linear(hidden, hidden, vb.pp("l2"))?,
            l3: candle_nn::linear(hidden, out_dim, vb.pp("l3"))?,
            vh: if dd_dim > 0 {
                Some(candle_nn::linear(hidden, dd_dim, vb.pp("vh"))?)
            } else {
                None
            },
        })
    }

    /// Forward pass returning raw logits (no softmax) and, if the value head is
    /// present, the regressed DD values. `x` is `(batch, in_dim)`.
    pub fn forward(&self, x: &Tensor) -> Result<(Tensor, Option<Tensor>)> {
        let h = self.l1.forward(x)?.relu()?;
        let h = self.l2.forward(&h)?.relu()?;
        let logits = self.l3.forward(&h)?;
        let value = self.vh.as_ref().map(|vh| vh.forward(&h)).transpose()?;
        Ok((logits, value))
    }
}
