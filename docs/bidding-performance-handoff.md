# Bidding Engine Performance Recovery and Rule-Compilation Record

## Summary

Recover Pons’s bidding throughput without disabling bilans, envelope-union reading, fallback projection, or authored reading, and without changing any auction, logit, inference, alert, provenance, or explanation.

Pre-stage-2 pinned-core baseline:

| CPU placement | Pons | Wrapped BBA |
|---|---:|---:|
| 96 MB V-cache CCD | 197.8 µs/decision | 166.2 µs/decision |
| 32 MB frequency CCD | 184.1 µs/decision | 212.3 µs/decision |

Stage-2 acceptance run (2026-08-05; two warmups and ten measured
repetitions, fixed batches of the 512-position corpus):

| CPU placement | Pons median | Wrapped BBA median | Pons/BBA 95% ratio CI | Hot cache/reference upper CI | Whole-deal cache/reference upper CI |
|---|---:|---:|---:|---:|---:|
| CPU 4, 96 MB V-cache CCD | 13.706 µs/decision | 167.961 µs/decision | 0.0811–0.0818 | 0.0127 | 0.0567 |
| CPU 14, 32 MB frequency CCD | 12.979 µs/decision | 205.341 µs/decision | 0.0625–0.0634 | 0.0129 | 0.0592 |

Every measured CV was at most 1.82%. Hardware counters were unavailable
under the host's `perf` policy; the separate Rust allocation count/bytes pass
completed on both CPUs.

During authoring, the mutable `Trie` remains the semantic authority for what an
auction means — authored rules, weights, envelope-union readings, alerts, and
guards — as well as the mechanical routing structure. Binding now preserves
the declarative row identities beside that authoritative trie and compiles its
routing and reading mechanics into immutable sidecars. The serving path can
therefore cache those mechanics aggressively without changing meaning.

The solution has two complementary parts:

- Cache hand-dependent computations once per classification, and auction-dependent state once per deal, appended step by step.
- Compile routing, rule indexes, projections, and reader metadata from the row data when a complete `Stance` is built.

Data flow:

`Rows/legacy authoring → mutable Pair/Trie → Pair::against() finalization → compiled rule/reader plans → per-deal auction step cache → per-decision cache`

This document records both the staged design and its implementation status.
Stages 1–5 and the intervening RKCB phase 1.5 are implemented. Stage 6 remains
follow-up work; the final acceptance measurements for stages 3–5 are recorded
in the explicitly marked section below once the pinned runs complete.

## Sequencing

The execution order was explicit:

1. Complete stages 1–2: the reference/performance harness and the classification-scoped cache.
2. Complete phase 1.5 of the declarative book layer, moving floating agreements such as RKCB into book-owned declarative data.
3. Begin stage 3 and the later compilation/cursor work only after phase 1.5 is complete.

That order is now complete through stage 5. The contested-book rows prerequisite
and phase 1.5 landed before compilation began, so most semantics — including
RKCB — were already represented in authored data and the trie exposed a bounded
inventory of escape hatches.

- Stages 3–5 compile the row data itself. Waiting avoided building against a moving substrate and proving parity twice; each rows batch already shipped its own byte-identity proof.
- The escape-hatch inventory — opaque `guarded` rows, the closure-classifier sections still awaiting a row form, the grafted 1NT book, and the `insert_advance_of_double`/`insert_sohl_over` producers — is stage 5's enumerated legacy slow path.
- Stages 1–2 freeze and optimize the current post-contested-rows serving behavior.  They do not compile row data and do not absorb phase 1.5's authoring work.

Non-gates: the RKCB/DOPI addendum lowered to root-level guarded fallbacks the
decoder models, and knob migration only widens which reading profiles can be
compiled — stage 4 falls back to the legacy path on a profile mismatch.

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

### 3. Preserve row structure until whole-system finalization (implemented)

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
  reading-profile identity
```

Mutable standalone `Trie` values retain the legacy path. Public `Pair`, `Stance`, `Trie`, `System`, `Classifier::classify`, and `Context::new` behavior remains unchanged.

The implementation keeps an authoring ledger through trie mutation, grafting,
merging, and flooring. Finalization consumes that ledger only in
`Pair::against()`, validates each preserved target against the authoritative
live `Arc`, and emits the flat decoder metadata. Stale overwritten or collided
sites are discarded rather than compiled. The test-only catalog finalizer
remains an independent oracle.

### 4. Compile rule tables (implemented, level 1)

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

The implemented `CompiledRules` sidecar contains the declaration-order call
groups, alert and Pass indexes, pass-exclusion ceiling, stable explanation
indexes, four distinct projection plans, and conservative dependency masks.
Context-independent pure projections are eagerly evaluated and interned across
the stance; a runtime profile mismatch uses the legacy folds.

Level-2 projection specialization per concrete seat-fanned key and
vulnerability is deliberately deferred. The deal cache already evaluates a
dynamic projection only once for the call that made it, while eager
site-by-vulnerability expansion would spend the limited compiled-stance memory
headroom before demonstrating an additional serving benefit. Revisit it only
with an interned representation and the same construction-time and memory
gates.

The current explicitly shareable face predicates are the RKCB recognizers in
the root instinct classifier. That classifier covers an unbounded set of
auction routes, so there is no finite concrete-route face mask to freeze in the
shipped stance. They therefore take the designed dynamic path: an explicit
`FaceId` is evaluated lazily once in an immutable decision and reused by
classification and explanation; historical reader contexts use an
effect-scoped memo. Public opaque `Rules::face` closures remain observable and
are evaluated on every legacy-equivalent consult.

### 5. Compile a reader-side authoring decoder (implemented)

Build a sidecar decoder from the preserved row patterns:

- Exact and exact-suffix patterns become direct transitions.
- First-call guards become first-uncovered-call dispatch.
- Bounded overcalls expand over their finite legal call slots.
- Typed rebases retain rewrite metadata and the eight-rebase limit.
- Opaque guards remain an ordered, non-enumerated slow path.

Initially, leave bidding resolution unchanged. Use the decoder only to reproduce the authoring classifier for every auction prefix and parity-check it against `Trie::resolve_at`.

After parity was established, `project_authored`’s repeated root-to-prefix
resolutions were replaced with one forward auction scan. That scan:

- Produces each call’s authoring classifier and at-the-time context.
- Reuses results for own-side projection, opponent alerts, Pass reading, cue/control reading, and probed overlays.
- Incrementally maintains opening, phase, last bid, penalty, strain masks, and passed-hand state.
- Keys opponent decoding by the actual modeled stance and reading profile.

This removes the former roughly quadratic prefix resolution and repeated `Context::new` scans.

The one-shot scan is also extended into a per-deal step cache — the mechanical optimization the semantics move makes safe. A call's authoring classifier, its at-the-time context, and its projection boxes are fixed the moment the call is made; extending the auction never changes them. A serving loop therefore keeps one append-only state per deal and reading profile:

- Compact per-call route/provenance records.
- An incremental mechanical-context cursor.
- Category-specific accumulated projection effects in auction order.
- Running envelope intersections per seat.

At shallow depths the linear one-shot reader is cheaper than initializing the
cursor and projection accumulators, so table serving activates this state
lazily at depth 8. Its first prepare reconstructs that prefix in one forward
scan; each later decision appends only the new calls and re-derives the running
intersection. Thus the deep auction-dependent share becomes depth-constant and
the cached tail is depth-linear instead of repeatedly re-reading every prefix,
without regressing ordinary short deals. The fold order equals auction order,
so appending reproduces box order bit-for-bit. The cache is deal-scoped and
append-only, keyed by stance and reading profile; a profile change, non-prefix
query, or opaque/cache-unstable resolution mid-deal drops that deal to the
legacy path. Historical-prefix questions such as RKCB pre-answer decoding keep
their separate results, and stage 2's classification-scoped cache remains for
hand-dependent values and one-off classifications outside a serving loop.

The cache intentionally keeps the causal sufficient state rather than copies
of every intermediate. It materializes each at-the-time `Context` once, folds
that call's boxes once into the ordered category accumulators, and retains the
final route provenance and rebase count. Typed rewrite metadata stays in the
decoder rather than being copied into every deal record. Thus extending an
auction never recomputes an earlier context or projection, without retaining
unused full contexts, raw boxes, or rewrite traces.

For the normal structured path, advancing the exact-transition cursor is
amortized O(1) per appended call and direct resolution is depth-constant.
Structured typed rebases remain the bounded exception: they retain their
rewrite plan and recurse through at most the existing eight-rebase limit.
Opaque guards, opaque rewrites, computed targets, a profile change, or another
cache-unstable route fall back before observable hooks are invoked out of
legacy order.

An incremental legal-call mask is deferred to stage 6. Stage 5's reader has no
consumer for it, and table selection continues to use the authoritative
`Auction::can_push` laws check. Adding the mask together with the deferred
fixed-array legal selection keeps one parity surface instead of maintaining
unused duplicate legality state.

### 6. Reduce remaining allocation and duplicate work

After caching and compiled decoding are measured:

- Represent common one-box `EnvelopeUnion` values inline, with heap-backed storage only for true unions.
- Reuse intersection buffers and defer `tidy` until semantic boundaries while preserving box order and exact results.
- Share immutable precompiled projection boxes instead of cloning `Vec`s.
- Replace evaluator feature `Vec`s with fixed arrays sized for each model version.
- Add an incremental laws-derived legal-call mask and replace sorted legal-call selection with an order-preserving fixed-array maximum only if every mask result and all tie behavior remain identical to `Auction::can_push` and the current selector.
- Collapse the duplicated net-collar/bilans expression into one named predicate after the cache-only implementation has established a bit-identical reference.

## Stages 3–5 completion and performance record

### Completion record

| Stage | Commit subject | Verification summary |
|---|---|---|
| 3 — preserved authoring ledger and finalizer | `perf(bidding): preserve row grammar through finalization (stage 3)` | Live-`Arc` validation, overwrite/collision/graft/merge tests, decoder metadata oracle |
| 4 — compiled rule sidecars and shared-face execution | `perf(bidding): compile rule and reader execution (stage 4)` | Bit-exact logits/explanations, projection-profile parity, pure/opaque hook-order tests |
| 5 — flat decoder, forward cursor, and deal step cache | `perf(bidding): activate causal per-deal caching (stage 5)` | Trie/decoder route identity and provenance, linear-depth cursor, cache/drop and whole-auction parity |

### Final correctness record

- Full test suite: **716 passed, 0 failed, 4 ignored** in 683.73 s.
- 20,000-deal optimized/reference parity: **passed** in 517.08 s; the
  cache-coverage floor also passed.
- `smoke-default --count 20000 --seed 1`: byte-identical,
  SHA-256 `33dd53efa4b796e2e1c4d3f809ddb1476112e1a1ca0092721b9ba86e6f78dd7b`
- `render-book`: byte-identical,
  SHA-256 `759527867f98600961e6ed9b3d757cb91f0d30dd7f72dbb1cd8e22e80be35bde`
- Profile, vulnerability, opaque-route, typed-rebase, hook-order, stance/probe
  identity, and cache-drop focused coverage: **passed**

### Final performance record

| Gate | CPU 4, 96 MB V-cache CCD | CPU 14, 32 MB frequency CCD | Verdict |
|---|---:|---:|---|
| Pons median decision time | 13.341 µs | 12.729 µs | Pass |
| Wrapped BBA median decision time | 182.831 µs | 232.510 µs | Pass |
| Pons/BBA 95% ratio CI | 0.0723–0.0734 | 0.0543–0.0551 | Pass |
| Depth-8 improvement over stage 2 | 2.06× | 2.06× | Pass |
| Depth-12 / depth-4 latency | 2.27× | 2.25× | Pass |
| Whole-deal cache/reference upper CI | 0.1436 | 0.1426 | Pass |

- `Pair::against()` construction ratio: **1.62×** (35.667 ms / 22.047 ms), pass.
- Compiled-stance retained-memory growth: **22.95%** (33,260 KiB /
  27,052 KiB allocator-trimmed RSS delta), pass.
- Full-classification Rust allocations: **99.158 allocations and 7,922.4
  requested bytes per decision**; whole-deal serving: **798.547 allocations
  and 68,609.8 requested bytes per deal** (native allocations are not
  observed).

CPU 4 used two warmups and ten measured repetitions; CPU 14 used three
warmups and twenty measured repetitions after shorter runs failed the 2% CV
stability gate. Every accepted CV was at most 1.67%. BBA remains
wrapper-inclusive. Hardware counters remained unavailable under the host
`perf` policy.

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

- The declarative-book prerequisite was completed before stages 3–5 began. The optimizer still consumes opaque and legacy entries through the slow path, and the payoff — and any coverage statement — scales with grammar coverage.
- Structured row patterns receive fast compiled routing automatically. Opaque guards preserve identity, declaration order, and legacy resolution.
- Auction- and hand-dependent inference/evaluator results are cached per decision, or per deal in the append-only step cache, never baked into the trie.
- No bidding feature, model, reading mode, or convention is disabled to meet the performance target.
- The historical 17.73 µs figure is a stretch reference, not a release gate; the controlled two-CCD BBA comparison and bit-identical behavior are the shipping criteria.
