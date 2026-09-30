# Floor sweep — verdict narratives, Phase 0 to Phase 2 (2026-09-28–30)

> **Archived 2026-09-29.** Extracted verbatim from
> [`docs/ai-bidder/floor-sweep.md`](floor-sweep.md) (now archived beside this file) at `c35a9036`,
> plus the epochs-600 divergence trace that the axis-3 verdict had left
> untraced. The live doc keeps the rules, the shipped state, one line per
> closed axis, the plan, the wiring, the runbook and the ledger; this file is
> the record of *why* each row reads as it does — the reseed signature and the
> shell-fit reading (Phase 0), the ensemble's PD column and KR3 timing
> (Phase 1), σ_ens and K = 8 (seeds 5–8), and the closed recipe axes.
> Axis 4's width verdict, its full divergence trace, the epochs follow-up
> reconciliation, and Axis 5's training gate were added 2026-09-30.

## Phase 0 — the noise floor

Train seeds 2 and 3 with the manifest command otherwise unchanged. A/B each
against the shipped net, 204,800 bd/arm/vul, both vuls, both scorers.

Read it three ways:

- **Reseeds swing CI-clear (≥ ±0.01 on either scorer).** Then (a) the
  shipped draw is one sample of a wide distribution and Phase 1's ensemble
  is the first lever; (b) every past floor A/B that changed the net *and*
  the recipe in one arm (v8, the three LSTM arms) was confounded by its
  draw — record that in each doc's verdict as a caveat, do not re-run them.
- **Reseeds wash (inside ±0.005).** Seed noise is already solved; skip Phase
  1 and go to Phase 2 with single draws.
- **In between.** Phase 1 still runs; Phase 2 reads every win against σ_seed.

Cost: two trains (20 min, one per GPU), two A/Bs (3 h idle time).

## Phase 0 verdict (2026-09-28)

Rows in the ledger; sd plain was +0.0304 / +0.0149 (seed 2) and +0.0286 /
+0.0211 (seed 3), CI-clear in three of four cells. Fired 15.5% (none), 13.5%
(both). Fidelity passed the filter (val_ce .4172 / .4165 vs shipped .4163).

**Reading: the first branch — reseeds swing CI-clear**, on both scorers
(plain none +0.022 and +0.015; PD none −0.020 on seed 3). But the swing is
not symmetric noise around seed 1. **Both reseeds moved the same way on both
scorers — plain DD up, PD down** — and with the same mechanism the v8 trace
found (§5 of [features-v8.md](../ai-bidder/features-v8.md)) and the LSTM's overbidding
diagnosis ([plan.md](../ai-bidder/plan.md) M5.2):

| none, all 204,800 boards × 2 tables | doubled contracts | auctions ending `XX` |
| --- | ---: | ---: |
| shipped (three control runs) | 5.57–5.62% | 372–433 |
| seed 2 | 6.24% | 603 |
| seed 3 | 6.40% | 750 |
| v8 | 5.83% | 645 |

The worst PD boards are the same family: our side acting over their 4NT
(`4NT 5♠ X`), late doubles and four/five-level redoubles (`3♠ 4♠ X XX`).

**Interpretation — the shell is fitted to seed 1.** The five floor rails
were arbitrated against the seed-1 net, and the accountant collar was
calibrated to its distribution (M5.2's flip plan already says so for v7).
Any other draw of the *same* recipe brings junk actions the shell was never
fitted to catch, and PD prices them. So:

1. **v8 and the three LSTM arms are confounded** (the plan's branch (b)).
   v8's signature (+0.017 / −0.013) sits inside the reseed spread; its
   verdict is "indistinguishable from a reseed", and its trace's causal
   story (the artificial block puts their 4NT off-distribution) is not
   needed to explain it. The LSTM arms' PD losses (−0.024 to −0.043) are
   larger than both reseeds, so "refuted" weakens but does not flip; their
   plain columns (+0.001 to +0.015) are at or below the reseeds'. Caveats
   recorded in both docs; not re-run.
2. **The decision table cannot rank candidates while the shell is seed-1's.**
   Every non-seed-1 net lands on the *suspect* row by construction. Rule 6's
   "rail re-arbitration before the flip" is too late: a candidate's PD column
   already carries the shell-fit tax.
3. **The plain-DD column says the shipped draw is below the mean** of its
   recipe by ~0.015–0.02 IMPs/board (none). Whether that is recoverable is
   exactly what PD, not plain, has to show.

**Next (proposed, not started).** Phase 1 still runs as planned (K = 4,
seed 4 to train) — averaging logits should shrink exactly the idiosyncratic
junk actions — but read its PD column against this table, not against zero.
If it also lands on the *suspect* row, the lever is re-fitting the shell to
the candidate (collar retune first: one knob, and the LSTM flip plan's arm 1
is the same work), not another recipe axis.


## Phase 1 — the ensemble (variance reduction at the same recipe)

`K = 4`: seeds 1–4, seed 1 being the shipped blob. Average the **logits**
(the 38-vector, elementwise mean) before the shell masks and argmaxes.
Logits, not probabilities: the consumers downstream read margins and
precedence, not odds ([logit-calibration.md](../ai-bidder/logit-calibration.md)), and a
mean of logits keeps the shipped calibration story intact. Serving cost is
`K × (176·256 + 256·256 + 256·38) ≈ 0.5 MFLOP` per decision, invisible
beside a single double-dummy solve (KR3 holds).

One A/B vs shipped. Ship if the decision table says so and the rails
re-arbitrate; try `K = 8` only if `K = 4` wins. Shipping form is `K`
embedded blobs (≈ 0.5 MB each): an ensemble does not fold into one MLP.
Size is settled (jdh8, 2026-09-28: disk is cheap); **compute is the KR3
budget**. Before a flip, time a bidding-only run (no DD) at `K = 1` vs the
candidate `K`; the forward pass is ~3 µs per decision at ~40 GMAC/s, so
`K = 4` should add ~10 µs per contested off-book decision — confirm it,
and let a measurable slowdown veto `K = 8`.

## Phase 1 verdict (2026-09-28)

**K = 4 wins every cell, on every scorer**, CI-clear by 4–7σ; row in the
ledger. Fired 9.97% (none), 8.70% (both), +0.45 to +0.89 IMPs/fired. sd
plain +0.0409 ±0.0088 / +0.0502 ±0.0103. Decision table: *win / win*.

The PD column is the news. Each member alone lands on the *suspect* row
(plain up, PD down); their logit mean lands on neither: PD is the
**largest** column, and the junk-double signature is gone —

| none, all 204,800 boards × 2 tables | doubled contracts | auctions ending `XX` |
| --- | ---: | ---: |
| shipped (this run's control) | 5.57% | 434 |
| K = 4 | 5.60% | 403 |
| (both vul: shipped / K = 4) | 4.67% / 4.62% | 434 / 383 |

So Phase 0's reading holds in the direction it predicted: the reseeds'
PD tax was idiosyncratic per-draw junk, which averaging cancels, and the
seed-1 shell does not need re-fitting to serve the ensemble. The plain
gain (+0.045 / +0.056) is 2–4× a single reseed's, i.e. more than "the
shipped draw was below its recipe's mean" — the variance reduction
itself is worth IMPs.

**KR3 timing (2026-09-28).** `examples/floor-timing.rs`: pons vs pons,
both sides on the floor under test, 20,000 seeded boards, single thread,
three interleaved repetitions, the box lightly loaded:

| floor | µs/board | µs/call |
| --- | ---: | ---: |
| `american` (embedded, K = 1) | 156–164 | 15.5–16.3 |
| `american-file`, K = 1 (shipped blob) | 159–166 | 15.7–16.4 |
| `american-file`, K = 2 | 221–231 | 22.1–23.1 |
| `american-file`, K = 4 | 351–358 | 35.0–35.8 |

Linear, **+65 µs/board per extra member**, so K = 4 is 2.2× the bidding
latency and K = 8 extrapolates to ~620 µs/board (~3.9×). The prediction
above ("~10 µs per contested off-book decision") was wrong by the call
count, not the pass cost: 65 µs ÷ ~3.2 µs per forward pass ≈ 20 passes per
board, i.e. **about two per call, on every call**, not on the ~10% that
diverge. Where the net runs on book-answered calls (the shell, a reading, a
cache miss) is unexplained and is the cheapest KR3 lever if it matters.
In the real harness the cost mostly vanishes behind BBA and the solver:
`sweep-k4` generation took 105 → 112 s (none) and 104 → 112 s (both),
+7%, and generation is ~7 of the run's 62 minutes.

Reading: K = 4 passes KR3 (the solver-bound pipeline slows ~1%; raw
bidding latency is still ~0.35 ms/board). K = 8 doubles that again for a
gain K = 4 has not shown is unsaturated — jdh8's call.

**Owed before the flip** (rule 6 and Phase 1's own gates):
1. ~~KR3 timing, bidding-only, `K = 1` vs `K = 4`.~~ Done above.
2. ~~Rail re-arbitration of the five shipped rails against the ensemble.~~
   Done below.
3. ~~Embed the three extra blobs; `american()` becomes the mean.~~ Done:
   `neural::classify_bba_v6` is the mean of four embedded draws, so every
   `with_floor_v6` path flips at once. The KR1 proof is the A/B itself: the
   embedded bytes are the sha256-pinned `sweep-k4` blobs, in the measured
   order (float summation is order-sensitive), through the same
   `classify_v6_mean`.
4. `K = 8` (seeds 5–8) — the plan's "only if `K = 4` wins" branch is now
   live; timing from 1. can veto it.

**Rail re-arbitration (2026-09-28).** One lumped arm, not five:
`scripts/ab-floor-rails.sh` adds `norail` (K = 4, all five rails off) to
`sweep-k4` on the same `SEED_BASE` and blobs, so `file` is the rails-on
control. The rails still earn, CI-clear by 7–10σ on every cell:

| rails on vs off, K = 4 | plain none / both | PD none / both | sd-PD none / both | fired |
| --- | --- | --- | --- | --- |
| this run | +0.0283 / +0.0344 | +0.0212 / +0.0252 | +0.0196 / +0.0242 | 0.54% / 0.58% |
| sum of the five seed-1 ship cells | +0.0285 / +0.0366 | +0.0398 / +0.0496 | | |

Plain is unchanged from seed 1's sum. PD is about half, the expected
overlap: the ensemble already drops some of the junk doubles the rails
were vetoing (Phase 1's doubled-contract table). The lumped arm cannot show
that each rail is positive on its own; the two smallest, the game-pull and
2NT-bid vetoes, could have gone inert. They were never large enough to
decide the flip, so they stay on.

No second-`SEED_BASE` confirmation: the plan's seed-2 confirmation is
for Phase 2 recipe axes, and every cell here clears its CI by ≥ 4×.


## Seeds 5–8 — the ensemble noise floor, and K = 8 (2026-09-28)

After the flip, rule 4's control is the K = 4 ensemble, and Phase 0 showed a
single non-seed-1 draw lands on the *suspect* row by construction (the
shell-fit tax, ±0.02 of draw noise). So a Phase 2 arm must be a K = 4
ensemble of its recipe, and its verdict is read against the draw noise of
**ensembles**, not of single nets. Seeds 5–8 measure both at once, in one
`sweep-k8` run on one `SEED_BASE`:

- `k4b` = seeds 5–8 alone vs shipped seeds 1–4: σ_ens, the rule-5 bar for
  every Phase 2 arm.
- `k8` = seeds 1–8 vs shipped: the K = 8 increment. The 1/K model (K = 4
  already holds 75% of the variance reduction, K = 8 holds 87.5%) predicts
  ≈ +0.005 plain, maybe +0.01 PD, for ~1.8× the bidding latency. Unless it
  clearly beats that, K = 8 waits for Phase 3 (best recipe × K), behind the
  unexplained two-passes-per-call cost that could halve it.

The launcher's failed-to-train gate was first shipped + 0.005 (0.4213);
seed 6 landed at 0.4216 (seeds 5/7/8: 0.4174 / 0.4175 / 0.4194) and the chain
halted. Relaunched 2026-09-29 at shipped + 0.010 (0.4263, the dd arms' gate):
0.0003 over a guessed cut is draw spread, not a failed train (rule 2), and
dropping a draw on CE would bias σ_ens low.

`scripts/ab-floor-file.sh` now takes any number of `NAME=a.f32,b.f32,…`
arms against one control; the no-argument form is Phase 0/1's `file` arm.

### Seeds 5–8 verdict (2026-09-29, `sweep-k8`, SEED_BASE 1790624432)

- **σ_ens is below the harness's resolution.** `k4b` sits inside its CI on
  7 of 8 cells (the edge case is PD none, −0.0086 ±0.0081), and all four
  sd cells are within ±0.0025. The two ensembles differ by ≲ 0.007, and that
  is mostly board noise, so a Phase 2 arm needs ≳ 0.01, CI-clear on both
  scorers, before it reads as a recipe. The four DD cells are all
  negative. Seed 1's shell-fit home advantage (Phase 0) survives averaging
  as a small bias toward the shipped seeds, so a new-seed ensemble starts a
  hair behind.
- **K = 8 matches the 1/K prediction and does not beat it.** Plain
  −0.0002 / +0.0048, PD +0.0048 / **+0.0076 ±0.0070**, sd-PD
  +0.0043 / **+0.0072 ±0.0071**. The model predicted ≈ +0.005 plain and up
  to +0.01 PD. That is real but small, and it costs ~1.8× the bidding
  latency, so K = 8 waits for Phase 3 as the plan said.


## Phase 2 axis 1 verdict — `--dd-weight` (2026-09-29, `sweep-dd`, SEED_BASE 1790629068)

Each arm is the K = 4 mean of init seeds 1–4 at that weight; the control is
the shipped K = 4.

- **0.1: a loss.** Every cell is negative. The DD cells sit inside their
  CIs, but the sd cells on none are CI-clear losses on both scorers
  (−0.0092 / −0.0094).
- **1.0: loss / loss.** Plain −0.0133 / −0.0119 are both CI-clear, and all
  four sd cells are CI-clear losses (−0.013 to −0.019). The damage grows
  with the weight.
- **Mechanism (training logs).** At weight 1.0 the DD head works: val MSE
  falls from 26 with no head to 0.02, and the train − val CE gap closes
  from 0.025 to ≈ 0. It is an effective regulariser, but held-out CE does
  not move (0.4165 vs 0.4172), and the calls get worse. The trunk spends
  capacity on trick prediction, and that capacity comes from the
  distinctions that decide the call. It is the textbook failure of a
  shared-trunk auxiliary task. Matching CE while losing IMPs is also a
  reminder that val_ce is not the objective.
- **Axis closed at 0.** Do not retry above 0. Anything below 0.1 is inside
  σ_ens by extrapolation.


## Phase 2 axis 2 gate — `--wd` skipped (2026-09-29)

Seed 2's log at 300 epochs, train / val CE: 0.3986 / 0.4204 at epoch 200 and
0.3920 / 0.4172 at 300. Held-out CE is still falling (−0.003 over the last
100 epochs), and the gap is only 0.025, so by the axis's own rule the net is
under-fit and weight decay is the wrong axis. The next axis is 3,
**`--epochs 600`**. 150 is dropped, because the curve is still descending at
300.


## Phase 2 axis 3 verdict — `--epochs 600` (2026-09-29, `sweep-ep`, SEED_BASE 1790668875)

The arm is the K = 4 mean of init seeds 1–4 trained for 600 epochs; the
control is the shipped K = 4. The trainer has no schedule, and seed 1's
epoch-300 line reproduces the shipped val_ce (0.4163), so each arm net is
the shipped net trained for 300 more epochs. The comparison is clean.

- **Plain: a loss.** DD −0.0104 ±0.0070 (none, CI-clear) / −0.0081 ±0.0084
  (both). sd plain is a CI-clear loss on both cells (−0.0162 / −0.0114).
- **PD: a wash.** DD +0.0041 / +0.0018, sd-PD −0.0035 / −0.0032, all inside
  their CIs.
- **Held-out CE improves while the calls get worse.** Final val_ce is
  0.4091 / 0.4118 / 0.4124 / 0.4178, about 0.005 below each seed's
  epoch-300 value, and top-1 rises 0.2–0.3 pp. The train − val gap stays
  near 0.027, so this is not classical overfitting. After axis 1, this is
  the second axis where the imitation metric and IMPs disagree. The shell
  (rails + collar) was fitted to 300-epoch nets, and sharper logits may
  cross its thresholds differently. That was Phase 0's lesson, and it is
  untested here.
- **Axis direction: do not train longer.** Under the decision table, a plain
  loss with a PD wash is closer to *loss* than to *inside σ_ens*, so the
  candidate is discarded. 150 epochs was dropped only because the CE curve
  was still falling at 300. That reason is now void, so 150 is a live
  candidate if the budget allows. Next in order is axis 4 (`--hidden`),
  which needs the `HID`-generic change in `neural.rs`.


*The shell hypothesis in the last bullet was tested the same day: see
[the epochs-600 divergence trace](#epochs-600-divergence-trace-2026-09-29) below.*

## Epochs-600 divergence trace (2026-09-29)

The axis-3 verdict left its own hypothesis ("sharper logits may cross the
shell's thresholds differently") untested. Traced the same day with
`probe-divergence --imps --jsonl` over both cells of `sweep-ep`
(27,389 divergent boards, priced double dummy on both scorers).

**Nothing reads the floor's logit scale.** The five rails are `-∞` masks
(`neural_floor.rs`), the table takes the argmax (`table.rs`), and the replay
sampler's 3-nat margin (`sampler.rs`) applies only at *authored* nodes, where
the book shadows the floor. Sharpness per se cannot move a call; only the
ranking changed. The shell is not the cause.

**What changed: the sharper net is more conservative, not looser.** By the
class of our first differing call, both cells summed:

| class | boards | plain IMPs | PD IMPs |
| --- | ---: | ---: | ---: |
| passed where the shipped net bid | 11,386 | −8,691 | +7,367 |
| bid where the shipped net passed | 9,038 | +6,267 | −4,692 |
| a different bid | 6,965 | −1,361 | −588 |

Games reached by the shipped net only: 4,940 boards, −7,763 plain. The
worst `call_off → call_on` cells are `4♥ → -` (−983), `X → -` (−967,
but +4,922 PD), `4♠ → -`, `3♥ → -`, `3♠ → -`, `3NT → -`; the best are their
mirrors (`- → 4♥` +1,033). Doubled finals fell from 5.60% to 5.37% (none)
and 4.56% to 4.41% (both); the LSTM's "looser bidding" signature is absent.
No auction prefix loses more than 98 IMPs, so this is a diffuse style shift
across the contested floor, not a hole at a node.

**Reading.** The labels are BBA's calls (one-hot, with rollout overrides on
≈ 7% of rows). Fitting them better bids more like BBA in competition, and
the shipped 300-epoch net's residual against BBA is a *directional*
aggression (it bids the games and raises BBA's labels pass on). Plain DD and
sd plain reward that residual; PD prices it at about zero. Longer training
removes an accidental bias whose sign the two scorers disagree on, which
cross-entropy — one equal weight per row — cannot see. The ensemble won by
removing seed variance; this axis lost by removing bias toward the teacher.
Whether the useful aggression comes from the rollout-override rows (the only
IMP-grounded labels) is the open question; the corpus carries no per-row
override flag, so testing it is relabel-grade work.

Per-board records: the scratchpad `ep600-{none,both}.jsonl` of the session
that ran the trace; regenerate with the command above on the `sweep-ep`
arm directories.

## Phase 2 axis 4 verdict — hidden width (2026-09-30)

**Keep the shipped width 256.** The 512-wide ensemble is *suspect*; 128
fails the training gate and has no IMP measurement. This is the original
Phase 2 Axis 4 (`--hidden`), not the renumbered live doc's former fourth
"Next" item (`--lr`, originally Axis 5).

The completed 2026-09-29 run is `ab-results/sweep-hid`, SHA `0c0c8446`,
`SEED_BASE=1790681184`, 32 × 6,400 = **204,800 boards/arm/vulnerability**.
Control is the shipped K = 4 width-256 ensemble; candidate is the K = 4
logit mean of seeds 1–4 at width 512. Both use the same book and rails.
The frozen M32 corpus, its 20 stems in manifest order, 300 epochs, learning
rate 0.001, zero weight decay / DD weight, batch 4096 and validation
fraction 0.10 are unchanged. All eight width artifacts fold 30 columns.
Blobs and sidecars: `/mnt/ssd-data/jdh8/pons-sweep/h{128,512}-s{1..4}`;
the runner pins the measured 512 blob hashes in `sweep-hid/weights.h512`.

| hidden width | seed 1 CE | seed 2 CE | seed 3 CE | seed 4 CE | gate |
| --- | ---: | ---: | ---: | ---: | --- |
| 512 | 0.407621 | 0.411068 | 0.410023 | 0.410526 | all pass |
| 128 | 0.475769 | 0.484872 | 0.477461 | 0.479849 | all fail |

The gate is the manifest seed-1 CE + 0.010 = **0.42631943**, applied to
each draw. The historical launcher used the rounded threshold 0.4263;
both thresholds give the same decisions for all eight draws.
Better label fidelity admitted 512 to the A/B; it did not rank
it above the shipped net. Width 128's rejection says only that this recipe
misses the fidelity gate, not that 128 was measured to lose at bridge.

| 512 − 256, IMPs/board (95% CI) | neither vulnerable | both vulnerable |
| --- | ---: | ---: |
| plain DD | +0.0017 ±0.0086 | +0.0110 ±0.0102 |
| perfect defense | **−0.0247 ±0.0093** | **−0.0114 ±0.0112** |
| sd-lead plain | +0.0143 ±0.0089 | +0.0174 ±0.0105 |
| sd-lead PD | −0.0080 ±0.0094 | −0.0042 ±0.0113 |

DD fired: 21,036 / 18,280 boards (10.27% / 8.93%); plain +0.016 / +0.123,
PD −0.241 / −0.128 IMPs/fired. SD fired: 29,174 / 25,491 (14.25% / 12.45%);
plain +0.100 / +0.140, PD −0.056 / −0.034 IMPs/fired. PD loses CI-clear
in both cells. SD-PD washes and supplies no positive evidence to rescue it.
No default flip, confirmation run or rail re-arbitration is warranted.

### Full divergence trace

On 2026-09-30, `probe-divergence --imps --jsonl` re-priced all 39,316
divergent records across both vulnerabilities. Totals reproduce the original
reports exactly: +341 / +2,255 plain IMPs and −5,063 / −2,337 PD IMPs.
Every first differing call is ours.

| our first differing call, both cells summed | records | plain IMPs | PD IMPs |
| --- | ---: | ---: | ---: |
| acted where 256 passed | 19,788 | +7,520 | −14,229 |
| passed where 256 acted | 9,993 | −6,252 | +7,533 |
| a different action | 9,535 | +1,328 | −704 |

The wider net is **more aggressive**. The largest negative call-pair
buckets are pass → double (−2,249 PD), pass → 3♦ (−1,670), pass → 3♣
(−1,514), and pass → 4♠ (−1,405). Our game reached only by the candidate
accounts for 6,451 records, +1,840 plain / −10,537 PD IMPs. In the full
204,800 table-A auctions per cell, doubled or redoubled finals rise
**5.567 → 5.910%** (none) and **4.557 → 4.721%** (both); the redoubled
subset rises 196 → 369 and 188 → 308.

The worst common-prefix bucket, with leading passes removed, is
`(1NT) 2♦ (X)` (150 records, −229 plain / −654 PD). That is under 9% of
the total −7,400 PD: removing that entire bucket still leaves −6,746.
The deficit spans many calls and prefixes; no single prefix dominates.
This grouping does not rule out defects in continuations shared across
prefixes, but the trace identifies no isolated repair that rescues 512.

Two worst-board traces illustrate the cost without standing in for the
population count:

- Record 2,097 (both cells), deal
  `N:964.3.AJT962.T83 Q853.AKJ964.43.7 J2.T87.Q8.J96542 AKT7.Q52.K75.AKQ`:
  North first overcalls 2♦ after their 1♥ opening and 2♣ response where
  256 passes. The candidate later plays 5♥ in South's four-card combined
  fit, taking zero tricks; control defends 6♥ by East, down one. −22 PD
  IMPs at each vulnerability.
- Record 139,533, deal
  `N:QJ8632.6.Q62.T65 AK95.KQ3.8.A9842 T7.JT92.AK973.J7 4.A8754.JT54.KQ3`:
  over their 5♦ keycard reply the first change is our 5♥ instead of double.
  North plays a five-card combined fit for two tricks; control defends
  6♥ by West, down two. −21 / −22 PD IMPs (none / both).

The inference from the full count is a broad increase in costly actions,
not a proof about why gradient descent found it. Extra capacity improves
cross-entropy yet fails the bridge objective again. Close this width trial
at 256; do not add a new rail merely to make the wider candidate pass.

Reproduce the trace sequentially under `scripts/idle-run.sh`:

```sh
for vul in none both; do
  scripts/idle-run.sh target/release/examples/probe-divergence \
    ab-results/sweep-hid/h512-$vul ab-results/sweep-hid/plain-$vul \
    --imps --jsonl ab-results/sweep-hid/trace.$vul.jsonl
done
```

`sweep-hid/summarize-width.py` produces `trace-summary.txt` from those
records and the full dumps. The actual Rust `classify_v6_mean` was also
checked against all eight folded artifacts' Candle fixtures: **64/64
argmaxes agree**, maximum absolute logit error **0.000732422 < 0.001**.
The harness, command, output and hashes are in `sweep-hid/parity-*` and
`parity.rs`. This rules out a detected width-layout/folding error on the
fixture rows; it does not substitute for the measured IMP verdict.

The width-generic refactor itself has a separate KR1 proof: independently
rebuilt `smoke-default` at `4d2aa01a` and `0c0c8446`, **20,000 boards**, seed
**1790703165**, covering all dealer × vulnerability cells, produced
byte-identical 722,519-byte dumps (SHA256
`96cb95681f804f3961ba7135b516d8d356115d7d17f26e427a438d981edbf51c`).
Build logs, distinct executable hashes and both dumps are in
`sweep-hid/smoke-width-refactor-seed1790703165/`.

Local checks on 2026-09-30: `cargo fmt`, `cargo test --all-features`,
`cargo +nightly clippy --all-targets --all-features -- -D warnings`, and
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` pass.
**Web has a pre-existing failure:** 26 tests pass, but
`tests::default_mixed_table_matches_the_old_symmetric_table` fails both
alone at HEAD and at independently compiled parent `4d2aa01a`. The mixed
table stops at `1NT (X) - - -`; its symmetric baseline instead continues
`1NT (X) - - XX - 2♣ - 2♠ (X) - - -`. Logs are
`sweep-hid/check-web{,-exact,-parent}.log`. Preserve both routes pending a
separate investigation of that declared/symmetric mismatch; this width
trial changes neither route and does not repair or suppress the test.

## Epochs follow-ups — Pass offset and 150 epochs (2026-09-30)

**Keep the shipped 300-epoch ensemble.** The completed Pass-offset arm
loses on plain DD at both vulnerabilities; 150 epochs failed its training
gate and was never measured in IMPs. These are verdicts on the two tested
recipes, not a rejection of every possible epochs treatment.

### What actually ran

`ab-results/sweep-ep150` contains **only the `ep600p` candidate**, despite
the directory name. Its 2026-09-29 run used `SEED_BASE=1790678007`,
32 × 6,400 = **204,800 boards/arm/vulnerability**, against the shipped
K = 4 `american()` control. The recorded SHA is `4d2aa01a`; the README
discloses an uncommitted width-generic v6 forward pass. The width refactor's
separate byte-identity proof is recorded in the Axis 4 section above.
Results finished at `2026-09-29T11:25:26Z`. Sources are the run's `README`,
`log`, `diff.ep600p.vs.plain.{none,both}.{plain,pd}.txt`, and
`sd.ep600p.vs.plain.{none,both}.txt`.

`ep600p` is the mean of four 600-epoch blobs with **0.0205 subtracted from
each Pass output bias**, leaving every other weight untouched. A binary
comparison on 2026-09-30 verifies exactly one changed float per blob:
index 120832, `b3[0]`, with the expected f32 rounding of −0.0205. All four
SHA256 hashes still match `sweep-ep150/weights.ep600p`. No new environment
flag or library setting was needed: the existing `PONS_FLOOR_WEIGHTS`
loader consumed `/mnt/ssd-data/jdh8/pons-sweep/ep600p-s{1..4}.f32`.

**Calibration discrepancy:** the plan called for matching Pass rate at
served floor nodes. The recorded calibration instead matched the net's
Pass rate on **roughly 423k contested corpus rows**, from 0.7519 to
the shipped net's 0.7506. Those rows are not the live distribution selected
by book fall-through and the floor's masks/gates. The corpus rates are
reported by the historical README/launcher; this reconciliation verified
the bias edit, not a fresh corpus-rate calculation or a served-node match.
Thus this run rejects this **corpus-calibrated offset recipe**; it does not
establish that a floor-node-calibrated tilt was tested. Keep the default
untouched and preserve that distinction rather than silently treating the
planned experiment as completed exactly as specified.

| `ep600p` − shipped, IMPs/board (95% CI) | neither vulnerable | both vulnerable |
| --- | ---: | ---: |
| plain DD | **−0.0092 ±0.0071** | **−0.0136 ±0.0084** |
| perfect defense | +0.0005 ±0.0077 | −0.0092 ±0.0092 |
| sd-lead plain | **−0.0132 ±0.0074** | **−0.0159 ±0.0086** |
| sd-lead PD | −0.0057 ±0.0078 | **−0.0122 ±0.0092** |

DD fired: **14,712 / 12,637** boards (7.18% / 6.17%). Plain totals are
−1,886 / −2,791 IMPs, or −0.128 / −0.221 IMPs/fired; PD totals are
+100 / −1,882, or +0.007 / −0.149 IMPs/fired. SD fired: **20,406 / 17,659**
(9.96% / 8.62%). Plain totals are −2,707 / −3,258 (−0.133 / −0.184 per
fired); PD totals are −1,168 / −2,497 (−0.057 / −0.141 per fired).
The rounded DD-PD vulnerable interval touches zero, so it is not called a
clear loss here. Plain DD already rejects the candidate in both cells,
and SD-PD provides no rescue. There is no same-seed unshifted-600 arm in
this run, so these numbers do **not** isolate the offset's incremental
effect relative to 600 epochs without the offset.

### Bounded worst-board trace

The union of the five worst deals in each of the four DD reports contains
**11 unique deals**. Repricing those deals at both original vulnerabilities
produces **17 divergent records** (9 none / 8 both); all **20/20** published
worst-board swings reproduce exactly. The five other vulnerability-records
have identical reached contracts and are omitted by `probe-divergence`.
This is the selected losing tail, not a population sample or a full trace.

The old `probe-layer-replay` executable predated the ensemble loader. With
no bidding A/B running, it was rebuilt from `af10a57d` without source
changes, then used with the original candidate blobs and current shipped
control. **All 209 recorded NS calls replay exactly** across both arms;
every first differing call in the 17 records is floor-owned. The following
examples therefore identify floor decisions, rather than guessing from
the auction alone. Indices below are local to the extracted tail files;
`selection.json` maps each back to its original shard and row.

| tail record | first differing call, shipped → candidate | consequence |
| --- | --- | --- |
| 0, none | South's `4♠ → -` after `2♠ (3♠) 4♥ -` | South has `AQJ854.98.84.Q98`; North has six spades and no hearts. Candidate passes out 4♥ in a two-card combined fit, taking three tricks; control reaches 6♠ making. −16 plain / −21 PD. |
| 2, both | North's `3♠ → -` after `(1♥) 1♠ (2♥) 3♥ (X)` | North holds `A8752.6.9876.QJT`, and partner holds three spades. Candidate leaves 3♥ doubled in a three-card combined fit, taking two tricks; control defends 5♥ doubled down two. −21 on both scorers. |
| 3, none / both | North's `2♦ → -` after `1NT (X) - - XX -` | North has zero HCP, `T98.7643.7643.95`. Candidate leaves 1NT redoubled down five. Control escapes to 3♣ by East (none) or 2♦ by North down five (both). Plain −19 / −20; PD −14 / 0. The vulnerable PD wash erases the real redouble difference. |
| 7, none | South's `5♣ → 5♥` after `(1♠) - (2♠) 4NT -` | South holds `T763.JT87.K.KQ54`; North is 1=0=6=6. Candidate plays 5♥ doubled in a four-card fit, taking two tricks; control plays 5♣ making twelve. −21 on both scorers. |
| 8, none | North's `X → 5♦` after `3♣ (3♠) - (4♦) - (4NT) - (5♣)` | North holds `QT.8643..KQJT952` and bids diamonds on a void; partner has two. Candidate takes zero tricks in 5♦ doubled, while control defends their making 5♦. −20 on both scorers. |

The selected tail contains **both harmful passes and wrong non-Pass
choices**. A shared Pass bias cannot repair the ordering of 5♣ versus 5♥
or double versus 5♦: it moves only Pass relative to the other logits.
Matching one aggregate Pass rate also cannot say *which* positions should
pass. Those are mechanisms visible on these boards, not a claim that they
account for most of the population loss. Some are failures to continue or
escape an already artificial-looking auction; the verified decisions are
on the floor, with the same authored book in both arms. This bounded trace
does not establish a new, isolated missing book continuation that would
rescue the recipe, nor does it rule out a shared floor improvement.

Artifacts are in `ab-results/sweep-ep150/tail-20260930/`: `extract.py`,
`selection.json`, the four extracted arm dumps, `trace.{none,both}.jsonl`,
and `layers.{plain,ep600p}.{none,both}.jsonl` with their logs. Reproduce
sequentially (build the two probes first only if needed, with no A/B live):

```sh
R=ab-results/sweep-ep150/tail-20260930
python3 "$R/extract.py"
export PONS_FLOOR_WEIGHTS=$(cut -d ' ' -f 3 ab-results/sweep-ep150/weights.ep600p | paste -sd,)
for vul in none both; do
  scripts/idle-run.sh target/release/examples/probe-divergence \
    "$R/ep600p-$vul.json" "$R/plain-$vul.json" \
    --imps --jsonl "$R/trace.$vul.jsonl" --show 11
  scripts/idle-run.sh target/release/examples/probe-layer-replay \
    "$R/ep600p-$vul.json" --floor american-file \
    --jsonl "$R/trace.$vul.jsonl" --out "$R/layers.ep600p.$vul.jsonl"
  scripts/idle-run.sh target/release/examples/probe-layer-replay \
    "$R/plain-$vul.json" --floor american \
    --jsonl "$R/trace.$vul.jsonl" --out "$R/layers.plain.$vul.jsonl"
done
```

### 150 epochs: training gate only

All four 150-epoch seeds finished training and folded 30 constant columns.
Validation CE from `/mnt/ssd-data/jdh8/pons-sweep/ep150-sN.json`, rounded
here to nine decimals:

| seed | validation CE | gate |
| --- | ---: | --- |
| 1 | 0.422818601 | pass |
| 2 | 0.424135476 | pass |
| 3 | 0.427407265 | fail |
| 4 | 0.430960476 | fail |

The predeclared per-draw gate is shipped seed-1 CE + 0.010 =
**0.42631942987442017**. The launcher's rounded 0.4263 threshold makes the
same decisions. `/mnt/ssd-data/jdh8/pons-sweep/launch-ep150.sh` drops the
whole K = 4 arm when any seed fails; `ab-results/sweep-ep150.log` records
seeds 3 and 4 being rejected before bidding. Thus **150 epochs has no IMP
verdict**, and dropping the two weak draws to measure a selected K = 2
would be a different experiment. No retraining or A/B is owed for this
already completed gate check; the next planned recipe axis is learning
rate.

## Phase 2 axis 5 — learning rate training gate (2026-09-30)

**Keep `--lr 0.001`.** All four draws at `--lr 0.0003`, 300 epochs
completed, but seed 1 narrowly exceeds the predeclared per-draw CE gate.
The K = 4 candidate therefore never entered A/B: **zero evaluation boards,
no IMP verdict**. This is a gate rejection of this recipe, not evidence
that lower learning rates lose at bridge.

| seed | validation CE | gate |
| --- | ---: | --- |
| 1 | 0.426501274 | fail |
| 2 | 0.424732804 | pass |
| 3 | 0.423721939 | pass |
| 4 | 0.425541580 | pass |

The cutoff remains shipped seed-1 CE + 0.010 = **0.42631943**. Seed 1
misses by **0.000181844**; the other three pass. Do not describe this as
an optimization failure or a measured score loss. Widening the gate or
dropping seed 1 after seeing these results would change the experiment.
The planned cosine-schedule follow-up was conditional on a learning-rate
signal; no IMP signal was obtained, so no schedule was added or tested.
Phase 2 supplies no replacement recipe; Phase 1's K = 4 stays shipped.

### Reproduction and provenance

Run SHA: `af10a57d8407e1ddadebe0664acbaef4fd39d0a7`. Only learning rate
changed from the shipped recipe: 176-feature v6 MLP, width 256, 300 epochs,
zero weight decay and DD weight, batch 4096, validation fraction 0.10,
init seeds 1–4. The frozen M32 corpus retains its 20 stems in manifest
order, **6,766,821 rows**, with 6,090,135 train and 676,686 validation rows.
Each draw folded the same **30 constant input columns**, leaving 146 live.
No corpus dump, relabel or new bank slice was needed.

The interrupted attempt had stopped seeds 1 and 2 before export. Its logs
and provenance were preserved under
`/mnt/ssd-data/jdh8/pons-sweep/lr3e4-interrupted-20260929T183033Z/`.
The existing `train-lr3e4.py` was rerun under `scripts/idle-run.sh`, one
draw on each GPU at a time (seeds 1/2, then 3/4). The restart's four
commands, trainer binary hash and all **60 corpus-file hashes** matched
the interrupted attempt. No checkpoint was available, so these were fresh
draws from the same initial seeds. The runner completed with exit 0.

Artifacts are `/mnt/ssd-data/jdh8/pons-sweep/lr3e4-s{1..4}` with `.f32`,
`.json`, `.fixture.json` and `.log` siblings. `ab-results/sweep-lr/README`
records the disposition; `training-provenance.json` holds the exact four
commands, corpus/trainer hashes, gate and SHA; `training-summary.json`
holds each final CE, folded-blob hash and gate result. No evaluation seed
was allocated. These training-only artifacts were not embedded or shipped.

Local checks pass: `cargo fmt`, `cargo test --all-features`,
`cargo +nightly clippy --all-targets --all-features -- -D warnings`, and
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`.
Logs are `ab-results/sweep-lr/check-{fmt,test,clippy,rustdoc}.log`.
No Rust source or public API changed; `web/` was not rerun.
