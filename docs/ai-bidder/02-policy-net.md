# Component B: the hand→call policy net

The half that does the floor's actual job: `(hand, auction) → Logits` over 38
calls, learned instead of hand-written.

> Status (2026-08-10): built, twice over. Phase 1 executed with a different
> teacher — the shipped floors hard-clone the BBA/EPBot oracle (current: v5,
> [`card-manifold.md`](card-manifold.md)), not `american()` self-distillation.
> Phase 2's search was built as M2/M3 (`american_search`, `search_floor.rs`)
> and then **deleted** in the variant tidy-up
> ([`archive/sound-search.md`](archive/sound-search.md)); re-deriving it is
> M8's prerequisite. The architecture and safety-shell design below remain the
> reference.

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

> **Measured 2026-09-03 (M5.2): built, refuted 3/3 arms, parked.** Two
> corrections to the paragraph above. It was built as a *recurrence*, not a
> transformer — at `T ≤ 20` steps attention buys nothing the LSTM's two gemvs
> per step do not. And the **adoption trigger was wrong**: it asks whether the
> MLP bottlenecks on *fidelity*, but the sequence model carried strictly more
> information than the MLP and still lost IMPs, because it spent the extra
> capacity on *looser* bidding (doubled 16.9% vs 11.8%) rather than better
> bidding. Fidelity was never the binding constraint — bidding discipline was.
> Numbers and flip plan in [`plan.md`](plan.md) M5.2.

### Output calibration

The net inherits **no** teacher scale: every shipped floor since 2026-07-19 is
trained on one-hot BBA labels (the README glossary's Distillation row), so its
raw logits sit wherever cross-entropy leaves them. The books' "~3-nat gap"
(`rules.rs`) is a claim about three sites, not a scale: argmax resolves
overlapping rules by order at any positive gap, the shipped rungs sit 0.05-0.5
nat apart, and nothing on the default path reads a book magnitude as odds. The
one scalar that gives the net's softmax a meaning is a temperature `T`
(divide logits by `T` before softmax), **fitted** on held-out NLL — temperature
scaling, Guo et al. 2017. It is argmax-invariant, so serving stays raw; only
the search's proposal odds and any display move. It was never built — session
2 of [logit-calibration.md](logit-calibration.md) owes it.

### The legality + safety shell (restating the key invariant)

The shell from [invariants §0.4](01-foundations.md#part-0--invariants) wraps the
net's 38 logits: **mask** illegal calls to `−∞`, and **override** the forced
situations the floor detects (`forced_advance`, `auction_forces_game`, transfer
completion, penalty sitting), *replacing or flooring* the net's logits with the
safe action. Small, deterministic, testable — the net is trusted for judgement,
never for the rails. It lets a model replace the floor without replacing its
*guarantees*.

---

## Training plan: distill, then search

### Phase 1 — Distillation (clone the current system)

**Goal:** a learned floor that *matches* `american()`, proving the entire
pipeline (features → train → distill to Rust → measure on the A/B harness) before
any attempt to beat it.

1. **Generate data.** Deal random boards; bid them out with the real
   `american()` system; at every decision point record
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

The distilled net (Phase 1) is the *raw policy*: one forward pass, no lookahead —
fast, but it commits to its first instinct. It "bids too fast." **Net + search is
the strong policy.** Search here is **one step of policy improvement**, the core
loop behind AlphaZero-style systems — and, as in AlphaZero, it is run **both at
training time and at play time**, not only to make training data. There is no deep
tree search: a bridge auction is short and the expensive part is the hidden-hand
uncertainty, not depth.

The engine — the same operator in both uses:

1. **Prior → shortlist.** The net's softmax proposes the plausible calls. Search
   only the top-`k` (it would waste a DD solve on a call the policy already knows
   is absurd). *Net proposes, search disposes.*
2. **Constrained sampling.** Given `(hand, auction)`, deal many full layouts for
   the other three hands *consistent with the auction* — every player's cards
   fall within the `Inferences` ranges their calls promised. (This is the future
   sampler the inference module was built for; it is a milestone in
   [`plan.md`](plan.md).)
3. **Evaluate each candidate call.** For each shortlisted call `c`, continue the
   auction (opponents and partner bidding via the current policy), reach a
   contract, and score it **double-dummy** (you already solve DD). Average over the
   sampled layouts → an EV for `c`. Single-dummy / Monte-Carlo cardplay is the more
   honest but pricier evaluator; DD is the practical start.
4. **Form an improved distribution.** A distribution peaked on the high-EV calls
   (softmax of EVs at some temperature, or the argmax with a margin). By
   construction it is *at least as good as* the current policy at this state — the
   policy-improvement theorem in plain terms.

The same four steps, used two ways:

#### As a runtime player (the "thinking" bidder)

Wrap steps 1–4 as a drop-in `Classifier`/`Bidder`, behind a `search` cargo
feature, and return the improved distribution directly. This *is* the policy at
the table: it simulates before it bids. It ships gated and slow on purpose —
strength over latency — and is the strongest *bidding* player we can field. The
deterministic forced-rails shell wraps it exactly as it wraps the bare net (see
below): the rails are never searched. Scope is **bidding only**; Monte-Carlo
cardplay is a separate, larger effort (no cardplay policy exists in `pons` yet)
and is out of scope here.

#### As an offline teacher (the path to the *fast* floor)

Take the improved distribution as a training target and **distill toward it**,
exactly like Phase 1 but with the search target replacing the teacher softmax.
Then **iterate**: the improved net becomes the policy used inside step 3's
continuations next round (self-play), the targets get a little better, and so on.
This bakes the search player's strength back into a single forward pass, so the
**fast (distilled) floor stays one matmul stack** and needs no runtime search —
the gated search player remains available when maximum strength is worth the wait.
Distillation, not the runtime player, remains the path to the fast floor.
(`instinct()` stays the untouched baseline; both learned floors are added
options.)

**Cost, and the one efficiency that makes it affordable.** Step 3 reads as "many
layouts × many calls × a DD solve each", but the DD solves are *shared*: solve
each sampled layout **once** with all strains (`NonEmptyStrainFlags::ALL`) and its
`TrickCountTable` scores *any* final contract×declarer on that exact layout. So
cost is **`n` DD solves total, not `k·n`** — plus `k·n` *cheap* continuation
auctions (matmuls). Budget the rest: search only at *decision points that matter*
(off-book, contested; forced nodes delegate to `instinct()` for free), cap `k` and
`n`, and cache.

**Risk.** The loop can chase double-dummy artifacts — DD is a clairvoyant
evaluator and rewards lines no human could find at the table. Mitigations:
single-dummy evaluation later, entropy via the EV temperature (don't let the
policy collapse to overconfident lines), and the A/B harness as the ground-truth
arbiter — for the runtime player *and* every distillation round. If a search
config or a retrained net *loses* IMPs/board against its predecessor, it is
rejected. The harness is the judge, not the training loss.

---

Next: [`03-description-lm.md`](03-description-lm.md) for Component A, or
[`04-integration-and-eval.md`](04-integration-and-eval.md) for how this net ships
and is measured.

## Ledger (memory compaction, 2026-08-16)

- **Policy `upgrade` ablation, round 1 (2026-07-24): keep the fused scalar.**
  Dropping feature 88 (`features_v3` 88→87) scored plain **−0.027/−0.056** and
  PD **−0.136/−0.150** NV/vul against the shipped net; a fresh 88-wide control
  scored plain **−0.019/−0.029**, PD **−0.035/−0.046**, isolating ≈**−0.10 PD**
  to the feature. The policy suit block `(len, suit_hcp)` cannot distinguish QJ
  doubleton from Kx, while `upgrade` carries that wasted-honour texture. Near-
  identical top-1/CE did not predict the IMP loss.
- **Round 2: decomposing the scalar was worse, not richer.** At `b089f05`, the
  91-wide `wasted×4` arm scored **−0.040/−0.115** NV and **−0.066/−0.138** vul
  (plain/PD). Adding `big_two_suiter` in the 92-wide arm lost across all four
  GPU-seeded draws: means **−0.050/−0.154** NV and **−0.089/−0.180** vul. For a
  small distiller, the teacher-aligned `upgrade/2` is better conditioned than
  asking the net to relearn its gated nonlinear formula. `--init-seed` fixed
  the earlier unseeded ≈0.02–0.05 IMPs/board retrain variance.
- **Fidelity is a filter, never a predictor (second instance, 2026-09-03).**
  Round 1 saw near-identical top-1/CE fail to predict an IMP loss. M5.2 saw the
  converse and worse: across its three arms, IMPs moved *opposite* to fidelity —
  the highest-fidelity arm (0.2808 CE / 90.6% top-1) lost the least PD and the
  reweighted arms lost more as their CE rose. Distillation fidelity bounds how
  well a net can imitate the teacher; it says nothing about whether imitating
  the teacher scores. Use it to reject an arm that failed to train, never to
  rank arms that did.
- **An outcome-over-par advantage rewards the opponents' mistakes
  (2026-09-03).** `A = imps(result − par)` looks like a policy advantage and is
  not one: a deal beats par mostly when the *other side* misbids, so `exp(β·A)`
  upweights rows whose merit is not ours. It was correctly PD-aware and had real
  force (sd 0.66) and still degraded the A/B monotonically. An advantage needs a
  baseline that holds the deal fixed and varies only the policy — for us, the
  control arm's own auction on the same deal.
</content>
