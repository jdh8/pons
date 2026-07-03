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

## Read before working

| Task | Read first |
| --- | --- |
| Any change to `src/bidding` | [docs/bidding-architecture.md](docs/bidding-architecture.md) — the book/floor/inference layer cake and its invariants |
| Measuring or shipping a bidding change | [docs/measurement.md](docs/measurement.md) — the A/B playbook. **No bidding change ships without it.** |
| Neural/AI bidder work | `.claude/skills/ai-bidder` + [docs/ai-bidder/](docs/ai-bidder/) (`README.md` then `plan.md`) |
| Long data-gen runs | [docs/shared-machine-data-gen.md](docs/shared-machine-data-gen.md) — this box is shared |
| Raw bidding-theory notes | [docs/bidding-theorems.md](docs/bidding-theorems.md) |

Repo skills: `author-convention` (end-to-end checklist for a new convention or
treatment) and `measure-ab` (running and interpreting an A/B). Use them.

## Workflow (non-negotiable)

- Develop and commit **directly on `main`** — no feature branches.
- Only commit or push when asked.
- After updating the codebase:
  1. `cargo fmt`
  2. `cargo test --all-features`
  3. Reproduce the CI gates locally — CI runs **floating latest stable** under
     `-D warnings`, often newer than the local toolchain, so use the strictest
     available: `cargo +nightly clippy --all-targets --all-features -- -D warnings`
     and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
  4. Update [CHANGELOG.md](CHANGELOG.md) with the change and its user impact
     (measured IMPs where applicable).
  5. Propose a clear, descriptive commit message.

## Iron rules

Each of these was paid for with a real regression or a wrong conclusion. The
two docs above hold the full story; the rules survive summarizing:

### Measurement (details: [docs/measurement.md](docs/measurement.md))

- Never ship a bidding change on analysis alone — run the A/B, score with
  **both** plain DD and perfect-defense, and read the verdict from the decision
  table. A PD-only win is a doubling artifact; a plain-DD wash + PD win is
  shippable default-on.
- DD is blind to obstruction, concealment, and right-siding. Preemptive ideas
  measuring negative is the harness, not the idea; right-siding-only ideas
  measuring zero is real.
- Before declaring a measured loss dead, trace the worst divergent boards —
  the usual culprits are an unauthored continuation or an over-broad trigger.
- Measure against the **real routing** (the contract those hands actually
  reach), and complete the convention — both sides' continuations — first.

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

- Never alias `ddss_sys` (`use ddss_sys as dds;` collides with `dds-bridge`).
- The distributed data-gen fleet is called the **fleet** (`scripts/fleet/` on
  its machines), never a "botnet".
- Rejected-but-interesting treatments stay as opt-in `set_*` knobs with the
  default system byte-identical — many are single-dummy re-measure candidates.

## Working with me

I am expert in bridge, math, and low-level programming, and I am **learning
ML** — teach ML concepts grounded in those (inference = matmuls in Rust;
softmax/logits already live in `src/bidding/array.rs`). Divide big tasks into
small well-specified chunks for cheaper subagents; keep design and integration
in the main loop.
