# Shrinkage and damped fixed points — what the poker R-loop taught, applied here

**Status (2026-10-24): A0, A1, A3, B, C and A4 lap 1 built and read in two
days. The probe's drift is the sample cliff, not a cycle (A2 dropped); the
κ-slope removes the cliff (`probed_kappa`, default 200, κ = 0 the old
behaviour byte for byte); A3 read the probed serving's loss as a **slope in
κ toward the base arm that never crosses it** — no κ ships; A4's retrain lap
under the probed boxes **stopped at lap 1**: the retrained floor, served
matched, is still −0.010 / −0.007 plain under the shipped floor, its gain in
its own regime (a4 − k200) sits inside ±0.008 of zero, and the probe's
*inner* residual fell 14% under the new net (the outer one was never
measured; see "Review corrections"). The probe stays off
the default path; Plan C's estimator is settled by a split-half check
(DerSimonian–Laird). See "Measured so far". **Re-read 2026-10-24 (jdh8:
LGTM): under transplanted labels the residual rule was not a live test, and
the next lever is the reading's encoding, not another lap — see "Next: what
lap 2 owes". Reviewed the same day (Codex, six findings, each confirmed
against the code and folded in under "Review corrections"): paired IMP
intervals at four cells decide, TV is a diagnostic, the residual rule needs
the outer residual, the relabel is piloted, the splice offsets go
version-aware.** **2026-10-24/25: v9 fails its gate, and A4's arms inside
v9 stop again. The new encoding mostly ignores the probe's boxes, and with
the boxes in both training and serving the probe reads flat; the relabel
pilot is the one test left (see the end of "Next").** Written after the
poker-trainer
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
third branch, **stop**. **The residual did not decide this** (corrected 2026-10-24, review
D3 and S1): the probe under the new net reads rms 0.160 → 0.137 (14%
lower), cliff share 20% → 19%, gain 0.00 → 0.00 — but every number in
`ProbeReport` is the *inner* iteration (the ⊤ harvest against the harvest
serving its own boxes, under one fixed net); the *outer* residual, the map
lap 1 trained under against the map the retrained net re-probes, was never
computed, so "a fixed point" was not established. The ruler read — the
whole corpus change (features moved in 23% of rows, labels fixed) moved the
policy by 1.1–1.2× what two reseeds do, at the same keys — compared a
204,800-board TV against a floor read at 19,200; a finite-sample TV is
biased upward, more at the smaller count, so that ratio understates the
move. Both are owed a re-read at matched counts before any lap 2. **The one
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
paired deals would read the corpus effect net of seed noise. *(Correction
2026-10-24, "Gate read, change 1" below: k8new − plain is itself −0.012
plain DD at every cell at 204,800 boards, the size of a4off − plain and
a4 − plain. Read against the reseed, the "mismatch the other way" is not
established. The disposition stands: nothing in lap 1 beat the reseed
either.)*

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

**Why lap 1 was unlikely to move, and what that does not prove.** The a4
corpus moved the features (23% of rows) and transplanted the M32 labels
unchanged ([features-v8.md §2](ai-bidder/features-v8.md), the stale-label
caveat). A BBA one-hot label depends on BBA's hand and BBA's reading of
*its* partner, never on our box; only the EV-argmax rows (the M32 cut,
460,341 of 6,766,821) can express what a tighter reading of partner is
worth, and those were rolled out under the ⊤ readings of 2026-09-13. Fixed
labels bound what the lap could *test*, not what the net could *learn*
(review D1): the probed box is a function of the auction the context block
already carries, so it adds no information about a fixed target, but a
finite net can still fit the same target better from a better basis —
"uninformative by construction" overstated it, and "a4 ≈ plain in the
matched regime" is an empirical result, not a corollary. What the
transplant does establish is that lap 1 never tested whether stale targets
are the bottleneck; that test is the relabel, so the relabel cannot be
gated on this lap's contrast (change 2 below). The "stop on the residual"
row in "Order and cost" was not a live test in lap 1 for a second reason
too: the residual it read was the inner one (change 3).

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

**The four changes, in order** (revised 2026-10-24 after the review):

1. **`features_v9` for the policy floor, before any lap 2** — the hull
   endpoints replaced by the shape-distribution block. A feature bump under
   transplanted labels, the v8 pipeline
   ([features-v8.md §4](ai-bidder/features-v8.md)): minutes to re-walk and
   splice, ~10 min to train K = 8 on dl02, no relabel. **Before the
   splice** (review D2): `scripts/splice-labels.py` hard-codes the v6
   layout (`VUL = slice(118, 120)`); a v9 inference block of another width
   shifts the vulnerability columns, keys stop matching, and the row
   silently keeps its BBA one-hot — the label mix moves in an experiment
   meant to hold it fixed. The chunk metadata carries `feature_version`, so
   the offsets come from a per-version table, and the run asserts the
   transplanted total at ≈ 424k (the count the 2026-09-26 and lap-1 splices
   both reached; the per-shard `.splice` tail prints it). Gate v9 under ⊤
   readings — non-inferior to v6 on both scorers at **four cells**
   (`VULS="none ns ew both"`, [measurement.md](measurement.md#the-checklist)
   item 8: the contested floor is a doubling treatment, and the runner's
   inherited `none both` is not "every cell"; review S2), read K = 8 vs K = 8
   on paired deals with the **paired IMP intervals deciding** and the TV
   ruler a diagnostic read at the same deals and count (review S1) — then
   re-run A4's four arms under v9 at the same four cells. This tests lap 1's
   mechanism directly and costs hours.
2. **The relabel, piloted rather than gated** (review D1). One shard
   relabelled at HEAD under the probed reading — ≈ 1/20 of the ≈ 170M DD
   tables, ≈ 2.5 h on a free box (10 × `DDSS_THREADS=64`), ≈ 10 h occupied
   ([shared-machine-data-gen.md](shared-machine-data-gen.md) "Sizing the
   solver") — and its EV-argmax labels diffed against the M32 cut: the share
   of labels the reading changes is the test of the stale-target hypothesis
   lap 1 could not run. A share near zero closes the relabel; otherwise the
   full relabel (the deferred 2026-09-26 "megachonk" and v8's lever list in
   one job, ≈ 2 days free, ≈ 8 occupied; detached, state in memory)
   follows, **independently of the sign of a4 − k200 under v9**.
3. **The outer residual before any lap 2** (review D3). `ProbeReport`'s
   `rms` / `floor` / `drifted` all compare the ⊤ harvest with the harvest
   serving its own boxes under one fixed net — the inner reading iteration,
   which a probe `G_θ(R) = c_θ` passes after one step for every θ while
   successive retrains keep moving `c_θ`. The loop's residual is the
   **outer** one: the served map a lap trained under (the walk's probe under
   the old net) against the map the new net re-probes, at the same seed and
   board count, against a floor from two reseeded nets of the same recipe.
   Neither map is saved today: a `--probe-dump` on the probe examples
   (`dump-teacher`, `probe-reading-census`) and a public path to the harvest
   and `residual`, ~100 lines. Lap 1's pair is reproducible — the probe is
   seeded (`0x9B0BE`) and byte-deterministic — from the shipped K = 8 and
   `ab-results/a4/nets/` at 100k boards, minutes each. The inner rms stays
   as a diagnostic; neither it nor the TV ruler decides whether a lap is
   owed.
4. **Branch-2 reads: define the decision, size on the limiting cell, prune
   the pairs** (review S3). Lap 1 read a4 − k200 at ±0.0075 / ±0.0090 on
   plain DD and ±0.0101 on PD (both vul) from 205,056 boards per arm per
   cell; the half-width scales as the inverse square root, so ±0.002 needs
   ≈ 2.9M boards (plain, none), ≈ 4.2M (plain, both), ≈ 5.2M (PD, both) —
   3M was sized on the narrowest interval. "a4 above k200" is now
   pre-registered as **the plain-DD lower bound above 0 in every cell with
   PD non-inferior**; a positive point estimate is not a verdict, and a
   half-width equal to δ is a precision target, not power. Size the
   extension on the PD-both cell and declare it before reading. Runtime
   from lap 1's log (generation ≈ 1 min per arm, the plain diff 1–2 min per
   pair off the shared DD cache, the 16-world single-dummy diff ≈ 20 min per
   pair, so the single-dummy diffs are ≈ 80% of a cell's ≈ 95 min):

   | at 3M boards per arm per cell, four cells | wall at `JOBS=96` |
   | --- | ---: |
   | as scripted (five pairs per cell) | ≈ 4 days |
   | a4 vs plain and a4 vs k200 only | ≈ 1.7 days |
   | the same, the four cells as parallel processes on a free box | under a day |

   The runner takes its cells and its diff pairs as parameters; the cells
   run as separate processes, since the single-dummy harness is one `Solver`
   and does not scale past ~64 threads. More boards cannot flip a4 − plain
   (−0.0101, a clear loss); they are the only way branch 2 can be read at
   all. Lap 1's branch 2 was **unresolvable at 204,800, not refuted**.

**Build record, change 1 (2026-10-24).** `features_v9` built as
[ai-bidder/features-v9.md](ai-bidder/features-v9.md) records: 416 wide, the
65-column shape-distribution block (histogram included — the union's two
modes are the case) for all four seats, the own seat on the whole-deck
prior; the splice keyed per feature version with the transplanted total
asserted; the lap-1 driver checked in as `scripts/feature-retrain.sh`.
Pipeline `NAME=v9 FEATURE_VERSION=9` launched the same day (state in
`/data/public/jdh8/pons/target/v9-pipeline.state`); the four-cell gate
runs `ab-floor-file.sh` at `VULS="none ns ew both"` with the `NETS=` line it
prints.

**Gate read, change 1 (2026-10-24): v9 fails.** `ab-results/v9-floor`,
204,800 bd/arm/vul (640 × 320, whole box), SEED_BASE 1792798343882790139,
three arms on paired deals: plain (shipped K = 8 v6), v9 (K = 8, seeds
9–16), and **k8new** — Plan B's reseeded v6 K = 8, the same seeds, trainer
and validation cut as v9, added as the control that separates the feature
effect from the reseed. IMPs/board, 95% half-widths, δ = 0.002:

| pair | scorer | none | ns | ew | both |
| --- | --- | ---: | ---: | ---: | ---: |
| v9 − plain | plain DD | −0.0067 ±0.0080 | −0.0042 ±0.0087 | **−0.0186 ±0.0094** | −0.0094 ±0.0096 |
| v9 − plain | PD | **−0.0203 ±0.0089** | **−0.0252 ±0.0098** | **−0.0282 ±0.0106** | **−0.0210 ±0.0108** |
| k8new − plain | plain DD | **−0.0116 ±0.0066** | **−0.0104 ±0.0070** | **−0.0133 ±0.0078** | **−0.0116 ±0.0078** |
| k8new − plain | PD | +0.0021 ±0.0072 | +0.0034 ±0.0078 | +0.0027 ±0.0087 | +0.0034 ±0.0087 |
| v9 − k8new | plain DD | +0.0050 ±0.0079 | +0.0066 ±0.0086 | −0.0051 ±0.0093 | +0.0024 ±0.0095 |
| v9 − k8new | PD | **−0.0223 ±0.0088** | **−0.0282 ±0.0097** | **−0.0305 ±0.0105** | **−0.0247 ±0.0106** |

(bold = loss.) The 16-world single-dummy read agrees: v9 − plain SD-PD
−0.012 to −0.027, a loss at all four cells; k8new − plain SD-plain −0.015 to
−0.020, a loss at all four, SD-PD flat. Reach-weighted TV at depth 3 (the
diagnostic, same deals): v9 − plain 0.021 / 0.019 (none / both), k8new −
plain 0.013 / 0.012, v9 − k8new 0.019 / 0.018. v9 moves the floor about
1.5 times as far as a reseed does.

Two readings, one per control:

- **The feature effect (v9 − k8new) is a doubling-priced loss**: plain DD
  flat in every cell, PD −0.022 to −0.031, a loss in every cell. The
  decision table discounts a PD-only *win* as a doubling artifact; a
  PD-only *loss* of this size, at all four cells and in single-dummy too,
  reads as v9 bidding into contracts that the defenders can double. By lane
  (`ab-dump-bucket --by lane`, ew): diffuse over the
  defensive lanes, with one concentrated hole, `1NT (2♥)` and `1NT (2♠)`
  (BBA's Multi-Landy, artificial), −1,338 of the cell's −6,254 PD IMPs on
  360 fired boards (−3 to −4.6 PD per fired board). No over-broad trigger
  to repair. v9 as built (the 65-column histogram) does not ship. Whether
  A4's arms still run under it is open. The probed v9 nets are trained
  (`ab-results/v9-probed/nets/`), and a4 vs a4off inside v9 would still
  test the probe at matched features. The 9-column Gaussian variant
  (`push_shape_gauss`, 192 wide) is the unmeasured alternative named in the
  build record.
- **The reseed costs plain DD at every cell (k8new − plain ≈ −0.012).**
  Plan B read this pair at 19,200 boards as a wash (±0.02 half-widths); at
  204,800 it is a loss in all four cells on plain DD and flat on PD. This
  **revises Plan B's "a retrained K = 8 that differs by TV ≈ 0.017 has moved
  nothing"** and lap 1's reading in "Measured so far": a4off − plain (−0.0111 / −0.0072)
  and a4 − plain (−0.0101 / −0.0071) are the size of the reseed cost, so
  "the retrained net served without the boxes is the mismatch the other
  way" cannot be told apart from "any retrain of this recipe loses about
  0.012 to the shipped ensemble". **From here on, a retrain is read against
  a reseed of the same recipe on paired deals, not against the shipped
  floor.** Why the shipped K = 8 beats its own recipe's reseeds is open
  (winner's curse from the sweep that picked it, the val-cut move in trainer
  `82d4a60d`, or something in how the shipped blobs were produced). Owed
  before any retrain can ship.

**A4's four arms inside v9 (read 2026-10-25): stop again; nothing ships.**
Run because jdh8 asked for it anyway: lap 1's mechanism re-tested at v9
features, with the control a reseed of the candidate's own recipe.
`ab-results/probe-retrain-v9`, 204,800 bd/arm/vul (640 × 320, whole box),
SEED_BASE 1792805017620440317. The runner's new `PLAIN_NETS` re-bases
plain and k200 on the ⊤-trained v9 K = 8 (`ab-results/v9/nets`). So
**plain** = v9 ⊤ nets with no probe, **k200** = the same nets with the
probe, **a4** = the v9-probed nets (`ab-results/v9-probed/nets`, the same
seeds 9–16 and labels) with the probe, and **a4off** = the v9-probed nets
with an empty map. IMPs/board, 95% half-widths, δ = 0.002:

| pair | scorer | none | ns | ew | both |
| --- | --- | ---: | ---: | ---: | ---: |
| a4 − plain | plain DD | −0.0023 ±0.0062 | −0.0002 ±0.0068 | +0.0008 ±0.0073 | −0.0037 ±0.0074 |
| a4 − plain | PD | +0.0009 ±0.0070 | +0.0043 ±0.0076 | +0.0059 ±0.0082 | +0.0006 ±0.0084 |
| a4 − k200 | plain DD | −0.0006 ±0.0062 | −0.0004 ±0.0068 | +0.0035 ±0.0073 | −0.0017 ±0.0074 |
| a4 − k200 | PD | +0.0007 ±0.0070 | +0.0020 ±0.0076 | +0.0066 ±0.0082 | +0.0001 ±0.0084 |
| a4off − plain | plain DD | −0.0024 ±0.0062 | −0.0013 ±0.0068 | +0.0017 ±0.0073 | −0.0026 ±0.0074 |
| a4off − plain | PD | +0.0003 ±0.0070 | +0.0026 ±0.0077 | +0.0062 ±0.0082 | +0.0001 ±0.0084 |
| k200 − plain | plain DD | **−0.0015 ±0.0014** | +0.0002 ±0.0017 | **−0.0027 ±0.0017** | **−0.0020 ±0.0017** |
| k200 − plain | PD | +0.0003 ±0.0015 | *+0.0021 ±0.0019* | −0.0008 ±0.0018 | +0.0005 ±0.0019 |
| a4 − a4off | plain DD | +0.0001 ±0.0014 | +0.0010 ±0.0016 | −0.0010 ±0.0017 | −0.0012 ±0.0016 |
| a4 − a4off | PD | +0.0005 ±0.0015 | +0.0017 ±0.0018 | −0.0005 ±0.0018 | +0.0003 ±0.0018 |

(bold = loss, italic = win; every other cell is inconclusive, except
a4 − a4off at `none` and `ns`, k200 − plain plain DD at `ns`, and its PD at
`none` and `both`, which are non-inferior.) Fired rates: a4 − plain,
a4 − k200 and a4off − plain 5.2–6.1%; k200 − plain 0.32–0.37%; a4 − a4off
0.30–0.35%. Lap 1 fired 2.6–2.9% on the two serving pairs. The 16-world
single-dummy read mostly agrees. Its one new loss is a4 − plain plain SD at
`none` (−0.0073 ±0.0065); the other three cells of that pair also read
negative (−0.0017, −0.0019, −0.0067). It confirms k200 − plain's plain-SD
loss at `none`, `ew` and `both`. a4 − a4off is non-inferior on both SD
scorers at `none` and `ns`, and a plain-SD loss at `both`
(−0.0018 ±0.0017).

Diagnostics at the same deals. Reach-weighted TV at depth 3 (none / ns / ew
/ both): the serving under either net is 0.0006–0.0007 at every cell, about
a ninth of lap 1's 0.0053–0.0059. a4off vs plain (the corpus effect at
matched seeds) is 0.0100–0.0110. The probe's inner report: under the v9 ⊤
net 5,579 keys, 160 drifted, rms 0.050 vs floor 0.449 (0.1×), cliff share
20%; under the v9-probed net 5,658 / 141 / 0.047 vs 0.457 (0.1×) / 19%,
gain 0.00 both. Lap 1 under v6: 1,110 / 952 drifted, rms 0.160 / 0.137
(0.4× / 0.3×).

Four readings:

- **The pre-registration stops it again.** a4 is not non-inferior to its
  matched reseed (plain) on plain DD in any cell. The intervals are
  ±0.006–0.007, three times δ. "a4 above k200" needs a plain-DD lower bound
  above 0 at every cell, and every cell straddles zero. v9 itself failed its
  gate, so nothing in this directory could ship whatever it read.
- **The encoding did what change 1 predicted for sensitivity.** The
  distributional block barely reacts to a κ-shrunk box. The serving moves
  about a ninth as many boards and a ninth of the TV, and the probe's
  drifted keys fall about 7×. That confirms "a box that excludes no hands
  moves no column" as a mechanism.
- **The mismatch survives, smaller, at the same cost per board it touches.**
  k200 − plain still loses on plain DD at `none`, `ew` and `both`, at
  −0.45 to −0.74 IMPs per fired board, with PD flat to a win. That is lap 1's
  doubling-priced shape at an eighth of the footprint.
- **Served matched, the probe is a no-op.** a4 − a4off is flat at all four
  cells and non-inferior at `none` and `ns`. Lap 1's PD win (+0.0075 /
  +0.0060) does not reproduce. Training under the probed boxes (a4off −
  plain, same seeds and labels) moves the policy two thirds of a v6 reseed
  floor (0.010–0.011 vs 0.017 / 0.015 at depth 3, a 19,200-board floor) and
  no IMP interval can see it. The SD lean in a4 − plain is noted, not a
  verdict.

What this settles: under transplanted labels the probe's boxes carry no
value the net can use in either encoding. Under v6 they carried a cost, a
train/serve mismatch, and under v9 most of that cost goes because the net
stops listening. **Disposition: A4 closes inside v9.** The probe stays off
the default path. Change 4's branch-2 extension is not worth running at v9,
a feature version that does not ship. The open questions stay jdh8's: the
Gaussian v9 variant, why the shipped K = 8 beats its reseeds, and the
relabel pilot (change 2). The relabel is now the only test left of whether
a probed reading is worth anything, because fixed labels cannot show it.
Runner defect found on the way, not fixed: every shard re-probes the same
100k boards with the fixed seed. At 640 shards that made each probed arm
≈ 2 h instead of minutes. The fix is to probe once per arm and pass the map
to the shards.

Not worth the compute: the floor sweep's closed axes (width, epochs, lr,
`--dd-weight`; [archive/floor-sweep.md](archive/floor-sweep.md)) and A2's
damping (gain ≈ 0 measured).

## Review corrections (2026-10-24)

Codex reviewed this plan at `adb957bc` (read-only: the TV cancellation was
checked against `ruler-tv.py` and the sample-size arithmetic recomputed; no
run). Each finding was checked against the code the same day and confirmed.
The review itself is not tracked; this table is its record — what the plan
claimed, what is true, and where the plan now says so.

| # | the plan said | what is true | where it now lands |
| --- | --- | --- | --- |
| S1 | movement must clear the TV floor before the IMP is worth reading | `ruler-tv.py` compares aggregate next-call distributions per prefix: two policies choosing different hands at the same frequencies read TV 0 while IMPs move; and Plan B's floor is one K = 8 pair at 19,200 boards, compared against a 204,800-board read of a finite-sample statistic | IMP intervals decide; TV is a diagnostic read at the same deals and count ("What A4 reads against"; the lap-1 read) |
| S2 | "every cell" non-inferior | `ab-probe-retrain.sh` inherits ab-lib's `none both`; the contested floor is a doubling treatment, checklist item 8 wants `none ns ew both` | every v9 and serving confirmation at four cells; the runner parametrized (changes 1, 4) |
| S3 | ≈ 3M boards resolves branch 2 | sized on the narrowest interval; the limiting cell is PD/both (±0.0101 → ≈ 5.2M); "above" was undefined; runtime unstated | decision defined, sized on PD/both, runtime table, pairs pruned (change 4) |
| D1 | fixed labels make the moved columns uninformative "by construction" | fixed labels bound the test, not what a finite net can fit; gating the relabel on a4 − k200 blocks the test of stale targets | one-shard relabel pilot, ungated (change 2); the "why lap 1 could not have moved" paragraph rewritten |
| D2 | the v8 transplant pipeline is reusable unchanged | `VUL = slice(118, 120)` hard-codes the v6 layout; a v9 block shifts it; mismatches silently keep BBA one-hots | version-aware offsets + transplanted-count assertion before the v9 splice (change 1) |
| D3 | residual at a fixed point, no lap 2 owed | `ProbeReport` measures the inner iteration (⊤ harvest vs served harvest under one net); the outer residual (training map vs re-probed map) was never computed | outer residual owed before any lap 2 (change 3); lap-1 read and status corrected |

What the review did not change: the lap-1 disposition (a4 − plain is a loss
at `none` on plain DD at any cell count), the probe staying off the default
path, A3's verdict, and v9 as the first step.

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

### What the analogy misses (2026-10-25)

Poker's chart is a mixed strategy; a bridge bidding policy is not (jdh8).
One hand maps to one call — the floor serves the argmax of the mean logits
(`neural::classify_mean`) and trains on one-hot labels — and a call's
**definition** is deterministic: the authored reading at a book node, the
teacher's preimage at a floor node. The hands a human would mix, the ones
inside two definitions at once, are few, and the harness never pays for
mixing: a common-payoff game against a fixed opponent has an optimal
deterministic joint policy, and concealment is invisible to double dummy.
Three of the four lessons therefore port differently than this plan
assumed, and A0–A4's reads are explained by it:

- **Lesson 2 does not port.** A definition is a set, not a response to a
  response. `Partnership::probe` estimates a fixed set from a finite
  harvest; there was never a best-responder to overshoot, so there was never
  a cycle for λ to damp. A0 measured gain 0 and dropped A2; the gain is zero
  by structure, and κ is a sampling knob (bias against variance) that a
  split-half check like Plan C's can set offline, without an A/B per κ.
- **Lesson 3's ruler is the wrong object.** In poker the per-class
  frequency *is* the strategy. Here the next-call frequency at a prefix is a
  marginal of the deal distribution through a pure policy, and two partitions
  can share it (review S1). The ruler for a pure policy on paired deals is
  the **per-decision disagreement rate**, split where a definition is
  authored into inside it (judgment) and outside it (a misbid); the paired
  IMP diff is its cost. Seed noise is not cost-free here as it is at a mixed
  equilibrium's indifference points: where two nets disagree at a hand, at
  most one is right, which is what makes the reseed gap readable board by
  board ([reseed-gap-handoff.md](reseed-gap-handoff.md) step 2b).
- **Lesson 1's prior is the definition, and only ⊤ axes may take a box.**
  The full-fold scope (`probed`) narrows an authored axis because the policy
  did not use part of the definition, so partner misreads the hand when the
  policy does use it: a judgment frequency read as a boundary. The vacuous
  scope (`probed_vacuous`) is the only principled one, and every arm since
  A3 ran it. Reversible default: the knob stays for the 2026-07-31
  reproduction; no new arm runs the full fold.

Lesson 4 stands and is sharper. The pair's score is piecewise constant in
the boundaries, so no serving-time knob (κ, λ, a temperature) has a gradient
to follow — A3's "slope toward base that never crosses it" — and the only
loop that moves a boundary is a retrain plus an A/B, with partner's model
trained under the boundary that is served. Label staleness is definition
drift, not judgment drift: §5 of the handoff's ⊤ arm measures exactly that.

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
residual says whether another is owed). The residual that rule reads is the
**outer** one — the map the lap trained under against the map its net
re-probes — not `ProbeReport`'s inner rms (review D3, "Next" change 3); and
the A/B that judges the lap reads four cells (review S2).

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
two ensembles of the same recipe. That floor is a diagnostic, not a gate
(corrected 2026-10-24, review S1): TV sees next-call frequencies per prefix,
not which hands make them, so a retrain can move IMPs at TV ≈ 0; and the
floor is one pair read at 19,200 boards, a finite-sample statistic that only
compares at the same deals and count. The paired IMP intervals decide, with
the TV localizing the move by key. The IMP side agrees: the reseeded K = 8 vs the shipped K = 8 is a wash at this
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
| A4 retrain lap — **lap 1 read 2026-10-24, stopped** (build record under § A4) | the deferred retrain as K = 8, ×2 | told: the retrain removes the serving's plain-DD cost but matched-probed still trails matched-⊤ by −0.010 / −0.007 plain; a4 − k200 inside ±0.008 of 0; inner residual 0.160 → 0.137; the net moved 1.1–1.2× the K = 8 floor at mismatched counts. **Caveats (2026-10-24):** transplanted labels bound the test; the outer residual was never computed, so "fixed point" is unproven; branch 2 unresolvable at this size (±0.0075 vs δ 0.002) | "Next: what lap 2 owes" |
| A4 inside v9 — **read 2026-10-25, stopped** ("Next", after the gate read) | 4 arms × 4 cells, whole box, ≈ 18 h | told: the v9 encoding cuts the serving's footprint ≈ 8× and its plain-DD cost with it; matched, the probe is flat (lap 1's PD win gone); a4 vs its own-recipe reseed inside ±0.007 of 0 | relabel pilot |
| review fold-in — **done 2026-10-24** | — | six gate corrections: IMP intervals at four cells decide, TV a diagnostic, sizing on PD/both with the decision defined, relabel piloted, splice offsets version-aware, outer residual owed | v9 |

A0 through A3 are KR1-neutral until A3's cell is read: every knob defaults
off, and the probe map is not on the default path.
