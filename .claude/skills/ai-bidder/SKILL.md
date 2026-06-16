---
name: ai-bidder
description: >
  Advance or resume the "AI instinct bidder" effort in pons — replacing the
  deterministic bidding floor with a learned model (a hand→call policy net and a
  description language model). Use when the user asks to work on the neural/AI
  bidder, the policy net, distillation, the constrained sampler, the description
  compiler/corpus, or references docs/ai-bidder/. Loads the design, respects the
  invariants, and teaches the ML grounded in math/bridge/low-level.
---

# AI instinct bidder — operating manual

The full design lives in [`docs/ai-bidder/`](../../../docs/ai-bidder/). Read
`README.md` first (the dream, the decisions, the ML glossary), then the part
relevant to the task. `plan.md` is the milestone map and the source of truth for
"what's next".

## Standing decisions (do not re-litigate without the user)

- **Distill, then search** — clone `two_over_one()` first (M1), beat it with
  constrained-sampling + double-dummy search second (M3).
- **Search runs at inference time too** — not only as an offline teacher. M2.3 is
  a gated, slow "thinking" bidder (`two_over_one_search()`) that simulates before
  it bids; **bidding only** (no cardplay engine). Distillation (M3) stays the path
  to the *fast (distilled)* floor; the live search bidder is an added gated option
  and the teacher whose EVs M3 distills. (`instinct()` stays the baseline; both
  learned floors are added options, never a removal.)
- **Both LM roles, sequenced** — authoring compiler (M4) before runtime
  meaning-encoder (M5).
- **Offline train, distill to Rust** — training is off-crate; only a lean
  weights artifact + a hand-rolled forward pass ship, feature-gated. `pons` stays
  dependency-light.

## How to work here

1. **Find the next unblocked chunk** in `plan.md` (start at M0; respect the
   `deps`). Confirm with the user before starting a milestone — the docs are a
   map, not a green light.
2. **Honor the invariants** in `01-foundations.md §0`. The learned floor is
   wrapped in a deterministic legality + forced-situation shell; the `instinct`
   test suite is the rails and must stay green. The net is trusted for judgement,
   never for the forced rails.
3. **Measure on the existing harness.** Every milestone's win condition is
   IMPs/board on the A/B duplicate match (`examples/instinct-floor` is the
   template), *not* training loss. Report board counts; treat sub-0.1 IMPs/board
   as noise unless the sample is large.
4. **Keep the baseline.** `instinct()` stays as default and comparison anchor; a
   neural floor is an added option (e.g. `two_over_one_neural()`), never a
   removal.
5. **Version artifacts together** — weights, feature-spec version, teacher/system
   version, data seed, git SHA. A model is meaningless without its exact feature
   extractor.

## Laziest thing that works (ponytail)

This effort is where over-engineering breeds — speculative abstractions, heavy
ML runtimes, training infra, a node authored per artificial bid. Stop at the
first rung that holds:

1. **Does it need to exist?** YAGNI. Smarten the keyless instinct floor
   (forced-to-game, transfer-completion) and floor the book; do **not** author a
   node per artificial bid. A milestone the user hasn't chosen is not work yet.
2. **Reuse the seam.** The output is already an ML interface — `Logits`,
   `Classifier`/`System`, `Inferences`, `Rules::explain`. Wire to those before
   writing new machinery.
3. **Inference is arithmetic.** A forward pass is a few matmuls + one
   nonlinearity, hand-rolled in Rust. Training (autodiff) stays off-crate; never
   a heavy ML dep in the default build.
4. **Shortest diff that moves IMPs/board.** Deletion over addition. Floor the
   book, don't replace it; `instinct()` is never removed.
5. Mark a deliberate shortcut with a `// ponytail:` comment naming the ceiling
   and the upgrade path.

## Teaching stance (important)

The user is expert in **bridge, math, and low-level programming**, and is
learning **ML**. When introducing any ML idea, ground it in those: a net is a
parameterized function fit by gradient descent; inference is matmuls + an
elementwise nonlinearity (hand-writable in Rust); softmax/logits are already in
the codebase; distillation is matching a teacher's distribution (cross-entropy);
embeddings are `encode_call`-style lookups with learned vectors. Prefer the
math/systems framing over ML jargon. The glossary in `README.md` is the
reference; extend it when a new term comes up.

## Divide and delegate

Mechanical chunks (data export, feature plumbing, harness variants, verification
harnesses) suit focused subagents. Keep architecture, the safety shell, ML target
design, and integration decisions in the main loop.

## Guardrail

The ponytail ladder, made non-negotiable: do **not** add a heavy ML runtime to
the crate's default build, and do not write crate ML code before its milestone is
explicitly chosen. Design docs may contain illustrative sketches; the crate stays
untouched until a milestone starts.
</content>
