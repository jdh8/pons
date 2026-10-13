# Bridge research methods and open corrections

Recorded 2026-10-14 against
[a73db546](https://github.com/jdh8/pons/commit/a73db546972ac49dfc424c9f5152aa9473e60f67).
This ledger tracks corrections to experiment design and interpretation before
further convention comparisons or retraining. P1 means address before relying
on the affected conclusion; P2 means a limitation or efficiency improvement
with a defined follow-up. A confirmed implementation or reasoning defect does
not by itself prove that a shipped treatment loses IMPs.

The working approach remains conditional generation for individual conventions,
solving only relevant divergences, and batching changes for global retraining.
The open work concerns independence, population coverage, scoring assumptions
and experiment identity. Historical numbers below are the campaign records;
their evidential limits stay attached to each finding.

## Corrections tracked in issues

| Priority | Issue | Completion belongs in |
| --- | --- | --- |
| P1 | [#6 Deal overlap between fresh-seed runs](https://github.com/jdh8/pons/issues/6) | `measurement.md` and affected replication ledgers |
| P1 | [#7 Non-inferiority margin instead of wash as no loss](https://github.com/jdh8/pons/issues/7) | `measurement.md`, the ship rules and affected verdicts |
| P1 | [#8 Direct A/B versus reference-relative IMP gain](https://github.com/jdh8/pons/issues/8) | scorer reports, `measurement.md` and anchor ledgers |

The entries below preserve the remaining review findings. Open an issue when
starting one, link it here, and close its checklist with the measured result or
an explicit decision. Keep previously published outputs identifiable when an
instrument or protocol changes.

## 4 Disclosure in SD dump rescoring

**P1 — recorded harness defect, unresolved.**
[measurement.md](measurement.md#scorers-srcscoringrs) records that changing the ON overcall
reading from 9–14 to an absurd 20–37 reproduces the same IMPs, as does dropping
the modern-negative-double flag. The intended disclosure manipulation is inert.
[ab-dump-sd](../examples/ab-dump-sd/main.rs), `lead_inputs` and the ON/OFF
partnership construction, are the relevant paths. The opposing-seat reader is
a suspected cause, not a completed diagnosis.

Existing results describe routing and auctions under that reader. They do not
establish the full effect of a treatment whose mechanism includes opponents
using our disclosure to choose a lead. The current blanket statement that all
dump-rescore verdicts stand needs a treatment-by-treatment qualification.

- [ ] Trace the leader's own system and its reading of our disclosed system.
- [ ] Make an absurd-band manipulation change the sampled model and demonstrate
  an effect on a diagnostic lead case.
- [ ] Identify affected concealment, lead-direction and disclosure-cost verdicts;
  remeasure those needed for a decision and update their evidence status.

## 5 Filter coverage and conditional comparisons

**P1 — confirmed coverage limitation and overbroad interpretation.**
[measurement.md](measurement.md#the-decision-table) says filters change the
per-board denominator but not the divergent set. In general the filtered
quantity is `E[delta | divergence, filter]`, not `E[delta | divergence]`.
These agree only under an appropriate coverage or distribution argument.

[bba-gen](../examples/bba-gen/main.rs), `is_1nt_opener`, admits 15–17 HCP,
no singleton or void, and at most one doubleton. The
[Landy shape runner](../scripts/ab-landy-notrump-shape.sh) explicitly records
that the wide-shape 1NT slice is invisible to its arms. Reporting per-divergent
IMPs does not recover that excluded population.

- [ ] Run an unfiltered bidding-only preflight and count divergences outside
  the raw-hand filter before interpreting an enriched result globally.
- [ ] Use the union of candidate triggers or separate balanced/wide-shape
  strata, and report the population weights and conditional effects together.
- [ ] Narrow the denominator claim in `measurement.md`.

The point estimate in
[probe-weak-two-major](../examples/probe-weak-two-major/main.rs) is internally
consistent: its mean is over boards reaching the face and its density is
reach/draws, so their product is total swing/draws. Its scaled CI treats the
estimated density as fixed; rate uncertainty is a smaller follow-up. Do not
describe this as a demonstrated normalization bug.

## 6 Asymmetric vulnerability in competitive experiments

**P1 — coverage risk with an observed sign reversal.**
[anchor.sh](../scripts/anchor.sh) and many treatment runners use none/both.
Competitive, sacrifice and doubling decisions also depend on favourable and
unfavourable vulnerability. The repo already has a decisive example in
[ab-landy-nt-vs-x.sh](../scripts/ab-landy-nt-vs-x.sh):

| Vulnerability | Plain DD IMPs/board | Plain SD IMPs/board |
| --- | ---: | ---: |
| EW vulnerable | +0.0147 | +0.0042 |
| NS vulnerable | -0.0107 | -0.0206 |

The favourable gate shipped after this four-cell comparison. This is evidence
for extending that practice, not a claim that all existing runners lack it.

- [ ] Confirm competitive treatments at none, ns, ew and both vulnerability.
- [ ] Rebid in each cell; repricing an existing auction cannot model the policy
  changes caused by vulnerability.
- [ ] State explicitly when a verdict covers only the measured cells.

## 7 Auction conditioning and the replay approximation

**P2 — observed range-sampling failure and an overstrong posterior claim.**
[sampler.rs](../src/bidding/sampler.rs), `within_ranges`, uniformly accepts
worlds inside the readings. This does not establish that the bidder would
produce the observed auction on those worlds.

The registered `1M - 2M -` seam experiment in
[exact-posterior.md](exact-posterior.md) measured range PD
`-0.0125 +/- 0.0601` against replay `+0.1388 +/- 0.0518` IMPs per seam decision.
Range left Pass for a try 511 times against replay's 108. The counted-reading
lane was parked. Its subsequent second-seed confirmation is subject to #6.
Exact counting of the reading set does not correct the set's conditioning bias.

Replay is also an approximation: `MARGIN = 3.0` allows calls below the best
legal logit, and `made_plausibly` abstains on unauthored or tombstoned nodes.
Scaling finite authored priorities can leave production argmax unchanged but
change the replay acceptance set. Those priorities are not calibrated call
likelihoods; see [logit-calibration.md](ai-bidder/logit-calibration.md).

- [ ] Describe the existing method as relaxed authored-rule conditioning;
  qualify the self-play posterior and ceiling claims.
- [ ] At representative seams compare strict, full-policy and relaxed replay
  for acceptance, calibration and paired true-deal gain.
- [ ] Use calibrated likelihoods if adding probabilistic opponent conditioning;
  retain the actual-deal judge and static-remap controls.

## 8 Sampling fallbacks and changes to the model

**P2 — implemented behaviour with unmeasured effect on published verdicts.**
[single_dummy.rs](../src/single_dummy.rs), `single_dummy_leads`, fills a short
range sample with unconstrained worlds. [ev.rs](../src/bidding/ev.rs),
`ev_all`, tops up replay with range-only worlds. Mid-play
`sample_defender_remnants` in [sampler.rs](../src/bidding/sampler.rs) can drop
soft auction readings while preserving hard play constraints.

Reading tightness or inconsistency can therefore change both information and
the mixture of fallback worlds. The current returned worlds do not expose the
mixture to the scorer. No particular A/B reversal has been attributed to this.

- [ ] Report draws, acceptance, fallback counts and per-board fallback rates.
- [ ] On important divergent sets compare a common fallback policy or an
  insufficient-sample/no-signal policy, and record sensitivity.

## 9 SD objectives calibration and scenario bounds

**P2 — confirmed model limitations, not an erroneous DDS result.**
[single_dummy.rs](../src/single_dummy.rs), `declarer_choice`, maximizes mean
tricks; the playout interface has strain but no contract level. Mean tricks
and making a contract are different objectives. As an illustrative example,
12 certain tricks make six, while 80% of 13 and 20% of 10 average 12.4 tricks
but fail six 20% of the time. The
[slam battery](../examples/probe-slam-battery/main.rs) already records
mean-max detours losing DD-fair slam cases.

The population-transfer section of
[measurement.md](measurement.md#known-biases) records a 28-percentage-point
6m alignment gap against Pavlicek. Fitting the better-aligned 6M/6NT cells
gives lambda(6) = 0.510 rather than the shipped 0.664 in
[common::SD_BLEND_LAMBDA](../examples/common/mod.rs). A fresh pons holdout
matching the same imported target tests simulation stability; it does not
establish matched human outcomes or the conditional treatment effect.

Plain/PD are scenario sensitivity checks, not guaranteed bounds on a paired
treatment effect. An illustrative same-board NV case has ON at 4♥ down one
and OFF at 3NT down one: both plain scores are -50 and both PD scores -100,
so both reported deltas are zero. An imperfect doubler who doubles only ON
produces a -50-point, -2-IMP delta, outside those endpoints. Bounds for
individual arms do not imply bounds from subtracting like endpoints.

- [ ] Add same-layout, same-strain cases with different contract objectives to
  distinguish safety play from mean-trick maximization.
- [ ] Report sensitivity to the objective, lambda and analytic shave on the
  actual divergent population; validate the aligned-cell fit before using it.
- [ ] Qualify point-estimate and truth-bracket language. A larger sample
  reduces sampling error, not these modelling assumptions.

## 10 Dataset identity and label transplantation

**P2 — confirmed guarantees are incomplete; actual contamination is unquantified.**

[trainer/src/data.rs](../trainer/src/data.rs), `load_mixture`, divides each
stem at `rows - round(rows * val_frac)`. It stores no board boundary for the
split, while [dump-teacher](../examples/dump-teacher/main.rs) emits multiple
auction/cell rows per board. The board-disjoint guarantee is therefore too
strong. In a contiguous, non-overlapping stem this can leak the boundary board;
it is not evidence of widespread leakage or invalid fresh-deal A/Bs.

[splice-labels.py](../scripts/splice-labels.py) keys by hand/context summaries,
vulnerability, one-hot and DD. The context in
[features.rs](../src/bidding/features.rs), `push_context`, does not encode
the full auction. Its positional fast path checks labels/DD/tags without
full row identity. Thus [features-v8.md](ai-bidder/features-v8.md)'s statement
that the key proves identical auctions is not established. This does not prove
the approximately 424k transplanted labels are wrong or explain v8's loss.

- [ ] Preserve a stable deal ID, dealer, vulnerability, full auction prefix and
  cell; group every row of a board into one training/validation partition.
- [ ] Validate both positional and keyed splices against that identity.
- [ ] Audit existing splices and report unmatched, ambiguous or changed rows
  before deciding which labels need recomputation.

## 11 Experiment identity and resumable runs

**P2 — confirmed validation gaps, with no identified published mixed run.**
[ab-lib.sh](../scripts/ab-lib.sh), `arm`, considers an arm complete by shard
count; `diffpair` skips when two report files are nonempty. Reusing a results
directory after changing flags or HEAD can therefore reuse an older run.
[common::load_dump](../examples/common/mod.rs) appends later shards without
checking their labels, vulnerability or configuration. Dumps carry seed/args
but not a full immutable code, weights and scorer identity.

- [ ] Save a manifest with source SHA, dirty diff identity, weights hash,
  generator/scorer/solver versions, configuration and expected shard IDs/counts.
- [ ] Resume only on matching manifests; validate every shard and report.
- [ ] Collect each worker's exit status and verify expected board counts, not
  just matching lengths between two incomplete arms.

## 12 Development banks and solver efficiency

**P2 — proposed workflow improvement, not a change to the standing bank rule.**
[pdd-bank-ledger.md](pdd-bank-ledger.md) prohibits bank-backed A/B reuse because
it correlates experiments. Correlation alone does not bias a paired comparison;
shared development deals can reduce variance between candidates. The danger is
adaptive tuning on the eventual confirmation data or pooling overlapping
samples as independent observations.

Keep training data, reusable development data and a locked confirmation set
separate. This would support multi-candidate and retraining screens while
retaining independent final confirmation. Continue following the current bank
policy until that proposed separation is explicitly adopted.

DD caching already exists in [bba-decompose](../examples/bba-decompose/main.rs)
and [anchor.sh](../scripts/anchor.sh). General
[common::solve_divergent](../examples/common/mod.rs) does not reuse that cache
and requests all strains. A cache or selected-position solve is a benchmark
candidate, not a demonstrated speedup.

- [ ] Define development reuse, data separation and final confirmation policy;
  record deal identities so overlap can be handled correctly.
- [ ] Benchmark cache reuse and reached-position solves against full tables.
- [ ] Count contract/declarer divergence for DD and auction/reading divergence
  for information-sensitive SD work; zero contract divergence does not mean
  zero potential disclosure effect.

## Convention comparisons and global retraining

The [1NT response ledger](one-notrump-constructive.md) and
[7ae1ecb](https://github.com/jdh8/pons/commit/7ae1ecb) support the implemented
`1NT - 3♦` five-five-major package against its then-current routing. The
200k-board, opponents-silenced A/B also repaired missing continuation
capabilities and tuned points floors on the same comparison. It does not
establish the best possible meaning for that bid slot.

- [ ] Compare complete candidate structures, including the hands displaced by
  each meaning, using a common raw-hand filter covering their trigger union.
- [ ] Rank by population-weighted total gain; a higher per-divergent mean at
  equal total gain does not establish a better global threshold.
- [ ] Freeze structure and thresholds before independent confirmation, and
  include reading opponents and doubled/overcalled continuations.

For retraining, distinguish the deployable convention-plus-matched-net package
from the effect of changing the net at fixed agreements. The Kickback
[configured-net experiment](ai-bidder/configured-net.md) found the old gain
was mostly a newer net. Even fixed weights with a changed configuration input
can alter unrelated auctions, so package gain is not pure convention attribution.
Crossed convention/net arms can diagnose this, with unmatched arms labelled
as diagnostics rather than playable systems.

The current floor is the K=8 v6 logit-mean ensemble, with rail re-arbitration;
see [floor-sweep.md](archive/floor-sweep.md). Preserve frozen corpus/recipe
comparisons, multiple initializations, and the distinction between teacher
fidelity and actual IMP performance. This review does not schedule retrains
that are otherwise deferred.

## Repaired instruments and historical evidence

| Case | Status and source | Consequence for reuse |
| --- | --- | --- |
| Flat-eight invite | [PR #4](https://github.com/jdh8/pons/pull/4) and [1NT ledger](one-notrump-constructive.md): DD's pass edge softened under SD-PD | Preserve the scorer and population with the claim |
| SD current-trick sequence collapse | [f6a7a7ae](https://github.com/jdh8/pons/commit/f6a7a7ae) repaired the instrument and calibration | Pre-fix playout results need their corruption warning |
| v5 factory siblings | [bde861ef](https://github.com/jdh8/pons/commit/bde861ef) synchronized the paths; [card-manifold.md](ai-bidder/card-manifold.md) records parity | Check system identity across declared and undeclared paths |
| Multi-double corpus reading | [bank ledger](pdd-bank-ledger.md) records the 2026-08-27 generator repair | Older declared-Multi rows can still carry the stale reading |
| Two PD definitions | [42bd3282](https://github.com/jdh8/pons/commit/42bd3282) unified the scorer on 2026-09-27 | Label pre/post-scorer series; do not attribute that break to bidding |

These are historical defects or revised interpretations, not a list of
unfixed runtime bugs. Preserve the original evidence and the repair together
when reopening a lane.
