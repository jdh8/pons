# pons

Tools for analyzing and simulating contract bridge hands. Compared to
[dds-bridge](https://crates.io/crates/dds-bridge), `pons` is the higher-level
layer; most development goes into the `bidding` module: a 2/1 game-forcing
system (`american()`), a deterministic instinct floor, an inference /
constrained-sampling engine, and the AI bidder that learns to replace the floor.

Search online for authoritative bidding sources, and ask me about bidding
theory. For 5-card major systems see my
[Strawberry Polish Club](https://polish.club/).

## Objective

Make `pons` the best open-source bridge engine. Key results, in order:

1. **KR1 — bridge score** (IMPs, per [docs/measurement.md](docs/measurement.md)).
2. **KR2 — clean software architecture.**
3. **KR3 — acceptable performance.**

KR1 outranks the rest: a KR2 or KR3 win ships only with a KR1
**non-inferiority proof** — a seeded `smoke-default` byte-identity of the
default system, or an A/B non-inferior on both scorers (every vulnerability
cell's 95% lower bound above −δ, δ = 0.002 IMPs/board). A CI that merely
straddles 0 is not a proof.

## Read before working

| Task | Read first |
| --- | --- |
| Any change to `src/bidding` | [docs/bidding-architecture.md](docs/bidding-architecture.md) — the book/floor/inference layer cake and its invariants |
| What BBA's book says about a call, and where its bilans floor takes over | [docs/ai-bidder/bba-book.md](docs/ai-bidder/bba-book.md); look a lane up with `probe-bba-book --render <run> --prefix "1♠ (2♥)"` |
| Closing the gap vs BBA (the anchor, its rules, the bucket ranking) | [docs/bba-gap-campaign.md](docs/bba-gap-campaign.md); run `scripts/anchor.sh`, work the report's buckets. History in `docs/archive/bba-gap-*.md` |
| Splitting the BBA gap into agreements vs judgement — the `bba` system (`--our-floor bba`, `BBA_BOOK=`) | [docs/bba-system.md](docs/bba-system.md); the seam is `Rules::choose_by`, the code `examples/common/bba_book.rs` |
| Closing the gap vs BEN (the north star; BBA = exploit guard) | [docs/ben-gap-campaign.md](docs/ben-gap-campaign.md), harness design [docs/ben-gen-design.md](docs/ben-gen-design.md), how BEN bids [docs/ben-architecture.md](docs/ben-architecture.md) |
| Disclosing our system to BBA (`.bbsa` cards) | [src/bidding/card.rs](src/bidding/card.rs) — cards are generated from the knob state; `cards/*.bbsa` are snapshots re-blessed with `cargo run --example bba-card` |
| A knob A/B moves the floor's regime input, or a floor swap needs its siblings moved | [docs/ai-bidder/card-manifold.md](docs/ai-bidder/card-manifold.md) — the regime input reads `Agreements`, not card rows; the sibling-factory rule |
| Measuring or shipping a bidding change | [docs/measurement.md](docs/measurement.md) — the A/B playbook. **No bidding change ships without it.** |
| Tuning a convention's range, or fixing which call leaks | [docs/convention-tuning.md](docs/convention-tuning.md) — sweep vs forensic |
| Retraining the floor after a feature bump, or reusing the M32 labels | [docs/ai-bidder/features-v8.md](docs/ai-bidder/features-v8.md), [docs/ai-bidder/features-v9.md](docs/ai-bidder/features-v9.md) — the zero-layout re-walk, the splice key, `scripts/feature-retrain.sh` |
| Any retrain that must ship — why the shipped K = 8 beats every reseed of its recipe | [docs/reseed-gap-handoff.md](docs/reseed-gap-handoff.md) — the unshuffled stem order, the 2 × 2 over data order and seeds |
| Retraining without a dump or relabel — seeds, ensembles, trainer flags | [docs/archive/floor-sweep.md](docs/archive/floor-sweep.md) — **closed 2026-09-30**; its closed axes are not retried. Parked big ideas: [docs/next-steps.md](docs/next-steps.md#parked-big-ideas) |
| Neural/AI bidder work | `.claude/skills/ai-bidder` + [docs/ai-bidder/](docs/ai-bidder/) (`README.md` then `plan.md`) |
| Reading a rule weight or `Logits` as a probability (softmax on book logits, a temperature, a demotion "in nats") | [docs/ai-bidder/logit-calibration.md](docs/ai-bidder/logit-calibration.md) — a book weight is precedence, not odds |
| A call reads as nothing, or an `Or` projects to `0..=37` | [docs/ai-bidder/sampled-projection.md](docs/ai-bidder/sampled-projection.md) — read a call off the bidder, not its rules |
| The Watermelon Dutch Doubleton knobs (5542, wide 1♣ + `1♦!` relay, Odwrotka, Multi `2♦`) | The book is the spec: <https://jdh8.github.io/watermelon-dutch/>; code in `src/bidding/american/{openings,wide_one_club,multi}.rs`, `rebids/odwrotka.rs`; all knobs default off, Odwrotka's A/B owed. Retired `dutch()`: [docs/archive/dutch-system.md](docs/archive/dutch-system.md) |
| Envelope-union readings (the historical DNF campaign) | [docs/dnf-migration.md](docs/dnf-migration.md) |
| Retiring a hand-written reader (`inference/readers.rs`) | [docs/reader-retirement.md](docs/reader-retirement.md) |
| A rule's constraint and its reading disagree; a "reading-only" change moved calls | [docs/reading-drift-handoff.md](docs/reading-drift-handoff.md) — a reading knob is a bidding knob under a neural floor |
| An authored call reads as less than its rule says; anything touching `ReadingScope`, `Points::project`, or the walk's blankets | [docs/authored-reading-handoff.md](docs/authored-reading-handoff.md) — alert is disclosure, not the reading switch |
| A fourth-suit call in any lane | [docs/fourth-suit.md](docs/fourth-suit.md) — authored in `1♥ - 1♠` only; XYZ owns the one-level prefixes |
| Competitive book (we open, they interfere) | [docs/competitive-book.md](docs/competitive-book.md) |
| Our 1NT constructive | [docs/one-notrump-constructive.md](docs/one-notrump-constructive.md) |
| Our 1NT contested (`1NT (2x)`, `1NT - resp (..)`) | [docs/one-notrump-competitive.md](docs/one-notrump-competitive.md) — their overcalls are artificial (BBA plays Woolsey Multi-Landy). Verdicts in `docs/archive/one-notrump-competitive-*.md` |
| The `1NT (2♦)` Multi lane | [docs/one-notrump-multi.md](docs/one-notrump-multi.md); regenerate the tree with `render-book --their-2d-multi --prefix "1NT 2♦"` |
| A minor-suit transfer in any lane, or anything above its completion | [docs/minor-transfer-slam.md](docs/minor-transfer-slam.md) — an uncapped minor transfer owes a `4m` rung with an authored answer |
| How BBA and BEN bid defensive auctions | [docs/defensive-auctions-reference.md](docs/defensive-auctions-reference.md) |
| Defensive round 1 (they open, we act) | [docs/defensive-overcalls.md](docs/defensive-overcalls.md), then [docs/takeout-double-layers.md](docs/takeout-double-layers.md) |
| Opener's answer to the K–K doubler's natural other major | [docs/multi-doubler-answer-handoff.md](docs/multi-doubler-answer-handoff.md) — built behind `multi_px_split`, unconditional arm owed |
| Pass/double inversion (a penalty double or converting pass flips our later `X` and `P`) | [docs/pdi.md](docs/pdi.md) — `grep '.penalty()'` is the live trigger inventory |
| The contested 5-level decision (P(double), q) | [docs/ai-bidder/competitive-accountant.md](docs/ai-bidder/competitive-accountant.md), calibration in [docs/ai-bidder/doubling-calibration.md](docs/ai-bidder/doubling-calibration.md) |
| Porting a book section to rows | [docs/declarative-rows.md](docs/declarative-rows.md); grammar in `rows.rs`'s module doc |
| Kickback / Redwood | [docs/ai-bidder/bba-kickback.md](docs/ai-bidder/bba-kickback.md) §7 |
| The decision cache, the compiled rule path, or `Context` beyond the mechanical facts | [docs/bidding-performance-handoff.md](docs/bidding-performance-handoff.md) — the only design record of those commits |
| Long data-gen runs | [docs/shared-machine-data-gen.md](docs/shared-machine-data-gen.md) — this box is shared |
| Training any net, or drawing a corpus | [docs/pdd-bank-ledger.md](docs/pdd-bank-ledger.md) — corpus from `/nfs2/jdh8/pons/`, test on fresh deals |
| Hand valuation, point counts, "how uncertain is this hand?" | [docs/binky-points.md](docs/binky-points.md) |
| Exact probabilities over a hidden hand; how other engines infer hands | [docs/exact-posterior.md](docs/exact-posterior.md) — what is closed, the survey, the phased plan |
| A game gate (points for 4M, 5m, 3NT; a 9th trump; 5m vs 3NT) | [docs/major-game-threshold.md](docs/major-game-threshold.md), [docs/minor-game-threshold.md](docs/minor-game-threshold.md), [docs/notrump-game-threshold.md](docs/notrump-game-threshold.md) |
| NLTC or any losing-trick count | [docs/nltc.md](docs/nltc.md) |
| Zar points, or wasted values | [docs/zar.md](docs/zar.md) |
| A slam gate (6M, 6m, 6NT, a grand; a keycard check's worth) | [docs/suit-slam.md](docs/suit-slam.md), [docs/notrump-slam.md](docs/notrump-slam.md) |
| System variants (`basic` / `advanced` profiles of `american()`) | [docs/system-variants.md](docs/system-variants.md) — a variant is an `Agreements` value, not a factory |
| Raw bidding-theory notes | [docs/bidding-theorems.md](docs/bidding-theorems.md) |

The point-count and slam docs are double-dummy censuses over ~102M deals; take
the numbers from them, not from memory.

Repo skills: `author-convention` (a new convention or treatment, end to end)
and `measure-ab` (running and interpreting an A/B). Use them.

## Workflow (non-negotiable)

- Develop and commit **directly on `main`**; a shipped win stands even if later
  parts of its plan fail. A feature branch has one use: **parking a whole
  idea** that measured a non-win when a credible follow-up (a retrain,
  unauthored siblings, a redesign) could flip it. Name it `park/<slug>`, push
  it, put the numbers and the flip plan in its last commit plus one row in the
  owning campaign doc; `git branch --list 'park/*'` is the index. Rebase onto
  `main` before re-measuring (control = `main` HEAD, same `SEED_BASE`).
- Only commit or push when asked.
- After updating the codebase:
  1. `cargo fmt`
  2. `cargo test --all-features`
  3. Reproduce the CI gates with the strictest local toolchain (CI runs
     floating stable under `-D warnings`):
     `cargo +nightly clippy --all-targets --all-features -- -D warnings` and
     `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
  4. **If the change touches the public API, `cd web && cargo test`.** `web/`
     is its own workspace with `default-features = false`; nothing above
     compiles it. It consumes each reading knob as a setter/getter pair, so
     narrowing a `pub` getter that `src/` never reads still breaks it.
  5. Update [CHANGELOG.md](CHANGELOG.md) with the change and its user impact
     (measured IMPs where applicable).
  6. Propose a commit message: a one-sentence subject (≤ 72 characters,
     `area: what changed`), blank line, then a body with the auction, the
     knob, the numbers and the verdict.

## Iron rules

Each was paid for with a real regression or a wrong conclusion.

### Measurement ([docs/measurement.md](docs/measurement.md))

- Never ship a bidding change on analysis alone. Run the A/B, score with
  **both** plain DD and perfect-defense, read the verdict from the decision
  table. A PD-only win is a doubling artifact; plain-DD non-inferior + PD win
  ships default-on. A wash whose lower bound reaches −δ is *inconclusive*,
  never "no loss"; a default kept on one is a recorded trade-off.
- DD is blind to obstruction and concealment: a preemptive idea measuring
  negative is the harness. Right-siding is half-visible: DD prices the lead
  direction, not the concealment, so it sees a right-siding idea only when
  declarer actually moves.
- Before declaring a measured loss dead, trace the worst divergent boards; the
  usual culprits are an unauthored continuation or an over-broad trigger.
- Measure against the real routing, and complete the convention first — both
  sides' continuations **and the interfered tails** (they double it, they
  overcall it).

### Architecture ([docs/bidding-architecture.md](docs/bidding-architecture.md))

- Every artificial call carries an `.alert(...)` and an `Inferences` reading;
  `artificial_calls_are_alerted` enforces the alert half. An unread
  artificial call is a phantom-suit disaster in competition.
- Learned floors wrap contested/defensive books only; the constructive book is
  floored by deterministic `instinct()`.
- A book node with finite mass shadows the floor. To give the floor a
  position, delete the node; to smarten deep continuations, improve the floor.
- Every rule table needs a finite catch-all; an all-−∞ table falls through to
  the floor.

### Operations

- Heavy runs: `scripts/idle-run.sh`, arms **sequential**, one fresh
  `SEED_BASE=$(date +%s)` per experiment shared across its arms, and never
  rebuild binaries while an A/B is in flight.
- The ddss `Solver` runs on the main thread only; parallelize bidding with
  rayon, never the solver.

### Conventions of the house

- Auctions are space-delimited: `1NT - 2♣ (X) XX -`. Every pass is `-`,
  only opponents' non-pass calls are parenthesized, implied passes are
  expanded (`1NT - 2♦ - 2♥`). `P*` is the row grammar's leading-pass fan;
  `P`/`(P)` only when quoting legacy or external input. Probe/render binaries
  ignore parentheses; only the row grammar (`rows.rs`) seat-checks them.
- A **shape** is suit-fixed (♠♥♦♣), a **pattern** suit-interchangeable. WBF
  form: shape `5431`, pattern `(5431)`, partial permutation `(54)(31)`.
  Informal form, the only one for a 10+ card suit: shape `5=4=3=1`, pattern
  `5-4-3-1`; never mix `-` and `=`. In either form `x` is any count.
- Never alias `ddss_sys` (`use ddss_sys as dds;` collides with `dds-bridge`).
- The distributed data-gen fleet is the **fleet** (`scripts/fleet/`).
- Rejected-but-interesting treatments stay as opt-in `set_*` knobs with the
  default byte-identical. **Finished code, owed measurement → knob; unfinished
  idea, owed work → `park/` branch.** A knob that exists only to neutralise
  another knob is scaffolding: branch it.
- Visibility: plain `pub` by default; `pub(crate)`/`pub(super)` only for a
  genuine implementation detail. Widening so `web/` can reach an item is the
  expected move.
- Tests live in their own file (`foo/tests.rs` beside `foo.rs`, `mod tests;`),
  never an inline `mod tests { … }`; move one out when met.
- In a `//!` module doc whose `pub mod` line the parent also documents, the
  blocks merge and resolve in the **parent** scope: fully qualify intra-doc
  links or the rustdoc gate fails without a file:line (grep the quoted text).
  The CI `test` matrix pins stable + 1.93 on ubuntu/macos/windows; a proptest
  can fail on one cell by RNG luck.

## Working with me

I am expert in bridge, math, and low-level programming, and I am **learning
ML**: teach ML concepts grounded in those (inference = matmuls in Rust;
softmax/logits live in `src/bidding/array.rs`). Divide big tasks into small
well-specified chunks for cheaper subagents; keep design and integration in
the main loop. Repetitive, well-specified edits (the same change across N
files, bulk ledger rows) go to `codex exec`, one invocation per file, every
diff reviewed; Claude subagents are for reasoning, not transcription.

Flag dead code and doc/spec/code discrepancies explicitly, with a proposed
reversible default; never silently resolve or ship them. I may need time to
decide; keep authoring the rest.

Memory discipline: durable facts (verdicts, numbers, mechanisms, rules) belong
in the checked-in docs; Claude's memory holds only transient state (in flight,
owed) plus pointers into the docs.
