# Integration and evaluation

How a model trained off-crate runs *inside* `pons`, and how every step is
measured. Decision taken: **offline train, distill to Rust**.

---

## Part 4 — Running the model in `pons`

### The shape of the problem

`pons` is a lean, pure-Rust crate. We do **not** want a heavy ML runtime as a
dependency of the library. The decision "distill to Rust" means: the training
toolchain stays outside; what lands in the crate is a small, self-contained
**forward pass** plus a **weights artifact**.

Recall the load-bearing fact: **inference is arithmetic.** For the Phase-1 MLP,
running the model is:

```text
# illustrative — NOT crate code yet
fn classify(features: &[f32]) -> Logits {
    let h1 = relu(affine(&W1, &b1, features));   // matmul + bias + max(0,·)
    let h2 = relu(affine(&W2, &b2, &h1));
    let z  =      affine(&W3, &b3, &h2);          // 38 logits
    Logits(Array::from_fn(|call| z[encode_call(call)]))
}
```

`affine` is a matrix-vector product and a bias add — a few nested loops over
`f32` slices, or SIMD if it ever matters (it won't; the model is tiny). No
dependency, fully deterministic, trivially testable. This is the entire runtime
cost of "AI in the crate" for Phase 1.

### Three ways to ship the distilled model, lightest first

1. **Expanded table / rules (lightest).** If the learned floor is effectively a
   lookup over a modest set of `(context-bucket → logits)`, distill it *all the
   way* into data: a generated `Rules` ladder or a static table the existing
   machinery already understands. Zero new runtime code, zero new concepts —
   just more (machine-authored, human-reviewed) rules. Viable when the policy is
   simple enough; check by measuring the table against the net.
2. **Weights + handwritten forward pass (recommended default).** Ship the weight
   matrices as a `const`/embedded artifact (or a small loader) and the ~30 lines
   of forward-pass arithmetic above, behind a feature flag (`ml` / `neural-floor`)
   so the default build stays exactly as lean as today. This is the sweet spot:
   full model fidelity, dependency-free, and the forward pass is code you are
   comfortable owning and verifying.
3. **A Rust ML backend (only if the model outgrows hand-rolling).** `candle` /
   `burn` / `tract` / `ort` can load and run the model, feature-gated. Reserve
   this for the Phase-2 transformer if its forward pass becomes inconvenient to
   maintain by hand. Adds a real dependency; opt-in only.

Start at 2 (or 1 if it suffices). Escalate to 3 only when forced.

### Format and reproducibility

- Export weights in a boring, stable format (raw `f32` little-endian, or `safetensors`).
- Record, alongside the artifact: the feature spec version, the teacher/system
  version, the training data seed/count, and the git SHA. A distilled model is
  only meaningful paired with the exact feature extraction that produced its
  inputs — version them together.
- The forward pass must be **bit-reproducible** across runs for the test suite.
  Pin the arithmetic (no fast-math reordering surprises that change argmax).

### Wiring it in

The model attaches exactly where `instinct()` does: as the root `Always`
fallback, behind the book, wrapped in the legality+safety shell from
[`02-policy-net.md`](02-policy-net.md). A new constructor (say
`two_over_one_neural()`) mirrors `two_over_one()` but swaps the floor. The
deterministic `instinct()` stays as the default and the comparison baseline —
nothing is removed, a new option is added.

---

## Part 5 — Evaluation

### The harness already exists

[`examples/instinct-floor`](../../examples/instinct-floor/main.rs) is the
template: A/B duplicate match, divergent boards solved double-dummy, swing
credited in **IMPs/board**, plus floor-activation telemetry (where the floor
fires, which off-book auctions are most common). Every milestone reuses this
pattern. Minimal new work: a variant that pits the *neural* floor against the
*deterministic* floor (and against bare books).

### What to measure each milestone

- **IMPs/board** of the candidate floor vs (a) the current `instinct()` floor and
  (b) bare books — with a board-count and a rough confidence interval. The win
  condition lives here, not in the training loss.
- **Telemetry:** does the neural floor fire on the same auctions? Where does it
  *diverge* from the deterministic floor, and is the divergence a gain or a loss?
  The most-divergent auctions are the cases to inspect by hand.
- **Safety regressions:** the `instinct` test suite, re-run against the neural
  floor through its shell. These must stay green — they are the rails.

### Guardrails specific to a learned component

- **Hand-auditable divergences.** When the neural floor and the deterministic
  floor disagree on a board that swung, surface the hand + auction + both calls.
  A learned model's mistakes are not in the source code, so the telemetry *is*
  the source of truth for debugging.
- **No silent regression past a milestone.** A Phase-2 search round that loses
  IMPs/board against the prior net is rejected. Keep every net's measured score;
  never replace a champion with a challenger that didn't beat it on the harness.
- **Calibration drift.** Watch that the neural floor's decisiveness (logit scale
  / temperature) stays sane; an overconfident floor that's wrong is worse than a
  hedged floor that's wrong, because the driver takes argmax.

### A note on statistical honesty

IMPs/board on a few hundred random boards is *noisy* — most boards don't diverge,
and divergent ones swing a lot. Report board counts, prefer thousands of boards
for a headline number, and treat sub-0.1 IMPs/board as noise unless the sample is
large. The current floor's "+0.5 IMPs/board" is the bar; clearing it convincingly
needs enough boards that the interval excludes zero.

---

That completes the design. The execution order is in [`plan.md`](plan.md).
</content>
