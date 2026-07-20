# The trick evaluator — bilans session C, learned

> Status: net built and validated; **consumed** by the instinct floor's
> game/slam boundary gates behind `set_bilans_floor` (session D, default off —
> the knob's A/B, `examples/ab-bilans-floor`, decides whether it ships
> default-on). The module is ungated and always builds; an earlier revision of
> this line claimed an `evaluator` Cargo feature that never existed.

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

**No auction in the input, ever.** The vector is own-hand summary (10 floats)
plus the LHO / partner / RHO range blocks (30 floats) — no calls, no seat, no
vulnerability. The auction enters *only* through the `Inferences` the book
already distilled from it.

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
| Feature extractor (40 floats) | `src/bidding/features.rs` — `features_eval` |
| Corpus generator | `examples/dump-evaluator` |
| Trainer (candle, off-crate) | `trainer/src/bin/evaluator.rs` |
| In-crate forward pass | `src/bidding/evaluator.rs` (ungated; 37 KB of weights, no deps) |
| Weights + sidecar + parity fixture | `src/bidding/weights/evaluator_v1.{f32,json,fixture.json}` |
| Truth head-to-head | `examples/eval-evaluator` |

**Corpus**: `.pdd` rows stream in with their double-dummy tables already solved
(`/nfs2/jdh8/`, ~94M deals), each deal is self-play-bid under both books, and
every decision node emits `[40 features][20 tricks/13]`. **No solver and no
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

Corpus: 100k deals from `22.pdd` × `american()` + `dutch()` = **2,005,619 rows**.
Held-out: a whole fleet shard (20k deals → 402,092 rows), deal-disjoint by
construction. `MAE` and `RMSE` are the mean head's error against the *realized*
deal, in tricks; `coverage` is the fraction of deals inside `μ ± 0.6745σ`,
nominally 50%. Every arm below is one sweep at `--seed 0`, 150 epochs.

### Architecture ladder

| net | params | val NLL | MAE | RMSE | coverage | below μ |
|---|---|---|---|---|---|---|
| linear 40 → 40 | 1,640 | −1.34398 | 1.564 | 1.963 | 51.1% | ≈49% |
| **shipped: MLP-64** | **9,384** | **−1.46271** | **1.499** | **1.893** | **50.1%** | **≈48%** |

The hidden layer earns its 7,744 extra parameters on every column at once:
0.119 of NLL, 0.065 tricks of MAE, 0.070 of RMSE, and coverage a point closer to
nominal. Trick-taking is not linear in (hand summary, range envelopes) — which
is the interesting half of the result, since bilans-style arithmetic *is* roughly
that linear class.

**Both metrics are reported because they answer different questions.** Squared
error is minimised by the conditional *mean*, absolute error by the conditional
*median*. `μ` is a mean head, so RMSE is the metric it actually optimises;
scoring it on MAE hands a systematic edge to anything aimed at the median
instead. They agree here, so nothing hinges on the choice — but see the skew note
under Calibration for where they come apart.

**Methodological note: fix the seed.** An earlier unseeded sweep put linear
*ahead* of MLP-64 on NLL — a backwards ladder conclusion. Two runs at identical
settings differed by 0.075 NLL and 2.7 points of coverage, while MAE moved only
0.004. The `μ` head is reproducible run to run; the `ln σ` head is not, so any
conclusion resting on NLL or coverage needs `--seed` pinned. `seed_params` in the
trainer overwrites every parameter after construction (candle's CPU device
rejects `set_seed`).

Carried over from the quartile parameterization this replaced (not re-measured
under NLL, because it is a fact about the function class rather than the loss):
widening to 128 hidden units bought **0.7%** of loss for 2.8× the parameters,
and training the 64 longer closed that gap anyway. So **64 is where the ladder
stops**.

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

Also carried over from the quartile ladder: a **52-bit one-hot hand** (texture)
scored slightly *worse* than the 10-float summary despite strictly more
information and 2,688 more parameters. At this data scale the summary's
inductive bias — HCP arithmetic and suit lengths handed over for free — beats
making the net rediscover them. The 40-float vector stands; the one-hot path is
closed unless a much larger corpus reopens it.

### Against the truth it replaces

`examples/eval-evaluator`, held-out shard (`shard-1010741…`, `--skip 20000`),
1000 boards / 10,242 nodes, replay sampler at 96 layouts/node — predicted
moments vs the sample-and-solve loop, at the same nodes:

| quantity | value |
|---|---|
| mean MAE vs sampled mean | 0.497 tricks |
| sd MAE vs sampled sd | 0.214 tricks |
| signed spread (predicted − sampled sd) | +0.087 (1.872 vs 1.785) |
| sampled mass below μ | 49.9% |
| P(make) MAE, all levels | 0.0434 |
| P(make) MAE, decision band (35–60%) | 0.1127 (contested 0.1285) |
| — sampler's own binomial noise floor | 0.0382 |
| **— net's own error, deconvolved** | **0.1060** (contested 0.1227) |

**Read the last row as the verdict.** ~10.6 points of P(make) error inside the
decision band is *larger* than the 8-point gap between the NV and vul game
thresholds (45.5% vs 37.5%). So the evaluator is a usable prior for where a
hand sits, but it cannot by itself decide a vulnerability-marginal game. That
is a statement about session D's design, not a defect: session D should treat
the net as a fast prior and reserve sampling for boards near a threshold,
rather than replacing sample-and-solve outright. Whether the residual costs
IMPs is an A/B question, which session D owes regardless.

**Deconvolve in quadrature, not linearly.** The measured 0.1127 is the net's
error against a *noisy estimate* of truth. Net error and sampling noise are
independent, so their squares add: √(0.1127² − 0.0382²) = 0.1060. An earlier
revision of the harness subtracted the two linearly and reported 0.0745 —
understating the net by ~45%. Both terms are MAEs of roughly Gaussian errors
and MAE = √(2/π)·σ for a Gaussian, so the √(2/π) factors cancel and the MAEs
compose in quadrature exactly as the σ's do.

The two calibration results survive at full n. **49.9% below μ** says the
Gaussian's symmetry assumption is genuinely met here — the one place the
parameterization could have failed cheaply, and it did not. **+0.087 wide**
means the net errs toward over-dispersion, the safe direction for a consumer
that integrates a CDF: it under-claims confidence rather than over-claiming.

A 40-board probe run earlier held every result's shape but was optimistic by
~7% on each error column — worth remembering before trusting a small slice.

The band is selected on the *predicted* probability, not the sampled one:
conditioning on a noisy empirical estimate landing in 35–60% would drag in
contracts that got there by sampling error and inflate the reported gap.

### The `--bare` arm, and why it did not settle what it was meant to

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

## Known ceilings

- **The Gaussian is symmetric; tricks are not.** On a good fit the trick
  distribution is left-skewed and walled at 13, which puts the true mean below
  the true median and leaves the fitted normal spilling probability past 13.
  `below_mean` measures how far off symmetry actually is; if a consumer ever
  pays IMPs for it, the upgrade path is categorical per-trick heads (an exact
  discrete CDF, ~280 outputs).
- **Texture is invisible.** The hand block carries honour *location* (which
  suit) but not texture — AJx and KQx read alike, spot cards not at all. The net
  absorbs that as spread rather than predicting through it. `--encoding onehot`
  measures what 52 card bits would buy.
- **Range envelopes are sound but loose.** `Constraint::project` guarantees every
  consistent hand falls inside the envelope, and opaque predicates project to
  `unknown()`. The net learns how much a loose envelope really pins down — that
  *is* the spread.
- **`ln σ` is hard-clamped** to `[−5, 0]` in `tricks/13` units (σ ∈ [0.087, 13]
  tricks) in both trainer and serving, to stop the classic heteroscedastic
  collapse where σ → 0 on easy rows and the loss runs to −∞. A head parked on the
  boundary gets no gradient back; a softplus parameterization is the upgrade if
  that ever bites.
