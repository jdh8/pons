# Reseed gap — why the shipped K = 8 beats every retrain of its recipe

**Handoff, 2026-10-25.** This is the next step of the shrinkage plan
([shrinkage-plan.md](shrinkage-plan.md)), and it blocks every retrain.
Nothing is in flight. The relabel pilot runs beside it as a CPU track (§5).

## 1. The problem

`ab-results/v9-floor` (204,800 boards per arm per vul, four cells) showed
that **k8new**, Plan B's "reseeded" v6 K = 8, loses about −0.012 IMPs/board
plain DD to the shipped K = 8 at every cell, with PD flat
([shrinkage-plan.md](shrinkage-plan.md) "Gate read, change 1"). Any
retrained floor is non-inferior to the default only if it beats the
*shipped* floor. So every retrain starts about 0.012 behind: v9, A4's nets,
a relabelled floor, and the Gaussian v9 variant alike. Until this gap is
explained, no retrain can ship whatever it gains.

The seed spread does not explain it on its own. The floor sweep put
σ_ens ≲ 0.007 for K = 4 against K = 4 (seeds 5–8 vs 1–4,
[archive/floor-sweep.md](archive/floor-sweep.md)). So K = 8 against K = 8
should differ by about 0.005 or less. All four cells reading −0.012 is not
four confirmations: the cells share the same 16 nets, so the gap is one
draw read four times.

## 2. The discrepancy to flag: a "reseed" changed four things

Plan B calls k8new "ten fresh draws of the shipped recipe" and reads "fresh
seed2 vs shipped seed2" as proof that "the val-cut change **is a reseed**, no
more". The manifests disagree with that. Compare
`src/bidding/weights/american_bba_v6_seed{2..8}.json` with
`ab-results/ruler/nets/seed*.json`:

| | shipped seeds 1–8 | k8new (seeds 9–16), v9, v9-probed, A4 lap 1 |
| --- | --- | --- |
| init seed | 1–8 | 9–16 (fresh seed 2 and 3 for Plan B's K = 1 read) |
| `--data` stem order | the recipe order: `uniform-0..7, enriched-0..3, axis-0004, axis-1000, …` | **alphabetical** (`ls`): `axis-0002, axis-0004, axis-0020, …` |
| validation cut | nominal row (`val_split` absent; 676,686 rows) | board start (`val_split: "board"`; 676,500) |
| trainer | `39318c7f` (seed 1), `67730fb2`, `013dee9b`, `32fc876c` | `fabcb6fe` |
| `data_seed` (manifest) | 100 | 303 |

**Stem order is not cosmetic.** `trainer/src/data.rs` `load_mixture`
round-robins the training heads in block chunks *in stem order*, and "the
epoch loop walks contiguous minibatches and never shuffles". The stem order
is therefore the whole minibatch sequence, and it is the same in every
epoch. All eight members of an ensemble share it. That makes data order an
effect **K-averaging cannot shrink**, and the sweep's σ_ens, measured at one
fixed order, never saw it. It is the leading hypothesis for the gap.

`data_seed` is the first stem's corpus seed (`ds.meta.seed`), so it only
records the order and adds nothing. The validation cut moves 186 rows of
6.77M. `scripts/feature-retrain.sh` builds its `--data` list with `ls`, so
every retrain since Plan B inherited the alphabetical order.

Proposed reversible default until §3 is read: treat the stem order as part
of the recipe. Any new training run passes the shipped order and
`--row-split`, so it matches the shipped recipe except for the factor under
test. That is one line in `feature-retrain.sh` and the dl02 command.

## 3. The experiment: a 2 × 2 over order and seeds

Use the HEAD trainer on dl02 (CUDA 12.8 and the device indices are in
CLAUDE.local.md) and the Plan B recipe (`--arch mlp --hidden 256 --epochs
300 --lr 0.001 --wd 0 --batch 4096 --val-frac 0.10 --dd-weight 0`). Train
on the M32 stems `target/corpus-relabel-m32/*` and fold with
`scripts/fold-constant-inputs.py`, as Plan B did. "Shipped order" is the
`data_stems` list copied from a shipped manifest.

| arm | seeds | order | cut | role |
| --- | --- | --- | --- | --- |
| **R** | 1–8 | shipped | `--row-split` | Reproduction: should recover the shipped blobs |
| **S** | 9–16 | shipped | `--row-split` | The seed effect at the shipped order |
| **O** | 1–8 | alphabetical | `--row-split` | The order effect at the shipped seeds |
| k8new | 9–16 | alphabetical | board | Already trained (`ab-results/ruler/nets/seed9..16`) |

**Step 1, R alone (about 30 min of GPU time, no A/B).** `cmp` each R blob
with `src/bidding/weights/american_bba_v6_seed{N}.f32`. Seed 1's blob is
`american_bba_v6.f32`, trained at `39318c7f`, an older trainer than seeds
2–8. If the blobs are bit-identical, how the shipped nets were produced is fully
understood, and R is the shipped floor. If they are not, read
`scripts/ruler-tv.py` on paired deals for R against shipped. TV near 0 means
GPU nondeterminism only. TV at the K = 8 floor (0.017) means trainer code
drift since `32fc876c` is a third factor, to be bisected before going on.

**Step 1 read (2026-10-25): R is the shipped floor.** All eight R blobs,
trained at `fabcb6fe` (plus the v9 `data.rs` change, which v6 never takes)
with each seed on its shipped manifest's GPU and folded here, are
bit-identical to `american_bba_v6{,_seed2..8}.f32`, seed 1's older
`39318c7f` included. The trainer and the GPUs are deterministic, there is
no code drift, and the shipped nets are fully reproducible from their
manifests. k8new therefore differs from shipped in exactly three factors:
seeds, stem order and the validation cut. Step 2 needs no R arm. Nets in
`ab-results/reseed-gap/nets/R`.

**Step 2, S and O (about 1 h of GPU time) and one A/B.** Run
`scripts/ab-floor-file.sh` at four cells with `BOARDS=204800`, the caps in
CLAUDE.local.md and a fresh `SEED_BASE`. The arms are
`S=… O=… k8new=…`, plus `R=…` if step 1 was not bit-identical. Read
the paired IMP intervals, with TV as a diagnostic:

| reading | means | next |
| --- | --- | --- |
| S ≈ shipped, O ≈ k8new < shipped | **Order is the gap.** The shipped order is a good draw, or alphabetical a bad one | §4 |
| O ≈ shipped, S ≈ k8new < shipped | **Seeds 1–8 are a lucky draw.** A winner's curse, nothing to recover | Retrains stay read against a reseed. Shipping one needs it to clear about 0.012. Record and move on |
| R ≠ shipped | **Production or trainer drift** | Bisect the trainer between `32fc876c` and `fabcb6fe`, then redo step 2 |
| Neither factor alone | Interaction, or both effects below resolution | Size the next read on the largest interval before spending more |

**Step 2b, the per-board forensic (minutes, no GPU).** The floor serves a
pure policy, one call per hand (§6), so where two ensembles disagree at a
decision at most one of them is right. On step 2's paired dump, list the
boards where shipped and k8new (then S and O) pick different calls at the
same prefix, score both calls by double dummy under a common continuation,
and count who was right, by lane. Member quality cannot do this: held-out
top-1 is flat to the third digit between the two sets (§6), so the gap lives
in which decisions the two argmaxes pick, or in how the eight members'
errors correlate, and a shared minibatch sequence is exactly what
correlates them.

Cost: about 1.5 h on dl02's two GPUs for 24 nets. The A/B is three or four
arms at four cells. The v9-floor run with three arms had the whole box, so
expect it to take longer at `JOBS=96`.

## 4. If order is the gap: a possible KR1 lever

A shared order is a common random effect. The fix is not to pick the "good"
order, which is a winner's curse of its own. Give each ensemble member its
own order, either a per-epoch shuffle seeded from the init seed or a stem
rotation per member, so K = 8 averages the order noise as well. That changes
the trainer (it never shuffles today, by design: see `load_mixture`'s doc on
the mixture's tail) and ships only after an A/B against shipped on both
scorers. If it wins, it is a floor win on its own and the new base for every
later retrain: v9 (Gaussian or histogram), the relabel, and A4.

## 5. The parallel CPU track: the relabel pilot, two arms

Revised from [shrinkage-plan.md](shrinkage-plan.md) "Next" change 2. A pilot
under the probed reading alone conflates how far the book has drifted since
the M32 cut (2026-09-13) with what the probe does to labels. After two flat
reads of the probe, the drift number is the one every retrain needs. So
relabel one recipe shard (axis-0004, the shard lap 1 audited) twice at HEAD
with `scripts/relabel-worker.sh` (`RECIPE_FILE` a one-line recipe, `LAYOUTS`
as M32, run as 4 × `DDSS_THREADS=32` per CLAUDE.local.md):

- **⊤ arm**: the default `DUMP_COMMON`. Its label diff against the M32 cut
  measures how stale the transplanted labels are, for every retrain,
  including the Gaussian v9 variant.
- **Probed arm**: `DUMP_COMMON` plus `--ns-probe 100000 --ns-probe-vacuous`.
  Its diff against the ⊤ arm is the probe's last test.

Measure the share of EV-argmax labels that move. A share near zero closes
the relabel. Otherwise the full relabel follows, and it should be trained
under whatever order and recipe §3 settles. Each arm takes about 10 h on
the shared box and about 2.5 h on a free one. Per the global rule, launch
detached, write the state to memory, and end the turn.

## 6. What stays decided

- Retrains read K = 8 against K = 8 on paired deals, at four cells. The
  paired IMP intervals decide and TV is a diagnostic
  ([shrinkage-plan.md](shrinkage-plan.md) "Review corrections").
- The probe stays off the default path (A3, A4 lap 1, A4 inside v9).
- v9 as built (the histogram) does not ship. The Gaussian variant waits on
  §3's recipe and §5's ⊤ number.
- **One hand maps to one call, and a call's definition is deterministic**
  (jdh8, 2026-10-25). The floor serves the argmax of the mean logits
  (`neural::classify_mean`) and trains on one-hot labels, and the harness
  never pays for mixing: our pair against a fixed opponent is a
  common-payoff game, which always has an optimal deterministic joint
  policy, and mixing's one payoff, concealment, is invisible to double
  dummy. A call means its authored definition, not the frequency with which
  the policy uses parts of it; at a floor node, where nothing is authored,
  the de facto definition is the teacher's preimage and partner's model of
  it is the same teacher. Three readings follow. The ruler for a retrain is
  the **per-decision disagreement rate** on paired deals, split where a
  definition is authored into inside it (judgment) and outside it (a
  misbid); TV stays a diagnostic, since a next-call frequency at a prefix is
  a marginal and two partitions can share it. Label staleness is
  **definition drift**, so §5's ⊤ number reads against the book changes
  since the M32 cut, not against the policy. And the probe's full-fold scope
  (`probed`) reads a judgment frequency as a boundary; only ⊤ axes take a
  box (`probed_vacuous`), see
  [shrinkage-plan.md](shrinkage-plan.md) "What the analogy misses".
- Member quality does not explain the gap. Held-out top-1 against the
  teacher labels, from the manifests (the two cuts differ by 186 rows, so
  this compares to the third digit):

  | ensemble | overall, per member | contested, per member |
  | --- | --- | --- |
  | shipped seeds 1–8 | 0.8517–0.8534, mean 0.8527 | 0.8294–0.8312 |
  | k8new seeds 9–16 | 0.8522–0.8533, mean 0.8528 | 0.8299–0.8311 |

  Ensemble-level agreement is not recorded; step 2b reads the decisions
  themselves instead.
