# The floor sweep — seed noise, ensembles, and the recipe's free parameters

**Status: Phase 1 shipped (2026-09-28) — K = 4 ensemble, win / win on
every cell; the five rails re-arbitrate as still earning; embedded and
flipped as the default floor. Owed: K = 8 and Phase 2 (jdh8's calls).** Precondition met: jdh8 kicked the
sweep off, un-deferring *training-only* work. The 2026-09-26 deferral was of retrains that
need a dump (hours) or a relabel (the fleet-week); this plan needs neither.
Every step below is a 10-minute train on the SSD corpus plus a 90-minute A/B.

Read first: [README.md](README.md) (vocabulary),
[features-v8.md](features-v8.md) §3 (the manifest command and the fold),
[02-policy-net.md](02-policy-net.md) ledger (why fidelity never ranks arms),
[../measurement.md](../measurement.md) (the decision table),
[../floor-rail-campaign.md](../floor-rail-campaign.md) stop criterion 5
(a new net re-arbitrates every shipped rail).

## Why this and not the other three ideas

The four candidates of 2026-09-28 were: improve the defensive system, add
Polish Club, an LSTM over the auction, better parameters for the net. The
other three are parked with reasons in
[../next-steps.md § Parked big ideas](../next-steps.md#parked-big-ideas).
This one won because:

- **The floor is the largest lever the ledger has ever recorded** (the v3
  floor swap alone bought `american()` +0.11 / +0.25 IMPs/board; v4 and v6
  each moved it again), and every open residue in
  [../next-steps.md](../next-steps.md) is now "retrain-gated". The net is
  the bottleneck of the whole programme.
- **The recipe has never been searched.** Every shipped net since v4 is
  *one draw of one configuration*: `--hidden 256 --epochs 300 --lr 0.001
  --wd 0 --batch 4096 --val-frac 0.10 --dd-weight 0 --init-seed 1`. No
  width, regularisation, schedule, or seed was ever compared on IMPs.
- **Seed noise alone was ≈ 0.02–0.05 IMPs/board** when the init was
  unseeded ([02-policy-net.md](02-policy-net.md) ledger, 2026-07-24). That
  spread is larger than every convention A/B shipped since. `--init-seed 1`
  made the draw *reproducible*; it did not make it *good*. We have never
  measured how far the shipped draw sits from the mean.
- **It is cheap in exactly the way the deferral wanted.** Corpus:
  `target/corpus-relabel-m32` (6.0 GB, 20 stems, on the SSD, rows carry
  `[176 features][38 teacher_softmax][20 dd_tricks]`). Train: 9 min 42 s on
  the 4090 (features-v8.md §3); two GPUs run two draws at once. A/B: 87 min
  at 204,800 bd/arm/vul (`ab-results/v8-floor.log`, 21:32→22:58).

## Rules

1. **The corpus is frozen.** M32 labels, v6 features, the 20 stems as the
   manifest lists them. No dump, no relabel, no feature bump — a feature bump
   is [features-v8.md](features-v8.md)'s programme, not this one. This keeps
   every arm an equal-data comparison against the shipped artifact.
2. **Fidelity is a filter, never a predictor** (measured twice,
   [02-policy-net.md](02-policy-net.md)). Held-out CE / top-1 rejects a run
   that failed to train; it never ranks arms. v8 had the best CE ever seen
   and measured *suspect*. Ranking is IMPs on the A/B, both scorers, read
   from the decision table.
3. **One binary per campaign.** Candidates load their weights from files at
   run time (the `american-file` arm below), so a whole sweep runs on one
   build and the never-rebuild-mid-flight rule holds by construction.
4. **Control is always the shipped `american()`** (M32 v6, seed 1). Fresh
   `SEED_BASE` per experiment, shared by its arms; arms sequential;
   `scripts/idle-run.sh`.
5. **A win smaller than the seed noise is a reseed, not a recipe.** Phase 0
   measures σ_seed first; every later verdict is read against it.
6. **Shipping = decision table + rail re-arbitration.** The five shipped
   floor rails were arbitrated against the seed-1 v6 net. A new net re-runs
   each rail's runner (floor-rail-campaign.md stop criterion 5) before the
   flip. Then `smoke-default` re-blesses and CHANGELOG records the numbers.

## Phase 0 — the noise floor

Train seeds 2 and 3 with the manifest command otherwise unchanged. A/B each
against the shipped net, 204,800 bd/arm/vul, both vuls, both scorers.

Read it three ways:

- **Reseeds swing CI-clear (≥ ±0.01 on either scorer).** Then (a) the
  shipped draw is one sample of a wide distribution and Phase 1's ensemble
  is the first lever; (b) every past floor A/B that changed the net *and*
  the recipe in one arm (v8, the three LSTM arms) was confounded by its
  draw — record that in each doc's verdict as a caveat, do not re-run them.
- **Reseeds wash (inside ±0.005).** Seed noise is already solved; skip Phase
  1 and go to Phase 2 with single draws.
- **In between.** Phase 1 still runs; Phase 2 reads every win against σ_seed.

Cost: two trains (20 min, one per GPU), two A/Bs (3 h idle time).

## Phase 0 verdict (2026-09-28)

Rows in the ledger; sd plain was +0.0304 / +0.0149 (seed 2) and +0.0286 /
+0.0211 (seed 3), CI-clear in three of four cells. Fired 15.5% (none), 13.5%
(both). Fidelity passed the filter (val_ce .4172 / .4165 vs shipped .4163).

**Reading: the first branch — reseeds swing CI-clear**, on both scorers
(plain none +0.022 and +0.015; PD none −0.020 on seed 3). But the swing is
not symmetric noise around seed 1. **Both reseeds moved the same way on both
scorers — plain DD up, PD down** — and with the same mechanism the v8 trace
found (§5 of [features-v8.md](features-v8.md)) and the LSTM's overbidding
diagnosis ([plan.md](plan.md) M5.2):

| none, all 204,800 boards × 2 tables | doubled contracts | auctions ending `XX` |
| --- | ---: | ---: |
| shipped (three control runs) | 5.57–5.62% | 372–433 |
| seed 2 | 6.24% | 603 |
| seed 3 | 6.40% | 750 |
| v8 | 5.83% | 645 |

The worst PD boards are the same family: our side acting over their 4NT
(`4NT 5♠ X`), late doubles and four/five-level redoubles (`3♠ 4♠ X XX`).

**Interpretation — the shell is fitted to seed 1.** The five floor rails
were arbitrated against the seed-1 net, and the accountant collar was
calibrated to its distribution (M5.2's flip plan already says so for v7).
Any other draw of the *same* recipe brings junk actions the shell was never
fitted to catch, and PD prices them. So:

1. **v8 and the three LSTM arms are confounded** (the plan's branch (b)).
   v8's signature (+0.017 / −0.013) sits inside the reseed spread; its
   verdict is "indistinguishable from a reseed", and its trace's causal
   story (the artificial block puts their 4NT off-distribution) is not
   needed to explain it. The LSTM arms' PD losses (−0.024 to −0.043) are
   larger than both reseeds, so "refuted" weakens but does not flip; their
   plain columns (+0.001 to +0.015) are at or below the reseeds'. Caveats
   recorded in both docs; not re-run.
2. **The decision table cannot rank candidates while the shell is seed-1's.**
   Every non-seed-1 net lands on the *suspect* row by construction. Rule 6's
   "rail re-arbitration before the flip" is too late: a candidate's PD column
   already carries the shell-fit tax.
3. **The plain-DD column says the shipped draw is below the mean** of its
   recipe by ~0.015–0.02 IMPs/board (none). Whether that is recoverable is
   exactly what PD, not plain, has to show.

**Next (proposed, not started).** Phase 1 still runs as planned (K = 4,
seed 4 to train) — averaging logits should shrink exactly the idiosyncratic
junk actions — but read its PD column against this table, not against zero.
If it also lands on the *suspect* row, the lever is re-fitting the shell to
the candidate (collar retune first: one knob, and the LSTM flip plan's arm 1
is the same work), not another recipe axis.

## Phase 1 — the ensemble (variance reduction at the same recipe)

`K = 4`: seeds 1–4, seed 1 being the shipped blob. Average the **logits**
(the 38-vector, elementwise mean) before the shell masks and argmaxes.
Logits, not probabilities: the consumers downstream read margins and
precedence, not odds ([logit-calibration.md](logit-calibration.md)), and a
mean of logits keeps the shipped calibration story intact. Serving cost is
`K × (176·256 + 256·256 + 256·38) ≈ 0.5 MFLOP` per decision, invisible
beside a single double-dummy solve (KR3 holds).

One A/B vs shipped. Ship if the decision table says so and the rails
re-arbitrate; try `K = 8` only if `K = 4` wins. Shipping form is `K`
embedded blobs (≈ 0.5 MB each): an ensemble does not fold into one MLP.
Size is settled (jdh8, 2026-09-28: disk is cheap); **compute is the KR3
budget**. Before a flip, time a bidding-only run (no DD) at `K = 1` vs the
candidate `K`; the forward pass is ~3 µs per decision at ~40 GMAC/s, so
`K = 4` should add ~10 µs per contested off-book decision — confirm it,
and let a measurable slowdown veto `K = 8`.

## Phase 1 verdict (2026-09-28)

**K = 4 wins every cell, on every scorer**, CI-clear by 4–7σ; row in the
ledger. Fired 9.97% (none), 8.70% (both), +0.45 to +0.89 IMPs/fired. sd
plain +0.0409 ±0.0088 / +0.0502 ±0.0103. Decision table: *win / win*.

The PD column is the news. Each member alone lands on the *suspect* row
(plain up, PD down); their logit mean lands on neither: PD is the
**largest** column, and the junk-double signature is gone —

| none, all 204,800 boards × 2 tables | doubled contracts | auctions ending `XX` |
| --- | ---: | ---: |
| shipped (this run's control) | 5.57% | 434 |
| K = 4 | 5.60% | 403 |
| (both vul: shipped / K = 4) | 4.67% / 4.62% | 434 / 383 |

So Phase 0's reading holds in the direction it predicted: the reseeds'
PD tax was idiosyncratic per-draw junk, which averaging cancels, and the
seed-1 shell does not need re-fitting to serve the ensemble. The plain
gain (+0.045 / +0.056) is 2–4× a single reseed's, i.e. more than "the
shipped draw was below its recipe's mean" — the variance reduction
itself is worth IMPs.

**KR3 timing (2026-09-28).** `examples/floor-timing.rs`: pons vs pons,
both sides on the floor under test, 20,000 seeded boards, single thread,
three interleaved repetitions, the box lightly loaded:

| floor | µs/board | µs/call |
| --- | ---: | ---: |
| `american` (embedded, K = 1) | 156–164 | 15.5–16.3 |
| `american-file`, K = 1 (shipped blob) | 159–166 | 15.7–16.4 |
| `american-file`, K = 2 | 221–231 | 22.1–23.1 |
| `american-file`, K = 4 | 351–358 | 35.0–35.8 |

Linear, **+65 µs/board per extra member**, so K = 4 is 2.2× the bidding
latency and K = 8 extrapolates to ~620 µs/board (~3.9×). The prediction
above ("~10 µs per contested off-book decision") was wrong by the call
count, not the pass cost: 65 µs ÷ ~3.2 µs per forward pass ≈ 20 passes per
board, i.e. **about two per call, on every call**, not on the ~10% that
diverge. Where the net runs on book-answered calls (the shell, a reading, a
cache miss) is unexplained and is the cheapest KR3 lever if it matters.
In the real harness the cost mostly vanishes behind BBA and the solver:
`sweep-k4` generation took 105 → 112 s (none) and 104 → 112 s (both),
+7%, and generation is ~7 of the run's 62 minutes.

Reading: K = 4 passes KR3 (the solver-bound pipeline slows ~1%; raw
bidding latency is still ~0.35 ms/board). K = 8 doubles that again for a
gain K = 4 has not shown is unsaturated — jdh8's call.

**Owed before the flip** (rule 6 and Phase 1's own gates):
1. ~~KR3 timing, bidding-only, `K = 1` vs `K = 4`.~~ Done above.
2. ~~Rail re-arbitration of the five shipped rails against the ensemble.~~
   Done below.
3. ~~Embed the three extra blobs; `american()` becomes the mean.~~ Done:
   `neural::classify_bba_v6` is the mean of four embedded draws, so every
   `with_floor_v6` path flips at once. The KR1 proof is the A/B itself: the
   embedded bytes are the sha256-pinned `sweep-k4` blobs, in the measured
   order (float summation is order-sensitive), through the same
   `classify_v6_mean`.
4. `K = 8` (seeds 5–8) — the plan's "only if `K = 4` wins" branch is now
   live; timing from 1. can veto it.

**Rail re-arbitration (2026-09-28).** One lumped arm, not five:
`scripts/ab-floor-rails.sh` adds `norail` (K = 4, all five rails off) to
`sweep-k4` on the same `SEED_BASE` and blobs, so `file` is the rails-on
control. The rails still earn, CI-clear by 7–10σ on every cell:

| rails on vs off, K = 4 | plain none / both | PD none / both | sd-PD none / both | fired |
| --- | --- | --- | --- | --- |
| this run | +0.0283 / +0.0344 | +0.0212 / +0.0252 | +0.0196 / +0.0242 | 0.54% / 0.58% |
| sum of the five seed-1 ship cells | +0.0285 / +0.0366 | +0.0398 / +0.0496 | | |

Plain is unchanged from seed 1's sum. PD is about half, the expected
overlap: the ensemble already drops some of the junk doubles the rails
were vetoing (Phase 1's doubled-contract table). The lumped arm cannot show
that each rail is positive on its own; the two smallest, the game-pull and
2NT-bid vetoes, could have gone inert. They were never large enough to
decide the flip, so they stay on.

No second-`SEED_BASE` confirmation: the plan's seed-2 confirmation is
for Phase 2 recipe axes, and every cell here clears its CI by ≥ 4×.

## Phase 2 — the recipe's free parameters, one axis at a time

Each axis: train at two seeds (both GPUs, 10 min), reject on failed-to-train
only, A/B the seed-1 candidate vs shipped, read against σ_seed, and confirm
with the seed-2 candidate before any flip. Order, with the reason:

1. **`--dd-weight`** (shipped 0; the rows carry 20 DD tricks, so the value
   head is live). It is the only signal in the trainer about *tricks* rather
   than the teacher's call: an auxiliary MSE head (`H → 20`, train-only,
   never exported) that shapes the shared trunk. It was zeroed for
   configured-net.md gate 1 so that a *feature* comparison would not also
   vary the objective, and every later recipe inherited the zero; no A/B
   ever priced it. Try 0.1 and 1.0. Highest ceiling, highest risk: a bad
   weight degrades the policy through the shared trunk — which is exactly
   what the A/B is for.
2. **`--wd`** (shipped 0; the only regulariser). Read the training log's
   train-loss vs `val_ce` gap at epoch 300 first: if held-out CE is still
   falling, the net is under-fit and wd is the wrong axis — go to 3.
   Otherwise 1e-4 and 1e-3.
3. **`--epochs`** 150 / 600, chosen by that same curve.
4. **`--hidden`** 512, and 128. Note the in-crate `forward` fixes `HID =
   256` as a `const`, so a width arm needs `total`/`forward` generic over
   the hidden width and a `hidden` field in the sidecar — a small,
   contained change in `neural.rs`. If 128 measures equal, ship 128: a KR3
   win with a KR1 non-inferiority proof (CLAUDE.md, checklist item 12).
5. **`--lr`** 3e-4 at 600 epochs. The trainer has no schedule; if this axis
   shows anything, add `--lr-schedule cosine` as a flag and retry once.

Budget: five axes × ~1.5 h plus confirmations ≈ 12–15 A/B-hours over idle
nights. Stop an axis after one confirmed direction; do not grid.

## Phase 3 — combine and ship

Best recipe × ensemble, one A/B, rail re-arbitration, `smoke-default`
re-bless, CHANGELOG. If Phase 2 found nothing, Phase 1's ensemble alone is
the deliverable.

## Wiring — the only code (built 2026-09-28)

- `neural::classify_v6_mean(blobs, features) -> Logits`: the elementwise
  logit mean of `forward::<IN_V6>` over `blobs`; `neural::decode` and
  `neural::V6_FLOATS` are public for loaders. A mean of copies of the
  shipped blob is the shipped net bit for bit
  (`v6_mean_of_copies_is_the_shipped_net`).
- `ConfiguredFloorV6::new_mean` — the same shell (rails, mask, gates) over
  a `Net::Mean` of run-time blobs; `american::american_mean(agreements,
  blobs)` builds the system.
- `examples/common/mod.rs`: the `american-file` arm reads
  `PONS_FLOOR_WEIGHTS=a.f32,b.f32,…` once per process. Example-side only;
  the library's default path never touches it, so `smoke-default` stays
  byte-identical. End to end, `american-file` on the shipped blob matched
  `american` on 400 boards byte for byte.
- `scripts/ab-floor-file.sh RESULTS_DIR`: `ab-v8-floor.sh` with the
  candidate arm `--our-floor american-file`; it logs each blob's
  `sha256sum` beside `SEED_BASE` and pins them in `$R/weights`, so a resume
  with other blobs fails loudly.
- Trainer: `--init-seed` and `--device-index` already exist. Width needs
  the sidecar `hidden` field read at load; a schedule needs a flag. Nothing
  else.

## Runbook

```sh
# 1. train — one draw per GPU; stems as the manifest lists them (features-v8.md §3)
cd trainer && cargo run --release --features cuda -- --arch mlp --cuda --device-index 0 \
  --data ../target/corpus-relabel-m32/<stem>... \
  --hidden 256 --epochs 300 --lr 0.001 --wd 0 --batch 4096 --val-frac 0.10 \
  --dd-weight 0 --init-seed 2 --weights-out /mnt/ssd-data/jdh8/pons-sweep/seed2
# 2. fold (features-v8.md §3) — shape-identical to the shipped artifact afterwards;
#    from the repo root, one --data per stem, expect "folded 30 columns"
python3 scripts/fold-constant-inputs.py /mnt/ssd-data/jdh8/pons-sweep/seed2 \
  --data target/corpus-relabel-m32/<stem>...
# 3. A/B, shipped control vs the file-loaded candidate
PONS_FLOOR_WEIGHTS=/mnt/ssd-data/jdh8/pons-sweep/seed2.f32 BOARDS=204800 setsid nohup \
  scripts/idle-run.sh scripts/ab-floor-file.sh ab-results/sweep-seed2 \
  >ab-results/sweep-seed2.log 2>&1 &
```

Write the arm's recipe, seed and blob hashes into the results directory's
`README` before launch; a sweep with twenty blobs and no provenance is noise.

## Decision table, per candidate

| plain DD | PD | verdict |
| --- | --- | --- |
| win | win | ship after rail re-arbitration and the seed-2 confirmation |
| win | erased | *suspect* (v8's row): trace the worst PD boards, do not ship |
| inside σ_seed | inside σ_seed | a reseed, not a recipe — discard |
| loss | loss | discard; record the axis direction so nobody retries it |

## Ledger

| date | arm | seeds | recipe delta | plain none / both | PD none / both | sd-PD | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-09-28 | `sweep-seed2` (SEED_BASE 1790532496) | 2 | none (reseed) | **+0.0215 ±0.0105** / +0.0085 ±0.0125 | −0.0019 ±0.0116 / −0.0037 ±0.0139 | +0.0100 ±0.0116 / +0.0040 ±0.0139 | *suspect* row; a reseed |
| 2026-09-28 | `sweep-seed3` (SEED_BASE 1790537586) | 3 | none (reseed) | **+0.0154 ±0.0106** / +0.0063 ±0.0127 | **−0.0195 ±0.0117** / **−0.0153 ±0.0140** | −0.0002 ±0.0117 / +0.0011 ±0.0140 | *suspect* row; a reseed |
| 2026-09-28 | `sweep-k4` (SEED_BASE 1790574650) | 1–4 | K = 4 logit mean (Phase 1) | **+0.0453 ±0.0084** / **+0.0562 ±0.0101** | **+0.0572 ±0.0092** / **+0.0774 ±0.0111** | **+0.0507 ±0.0093** / **+0.0672 ±0.0111** | **win / win** — ship after timing + rail re-arbitration |
| 2026-09-28 | `sweep-k4` `norail` (SEED_BASE 1790574650) | 1–4 | K = 4, five rails off; read as rails on − off | **+0.0283 ±0.0030** / **+0.0344 ±0.0035** | **+0.0212 ±0.0030** / **+0.0252 ±0.0035** | **+0.0196 ±0.0029** / **+0.0242 ±0.0035** | rails still earn — keep all five |

## Out of scope

- Feature bumps and relabels ([features-v8.md](features-v8.md)); the LSTM
  (`park/lstm-floor`, [plan.md](plan.md) M5.2); the evaluator net
  ([evaluator-net.md](evaluator-net.md)); consuming the fitted temperature
  or the collar ([logit-calibration.md](logit-calibration.md)).
- Corpus staleness: the labels are from 2026-09-13 and the book has moved.
  Control and candidate share it, so it cancels inside this campaign.

## Open questions for jdh8

1. Un-defer training-only work (the precondition)?
2. ~~Is `K` embedded blobs acceptable for KR3?~~ **Answered 2026-09-28:
   yes, disk is cheap; keep the computation small** (Phase 1's timing check).
3. Any objection to pricing `--dd-weight`? The recorded reason for 0 is a
   controlled-comparison choice (configured-net.md gate 1), not a verdict.
