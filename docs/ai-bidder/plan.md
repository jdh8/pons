# Phased plan

Small, well-specified, individually measurable chunks; each milestone names a
**deliverable**, a **measure**, and its **deps**. A map, not a green light —
nothing starts until explicitly chosen.

Legend: ⬜ not started · 🚧 in progress / blocked · ✅ done.

**Standing principles** (apply to every milestone; not repeated below):

- **Spend runtime for better calls.** It is usually fine to do real work in
  `pons` at decision time — search, inference, simulation — to bid/play better.
  The bottleneck is the double-dummy solver, not our per-call logic; optimize
  decision quality first, runtime only when it actually bites.
- **BBA is a reference for bridge *and* for programming.** EPBot is a mature
  engine to learn from on both axes. When building any new feature, compare with
  BBA — reverse-engineering it is fine (see [`bba-floor.md`](bba-floor.md) for
  the method: `strace`, the `MB.TXT` export, the introspection FFI).

---

## Milestone 0 — Corpus + baseline lock-in (no ML)

The foundation. Pure bridge/Rust work; de-risks everything downstream.

- ✅ **M0.1 Rule labels.** Add a short string label to each `rule(...)` in the
  books (or a parallel map). *Deliverable:* `explain()` can name a human-readable
  meaning per winning rule. *Measure:* every node `explain()`s to a non-empty
  label. *Deps:* none. **Done (Hybrid):** opt-in mechanism only — `Rule.label`
  (`&'static str`, empty default), `Rules::note("…")` builder, `Rule::label()`
  accessor, and `Classifier::as_rules()` to recover a node's authored `Rules`
  from the type-erased trie. No bulk authoring; descriptions are auto-derived
  (M0.2) and patched with `note` where needed.
- ✅ **M0.2 Corpus exporter.** A dev tool that walks the trie and emits the
  per-node corpus records `{auction, call, tags, description, constraint-summary}`
  (schema in [foundations §1d](01-foundations.md#1d-the-description-corpus-component-as-prerequisite)).
  *Deliverable:* a corpus file for the 2/1 system. *Measure:* record count ≈ node
  count; spot-check 20 records for accuracy. *Deps:* M0.1. **Done:**
  `examples/export-corpus` → JSONL; 770 authored nodes, 2314 `(node,call)`
  records (2074 constructive, 240 defensive, 0 competitive — that book is mostly
  rebases/fallbacks), 1479 with a specific WBF tag. Shallow natural nodes
  (openings, NT responses, T/O doubles, 2/1, raises, weak-twos) verified
  accurate; deep artificial trees (RKCB/BTU) tagged coarsely → `note`-patch
  targets. No `constraint` field yet (constraints are eval-only, unreadable).
- ✅ **M0.3 Feature extractor (spec + reference impl).** Define `features(hand,
  context) -> Vec<f32>` (foundations §1a–1b): suit-exchangeable hand block +
  `Context`/`Inferences` summary + vul + seat. *Deliverable:* a documented,
  versioned feature vector and a Rust function producing it. *Measure:* unit
  tests pin the layout; round-trips a few known hands/auctions. *Deps:* none.
  **Done:** `bidding::features` — `FEATURES_V1`, 160 floats (76 hand + 6 global
  evals + 36 context + 40 inferences + 2 vul), `FEATURES_VERSION`/`FEATURES_LEN`
  + `OFFSET_*`/`LEN_*` constants, 11 layout-pinning tests. Tags chosen for the
  corpus: WBF abbreviations (`wbf-abbreviations.md`).
- ✅ **M0.4 Teacher dump.** Using the feature extractor, bid out random boards
  with `american()` and record `(features, teacher_softmax)` at each decision,
  oversampling off-book/contested auctions. *Deliverable:* a training dataset.
  *Measure:* dataset stats (size, off-book fraction, call-distribution sanity).
  *Deps:* M0.3. **Done:** `examples/teacher-dump` → flat LE-`f32` (198/row) +
  JSON sidecar (versioned). Sanity at 3000 boards: 28951 rows, every softmax sums
  to 1.0, ~72% contested, sane call histogram (P 57%, X 6.5%, openings…). Random
  boards already yield mostly-contested rows; targeted off-book oversampling is
  left to M1 data prep.

Exit M0: ✅ we have a corpus, a versioned feature spec, and a teacher dataset —
without writing a line of ML.

---

## Milestone 1 — Distilled floor, end-to-end (Phase 1 of Component B)

Prove the whole pipeline by *cloning* the current system.

- ✅ **M1.1 Train the MLP** (off-crate). Fit the summary-feature MLP
  ([policy-net Phase 1](02-policy-net.md#phase-1--mlp-on-summary-features-the-recommended-start))
  to the M0.4 dataset. *Deliverable:* a weights artifact + held-out cross-entropy.
  *Measure:* held-out top-1 agreement with the teacher (target high, e.g. >95% on
  on-book, lower but sane off-book). *Deps:* M0.4. **Done:** `trainer/` (off-crate
  candle workspace, `exclude`d from the package) → `american_v1.{f32,json}`,
  a 160→256→256→38 MLP distilled from `american()`. 80 epochs, val CE 0.249,
  top-1 93.8% overall (94.4% constructive, 93.6% contested). Sidecar records
  feature/teacher version, data seed, and git SHA.
- ✅ **M1.2 Rust forward pass** behind an `ml`/`neural-floor` feature flag
  ([integration Part 4](04-integration-and-eval.md#part-4--running-the-model-in-pons)).
  *Deliverable:* `classify(features) -> Logits` in-crate, dependency-free.
  *Measure:* its logits match the off-crate model bit-closely on a fixture set
  (the cross-language equivalence test). *Deps:* M1.1, M0.3. **Done:**
  `bidding::neural::classify` behind `neural-floor` (weights `include_bytes!`d, no
  ML runtime). `matches_candle_fixture()` reproduces the candle logits within
  1e-3 and matches the arg-max on every fixture row.
- ✅ **M1.3 Safety shell.** Wrap the net with the deterministic legality +
  forced-situation override ([invariants §0.4](01-foundations.md#part-0--invariants)).
  *Deliverable:* a `Classifier` safe to attach as the floor. *Measure:* the five
  §0.4 safety properties pass against the shelled net (the rails, enforced by
  construction); aggregate teacher-parity is measured by M1.4 — *not* per-auction
  identity with `instinct()`, infeasible for a ~94%-accurate net. *Deps:* M1.2.
  **Done:** `bidding::neural_floor::NeuralFloor` + `american_neural()`. Forced
  auctions (`instinct::forced` — partner's live takeout double, an auction forcing
  game, a just-made transfer over our strong NT) delegate to `instinct()`
  verbatim; everything else is the net, legality-masked via `Auction::can_push`
  (`Pass` stays finite). Five gated rails tests green. *Decision:* hand-conditioned
  game forces (a strong-NT responder who *holds* game values) are judgement the
  net is trusted with, not a hard rail.
- ✅ **M1.4 A/B measurement.** A variant of the instinct-floor example: neural
  floor vs deterministic floor vs bare books. *Deliverable:* IMPs/board numbers.
  *Measure:* parity with the deterministic floor (≈ 0 IMPs/board against it) and
  +0.5 preserved vs bare books, over enough boards. *Deps:* M1.3. **Done:**
  `examples/neural-floor` (gated), two duplicate matches with 95% CIs. At 8000
  boards, vul none: neural vs deterministic −0.014 IMPs/board, CI [−0.054, +0.026]
  (contains 0 — *parity*, the authoritative head-to-head); neural vs bare +0.587
  IMPs/board, CI [+0.517, +0.656] (the deterministic floor's ≈ +0.5 worth
  preserved, marginally above on this sample).

Exit M1: ✅ a learned floor that *equals* the hand-written one, shipped lean,
proven on the harness. The machine now does the floor's job — not yet better.

---

## Milestone 2 — Constrained sampler (the search prerequisite)

The piece `Inferences` was built for; needed before any "beat the teacher" work.

- ✅ **M2.1 Sampler.** Given `(auction)`, deal the other hands consistent with
  every player's `Inferences` ranges. *Deliverable:* `sample_layouts(context, n)`.
  *Measure:* soundness — every sampled hand falls within its shown ranges
  (property test); coverage — the dealt distribution isn't degenerate. *Deps:*
  none (builds on `Inferences`). **Done:** `bidding::sampler::sample_layouts(hand,
  seat, &Inferences, rng, n) -> Vec<FullDeal>` (ungated — the natural completion
  of `Inferences`). Rejection sampling on `contract_bridge::deck::fill_deals`:
  the actor's hand is pinned into a partial deal so each draw deals only the
  other 39 cards, kept iff LHO/partner/RHO land within their shown ranges
  (lengths + `constraint::point_count`, the shared upgraded-points scalar). An
  `n * 256` attempt budget terminates tight/infeasible auctions, returning ≤ `n`
  layouts so a shortfall is visible to the caller. Six tests: soundness
  (proptest), count met on feasible auctions, non-degenerate coverage, empty on
  an infeasible auction, zero-request. `rand` promoted to a direct dep (already
  transitive via `contract-bridge`, so the tree is unchanged). *Signature note:*
  the actor's `hand` and absolute `seat` are explicit parameters — `Context`
  carries neither — and `&Inferences` is taken directly (read via
  `Inferences::read`) so the core is testable without crafting an auction.
- ✅ **M2.2 Call EV evaluator.** For a candidate call, continue the auction under
  the current policy over sampled layouts, reach a contract, score double-dummy,
  average. *Deliverable:* `ev(hand, context, call) -> f32`. *Measure:* sanity on
  known textbook decisions (it should prefer the obviously-right call). *Deps:*
  M2.1, the policy from M1. *Note:* this evaluator feeds **both** M2.3 (the live
  player) and M3.1 (offline training targets) — same engine, two uses. The
  double-dummy solves are shared across candidate calls: solve each sampled layout
  once with `NonEmptyStrainFlags::ALL` and score every candidate contract from its
  `TrickCountTable`, so cost is `n` solves, not `k·n`. **Done:**
  `bidding::ev::{ev, ev_all}` (ungated). `ev_all` samples layouts, solves each
  once with `ALL`, and prices a whole candidate slate off the shared
  `TrickCountTable`s; `ev` wraps it for one call. The rollout reuses
  `Table::bid_out_from` — seed the candidate onto the prior auction, then bid out
  — with the continuation policy seated in **all four seats** (self-play). EVs are
  average scores in points in the actor's favour; an illegal candidate, or an
  auction too tight or infeasible to sample, returns `NaN` ("no signal"). Five tests:
  ranking sanity (sound game > hopeless grand, grand prices out negative),
  fixed-seed determinism, the illegal-candidate and infeasible `NaN` paths, and
  the empty slate. *Decision (settled this milestone):* the continuation policy is
  a `System` **parameter**, not hardwired — `ev`'s `policy: &impl System` defaults
  callers to the deterministic `american()` for debuggable validation (and ≈
  the M1 net at bootstrap); M3.2 swaps in successive nets with no change to this
  code.
- ✅ **M2.3 Live search bidder (gated).** Wrap M2.2 as a runtime
  `Classifier`/`System`: at each non-forced decision, use the net's softmax as a
  prior to shortlist the top-`k` legal calls, run `ev` over sampled layouts,
  return a distribution peaked on the high-EV calls — behind a `search` cargo
  feature, wrapped in the same forced-rails shell as `NeuralFloor`. This *is*
  "simulations in action": the policy simulates before it bids. *Deliverable:* a
  feature-gated `american_search()` / `SearchFloor`. *Measure:* A/B IMPs/board
  vs the deterministic floor (strictly positive) **and** vs the distilled net
  (search should beat the raw policy), over a board count large enough to exclude
  zero; the five §0.4 rails tests stay green against the shelled search bidder.
  *Deps:* M2.2, M1 (net as prior/policy). *Decisions:* bidding only; slow & gated
  is acceptable (knobs — `n` layouts, `k` shortlist, EV temperature — default to
  strength, not latency); the default build and `instinct()` baseline are
  untouched. **Done:** `bidding::search_floor::SearchFloor` + `american_search()` behind
  the `search` feature (⊇ `neural-floor`). Shell mirrors `NeuralFloor` (`forced` →
  `instinct()`); the judgement middle masks the net prior, shortlists top-`k = 8`,
  prices them with `ev_all` over `n = 128` layouts, and re-seats onto an EV-ranked
  band (`prior_max + 3` nats, EV-temp `100` pts/nat; `Pass` and the un-evaluated
  tail stay finite; all-`NaN` → bare net). *Decisions:* continuation policy is
  **neural self-play** (`american_neural()` all four seats — the policy M3.2
  iterates); budget defaults to **strength** (`n = 128`, `k = 8`, ≈ 1.4 s/decision
  — cost ≈ linear in `n`, the shared DD solve dominating; `k` ≈ 45 ms/extra
  candidate). *Determinism* (§0.5): rollout RNG seeded from the feature vector.
  *Seat:* actor canonicalized to North (EV is actor-relative, so free). Seven
  gated tests green (five §0.4 rails + determinism + EV-band); `examples/search-floor`
  is the A/B harness. Headline IMPs/board await a long run (search is slow by design).

Exit M2: we can ask "what is each call actually worth on this hand?" — the signal
the books never had — and we can *bid by it* at the table (M2.3), gated.

---

## Milestone 3 — Search-improved floor (Phase 2 of Component B)

Distill M2.3's strong-but-slow search bidder back into one forward pass: the
default floor stays fast, the gated search bidder remains for maximum strength.
(`instinct()` stays the baseline; both learned floors are added options.)

- ✅ **M3.1 Improvement targets.** Run the M2.3 search bidder over sampled
  decisions and record its improved distribution as the training target
  ([policy-net Phase 2](02-policy-net.md#phase-2--search-beat-the-teacher)).
  *Deliverable:* a dataset of `(features, search_target)`. *Measure:* targets
  differ from the teacher mainly off-book/contested (where the books were silent).
  *Deps:* M2.3. **Done:** a gated `search-dump` example (sister to `teacher-dump`) self-plays
  the search bidder over seeded boards and writes `(features, search_softmax)` rows
  in the **same `f32`/`.json`/`.tags` layout the trainer already reads** — a
  trainer-compatible *superset* of `teacher-dump`, identical on book nodes and
  upgraded off-book (the `.tags` byte gains `bit1` = off-book; activation read from
  `Stance::classify_with_provenance`, `depth == 0 && fallback.is_some()`). Measure:
  arg-max disagreement + mean total-variation vs both the deterministic teacher
  (`american`) and the raw net prior (`american_neural`), split off/on-book
  and contested/constructive. 40-board smoke: **~51 % arg-max disagreement, ~0.53
  mean TV off-book vs `0`/`0` on-book** (identical book logits by construction); all
  off-book rows contested (the floor sits only under the competitive/defensive
  books). The additive `american_search_with(SearchFloor)` constructor (gated
  `search`) exposes `--layouts`/`--shortlist`/`--temperature`; the full production
  dataset feeds M3.2.
- ✅ **M3.2 Train + iterate.** Retrain toward the search target; feed the improved
  net back into M2.2's continuations; repeat. *Deliverable:* successive nets.
  *Measure:* each round's A/B IMPs/board vs the prior net — **accept only gains**.
  *Deps:* M3.1. **Round 1 done:** trained a v1-featured net on the 10 000-board
  `search-dump` (97 701 rows, git_sha `1d43577`) toward the search softmax —
  `neural::classify_search`, the `NeuralFloorSearch` shell (same forced-rail
  delegation + legality mask), and `american_neural_search()` (gated
  `neural-floor`; baselines untouched). Held-out fit to the harder target: val-CE
  0.776, top-1 89.4 % constructive / 73.8 % contested (looser than the teacher clone
  by design). **A/B (20 000 boards, vul none): +0.787 IMPs/board vs the v1 net** (CI
  [+0.718, +0.857]), +0.700 vs the deterministic floor, +0.816 vs bare — a decisive
  gain by the harness metric. *Caveat:* 75 % divergence from the v1 net and a
  DD-scored A/B (like the teacher) mean the magnitude likely overstates real-table
  value; the gain concentrates off-book/competitive, as M3 intended. **Round 2
  done (promoted):** regenerated the search-dump with the round-1 net as the
  rollout continuation policy *and* the doubling-aware `ev_all` (104 476 rows / 10k
  boards, git_sha `6a4ae96`), retrained identically (val-CE 0.967, top-1 88.1 %
  constructive / 70.3 % contested — a harder, more disciplined target). **A/B
  (20 000 boards) round-2 vs round-1, on the default perfect-defense measure
  (failing contracts priced doubled): +1.661 IMPs/board vul none (CI
  [+1.550, +1.772]), +2.069 vul both (CI [+1.957, +2.181]).** Round 2 learned to
  *stop reaching doubled-down contracts* — the discipline its doubling-aware
  targets reward. It also beats the deterministic floor on the same measure (+0.178
  vul none, +1.716 vul both; CIs exclude 0). On the optimistic double-dummy bound
  (down contracts scored undoubled) the step is parity vul none (+0.046) and a gain
  vul both (+0.424) — never worse on either bound. Promoted: the round-2 weights
  replaced the production search net in place (`american_neural_search()` is now
  round 2 everywhere; the temporary comparison wiring was reverted).
- ✅ **M3.3 Champion.** The best net by harness score becomes the optional neural
  floor. *Measure:* strictly positive IMPs/board vs the deterministic floor, with
  a board count large enough to exclude zero. *Deps:* M3.2. **Done:** the round-2
  search net is the champion — on the default perfect-defense measure it beats the
  deterministic floor at 20 000 boards by +0.178 vul none (CI [+0.075, +0.282]) and
  +1.716 vul both (CI [+1.608, +1.824]), and is positive on the optimistic
  double-dummy bound too (+0.123 / +0.583). It is the in-place production search net
  (`american_neural_search()`, gated `neural-floor`); `instinct()` stays the
  default and baseline, this is the optional learned floor it intended.

Exit M3: ✅ a floor that beats the hand-written one on cardplay-grounded evidence —
decisively on the default perfect-defense measure, and at parity-or-better on the
optimistic double-dummy bound, across two search-distillation rounds.

---

## Milestone 4 — Component A, Role 1: authoring compiler

Parallelizable with M1–M3 once M0 exists; high near-term leverage.

- ✅ **M4.0 Self-describing DSL (readable books).** Make the `Constraint` DSL
  render its own meaning (`Constraint → English`), the inverse of the compiler and
  the round-trip substrate that makes M4.1/M4.2 verifiable. *Deliverable:* a
  readable face for every authored book. *Measure:* every corpus node renders a
  truthful constraint description; rails stay green (`eval` unchanged). *Deps:*
  none (pure Rust). **Done:** `Constraint::describe() -> Description` (default `Opaque`, non-breaking):
  each of the ~21 primitives became a named struct that names itself, combinators
  compose into an `All`/`Any`/`Not` tree, and `Description: Display` renders prose
  ("12–21 points, and 5+ ♠"). `described(label, cond)` is the labeled escape hatch
  for bespoke predicates (better-minor, Michaels/Unusual lengths, RKCB keycards),
  driving the corpus to **0 opaque**. `Rule::describe()` surfaces it; `render-book`
  prints the books as prose; `export-corpus` emits a truthful `constraint` field
  (precedence: `note` label → constraint render → structural gloss) + opaque count.
  770 nodes / 2314 records, 0 opaque; all 353 tests green. *Decision:* led M4 with
  this per the user's "make books more readable" steer — it is the readability
  deliverable **and** the verification substrate M4.1's compiler needs.
- ✅ **M4.1 DSL spec prompt.** A precise `Constraint`-DSL grammar + vocabulary +
  gold `(English, Rust)` pairs from existing rules. *Deliverable:* a compiler
  prompt/spec. *Measure:* it reproduces held-out existing rules from their English
  gloss. *Deps:* M0.2, M4.0 (the self-describing DSL *is* the executable spec, and
  its `describe()` is the round-trip checker). **Done:** [`dsl-spec.md`](dsl-spec.md)
  — a pasteable English→`Constraint` prompt: the `&`/`|`/`!` grammar and its
  `describe()` rendering, a vocabulary table for all 21 primitives (exact gloss +
  range conventions), the `described(...)` escape-hatch discipline, gold pairs
  harvested from the live books, and explicit compile instructions.
  `tests/dsl_roundtrip.rs` is the mechanical round-trip: it pins every primitive
  gloss and the combinator/range rendering against `describe()`, and reproduces
  **12/12 held-out real rules** from their gloss alone (exact identity). *Measure
  met:* 100% held-out reproduction; the lone ambiguity is range spelling (`..=11`
  vs `..12`), where several Rust forms render one gloss and the checker accepts
  any. *Scope:* the round-trip verifies structure + primitive arguments, but for a
  `described` atom only its label (a closure body never appears in a gloss) —
  behavioral correctness is M4.2. The same model authored the spec and acted as
  compiler, so this proves sufficiency + guards `describe()` drift, not adversarial
  generalization (M4.2 tests that).
- ✅ **M4.2 Verification harness.** Given a candidate `Constraint`, check it
  compiles and matches intent over random hands (and against the original rule
  when porting). *Deliverable:* a verifier. *Measure:* catches deliberately-broken
  constraints. *Deps:* M4.1. **Done:** `bidding::verify` (ungated) — where M4.1's
  round-trip is a *string* compare (`describe() == gloss`), this is a *behavioral*
  one. `compare(reference, candidate, rng, n)` samples `n` random hands (crisp
  accept = finite logit) and returns a `Report`: accept rates + a bounded sample
  of counterexample hands. `accepts`/`predicate` adapt a `Constraint`; a book
  `Rule`'s public `eval` is the porting oracle (`compare_against_rules`);
  `check_examples` scores against hand labels. `tests/dsl_verify.rs` is the
  measure — it catches the canonical "5+ ♥"→`len(♥, 4..)` break, off-by-one bands,
  swapped `&`/`|`, dropped/extra clauses, and a `described` closure with `>` where
  intent is `≥` (the escape-hatch body the round-trip cannot see — the reason M4.2
  exists), while faithful recompiles agree. `examples/verify-constraint` runs the
  author-verify loop on the real 1♠ opening (faithful → 0 disagreements; broken →
  caught, every witness a four-card spade hand) plus the escape-hatch blind spot.
  *Decisions:* fixed (caller-supplied, default-empty) `Context` — the dominant
  disagreements and every `described` hand predicate are context-free; sampling is
  strong evidence, not proof, so `n` is taken large (tests/example use 8000).

Exit M4: book authoring is "write the meaning, verify, commit" — the compiler +
verifier accelerate extending and refining the 2/1 books.

---

## Milestone 5 — Component A, Role 2: meaning-aware policy

The portability dream. Last, because it needs the most prerequisites.

- ✅ **M5.1 Tag features.** Feed the discrete `tags` per prior call into the
  policy as categorical inputs. *Measure:* no regression; ideally a small gain.
  *Deps:* M0.2, M1. **Done:** `bidding::tags` (shared structural reader, lifted
  from `export-corpus`), `features_v2` (244 = 160 + last-4-calls × 21-tag
  multi-hot, version 2), `classify_v2`/`NeuralFloorV2`/`american_neural_v2`
  (gated), layout-agnostic trainer + `teacher-dump --features-version 2`.
  **Result (20k-board A/B, vul none):** distillation fidelity up (teacher top-1
  95.0% vs v1 93.8%, val CE 0.235 vs 0.249) but **IMPs/board at parity vs v1**
  (−0.016, CI [−0.039, +0.007]); floor worth preserved (+0.540 vs bare). The
  teacher is the ceiling for pure distillation — the tag inputs are now in place
  to pay off when distilling the search target (M3.2).
- ⬜ **M5.2 Sequence-model policy.** Move Component B to a small transformer over
  the call sequence. *Measure:* matches or beats the MLP on the harness. *Deps:*
  M1 (as baseline).
- ⬜ **M5.3 Meaning encoder.** Embed each prior call's text description as a
  meaning vector and feed it to the sequence-model policy, so the system enters the
  net as *meanings* rather than baked-in weights. *Measure:* matches or beats the
  tag-feature net on the 2/1 harness. *Deps:* M5.2. *Note:* the longer-term payoff
  — one net bidding *any* system from its written notes — needs training data
  spanning more than one system to be measurable; with the codebase now 2/1-only,
  that cross-system measurement is out of scope until a second system's corpus
  exists.

Exit M5: the 2/1 policy is driven by written meanings rather than baked-in
weights, laying the groundwork for cross-system portability.

---

## Milestone 6 — Deeper deterministic floor (inference + conventions)

Motivated by the BBA floor study ([`bba-floor.md`](bba-floor.md)): BBA's floor
is parametric and fires conventions (the probe caught `4NT = Blackwood` on a
depth-8 auction), whereas pons's `instinct()` is all-natural and stalls below
slam off-book. Smarten the keyless floor directly — the baseline every A/B
measures against — *without authoring a node per sequence*
(`feedback_instinct_floor_over_node_authoring`); `instinct()` stays default and
the rails stay green. Each chunk's measure is IMPs/board on the `instinct-floor`
A/B vs baseline, and the BBA gap (S.1's −2.6) on the relevant auctions.

- ✅ **M6.1 Parametric auction inferences.** Push the floor deeper by *deriving*
  facts from the auction rather than authoring, via the existing `Inferences` /
  `inference.rs` reader. Canonical case: `1NT–2♦–2♥–4♥` — responder transferred
  (5+♥) then jumped past the choice-of-games `3NT` to `4♥`, so the floor can
  *know* a 6-card major and act on it. *Deliverable:* a few derived inferences
  the floor reads on demand. *Measure:* no regression, ideally a gain on
  transfer/limit auctions. *Deps:* none (reuses `inference.rs`). **Done:** a
  post-walk `transfer_major_reading` in `inference.rs` (the generic walk
  suppresses the artificial transfer + completion, so it is derived after the
  fact, like the Rubens cue): a completed Jacoby major transfer → 5+ in the
  major, a follow-up jump-to-`4M` or raise-to-`3M` → 6+ (the `3M` raise also pins
  invitational 8–9, mirroring the Stayman raise); both majors, 1NT + 2NT,
  uncontested. Plus a **six-two arm** in `instinct()`'s `known_major_fit`
  (`len(major,2..) & partner_shown_len(major,6..)`) so opener acts on the shown
  six opposite a doubleton — the exact gap `project_sat-slam-try` flagged.
  Verified off-book by `classify_with_provenance` (not shadowed —
  `project_floor_shadowed_by_book_nodes`): `1NT–2♦–2♥–3♥`/`–4♥` fire the floor,
  and a max accepts `1NT–2♦–2♥–3♥` → `4♥`. **A/B** (seeded constructive,
  `stayman-abc` harness, baseline vs M6.1, opponents silenced, 200k boards):
  **+1.94 IMPs/divergent vul none, +2.25 vul both** (306 divergent, +0.003
  IMPs/board); whole inference floor still +0.05 IMPs/board (`inference-floor`,
  20k). Length-only on the `4M` jump (slam machinery is M6.4); the derived 6+ also
  makes the sampler sound on transfer auctions.
- 🟡 **M6.2 Rule projection — read a call's meaning off its rule.** Full design:
  [`rule-projection.md`](rule-projection.md). The seven `*_reading` decoders in
  `inference.rs` (M6.1's `transfer_major_reading` among them) re-derive, by hand
  and per-convention, what an authored call's `Constraint` already states. Add a
  third fold on the DSL — `Constraint::project(context) -> Inference`, the forward
  dual of `eval` — and a single generic pass that, walking `context.prefixes()`,
  projects the rule of each artificial prior call (artificial = its projection
  floors a suit it did not name), replacing the per-convention decoders. Promoted
  ahead of more conventions so the retire-readers cleanup lands *before* M6.3
  piles on more hand-written readers. *Deps:* M4 (the DSL), M6.1 (the reader it
  generalizes).
  - ✅ **M6.2a `Constraint::project` fold + soundness property test.** Shipped
    2026-06-25. `len` keeps both bounds, `points`/`hcp` floor-only (sound in both
    fuzzy modes), `&`→intersect, `|`→union, opaque/`!`→no-info, default no-info
    (non-breaking). Invariant `eval` finite ⟹ hand ∈ `project`, tested over ~32k
    hands. `Inference::intersect`/`union`, `Range::union` added. The data
    substrate; no consumer wired yet.
  - ✅ **M6.2b Validate the projection pass reproduces the readers.** Shipped
    2026-06-25. `Rule::project` (the reading-side fold, mirrors `Rule::describe`) +
    the generic `authored_reading` pass (`#[cfg(test)]`-only: walk
    `context.prefixes()`, project each artificial call's rule, narrow the bidder's
    seat) + an equivalence test proving the pass reproduces the three declarative
    readers — `transfer_major`, `leaping_michaels`, `landy` core — *exactly*
    (signature suit lengths and points) on prefixed contexts built from the real
    book via a `#[cfg(test)] Stance::prefixed_context` seam. No production wiring,
    no deletions, no behavior change — the mechanism is proven before the
    cross-cutting refactor.
  - ✅ **M6.2c Wire + retire the declarative readers — SHIPPED 2026-06-25.** The
    keyless leak turned out to be a *single* production site: `SearchBook::classify`
    (`search_floor.rs:241`) re-derived `Context::new` with no prefixes, feeding both
    `features` and the EV sampler's `Inferences::read`; the floors are `Classifier`s
    that already receive the book's prefixed context, and the other keyless
    `Inferences::read` callers are all `#[cfg(test)]`. So `Stance::prefixed_context`
    is made real, `SearchBook` prefixes itself with it, and `Inferences::read` folds
    `project_authored` in — the `project` artificial-detector drives *both*
    suppression and recording, so `transfer_major_reading`,
    `leaping_michaels_reading`, and `landy_reading` are **deleted** (only the Landy
    advancer-relay survives as a `landy_advance_suppress` stub). *Payoff is
    architectural* (single source of truth; lets rule-replay stand alone). Two sound
    reading changes fall out: a completed transfer pins its *five*-card floor (the
    old reader's six-card jump upgrade drops — a natural-suit raise is outside the
    projection's artificial-only scope), and Woolsey's `2♣` reads its true **4-5**
    majors. `instinct()` bids by rule and is unchanged; only search bidders read the
    projection. Gate: `ab-landy` reproduces its DD-negative value (reading proven
    byte-identical by the M6.2b equivalence test); `ab-search-floor` no gross
    regression.
  - ✅ **M6.2d Stage 4 — re-author the opaque conventions. SHIPPED 2026-06-25.**
    DONT/Woolsey/Multi were authored with the opaque `described()` escape hatch, so
    they projected no info and the detector could not see them. Instead of bare `len`
    conjuncts, added higher-order **`or`/`and` suit-set length combinators**
    (`constraint.rs`: `and` floors every named suit — tight; `or` unions the arms —
    loose) so each convention states its lengths declaratively *and* projects off its
    own rule. Re-authored all seven shapes with them, each guarded by `verify::compare`
    (8k hands) against its intended spec. Per the user, switched to the **simpler /
    traditional shapes** (a measured behavior change, not a wash): **DONT default → 4-4**
    (`set_direct_dont_four_four` flips on; Landy stays 5-4, Michaels 5-5), and Multi
    `2♦` drops its longer-major / no-6-6 guard. Muiderberg keeps its exactly-5 +
    other-major caps — the Woolsey structure needs disjoint shapes (uniform 1.9 weights
    never tie). Recordings in `inference.rs` unchanged (floors already match); the
    both-majors family now also auto-detects via projection (idempotent). **A/B**
    (`ab-landy` 60k, none-vul, vs natural): DONT 4-4 −0.362/div plain, −1.397 PD
    (obstruction wall, single-dummy value); Woolsey +0.414/div plain, +0.065 PD. Both
    stay opt-in. The relay-suppression stubs (`landy_advance_suppress`, the
    `multi`/`woolsey_x`/`dont` suppressions) survive as designed.
- 🟡 **M6.3 Competitive conventions on the floor.** Already the active line for
  ~25 commits — the deliverable is the 1NT-defense + competitive-double structure
  that shipped, not the old "Rubens advances" sketch. *Landed:*
  - **Natural penalty-X + natural overcalls** (`set_natural_defense`, default-on)
    — the DD-positive baseline — with its floor reading.
  - **Conventional defenses, opt-in** because they are DD-negative (the obstruction
    wall): Woolsey "Multi-Landy" (`set_woolsey`) and direct-seat DONT
    (`set_direct_dont`), each with a suppress-and-narrow floor reading
    (`dont_reading`, Woolsey takeout-X reading).
  - **Passed-hand both-majors X** of their 1NT (DD-positive, promoted default-on).
  - **`[1NT,(X)]` runout** (default-on) + Phase 2 (encircling penalty-X of the
    escape, direct minor escape via `set_unusual_2nt`).
  - **Double styles:** responder's X of a 1NT overcall now Optional by default
    (bf6e5cd) + the optional-latch knob; the defensive `(1NT)-X-(2Y)-X` latch
    (`set_latch_style`, opt-in, DD-wash); the penalty-double latch (default-on).
  - **Transfer-Lebensohl / Rubinsohl** threads over interference.
  - **Rubens advances, knobbed + measured + tails authored** (the old sketch's
    unfinished *measure*, done 2026-07-02). `set_rubens_advances` (default-on;
    `--no-ns-rubens`) with a knob-off natural new-suit advance so the off arm is
    a fair baseline (the advancer otherwise had *no* call — `we_have_not_bid` is
    side-level). Round 1 (204.8k bd, 2217 fired) lost plain −0.0111/PD −0.0240,
    concentrated in unauthored tails: 26% of divergent boards *passed out the
    two-level cue-raise in their suit*, opener's X silenced the completion
    (phantom suit doubled), and double-then-bid structures were misdetected.
    Tails authored (cue answer signoff/game, completion through the X,
    `overcall_shape` requires the side's first action): round 2 (fresh seeds,
    1378 fired) **plain +0.0012 ± 0.0016 (wash), PD −0.0029 ± 0.0019**. Round 3
    added the **both-sides continuations** (into-partner completion graded
    2Y/3Y/game, raiser drives with 14+, new-suit completion = the
    would-pass-a-natural-NF-2T hands with fit-raise breaks, transferee
    invite/game rebids): **plain +0.0016 ± 0.0015 (win, CI excludes 0), PD
    −0.0009 ± 0.0017 (wash)** at 1144 fired — the USP structure beats natural
    once completely authored; **default-on stands**. The one-level transfers
    also now *record* their meaning (`set_rubens_transfer_reading`, default-on,
    own-side only): +0.0005 ± 0.0004 plain, +2.55/fired — closing the
    suppress-only asymmetry with the cue-raise.
  *Measure:* contested IMPs/board vs baseline + vs BBA, **but** the DD harness is
  blind to obstruction (`project_preemption-dd-negative`,
  `project_bba-1nt-comparison`), so most conventional defenses are kept opt-in and
  the real gap is single-dummy; constructive competition still wins on DD. *Deps:*
  none. *Note:* verify the floor rule fires and isn't shadowed
  (`project_floor_shadowed_by_book_nodes`); contested is where the learned floors
  already live (`project_floors_contested_only`).
- ✅ **M6.4 Slam machinery on the floor — SHIPPED 2026-07-02 (five A/B rounds).** Slam
  bidding is inherently conventional and arises in the deep auctions the floor
  owns. The RKCB precondition was declared met (the book's 1430 reuse kept
  measuring wins: minor keycard +6.80, Jacoby slam try +1.42, Texas drive
  +5.87 per fired), so both halves shipped together, each behind a default-on
  knob:
  - **Floor RKCB 1430** (`set_floor_rkcb`, `--no-ns-floor-rkcb`): at combined
    33 with a known eight-card fit the floor asks `4NT` (outweighing the direct
    milestone 6), partner answers the shared 1430 ladder
    (`american::slam::count_keycards`), and the asker signs off at five /
    bids six / bids seven-at-37 by the combined count — **instinct decodes
    instinct**, no trie node installed. Trump is *derived* (max of our length
    + partner's shown floor reaching eight; the answerer falls back to
    partner's shown 5+ suit), BWS's "agreed suit makes 4NT keycard" — a 4NT
    raise of our own notrump stays quantitative on both sides of the seam.
    Ask/answers alerted `floor:rkcb` (projection suppresses the phantom
    suits); the answerer always respects the asker's placement. No floor 5NT
    king ask (grand rides the 37 milestone) — the named ceiling.
  - **Control-bid reading** (`set_control_bid_reading`,
    `--no-ns-control-bid-reading`): the deterministic rule, calibrated by the
    A/B to what the system *actually bids* — an undisturbed four-plus-level
    new suit is a **control bid iff the bidder bypassed it** (biddable more
    cheaply at their first suit-showing call: `1♦–1♠–2♦–4♥` agrees diamonds;
    `1NT–2♥–2♠–4♥` through the transfer overlay agrees spades — suppress the
    phantom, record support 3+/own 6+ and 13+ points).  A suit *above* the
    first-shown one was **never denied → natural 6+**: the book responds 1♥ /
    transfers to hearts holding 6♠5♥ (probed), so `1♣–1♥–2♣–4♠` and the
    post-transfer `1NT–2♦–2♥–4♠` are to-play — round 1 read them as controls
    ("shown another suit ⟹ can't be longest") and bled −6.1/fired pulling
    natural 4♠s.  Silent bidder → natural (`1♦–4♥` floored 6+) except below
    partner's major game (`1♥–4♣` splinter-possible, unread); undeniable
    minors unread.  Plus the **never-pass-a-cue signoff** (return to trump at
    the cheapest level) — the Rubens round-1 lesson applied up front.
  *Measure:* 204.8k boards vs BBA per round, paired `ab-dump-diff`, both knobs
  off as the baseline arm.  **Round 1 (naive longest-suit rule +
  fit-only answerer trump, `SEED_BASE=1782987009`): plain −0.0030 ± 0.0009,
  −6.1/fired (100 fired)** — two leaks: natural 4♠s-above-the-first-suit
  pulled to the "agreed" minor, and 4NT asks *passed out* when the answerer
  could not derive the trump (opener's unsupported rebid suit) — fixed by the
  bypass rule + widening the answerer's fallback to either seat's shown 5+
  suit.  **Round 2 (`1782987977`): plain −0.0002 ± 0.0003 (wash), −1.35/fired
  (26 fired), PD identical** — the residue was two more leaks, both fixed:
  the ask must be *decodable* (trump = a shown-5+ suit, else partner passes
  out the 4NT — an 8-fit against a four-card Puppet answer is known only to
  the asker), and the respect-signoff pass steps aside when our ambiguous
  5♣/5♦ answer held the **high** count (answerer corrects with a maximum —
  suppressing that correction cost 11 IMPs a board).  **Round 3
  (`1782988401`): plain −0.0003 ± 0.0002, −1.71/fired (31 fired)** — two
  final holes, both at the book-ask/floor-answer seam: the {1,4}/{0,3}/{2}
  ladder (the book's too!) has **no answer for five keycards** (a 2♣ rock
  passed out its raiser's 4NT) → the floor's 5♣ now covers {1,4,5}; and the
  respect-signoff kept suppressing *winning* corrections over the book's
  pessimistic signoffs (after a 5♥ answer with two own keycards the total is
  four, one missing, yet the book stops) → respect narrowed to **our own
  count ≤1** (the only case the total provably cannot be slam-safe),
  subsuming round 2's high-ambiguous carve.  **Round 4 (`1782988848`): plain
  −0.0003 ± 0.0003, −2.40/fired (25 fired)** — the convergent finding: every
  remaining loss was the machinery rerouting combined-33 hands *away from the
  milestone 6NT power-blast* (minor-trump asks driving 6♦ where 6NT makes;
  the natural-6+ recording feeding the milestone's 6-2 arm into thin 6♥
  slams; a correct 5♦-signoff where 6NT still made on power — DD monetizes
  honors at 33+, keycard discipline pays only on real major fits).  Round-5
  cuts: `keycard_trump` restricted to **majors with 3+ our side**; the
  to-play reading **records nothing** (pre-M6.4 envelope — flooring the six
  is what created new 6-2 reroutes); the control-bid witness moved onto
  `Inferences::control_bid` (exact — "named suit unread" can no longer tell
  control from to-play).  **Round 5 (`1782989478`): 4 fired / 204.8k, delta
  exactly 0.0000 plain and PD** — the four divergences are all
  `2NT–3♥–3♠–4NT–…–6♠` checking keycards into a real six-card fit where the
  baseline blasts an equal-value 6NT.  **Wash with the safety net kept →
  default-on stands** (plain-wash policy).  *Meta-lesson:* on plain DD at
  33-plus combined, the 6NT power-blast is near-optimal — keycard discipline
  buys almost nothing the milestones don't already have; the durable value of
  M6.4 is the *reading* (control bids never passed out, keycard answers never
  read as phantom suits) at the off-book seams.  *Wrinkle stands:* the
  floor-meets-book seam (a booked partner reading a floor 4NT) is guarded
  only by the trump-derivation and quantitative gates; full gating deferred.

Exit M6: the deterministic floor explores slam and handles the key competitive
conventions, narrowing the BBA gap in exactly the deep/contested auctions where
the books are thinnest — by deriving and generalizing, not by enumeration.

---

## Milestone 7 — Search at every leaf (rules propose, DD disposes)

Full design: [`05-search-at-every-leaf.md`](05-search-at-every-leaf.md). Today an
authored leaf is the final word: `Trie::classify_floored` returns a book node's
logits verbatim when they have mass, and the live double-dummy search
(`SearchFloor`) is wired only as the contested-book floor — so DD search runs
*only where the book is silent*. Leaping Michaels proved the upside of crossing
that line by hand (cap the authored advance at game, decode the convention,
+2.8 IMPs/board for search over the rule floor). M7 generalizes it: an authored
bid encodes *meaning* (its constraints), and DD search makes the *judgement* (its
weights). The leaf's logits become a *prior*, not a verdict — fed through the
existing `shortlist → ev_all → blend` seam. All §0 safety invariants are
inherited verbatim; `instinct()` and `american()` are untouched; the new bidder is
opt-in behind `search`.

- 🚧 **M7.0 Search-aware classification path** (wiring shipped, commit 496260b;
  the as-is measure **regresses** — see the result below, blocked on M7.1). Add a
  path so a resolved *book*
  leaf with mass (and not forced) feeds its logits as the search prior instead of
  being terminal, reusing `shortlist`/`ev_all`/`blend` unchanged. Candidate set =
  **book finite calls ∪ neural top-k**, so DD can override a one-call rule.
  *Deliverable:* a new gated constructor (e.g. `american_search_book()`) alongside
  `american_search()`; a `Pair`/`Trie`-level wrapper, not a `Trie::classify_floored`
  rewrite if avoidable. *Measure:* parity-or-better vs `american_search()` on
  contested (`search-floor` harness); the `instinct` rails stay green (forced →
  deterministic, before any search). *Deps:* none (the seam exists). **Done:**
  `SearchBook` — a `System` wrapping a *bound* `Stance` (search_floor.rs), plus
  `american_search_book(them)`. It runs the search at every **non-forced authored
  book leaf** (`provenance.fallback == None`, with mass), feeding the leaf logits
  through the existing seam: candidate set = the rule's finite calls ∪ the net's
  top-`k`, `ev_all`-priced, `blend`ed back over the leaf prior. The reusable core
  (`price_and_blend`) was *extracted* from `SearchFloor::classify` — byte-identical
  refactor, the old `deterministic_given_a_decision` test is the guard — so both
  search bidders share one EV-pricing path. Rails inherited verbatim: a `forced`
  auction delegates to the wrapped stance (no search), and an auction that falls
  past the book to a fallback floor (the `SearchFloor` on contested, `instinct` on
  constructive) is returned as that floor gave it — only a real authored leaf is
  re-priced. *Key:* the authored leaf still owns the **meaning** — an opening keeps
  `Pass = -∞`; DD re-judges only among the calls the rule (∪ net) proposes, never
  resurrecting a call the agreement forbade. Four gated rails/determinism tests
  green; `examples/search-book` is the A/B harness (`SearchBook` vs `american_search`
  for the parity verdict, vs `american` for the deterministic reference). **It is
  the M7 *treatment arm***: keep both names during M7 to measure; on a win, collapse
  leaf-wrapping into `american_search` as a default-on knob and delete `_book`
  (gate per book if it splits contested vs constructive). **RESULT (120 boards, vul
  none, seed 1, perfect-defense measure — failing contracts doubled, the corrected
  `ns_score`):** leaf-pricing **regresses −2.958 IMPs/board vs `american`** (CI
  [−4.605, −1.312], excludes 0 — a clear loss) and **−1.700 vs `american_search`**
  (CI [−3.552, +0.152] — point estimate firmly negative, the wider SE leaves 0 just
  inside). The win condition FAILS. (For reference, the earlier *optimistic*-bound
  numbers were −1.133 / −1.925; perfect defense makes the loss vs `american` worse,
  as expected — it doubles the overbids. Baseline context: `american_search` itself
  is only −1.050 vs `american` under perfect defense, CI [−2.237, +0.137] — the live
  search's edge over the floor was a scoring artifact, so leaf-pricing builds on a
  shaky base.) The divergent dump is unambiguous: all three
  worst losses are leaf-pricing reaching a **redoubled grand (`7♣xx`)** that fails,
  in wild *competitive* auctions. This is the **M7.1 soundness gate** biting (§3 of
  05-search-at-every-leaf.md): competitive leaves are mostly *undecoded*, so the
  sampler deals partner ranges too wide → 7♣ "makes" on the biased layouts → DD
  picks it → it dies at the real table. Same pathology as
  [[project_m31-7nt-sacrifice-instability]] and [[project_preemption-dd-negative]]
  (DD/perfect-defense is blind to obstruction and rewards sacrifice/escalation real
  defense punishes). **Lesson: M7.1 is a hard prerequisite, not a quality nicety —
  do NOT wrap competitive leaves before decoding them.** Next: M7.1 decode sweep +
  the design's explicit fallback (no usable decode → keep authored logits, skip the
  search), or first test M7.2 constructive-only leaves where there is no competitive
  escalation. The bidder stays gated/opt-in; `american()`/`instinct()` unchanged.
- ⬜ **M7.1 `Inferences::read` completeness sweep.** The soundness gate: DD EV is
  only as good as the decode, since the sampler conditions on `Inferences::read`
  ranges. An undecoded convention widens partner's range (sound but biased EV —
  never a crash, `inference.rs` is superset-by-construction), so quietly-weak EV is
  the failure mode. Audit every authored convention; add the missing `read` arms
  (same post-walk seam as `leaping_michaels_reading` / `transfer_major_reading`).
  *Deliverable:* a decode per convention, + the explicit fallback "no usable decode
  → keep the authored logits, skip the search here." *Measure:* a sampler-soundness
  check per convention; A/B per decode. *Deps:* none, but **gates M7.0/M7.2
  quality** (not correctness).
- ⬜ **M7.2 Extend to constructive leaves.** Wrap constructive book nodes too — the
  literal "every leaf." Re-test the `project_floors_contested_only` boundary: DD-
  pricing the *authored candidates* is a different experiment from putting the raw
  *net* on constructive (which lost 0.8 IMPs/board), so it must clear its own A/B
  before shipping. *Measure:* constructive A/B (`constructive-abc` template); expect
  gains in reach (games/slams), not in light competition — the DD harness is blind
  to obstruction (`project_preemption-dd-negative`). *Deps:* M7.0, M7.1.
- ⬜ **M7.3 (optional) Continuation policy = full system.** The rollout finishes
  with the bare distilled net (`POLICY`), not book+floor, so a *book* leaf is
  priced assuming the *net* continues — a fidelity mismatch. If M7.0/M7.2 leave
  measurable EV bias, swap the rollout continuation to the book+floor system.
  *Measure:* IMPs/board delta vs the bare-net continuation. *Deps:* M7.0.

Exit M7: every authored leaf where judgement matters is priced by double-dummy
cardplay, not a fixed weight — the system reaches the contracts the specific cards
are *for*, while the authored constraints still carry the meaning partner relies
on.

---

## Side-track S — External reference bidder (BBA / EPBot)

Optional, parallelizable, **pure tooling** — never touches the default build, the
`instinct()` baseline, or any invariant. Edward Piwowar's BBA/EPBot is a mature,
rule-based, ~100%-reproducible engine; we drive it as a black box (native
`libEPBot.so` via FFI — see S.0). It plugs into three existing slots, strongest first:

- ✅ **S.0 Feasibility + harvest harness.** *Original plan was Wine (WineHQ +
  wine-mono / `dotnet48`) shelling out to the .NET binary; superseded by a
  cleaner route:* a native `libEPBot.so` linked directly via FFI (`libloading`,
  no Wine, no subprocess, no PBN round-trip). A Rust `BbaOracle` deals with
  `full_deal` and drives EPBot in-process. *Deliverable:* `examples/bba-oracle`
  round-tripping deals → BBA → `Auction` — done. *Measure:* zero parse errors;
  auctions spot-checked against hands. The ABI (`set_bid`,
  `epbot_get_cards`, the `T` ten encoding) was decompiled and confirmed here and
  generalized in S.1. *Deps:* none (external tool). **Done:** commit `6f4f70d`.
- ✅ **S.1 Eval anchor (feeds every milestone's measure).** A/B duplicate match,
  our `american()` vs **BBA's 2/1** card — apples-to-apples, so divergences are
  pure quality gaps in our DSL, not system differences. Reuses the `instinct-floor`
  / `scoring.rs` / `ddss` harness. *Deliverable:* IMPs/board (ours vs BBA) +
  divergence-board dump. *Measure:* a CI excluding noise; the dump names concrete
  under-bidding auctions. *Deps:* S.0. *Value:* turns "did we improve?" into "how
  far from a mature engine?" — calibrates the M1/M3 gains. **Done:**
  `examples/bba-match` — `BbaOracle: System` drives EPBot system 0 ("2/1GF - 2/1
  Game Force", verified by name), one fresh bot per decision (S.0 ABI generalized:
  `set_bid(bot, position, bid, meaning)` and `set_system_type(bot, position,
  system)` decompiled + confirmed; the ten is EPBot-canonical `T`; the dealer is
  canonicalized to position 0 so `classify` is pure in `(hand, vul, auction)`).
  Reports IMPs/board with a 95% CI + the worst divergent boards. **2000 boards, vul
  none: −2.59 IMPs/board, CI [−2.83, −2.35]** — our floor trails BBA's mature 2/1
  by ≈ 2.6, the gap concentrated in competitive/contested auctions (the thinnest
  part of the books). 371 tests green; `libloading` stays a dev-dependency, default
  build untouched.
- ⬜ **S.2 (optional) Imitation teacher for M3.** BBA's calls as an extra,
  cheap/deterministic target alongside the M2.3 search teacher. *Caveat:* imitating
  BBA is capped at BBA — it cannot *exceed* a human system the way the double-dummy
  search teacher can, so this is a sanity/regularizer signal, **not** the path to
  "beat the floor." *Deps:* S.0, M3.1.

Slots in: S.1 → eval harness (now) · S.2 → M3 (optional).

---

## Critical path and what to do first

```
M0  ──► M1 ──────────────► (working learned floor, = teacher)
  │       │
  │       └─► M2 ──► M2.3 ──► (gated live search bidder: net+search > raw net)
  │              │      │
  │              │      ├─► M3 ──► (distill it → fast default floor > teacher)  ← the real goal
  │              │      │
  │              │      └─► M7 ──► (search wraps authored leaves: rules propose, DD disposes)
  │
  └─► M4 ─────────────────► (faster 2/1 authoring)
            │
            └─► (with M5.2) ─► M5 ─► (meaning-driven 2/1 policy)  ← the dream

M6 (deeper deterministic floor) ─► feeds M7's decode sweep (Inferences::read)

S (BBA/EPBot) ─► external eval anchor (now) · teacher → M3 (optional)
```

**Recommended first chunk:** all of **M0**. It is pure bridge/Rust, unblocks
every branch, and produces three durable assets (corpus, feature spec, teacher
dataset) that survive any later change of ML mind. After M0, **M1** is the
smallest path to a real "the machine bids" result, and **M4.1–M4.2** can run in
parallel since they only need the corpus.
</content>
