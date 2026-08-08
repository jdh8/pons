# The card manifold — four trained coordinates out of 280

**Status: diagnosed, nothing built.** The configured net reads a 280-float
convention-card block, 76% of its 368 inputs. Across the entire v4 corpus
**four slots per side move**; the other 272 coordinates are constant, and the
trainer ran `wd = 0`, so their weights sit at their initialisation draw.
Flipping one injects a *random* vector into the hidden layer — that is the
≈ −0.015 IMP/board "frozen-coordinate tax" measured twice on 2026-08-07.

Two remedies, and they are **separable**:

- **The fold** (§[The fold](#the-fold-exact-free-unbuilt)) — a constant input is
  algebraically a bias term, so folding it into `b₁` and zeroing its column is
  *exact on the training distribution* and drives the tax to zero. No retrain,
  no corpus, no measurement. Card rows become **safe**.
- **The retrain** (§[The retrain](#the-retrain-v5-conditional-on-e3)) — a corpus
  that varies chosen card rows. Card rows become **meaningful**. Costs a corpus,
  and is gated on E3 below.

Read [configured-net.md](configured-net.md) first — this file is a correction to
it, not a replacement. The measured numbers live in
[declarative-rows.md](../declarative-rows.md) and are cited, not restated.

**Why anyone should care:** the card is currently a *write-only* channel. We can
disclose an agreement to BBA through it, but we cannot change what we disclose
without paying the tax — which is why a known misdisclosure ships today in a
comment rather than in the value ([card.rs](../../src/bidding/card.rs), the
`"Two way game tries"` arm). It is also why
[declarative-rows.md](../declarative-rows.md)'s Phase 3 has no measurable payoff.

## Why: the tax, priced

Three measurements, all 2026-08-07, all recorded with their provenance in
[declarative-rows.md](../declarative-rows.md):

| perturbation | plain DD | perfect defense | source |
| --- | --- | --- | --- |
| our own card, **1** frozen bit (`Two way game tries` → its honest `0`) | **−0.0174** ±0.0064 NV / **−0.0130** ±0.0077 vul | wash | `declarative-rows.md` §"A one-bit `theirs` perturbation" |
| `theirs`, **1** frozen bit (`--their-ns`, three arms) | ≈ **−0.01** | ≈ **−0.02** | same section |
| `theirs`, **43** frozen bits (cell A, vs BBA's real 2/1 card) | **−0.7015** ±0.1622 | **−1.1615** ±0.1950 | §"2a measured" |

Cell A loses about the whole BBA gap. Note the scaling: 43 bits cost roughly 43×
one bit, which is what independent random perturbations do and what a *feature
the net understands* would not.

The tell that it is the coordinate and not the meaning: the `Two way game tries`
correction moved 2647 of 20 000 `smoke-default` boards, and the worst of them
are `1♣ - P - 1♠ - 4NT` and a left-in `X` — auctions with no game try anywhere.

### The mechanism

`classify_bba_v4` is a bare MLP over the raw feature vector — `Linear(368,256) →
ReLU → Linear(256,256) → ReLU → Linear(256,38)`, 170 022 floats, no batchnorm,
no layernorm, no scaling, no embedding table between `features_v4` and `W₁x + b₁`
([neural.rs](../../src/bidding/neural.rs), `forward`). `encode_card` writes plain
`0.0`/`1.0` ([features.rs](../../src/bidding/features.rs)).

So for the first hidden layer, `h = W₁x + b₁ = Σᵢ xᵢwᵢ + b₁`. Split the inputs
into the ones that move in the corpus (L) and the ones constant at `cᵢ` (F):

```
h  =  Σ_{i∈L} xᵢ wᵢ  +  Σ_{i∈F} cᵢ wᵢ  +  b₁
                        └──── a constant vector ────┘
```

The second term is the same on every training row. It is therefore
**indistinguishable from the bias**: no gradient ever separates `wᵢ` (i ∈ F) from
`b₁`, and any split of the constant between them fits the data identically.
`wᵢ` is not "learned to be small" — it is *unidentified*, and it stays wherever
`--init-seed 1` put it. With `wd = 0.0`
([american_bba_v4.json](../../src/bidding/weights/american_bba_v4.json)) nothing
shrinks it either; candle's `AdamW` decay is decoupled, so it is the only force
that could have.

At serving, flipping such a coordinate from `cᵢ` to `1 − cᵢ` adds `(1 − 2cᵢ)wᵢ`
to every hidden pre-activation. That is not a wrong opinion about the agreement.
It is a **random** one — `wᵢ` is a fresh draw from
[`candle_nn::linear`](../../trainer/src/model.rs)'s default kaiming-normal,
std `√2/√368 ≈ 0.07` per component across all 256 hidden units.

### How many coordinates are frozen: 272 of 280

`dump-teacher`'s side configs are `{dutch, kickback}`, rotated through six table
configurations giving eight ordered cells
([configured-net.md](configured-net.md) §"The cells"). Both blocks see all four
side configs, so per side the union of everything that ever moves is:

| what moves | in-block slot | absolute (ours / theirs) |
| --- | ---: | ---: |
| base-system one-hot bit 0 (2/1 GF) | 0 | 88 / 228 |
| base-system one-hot bit 2 (WJ) | 2 | 90 / 230 |
| `1D opening with 5 cards` | 7 | 95 / 235 |
| `Kickback 1430` | 77 | 165 / 305 |

Four of 140 per side, **eight of the 280 card inputs**. The remaining **272 are
provably constant** — pinned from the other direction by two existing tests:
`the_base_system_is_encoded` asserts `differing == 3` for american↔dutch, and
`a_convention_knob_moves_the_card_block` asserts `differing == 1` for
kickback ([features/tests.rs](../../src/bidding/features/tests.rs)).

This is the fine print behind the invariant `configured-net.md` already states —
*only vary configuration the card can express* — and behind the warning in
[features.rs](../../src/bidding/features.rs)'s extractor doc, *"a v4 net is only
responsive along the axes its corpus actually varied."* What was never written
down is the **cost of the axes it did not vary**, which is what the table above
prices.

## The fold: exact, free, unbuilt

Since the 272 columns only ever contribute a constant, move that constant into
the bias where it belongs and zero them:

```
for each input i whose corpus value is constant at c_i:
    b1[h] += c_i * W1[h*368 + i]   for all h in 0..256     # accumulate in f64
    W1[h*368 + i] = 0
```

`l1.weight` is `[256, 368]` row-major (candle's `Linear`, `y = x·Wᵀ + b`), so
input coordinate `i` is at stride-368 positions `h*368 + i`
([neural.rs](../../src/bidding/neural.rs),
[trainer/src/model.rs](../../trainer/src/model.rs)).

**On the training distribution this is exact**, not an approximation: `h` is
unchanged for every row the net was fitted on, so every trained cell behaves
identically. At serving, a frozen row becomes a no-op instead of noise. The tax
goes to **zero**, not "down".

**Tool.** One script, `scripts/fold-constant-inputs.py`, modelled on
[pair-flip-diagnostic.py](../../scripts/pair-flip-diagnostic.py)'s read pattern
(`np.fromfile(stem + ".f32", dtype="<f4")`). Constancy comes from a **full
streamed min/max scan of the corpus stems over all 368 inputs** — not from
sampling, and not from re-deriving the four cards in Python. A sampled scan can
miss a rare variation and zero a live column, which would be a silent bidding
change. `np.memmap` in chunks. Scan all 368, not just the card block: any
constant v3 coordinate folds by the same argument.

**⚠ Deviation, as shipped (2026-08-08).** The scan had no input: the v4 corpus
(`target/corpus-v4/`, 12 stems, ~5.7 GB) no longer exists on disk — the sidecar
records the draw exactly, but regenerating it costs ≈5M EPBot calls, roughly a
v5 dump. The shipped script therefore runs in **geometry mode**: the fold set
is the card block minus the eight live slots hardcoded from the cell geometry
(`{88, 228} + {0, 2, 7, 77}`, the derivation in §"How many coordinates are
frozen"), and the fixture's 8 stored feature rows supply only the constant
*values* (exact 0/1 bits, all rows agree — asserted). The fixture **cannot**
substitute for the scan: six of the eight live slots are constant across its 8
rows, so a constancy-derived set would wrongly fold them. In exchange, v3
coordinates (0..88) stay unfolded, and the certificate moved from ex-ante scan
to the gates below — all three held: fixture un-re-blessed, smoke
byte-identical, E1's flip 0 of 20 000. Scan mode is owed at the v5 export,
where a corpus is on disk by construction.

The rewritten blob must keep all 170 022 floats —
[neural.rs](../../src/bidding/neural.rs) has a compile-time size assert.

### ⚠ The gate

The fold is exact in exact arithmetic, so the only thing that can move is f32
rounding on a reassociated sum: ~1e-7 absolute, against logits of order 1.

- **Expect `examples/smoke-default` byte-identical.** If it moves, every moved
  board must be a top-2 logit near-tie; say so in the CHANGELOG rather than
  shipping it quietly. A diff larger than a handful of near-ties means the
  constancy scan was wrong, not that the arithmetic was.
- `matches_candle_fixture_bba_v4` ([neural/tests.rs](../../src/bidding/neural/tests.rs))
  holds pre-fold logits at a 1e-3 bar with exact argmax. It should pass
  unchanged. **Re-blessing it is not a formality** — if it fails, that is
  evidence the fold moved more than rounding, and the cause must be found before
  the fixture is regenerated.

### Two alternatives, and why not

- **Train with `wd > 0` instead.** Decoupled AdamW shrinks by `(1 − lr·wd)` per
  step. The shipped run was `lr = 1e-3` over 300 epochs × ⌈3 026 601/4096⌉ = 221 700
  steps, so meaningful decay needs `lr·wd·steps ≳ 5`, i.e. **`wd ≳ 0.02`** —
  which shrinks the weights that matter just as hard. It also only helps the
  *next* net. The fold is exact, targeted, and retroactive.
- **Fold inside the trainer's export.** That cannot fix v4 without retraining
  it. Run the script as the last step of any train instead; the trainer stays
  untouched.

## The three cheap experiments

Ordered. None needs a retrain, and E1 needs no A/B at all.

| # | run | cost | what it decides |
| --- | --- | --- | --- |
| **E1** | fold, then set `"Two way game tries"` to its honest `0` and dump `smoke-default` | seconds, deterministic | **the fold works.** The row is frozen, therefore zeroed, therefore flipping it must move **0 of 20 000** boards. Converts a −0.015/board A/B into an assertion. **✓ PASSED 2026-08-08**: sha `59a27d7f…` identical across baseline, fold, and honest-0 (2647 boards moved pre-fold). |
| **E2** | folded vs unfolded `--declare-opponents` cell A (vs BBA's real 2/1 card, 43 rows differing) | 2000 bd/arm/vul, harness exists | **the diagnosis, experimentally.** −0.70/−1.16 must collapse to ~0. The cheapest possible test of everything above: if cell A does *not* collapse, the frozen-coordinate story is wrong. **✓ PASSED 2026-08-08**: on the original deals (seed 424242, both arms at HEAD), **0 of 2000 boards fired at either vul** — all 43 differing rows sit on zeroed columns, so the whole −0.70/−1.16 was the tax. Declaring BBA's card is a no-op until a retrain thaws axes. |
| **E3** | **cell B at scale** — `--their-floor dutch --declare-opponents`, 204 800 bd/arm/vul, both vuls, dual scoring | minutes | **the retrain's go/no-go.** Cell B moves slots {0, 2, 7}, all genuinely *trained*. A wash here says the trained coordinates are worth ~0 too, and the retrain is dead. **Run 2026-08-08** (`scripts/ab-declared-opponents.sh`, `ab-results/card-fold-e3`): plain **wash** both vuls (−0.0021 ±0.0033 NV / +0.0003 ±0.0039), PD **+0.0024 ±0.0042 NV / +0.0062 ±0.0048 vul** — CI-clear positive in one of two PD cells, fired 1.8/1.5%, +0.41 PD IMPs/fired at vul. By the decision table this is the *plain-wash + PD-win* row, not the killing wash — the trained channel is real but worth ≈ +0.004 PD/board at the **widest** trained axis, which brackets what any single thawed axis could add. |

E3 is not a new gate: it is the run
[declarative-rows.md](../declarative-rows.md) §"2a measured" already books as
owed, and commit `734aa66` made the Dutch seat's own config truthful, which it
was not when that prescription was written.

E1 and E2 are properties of the *fold* and do not depend on E3's outcome. E3
alone decides whether to spend a corpus.

*Repro:* E1 — `cargo run --example smoke-default`, diff against the pre-fold
dump. E2/E3 — per [`.claude/skills/measure-ab`](../measurement.md): fresh
`SEED_BASE=$(date +%s)` shared across an experiment's arms, arms **sequential**
under `scripts/idle-run.sh`, score with **both** plain DD and perfect defense,
read the verdict off `measurement.md`'s decision table. E2 reuses
`--declare-opponents`; E3's shape is `scripts/ab-declared-book.sh`'s with
`--declare-opponents` in place of `--declare-their-book`.

## The retrain: v5, conditional on E3

**The central design claim, and the reason the fold comes first:** with the fold
in the pipeline, **partial coverage is safe**. A v5 net that thaws eight axes has
eight trained coordinates and 264 zeroed ones; the un-thawed rows are *inert*,
not landmines. So the retrain never has to cover all 280, and it can be
incremental — thaw the axes you want to measure, fold the rest, repeat. Without
the fold, every row the corpus does not reach is a trap that fires the first time
someone sets its knob.

### Axis selection

Candidates are the **24 match arms in `american_row` that read live state**
(`configured-net.md` §"The sync"); the other ~196 knobs move the bidding and are
invisible to the card. Each candidate clears three gates *before* it enters a
corpus. This is a probe, not a guess — the list is deliberately not pre-committed
here.

1. **Truthful.** Arming the pons knob really moves the row *and* really moves our
   book. Otherwise two cells collide into identical feature vectors with
   contradictory targets — the exact mixed-net failure this whole design exists
   to prevent ([dump-teacher](../../examples/dump-teacher/main.rs), `SideConfig`'s
   doc comment).
2. **Sticky in EPBot.** The card is pushed as teacher overrides through
   `to_convention_card` and read back by `verify_card`. ⚠ **A row that only
   passes by sitting in `KNOWN_UNSTICKY` is disqualified.** That allowlist is
   sound only for rows we never vary — `configured-net.md` §"Three traps" already
   says the `Multi` entry must go if Multi ever becomes a knob. Varying a row
   EPBot silently refuses means the teacher's targets contradict the label.
3. **High-frequency.** The argument that put Dutch in the v4 corpus: a bit
   deciding ~0.05% of boards leaves its weight near initialisation, so a
   high-frequency axis has to carve the pathway that rare bits then ride. Start
   with axes that move many boards.

**Probed 2026-08-08** (`examples/probe-card-axes.rs`, 20 000 seeded boards per
axis, all four seats the default system, flip vs shipped defaults, auctions
diffed). Every axis moves auctions, so gate 1's book half passes everywhere;
gate 2 is enforced at dump time (none of the 24 rows is in `KNOWN_UNSTICKY`
today). The gate-3 ranking:

| axis | moved of 20 000 | % |
| --- | ---: | ---: |
| Two Way NMF (XYZ) | 1202 | 6.01 |
| NT defense (Landy rows) | 1102 | 5.51 |
| Lebensohl rows | 731 | 3.65 |
| Checkback (NMF) | 458 | 2.29 |
| 1NT minor scheme | 406 | 2.03 |
| 1NT shape ladder | 403 | 2.02 |
| Landy range | 338 | 1.69 |
| Jordan Truscott 2NT | 338 | 1.69 |
| 1NT offshape 4441/5422 | 146 | 0.73 |
| Garbage Stayman | 67 | 0.34 |
| Support double/redouble | 59 | 0.29 |
| Fourth suit forcing | 48 | 0.24 |
| Responsive double | 44 | 0.22 |
| Leaping Michaels | 38 | 0.19 |
| Super acceptance | 27 | 0.14 |
| 1N-3M splinter | 8 | **0.04** — fails the bar above |

The natural thaw set is the top eight (≥ 1.69%, at or above cell B's own
1.5–1.8% fired rate); the sub-0.5% tail waits for the pathway those eight
carve.

### Corpus shape

Keep `DEFAULT_CELLS` for the bulk so gate 1 stays comparable to v4, and add
randomized shards on top: `SideConfig` grows from `{dutch, kickback}` to carry a
knob set drawn per shard from the curated axis list under a deterministic
`--axis-seed`.

Start at v4's size — 250k uniform + 500k enriched → ~3.4M rows over 12 shards
(`configured-net.md` §"Phase 3, measured") — and **scale only if the diagnostic
below says to.** "A card row is a modifier of my behaviour" is one mechanism
shared across rows; there is no reason to assume it needs k× the data for k axes,
and the diagnostic will say directly whether it did.

Register the bank window in [pdd-bank-ledger.md](../pdd-bank-ledger.md) **before**
the run. Draw from `22.pdd`; leave `24.pdd`'s remaining ~19.7M rows for A/B
slices that genuinely need pre-solved tables.

Carry forward v4's three dump traps unchanged: `--teacher bba` (the default is
the Rust floor, not EPBot), `--dd-weight 0` (bank rows carry DD tables), and the
`Multi`/`verify_card` allowlist — now with gate 2 above constraining it.

### Gates

Generalize [pair-flip-diagnostic.py](../../scripts/pair-flip-diagnostic.py) from
slot 77 to an arbitrary slot. It is already the pre-gate that separates "the net
never learned the bit" from "the bit is worth little", and for v5 it becomes
two-sided:

- **every thawed axis** moves argmax on a healthy fraction of held-out
  teacher-moving pairs. v4's slot 77 managed 223/492 = 45.3%, mean max-|logit|
  shift 6.98 — that is the bar to beat, and 45% was already described as leaving
  headroom.
- **every frozen row moves nothing.** The fold guarantees this, which makes it a
  cheap regression test on the export rather than a hope.

Then the two A/B gates from [measurement.md](../measurement.md): v5 at defaults
must not regress against shipped v4, and — the point of the exercise — each
thawed axis's own knob A/B becomes interpretable for the first time, because its
result is no longer its effect plus a random vector.

## What this revives

[declarative-rows.md](../declarative-rows.md)'s Phase 3 (the knob migration, whose
terminal step is `Card::from(&config)`) is recorded as *not scheduled*, on the
grounds that its payoff is structural and the one thing that could make it a
measured win — the declaration channel — is unmeasurable. That is right, and this
file supplies the missing *why*, plus one correction:

- `Card::from(&config)` is **structurally inert at defaults**. It changes where
  the card reads from, not what it says.
- What Phase 3 actually buys is that every knob's card row becomes a live input
  the floor reads. Today that is a **liability** — the tax. After the fold it is
  **free but meaningless**. After the retrain it is the thing the migration
  exists for.
- So **Phase 3's gate is not cell B; it is whether the card block is a trained
  input at all.** Cell B (E3) remains the cheap go/no-go for spending a corpus on
  making it one.

### The standing consequence for measurement

Until the fold landed, **any knob A/B that moved one of the 24 card-expressible
rows measured its own effect plus the tax**, and the tax was larger than most
conventions' real effect. Both live instances are resolved (2026-08-08):

- `"Two way game tries"` now ships its honest `0` in the value — E1's flip
  moved 0 of 20 000 boards, so the correction that used to cost −0.015/board of
  pure noise was free. (The row also leaves the retrain's axis-candidate list:
  its honest value is knob-independent until the shortness try is authored.)
- Commit `b750a4e`'s `1NT opening shape 4441` row (reads
  `one_notrump_offshape()`, default-off): its future A/B no longer inherits the
  tax — the row's column is zeroed, so arming the knob moves the *bidding*
  only, not a random vector through the card.

The general statement survives for any **future** unfolded export, which is
what `folded_card_columns_are_exactly_zero` exists to catch.
