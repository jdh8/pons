//! The distilled policy net: a plain MLP `160 -> H -> H -> 38`, and its
//! sequence-aware sibling [`LstmPolicy`].
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
//!
//! [`LstmPolicy`] (M5.2) prepends a single-layer LSTM over the auction's call
//! tokens and feeds `[h_last | x]` to the same [`Mlp`], which is why the head
//! is built with `vb.pp("head")` and the arithmetic downstream of the encoder
//! is unchanged. [`Policy`] picks between the two; the `Mlp` arm keeps the
//! **root** `VarBuilder`, so `--arch mlp` still exports the same six tensors
//! under the same six names in the same order as before this file grew.

use candle_core::{Result, Tensor};
use candle_nn::{LSTM, LSTMConfig, Linear, Module, RNN, VarBuilder};

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

/// Ordered parameter names for [`LstmPolicy`], likewise the export order.
///
/// The four `lstm.*` names are exactly what `candle_nn::lstm` registers under
/// `vb.pp("lstm")` (`weight_ih_l{layer}`, `weight_hh_l{layer}`,
/// `bias_ih_l{layer}`, `bias_hh_l{layer}` with `layer_idx = 0`), and the six
/// `head.*` names are [`PARAM_NAMES`] under `vb.pp("head")`. Gate rows inside
/// the `4H`-tall `weight_ih`/`weight_hh`/biases are candle's chunk order,
/// `i, f, g, o`.
pub const PARAM_NAMES_LSTM: [&str; 10] = [
    "lstm.weight_ih_l0",
    "lstm.weight_hh_l0",
    "lstm.bias_ih_l0",
    "lstm.bias_hh_l0",
    "head.l1.weight",
    "head.l1.bias",
    "head.l2.weight",
    "head.l2.bias",
    "head.l3.weight",
    "head.l3.bias",
];

/// A single-layer LSTM over the auction tokens whose final hidden state is
/// concatenated with the flat feature vector and fed to the existing [`Mlp`].
///
/// The LSTM is a pure auction encoder: nothing but the token sequence reaches
/// it, so the head keeps the v6 static block verbatim in its last
/// `features_len` input columns and the crate's fold script can still zero the
/// constant ones.
pub struct LstmPolicy {
    lstm: LSTM,
    head: Mlp,
    hidden_lstm: usize,
}

impl LstmPolicy {
    /// Build the encoder + head, registering variables under `vb` as
    /// [`PARAM_NAMES_LSTM`]. `features_len` is the dump's flat feature width —
    /// read from the sidecar, never assumed.
    pub fn new(
        token_len: usize,
        hidden_lstm: usize,
        features_len: usize,
        hidden: usize,
        out_dim: usize,
        dd_dim: usize,
        vb: VarBuilder,
    ) -> Result<Self> {
        Ok(Self {
            lstm: candle_nn::lstm(token_len, hidden_lstm, LSTMConfig::default(), vb.pp("lstm"))?,
            head: Mlp::new(
                hidden_lstm + features_len,
                hidden,
                out_dim,
                dd_dim,
                vb.pp("head"),
            )?,
            hidden_lstm,
        })
    }

    /// `x` is `(batch, features_len)`, `seq` is `(batch, steps, token_len)`
    /// right-padded with zero tokens, and `len` is `(batch,)` u32 — how many
    /// leading tokens are real.
    ///
    /// Padding is handled by *reading the state back out* rather than by
    /// masking: the per-step hidden states get a zero state prepended, so
    /// index `len` is the state after exactly `len` real steps and `len == 0`
    /// selects the zero state (an empty auction). Left-padding instead would be
    /// wrong once the biases train — a zero input still moves the state, since
    /// `c' = σ(b_i) ∘ tanh(b_g) ≠ 0`.
    pub fn forward(
        &self,
        x: &Tensor,
        seq: &Tensor,
        len: &Tensor,
    ) -> Result<(Tensor, Option<Tensor>)> {
        let batch = seq.dim(0)?;
        let states = self.lstm.seq(seq)?;
        let hs = self.lstm.states_to_tensor(&states)?; // (B, S, H)
        let zero = Tensor::zeros((batch, 1, self.hidden_lstm), hs.dtype(), hs.device())?;
        let padded = Tensor::cat(&[&zero, &hs], 1)?; // (B, S+1, H)
        // `gather` wants an index of the same rank as the source and returns the
        // index's shape, so the (B,) lengths become (B,1,H); the `contiguous`
        // is load-bearing (the CPU kernel rejects strided ids).
        let idx = len
            .reshape((batch, 1, 1))?
            .broadcast_as((batch, 1, self.hidden_lstm))?
            .contiguous()?;
        let h_last = padded.gather(&idx, 1)?.squeeze(1)?; // (B, H)
        self.head.forward(&Tensor::cat(&[&h_last, x], 1)?)
    }
}

/// Which policy the trainer is fitting.
///
/// The `Mlp` arm is the shipped v6 path and must stay byte-identical: it takes
/// the **root** `VarBuilder` (no `head.` prefix) and reports the un-prefixed
/// [`PARAM_NAMES`], so its `.f32` export and the crate's fixture parity are
/// untouched by the LSTM's existence.
pub enum Policy {
    Mlp(Mlp),
    Lstm(LstmPolicy),
}

impl Policy {
    /// `seq` is `(tokens, len)`; the `Mlp` arm ignores it, the `Lstm` arm
    /// requires it.
    pub fn forward(
        &self,
        x: &Tensor,
        seq: Option<(&Tensor, &Tensor)>,
    ) -> Result<(Tensor, Option<Tensor>)> {
        match self {
            Self::Mlp(mlp) => mlp.forward(x),
            Self::Lstm(lstm) => {
                let (tokens, len) = seq.ok_or_else(|| {
                    candle_core::Error::Msg(
                        "the LSTM policy needs an auction-token batch, but the dump has no .seq \
                         sibling"
                            .to_string(),
                    )
                })?;
                lstm.forward(x, tokens, len)
            }
        }
    }

    /// Export order for the weights blob and the sidecar's `param_order`.
    pub fn param_names(&self) -> &'static [&'static str] {
        match self {
            Self::Mlp(_) => &PARAM_NAMES,
            Self::Lstm(_) => &PARAM_NAMES_LSTM,
        }
    }
}
