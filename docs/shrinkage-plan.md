# Shrinkage and damped fixed points — what the poker R-loop taught, applied here

**Status (2026-10-23, evening): A0, A1, A3 and C built and read in one day.
The probe's drift is the sample cliff, not a cycle (A2 dropped); the κ-slope
removes the cliff (`probed_kappa`, default 200, κ = 0 the old behaviour byte
for byte); A3 read the probed serving's loss as a **slope in κ toward the
base arm that never crosses it** — no κ ships, the retrain lap (A4, deferred
by jdh8) is the one lever left; Plan C's estimator is settled by a split-half
check (DerSimonian–Laird). See "Measured so far".** Written after the poker-trainer
preflop R-loop converged (five laps, 2026-10-14 → 2026-10-23; record in
`../../poker-trainer/docs/design/07-preflop-solver.md` "Lap 2" … "Lap 5",
estimator in `../../poker-trainer/scripts/calibrate-r.py`). Every number from
that loop below is quoted from those two files. Nothing here moves a call
until its own A/B under [measurement.md](measurement.md).

## The one-line verdict

`Partnership::probe` is the same machine as the poker R-loop — a measurement
of the policy's own behaviour fed back as the policy's input — and it was
iterated the way the R-loop was in lap 2, undamped and with a hard sample
cliff. The R-loop fixed that with two separate knobs, **evidence-weighted
shrinkage toward the prior the policy was solved under** (κ) and **a damped
step sized by the measured gain** (λ), plus **a seed-noise floor for the
ruler**. All three port to the probe without changing what consumers read.
Two smaller ports follow: a seed-noise ruler for retrains, and shrunk
per-node pools for the bucket ranking.

## Measured so far (2026-10-23)

**A0, the ruler.** `ProbeReport` now carries `rms`, `floor`, `cliff` and
`gain` (`Partnership::probe` runs three harvests: the second iteration's
served boxes are re-harvested on the next `boards` seeds for the floor, so
the floor is unpaired and bounds the paired residual's noise from above).
`probe-reading-census --probe N [--probe-vacuous]` prints it. Default
`american()`, the v2 census's probe seed (`1785200001 + 0x9B0BE`), `-c 100`:

| serving | boards | keys | drifted | rms | floor | rms / floor | cliff share | gain |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| full fold | 100k | 508 | 187 | 1.494 | 1.437 | 1.0 | 88% | −0.01 |
| vacuous | 100k | 471 | 14 | 0.324 | 1.468 | 0.2 | 89% | −0.00 |
| full fold, seed + 1 | 100k | 511 | 207 | 1.634 | 1.258 | 1.3 | 85% | −0.02 |
| full fold | 400k | 2,414 | 1,403 | 1.942 | 1.094 | 1.8 | 82% | −0.03 |

The pre-registered reading resolves to the second branch: **the 241/520
drift was never a 2-cycle.** The gain along the step is zero in every cell
(the poker loop read −0.87), so the served boxes neither push the
measurement back toward ⊤ nor contract it; the residual sits at the seed
floor; and 82–89% of that floor is keys flickering across `MIN_SAMPLES` — ⊤
in one harvest, a box in the other — not endpoint jitter inside stored keys.
The 400k row's 1.8× is the same mechanism, paired: installing boxes shifts
traffic and keys near the cliff cross it. Consequences: **A2 (damping) is
dropped** — λ = 1/(1 − 0) is today's full step; **A1 is the whole of this
plan's prize for the probe**, and it should lower the floor itself, which is
its gate when it lands (pre-registered: cliff share well under 80% at
κ = 200, the floor under 1.0). The v2 loss stays what
[sampled-projection.md](ai-bidder/sampled-projection.md) named it — the net
served boxes it did not train under — and A4's lap structure stands without
the damping.

**A1, the κ-slope.** `ReadingProfile::probed_kappa` (default 200; `κ = 0`
is the cliff). The probe stores the raw `Observed` per key from 20 samples
(`Partnership::STORE_FLOOR`) and `probed_box` estimates at serve time:
each endpoint of the slack-padded box shrunk toward ⊤ by `w = n / (n + κ)`,
rounded to nearest, so a thin key is never narrower than the cliff served.
Two deviations from the text below, both deliberate: the prior is **⊤**, not
the symbolic reading — identical under the vacuous scope (only ⊤ axes take a
box) and equivalent under the full fold, which intersects the served box
with the symbolic reading anyway — and κ is the **reader's** knob, passed
into `probed_box`, because an undeclared mirror carries its own copy of the
map and profile (the first build read κ off the serving book, and `κ = 0`
reproduced A0 only after the fix). The same command as A0, by κ:

| κ | serving | boards | keys served | drifted | rms | floor | rms / floor | cliff share | gain |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | full | 100k | 508 | 187 | 1.494 | 1.437 | 1.0 | 88% | −0.01 |
| 50 | full | 100k | 5,745 | 4,854 | 0.984 | 0.757 | 1.3 | 52% | −0.01 |
| 200 | full | 100k | 5,709 | 3,860 | 0.462 | 0.472 | 1.0 | 17% | −0.00 |
| 200 | vacuous | 100k | 5,748 | 1,027 | 0.153 | 0.469 | 0.3 | 18% | +0.00 |
| 200 | full | 400k | 16,373 | 15,335 | 0.702 | 0.453 | 1.5 | 10% | −0.01 |
| 1000 | full | 100k | 4,676 | 1,627 | 0.212 | 0.332 | 0.6 | 4% | −0.00 |

The pre-registered gate passes at κ = 200 (cliff share 88% → 17%, floor
1.44 → 0.47), with two honest caveats. The floor falling with κ is partly
tautological — a box nearer ⊤ has less room to move, and κ → ∞ reads 0 —
so the **cliff share** is the signal, not the floor. And `drifted` is now a
count of thin keys moving by a rounding step (3,860 of 5,709), so the key
count is retired as a reading; the ruler is the reading. Every stored key
serves from n = 20 on: the sub-`MIN_SAMPLES` tail that
[sampled-projection.md](ai-bidder/sampled-projection.md) called "a
boards-count dial" now serves wide, sound boxes instead of nothing. None of
this is an IMP: the 2026-07-31 arms reproduce with `--ns-probe-kappa 0`,
and whether κ = 200 (or an interior λ) turns the v2 loss into a slope is
A3's cell.

**A3, the κ pair.** `scripts/ab-probe-kappa.sh` → `ab-results/probe-kappa`,
`SEED_BASE` 1792752006877283681, 96 × 2,136 boards per arm per vulnerability,
arms base / κ 0 / κ 200, the probed arms `--ns-probe 100000
--ns-probe-vacuous`; direct A/B at table A, δ = 0.002:

| pair | vul | fired | plain IMPs/board | verdict | PD IMPs/board | verdict |
| --- | --- | --- | --- | --- | --- | --- |
| κ 0 − base | none | 3.22% | −0.0173 ±0.0040 | loss | −0.0001 ±0.0045 | inconclusive |
| κ 0 − base | both | 2.91% | −0.0089 ±0.0048 | loss | +0.0113 ±0.0056 | win |
| κ 200 − base | none | 2.86% | −0.0105 ±0.0038 | loss | +0.0001 ±0.0043 | inconclusive |
| κ 200 − base | both | 2.56% | −0.0044 ±0.0046 | inconclusive | +0.0089 ±0.0053 | win |
| κ 200 − κ 0 | none | 1.72% | +0.0068 ±0.0030 | win | +0.0003 ±0.0034 | inconclusive |
| κ 200 − κ 0 | both | 1.49% | +0.0046 ±0.0036 | win | −0.0022 ±0.0041 | inconclusive |

Three readings. The **negative control holds in kind, not in size**: κ 0 is
the 2026-07-31 arm and loses on plain with PD flat or a both-vul PD win — the
doubling artifact, and the worst boards are the July mechanism (the
contested net doubling, redoubling and driving to `4NT` on partner boxes it
never trained on) — but at a third the size and a third the fired rate
(−0.0467 → −0.0173, ~10% → 3%): `ReadingScope::All` shipped in between and
leaves the vacuous scope fewer ⊤ axes to fill. **The loss is a slope, not a
wall**: κ 200 beats κ 0 on plain in both cells, the plan's second reading.
**But the slope points at base and never crosses it**: κ 200 is still a
plain loss at `none`, the both-vul cell's lower bound (−0.0090) is under
−δ, and κ = ∞ *is* the base arm. Shrinking toward ⊤ trades information for
safety one for one; the serving never becomes a win because the net has
never seen a served box, which is A4's job. Disposition: **no κ ships**
(the knob stays off the default path; when the probe serves, it serves at
κ = 200); κ 1000 is not run — on plain it can only approach base from
below, and the PD both-vul win is the artifact the decision table discounts;
**A4 is the one lever left**, and it runs as a lap with this ruler (A0) and
this estimator (A1), no damping (A2).

**C, the shrunk pools.** `scripts/shrunk-pools.py` (node = bucket ×
(rule, our call); κ = σ²/τ², first by the unweighted method of moments the
plan specified below, since switched — see the end of this paragraph). On `3debdd0f`, both arms: **no top-10 node of any top-5 bucket
moves**. Every one has n ≥ 150 and κ is 2–8 in the big buckets (median 4.1
across buckets), so n + κ ≈ n. The thin-and-huge lane this plan imagined is
not in the current anchor — the thinnest high-loss nodes are n 18–45 at
−150 to −250 IMPs, nowhere near a top 10. Membership moves at rank 10 only
(shipping `Defensive / floor / round-2` demotes `5♦` n 39 for `2NT` n 294;
instinct opening demotes `3♦` n 721 for `1♣` n 1,167). The one bucket where
shrinkage bites is an estimator defect, not an insight: `Defensive / floor /
round-1` (27 nodes, most n ≈ 10–100) reads PD τ² = 0, κ = ∞, and the
*resolved* `2♥` node (n 716, −1,024 plain) is halved, because the unweighted
`mean_j(σ²/n_j)` is dominated by the thin floor nodes and swallows the
spread of means. **Resolved the same evening by a split-half check**
(`--holdout`): every node's boards split in two by deal, each estimator fit
on one half and scored on the other half's node means by n-weighted squared
error, both directions, summed over buckets — the way any hyperparameter of
a hierarchical model is judged. Ratio to raw means, lower is better:

| arm / scorer | unweighted MoM | DerSimonian–Laird | Paule–Mandel | full pooling |
| --- | --- | --- | --- | --- |
| shipping / plain | 0.79 | **0.68** | 0.76 | 0.75 |
| shipping / PD | 0.78 | **0.69** | 0.75 | 0.84 |
| instinct / plain | 0.78 | **0.68** | 0.76 | 0.84 |
| instinct / PD | 0.78 | **0.70** | 0.75 | 0.84 |

DL wins every cell, and by the most in the bucket MoM collapsed
(`Defensive / floor / round-1`: DL 0.57–0.58 of raw, MoM 0.81 plain and 1.10
PD, worse than not shrinking). Shrinkage is worth ~30% of the estimation
error even though it moved no top-10 ranking; every shrinker loses in two
small buckets (17 and 28 nodes) where τ² is underestimated, the known cost
of empirical Bayes with few groups. **Default is now DL**; `--tau2 mom|pm`
keep the alternatives, `--holdout` re-asks the question at the next anchor.
Under DL the thin-node bucket reads κ 119 plain / 58 PD and the resolved
`2♥` node −1,024 → −906, and the self-test's 12-board node collapses −399 →
−191, which is what this plan promised and MoM never delivered. The script
still changes no top-10 of a top-5 bucket; it is a check to run at each
re-anchor, not a lane.

## What the R-loop taught (the four lessons)

The poker loop: a realization table R (per hand class × position) is what the
preflop chart is solved under; the postflop store measures G(R), what each
class actually realizes when the chart solved under R is played; iterate
R ← G(R).

1. **A thin-evidence class measured at full weight flips the loop.** Trash
   offsuits that R₁ folded never reached a flop, so their tiny reach measured
   an off-range hand the villain did not expect (R ≈ 1.3); the next chart
   defended them, they realized 0.65 in range, the chart after folded them.
   Residual ‖G(R) − R‖ sat at 0.128 → 0.126, gain along the step −0.87: a
   2-cycle. 75% of the error was classes with ~0.02% of the summed share.
   Fix: `G_κ = (Σ share·r + κ·R_prior)/(Σ share + κ)`, κ = 0.005 of summed
   share, R_prior = the table the measured charts were solved under. A
   folded class keeps its prior; a defended one is measured. Residual
   0.148 → 0.064 → 0.027.
2. **Shrinkage damps the cycle but does not kill it; measure the gain and
   damp the step.** At κ the gain along the step was still −0.86 (a full
   step overshoots by its own length). The linear fixed point along the step
   is `R + λ(G(R) − R)`, λ = 1/(1 − gain) = 0.54. Lap 5 under that step:
   residual 0.017, gain −0.35. Aitken Δ² extrapolation bought nothing
   because successive steps were nearly orthogonal (ρ = −0.14) — measure
   the gain each lap, do not extrapolate from two points.
3. **The ruler needs a noise floor before a residual means anything.** Two
   charts solved under the *same* R with different seeds differ by 0.033 rms
   total variation per class; lap-to-lap moves were 0.09–0.13. So the moves
   were real at 3–4× noise, and they were indifference flips (EV gaps
   ≤ 0.05 bb, frequencies 0 ↔ 1), not macro shape. Per-action EVs were too
   noisy to score one chart under another's values; the frequency rms was
   the only usable chart-space ruler.
4. **The consumer must be re-solved under the input it is served, and the
   step kept small enough that its best response does not overshoot.** The
   chart is a best-responder to R, which is why an undamped loop flips. The
   same is true of any learned policy fed a reading it did not train under.

## Where each lesson lands in pons

| pons mechanism | R-loop analogue | what is wrong today | lesson |
| --- | --- | --- | --- |
| `Partnership::probe` (book.rs): self-play boxes per prefix key, served through `Inferences::read` under `probed` / `probed_vacuous` | R ← G(R): the probed box is the measurement, the bidder the policy | hard cliff `MIN_SAMPLES = 200` (store or ⊤), constant slack ±2 points / ±1 length regardless of n, two undamped iterations, drift reported as a key count (241/520, later 203/527), no noise floor; both servings **lost** (v2: plain −0.0467/−0.0658, PD −0.1118/−0.1337) with the mechanism named as the net fed boxes tighter than its training distribution | 1, 2, 3, 4 |
| the deferred retrain + rail re-arbitration ([next-steps.md](next-steps.md) Owed; [floor-rail-campaign.md](floor-rail-campaign.md) stop criterion 5) | a lap: rails were tuned against net v6, the retrain moves the net, the rails must be re-tuned | no ruler for "how far did the policy move vs. seed noise"; `smoke-default` only proves byte identity; rails and retrain would move in one undamped step | 3, 4 |
| bucket / node ranking ([bba-gap-campaign.md](bba-gap-campaign.md) Current ranking; next-steps lanes "sized off 8M deals", "12 boards per ~2M") | share-weighted blend | pools are raw `n · mean`; a 12-board lane with a large per-board loss outranks a 1,500-row wash; "below what three seeds resolve" is a judgment, not a number | 1 |
| `SD_BLEND_LAMBDA` per level (`examples/common/mod.rs`), the q table (`doubling-calibration.md`), convention range sweeps | per-cell estimates | level 7 fitted on n = 16; q cells all clear n ≥ 200 by 10×; sweeps pick an argmax of noisy cells but confirm on fresh seeds | 1 (minor, see "Not worth building") |

## Plan A — the probe: shrink, damp, and read against a floor

The prize is the one priced in
[ai-bidder/sampled-projection.md](ai-bidder/sampled-projection.md): 0.65–1.27
IMPs/board by the negative control, about the whole remaining gap to BBA. Both
servings lost, and the doc's own disposition is "knob off + probe-first
retrain". This plan inserts the three R-loop knobs *before* the retrain, because
each is cheap and each is a reason the retrain might not be needed at full
strength.

### A0 — a ruler and its noise floor (half a day, no behaviour change)

Replace `ProbeReport { keys, drifted }` with a traffic-weighted residual:

- per key `k` with `n_k` samples, per axis `a` (points, four lengths), the
  served endpoints `lo_{k,a}, hi_{k,a}` in points / cards;
- residual between two maps `rms = sqrt(Σ_k n_k Σ_a (Δlo² + Δhi²) / (Σ_k n_k · 10))`;
- gain along the step `g = Σ w·ΔG·ΔR / Σ w·ΔR²` where ΔR is the step just
  taken and ΔG the response (lesson 2);
- **noise floor**: one extra harvest with a different seed at the *same*
  served boxes; its rms against the first is the floor (lesson 3).

Keep `drifted` as a column, but the verdict reads `rms / floor`. Print all
three from `examples/probe-pass-meaning`. Pre-registered reading: the
241/520 drift is either real (rms ≫ floor, gain < 0 — a 2-cycle) or a
one-sample endpoint jitter at n ≈ 200 (rms ≈ floor); today nobody knows
which.

### A1 — κ-shrunk boxes toward the symbolic reading (one day)

**Built 2026-10-23** as `probed_kappa` ("Measured so far" has the numbers
and the two deviations: prior ⊤, reader-side κ). The text below is the
design as planned.

Store the measurement, not the estimate. The probe map becomes
`HashMap<Vec<Call>, Observed>` (raw spans + count; `Observed` is already the
aggregate type), and the estimator moves to serve time in the `Inferences::read`
fold, where the complete symbolic reading for that prefix is already in hand
(the vacuous scope compares against it today):

```
served_lo = sym_lo + w(n) · (obs_lo − slack_lo − sym_lo)
served_hi = sym_hi + w(n) · (obs_hi + slack_hi − sym_hi)
w(n) = n / (n + κ)
```

- `κ = 200` by default, so at today's `MIN_SAMPLES` the box sits halfway
  between the symbolic reading and the observed span, and a key with
  n = 10,000 is 98% observed. The cliff becomes a slope; the
  "sub-`MIN_SAMPLES` tail is a boards-count dial" sentence stops being true
  because every key serves something sound. Keep a storage floor
  (n ≥ 20) purely for map size.
- Direction of safety is preserved: a thin key is *wider* than today, never
  narrower, and the catastrophic side (a box the bidder's own hands fall
  outside) only gets safer. The constant slack stays as the n → ∞ guess;
  `ponytail:` its order-statistic replacement (`(hi − lo)/(n − 1)` per
  endpoint) is the upgrade if A0 shows endpoint jitter dominating the floor.
- The symbolic reading is the prior for the same reason the solved-under R
  table was in poker: it is what the consumer (the v6/M32 net) trained
  under. Shrinking toward it is shrinking toward the net's training
  distribution, which is the named mechanism of the v2 loss.
- Consumers do not change: `probed_box` returns the same `Envelope` it
  returns today, computed from `(Observed, symbolic, κ)` instead of looked
  up. The fixed-point iteration serves through the same fold, so the boxes
  stay consistent with their serving policy (the existing invariant).

Gate: `smoke-default` byte-identical with the knob off (it is off by
default); A0's rms at κ = 200 vs κ = 0 (today's behaviour reproduced
exactly with `κ = 0`, `n ≥ 200`).

### A2 — damped iteration sized by the measured gain (half a day)

**Dropped 2026-10-23 on A0's reading** (gain −0.03 … −0.00 in every cell;
"Measured so far"): λ = 1/(1 − 0) is the step the probe already takes. The
design stays here for a retrain lap (A4) that reads a negative gain.

Iteration `i + 1` serves `R_i + λ (G(R_i) − R_i)` per endpoint, with
λ = 1/(1 − g) clipped to [0.25, 1] from A0's gain, and runs until
`rms ≤ floor` or four iterations. Two iterations with λ = 1 is today's
behaviour. Expected from the poker record: gain negative on the first step,
λ ≈ 0.5, residual at the floor by iteration 3.

### A3 — the λ-serving sweep: is the v2 loss a wall or a slope? (one A/B cell)

**Read 2026-10-23** as the κ pair {0, 200} instead of a λ sweep (A1 made κ
the serving dial): a slope toward base that never crosses it; no κ ships
("Measured so far"). The design below stands as written.

The cheapest experiment this whole plan enables, and it needs no retrain.
Serve `box = symbolic + λ_serve · (shrunk − symbolic)` under the existing
`probed_vacuous` scope at λ_serve ∈ {0.25, 0.5, 1.0}, on the 200-board smoke
first, then one `none` cell (bidding ≈ 2 min/arm, DD once per λ). Two
pre-registered readings:

- **monotone loss in λ** — the net's out-of-distribution wall; nothing
  short of the retrain helps, and A4 is owed;
- **interior optimum** — a damped serving ships on its own full A/B, and the
  retrain's job shrinks to the residual.

Negative control: λ_serve = 0 must reproduce the base arm byte-for-byte.

### A4 — the probe-first retrain, now as a lap (owed; retrains deferred by jdh8)

When the deferred retrain runs, it is lap 1 of a loop, not a one-shot: probe
under the shipped net → shrunk boxes → retrain the floor under them → re-probe
under the new net → read A0's `rms / floor` and gain. The R-loop needed five
laps at κ and λ to converge from a 2-cycle; budget for two here and stop on the
residual rule, not on the A/B alone (lesson 4: the A/B judges the lap, the
residual says whether another is owed).

## Plan B — a seed-noise ruler for retrains (built and read 2026-10-23)

Lesson 3, applied to the floor. Before the deferred retrain: train the
current recipe twice with different training seeds, bid the same 20k boards
with each, and record per-prefix-key call-frequency total variation between
them, reach-weighted. That number is the floor every retrain's movement is
read against; the shipped-vs-retrained TV, localized by key, says whether a
retrain moved macro shape or traded indifferent keys — the distinction the
poker chart-space check made, and the one the "re-arbitrate every rail"
criterion cannot make today.

**The ruler is `scripts/ruler-tv.py`** (new; the plan's first draft credited
the TV to "the existing `ab-reading-drift` machinery", but that runner only
ever produced IMP diffs — no per-key TV existed before this script). It reads
two or more `ab-lib` arm directories bid over the **same deals** and, at every
auction prefix where our side chose the next call (both tables, dealer folded
out), takes TV_k = ½ Σ|p_k − q_k| of the next-call distributions and sums
reach_k · TV_k. A key one arm never reached counts TV 1 at its reach, so a
divergence upstream is charged again at every deeper key: the **depth-3
number is the headline** (our first three calls), the full-depth number ≈ 3×
it goes in the ledger beside it. `--halves` is the sampling baseline, and it
is the first rule: the **same net bid on different deals reads 0.15** at
19,200 boards, four times any seed effect, so **the ruler reads paired deals
only** (ab-lib's shared `SEED_BASE` is what makes it a ruler at all).

**Read (`ab-results/ruler`, 19,200 bd/arm/vul, SEED_BASE 1792758002859020831).**
Ten fresh draws of the shipped recipe on dl02 (CUDA 12.8, trainer `82d4a60d`
with its board-start validation cut; seeds 2, 3 and 9–16; held-out CE
0.4165–0.4201, all under the sweep's 0.42632 gate; each 7 min on a 4090,
folded 30 columns), the shipped recipe's own single draws `american_bba_v6_seed{2,3}`
from the repo, and the shipped K = 8 mean as control. Reach-weighted TV, depth 3,
NV / both vul:

| pair | NV | both | reads as |
| --- | ---: | ---: | --- |
| two fresh single draws (seed2 vs seed3) | 0.0435 | 0.0388 | the **K = 1 floor** |
| shipped single draws (s2old vs s3old) | 0.0462 | 0.0421 | same floor, old trainer |
| fresh seed2 vs shipped seed2 (same init seed, trainer's val cut moved) | 0.0437 | 0.0398 | the val-cut change **is a reseed**, no more |
| any single draw vs the shipped K = 8 | 0.033–0.034 | 0.030 | ≈ floor/√2: the mean sits at the cloud's centre |
| **reseeded K = 8 (seeds 9–16) vs shipped K = 8** | **0.0169** | **0.0150** | the **K = 8 floor**, 0.39 of K = 1 ≈ 1/√8 |
| same net, different deals (`--halves`) | 0.15 | 0.14 | sampling; why only paired deals count |

Full depth: K = 1 floor 0.123 / 0.108, K = 8 floor 0.049 / 0.043, single vs
shipped 0.097 / 0.085. The top keys everywhere are the second-seat defensive
positions after their opening and response (`1♥ - 1NT`, `1♣ - 1♦`, `1♦ - 1♥`,
TV 0.1–0.4 at a few hundred visits) — the contested book the net floors;
constructive keys do not appear, as the partition says they should not.

**What A4 reads against.** A retrained K = 8 ensemble that differs from the
shipped one by TV ≈ 0.017 / 0.015 at depth 3 has moved **nothing** — that is
two ensembles of the same recipe. Movement has to clear that floor, localized
by key, before the A/B's IMP is even worth reading as a retrain's; and the
IMP side agrees: the reseeded K = 8 vs the shipped K = 8 is a wash at this
size (plain −0.0015 ±0.0216 NV / +0.0190 ±0.0262 both; PD +0.0106 ±0.0233 /
+0.0360 ±0.0289), while every K = 1 draw loses −0.04 to −0.06 plain and
−0.06 to −0.10 PD to it — the ensemble gap the floor sweep measured
(K = 4 over K = 1: +0.045 / +0.056), not a retrain effect. A retrain **must
be read as K = 8 vs K = 8**; a single draw against the shipped floor measures
the ensemble, whatever else it was meant to measure.

Costs: ten draws ≈ 40 GPU-minutes on dl02 (two at a time), the six-arm A/B
≈ 25 min here at `JOBS=96` under the poker solver, the TV read 10 s per pair.
The runner fix that fell out: `ab-floor-file.sh` left `PONS_FLOOR_WEIGHTS`
exported into the next vul's control arm, whose `--provenance` then listed a
weights file it never used, and every resume of a multi-vul floor-file dir
tripped the manifest guard; it now unsets the variable before the control.

Then the rails: keep them fixed through the retrain lap (the constant R of a
solve), re-arbitrate after, and read each rail's flip as a step with a gain.
Flipping rails and retraining in one step is the undamped iteration of lap 2.

## Plan C — shrunk pools for the bucket ranking (an afternoon, pure Python)

Lesson 1, in its textbook form. For a bucket's nodes with per-board means
`μ̂_j`, counts `n_j`, within-node variance `σ²` (pooled from the dumps) and
between-node spread `τ²` (as specified: method of moments,
`var(μ̂_j) − mean(σ²/n_j)`, floored at 0; **built as DerSimonian–Laird**
after the split-half check in "Measured so far" — the same moment with the
nodes weighted by `n_j / σ²`):

```
μ_j^post = (n_j · μ̂_j + κ · μ̄) / (n_j + κ),   κ = σ² / τ²
pool_j   = n_j · μ_j^post
```

Add a `shrunk` column beside `plain` / `PD` in the decompose / `ab-dump-bucket`
output and rank on it. A 12-board lane at −400 IMPs collapses toward the bucket
mean; a 1,500-row lane at −0.9/board does not move. The "below what three seeds
resolve" judgment becomes `n_j + κ` against the sample size a δ-wide CI needs.
No bidding change, no A/B; it changes which lane is worked next, which is where
the campaign's days go.

## Not worth building (lesson 1 applies, the gain does not)

- **The q table** — every shipped cell has n ≥ 2,400 and ±0.02 Wilson
  intervals; shrinkage would move nothing.
- **`SD_BLEND_LAMBDA`** — level 7 is n = 16 and used by ~0 boards; the
  λ(4) > λ(5) inversion replicated. Leave the per-level fit; `ponytail:` shrink
  toward a monotone fit if a level under n = 200 ever carries weight.
- **Convention sweeps** — the fresh-seed confirmation rule already pays the
  winner's-curse tax; a fitted-curve optimum would save one confirmation run
  per sweep, not more.
- **The decision table** — δ, the worst-cell rule and the pre-registered CI
  stay exactly as they are. Shrinkage is for *estimates that feed a policy or
  a ranking*, never for the verdict that ships one.

## Order and cost

| step | cost | what it buys | blocks |
| --- | --- | --- | --- |
| A0 ruler + floor — **done 2026-10-23** | ½ day | told: never a cycle, the floor is the cliff | A1–A4 |
| C shrunk pools — **done 2026-10-23** | ½ day | told: no lane moves at this anchor; estimator choice owed | — |
| A1 κ boxes — **done 2026-10-23** | 1 day | the cliff gone (cliff share 88% → 17%); κ = 0 byte-identical | A3 |
| A2 damped iteration — **dropped** | — | gain ≈ 0 measured; kept as design for A4 | — |
| A3 κ pair — **done 2026-10-23** | 3 arms × 2 vul | told: a slope toward base, never above it; no κ ships | A4's scope |
| B retrain ruler — **done 2026-10-23** | 10 draws + 6 arms × 2 vul | told: K = 8 floor TV 0.017 / 0.015 (depth 3), the val cut is a reseed, read retrains K = 8 vs K = 8 on paired deals | A4 |
| A4 retrain lap — **owed** (retrains deferred by jdh8) | the deferred retrain as K = 8, ×2 | the prize; the one lever A3 left; stop rule = B's floor | — |

A0 through A3 are KR1-neutral until A3's cell is read: every knob defaults
off, and the probe map is not on the default path.
