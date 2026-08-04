# Bidding Engine Performance Recovery and Rule-Compilation Plan

## Summary

Recover Pons’s bidding throughput without disabling bilans, DNF, fallback projection, or authored reading, and without changing any auction, logit, inference, alert, provenance, or explanation.

Current pinned-core baseline:

| CPU placement | Pons | Wrapped BBA |
|---|---:|---:|
| 96 MB V-cache CCD | 197.8 µs/decision | 166.2 µs/decision |
| 32 MB frequency CCD | 184.1 µs/decision | 212.3 µs/decision |

The solution has two complementary parts:

- Cache auction- and hand-dependent computations once per classification.
- Use the ongoing declarative row/rule work to compile routing, rule indexes, projections, and reader metadata when a complete `Stance` is built.

Data flow:

`Rows/legacy authoring → mutable Pair/Trie → Pair::against() finalization → compiled rule/reader plans → per-decision cache`

This document is planning only; no repository changes are authorized here.

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

The row layer currently lowers structured patterns into opaque classifier and guard objects too early. Preserve the following metadata through authoring:

- Stable, unique `PatternId`.
- `PatternKind`: exact node, exact suffix, first uncovered call, bounded overcall, rebase, or opaque guard.
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

### 6. Reduce remaining allocation and duplicate work

After caching and compiled decoding are measured:

- Represent common one-box DNF values inline, with heap-backed storage only for true unions.
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
  - Inference envelopes, DNF boxes and ordering.
  - Evaluator feature vectors.
  - Alerts, announcements, rendered books, and explanation rule indices.
- Byte-compare `smoke-default --count 20000 --seed 1` and `render-book` before and after.
- Repeat under DNF on/off, fallback projection on/off, Pass/table reading on/off, bilans/net-collar combinations, evaluator variants, all vulnerabilities, and configured/unconfigured floors.
- Test exact-node rejection followed by fallback, cloned configured contexts, different hands sharing a bare context, separate threads with different knob profiles, systems-on auction stripping, RKCB historical-prefix reads, typed guards, opaque guards, and rebases.
- For every prefix in the fixed corpus, require the compiled authoring decoder to select the same classifier and provenance as legacy resolution.
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

- The ongoing row migration continues independently; the optimizer consumes both row-authored and legacy trie entries, so full migration is not a prerequisite.
- Structured row patterns receive fast compiled routing automatically. Opaque guards preserve identity, declaration order, and legacy resolution.
- Auction- and hand-dependent inference/evaluator results are cached per decision, never baked into the trie.
- No bidding feature, model, reading mode, or convention is disabled to meet the performance target.
- The historical 17.73 µs figure is a stretch reference, not a release gate; the controlled two-CCD BBA comparison and bit-identical behavior are the shipping criteria.
