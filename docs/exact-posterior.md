# Exact hand posteriors — what the web counter is worth to bidding

**Status, 2026-10-02 (HEAD `d54eca62`): Phase 0 built and pinned on all four
pins; Phase 1's kill gate run and read — use A as designed FAILS, and a
hand-free by-product passes (§5, §7). Phase 2's re-scoped gate reaches 0.5%
of boards and is not built. Phase 3a fails by the route that needs no new
API — a reading is not a reply forecast — and the proposed default is to
park Phase 3; that decision is open.** This document records (1) what the web
crate's Odds/Partner counter is, (2) which uses of an exact posterior this repo
has already refuted, (3) the literature — bridge engines, other
imperfect-information games, the general machinery — and (4) a phased plan with
a kill gate in front of every build. Implementation and verification are left
to future sessions.

The one-line verdict: **an exact posterior over a hidden hand is cheap and, as
far as the survey found, new to bridge bidding — but its value is on the
*output* side (how far to trust a reading, how partner will answer, how to
deal), not as net inputs, which three campaigns here already closed.**

## 1. What exists

### The web counter (the only surviving implementation)

[`web/app.js`](../web/app.js), tabs Odds and Partner; grammar and timings in
[`web/README.md`](../web/README.md).

- **Model.** A hand description is a **union of boxes** — a DNF. A box is four
  suit cells (a length range or a holding regex), a points range on a chosen
  gauge (HCP, UP, support points per trump), and a `where` predicate.
- **`point_census()`** ([`web/src/lib.rs`](../web/src/lib.rs)) exports, per
  suit length, every holding's `(honors, hcp, wasted, hcp_plus, count)` from the
  crate's own evaluators, so the gauges cannot drift from the bidder's.
- **`gaugeJoint` / `oddsCount`** — one hand: convolve the per-suit censuses
  across the 560 shapes, carrying six gauge sums jointly (45k states over all
  shapes) and **one bit per box** for which boxes each suit's cell still admits.
- **`companionCount(mine, known, query)`** — two hands: counts (your hand,
  partner's hand) pairs with your hand in `mine`, partner's in `known`, and
  partner's also in `query`, exactly. Per suit the two hands draw disjoint
  holdings (honors A–T by identity, the eight spots by count). A shape pair
  **meets in the middle**: ♠♥ and ♦♣ convolve into half-tables, and the boxes'
  (your HCP, partner's HCP) rectangles are read off the ♦♣ table's prefix sums.
  Suits no box tells apart are counted once per symmetry orbit (24:1 for
  points-only boxes).
- **Cost.** Tens of ms for most questions; ~0.5 s when every suit is named
  (239,344 shape pairs); slower for a `where` coupling both hands' points
  (`up + my.up >= 25`, a staircase) or a suit's HCP read beside another suit.

What it can express that the dropped Rust oracle could not:

| capability | why it matters for bidding |
| --- | --- |
| your hand as a DNF, not thirteen cards | node-level questions with no deal: reach and frequency of an auction |
| predicates coupling both hands (`s + my.s >= 8`) | fit and combined-strength probabilities |
| honors by identity | P(partner holds the ♠A), keycard counts |
| a bit per box (Venn cells of ≤ 26 boxes) | any Boolean combination of boxes — set differences, hence a **precedence ladder's partition** |

### The dropped Rust oracle

[`dnf-migration.md`](dnf-migration.md) rows MARG and MASS (2026-07-24).
`MassOracle` priced `P(hidden hand ∈ reading | seen hand)` in closed form —
per-suit 2^k subset census → shape-walk convolution into (shape, HCP, veto)
fibres; build ≈ 0.35 ms, union query ≈ 0.1 ms, ~150× faster than a 50k-draw
Monte Carlo and equal to it within binomial noise. It was dropped with its only
consumer. The row names its own re-open triggers: *an output-side consumer that
bypasses the net (sampler mass budgeting, tidy pricing, reading forensics)*.

The shape kernel that did land, `features::walk_shapes` / `shape_of`
([`src/bidding/features.rs`](../src/bidding/features.rs)), is the same idea
restricted to lengths: 560 atoms, exact hypergeometric weights, any-box
membership.

### Prior art outside the repo

- **Pavlicek's Companion Hand Calculator** — the Partner tab's model for a
  *single* box: your hand, the companion's known shape and HCP, an And/Or query,
  summed over the 560 patterns. (P)
- **Thomas Andrews' Deal, `smartstack`** (`lib/handFactory.tcl`) — the closest
  algorithmic match. It tabulates holdings per suit keyed by (length, evaluator
  value), enumerates every (shape, per-suit value tuple) in range with count =
  the product of the four list sizes, picks a cell by cumulative probability
  (binary search), then one uniform holding per suit — exactly uniform over
  qualifying hands. One seat, one shape class × one additive range, sampling
  only. (✓ mechanism; the "~14 s build" figure is P)
- **redeal `SmartStack`** — the same idea in Python. (P)

DNF unions, query evaluation, two-hand coupling and the convolution itself
appear to be new. The survey found **no engine that computes an exact Bayesian
posterior while bidding**; every sourced engine samples.

## 2. Closed — do not retry without the named trigger

| attempt | what it fed the evaluator | result | record |
| --- | --- | --- | --- |
| MARG | Monte Carlo card marginals through a marginal net | marginals-only **lost** the NLL gate (−1.4983 vs −1.5118); beside the hulls, par | [dnf-migration.md](dnf-migration.md) MARG |
| MASS | exact union masses, a mixture of per-box net queries | **lost** +0.069 / +0.095 nats; narrow boxes are OOD for a hull-trained net | [dnf-migration.md](dnf-migration.md) MASS |
| SHAPE (`features_eval_v4`) | exact shape distribution (E, sd, log-mass) replacing length endpoints | NLL par; A/B **−0.0037 ±0.0028** plain; ships off | [evaluator-net.md](ai-bidder/evaluator-net.md) § Shape-distribution reading |
| strength kernel | exact HCP distribution | replacement **lost** (+0.0011 NLL); the crisp band failed its gate | same doc § Strength-distribution reading |

The mechanism behind all four: at a recurring node the training labels came
from real deals distributed over the union at their true masses, so **the hull
row's fitted (μ, σ) already is the union-conditional**. Re-conditioning at
serve time adds extrapolation error and no information.

The literature mostly agrees (§3.2): an auxiliary belief loss hurt in two
independent papers, and the strongest published bidder uses no belief at all.
The one positive result fed belief to the policy as *inputs*, against a weaker
baseline.

Re-open only on the ledger's own triggers: a reading-fidelity jump that makes
box identity informative, or a training-side per-box-conditioned corpus.

## 3. Literature

Verification legend, per claim:

- **✓** — checked this session (2026-10-01) against the paper's text or the
  project's source.
- **P** — a research subagent reported reading it in the primary source; not
  independently re-checked.
- **S** — secondary source, abstract, or search snippet.
- **R** — recalled; unverified.

### 3.1 How bridge engines infer hidden hands while bidding

| engine | inference | what it is used for | status |
| --- | --- | --- | --- |
| GIB (Ginsberg 2001) | deals to suit-length constraints, then keeps deals on which its own bidding database reproduces the observed calls — a **hard filter** | Borel simulation: project each candidate's auction with the database, score double-dummy, take the best total | ✓ |
| GIB on BBO | ~20–30 deals per bidding simulation; a bonus to the book bid; simulation suppressed when the book bid is likely enough | close decisions among lower-priority rule matches | S (forum reverse-engineering) |
| BEN | a `binfo` net predicts per-seat HCP and shape and seeds the dealer; each deal is scored by the minimum net probability of that seat's actual calls | roll out each candidate with the net to pass-out, score DD, add a net-prior adjustment; low quality or too few samples → trust the net | P (`sample.py`, `botbidder.py`) |
| BEN, the numbers | `sample_boards_for_auction = 30000`, `sample_hands_auction = 200`, `min_sample_hands_auction = 15`, `bidding_threshold_sampling = 0.70`, `bid_accept_threshold_bidding = 0.40`, `bid_extend_bid_threshold = 0.01`, `exclude_samples = 0.01`, `search_threshold = [0.10, 0.07, 0.06, 0.05, 0.04, 0.03, 0.03]` | — | ✓ (`src/config/default.conf`) |
| WBridge5 | Monte Carlo simulation on a human rule system | not sourced | S |
| Jack | sampled deals; a rule base anticipates partner's reply instead of nested simulation ("invite when partner decides correctly on enough samples") | invitations | S |
| Q-plus | rule-based, "dealing many possible hands and then calculating the outcome" | judgement calls | P (quote), S (detail) |
| Synrey | Monte Carlo search on a human system | not sourced | S |
| Blue Chip | large database with fuzzy best-fit; reconstructs hands from the bidding in play | explicit range reasoning in bidding **not confirmed** | S |
| BBA / EPBot | deterministic rules, no simulation | — | P (site); our own decompile agrees ([bba-floor.md](ai-bidder/bba-floor.md)) |
| Micro Bridge, Shark, RoboBridge, Bridge Baron, Argine | no technical bidding detail found | — | R |
| NooK (NukkAI) | declarer play only | — | S |

Ginsberg on what a hard filter does to a simulation (✓, JAIR 14 §4): the Borel
algorithm "introduces substantial instability"; if the database is conservative
each player assumes a conservative partner and "the partnership as a whole ends
up overcompensating"; and where the database has a hole the simulation exploits
it — GIB bids 7♦ "often on the grounds that doing so will cause the opponents
to make a mistake". His remedy is to restrict candidates to calls near the
database's.

### 3.2 Academic bridge bidding — does an explicit belief help?

| paper | belief | did it help? | status |
| --- | --- | --- | --- |
| Rong et al. 2019 | an estimation net (ENN) outputs **52 probabilities** for partner's cards, fed to the policy net as features | **yes**: RL-PNN+ENN beats RL-PNN by **1.0856 IMPs**; +0.25 vs WBridge5 | ✓ |
| Gong et al. 2019 ("Simple is better") | auxiliary belief loss, weight `r` | **no**: `r = 0 / 0.01 / 0.1 / 1` → 2.31 / 1.90 / 1.63 / 1.22 IMPs vs their baseline; "training belief using supervision from partner's hand does not help". +0.41 vs WBridge5 on 64 boards | ✓ |
| Tian et al. 2020 (JPS) | auxiliary belief loss, weight `r` | **no**: `r = 0 / 0.001 / 0.01 / 0.1` → 2.99 / 2.86 / 2.77 / 2.53. +0.63 ±0.22 vs WBridge5 over 1000 boards | ✓ |
| Lockhart et al. 2020 | particles (complete deals) selected "based on our estimated probability that they would result in the actual actions observed"; rollouts under a stochastic policy | auxiliary partner-card loss "did not improve performance". **Test-time search did**: compatible policy +0.28 → +0.48 (partnering WBridge5) and +0.36 → +0.56 (team); partnership policy +0.57 → +0.85 (team) but +0.11 → +0.12 partnering WBridge5 | ✓ |
| Kita et al. 2024 | none — supervised pretraining, PPO, fictitious self-play | **+1.24 ±0.19** vs WBridge5 over 1K boards, above the earlier systems its table lists (JPS +0.63) | ✓ |
| Qiu et al. 2024 | a belief net predicts hidden cards for test-time search | +0.98 vs WBridge5; no ablation visible | S (abstract) |
| Yeh & Lin 2016 | none; Q-net on hand + history, uncontested | not tested | P |
| Amit & Markovitch 2006 (PIDM) | model-based Monte Carlo with partner and opponent models | co-trained pair surpassed the then state of the art | S (abstract); mechanics R |
| DeLooze & Downey; Ando & Uehara | — | — | R |

Two readings worth keeping:

- **Belief as a loss term hurts; belief inside search helps; belief as policy
  inputs helped once.** Rong's gain is against a policy with no inference
  machinery at all — pons already hands its nets a reading, which
  [sampled-projection.md](ai-bidder/sampled-projection.md)'s negative control
  prices at 0.65–1.27 IMPs/board. That is the same effect, already banked.
- **Lockhart's caveat is ours too**: "The policies used to filter particles and
  generate rollouts are approximations when partnering WBridge5 or a human", and
  test-time search causes "a divergence between the policy we follow and the
  policy we use to filter particles". Their search gain vanished exactly where
  the partner was not the modelled policy (+0.11 → +0.12).

### 3.3 Other imperfect-information games

**Poker.**

- DeepStack's value net takes both players' *ranges* — probability vectors over
  hand buckets — plus pot and board, and returns per-hand counterfactual
  values. ReBeL and Student of Games generalise this to a public belief state:
  public observations plus a distribution over each player's private states,
  which the value net takes as input. (P)
- The update is Bayes with the policy's action probabilities:
  `posterior(hand) ∝ prior(hand) · π(action | hand)`, then card removal. (P)
- When the opponent model is wrong: "unsafe" subgame solving assumes both sides
  followed the blueprint and has no guarantee (Brown & Sandholm); DeepStack
  avoids tracking the opponent's range at all, carrying its own range plus
  opponent counterfactual values; off-tree actions are re-solved, not
  translated. Pluribus calls belief-conditioned leaf values too expensive and
  uses a few continuation strategies. (P; Libratus specifics R)

**Hanabi** — the closest analogue: a cooperative signalling game.

- **BAD** (Foerster et al. 2019): a public belief updated by
  `P(f | u) ∝ P(u | f, π) · P(f | B)`. The per-card factorisation "can yield
  beliefs that are not even self-consistent", repaired by iterated
  re-marginalisation. The shipped belief **mixes** the Bayesian and the
  convention-free one, `V2 = (1 − α)·BB + α·V1` with `α = 0.01`; the agent
  "conveys around 40% of the information via conventions". (✓)
- **SAD**: ε-greedy exploration turns the 0/1 filter into
  `(1 − ε)·1[greedy] + ε/|U|`, which blurs the posterior. (P)
- **SPARTA** (Lerer et al. 2020): an **exact** belief — the set of hands
  consistent with the partner's policy, up to ~10 million at the start.
  Multi-agent search is gated by a *max range* (10,000 in their experiments);
  an agent deviates from the blueprint only when the search's gain exceeds a
  threshold (0.05). The paper names Bridge as a setting the method may apply
  to. (✓) Search under a wrong partner model can leave a belief containing no
  states; the fix was uncertainty in the partner model plus the deviation
  threshold, since a deviation corrupts the partner's beliefs. (P)
- **Learned Belief Search**: a learned autoregressive belief recovers most of
  exact search's benefit at a fraction of the compute. (S)
- **Off-Belief Learning**: interpret past actions as if played by a fixed
  grounded policy, then iterate; beliefs induced by self-play conventions are
  arbitrary and fragile with an unfamiliar partner. (P)
- Hat-guessing strategies (Cox et al.; Bouzy) are hand-designed modular codes. (S)

**Trick-taking and others.**

- **Kermit, Skat** (Buro, Long, Furtak, Sturtevant 2009): with deterministic
  players `P(move | world)` "will always be either 1 or 0. This makes the
  prediction brittle". They instead learn `P(feature | bid)` tables offline —
  suit lengths, high cards — and score worlds by their product under an
  independence assumption. (✓ quote; P detail)
- **Solinas et al. 2019**: a per-card location net; a configuration's
  probability is the product of per-card probabilities. **Rebstock et al.
  2019** weight sampled states by the product of a human-trained policy's
  probabilities over the history, and criticise the independence assumption.
  Their metric is the True State Sampling Ratio. (P)
- **History filtering** (Solinas et al. 2023): constructing consistent
  histories is hard in general; MCMC for trick-takers. (P)
- **αμ** (Cazenave & Ventos): repairs strategy fusion and non-locality in
  perfect-information Monte Carlo for bridge card play. (S)
- **Belief-free successes**: DeepNash (Stratego) builds no belief and does no
  search; DouZero is search-free; PerfectDou trains with perfect information
  and executes without. (P; Suphx's oracle guiding S/R)
- **Scrabble** (Richards & Amir 2007): Bayes on the opponent's leave with a 0/1
  greedy likelihood, which the authors call overly simple. (P)

### 3.4 General machinery

- **Probabilistic circuits.** A smooth, decomposable circuit gives marginals in
  one pass, and a conditional is a ratio of two marginals (Choi et al. 2020).
  The suit convolution *is* such a circuit: suits are the decomposition, the
  shape constraint Σ = 13 the only coupling. Overlapping boxes need
  determinism to sum, which the counter gets by enumerating atoms and testing
  any-box membership rather than adding box masses. (P; the hardness of general
  DNF counting R)
- **Semantic loss** (Xu et al. 2018): train against `−log P(output satisfies
  the constraint)`, computed by weighted model counting on a compiled circuit.
  DeepProbLog compiles to SDDs and backpropagates through them. (P)
- **Information theory of bidding — evidence is thin.** No peer-reviewed
  analysis was found. Forum-level only: Fibonacci counting of relay capacity; a
  2023 BBO thread clustering hands by expected IMP loss, with the rebuttal that
  halving entropy "does not coincide with maximising our bridge score"; a 2025
  BridgeWinners entropy article. No instance of a convention optimised for
  mutual information under exact counting was found. (S)

### 3.5 Calibration — from precedence to likelihood

Nothing was found on converting precedence-ordered expert rules into
likelihoods. That agrees with
[logit-calibration.md](ai-bidder/logit-calibration.md): a book weight is
precedence, and odds come from the net at a fitted temperature. The usable
pieces from elsewhere:

- an ε-mixture on a deterministic policy (SAD) and partner-model uncertainty
  (SPARTA) (P);
- an α-mix with the grounded belief (BAD) (✓);
- offline `P(feature | move)` tables from logged play (Kermit) (P);
- KL-anchored search, `u − λ·KL(π ‖ τ)` (piKL) (P);
- quantal response and Rational Speech Acts softmax models (R).

### 3.6 What a DNF's log-odds can and cannot be

The calibration doc wants `P(call | hand)`. Counting gives the other
direction: `P(hand set | reading, my hand)`. So an exact DNF logit **cannot**
supply the book's missing odds. What it does supply:

- `P(call | node)` — a node's call frequency over the deal prior (row (iii) of
  that doc's coarsening table), where the calls' regions are box-expressible;
- `P(partner's next call | my hand, partner's reading)` — the observer's
  forecast of the reply. No other component of pons computes this, and Phase 3
  below is built on it.

## 4. Where an exact posterior could pay

Ranked by evidence over effort. Each has a kill gate in §5.

### A. Evidence as a trust gate on a reading

**The defect it targets is measured.** `probe-reading-sound` (2026-07-29,
10,000 deals, BBA at the opponent seats): our boxes exclude the hidden seat's
actual hand **8.24% / 8.34%** of the time at LHO / RHO and **3.29%** at
partner ([evaluator-net.md](ai-bidder/evaluator-net.md) § Reading soundness).
A box that excludes the truth is not a loose prior, it is a wrong one — and
that section ends: *slack on the opponent boxes is the larger lever*.

**The model.** Let `R` be a seat's reading and `m = P(hand ∈ R | my thirteen
cards)`, exact. With `a = P(call | hand ∈ R)` and `e = P(call | hand ∉ R)`,

```text
P(hand ∈ R | call, my cards) = a·m / (a·m + e·(1 − m))
```

or in odds, against the mass `m̄` the reading has for a typical observer:

```text
odds(R sound | my cards) = odds(R sound) × [m / (1 − m)] / [m̄ / (1 − m̄)]
```

`odds(R sound)` per call is what `probe-reading-sound`'s worklist already
measures. `m` spans orders of magnitude — 10⁻¹ for a common opening, 10⁻⁶ when
my own cards contradict the reading — which is exactly the regime a sampler
cannot resolve and a counter can.

**Outside support.** BEN's "quality" is this quantity estimated by sampling and
used the same way (below threshold → stop trusting the samples). SPARTA hit
empty beliefs under a wrong partner model and fixed it with model uncertainty.
Kermit calls the 0/1 filter brittle. GIB's hard filter is the documented
failure.

**The honest limit.** `m` detects only a reading that **my own cards** make
unlikely. BBA's Multi `2♦` read as diamonds was excluded 100% of the time, and
`m` is unremarkable there unless I hold diamonds. So it may not separate wrong
readings from sound ones at all. That is what Phase 1 measures before anything
is built.

**Measured 2026-10-02 — the limit is the whole story.** The mass separates
wrong readings from sound ones, but through the reading's narrowness, not
through my cards (§5 Phase 1, result).

**Do not confuse with the v4 log-mass column** (+0.0007 NLL as a net input,
§2). That fed the mass to the net; this acts on the *reading* before any
consumer sees it.

### B. One-ply reply lookahead ("exact invite arithmetic")

Jack's rule — invite when partner will decide correctly on enough samples —
made exact:

```text
EV(call) = Σ over partner's replies r of
           P(r | my cards, partner's reading) × value(my cards, reading after r)
```

- The reply masses are Venn-cell counts over the next node's rule projections
  (the bit-per-box state gives a ladder's partition directly).
- The value is [`trick_estimates`](../src/bidding/evaluator.rs) on the reading
  the reply would produce — an **in-distribution** query, because that reading
  is exactly the hull the net sees at the next node. This is what MASS's
  narrow-box mixture was not.
- No sampler, no solver.

The literature's guards apply in full: restrict candidates to the book's finite
rungs (Ginsberg), deviate only above a threshold (SPARTA's 0.05; GIB's book-bid
bonus), and expect self-referential overcompensation if both partners run it.

Limits: projections are not rules (a `pred` gate projects ⊤); a floor-made
reply does not project at all; and one ply prices only auctions the reply
ends. The first target is therefore a seam whose reply is terminal — an
invitation and its accept/decline.

### C. A rejection-free sampler (smartstack for DNFs)

Draw a (shape, points) cell by exact mass, then holdings; no rejection for one
hidden hand, and infeasibility is one query instead of `REPLAY_DRY_LIMIT = 2²⁰`
dry draws.

**Sized today and found small.** `probe-replay-yield` at `fb8ce92c`:

| auction | range fill | µs/world | replay fill | µs/world |
| --- | --- | --- | --- | --- |
| `1♥ (2♣) ?` | 100% | 50 | 100% | 220 |
| `1NT (X) ?` | **53.6%** | 1672 | 40.0% | 11224 |
| `(1NT) 2♣ (2♥) ?` | 100% | 83 | 100% | 1852 |
| `1♥ (1♠) - (2♠) 3♥ - ?` | 100% | 207 | 100% | 431 |
| `1NT - 2♦ - 2♥ - ?` | 100% | 6 | 100% | 10 |
| `1♠ - 2♣ - 2♦ - 2♥ - 3♣ - ?` | 100% | 164 | 100% | 238 |
| `2NT - 3♣ - 3♠ - 4NT - 5♥ - ?` | 100% | 37 | 100% | 428 |
| `2♠ - 2NT - 3♠` | 100% | 87 | 60.0% | 5028 |
| seven more book nodes | 100% | ≤ 8 | 100%, or 0% where unreachable | ≤ 118 |

The range sampler fills 14 of 15 auctions at under 210 µs/world; the double-dummy
solve each kept world then pays dominates. The replay shortfalls come from
`rules_accept` — the bidder's own classify — which is not a DNF and which no
counter speeds up. The sampler is off the default bidding path
([logit-calibration.md](ai-bidder/logit-calibration.md) §2). **Parked** until
a search consumer exists.

### D. Considered and not planned

- **Belief vectors as net inputs** (the poker / ReBeL transfer) — closed, §2.
- **Mutual-information convention design** — no prior art; information is not
  IMPs; DD is blind to obstruction ([measurement.md](measurement.md)).
- **Semantic-loss training** — a retrain lever, and retrains are deferred
  ([next-steps.md](next-steps.md)).
- **Empirical `P(call | cell)` tables** (Kermit) — this is what
  [sampled-projection.md](ai-bidder/sampled-projection.md) already derives by
  probing the bidder; the counter would add the normaliser, nothing more.
- **Grounded vs convention posterior pair** (OBL / BAD's α-mix) — pons already
  holds two readings of a seat, the strict table reading and the announced
  disclosure overlay (`probe-reading-sound` tests both). Their disagreement is
  a free trust signal that needs **no counter**; worth one column in the
  Phase 1 probe, not a phase of its own.  **Closed 2026-10-02: the two
  predicates disagree on 0 of 125,961 readings** (and 0 of 125,871 on a
  second seed) — at the default knob state there is nothing to read.
- **Reading forensics by exact frequency** — sampling the real bidder already
  answers it at µs per hand; exactness only matters at nodes too rare to
  sample, which is use C again.

## 5. Plan

Each phase is one or two sessions. A phase starts only when the gate before it
passed, and a failed gate is recorded here as a row in §7 — a refusal is a
result. The gate thresholds below are **proposed defaults: fix them before the
run, not after**.

### Phase 0 — the counter in Rust

*Goal:* `mass(hand, &EnvelopeUnion) → f64`, the probability that one hidden
seat's hand lies in the union given my thirteen cards.

1. Port from `gaugeJoint` (one hand), simplified by a concrete seen hand: per
   suit, enumerate the subsets of the unseen cards (≤ 2¹³), keyed by what the
   boxes read — length, HCP, the upgrade veto, and per-trump support terms;
   convolve over the 560 shapes; test any-box membership per fibre. This is the
   MASS row's design, and its cost target stands: build ≈ 0.35 ms, query
   ≈ 0.1 ms.
2. Semantics: the **lenient** [`Envelope::admits`](../src/bidding/inference/envelope.rs)
   (lengths + `points`), because that is what `EnvelopeUnion::contains` and
   the sampler test. Every `PointScale`.
3. Pins: (i) membership parity with `admits` on random hands × random boxes;
   (ii) closed forms — the unknown envelope gives `C(39,13)`, a lone length
   range is hypergeometric; (iii) agreement with a 50k-draw Monte Carlo inside
   binomial noise (the retired `probe-exact-mass` criterion); (iv) a handful of
   fixtures cross-checked against `companionCount` in the browser console.
4. Placement, proposed default: `examples/common/` until Phase 2 gives it an
   in-crate consumer — no public API moves, so `web/` cannot break and
   `smoke-default` is byte-identical by construction.

*Not in scope:* two hands, three hands, honors by identity, the bit-per-box
signature. Phase 3 adds the signature if it is reached.

**Built 2026-10-02** — `examples/common/mass.rs` (`Counter::new(seen)`,
`count`, `mass`), pinned by `examples/probe-exact-mass`:

| pin | result |
| --- | --- |
| (i) parity | the per-suit keys rebuild `point_count` on 4,000 hands; membership equals `EnvelopeUnion::contains` on every one of ~150M sampled hands |
| (ii) closed forms | unknown = `C(39,13)` exactly; three lone length ranges = the hypergeometric sum exactly, 1,000 hands |
| (iii) Monte Carlo | 3,000 self-play readings × 50k draws: z has mean −0.016, sd 1.005 (worst 3.98); at 500k draws sd stays 1.037, so there is no bias for more draws to expose |
| (iv) browser | `companionCount` in headless Firefox 156 recounts 1,034 self-play readings (up to 22 boxes): 2,068 counts — each fixture's total and its query — 1,884 equal to the integer, 184 past 2⁵³ and within 10⁻⁹, none declined |
| cost | build ≈ 0.37 ms, union query ≈ 0.03 ms (targets 0.35 / 0.1) |

Pin (iv) is repeatable: `probe-exact-mass --fixtures` prints the readings and
[`scripts/web-counter-crosscheck.py`](../scripts/web-counter-crosscheck.py)
serves `web/` with a harness appended to `app.js`, so the browser runs the
page's own code and posts its counts back. The web names my spots by count,
so it counts my hand `Π C(8, spots)` times over; the script divides that out.

`Counter::new(Hand::EMPTY)` counts over all `C(52,13)` hands — the reading's
mass for an observer who has seen nothing, which Phase 1 turned out to need.

### Phase 1 — kill gate for A: does evidence separate wrong readings?

No double-dummy, no retrain, no bidding change.

1. **Step 0 — re-baseline.** Re-run `probe-reading-sound` at HEAD. The 8.3% /
   3.3% figures are from 2026-07-29; the authored-reading program, declared
   opponent readings and the reader retirements have landed since.
2. Extend the probe to emit one row per (decision node, hidden seat): `m`, the
   truth-excluded flag, the seat's last call, whether that call was alerted,
   and whether the table reading and the announced overlay disagree.
3. Report: AUROC of `−ln m` for excluded vs sound readings, pooled and within
   the five calls that head the worklist; precision and recall across
   thresholds; the same for the two-readings-disagree flag alone.
4. **Gate (proposed):** PASS iff, on the opponent seats, some threshold gives
   precision ≥ 50% at recall ≥ 10% of excluded readings, **and** the lift
   survives within calls (pooled AUROC is not just call identity — a static
   per-call prior needs no counter). Roughly: flagging ≤ 2% of readings, half
   of them rightly. Below that the gate cannot pay for its false alarms, each
   of which blanks a sound reading (the blind-inference control prices a
   blanked reading at 1.8–3.9 IMPs per divergent board).
5. A FAIL closes use A and leaves the disagreement flag, if it separates, as a
   counter-free knob candidate.

**Result, 2026-10-02** — `probe-reading-sound -c 10000`, BBA 2/1 at the
opponent seats, gate fixed at the proposed defaults before the run. Seed
1790871004, with seed 1790871902 in brackets.

Step 0, the re-baseline: readings exclude the truth **7.82% / 1.33% / 7.87%**
at LHO / partner / RHO [7.63 / 1.27 / 7.71]. Since July the opponent rate
barely moved (8.24 / 8.34) and partner's fell by more than half (3.29).

Three scores, each rising with suspicion: `−ln m` is the plan's; it splits
into the reading's **narrowness** `−ln m̄` (`m̄` = its mass with no cards
seen, a function of the auction alone) and the **cards' own evidence**
`logit m̄ − logit m` — §4 A's odds formula. Opponent seats, 83,974 readings
[83,914]:

| score | AUROC | best recall at precision ≥ 50% | precision at recall ≥ 10% |
| --- | --- | --- | --- |
| `−ln m` (my cards seen) | 0.845 [0.842] | 33.6% [32.2%] | 88.8% [85.0%] |
| `−ln m̄` (narrowness, no hand) | 0.839 [0.834] | 28.7% [28.1%] | 90.6% [88.0%] |
| `logit m̄ − logit m` (my cards' evidence) | **0.554 [0.552]** | **5.6% [5.9%]** | **28.0% [31.0%]** |

- **Use A as designed FAILS.** `−ln m` clears the letter of the gate, and
  clears it inside each of the five worst calls (AUROC 0.64–0.92). But that
  control was too weak: a seat's *last* call does not fix its reading, and
  the score is the reading's identity — narrowness alone reproduces it. My
  own cards add 0.006 AUROC. Their evidence by itself misses the gate
  pooled and in four of the five calls (it scrapes through after `2♠`:
  11.8% recall at ≥ 50%, 59.7% precision at 10%). This is §4 A's honest
  limit, measured: a foreign meaning is wrong for everyone, not for the
  observer whose cards happen to contradict it.
- **Partner: no signal** on any score (AUROC 0.62 / 0.65 / 0.46; precision
  reaches 50% on at most 0.2% of the excluded readings).
- **The by-product passes.** An opponent reading whose mass is under
  `e^−5.61` ≈ 0.37% of all hands excludes the truth **90.6% [88.0%]** of the
  time. That flags 943 [926] readings — 1.1% of them, 13% of the excluded —
  against the gate's "≤ 2% flagged, half rightly".
- **Where they live:** 369 [370] nodes, a long tail headed by BBA's
  *uncontested* auctions read with our meanings — jump responses and rebids
  (`1♠ - 3♦` 32 of 32 excluded, `1♥ - 3♦` 22/22, `1♥ - 2♠` 20/20,
  `1♠ - 1NT - 3♣` 16/16, `1♣ - 2♣ - 2♦` 16/16) and reverses
  (`1♦ - 1♠ - 2♥` 16/22). We presumably pass through most of these — not
  counted yet — which is the reason to doubt the IMPs before building
  anything.

### Phase 2 — the gate as a knob (only if Phase 1 passes)

**Re-scoped 2026-10-02; counted the same day and not built (below).** Phase 1 failed for the
hand-conditional gate this section describes and passed for a hand-free one,
so the candidate is now: *an opponent seat whose reading has prior mass under
≈ 0.37% reads as `Envelope::unknown`*. Two consequences:

- item 2's design risk **dissolves** — narrowness is a function of the
  auction alone, so it lives in the `Inferences` read and its cache, and the
  one hand-free `Counter` is built once (30 µs a query);
- item 3's expectation **stands**, with a second reason for a small or null
  result: the flagged readings sit in the opponents' uncontested auctions
  ([evaluator-net.md](ai-bidder/evaluator-net.md) already found the
  soundness gap does not show in the trick estimate).

Proposed default: do **not** build it yet. First count, with the existing
dumps, how often our side acts at all at a flagged node; build only if that
is not negligible. The lane-by-lane alternative — reading BBA's jump
responses as BBA plays them — is the declared-opponent reading, which has its
own record.

**Counted 2026-10-02 — negligible; not built.** `probe-reading-sound` now
ends with the reach of the flagged readings (same 10,000 deals, seed
1790871004, second seed in brackets):

| | count |
| --- | --- |
| flagged opponent readings | 943 [926] |
| boards carrying one | 231 [243] of 10,000 |
| boards where our side still makes a non-pass call | **53 [47]** |
| our actions at a flagged reading | 59 [52]: 33 [26] doubles, 26 [26] bids |
| … of them at a reading that was in fact sound | 9 [11] |

- The gate can change a call on at most **0.5% of boards**, and half of what
  we do there is double an artificial call on their way to slam
  (`1♠ - 3♦ - 4♥ X`, `… 4NT - 5♣ X`) — a lead-direction decision made on our
  own cards, which a blanked reading of theirs should not move.
- That leaves ≈ 26 bids per 10,000 boards, mostly an overcall after their
  reverse or jump (`1♦ - 1♠ - 2♥ 2♠`, `1♠ - 3♦ 4♥`). To be worth the 0.001
  IMPs/board this repo ships at, the gate would have to gain 0.4 IMPs on
  *every one* of them, through a floor that never saw a gated reading — and
  one flagged action in six sits on a sound reading, where blanking costs.
- Passes are not counted: a blanked reading could also turn a pass into an
  action. On the other ≈ 180 boards we pass throughout from the flagged
  node on, and which of those passes are live was not examined.

Phase 2 is closed on reach, not on a measured A/B. Re-open if the flagged
set grows — a different opponent (BEN), or a reading-fidelity change that
moves the 0.37% line.

The original text follows, for the record.

Use the `author-convention` and `measure-ab` skills.

1. Opt-in knob, default off; when the posterior that a seat's reading is sound
   falls below the Phase 1 threshold, that seat's offending call reads as
   `Envelope::unknown` — a state every consumer already meets.
2. **Design risk, decide first:** `Inferences` is a function of the auction
   alone and is cached on it
   ([bidding-performance-handoff.md](bidding-performance-handoff.md)). A
   hand-conditional reading cannot live in the read; it must be applied at the
   consumer (feature encoding, `Context`), where the actor's hand is known —
   as `features::shape_of` already does with `Unseen`.
3. **Expectation, stated in advance:** a reading knob is a bidding knob under
   a neural floor ([reading-drift-handoff.md](reading-drift-handoff.md)); the
   v6 floor never saw a gated reading, and retrains are deferred. A loss here
   may be retrain-gated. If it loses with a credible flip plan, it is a
   `park/` branch, not a knob.
4. A/B vs BBA on both scorers per [measurement.md](measurement.md); vs BEN as
   the second reference, since the defect is foreign meanings.

### Phase 3 — reply lookahead, design probes before any build

1. **3a — are the nodes countable?** At candidate authored nodes, compare the
   reply partition's exact masses (from rule projections) with reply
   frequencies from sampled partner hands run through the real bidder. A node
   is *countable* where they agree; a ⊤ projection or a floor-made reply makes
   it not. Census first — if few seams are countable, stop.
2. **3b — does the argmax beat the book offline?** On a corpus of decisions at
   one terminal-reply seam, compute `EV(call)` for the book's finite rungs and
   compare the argmax with the book's call on the true deals, paired, DD and
   PD. Read the seam's existing verdicts first
   ([one-notrump-constructive.md](one-notrump-constructive.md),
   [convention-tuning.md](convention-tuning.md)) — several were already swept.
3. **Gate (proposed):** a paired gain whose interval excludes zero on both
   scorers at the seam, with a deviation threshold tuned on a held-out half.
4. Only then a knob and an A/B. Candidates stay inside the book's rungs.

**3a, the cheap route, 2026-10-02 — FAILS; the ladder is not built.** Item 1
asks for a partition from rule projections, which needs a hand-free view of a
node's rule chain that the crate does not export. The route that needs
nothing new was tried first: read the auction once more under each reply `r`
and take the exact mass `m_r` of the reading `r` leaves of the caller, given
the observer's cards; the forecast is `m_r / Σ m`. `examples/probe-reply-count`
scores it on self-play (all four seats `american()`, 20,000 deals, seed
1790873246, second seed 1791873246 in brackets) against the call actually
made, beside the same masses with no cards seen and the node's own
leave-one-out reply frequencies. Gate fixed before the first run: a node met
≥ 30 times is *countable* iff the made call gets zero mass ≤ 2% of the time
and the forecast's log-loss is no worse than the node's frequencies.

| calls at nodes met ≥ 30 times | log-loss, nats |
| --- | --- |
| exact forecast, observer's cards seen | 1.627 [1.629] |
| the same masses, no cards seen | 1.685 [1.688] |
| the node's own reply frequencies | **1.401 [1.399]** |

- **A reading is not a reply forecast.** Over every legal reply, `Σ m`
  averaged 20 times the caller's mass before the call (200-deal smoke): a
  call the node never makes reads as *anything*. Cut to the calls each node made on some other deal —
  the fairest reply set — the replies still overlap 2.6-fold, and a 90–100%
  forecast comes true 93% of the time.
- **461 [480] nodes have a choice** (their own frequencies lose > 0.05 nats);
  **38 [42] are countable**, and the observer's hand saves 0.05 nats at
  **6 [5]** — two of them on both seeds (the opening passes, `1NT (2♣) -`).
  The other countable nodes are forced replies, countable for free.
- **The target seams are the worst case.** At every invitation met —
  `1NT - 2♠ -`, `1♠ - 3♠ -`, `1♥ - 3♥ -`, `1NT - 2♣ - 2♦ - 2NT -`,
  `1M - 2M - 3M -` — accept and decline leave the **same** reading
  (`probe-call-reading`: opener is 15–18 after both `2NT` and `3♣`, 10–21
  after both `4♠` and a pass), so the forecast is a coin flip — ln 2, or
  ln 3 where a third reply was met — and equal to three decimals with and
  without the observer's hand in every such row. That takes out both factors of §4 B's sum at its first target: the
  reply masses, and `value(reading after r)`, which cannot tell the replies
  apart either.

What is left of Phase 3, and the proposed default — **park it**:

- The ladder route (first-match over rule projections, hence a partition by
  construction) is untested. It needs the node's rule chain without a hand —
  book node, guarded fallbacks, `instinct()` — as new public API, and a
  first-match method on the counter.
- Before that build, the cheaper kill test is 3b with a **sampled** forecast:
  partner's hands drawn from the reading and run through the real bidder are
  the ceiling of any counted forecast.
- And 3b's prior is poor on the record. Its value term is the evaluator, and
  the 1NT invitation seams are already swept
  ([one-notrump-constructive.md](one-notrump-constructive.md) § Evaluator
  verdicts): eight eval-net cells crossed zero in the no-major class, and
  `set_stayman_net_force` won its screen and lost live.

### Parked — use C, the sampler

Re-open when DD-search-at-leaves exists
([ben-gap-campaign.md](ben-gap-campaign.md) Phase 3), or when
`probe-replay-yield` shows a *range* fill below 100% on an auction a live
consumer actually samples. `1NT (X)` is the one such cell today.

## 6. Flags

- **An invitation's answer reads as nothing** (2026-10-02, found by Phase
  3a). After `1NT - 2♠`, opener's `2NT` and `3♣` both leave 15–18; after
  `1♠ - 3♠`, `4♠` and a pass both leave 10–21; likewise
  `1NT - 2♣ - 2♦ - 2NT` and `1M - 2M - 3M`. The book decides these by rule,
  so no call depends on it today, and the auction usually ends there.
  Proposed default: leave it; it belongs to
  [authored-reading-handoff.md](authored-reading-handoff.md) if a consumer
  ever reads past an invitation.
- **`1♠ - 2♠` reads with no spade length** (2026-10-02, seen in passing with
  `probe-call-reading`): points 1–11, ♠ 0–13. Not investigated — it may be a
  hull over boxes or a `pred` gate projecting ⊤. Proposed default: one row in
  the reading-drift queue ([reading-drift-handoff.md](reading-drift-handoff.md)),
  nothing changed here.
- **3a's reply set was cut after the first smoke run.** The gate's thresholds
  were fixed before any run; restricting replies to the calls a node made on
  another deal came after a 200-deal smoke showed unmade calls reading as
  ⊤. The cut favours the forecast, and it still fails.

- **Doc/code discrepancy, recorded 2026-10-01.**
  [dnf-migration.md](dnf-migration.md) said the MARG and MASS implementations
  were archived in `target/marginal-campaign-archive/` and
  `target/mass-oracle-archive/`, with corpora and nets under `target/` and a
  plan file under `~/.claude/plans/`. **None of these exist.** Both rows now
  carry a dated note. Consequence: Phase 0 is a port from `web/app.js`, not a
  one-hour reconstruction.
- **The soundness numbers are two months old.** Everything in §4 A leans on
  8.3% / 3.3%; Phase 1 step 0 re-measures them before anything else.
- **A research subagent's summary conflated two tables**: the belief-loss
  ablation 2.99 → 2.53 is Tian et al. (JPS), not Gong et al., whose own table
  reads 2.31 → 1.22. §3.2 carries both as checked. Claims marked P were not
  re-checked the same way.
- **The Rust counter covers the default `PointScale` only.** `point_count_on`
  is `pub(crate)`, so `mass::points` copies the shipped scale's formula and
  `probe-exact-mass` pins the copy. Phase 0 said "every `PointScale`"; the
  fibre key (shape, HCP, wasted holdings) already carries what the other
  three need. Proposed default: add them as a `match` when the counter moves
  in-crate, not before.
- **Phase 1's probe measures the announced union**, because `Inferences` has
  no public accessor for the strict one. The two agree about the true hand
  at every reading taken, so the exclusion flag is the strict reading's too;
  whether they are the same *set* was not tested, and `m` is the announced
  one's. The planned *alerted*
  column was not built — the gate does not read it and a verdict was reached
  without it.
- **Phase 1's "within calls" control was too weak as written** (a seat's last
  call does not pin its reading). The narrowness split replaced it; the gate
  text above is kept as pre-registered.
- **Counting is exact only relative to the reading.** Pavlicek's own caveat on
  his calculator applies to every use here: what the bidding makes "known" is
  rarely exact.

## 7. Ledger

| date | phase | result |
| --- | --- | --- |
| 2026-10-01 | survey + plan | this document; `probe-replay-yield` re-run (§4 C); nothing built |
| 2026-10-02 | Phase 0 | counter built in `examples/common/mass.rs`; pins (i)–(iii) pass; build 0.37 ms, query 0.03 ms |
| 2026-10-02 | Phase 0 pin (iv) | the web counter agrees on all 1,034 fixtures, headless Firefox 156 — Phase 0 complete |
| 2026-10-02 | Phase 1 step 0 | re-baseline 7.82% / 1.33% / 7.87% (LHO / partner / RHO), 10,000 deals, seed 1790871004 |
| 2026-10-02 | Phase 1 gate | **use A FAILS**: my cards' own evidence AUROC 0.554, 5.6% recall at precision ≥ 50%. By-product **passes**: narrowness alone, AUROC 0.839, 90.6% precision where recall first passes 10% (1.1% of opponent readings flagged). Replicated on seed 1790871902. Logs: `pons-ab-results/exact-posterior/` |
| 2026-10-02 | §4 D disagreement flag | closed — strict and announced readings disagree on 0 of 125,961 readings |
| 2026-10-02 | Phase 2 reach | **not built**: 231 [243] of 10,000 boards carry a flagged reading and our side still acts on 53 [47], half of it doubles of artificial slam-zone calls. Logs: `phase2-reach*.log` |
| 2026-10-02 | Phase 3a, reading route | **FAILS**: log-loss 1.627 against 1.401 for the node's own frequencies; 38 of 461 nodes with a choice countable, the hand helping at 6; accept and decline read the same at every invitation seam. Ladder route unbuilt. Logs: `phase3a-census*.log` |

## 8. Sources

Bridge engines and tools:

- Ginsberg, "GIB: Imperfect Information in a Computationally Challenging
  Game", JAIR 14 (2001) — <https://arxiv.org/abs/1106.0669>
- GIB reverse-engineering thread —
  <https://www.bridgebase.com/forums/topic/88527-reverse-engineering-gib-part-1-a-treatise-on-the-insanity-of-bidding-simulations/>
- BEN — <https://github.com/lorserker/ben> (`src/sample.py`,
  `src/botbidder.py`, `src/config/default.conf`); see also
  [ben-architecture.md](ben-architecture.md)
- Andrews' Deal — <https://github.com/thomasoa/andrews-deal>
  (`lib/handFactory.tcl`);
  <http://bridge.thomasoandrews.com/bridge/deal/commands.html>
- redeal — <https://github.com/anntzer/redeal>
- Pavlicek, Companion Hand Calculator —
  <https://www.rpbridge.net/cgi-bin/xch1.pl>
- Q-plus — <http://q-plus.com/engl/warum/warum.htm>
- BBA — <https://sites.google.com/view/bbaenglish>
- Blue Chip Bridge — <https://www.computerbridge.se/blue-chip-bridge/>
- Bethe, computer bridge review —
  <https://cs.nyu.edu/~pbethe/bridgeReview200908.pdf>

Academic bridge bidding:

- Amit & Markovitch 2006 —
  <https://link.springer.com/article/10.1007/s10994-006-6225-2>
- Yeh & Lin 2016 — <https://arxiv.org/abs/1607.03290>
- Rong et al. 2019 — <https://arxiv.org/abs/1903.00900>
- Gong et al. 2019 — <https://realworld-sdm.github.io/paper/42.pdf>
- Tian et al. 2020 (JPS) — <https://arxiv.org/abs/2008.06495>
- Lockhart et al. 2020 — <https://arxiv.org/abs/2011.14124>
- Kita et al. 2024 — <https://arxiv.org/abs/2406.10306>
- Qiu et al. 2024 —
  <https://ieee-jas.net/article/doi/10.1109/JAS.2024.124488>
- Cazenave & Ventos, αμ — <https://arxiv.org/abs/1911.07960>
- NooK — <https://www.deeplearning.ai/the-batch/bridge-to-explainable-ai>

Poker:

- DeepStack — <https://arxiv.org/abs/1701.01724>
- Safe and nested subgame solving — <https://arxiv.org/abs/1705.02955>
- Pluribus — <https://noambrown.com/papers/19-Science-Superhuman.pdf>
- ReBeL — <https://arxiv.org/abs/2007.13544>
- Student of Games — <https://arxiv.org/abs/2112.03178>

Hanabi:

- BAD — <https://arxiv.org/abs/1811.01458>
- SAD — <https://arxiv.org/abs/1912.02288>
- SPARTA — <https://arxiv.org/abs/1912.02318>
- Learned Belief Search — <https://arxiv.org/abs/2106.09086>
- Off-Belief Learning — <https://arxiv.org/abs/2103.04000>
- Cox et al., hat-guessing —
  <https://www.tandfonline.com/doi/abs/10.4169/math.mag.88.5.323>
- Bouzy —
  <https://helios2.mi.parisdescartes.fr/~bouzy/publications/bouzy-hanabi-2017.pdf>

Trick-taking and other games:

- Kermit (Buro et al. 2009) — <http://ijcai.org/Proceedings/09/Papers/236.pdf>
- Solinas et al. 2019 — <https://arxiv.org/abs/1903.09604>
- Rebstock et al. 2019 — <https://arxiv.org/abs/1905.10911>
- History filtering — <https://arxiv.org/abs/2311.14651>
- DeepNash — <https://arxiv.org/abs/2206.15378>
- DouZero — <https://arxiv.org/abs/2106.06135>
- PerfectDou — <https://arxiv.org/abs/2203.16406>
- Suphx — <https://arxiv.org/abs/2003.13590>
- Richards & Amir, Scrabble — <http://ijcai.org/Proceedings/07/Papers/239.pdf>
- Battleship, expected information gain — <https://arxiv.org/abs/2402.19471>

General machinery and calibration:

- Choi, Vergari, Van den Broeck, "Probabilistic Circuits" —
  <http://starai.cs.ucla.edu/papers/ProbCirc20.pdf>
- Xu et al., semantic loss — <https://arxiv.org/abs/1711.11157>
- DeepProbLog — <https://arxiv.org/abs/1805.10872>
- piKL — <https://arxiv.org/abs/2112.07544>
- Eccles et al., biases for emergent communication —
  <https://proceedings.neurips.cc/paper_files/paper/2019/file/fe5e7cb609bdbe6d62449d61849c38b0-Paper.pdf>
- Useful space principle —
  <https://en.wikipedia.org/wiki/Useful_space_principle>
- BBO threads — relay capacity
  <https://www.bridgebase.com/forums/topic/67919-relay-system-and-fibonacci/>;
  clustering
  <https://www.bridgebase.com/forums/topic/88512-using-hierarchical-clustering-to-generate-a-bidding-system/>
- BridgeWinners, entropy and information —
  <https://bridgewinners.com/article/view/entropy-and-information-in-bridge-2-yvz0ma0tga/>
