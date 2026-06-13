# Component B: the hand→call policy net

The half that does the floor's actual job: `(hand, auction) → Logits` over 38
calls, learned instead of hand-written.

---

## A 5-minute ML primer, in your terms

Skip if comfortable. Otherwise, the whole of "training a classifier" is:

1. **A model is a function with knobs.** `f_θ : ℝⁿ → ℝ³⁸`. Input `x` is the
   feature vector from [`01-foundations.md`](01-foundations.md); output is 38
   logits — *the same `Logits` you already produce*. `θ` is a big pile of `f32`s
   (the weight matrices and biases).

2. **The function is layers of `σ(W·x + b)`.** Concretely, an MLP
   ("multi-layer perceptron"):

   ```text
   h₁ = ReLU(W₁·x  + b₁)     # W₁ : (128 × n),  ReLU(t)=max(0,t)
   h₂ = ReLU(W₂·h₁ + b₂)     # W₂ : (128 × 128)
   z  =      W₃·h₂ + b₃      # W₃ : (38 × 128)  → the 38 logits
   ```

   Inference is exactly those three lines: matmuls and a `max(0,·)`. Hand-writable
   in Rust. The nonlinearity `σ` is what lets a stack of linear maps represent
   something other than one linear map — without it, `f_θ` collapses to a single
   matrix.

3. **Training picks `θ` to minimize a loss.** Define `L(θ)` = average over
   training examples of how wrong `f_θ` is. For us, *cross-entropy* between the
   teacher's distribution `p` and the net's `q = softmax(z)`:
   `L = −Σᵢ pᵢ log qᵢ`. It is zero iff `q = p`.

4. **Gradient descent.** Compute `∇_θ L` (the direction in knob-space that most
   reduces `L`) and step `θ ← θ − η·∇L`. Repeat over the data many times. The
   gradient comes from **automatic differentiation** — the toolchain's one piece
   of real magic, and the reason training lives off-crate. Think of it as the
   chain rule, applied mechanically to the computation graph.

That's it. No step is conceptually beyond calculus + linear algebra you have.
The toolchain (PyTorch / JAX / `burn`) automates step 4; we own steps 1–3.

---

## Architecture

Start small. The floor it replaces is a few hundred rules; the net does not need
to be large to match it.

### Phase 1 — MLP on summary features (the recommended start)

- **Input:** the fixed-size vector from §1a+§1b of foundations — suit-exchangeable
  hand features + `Context`/`Inferences` summary + vulnerability + seat. A few
  hundred floats.
- **Body:** 2–3 hidden layers, width 128–256, ReLU. This is a *tiny* model
  (tens of thousands of parameters), trains in minutes on a CPU, and its forward
  pass is a handful of small matmuls — trivial to ship to Rust.
- **Head:** a linear layer to 38 logits.
- **Suit symmetry:** apply the shared per-suit encoder (Deep Sets) to the four
  suit-vectors, pool (concatenate the pooled sum/max with the global features),
  then the MLP body. This is the only "non-obvious" architectural piece and it
  is optional — a plain MLP on flat features works too, just less
  sample-efficiently.

### Phase 2+ — sequence model (only if needed)

A small transformer over the literal call sequence (with Component A's meaning
embeddings, see [`03-description-lm.md`](03-description-lm.md)). More capacity,
more information (order/path), and the substrate for cross-system play. Adopt it
when the summary-feature MLP demonstrably bottlenecks — not before. Bigger model,
bigger Rust forward pass, more ways to be wrong.

### Output calibration

The books speak in logits where a ~3-nat gap is "near-deterministic after
softmax" (see `rules.rs`). The net's raw logits will be on whatever scale
training lands. Two fixes:

- For **distillation**, the net learns the teacher's *distribution*, so it
  inherits the teacher's scale automatically — no extra work.
- Keep a **temperature** scalar `T` (divide logits by `T` before softmax) as a
  single post-hoc knob, tuned on held-out boards, so the floor's
  decisiveness/mixing matches what the driver expects.

### The legality + safety shell (restating the key invariant)

The net outputs 38 logits unconditionally. Wrap it:

1. **Mask** illegal calls to `−∞` (the driver does this already; do it in the
   shell too so the distribution is honest).
2. **Override** the handful of forced situations the floor detects deterministically
   (`forced_advance`, `auction_forces_game`, transfer completion, penalty
   sitting). In those states the shell *replaces or floors* the net's logits with
   the safe action. The net is trusted for judgement, never for the rails.

This shell is small, deterministic, testable, and is what lets us trust a learned
component. It is the reason a model can replace the floor without replacing the
floor's *guarantees*.

---

## Training plan: distill, then search

### Phase 1 — Distillation (clone the current system)

**Goal:** a learned floor that *matches* `two_over_one()`, proving the entire
pipeline (features → train → distill to Rust → measure on the A/B harness) before
any attempt to beat it.

1. **Generate data.** Deal random boards; bid them out with the real
   `two_over_one()` system; at every decision point record
   `(features, teacher_softmax)`. Millions of `(hand, auction)` examples are free
   — they're just simulation. *Weight* the sampling toward off-book auctions (the
   floor's actual domain), e.g. by oversampling competitive sequences, so the
   student spends its capacity where it will be used.
2. **Train** the MLP to minimize cross-entropy to `teacher_softmax`. Hold out a
   fraction of boards to measure generalization.
3. **Distill to Rust** (see [`04-integration-and-eval.md`](04-integration-and-eval.md)):
   export weights, evaluate the forward pass in-crate.
4. **Measure** on the A/B harness: distilled-floor pair vs current-floor pair.
   Success = parity (≈ 0 IMPs/board against the teacher) and the +0.5 against
   bare books preserved. This validates the machinery.

Why bother cloning if it can't beat the teacher? Because it de-risks everything
downstream: it proves the representation carries enough signal, the Rust forward
pass is correct (its logits should track the teacher's), and the harness wiring
works — *before* we introduce the much noisier search signal. It also yields a
fast, smooth, **sampleable** policy (the teacher is a hard `Rules` ladder; the
net is a calibrated distribution), which is itself useful as a sampling prior.

### Phase 2 — Search (beat the teacher)

**Goal:** improve the policy beyond the books in the off-book auctions, using
the cardplay truth the books never consulted.

The engine is **one step of policy improvement**, the core loop behind
AlphaZero-style systems, here without deep tree search because a bridge auction
is short and the expensive part is the hidden-hand uncertainty, not depth:

1. **Constrained sampling.** Given `(hand, auction)`, deal many full layouts for
   the other three hands *consistent with the auction* — every player's cards
   fall within the `Inferences` ranges their calls promised. (This is the future
   sampler the inference module was built for; it is a milestone in
   [`plan.md`](plan.md).)
2. **Evaluate each candidate call.** For each legal call `c`, continue the auction
   (opponents and partner bidding via the current policy), reach a contract, and
   score it **double-dummy** (you already solve DD). Average over the sampled
   layouts → an EV for `c`. Single-dummy / Monte-Carlo cardplay is the more honest
   but pricier evaluator; DD is the practical start.
3. **Form an improved target.** A distribution peaked on the high-EV calls (e.g.
   softmax of EVs at some temperature, or the argmax with a margin). This target
   is, by construction, *at least as good as* the current policy at this state —
   that is the policy-improvement theorem in plain terms.
4. **Train toward it**, exactly like distillation but with the search target
   replacing the teacher softmax. Then **iterate**: the improved net becomes the
   policy used inside step 2's continuations next round (self-play). Each round
   the evaluator is bidding with a slightly better policy, so the targets get
   slightly better, and so on.

**Honesty about cost and risk.** Step 2 is expensive (many layouts × many calls ×
a DD solve each). Budget it: sample only at *decision points that matter*
(off-book, contested), cap layouts, and cache. Risk: the loop can chase
double-dummy artifacts (DD is a clairvoyant evaluator and rewards lines no human
could find at the table). Mitigations — single-dummy evaluation, entropy
regularization (don't let the policy collapse to overconfident lines), and the
A/B harness as the ground-truth arbiter every iteration. If a round of search
*loses* IMPs/board against the previous net, that round is rejected. The harness
is the judge, not the training loss.

---

Next: [`03-description-lm.md`](03-description-lm.md) for Component A, or
[`04-integration-and-eval.md`](04-integration-and-eval.md) for how this net ships
and is measured.
</content>
