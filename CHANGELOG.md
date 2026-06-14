# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] — Unreleased

### Added

- **Strawberry Polish Club defensive book** (AI-bidder **M4.3**, follow-up pass):
  `bidding::polish_club`'s previously-empty Defensive book is now authored from the
  system's notes (the `Defense/` chapters of <https://polish.club>), so our actions
  when the opponents open are answered by the system rather than the `instinct`
  floor. Over their one-of-a-suit opening: **NLTC-gauged** natural overcalls and
  preemptive jumps (a one-level overcall is `8.5–6.0` losers, a two-level `7.5–6.0`,
  a jump `9.5–8.0`), a takeout double (`≤7.5` losers, short in their suit), the
  15–18 1NT, the **Bailey cue** of their suit (the highest unbid suit plus another),
  and the **Unusual 2NT** (the two lowest unbid). A distinct **balancing seat**
  (plain-HCP, lighter, where a 4-4-4-1 doubles rather than overcalling a four-card
  suit), **Landy** over their 1NT, natural-with-takeout over their weak two, and a
  takeout-flavored structure over **Multi 2♦**, plus the principal advances (raising
  an overcall, advancing a takeout double, the Unusual 2NT, and responsive doubles).
  Deep transfer/relay advance tails (Rubens, Rumpelsohl, the Bailey-cue and Multi
  continuations) and the Competitive book stay on the `instinct` floor, as the
  constructive backbone leaves its relay tails. Every rule renders truthfully, so
  `export-corpus --system polish-club` is still **0-opaque** and now emits **339
  records** (up from 59). This adds one DSL primitive — **`nltc(range)`** (faithful
  NLTC bands, mirroring the existing `nltc_at_most`; `describe()`s as e.g.
  `6.0–8.5 NLTC`) — and `tests/polish_club_defense.rs` (9 behavioral spot-checks).
  No new crate dependencies; `instinct` stays the baseline and the floor.
- **Search-improved distillation targets** (AI-bidder **M3.1**): a new gated
  `search-dump` example (behind the `search` feature) that bids out random boards
  with the M2.3 live double-dummy search floor and records, at every decision, a
  training row of `(features, search_target)` — the improved call distribution the
  net is distilled toward in M3.2. The output is **byte-identical in layout to
  `teacher-dump`** (a flat little-endian `f32` file of `160 + 38 = 198` floats per
  row, a `.json` sidecar, and a `.tags` file), so the off-crate trainer consumes it
  unchanged; the only difference is the target, which the search improves on the
  teacher exactly where the books were silent (the file is a trainer-compatible
  *superset* of `teacher-dump`, identical on book nodes and upgraded off-book). The
  `.tags` byte gains a second bit (`bit1` = off-book / search fired, alongside the
  existing `bit0` = contested phase). The example also prints and records **the
  M3.1 measure**: at each row it classifies the deterministic teacher
  (`two_over_one`) and the raw net prior (`two_over_one_neural`) and reports, split
  by off-book/on-book and contested/constructive, the arg-max disagreement rate and
  the mean total-variation distance — confirming the targets differ from the teacher
  *mainly off-book* (on-book rows are `0` by construction; a 40-board smoke run shows
  ~51 % arg-max disagreement and ~0.53 mean TV off-book vs `0`/`0` on-book). A small
  additive constructor, **`two_over_one_search_with(SearchFloor)`** (gated `search`,
  re-exported at the crate root), lets data-generation runs trade strength for speed
  via the `--layouts`/`--shortlist`/`--temperature` knobs; `two_over_one_search()`
  is now exactly `two_over_one_search_with(SearchFloor::default())`. No change to the
  default build, the safety shell, or the `instinct`/`search_floor` rails; no new
  crate dependencies.
- A **second authored system, Strawberry Polish Club** (AI-bidder **M4.3**),
  exposed as `polish_club()` / `bare_polish_club()` (family
  `Family::POLISH_CLUB`) in a new `bidding::polish_club` module — the port half
  of the S.2 reference, authored from the system's own notes
  (<https://polish.club>, [source](https://github.com/jdh8/polish.club)) with the
  M4.1 DSL spec and the M4.2 `verify` harness. It is a *genuinely different*
  system from the existing `two_over_one_strawberry` (a `NATURAL`-family 2/1 with
  a few polish.club conventions); here 1♣ is the artificial small-club itself.
  This first pass authors the **Constructive backbone**: the full opening ladder
  (the three-variant forcing 1♣, the natural 1♦, five-card majors, the inclusive
  15–17 1NT envelope, Ekren 2♣, Multi 2♦, Muiderberg 2♥/2♠, unusual 2NT,
  three-level preempts) and the defining first responses (the artificial 1♣
  framework — the negative 1♦ relay and the positives, forcing by omission — plus
  natural responses to 1♦/1♥/1♠, with the shared 1NT reusing the verified 2/1
  notrump responses). The deep relay tails (Checkback Gladiator, Odwrotka, the
  strong-club rebid relays, the preempt continuations) and the Competitive and
  Defensive books are left to the `instinct` floor, which is attached to *all
  three* books (including the constructive one) so no uncontested auction strands;
  `instinct` stays the baseline and this is an added system, never a removal. Every
  authored constraint renders truthfully (the bespoke shapes use the `described`
  escape hatch), so `export-corpus --system polish-club` emits a **0-opaque**
  second corpus (the artifact M5 needs); `export-corpus` gained a
  `--system {two-over-one|polish-club}` flag (default unchanged) and a `system`
  field per record. A new `polish-club-reference` example cross-checks the port
  against BBA's WJ (informational — the notes are authoritative, divergences are
  reported not failed): on 1000 boards our openings agree with WJ **86% on the
  overlap** (1-level + Multi 2♦) and the disagreement lists (e.g. our inclusive
  1NT absorbing 15–17 five-card-major/six-card-minor hands WJ opens in a suit; our
  Ekren/Muiderberg firing where generic WJ passes) are the next authoring targets.
  The eight curated textbook openings are now hard assertions against *our* system
  (`tests/polish_club.rs`), alongside a 0-opaque guard and a reach-game check. No
  new crate dependencies (the BBA example stays `libloading`/`clap` dev-deps).
- A **WJ (Polish Club) reference set** (AI-bidder Side-track S, S.2): a new
  `bba-wj-reference` example that harvests BBA's **WJ — *Wspólny Język* / Polish
  Club** bidding (EPBot **system type 2**, confirmed both from `WJ.bbsa`'s
  `System type = 2` and behaviorally — an 18-balanced hand opens 1♣, the Polish
  Club catch-all, not 1NT/1♠) as ground truth for the upcoming Polish Club port
  (M4.3) and a head-start on the second system corpus (M5). A table of WJ bidders
  self-plays random boards (reusing the S.1 `BbaOracle` flow — one fresh bot per
  decision, dealer canonicalized to position 0), and every `(auction, call)`
  becomes a JSONL record carrying the actor's hand and — for the **first round**
  of bidding — BBA's *self-reported meaning*: a short systemic label
  (`"Polish 1C"`, `"Multi"`, `"5+ !H"`) **plus parsed constraint ranges** (a point
  range and per-suit length ranges that map straight onto the `Constraint` DSL the
  port authors against). The meaning is read through two more EPBot calls
  recovered here by disassembly — `epbot_get_info_meaning` and
  `…_meaning_extended`, which fill a string buffer for the bid at a given position;
  they report systemic meanings reliably only for the **first four calls**, so the
  harness captures positions `0..4` and drops EPBot's trivial-range "no info"
  sentinel (openings and first responses are a system's defining, most-distinct
  slice and exactly what the port builds first). A curated set of eight **textbook
  Polish Club openings** doubles as the milestone's measure: the system-defining
  calls (strong-balanced → 1♣, 15–17 balanced → 1NT, five-card majors → 1♥/1♠) are
  hard assertions against BBA as ground truth (all green), the rest recorded —
  surfacing concrete WJ knowledge for the port (a weak six-spade hand opens
  **2♦ "Multi"**; 1♦ shows **5+**). Output is JSONL plus a versioned JSON sidecar
  (system, seed, git SHA, schema, counts) written under `target/` (gitignored);
  ~21000 records over 2000 boards, ~38% carrying a meaning. The proper
  `bidding::verify` per-auction check against the *ported* books lands with M4.3
  (no ported books exist yet) — S.2 produces the reference those checks will diff
  against. Still purely external tooling: `libloading` **dev-dependency** only, the
  proprietary binary stays git-ignored under `/vendor/`, and the crate's default
  build, dependencies, and `instinct()` baseline are untouched.
- A **BBA/EPBot eval anchor** (AI-bidder Side-track S, S.1): a new `bba-match`
  example that pits our deterministic `two_over_one()` floor against **BBA's own
  2/1 Game Force card** (EPBot system 0, verified by name) in an A/B duplicate
  match — apples-to-apples, so every divergence is a pure quality gap in our DSL,
  not a difference of methods. A `BbaOracle` implements pons's public `System`
  trait by driving a fresh EPBot bot per decision (configure all four seats,
  deal the actor's hand, replay the auction with `epbot_set_bid`, read the call
  with `epbot_get_bid`), with the dealer canonicalized to position 0 so
  `classify` stays a pure function of `(hand, vul, auction)`. The S.0 ABI was
  generalized to full auctions: `epbot_set_bid(bot, position, bid, meaning)` and
  `epbot_set_system_type(bot, position, system)` were decompiled and confirmed,
  the ten is EPBot-canonical `T` (`Holding`'s `Display`, verified via
  `epbot_get_cards`), and an earlier "crash on the second bot" was traced to a
  pointer-truncation bug in a throwaway probe, not the library. The harness
  reuses the `instinct-floor`/`scoring`/`ddss` machinery, reports IMPs/board with
  a 95% confidence interval, and dumps the worst divergent boards (the deal plus
  both tables' auctions) as concrete authoring targets. Measured at 2000 boards,
  vul none: **−2.59 IMPs/board, 95% CI [−2.83, −2.35]** — our floor trails a
  mature engine by ≈ 2.6 IMPs/board, the gap concentrated in competitive/
  contested auctions (where the books are thinnest). Still purely external
  tooling: a `libloading` **dev-dependency** only, the proprietary binary stays
  git-ignored under `/vendor/`, and the crate's default build, dependencies, and
  `instinct()` baseline are untouched.
- A **BBA/EPBot reference-bidder spike** (AI-bidder Side-track S, S.0): a new
  `bba-oracle` example that drives Edward Piwowar's EPBot engine as a black-box
  bidding oracle to benchmark our bidding against a mature, rule-based system —
  the way the open-source BEN engine was trained on BBA-bid deals. EPBot ships a
  self-contained native Linux library (`libEPBot.so`, .NET-NativeAOT), so the
  example `dlopen`s it and calls the `epbot_*` C ABI **directly — no Wine, no
  .NET runtime, no subprocess**. The undocumented ABI was recovered (objdump +
  a pure-Python decompile of `EPBotFFI`) and is documented inline: `epbot_create`/
  `_new_hand`(7 args; the four holdings as one `'\n'`-joined string)/`_get_bid`/
  `_destroy`, plus the bid-code encoding (`0/1/2 = Pass/X/XX`, contract =
  `5 + (level-1)*5 + strain`). The spike bids known hands to their textbook 2/1
  openings. Purely external tooling: a `libloading` **dev-dependency** only, the
  proprietary binary stays git-ignored under `/vendor/`, and the crate's default
  build, dependencies, and `instinct()` baseline are untouched. The full harness
  (complete auctions + a `BbaOracle` `System` + the 2/1 A/B match) is S.1, planned
  in [`docs/ai-bidder/plan.md`](docs/ai-bidder/plan.md) "Side-track S".
- A **behavioral constraint verifier** (M4.2 of the AI-bidder effort): a new
  ungated `bidding::verify` module that checks a candidate `Constraint` *accepts
  the right hands*, complementing M4.1's round-trip check that it *renders* to the
  right gloss. M4.1's `describe().to_string() == gloss` is a string compare, so it
  is blind to the body of a `described("label", closure)` escape hatch (only the
  label renders) and to whether a primitive's bounds match looser human intent
  when porting. `verify::compare(reference, candidate, rng, n)` samples `n` random
  hands and returns a `Report` (accept rates plus a bounded sample of
  counterexample hands) of where the two disagree; `accepts`/`predicate` adapt a
  `Constraint` to the comparison, a book `Rule`'s public `eval` serves as the
  porting oracle (`compare_against_rules`), and `check_examples` checks a
  constraint against hand-labeled intent. A new `tests/dsl_verify.rs` is the
  milestone measure — it catches a battery of deliberately-broken constraints
  (the canonical "5+ ♥" mis-compiled to `len(♥, 4..)`, off-by-one bands, a
  swapped `&`/`|`, dropped/extra clauses, and a `described` closure that uses `>`
  where intent is `≥`) while faithful recompiles agree. A new `verify-constraint`
  example runs the M4.3 porting loop on real book data: it pulls the 1♠ opening
  from the 2/1 books and shows a faithful recompile (0 disagreements) versus a
  broken one (caught, every counterexample a four-card spade hand), then the
  escape-hatch blind spot (two "prefers diamonds" closures that render
  identically yet disagree on equal-length hands). Offline tooling; nothing
  learned ships, and the instinct/neural/search rails stay green.
- A **DSL authoring-compiler spec** (M4.1 of the AI-bidder effort):
  [`docs/ai-bidder/dsl-spec.md`](docs/ai-bidder/dsl-spec.md) is a precise,
  pasteable English→`Constraint` prompt — the grammar (the `&`/`|`/`!` tree and
  how `describe()` renders it), a vocabulary table for all 22 primitives with
  their exact glosses and range conventions, the `described(...)` escape-hatch
  discipline, gold `(English, Rust)` pairs harvested from the live 2/1 books, and
  explicit compile instructions. It turns book authoring — and the planned Polish
  Club port — into "write the meaning, verify, commit": an LLM proposes a
  `Constraint`, deterministic Rust verifies it. The spec is offline tooling;
  nothing learned ships. A new `tests/dsl_roundtrip.rs` is that mechanical check —
  it pins every primitive gloss and the combinator/range rendering against
  `describe()`, and reproduces 12 held-out real rules from their gloss alone (100%
  exact round-trip), so the spec is provably sufficient and `describe()` cannot
  drift from it unnoticed. The behavioral verifier (accept/reject over random
  hands) is the next milestone, M4.2.
- A **self-describing constraint DSL** (M4 of the AI-bidder effort, the
  authoring compiler's foundation): `Constraint::describe()` now renders any
  authored constraint to canonical English, the inverse of `eval()`. Until now a
  `Constraint` was eval-only and opaque — once built as `Arc<dyn Constraint>` you
  could run it but never read what it *meant*, and the corpus exporter had to
  re-guess descriptions structurally from the bid shape, divorced from the real
  logic. Now every primitive names itself (`hcp(15..=17)` → "15–17 HCP",
  `len(Spades, 5..)` → "5+ ♠", `support(3..)` → "3+ card support for partner")
  and the combinators compose those: `&` reads as a comma list ("12–21 points,
  and 5+ ♠"), `|` as ", or", `!` as "not (…)", with nested groups parenthesized.
  The new public `Description` tree carries the structure (so a terse WBF-tag
  renderer can be added later without touching primitives) and `impl Display`
  prints the prose. This is the readable face of a book — the meaning is read
  straight from the logic it bids on, so author and reader cannot drift, and it
  is the verification substrate the later English→`Constraint` LLM compiler will
  round-trip against. **Non-breaking and behaviour-preserving:** `describe()` has
  a default (`Opaque`) so external impls compile unchanged, the ~21 primitives
  were turned from anonymous closures into named structs with byte-identical
  `eval`, and the full instinct/neural/search rails stay green.
- `bidding::constraint::described(label, condition)` — a labeled escape hatch: a
  one-off predicate that carries its own meaning, where a bare `pred()` renders
  `Opaque`. Used to label the books' bespoke predicates (better-minor selection,
  Michaels/Unusual length comparisons, the RKCB keycard/queen/king holdings), so
  every node in the 2/1 corpus now describes truthfully.
- `bidding::rules::Rule::describe()` — the meaning of a rule's call, read from
  its constraint.
- A `render-book` example: prints the floor-less 2/1 books as readable prose —
  each auction, then per call its weight and the constraint's own English
  description — including the full RKCB 1430 ladder ("exactly 2 keycards, and
  holds the ♠ queen"). A stderr coverage metric counts any rules still opaque (0
  for the corpus books).
- The `export-corpus` exporter now emits a truthful `constraint` field from
  `Rule::describe()` and makes it the default `description` (precedence: a
  hand-authored `note()` label, then the truthful constraint render, then — only
  for a bare opaque predicate — the structural gloss). At 770 nodes / 2314
  records the corpus is now **0 opaque**: every record carries its real meaning,
  not a re-guessed one. The `tags` field (the controlled WBF vocabulary) is
  unchanged.
- `bidding::search_floor::SearchFloor` and `two_over_one_search()` behind a new
  `search` feature — the gated live double-dummy search bidder (M2.3 of the
  AI-bidder effort, completing Milestone 2). This is "simulations in action": at
  each non-forced decision the floor *thinks* before it bids. It wears the same
  deterministic safety shell as the neural floor — auction-determined forced
  situations delegate to `instinct()` verbatim, so the §0.4 rails hold by
  construction — but in the judgement middle it no longer trusts the net's single
  forward pass. Instead the net is only a *prior*: it shortlists the top-`k` legal
  calls, prices each by cardplay over `n` sampled layouts with `ev_all` (the M2.2
  evaluator), and re-seats the evaluated calls onto an EV-ranked band above the
  prior tail, so the driver bids the highest-EV call while every legal call keeps
  a sane fallback logit and `Pass` stays finite. "Net proposes, search disposes."
  The rollouts finish under self-play with our own distilled net
  (`two_over_one_neural()`) — the continuation policy M3.2 will iterate. The knobs
  (`layouts`, `shortlist`, `temperature`) default to *strength, not latency*
  (`n = 128`, `k = 8`, ≈ 1.4 s per decision — `n` and `k` raised together so the
  wider shortlist's extra candidates are scored against tight EV estimates, not
  noise); shrink them for a faster, noisier bidder. `classify` stays
  a pure function despite sampling: the rollout RNG is seeded deterministically
  from the decision's feature vector, so the same hand and auction always yield
  the same logits (invariant §0.5). The `search` feature implies `neural-floor`
  (it needs the prior net and the forced-rails shell); the default build,
  `instinct()`, and `two_over_one()` are untouched — this is an added gated
  option, never a replacement. Seven gated tests cover the five §0.4 rails against
  the shelled search bidder, determinism, and the EV-band ordering. A gated
  `search-floor` example A/Bs it against the deterministic floor (it should beat
  the hand-written ladder) and against the distilled net (search should beat the
  raw policy it proposes from). The search is slow by design — every decision is a
  double-dummy search — so a real interval needs a long run; the live bidder is
  also the teacher whose improved EVs M3 will distill back into a fast forward
  pass.
- `bidding::ev::{ev, ev_all}` — the call-EV evaluator (M2.2 of the AI-bidder
  effort). For a candidate call it answers the question the rule books never
  could: *what is this call worth?* — by Monte-Carlo rollout grounded in
  cardplay. It samples layouts with `sample_layouts`, seeds the candidate onto
  the prior auction, lets a continuation policy bid each layout out, scores the
  contract reached double dummy, and averages the result in the actor's favour.
  `ev_all` scores a slate of candidates over the *same* layouts and a single
  double-dummy solve per layout, so the cost is `n` solves rather than `k · n`;
  `ev` is the one-call wrapper. The continuation policy is a `System` parameter,
  not hardwired — callers pass the deterministic `two_over_one()` for now, and
  the M3 search-improvement loop will swap in successive nets without touching
  this code; all four seats bid the same policy (a self-play assumption). EVs are
  average scores in points (positive good for the actor); a call illegal in the
  prior auction, or an auction so tight no layout can be sampled, scores `NaN`
  (read as *no signal*). The evaluator is ungated. Five tests cover the ranking
  sanity (a sound game out-values a hopeless grand, which prices out negative),
  determinism under a fixed seed, the illegal-candidate and infeasible-auction
  `NaN` paths, and the empty-slate case. This evaluator is the shared engine
  behind both the M2.3 live search bidder and the M3 offline training targets.
- `bidding::sampler::sample_layouts` — the constrained layout sampler (M2.1 of
  the AI-bidder effort, starting Milestone 2). The inverse of `Inferences`: given
  the player to act, their hand, and their seat, it deals the other three hands
  at random so that LHO, partner, and RHO each fall within their shown length and
  point ranges. It pins the actor's thirteen cards into a partial deal and
  rejection-samples on top of `contract_bridge::deck::fill_deals`, so accepted
  layouts satisfy every range by construction; an attempt budget bounds tight or
  infeasible auctions, which may return fewer layouts than requested rather than
  loop forever. This is the substrate the M2.2 call-EV evaluator will score each
  candidate call over by double dummy. The sampler is ungated — the natural
  completion of `Inferences`, which was built for exactly this — and takes the
  caller's RNG, so the learned floor stays deterministic. Six tests cover
  soundness (a property test over random hands), the count being met on feasible
  auctions, non-degenerate coverage, and termination on an infeasible auction.
- `bidding::constraint::point_count` — the upgraded-points scalar (raw HCP plus
  the fuzzy-strength `upgrade`) that the suit-oriented `points` constraint gauges
  and that `Inferences` records its point ranges on. A single definition now
  shared by the new sampler and the `Inferences` soundness test, so the value can
  never drift from the ranges checked against it.
- `rand` is now a direct dependency (`0.10`). It was already compiled
  transitively via `contract-bridge`'s `rand` feature, so the dependency tree is
  unchanged; the sampler simply names it directly to take the caller's RNG.
- `bidding::neural_floor::NeuralFloor` and `two_over_one_neural()` behind the
  `neural-floor` feature — the safety shell that makes the distilled net usable
  as a floor (M1.3 of the AI-bidder effort, completing Milestone 1). The shell is
  a drop-in `Classifier`: in auction-determined forced situations (partner's live
  takeout double, an auction that forces game, a just-made transfer over our
  strong notrump) it delegates to the deterministic `instinct()` ladder verbatim
  — the learned net is never trusted on the rails — and everywhere else it
  returns the net's logits legality-masked with `Auction::can_push`, keeping
  `Pass` finite so a distribution always exists. `two_over_one_neural()` mirrors
  `two_over_one()` with this floor swapped in; the deterministic `instinct()`
  floor stays the default and baseline (nothing is removed, an option is added).
  Five gated tests pin the five §0.4 safety properties against the shelled net.
  Hand-conditioned game forces are left to the net as judgement, measured by the
  example below, not hard-railed.
- `examples/neural-floor` behind the `neural-floor` feature — the A/B measurement
  for the learned floor (M1.4 of the AI-bidder effort). Two duplicate matches
  with 95% confidence intervals: the neural floor against the deterministic floor
  (the distillation parity target) and against bare books (the floor's worth).
  At 8000 boards (vul none) the neural floor is at parity with the deterministic
  floor — −0.01 IMPs/board, CI [−0.05, +0.03], containing zero — while preserving
  the floor's gain over bare books (+0.59 IMPs/board, CI [+0.52, +0.66], against
  the hand-built floor's recorded ≈ +0.5). The distilled floor *equals* the
  hand-written one on the harness; the machine now does the floor's job.
- `bidding::neural` behind the new `neural-floor` cargo feature — the in-crate
  forward pass for the distilled floor (M1.2 of the AI-bidder effort). A
  hand-rolled `f32` matmul + ReLU that embeds the trained `two_over_one_v1`
  weights with `include_bytes!` and evaluates `classify(features) -> Logits`
  with no ML dependency. The feature is off by default, so the standard build is
  byte-for-byte unchanged. A parity test reproduces the trainer's candle logits
  on the exported fixture within `1e-3` and matches the arg-max (chosen call)
  exactly. The deterministic safety shell — legality masking and forced-situation
  overrides — follows in M1.3.
- `trainer/` — the off-crate distillation trainer (M1.1 of the AI-bidder
  effort). A self-contained cargo workspace built with `candle` that is never
  compiled by the `pons` build (an empty `[workspace]` table decouples it, and
  the crate carries no ML dependency). It fits a `160 → 256 → 256 → 38` MLP to
  the teacher's softmax by soft-target cross-entropy and exports the weights as
  a flat little-endian `f32` artifact plus a versioned sidecar and a
  forward-pass parity fixture into `src/bidding/weights/`. On a 484k-row dataset
  it reaches ≈94% held-out top-1 agreement with `two_over_one()` (validation
  cross-entropy 0.25 against a 0.20-nat teacher-entropy floor). The weights ship
  in-repo for M1.2 to embed and run by a hand-rolled forward pass; the library
  itself is unchanged.
- `examples/teacher-dump` — the distillation dataset generator (M0.4 of the
  AI-bidder effort, completing Milestone 0). Bids out random boards with
  `two_over_one()` and writes one `(features, teacher_softmax)` row per decision
  to a flat little-endian `f32` file (160 features + 38-way softmax = 198 floats
  per row) plus a JSON sidecar pinning the feature version, teacher, seed, git
  SHA, and counts, plus a sibling `.tags` file (one `u8` per row marking
  contested-phase decisions) so the trainer can split held-out agreement by
  phase. A dev tool, not part of the library API.
- `examples/export-corpus` — the description-corpus exporter (M0.2 of the
  AI-bidder effort). Walks the floorless 2/1 books, recovers each authored node
  through `Classifier::as_rules()`, and emits one JSONL record per `(node, call)`
  with WBF tags derived structurally from the auction (and the rule's `note`
  label as the description where present). A dev tool, not part of the library
  API.
- `bidding::rules`: rules can now carry a human-readable `label` (M0.1 of the
  AI-bidder effort). `Rules::note("…")` chains after `rule(…)` to label the
  preceding rule, `Rule::label()` reads it back (empty by default, so the
  authored books are unchanged), and `Classifier::as_rules()` downcasts a
  type-erased trie classifier back to its authored `Rules` — the hook the
  description-corpus exporter uses to recover each node's calls and labels.
- `bidding::features` — a versioned feature extractor (M0.3 of the AI-bidder
  effort). `features(hand, context)` returns a `Vec<f32>` of exactly 160
  values (`FEATURES_LEN`) encoding five blocks: per-suit rank/length/honor
  indicators (76), global hand evaluations (HCP, upgraded points, Fifths,
  CCCC, NLTC, balanced flag — 6), laws-only auction facts (our/their strains,
  contract-to-beat, partner's last bid, penalty, seat, we-opened — 36),
  inferences per seat (length ranges, point ranges from `Inferences::read` —
  40), and relative vulnerability (2). Layout is versioned as
  `FEATURES_VERSION = 1`; block offsets are exported as `OFFSET_*` /
  `LEN_*` constants.
- `bidding::constraint::cccc_at_least`, gating on the Kaplan–Rubens CCCC
  ("Four C's") evaluation newly available as `contract_bridge::eval::cccc`
  (validated bit-for-bit against Richard Pavlicek's published distribution
  over all 635,013,559,600 hands). CCCC weighs honor placement together with
  shape, making it the right gauge for suit contracts; Fifths remains the
  gauge toward notrump, especially 3NT. The `check-nltc` example gains a CCCC
  column: on 2000-deal pair sums it correlates 0.72 with par suit-contract
  tricks, in the same league as BUM-RAP+ (0.75) and ahead of NLTC.
- `bidding::constraint::points`, the fuzzy-strength gauge: raw HCP plus an
  `upgrade` (also exported) for clean unbalanced hands. An unbalanced hand
  whose short suits (≤ 2 cards) waste no honors gets +1, and +1 more with ten
  or more cards in its two longest suits; any A/K/Q/J in shortness voids the
  upgrade, except the working Ax and Kx. Balanced hands never upgrade, so
  `points` coincides with `hcp` for them.
- `bidding::constraint::fifths`, gating on Thomas Andrews's computed point
  count for 3NT (an `f64` range on the same 40-point scale as HCP).
- The `fuzzy-strength` example: an A/B duplicate match where the same 2/1
  books bid with fuzzy strength on one side and raw HCP on the other, via a
  per-thread ablation hook read at classification time. `--policy
  points|fifths|both` ablates the two gauges separately.
- `bidding::inference`, a per-player summary of what the calls have shown —
  each suit's length range and the point range — accumulated across an auction
  and derived purely from the calls under standard 2/1 meanings (`Inferences`,
  `Inference`, the inclusive `Range`, and a `Relative` seat). It fills the gap
  the eval-only `Constraint` leaves: a `len(..)` rule's length cannot be read
  back out, so the summary is reconstructed from the calls (like the instinct
  floor's `Interpretation`). Every range only ever *narrows*, so a hand that
  made the calls always falls within it — the soundness the future constrained
  sampler relies on; the deriver stays silent on artificial structures
  (Stayman, transfers, the strong-2♣ responses) rather than misread them.
- `bidding::constraint::partner_shown_len` and `partner_shown_points`, crisp
  predicates reading partner's guaranteed minimum from `Inferences` — what
  partner's calls *promised*, where `support` grades what *our* hand holds.
- The `inference-floor` example: an A/B duplicate match measuring the
  inference-aware floor against the pre-inference floor via the
  `set_inference_aware` ablation hook.

### Changed

- **The 2/1 system is now sharp on shape, fuzzy on strength.** Roughly a
  hundred rule sites across openings, responses, raises, rebids, the
  game-force structure, Stenberg, weak twos and their Ogust ladder, strong
  2♣, competition, defense, Rubens advances, and the instinct floor swap raw
  `hcp` for `points` wherever strength gates a suit-oriented call — including
  caps, so a clean shapely maximum upgrades *out* of a weak two. Boundary
  pairs (an overcall cap and its double-then-bid floor) convert together so
  no upgraded hand falls between bands.
- Notrump-defining ranges (1NT/2NT openings, opener's balanced 12–14 / 18–19
  / 22–24 / 25–27 rebids, the 13–15 and 15–17 balanced 3NT rebids) gauge
  `fifths` over half-open bands (`hcp(15..=17)` → `fifths(15.0..18.0)`), so a
  queen-heavy 20-count now opens 1♣ planning a 2NT rebid while a ten-rich
  14-count upgrades into the strong notrump. Responder's notrump ladders
  (including the whole BTU structure) intentionally stay on raw HCP for a
  follow-up. The last-resort 1NT rebid fallbacks became unconditional
  (`hcp(0..)`) so light or off-band openers always retain a book call.
- **The instinct floor now reads partner's shown shape.** In a forced-to-game
  auction it bids a *known* eight-card-plus major fit (our five-card suit
  opposite partner's shown three-card support, or our support opposite
  partner's shown five-card suit — e.g. opposite our 1NT after partner's
  natural, forcing three-level major) rather than the shape-blind 3NT.
  Measured by the `inference-floor` example over 20,000-board A/B matches, it
  scores non-negative at every vulnerability (+0.00 to +0.01 IMPs/board, ~1.5
  to 3 IMPs per divergent board) while diverging on only ~0.3% of boards — the
  triggering auctions are rare, so the gain is small but real and never a
  regression.
- Measured by the `fuzzy-strength` example over 20,000-board A/B matches per
  configuration, the combined policy scores level with raw HCP (runs between
  −0.04 and +0.03 IMPs/board, within noise at this sample size) while
  diverging on ~21% of boards. Ablated at 20,000 boards apiece, each half
  alone also measures −0.01 IMPs/board (`points` diverging on 17% of boards,
  `fifths` on 5%). The policy is kept for its descriptive value — sharper
  announced ranges at equal measured strength — and the ablation hooks stay
  for tuning the halves separately.

### Fixed

- The instinct floor no longer passes a game-forcing 2♣ auction out below 3NT.
  The strong 2♣ opening is forcing for one round only; it is responder's
  *answer* that settles the game force — the 0–3 HCP double negative (2♥) keeps
  the option to stop short, while the waiting 2♦ or any positive commits both
  partners to at least game. The floor now reads that force off responder's
  call (its second convention, after the strong notrump), so off-book
  continuations such as `2♣ – 2♦ – 2NT` reach 3NT or the cheapest major game
  instead of dying in a partscore, while `2♣ – 2♥ – 2NT` may still be passed.
  The forced-to-game rules (for both the 2♣ and strong-notrump conventions)
  also step aside whenever we are penalizing the opponents with a double of our
  own, so a game force never pulls partner's penalty double of a partscore.
  Instinct now reconstructs this forcing state through a small `Interpretation`
  of the auction it owns, keeping system interpretation out of the mechanical
  `Context`.

## [0.9.0] — 2026-06-13

### Added

- `scoring`: per-board scoring primitives promoted from the `instinct-floor`
  example so every simulation harness shares one scorer —
  `final_contract(auction, dealer)` (the last bid with its doubles and the
  absolute declarer), `ns_score(result, tricks, vulnerability)` (the signed
  NS score of a final contract priced from a double-dummy `TrickCountTable`,
  0 for a pass-out), and `imps(diff)` (the standard WBF scale). The
  `instinct-floor`, `practice-bidding`, and `defend-2sx-or-3nt` examples now
  use these instead of private copies.
- `Table::bid_out_from`: continue a seeded auction (positioned from the
  dealer) until it ends; `Table::bid_out` is now this with an empty seed.
  This is the driver for forcing an auction prefix and letting the systems
  finish the board, as the `defend-2sx-or-3nt` example does with its
  `(2♠) X (P) + decision` seeds.
- `two_over_one_strawberry()` (with its floor-less `bare_two_over_one_strawberry()`
  ablation), an opt-in variant of the 2/1 system that layers in three optional
  conventions from the author's *Strawberry Polish Club* notes
  (<https://polish.club>), each chosen to stay applicable to a 2/1 framework,
  while leaving the canonical `two_over_one()` untouched for A/B comparison.
  Exported at the crate root alongside `two_over_one`. To avoid authoring a node
  per artificial continuation, the variant also floors its *constructive* book;
  with the strong-notrump instinct rules a game-forcing 1NT auction reaches game
  even where the book stops (covered by an end-to-end test that plays full
  auctions through the stance).
  - **Strawberry Stenberg 2NT** (`stenberg`) replaces Jacoby 2NT as opener's
    rebid after `1M – 2NT`: the cheapest step shows a minimum, every other rebid
    a maximum that describes a side fragment, a five-card side suit, or a
    two-suiter, with RKCB 1430 below the agreed major.
  - **BTU responses to the strong 1NT** (`btu_notrump`) replace the baseline
    1NT response block — BTU Stayman 2♣ (with the invitational 5=♠ relay and
    Smolen), Jacoby transfers 2♦/2♥ with super-accepts, minor-suit transfers
    2♠/2NT, Puppet Stayman 3♣, splinters, South African Texas, and the
    quantitative ladder — while keeping the 2NT-strength and 18–19-rebid
    structures shared with the baseline.
  - **Rubens (transfer) advances** (`rubens`) overlay the defensive book: over
    partner's overcall, the step just below partner's suit is a limit-plus
    transfer raise that partner completes.
- The `practice-bidding` example: an interactive harness for judging the
  bidding books by sitting at the table. It shuffles random boards — optionally
  rejection-sampled against `--min-hcp` / `--max-hcp` / `--min-suit` bounds on
  your hand — and lets you bid one seat while pons bots bid the others:
  `--bots 3` seats bots at all three other chairs, `--bots 1` gives you a bot
  partner against silently passing opponents for uncontested practice. After
  each of your calls it prints the book's top three candidates with softmax
  weights and tracks your agreement rate with the bot's first choice. When the
  auction ends it reveals the deal and renders two double-dummy verdicts via
  `ddss`: the final contract scored on the actual layout (with the par score
  for reference, both signed from your side), and its make rate, mean score,
  and trick range across `--simulations` reshuffles of the two unseen opposing
  hands with your side's cards held fixed.
- `bidding::instinct`: the instinct bidder, a keyless floor for off-book
  auctions. `instinct()` is one context-driven `Rules` ladder that answers
  *every* auction with a sane natural action: penalty pass only with a trump
  stack, raises of partner's suit with three-card support and rising strength
  per level, notrump and five-card-suit overcalls when we have not bid,
  takeout doubles of their low suit bids, and pass. Partner's *live takeout
  double* (the auction ends `… (bid) X (Pass)` with their suit bid at the
  three level or below doubled) is recognized mechanically and never passed
  without a trump stack — the advance ladder guarantees an action down to a
  cheapest-notrump escape. Every instinct call is natural, so the floor stays
  coherent when both partners land on it; strength-showing cue-bids are
  deliberately excluded until both sides of the convention can be authored.
  Floor activations are observable as `Provenance { depth: 0, fallback:
  Some(_), .. }` from `Trie::resolve` — in simulation, the most-hit auctions
  are the next nodes worth authoring properly.
- `bidding::constraint`: `they_bid(strain)` and `short_in_their_suits()`
  (takeout shape: at most three cards in each suit the opponents have bid),
  promoted from private helpers in the 2/1 books.
- `Stance::classify_with_provenance`: the same routing and logits as the
  `System` implementation, plus the resolution `Provenance` — the telemetry
  hook for counting instinct-floor activations (`depth == 0` with `fallback`
  set) and ranking which off-book auctions to author next.
- `bidding::two_over_one::bare_two_over_one`: the 2/1 pair *without* the
  instinct floor — the ablation handle; `two_over_one()` is now this pair
  with the floor attached.
- `instinct-floor` example: an A/B duplicate match (floored vs bare 2/1 on
  identical boards, swings scored double dummy and credited to the floored
  team in points and IMPs) plus floor telemetry (activation rate and the
  most-hit off-book auctions). First run: the floor is worth about +0.5
  IMPs/board against its own absence, and the telemetry's top entries —
  later-seat openings falling off the defensive book — drove the seat-fan
  fix above.
- `bidding::two_over_one::defense_to_weak_two` and `advance_double`: defense to
  the opponents' weak twos and advancing partner's takeout double, filling the
  one gap the `defend-2sx-or-3nt` example needed. The defensive book now
  answers a `(2♦/2♥/2♠)` opening with a takeout double, a natural 15–18 2NT
  overcall, and cheapest-level suit overcalls; `advance_double` answers
  `(opening) X (P)` for advancer with a penalty pass on a trump stack, a
  major-suit game jump, 3NT with a stopper, cheapest-level new suits, and a
  lebensohl-style escape to the cheapest notrump. Bid levels are derived from
  the opening, so the one advancer builder serves both one-bids and weak twos
  (it is now also registered after `(1x) X (P)`).
- `bidding::context`: `Context`, the mechanical auction context passed to
  classifiers and constraints — vulnerability (relative to the side to act),
  the raw table auction, and facts derived from it (bid strains per side,
  partner's last bid, the contract to beat, doubling state, passed-hand and
  seat facts, `min_level`). Also `context::relative(AbsoluteVulnerability,
  Seat)`, the only vulnerability conversion in the crate: drivers convert
  once per `classify` call, and systems pass the relative value through
  unchanged.
- `bidding::constraint`: a composable constraint vocabulary for authoring
  rules. A `Constraint` maps `(Hand, &Context)` to a logit contribution;
  crisp predicates return `0.0`/`-∞`. Primitives: `hcp`, `len`, `balanced`,
  `nltc_at_most`, the context-relative `support`, `stopper_in_their_suits`,
  `passed_hand`, `undisturbed`, `vulnerable`, `they_vulnerable`, `nth_seat`,
  and the `pred` escape hatch. Constraints compose with `&` (sum, AND for
  crisp), `|` (max, OR), and `!` (crisp flip) on the `Cons` wrapper.
- `bidding::rules`: `Rules`, an ordered rule list acting as a `Classifier`.
  Each `Rule` ties a call to a constraint with a weight (soft priority); a
  call's logit is the **max** of `weight + constraint` over its rules.
  `Rules::explain` reports the winning rule per call — "why did you bid
  that".
- `bidding::fallback`: guarded fallbacks generalizing the trie over
  competitive auctions. `Trie::fallback_at` attaches ordered `(Guard,
  Fallback)` entries to a node; `Trie::resolve` answers from the exact book
  first, then walks up from the deepest reachable node taking the first
  admitted fallback, reporting a `Provenance` (depth, entry index, rebase
  count). `Fallback::Rebase` rewrites the auction and re-resolves (at most
  `REBASE_LIMIT` times) — "system on over their double" is
  `FirstIs(Call::Double)` + `ReplaceNext(Call::Pass)` instead of a copied
  subtree. Stock guards: `Always`, `Undisturbed`, `FirstIs`,
  `OvercallAtMost`.
- `bidding::compose`: lazy `System` combinators. `a.vs(b)` composes a table
  where `a`'s partnership is the dealer's side, dispatching purely by
  auction-length parity; the opposing slot is also where an approximate
  opponent model goes. `a.or_else(b)` layers `a` over a fallback system,
  falling through on `None` or logits without probability mass. A blanket
  `impl System for &S` lets `(&a).vs(&a)` work without cloning.
- `bidding::book`: role-aware pair books, split three ways by the `Phase` of
  the auction. A `Constructive` book covers the strictly uncontested auctions
  (openings keyed per seat by their leading passes, and the continuations
  while the opponents only pass), a `Competitive` book covers the auctions
  where we open and they intervene (negative doubles, "system on" rebases),
  and a `Defensive` book covers the auctions where they open (overcalls,
  doubles, defense). All three wrap and deref to `Trie`; each adds a *gated*
  `System` impl that answers only for its phase. `Phase::of(&[Call])` is the
  single routing point that assumes a standard pass — forcing/strong-pass
  openings stay out of scope (use a bare `Trie`).
- `bidding::book`: `Pair` and `Stance` — a pair's authored system and its
  bound form. A `Pair` assembles the three books with a `Family` identity (an
  open `&'static str` newtype with stock constants such as `NATURAL` and
  `POLISH_CLUB`; downstream systems mint their own) plus optional
  per-opponent-family overrides (`competitive_vs`, `defensive_vs`), because
  what the competitive and defensive books mean depends on the opponents'
  system. Binding happens once at table assembly: `pair.against(them)`
  selects the books for that opposing family and returns a `Stance`, the
  `System` that classifies by `Phase`. `against` merges a structural clone of
  the constructive trie into the bound competitive trie (classifiers stay
  `Arc`-shared), so a competitive rebase such as "system on over their
  double" lands in the uncontested core; constructive-phase queries use the
  unmerged constructive trie, so no competitive fallback can leak into
  undisturbed auctions. Seat-dependent systems (e.g. transfer openings in
  1st/2nd seat only) need no extra machinery — each seat already has its own
  subtree under its leading-pass prefix — and vulnerability-dependent
  agreements are authored with the `vulnerable()`/`they_vulnerable()`
  constraints.
- `bidding::table`: `Table`, the absolute-seat table driver. It seats two
  systems as North/South and East/West with a dealer and an
  `AbsoluteVulnerability`, rotates the seat to act, converts the
  vulnerability once per call, filters illegal calls with the contract-bridge
  law (`Auction::can_push`), and bids a deal out (`classify`, `next_call`,
  `bid_out`). `Table::of_pairs(ns, ew, dealer, vul)` binds two `Pair`s
  against each other's families and seats them. `Versus` remains the lazy,
  dealer-relative composer; `Table` is the full-board driver and deliberately
  not a `System` (the trait speaks relative vulnerability).
- `Trie::merge`: structural union for assembling a system from separately
  authored fragments (uncontested core + competitive packages). On collision
  `self` keeps its classifier and the keys are reported back; fallback lists
  concatenate with `self`'s first; `Arc`s are reused.
- `classifier`, `guard`, and `rewriter`: identity functions giving plain
  closures the higher-ranked `&Context`/`&[Call]` signature the compiler
  cannot generalize on its own.
- `bidding::two_over_one`: the first concrete, reusable system —
  `two_over_one()` builds a `Pair` (family `NATURAL`) for basic Two-over-One
  Game Forcing (re-exported as `pons::two_over_one`). It covers the uncontested openings
  (strong 15–17 / 20–21 notrumps, strong artificial 2♣, five-card majors,
  better-minor 1♣/1♦, weak twos, preempts, a lighter 3rd/4th-seat major), the
  first response to every one-level opening (the 2/1 game force, the forcing
  1NT, single/limit/Jacoby-2NT/weak-jump raises, 1♠ over 1♥, four-card majors
  up the line over a minor), the 1NT structure (Stayman and Jacoby transfers
  with their opener completions), one round of opener's rebids (after a
  one-level new suit and the forcing 1NT), negative doubles with "system on
  over their double", and a defensive book (natural overcalls, takeout
  doubles, the 15–18 1NT overcall, simple advances, and a penalty-oriented
  defense to their 1NT). The public sub-builders `openings`, `major_responses`,
  `minor_responses`, `notrump_responses`, and `defense_to_suit` return `Rules`
  for reuse and testing, and `competition()` returns the `Competitive` book.
  Authored entirely from the existing vocabulary; no new infrastructure. A
  `two-over-one` example (`cargo run --example two-over-one`) bids out random
  boards end to end with both sides playing the system, seated via
  `Table::of_pairs`.
- `bidding::two_over_one`: the system is now a complete 2/1 card rather than
  a basic slice. New in this pass, each in its own submodule:
  - **2/1 game-force continuations** through the slam-try level: opener's
    rebids after every two-level response (jump rebid, raise, six-card
    rebid, 12–14/18–19 2NT, new suits up the line), responder's rebids
    (trump-setting 3M, second-suit raises, 3NT), opener's 4NT hook once
    trump is set, and an `Undisturbed`-guarded *game backstop* so no
    uncovered game-forcing continuation dies below game.
  - **Forcing 1NT continuations**: the three-card limit raise (1NT then
    3M), 2NT invites, weak six-card runouts, preference, and opener's
    accept/decline.
  - **Jacoby 2NT rebids**: side-suit shortness at the three level, a good
    five-card second suit at the four level, 3M (18+) / 3NT (15–17) with no
    shortness, 4M minimum; responder drives with 4NT.
  - **Splinters** (double jump = four trumps, 10–13, singleton/void),
    **inverted minors**, and **weak jump shifts** (6-card suit, 2–5 HCP),
    with opener continuations.
  - **The strong 2♣ structure**: 2♦ waiting, 2♥ double negative (0–3),
    natural five-card positives and the 2NT balanced positive; opener's
    natural rebids (2NT = 22–24, 3NT = 25–27) and responder continuations.
  - **The 2NT machinery** at every strength: three-level Stayman and Jacoby
    transfers over the 2NT opening *and* the 2♣–2x–2NT rebids ("system
    on"), plus quantitative 4NT raises over 1NT, 2NT, and the 18–19 2NT
    rebid, with opener's graded accept/decline.
  - **Weak-two continuations**: Ogust 2NT (min/max × bad/good suit, 3NT =
    solid), RONF preemptive raises, forcing new suits with opener's reply.
  - **RKCB 1430** below every major-suit trump agreement (after Jacoby
    rebids, splinters, game-force trump setting, and the 2♣ major raise):
    the 1430 answers, asker continuations with a documented 0/3–1/4
    ambiguity policy, the 5NT king ask, and grand-slam decisions.
  - **Fuller competition**: cue-bid (limit-plus) raises, preemptive jump
    raises, competitive raises, negative doubles over all four openings
    (system on over their double everywhere), weak jump shifts in
    competition, support doubles/redoubles (exactly three-card support),
    and opener's forced answer to a negative double.
  - **Two-suited defense**: Michaels cue-bids and the unusual 2NT with
    longer-suit advances and game jumps, plus responsive doubles after
    partner's takeout double and their raise.

  Still left for later passes: lebensohl and reopening actions in deeper
  competitive auctions, responder's natural rebids after `1m–1M–2m`, and
  minor-suit keycard.
- `bidding::constraint`: four new public primitives — `top_honors(suit,
  range)` (count of A/K/Q for suit quality), `stopper_in(suit)`,
  `partner_suit_is(suit)` (pins partner's last bid suit, where `support`
  only grades it), and `min_level_is(level, strain)` (the legality anchor
  for rules whose call sits at a dynamic level, such as cue bids).

### Changed

- **Breaking** (within this release's 2/1 card, never published in an earlier
  version): raise meanings moved to the
  modern defaults. `1m–2m` is now the strong inverted raise (10+, forcing)
  and `1m–3m` the weak preemptive one; direct `1M–3M` limit raises promise
  four trumps, with the three-card limit raise routed through the forcing
  1NT.

- Reduced Clippy noise across bidding internals and tests: several small
  closure-coercion/context helpers are now `const fn`, builder-style
  constraint constructors gained `#[must_use]`, doc-code link formatting was
  cleaned up, and float assertions in tests were refactored to robust helper
  predicates instead of direct float equality.

- **Breaking:** `Partnership` is replaced by `Pair` + `Stance`, and book
  routing is now three-way via `Phase::of`. `Constructive` answers *only*
  strictly undisturbed auctions; competition over our openings moves out of
  the constructive trie into the new `Competitive` book (`two_over_one`'s
  negative doubles and system-on now live in its `competition()` builder).
  Assemble the three books with `Pair::new(family, constructive, competitive,
  defensive)` and bind once against the opponents' family —
  `pair.against(them)` returns the `Stance` that implements `System`; a
  `Pair` itself is authoring material, not a `System`. `two_over_one()` now
  returns the `Pair`.
- pons now requires `contract-bridge` 0.1.2 for the newly public
  `Auction::can_push`, the dry-run legality check behind `Table::next_call`.

- **Breaking:** `bidding::trie::Classifier::classify` now takes
  `(Hand, &Context)` instead of
  `(Hand, RelativeVulnerability, CommonPrefixes)`. The context carries the
  vulnerability and (optionally) the common prefixes. Closure classifiers
  change from `|hand, vul| …` to `classifier(|hand, context| …)`.
- **Breaking:** the vulnerability-indexed `Forest` (`[Trie; 4]`, with `from_fn`
  and the `Index<RelativeVulnerability>` impls) and its `SeatClasses` mask are
  removed in favor of `bidding::book`. Vulnerability conditions live in
  constraints (`vulnerable()` / `they_vulnerable()`), seats are explicit leading
  passes in the book key, and seat-dependent strength is a constraint
  (`nth_seat` / `passed_hand`). Author a pair's notes with the role-aware
  books assembled into a `Pair` (see above) instead of a bare `Trie`, which
  stays as the low-level table-model escape hatch (and the way to express
  forcing passes).
- `System for Trie` resolves through fallbacks (`Trie::resolve`) instead of
  exact lookup only, so a trie with fallbacks now answers auctions outside
  its book. `get`, `longest_prefix`, `common_prefixes`, and `suffixes` are
  unchanged. The `System` docs pin the vulnerability convention: `vul` is
  relative to the side to act, and composite systems pass it through
  unchanged.

- **Breaking:** `stats::average_ns_par`'s vulnerability parameter is now
  `contract_bridge::AbsoluteVulnerability` instead of `ddss::Vulnerability`.
  `AbsoluteVulnerability` is a new NS/EW bit set in `contract-bridge` 0.1.1 (now
  the minimum required version) that mirrors the existing `RelativeVulnerability`
  for symmetry. The four values map one-to-one — replace
  `ddss::Vulnerability::{NONE, NS, EW, ALL}` with the same constants on
  `contract_bridge::AbsoluteVulnerability`. The double-dummy solver is unchanged
  (still `ddss`).
- `bidding::instinct` learned to bid opposite our own strong notrump: it
  completes a standard transfer (Jacoby 2♦/2♥, 3♦/3♥ over 2NT, South African
  Texas 4♣/4♦) and, rather than pass a *forced* game out, bids the cheapest
  game — a six-card major, else 3NT — when a responder holds game values
  opposite a 15–17 1NT / 20–21 2NT, or when opener is forced past invitation.
  This is the one convention instinct reads, so the deep BTU / strawberry 1NT
  structures stay sound even where the book does not author every continuation.
  Authored rules and weak hands are unaffected: the floor is reached last and
  still defaults to Pass below an invitation.
- `notrump::register` split into `register_one_nt` (the 1NT-opening response
  block) and `register_two_nt_and_rebids` (the 2NT-strength and 18–19-rebid
  structures), so the strawberry variant can swap in BTU for the former while
  reusing the latter. `two_over_one()` is unchanged.
- `two_over_one()` attaches the instinct floor (see `bidding::instinct` under
  *Added*) to its competitive and defensive books as a root `Always`
  fallback, so the bound stance never falls off the book in a contested
  auction. Auctions that previously classified as `None` — and so were passed
  by drivers, including passing partner's takeout double on a worthless hand
  — now get a natural answer: their three-level preempts, jump overcalls past
  the negative-double range, and deep competitive continuations among them.
  Authored rules are unaffected: resolution reaches the root fallback last.
  The standalone `competition()` and `defensive()` builders stay floor-less.
- The `defend-2sx-or-3nt` example is now a flavor-comparison harness for the
  `(2♠) X (P)` defend-vs-declare decision. West's weak-two opening still comes
  from the real `two_over_one` system, while North's takeout double and South's
  Pass-vs-3NT advance are swept across alternative *flavors* — Shape / Support /
  Sound doubles and Defense / Balanced / Offense responses — each written as a
  crisp constraint in the `bidding::constraint` vocabulary. It reports
  per-double-flavor population stats and per-response-policy regret against a
  double-dummy oracle.
- The `defend-2sx-or-3nt` example studies a *realistic* population and plays
  realistic auctions. Deals are accepted through a four-gate funnel — West
  opens 2♠ per the system (at the table's actual vulnerability, via `Table`),
  North doubles by a swept flavor, *East's pass over the double is the
  system's own call* (deals where East would raise never reach South), and
  *South's decision is live* (the system's advance over `(2♠) X (P)` is Pass
  or 3NT, not a suit bid or an escape). Neither branch assumes the auction
  stops at South's call: the table bids both continuations out with
  `Table::bid_out_from` — West may run from the penalty pass, East/West may
  double 3NT or sacrifice — and the *final* contract is scored with
  `scoring::final_contract` / `scoring::ns_score`. New divergence telemetry
  reports how often (and where) the bid-outs left the nominal 2♠×/3NT
  contracts. The funnel is far tighter than the old gate, so
  `--max-attempts-per-deal` now defaults to 20000 (was 5000); numbers are not
  comparable with earlier runs — the live population is markedly more
  NS-favorable, and the swept response policies now beat both trivial
  baselines instead of losing to "always 3NT".
- docs.rs now documents the crate with `--all-features`
  (`[package.metadata.docs.rs]`), so the `serde` impls appear in the rendered
  docs.

### Fixed

- The defensive book's entry tables are now seat-fanned. `defense_to_suit`,
  `defense_to_weak_two`, `defense_to_notrump`, and the advances of natural
  overcalls were keyed only at the raw opening with no leading passes, so
  they answered only when the opponents opened in *first seat*; with any
  leading pass — `(P) 1♦`, our dealer passing first — the same decisions fell
  off the book (and before the instinct floor, were silently passed). Found
  by the first run of the `instinct-floor` telemetry.
- Broken intra-doc links in `bidding::two_over_one`: replaced the unresolvable
  `[`slam`]` reference (a private module) with plain backtick notation, and
  qualified `[`Pair::against`]` with its full crate path so rustdoc can resolve
  it from `competition`. The strawberry builder's links to its private
  convention modules (`stenberg`, `btu_notrump`, `rubens`, the `notrump`
  register blocks) likewise became plain backticks, and the `bidding::instinct`
  links now disambiguate the module from the `instinct()` function.
- Seat-fan coverage gaps: responses and continuations now answer after
  4th-seat openings (leading-pass fan extended to three passes), and the
  defensive book answers when their opening arrives after leading passes —
  previously `[P, 1♦]` and kin were silently off-book.

## [0.8.0] — 2026-05-24

### Changed

- **Breaking:** Replace the `dds-bridge` dependency with `ddss` (a
  performance-oriented DDS fork) and the `dds-bridge-sys` dev-dependency
  with `ddss-sys`. Most public types are structurally compatible — `Par`,
  `ParContract`, `TrickCountTable`, `TrickCountRow`, and `Vulnerability` all
  live at the same paths under `ddss::*` — so downstream callers usually
  only need to swap the crate name in imports. Two shape changes:
  - `dds_bridge::Solver::default()` → `ddss::Solver::lock()`. The new
    handle holds a reentrant lock, so its solve methods take `&self` (drop
    the `mut`) and the type is `!Send`.
  - The free `dds_bridge::solve_deals(&deals)` is now a method that takes
    a non-empty strain selector: `Solver::lock().solve_deals(&deals,
    NonEmptyStrainFlags::ALL)` reproduces the old all-strains behavior.
  `calculate_par` remains a free function with the same signature and can
  be called with or without a held `Solver` (it acquires the global ddss
  lock internally; the lock is reentrant per thread).
- **Breaking:** Auction primitives (`Call`, `Auction`, `IllegalCall`,
  `RelativeVulnerability`, and their parse errors), the entire `eval`
  module (`HandEvaluator`, `SimpleEvaluator`, `hcp`, `shortness`,
  `fifths`, `bumrap`, `ltc`, `nltc`, `zar`, `hcp_plus`, `FIFTHS`,
  `BUMRAP`, `BUMRAP_PLUS`, `NLTC`), and the entire `deck` module
  (`Deck`, `full_deal`, `FillDeals`, `fill_deals`) move into the new
  `contract-bridge` crate. Update imports such as
  `use pons::bidding::Call;` → `use contract_bridge::auction::Call;`,
  `use pons::eval::hcp;` → `use contract_bridge::eval::hcp;`, and
  `use pons::deck::full_deal;` →
  `use contract_bridge::deck::full_deal;`.
- **Breaking:** `pons` no longer re-exports bridge data types
  (`Hand`, `Strain`, `Bid`, `Seat`, etc.) — these live in the new
  `contract-bridge` crate, not `dds-bridge`. Replace
  `use dds_bridge::Hand;` with `use contract_bridge::Hand;`.
- Track `dds-bridge`'s flattening of the `solver` module to the crate
  root: `dds_bridge::solver::*` imports become `dds_bridge::*` (e.g.
  `dds_bridge::solver::Vulnerability` → `dds_bridge::Vulnerability`).
- Relocated tests that exercised only lower-crate APIs out of `pons`,
  so failures point at the crate they actually cover. `tests/eval.rs`,
  `tests/deck.rs`, `tests/proptest_roundtrip.rs`, and `tests/solver.rs`
  are removed; the auction block in `tests/bidding.rs` and the
  contract-bridge/ddss serde tests in `tests/serde.rs` are removed in
  place, leaving only `Array`/`Map`/`Logits` tests and pons stats serde
  respectively. The moved tests now live in `contract-bridge` (auction,
  deck, eval, proptest, serde) and `ddss`/`dds-bridge` (large-batch
  solver). No behavior or public-API change in pons.
- Dev-dependencies pruned: `approx` and `ddss-sys` are no longer used
  by anything in `pons` and are removed from `Cargo.toml`.

### Removed

- `pons::deck` and `pons::eval` modules (moved to `contract-bridge`).
- The crate-root re-exports `Deck`, `full_deal`, `HandEvaluator`,
  `Auction`, `Call` (moved to `contract-bridge`).
- The `generate-deals` and `notrump-tricks` examples. They no longer
  depended on anything in `pons` and now live with the crates they
  actually need: `generate-deals` in
  [`contract-bridge`](https://github.com/jdh8/contract-bridge/tree/main/examples/generate-deals)
  and `notrump-tricks` in
  [`ddss`](https://github.com/jdh8/ddss/tree/main/examples/notrump-tricks)
  (with a [parallel
  copy](https://github.com/jdh8/dds-bridge/tree/main/examples/notrump-tricks)
  in `dds-bridge`).

### Fixed

- README's `average_ns_par` doctest no longer overflows the stack on
  Windows. The fix is in `ddss` 0.1.2 (now the minimum required
  version): the batch solver's FFI packs are allocated directly on the
  heap via `Box::new_zeroed`, instead of routing through a stack
  temporary as `Box::default()` does at opt-level 0.

### Internal

- Set `[profile.dev.package."*"]` to `opt-level = 2`, so dependencies —
  most notably `ddss-sys`'s C++ DDS engine via `cc` — are optimized in
  dev builds. Pons's own Rust stays at opt-level 0 so any future
  stack-temp-class bug in this crate's own code still surfaces under
  `cargo test`. Big speedup for the `average_ns_par` doctest and
  `tests/par.rs`.

## [0.7.0] — 2026-05-20

### Changed

- **Breaking:** Bump `dds-bridge` to **0.19** and `dds-bridge-sys` to **3.0**
  (the latter is a dev-dependency used only by `tests/solver.rs`). The
  underlying DDS C++ library moves to v3.0.0 with PascalCase struct names
  and snake_case fields; `pons`'s own safe API is unaffected, but downstream
  users who pin to older versions of these dependencies should also bump
  them in lockstep. See the `dds-bridge-sys` v3.0.0 and `dds-bridge` v0.19.0
  changelogs for the rename map.

### Added
- New `defend-2sx-or-3nt` example: compares the expected NS score from
  defending 2♠× vs declaring 3NT after the auction `(2♠) X (P)`. The
  bidding system is a single `Trie` with three classifiers — West's
  weak-two opening at `[]`, North's takeout double at `[2♠]`, and South's
  natural call at `[2♠, X, P]` (which may be Pass, 3NT, or an
  out-of-scope call such as a 3-level new suit, jump in hearts, or
  Lebensohl 2NT). South's classifier is used only as an eligibility
  filter: deals are rejection-sampled so only those where West opens 2♠,
  North doubles, *and* South naturally faces a P-or-3NT decision are
  kept and double-dummy solved. Each accepted deal is scored under three
  strategies — always defend 2♠×, always declare 3NT, and a per-deal
  oracle that picks the higher of the two — giving an upper bound on
  what any policy keyed on South's hand could achieve. Scoring uses
  `dds_bridge::Contract::score`. Accepts an optional `--south` for
  hand-specific analysis (errors if the hand falls out of scope) or
  randomizes all four seats when omitted.

## [0.6.1] — 2026-04-25

### Changed
- Updated `dds-bridge` dependency to 0.18
- `full_deal` now returns `FullDeal` (was `Deal`)
- `fill_deals` now takes a pre-validated `PartialDeal`; no longer returns `Result`
- Track `dds-bridge`'s trick-count rename: `solver::TricksTable` → `solver::TrickCountTable` in `stats::HistogramTable`'s `FromIterator` impl and in the `check-zar` / `check-nltc` examples. Pure rename on the consumer side.
- The `serde` feature now also pulls in `serde_with` (optional dep).

### Internal
- Replaced the last hand-written `serde_impl` submodule (on `Deck`) with `serde_with::SerializeDisplay` / `DeserializeFromStr` derives. No change to the serialized form.
- Replaced non-const `.unwrap()` in tests and the `Auction::declarer` doctest with `?` propagation. Tests with a single fallible error type return `Result<(), E>`; tests mixing error types or unwrapping `Option` return `anyhow::Result<()>`.
- Moved inline `mod tests` blocks in `bidding.rs` and `deck.rs` into dedicated `bidding/tests.rs` and `deck/tests.rs` files. No behavior change.

## [0.6.0] — 2026-04-19

### Added
- Optional `serde` feature for serialization/deserialization support
- `Display` and `FromStr` implementations for `Deck` and bidding types
- `Classifier` promoted to a trait (was a plain `fn` in 0.5.0)
- Constructors for `Forest`
- `FusedIterator` implementation for `Trie` iterators
- `Debug` on `Trie` and iterator types
- Slicing API for `Auction`; `Index<Range<Bid>>` and bid-range indexing on `Array`
- `Logits::softmax` (replaces `to_odds`); returns `None` when all logits are `-∞`
- `fill_deals` helper
- Criterion benchmarks for shuffle, trie, and parallel solving
- proptest-based roundtrip and histogram invariant tests

### Changed
- `System::classify` now takes a slice
- `Auction::push` is panicking; confusing `force_push` removed
- `Deck` rejects duplicate cards
- `RelativeVulnerability` renamed from previous type
- Converters borrow instead of consuming
- Public fields replaced with getters
- Error types marked `#[non_exhaustive]`
- `average_ns_par` return type improved; redundant count parameter removed
- Random deal generation moved to `dds-bridge`; local `solver` module renamed to `random`
- Deterministic stats moved to `mod stats`
- MSRV pinned to 1.93
- Updated `dds-bridge` dependency to 0.16

### Fixed
- Memory leak in `Array::try_map`
- `hcp_plus` calculation

### Internal
- Added `#[inline]` to trivial getters on `Copy` types
- Aligned `HistogramRow::count` to take `self` (non-breaking: `HistogramRow: Copy`)
- Deduplicated `Map::get_mut`
- Bidding context lives with the stored classifier; shared API between systems and classifiers
- Hardened GitHub workflow; CI enforces `fmt`, `clippy`, and doc warnings
- Expanded README; documented the `map` module

## [0.5.0] — 2026-03-25

### Added
- `Array<T>` modeling `Call -> T`, with `Array`-like and full iterator API
- `Map` with iteration over keys, values, and entries; separated iteration for arrays
- `Logits` module (under `mod array`); `Logits::to_odds`
- Abstract bidding table supporting multiple calls per node
- Classifier concept (as a plain `fn`) replacing the filter-based approach
- Own `bidding::Vulnerability` type
- Absolute `bidding::Frequency` for easier filtering
- Different indices for X (double) and XX (redouble)

### Changed
- Edition updated to Rust 2024
- Magic number 38 replaced with a named constant

## [0.3.1] — 2025-05-31

### Fixed
- `Strategy` now requires `RefUnwindSafe` so `Trie` stays `UnwindSafe`

### Internal
- Inlined small functions for optimization

## [0.3.0] — 2025-05-30

### Added
- Core bridge data structures: `Card`, `Suit`, `Hand`, `Deck`, `Holding`
- `SmallSet` trait for `Holding` and `Hand`
- DDS (double-dummy solver) integration via `dds-bridge`
- Contract scoring
- Bitset operators for `Holding` and `Hand`
- Basic CLI to solve random deals
- Hand evaluation (LTC, NLTC, BUM-RAP, Zar points)
- `Auction` with `push`, `pop`, and `truncate`
- `Trie` for bidding strategies, with depth-first iteration, suffix and prefix iterators
- Statistics utilities for evaluators; histograms

[0.9.0]: https://github.com/jdh8/pons/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/jdh8/pons/compare/0.7.0...0.8.0
[0.7.0]: https://github.com/jdh8/pons/compare/0.6.1...0.7.0
[0.6.1]: https://github.com/jdh8/pons/compare/0.6.0...0.6.1
[0.6.0]: https://github.com/jdh8/pons/releases/tag/0.6.0
[0.5.0]: https://github.com/jdh8/pons/releases/tag/0.5.0
[0.3.1]: https://github.com/jdh8/pons/releases/tag/0.3.1
[0.3.0]: https://github.com/jdh8/pons/releases/tag/0.3.0
