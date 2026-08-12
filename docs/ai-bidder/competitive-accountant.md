# Competitive accountant — pricing the 5-level contested decision

**Status: designed 2026-08-12; gates 0, 1 and 2 run the same day and all
passed; the gate is built the same day behind
`InstinctProfile::competitive_accountant` (default off, knob-off byte-identity
proven — see "The gate as built"); nothing measured on boards, ships nothing
until its A/B.** This is the design for the
live remainder of [bba-floor.md](bba-floor.md) §7 row D — doubled-contract
pricing and the contested decisions the constructive accountant deliberately
left out. Evidence and every P(double) number live in the sibling
[doubling-calibration.md](doubling-calibration.md); physics ledger:
[evaluator-net.md](evaluator-net.md); the adjacent *refuted* precedent is
`InstinctProfile::net_collar` (`src/bidding/instinct.rs`, `scripts/ab-net-collar.sh`).
Scope decisions taken with jdh8 2026-08-12: v1 is **one gate at the contested
game-level node pricing all three calls** (pass / bid on / double), doubling
priced by the **empirical q(level, vul) table**, validated by its **own A/B**;
vulnerability-aware from the start.

## Why this decision, and why the floor owns it

The `two_level_minor_overcall_tight` refutation's population pass (the sibling's
first table) showed the knob was a declare-vs-defend switch: the loose default
declares 3.5× as often, gets doubled 3× as often, and still wins 0.58 IMPs per
diverged board under plain DD. The value is earned exactly in the high-level
contested contracts the tight arm never reaches — so the improvable decision is
**"they are in game or higher; do we pass, bid on, or double?"**, not any entry
gate upstream of it.

Nothing prices that decision today:

| the call at that node | today's owner |
| --- | --- |
| Pass advancing our takeout X | settle rail (`settle_floor`, defend-pass w135 at `instinct.rs:4292`, trump-stack forced pass) |
| Pass elsewhere | the unforced-Pass legality rule and the −500 last resort |
| raise partner's suit | the competitive raise ladder — **stops at level 4** (`instinct.rs:4371`) |
| new suit at the 4-level | `free_bid_gate` (`instinct.rs:1357`, settle-floor rein) |
| X | four double rules, **all** gated `their_live_bid_at_most(3)` (`instinct.rs:1458`) — the book cannot double a 4- or 5-level bid |
| everything else | the learned floor's judgement logits, legality-masked, **no accountant input** (`src/bidding/neural_floor.rs`) |

So the node belongs to the floor already, and the iron rule keeps it there: a
book node with finite mass **shadows** the floor, so authoring 5-level book
rules would take these exact decisions away from the net. The gate is therefore
a **floor-side stage**, not book mass — and `context.trick_estimates`' only
consumers remain accountant gates, so this is the second consumer the evaluator
was always priced for.

## The backend question — do we need BBA-style arithmetic accounting?

**No.** Asked directly by jdh8 at design time; three grounds, all measured:

1. **Arithmetic physics lost twice.** The auditable-backend gate
   (`bba-floor.md` §7 row F; `evaluator-net.md` "Is arithmetic enough?"): the
   widest BBA-shaped least-squares rung is 0.34 tricks of MAE and 0.23 nats
   behind the shipped net on the same held-out shard — refused offline, no
   boards spent. Its collar sequel `net_collar` (pair arithmetic as the
   criterion, net as a one-way rider) lost **all four A/B cells CI-clear**
   (−0.031/−0.027 NV, −0.047/−0.037 vul).
2. **BBA has no arithmetic worth copying for this.** §5.5 read Stage 4 off the
   decompiled source: its "expected score" is a points→level ladder
   (`(total_points + 1)/3 − 6` plus eight tabulated overrides) with a
   three-point finite-difference sensitivity bucket — no σ, no
   `Σ P(T=k)·score(k)`. The probabilistic par row D asks for **does not exist
   in BBA**; the §5 gloss claiming otherwise is graded "read, and WRONG as
   stated".
3. **We already own the arithmetic that matters — the economics.** Bridge
   scoring is exact arithmetic: `break_even` is its closed form for
   undisturbed our-side decisions, and `stats.rs`'s `average_ns_par` already
   computes the full `Σ hist(k)·score(k)` par with perfect-defense doubling
   (`normal.min(doubled)`) and a competitive-equilibrium loop over the four
   seats. The accountant's composition — **learned physics × exact arithmetic
   economics** — is the same Stage-3/Stage-4 split BBA itself runs. We keep the
   split and extend the economics side to two declarers and doubled contracts.

What BBA still contributes: the two-sided *structure* (its Stage 3 counts what
they take too) and the unread `expected_double` arithmetic as a calibration
cross-check (sibling doc, no A/B needed). And the auditability argument that
motivated arithmetic survives in the right place: everything in this gate
*except* the trick estimate — the q table, the score sums, the ε margin — is
closed-form and printable; only the physics is learned, and its calibration is
a published table.

## Design

### The three prices

At the trigger node (below), one memoized forward pass (`Context::trick_estimates`)
yields 20 Gaussians; the gate reads two of them — ours and theirs — and prices
three leaf outcomes, all signed to our side, all in raw score points:

```text
P(T = k)   = cdf(k + ½) − cdf(k − ½)          Gaussian::cdf, half-trick buckets
score(C,k) = Contract::score(k, vul)           contract-bridge crate, exact

EV(pass)   = − Σₖ P(T'ₜₕₑₘ = k) · score(their contract,          k)
EV(X)      = − Σₖ P(T'ₜₕₑₘ = k) · score(their contract doubled,  k)
EV(bid C)  =   Σₖ P(Tᵤₛ    = k) · [ qₖ·score(C doubled, k) + (1−qₖ)·score(C, k) ]
               qₖ = P(X | make) ≈ 0.33   when k ≥ needed(C)
               qₖ = P(X | fail) ≈ 0.65   otherwise
```

**`EV(bid C)` corrected 2026-08-12 by gate 2b.** As first written the making
branch was priced `score(C, k)` outright — never doubled — with q on the failing
branch only. The DD check measures `P(X | we make) = 0.27–0.38` at this exact
node, so a third of the contracts we buy here are doubled *and make*. Both
branches carry a rate; the numbers and their derivation are in
[doubling-calibration.md](doubling-calibration.md).

- `their_declarer(context, strain)` is the missing LHO/RHO mirror of
  `our_declarer` (first of their side to name the strain);
  `TrickEstimates::get` already accepts any `Relative` — the opponent columns
  are computed on every forward pass and merely never read today.
- `expected_score(Gaussian, Contract, vul)` is the Gaussian twin of
  `average_ns_par`'s integer kernel (`stats.rs:296`), and lives beside
  `break_even` in instinct's accountant section: economics stays with the
  accountant, `evaluator.rs` stays pure physics. The Φ tails are the point —
  the argument at `evaluator-net.md` ("a floor that cannot see the
  doubled-down-three branch is a floor that overbids") was made for exactly
  this integral.
- Candidate bids priced: each legal bid in a strain **our side has already
  named**, at its cheapest legal level. Jumps and fresh strains stay unpriced
  (net's judgement, rare at this node).

Three deliberate leaf approximations, recorded: EV(X) assumes the double ends
the auction (pulled doubles unpriced); EV(bid) assumes we buy the contract
(their save-over-our-save unpriced); cheapest level only. The exact treatment
of all three is the equilibrium loop `average_ns_par` already implements over
histogram tables — that full probabilistic par is the chartered v2, not v1.
Plain DD holds the veto if the approximations mislead.

Units caveat, also recorded: comparing expected *points* ignores IMP concavity
(the M3.1 lesson — raw points over-reward huge swings). A demoting collar with
a margin is robust to this in a way an argmax bidder is not; the IMP-curve
transform over the per-trick buckets is the named upgrade if forensics blame
the units.

### The doubling model

**Measured 2026-08-12; both halves of this section's original claim were
wrong, in opposite directions.** The sibling's calibration says:

- **q at the trigger is ≈0.52, flat** across level and vulnerability
  (0.487–0.553, four cells, every n ≥ 2,466) — not the 0.03–0.18 level ladder
  the marginal table shows. That ladder is a composition effect: higher
  contracts are more often contested, and being contested is what draws the
  double. Keying the table on `(level, vul)` currently buys almost nothing and
  could collapse to a constant if a second opponent's slice stays flat.
- **q belongs on *both* branches.** The DD check measures a failure-conditioning
  lift of only **1.6–2.5×** at this node — `P(X | fail) ≈ 0.65` against
  `P(X | make) ≈ 0.33`. A third of the contracts we buy here are doubled and
  *make*, so the failing-branch-only treatment below is retired. Marginally the
  lift is 4.8–7.0×, which is the reading that would have been adopted without
  conditioning on the trigger.

Method lesson, standing and now twice-paid: **condition the calibration on the
trigger, not on a proxy for it.** Both errors above came from reading a rate off
the marginal population, and both would have shipped silently into the A/B —
where, per this doc's own opening argument, an A/B cannot arbitrate them.

The original text, kept because its reasoning is still the right reasoning:
`q(level, vul)` — the sibling's calibrated table — applied to the failing
branch only. This realizes the dial the shipped code already
names: `break_even`'s game rows price the failing branch doubled outright — the
comment at `instinct.rs:3404` calls `q = P(doubled | we fail)` interpolating
the plain/doubled brackets "the upgrade if the gates measure close". The
constructive gates keep q = 1 (adverse-selection premium at the firing margin,
measured and shipped); the competitive gate gets the honest empirical rate,
because here the *entire* plain-vs-PD disagreement is the doubling model and
both endpoints are known failure modes (sibling's alternatives table:
always-doubled reproduces the veto bracket, never-doubled reproduces the M3.1
flood).

### Gate shape — a demoting collar, not an argmax bidder

A new stage in the judgement path of `neural_floor.rs`, after legality masking.

**Trigger** (all required): knob on ∧ judgement (not `instinct::forced`) ∧ the
last live undoubled bid is theirs at **level ≥ 4** ∧ our side has named at
least one strain. (3NT-by-them is a candidate extension, recorded, not v1.)

**Action** — reprice only among calls the net already ranks; never introduce
one:

- **mask a candidate bid C** (−∞) when `EV(bid C) < max(EV(pass), EV(X)) − ε`
  — the anti-phantom-save veto, the disaster-tail direction the gate exists
  for;
- **mask X** when `EV(X) < EV(pass) − ε` — the phantom penalty double;
- **demote Pass by a finite logit penalty** when `EV(X) > EV(pass) + ε` —
  never −∞: Pass-always-legal is the floor's invariant, so the missing-double
  lever tilts the argmax rather than forcing it.

ε = `COMPETITIVE_MARGIN`, one underived constant in the `SLAM_ENTRY_P` idiom:
initial value 300 points (≈ the swing of one trick at a doubled 5-level
contract, comfortably above the estimator's per-column MAE priced in points);
**sweep it if the A/B lands close**, and the finite Pass penalty is sized in
the book's ~3-nat logit convention.

### Why this is not the refuted `net_collar`

`net_collar` made pair-point arithmetic the *criterion* and demoted the net to
a one-directional rider; it lost all four cells, and the standing verdict is
that the pair arithmetic was the worse **physics**. This gate inverts that
composition: the net stays the only physics — both sides' Gaussians — and what
is added is **economics** the net structurally lacks: score tables,
vulnerability, q. The net emits tricks; it never sees vulnerability or scoring
(deliberately — "the net is physics, the caller is economics"). Composing net
physics with exact economics is precisely the shape that already shipped and
won twice as `accountant_floor`. Second difference: at the game/slam boundary
there was an authored criterion to collar *toward*; at the contested 5-level
there is none — the choice is not arithmetic-vs-net but **priced-vs-unpriced**.

The honest caveat as first written: the evaluator was trained on the corpus's
auction mix, and its opponent-declarer columns had **never been validated**
separately — while the opponent inference boxes they condition on exclude the
true hand ~8.3% of the time (vs 3.3% for partner; `probe-reading-sound`,
`evaluator-net.md`). That risk was priced *before* boards were spent: it is
pre-A/B gate 1, ε absorbs estimator error at the margin, and plain DD holds
the veto at the end.

**Measured 2026-08-12 and retired.** Gate 1 ran (`examples/eval-columns`,
below): on the gate's own trigger slice the opponent columns cost **+0.0225
tricks** of MAE against ours and cover *better* by 0.42 points — an order of
magnitude inside the 0.15-trick bound, σ factor 1.000. The soundness asymmetry
did not translate into worse physics, which is itself worth remembering before
pricing future work off the 8.3% figure alone.

### Knobs and wiring

- `InstinctProfile::competitive_accountant: bool` — **default off**; off = the
  stage is skipped entirely, output byte-identical to today.
- `COMPETITIVE_MARGIN` and the q table as consts beside `break_even`, the
  table with a provenance comment (seed `1786488117`, sha `abdafcc`,
  `doubling-calibration.md`).
- `bba-gen --ns-competitive-accountant`; runner
  `scripts/ab-competitive-accountant.sh` modeled on `ab-net-collar.sh`
  (SEED_BASE persisted, arms sequential, `idle-run.sh` discipline).
- Public-API addition (field + setter): **`cd web && cargo test` at
  implementation time** — nothing else compiles that workspace.

### What the gate publishes

Nothing. A logit-mask stage is projection-invisible, like
`net_break_even_gate`'s default-⊤ reading (the reach ceiling,
`evaluator-net.md`): sound but blank. Accepted for the same reason as there —
these calls are near-terminal, so the vacuous reading has almost no seat left
to mislead — and recorded rather than fixed. If a continuation ever consumes
this node's reading, the finite-criterion problem returns here first.

## Decisions taken 2026-08-12 (grilling session, before any code)

| decision | answer |
| --- | --- |
| order | both offline gates **before** any gate code |
| v1 action set | all three (veto bid C, mask X, demote Pass) in one arm |
| floors | **both** `ConfiguredFloorV5` and the v4 twin |
| units | raw score points, `COMPETITIVE_MARGIN = 300` const |
| q population | resolved past both candidates — q is conditioned on the **trigger itself** (sibling doc), and comes out flat at **≈0.52** |
| gate-1 failure branch | **σ-inflate and keep all three actions** — *not* the collapse to veto-only this doc originally named |
| σ factor | the coverage-matching scalar, off `eval-columns`' ratio histogram |
| DD-rightness check | blocking, both vuls, both tables |
| attribution | three `AtomicU64` + a plain `pub` accessor, written to the shard JSON |
| coverage criterion | **relative** to the me/partner columns (±3 points); absolute reported beside it |
| gate 0 (new) | trigger rate, pre-registered floor 1% of boards; below it, widen to 3NT-by-them |
| dutch | ships on the `american()` A/B and v4 inherits; record in `docs/dutch-system.md` when the code lands |
| `expected_double` | deferred past the A/B — it cannot change the implementation |

## The gate as built *(2026-08-12, Stage 2)*

Everything above is the design; this section is what the code does, and every
place the two differ.

| piece | where |
| --- | --- |
| `Q_MAKE = 0.33`, `Q_FAIL = 0.65`, `COMPETITIVE_MARGIN = 300.0`, `PASS_DEMOTION = 3.0` | `src/bidding/instinct.rs`, beside `break_even` |
| `expected_score(Gaussian, Bid, vul_declarer, double_rate)` | same section — `Σ P(T = k)·score(k)` over half-trick buckets, both tails folded into 0 and 13 |
| `their_declarer` | `our_declarer`'s mirror, same section |
| `competitive_gate(&mut Logits, Hand, &Context)` | same section, `pub(crate)` |
| the call site | `neural_floor.rs`, one line after `mask_illegal` in **both** `ConfiguredFloorBba` (v4) and `ConfiguredFloorV5` |
| knob | `InstinctProfile::competitive_accountant`, default off, plus `Default`/`nondefault()` |
| harness | `bba-gen --ns-competitive-accountant`, `scripts/ab-competitive-accountant.sh` |
| attribution | `instinct::competitive_counts() -> [u64; 3]` (vetoes, double masks, pass demotions) |

Four deviations from the design above, all deliberate:

1. **q ships as two constants, not a `(level, vul)` table.** Gate 2a measured
   it flat at the trigger — the four cells span 0.487–0.553 for `P(X)` and the
   make/fail split is 0.274–0.381 against 0.613–0.698. A table keyed on
   coordinates that do not move the number is four rows of provenance for
   nothing; the cell ranges are in the constants' doc comments, so a future
   sweep starts from the same evidence.
2. **The whole gate lives in `instinct.rs`**, not split across
   `neural_floor.rs` as the plan had it. It needs `our_declarer`, `pinned`, and
   the accountant constants — all private to that section — so hosting it there
   keeps them private and the floor diff is one line per floor. Economics stayed
   with the accountant either way, which was the actual constraint.
3. **The counters print to `bba-gen`'s stderr summary, not the shard JSON.**
   `scripts/ab-lib.sh` already captures shard stderr into the run log, and the
   dump schema is parsed by every scorer — a new field there buys nothing and
   risks the parsers. One shard is one process, so an arm's totals are a `grep`
   and a sum.
4. **No σ inflation**, gate 1 having passed — the failure branch's `k` never
   materialised.

Knob-off inertness is proven, not argued: `smoke-default --count 20000 --seed 1`
hashes `39a9a31a707a2456f34be5f55dbfcc6510d7ce2344abbc464fb115fbf6ba84a6` at both
`7d8b23b` and the gate commit.

Checks that ship with it: `expected_score_sums_the_trick_distribution` (the
kernel against an out-of-crate `erf` computation of the same sum, plus the
13-trick fold) and `the_competitive_gate_vetoes_the_phantom_save` (`1♠ (5♥)` to
a bust advancer — `5♠` masked, Pass finite, the counter incremented, and the
same auction with a real hand untouched, so the veto prices the *hand*).

Still unbuilt, by decision: the ε sweep (only if the A/B lands close) and
everything in "Out of scope" below.

## Measurement plan

Four gates, in order; the first three are offline and spend no boards.

0. **Trigger rate** *(added 2026-08-12; **PASS**)*. A gate that fires on a
   sliver cannot be resolved by a standard A/B, and learning that after the
   boards are spent is the expensive way. `examples/eval-columns` walks 200k
   real auctions and counts the trigger: **74,919 nodes, 3.733% of judgement
   nodes, touching 31,740 of 200,000 boards = 15.87%** — sixteen times the
   pre-registered 1% floor. The trigger therefore stays at level ≥ 4 and
   3NT-by-them remains a v2 extension. Caveat recorded: both sides bid
   `american()` in that probe while the A/B's opponents are BBA, so it is a
   self-play proxy for reach; the margin is wide enough that the proxy does not
   matter.
1. **Their-columns reliability** (hard gate) *(**PASS**)*. Score the shipped
   `evaluator_v3_dnf` **per declarer column** on the gate's own trigger slice.
   Both criteria are *relative*: LHO/RHO MAE within **0.15 tricks** of
   me/partner, and their coverage at μ ± 0.6745σ within **3 points** of ours.

   Two corrections to this gate as originally written, both made *before*
   running it. First, it asked for an absolute 45–55% coverage band; the
   shipped net sits at ≈48% pooled, so an absolute band mostly grades global
   calibration — a property all four columns share — rather than the
   their-columns question the gate asks. Second, it named "the held-out
   evaluator shard", **an artifact that has not existed since the v3
   campaign**: no `dump-evaluator` corpus survives anywhere on disk, the
   trainer has no weight-load path (so it can only ever score a *fresh* net),
   and `eval-evaluator` walks all four columns but folds them into one `Mean`,
   carries no coverage metric, and scores against the layout sampler rather
   than the deal. `examples/eval-columns` closes that gap — it scores the
   shipped weights against the `.pdd` bank's own double-dummy labels, with no
   solver and no corpus file.

   Measured on 200k deals of `22.pdd` from row 5M (full table in
   [evaluator-net.md](evaluator-net.md) § "Declarer columns"):

   | slice | Δ MAE (theirs − ours) | Δ coverage | σ factor |
   | --- | ---: | ---: | ---: |
   | all | −0.0019 | +0.05pp | 1.000 |
   | contested | +0.0036 | +0.16pp | 1.000 |
   | **gate** | **+0.0225** | **+0.42pp** | **1.000** |

   The opponent columns are **not worse** — an order of magnitude inside the
   bound, with coverage marginally *better* — so no σ inflation applies and all
   three gate actions stand. This contradicts the honest caveat recorded above,
   which reasoned from the reading-soundness asymmetry (opponent boxes exclude
   the truth 8.3% of the time versus 3.3% for partner). A box that is wrong
   more often evidently still yields an equally good trick estimate; the caveat
   is retired as measured, not as argued away.
2. **q calibrated** *(filled; **two design corrections**)*. The sibling's q
   table is measured and its DD-rightness check is run. Both of this design's
   stated assumptions failed: q at the trigger is **≈0.52 flat**, not the
   marginal 0.03–0.18 ladder, and it belongs on **both branches**
   (`P(X | fail) ≈ 0.65`, `P(X | make) ≈ 0.33`, lift only 1.6–2.5×), not the
   failing one. `EV(bid C)` above is corrected accordingly. This is what the
   gate ordering bought: both errors would otherwise have shipped into an A/B
   that cannot arbitrate them.
3. **The A/B.** Standard scale (204.8k boards/arm/vul), fresh
   `SEED_BASE=$(date +%s)`, arms sequential under `scripts/idle-run.sh`, never
   rebuild mid-run, watch the runner PID. Score plain DD + PD, rescore SD-PD
   (`sd-pd-dumps.sh`); pool brackets with `scripts/ab-score.awk` — never
   `ab-aggregate.sh` on `--score both` output (documented `/^Delta/`
   double-count). Verdict from `measurement.md`'s decision table; **plain DD
   holds the veto**; honest-realism pair [plain DD, SD-PD].

| arm | boards/vul | plain DD ±CI | PD ±CI | SD-PD ±CI | IMPs/fired | seed | sha | verdict |
| --- | ---: | --- | --- | --- | ---: | --- | --- | --- |
| `competitive_accountant` on vs off | | | | | | | | *unrun* |

Post-ship only: re-test `two_level_minor_overcall_tight` through the shipped
gate — its measured loss was walking into unpriced 5-level decisions, so a
priced floor is the first legitimate reason to re-open it. Any such re-measure
diffs the fired rate against `abdafcc`'s 1.76%/1.75% first (two runs that fire
differently do not cover the same hands).

## Out of scope (decided, not neglect)

- **Full probabilistic par over all 20 columns** — the chartered v2. The
  machinery exists (`average_ns_par`: kernel, `min(normal, doubled)`,
  suffix-max, equilibrium loop); the work is a Seat↔Relative remap, an
  f64-weight twin of its integer kernel, and a consumer surface. Do it after
  the leaf gate's A/B says the physics holds up.
- **Our under-doubling as a campaign** — the sibling records the 1.2–3.6%
  finding; raising the book's 3-level X wall or auditing the net's X judgement
  is separate work.
- **Pulled doubles, save-over-save, jumps, fresh strains, XX** — the leaf
  approximations above; the equilibrium loop owns them in v2.
- **3NT-by-them trigger** — candidate extension after the level-≥4 gate
  measures.
- **Matchpoints** — BBA's `C_MP_SCORE` switch is real and ours is IMPs-only
  by charter.
- **Evaluator retrain / categorical per-trick heads** — known ceilings in
  `evaluator-net.md`; nothing here needs them yet.
- **Backfilling q into `break_even`'s game rows** — the constructive q = 1
  premium is shipped and measured; unify only if the competitive gate's
  forensics implicate it.
- **Re-litigating `net_collar`** — different criterion, settled loss.
