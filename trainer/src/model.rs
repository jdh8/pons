//! The distilled policy net: a plain MLP `160 -> H -> H -> 38`.
//!
//! This is the exact arithmetic M1.2 reimplements by hand in the crate:
//! `z = affine(W3, relu(affine(W2, relu(affine(W1, x)))))`. `candle_nn::linear`
//! stores each weight as `(out, in)` row-major and computes `x · Wᵀ + b`, so the
//! exported layer order `W1,b1,W2,b2,W3,b3` maps directly onto the hand-rolled
//! `affine` in `src/bidding/neural.rs`.

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
}

impl Mlp {
    /// Build the net, registering trainable variables under `vb`.
    pub fn new(in_dim: usize, hidden: usize, out_dim: usize, vb: VarBuilder) -> Result<Self> {
        Ok(Self {
            l1: candle_nn::linear(in_dim, hidden, vb.pp("l1"))?,
            l2: candle_nn::linear(hidden, hidden, vb.pp("l2"))?,
            l3: candle_nn::linear(hidden, out_dim, vb.pp("l3"))?,
        })
    }

    /// Forward pass returning raw logits (no softmax). `x` is `(batch, in_dim)`.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let h = self.l1.forward(x)?.relu()?;
        let h = self.l2.forward(&h)?.relu()?;
        self.l3.forward(&h)
    }
}
