# Precedence vs probability: what a book logit means, and where the odds come from

**Status: SETTLED 2026-09-03 by interview** (`/grill-me`, fifteen decisions,
every recommendation accepted). Session 1 shipped the same day: this document,
display hygiene (`web/src/lib.rs` `top3()`, `examples/practice-bidding`), and
the two doc-drift repairs in §5. **No bidding change**: `smoke-default --count
20000 --seed 1` is unchanged at `38ee1e21…` before and after
([../measurement.md](../measurement.md) item 12). Sessions 2 and 3 shipped the
same day and under the same gate; **session 4+ is owed**, and session 3's census
changed the next task — the ledger is §6. This is the calibration story the
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

*Built in session 3* as `examples/probe-rollout-label`. Four scope choices are
explicit in its module doc. It defaults to **self-play**, because `BbaOracle`
creates and destroys a native bot for every call (`with_bot`,
`examples/common/oracle/mod.rs:457`) and the replay sampler selects worlds under
our policy; `--opponent bba` remains available. It uses **`ns_score_pd`**, not
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
draws **two independent pools of `M` replay layouts**: select on the first, then
price that same call on the second. The held-out continuous mean is unbiased
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
| 3 (2026-09-03) | `examples/probe-rollout-label`: top-`k` ∪ own call from the restricted proposal, independent `M`-layout selection and validation pools from the replay sampler, one shared solve per layout, both scorers, paired baseline, replay-short fallback, deal-clustered intervals; the `M`-series and headline census above | byte-identity: no `src/` change at all, `smoke-default --count 20000 --seed 1` = `38ee1e21…` unchanged | **done** — never relabel from the in-sample estimate: at `M = 32` its +0.250/+0.543 DD/PD shrinks to +0.036/+0.245 held out, while `M = 128` confirms that a smaller real signal exists on both scorers |
| 4+ | specify the production label gate (including independent validation and its `M`), add the vulnerability axis, settle self-play versus BBA continuation, then relabel a corpus slice → fit `T` → retrain → A/B on both scorers. At `M = 128`, a 100k-decision selection+validation audit is ~52 box-hours; the full ~2.3M choice-bearing population is ~50 box-days. The sampler's importance-weighted acceptance is sized against the same harness (`probe-replay-yield` sizes the variance); `PASS_DEMOTION` = `3·T` inside the collar retune (plan.md M5.2 flip plan arm 1) | the [../measurement.md](../measurement.md) decision table, both scorers and both vulnerabilities | owed |

Sessions 2 and 3 change no call and are provable by the hash. Everything in
session 4+ moves calls and is measured, not argued.

**Why session 3 stops before the retrain.** The design's target rule uses the
same `M` layouts to select and bless a label. On the 2,000-deal census that
reports +0.2499/+0.5434 DD/PD and a 20.98% both-scorer relabel rate; the same
selected calls on independent layouts are worth only +0.0362/+0.2446 and clear
both margins 11.01% of the time. The original labels therefore encode a large
selection artifact. A bigger margin does not remove it. Independent validation
does, but its threshold is a new label rule, and session 3 measured only
self-play at neither vulnerable. Session 4 must specify that rule and add the
missing axes before touching the corpus. `M = 128` being held-out-positive on
both scorers is the reason to continue, not permission to skip those gates.

## 7. Flagged, not fixed

Findings from the inventory that are wrong on paper and (mostly) harmless in
production. Each carries a reversible default so nothing is silently resolved.
The first two are session 3's; the rest are session 1's.

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
