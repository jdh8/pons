# Phased plan

Small, well-specified, individually measurable chunks. Each milestone names a
**deliverable**, a **measure** (how we know it worked), and its **deps**. Chunks
are sized to be handed to a focused subagent where they're mechanical, and kept
in the main loop where they're design (per the "divide and delegate" working
style). **Nothing here is started until explicitly chosen** — this is the map,
not a green light.

Legend: ⬜ not started.

---

## Milestone 0 — Corpus + baseline lock-in (no ML)

The foundation. Pure bridge/Rust work; de-risks everything downstream.

- ⬜ **M0.1 Rule labels.** Add a short string label to each `rule(...)` in the
  books (or a parallel map). *Deliverable:* `explain()` can name a human-readable
  meaning per winning rule. *Measure:* every node `explain()`s to a non-empty
  label. *Deps:* none.
- ⬜ **M0.2 Corpus exporter.** A dev tool that walks the trie and emits the
  per-node corpus records `{auction, call, tags, description, constraint-summary}`
  (schema in [foundations §1d](01-foundations.md#1d-the-description-corpus-component-as-prerequisite)).
  *Deliverable:* a corpus file for the 2/1 system. *Measure:* record count ≈ node
  count; spot-check 20 records for accuracy. *Deps:* M0.1.
- ⬜ **M0.3 Feature extractor (spec + reference impl).** Define `features(hand,
  context) -> Vec<f32>` (foundations §1a–1b): suit-exchangeable hand block +
  `Context`/`Inferences` summary + vul + seat. *Deliverable:* a documented,
  versioned feature vector and a Rust function producing it. *Measure:* unit
  tests pin the layout; round-trips a few known hands/auctions. *Deps:* none.
- ⬜ **M0.4 Teacher dump.** Using the feature extractor, bid out random boards
  with `two_over_one()` and record `(features, teacher_softmax)` at each decision,
  oversampling off-book/contested auctions. *Deliverable:* a training dataset.
  *Measure:* dataset stats (size, off-book fraction, call-distribution sanity).
  *Deps:* M0.3.

Exit M0: we have a corpus, a versioned feature spec, and a teacher dataset —
without writing a line of ML.

---

## Milestone 1 — Distilled floor, end-to-end (Phase 1 of Component B)

Prove the whole pipeline by *cloning* the current system.

- ⬜ **M1.1 Train the MLP** (off-crate). Fit the summary-feature MLP
  ([policy-net Phase 1](02-policy-net.md#phase-1--mlp-on-summary-features-the-recommended-start))
  to the M0.4 dataset. *Deliverable:* a weights artifact + held-out cross-entropy.
  *Measure:* held-out top-1 agreement with the teacher (target high, e.g. >95% on
  on-book, lower but sane off-book). *Deps:* M0.4.
- ⬜ **M1.2 Rust forward pass** behind an `ml`/`neural-floor` feature flag
  ([integration Part 4](04-integration-and-eval.md#part-4--running-the-model-in-pons)).
  *Deliverable:* `classify(features) -> Logits` in-crate, dependency-free.
  *Measure:* its logits match the off-crate model bit-closely on a fixture set
  (the cross-language equivalence test). *Deps:* M1.1, M0.3.
- ⬜ **M1.3 Safety shell.** Wrap the net with the deterministic legality +
  forced-situation override ([invariants §0.4](01-foundations.md#part-0--invariants)).
  *Deliverable:* a `Classifier` safe to attach as the floor. *Measure:* the
  entire `instinct` test suite passes against the shelled net. *Deps:* M1.2.
- ⬜ **M1.4 A/B measurement.** A variant of the instinct-floor example: neural
  floor vs deterministic floor vs bare books. *Deliverable:* IMPs/board numbers.
  *Measure:* parity with the deterministic floor (≈ 0 IMPs/board against it) and
  +0.5 preserved vs bare books, over enough boards. *Deps:* M1.3.

Exit M1: a learned floor that *equals* the hand-written one, shipped lean, proven
on the harness. The machine now does the floor's job — not yet better.

---

## Milestone 2 — Constrained sampler (the search prerequisite)

The piece `Inferences` was built for; needed before any "beat the teacher" work.

- ⬜ **M2.1 Sampler.** Given `(auction)`, deal the other hands consistent with
  every player's `Inferences` ranges. *Deliverable:* `sample_layouts(context, n)`.
  *Measure:* soundness — every sampled hand falls within its shown ranges
  (property test); coverage — the dealt distribution isn't degenerate. *Deps:*
  none (builds on `Inferences`).
- ⬜ **M2.2 Call EV evaluator.** For a candidate call, continue the auction under
  the current policy over sampled layouts, reach a contract, score double-dummy,
  average. *Deliverable:* `ev(hand, context, call) -> f32`. *Measure:* sanity on
  known textbook decisions (it should prefer the obviously-right call). *Deps:*
  M2.1, the policy from M1.

Exit M2: we can ask "what is each call actually worth on this hand?" — the signal
the books never had.

---

## Milestone 3 — Search-improved floor (Phase 2 of Component B)

The point of the exercise.

- ⬜ **M3.1 Improvement targets.** Turn per-call EVs into a training target
  distribution ([policy-net Phase 2](02-policy-net.md#phase-2--search-beat-the-teacher)).
  *Deliverable:* a dataset of `(features, search_target)` over sampled decisions.
  *Measure:* targets differ from the teacher mainly off-book/contested (where the
  books were silent). *Deps:* M2.2.
- ⬜ **M3.2 Train + iterate.** Retrain toward the search target; feed the improved
  net back into M2.2's continuations; repeat. *Deliverable:* successive nets.
  *Measure:* each round's A/B IMPs/board vs the prior net — **accept only gains**.
  *Deps:* M3.1.
- ⬜ **M3.3 Champion.** The best net by harness score becomes the optional neural
  floor. *Measure:* strictly positive IMPs/board vs the deterministic floor, with
  a board count large enough to exclude zero. *Deps:* M3.2.

Exit M3: a floor that beats the hand-written one on cardplay-grounded evidence.

---

## Milestone 4 — Component A, Role 1: authoring compiler

Parallelizable with M1–M3 once M0 exists; high near-term leverage.

- ⬜ **M4.1 DSL spec prompt.** A precise `Constraint`-DSL grammar + vocabulary +
  gold `(English, Rust)` pairs from existing rules. *Deliverable:* a compiler
  prompt/spec. *Measure:* it reproduces held-out existing rules from their English
  gloss. *Deps:* M0.2.
- ⬜ **M4.2 Verification harness.** Given a candidate `Constraint`, check it
  compiles and matches intent over random hands (and against the original rule
  when porting). *Deliverable:* a verifier. *Measure:* catches deliberately-broken
  constraints. *Deps:* M4.1.
- ⬜ **M4.3 Polish Club port (assisted).** Use M4.1+M4.2 to author the Polish Club
  books from their written notes. *Deliverable:* a second system's books + corpus.
  *Measure:* the ported system bids textbook auctions correctly; produces the
  second corpus needed for Component A Role 2. *Deps:* M4.2.

Exit M4: book authoring is "write the meaning, verify, commit" — and a second
system exists.

---

## Milestone 5 — Component A, Role 2: meaning-aware policy

The portability dream. Last, because it needs the most prerequisites.

- ⬜ **M5.1 Tag features.** Feed the discrete `tags` per prior call into the
  policy as categorical inputs. *Measure:* no regression; ideally a small gain.
  *Deps:* M0.2, M1.
- ⬜ **M5.2 Sequence-model policy.** Move Component B to a small transformer over
  the call sequence. *Measure:* matches or beats the MLP on the harness. *Deps:*
  M1 (as baseline).
- ⬜ **M5.3 Meaning encoder + cross-system training.** Embed text descriptions as
  meaning vectors; train across 2/1 **and** Polish Club. *Measure:* the *same* net
  bids both systems from their notes, each competitive with its single-system
  baseline on the harness. *Deps:* M4.3, M5.2.

Exit M5: one model, any system, driven by written meanings.

---

## Critical path and what to do first

```
M0  ──► M1 ──────────────► (working learned floor, = teacher)
  │       │
  │       └─► M2 ─► M3 ──► (learned floor > teacher)      ← the real goal
  │
  └─► M4 ─────────────────► (faster authoring + 2nd system)
            │
            └─► (with M5.2) ─► M5 ─► (cross-system bidder)  ← the dream
```

**Recommended first chunk:** all of **M0**. It is pure bridge/Rust, unblocks
every branch, and produces three durable assets (corpus, feature spec, teacher
dataset) that survive any later change of ML mind. After M0, **M1** is the
smallest path to a real "the machine bids" result, and **M4.1–M4.2** can run in
parallel since they only need the corpus.
</content>
