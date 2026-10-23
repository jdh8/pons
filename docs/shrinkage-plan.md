# Shrinkage and damped fixed points — what the poker R-loop taught, applied here

**Status (2026-10-24): A0, A1, A3, B, C and A4 lap 1 built and read in two
days. The probe's drift is the sample cliff, not a cycle (A2 dropped); the
κ-slope removes the cliff (`probed_kappa`, default 200, κ = 0 the old
behaviour byte for byte); A3 read the probed serving's loss as a **slope in
κ toward the base arm that never crosses it** — no κ ships; A4's retrain lap
under the probed boxes **stopped at lap 1**: the retrained floor, served
matched, is still −0.010 / −0.007 plain under the shipped floor, its gain in
its own regime (a4 − k200) sits inside ±0.008 of zero, and the probe's
residual and gain under the new net read a fixed point. The probe stays off
the default path; Plan C's estimator is settled by a split-half check
(DerSimonian–Laird). See "Measured so far". **Re-read 2026-10-24 (jdh8:
LGTM): under transplanted labels the residual rule was not a live test, and
the next lever is the reading's encoding, not another lap — see "Next: what
lap 2 owes".** Written after the poker-trainer
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

## Measured so far (2026-10-23 → 24)

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

**A4, lap 1 (read 2026-10-24).** `ab-results/probe-retrain`, `SEED_BASE`
1792774741138280180, 96 × 2,136 boards per arm per vulnerability, arms
plain / k200 / a4 / a4off as § A4 defines them (the probed arms
`--ns-probe 100000 --ns-probe-vacuous`, κ 200); direct A/B at table A,
δ = 0.002:

| pair | vul | fired | plain IMPs/board | verdict | PD IMPs/board | verdict |
| --- | --- | --- | --- | --- | --- | --- |
| a4 − plain | none | 8.31% | −0.0101 ±0.0074 | loss | −0.0016 ±0.0081 | inconclusive |
| a4 − plain | both | 7.34% | −0.0071 ±0.0089 | inconclusive | −0.0037 ±0.0099 | inconclusive |
| a4 − k200 | none | 8.83% | +0.0037 ±0.0075 | inconclusive | −0.0011 ±0.0083 | inconclusive |
| a4 − k200 | both | 7.87% | −0.0002 ±0.0090 | inconclusive | −0.0124 ±0.0101 | loss |
| a4off − plain | none | 8.68% | −0.0111 ±0.0075 | loss | −0.0087 ±0.0083 | loss |
| a4off − plain | both | 7.65% | −0.0072 ±0.0090 | inconclusive | −0.0097 ±0.0100 | inconclusive |
| a4 − a4off | none | 2.56% | +0.0010 ±0.0037 | inconclusive | +0.0075 ±0.0042 | win |
| a4 − a4off | both | 2.34% | −0.0002 ±0.0046 | inconclusive | +0.0060 ±0.0053 | win |
| k200 − plain | none | 2.86% | −0.0136 ±0.0039 | loss | −0.0003 ±0.0043 | inconclusive |
| k200 − plain | both | 2.57% | −0.0065 ±0.0047 | loss | +0.0091 ±0.0054 | win |

Single-dummy (`ab-dump-sd`, 16 worlds) agrees in every cell: a4 − plain
−0.0098 ±0.0077 loss / −0.0061 ±0.0092 inconclusive on plain SD; a4 − k200
+0.0070 ±0.0078 non-inferior / +0.0052 ±0.0093 inconclusive on plain SD,
+0.0013 / −0.0075 inconclusive on SD-PD; a4 − a4off plain-flat with the same
SD-PD win (+0.0058 ±0.0042 / +0.0037 ±0.0053).

Ruler (`scripts/ruler-tv.py`, reach-weighted TV, NV / both; depth 3 first,
full depth in brackets; Plan B's K = 8 floor is 0.0169 / 0.0150
[0.049 / 0.043]):

| pair | depth 3 | full | reads as |
| --- | --- | --- | --- |
| a4off vs plain (the retrain alone) | 0.0195 / 0.0183 | 0.053 / 0.047 | **1.1–1.2× the K = 8 floor** |
| a4 vs plain (the candidate) | 0.0184 / 0.0172 | 0.051 / 0.045 | same |
| k200 vs plain (the serving under the shipped net) | 0.0059 / 0.0053 | 0.016 / 0.014 | a third of the floor |
| a4 vs a4off (the serving under the new net) | 0.0053 / 0.0050 | 0.014 / 0.013 | same third |

(The 01:30 partial read quoted 0.053 "at depth 3" against the 0.017 floor,
≈ 3×; that number was the full-depth read — the script's default depth is
0 — and the apples-to-apples ratio is the 1.1–1.2× above.) The retrain's
top keys are the floor's own: their opening and response with us in fourth
seat (`1♦ - 1♥` TV 0.05 at 10k visits, `1♥ - 1♠` 0.07–0.09, `1♠ - 2♠` 0.19,
`1♠ - 2♦` 0.31–0.33 at 1.3k), where two reseeds also disagree; the serving's
top keys are advancer's and the doubler's (`1♦ 1♠ -` 0.10–0.16, `1♣ 1♦ X`
0.21–0.23), the positions where a served box changes what the net sees. The
probe's own report under each net (identical in both cells, the probe seed
being fixed): shipped net 5,650 keys, 1,110 drifted, residual rms 0.160 vs
floor 0.451 (0.4×), cliff share 20%, gain 0.00; a4 net 5,654 / 952 / 0.137
vs 0.459 (0.3×) / 19% / −0.00.

Four readings. **The first branch fails**: a4 loses to plain on plain DD at
`none` (−0.0101, lower bound −0.0175) with PD inconclusive in both cells, so
the serving does not ship with the retrained floor. **The lap did not move
the wall**: a4 − k200, the retrain's gain in its own regime, is +0.0037 /
−0.0002 on plain DD with ±0.008 intervals and a PD loss in the both-vul
cell — "a4 above k200" is not established by any verdict, which is the
third branch, **stop**. **The residual rule says the same**: the probe
under the new net reads rms 0.160 → 0.137 (14% lower), cliff share
20% → 19%, gain 0.00 → 0.00 — a fixed point, not a step toward one, so no
second lap is owed by the plan's own rule, and the ruler agrees: the whole
corpus change (features moved in 23% of rows, labels fixed) moved the
policy by 1.1–1.2× what two reseeds do, at the same keys. **The one
mechanism the lap did isolate**: the serving's plain-DD cost is a
train/serve mismatch and the retrain removed it — under the shipped net the
probed serving loses −0.0136 / −0.0065 on plain, under the retrained net the
same serving is flat (+0.0010 / −0.0002) and wins PD in both cells (the
PD-only win the decision table discounts as doubling-priced) — but the
retrained net served without the boxes is the mismatch the other way and
costs the same (a4off − plain −0.0111 / −0.0072 plain, −0.0087 / −0.0097
PD), so the two matched regimes, (⊤ trained, ⊤ served) and (probed trained,
probed served), end −0.0101 / −0.0071 apart on plain DD in the symbolic
regime's favour. Served matched, the shrunk boxes are not better
information than ⊤ for this net; the information they add is
doubling-priced. Disposition: **lap 1 stops; nothing ships; the probe stays
off the default path** (A3's verdict stands). The lap-1 nets stay in
`ab-results/a4/nets/` (not in the repo); `dump-teacher --ns-probe` and the
runner stay as built for any later lap. Not owed, but cheap if the question
is ever asked: a4off against Plan B's `k8new` (same init seeds 9–16) on
paired deals would read the corpus effect net of seed noise.

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

## Next: what lap 2 owes (2026-10-24, agreed with jdh8)

Two questions asked of lap 1's stop — does more compute change it, and is
the data structure the problem — and the answers, with the three changes
they make to this plan.

**Why lap 1 could not have moved.** The a4 corpus moved the features (23%
of rows) and transplanted the M32 labels unchanged
([features-v8.md §2](ai-bidder/features-v8.md), the stale-label caveat).
A BBA one-hot label depends on BBA's hand and BBA's reading of *its*
partner, never on our box; only the EV-argmax rows (the M32 cut, 460,341 of
6,766,821) can express what a tighter reading of partner is worth, and those
were rolled out under the ⊤ readings of 2026-09-13. The retrained net could
therefore learn at most that the moved columns are uninformative about a
fixed target: "a4 ≈ plain in the matched regime" and the residual's fixed
point are partly by construction. The "stop on the residual" row in "Order
and cost" was not a live test in lap 1.

**The encoding is the data structure that bit, not the rung overload.** The
precedence/logit overload of the `i16` weight is audited and harmless here
([logit-calibration.md §2](ai-bidder/logit-calibration.md): nothing on the
default path reads a book rung as a magnitude, book and net logits never
share a `Logits`); splitting it is a KR2 refactor with no KR1 effect. What
the v6 floor receives is each hidden seat's box as **hull endpoints**
(`min/13`, `max/13` per suit, `push_inference_v6` in
`src/bidding/features.rs`), and the encoder's own doc says why that is the
wrong shape: `♥5..13` and `♥5..8` are the same claim to four digits of mass
yet the max column moves 1.00 → 0.62, "which is why every hull-tightening
chop has had to buy a retrain before it could be judged on merit". A
κ-shrunk probed box is exactly such a chop, served live, at endpoint values
the net saw rarely, and in the disjunctive positions (Or walls, rule
competition) where a single hull is the loosest summary. The distributional
twin exists for the evaluator net — `push_shape_dist` conditions a per-suit
length distribution on the *union* of boxes (mean, sd, marginal histogram,
log-mass), so a box that excludes no hands moves no column, and the probe
could serve a union of clusters instead of one hull
([sampled-projection.md](ai-bidder/sampled-projection.md), the cap table).

**The three changes, in order:**

1. **`features_v9` for the policy floor, before any lap 2** — the hull
   endpoints replaced by the shape-distribution block. A feature bump under
   transplanted labels, the v8 pipeline
   ([features-v8.md §4](ai-bidder/features-v8.md)): minutes to re-walk and
   splice, ~10 min to train K = 8 on dl02, no relabel. Gate it first under
   ⊤ readings (v9 non-inferior to v6 on both scorers, read K = 8 vs K = 8
   on paired deals against Plan B's floor), then re-run A4's four arms
   under v9. This tests lap 1's mechanism directly and costs hours.
2. **If a4 − k200 turns positive under v9, the relabel at HEAD under the
   probed reading** — the only way the labels can carry the box, and the
   deferred 2026-09-26 "megachonk" and v8's lever list in one job. ≈ 170M
   DD tables: **≈ 2 days on a free box** (10 × `DDSS_THREADS=64`), ≈ 8
   occupied ([shared-machine-data-gen.md](shared-machine-data-gen.md)
   "Sizing the solver"). Detached, state in memory.
3. **Size branch-2 reads at ≈ 3M boards per arm per vulnerability.** Lap
   1's a4 − k200 was read at ±0.0075, four times δ; resolving it needs
   ≈ 14× the boards. The lap-1 arms took ≈ 1.6 h per arm pair at
   `JOBS=96`, so that is a few hours per pair on the whole box. More boards
   cannot flip a4 − plain (−0.0101, a clear loss); they are the only way
   branch 2 can be read at all. Lap 1's branch 2 was **unresolvable at
   204,800, not refuted**.

Not worth the compute: the floor sweep's closed axes (width, epochs, lr,
`--dd-weight`; [archive/floor-sweep.md](archive/floor-sweep.md)) and A2's
damping (gain ≈ 0 measured).

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

### A4 — the probe-first retrain, now as a lap (lap 1 read 2026-10-24: stop)

**Lap 1 built and launched 2026-10-23 ~22:50, read 2026-10-24 ("Measured
so far", A4): the third pre-registered branch, stop.** The build, so the
read can be reproduced:

- **Corpus** (`target/corpus-a4-chunks` → `target/corpus-a4`): the M32 recipe's
  188 fleet windows re-walked at HEAD by `scripts/relabel-worker.sh`
  (`LAYOUTS=0`, 24 workers under `idle-run`) with the new
  `dump-teacher --ns-probe 100000 --ns-probe-vacuous` (κ = crate default 200,
  probe seed `0x9B0BE`, the same harvest `bba-gen --ns-probe` serves), v6
  features; labels transplanted from the M32 cut by `scripts/splice-labels.py
  --any-identity` ([features-v8.md](ai-bidder/features-v8.md) §2, the
  2026-09-26 choice, so the only thing that moves vs the shipped corpus is the
  reader's `Inferences` block). The smoke chunk (`axis-0004/chunk-0`): probe on
  vs off, features moved in 23.45% of 105,876 rows, labels 0; the plain
  re-walk has 236 more rows than the fleet's chunk (BBA's auctions moved with
  the card our live knobs generate), so the splice pairs by key, not position.
  **Splice fixed and the driver relaunched 2026-10-24 00:21 (v2).** The v1
  splice (23:18–23:19) matched the axis shards at under 1%: every re-walked
  deal sits under two cells, so its reading-free key is duplicated among the
  old rows and the splice dropped it as *ambiguous* — a case the shipped
  corpus's 2026-09-26 splice never met, those shards being positional then.
  `scripts/splice-labels.py` now pairs a duplicated old key to the one old row
  of the same rebuilt identity (`resolved` in its report). Audited after the
  fix, default mode: axis-0004 422,366 of 425,350 rows transplanted (99.3%,
  28,181 M32 relabels, identity 0); enriched-0 171,021 of 172,771 (99.0%,
  12,426 relabels; 46 prefix mismatches, which `--any-identity` transplants);
  uniform-0 165,481 of 332,640 under the default and 279,625 under
  `--any-identity` (the retired Dutch cells, [features-v8.md](ai-bidder/features-v8.md)
  §2), 18,505 relabels — on course for ≈ 424k relabels, the 2026-09-26
  splice's total. The v1 driver also audited under `--any-identity`, whose
  exit 1 fires on any identity mismatch, so the uniform shards would have
  stopped it even with the pairing right; v2 audits in the default mode
  (features-v8.md's pipeline, exit 1 = a tag mismatch) and splices under
  `--any-identity` as pre-registered. Per-shard record:
  `target/corpus-a4/<shard>.{validate,splice}`.
  The reader's own probe report under the shipped net: 5,725 keys, 1,124
  drifted, residual 0.4× the seed floor, cliff share 20%, gain 0.00.
- **Nets**: eight draws of the Plan B recipe (`--arch mlp --hidden 256
  --epochs 300 --lr 0.001 --wd 0 --batch 4096 --val-frac 0.10 --dd-weight 0`),
  init seeds 9–16 — the reseeded ensemble's twin, so a4off vs Plan B's `k8new`
  is the corpus effect net of seed noise — trained on dl02 (two per GPU on the 4090 and
  the 4070 Ti SUPER, CUDA 12.8, trainer at `fabcb6fe`), folded in scan mode over the lap corpus;
  blobs in `ab-results/a4/nets/`.
- **A/B** `scripts/ab-probe-retrain.sh ab-results/probe-retrain`, 204,800
  bd/arm/vul: plain (shipped K = 8) / k200 (shipped + probed serving, A3's
  arm paired on this seed) / a4 (new K = 8 + probed serving, the candidate) /
  a4off (new K = 8, map empty). Pre-registered: a4 non-inferior to plain on
  plain DD in every cell and a PD win ships the serving default-on with the
  retrained floor; a4 above k200 but under plain is the wall moved, not
  passed — lap 2 owed, stop rule Plan B's floor; a4 at or under k200 stops
  the lap. Ruler read: a4off vs plain with `scripts/ruler-tv.py` against
  0.017 / 0.015; each probed arm's stderr carries A0's report under its net
  (the residual the lap stops on).
- Driver: `/data/public/jdh8/pons/target/a4-pipeline.sh` (state and log
  beside it).

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
| A4 retrain lap — **lap 1 read 2026-10-24, stopped** (build record under § A4) | the deferred retrain as K = 8, ×2 | told: the retrain removes the serving's plain-DD cost but matched-probed still trails matched-⊤ by −0.010 / −0.007 plain; a4 − k200 inside ±0.008 of 0; residual at a fixed point; the net moved 1.1–1.2× the K = 8 floor. **Caveat (2026-10-24):** under transplanted labels the residual could not move, so the residual rule was not a live test; branch 2 unresolvable at this size (±0.0075 vs δ 0.002) | "Next: what lap 2 owes" |

A0 through A3 are KR1-neutral until A3's cell is read: every knob defaults
off, and the probe map is not on the default path.
