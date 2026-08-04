# Bidding Engine Performance Recovery and Rule-Compilation Plan

## Summary

Recover Pons’s bidding throughput without disabling bilans, envelope-union reading, fallback projection, or authored reading, and without changing any auction, logit, inference, alert, provenance, or explanation.

Current pinned-core baseline:

| CPU placement | Pons | Wrapped BBA |
|---|---:|---:|
| 96 MB V-cache CCD | 197.8 µs/decision | 166.2 µs/decision |
| 32 MB frequency CCD | 184.1 µs/decision | 212.3 µs/decision |

The `Trie` today carries both semantics and mechanics: semantics — what an auction means (authored rules, weights, envelope-union readings, alerts, guards) — and mechanics — how a key routes (children maps, the fallback walk, the rebase loop). The declarative book layer is moving the semantics out into row data. What remains in the trie afterwards is pure mechanics, which the stages below may compile and cache aggressively — compiled routing, step-by-step auction caching — without touching meaning.

The solution has two complementary parts:

- Cache hand-dependent computations once per classification, and auction-dependent state once per deal, appended step by step.
- Compile routing, rule indexes, projections, and reader metadata from the row data when a complete `Stance` is built.

Data flow:

`Rows/legacy authoring → mutable Pair/Trie → Pair::against() finalization → compiled rule/reader plans → per-deal auction step cache → per-decision cache`

This document is planning only; no repository changes are authorized here.

## Sequencing: execution awaits the declarative book layer

No stage below starts until the rows migration has ported the contested books — `competition()` and `defensive()` as package lists — so that most semantics live in row data and the trie holds mechanics plus an enumerated set of escape hatches.

- Stages 3–5 compile the row data itself. Compiling mid-port means building against a moving substrate and proving parity twice; each rows batch already ships its own byte-identity proof.
- The escape-hatch inventory — opaque `guarded` rows, the closure-classifier sections still awaiting a row form, the grafted 1NT book, the `insert_advance_of_double`/`insert_sohl_over` producers — is exactly stage 5's legacy slow path. It must be closed as an enumeration before decoder coverage can be stated.
- Stages 1–2 do not depend on rows, but execute with the rest: stage 1 freezes an internal reference implementation, and freezing the pre-rows one wastes the freeze.

Non-gates: the floating addendum (RKCB/DOPI) lowers to root-level guarded fallbacks the decoder already models, and knob migration only widens which reading profiles can be compiled — stage 4 already falls back to the legacy path on a profile mismatch.

## Implementation Plan

### 1. Establish the performance and parity harness

- Add fixed position corpora covering shallow/deep authored nodes, neural floors, constructive and forced instinct floors, RKCB/slam tails, and inference depths 2/4/8/12+.
- Construct the stance and warm model weights outside timed regions. Benchmark `Inferences`, evaluator features, evaluator forward, `instinct`, full classification, legal-call selection, and whole-deal bidding separately.
- Record allocations and bytes alongside time. Use hardware counters where permissions allow.
- Preserve the current implementation as an internal reference path so optimized and legacy results can be compared in one process.

### 2. Add a classification-scoped dynamic cache

Introduce a crate-private `DecisionCache`, attached to `Context` through an `Arc` only for one classification:

```text
DecisionCache
  hand
  active reading/evaluator profile
  OnceLock<Inferences>
  OnceLock<TrickEstimates>
  OnceLock<Interpretation>
  test-only initialization counters
```

- `Context::new` remains uncached so diagnostics can reuse a context across hands and knob changes.
- Serving entry points create one decision scope before trie resolution. Exact-node rejection, fallback classification, `Rules::explain`, and `Context::clone().with_config(...)` share the same cache.
- Structural context changes such as attaching different prefixes or a different opponent system clear or reject an existing cache; attaching neural configuration preserves it.
- Add crate-private `Context::inferences()` and `Context::trick_estimates(hand)` accessors. Public `Inferences::read` remains an uncached, owned-result API.
- Migrate full-auction inference/evaluator consumers to these accessors. Genuinely different historical-prefix questions, such as RKCB pre-answer decoding, keep separate results.
- Snapshot all thread-local reading/evaluator knobs at decision entry and assert in debug builds that the cache remains on its creating thread.
- Convert bilans predicates into a named constraint using `Context::trick_estimates`; retain existing eager `And`/`Or`, rule order, and floating-point evaluation order in this stage.

Acceptance for this stage:

- At most one full-auction inference construction and one trick-evaluator forward per applicable decision.
- At least 4× improvement on hot instinct positions and 25% on whole-deal throughput.
- No change to any output bit or provenance.

### 3. Preserve row structure until whole-system finalization

The row layer currently lowers rows immediately through the legacy verbs into type-erased classifier and guard objects. Preserve the authored `Pattern`/`Row` values alongside the lowered objects until finalization:

- Stable, unique `PatternId`.
- The pattern's shipped grammar construct: exact node or table, uncontested interleave (`after`), first-call dispatch (`first`), bounded overcall (`up_to`), typed rebase, or the opaque `guarded` escape hatch.
- Concrete seat-fanned keys and declaration order.
- Rule-table identity independent of rendered labels or diagnostic samples.

Do not globally compile inside `rows::compile_into`; books are still mutated, grafted, merged, and floored afterward. Finalize in `Pair::against()`, after the constructive/competitive merge and all fallbacks are present.

The resulting internal runtime book contains:

```text
BoundBook
  finalized Trie
  CompiledRules registry
  AuthoringDecoder
  per-site projection plans
  reading-profile identity
```

Mutable standalone `Trie` values retain the legacy path. Public `Pair`, `Stance`, `Trie`, `System`, `Classifier::classify`, and `Context::new` behavior remains unchanged.

### 4. Compile rule tables

For every rule-backed classifier, build an immutable `CompiledRules` while retaining the original authored `Rules` for rendering and explanation.

Each plan contains:

- Declaration-order rule indices grouped by output call.
- Alerted-rule indices grouped by call.
- Pass-rule indices, maximum Pass weight, and stronger non-Pass rules needed by pass exclusion.
- Separate forward, band, complement, and announcement projection plans.
- A dependency mask for hand summaries, context facts, inference, and trick estimates.
- Stable original rule indices for `explain_call`.

Rules must still use max-logit-per-call semantics. Compilation may index, pre-bind, constant-fold, and reuse values, but must not reorder constraint operands or rules.

Move hand-independent instinct predicates into explicit face metadata where possible. Compile those face masks per concrete route/profile; evaluate remaining dynamic face predicates once per decision. Keep the root instinct classifier and its provenance unchanged rather than relocating rules to different trie nodes.

Precompute projection data in two levels:

1. Initially, only context-independent fragments.
2. For exact nodes and finite structured patterns, specialize per concrete seat-fanned key, vulnerability, and at-the-time context after parity infrastructure exists.

Keep `project`, `project_band`, `project_complement`, and `announce` distinct. Compile the active/default reading profile; if runtime knobs do not match it, use the legacy projection path. Additional profiles may be compiled later if their benchmarks justify the memory.

### 5. Compile a reader-side authoring decoder

Build a sidecar decoder from the preserved row patterns:

- Exact and exact-suffix patterns become direct transitions.
- First-call guards become first-uncovered-call dispatch.
- Bounded overcalls expand over their finite legal call slots.
- Typed rebases retain rewrite metadata and the eight-rebase limit.
- Opaque guards remain an ordered, non-enumerated slow path.

Initially, leave bidding resolution unchanged. Use the decoder only to reproduce the authoring classifier for every auction prefix and parity-check it against `Trie::resolve_at`.

Once parity is established, replace `project_authored`’s repeated root-to-prefix resolutions with one forward auction scan. That scan will:

- Produce each call’s authoring classifier and at-the-time context.
- Reuse results for own-side projection, opponent alerts, Pass reading, cue/control reading, and probed overlays.
- Incrementally maintain opening, phase, last bid, penalty, strain masks, passed-hand state, and legal-call masks.
- Key opponent decoding by the actual modeled stance and reading profile.

This removes the current roughly quadratic prefix resolution and repeated `Context::new` scans.

Then extend the one-shot scan into a per-deal step cache — the mechanical optimization the semantics move makes safe. A call's authoring classifier, its at-the-time context, and its projection boxes are fixed the moment the call is made; extending the auction never changes them. A serving loop therefore keeps one append-only state per deal and reading profile:

- Per-call records: authoring classifier, at-the-time context facts, projection boxes, rebase rewrites taken.
- Incrementally maintained context facts and legal-call masks.
- Running envelope intersections per seat.

Each decision appends the new call's record and re-derives only the running intersection, so the auction-dependent share of a decision becomes depth-constant and a deal's total reading work depth-linear, replacing today's per-decision full re-read. The fold order equals auction order, so appending reproduces box order bit-for-bit. The cache is deal-scoped and append-only, keyed by stance and reading profile; a knob change, probe overlay, or opaque-node resolution mid-deal drops that deal to the legacy path. Historical-prefix questions such as RKCB pre-answer decoding keep their separate results, and stage 2's classification-scoped cache remains for hand-dependent values and one-off classifications outside a serving loop.

### 6. Reduce remaining allocation and duplicate work

After caching and compiled decoding are measured:

- Represent common one-box `EnvelopeUnion` values inline, with heap-backed storage only for true unions.
- Reuse intersection buffers and defer `tidy` until semantic boundaries while preserving box order and exact results.
- Share immutable precompiled projection boxes instead of cloning `Vec`s.
- Replace evaluator feature `Vec`s with fixed arrays sized for each model version.
- Replace sorted legal-call selection with an order-preserving fixed-array maximum only if all tie behavior remains identical.
- Collapse the duplicated net-collar/bilans expression into one named predicate after the cache-only implementation has established a bit-identical reference.

## Verification and Acceptance

### Correctness

- Compare optimized and legacy paths over at least 20,000 seeded deals:
  - All 38 logits via `f32::to_bits`.
  - Selected call.
  - Provenance and fallback/rebase count.
  - Inference envelopes, envelope-union boxes and ordering.
  - Evaluator feature vectors.
  - Alerts, announcements, rendered books, and explanation rule indices.
- Byte-compare `smoke-default --count 20000 --seed 1` and `render-book` before and after.
- Repeat under envelope-union reading on/off, fallback projection on/off, Pass/table reading on/off, bilans/net-collar combinations, evaluator variants, all vulnerabilities, and configured/unconfigured floors.
- Test exact-node rejection followed by fallback, cloned configured contexts, different hands sharing a bare context, separate threads with different knob profiles, systems-on auction stripping, RKCB historical-prefix reads, typed guards, opaque guards, and rebases.
- For every prefix in the fixed corpus, require the compiled authoring decoder to select the same classifier and provenance as legacy resolution.
- For every prefix in the fixed corpus, require the appended step-cache state to equal the from-scratch read bit-for-bit — boxes, box order, and provenance — including a mid-deal knob change that must drop to the legacy path.
- Any changed output ends the performance-only track; it becomes a separate bidding change requiring fresh paired bidding A/B measurement.

### Performance

- Use at least ten warmed release repetitions, pinned to CPU4 and CPU14 with SMT siblings idle; require coefficient of variation no greater than 2%.
- Compare Pons and BBA on the same fixed, depth-balanced corpus, half harvested from each engine’s auctions. Report BBA explicitly as wrapper-inclusive because it creates a bot and replays the prefix per decision.
- Required final gate: the upper 95% confidence bound of `Pons/BBA` decision time is below 1.0 on both CCDs.
- Target final median: `Pons/BBA ≤ 0.80` on both CCDs.
- Depth-8+ inference must improve by at least 2×, and depth-12 latency must be no more than 3× depth-4 latency.
- No representative hot path may regress by more than 5%; no whole workload may regress by more than 3%.
- `Pair::against()` construction time may grow by at most 2× and compiled stance memory by at most 25%. Compilation must be eager there, with no first-decision compilation pause.

## Assumptions

- Execution awaits the declarative book layer: stages 3–5 compile row data, so the contested-book port is a prerequisite (see Sequencing). The optimizer still consumes opaque and legacy entries through the slow path, and the payoff — and any coverage statement — scales with grammar coverage.
- Structured row patterns receive fast compiled routing automatically. Opaque guards preserve identity, declaration order, and legacy resolution.
- Auction- and hand-dependent inference/evaluator results are cached per decision, or per deal in the append-only step cache, never baked into the trie.
- No bidding feature, model, reading mode, or convention is disabled to meet the performance target.
- The historical 17.73 µs figure is a stretch reference, not a release gate; the controlled two-CCD BBA comparison and bit-identical behavior are the shipping criteria.
