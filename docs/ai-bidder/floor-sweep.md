# The floor sweep — seed noise, ensembles, and the recipe's free parameters

**Status: plan, written 2026-09-28. Nothing has run.** Precondition: jdh8
un-defers *training-only* work. The 2026-09-26 deferral was of retrains that
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
  `target/corpus-relabel-m32` (6.0 GB, 60 stems, on the SSD, rows carry
  `[176 features][38 teacher_softmax][20 dd_tricks]`). Train: 9 min 42 s on
  the 4090 (features-v8.md §3); two GPUs run two draws at once. A/B: 87 min
  at 204,800 bd/arm/vul (`ab-results/v8-floor.log`, 21:32→22:58).

## Rules

1. **The corpus is frozen.** M32 labels, v6 features, the 60 stems as the
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

## Wiring — the only code

- `neural::classify_v6_with(weights: &[f32], features: &[f32]) -> Logits`:
  a public wrapper over the private `forward::<IN_V6>`. One line.
- A floor variant in `neural_floor.rs` holding `Vec<Vec<f32>>` (decoded
  blobs) whose classify averages logits over the blobs (`K = 1` is a plain
  file-loaded net). Reuse `decode`.
- `examples/common/mod.rs`: an `american-file` arm reading
  `PONS_FLOOR_WEIGHTS=a.f32,b.f32,…`. Example-side only; the library's
  default path never touches it, so `smoke-default` stays byte-identical.
- `scripts/ab-floor-file.sh RESULTS_DIR`: `ab-v8-floor.sh` with the
  candidate arm `--our-floor american-file`; it logs `PONS_FLOOR_WEIGHTS`
  and each blob's `sha256sum` beside `SEED_BASE`, or the result is
  unattributable.
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
# 2. fold (features-v8.md §3) — shape-identical to the shipped artifact afterwards
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

## Out of scope

- Feature bumps and relabels ([features-v8.md](features-v8.md)); the LSTM
  (`park/lstm-floor`, [plan.md](plan.md) M5.2); the evaluator net
  ([evaluator-net.md](evaluator-net.md)); consuming the fitted temperature
  or the collar ([logit-calibration.md](logit-calibration.md)).
- Corpus staleness: the labels are from 2026-09-13 and the book has moved.
  Control and candidate share it, so it cancels inside this campaign.

## Open questions for jdh8

1. Un-defer training-only work (the precondition)?
2. Is `K` embedded blobs (≈ 2 MB at `K = 4`) acceptable for KR3, or must a
   winning ensemble be distilled back into one net before it ships?
3. Any objection to pricing `--dd-weight`? The recorded reason for 0 is a
   controlled-comparison choice (configured-net.md gate 1), not a verdict.
