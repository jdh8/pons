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

Blanking is `Envelope::unknown()`'s `[0, 1]` encoding, not zeros — zeros are
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

#### Auction-input ablation (2026-07-27): the gate PASSES

The standing commitment — *no auction in the input, ever* — was put to the NLL
gate, expecting a refusal, and instead measured the largest single featurization
delta in this document. Two arms, fresh corpus: 500k deals of `22.pdd` under
`american()` + `dutch()`, `--encoding bits --auction --dnf --seed 1` (data
`77091ca`, trainer `b1078aa`), 10,181,930 rows, val = the deal-disjoint 10%
tail. The dump appends a 160-float auction block to the 79-float superset: for
each of the last 4 calls, 7 bid-encoding floats + 3 pass/X/XX bits + the 21
WBF-tag multi-hot + 1 alerted bit + an 8-bucket FNV-1a hash of the alert name.
Family columns are omitted — both pooled systems are `Family::NATURAL`, so they
are dead by construction, and that *is* the "families" verdict.

| arm | live cols | val NLL | MAE (tricks) |
|---|---|---|---|
| `ben` | 54 | −1.51059 | 1.441 |
| **`ben-auction`** | **214** | **−1.55253** | **1.388** |

**Δ = 0.0419 NLL and 0.053 tricks of MAE** — ~70× the 0.0006 seed spread, ~5×
the texture win that shipped v2. The gain concentrates exactly where the
⊤-census says the readings starve: contested 0.050, nt-contested 0.084,
suit-game 0.066, **slam 0.105** (slam MAE 2.62 → 2.46), against constructive's
0.029. Both systems gain (american 0.038, dutch 0.046), so at least across this
pool the win is not one system's memorized quirks. Absolute NLLs are not
comparable to the 2026-07-22 table (different corpus); only the within-table
difference means anything.

Read this jointly with the blind negative control: the reading channel as a
whole is worth 0.65–1.27 IMPs/board, and this ablation shows the *hulls are not
carrying all of it* — the raw calls hold signal that the projected envelopes
drop, which is the starvation diagnosis stated as an NLL number. It is a
**ceiling claim, not a shipping decision**: `features_v2` fed the same last-4 ×
21-tag block to the *policy* net, won distillation CE, and measured IMP parity
(−0.016, CI [−0.039, +0.007]) — an NLL win does not automatically survive the
A/B. And the price is real: auction-coupled weights surrender knob-invariance
and cross-system pooling, the properties the hull interface was chosen for.
Per the gate protocol the next step is a separate, user-approved campaign
(crate featurization, serving plumbing, the A/B) — nothing ships from this
table. The sampled-projection track remains the fix that captures the same
signal *inside* the hull interface; this number is now also that track's
target: readings good enough to close a 0.042 NLL gap.

**Same-day follow-ups: what inside the block pays, and whether canonical
hulls close the gap.** Two questions, three more arms. First, are the tag and
alert columns doing anything the hulls don't already do (the hulls *are* the
disclosure, compressed)? `ben-calls` keeps only the 10 call-identity columns
per slot — bid encoding plus pass/X/XX — and zeroes the 21 tags, the alerted
bit, and the hash. Second, is the auction win really just *slack* hulls —
would canonicalized readings recover it? `dump-evaluator --closed-hulls`
re-extracts the same corpus with both box closures folded (`narrow_to_sum` +
`narrow_to_upgrade`) around feature extraction only: the auctions are bid
knob-off, and the `.tags` stream is **byte-identical** between the corpora,
so rows and targets align exactly. The closures bite hard as geometry —
**72.6% of rows** have at least one tightened hull float, mean move 0.36 on
touched floats, every move a pure tightening (no min down, no max up) — the
`5+/5+ ⇒ each ≤ 8` arithmetic, applied everywhere.

| arm | hulls | auction block | val NLL | Δ vs `ben` |
|---|---|---|---|---|
| `ben` | as-written | — | −1.51059 | — |
| `ben-closed` | canonical | — | −1.51220 | 0.0016 |
| `ben-calls` | as-written | calls only | −1.54826 | 0.0377 |
| `ben-auction` | as-written | calls+tags+alerts | −1.55253 | 0.0419 |
| `ben-auction-closed` | canonical | calls+tags+alerts | −1.55232 | 0.0417 |

Three verdicts. **Bare calls carry 90% of the block** (0.0377 of 0.0419);
tags + alerts add 0.0043 — real (~7× seed noise) but a sliver, so the hulls
do work as disambiguators for most of what the tags spell out. **Hull
canonicalization is REFUSED at the gate**: 0.0016, under the 0.002 line,
despite rewriting nearly three-quarters of all rows. The net trained on
as-written hulls had already learned the system's writing style — slack that
is *consistent* is slack a net can compensate for, so exact-but-loose and
exact-but-tight endpoints carry nearly the same information. And the closure
**does not eat the auction margin**: on canonical hulls the block is still
worth 0.0401, and with the block present the closure's own contribution
rounds to zero (−1.55253 vs −1.55232). What the raw calls add is
*information the hulls never held* — which call was actually made — not
sharper arithmetic on what they do hold. Canonicalization remains the right
hygiene for the symbolic consumers (sampler acceptance, disclosure, any
arithmetic gate reads endpoints, not styles), but for the net the failure
mode is missing mass, not messy bounds — which is one more arrow pointing at
coverage, i.e. sampled projection, rather than at rewriting the readings we
already have.

**Served and SHIPPED default-on (same day): evaluator v3, calls-only.** The
campaign the gate opened ran to completion in one day.  `features_eval_v3` =
the 54-float hull vector + the last four bare call identities (94 floats;
tags/alerts left on the table per `ben-calls`), dumped verbatim by
`dump-evaluator --encoding eval3` so corpus and serving agree by
construction; the native twin `evaluator_v3_dnf` hit −1.54872 val NLL,
reproducing the masked `ben-calls` arm (−1.54826); serving rides
`trick_estimates_with_auction` behind `evaluator::set_eval_auction`, honoured
only in the DNF regime the twin was trained on.  The A/B (204,800
boards/arm/vul, `SEED_BASE` 1785138816, vs BBA 2/1) came back `win | win` on
every cell:

| vul | scorer | Δ IMPs/board | 95% CI | fired | IMPs/fired |
|---|---|---|---|---|---|
| none | plain | **+0.0180** | ±0.0042 | 1.33% | +1.35 |
| none | PD | +0.0222 | ±0.0045 | 1.33% | +1.66 |
| both | plain | **+0.0284** | ±0.0056 | 1.58% | +1.80 |
| both | PD | +0.0360 | ±0.0061 | 1.58% | +2.28 |

Both plain-DD CIs clear zero by 4–5×, so this ships **default-on** — and it
is the counterexample to the `features_v2` precedent: an auction-input NLL
win *can* survive the A/B when the consumer is the evaluator (whose μ/σ
directly price game/slam boundaries) rather than the policy net.  The knob's
consumers are only the bilans gates, so the divergences are re-priced
game/slam decisions (1.3–1.6% of boards at +1.3 to +2.3 IMPs each).  What
was spent: the evaluator's weights are now coupled to the auctions of the
systems in the corpus (american + dutch), so routing changes owe the twin
protocol one retrain, and future systems must be dumped into the pool.  The
serving/training reading mismatch (bare `Context` at the call sites vs
`Stance::infer` in the corpus) is inherited from v2, unchanged, and still
open in the ledger.

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

---

## Is arithmetic enough? The auditable-backend gate  *(2026-07-26)*

jdh8's question, after BBA's own engine turned out to be plain arithmetic
([`bba-floor.md`](bba-floor.md) §5.5): **is a learned net the right backend for
bilans at all?** The worry was not speed — it was that the net is a black box
that DNF chop C1 caught *degrading on a provably true input tightening*, with
`--no-ns-bilans` attributing 47–64 % of that loss to the net alone.

[`examples/eval-arithmetic`](../../examples/eval-arithmetic/main.rs) prices the
alternative. It fits BBA-shaped least-squares rungs on the existing
`eval-train-1m-dnf` corpus (2 M rows; 13 coefficients do not need 20 M) and
scores them with the trainer's exact loss, `L = s + ½(t − μ)²e^{−2s}`, on
`eval-test-bits-dnf` — **the same held-out shard `evaluator_v2_dnf` was scored
on**, so the last row of the table is directly comparable. The rungs are nested,
so the whole ladder is leading principal submatrices of one Gram matrix per
target: one pass to fit, seconds to run, no solver and no sampler.

| rung | cols | NLL | MAE tricks | slam MAE |
|---|---:|---:|---:|---:|
| R0 const | 1 | −1.02475 | 2.334 | 5.993 |
| R1 +strength | 2 | −1.07559 | 2.218 | 5.354 |
| R2 +fit | 3 | −1.10227 | 2.162 | 5.137 |
| R3 +texture | 6 | −1.18405 | 1.983 | 4.288 |
| R4 +defence | 8 | −1.24744 | 1.849 | 3.830 |
| R5 +shape/width | 13 | −1.28560 | 1.779 | 3.612 |
| *linear on all 79 raw features* | 80 | — | *1.553* | — |
| **`evaluator_v2_dnf`** | 54→256→256 | **−1.51183** | **1.441** | **2.646** |

**Verdict: the replacement is refused.** The widest auditable rung is 0.34
tricks of MAE and 0.23 nats behind the net. The ship rule for this campaign was
deliberately the A/B rather than the NLL gate — the goal was auditability, not
accuracy — but a backend 0.34 tricks worse in the mean has no realistic path
through a non-regression A/B, and spending 200 k boards × 2 vulnerabilities to
confirm that would be spending the measurement to learn what the offline number
already says.

**The wide diagnostic is the useful part.** A plain linear fit on *all 79* raw
corpus features (`--oracle` truth columns excluded) lands at MAE 1.553. So the
gap from the 13-term arithmetic to the net splits:

- **0.226 tricks — feature compression.** Information the corpus already
  carries that 13 hand-picked terms throw away. Two-thirds of the gap.
- **0.112 tricks — nonlinearity.** What the two hidden layers buy over a linear
  map on identical inputs. One-third of the gap.

That ratio is the actionable finding: **on this input set, better features are
worth about 2× better function class.** It also bounds the ceiling for any
future arithmetic backend — even a *perfect* linear model on every column the
net sees still trails it by 0.112 tricks.

### What the coefficients say

R4's eight terms are the readable ones (R5 fits better but its terms stop
reading as bridge quantities — adding `spots` makes `pair HCP` absorb collinear
mass and pushes `kings` slightly negative in the majors, which is a real cost
under an auditability goal). In tricks per unit, our side declaring:

| strain | const | pair HCP | fit | aces | kings | trump HCP | their HCP | shortness |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| NT | 5.637 | 0.145 | −0.209 | 0.848 | 0.471 | 0.061 | −0.083 | 0.000 |
| ♠ | 5.974 | 0.120 | 0.404 | 0.559 | 0.290 | 0.123 | −0.069 | −0.682 |
| ♥ | 6.165 | 0.114 | 0.392 | 0.585 | 0.312 | 0.117 | −0.069 | −0.677 |
| ♦ | 2.809 | 0.109 | 0.501 | 0.636 | 0.347 | 0.103 | −0.059 | −0.448 |
| ♣ | 2.814 | 0.110 | 0.488 | 0.618 | 0.334 | 0.106 | −0.061 | −0.427 |

Three of these are worth keeping in mind whatever the backend is:

- **`pair HCP` ≈ 0.11 tricks/point in suits — one trick per 3 HCP, recovered
  from double-dummy labels with no bridge knowledge in the loop.** That is
  exactly BBA's `level = (total_points + 1) / 3 − 6` ladder, arrived at
  independently. The folklore constant is right.
- **An ace is worth ~1.9 kings in tricks**, above and beyond the HCP the fit
  already charges for both — and the premium is largest at notrump (0.848).
  4-3-2-1 undervalues aces for *trick-taking* even where it prices *strength*
  correctly.
- **`fit` is +0.4 to +0.5 tricks per combined trump card, and −0.21 at
  notrump.** The sign flip is the whole reason a single "hand value" scalar
  cannot serve both strains, and it is the measured form of the
  [`binky-points.md`](../binky-points.md) alignment limitation — except that
  here it is reachable, because the estimator sees *both* hulls rather than one
  hand's holdings.

### Consequences

1. **`set_bilans_floor` keeps the net.** The doubt that opened this session is
   answered on the numbers, in the direction of the status quo.
2. **The C1 fragility is not fixed by this and stays open.** An arithmetic
   backend would have been monotone in the reading by construction; the net is
   not, and the pre-registered C1 falsification never got to run because its
   subject was refused at the gate. The live candidate for C1 remains a
   knob-matched retrain, which is what F2b already did for the DNF flip.
3. **What would re-open a replacement:** the feature-compression term, not the
   function class. The corpus's range blocks carry lengths and `points` only —
   `Envelope::strength.suit_hcp` is never encoded
   (`features::push_inference`), so BBA's actual per-suit parameterisation
   (*combined* honour mask × both lengths) is unreachable here. Emitting those
   24 columns from `dump-evaluator` is a ~62-minute regen with no DD; check
   first how often `suit_hcp` is narrowed at real nodes, since the SHCP chop
   shipped it with no default-on consumer.
4. **One BBA idea is free and independent of all of this:**
   `level = max(level, winning_tricks − 6)` — a trick count that can only ever
   *raise* the level, never lower it. That asymmetry is what a floor wants, and
   it needs no backend change.

## The reach ceiling — what a net gate publishes  *(2026-07-26)*

Consequence 4 generalises, and following it out finds a defect in how
`set_bilans_floor` is wired. Nothing measured here; this is a reading of the
code and of the projection algebra, recorded before it bites.

### The gate reads as nothing

[`instinct.rs:1985`](../../src/bidding/instinct.rs#L1985):

```rust
(!bilans_floor() & authored) | net_break_even_gate(bilans_enabled, true, strain, tricks)
```

`net_break_even_gate` is a `pred`, so it inherits the default
`project` — `Dnf::unknown()`, [`constraint.rs:73`](../../src/bidding/constraint.rs#L73) —
and a DNF union containing ⊤ is ⊤. **Knob-on, every one of the eleven
converted milestones publishes a vacuous reading**, the same Or wall that
erased the 2/1 (see [`sampled-projection.md`](sampled-projection.md)).

This is not a missing fold. `project`'s soundness contract is
`finite eval ⇒ inside the projection`, and the net accepts hands no box
contains, so ⊤ *is* the correct projection. Tightening the reading requires
tightening the **criterion**; there is no cheaper rung.

Note also that knob-on the authored arm is masked off entirely, so the net
does not merely add to the arithmetic — it *replaces* it, holding an unbounded
veto over hands the point sums accept.

### Why it shipped anyway, and where it stops

Every converted site is a game or slam milestone — 3NT, 4M, 5m, 6, 7. Those
calls are mostly terminal, so the reading destroyed is one nobody consumes,
which is how the arm won all four cells with a hole in it. *Mostly*: a 4♥ that
gets bid over still owes partner and the opponents a reading.

The ceiling is what happens if the net is ever moved off terminal calls — a
limit raise, an invite, a 2/1. There the vacuum compounds: our call reads ⊤, so
partner's `Inferences::read` hands *the net itself* a ⊤ box for our seat, and
partner's estimate is computed on nothing. **Arithmetic has no such ceiling**;
its criterion is its reading by construction. That is a structural argument for
an auditable backend that the accuracy gate above does not capture, and it is
independent of the 0.34 tricks.

The general escape already exists —
[behavioural acceptance sampling](sampled-projection.md) replays the bidder as
the acceptance test, giving the exact reading whatever the criterion (the
measured `true 11..=26` against `projected 0..=37`). It costs a bidder call per
sampled layout against a free interval intersection.

### The fix, from the projection algebra

`And::project` intersects and `Or::project` unions
([`constraint.rs:296`](../../src/bidding/constraint.rs#L296)), so the two
directions a net gate can move a decision cost very different amounts:

| direction | shape | projection | cost |
|---|---|---|---|
| **veto** — net may only decline | `authored & net` | `box(authored) ∩ ⊤` = `box(authored)` | **free**; accepted set shrinks inside the promise |
| **accelerator** — net may only add | `authored \| (collar & net)` | `box(authored) ∪ box(collar)` | needs a finite `collar` |

A veto is already disclosure-correct and needs no collar: the reading stays
`authored`, loose in the safe direction. Only the accelerator adds hands
outside the box, and there the collar is forced by the soundness contract
above.

The candidate diff is one signature, and it *deletes* the mask:

```rust
fn points_or_net(
    authored: Cons<impl Constraint + Clone>,
    collar: Cons<impl Constraint + Clone>, // how far below `authored` the net may reach
    strain: Strain,
    tricks: u8,
) -> Cons<impl Constraint + Clone> {
    authored | (collar & net_break_even_gate(bilans_enabled, true, strain, tricks))
}
```

Knob-off the net arm is `-∞`, so `collar & -∞` is `-∞` and the gate is exactly
`authored` — byte-identical, and `!bilans_floor()` goes away. Call sites read
`points_or_net(combined_points(25), combined_points(23), strain, 11)`, which
puts the collar in plain sight where the threshold is. 23 is not arbitrary: it
is the invitational band, where a human already calls it judgment.

Which direction per site is BBA's answer, and it is the cheap/expensive
asymmetry — **accelerate at game, veto at slam**. `max(level, winning − 6)`
raises the level; `losing ≤ 1` is a *necessary* condition on the 33-point slam
override. Bidding a game the points do not support is cheap; bidding a slam
that fails is not.

**If measured, it is three arms, not two.** Today's replace-the-arithmetic
wiring holds that unbounded veto at the game sites too, so the collar gives it
back. Some of the win/win/win/win may be thin 25-counts the net declined, and
only an A/B separates the collar from the veto from the status quo.

### Correction — the collar is a behaviour fix, not a disclosure fix *(2026-07-26)*

The table above computes the accelerator's projection as
`box(authored) ∪ box(collar)`, which presumes `combined_points(25)` projects
finitely. **It does not.** All four pair-level gates in the floor are `pred`:
[`combined_points`](../../src/bidding/instinct.rs),
`combined_hcp`, `fit_sum_game` and `slam_entry_reached`. So `box(authored)` is
already ⊤ at every one of the nine `points_or_net` sites, and the collar
formula is `⊤ ∪ ⊤ = ⊤`. No call site conjoins an own-hand point band either.

The consequence is worth stating plainly, because the section above got it
wrong: **the arithmetic does not advertise itself either.** The net gate is not
the only ⊤ at these milestones, merely the newest one. What the collar buys is
that the arithmetic is the *criterion* again — it does not buy a reading.

Nor is that cheaply fixable. A contextual `project` on a pair gate would

- **recurse** — `project` → `Inferences::read` → `project_authored` → `project`;
- **hit the wrong-seat trap** — `project_call` evaluates rules in the *reader's*
  context ([`inference.rs`](../../src/bidding/inference.rs), the `project_call`
  closure), so `partner()` there is not the bidder-at-index's partner. That is
  the F2b′ bug, −13 IMPs/board.

And no *static* own-hand floor is sound across auctions: partner's shown minimum
ranges 0…22, so `combined 25` implies only own ≥ 3. Making pair gates project
therefore needs `Constraint::project` widened to carry the actor's seat and the
fold's partial readings — see the follow-ups below.

### The direction is derived, but only over vul-pairs

"Bidding a game the points do not support is cheap" is the right conclusion from
the wrong argument, and [`break_even`](../../src/bidding/instinct.rs) supplies
the right one — with a wrinkle. The table **mixes scoring conventions**: per the
bid-scoring split (a gate prices a *call*, and calls score under perfect
defense) the game row's failing branch is priced **down one doubled**, while the
slam and grand rows keep the **plain/undoubled** values.

| convention | game NV / vul | small slam | grand |
| --- | --- | --- | --- |
| plain (undoubled) | 5/11 = **0.455** / 6/16 = **0.375** | 0.500 | 0.560–0.583 |
| PD (down one doubled) | 6/12 = 0.500 / 8/18 = 0.444 | 0.500 | ≈ unchanged |

Two boundary facts, both of which break the naive strict form of the argument:

- **The non-vul game row is exactly 0.500, uniformly across all three
  milestones** — not a 5m artifact. Doubled failure against the partscore at the
  same trick count: 3NT 400 vs 2NT+1 150, 4M 420 vs 3M+1 170, 5m 400 vs 4m+1
  150, all 250 → 6 IMPs to gain; 120/140/130 against −100, i.e. 220/240/230 → 6
  to risk. All six sit in the 220–260 = 6 band, which is why one row covers
  three milestones. Vul is likewise uniformly 450 → 10 against 320/340/330 → 8.
- **The small slam is exactly 0.500 on *both* conventions, structurally.** The
  slam bonus is symmetric (6♠ NV 980 vs 4♠+2 480 = 500 → 11 to gain, 4♠+1 450 vs
  −50 = 500 → 11 to risk; vul 1430 vs 680 and 650 vs −100, both 750 → 13), and
  doubling the undertrick moves neither side out of its bucket (NV 550 → still
  11, vul 850 → still 13). Which is also *why* the game row could be re-derived
  under doubling while the slam rows stayed plain at no cost.

So `break_even < 0.5` vs `≥ 0.5` does **not** separate game from slam — non-vul
game and the small slam are tied on the line. The split survives in the
never-above / never-below form, read over vul-*pairs*, which is the right
granularity anyway because the shape is fixed at rule-construction time and must
serve both vulnerabilities:

| decision | `tricks` | over both vuls | net's licence |
| --- | --- | --- | --- |
| game | ≤ 11 | never *above* even money | accelerate — `points_or_net` |
| slam | ≥ 12 | never *below* even money | veto — `points_and_net` |

At exactly even money the economics give no direction, so the small slam's
tie-break is structural rather than economic: **a veto is the free shape** (it
only shrinks the accepted set, so it keeps `authored`'s reading), an accelerator
is not. Pinned by `break_even_keys_the_collar_direction`, whose bounds are
non-strict *on purpose* — tighten them to `<` / `>` and the two boundary rows
fail.

### Landed, then REFUTED — `set_net_collar` *(2026-07-26)*

`points_or_net` gained a `collar` argument and a sibling `points_and_net`; the
nine milestones split 4 accelerate (3NT, 5m, 4M, fitted 4M, collar
`COLLAR_SLACK = 2` below the threshold) / 5 veto (6M, 7M, 6NT, 7NT, the RKCB
grand). Knob `set_net_collar`, default **off**, harness `bba-gen
--ns-net-collar`, A/B `scripts/net-collar-ab.sh`.

Byte-identity is *not* expressible as a unit test — the legacy expression no
longer exists in the tree — so it was proven against a baseline worktree at the
parent commit: 3200 boards × both vuls × identical seed, **0 boards differ**
(the only delta is the output path echoed into `gen_args`). The four existing
bilans pins are also unmoved.

Smoke, same seed, knob-on vs knob-off (3200 bd/vul):

| vul | boards diverging | collar bids LOWER | HIGHER | same level |
| --- | --- | --- | --- | --- |
| none | 79 (2.47%) | 70 | 6 | 3 |
| both | 89 (2.78%) | 77 | 5 | 7 |

**The veto does essentially all the work; the accelerator is close to inert.**
Top families are `4♠ → Pass`, `6NT → Pass`, `6NT → 3NT`, `4♥ → Pass`,
`3NT → Pass`, `6♠ → Pass`, `6♥ → 4♥` — the collar is removing overbids the net
alone was making, not finding thin games. Even the handful of "higher" boards
are not game-site accelerations: the best example is off `3NT` → collar `4♦`,
the 3NT veto letting a lower-ranked rule win.

That has a consequence for the three-arm design proposed above: **collar and veto
would measure nearly identically**, because the arm they differ in (the
accelerator) barely fires. Two arms suffice, and if the collar loses, the
accelerator is not where to look for the reason.

The flagship board is the F1 forensic's 6NT blast, reached from the other side.
Seed 20260726 board 33, `AJ843.AK7.KJ52.7` at `1NT–2♥–2♠–3♦–3NT`: the shipped
wiring bids **6NT on a combined 31** because `combined_hcp(33)` was masked off
entirely; collared, `authored & net` declines and the auction rests in 3NT. F1
fixed the net's *inputs*; the collar restores the point floor the net was allowed
to ignore. Pinned by `net_collar_vetoes_the_notrump_slam_below_thirty_three`.

#### The A/B — all four cells lose

`scripts/net-collar-ab.sh`, sha `6fe1a27`, `SEED_BASE=1785059133`, 204,800
boards per arm per vulnerability. Arms are knob-off (shipped: the net holds the
whole criterion) vs `--ns-net-collar`. Positive = the collar wins.

| vul | scorer | IMPs/board | fired | IMPs/fired |
| --- | --- | --- | --- | --- |
| none | plain DD | **−0.0308** [±0.0041] | 2503 (1.22%) | −2.52 |
| none | perfect defense | **−0.0269** [±0.0043] | " | −2.20 |
| both | plain DD | **−0.0473** [±0.0055] | 3048 (1.49%) | −3.18 |
| both | perfect defense | **−0.0366** [±0.0059] | " | −2.46 |

Loss at both vulnerabilities on both scorers, every interval clear of zero, and
*larger* on plain DD than under perfect defense — the opposite signature to an
overbid-removal, which is what the collar was built to be. `set_net_collar`
stays **default off**, an opt-in knob and nothing more.

**The smoke run's direction was right and its sign was backwards.** It measured
that ~88% of diverging boards had the collar bidding lower and read that as
"removing overbids the net alone was making". Those lower calls are what costs
the IMPs. Grouping the 40 worst boards (the loss tail, not a census) by the
first divergent call, both vuls under PD:

| family | boards | IMPs |
| --- | --- | --- |
| `7NT → 3NT` | 15 | −234 |
| `7♠ → 4♠`, `7♥ → 4♥` | 17 | −265 |
| `4♠ → Pass`, `4♥ → Pass` | 22 | −328 |
| `4m → 3NT` | 9 | −120 |

The shipped net bids those grands and they **make**. Board
`AQ9.98.AQT32.JT6` opposite `KJT4.AKQ.K96.A72` is the clean one: `2NT–7NT`
cold on a combined 33, and the collar rests in 3NT for −14.

**Both shapes lose, so neither half is salvageable and the third arm is moot.**
The veto is refuted directly. The accelerator is too, and by a mechanism the
design missed: `COLLAR_SLACK = 2` caps the net's reach *below* the authored
threshold, but the shipped wiring's reach is **unbounded**, so at the game sites
the collar does not add hands — it takes away the ones the net was reaching for
past 2 points. Every `4M → Pass` above is that. An accelerator whose collar is
tighter than the incumbent's reach is a veto wearing the other shape's name.

**What this settles, one level up.** The doubt that opened this whole session —
that the net is a black box with an unbounded licence — is answered
empirically at all nine milestones: the net's reach below the point thresholds
**earns IMPs**, and pair-level point arithmetic used as either floor or
criterion is the worse of the two. That is the same verdict as the
[auditable-arithmetic gate](#is-arithmetic-enough-the-auditable-backend-gate-2026-07-26),
now paid for in boards rather than in NLL. The reach-ceiling *disclosure*
argument is untouched — it was never an accuracy claim — but its proposed fix
is not free, and follow-up A is now the only route that does not cost IMPs.

### Follow-ups this leaves open

- **A — project with seat and partial state.** Widen `Constraint::project` to
  `project(&self, ctx, who: Relative, partial: &[Dnf; 4])`. Non-recursive:
  `project_authored`'s fold is already sequential over `0..len` and already
  holds `players`, so the readings from indices `< i` are available at index `i`
  without re-entering `Inferences::read`; `who` kills the wrong-seat trap.
  `combined_points(t)` then emits `points((t − partner_min)..)`. Cost: ~20
  `project` impls in `constraint.rs` plus every ratchet re-pin. This is the only
  route that makes the *arithmetic* self-describing.
- **B — behavioural acceptance reading.** The general escape from
  [sampled-projection.md](sampled-projection.md): replay the bidder as the
  acceptance test and hull the accepted layouts. Works for *any* criterion
  including the net, so it dissolves the projectability question instead of
  answering it — and it is the only route that ever reads the learned contested
  floor. Costs a bidder call per layout against a free interval intersection.
- **C — the two sites left alone.** `net_makes` converts no authored arithmetic
  (it exists only knob-on, one caller: the business XX of a doubled 1NT), so
  neither shape applies. `slam_entry_reached` is a whole-`pred` if/else rather
  than Cons algebra, so collaring it is a restructure; note also that
  `SLAM_ENTRY_P = 0.35` is *deliberately* below break-even because the ask buys
  information, so it is an accelerator by intent and an RKCB ask is not a final
  contract.
- **D — 5m should probably not share 4M's threshold.** The 5m milestone gates
  eleven tricks on the same `combined_points(25)` as 4M's ten. `break_even`
  cannot see the difference (its game row is one constant over `tricks ≤ 11`),
  and its partscore derivation *is* right at this site, since the rule only fires
  behind `!stopper_in_their_suits()` — the alternative genuinely is a partscore,
  not 3NT. What is unpriced is the point threshold. Own A/B, own knob, per
  [convention-tuning.md](../convention-tuning.md); sweeping 26 / 27 is the cheap
  first arm. The collar A/B has since settled (refuted), so this is unblocked —
  but note it now sweeps a threshold the shipped wiring **masks off**, so the
  knob has to reach the arithmetic before the sweep can move anything.
- **E — 5m cannot be bid on a seven-card fit, and sometimes must be.**
  `known_eight_card_fit` returns `false` whenever `mine + partner.min < 8`, so 5♦
  on a 4-3 is unauthored and the hand falls through to a stopperless 3NT or a
  partscore. The layout where that bites is exactly the one the site already
  guards for — their suit unstopped, so notrump is off — and a 4-3 minor fit is
  then the only game. Sketch: admit a known **seven**-card fit at a higher
  threshold (`combined_points(26)`, roughly 3NT's strength and a shade above
  4M's), with the eight-card fit keeping whatever D settles. Beware: this widens
  the same rule the collar touches, so it is a third experiment, not a rider.

## Follow-up F — split disclosure from projection: `announced()` *(2026-07-26)*

Follow-ups A and B both answer "how do we make the *criterion* projectable".
There is a third route neither considers: stop requiring it to be. Let the net
decide and have the arithmetic say what the call means.

### Why `reads_as` is the wrong tool for it

The mechanism looks like it already exists —
[`reads_as`](../../src/bidding/constraint.rs) is exactly "evaluate as X, project
as Y", and its doc even names an evaluator-net gate as the motivating case. But
it overrides `project`, and `project` is what
[`within_ranges`](../../src/bidding/sampler.rs) accepts against. Publish a box
tighter than the criterion there and the sampler stops dealing the very hands
the net bids on — false precision, the catastrophic half of
[sampled-projection.md](sampled-projection.md)'s safety inversion. Nor does
replay rescue it: `sample_layouts_replay` **conjoins** `within_ranges` rather
than replacing it, so a too-tight range is a floor under both acceptance modes.
`reads_as`'s own doc concedes this — "acceptable for an opt-in arm whose A/B
must vindicate it, not for a default-on conversion".

### The observation that dissolves it

`Inferences` serves two masters that want opposite things.

| consumer | needs | because |
| --- | --- | --- |
| `admits` / `within_ranges` / sd-lead sampling | a box that **contains** the truth | a hand that made the call must be dealable |
| `features_v3`, `features_eval`, disclosure | the box the **agreement** names | a call is explained by the partnership's agreement, never by the bidder's cards |

The second is not bound by containment — that is what "agreement" *means* at the
table, and `features.rs` already says so in as many words. Only the first is.
Today one object serves both, so the sampler's contract is imposed on
disclosure, and the net's ⊤ propagates to a partner computing its estimate on
nothing.

So: a second fold. `Constraint::announce`, defaulting to `project`, forwarded
through `And` (intersect) / `Or` (disjoin) / `Cons`; `Rule::announce_dnf`; a
parallel overlay in `project_authored`; `Inferences::announced(who)`. The two
arrays are byte-identical everywhere until

```rust
pub fn announced(judgment: ..., agreement: ...) -> ...
```

splits them — `eval` and all three projections stay on the judgment (the net's
⊤ survives, the sampler never notices), `announce` and `describe` take the
agreement. Knob `set_announced_reading`, default off.

### Two things the build learned that the design did not know

**The agreement must union over *alerted* rules only.** The floor's 4NT keycard
ask shares its bid with an unalerted weight-0.3 catch-all, and the fold's
`reduce(Dnf::disjoin)` over every rule for that call unioned the agreement
straight back to ⊤. For the projection that union is mandatory — any of those
rules could have produced the call, so the sampler may not exclude a hand any of
them accepts. For disclosure it is simply wrong: an alerted call is explained as
*the convention*, not as the residue sharing its bid. This is the same
discrimination `decode` already makes one line above.

**The announce must ride `points`, not `support_points`.**
`features::push_inference` encodes four lengths and `points` — nothing else. A
box on `strength.support_points` is invisible to every net that reads the
auction, which is the same write-only-axis trap the DNF ledger's C2 hit with
`strength.hcp`. Check the encoder before choosing an axis.

### The pilot, and what it measured

`slam_entry_reached` at the 4NT RKCB ask — follow-up C's site, the one converted
milestone that is not a final contract, and the worst reading in the tree
(`.alert(RKCB_FLOOR)` suppresses the natural reading *and* the projection is ⊤,
so it announces nothing at all). `examples/probe-announced-rkcb`, 100k boards:

| quantity | value |
| --- | --- |
| ask fires | 2.23% of boards |
| **floor**-authored (the pilot's reach; a book node shadows the floor) | 59.8% of those, **1.35% of boards** |
| asker's own `point_count` | p5 = 11, p10 = 12, median 16 |
| asker's own seat floor *already* read | p25 = **0**, median 10, mean 7.8 |
| `points(11..)` bites | 80.6% of firings, mean **+4.24** points of floor |

`RKCB_ASK_ANNOUNCE = 11` covers 95% of firings. Deliberately **not**
`FLOOR_SLAM_ENTRY`'s 29 combined: partner's shown floor at these nodes is median
6, so the arithmetic would claim ~23 of our own while the net actually fires at
16. **The net reaches some seven points below the sum it replaced**, so
inheriting that sum as the agreement would misdescribe the median hand, not just
the tail. An agreement is calibrated to what the criterion *does*.

**And the pilot is bid-inert.** Same-seed divergence, 60k boards, the two knobs
isolating the nested halves:

| arm | boards diverging |
| --- | --- |
| agreement overlay (alerted-only union), `set_rkcb_announce` off | 1400 (2.333%) |
| + the RKCB agreement | 1405 (2.342%) |

Five boards in sixty thousand. Not a failure of the mechanism — a fact about the
site: after `4NT` the auction runs on keycard answers and `keycard_total`,
neither of which reads partner's points, so **nothing there decides on the
reading**. Its payoff is disclosure and lead/defence sampling, which is the
ground `set_pass_reading`, `set_cue_reading` and `set_table_alert_reading`
shipped on — all three bid-inert, all three shipped for reading soundness.

The corollary generalises the reach-ceiling argument: a vacuous reading only
costs where something *reads* it. Before wrapping the next site, check that a
consumer exists, the same way this section checked the encoder before choosing
an axis.

### The A/B — a wash in all four cells

Since the pilot is inert, the measurement is of the alerted-only union.
`scripts/announced-reading-ab.sh`, `SEED_BASE=1785070714`, 204,800 boards per
arm per vulnerability, arms `off` vs `--ns-announced-reading`. Positive = the
agreement overlay wins.

| vul | scorer | IMPs/board | fired | IMPs/fired |
| --- | --- | --- | --- | --- |
| none | plain DD | −0.0008 [±0.0028] | 1947 (0.95%) | −0.085 |
| none | perfect defense | +0.0020 [±0.0033] | " | +0.206 |
| both | plain DD | +0.0010 [±0.0035] | 1850 (0.90%) | +0.105 |
| both | perfect defense | +0.0039 [±0.0040] | " | +0.430 |

**Every interval straddles zero.** Directionally positive in three cells of
four, and PD sits above plain at both vulnerabilities (+0.0028 and +0.0029) —
the overbid-removal signature, and the opposite of the collar's, which lost
*worse* on plain. But it is well inside the noise and must not be read as a win;
at 0.9% firing the per-fired numbers are what the intervals are made of.

So the change is free rather than profitable, and its case rests where it always
did — on disclosure. That is the same ground `set_pass_reading`,
`set_cue_reading` and `set_table_alert_reading` shipped on, with one difference
worth stating plainly: those three were *bid-inert* (0–1 divergent boards per
211k) so their A/B was a wash by construction, while this one genuinely moves
0.9% of boards and comes back level. That is a stronger result than inertness,
not a weaker one — it says the tighter agreement does not mislead the nets.

Left **default off** pending an explicit call on the default.

Worth noting for whoever picks this up: the worst boards diverge *early* —
`1♥ 1♠ 2♠ 2NT …` against `1♥ 1♠ 2♠ – …`, the fourth call — and cluster on
competitive auctions with `6♣ X` sacrifices, nowhere near the RKCB site. That is
the alerted-only union reaching every alerted call in the book, exactly as the
isolation runs said. If the default is ever flipped, the per-family attribution
to run first is which *alerted conventions* the tightened agreement moved, not
the pilot.

## Shape-distribution reading — the hull is not a function of the reading  *(2026-07-27)*

Shipped as `features_eval_v4` behind `set_eval_shape`, default off pending its
A/B.  Replaces each hidden seat's four length `{min, max}` pairs with a summary
of the **distribution over shapes** the reading implies.  Three columns wider
than v3 (97 vs 94), NLL-identical, and invariant where the endpoints are not.

### The defect being fixed

`push_inference` encodes a reading as an axis-aligned box.  A box is not a
function of the information: `♥5..13, ♠5..13` and `♥5..8, ♠5..8` admit the
*identical* set of hands — with 5+ spades you cannot hold 9 hearts — yet `max/13`
reads 1.00 in one and 0.62 in the other.  So `set_sum_closure`, which provably
rejects no hand, still displaces the net's inputs by multiple σ, and every
reading-fidelity chop has had to buy a retrain before it could be judged on
merit (chop C1, `docs/dnf-migration.md`).

### The kernel

There are exactly 560 length vectors `l = (l♠, l♥, l♦, l♣)` with `Σ l = 13`.
With `n_s = 13 − my_len_s` unseen cards per suit, one hidden seat holds shape `l`
with hypergeometric weight `w(l) = ∏_s C(n_s, l_s)`.  The reading is a union of
boxes; keep the atoms admitted by any box, renormalise, read off `E[l_s]`,
`sd[l_s]` and `−ln(admitted/total) / ln C(39,13)`.

Enumerating the **atoms** is what makes this affordable: union membership is
"is `l` in any box", so there is no inclusion–exclusion, no overlap cap, and none
of the machinery that made the archived `MassOracle` (0.35 ms build + 0.1 ms per
query) unaffordable.  Measured dump cost: +6%.

### The ablation — two rounds, twelve arms

Round one (10.18M rows) priced the pieces; round two (8.15M rows, control
re-anchored at −1.54558) chose the family.  Seed spread 0.0006.

| arm | live cols | val NLL | Δ | MAE | slam-MAE |
| --- | --- | --- | --- | --- | --- |
| `shape-control` — endpoints | 94 | −1.54558 | — | 1.396 | 2.456 |
| `shape-gauss` — E, sd | 94 | −1.54493 | −0.00065 | 1.397 | 2.456 |
| `shape-mass` — log-mass alone | 73 | −1.52264 | **−0.02294** | 1.426 | 2.534 |
| `shape-gauss-mass` **(shipped)** | 97 | −1.54562 | **+0.00004** | 1.396 | 2.456 |
| `shape-hist` — 14-bin marginal | 238 | −1.54577 | +0.00019 | 1.395 | 2.446 |
| `shape-hist-mass` | 265 | −1.54639 | +0.00081 | 1.395 | 2.449 |
| `shape-hybrid` — + endpoints | 289 | −1.54640 | +0.00082 | 1.395 | 2.449 |

Round one, on its own corpus: `shape-moments` −0.0004, `shape-cov` −0.0004,
`shape-full` +0.0011, `shape-hybrid` +0.0012.

Four readings, all load-bearing:

- **The covariances are worth 0.00001.** The joint term across suits is the
  entire intuitive case for a shape *distribution* over four per-suit readings,
  and it bought nothing.  They are not carried.
- **The log-mass column is a modifier, not a substitute.** +0.0007 beside a
  length reading, −0.023 in place of one.  "How much do I know" cannot stand in
  for "what does partner hold".
- **The endpoints are spent.** `hybrid` ties `hist-mass` at 0.00001, replicated
  across two corpora and two shape blocks.  This *inverts* the MARG/MASS verdict
  that "hulls are the sufficient statistic": those campaigns measured estimated
  marginals *beside* the endpoints, and against exact ones it is the endpoints
  that are redundant.
- **The marginal is not worth its width.** +0.0008 for 168 extra columns per row
  and a 43% wider forward pass — 1.35× the seed spread, ≈0.0005 IMPs/bd by the
  calls-tail scaling.  Hence the Gaussian ships.

### Par is the correct result

`p(tricks | my hand, reading)` depends on the reading only through the set of
hands it admits.  The endpoints are a *non-injective* encoding of that set, so
the net must learn "these two inputs mean the same thing" from data — and only
learns it where the corpus has examples.  The moments are a function of the
admitted set itself, so the ambiguity is gone by construction rather than by
training.  MASS already showed the fitted `(μ, σ)` at a hull row *is* the
union-conditional; on covered patterns a re-parameterization cannot pay.  The
gain lives on patterns the corpus does not cover — which is exactly what a
closure chop manufactures.

Shipped artifact, trained natively on the 97-wide layout at the v3 protocol
(500k deals of `22.pdd`, seed 1, pooled american+dutch, `--dnf`, 9,165,496 train
rows): **−1.54856 / MAE 1.392 / slam-MAE 2.474**, against the shipped v3's
−1.54872 / 1.392 / 2.479.

### What is owed

The A/B, in two parts.  **A/B-1** (v4 vs v3) is a *non-inferiority* check only:
at 204.8k bd/arm/vul the interval is ±0.004 IMPs and the effect is ~0.0005, so
it can refute a regression and cannot detect a win.  **A/B-2** is the one with
signal — `set_sum_closure` on vs off *under the shape twin*, against the same
pairing under endpoints, where the closure is a known feature-perturbation loss.
A wash-or-win there is what unblocks the closure roadmap and is the result this
campaign exists for.
