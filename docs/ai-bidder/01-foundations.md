# Foundations: invariants, success metric, and representation

This is Part 0 (what the model must *be*) and Part 1 (how the world becomes
numbers). Get these right and Components A and B are "just" function fitting.

---

## Part 0 — Invariants

The model replaces the floor, so it inherits the floor's contract. Anything we
build must satisfy these, and the existing
[`instinct` tests](../../src/bidding/instinct.rs) become the regression suite.

1. **Drop-in type.** It is a `Classifier` (`classify(hand, &Context) -> Logits`)
   or a `System` (`classify(hand, vul, auction) -> Option<Logits>`). No new
   plumbing in the driver.
2. **Legality is the driver's job, but help it.** The floor may assign finite
   logits only to calls that *could* be legal; the driver already filters to
   legal calls and falls back to `Pass`. The model must never make `Pass`
   unavailable (keep a finite logit on `Pass`), so a distribution always exists.
3. **Floor-under-book precedence is preserved.** The model attaches as the root
   `Always` fallback, reached last by `Trie::resolve`. It answers only what the
   book didn't. (Later we may also let it *propose* and have the book *veto*,
   but not in Phase 1.)
4. **Safety properties the floor encodes must hold.** These are non-negotiable
   and are exactly the current tests:
   - never pass partner's live takeout double on a hand that can act
     (`forced_advance_never_passes`);
   - never pass below a forced game (`forced_to_game_*`);
   - complete partner's transfer opposite our notrump
     (`completes_partners_transfer_over_notrump`);
   - sit for our own penalty double (`forced_game_steps_aside_when_penalizing`);
   - never double our own side (`doubles_only_their_live_bids`).

   A learned model has no built-in respect for these. **We enforce them by
   construction**, not by hoping the net learns them: the model's logits are
   *masked/overridden* by a thin deterministic shell for the handful of forced
   situations the floor already detects (`forced_advance`, `auction_forces_game`,
   …). The net handles the vast "judgement" middle; the shell guards the rails.
   This is the single most important design stance for trust.
5. **Determinism in tests.** Inference must be reproducible: fixed weights, no
   RNG in the forward pass. (Sampling, if used by a driver, is the driver's RNG,
   not the model's.)

### Success metric

The same yardstick the floor was measured on: the **A/B duplicate match** in
[`examples/instinct-floor`](../../examples/instinct-floor/main.rs). Bid every
board twice with seats swapped, score divergent boards double-dummy, credit the
swing in **IMPs/board**.

- **Baseline to beat:** the current `instinct()` floor, worth ≈ **+0.5
  IMPs/board** over bare books (the recorded measurement).
- **Phase 1 (distill) target:** *parity* with the current floor (≈ 0.0
  IMPs/board *against it*, while keeping the +0.5 against bare books). A clone
  that matches the teacher proves the pipeline end-to-end.
- **Phase 2 (search) target:** strictly positive IMPs/board *against the current
  floor*. This is the point of the whole exercise.

Always report with a confidence interval over boards (the swing per board is
noisy; a few hundred boards has a wide error bar). Treat sub-0.1 IMPs/board
differences as noise unless the board count is large.

---

## Part 1 — Representation: turning the world into numbers

A network is `f_θ : ℝⁿ → ℝ³⁸`. We must define the input vector `x ∈ ℝⁿ` (the
*features*) and, for training, the target. Three things get encoded: the **hand**,
the **auction**, and (for Component A's runtime mode) the **descriptions**.

### 1a. The hand

A hand is 13 of 52 cards: a subset, i.e. a 52-bit mask. You already pack
holdings per suit in `contract-bridge`. Two encodings, both cheap:

- **Flat 52-bit indicator.** `x_card ∈ {0,1}⁵²`, one bit per (suit, rank). Simple,
  loses no information. Downside: the network must *learn* that the four suits
  play symmetric roles (mostly).
- **Per-suit, suit-exchangeable (recommended).** Encode each suit as a small
  feature vector — its 13-bit rank mask plus cheap derived scalars (length, top
  honors, HCP, a stopper bit) — giving a `4 × d` matrix. Then process the four
  suit-vectors with **one shared** per-suit function and pool. This is **Deep
  Sets**: bake in the symmetry "relabeling the suits relabels the output"
  instead of making the net rediscover it from data.

> **Why not a CNN?** A CNN slides one filter across positions and assumes
> *translation invariance* — pattern at position `k` means the same at `k+1`.
> True for pixels, false for ranks: an Ace is categorically special, not "a Two
> shifted up two ranks". The real symmetry in a hand is **suit exchangeability**
> (a permutation symmetry), which Deep Sets captures exactly and convolution
> does not. This is why the answer to your "CNN?" is "no — but there *is* a
> symmetry worth exploiting, just a different one."

A subtlety: suits are *not fully* exchangeable in bidding — majors outrank
minors, and strain ordering matters for legality. So the suit encoder is shared,
but we append a small per-suit identity feature (is-major, strain rank) so the
net can break the symmetry where the rules do. Equivariance where it holds,
explicit features where it doesn't.

Strength evaluators you already trust (HCP, `points`/`upgrade`, `fifths`,
`cccc`, `NLTC`) make excellent **hand-crafted features** appended to the raw
bits. A net *can* learn HCP from raw cards, but handing it a feature it would
otherwise spend capacity rediscovering is free accuracy — and it ties the model
to the same strength scale the books use.

### 1b. The auction

The auction is a variable-length sequence of `Call`s (vocabulary of 38, the same
`encode_call` indexing in `array.rs`). Two encodings:

- **Summary features (Phase 1, recommended start).** Reuse what already exists:
  the [`Context`](../../src/bidding/context.rs) facts (who bid which strains, the
  contract to beat, penalty state, passed-hand, seat, vulnerability) **and** the
  [`Inferences`](../../src/bidding/inference.rs) per-player shown ranges (4 suits ×
  {min,max} length + {min,max} points, per relative seat). This is a *fixed-size*
  vector — no sequence model needed. It already contains most of what the floor
  reasons over, so a small MLP on it can imitate the floor well.
- **Token sequence (Phase 2+ / Component A runtime).** The literal sequence of
  calls as tokens, fed to a small sequence model (a transformer). Strictly more
  information than the summary (it sees order and exact path), and it is the
  natural place to inject Component A's per-call meaning embeddings.

**Start with summary features.** They turn the whole problem into "fit an MLP to
a fixed-size vector", which is the simplest possible ML setup and reuses
battle-tested code. Graduate to tokens only when the summary's information loss
is the thing holding back accuracy.

Vulnerability is two bits relative to the side to act (we / they vulnerable),
already computed by `context::relative`. Seat / role is implied by auction
parity; pass it explicitly as a feature.

### 1c. The training target

What does the net try to output? This is where the three training regimes differ
(see [`02-policy-net.md`](02-policy-net.md)). For the foundation:

- **Distillation target (Phase 1).** The teacher is the assembled
  `american()` system. For a sampled `(hand, auction)`, the target is the
  teacher's **full softmax distribution** over calls — not just its argmax. The
  net minimizes cross-entropy to that distribution. Matching the *distribution*
  (soft targets) transfers far more than matching the single best call: it
  teaches the net the teacher's *near-misses and mixed strategies*, which is most
  of the useful signal. This is the classic Hinton distillation result.
- **Search target (Phase 2).** For a sampled `(hand, auction)`, deal many full
  layouts consistent with it (constrained sampler, built on `Inferences`), play
  each candidate call's resulting contract double-dummy, and form a target
  distribution peaked on the highest-EV call(s). The net is trained toward that.

Both produce a target distribution over the same 38 calls; the architecture is
identical. Only the *teacher* changes (a fixed system vs a search procedure).

### 1d. The description corpus (Component A's prerequisite)

Component A reads "tags and descriptions from each call". That data must exist.
Proposed schema — one record per *node* (an auction prefix the book classifies):

```text
auction:      [P, 1NT, P, 2D]          # the trie key
call:         2H                        # the call this node's classifier favors
tags:         [transfer, completion, major]
description:  "Completing partner's red-suit transfer to hearts."
constraint:   (the Rust Constraint, or its explain() summary)
```

Bootstrapping sources, cheapest first:

1. **`Rules::explain()`** already names the winning rule per call. Pair it with a
   short per-rule label (a string we add next to each `rule(...)`), and the
   corpus generates itself from the books.
2. **Doc comments** on the `american` modules are dense, accurate prose — a
   ready-made description source, harvestable semi-automatically.

The corpus is small (hundreds–thousands of nodes), high-quality, and **bridge
data, not ML data** — squarely your domain. It was the first deliverable built
(M0.2) and unblocks both Component A's compiler (text↔constraint pairs to learn
from / test against) and its runtime encoder.

---

Next: [`02-policy-net.md`](02-policy-net.md) builds the policy net on this
representation.
</content>
