# The trick evaluator — bilans session C, learned

> Status: net built, validated, and **shipped default-on**. The instinct
> floor's game/slam boundary gates price `P(make)` through it, behind
> `set_bilans_floor` (default *on* — `BILANS_FLOOR` is `Cell::new(true)`; the
> knob now only turns it off). `examples/ab-bilans-floor` measured it twice, at
> v1 and again at the v2 weights — see [Against the shipped
> floor](#against-the-shipped-floor). The module is ungated and always builds;
> an earlier revision of this line claimed an `evaluator` Cargo feature that
> never existed, and a later one still called the knob default-off after it had
> shipped.

## What it is

One forward pass answers the question BBA's *bilans* engine answers by
reconstructing hands and counting winners and losers
([bba-floor.md](bba-floor.md) §5, stages 2–3):

> Given my own thirteen cards, and range envelopes on the other three hands,
> how many double-dummy tricks does each declarer take in each strain?

It is an **amortization** of `sample_layouts` + `solve_deals` — the
sample-and-solve loop `ev_all` pays ~1.4 s per decision for — compressed into a
few thousand multiply-adds.

```rust
let inferences = stance.infer(vul, &auction);
let t = trick_estimates(hand, &inferences);
t.p_at_least(Strain::Spades, Relative::Me, 10)   // P(we make 4♠)
```

## Two design commitments

**No auction in the input, ever.** The vector is own-hand summary (24 floats in
shipped v2, 10 in v1 — see the
[featurization sweep](#featurization-sweep)) plus the LHO / partner / RHO range
blocks (30 floats) — no calls, no seat, no vulnerability. The auction enters
*only* through the `Inferences` the book already distilled from it. Widening the
hand block does not touch this commitment; it is a claim about the auction, not
about width.

That is what makes the evaluator **bidding-system agnostic**: (own cards,
ranges) → tricks is physics, true under `american()`, `dutch()`, or any future
book, so corpora generated under different systems pool into one training set
and a book change does not invalidate the weights. The one residual system
dependence is *coverage* — which range shapes actually occur — which is why the
corpus pools two books.

Vulnerability and scoring are deliberately absent: the net is physics, the
caller is economics.

**A distribution, not a point estimate — `(μ, ln σ)` fit by Gaussian NLL.**
Two heads per target, trained by negative log-likelihood (dropping the constant
`½·ln 2π`):

```text
L(t; μ, s) = s + ½·(t − μ)²·exp(−2s),   s = ln σ
```

The two terms are a bargain the net has to strike: `s` charges it for claiming
uncertainty, `exp(−2s)` charges it much more for being confidently wrong. Its
minimiser is exactly the pair we want — the conditional mean and the conditional
standard deviation of the trick count given the information state.

**Why single deals suffice.** Each corpus row is *one* real deal consistent with
its ranges — one unbiased draw from the posterior over hidden hands, with its
exact double-dummy table already cached. Minimising NLL over the population
drives the heads to the conditional moments. The spread emerges from the
population; no state is ever sampled twice. It costs one extra output column and
no extra labels: the net just has to explain the size of its own residual.

**Why the moments and not knots.** `(μ, σ)` is a sufficient statistic, so every
threshold bidding cares about is a closed-form `Φ` away, with no interpolation
and no clamping, and the CDF stays smooth out into the tails. It is also 40
outputs rather than 60.

Those thresholds, worked out as IMP break-evens against a cold alternative:

| decision | non-vulnerable | vulnerable |
|---|---|---|
| partscore → game | 5/11 = **45.5%** | 6/16 = **37.5%** |
| game → small slam | 11/22 = **50.0%** | 13/26 = **50.0%** |
| small slam → grand | 14/24 = **58.3%** (♠♥) … 14/25 = 56.0% (NT, ♣♦) | 17/30 = **56.7%** … 16/29 = 55.2% (♣♦) |

The whole span is **[0.375, 0.583]**, which is what `eval-evaluator`'s `BAND`
reports error inside. Two notes: a small slam is 50% at *both* vulnerabilities
because the slam and game bonuses scale together, and the widely-quoted "a grand
needs 2:1 odds" (67%) is a safety margin for not knowing the small slam is cold,
not the break-even. At matchpoints all of these collapse to 50%.

**Reading a probability off Φ is also strictly more precise than interpolating
one.** A piecewise-linear CDF through `(Q1,¼),(Q2,½),(Q3,¾)` is exact at three
points and approximate everywhere else:

| z | true Φ | 3-knot interpolation | error |
|---|---|---|---|
| ±0.34σ (segment midpoint) | 0.368 | 0.375 | 0.7 pts |
| ±0.67σ (a knot) | 0.250 | 0.250 | 0 |
| ±1.0σ | 0.159 | 0.129 | 2.9 pts |
| ±1.35σ | 0.089 | **0.000 / 1.000** | **8.9 pts** |

Inside the interquartile range that is tolerable — worst case 0.7 points, and
note the segment midpoints fall at 0.375 and 0.625, i.e. exactly on the
vulnerable-game threshold and near the grand-slam one, so the piecewise fit is
at its worst precisely where bidding decides. Even there it is an order of
magnitude below the net's own error.

Outside it the interpolation collapses. The outer segments extrapolate at the
inner slope and clamp, so the fitted CDF hits exactly 0 and 1 at **±1.35σ**,
where the truth is still 8.9%. At this net's σ ≈ 1.9 tricks that declares
anything beyond ~2.5 tricks from μ to be *impossible* — including going down
three in a doubled sacrifice.

That is a problem for the consumer more than for the net. Session D integrates
an expected score, `Σ_k P(T = k)·score(k)`, over the whole distribution; a CDF
with no mass past ±2.5 tricks hides every disaster tail and every windfall, and
biases expected score toward the middle. A floor that cannot see the
doubled-down-three branch is a floor that overbids — the failure mode that
killed the WJ floor A/B. Φ is smooth and nonzero everywhere, so every
`Φ(k+½) − Φ(k−½)` bucket is a real number.

**Why no distillation bias.** The target is double-dummy truth on the actual
deal, not a teacher's opinion. The failure mode that killed the WJ floor A/B —
importing the teacher's overbid along with its skill — structurally cannot
happen here.

**What the Gaussian costs, measured not assumed.** It presumes symmetry and
unbounded support. Trick counts are neither: they are discrete, left-skewed on a
good fit, and hard-bounded at 13. Both the trainer and `eval-evaluator` report
**`below_mean`** — the fraction of realized deals falling under μ, nominally 50%
— so the size of that mismatch is a number in the table rather than a hope. See
[Known ceilings](#known-ceilings).

## Artifacts

| Piece | Path |
|---|---|
| Feature extractor (54 floats) | `src/bidding/features.rs` — `features_eval` |
| Corpus generator | `examples/dump-evaluator` |
| Trainer (candle, off-crate) | `trainer/src/bin/evaluator.rs` |
| In-crate forward pass | `src/bidding/evaluator.rs` (ungated; 352 KB of weights, no deps) |
| Weights + sidecar + parity fixture | `src/bidding/weights/evaluator_v2.{f32,json,fixture.json}` |
| Truth head-to-head | `examples/eval-evaluator` |

**Corpus**: `.pdd` rows stream in with their double-dummy tables already solved
(`/nfs2/jdh8/`, ~94M deals), each deal is self-play-bid under both books, and
every decision node emits `[54 features][20 tricks/13]`. **No solver and no
EPBot run** — the generator is bidding-bound, ~1700 deals/s single-threaded.
The label is `gib::relativized_tricks`: strain-major in GIB order (NT ♠ ♥ ♦ ♣) ×
declarer `[me, lho, partner, rho]`, actor-relative like the input.

The walk is **deal-major** so a contiguous validation tail stays deal-disjoint —
the ~10 rows a board contributes all share one DD label, and a shuffled split
would leak it. Ranges come from `Stance::infer`, never a bare `Context`: the
trie-prefixed reading is what decodes conventional calls off their authoring
rules, and training on the looser reading would be training on the wrong
distribution.

## Results

Corpus: 100k deals from `22.pdd` × `american()` + `dutch()` = **2,005,619 rows**
— except the [featurization sweep](#featurization-sweep), which runs on a 10×
corpus (20,143,935 rows) and whose NLL column therefore compares only within
itself. Held-out: a whole fleet shard (20k deals → 402,092 rows), deal-disjoint by
construction. `MAE` and `RMSE` are the mean head's error against the *realized*
deal, in tricks; `coverage` is the fraction of deals inside `μ ± 0.6745σ`,
nominally 50%. All runs are 150 epochs.

**Two trainers produced the numbers below, and it matters which.** The
architecture ladder was re-run on 2026-07-21 after the training loop was found
never to permute a deal-major corpus (`c019ea5`), at **three seeds per rung**.
Every *other* arm — the shipped net's slices, the ablations, the `--bare` and
quartile comparisons — is still a single `--seed 0` sweep on the old, unshuffled
loop, and is flagged where the defect plausibly moved it.

### Architecture ladder

Three seeds per rung, on the fixed trainer. Medians; the NLL spread column is
max − min across the rung's seeds.

| net | params | val NLL | NLL spread | MAE | RMSE | coverage |
|---|---|---|---|---|---|---|
| linear 40 → 40 | 1,640 | −1.346 | 0.143 | 1.561 | 1.959 | 48.8–53.9% |
| MLP-64 *(shipped)* | 9,384 | −1.463 | 0.149 | 1.499 | 1.893 | 48.7–53.9% |
| MLP-128 | 26,920 | −1.475 | 0.0011 | 1.485 | 1.877 | 48.6% |
| **MLP-256** — the knee | **86,568** | **−1.485** | **0.0006** | **1.472** | **1.863** | **48.4%** |
| MLP-512 *(2 seeds)* | 304,168 | −1.483 | 0.0007 | 1.472 / 1.474 | 1.865 | 47.9–48.2% |

The hidden layer earns its 7,744 extra parameters on every column at once:
0.117 of NLL, 0.062 tricks of MAE, 0.066 of RMSE. Trick-taking is not linear in
(hand summary, range envelopes) — which is the interesting half of the result,
since bilans-style arithmetic *is* roughly that linear class.

**The knee is 256, and width keeps paying much longer than recorded.** MAE falls
0.062 → 0.014 → 0.013 → 0.000 across the four steps. The previous verdict — "64
is where the ladder stops", carried over from the quartile sweep on the strength
of a 0.7% gain at 128 — was an artifact of the unshuffled loop: 128 beats 64 by
0.93% here, and 256 takes another 0.88% on top.

**512 is where it actually stops**, and it stops by overfitting rather than by
saturating. It ties 256 on MAE, loses on NLL at *every* seed (−1.4833/−1.4825
against −1.4851/−1.4850/−1.4845, no overlap), and quadruples the train−val gap
per rung: **0.005 at 128, 0.014 at 256, 0.053 at 512**. Its final-epoch val NLL
is −1.4744, no better than 128 — it is the best-val checkpoint that recovers it
to −1.483, which is that change's first visible save.

A capacity ladder that ends in memorisation rather than in a plateau is a
*corpus* limit, not an architecture limit. 100k deals of ~94M available is the
binding constraint at 256 units, which is what makes streaming the corpus worth
building. The rival reading — that 40 input floats are the ceiling and more deals
would not help either — this ladder cannot exclude.

**Cost is no longer the objection.** MLP-256 is 86,016 MACs against MLP-64's
9,216, but since the forward pass moved to nalgebra (`9023796`) a 256-wide
evaluation costs ~2.3 µs against the ~3.3 µs the 64-wide net cost on the old
scalar kernel. The 9.2× parameter increase is cheaper in wall-clock than what it
replaces.

**Both metrics are reported because they answer different questions.** Squared
error is minimised by the conditional *mean*, absolute error by the conditional
*median*. `μ` is a mean head, so RMSE is the metric it actually optimises;
scoring it on MAE hands a systematic edge to anything aimed at the median
instead. They agree here, so nothing hinges on the choice — but see the skew note
under Calibration for where they come apart.

**Methodological note: the `ln σ` seed lottery is a narrow-net phenomenon.** An
earlier unseeded sweep put linear *ahead* of MLP-64 on NLL — a backwards ladder
conclusion — and this doc recorded the fix as "pin `--seed`", on the reading that
the `μ` head is reproducible run to run while the `ln σ` head is not.

The spread column above shows that is only true at the bottom of the ladder. One
seed in three lands in a wide-σ basin at hidden 0 and 64 (spread 0.143 and 0.149,
bad seeds at coverage 53.9%), and the basin is **gone by 128** (0.0011, then
0.0006 at 256). A bad seed is visible at *epoch 1* and never recovers, so it is an
initialisation basin, not a training instability — and a net wide enough to fit
the corpus does not fall into it.

Two consequences. A single-seed ladder was never able to separate 64 from 128,
because the lottery is an order of magnitude larger than the width effect it was
measuring; use three seeds and compare medians. And MAE is the metric that can
carry a width verdict regardless — it held to ±0.004 across every seed, including
the bad ones, at every rung.

Shuffling the corpus did not fix the basin (spread went 0.072 → 0.153 across
three seeds); width did. `seed_params` still overwrites every parameter after
construction, because candle's CPU device rejects `set_seed`.

### Slices of the shipped net

| slice | NLL | MAE | RMSE | coverage | below μ | targets |
|---|---|---|---|---|---|---|
| american | −1.47128 | 1.489 | 1.880 | 49.5% | ≈48% | 4.04M |
| dutch | −1.45407 | 1.508 | 1.905 | 50.8% | ≈48% | 4.00M |
| constructive | −1.41766 | 1.578 | 1.997 | 50.4% | ≈48% | 2.97M |
| contested | −1.48915 | 1.452 | 1.829 | 50.0% | ≈48% | 5.07M |

**The system slice is the empirical form of the agnosticism claim**: one set of
weights, two books, **1.3% apart** on MAE (1.489 vs 1.508). Neither system is
meaningfully easier, which is what "the net reads ranges, never calls" predicts
— and it means a book change does not invalidate the weights.

The phase slice reads the right way round too: contested auctions are *easier*
(MAE 1.452 vs 1.575), not harder. More calls have been made, so the envelopes
are tighter and there is less posterior spread left. A net that had merely
memorised a prior over deals would show the opposite, or nothing.

NLL is negative because the constant `½·ln 2π` is dropped and σ < 1 in the
corpus's `tricks/13` units, so the `ln σ` term is negative. It is a training
signal, not a readable quantity — MAE and coverage are the interpretable ones,
and only NLL differences within a column mean anything.

**Read `below μ` as ≈48% and no finer.** All ~20 rows a board contributes share
one double-dummy table, so every slice is scored against the same underlying
label values and only the paired μ differs. The four slices agree to five
decimals, far tighter than their nominal binomial error — the effective sample
for a label-shape statistic is the ~20k distinct deals, not the 8M targets.

**About half of that 2-point skew was the trainer, not the Gaussian.** On the
fixed loop `below μ` sits at **49.9–50.1%**, dead nominal, on every seed and
every rung — including the bad ones. The shipped net's ≈48% is a real property
of the shipped weights, so the table stands as measured; but the reading this
doc drew from it — that the symmetry assumption was costing ~2 points — charged
to the parameterization what belonged to fixed batch composition. Symmetry is
holding better than recorded, which weakens rather than strengthens the case for
categorical per-trick heads.

### Ablations

| variant | val NLL | MAE | RMSE | coverage |
|---|---|---|---|---|
| baseline | −1.46271 | 1.499 | 1.893 | 50.1% |
| **ranges blanked** (own hand only) | −1.21246 | 1.923 | 2.370 | 47.2% |

**The ranges buy 0.424 tricks of MAE** (0.477 RMSE) — the headline number for the
whole design. Blanked, the net sees only its own 13 cards and must predict the
unconditional trick distribution; fed the envelopes the inference engine
extracts, it cuts its error by 22%. That gap is the entire value of routing the
auction through `Inferences` rather than feeding it raw, and it is what the
session-D floor will be buying.

Blanking is `Inference::unknown()`'s `[0, 1]` encoding, not zeros — zeros are
out-of-distribution and would measure a different, meaningless thing.

The blanked arm also **under-covers** (47.2% against a nominal 50%): with the
envelopes gone the conditional is a mixture over states the net cannot tell
apart, and it fits a σ too narrow for the spread that actually results. Note that
an earlier *unseeded* blank arm over-covered at 57.9%, and coverage is exactly
the init-sensitive statistic — so read the direction here as provisional and the
magnitude not at all.

#### Featurization sweep

Six arms, 2026-07-22, on the fixed trainer at the width the ladder chose: hidden
256, `--seed 1`, 150 epochs, batch 4096, lr 0.001, **20,143,935 train rows** from
`/nfs2/jdh8/22.pdd` under `american()` + `dutch()` (data `b36bcce`, trainer
`095ac85`). The corpus is dumped **once**, as `--encoding bits`' 79-float research
superset, and each arm zeroes the columns it does not want via `--arm`. Same
rows, same batch order, same parameter count — the only thing that moves is what
the first layer can see. Reported NLL is the restored best-val checkpoint.

| arm | live cols | val NLL | MAE |
|---|---|---|---|
| **`ben`** | **54** | **−1.51051** | **1.443** |
| `full` | 79 | −1.51043 | 1.443 |
| `baseline` | 40 | −1.50284 | 1.451 |
| `baseline-drop-both` | 38 | −1.50232 | 1.452 |
| `baseline-drop-hcp` | 39 | −1.50222 | 1.452 |
| `baseline-drop-upgrade` | 39 | −1.50229 | 1.452 |

**Texture pays 0.008 of NLL and 0.008 tricks of MAE** — `ben` (54) over
`baseline` (40), against the 0.0006 seed spread the architecture ladder measured
at hidden 256, so ~13× noise. Small, and real.

`ben` swaps each suit's `(len, suit_hcp)` for **six columns: one bit each for
A/K/Q/J/T, plus a spot count** (ranks 2–9, hence the divisor 8), and drops both
globals. That is strictly finer at *no* representational cost, because the six
recover the two exactly and linearly — `len` is the spot count plus the honours
flagged, `suit_hcp` is `4/3/2/1` against the top four bits, and global `hcp` is
that same dot product over all sixteen. The first layer can rebuild `baseline` in
four weights per suit if that is what it wants; what it gains is the distinction
`baseline` throws away. **AJx and KQx are both 5 HCP in three cards, and now read
differently.**

Two sub-negatives fall out of the same table and are worth exactly as much as the
win. **`full` (79) ties `ben` (54) to 8e-5** — well inside the 0.0006 seed spread
— so the 25 columns `full` adds buy nothing, and two of them were named
hypotheses rather than filler. The range **widths** (`max − min` carried beside
every `(min, max)` pair, on the theory that a net reads a width more cheaply than
it learns to subtract) measure **0.000**. And `suit_hcp` kept as a **skip
connection** alongside the honour bits — the four-weight sum handed over rather
than learned — measures **0.000** as well. Both are dead as authored, and neither
is a reason to widen the dump.

**The two globals are free to delete**, which *refutes* their standing as the
hand block's last deletion candidate — there was nothing there to lose. `ben`
drops `hcp/40` and `upgrade/2` outright and is still champion, and the three
`baseline-drop-*` arms land within **0.0006** of `baseline`, the seed spread
exactly. The mechanism is that neither was ever new information: `hcp` is the
four `suit_hcp` columns summed, and `upgrade` is the legacy shape term from when
`point_count` was `raw_hcp + upgrade`, a scale `RuleOfNFloored` has since
replaced.

**This retires the carried-over one-hot verdict, and the hedge that came with it
was right about the mechanism.** The quartile ladder's **52-bit one-hot hand**
scored slightly *worse* than the 10-float summary despite strictly more
information and 2,688 more parameters, and this doc drew "texture does not pay"
from it while warning that the arm proved too much: it deleted the summary
*wholesale*, so it had to rediscover suit lengths and HCP arithmetic from raw
card bits, and said nothing about **adding** texture to a summary that stays. The
sweep above is exactly that missing experiment, and it splits the difference.
Texture **does** pay — but only 0.008, and only as honour bits beside a retained
summary, never as 52 raw card bits. The one-hot path stays closed; the
honour-bit path is open, and it is the champion.

**Two caveats, and the first is a gap rather than a result.** The **cross-GPU
control was cancelled**, so the hardware confound is *inferred*, not measured:
arms ran across lanes, and two cross-lane pairings agreed to **~1e-4**, which is
the only bound there is. Against the **0.008** headline that is ~80× of margin
and the verdict holds; but `full` ≡ `ben` at 8e-5 sits *inside* the inferred
bound, so that tie can be read as a tie and nothing finer. Second, every arm is a
**single seed**, and the 0.0006 spread it is judged against was measured on the
architecture ladder — a different corpus, ten times smaller. For the same reason
the absolute NLL here is not comparable to any other table in this doc; only
differences *within* this one mean anything.

#### Hidden-seat axis survey

The keycard track's two-factor pricing — **realizable gain ≈ ceiling (oracle
slice-MAE reduction) × reach (fraction of the slice the auction actually
discloses)** — generalized from one axis to a survey. Keycards cleared the
ceiling (−1.257 tricks on the slam slice) and died on reach (0.54% ⟹ ≈0.007
tricks); the survey prices the other candidate axes the same way before any
of them is built.

Harness (all off-crate; shipped crate byte-identical):

- `dump-evaluator --oracle-all` extends the `bits` superset to **147**
  features: the 8 keycard columns verbatim, then per-axis truth for all three
  hidden seats [LHO, partner, RHO] — quality (per-suit `suit_hcp/10`, cols
  87..99), shortness (`len ≤ 1` bits, 99..111), controls (per-suit ace+king
  bits, 111..135), stopper (A/Kx/Qxx/Jxxx bits, 135..147). Per-suit truth,
  never "the shown suit" — a shown-suit collapse would manufacture the fit
  indicator the projection design forbids. The 87-wide `--oracle` corpus is
  retired by the width bump; same seed regenerates the same auctions.
- Trainer arms `ben-oracle-quality|shortness|controls|stopper` (masks keep
  one tail block each; `ARM_FEATURES` 87→147), plus two new eval slices where
  the ceilings are read: **suit-game** (suit-strain targets, truth ≥ 10
  tricks — shortness's home turf) and **nt-contested** (NT targets on
  contested rows, truth ≥ 9 — quality's and stopper's).
- `probe-keycard-reach` now measures all five axes in one walk. Per axis, a
  *book* latch (the winning rule's prose disclosed the axis: `"HCP in"`/
  `"of the top honors in"`, `"≤1 ♣/♦/♥/♠"`, `"control"`, `"stopper in"`) and a
  *structural* latch (Ogust answer position, strong 2♣ opening, 2NT/3NT over
  their shown suit) — except shortness, whose second kind is the live
  **envelope** (`Stance::infer` already caps a suit at ≤1), i.e. the portion
  the range features already realize. Survey reach is the **disclosed-seat
  fraction** of the 3 hidden seats. Scripted-auction tests pin every prose
  needle so a rewording fails the build instead of silently reading zero.

A 5k-deal smoke run (seed 1) already shows the shape of the answer: controls
book-reach is exactly 0 (nothing in the American book is authored on
controls), and shortness prose-disclosure (≈1.8% of suit-game seat-cells)
exceeds envelope-realized shortness (≈0.16%) by ~12× — the projection
OR-union gap, the same defect family as the 2/1 reading erasure. The full
survey (ceilings + 500k-deal reach) has not run yet; recipe:

```sh
dump-evaluator --deals /nfs2/jdh8/22.pdd --count 500000 --seed 1 \
    --encoding bits --oracle-all --out <corpus>       # ~15 min, 6.7 GB
probe-keycard-reach --deals /nfs2/jdh8/22.pdd --count 500000 --seed 1
# reach first; then arms in descending reach order, matching the Phase-3
# sidecar's epochs/seed; `--arm ben` must reproduce slam-MAE ≈ 2.664 first
```

Decision rule, per axis: ceiling × book-reach ≥ ~0.05 tricks on its slice ⟹
worth a Phase-4-style projection build; below ⟹ recorded and closed, like
keycards. Ceilings overlap (quality partially implies stopper and controls),
so the products rank candidates — they do not sum.

### Against the truth it replaces

`examples/eval-evaluator`, held-out shard (`shard-1010741…`, `--skip 20000`),
1000 boards, replay sampler at 96 layouts/node — predicted moments vs the
sample-and-solve loop, at the same nodes. Run at v1 and again at v2, same
shard/skip/boards/layouts, so the columns compare directly (v1 priced 10,242
nodes, v2 10,275 — which nodes starve for samples shifts slightly with the net):

| quantity | v1 | v2 |
|---|---|---|
| mean MAE vs sampled mean | 0.497 tricks | **0.417** tricks |
| sd MAE vs sampled sd | 0.214 tricks | **0.188** tricks |
| signed spread (predicted − sampled sd) | +0.087 (1.872 vs 1.785) | **−0.038** (1.748 vs 1.786) |
| sampled mass below μ | 49.9% | 49.8% |
| P(make) MAE, all levels | 0.0434 | **0.0375** |
| P(make) MAE, decision band (35–60%) | 0.1127 (contested 0.1285) | **0.0954** (contested 0.1092) |
| — sampler's own binomial noise floor | 0.0382 | 0.0387 |
| **— net's own error, deconvolved** | **0.1060** (contested 0.1227) | **0.0872** (contested 0.1021) |

**Read the last row as the verdict, and v2 moved it the right way** — 10.6 → 8.7
points of P(make) error inside the decision band, the sharpening the
[featurization sweep](#featurization-sweep) predicted and the A/B then priced in
IMPs. But 8.7 points is still *just over* the 8-point gap between the NV and vul
game thresholds (45.5% vs 37.5%), so the qualitative conclusion is unchanged: the
evaluator is a usable prior for where a hand sits, and it still cannot by itself
decide a vulnerability-marginal game. That remains a statement about session D's
design, not a defect — treat the net as a fast prior and reserve sampling for
boards near a threshold. The margin is now thin enough that a further sharpening
could cross it; the A/B, not this number, is the ship gate either way.

**Deconvolve in quadrature, not linearly.** The measured band MAE is the net's
error against a *noisy estimate* of truth. Net error and sampling noise are
independent, so their squares add: v2's √(0.0954² − 0.0387²) = 0.0872. An earlier
revision of the harness subtracted the two linearly (and reported 0.0745 at v1) —
understating the net by ~45%. Both terms are MAEs of roughly Gaussian errors
and MAE = √(2/π)·σ for a Gaussian, so the √(2/π) factors cancel and the MAEs
compose in quadrature exactly as the σ's do. The harness now prints the
deconvolved figure directly.

**The one row that got worse is the safe one.** v1 ran **+0.087 wide** —
over-dispersed, the safe direction for a consumer that integrates a CDF, since
it under-claims confidence. v2 is **−0.038 wide**, a hair the other way. Two
things keep that from being a regression: the magnitude is half v1's and inside
the earlier probe's ~7% shot noise, and the comparison is against the *replay*
sampler, which [the `--bare` arm](#the---bare-arm-and-why-it-did-not-settle-what-it-was-meant-to)
showed is itself over-tight (bare's sampled sd is 1.904). The net at 1.748 is the
only one of the three anchored to double-dummy truth; that it now sits between
the two samplers rather than above both is consistent with a sharper fit, not an
overconfident one. **49.8% below μ** confirms the Gaussian's symmetry assumption
still holds at v2 — the cheap failure the parameterization could have had, and
did not.

A 40-board probe run earlier held every result's shape but was optimistic by
~7% on each error column — worth remembering before trusting a small slice.

The band is selected on the *predicted* probability, not the sampled one:
conditioning on a noisy empirical estimate landing in 35–60% would drag in
contracts that got there by sampling error and inflate the reported gap.

### The `--bare` arm, and why it did not settle what it was meant to

*All figures in this subsection are the **v1** net; the arm was not re-run at
v2. Its methodological findings — replay is over-tight, the fork needs an
unbiased denominator — are version-independent and stand. And the fork it left
open ("train harder" vs "the hand encoding is the ceiling") was since answered
from the other side: the [featurization sweep](#featurization-sweep) showed the
`(len, suit_hcp)` encoding **was** leaving ~0.008 NLL on the table, and
recovering it is what v2 ships.*

Same 1000 boards through range-only `sample_layouts` instead of rule-replay.
Bare draws from the projected envelope — exactly the information the net
receives — while replay draws from the tighter, rule-consistent set:

| quantity | replay | bare |
|---|---|---|
| mean MAE | 0.497 | 0.488 |
| sd MAE | 0.214 | 0.181 |
| signed spread (predicted − sampled sd) | +0.087 (1.872 vs 1.785) | **−0.031** (1.873 vs 1.904) |
| P(make) MAE, band | 0.1127 | 0.0987 |
| noise floor | 0.0382 | 0.0389 |
| **net's own error, deconvolved** | **0.1060** (contested 0.1227) | **0.0907** (contested 0.1010) |

The arm was launched to split 0.1060 into the net's *learning* error and the
price of the range representation — the fork between "train harder" and "40
floats is the ceiling". **It does not, and the reason is worth more than the
answer would have been.**

The prediction recorded beforehand was that the net would look *narrower* than
bare's truth and *worse* against it. The sign flipped exactly as called: the
sampled sd rises past the net's, turning +0.087 into −0.031. The magnitude went
the other way — the net scores **better** against bare (0.0907) than against
replay (0.1060).

The stated criterion for that outcome was "the net learned the envelope rather
than the reality". That conclusion is not safe, because **the decomposition
assumed replay was truth, and it is not.** The net was fit to neither sampler:
its labels are DD tables on real deals. All three disagree about spread, and
the net's 1.872 lands *between* the two samplers (replay 1.785, bare 1.904),
much nearer bare. The net is the only one of the three anchored to ground
truth, so the cleanest reading is that **`set_rule_accept` replay is over-tight**
— rejecting layouts real bidding does produce — and bare is mildly over-loose.

Consequences, in order of how much they should change behaviour:

1. **The fork stays open.** Resolving it needs a denominator that is not a
   biased sampler — e.g. scoring against held-out *real* deals grouped by
   near-identical range envelopes, where the empirical spread is the true
   posterior spread by construction.
2. **Prefer the replay-arm 0.1060 as the number of record** anyway. It is the
   conservative end, and the rule-consistent distribution is the one session D
   actually meets at a node.
3. **The sampler bias is a finding in its own right**, independent of this net:
   anything else calibrated against `sample_layouts_replay` inherits a spread
   that appears to run ~0.09 tricks tight.

### Comparison with the quartile parameterization

This net replaced a three-knot quantile version (Q1/Q2/Q3 by pinball loss) at
the same width, corpus, and hyperparameters, so the shape-independent metrics
compare directly:

| | quartiles (pinball) | Gaussian (NLL) |
|---|---|---|
| params | 10,684 | **9,384** |
| MAE vs realized deal | **1.494** tricks | 1.498 tricks |
| central-50% coverage | 49.7% | **50.2%** |
| CDF at a threshold | interpolate 3 knots, clamp | closed-form `Φ` |

A wash on accuracy — 0.004 tricks, 0.3% relative, in favour of the quartiles and
far inside anything that matters — a shade better on calibration, and 12% fewer
parameters. The two parameterizations extract essentially the same information;
the Gaussian delivers it as a sufficient statistic with a smooth CDF, which is
what session D has to integrate against a score table.

The loss values themselves are *not* comparable — pinball and NLL are different
scales.

### Against the shipped floor

Calibration is not the ship gate; IMPs are. `examples/ab-bilans-floor` plays the
floor with the gates priced by the net against the same floor pricing them by
point arithmetic. It has run twice, at 200k boards per vulnerability and the
**same seed** (1784589590), once at the v1 weights and again at v2:

| vulnerability | scorer | v1 | **v2** |
|---|---|---|---|
| none | plain DD | +0.036 [+0.030, +0.042] | **+0.068** [+0.061, +0.076] |
| none | perfect defense | +0.009 [+0.002, +0.016] | **+0.048** [+0.040, +0.056] |
| both | plain DD | +0.065 [+0.057, +0.073] | **+0.110** [+0.100, +0.120] |
| both | perfect defense | +0.013 [+0.003, +0.022] | **+0.070** [+0.059, +0.081] |

IMPs per board, net-positive meaning the net-priced gates win; 95% CIs. Both
scorers win at both vulnerabilities in both runs, so the knob ships default-on
under the decision table with no doubling-artifact caveat.

**The re-run is paired, and that is the point.** The A/B's arms are knob-on vs
knob-off, and the knob-*off* arm never touches the net — it is a fixed
reference. Re-running the recorded seed rather than a fresh one therefore makes
the change in margin the *weights'* effect on an identical deal set. (This is
the one place [seed hygiene](../measurement.md) is deliberately set aside: it is
a paired re-run of one experiment with one variable moved, not a new
experiment.) Read that way, v2 roughly doubles the plain-DD margin and
**quintuples** the perfect-defense one — every interval disjoint from v1's.

That the perfect-defense column gains most is the expected shape: PD punishes
overbidding hardest, so a sharper `sd` — which is what the featurization sweep
bought — shows up first as *fewer* contracts bid past their break-even.

## Known ceilings

- **The Gaussian is symmetric; tricks are not.** On a good fit the trick
  distribution is left-skewed and walled at 13, which puts the true mean below
  the true median and leaves the fitted normal spilling probability past 13.
  `below_mean` measures how far off symmetry actually is; if a consumer ever
  pays IMPs for it, the upgrade path is categorical per-trick heads (an exact
  discrete CDF, ~280 outputs). **Least binding of the three, and less binding
  than recorded** — on the fixed trainer `below μ` is 49.9–50.1%, so most of the
  skew this bullet was sized from was batching.
- **Texture is no longer invisible — it is measured, and it is small.** The old
  v1 hand block carried honour *location* (which suit) but not texture: AJx and
  KQx read alike, spot cards not at all, and the net absorbed that as spread
  rather than predicting through it. The
  [featurization sweep](#featurization-sweep) prices exactly that at **0.008 NLL
  / 0.008 tricks of MAE**, recovered by replacing each suit's `(len, suit_hcp)`
  with A/K/Q/J/T bits plus a spot count. That is the champion `ben`
  featurization, and **v2 ships it**: `features_eval` is now 54 floats and the
  in-crate weights are the `ben` arm gathered to 54 columns. What stays invisible
  even after the swap is spot-card *identity* below the ten: the count collapses
  ranks 2–9, so **T9x and T2x still read alike**, and nothing measures what that
  last slice is worth. The `--encoding onehot` arm this bullet used to name is
  the wrong instrument — the dumper emits the 79-float superset at
  `--encoding bits` and the trainer selects the arm with `--arm`.
- **Range envelopes are sound but loose.** `Constraint::project` guarantees every
  consistent hand falls inside the envelope, and opaque predicates project to
  `unknown()`. The net learns how much a loose envelope really pins down — that
  *is* the spread.
- **Wide envelopes bias the gates upward — the competitive-auction note.**
  When the bilans floor shipped default-on (2026-07-21) it changed six floor
  positions, and *every* competitive one moved up a level: a limit raise over a
  jump overcall 3♥→4♥, a game opposite a 3-level overcall Pass→4♥, a 21-count
  opposite an overcall 4♠→6♠.

  There is a mechanism that predicts exactly this, and it is worth stating
  because it is a property of the *gate*, not of the net's accuracy. A gate
  fires on `P(≥ tricks) ≥ break-even`, and that probability is read off the
  fitted Gaussian `(μ, σ)`. When μ sits **below** the trick target — the
  borderline case every gate lives on — a **larger σ raises** `P(≥ tricks)`,
  because more of the bell spills past the target. Wide inference envelopes
  produce large σ. Competitive auctions have the widest envelopes there are
  (partner's overcall reads as "8+ points, 5+ suit"). So the looser the
  reading, the more the gate bids.

  Note this is not the net being wrong: σ is *correctly* large there, and the
  A/B is net-positive at both vulnerabilities. It is that a break-even
  comparison on a symmetric distribution converts uncertainty into aggression
  in one direction only. Whether that is right is an empirical question the
  aggregate answers yes to today; the forensic — bucket a divergent set by
  envelope width and check whether the losses concentrate in the wide bucket —
  has not been run.

- **`ln σ` is hard-clamped** to `[−5, 0]` in `tricks/13` units (σ ∈ [0.087, 13]
  tricks) in both trainer and serving, to stop the classic heteroscedastic
  collapse where σ → 0 on easy rows and the loss runs to −∞. A head parked on the
  boundary gets no gradient back; a softplus parameterization is the upgrade if
  that ever bites. **It has not bitten, and the case for pre-emptively writing it
  is now gone**: the wide-σ runs that made the clamp look like the suspect were
  the narrow-net initialisation basin, and that basin does not exist at 128 units
  or above.
