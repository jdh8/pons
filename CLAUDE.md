# pons

This crate provides tools for analyzing and simulating hands in contract bridge.
Compared to [dds-bridge](https://crates.io/crates/dds-bridge), `pons` focuses on
higher-level abstractions — most development goes into the `bidding` module:
a 2/1 game-forcing system (`american()`), a deterministic instinct floor, an
inference/constrained-sampling engine, and the AI-bidder effort that learns to
replace the floor.

Feel free to search online for authoritative sources on bridge bidding, and to
ask me questions about bidding theory. I am not an expert (yet), but I have
played a long time and read a lot. For 5-card major systems, see my
[Strawberry Polish Club](https://polish.club/).

## Objective

Make `pons` the best open-source bridge engine. Key results, in order:

1. **KR1 — bridge score** (IMPs, measured per [docs/measurement.md](docs/measurement.md)).
2. **KR2 — clean software architecture.**
3. **KR3 — acceptable performance.**

KR1 is the objective's direct proxy and outranks the rest: a KR2 or KR3 win
ships only with a KR1 **non-inferiority proof** — a seeded `smoke-default`
byte-identity of the default system, or an A/B that is a non-loss on both
scorers (measurement.md, checklist item 12).

## Read before working

| Task | Read first |
| --- | --- |
| Any change to `src/bidding` | [docs/bidding-architecture.md](docs/bidding-architecture.md) — the book/floor/inference layer cake and its invariants |
| **What BBA's book actually says** — the meaning of any call by its rules, and where its bilans floor takes over | [docs/ai-bidder/bba-book.md](docs/ai-bidder/bba-book.md) — the interpretation walk, the book/floor boundary and the two caveats that bound it, and the dictionaries (convention ids, `feature` slots, the Polish glossary). Look a lane up with `probe-bba-book --render <run> --prefix "1♠ (2♥)"` |
| Closing the gap vs BBA (the anchor, its rules, the bucket ranking) | [docs/bba-gap-campaign.md](docs/bba-gap-campaign.md) — protocol, rules, runbook, current ranking, headline trail; run `scripts/anchor.sh`, work the report's buckets. History in `docs/archive/bba-gap-*.md` |
| Closing the gap vs BEN (the new north star; BBA = exploit guard) | [docs/ben-gap-campaign.md](docs/ben-gap-campaign.md), harness built (`examples/ben-gen`, design in [docs/ben-gen-design.md](docs/ben-gen-design.md)); how BEN bids in [docs/ben-architecture.md](docs/ben-architecture.md) |
| Disclosing our system to BBA (`.bbsa` cards) | [src/bidding/card.rs](src/bidding/card.rs) — cards are **generated** from the live knob state; `cards/*.bbsa` are golden snapshots, re-blessed with `cargo run --example bba-card` |
| A knob A/B moves the floor's regime input, or a floor swap needs its siblings moved | [docs/ai-bidder/card-manifold.md](docs/ai-bidder/card-manifold.md) — the compact regime input reads **`Agreements`, not card rows**. Phase 5's v6 retrain is the shipped floor; the document holds the v4/v5 history, bias fold, and sibling-factory rule — a system name must reach the same net on its declared and undeclared paths |
| Measuring or shipping a bidding change | [docs/measurement.md](docs/measurement.md) — the A/B playbook. **No bidding change ships without it.** |
| Tuning an existing convention's range, or fixing which call leaks | [docs/convention-tuning.md](docs/convention-tuning.md) — sweep vs forensic; classify constructive/competitive first |
| Neural/AI bidder work | `.claude/skills/ai-bidder` + [docs/ai-bidder/](docs/ai-bidder/) (`README.md` then `plan.md`) |
| A call reads as nothing, or an `Or` projects to `0..=37` | [docs/ai-bidder/sampled-projection.md](docs/ai-bidder/sampled-projection.md) — read a call off the *bidder*, not its rules; includes the measured 2/1 reading bug |
| The Dutch system (champion candidate; wide non-forcing 1♣) | [docs/dutch-system.md](docs/dutch-system.md) — campaign ledger + phase plan; full bidding spec in [docs/dutch-spec.md](docs/dutch-spec.md) |
| Envelope-union readings (the historical DNF campaign; killing the Or wall) | [docs/dnf-migration.md](docs/dnf-migration.md) — chop ledger, knob matrix, the one migration rule |
| Retiring a hand-written convention reader (`inference/readers.rs`) | [docs/reader-retirement.md](docs/reader-retirement.md) — inventory, per-reader migration rule, ledger |
| A rule's constraint and its reading disagree; a "reading-only" change moved calls | [docs/reading-drift-handoff.md](docs/reading-drift-handoff.md) — the three reading regimes, why the historical DNF campaign left one uncovered, and why a reading knob is a bidding knob under a neural floor |
| An authored call reads as less than its rule says (a natural bid reads as nothing, a weak call reads as unlimited); anything touching `ReadingScope`, `Points::project`, or the walk's blankets | [docs/authored-reading-handoff.md](docs/authored-reading-handoff.md) — alert is disclosure, not the reading switch; the floor-only ceilings; the phased program (ceilings → `All` → substitute → retrain) with N2 as the testbed |
| Competitive book (we open, they interfere) | [docs/competitive-book.md](docs/competitive-book.md) — wiring idiom, package designs, campaign ledger |
| Our 1NT constructive (opening shape/range, Stayman family, transfers, Puppet, slam structure, the invite/GF seams and their evaluator verdicts) | [docs/one-notrump-constructive.md](docs/one-notrump-constructive.md) — the shipped structure, its knobs, and the A/Bs that chose them; the flat-4333 curse; the BBA comparison |
| Our 1NT contested (`1NT (2x)`, `1NT - resp (..)`) | [docs/one-notrump-competitive.md](docs/one-notrump-competitive.md) — **their overcalls are artificial** (BBA plays Woolsey Multi-Landy); per-call cost census, book/floor line, package queue. History in `docs/archive/one-notrump-competitive-closed.md` |
| The `1NT (2♦)` Multi lane specifically — which seat is authored, what each call reads as | [docs/one-notrump-multi.md](docs/one-notrump-multi.md) — the tree map (regenerate with `render-book --their-2d-multi --prefix "1NT 2♦"`), their pass-or-correct ladder, and the floor-owned holes; verdicts stay in §N4 of the campaign |
| A **minor-suit transfer** in any lane — ours, a counter-table's — or anything above its completion | [docs/minor-transfer-slam.md](docs/minor-transfer-slam.md) — the cross-lane census and the rule it produced: **an uncapped minor transfer owes a `4m` rung above its `3NT`, and that rung owes an authored answer**, because an unauthored `4m` reads as nothing and the floor's keycard ask is gated on `undisturbed` |
| Defensive round-1 redesign (they open, we act: overcalls, 1NT, takeout X) | [docs/defensive-overcalls.md](docs/defensive-overcalls.md) — 1NT + suit overcalls, first package; then [docs/takeout-double-layers.md](docs/takeout-double-layers.md) — the 4-4-major rung table and doubler rebids |
| Opener's answer to the K–K doubler's natural other major; the both-vul PD cell `multi_doubler_major` shipped with open | [docs/multi-doubler-answer-handoff.md](docs/multi-doubler-answer-handoff.md) — the measured hole is an **incomplete answer table** (no notrump rung, no escape, so a maximum with a stopper and short support passes `2♠` in a 4-2), not the rung; the repair is built behind `multi_px_split` and its unconditional arm is owed |
| Pass/double inversion — a penalty-oriented double or a converting pass flips the meaning of our later `X` and `P` | [docs/pdi.md](docs/pdi.md) — the trigger set (`grep '.penalty()'` is the live inventory), the tag + structural-conversion mechanism, the `pdi_latch` knob, and why the alert cannot carry this |
| Competitive/sacrifice accountant (the contested 5-level decision; P(double), q) | [docs/ai-bidder/competitive-accountant.md](docs/ai-bidder/competitive-accountant.md) — the floor-side gate design; evidence + q calibration in [docs/ai-bidder/doubling-calibration.md](docs/ai-bidder/doubling-calibration.md) |
| Porting a book section to rows, or the open Phases 2/3 | [docs/declarative-rows.md](docs/declarative-rows.md) — status ledger, escape-hatch inventory, the floor/knob coupling; grammar lives in `rows.rs`'s module doc |
| Kickback / Redwood (relocating the keycard ask below 4NT) | [docs/ai-bidder/bba-kickback.md](docs/ai-bidder/bba-kickback.md) §7 — BBA's next-suit-up ladder (the walk-up retired 2026-08-02), the three-arm A/B design, the two build traps (structural `alerted`, 4NT-as-answer), and the 4NT question deferred to control bids |
| The decision cache, the compiled rule path, or anything in `Context` beyond the mechanical facts | [docs/bidding-performance-handoff.md](docs/bidding-performance-handoff.md) — ~690 lines that landed in two commits with empty bodies and no CHANGELOG; this is their only design record |
| Long data-gen runs | [docs/shared-machine-data-gen.md](docs/shared-machine-data-gen.md) — this box is shared |
| Training any net, or drawing a corpus | [docs/pdd-bank-ledger.md](docs/pdd-bank-ledger.md) — **corpus from `/nfs2/jdh8/pons/`, test on fresh deals**; slice ledger + remaining-rows warning |
| Hand valuation, point counts, or "how uncertain is this hand?" | [docs/binky-points.md](docs/binky-points.md) — additive (μ, σ²) per holding; the gauge statement, and why an additive table sees only ~24% of the pair-level spread |
| Raw bidding-theory notes | [docs/bidding-theorems.md](docs/bidding-theorems.md) |

Repo skills: `author-convention` (end-to-end checklist for a new convention or
treatment) and `measure-ab` (running and interpreting an A/B). Use them.

## Workflow (non-negotiable)

- Develop and commit **directly on `main`** — every win, however small,
  lands on `main`, and a shipped win **stands** even if later parts of its
  plan fail. A feature branch has exactly one use: to **park a whole idea**
  that starts as a non-win (measured loss or wash) when a credible follow-up
  plan (a retrain, unauthored siblings, a redesign) could flip it to a win.
  Name it `park/<slug>`, push it, put the numbers and the flip plan in its
  last commit plus one row in the owning campaign doc; `git branch --list
  'park/*'` is the whole index. Rebase onto `main` before re-measuring
  (control = `main` HEAD, same `SEED_BASE`); it merges when the A/B reads a
  win.
- Only commit or push when asked.
- After updating the codebase:
  1. `cargo fmt`
  2. `cargo test --all-features`
  3. Reproduce the CI gates locally — CI runs **floating latest stable** under
     `-D warnings`, often newer than the local toolchain, so use the strictest
     available: `cargo +nightly clippy --all-targets --all-features -- -D warnings`
     and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
  4. **If the change touches the public API, `cd web && cargo test`.**  `web/` is
     its own workspace, so *nothing* above compiles it — not `--all-targets`, not
     `--all-features` — and it links pons with `default-features = false`.  CI's
     `web` job catches a break; no local command will unless you run this one.
     It consumes each reading knob as a **setter/getter pair**, so narrowing a
     `pub` getter that `src/` never reads still breaks the build.
  5. Update [CHANGELOG.md](CHANGELOG.md) with the change and its user impact
     (measured IMPs where applicable).
  6. Propose a clear, descriptive commit message.

## Iron rules

Each of these was paid for with a real regression or a wrong conclusion. The
two docs above hold the full story; the rules survive summarizing:

### Measurement (details: [docs/measurement.md](docs/measurement.md))

- Never ship a bidding change on analysis alone — run the A/B, score with
  **both** plain DD and perfect-defense, and read the verdict from the decision
  table. A PD-only win is a doubling artifact; a plain-DD wash + PD win is
  shippable default-on.
- DD is blind to obstruction and concealment. Preemptive ideas measuring
  negative is the harness, not the idea. **Right-siding is half-visible**: DD
  prices the lead direction (a different declarer puts a different defender on
  lead), not the concealment, so it sees a right-siding idea iff declarer
  actually moves — if it doesn't, measuring zero is real.
- Before declaring a measured loss dead, trace the worst divergent boards —
  the usual culprits are an unauthored continuation or an over-broad trigger.
- Measure against the **real routing** (the contract those hands actually
  reach), and complete the convention first — both sides' continuations
  **and the interfered tails** (they double it, they overcall it): an
  artificial call with only constructive responses authored is not complete.

### Architecture (details: [docs/bidding-architecture.md](docs/bidding-architecture.md))

- Every artificial call carries an `.alert(...)` and an `Inferences` reading;
  the invariant test `artificial_calls_are_alerted` enforces the alert half.
  An unread artificial call becomes a phantom-suit disaster in competition.
- Learned floors wrap **contested/defensive books only**; the constructive
  book is floored by deterministic `instinct()`. Keep the partition.
- A book node with finite mass **shadows** the floor. To give the floor a
  position, delete the node; to smarten deep continuations, improve the floor
  rather than authoring a node per bid.
- Every rule table needs a finite catch-all; a table that rejects a hand
  (all-−∞) falls through to the floor.

### Operations

- Heavy runs: `scripts/idle-run.sh`, arms **sequential** (one run saturates
  the box), fresh `SEED_BASE=$(date +%s)` per experiment shared across its
  arms, and **never rebuild binaries while an A/B is in flight**.
- The ddss `Solver` runs on the main thread only; parallelize bidding with
  rayon, never the solver.

### Conventions of the house

- Human-authored auctions are space-delimited: `1NT - 2♣ (X) XX -`.  Write
  every pass as `-`, parenthesize only opponents' non-pass calls, and expand
  implied passes (`1NT - 2♦ - 2♥`).  `P*` remains the row grammar's
  leading-pass fan; use `P`/`(P)` only when discussing legacy input or quoting
  an external format.  Probe/render binaries ignore parentheses in auction
  input; only the row grammar (`rows.rs`) seat-checks them.
- Never alias `ddss_sys` (`use ddss_sys as dds;` collides with `dds-bridge`).
- The distributed data-gen fleet is called the **fleet** (`scripts/fleet/` on
  its machines), never a "botnet".
- Rejected-but-interesting treatments stay as opt-in `set_*` knobs with the
  default system byte-identical — many are single-dummy re-measure candidates.
  The split with a `park/` branch: **finished code, owed measurement → knob**;
  **unfinished idea, owed work → branch**. A knob that exists only to
  neutralise another knob is scaffolding — the smell that says "branch".
- Visibility: plain `pub` by default; `pub(crate)`/`pub(super)` only for a
  genuine implementation detail that must cross a module boundary (widening
  to `pub` so `web/` can reach an item is the expected move, not a concession).
- Tests live in their own file (`foo/tests.rs` beside `foo.rs`, declared
  `mod tests;`) — never a new inline `mod tests { … }`; move one out when met.
- In a `//!` module doc whose `pub mod` line the parent also documents, the two
  blocks merge and resolve in the **parent** scope: fully qualify intra-doc
  links (`` [`x`][crate::bidding::neural::x] ``) or the rustdoc gate fails
  without a file:line (grep the quoted text). Links on private items are
  unchecked. The CI `test` matrix pins stable + 1.93 on ubuntu/macos/windows;
  a proptest can fail on one cell by RNG luck.

## Working with me

I am expert in bridge, math, and low-level programming, and I am **learning
ML** — teach ML concepts grounded in those (inference = matmuls in Rust;
softmax/logits already live in `src/bidding/array.rs`). Divide big tasks into
small well-specified chunks for cheaper subagents; keep design and integration
in the main loop. Repetitive, well-specified edits (the same change across N
files, bulk ledger rows) go to `codex exec` (my Codex subscription), one
invocation per file, every diff reviewed — Claude subagents are for reasoning,
not transcription.

Flag dead/unreachable code and doc/spec/code discrepancies explicitly, with a
proposed reversible default — never silently resolve or ship them. I may need
time, or a discussion with other players, to decide; keep authoring the rest.

Memory discipline: durable facts (verdicts, numbers, mechanisms, reference
distillations, rules) belong in these checked-in docs; Claude's memory holds
only transient state (in flight, owed) plus pointers into the docs.
