# Binky Points with error bars

> A hand valuation that publishes a **spread**, not just a centre. Additive
> numbers per suit holding — tricks and trick² — so a partnership reads off a
> distribution and prices `P(make)` in closed form.
>
> Generated tables: [binky-table.md](binky-table.md) (notrump),
> [binky-table-suit.md](binky-table-suit.md) · machine-readable
> `web/binky.json`, `web/binky-suit.json` · interactive: the **Evaluate** tab of
> the [web UI](../web) · harness: `examples/binky`

## Why

Thomas Andrews' [Binky Points](https://bridge.thomasoandrews.com/valuations/)
fit one number per hand to double-dummy truth, additive across the partnership,
and — like Andrews — this publishes **notrump and best-suit separately**, because
they are different physics. So does every other published count: Work, Fifths,
BUM-RAP, Zar. None of them says how *uncertain* that number is.

Uncertainty is what a bidding decision consumes. A game gate fires on
`P(T ≥ 9) ≥ break-even`, which is a statement about a distribution; a point count
hands the gate a single number, and the gate supplies the spread implicitly, as a
constant, by comparing points against a fixed threshold.

This crate already learned that spread once — `src/bidding/evaluator.rs` is a
heteroscedastic net emitting `(μ, ln σ)`. But it needs partner's *inferred
range*, weighs 90k floats, and is not something you can tell partner. The
question here is older: **how much of the spread survives compression into an
additive per-hand table?**

## The kill gate that had to pass first

`probe-trick-variance`'s **Cut C**, on 8M deals from `/nfs2/jdh8/pons/24.pdd`. At
*matched* partnership HCP, bucket N-S notrump tricks by longest combined suit:

| 24–26 HCP | NT μ | NT σ |
| --- | ---: | ---: |
| longest fit ≤ 7 | 8.744 | 1.030 |
| longest fit 8 | 8.585 | 1.330 |
| longest fit 9 | 8.559 | 1.649 |
| longest fit ≥ 10 | 8.486 | **1.897** |

The mean *falls* 0.26 tricks while the spread nearly **doubles**. In notrump a
nine-card suit is a coin flip — it runs or it does not — and that is an axis μ
structurally cannot see.

Pre-registered threshold: 0.3 tricks, from `dP/dσ ≈ 0.121/σ` at the vulnerable
game break-even (37.5%) and `0.045/σ` at the non-vulnerable one (45.5%), against
this repo's standing 8-point NV/vul yardstick. Measured: **0.83–0.96** on all
deals, **0.49–0.70** on the honest replication (both opponents under 12 HCP with
no six-card suit, so no real auction would disclose the E-W split). Note σ has
*zero* effect exactly at the boundary; it bites only through the asymmetry, which
is why the vulnerable coefficient is 2.7× the non-vulnerable one.

## The model

**Labels.** Notrump tricks, or best-*suit* tricks (notrump excluded, so the two
tables answer disjoint questions). Both are `max(declarer N, declarer S)`, one
sample per deal — E-W is not a second sample, since in a fixed strain
`T_EW = 13 − T_NS` exactly.

**Basis.** A holding is keyed by (subset of {A,K,Q,J,T}, spot count 0..=8) → 288
cells, of which 223 clear the occupancy floor; the rest fold into their nearest
neighbour by dropping spots. **One table serves all four suits**: both labels are
invariant under permuting suits (a max over the four suit strains is permuted,
not changed), so holdings are exchangeable and four per-suit tables would be four
noisy copies of one.

```
mu     = mean_const + sum of the 8 holdings' mu entries      (tricks)
sigma2 =  var_const + sum of the 8 holdings' var entries     (trick^2)
P(T >= k) = Phi((mu - k + 0.5) / sqrt(sigma2))
```

**Variance adds; log-variance does not.** Given both N-S hands the only
randomness left is the E-W split of 26 cards, and it enters as near-independent
per-suit events — a two-way finesse contributes ≈ +0.25 trick², AKQJT opposite
xxx contributes 0. Independent sources add *variances*; a log-additive model
would say two independent 50/50 suits **multiply**. `ln σ` appears nowhere: it is
a trainer's parameterization (free positivity under gradient descent), and
nothing here does gradient descent.

### The gauge — read this before quoting a weight

The design is rank-deficient by **two** directions. Every row satisfies
`Σ n_c = 8` *and* `Σ n_c·size(c) = 26`, so the null vector `z_c = 13 − 4·size(c)`
survives even with no intercept column: value slides freely between "per holding"
and "per card", and two slices would otherwise publish visibly different tables
for the same law.

**The table is only defined up to `w_c → w_c + α + β·size(c)` with
`8α + 26β = 0`.** It is pinned by `Σ p_c w_c = 0` and `Σ p_c size(c) w_c = 0`,
which makes the constant the population mean and every weight read as *excess
versus an average holding, net of any pure per-card credit*. That statement
travels with the numbers.

## Two variance columns, because there are two honest questions

This is the artifact's central correction, and it was found by the benchmark
rather than by reasoning.

Fitting the variance head to the squared residual — the obvious thing, and what
the first version shipped — estimates

```
E[(T - mu_hat)^2 | n]  =  Var(T | N,S)  +  (mu_true(N,S) - mu_hat(n))^2
```

that is, **the hand's uncertainty plus the model's own squared bias**. On the
notrump table the second term is nearly half the total. So the published column
tracked where the *estimator* was ignorant, not where the *cards* were volatile.

Both quantities are legitimate, and the artifact now publishes both:

| column | fitted against | answers | use it for |
| --- | --- | --- | --- |
| **predictive** `var` | the model's squared residual | "given only this table's reading, how uncertain is the trick count?" | pricing `P(make)` — this is what makes the probabilities calibrated |
| **physical** `physical var` | reshuffled E-W truth | "given the actual two hands, how volatile are they?" | describing the cards |

The decomposition closes numerically: predictive `σ² = 1.578` ≈ physical `0.900`
+ model error `0.678`.

Fitting the physical column costs solver time, and **two shuffles per pair is
enough — not a compromise.** `E[(T₁ − T₂)²] = 2·Var(T | N,S)` exactly, so the
Bessel-corrected sample variance is *unbiased* at every `M ≥ 2`; per-row noise
costs rows, not correctness, in a regression. 150k pairs therefore cost 300k
solves rather than millions.

## The benchmark: fixed N-S, shuffled E-W

Every other check compares a prediction against one realised deal or a bucketed
proxy. This one fixes both N-S hands and reshuffles the East-West split, so the
sample moments converge on `E[T | N,S]` and `sd(T | N,S)` — exactly what the
columns claim to be.

**There is no sampler bias here, and that is the point.** Conditioned on the two
N-S hands and nothing else, the posterior over the hidden 26 cards *is* uniform
over E-W splits. No inference, no envelope, no rule replay. This is the unbiased
denominator [`ai-bidder/evaluator-net.md`](ai-bidder/evaluator-net.md) records as
still owed for the learned net; a per-hand table is the one estimator cheap
enough to get it for free.

500 pairs × 200 shuffles, notrump, held-out slice:

| | MAE | noise floor | deconvolved | signed spread | corr with true σ |
| --- | ---: | ---: | ---: | ---: | ---: |
| μ | 0.5892 | 0.0505 | 0.5871 | +0.0715 (bias) | — |
| σ **predictive** | 0.4399 | 0.0357 | 0.4384 | **+0.3462** | 0.262 |
| σ **physical** | **0.2702** | 0.0357 | **0.2678** | **+0.0530** | **0.331** |

Deconvolution is in **quadrature**, never linearly: the true moments come from
`M` draws, so the sample mean carries `SE = σ/√M` and the sample sd `SE ≈
σ/√(2M)`; both errors are roughly Gaussian and `MAE = √(2/π)·SE`, so the factors
cancel. Doing this linearly once understated the learned net by ~45%.

Fitting against truth **cuts σ error by 39% and removes 85% of the
over-dispersion**. By true-σ quintile:

| | true σ | predictive | physical |
| --- | ---: | ---: | ---: |
| Q1 | 0.465 | 1.172 | 0.898 |
| Q3 | 0.837 | 1.276 | 0.947 |
| Q5 | 1.479 | 1.308 | 1.014 |

Read that table honestly. The physical column is **unbiased in aggregate** and
**weakly discriminating**: corr 0.331 is R² ≈ 0.11, and its quintile means move
0.12 while truth moves 1.01. It gets the average conditional spread right; it
cannot rank *which* hands are volatile. That is the additivity wall, measured on
the right quantity — see [the limitation](#the-limitation-stated-plainly).

## Results — notrump

### The mean column replicates the known answer

Three-card holdings, relative to `xxx` (−1.220):

| | A | K | Q | J | T |
| --- | ---: | ---: | ---: | ---: | ---: |
| excess over `xxx` | 2.316 | 1.554 | 0.929 | 0.428 | 0.150 |
| pegged A = 4 | 4 | **2.68** | **1.60** | **0.74** | **0.26** |

That is the Fifths/BUM-RAP family, and it independently reproduces this repo's
earlier point-count distillation (K ≈ 2.5, Q ≈ 1.4, J ≈ 0.75, T ≈ 0.3) from a
different basis — honour *sets* per holding rather than free per-honour counts.
Work 4-3-2-1 overvalues the king and ignores the ten.

The earlier `examples/eval-pointcount` fit used 200k pre-solved pair samples.
Free honour weights improved R² by only **0.017** (about **0.05 tricks σ**) over
calibrated 4-3-2-1, while one shortness term bought about **0.17 tricks σ** in
suit contracts and nothing in notrump. Its raw data-count 1NT band selected
9.51% of hands versus Work's 10.13%; widening it to about 14.5–17.6 matched
10.14%, and the resulting ace/ten-rich-for-K/Q/J-rich boundary swap was
**+0.030 ± 0.042 tricks/hand**, statistically zero. The useful property of a
one-hand point count is therefore partnership coordination, not a tiny increase
in fit accuracy; extra precision pays when a captain can also use partner's
inferred range.

Held-out RMSE against single realised deals, tricks:

| model | RMSE |
| --- | ---: |
| HCP only | 1.3367 |
| HCP + shortness | 1.3354 |
| **holding table** | **1.2572** |

Shortness buying nothing (0.0013) is correct rather than surprising — this is
notrump. And 1.2572 against a realised deal is consistent with the benchmark's
0.5892 against the *conditional mean*: the rest is the physical spread itself,
`√(0.589²·π/2 + 0.895²) ≈ 1.16`.

Calibration: bias −0.001 tricks, 48.4% of deals below μ, and the reliability
diagram sits on the diagonal in every decile for both `P(T ≥ 9)` and `P(T ≥ 10)`
— the 40–50% bucket realises 46.8%, the 80–90% bucket 85.7%. Reliability is
reported instead of central-50% coverage on purpose: conditional on both hands
the trick count often has a two-trick jump on a suit break, and coverage is blind
to exactly that while the decision lives in the tail. **This is the predictive
column doing its job** — it is calibrated *because* it carries the model's error.

### The variance column is new information

**Gate 1 — algebraic.** Regressing `v` on `span{1, size, μ}` across cells gives
R² = 0.031: **96.9% of the variance column is not recoverable from the mean
column**.

**Gate 2 — within-μ.** Bucket held-out deals by predicted μ, split at the median
predicted σ, compare the *realised* SD of the halves. Predicted σ sorts empirical
spread in every bucket, recovering 82–94% in the populated middle. Note this gate
measures spread *around the model's μ*, so it validates the predictive column and
says nothing about the physical one — which is precisely why it missed the
over-dispersion, and why the benchmark exists.

**Gate 3 — decisions.** Against a σ-blind gate using one global SD (1.257) for
every hand, on the live band `|μ − 8.5| < 0.5`: the σ-aware gate disagrees on
**5.3%** of boards and is right on **55.9%** of the disagreements.

### What the honours actually buy — and the correction

| holding | μ | predictive var | **physical var** |
| --- | ---: | ---: | ---: |
| singleton A | +1.798 | −0.099 | **−0.074** |
| singleton K | +0.552 | **+0.482** | **−0.085** |
| singleton Q | +0.107 | +0.218 | −0.063 |
| singleton J | −0.126 | +0.173 | +0.040 |
| singleton x | −0.335 | +0.113 | +0.051 |
| QJ | +0.156 | **+0.346** | **−0.112** |
| AK | +2.775 | −0.105 | −0.108 |
| KQxxxxxx | −1.240 | +0.771 | +0.628 |
| AKQTxxxxx | +1.224 | +3.266 | +0.569 |

**An earlier revision of this doc read the predictive column as physics and got
the headline backwards.** It said "aces buy certainty, kings and queens buy
lottery tickets", from the stiff king's +0.482 and QJ's +0.346. Both are model
error: what a stiff king is worth depends on where the ace sits, which is
*alignment*, which an additive table cannot see — so the misfit lands in the
residual and the residual is what that column fits. Physically a stiff king
**narrows** the distribution (−0.085), as does QJ (−0.112).

The real pattern is cleaner and the opposite in emphasis: **every honour carries
negative physical variance; spots, low honours, and length carry positive.** An
honour is a defined asset — it mostly is a trick, and it is a trick regardless of
how the opponents' cards lie. A long suit's extra tricks depend on the break and
on entries, which is exactly what the E-W shuffle randomises. `KQxxxxxx` at
+0.628 and `AKQTxxxxx` at +0.569 are the volatility; `A`, `K`, `Q`, `AK` are the
ballast.

## Results — best suit: the σ column does not survive

The plan pre-registered the risk: best-strain tricks are dominated by **fit**, an
additive per-hand table cannot see whether North's five spades face South's three
or South's void, and that misspecification has nowhere to land except σ — where it
would be published as uncertainty. The benchmark says exactly that happened.

| | notrump | best suit |
| --- | ---: | ---: |
| μ MAE (tricks) | 0.589 | 0.614 |
| μ bias | +0.072 | +0.055 |
| predictive σ MAE | 0.440 | 0.351 |
| predictive over-dispersion | +0.346 | +0.328 |
| **corr(predictive σ, true σ)** | **0.262** | **0.059** |
| physical σ MAE | 0.270 | 0.123 |
| physical over-dispersion | +0.053 | +0.015 |
| corr(physical σ, true σ) | 0.331 | 0.331 |

*(The two physical correlations agreeing to three decimals is coincidence, not a
shared code path — the reported numbers come from different runs and different
tuple slots. Correlation is scale-invariant, so two columns that are each "a
constant plus the same weak length signal" land in the same place.)*

The **mean** column is fine: 0.614 tricks MAE against notrump's 0.589, on a label
with more spread to explain. An additive table predicts best-suit trick *counts*
about as well as it predicts notrump ones.

The **σ** column is not fine, and the aggregate MAE hides it. Sorted by true σ:

| quintile | true σ | predictive σ | physical σ |
| --- | ---: | ---: | ---: |
| Q1 | 0.394 | 0.931 | 0.625 |
| Q2 | 0.569 | 0.987 | 0.650 |
| Q3 | 0.649 | 0.991 | 0.659 |
| Q4 | 0.732 | 1.004 | 0.676 |
| Q5 | 0.868 | 0.941 | 0.679 |

True σ more than doubles down that column. Predicted σ moves by 7% and **not
monotonically** — it is a constant wearing error bars. The physical column rises
9% against truth's 120%: right on average, ranking nothing. Its small MAE (0.123)
measures only that it guessed the average spread correctly, which a single number
would also have done.

Compare notrump, where the same table is weak but not inert: physical σ climbs
0.898 → 1.014 (+13%) across quintiles, monotonically, against truth's +218%.

**Verdict: publish the best-suit mean, not the best-suit spread.** The generated
`web/binky-suit.json` and [binky-table-suit.md](binky-table-suit.md) keep both
columns because the harness writes them, and the μ half is genuinely usable — but
no decision should read the suit table's σ, and the Evaluate tab labels it. This
is the honest negative the plan asked for, and it costs one afternoon instead of
a shipped miscalibration.

The asymmetry is itself the finding. Notrump tricks are close to a sum of
per-suit contributions, so additivity is a mild assumption and σ is weakly real.
Best-suit tricks are a **max over strains of a quantity that depends on how two
hands mesh** — additivity is the wrong functional form, and no amount of data
fixes a wrong form. Same wall as
[docs/bidding-architecture.md](bidding-architecture.md)'s standing result that fit
is a bidding problem, not an evaluation one.

## The limitation, stated plainly

An additive per-hand table **cannot see alignment**, and both instruments now
price it.

Cut C's driver was the *combined* suit length; a table keyed on one hand's
holdings credits "long suit somewhere", not "opposite partner's":

| partnership HCP | empirical σ spread across fit classes | table predicts | recovered |
| --- | ---: | ---: | ---: |
| 18–20 | 0.964 | 0.211 | 22% |
| 21–23 | 0.833 | 0.214 | 26% |
| 24–26 | 0.870 | 0.212 | 24% |
| 27–29 | 0.894 | 0.205 | 23% |

And the benchmark says the same thing from the other side: the physical column
correlates 0.331 with true conditional σ, R² ≈ 0.11.

**So: the table gets the average spread right and ranks individual hands
weakly.** The residual is alignment, and no point count of any kind can express
it — this is the same wall that makes fit-finding a *bidding* problem rather than
an evaluation problem. It is also the honest ceiling on the whole genre: the
learned evaluator beats this precisely because it is not additive.

Two more caveats to keep with the numbers:

- **Double-dummy defence understates both level and spread.** Single-dummy play
  loses tricks *and* adds variance the label cannot see, so the published σ is a
  lower bound and `P(make)` runs optimistic.
- **The integer label carries ~1/12 trick² of discretisation variance**
  (Sheppard), about 3% of σ. Mentioned, not corrected.

## Using it

The **Evaluate** tab of the web UI is the calculator: type or import two hands,
get μ, both σ's, and `P(make)` at every level against the IMP break-evens. Hands
move both ways with the Edit tab.

Its **"Verify with double dummy"** button runs the benchmark on *your* hands, in
the browser, via the pure-Rust `pons-dds`: it fixes N-S, reshuffles E-W, and
overlays the table's physical Gaussian on the observed histogram. 100 shuffles is
the default and is interactive (σ's own standard error ≈ 0.067 tricks); 1000 is
offered for a tighter read (≈ 0.021) and only earns its wait for per-hand
judgement.

## Reproducing

```sh
# the kill gate, no model fitted
cargo run --release --example probe-trick-variance -- \
    --deals /nfs2/jdh8/pons/24.pdd --skip 5000000 --count 8000000

# the fit, both variance columns, the benchmark, and both artifacts
scripts/idle-run.sh cargo run --release --example binky -- \
    --deals /nfs2/jdh8/pons/24.pdd --skip 10000000 --count 20000000 \
    --test-skip 40000000 --test-count 2000000 --label notrump \
    --variance-truth 150000 --benchmark 500 --shuffles 200 \
    --json web/binky.json --markdown docs/binky-table.md
```

`--label best-suit` writes the suit pair. The `.pdd` bank arrives pre-solved, so
only `--variance-truth` and `--benchmark` run the solver, and both batch 256
pairs per `solve_deals` — a two-layout batch per lock measured ~20× slower.

`cargo test --example binky` holds the machinery: an exact linear law recovered
from noiseless rows shaped like real deals (eight holdings, twenty-six cards), and
the gauge proven to map two representatives of one law onto the same table.
`cargo test` in `web/` holds the browser verdict: it accumulates across chunks,
is genuinely conditional (25 HCP must land at 7–11 tricks, not the unconditional
6.5), and refuses impossible hands.

This is a diagnostic and a publication. **It changes no bidding behaviour and
owes no A/B** — the crate is byte-identical.
