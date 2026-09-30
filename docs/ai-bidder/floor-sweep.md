# The floor sweep — seed noise, ensembles, and the recipe's free parameters

**Status (2026-09-30): closed. Phase 3 shipped — the K = 8 logit-mean
ensemble of init seeds 1–8 is the default floor (confirmed on fresh deals:
plain wash, PD win both cells; rails re-arbitrated), replacing Phase 1's
K = 4 of seeds 1–4. Phase 2 axes 1–5 are closed: `--dd-weight` loses, `--wd` is
skipped by its gate, `--epochs 600` loses on plain and washes on PD, and
`--hidden 512` loses on PD and 128 loses on plain DD and sd-PD. Width stays
256. The Pass-offset follow-up and 150 epochs also lose on plain DD.
`--lr 3e-4` reads *suspect* (plain win none, PD loss both). The three
gate-rejected arms were measured anyway (`sweep-gated`) and none wins.
No Phase 2 recipe advances, so the recipe is unchanged and the ensemble
is the whole deliverable. The one lever left is relabel-grade and deferred
(Next).** Axis numbers refer to the
original Phase 2 plan: **Axis 4 is width; Axis 5 is learning rate**, regardless
of the order of remaining tasks below. Verdict narratives and divergence traces:
[../archive/floor-sweep-verdicts.md](../archive/floor-sweep-verdicts.md).
Precondition met: jdh8 kicked the sweep off 2026-09-28, un-deferring
*training-only* work (the 2026-09-26 deferral was of retrains that need a
dump or a relabel; this plan needs neither). Every step is a 10-minute
train on the SSD corpus plus a 90-minute A/B.

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
4. **Control is always the shipped `american()`** (M32 v6; seed 1 until the
   2026-09-28 flip, the K = 4 mean of seeds 1–4 until 2026-09-30, the K = 8
   mean of seeds 1–8 since). Fresh
   `SEED_BASE` per experiment, shared by its arms; arms sequential;
   `scripts/idle-run.sh`.
5. **A win smaller than the seed noise is a reseed, not a recipe.** Phase 0
   measures σ_seed first; every later verdict is read against it.
6. **Shipping = decision table + rail re-arbitration.** The five shipped
   floor rails were arbitrated against the seed-1 v6 net. A new net re-runs
   each rail's runner (floor-rail-campaign.md stop criterion 5) before the
   flip. Then `smoke-default` re-blesses and CHANGELOG records the numbers.

## Shipped state (2026-09-30)

- **K = 8 logit mean** of init seeds 1–8, embedded in that order (the
  `sweep-k8-confirm` blobs, sha256-pinned in its `weights.k8`). Over K = 4,
  pooled across `sweep-k8` and `sweep-k8-confirm` (409,600 bd/vul): plain
  +0.0008 ±0.0039 / +0.0044 ±0.0045 (wash), PD **+0.0076 ±0.0042 /
  +0.0082 ±0.0049** (win both). The embedded default reproduces the
  measured `american-file` arm board for board. Rails re-arbitrated under
  K = 8 (rails on − off: plain +0.025 / +0.030, PD +0.016 / +0.019); all
  five kept.
- **KR3 (K = 8):** 587 µs/board bidding-only (`floor-timing`, 20,000
  boards, three reps) vs 343 for K = 4: 1.71× K = 4, ~3.7× the single net.
  In the harness generation grows 110 → 128 s (+15%), about 2% of an
  end-to-end A/B. The crate is 8.1 MiB of crates.io's 10 MB.
- *Phase 1, superseded:* **K = 4 logit mean** of init seeds 1–4, embedded; `classify_bba_v6` is the
  mean, so every `with_floor_v6` path flipped at once. Plain +0.045 / +0.056,
  PD +0.057 / +0.077, sd-PD +0.051 / +0.067 (none / both), CI-clear by
  4–7σ. Single reseeds each land on the *suspect* row (plain up, PD down,
  more junk doubles); their mean does not — the variance reduction itself
  is worth IMPs.
- **KR3:** +65 µs/board per member (the net runs about twice per call, on
  every call — unexplained, the cheapest KR3 lever if it matters). K = 4 is
  2.2× the bidding latency and ~1% of an end-to-end A/B.
- **Rails:** all five still earn under the ensemble (rails on − off: plain
  +0.028 / +0.034, PD +0.021 / +0.025, CI-clear); kept.
- **σ_ens ≲ 0.007** (seeds 5–8 vs 1–4), below what 204,800 boards resolve,
  so a Phase 2 arm must read ≳ 0.01 CI-clear on both scorers. **K = 8** sits
  on the 1/K prediction (PD +0.005 / +0.008) at 1.8× latency; shipped in Phase 3 (above).

## Closed axes

Each arm was the K = 4 mean of seeds 1–4 at the candidate recipe against the
then-shipped K = 4; detail in the archive.

- **`--dd-weight` 0.1 / 1.0 — loss / loss**, growing with the weight. The
  auxiliary trick head regularises (the train − val gap closes) but the
  trunk spends its capacity on tricks and the calls get worse. Closed at 0;
  do not retry above it.
- **`--wd` — skipped by its gate.** Held-out CE was still falling at 300
  epochs with a 0.025 gap: under-fit, wrong axis.
- **`--epochs 600` — plain loss** (−0.010 none, sd plain CI-clear on both
  cells), **PD wash**; discarded. The trace (archive) says why: the sharper
  net is *more conservative* — it passes where the shipped net bid
  (−8,691 plain / +7,367 PD over 11,386 boards) and misses games; no single
  node loses more than 98 IMPs; doubled finals fall 5.60 → 5.37%. Nothing
  reads the floor's logit scale (rails are `-∞` masks, the table argmaxes,
  the sampler margin fires only at authored nodes), so the shell is not the
  cause. The labels are BBA's calls: fitting them better bids more like BBA,
  and plain DD rewards the shipped net's accidental aggression while PD
  prices it at zero. Held-out CE improved on both losing axes — rule 2,
  twice more.
- **Axis 4, `--hidden` 512 / 128 — keep 256.** The 512-wide K = 4 passes
  the training gate but measures *suspect*: plain wash / win, PD loss at
  both vulnerabilities (−0.0247 / −0.0114 IMPs/board, CI-clear). The full
  39,316-record trace finds more action where the shipped net passes:
  +7,520 plain / −14,229 PD IMPs, outweighing the reverse direction. No
  single prefix explains the loss; Rust fixture parity passes. All four
  128-wide draws fail the CE gate (0.4758–0.4849 vs 0.426319); measured
  anyway in `sweep-gated`, 128 is the mirror image: **plain loss**
  (−0.0242 / −0.0199), PD win / wash, **sd-PD loss** — the decision
  table's PD-artifact row. The two widths bracket 256 from opposite sides.
  Details and reproduction in the archive's
  [Axis 4 verdict](../archive/floor-sweep-verdicts.md#phase-2-axis-4-verdict--hidden-width-2026-09-30).
- **Epochs follow-ups — neither advances.** In `sweep-ep150`, the measured
  arm is **600 epochs with Pass bias −0.0205**, not 150 epochs. Plain DD
  loses at both vulnerabilities (−0.0092 / −0.0136 IMPs/board); PD is
  +0.0005 / −0.0092, and sd-PD loses with both sides vulnerable. The offset
  matched the raw Pass rate on contested **corpus rows**, not live floor
  nodes, so this rejects the measured calibration without settling every
  possible Pass tilt. The 150-epoch seeds have CE 0.422819 / 0.424135 / 0.427407 /
  0.430960: seeds 3 and 4 exceed 0.42631943. Measured anyway in
  `sweep-gated`: **plain loss** (−0.0093 / −0.0084, the both-vul CI touching
  zero), PD wash, sd plain CI-clear loss both cells. 150 and 600 both lose
  plain to 300. Keep 300 epochs.
  [Verdict and bounded trace](../archive/floor-sweep-verdicts.md#epochs-follow-ups--pass-offset-and-150-epochs-2026-09-30).
- **Axis 5, `--lr 3e-4` at 300 epochs — *suspect*.** All four
  draws completed and folded 30 columns. CE is 0.426501 / 0.424733 /
  0.423722 / 0.425542; only seed 1 exceeds 0.42631943, by 0.000181844, so
  the gate withheld it. `sweep-gated` measured it: **plain win** none
  (+0.0088), wash both; **PD loss** both (−0.0162); sd-PD wash. That is
  v8's row, the same one every single reseed and `--hidden 512` landed on,
  and the plain gain is under the 0.01 bar. Keep `--lr 0.001`; no IMP
  signal triggers the conditional cosine-schedule trial. [Recipe and provenance](../archive/floor-sweep-verdicts.md#phase-2-axis-5--learning-rate-training-gate-2026-09-30).

## Next

For any reopened axis: train **four** seeds at the candidate recipe (two per
GPU, ~20 min), A/B their K = 4 mean vs shipped, and read against σ_ens.
Stop an axis after one confirmed direction; do not grid.

**The CE gate filters divergence only.** Reject a draw that failed to train:
NaN, a stalled loss, or CE far past shipped val_ce + 0.010 (h128 missed by
0.05–0.06). A draw within a few thousandths of the line goes to A/B: a
0.0002 miss is draw spread (the `sweep-k8` relaunch precedent). `sweep-gated`
measured all three rejected arms and all three lost on IMPs, so the gate cost
nothing this time. The line is a warning, not a verdict.

- **Phase 3 — done 2026-09-30.** K = 8 confirmed on fresh deals and
   shipped. K = 16 is not worth running: the 1/K prediction puts its
   increment at about half of K = 8's (≈ +0.004 PD), below what 409,600
   boards resolve, at another ~1.7× latency.
- **Root cause, deferred (relabel-grade).** The rollout-override rows are
   the only IMP-grounded labels. Test whether the states where the sharper
   net reverted to pass neighbour override rows; if so, upweight them. The
   corpus carries no per-row override flag, so this needs a dump and sits in
   [../next-steps.md](../next-steps.md)'s deferred list.

## Phase 3 — combine and ship (shipped 2026-09-30)

No recipe advanced, so Phase 3 was the ensemble alone at K = 8: the
confirmation A/B, rail re-arbitration, the embed, CHANGELOG. The plan as
written:

Best recipe × ensemble (K = 8 if the 1/K increment still pays at that
recipe), one A/B, rail re-arbitration, `smoke-default` re-bless, CHANGELOG.
If Phase 2 found nothing, Phase 1's ensemble alone is the deliverable.

## Wiring — the only code (built 2026-09-28)

- `neural::classify_v6_mean(blobs, features) -> Logits`: the elementwise
  logit mean of the v6 forward pass over `blobs`, each at the hidden width
  its length names (`neural::v6_width`: 128, 256 or 512, built 2026-09-29
  for the `--hidden` axis); `neural::decode` is public for loaders. A mean of copies of the
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
- Trainer: `--init-seed`, `--device-index` and `--hidden` already exist; a
  schedule needs a flag. Nothing else.

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
| loss | inside σ_ens | discard (axis 3's row): a PD wash does not rescue a plain loss |
| loss | loss | discard; record the axis direction so nobody retries it |

## Ledger

| date | arm | seeds | recipe delta | plain none / both | PD none / both | sd-PD | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-09-28 | `sweep-seed2` (SEED_BASE 1790532496) | 2 | none (reseed) | **+0.0215 ±0.0105** / +0.0085 ±0.0125 | −0.0019 ±0.0116 / −0.0037 ±0.0139 | +0.0100 ±0.0116 / +0.0040 ±0.0139 | *suspect* row; a reseed |
| 2026-09-28 | `sweep-seed3` (SEED_BASE 1790537586) | 3 | none (reseed) | **+0.0154 ±0.0106** / +0.0063 ±0.0127 | **−0.0195 ±0.0117** / **−0.0153 ±0.0140** | −0.0002 ±0.0117 / +0.0011 ±0.0140 | *suspect* row; a reseed |
| 2026-09-28 | `sweep-k4` (SEED_BASE 1790574650) | 1–4 | K = 4 logit mean (Phase 1) | **+0.0453 ±0.0084** / **+0.0562 ±0.0101** | **+0.0572 ±0.0092** / **+0.0774 ±0.0111** | **+0.0507 ±0.0093** / **+0.0672 ±0.0111** | **win / win** — ship after timing + rail re-arbitration |
| 2026-09-28 | `sweep-k4` `norail` (SEED_BASE 1790574650) | 1–4 | K = 4, five rails off; read as rails on − off | **+0.0283 ±0.0030** / **+0.0344 ±0.0035** | **+0.0212 ±0.0030** / **+0.0252 ±0.0035** | **+0.0196 ±0.0029** / **+0.0242 ±0.0035** | rails still earn — keep all five |
| 2026-09-29 | `sweep-k8` `k4b` (SEED_BASE 1790624432) | 5–8 | none (K = 4 of fresh seeds) | −0.0072 ±0.0074 / −0.0023 ±0.0088 | −0.0086 ±0.0081 / −0.0067 ±0.0097 | −0.0015 ±0.0081 / −0.0002 ±0.0097 | σ_ens ≲ 0.007 (unresolved) |
| 2026-09-29 | `sweep-k8` `k8` (SEED_BASE 1790624432) | 1–8 | K = 8 logit mean | −0.0002 ±0.0055 / +0.0048 ±0.0064 | +0.0048 ±0.0060 / **+0.0076 ±0.0070** | +0.0043 ±0.0060 / **+0.0072 ±0.0071** | on the 1/K prediction; waits for Phase 3 |
| 2026-09-30 | `sweep-k8-confirm` `k8` (SEED_BASE 1790749966) | 1–8 | K = 8 logit mean, same blobs, fresh deals (Phase 3) | +0.0018 ±0.0055 / +0.0039 ±0.0064 | **+0.0104 ±0.0059** / **+0.0087 ±0.0070** | **+0.0067 ±0.0060** / +0.0049 ±0.0071 | confirmed: plain wash, PD win both cells; pooled with `sweep-k8` PD +0.0076 ±0.0042 / +0.0082 ±0.0049, plain +0.0008 ±0.0039 / +0.0044 ±0.0045 — ships after rail re-arbitration |
| 2026-09-30 | `sweep-k8-confirm` `norail` (SEED_BASE 1790749966) | 1–8 | K = 8, five rails off; read as rails on − off | **+0.0245 ±0.0029** / **+0.0295 ±0.0033** | **+0.0159 ±0.0027** / **+0.0188 ±0.0032** | **+0.0145 ±0.0027** / **+0.0183 ±0.0032** | rails still earn under K = 8 — keep all five |
| 2026-09-29 | `sweep-dd` `dd0.1` (SEED_BASE 1790629068) | 1–4 | `--dd-weight 0.1`, K = 4 | −0.0048 ±0.0074 / −0.0018 ±0.0087 | −0.0020 ±0.0081 / −0.0048 ±0.0097 | **−0.0094 ±0.0081** / −0.0091 ±0.0097 | loss-leaning; discard |
| 2026-09-29 | `sweep-dd` `dd1.0` (SEED_BASE 1790629068) | 1–4 | `--dd-weight 1.0`, K = 4 | **−0.0133 ±0.0074** / **−0.0119 ±0.0088** | −0.0038 ±0.0081 / −0.0091 ±0.0097 | **−0.0132 ±0.0081** / **−0.0144 ±0.0098** | loss / loss — axis closed at 0 |
| 2026-09-29 | `sweep-ep` `ep600` (SEED_BASE 1790668875) | 1–4 | `--epochs 600`, K = 4 | **−0.0104 ±0.0070** / −0.0081 ±0.0084 | +0.0041 ±0.0077 / +0.0018 ±0.0092 | −0.0035 ±0.0078 / −0.0032 ±0.0092 | plain loss (sd plain CI-clear both), PD wash — discard; do not train longer |
| 2026-09-29 | `sweep-hid` `h512` (SEED_BASE 1790681184; traced 2026-09-30) | 1–4 | `--hidden 512`, K = 4 | +0.0017 ±0.0086 / **+0.0110 ±0.0102** | **−0.0247 ±0.0093** / **−0.0114 ±0.0112** | −0.0080 ±0.0094 / −0.0042 ±0.0113 | *suspect* — broader aggression, keep 256 |
| 2026-09-29 | `h128` training gate | 1–4 | `--hidden 128`, K = 4 proposed | not run | not run | not run | all four CE values exceed 0.426319; no IMP verdict |
| 2026-09-29 (reconciled 2026-09-30) | `sweep-ep150` `ep600p` (SEED_BASE 1790678007) | 1–4 | 600 epochs, Pass bias −0.0205, K = 4 | **−0.0092 ±0.0071** / **−0.0136 ±0.0084** | +0.0005 ±0.0077 / −0.0092 ±0.0092 | −0.0057 ±0.0078 / **−0.0122 ±0.0092** | plain loss both cells; no recovery from this corpus-calibrated tilt |
| 2026-09-29 (reconciled 2026-09-30) | `ep150` training gate | 1–4 | `--epochs 150`, K = 4 proposed | not run | not run | not run | seeds 3/4 exceed 0.42631943; no IMP verdict |
| 2026-09-30 | `lr3e4` training gate | 1–4 | `--lr 3e-4`, 300 epochs, K = 4 proposed | not run | not run | not run | seed 1 exceeds 0.42631943 by 0.000181844; other seeds pass; superseded by `sweep-gated` |
| 2026-09-30 | `sweep-gated` `lr3e4` (SEED_BASE 1790714718) | 1–4 | `--lr 3e-4`, 300 epochs, K = 4 | **+0.0088 ±0.0076** / −0.0004 ±0.0090 | −0.0050 ±0.0084 / **−0.0162 ±0.0100** | −0.0009 ±0.0084 / −0.0078 ±0.0100 | *suspect* — keep `--lr 0.001` |
| 2026-09-30 | `sweep-gated` `ep150` (SEED_BASE 1790714718) | 1–4 | `--epochs 150`, K = 4 | **−0.0093 ±0.0071** / −0.0084 ±0.0084 | −0.0008 ±0.0077 / −0.0013 ±0.0092 | −0.0040 ±0.0078 / −0.0084 ±0.0093 | plain loss, PD wash — discard; keep 300 epochs |
| 2026-09-30 | `sweep-gated` `h128` (SEED_BASE 1790714718) | 1–4 | `--hidden 128`, K = 4 | **−0.0242 ±0.0081** / **−0.0199 ±0.0097** | **+0.0096 ±0.0089** / +0.0036 ±0.0107 | **−0.0116 ±0.0090** / **−0.0159 ±0.0108** | plain loss, PD win (artifact row), sd-PD loss — keep 256 |

## Out of scope

- Feature bumps and relabels ([features-v8.md](features-v8.md)); the LSTM
  (`park/lstm-floor`, [plan.md](plan.md) M5.2); the evaluator net
  ([evaluator-net.md](evaluator-net.md)); consuming the fitted temperature
  or the collar ([logit-calibration.md](logit-calibration.md)).
- Corpus staleness: the labels are from 2026-09-13 and the book has moved.
  Control and candidate share it, so it cancels inside this campaign.

## Open questions for jdh8

1. ~~Un-defer training-only work (the precondition)?~~ **Answered 2026-09-28:**
   training-only sweep authorized; dump/relabel work remains deferred.
2. ~~Is `K` embedded blobs acceptable for KR3?~~ **Answered 2026-09-28:
   yes, disk is cheap; keep the computation small** (Phase 1's timing check).
3. ~~Any objection to pricing `--dd-weight`?~~ Taken as no: jdh8 said
   go on Phase 2 (2026-09-28), whose first axis is `--dd-weight`. The recorded reason for 0 is a
   controlled-comparison choice (configured-net.md gate 1), not a verdict.
