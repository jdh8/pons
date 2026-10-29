# Reseed gap — why the shipped K = 8 beats every retrain of its recipe

**Handoff, 2026-10-25.** This is the next step of the shrinkage plan
([shrinkage-plan.md](shrinkage-plan.md)), and it blocks every retrain.
§3 is read (below). The relabel pilot (§5) is in flight from 2026-10-25.
**Read 2026-10-25 (§3):** seeds about −0.007, order about −0.003, and they
add up to k8new's −0.011. No lane carries the gap. The axis is pass against
act. Recorded as mostly lucky seeds (jdh8). **Read 2026-10-27 (§5):** the
probe is flat, staleness is 2–3% of decisions beyond the cut's own redraw
noise, and that noise (≈ 11–13% of decisions at M = 32) is the finding.

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

**Step 2 read (2026-10-25): neither factor alone; both cost, and they add.**
`ab-results/reseed-gap/ab`, `SEED_BASE` 1792877109255232112, 205,056
boards per arm per cell, each arm paired against the shipped floor (`plain`
is `--our-floor american`). Per cell, plain DD:

| arm | differs from shipped in | none | ns | ew | both | pooled |
| --- | --- | --- | --- | --- | --- | --- |
| O | order | −0.0030 ±0.0065 | −0.0007 ±0.0070 | −0.0040 ±0.0077 | −0.0057 ±0.0078 | **−0.0034 ±0.0036** |
| S | seeds | −0.0059 ±0.0065 | −0.0049 ±0.0071 | −0.0072 ±0.0077 | −0.0093 ±0.0078 | **−0.0068 ±0.0036** |
| k8new | seeds, order, cut | −0.0103 ±0.0065 | −0.0064 ±0.0071 | −0.0140 ±0.0079 | −0.0133 ±0.0078 | **−0.0110 ±0.0037** |

PD pooled: O +0.0034, S −0.0012, k8new +0.0026 (each ±0.0040).
Single-dummy plain (16 worlds) pooled: O −0.0043, S −0.0060, k8new −0.0154.
"Pooled" is the mean of the four cells, with the cells' intervals combined
in quadrature. Every arm fails non-inferiority to shipped in every cell.

- **k8new reproduces the gap** (−0.011 against v9-floor's −0.012), so the
  target is real and stable across seed bases.
- **Seeds are the larger part**, about −0.007, outside its interval. Seeds
  1–8 are a good draw: a winner's curse on about 60% of the gap.
- **Order is about −0.003**, and its interval reaches 0. It is unresolved,
  and §4's lever is worth at most that much.
- **The two add up**: −0.0068 + −0.0034 = −0.0102 against k8new's −0.0110,
  which leaves nothing for the validation cut. The cut is not separable from
  order anyway. `mix` round-robins the stems by progress and never shuffles,
  so moving each stem's cut by a few rows shifts every block and minibatch
  boundary. That makes the cut a small, fresh order draw, not a factor of its
  own.
- **The four ensembles sit about equally far apart.** Per-decision
  disagreement on paired deals, as the share of our calls at a shared prefix
  where the two arms part, is 1.78–1.85% at none and 1.59–1.63% at both for
  all six pairs. Reach-weighted TV (`ruler-tv.py`, default depth) is
  0.036–0.038 at none and 0.031–0.033 at both. Shipped–k8new is the widest
  pair by only 2–3%. S and O are no closer to shipped than to k8new. The gap
  is in *which* side wins the 1-in-55 disagreements, not in how often they
  happen.

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

**Step 2b read (2026-10-25): no lane, but a pass/act tilt.** `ab-dump-diff
--jsonl` writes each contract-divergent board with both scorers' swings. It
reproduces all twelve step 2 headlines exactly.
`scripts/floor-divergence.py` charges each board to its first divergent call,
the direct A/B's own attribution rather than a common-continuation
counterfactual. Outputs are in `ab-results/reseed-gap/forensic/`. Every first
divergence is our call, as it must be. 37% of the boards diverge in
competitive auctions (we opened) and 63% in defensive ones (they opened).
Nothing is constructive.

- **No lane carries the gap.** k8new's worst lane, `1♦ - 1♥` in first seat,
  is −0.0005 IMPs/board, and the worst twelve sum to about −0.004. The loss
  is spread over hundreds of prefixes, as a draw-level effect should be.
- **The axis is pass against act.** Pooled over the four cells, plain then
  PD, in IMPs/board:

  | arm | ON passes, shipped acts (n) | plain | PD | ON acts, shipped passes (n) | plain | PD | passive share |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | O | 19,105 | −0.0181 | +0.0112 | 18,130 | +0.0141 | −0.0095 | 0.513 |
  | S | 18,987 | −0.0180 | +0.0114 | 18,521 | +0.0117 | −0.0139 | 0.506 |
  | k8new | 21,283 | −0.0225 | +0.0115 | 16,875 | +0.0130 | −0.0085 | **0.558** |

  Bidding lower or higher than shipped shows the same sign pattern, at
  about −0.003 and +0.0035 for every arm. On these boards plain DD pays the
  side that acts and PD pays the side that passes, which is the familiar
  doubling bracket. So per board, "who was right" is mostly "who was more
  aggressive", read through the scorer.
- **Shipped picks better spots to act.** S and O are balanced on pass
  against act (share 0.51) and still lose on that axis. Where shipped alone
  acts it gains 0.78 IMP per board. Where the arm alone acts it gains only
  0.52 (S) or 0.64 (O). That is the seed and order loss, as a per-board
  rate.
- **k8new is also more passive.** Its passive share is 0.55–0.57 in each of
  the four cells, against S and O's 0.49–0.53, about 4,400 extra passes
  where shipped competes. This is the most precise statistic in the read
  (n ≈ 38k per arm). It is k8new-specific: the two arms that share either
  seeds or order with shipped keep shipped's balance. It is also why
  k8new's PD reads +0.0026 while its plain reads −0.011. The tilt is a draw
  of the shared random effects (seeds and order), not a recipe change, so it
  is not a lever.

**Decided (jdh8, 2026-10-25): the gap is mostly lucky seeds.** It is
recorded as about two-thirds winner's curse and one-third unresolved order.
Retrains keep reading against shipped, which they must beat by about
0.007–0.011. They also read against a reseed of their own recipe (k8new for
v6), to separate a recipe effect from a draw. §4 stays a cheap optional bet
at ≤ 0.003: not scheduled, not blocking. §5 started the same day.

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

**Read 2026-10-27.** Both arms ran at `d705cc94` (driver and logs in
`/data/public/jdh8/pons/target/relabel-pilot/`, cuts at `<arm>/cut/axis-0004`;
the comparison pairs rows by `splice-labels.py`'s key and identity, 422,366 of
425,350 rows paired, 2,954 unmatched, 30 ambiguous, none refused). Shares are
of the 204,211 net-served decisions unless said otherwise.

| diff | labels moved | of labels fired by either cut | fired, left vs right |
| --- | ---: | ---: | --- |
| probed cut vs ⊤ cut (same walk, same draw) | 2.9% (5,850 rows) | 22% | 23,395 vs 23,703 |
| ⊤ cut vs M32 cut (HEAD vs `39318c7f`, a new draw) | 14.6% (29,553; 7.1% of rows) | 76% | 23,463 vs 28,181 |
| **noise floor**: ⊤ re-cut with the pools swapped (`scripts/cut-noise.py`) | 4.7% | 34.5% | 23,703 vs 23,684 |
| noise floor, M32's own returns swapped | 5.1% | 31.7% | 28,252 vs 28,219 |

A swap shares its 64 layouts between the two cuts; two independent draws
differ more. At M = 16, where the 64 layouts hold two independent cuts, the
independent diff is 2.3× the swap (⊤: 11.3% vs 4.9%; M32: 13.3% vs 5.3%), so
a redraw alone at M = 32 moves ≈ 11–13% of decisions. Three readings:

1. **The probe is flat.** Its 2.9% sits under the swap floor. The probe's
   last test closes; the labels it would cut are the ⊤ labels.
2. **Staleness is small.** Of the 14.6% that moved between the M32 cut and
   the ⊤ cut, ≈ 11–13 points are the redraw; the book's drift since
   2026-09-13 is ≈ 2–3% of decisions. What is systematic is the firing
   rate: the gate fires 17% less often at HEAD on the same rows (23.5k vs
   28.2k), and a swap reproduces a fire count to 0.1%, so the book now
   agrees with the EV-argmax more often. A full relabel (≈ 170M tables, 2
   days free, 8 occupied) buys that 2–3% and the fewer firings; the rest of
   what it changes is dice.
3. **The gate is the finding.** One label in three that M = 32 fires does not
   survive a swap of its own layouts, at either commit. That noise is in
   every label the shipped floor trained on. Depth is the lever (M = 64
   needs 128 layouts, twice the cost); the cheap read before any 170M-table
   spend is to **extend the ⊤ arm's four chunks to 128 layouts** (10.7M
   more tables; the chunk count caps it at four processes, ≈ 4 h on a free
   box at 4 × `DDSS_THREADS=64`): that gives the true independent noise at
   M = 32 and the M = 64 gate's self-agreement, and decides M for the
   relabel if one is run. Decision owed (jdh8).

**Read 2026-10-27, 128 layouts.** The ⊤ arm's four chunks were extended
from 64 to 128 layouts in place at `f8999e18` (`extend.sh` beside
`driver.sh`, same `OUT`, `LAYOUTS=128`; 4 × `DDSS_THREADS=32` under the
poker solver, 10 h 16 min for 10.9M tables, ≈ 298 tables/s). Every worker
read its first 64 layouts back from `.dd` and solved only the new 64, and
`dump-teacher --cut 32` on the extended chunks reproduced the 64-layout cut
byte for byte (`.f32` and `.tags`; the sidecar differs only in `git_sha` and
`layouts`), as the module doc's extension clause promises. The 128 layouts
hold one M = 64 cut (`top/cut64`: 25,026 fired on 169,972 eligible, against
170,051 at 64) and two **independent** M = 32 cuts, on `[0, 64)` and
`[64, 128)`. `scripts/cut-noise.py -m 64` and the new `scripts/cut-stakes.py`
(every pair of cuts, the stakes and the held-out test below) over the 204,211
decisions:

| pair of cuts | labels moved | of labels fired by either | fired, left vs right |
| --- | ---: | ---: | --- |
| M = 32, pools swapped, first half (the §5 floor) | 4.7% | 34.5% | 23,703 vs 23,684 |
| M = 32, pools swapped, second half | 4.8% | 35.1% | 23,641 vs 23,614 |
| **M = 32, the two independent halves** | **9.8%** | **60.8%** | 23,703 vs 23,641 |
| M = 64, pools swapped | 4.6% | 32.2% | 25,026 vs 25,018 |
| M = 64 vs either M = 32 cut | 6.8–7.0% | 45–46% | M = 64 keeps 71% of an M = 32 cut's firings |

The four pairings of an M = 32 half against the other agree to 0.04 points
(9.79–9.83%). Three more readings:

4. **The independent M = 32 noise is 9.8%**, measured, a little under the
   11–13% scaled from M = 16. Reading 2 revises upward: the 14.6% that moved
   between the M32 cut and the ⊤ cut, net of a 9.8% redraw, leaves ≈ 5% of
   decisions as the book's drift (union-of-moves arithmetic, (14.6 − 9.8) /
   (1 − 0.098) = 5.3, two moves of one decision counted once), not 2–3%.
   That plus the 17% lower firing rate is what a full relabel at M = 32 buys.
5. **Depth buys little.** From M = 16 to 32 to 64 the swap floor reads 4.9%,
   4.7%, 4.6% and the independent diff 11.3%, 9.8%, ≈ 9.5% (the last scaled
   by the M = 32 ratio of 2.06; a 1/√M law would read 11.3, 8.0, 5.7). The
   M = 64 gate fires 5.6% more often than M = 32 and disagrees with its own
   swap on 32% of fired labels against 34.5%. Doubling the double dummy of a
   relabel (≈ 340M tables) would shave two points off the noise share.
6. **The disagreements are real stakes, mostly missed, not wrong.** Where
   the two independent M = 32 cuts disagree, the two labels' means over all
   128 layouts differ by a median 0.86 IMP plain (0.77 PD); 44% differ by
   1 IMP or more and only 13% by under 0.25, so the gate is not breaking
   ties. Tested on the other half's 64 layouts, which the cut never saw, an
   M = 32 firing beats the own call on both scorers 83% of the time, clears
   the margin on both 77%, loses on both 7%, with a mean held-out swing of
   2.1 IMP plain (2.6 PD). Split by whether the other half's cut repeats
   the label: the 55% it repeats gain 3.1 / 3.9 IMP held out; the 45% it
   does not **still gain 1.0 / 1.0 IMP held out** and beat the own call on
   both scorers 63% of the time. So the one-label-in-three that a swap
   moves is mostly a real but marginal gain the other draw's gate failed
   to fire, a recall miss, not a false label; the floor's training labels
   err by omission at the 1-IMP scale.

Verdict on M (recommendation; the decision is jdh8's): **M = 64 is not
worth its 2×**, so a relabel, if one runs, runs at M = 32 for ≈ 5% of
decisions and the lower firing rate, trained under whatever order and recipe
§3 settles. Pilot data: `top/chunks` at 128 layouts with `.dd`/`.ret`,
`top/cut` (M = 32, 64-layout) and `top/cut64`.

**Time.** Each arm took 12 h, not 10: a 5,000-deal chunk of this shard
prices 50.6k decisions at 64 layouts, 2.69M tables (10.7M per arm), and the
four `DDSS_THREADS=32` workers under `SCHED_IDLE` solved 254 tables/s, the
occupied-box rate of [shared-machine-data-gen.md](shared-machine-data-gen.md)
"Sizing the solver". The probed arm's 12 h were avoidable: its draw matched
the ⊤ arm's to 99.99% (2,685,577 of 2,685,902 layouts of chunk 0), but its
`.dd` cache lives beside its own chunk and nothing seeded it from the sibling
arm, so it solved every table again (`0 from .dd`). `scripts/relabel-worker.sh`
now takes `DD_FROM` (a sibling root whose `.dd` is copied in before the
binary runs); a second arm of one window is minutes of solving with it.

**Full relabel, read 2026-10-29.** The whole v6 recipe (20 shards, 188
chunks, 6,781,198 rows against the M32 cut's 6,766,821) relabelled at
`54659d41`, M = 32 (64 layouts), from 2026-10-28 00:10 to 2026-10-29 10:29
(+08): 34.3 h, 156.9M tables, 1,270 tables/s from 10 workers ×
`DDSS_THREADS=64` on the free box (the poker solver was down; the ≈ 1,150
tables/s forecast held). Driver, logs and data:
`/data/public/jdh8/pons/target/relabel-head/{driver.sh,driver.log,worker-*.log,chunks,cut,cut.log}`.
axis-0004's `.dd` was seeded from the pilot's ⊤ arm through `DD_FROM`, and
its cut reproduces the pilot's byte for byte (398,127,600 bytes, 23,703
fired). The cut: 3,244,804 decisions, 2,620,106 at ≥ 64 layouts, **377,516
relabelled** (14.4% of the eligible, 11.6% of decisions, 5.6% of rows). Per
shard the axis shards fire 23.2–23.9k of ≈ 169–170k eligible, the uniform
18.4–18.7k of ≈ 133k, the enriched 10.2–10.3k of ≈ 50–51k.

One stumble. The driver's own cut exited 1 on `axis-0002: chunk 2's sidecar
disagrees with chunk 0's`. `dump-teacher` stamps `git_sha` by running
`git rev-parse HEAD` **at run time**, not from the build, so the two
docs-only commits that landed during the run (`99509eea`, `8530e82c`) split
the 188 sidecars 17 / 136 / 35 while the binary (built 00:10 at `54659d41`,
never rebuilt) and the walk never changed. The sidecars as written are kept
in `relabel-head/sidecars-as-written/`; the 188 live ones were stamped
`54659d41` and the cut rerun (5 s: pure file work, zero pending solves).
The same stamp gates `.ret` reuse (a worker at another sha re-prices from
`.dd`), so a run that straddles any commit, even a docs one, forfeits its
returns on a resume. **Discrepancy flagged, not fixed** (jdh8's call): the
stamp should be the build's commit, e.g. `option_env!("PONS_GIT_SHA")` set
by the launching script or a build script, falling back to the runtime
`rev-parse`; the cut's `GLOBAL` pin is right as it stands.

**Retrain and A/B, read 2026-10-29: a loss, nothing ships.** K = 8 twice on
`relabel-head/cut` on dl02 (14:15–15:14 +08, 16 nets in 58 min; the trainer
is the 2026-10-24 `fabcb6fe` build, arm R's recipe: Plan B's flags, the
shipped stem order, `--row-split`). Seeds 1–8 are arm **new**, 9–16 arm
**reseed**. Final val top-1 86.4–86.9% overall, 84.8–85.2% contested (a
row split; the ruler's 85.3/83.1 is a board split and not comparable). The
nets were scan-folded against the HEAD cut (originals
`dl02:/mnt/ssd-data/jdh8/pons-relabel-head/nets`, folded
`…/nets-folded`). A/B: `scripts/ab-floor-file.sh ab-results/relabel-head`
with new, reseed and **k8new** (`ab-results/ruler/nets/seed9..16`, the M32
recipe's reseed) as arms against the shipped K = 8 as `plain`, four cells,
204,800 boards per arm per cell (32 × 6,400), `SEED_BASE=1793264477711196890`,
on dl02 17:01–21:49 +08. IMPs/board against shipped, plain DD / perfect
defense (95% CI):

| arm | none | ns | ew | both | mean |
| --- | --- | --- | --- | --- | --- |
| new | −0.0320 ±0.0088 / −0.0227 ±0.0095 | −0.0176 ±0.0094 / −0.0104 ±0.0103 | −0.0492 ±0.0105 / −0.0361 ±0.0115 | −0.0293 ±0.0105 / −0.0159 ±0.0115 | **−0.032 / −0.021** |
| reseed | −0.0323 ±0.0089 / −0.0195 ±0.0095 | −0.0152 ±0.0094 / −0.0108 ±0.0103 | −0.0458 ±0.0105 / −0.0297 ±0.0115 | −0.0285 ±0.0106 / −0.0142 ±0.0115 | **−0.031 / −0.019** |
| k8new | −0.0136 ±0.0063 / +0.0011 ±0.0069 | −0.0038 ±0.0068 / +0.0068 ±0.0076 | −0.0190 ±0.0074 / −0.0049 ±0.0082 | −0.0133 ±0.0075 / −0.0019 ±0.0082 | −0.012 / +0.000 |

Every new and reseed cell is a loss on both scorers, and on the 16-world
single-dummy diff (plain and PD) as well. The calls diverge on 10.5% of
boards against k8new's 5.9%. Three readings:

1. **The harness is sound.** k8new reproduces §1's −0.011 reseed gap.
2. **The loss is the data, not the draw.** new and reseed agree to 0.002,
   and both sit ≈ 0.02 below k8new: the relabelled corpus, or something
   between it and the served net, costs the floor about 0.02 IMPs/board
   against the M32 corpus under one recipe. The bar was to beat shipped by
   ≈ 0.007–0.011; the read misses it by ≈ 0.04.
3. **The largest single piece is a learned 4NT over their game.**
   `ab-dump-bucket --by side|node` (ew cell, new): boards they open carry
   −9,134 of −10,067 plain and −7,595 PD against a −7,387 total (boards we
   open are −933 / +208). The top nodes are our `4NT` where the control passes:
   `1NT - 2♣ - 2♠ - 4♠ ⟨4NT⟩` in each seat, `1♥ - 3NT - 4♥ - - ⟨4NT⟩`,
   `2♠ - 2NT ⟨4NT⟩`, `2♥ - 2NT ⟨4NT⟩`, ≈ −11 IMPs a board. Every node whose
   ON call is 4NT: new 274 boards −1,977 / −2,500 (none), 271 −2,500 /
   −2,636 (ew); reseed 252 −2,186 / −2,272, 277 −2,458 / −2,589; **k8new 2
   boards each**. So 25–30% of the loss is one call the HEAD-cut nets learned
   and the M32 nets never play. Over a game they already reached it is a
   sacrifice, which double dummy prices, not an obstruction it misses. The
   rest of the loss is spread thinly over defensive nodes (passing where
   the control competes over their Jacoby 2NT or a raise).

**Owed next (not started):** the label forensic that separates a corpus
effect from a pipeline one. Count 4NT labels at their-game positions
(`… 4M ⟨our turn⟩`, `2M - 2NT ⟨our turn⟩`) in `relabel-head/cut` against
the M32 cut. More 4NT labels at HEAD means the relabel's rollouts reward
the sacrifice (a candidate: the rollout's opponents fail to double it,
where the A/B's do); the same count means the cause sits between the corpus
and the served net (the fold below, the corpus build). The cut lives on
Compute-Node02 only. The Gaussian v9 variant stays undecided.

**Flag: slot 0 of the compact agreements is constant in the HEAD cut.** The
fold took 32 columns where the M32-corpus nets took 30. The two new ones are
features 120 and 148, slot 0 of each side's compact-agreements block. In
the M32 corpus (cut at `39318c7f`, 2026-09-13) slot 0 was the `dutch()`
system, and `dump-teacher`'s `{dutch, kickback}` cells rotated it. On
2026-09-24 (`13131b2c`) the Dutch system was dropped, slot 0 became
`opening.wide_one_club` read off the knob, and the dump's cells became
`kickback` plus the 16 `AXES`, none of them wide 1♣. So every corpus dumped
since is constant at slot 0, and a net trained on one folds it:
`wide_one_club = true` becomes invisible to the floor. The shipped K = 8
still reads the slot, but as the whole Dutch system it trained on. That
matters because the knob's 2026-09-24 whole-arm win (+0.018 plain, both
cells) came mostly from boards they open, through this slot moving the
floor. Under a HEAD-corpus floor that win is gone by construction, and a
re-measure of the knob would read the floor's blindness, not the opening.
Harmless for this read (the slot is 0 on the default system, both sides).
**Decided (jdh8, 2026-10-29): leave the dump as it is and
treat `wide_one_club`'s floor regime as tied to the shipped K = 8.** The
knob stays off, and any future corpus that should see it adds a
`wide_one_club` axis to `AXES` as bit 16. That is cheap, but it only
matters if wide 1♣ comes back as a candidate.

## 6. What stays decided

- **The shipped K = 8 is a lucky draw** (jdh8, 2026-10-25, §3 step 2):
  a retrain ships only if it beats shipped, and it is read against a reseed
  of its own recipe to tell a recipe effect from a draw.
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
- **`wide_one_club`'s floor regime belongs to the shipped K = 8** (jdh8,
  2026-10-29, §5). Corpora keep slot 0 constant; a `wide_one_club` axis
  joins `AXES` as bit 16 only if wide 1♣ returns as a candidate.
