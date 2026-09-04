# Precedence vs probability: what a book logit means, and where the odds come from

**Status: SETTLED 2026-09-03 by interview** (`/grill-me`, fifteen decisions,
every recommendation accepted). Session 1 shipped the same day: this document,
display hygiene (`web/src/lib.rs` `top3()`, `examples/practice-bidding`), and
the two doc-drift repairs in §5. **No bidding change**: `smoke-default --count
20000 --seed 1` is unchanged at `38ee1e21…` before and after
([../measurement.md](../measurement.md) item 12). Sessions 2 and 3 shipped the
same day and under the same gate; sessions 4, 5 and 6 followed on 2026-09-04 and
each moved the next task — **session 7 is owed**, and the ledger is §6. This is the calibration story the
M5.2 flip plan in [plan.md](plan.md) M5.2 needs before its collar retune can be sized, and the
scale question [competitive-accountant.md](competitive-accountant.md) and
[new-suit-veto.md](new-suit-veto.md) both stepped around by acting on masks
and demotions rather than on magnitudes.

The one-line verdict: **a book weight is a precedence, not a log-probability;
nothing on the default path reads it as a magnitude; the odds, when a consumer
needs them, come from the net at a fitted temperature.**

## 1. The overload

One `i16` does two jobs. `src/bidding/rules.rs:1-23` says both out loud: the
logit of a call is the **max** of `weight + constraint` over its rules; weights
are "soft priority in centinats — 155 is 1.55. A gap of about 300 is
near-deterministic after softmax"; the unit is integral on purpose so that rung
equality is a diagnostic claim; equal weight on one call is an alternative
justification, differing weights are authored precedence ("the lower rule
speaks only for hands the higher one rejects"). *Priority* is the argmax job.
*Near-deterministic after softmax* is the probability job. The module doc
asserts both, and the second is only true where the first was authored at
300-centinat spacing, which it almost never is.

Every constraint is crisp. `src/bidding/constraint.rs:749-752` is the whole
mechanism, `const fn crisp(condition: bool) -> f32` returning `0.0` or `-inf`,
and each of the leaf impls among the 40 `impl … Constraint for` sites in
`src/bidding` (`grep -rn "impl.*Constraint for" src/bidding` recounts them;
28 call `crisp` directly) goes through it; the combinators preserve it —
`And` sums, `Or` takes the max, and `{0, -inf}` is closed under both. The one
escape, the bare-closure impl at `constraint.rs:352`, is used with a graded
value only in `rules/tests.rs:201`; every book site reaches it through
`pred`, which wraps in `crisp` (`constraint.rs:950`).
So a book logit is never a graded number: it is `-inf`, or exactly the authored
rung. No rule is "80% admissible"; the book has no notion of a hand that
*nearly* qualifies. That is the right shape for precedence and the wrong shape
for probability, because a distribution over calls at a node then depends only
on which rungs are finite, never on how well the hand fits.

**The rung histogram.** There are 1,904 `.rule(` call sites in `src/bidding`
(`grep -rc "\.rule(" src/bidding --include=*.rs`, summed). Among the 1,819
whose weight is a numeric literal (the rest name a constant or a ladder
expression such as `100 + major_bonus`), the rungs are:

| rung | sites | rung | sites | rung | sites |
| --- | --- | --- | --- | --- | --- |
| 100 | 403 | 110 | 65 | 20 | 32 |
| 0 | 211 | 145 | 54 | 155 | 31 |
| 150 | 133 | 160 | 54 | 190 | 29 |
| 120 | 122 | 200 | 51 | 180 | 25 |
| 130 | 115 | 90 | 48 | 125 | 20 |
| 50 / 140 | 94 each | | | 300 | 5 (3 in the book, 2 in tests) |

(paren-balanced parse of every `.rule(` call, weight = second argument; the
long tail below 20 sites is omitted)

Adjacent rungs sit **5-50 centinats** apart: 0.05-0.5 nat. At 0.05 nat the
softmax ratio between neighbours is `e^0.05 ≈ 1.05`, a coin flip; at 0.5 nat it
is `e^0.5 ≈ 1.65`, still 38/62. Softmax calls nearly every neighbouring pair a
near-tie. Argmax is strict at any positive gap. The authors sized the rungs
for the argmax, which is why the ladders work; the "300 is near-deterministic"
sentence describes a spacing only three book sites use (the two 2♣ openings
and one 2NT response), plus two at 320 in choice-of-games.

**The milestone example.** A `3NT` or `4M` rule at a node outranks the suit
rungs beneath it — when it is admissible it must win, that is the whole point
of authoring it above them — yet it is admissible for a small minority of the
hands that reach the node. Precedence order is not frequency order. A softmax
that read the rung as a log-odds would put the game bid's mass *above* the
suit bids' at every hand where it is finite, and at zero everywhere else; the
node's true call frequency is neither. One number cannot carry both an order
over rules and an odds over hands, and the book only ever needed the order.

## 2. Consumer inventory

Every reader of a `Logits` value in the tree, classified by what it needs
from the number. Files edited this session are cited by function name.

| class | site | what it reads |
| --- | --- | --- |
| precedence | `src/bidding/table.rs:105-119` `select_with_legal_state` | first-max argmax (strict `>`) over finite legal logits, default Pass. **Production.** |
| precedence | `src/bidding/rules.rs:1128-1136` `stronger_siblings` | `plan.weight > threshold`, strict |
| precedence | `src/bidding/inference/projection.rs:143-192` | the exclusion fold: `rule.weight() > floor`; `exclude_siblings` skips `*weight <= threshold` |
| precedence | `src/bidding/rules.rs:582-593` `explain` | earliest rule wins a tie (strict `>` in authored order) |
| precedence | `src/bidding/rows.rs:1441` `weight_tie_report` (`cfg(test)`) | a tie is reported, never resolved by magnitude |
| magnitude | `src/bidding/sampler.rs:88` `const MARGIN: f32 = 3.0`, read at `:282` in `made_plausibly` | `logits[made] >= best - MARGIN`; gated at `:264` on `policy.authored_at`. Reached only through `sample_layouts_replay` (via `rules_accept`, `sampler.rs:153`): callers `src/bidding/ev.rs:87` (the search), `examples/probe-replay-yield.rs:220`, `examples/eval-evaluator/main.rs:189`, and the unit test `src/bidding/sampler/tests.rs:219`. Not on the default bidding path. |
| magnitude | `src/bidding/instinct.rs:3572` `const PASS_DEMOTION: f32 = 3.0`, applied at `:3705` inside `competitive_gate` (`:3669`) | subtracts 3 nats from Pass; doc says "sized in the book's ~3-nat convention". `competitive_gate` is called only from `src/bidding/neural_floor.rs:116` and `:148`, i.e. on the **net's** logits. **The one default-path magnitude consumer**, and it reads the net's scale, not the book's. Never calibrated. |
| magnitude | `examples/dump-teacher/main.rs:970-976` | masks illegal calls to `-inf`, then softmax: the training target is a softmax of the *teacher's* logits. Under `--teacher american` (`:667`, `:813` build `american_instinct().bind()`; `:666`, `:812` `dutch_instinct()` when the system is Dutch) that would be a softmax over book rungs. Every shipped corpus is `--teacher bba` (`scripts/*.sh` contain only `--teacher bba`), whose labels are one-hot: `examples/common/oracle/mod.rs:1195-1199` `one_hot(call)` is `0.0` for the call and `-inf` elsewhere. |
| magnitude (monotone) | `src/bidding/rows.rs:1177-1184` `classified()`, at the two stolen-relay transplants `american/defense/gladiator.rs:174` and `american/competition/lebensohl.rs:3598` | classifies with the uncontested table, moves the `2♣` logit onto `X` and sets `2♣` to `-∞` ("X inherits 2♣ exactly"); the reading side is the `rebase` at `gladiator.rs:154` and the reader (`src/bidding/inference/readers/tests.rs:515` `gladiator_stolen_relay_double_is_read_as_the_relay`). One entry relabelled, the rest untouched; ranks preserved, so magnitude-safe. (`classified()` is also used for Context-reading tables in `backstop.rs` and `negative_double.rs`, which transplant nothing.) |
| display | `web/src/lib.rs` `top3()` | **was**: `logits.softmax()` on the unmasked array, legality filtered afterwards, so an illegal call's mass sat in the denominator; printed percentages at authored nodes too. Fixed this session (§4 display rule). |
| display | `examples/practice-bidding/main.rs`, the top-3 block in the human-turn branch | same defect, same fix. |

**The finding.** No default-path consumer reads a *book* magnitude. The
production selector is a strict first-max; the reading engine compares rungs
for order; the sampler's margin is off the default path and gated to authored
nodes; `PASS_DEMOTION` is on the default path but only ever meets the net's
logits. Book and net logits never coexist in one `Logits`:
`src/bidding/common.rs:63-91` `with_floors` attaches the floor as the root
`Always` fallback ("resolution reaches the root last, so the floor never
overrides an authored rule"), and `src/bidding/trie.rs:527-538`
`resolve_floored` returns the exact node's logits iff `has_mass()`, else walks
the fallback chain from depth 0. A `Logits` value is therefore *either* rungs
*or* net outputs, never a mixture, and the "the net must match the book's
scale" premise (§5) had no consumer that would ever compare the two.

At the session-1 audit, no temperature existed anywhere. Session 2 added the
held-out fitter in `trainer/src/calibrate.rs` and its weights-sidecar fields;
serving still reads neither, and `src/bidding/array.rs:373-383`
`Logits::softmax` remains max-subtracted with no scale parameter. The routing
facts the rest of this doc leans on: `Bidder::authored_at`
(`src/bidding.rs:141-157`,
default `true`; its doc says the replay sampler enforces its reading only at
authored nodes) is overridden by `Partnership` at
`src/bidding/book.rs:1249-1252` and by `Trie` at `src/bidding.rs:236-247`;
`Table::classify` (`src/bidding/table.rs:167-175`) does the seat rotation and
`relative(self.vul, seat)`; `Table::authored_at` (`:194`) is the routing twin
the source names (of `Bidder::authored_at`: a fact about the node),
`Table::classify_with_provenance` (`:280`) returns the logits with their
`Provenance`, `Provenance::is_authored` (`src/bidding/trie.rs:174`, the one
predicate `Trie::authored_at` also applies) is the per-hand test the displays
read, and `Table::infer` (`:307-315`, `Partnership`-only) repeats the same
rotation.

## 3. Three coarsenings

A **calibrated logit** at a node is `log P(reference call | σ(h), node)`, up to
one additive constant per node (softmax is shift-invariant), for
some coarsening `σ` of the hand `h`. The reference is BBA, and BBA is
deterministic: given the full hand it makes exactly one call, so `P(call | h)`
is one-hot and there is nothing to calibrate. Probability only appears once
you throw information away — and what appears is a property of a *population*:
`P(call | σ(h), node)` is the pushforward, through `σ` and BBA, of the corpus's
deal process (deals dealt, bid by the table's bidders up to the prefix, and
reaching the node), not of the plain deal prior. Change the population (the
enriched slam slice, the replay cells) and the conditional changes with it, so
a calibrated number belongs to a (net, corpus) pair, never to the net alone.
Three choices of `σ`, finest to coarsest:

| | σ(h) | P(call \| σ(h), node) | who wants it |
| --- | --- | --- | --- |
| (i) | the full hand | one-hot: BBA's call with mass 1 | the corpus label; the production argmax |
| (ii) | a summary: the net's 176 input features, or the book's admissible set at the node | the odds that a sampler or a search needs — "given what a bidder could see of this hand, how likely is each call" | replay sampler, rollout harness, displays at floor nodes |
| (iii) | nothing | `P(call \| node)`: the node's raw call frequency over the deal population | audits, node histograms |

The book's rungs live in none of these rows. They are an order over *rules*,
which is what §1 showed: a rung is not a function of the hand beyond the crisp
admit/reject.

**Why the net already estimates (ii).** Training minimises cross-entropy
against one-hot labels: for a deal with label `c*`, the loss is
`-log softmax(z)[c*]` where `z = W₃·h₂ + b₃` is the last of the net's three
affine maps (`src/bidding/neural.rs` `forward`: 176 → 256 → 256 → 38, ReLU
between; `h₂` is the second hidden activation — a single `W·features + b`
would be multinomial logistic regression).
Cross-entropy is a *strictly proper scoring rule*: over the population of hands
sharing the same `σ(h)` (the same 176 floats), the expected loss has a unique
minimiser, the population conditional `P(c | σ(h), node)`; on a finite corpus
the empirical-risk minimiser is the empirical conditional frequency of the
labels. Both assume the function class can represent that conditional — a
two-hidden-layer MLP over 176 inputs approximates it, so "at the optimum"
below hides an approximation error as well as the finite-corpus one. So the
v6 net's softmax, at the optimum, *is* `P(BBA's call | features, node)`, row
(ii) with `σ` = its feature map. The label is one-hot per deal; the target the
optimiser converges to is a distribution, because many hands map to the same
features (the hand block is ten floats — per-suit length and HCP, total HCP,
the upgrade — `push_hand`, `src/bidding/features.rs:126`; the rank of a spot
card never reaches the net). No teacher scale is inherited (the teacher gave a
one-hot), and none is needed.

"At the optimum" is the catch. A net trained to convergence on a finite corpus
is over-confident on held-out data: its argmax is right as often as the corpus
says, but its top softmax mass is higher than its hit rate. Guo, Pleiss, Sun,
Weinberger, "On Calibration of Modern Neural Networks", ICML 2017, measured
this across architectures and found one scalar fixes most of it:

```text
p = softmax(z / T)        T > 0, one scalar per net
```

fitted by minimising held-out negative log-likelihood over `T` alone — on the
legality-masked logits, the distribution every consumer in §4 reads (the
trainer's cross-entropy is unmasked: the label is legal by construction, the
net's illegal-call mass is not). One scalar cannot overfit a split of this
size, but the ECE reported *after* the fit is in-sample for `T`; report it on
fresh deals ([../pdd-bank-ledger.md](../pdd-bank-ledger.md)). Dividing
every logit by the same positive `T` leaves the order unchanged, so **argmax is
`T`-invariant** and serving needs nothing new; only a consumer that reads a
magnitude (a margin, a demotion, a percentage) sees `T`. `T > 1` flattens
(the net was over-confident), `T < 1` sharpens. In this tree the whole
mechanism is `Logits::softmax` in `src/bidding/array.rs` preceded by one
scalar division: the matmuls are untouched.

The report card is **expected calibration error**. Bin held-out predictions by
their top softmax mass (confidence); in each bin `b` with `n_b` rows, compare
the fraction the argmax got right (`accuracy_b`) with the mean confidence
(`confidence_b`):

```text
ECE = Σ_b (n_b / N) · |accuracy_b − confidence_b|
```

A perfectly calibrated net has `ECE = 0`: when it says 70%, it is right 70% of
the time. Fitting `T` typically cuts ECE by a factor of several without moving
a single argmax; the bridge reading is "when the net says `3NT` at 70%, the
replayed BBA hands actually bid `3NT` 70% of the time at that node". That
ECE scores only the *top* label. The consumers in §4 read the whole row —
the sampler's weight multiplies the mass on the *made* call, often the net's
second or third choice — so the training report carries the held-out NLL
beside ECE (it is what `T` was fitted on, and it scores every entry), and a
class-wise ECE (the same bins per call, over every call's mass) once the
sampler's weights are tuned from the report.

## 4. The design

Fifteen decisions, all accepted, expanded from the interview's settled table.

**Precedence stays authored.** The `i16` rungs, all 1,904 sites, the strict
first-max selector, the exclusion fold: untouched. No second "probability"
column beside the weight. The book's job is the order, and §2 shows nothing on
the default path ever needed more from it.

**The proposal hook.** Where a consumer needs odds at an *authored* node (the
book has finite mass, so the floor is shadowed), the odds are the **net's**
softmax at temperature `T`, restricted to the book's admissible calls at that
node (the calls with finite rungs that are also legal at the node — a fallback
book can offer a call the auction no longer allows, per `made_plausibly`'s
comment in `src/bidding/sampler.rs`), renormalised over that set: with `A` the
admissible set and `p = softmax(z / T)`, the hook returns `p_c / Σ_{a∈A} p_a`
for `c ∈ A` and `0` outside, i.e. `P(call = c | call ∈ A, features)`. That
conditions on the *event* that BBA's call lies in our book's admissible set,
not on the admissible set as a feature of the hand (row (ii)'s second `σ`);
the two coincide only where BBA and the book agree on admissibility, which is
what the audit probe counts. When the
restricted mass is ~0 (below an epsilon: the net put essentially nothing on
any admissible call), fall back to the book's one-hot argmax. The hook calls
the v6 forward pass **directly**, not through the floor shell: the constructive
floor is the ladder (`with_floors`, `src/bidding/common.rs:63-91`), so going
through the shell at a constructive node would return rungs again, which is
the thing being replaced. Scope covers both scales, the book's (never a
magnitude) and the net's (`PASS_DEMOTION` is its one default-path reader).

**The display rule.** Where an *authored* node answered the hand (read off the
resolution provenance via `Table::classify_with_provenance` and
`Provenance::is_authored`,
not off the node: an authored node that rejects the hand falls through to the
floor), the display shows the **ladder**: the legal finite calls in precedence
order with their rung values in nats, no percentages, because a percentage there
would be a softmax of an order. Where the floor answered, it shows percentages
from a softmax taken **after** masking illegal calls to `-inf`, so the
denominator holds only calls the seat can make, and at temperature `T` once
session 2 has fitted one — until then the percentages are the raw softmax,
over-confident by the amount §3 describes: hygiene, not yet odds. This is the
session-1 fix to `top3()` and the practice-bidding block, and it is hygiene, not
a bidding change: the selector never read the display.

**The replay sampler's end state.** Today `made_plausibly`
(`src/bidding/sampler.rs:282`) accepts a layout when every non-actor's made
call sits within 3 nats of that seat's best legal logit, a hard margin. The
end state replaces the margin with an **importance weight**: the likelihood of
the observed auction under the proposal,

```text
w(layout) = Π over non-actors' authored decisions of P(made call | that seat's hand in the layout, prefix)
```

with `P` the restricted, renormalised, temperature-`T` softmax from the hook.
In importance-sampling terms the layout *proposal* is the constrained deal
draw (the prior given the actor's hand), the *target* is the posterior given
the auction, and `w` is their ratio — the likelihood of the observed calls
under the hook's model; the hook supplies the target's likelihood factor, it
is not the proposal. A non-actor's decision at a floor node contributes a
factor of 1, matching today's abstention (`sampler.rs:264`); weighting it by
the net's unrestricted softmax instead is a separate, reversible choice,
default off.
The trade-off is yield against variance: a hard margin discards layouts and
keeps the survivors equal-weighted; importance weights keep every layout but a
heavy-tailed `w` collapses the effective sample size (`(Σw)² / Σw²`) toward a
handful of layouts. `examples/probe-replay-yield.rs` is the tuning tool for
both, and until a search consumer is live the hard 3-nat `MARGIN` stays,
documented as a yield heuristic rather than a calibrated quantity.

**The rollout harness.** The KR1 gate for everything above. At a decision
node take the proposal's **top-k (k = 3)** calls, deterministic (no sampling);
roll each out with our policy against BBA (the anchor harness of
[../bba-gap-campaign.md](../bba-gap-campaign.md)); score by both **DD and PD**
([../measurement.md](../measurement.md)). The **paired baseline** is the
policy's own call's return on the same deal and prefix, so a call is credited
only for beating what we would have done anyway, on the same board. The
**target rule**: the training label becomes the EV-argmax one-hot where its
gain over the baseline clears a margin, else BBA's label stands. Two words
in that rule are load-bearing. *EV* is an expectation over the 39 cards the
actor cannot see: a candidate's return on the one dealt layout is a
double-dummy result, and the call that wins it is the one that happened to
fit the hidden hands, so a per-deal argmax labels the net with information
it never has at the table. Averaging over the corpus does not repair this —
cross-entropy on per-deal winners converges to "how often each call wins",
a plurality vote whose argmax is not the argmax of expected IMPs. Each
candidate is therefore scored as the mean return over `M` layouts drawn
consistent with the actor's hand and the prefix (the replay sampler's job,
which makes the sampler a prerequisite of the harness rather than its
follow-on), and the margin guards the estimate's noise, not the missing
expectation. *Baseline* means the policy's own call is always in the
candidate set: at an authored node it is the book's argmax, which need not
sit in the net's top-3 (the disagreement the audit probe counts), so the set
is top-3 ∪ {own call} and the own call's rollout is the paired control. The
fallback is asymmetric by design: when the own call beats BBA's label by more
than the margin the label still reverts to BBA — the teacher stays the
authority off the margin. Reversible default; the retrain's A/B may overturn it.
This is what M5.2's arm 3 lacked ([plan.md](plan.md) M5.2, refuted 3/3, parked on
`park/lstm-floor`): it reweighted the one BBA-vs-BBA trajectory per deal by
`exp(0.10·A)` with `A = imps(result − par)` and credited the terminal
advantage to every decision on it, having never sampled an alternative
auction. The harness samples the alternatives, and pairs them.

*Built in session 3* as `examples/probe-rollout-label`. Five scope choices are
explicit in its module doc. It defaults to **self-play**, because `BbaOracle`
creates and destroys a native bot for every call (`with_bot`,
`examples/common/oracle/mod.rs:457`) and the replay sampler selects worlds under
our policy. `--opponent bba` was **measured** rather than left assumed (§4a
below): it is affordable and it changes no conclusion. It uses **`ns_score_pd`**, not
`ev_all`'s call-only `ns_score_bid`, because these are complete auctions that
may contain a real double. No shipped sidecar yet carries a fitted `T`, but
temperature cannot reorder the candidates; it changes which decisions clear
the epsilon gate and therefore enter the evaluated population. Finally, session
3 is **neither-vulnerable only**; a relabelling run must add the vulnerability
axis before training. The work runs in four phases: rayon harvest, rayon layout
draws, one main-thread `solve_deals` over the flattened layouts, then rollout
and pricing. Each layout is solved once and shared across every candidate.

**The winner's curse is the finding.** The target rule selects the largest of
`k` noisy `M`-layout estimates and reports that same estimate. The maximum is
upward-biased even when every candidate is worth zero. The harness therefore
splits its layouts into **two independent pools of `M`**: select on the first,
then price that same call on the second. Mechanically this is *one* `2M` draw
from `sample_layouts_replay` halved at the midpoint, not two sampler calls —
`sample_with` is plain rejection sampling, so conditional on the draw filling,
the accepted layouts are i.i.d. and the halves are exchangeable. The held-out continuous mean is unbiased
for that `M`-layout selector; the thresholded held-out firing rate is not itself
an unbiased population fraction. Short replay draws are skipped rather than
topped up from a different distribution, board and layout RNG streams occupy
disjoint seed ranges, and every ± below is a 95% interval clustered by source
deal.

Same 400 deals, seed 1, `k = 3`, `M` alone moving (631 usable decisions per row;
2 of 633 replay draws were short):

| `M` | in-sample DD | **held-out DD** | curse | in-sample PD | **held-out PD** | curse |
| --- | --- | --- | --- | --- | --- | --- |
| 8 | +0.449 ± 0.063 | **−0.104 ± 0.089** | 0.553 | +0.774 ± 0.096 | **+0.158 ± 0.124** | 0.616 |
| 32 | +0.247 ± 0.045 | **+0.044 ± 0.055** | 0.203 | +0.519 ± 0.072 | **+0.225 ± 0.084** | 0.294 |
| 128 | +0.162 ± 0.036 | **+0.105 ± 0.037** | 0.057 | +0.394 ± 0.061 | **+0.329 ± 0.066** | 0.065 |

At `M = 8`, the held-out scorers disagree: DD is negative while PD is positive,
and both intervals exclude zero. At `M = 32`, the small run finds PD regret but
cannot distinguish the DD value from zero. At `M = 128`, both held-out intervals
are positive and the curse has shrunk sharply. A margin can trade false
positives for false negatives, but cannot debias reusing the selection layouts;
**`M` and independent validation are separate constraints.** The larger
headline census (2,000 deals, `M = 32`, 3,169 usable decisions) makes the small
DD signal visible: in-sample +0.2499/+0.5434 DD/PD, held-out **+0.0362 ± 0.0244 /
+0.2446 ± 0.0372**.

The same comparison as a relabel rate at `margin = 0.25` IMPs:

| run | rule | in-sample | held-out |
| --- | --- | --- | --- |
| headline, `M = 32` | plain DD | 23.76% | **13.03%** |
| | perfect defense | 38.88% | **24.61%** |
| | both | 20.98% | **11.01%** |
| `M = 128` | plain DD | 16.32% | **12.68%** |
| | perfect defense | 31.06% | **27.26%** |
| | both | 13.95% | **10.78%** |

The headline both-scorer rate is **47.5% lower** on held-out layouts. The two
firing sets are not nested, so that is a rate comparison, not a claim that 47.5%
of particular labels are false. It puts the plausible relabelled population
near 11% of choice-bearing decisions, about 3–4% of all authored decisions.

At `M = 128` the held-out advantage is positive on **both** scorers, so the
signal is not merely a perfect-defense doubling artifact. PD is about three
times plain DD. The headline's **in-sample** node table has Pass as the plurality
target among its 20 printed top swaps (56, ahead of 1NT's 36), while
`1NT -> 1♦ x23` is the largest single entry. That exploratory breakdown is not
held-out validated, so it does not establish where the debiased signal lies.

**Cost is almost entirely double dummy.** The headline spends 0.0 s walking,
0.6 s sampling, 4.0 s rolling out, and **1,479.2 s solving 202,816 layouts**:
7.29 ms/layout and 99.7% of wall clock. The DD solve budget is independent of
`k` because each solve is shared; the small rollout remainder still scales with
the number of candidates. Only 34% of authored decisions offer more than one
admissible call (1.60/deal here versus `probe-book-vs-net`'s 4.72). At `M = 128`,
selection plus independent validation costs about 1.87 s/decision: a
100k-decision audit is ~52 box-hours and a 2.3M-decision full pass ~50 box-days.
Selection alone is half that price but cannot validate its own labels.

**The opponent does not move the finding.** The module doc's fear — that a
serialised `BbaOracle` makes `--opponent bba` unaffordable — is wrong, and
measuring it costs two minutes. Same 400 deals, seed 1, `M = 8`, the *only*
change being who bids the other three seats during the rollout:

| opponent | in-sample DD | **held-out DD** | in-sample PD | **held-out PD** | both-rule held out | wall clock |
| --- | --- | --- | --- | --- | --- | --- |
| self-play | +0.4487 ± 0.0633 | **−0.1040 ± 0.0891** | +0.7740 ± 0.0956 | **+0.1583 ± 0.1237** | 10.62% | 92.2 s |
| BBA | +0.5097 ± 0.0689 | **−0.0323 ± 0.0918** | +0.8354 ± 0.1019 | **+0.2074 ± 0.1316** | 12.04% | 134.8 s |

Every held-out interval overlaps its twin, both scorers keep their sign, and the
curse is the same size (0.54/0.63 against self-play's 0.55/0.62). The rollout
phase does go 0.3 s → 39.8 s (133×, one bot spawn per opponent call), but double
dummy still owns 69% of the clock, so the arm costs **1.46× wall clock**, not the
feared blow-up. Both arms harvest the identical 631 decisions, because the
opponent enters only at bid-out time: the walk, the candidate sets and the
layouts are shared. **Session 4 does not need to re-litigate self-play versus
BBA at this `M`** — it is decided by measurement, and cheap enough to re-check
at whatever `M` the production gate lands on.

The epsilon census is unchanged in direction: `thin` is 5/638 at `T = 1`,
1/638 at `T = 1.5`, and 0/638 at `T = 2`. That population requires at least two
admissible calls; `probe-book-vs-net` also counts single-rung nodes, explaining
why its rate is higher. Six of 3,175 headline candidate decisions (0.19%)
returned a short replay draw and were skipped explicitly.

**`PASS_DEMOTION` → `3·T`.** The demotion is currently 3 raw nats on the net's
logits, a magnitude read on an uncalibrated scale. Restating it as `3·T` nats
makes it a fixed *probability ratio* (`e^3 ≈ 20:1` at calibrated odds) rather
than a fixed logit gap. This lands inside the flip plan's collar-retune A/B
(plan.md M5.2 flip plan arm 1, [competitive-accountant.md](competitive-accountant.md)),
not before: it moves calls, so it needs the decision table, and it shares the
retune's measurement.

**The `T` that restatement needs is now measured: `T = 1.1298` for the shipped
`american_bba_v6`** (session 4, 2026-09-04), so `3·T` = **3.389 nats** — a 13%
deepening of the demotion, not the 2-3× a badly over-confident net would have
implied. Session 2's fitter runs at the *end* of a training job, so a shipped
artifact had no temperature until something loaded it back; `--weights-in` on
the trainer is that mode. It reads `<stem>.f32` into the same model, skips
training, and runs the existing evaluate → fit → export path, so the number is
produced by exactly the code that would have produced it in the original run:

```sh
cd trainer && ./target/release/pons-trainer \
  $(python3 -c "import json;print(' '.join('--data '+s for s in \
     json.load(open('../src/bidding/weights/american_bba_v6.json'))['data_stems']))") \
  --weights-in ../src/bidding/weights/american_bba_v6 \
  --weights-out /tmp/v6-cal --fixture 0
```

On v6's own held-out tail (676,829 rows, the split the sidecar records), the fit
reads **NLL 0.3010 → 0.2984** and **ECE 0.0117 → 0.0015**, an 8× calibration
improvement for a 13% rescale. The load is provably the shipped artifact:
re-exporting it is byte-identical, and `val_top1` reproduces the sidecar's
0.8928 / 0.8890 / 0.8950 exactly (`val_ce` moves in the last ulp only, from
`evaluate`'s minibatched accumulation). The auxiliary DD value head is **not** in
`PARAM_NAMES` and therefore not in the blob, so it loads at its random init and
a `--weights-in` sidecar reports `val_dd_mse` as null rather than as noise.

The shipped `american_bba_v6.json` is deliberately **left untouched**: it is a
provenance record of a training run that did not fit a temperature, and quietly
back-filling four calibration fields into it would make a later fit look like
that run's own output. The number lives here; folding it into the sidecar is a
one-line edit if jdh8 wants it there.

**Trainer side.** After training, fit `T` on the held-out split by NLL,
report ECE before and after, and write a `temperature` field into the weights
sidecar. Serving stays raw: argmax is `T`-invariant, so the default system is
byte-identical and the smoke hash is the gate.

**The audit probe.** `examples/probe-book-vs-net`: walk deals, and at
every authored node compare the book's argmax with the restricted net's
argmax; bucket the disagreements by node. This is the first place the two
orders are ever put side by side, and it prices the epsilon fallback (how
often is the restricted mass ~0) before any consumer depends on it. Built in
session 2 as a self-play `american()` walk at neither vulnerable (both books
reached), calling `classify_bba_v6` directly rather than through the floor
shell, exactly as the hook does. Renormalising cannot reorder, so **the
disagreement census does not depend on `T`** — only the mass does.

The first census, `-c 20000 -s 1` (94,340 authored decisions over 1,109
distinct nodes):

| quantity | value |
| --- | --- |
| book argmax == restricted-net argmax | **91.28%** (8,231 disagreements) |
| admissible mass < 1e-6 / 1e-4 / 1e-3 / 1e-2 / 1e-1, at `T = 1` | 0.38% / 1.34% / 2.55% / 5.02% / 9.85% |

The epsilon is a *statistical* threshold, not a numerical one: `f32` softmax
does not underflow anywhere near 1e-6 (a mass of 1e-4 is seven nats below the
mode, and `exp` in `f32` reaches ~e^-88), so what the rung buys is refusing to
renormalise a distribution the net has effectively declined to place.

**It also moves with `T`**, because the mass is measured after the temperature
divide and a flatter softmax puts more of itself on any fixed set. Same seed,
same walk:

| rung | `T = 1` | `T = 1.5` | `T = 2` |
| --- | --- | --- | --- |
| 1e-4 | 1.34% | 0.40% | 0.11% |
| 1e-3 | 2.55% | 1.02% | 0.42% |
| 1e-2 | 5.02% | 2.64% | 1.50% |

So the `T = 1` column is an *upper bound* on how often the fallback fires: a
distilled net calibrates at `T > 1` (it is over-confident, §3), and every rung
loosens as the fitted `T` lands. Size the epsilon at the `T` the hook will run
at — `probe-book-vs-net -t` takes it for exactly this.

**`1e-4`, decided by jdh8 2026-09-03** (recommended in session 2, confirmed
after session 3's census). It is the default of every consumer that reads the
hook — today only `probe-book-vs-net -e` and `probe-rollout-label -e`, both
`1e-4`. The choice between it and `1e-3`
is 1.21% of authored
decisions at `T = 1` (0.62% at `T = 1.5`), and the deciding argument is not the
count but *which way the fallback errs for the consumers that exist*:

* the fallback is the book's **one-hot**, so at a fallen-back node the replay
  sampler's importance weight becomes 0 or 1 — a hard rejection of every layout
  whose non-actor made a call other than the book's argmax. That is precisely
  the hard `MARGIN` behaviour the importance weight was designed to replace, so
  a *higher* epsilon reintroduces the thing being removed on more nodes.
* in the rollout harness a one-hot proposal has no top-3: the candidate set
  collapses to the policy's own call, the node produces no alternative to roll
  out, and the label stays BBA's. A higher epsilon buys fewer labelled nodes.
* the thin nodes **are** the disagreeing nodes (`1♠ - 1NT -`: 29.8% disagree,
  27.8% thin). So epsilon is a dial on how much book/net disagreement the hook
  is allowed to see, and raising it silences the net exactly where it dissents.
  For an audit consumer that is self-confirming.

The one consumer that prefers `1e-3` is a **display**: "the net has no opinion
here" is honest, and a renormalised tail shown as percentages is not. If a
display ever reads the hook it should carry its own, higher rung rather than
raise this one.

Two residues, both cheap and neither blocking: the fallback could be **uniform
over `A`** instead of the book's one-hot (it keeps the sampler soft, and says
"no opinion" without asserting the book's order as odds), and the rung could be
expressed in nats below the mode rather than as an absolute mass, which would
make it `T`-invariant by construction. Neither is built; nothing consumes the
hook yet.

The disagreement is heavily *not* uniform. The opening seat is near-agreement
(2.1% over 37,208 decisions, the top swap our `1NT` against BBA's `1♦`), while
authored responses run 10-30% and the thin nodes concentrate in the same
places — `1♠ - 1NT -` disagrees 29.8% and is thin 27.8% of the time. Read that
as the honest scope of the hook: at a well-populated node the two orders mostly
agree and the odds are informative; at a narrow authored continuation the net
is being asked about calls it was never trained to rank there.

## 4b. The gate flip: pricing the population production actually asks the net about

[logit-calibration-session4-handoff.md](logit-calibration-session4-handoff.md)
refused to spend §6's ~50 box-days until two doubts were answered, and specified
a ~2-hour experiment to answer them. This is that experiment, run 2026-09-04 at
seed 1 with `scripts/idle-run.sh`, one arm at a time.

**D1 — §4's census prices decisions production never asks the net about.** The
harvest gated on `Provenance::is_authored`, and a book node with finite mass
*shadows* the floor (the architecture's iron rule; mechanically
`Trie::classify_floored` over the depth-0 root fallback `with_floors` installs —
not `Bidder::or_else`, which has no non-test caller, so the handoff's
`compose.rs:74-79` citation names the wrong mechanism for the right behaviour).
So at **100%** of §4's population, production bids the book. Relabelling those
decisions moves the net's weights but not the policy at those nodes; the payoff
would have to arrive entirely by generalisation, and the census measures that at
zero decisions.

**D2 — Pass as the plurality relabel target may be double dummy's signature.**
[../measurement.md](../measurement.md)'s iron rule is that DD is blind to
obstruction and concealment, and an oracle that cannot price what a bid conceals
prefers passing. §4's node table has Pass as the plurality target, and PD runs
~3× plain DD throughout.

### What the flip changes

`probe-rollout-label` gains `--population {authored,net-served}`. `authored` is
§4's arm, unchanged and kept so its numbers reproduce from one binary.
`net-served` harvests only what the floor shell's net answers. `!is_authored()`
is necessary but not sufficient — the root fallback resolves to **three** floors
([`common.rs`](../../src/bidding/common.rs) `with_floors`), and the
deterministic `instinct()` ladder answers two of them:

| where the unauthored decision lands | who answers | net evaluated? |
| --- | --- | --- |
| constructive book (`Phase::Constructive`) | the `instinct()` ladder | no |
| contested book, `forced(context)` | the same ladder, as a rail | no |
| contested book, not forced | `classify_bba_v6` → mask → both gates | **yes** |

so the predicate is `!is_authored() && Phase::of(&auction) != Phase::Constructive
&& !forced(&context) && admissible.len() > 1`. `instinct::forced` is widened
`pub(crate)` → `pub` for it — a visibility change and a doc comment, the same
move session 3 made for `select_legal_call`.

**The proposal is the shell's logits, not a second forward pass.** At a
net-served node the `Logits` `classify_with_provenance` returns *are* the
production distribution: the net after `mask_illegal`, `competitive_gate` (with
`PASS_DEMOTION`) and `new_suit_gate`. Using them directly means the own call is
the shell's argmax by construction, and a gate-vetoed call is `-∞` and can never
be a candidate — correct, because a retrained net's preference for it could not
reach the table through those same gates either. Two consequences: the legality
mask leaves the admissible mass at 1, so `--epsilon` never fires and `thin` is
identically 0; and with `k = 3` over ≥ 2 finite calls the candidate set always
holds an alternative, so `flat` is 0 too. Both columns are still printed.
`--vul` was added in the same change (the playbook's two cells); the raw-net-
versus-shell question — are the gates costing IMPs? — is a separate arm and is
deliberately not folded in.

**The refactor is provably inert on §4's population.** `--population authored
-c 400 -s 1 -m 8` reproduces §4a's row to the last digit: 638 choice-bearing
decisions, `thin` 5, 631 rolled out, 2 of 633 draws short, held-out
**−0.1040 ± 0.0891** DD and **+0.1583 ± 0.1237** PD, both-rule held out 10.62%.

### The population is three times *denser*, not thinner

The handoff expected the net-served slice to be thin ("contested, unauthored,
unforced"). It is the opposite: **4.81–4.87 choice-bearing decisions per deal**
against the authored census's 1.60. The walk is self-play, so it harvests all
four seats, and the defensive book — *they* opened, we act — is the thinnest
part of the authored system, so most of its decisions fall to the floor. The
`M = 128` rows therefore needed `-c 200`, not the ~1,500 deals the handoff
budgeted against a guessed-lower density.

### The rows

Seed 1, `k = 3`, `margin = 0.25` IMPs, self-play, ± is a 95% interval clustered
by source deal.

| population | `M` | deals | decisions | in-sample DD | **held-out DD** | in-sample PD | **held-out PD** | curse DD/PD |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| authored (§4a) | 8 | 400 | 631 | +0.4487 ± 0.0633 | **−0.1040 ± 0.0891** | +0.7740 ± 0.0956 | **+0.1583 ± 0.1237** | 0.553 / 0.616 |
| authored (§4) | 128 | 400 | 631 | +0.162 ± 0.036 | **+0.105 ± 0.037** | +0.394 ± 0.061 | **+0.329 ± 0.066** | 0.057 / 0.065 |
| net-served, none | 8 | 400 | 1,931 | +1.1066 ± 0.0750 | **+0.4476 ± 0.0968** | +0.9544 ± 0.0837 | **+0.2520 ± 0.1034** | 0.659 / 0.702 |
| net-served, none | 128 | 200 | 956 | +0.7175 ± 0.0844 | **+0.6134 ± 0.0821** | +0.5483 ± 0.0912 | **+0.4641 ± 0.0874** | 0.104 / 0.084 |
| net-served, both | 128 | 200 | 919 | +0.7838 ± 0.1115 | **+0.6841 ± 0.1066** | +0.6456 ± 0.1065 | **+0.5626 ± 0.1011** | 0.100 / 0.083 |

Both `M = 128` cells land in the **first row** of the handoff's verdict table:
positive on both scorers, every interval excluding zero, the two vulnerability
cells overlapping each other (so the signal is not the competitive book's
vulnerability axis), and the D2 line clean. The `M = 32` tie-break row the
handoff queued "only if the two `M = 128` rows disagree" was therefore **not
run**. The reading is: **the net's own population carries the signal, and it is
about six times the size of the signal on the population §4 measured**
(+0.61/+0.68 held-out DD against +0.105). Session 4 earns its double dummy —
restricted to the net-served slice, which is the whole point of D1.

Two things the table does not say. The `M = 8` net-served row is 400 deals and
the `M = 128` rows are 200, so the within-population `M` comparison is not on
identical deals (the authored rows are all 400). And the held-out relabel rates
are much higher here than on the authored population — 17.15% / 19.59% under the
both-scorer rule against 10.78% — so the slice is not only denser in decisions
but denser in *labelled* decisions, which is what the 50-box-day estimate was
priced against.

### D2, answered — and it inverts across the two populations

The report adds, per rule, the Pass share of the held-out relabel *targets*
against the Pass share of the own calls they displace, over the population's own
base rate. A relabel needs `winner != own`, so the two columns are disjoint per
decision: read them as a **flow into and out of Pass**.

| population, `M = 128` unless noted | rolled-out decisions already passing | rule | → Pass | Pass → |
| --- | --- | --- | --- | --- |
| authored, `M = 8` | **1 of 631 (0.16%)** | plain DD | 46.67% | 0.00% |
| | | perfect defense | **64.44%** | 0.00% |
| | | both | 47.76% | 0.00% |
| net-served, none | **764 of 956 (79.92%)** | plain DD | 9.04% | 78.25% |
| | | perfect defense | 27.88% | 57.96% |
| | | both | 18.29% | 65.24% |
| net-served, both | **752 of 919 (81.83%)** | plain DD | 13.31% | 74.35% |
| | | perfect defense | 28.63% | 59.83% |
| | | both | 22.22% | 65.56% |

On the **authored** population D2's fear is confirmed outright: essentially
nothing passes there (1 decision in 631), yet Pass is the target of 46.7% of
plain-DD relabels and **64.4%** of perfect-defense ones. That is the blind spot
exactly as [../measurement.md](../measurement.md) predicts it, and PD — the
scorer that also gets to double — is the worse offender. §4's "Pass is the
plurality target" line was not an artifact of the in-sample breakdown.

On the **net-served** population the flow reverses. Four in five of those
decisions already pass, and the relabels overwhelmingly move *out* of Pass: DD
targets Pass on 9.0% of relabels while displacing a Pass on 78.3%. Even PD's
27.9% target share sits far below the 79.9% base rate. The signal here is not
"pass more"; it is "the floor is passing where the table can compete", which is
a floor gap, not an oracle artifact.

The two other order-reversals point the same way. On the authored population PD
runs about 3× plain DD; on the net-served population **plain DD is the larger of
the two** (+0.613 vs +0.464 at `M = 128`, neither vulnerable), so the value is
not a perfect-defense doubling artifact. And the winner's curse is smaller here
at every `M` (0.10/0.08 against 0.06/0.07 authored at `M = 128`, but 0.66/0.70
against 0.55/0.62 at `M = 8`), because the per-decision signal is larger.

### Where the mass sits

Both `M = 128` node tables are dominated by fourth-seat decisions over an
opponent's **1NT** auction — `1NT - 2♥`, `1NT - 2♠ - 2NT`, `1NT - 2♦ - 2♥`,
`1NT - 2♣` — with `P -> X` and `P -> 3m` as the recurring swaps. Both-vulnerable
adds a strong-club cluster (`2♣`, `2♣ - 2♦`, both `P -> 2♠`) and the `1NT 2♥ X`
lane of [../one-notrump-competitive.md](../one-notrump-competitive.md). Read against
the self-play walk this is our own defensive book being asked to compete over
our own 1NT structure, and passing. It is a **floor-gap report** as much as a
calibration result, and it belongs to
[../competitive-book.md](../competitive-book.md) and
[../one-notrump-competitive.md](../one-notrump-competitive.md) whatever the
relabelling programme does next.

### Cost

`M = 8` at 400 deals: 30,896 layouts, 303.6 s. `M = 128` at 200 deals: 244,736
layouts in 2,248.4 s neither-vulnerable and 235,264 in 1,753.0 s both — 9.2 and
7.5 ms/layout, the spread being how busy the shared box was, against §4's
quiet-box 7.29 ms. Double dummy is 99.2% of wall clock, as §4 found. The whole
sequence — sizing row, two `M = 128` cells, and the authored regression check —
cost about **76 minutes**, inside the handoff's two-hour budget.

## 4c. The corpus population: BBA's walk, BBA's label, and the re-priced pass

§4b bought the right to spend double dummy on the net-served slice. Session 5
spends the first hour of it on the population that would actually be
relabelled, then specifies the rule and re-prices the pass.

### The walk *is* the corpus, so there is no corpus reader

The session-4 handoff assumed a corpus-fed mode meant reading `.f32` rows back.
It cannot: a v6 row carries a ten-float *disclosable* hand summary and no board
id (`src/bidding/features.rs:1287-1304`), so neither the hand nor the deal is
recoverable from a row. What makes the mode tractable is the other direction —
`dump-teacher`'s walk is exactly reproducible:

* deals come from the `.pdd` bank strictly in order, `deal index == board index`
  (`examples/dump-teacher/main.rs:920-937`);
* dealer and vulnerability are two `StdRng` draws per board, taken *before* the
  enrich filter can skip anything (`:826`, `:924-925`), and `rand` is pinned at
  `0.10.2` across HEAD and both corpus shas;
* the auction advances by the teacher's legal argmax, and under `--teacher bba`
  the target is a strict one-hot — so **the row's label is the call that
  advances the walk** (`:964-980`, `:1044-1047`).

`probe-rollout-label`'s pricing half never needed the corpus either: it consumes
only *(hand, seat, dealer, prefix)*, because `sample_layouts_replay` re-draws
the other three hands from our own inferences. So the corpus-fed mode is one
flag on the **walk**, not a file format.

`--walk {self,teacher}`, default `self` so §4 and §4b reproduce. Under
`teacher` the auction advances by BBA's legal argmax at all four seats —
`dump-teacher`'s own walk — and `own`, the paired baseline, is **BBA's call**:
the label a relabel would overwrite. BBA's call may fall outside `admissible`
(our floor gives it `-inf`); that is legal, informative, and all `advantages`
needs. The binary also gained the four-way **slice histogram** over every
choice-bearing decision, which is the denominator a corpus pass is priced
against.

### The slice mix

Every choice-bearing decision resolves through one of four floors, and only the
last consults the net. Paired: seed 1, the same 400 deals, both walks.

| walk | choice-bearing/deal | authored | constructive-floor | forced-rail | **net-served** |
| --- | --- | --- | --- | --- | --- |
| self | 6.71 | 23.8% (1.59/deal) | 1.6% | 2.1% | **72.5%** (4.87/deal) |
| teacher | 7.09 | 20.9% (1.49/deal) | 3.8% | 1.7% | **73.6%** (5.22/deal) |

The self row reproduces §4b's 4.81–4.87 net-served decisions/deal and §4's 1.60
authored ones exactly. The headline `M = 128` runs at 200 deals agree (74.7% /
5.28 neither vulnerable, 74.0% / 5.21 both).

**The corpus walk is denser, but only slightly.** BBA's auctions carry 5.7% more
choice-bearing decisions per board and 7.2% more net-served ones, and the
*share* barely moves (72.5% → 73.6%). What changes between the walks is not the
size of the slice but its shape — two sections down.

### The rows

Same seed, same deals, same `M`, same `k = 3` and `margin = 0.25` IMPs as §4b —
the only change is who advances the auction and whose call is the baseline. ±
is a 95% interval clustered by source deal.

| walk, vul | decisions | priced | in-sample DD | **held-out DD** | in-sample PD | **held-out PD** | curse DD/PD | both-rule held out |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| self, none (§4b) | 956 | 956 | +0.7175 ± 0.0844 | **+0.6134 ± 0.0821** | +0.5483 ± 0.0912 | **+0.4641 ± 0.0874** | 0.104 / 0.084 | 17.15% |
| self, both (§4b) | 919 | 919 | +0.7838 ± 0.1115 | **+0.6841 ± 0.1066** | +0.6456 ± 0.1065 | **+0.5626 ± 0.1011** | 0.100 / 0.083 | 19.59% |
| **teacher, none** | 1,056 | 881 | +0.8091 ± 0.1265 | **+0.7483 ± 0.1311** | +0.5855 ± 0.1064 | **+0.5163 ± 0.1095** | 0.061 / 0.069 | 16.57% |
| **teacher, both** | 1,043 | 872 | +0.8962 ± 0.1527 | **+0.8074 ± 0.1496** | +0.6868 ± 0.1292 | **+0.6163 ± 0.1336** | 0.089 / 0.071 | 17.55% |

**The value survives the move to the corpus population, and grows.** Both
teacher cells land in the handoff's first verdict row — positive on both
scorers, every interval excluding zero, the two vulnerability cells overlapping.
Plain DD is again the larger scorer, so the signal is not a perfect-defense
doubling artifact, and the winner's curse is *smaller* here than on the
self-play walk (0.06/0.07 against 0.10/0.08) because the per-decision signal is
larger. D2 stays clean and inverted: 75.60% of priced decisions already
pass, while plain DD targets Pass on only 7.00% of its held-out relabels and
displaces a Pass on 76.68%.

This is the measurement the ledger asked for, and it answers the last standing
doubt about the programme's premise. §4b priced relabelling *our* call in *our*
auctions; a reader could object that BBA's label in BBA's auctions is a
different, harder target. It is not — it is a slightly better one.

### The corpus population is diffuse, and that is the argument for the floor

The two §4b node tables were dominated by a handful of lanes: fourth-seat
actions over our own 1NT (`1NT - 2♥`, `1NT - 2♦ - 2♥`, `1NT - 2♣`). The teacher
walk's is not. Its 881 priced decisions sit at **918 distinct nodes** — more
nodes than decisions. The self-play walk's own `M = 8` row puts 1,931 decisions
at 1,561 nodes, so scaled to a common decision count the teacher walk is about
20% more diffuse again — and its nodes are spread across the opponents' full BBA
constructive repertoire in the balancing and fourth seats:
`1NT - 2♣ - 2♦ - 2NT`, `1♦ - 1♥ - 2♥`, `1♠ - 1NT`, `1♣ 1♠ - - X`. The swaps are the same story as §4b
(`P -> X`, `P -> 3♣`, `P -> 2♠`: the floor passes where the table can compete),
but the *shape* is different, and it settles a design question. Under a
self-play walk both sides bid our system, so the contested nodes the census
visits cluster in our own defensive book and look authorable. Under the corpus's
own walk they do not: **918 nodes for 881 decisions cannot be closed by
authoring**, and the architecture's own rule — a book node with finite mass
shadows the floor — says authoring them would only take the decisions away from
the net. Improving the floor is the only move that scales here, which is exactly
what a relabel is.

### The production label gate

Settled by §4a, §4b and this section; the remaining dials are `M` and `margin`.

| stage | rule | why |
| --- | --- | --- |
| population | net-served only: `!is_authored() && Phase != Constructive && !forced(context)` | D1 — a book node with finite mass shadows the floor, so relabelling elsewhere buys only generalisation |
| proposal | the floor shell's own gated logits (`mask_illegal` → `competitive_gate` → `new_suit_gate`) restricted to the legal set, softmax at `T = 1.1298`; top `k = 3` ∪ BBA's call | re-running the raw net would price a policy that cannot reach the table |
| decline | restricted mass < `1e-4` → keep BBA's label | jdh8's epsilon, 2026-09-03 (§4); both consumers already default to it |
| pricing | `2M` layouts from `sample_layouts_replay`; **select** the argmax advantage on the first `M`, **validate** the same call on the second `M`; both scorers | session 3's iron finding — never relabel from the in-sample estimate |
| fire | the same call wins under both scorers, differs from BBA's, and clears `margin` **on the validation half** under both | measurement.md's non-inferiority reading, applied to a label |
| else | keep BBA's one-hot | a false negative costs only opportunity; a false positive corrupts the target |

Two notes on the shape. The gate is **precision-biased on purpose**: BBA's label
is the status-quo baseline, so a missed relabel is free and a wrong one is not.
And `k` is free — the DD solve is shared across candidates, so raising `k` costs
only rollout time (0.4% of wall clock here); it stays at 3 because more
candidates buys more winner's curse, not more cost.

**Cross-fitting is available and not recommended yet.** Running the rule in both
directions (select on A/validate on B *and* select on B/validate on A, fire only
on agreement) costs no extra double dummy and is strictly stricter than the rule
above. It is the dial to reach for if the first retrain's A/B reads a loss
attributable to label noise; until then it only shrinks an already-small
perturbation and forfeits the unbiased self-estimate of the relabel rate. This
answers the session-4 handoff's open call 2 (cross-fitting versus a larger `M`):
**neither — a smaller `M`**, for the reason below.

### The re-priced pass

§4 estimated `2.3M decisions × 1.87 s = ~50 box-days`. Three inputs move, and
the net effect is *worse*, not better.

Cost is `2M × 7.3 ms` per decision and double dummy is 99.2% of wall clock
(measured here: 1,684.9 s solving 225,536 layouts, **7.47 ms/layout**), so

    box-seconds = decisions × 2M × 7.3 ms

A v6-sized corpus is **6,768,279 rows over 636,837 boards — 10.63 rows/board**
(§4d corrects this: the board count is derived circularly from `uniform-0`'s own
ratio, the counted denominator is **619,076 auctions**, and the starvation
discount is missing — the corrected pass is 3.6 / 14.6 / 58.3 box-days)
(`uniform-0`: 332,124 rows from 31,250 boards), 62.6% of them contested by
`.tags`. At the measured **5.22 net-served decisions/board**, that is
**3.32M decisions, 49% of corpus rows** — not §4's 2.3M/34%, which was the
choice-bearing rate of the *authored* population on a self-play walk.

| `M` | s/decision | full pass | held-out DD | IMPs per box-hour |
| --- | --- | --- | --- | --- |
| 8 | 0.117 | **4.5 box-days** | +0.4476 | **13,815** |
| 32 | 0.467 | 18.0 box-days | not measured | — |
| 128 | 1.866 | **71.8 box-days** | +0.6134 | 1,183 |

So the density finding raises the `M = 128` bill from 50 to **72 box-days**, and
the lever that pays is not the population — it is `M`. Value per decision is
sublinear in `M` (§4b: `M = 8` retains 73% of `M = 128`'s held-out DD) while
cost is exactly linear, so **`M = 8` buys 11.7× more IMPs per box-hour** and
takes the same full pass to **4.5 box-days**. The corpus is not the scarce
resource — more boards can always be dumped — so the right operating point is
the smallest `M` whose labels still hold up, and `M = 128` was never it.

**Owed before committing the pass: extend the `M`-series downward** on the
net-served slice (`M = 2, 4, 8, 16` at a fixed deal count) and read where the
held-out advantage starts to fall away. That is under an hour of box time and it
is the single number that sets the budget. **Run in §4d**, which answers
`M = 32`, not `M = 8`: perfect defense is the `M`-limiting scorer, and the
margin gate does not tighten as the estimator gets noisier.

### How the pass runs

Relabelling the shipped corpus in place is possible — rewriting 38 target floats
per row leaves the sidecar, `.tags` and `.seq` untouched and row-aligned, so the
trainer's commensurability gate still passes — but it requires re-walking
`corpus-v6`, which is byte-reproducible only at its dump commit `2931a2df`. The
cheaper route is to make the relabel decision **inside `dump-teacher`**, where
the deal is still in scope, and dump a fresh corpus at one commit: ~10 minutes
for the dump, the DD budget above, then 6–8 minutes to retrain (not the "one
GPU-hour" plan.md:405-411 records), fit `T`, and take it to the A/B.

### `3·T` fixes the units and inherits an arbitrary number

`PASS_DEMOTION`'s own doc comment (`src/bidding/instinct.rs:3569-3572`) says it
is **"sized in the book's ~3-nat convention"** — and §1 is the finding that the
book's ~3-nat gaps are *precedence, not odds*. Restating the demotion as `3·T`
is therefore right about the **units** (it makes the gate a fixed probability
ratio, `e⁻³ ≈ 1:20` against Pass, instead of a fixed logit gap on an
uncalibrated scale) while inheriting a **number that never meant odds**. There
is no derivation behind the 3; it was borrowed from a rung convention this
document exists to correct, and §5 has already fixed two other documents for the
same import.

The A/B has to run either way and a `bba-gen` cell is single-digit minutes, so
the recommendation changes from *rescale* to **sweep**: run the arm at
`{1, 2, 3, 4}·T` nats (1.13 / 2.26 / 3.39 / 4.52) with `3.0` as control. Four
arms × two vulnerabilities is roughly 17 minutes of box time and converts a
units fix into an actual calibration of the collar. The blocker is code, not
clock: `PASS_DEMOTION` is a private `const`, so the arm needs an
`InstinctProfile` field, an `Agreements` route, a `bba-gen` flag and — because
`web/` consumes knobs as setter/getter pairs — a `web/` pair. It stays scheduled
inside the collar retune (plan.md M5.2 flip plan arm 1), whose own finding is
that the collar is calibrated to *one floor's distribution*; a sweep against v6
answers the v6 question only.

### Residues

* **The sampler starves on BBA's auctions.** Under a self-play walk the replay
  sampler returns a short draw on 0.3–0.8% of decisions (measured this session on
  the same deals: 2 of 633 authored, 16 of 1,947 net-served). Under the teacher
  walk it starves on **16.4–16.6%** (175 of 1,056 neither vulnerable, 171 of
  1,043 both),
  because the layouts are drawn from *our* inferences about *BBA's* calls. Those
  decisions are dropped, so a relabel pass systematically skips the auctions we
  read worst — a reading-coverage report as much as a sampling one, and one that
  belongs beside [sampled-projection.md](sampled-projection.md). It is not a
  budget tax — a short draw is discarded *before* the solve, so it costs sampling
  time only — but it is a 17% hole in the slice's **coverage**, and the hole is
  not random.
* **How often is BBA's own call `-inf` in our floor?** The teacher walk makes
  this measurable for the first time (`own` need not be in `admissible`).
  **Counted in §4d: 2 of 1,056 (0.19%)** — closed.
* **The corpus is six cells, three of them Dutch.** `scripts/dump-v6.sh`'s
  uniform shards rotate `cells[board % 6]`, so the slice mix above is measured on
  the `american` default and approximates the corpus's own mix.
* **`corpus-v7`'s `.f32` is *not* byte-identical to `corpus-v6`'s**, contrary to
  [../pdd-bank-ledger.md](../pdd-bank-ledger.md) and three other places:
  `uniform-0` is 332,041 rows against 332,124, `axis-0004` differs in size, and
  even the same-size `enriched-0` files differ by md5. A dump is byte-reproducible
  only at its own commit. **Flagged, not fixed** — the ledger row needs a
  correction, and this is a further reason to relabel into a fresh corpus.
* **A retrain costs 6–8 minutes, not plan.md:405-411's "one GPU-hour."** The
  whole 6.09M × 176 feature tensor uploads in one `Tensor::from_slice` (~4.3 GB).
  Flagged; it changes how cheap a relabel experiment's tail is.

## 4d. The `M`-series: the budget number, and a re-priced pass

§4c named the one measurement that sets the budget — extend the `M`-series
downward and read where the held-out advantage falls away — and priced it at
under an hour. This is that hour: **57 minutes for seven rungs**, seed 1, 200
deals, `--walk teacher --population net-served --vul none`, `k = 3`,
`margin = 0.25` IMPs — plus 12 more for the two both-vulnerable confirmations
below.

**The ladder is paired.** Every rung runs the same seed over the same 200
deals, and the harvest is `M`-invariant by construction — the proposal, the
candidate set and the slice predicate never see `M`, only the rollout does. All
seven rows report 1,056 net-served decisions at 918 distinct nodes, reproducing
§4c's teacher/none census exactly. So this is the within-population `M`
comparison §4b could not make: its `M = 8` row was 400 deals against the
`M = 128` rows' 200, and §4c's "`M = 8` retains 73% of `M = 128`" compared a
*self-play* row to a *self-play* row and then spent the ratio on the corpus
population. On the corpus population, on identical deals, it retains **52%**.

The `M = 128` rung is also the refactor's inertness proof: it reproduces §4c's
teacher/none row digit-for-digit — 881 priced, in-sample +0.8091 ± 0.1265 and
+0.5855 ± 0.1064, held out **+0.7483 ± 0.1311** and **+0.5163 ± 0.1095**,
both-rule 16.57% — so this session's one `src/`-free probe addition (the
`own_inadmissible` counter below) moves nothing.

### The rows

± is a 95% interval clustered by source deal. "fire" is the production gate of
§4c: the same call wins under both scorers, differs from BBA's, and clears the
margin **on the validation half**. "pass" is the corrected full-corpus cost of
§4c's relabel, re-derived below.

| `M` | priced | starved | **held-out DD** | **held-out PD** | curse DD | fire | pass | IMPs/box-hr |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2 | 885 | 16.19% | +0.1497 ± 0.2140 | **−0.1350 ± 0.2148** | 1.902 | 11.86% | 0.9 bd | 18,476 |
| 4 | 884 | 16.29% | +0.3733 ± 0.2069 | **+0.0665 ± 0.2051** | 1.196 | 12.90% | 1.8 bd | 23,035 |
| 8 | 884 | 16.29% | +0.3917 ± 0.1360 | +0.1721 ± 0.1408 | 0.859 | 11.09% | 3.6 bd | 12,089 |
| 16 | 883 | 16.38% | +0.5137 ± 0.1446 | +0.2584 ± 0.1345 | 0.497 | 13.25% | 7.3 bd | 7,927 |
| **32** | 882 | 16.48% | **+0.6235 ± 0.1311** | **+0.4632 ± 0.1136** | 0.250 | 13.83% | **14.6 bd** | 4,811 |
| 64 | 882 | 16.48% | +0.7305 ± 0.1304 | +0.5222 ± 0.1114 | 0.091 | 17.23% | 29.1 bd | 2,818 |
| 128 | 881 | 16.57% | +0.7483 ± 0.1311 | +0.5163 ± 0.1095 | 0.061 | 16.57% | 58.3 bd | 1,443 |

Four readings. The first fixes the top of the ladder, the next two are the
reason the answer is not the bottom of it.

**The curve saturates at `M = 64`.** `M = 64` and `M = 128` are statistically
indistinguishable on both scorers (+0.7305 ± 0.1304 against +0.7483 ± 0.1311 DD,
+0.5222 ± 0.1114 against +0.5163 ± 0.1095 PD) at **half** the cost, and their
winner's curse is already down to 0.09 and 0.06 IMPs. So `M = 128` — the rung
§4b and §4c did all their headline work at — buys nothing at all over `M = 64`,
and the asymptote of this estimator is reached by 64 layouts. Everything below
is quoted against `M = 64` as the asymptote rather than against `M = 128`.

**Perfect defense is the `M`-limiting scorer.** As a fraction of the asymptote,
plain DD reaches 20 / 51 / 54 / 70 / 85% at `M` = 2 / 4 / 8 / 16 / 32;
perfect defense reaches **−26 / 13 / 33 / 49 / 89%**. PD needs roughly four
times the layouts DD does to reach the same fraction of its asymptote, which is
what a scorer that also gets to double should look like: the doubled swings are
the high-variance tail, and a small pool misses them. This matters because the
production gate is a **both-scorer** rule, so the noisier half sets the rung.
At `M = 2` PD's interval sits **below zero**, and at `M = 4` it straddles it —
so on [../measurement.md](../measurement.md)'s own reading, applied to a label,
neither rung produces a defensible one at any price. The first rung that clears
both scorers is `M = 8`, and it clears PD by 0.03 IMPs.

**The margin gate does not tighten as the estimator gets noisier.** Fire rate
runs 11.86 → 12.90 → 11.09 → 13.25 → 13.83 → 17.23% from `M = 2` to `M = 64`
— a 45% rise — while the held-out advantage rises 390%. So a low-`M` pass
relabels very nearly the same *volume* of decisions as a high-`M` one, on
labels worth a fifth as much. §4c calls the gate "precision-biased on purpose",
and it is — but the bias is against **BBA's baseline** (a candidate must beat
it by `margin`), not against **estimator noise**, and nothing in the rule scales
with the pool it was estimated from. The winner's curse column is the same fact
from the other side: 1.90 IMPs of selection bias at `M = 2` against 0.06 at
`M = 128`, and the gate is blind to all of it.

**IMPs per box-hour is a degenerate objective.** It peaks at `M = 4` and would
peak at `M = 1`, because value per decision is sublinear in `M` while cost is
exactly linear — §4c's own observation, followed to its conclusion. The reason
not to follow it is that the ratio silently assumes the two ways of spending a
box-hour are interchangeable, and they are not: relabelling twice as many
decisions at half the pool needs twice the corpus (cheap — more boards can
always be dumped) but yields labels that are *wrong* more often, and §4c's own
gate design says why that is not symmetric — "a false negative costs only
opportunity; a false positive corrupts the target." The corpus is extensible;
the retrain is one shot.

### The pass, re-priced again — and §4c's denominator was circular

Two independent corrections, both downward.

**The board count is not a corpus quantity.** §4c reads "6,768,279 rows over
636,837 boards — 10.63 rows/board". The 636,837 is `6,768,279 ÷ 10.628`, where
10.628 is `uniform-0`'s own rows/board — the ratio is assumed corpus-wide and
the board count then derived back out of it, so the figure confirms nothing.
The 20 sidecars in `target/corpus-v6/` say the corpus is **910,000 boards**
(8 × 31,250 uniform + 8 × 20,000 axis + 4 × 125,000 enriched). The right
denominator is neither: the twelve `replay = true` shards bid **two cells per
board** and the eight uniform shards bid one (`cells[board % 6]`), and the
enriched shards keep only 24,538 of their 500,000 attempts. So the corpus is

    250,000 (uniform) + 320,000 (axis) + 49,076 (enriched) = 619,076 auctions

and **10.93 rows/auction**, not 10.63 rows/board. At §4c's measured 5.22
net-served decisions per auction that is **3.23M decisions, 47.7% of corpus
rows** — close to §4c's 49%, but now resting on a counted quantity.

**Starved decisions cost no double dummy.** A short replay draw is discarded
*before* the solve, so the 16.5% the sampler starves on never reaches the
solver. §4c's formula multiplies decisions, not priced decisions. Applying the
discount leaves **2.70M priced decisions**, and

    box-seconds = 2,698,367 × 2M × 7.29 ms   →   0.455 box-days per unit of M

| `M` | §4c | corrected |
| --- | --- | --- |
| 8 | 4.5 box-days | **3.6** |
| 32 | 18.0 | **14.6** |
| 128 | 71.8 | **58.3** |

A box-day is a wall-clock day of this whole 32-core box: the probe solves
through one `Solver::lock(None)` batch, which auto-detects every core, so the
7.29 ms/layout is already the fully-parallel rate and the figure does **not**
divide by adding processes on one machine. It divides only across machines.

### The vulnerability axis, at the rung that matters

§4b settled vulnerability at `M = 128`; the recommendation below lives at
`M = 32`, so it is confirmed there rather than assumed. Same seed, same 200
deals, `--vul both`:

| `M`, vul | priced | **held-out DD** | **held-out PD** | fire |
| --- | --- | --- | --- | --- |
| 16, none | 883 | +0.5137 ± 0.1446 | +0.2584 ± 0.1345 | 13.25% |
| 16, both | 875 | +0.6465 ± 0.1787 | +0.4221 ± 0.1574 | 13.94% |
| 32, none | 882 | +0.6235 ± 0.1311 | +0.4632 ± 0.1136 | 13.83% |
| 32, both | 873 | +0.7423 ± 0.1620 | +0.5480 ± 0.1387 | 15.58% |

Both-vulnerable runs a little richer at each rung and every interval overlaps
its `none` partner, exactly as §4b and §4c found at `M = 128`. The axis stays
settled, and `none` stays the binding cell — which is the conservative one to
have chosen the rung on.

### The verdict

**`M = 32`, at 14.6 box-days.** It is the first rung where perfect defense is
solid rather than marginal (+0.4632 ± 0.1136, clear of zero by 0.35 IMPs
against `M = 8`'s 0.03), it captures **85% of the asymptote's plain-DD value and
89% of its perfect-defense value at half the asymptote's cost**, and it holds on
both scorers at both vulnerabilities. The step to `M = 64` buys the last
15% / 11% for another 14.6 box-days; the step beyond that buys nothing at all.

The fallbacks are priced, which is the point of having the ladder:

* **`M = 64`, 29.1 box-days** if a second machine halves the wall clock. It is
  the asymptote, and it is the *only* reason to spend above `M = 32`.
* **`M = 16`, 7.3 box-days** if 14.6 wall-clock days on one box is not
  available and no second machine is. It keeps 70% / 49% of the value and PD
  still clears zero — and at both-vulnerable it is already worth
  +0.6465 / +0.4221.
* **`M = 8`, 3.6 box-days** is the floor of defensibility, not a
  recommendation: it is the smallest rung that clears both scorers at all, and
  it clears PD by 0.03 IMPs.
* `M = 2` and `M = 4` are **excluded on the measurement rule**, not on cost.

This supersedes §4c's "`M = 8` … and `M = 128` was never it". `M = 128` is
still not it — and now for a second reason, that `M = 64` matches it for half
the money. Neither is `M = 8`.

### Residues

* **The owed count is answered, and it is small.** §4c owed one line: how often
  is BBA's own call `-inf` in our floor? The teacher walk makes it measurable
  because `own` need not be in `admissible`. It is **2 of 1,056 (0.19%)**, at
  every rung. Our floor can represent essentially every call BBA's walk makes;
  the coverage problem is the sampler's, not the book's.
* **The sampler's 16.5% hole is infeasibility, not budget — settled here for
  free.** A short draw from a *draw-cap* limit scales with `M`: the effective
  acceptance floor is `2M / REPLAY_DRAW_CAP`, which moves 16× from `M = 8` to
  `M = 128`. A short draw from a **zero-measure** accept region does not move at
  all. Measured across a 64× span of `M`: 16.19 / 16.29 / 16.29 / 16.38 /
  16.48 / 16.48 / 16.57%. Flat. So raising `REPLAY_DRAW_CAP` or `REPLAY_DRY_LIMIT`
  recovers nothing, and the hole is what §4c suspected — our inferences about
  *BBA's* calls are jointly infeasible on those auctions. It belongs to
  [sampled-projection.md](sampled-projection.md), and it is a **reading**
  repair, not a sampling one.
* **The fired subset's own advantage cannot be read off this probe.** The gate
  fires on `held_out > margin` and the report averages that same half, so the
  *unconditional* held-out mean (the column above, and the one §4c's budget
  uses) is unbiased for "relabel every decision to the selector's pick", but the
  mean *among fired* decisions is selected-on-sample and would overstate. Under
  a gate with any skill the unconditional mean is a **lower** bound on the
  pass's value, so the budget above is conservative. Reading the gated value
  honestly needs a three-way split (select on `M`, gate on `M`, price on a third
  `M`) at 1.5× the layouts. **Flagged, not built** — it changes no decision
  here, because every rung is scored the same way.
* **A cascade is the obvious next idea and it does not obviously win.** Price
  every decision at a cheap `M`, then re-price only the survivors at a dear one.
  It beats a flat `M = 32` only if the cheap filter keeps under ~19% of
  decisions while retaining the asymptote's recall, and `M = 8`'s own DD-alone rate
  is 28.4% — so on the numbers here it lands at ~20 box-days, worse. What would
  settle it is the per-decision agreement between the `M = 8` and `M = 128`
  selections, which these seven runs contain but do not print. **Flagged**; a
  `--dump-decisions` flag is the cheap way to look.

## 5. Doc drift corrected

Two documents described a mechanism that never existed. Both are rewritten
this session to point here.

| doc | said | why wrong | says now |
| --- | --- | --- | --- |
| `docs/ai-bidder/02-policy-net.md:94-106` "### Output calibration" | distillation "inherits the teacher's scale automatically"; keep a temperature scalar `T` as "a single post-hoc knob, tuned on held-out boards" so the floor's decisiveness "matches what the driver expects" | every shipped floor since 2026-07-19 clones **one-hot** BBA labels (README glossary, Distillation row): a one-hot has no scale to inherit, and the target the net converges to is §3 row (ii), which owes nothing to any teacher scale. At the session-1 audit the `T` knob had not been built, and the driver expects an argmax, which `T` cannot move. | no teacher scale to inherit (one-hot labels); the ~3-nat gap is the book's precedence convention; session 2 built the held-out-NLL fitter and sidecar field; argmax-invariant serving remains raw; link here |
| `docs/ai-bidder/README.md:92` glossary "Temperature / calibration" | "Scaling logits before softmax. The books use a ~3-nat gap convention; the net must match that scale." | the books have no scale to match: §1's rungs are 5-50 centinats apart and crisp, and §2 finds no consumer that ever compares a book logit with a net logit (they never share a `Logits`). "Must match" named an obligation with no reader. | one scalar `T` before softmax; the books' ~3-nat gaps are precedence, not odds; session 2 built the held-out fitter, and each new artifact still has to run it; link here. The document map's ledgers paragraph (`README.md:110-123`) gains this page. |

The `rules.rs:1-23` module doc's "300 is near-deterministic after softmax" is
left as written: it is true as arithmetic, and the doc's own sentences about
priority and integral rungs already carry the precedence reading. §7 has the
one comment that does mislead.

## 6. Ledger

| session | deliverables | gate | status |
| --- | --- | --- | --- |
| 1 (2026-09-03) | this document; display hygiene (`top3()`, practice-bidding: mask then softmax, ladder where an authored node answered the hand, odds where the floor did); `02-policy-net.md` §Output calibration and `README.md:92` rewritten; §7 flags recorded | `smoke-default --count 20000 --seed 1` = `38ee1e21…`, unchanged | **done** |
| 2 (2026-09-03) | `trainer/src/calibrate.rs`: `T` fitted on the held-out split by golden section on the same soft-target cross-entropy, NLL and ECE before/after in the training report, `temperature` + the four metrics in the weights sidecar; `examples/probe-book-vs-net` and its census (§4) | byte-identity: no `src/` change at all, `smoke-default --count 20000 --seed 1` = `38ee1e21…` unchanged | **done** — the epsilon it recommended, **`1e-4`**, was decided by jdh8 on 2026-09-03 (§4) |
| 3 (2026-09-03) | `examples/probe-rollout-label`: top-`k` ∪ own call from the restricted proposal, independent `M`-layout selection and validation pools from the replay sampler, one shared solve per layout, both scorers, paired baseline, replay-short skip, deal-clustered intervals; the `M`-series, headline census and the `--opponent bba` arm above | byte-identity: no bidding change; the only `src/` edit widens `table::select_legal_call` to `pub` (no behaviour), `smoke-default --count 20000 --seed 1` = `38ee1e21…` unchanged | **done** — never relabel from the in-sample estimate: at `M = 32` its +0.250/+0.543 DD/PD shrinks to +0.036/+0.245 held out, while `M = 128` confirms that a smaller real signal exists on both scorers |
| 4 (2026-09-04) | the **gate-flip experiment** the session-4 handoff demanded before spending §6's double dummy (§4b): `probe-rollout-label --population {authored,net-served}` with the three-way net-served predicate, the floor shell's own logits as the proposal, `--vul`, and the D2 Pass census; `instinct::forced` widened to `pub`; the trainer's `--weights-in`, which fits `T` for a *shipped* artifact without retraining | byte-identity: no bidding change; the only `src/` edit widens `instinct::forced` from `pub(crate)` to `pub` (visibility and a doc comment). The refactor is separately proven inert by re-running §4a's row through the new binary: 631 decisions, held-out −0.1040 ± 0.0891 / +0.1583 ± 0.1237, digit-for-digit | **done** — the net-served population is **positive on both scorers at both vulnerabilities** (held-out DD +0.613/+0.684, PD +0.464/+0.563), ~6× §4's authored signal, D2 clean and inverted; and **`T` = 1.1298** for `american_bba_v6`, so `PASS_DEMOTION` → `3·T` = 3.389 nats |
| 5 (2026-09-04) | §4c: `probe-rollout-label --walk {self,teacher}` — the **corpus-fed mode**, which needs no corpus reader (a v6 row has no board id, but the pricing half only ever consumed *(hand, seat, dealer, prefix)*); the four-way **slice histogram**; the two `M = 128` teacher cells; the production **label gate** spec; the **re-priced pass** | byte-identity: **no `src/` edit at all**, so `smoke-default` cannot move. The refactor is proven inert by re-running §4a's authored row *and* §4b's `M = 8` net-served row through the new binary: 631 decisions at +0.4487 ± 0.0633 / −0.1040 ± 0.0891 / +0.7740 ± 0.0956 / +0.1583 ± 0.1237, and 1,931 at +1.1066 ± 0.0750 / +0.4476 ± 0.0968 / +0.9544 ± 0.0837 / +0.2520 ± 0.1034 — both digit-for-digit | **done** — the value **survives the move to the corpus population and grows**: held-out DD +0.7483 ± 0.1311 (none) / +0.8074 ± 0.1496 (both), PD +0.5163 ± 0.1095 / +0.6163 ± 0.1336, against §4b's self-play +0.613/+0.684 and +0.464/+0.563, with a *smaller* winner's curse. The slice is 49% of corpus rows, so `M = 128` costs **72 box-days, not 50** — and `M = 8` buys 11.7× more IMPs per box-hour, taking the same pass to **4.5 box-days** |
| 6 (2026-09-04) | §4d: the **`M`-series** on the corpus population, seven rungs paired on identical deals (`M` = 2/4/8/16/32/64/128, seed 1, 200 deals, teacher walk, net-served, `--vul none`) plus two both-vulnerable confirmations at `M` = 16/32, 69 minutes of box time; the `own_inadmissible` counter that closes §4c's owed line; the corpus denominator re-counted from the 20 sidecars; the starvation discount applied to the budget | byte-identity: **no `src/` edit at all**, so `smoke-default` cannot move. The one probe addition is proven inert by the `M = 128` rung reproducing §4c's teacher/none row digit-for-digit — 881 priced, +0.8091 ± 0.1265 / **+0.7483 ± 0.1311** / +0.5855 ± 0.1064 / **+0.5163 ± 0.1095**, both-rule 16.57% | **done** — the answer is **`M = 32` at 14.6 box-days**, not `M = 8`. The curve **saturates at `M = 64`** (indistinguishable from `M = 128` on both scorers at half the cost), so the rung §4b and §4c did their headline work at buys nothing. Perfect defense is the `M`-limiting scorer (it reaches 33% of its `M = 128` value at `M = 8` where plain DD reaches 52%), and the margin gate is nearly volume-neutral in `M` (fire 11.9 → 16.6% while value rises 400%), so it filters against BBA's baseline but **not** against estimator noise. IMPs-per-box-hour is degenerate — it peaks at `M = 4` and would peak at `M = 1`. §4c's denominator was **circular** (636,837 "boards" is `6,768,279 ÷ 10.628`); the counted figure is **619,076 auctions**, and with the missing starvation discount the pass re-prices to **3.6 / 14.6 / 58.3** box-days at `M` = 8 / 32 / 128. The sampler's 16.5% hole is settled as **zero-measure infeasibility, not budget** — flat across a 64× span of `M` — so it is a reading repair, not a cap raise |
| 7 (2026-09-04) | the **relabel build and its fleet**: `dump-teacher --relabel` harvests the net-served decisions of the corpus walk (our reader's provenance, `Phase`, `forced`), rolls each out through the shared pricer (`examples/common/rollout.rs`, lifted out of `probe-rollout-label`) and stores **raw per-layout returns** — `[candidate][layout] → (DD, PD)` swings over BBA's call — in a `.ret` sibling; `--cut M` reads every chunk, selects on `[0, M)`, validates on `[M, 2M)`, and overwrites the one-hot where §4c's gate fires, refusing sidecars that disagree, non-contiguous tilings, and chunks short of `2M`. Streams are seeded from the **bank index** (per board and per decision), so chunks split anyhow concatenate byte-identically; an existing `.ret` is **extended** (only new layouts solved). Fleet: `scripts/relabel-worker.sh` (stride/offset over the v6 recipe, existence gate, SIGHUP drain), `scripts/pons-worker@.service`, `scripts/fleet-relabel.sh` (`provision`/`start`/`status`/`collect`/`mopup` over `~/.config/pons/hosts`); the section in [../shared-machine-data-gen.md](../shared-machine-data-gen.md) | byte-identity: **no `src/` edit at all**. The probe refactor is inert — `probe-rollout-label -c 40 -s 1 -m 4 --walk teacher` prints the same 42 lines before and after. Three tests pin the build: split-then-cut = whole-then-cut, an extended draw cuts like a native one (at the old `M` and the new), and the cut refuses a foreign SHA / a short chunk / a gap | **built, run owed** — the corpus pass (`start 64` → `--cut 32`, ~7 days on the four-box fleet) has not been launched |
| 7+ | **the run**: `fleet-relabel.sh provision` → `start 64` → `collect` → `mopup 64` → `--cut 32` → fit `T` → retrain (`trainer --weights-in`) → A/B; pass 2 (`start 128` … `--cut 64`) only if the A/B is marginal. Then `PASS_DEMOTION` as a **`{1,2,3,4}·T` sweep**, not a `3·T` rescale (§4c), inside the collar retune (plan.md M5.2 flip plan arm 1). Also queued by §4b: the **raw-net-versus-shell** arm (are the gates costing IMPs?) | the [../measurement.md](../measurement.md) decision table, both scorers and both vulnerabilities | owed |
| ~~6+~~ | ~~**extend the `M`-series downward**~~ (`M = 2, 4, 8, 16` on the net-served slice, under an hour) — it is the one number that sets the budget; then relabel inside `dump-teacher` → fresh corpus → fit `T` → retrain → A/B. The population axis is **settled** by §4c, the opponent axis by §4a, the vulnerability axis by §4b. Cross-fitting is **declined** in favour of a smaller `M` (§4c). `PASS_DEMOTION` as a **`{1,2,3,4}·T` sweep**, not a `3·T` rescale (§4c), inside the collar retune (plan.md M5.2 flip plan arm 1) | the [../measurement.md](../measurement.md) decision table, both scorers and both vulnerabilities | owed |

Sessions 2 and 3 change no call and are provable by the hash — session 3's one
`src/` edit widens `table::select_legal_call` to `pub` so the probes call
production's selector rather than reimplement its tie-break, which is a
visibility change and nothing else. Everything in session 4+ moves calls and is
measured, not argued.

**Why session 3 stops before the retrain.** The design's target rule uses the
same `M` layouts to select and bless a label. On the 2,000-deal census that
reports +0.2499/+0.5434 DD/PD and a 20.98% both-scorer relabel rate; the same
selected calls on independent layouts are worth only +0.0362/+0.2446 and clear
both margins 11.01% of the time. The original labels therefore encode a large
selection artifact. A bigger margin does not remove it. Independent validation
does, but its threshold is a new label rule. Session 4 must specify that rule
before touching the corpus. `M = 128` being held-out-positive on both scorers is
the reason to continue, not permission to skip that gate.

**What the probe measures, and what it does not.** The probe walks a self-play
`american()` auction and displaces **the book's argmax** at **our** node
distribution. §4's target rule displaces **BBA's one-hot** on the
**BBA-generated corpus**, and reverts to BBA off the margin. Three things
therefore differ between the census and the rule it prices: the population of
decisions (our auctions reach different nodes than BBA's), the baseline being
displaced, and the asymmetric BBA fallback, which has no counterpart here at
all. This does not touch the winner's-curse result — that is a property of
"select the max of `k` noisy estimates and report it", and holds whatever the
baseline is, which is why the refutation stands. It does mean the **relabel
rates** (20.98% → 11.01%, and the ~3-4%-of-authored-decisions figure derived
from them) are proxies, not the rates the production rule would fire at.
Closing the gap needs a corpus-fed mode: harvest decisions from the shipped
`dump-teacher` corpus with BBA's label as the baseline, rather than from a
self-play walk. That is session 5's first build, and it is the remaining axis
that most changes the numbers — the *population* half is answered in §4b, which
also settles which slice the corpus pass should spend its budget on.

**The two axes session 3 could not settle, and the one it did.** Self-play
versus BBA **is settled**: §4a measures both arms and finds every held-out
interval overlapping, both signs preserved, at 1.46× wall clock. Vulnerability
**is settled by §4b**: `--vul` is now a flag, and the two `M = 128` net-served
cells agree on sign and overlap on interval, so the signal is not living in the
competitive book's vulnerability axis. What session 3 could not settle and
session 4 did is the *population*: §4b shows the authored census was pricing
decisions production never asks the net about, and the slice it does ask about
carries a signal about six times larger.

## 7. Flagged, not fixed

Findings from the inventory that are wrong on paper and (mostly) harmless in
production. Each carries a reversible default so nothing is silently resolved.
The first four are session 6's, then session 5's, then session 3's, then session 1's.

**`dump-teacher`'s reader is built symmetric while the row it writes is
asymmetric — inert today, load-bearing the moment a relabel lands.**
`examples/dump-teacher/main.rs:778-782` builds the per-side reader with
`american(&agreements).bind()` / `dutch(&agreements).bind()`, and both hard-wire
`CompactConfig::symmetric(...)` into their v6 floor
(`src/bidding/american.rs:148-156`, `src/bidding/dutch.rs:51-57`). The **row's**
compact block is the asymmetric `CompactConfig::new(&ours, &theirs)`
(`:796-802`, applied at `:1010`). Today nothing notices: the module doc at
`:10-13` records that a floor projects nothing (`Classifier::as_rules` is
`None`), so the floor's own config cannot reach the features. **A relabel makes
it load-bearing** — the proposal would come from a floor conditioned on
`symmetric(ours)` while the features the trainer sees say `(ours, theirs)`, i.e.
the two halves of the row would disagree about the regime. It bites in 2 of the
6 `DEFAULT_CELLS` (`:520-527`: the mixed `(A_ON, A_OFF)` and `(A_OFF, D_OFF)`)
plus their mirrors. This is exactly the sibling-factory rule of
[card-manifold.md](card-manifold.md) — a system name must reach the same net on
its declared and undeclared paths — biting where that document predicted.
**Default: leave `dump-teacher` alone until the relabel is built; then build the
reader per *pair*** with the entry points that already exist and take a
`theirs: &ConventionCard` — `american_with_card` (`american.rs:200-212`) and
`dutch_with_card` (`dutch.rs:84-90`) — gated behind the relabel flag so a
non-relabelling dump stays byte-identical, and record which was used in the
sidecar.

**`probe-rollout-label`'s doc comment claims the teacher walk *is*
`dump-teacher`'s walk. It is not, for any real corpus shard.**
`examples/probe-rollout-label.rs:227-231` says `--walk teacher` runs "the walk
`dump-teacher` runs, so this is the corpus population". The probe loads
`BbaOracle::load(DEFAULT_LIB, SYSTEM_2_OVER_1, Vec::new())` with **no convention
card and no `with_opponents`**, holds our side at `american(&Agreements::default())`
with no `--system` flag at all, and pins vulnerability with `--vul`.
`corpus-v6` spans **14 distinct table configurations** across its 20 shards, with
cards disclosed and vulnerability drawn per board. So §4c's 5.22 net-served
decisions per auction — the multiplier the whole budget rests on — is measured
in one configuration and applied to fourteen. **Default: leave the code, reword
the comment**, and treat the budget's decision count as ±1 cell of uncertainty
rather than a measured constant. The cheap check, if it is ever worth one, is a
`--system`/`--their-system` pair on the probe and a re-run of the slice
histogram on two or three of the mixed cells.

**The uniform shards' sidecars say `american` for rows the Dutch book bid.**
`dump-teacher/main.rs:1113-1116` writes `our_system` / `their_system` from
`args.system` / `args.their_system` regardless of `--cell`, while the eight
uniform shards rotate `cells[board % 6]` (`:520-527`) — of which 2.5 of 6 are
Dutch-acting, so **41.7% of uniform rows are bid by a book the sidecar does not
name**. Nothing is lost: `cells` is written alongside at `:1108-1111`, so the
information is recoverable, and no shipped artifact reads `our_system`.
**Default: leave.** Correct the field the next time the dump script moves, or
document it in [../pdd-bank-ledger.md](../pdd-bank-ledger.md).

**`knobs.rs:540` names an A/B binary that does not exist.** The `rule_accept`
doc points at `ab-search-floor --no-rule-accept`; there is no `ab-search-floor`
among the 131 entries in `examples/`, and `--no-rule-accept` appears in no source
file. Stale doc, no reader. **Default: leave; drop the sentence when the knob is
next touched.**

**`corpus-v7`'s `.f32` is not byte-identical to `corpus-v6`'s.**
[../pdd-bank-ledger.md](../pdd-bank-ledger.md):139 and three other places in the
tree say it is. On disk `corpus-v7/uniform-0` holds 332,041 rows against
`corpus-v6/uniform-0`'s 332,124, `axis-0004` differs in file size, and even the
same-size `enriched-0` pair differs by md5; a 40-board re-run at HEAD's flags
reproduces the v7 bytes and diverges from v6 at row 256. The cause is that a
dump is byte-reproducible only at **its own commit** — `corpus-v6` at
`2931a2df`, `corpus-v7` at `47ec143d` — which plan.md:400-404 already records for
row counts without drawing the conclusion. Nothing shipped depends on the claim:
`american_bba_v6` names its own 20 stems and its sidecar pins the sha.
**Default: leave the corpora; correct the ledger row** in the same change that
next touches it, and dump a relabel corpus fresh at one commit rather than
rewriting a shipped one in place (§4c).

**A retrain costs 6–8 minutes, not "one GPU-hour".** plan.md:405-411 budgets a
regression retrain at a GPU-hour. The on-disk logs say otherwise —
`american_bba_v6_their` 6 min 08 s, the v6-stems reproduction 7 min 54 s — because
the whole 6.09M × 176 feature tensor uploads in a single `Tensor::from_slice`
(~4.3 GB) and the epoch loop never shuffles. The LSTM arm is the one that costs
hours (2 h 34 min). **Default: leave plan.md's prose**, which is load-bearing
only as a discouragement; correct it when M5.2 next moves. It matters here
because it makes a relabel experiment's tail nearly free, which is what §4c's
budget assumes.

**A keyless `Context` before `ev_all`, so the rollout samples against nothing.**
`examples/ab-lebensohl/main.rs:259` builds `Context::new(relative(vul, seat),
auction)` and hands it to `ev_all` at `:262`. A bare context carries no system
and no prefixes, so `Table::infer`'s doc applies — it "silently skips every
projection-based reading and hands back a vacuous `0..=37`" — and `ev_all`'s
`within_ranges` half then accepts **every** layout. Only the rule-replay half
(`rules_accept`, which reads the policy and the auction directly, not the
inferences) still constrains, and it abstains at every unauthored node. So that
A/B's PD relay gate ran on a materially weaker sampler than the design intends.
The correct constructor is `Partnership::prefixed_context`
(`src/bidding/book.rs:958`), which is what `probe-book-vs-net` and
`probe-rollout-label` use. `examples/probe-replay-yield.rs:207` has the same bare
context, which makes its "range fill" column meaningless as a baseline (a
vacuous reading fills 100% by construction) — the "replay fill" column and the
ratio's *direction* survive. **Default: leave both.** The Lebensohl verdict was
taken and its re-measure is a separate decision; fix the context in the same
change that re-measures it.

**`bba-score --score pd` prices with `ns_score_bid`, not `ns_score_pd`.**
`examples/bba-score/main.rs:158` maps the reached contract down to its `Bid`
before scoring, which **discards any real double or redouble on the table** and
re-derives the penalty from whether the contract fails double dummy. That is the
call-evaluation scorer; `src/scoring.rs:152-164` says in as many words to use
`ns_score_pd` instead "to score a duplicate A/B once a side may *defend* by
passing", and [../measurement.md](../measurement.md) names `ns_score_pd` as the
PD bracket. The anchor pipeline does not go through it —
`bba-decompose/main.rs:562-564` and `ab-dump-diff/main.rs:183,207` both use
`ns_score_pd` — so no campaign number is affected. **Default: leave.** Switch it
the first time a verdict is read off `bba-score --score pd`, and note that doing
so will move that binary's historical PD numbers.

**`max_by` returns the last maximum.** `Iterator::max_by` breaks a tie in
favour of the *last* element, the opposite of production's first-max
`select_with_legal_state`. Sites: `examples/eval-evaluator/main.rs:342`,
`examples/dump-evaluator/main.rs:729`, `examples/probe-admit-node.rs:98`,
`examples/eval-columns/main.rs:313`, `examples/probe-1m-raise-ev/main.rs:125`,
`examples/probe-dutch-1s-points.rs:28`, `examples/probe-keycard-reach/main.rs:199`,
`examples/dump-teacher/main.rs:1165` (the `argmax_legal` that advances the
teacher auction at `:1044` — inert under `--teacher bba`, whose one-hot labels
cannot tie), and the doc examples at `src/bidding/american.rs:142` and
`src/bidding/dutch.rs:45`. Ties are real
(§1: 211 sites at rung 0, and equal rungs are an authored claim), so a probe
and production can disagree on a tied node. None of these has been used for a
verdict. **Default: leave.** Fix the first one that is ever read for a
verdict, by iterating with strict `>` as `table.rs:105-119` does.

**The ladder's `-5` comment.** `src/bidding/instinct.rs:4682-4685` justifies
the unconditioned Pass (weight `-500` at `:4714`; the comment still says
`-5`, a pre-centinat unit) as sitting "far enough below every forced
action (≥ 3 nats) that sampling drivers never pass a forced auction by
accident". No sampling driver reads the ladder's magnitudes: the replay
sampler's margin is gated to authored nodes (`sampler.rs:264`) and the ladder
is the constructive *floor*. The weight does its real job (keeping the logits
finite when every action is illegal) regardless of size. **Default: leave the
weight, reword the comment when the ladder is next touched.**

**`Versus` and `OrElse` inherit `authored_at = true`.** The two composers in
`src/bidding/compose.rs` (`:43`, `:73`) implement `Bidder` without overriding
`authored_at`, so the trait default (`src/bidding.rs:154`, "assume authored")
answers for any composed bidder, and `Table::authored_at` over one
(`ns.vs(ew)`, `book.or_else(floor)`) says "authored" unconditionally. Neither
display composes one — both seat `Partnership`s, and both read the per-hand
provenance rather than this method — but the new public method exposes the
gap. **Default: leave.** If a composed bidder is ever handed to the replay
sampler or a display, dispatch `Versus` by parity as its `classify` does and
`OrElse` to its first arm, and note that a hand the first arm rejects is
answered by the second, so provenance, not `authored_at`, is the per-hand
answer there too.
