# Phased plan

Small, well-specified, individually measurable chunks; each milestone names a
**deliverable**, a **measure**, and its **deps**. A map, not a green light —
nothing starts until explicitly chosen.

Legend: ⬜ not started · ✅ done.

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
  with `two_over_one()` and record `(features, teacher_softmax)` at each decision,
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
  candle workspace, `exclude`d from the package) → `two_over_one_v1.{f32,json}`,
  a 160→256→256→38 MLP distilled from `two_over_one()`. 80 epochs, val CE 0.249,
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
  **Done:** `bidding::neural_floor::NeuralFloor` + `two_over_one_neural()`. Forced
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
  callers to the deterministic `two_over_one()` for debuggable validation (and ≈
  the M1 net at bootstrap); M3.2 swaps in successive nets with no change to this
  code.
- ✅ **M2.3 Live search bidder (gated).** Wrap M2.2 as a runtime
  `Classifier`/`System`: at each non-forced decision, use the net's softmax as a
  prior to shortlist the top-`k` legal calls, run `ev` over sampled layouts,
  return a distribution peaked on the high-EV calls — behind a `search` cargo
  feature, wrapped in the same forced-rails shell as `NeuralFloor`. This *is*
  "simulations in action": the policy simulates before it bids. *Deliverable:* a
  feature-gated `two_over_one_search()` / `SearchFloor`. *Measure:* A/B IMPs/board
  vs the deterministic floor (strictly positive) **and** vs the distilled net
  (search should beat the raw policy), over a board count large enough to exclude
  zero; the five §0.4 rails tests stay green against the shelled search bidder.
  *Deps:* M2.2, M1 (net as prior/policy). *Decisions:* bidding only; slow & gated
  is acceptable (knobs — `n` layouts, `k` shortlist, EV temperature — default to
  strength, not latency); the default build and `instinct()` baseline are
  untouched. **Done:** `bidding::search_floor::SearchFloor` + `two_over_one_search()` behind
  the `search` feature (⊇ `neural-floor`). Shell mirrors `NeuralFloor` (`forced` →
  `instinct()`); the judgement middle masks the net prior, shortlists top-`k = 8`,
  prices them with `ev_all` over `n = 128` layouts, and re-seats onto an EV-ranked
  band (`prior_max + 3` nats, EV-temp `100` pts/nat; `Pass` and the un-evaluated
  tail stay finite; all-`NaN` → bare net). *Decisions:* continuation policy is
  **neural self-play** (`two_over_one_neural()` all four seats — the policy M3.2
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
  (`two_over_one`) and the raw net prior (`two_over_one_neural`), split off/on-book
  and contested/constructive. 40-board smoke: **~51 % arg-max disagreement, ~0.53
  mean TV off-book vs `0`/`0` on-book** (identical book logits by construction); all
  off-book rows contested (the floor sits only under the competitive/defensive
  books). The additive `two_over_one_search_with(SearchFloor)` constructor (gated
  `search`) exposes `--layouts`/`--shortlist`/`--temperature`; the full production
  dataset feeds M3.2.
- ✅ **M3.2 Train + iterate.** Retrain toward the search target; feed the improved
  net back into M2.2's continuations; repeat. *Deliverable:* successive nets.
  *Measure:* each round's A/B IMPs/board vs the prior net — **accept only gains**.
  *Deps:* M3.1. **Round 1 done:** trained a v1-featured net on the 10 000-board
  `search-dump` (97 701 rows, git_sha `1d43577`) toward the search softmax —
  `neural::classify_search`, the `NeuralFloorSearch` shell (same forced-rail
  delegation + legality mask), and `two_over_one_neural_search()` (gated
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
  replaced the production search net in place (`two_over_one_neural_search()` is now
  round 2 everywhere; the temporary comparison wiring was reverted).
- ✅ **M3.3 Champion.** The best net by harness score becomes the optional neural
  floor. *Measure:* strictly positive IMPs/board vs the deterministic floor, with
  a board count large enough to exclude zero. *Deps:* M3.2. **Done:** the round-2
  search net is the champion — on the default perfect-defense measure it beats the
  deterministic floor at 20 000 boards by +0.178 vul none (CI [+0.075, +0.282]) and
  +1.716 vul both (CI [+1.608, +1.824]), and is positive on the optimistic
  double-dummy bound too (+0.123 / +0.583). It is the in-place production search net
  (`two_over_one_neural_search()`, gated `neural-floor`); `instinct()` stays the
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
  `describe()` rendering, a vocabulary table for all 23 primitives (exact gloss +
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
  generalization (M4.2/M4.3 test that).
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
  M4.3 loop on the real 1♠ opening (faithful → 0 disagreements; broken → caught,
  every witness a four-card spade hand) plus the escape-hatch blind spot.
  *Decisions:* fixed (caller-supplied, default-empty) `Context` — the dominant
  disagreements and every `described` hand predicate are context-free; sampling is
  strong evidence, not proof, so `n` is taken large (tests/example use 8000).
- ✅ **M4.3 Polish Club port (assisted).** Use M4.1+M4.2 to author the Polish Club
  books from their written notes. *Deliverable:* a second system's books + corpus.
  *Measure:* the ported system bids textbook auctions correctly; produces the
  second corpus needed for Component A Role 2. *Deps:* M4.2. **Done (constructive
  backbone):** `bidding::polish_club` — `polish_club()` / `bare_polish_club()`
  (`Family::POLISH_CLUB`), the *Strawberry Polish Club* (<https://polish.club>),
  authored from chapter sources with the M4.1 spec + M4.2 `verify`. Opening ladder:
  three-variant forcing 1♣, natural 1♦, five-card majors, inclusive 15–17 1NT,
  Ekren 2♣, Multi 2♦, Muiderberg 2♥/2♠, unusual 2NT, preempts. First responses:
  the artificial 1♣ framework (negative 1♦ relay + positives, forcing by omission),
  natural 1♦/1♥/1♠, shared 1NT reusing 2/1's notrump responses. Competitive book +
  deep relay tails floored by `instinct` (attached to all three books).
  `export-corpus --system polish-club` → **0-opaque** corpus (bespoke shapes via
  `described`); `tests/polish_club.rs` hard-asserts 8 textbook openings + 0-opaque +
  reach-game. `polish-club-reference` cross-checks vs BBA WJ (informational;
  notes authoritative): **86% opening agreement on the overlap** (1-level + Multi
  2♦), 1000 boards. **Defensive book (done):** from `Defense/` — NLTC-gauged
  overcalls + preemptive jumps, takeout double, 1NT, the Bailey cue (highest unbid +
  another), Unusual 2NT over a one-suiter; plain-HCP balancing (4-4-4-1 doubles,
  not a four-card overcall); Landy over their 1NT; natural-with-takeout over their
  weak two; takeout-flavored over Multi 2♦; principal advances. Adds the
  `nltc(range)` DSL primitive (faithful NLTC bands); `tests/polish_club_defense.rs`
  spot-checks 9 actions; corpus now **339 records**, 0-opaque.
  *Deferred:* opener's rebid relays, preempt response trees, the **Competitive
  book**, deep defensive transfer/relay tails (floored); BTU 1NT responses.

Exit M4: book authoring is "write the meaning, verify, commit" — and a second
system exists.

---

## Milestone 5 — Component A, Role 2: meaning-aware policy

The portability dream. Last, because it needs the most prerequisites.

- ✅ **M5.1 Tag features.** Feed the discrete `tags` per prior call into the
  policy as categorical inputs. *Measure:* no regression; ideally a small gain.
  *Deps:* M0.2, M1. **Done:** `bidding::tags` (shared structural reader, lifted
  from `export-corpus`), `features_v2` (244 = 160 + last-4-calls × 21-tag
  multi-hot, version 2), `classify_v2`/`NeuralFloorV2`/`two_over_one_neural_v2`
  (gated), layout-agnostic trainer + `teacher-dump --features-version 2`.
  **Result (20k-board A/B, vul none):** distillation fidelity up (teacher top-1
  95.0% vs v1 93.8%, val CE 0.235 vs 0.249) but **IMPs/board at parity vs v1**
  (−0.016, CI [−0.039, +0.007]); floor worth preserved (+0.540 vs bare). The
  teacher is the ceiling for pure distillation — the tag inputs are now in place
  to pay off when distilling the search target (M3.2).
- ⬜ **M5.2 Sequence-model policy.** Move Component B to a small transformer over
  the call sequence. *Measure:* matches or beats the MLP on the harness. *Deps:*
  M1 (as baseline).
- ⬜ **M5.3 Meaning encoder + cross-system training.** Embed text descriptions as
  meaning vectors; train across 2/1 **and** Polish Club. *Measure:* the *same* net
  bids both systems from their notes, each competitive with its single-system
  baseline on the harness. *Deps:* M4.3, M5.2.

Exit M5: one model, any system, driven by written meanings.

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
  our `two_over_one()` vs **BBA's 2/1** card — apples-to-apples, so divergences are
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
- ✅ **S.2 Polish Club reference (feeds M4.3 + M5).** Harvest BBA's **WJ (Polish
  Club)** auctions as ground truth for the M4.3 port and a head-start on the second
  corpus M5 needs. *Deliverable:* a WJ reference set + per-auction checks for the
  ported books via `bidding::verify`. *Measure:* the ported system agrees with BBA
  on textbook WJ auctions. *Deps:* S.0; pairs with M4.3. **Done (reference half):**
  `examples/bba-wj-reference` — WJ bidders (**EPBot system type 2**, confirmed from
  `WJ.bbsa`'s `System type = 2` *and* behaviorally: an 18-balanced hand opens 1♣)
  self-play random boards; every `(auction, call)` becomes a JSONL record with the
  hand and — for the **first round** — BBA's *self-reported meaning*: a systemic
  label (`"Polish 1C"`, `"Multi"`, `"5+ !H"`) **plus parsed constraint ranges**
  (point + per-suit length, straight onto the `Constraint` DSL). Meaning capture
  uses two FFI calls recovered by `objdump`
  (`epbot_get_info_meaning[_extended](bot, position, buf, len)`), reliable only for
  the **first four calls** — past position 4 the indices report per-seat hand
  inferences, so it captures positions `0..4` and drops the "no info" sentinel. 8
  **textbook Polish Club openings** double as the *measure*: the defining calls
  (strong-balanced→1♣, 15–17→1NT, 5-card majors→1♥/1♠) are hard assertions (green;
  BBA is ground truth), the rest recorded (a 6-spade weak hand opens **2♦
  "Multi"**; 1♦ shows **5+**). 21070 records / 2000 boards, ~38% with a meaning;
  output to `target/` (gitignored) with a versioned sidecar (system, seed, git SHA,
  schema, counts). 371 tests green (dev-only example; curated fixtures are the
  gate); `libloading` stays a dev-dependency, default build untouched. **Port half
  in M4.3:** `polish-club-reference` drives the *ported* books against WJ (86% on
  the overlap openings); this S.2 reference is what it diffs against.
- ⬜ **S.3 (optional) Imitation teacher for M3.** BBA's calls as an extra,
  cheap/deterministic target alongside the M2.3 search teacher. *Caveat:* imitating
  BBA is capped at BBA — it cannot *exceed* a human system the way the double-dummy
  search teacher can, so this is a sanity/regularizer signal, **not** the path to
  "beat the floor." *Deps:* S.0, M3.1.

Slots in: S.1 → eval harness (now) · S.2 → M4.3 / M5 · S.3 → M3 (optional).

---

## Critical path and what to do first

```
M0  ──► M1 ──────────────► (working learned floor, = teacher)
  │       │
  │       └─► M2 ──► M2.3 ──► (gated live search bidder: net+search > raw net)
  │              │      │
  │              │      └─► M3 ──► (distill it → fast default floor > teacher)  ← the real goal
  │
  └─► M4 ─────────────────► (faster authoring + 2nd system)
            │
            └─► (with M5.2) ─► M5 ─► (cross-system bidder)  ← the dream

S (BBA/EPBot) ─► external eval anchor (now) · WJ reference → M4.3/M5 · teacher → M3
```

**Recommended first chunk:** all of **M0**. It is pure bridge/Rust, unblocks
every branch, and produces three durable assets (corpus, feature spec, teacher
dataset) that survive any later change of ML mind. After M0, **M1** is the
smallest path to a real "the machine bids" result, and **M4.1–M4.2** can run in
parallel since they only need the corpus.
</content>
