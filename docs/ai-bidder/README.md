# The AI instinct bidder

> A design effort, not yet code. Everything here is a plan. The crate is
> untouched until a milestone in [`plan.md`](plan.md) is explicitly started.

## The dream

Replace the deterministic [instinct floor](../../src/bidding/instinct.rs) — the
hand-built `Rules` ladder that answers every off-book auction — with a learned
model, in two cooperating halves:

1. **A description language model (Component A).** Each call in a system has a
   *meaning* in words ("15–17 balanced", "natural game-forcing, 5+ hearts",
   "takeout"). This half turns those meanings into something the machine can
   use — first as a *compiler* (English meaning → `Constraint`), later as a
   *runtime encoder* (the meaning of each prior call becomes context the policy
   reads, so one model can bid any system from its written notes).

2. **A hand→call policy net (Component B).** Given a hand and an auction, emit
   `Logits` over all 38 calls — exactly the floor's current job, exactly the
   floor's current output type. Learned, not hand-written.

## Why this codebase is unusually ready for it

- **The output is already an ML interface.** [`Logits`](../../src/bidding/array.rs)
  is "log odds … useful for machine learning" with `softmax` built in. A learned
  policy is a drop-in [`Classifier`](../../src/bidding/trie.rs) / `System` — same
  signature, same 38-way output. The seam exists today.
- **The self-improvement engine is half-built.**
  [`Inferences`](../../src/bidding/inference.rs) (per-player shown shape and
  strength, read from the calls) is explicit groundwork for *constrained
  sampling*: deal layouts consistent with an auction. Sampling + double-dummy
  cardplay → expected value is the training signal that lets a policy *exceed*
  the books rather than merely copy them.
- **The descriptions don't exist as data yet.** They live as Rust doc comments
  and constraint code. [`Rules::explain`](../../src/bidding/rules.rs) (the
  winning rule per call) is the hook toward generating them; the corpus is
  Component A's prerequisite.

## Decisions taken (2026-06-13)

| Fork | Choice | Consequence |
|------|--------|-------------|
| Policy training signal | **Distill, then search** | Phase 1 clones the current 2/1 system for a fast, measurable drop-in floor; Phase 2 adds constrained-sampling + double-dummy search to beat the teacher. |
| Description LM role | **Both, sequenced** | An *authoring compiler* (English → `Constraint`) first; a *runtime meaning-encoder* later. |
| Where the model runs | **Offline train, distill to Rust** | Training happens off-crate with a real toolchain; only a distilled artifact (small weights, or an expanded table) ships, evaluated by a dependency-light forward pass. `pons` stays lean. |

## The one fact that makes "distill to Rust" the right call

**Training is hard; inference is arithmetic.**

- *Training* — finding the weights — needs automatic differentiation, a GPU is
  nice, and a mature toolchain (PyTorch / JAX / `burn` / `candle`). That part
  stays **outside** the crate.
- *Inference* — using the weights — is a handful of matrix multiplies and one
  elementwise nonlinearity. `z₁ = σ(W₁·x + b₁); z₂ = σ(W₂·z₁ + b₂); …`. That is
  a few `for` loops (or SIMD) over `f32` arrays, the kind of code you already
  write. It lands **inside** the crate with no heavy dependency.

So the model that replaces the floor is, at runtime, just a function from an
input array of hand+auction features to the 38-entry `Logits` array — the same
shape the floor returns now.

## ML, in terms you already own

You are strong in **bridge, math, and low-level programming** and new to ML.
This glossary maps each ML idea to one of those. Every design doc here uses
these framings.

| ML term | What it actually is |
|---------|---------------------|
| Neural network | A parameterized function `f_θ : ℝⁿ → ℝᵐ`. A function with numeric knobs `θ`. Nothing more mysterious. |
| Layer | An affine map then an elementwise nonlinearity: `x ↦ σ(W·x + b)`. `W` a matrix, `b` a vector, `σ` e.g. `ReLU(t)=max(0,t)`. |
| Weights / parameters | The numbers in the `W`s and `b`s. Found by training, frozen at inference. |
| Forward pass / inference | Evaluate `f_θ`: matmuls + `σ`. Pure arithmetic; hand-writable in Rust. |
| Training | Pick `θ` to minimize a scalar loss `L(θ)` by gradient descent: `θ ← θ − η·∇L`. Needs autodiff to get `∇L`. The only part needing a toolchain. |
| Loss | A scalar "how wrong". For classification, cross-entropy. |
| Softmax | `pᵢ = e^{zᵢ} / Σⱼ e^{zⱼ}`. Logits → probabilities. **You already have this** (`Logits::softmax`). |
| Cross-entropy / KL | A distance between two distributions: `H(p,q) = −Σ pᵢ log qᵢ`; `KL(p‖q)=Σ pᵢ log(pᵢ/qᵢ)`. Distillation minimizes it. |
| Logits | Pre-softmax log-odds scores. **Exactly your `Logits` type.** |
| Embedding | A lookup table: discrete token → trainable vector in `ℝᵈ`. Like `encode_call` in `array.rs`, but the cell holds a learned vector, not a count. |
| Tokenization | Splitting text into discrete units that index the embedding table. |
| Attention (transformer) | For each position, a weighted average of all positions' vectors; the weights are `softmax` of learned dot-products. A differentiable, content-addressable lookup. |
| Deep Sets / equivariance | If the input is a set (the 4 suits are exchangeable), apply one shared per-element function, then pool (sum/mean). Bakes the symmetry into the architecture. |
| CNN | A small filter slid across positions; assumes *translation invariance*. Bad fit for card ranks (an Ace is not "a Two shifted up"). |
| Distillation | Train a fast "student" to copy a "teacher"'s output distribution. Here: student net copies `two_over_one()`'s softmax. |
| Policy | A function: state → distribution over actions. **Your floor is already a (deterministic) policy.** The net is a learned one. |
| Policy improvement / search | Use a slow accurate evaluator (DD over sampled layouts) to score each candidate call, then nudge the policy toward the higher-EV one. Iterate. Run it **at training time** (to make targets) *and* **at play time** (net+search beats the raw net). |
| Prior policy | The cheap policy (the net's softmax) used to *propose* which calls are worth evaluating — search only the top-`k`. "Net proposes, search disposes." |
| Rollout | Play the auction out to a contract under the current policy, then score that contract double-dummy on one sampled layout. Average rollouts → a call's EV. |
| Test-time / inference-time search | Running the policy-improvement operator *at the table*, not only during training. The reason a slow, gated "thinking" bidder beats the fast one-matmul floor. |
| Self-play | The system generates its own training auctions by bidding against itself, scored by the evaluator. |
| Temperature / calibration | Scaling logits before softmax. The books use a ~3-nat gap convention; the net must match that scale. |
| Overfitting / generalization | Memorizing noise vs learning signal. Held-out boards measure the difference. |

## Document map

- [`01-foundations.md`](01-foundations.md) — invariants the model must honor, the
  success metric, and how a hand + auction + the descriptions become numbers
  (the representation).
- [`02-policy-net.md`](02-policy-net.md) — Component B: an ML primer, the network
  architecture, and the distill-then-search training plan.
- [`03-description-lm.md`](03-description-lm.md) — Component A: the corpus, the
  authoring compiler, and the later runtime meaning-encoder.
- [`04-integration-and-eval.md`](04-integration-and-eval.md) — how the distilled
  model runs inside `pons`, and how every milestone is measured on the existing
  A/B IMPs harness.
- [`plan.md`](plan.md) — the phased roadmap: small, well-specified, individually
  measurable chunks.
</content>
</invoke>
