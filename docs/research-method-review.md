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

Every entry was re-verified against the code on 2026-10-14; the corrections
from that pass (overstatements, missed prior records, a sharper diagnosis) are
folded in below.

## Closed issues

Issues [#6](https://github.com/jdh8/pons/issues/6),
[#7](https://github.com/jdh8/pons/issues/7),
[#8](https://github.com/jdh8/pons/issues/8) and
[#9](https://github.com/jdh8/pons/issues/9) are closed. Their ledgers, the
[#7 non-inferiority inventory](archive/research-method-review-closed.md#issue-7-non-inferiority-inventory) with
its re-measures and recorded trade-offs and the
[#9 cell-count audit](archive/research-method-review-closed.md#issue-9-cell-counts-cited-as-replication),
the [filter-scope rule](archive/research-method-review-closed.md#filter-scope-the-rule-and-the-runner-classification)
with its runner-by-runner classification, the finished parts of §4, §5 and §7, and all of
[§6](archive/research-method-review-closed.md#6-asymmetric-vulnerability-the-evidence-and-the-four-cell-runs)
(closed 2026-10-18 with the last four doubling knobs) and
[§11](archive/research-method-review-closed.md#11-experiment-identity-and-resumable-runs)
(closed 2026-10-18 with the BEN fleet manifest) are archived verbatim in
[archive/research-method-review-closed.md](archive/research-method-review-closed.md).

Once an item here is done, record the measured result or the explicit
decision in the doc that owns the conclusion, then move the item's record to
that archive (or delete it when the owning doc already holds everything).
Keep previously published outputs identifiable when an instrument or protocol
changes.

## 4 Disclosure in SD dump rescoring

**P1, harness fixed 2026-10-16.** `ab-dump-sd`'s East–West leader read only
our alerted calls, so an unalerted natural call's reading was never priced
when we declared. `common::TableReader` now reads each pair's calls through
its own book: `ab-dump-sd` in `7c60e309`, the self-play harnesses in
`430fac29`. Reports print `leader reading: disclosed|legacy`, and
`--legacy-leader-reading` reproduces old outputs; the default system is
byte-identical. The cause, the fix and the re-measures are
[archived](archive/research-method-review-closed.md#4-disclosure-in-sd-dump-rescoring-the-fix-and-the-settled-re-measures).

Settled from the re-measures: the natural overcall of 1NT ships HCP floor 0
with the authored advance
([the sweep](defensive-overcalls.md#every-live-floor-against-floor-0-the-sweep-2026-10-16--floor-0-ships));
the weak-two `5:10` HCP gauge stays off by its pre-registered rule
(insufficient data; a larger run on a fresh seed would settle its PD column).
Decided by jdh8 on 2026-10-16: Modern negative doubles stay on as a
recorded trade-off ([competitive-book.md](competitive-book.md#p3b-modern-re-measured-under-the-disclosed-sd-leader-2026-10-16)),
and the natural overcall band stays `8:14`
([convention-tuning.md](convention-tuning.md)). Both verdict rows are
[archived](archive/research-method-review-closed.md#4-disclosure-in-sd-dump-rescoring-the-fix-and-the-settled-re-measures).

**Open.**

- Thirteen more verdicts cite the sd row but stand on DD or on an alerted
  call (`nt_overcall_systems_on`'s major cells the strongest).
- The SD-blend λ (`common::SD_BLEND_LAMBDA`) was fitted with
  `probe-sd-calibration` on the old reader; a refit may be owed.

## 5 Filter coverage and conditional comparisons

**P1, resolved for 1NT triggers 2026-10-16.** A filtered A/B estimates
`E[delta | divergence, filter]`, which equals `E[delta | divergence]` only
under a coverage argument; [measurement.md](measurement.md#the-decision-table)
now states the condition, and the strict-scan verdicts are scoped by
[the filter scope](archive/research-method-review-closed.md#filter-scope-the-rule-and-the-runner-classification).

The preflight exists (`7a43ddc5`): `examples/filter-coverage` takes two
unfiltered arms on one seed and splits their divergent boards by every
`bba-gen` scan, strict and `--wide-1nt`, reporting density, rejected share
and rejected examples; the scans have one definition, `common::filters`
(`bba-gen`'s accepted sets byte-identical). On 4,000 `--nt-fifths` boards the
strict `--filter-1nt` missed 6 of 48 divergent boards (all 5m422/6m322), the
wide candidate none. It counts `table_a` divergence only. `ab-lebensohl`,
`ab-one-nt-runout` and `ab-uvu` keep their own strict `is_1nt_opener`.

**Open.**

- For a trigger that is not a 1NT opening, use the union of candidate
  triggers or separate strata, and report the population weights and
  conditional effects together.
- [probe-weak-two-major](../examples/probe-weak-two-major/main.rs): its point
  estimate is internally consistent (mean over boards reaching the face times
  reach/draws is total swing/draws), so this is not a normalization bug. Its
  scaled CI treats the estimated density as fixed (rate uncertainty is a
  smaller follow-up), its stage-1 `accepts()` predicate has the bba-gen
  filters' coverage limit, and stopping at `--count` gives `boards/draws` a
  small stopping-rule bias.

## 7 Auction conditioning and the replay approximation

**P2 — observed range-sampling failure and an overstrong posterior claim.**
[sampler.rs](../src/bidding/sampler.rs), `within_ranges`, uniformly accepts
worlds inside the readings. This does not establish that the bidder would
produce the observed auction on those worlds.

The registered `1M - 2M -` seam experiment in
[exact-posterior.md](exact-posterior.md) measured range PD
`-0.0125 +/- 0.0601` against replay `+0.1388 +/- 0.0518` IMPs per seam decision.
Range left Pass for a try 511 times against replay's 108. Range's plain DD was
positive (+0.0737 ± 0.0526); only PD failed. Replay (`rule_accept`) is the
shipped default in [knobs.rs](../src/bidding/inference/knobs.rs), so the failure
describes the non-default range path. The counted-reading lane was parked. Its second-seed confirmation shares 91.2% of its deals with the first run (#6), so it is not independent.
Exact counting of the reading set does not correct the set's conditioning bias.

Replay is also an approximation: `MARGIN = 3.0` allows calls below the best
legal logit, and `made_plausibly` abstains on unauthored or tombstoned nodes.
Scaling finite authored priorities can leave production argmax unchanged but
change the replay acceptance set. Those priorities are not calibrated call
likelihoods; see [logit-calibration.md](ai-bidder/logit-calibration.md).

The wording that called replay "the self-play posterior" was corrected
2026-10-16 ([archived](archive/research-method-review-closed.md#7-auction-conditioning-the-documentation-pass)).

- **Owed:** the strict / full-policy / relaxed comparison, pre-registered in
  [logit-calibration.md](ai-bidder/logit-calibration.md) §4e (three seams,
  acceptance, PIT calibration, paired true-deal gain, decision at δ). It
  needs `probe-seam-lookahead` arms and a fix to `probe-replay-yield`'s bare
  `Context` first.
- Use calibrated likelihoods if adding probabilistic opponent conditioning;
  retain the actual-deal judge and static-remap controls.

## 8 Sampling fallbacks and changes to the model

**Instrumented 2026-10-16 (`fe2fdc41`).** `SampleStats` (requested, draws,
accepted, fallback draws, fallback) comes back from `*_with_stats` siblings
of `sample_layouts`, `sample_defender_remnants`, `ev_all`,
`single_dummy_leads`, `single_dummy_playout` and
`single_dummy_declarer_tricks`; the plain functions are wrappers with the
same worlds and RNG stream. `ab-dump-sd` prints a `Sampling` line per arm. On
the wide Lebensohl seed (t24 vs t23, none, 16 worlds) 0.92–0.99% of leader
worlds were unconstrained fallback, on 1.93% of the 881 fired boards, at
0.19% acceptance (about 530 draws per kept world). **Open:** the other SD
harnesses, the sd-playout line on real data, and the sensitivity comparison
(a `--fallback-policy as-is|common|drop` over the degraded boards).

The original finding:
[single_dummy.rs](../src/single_dummy.rs), `single_dummy_leads`, fills a short
range sample with unconstrained worlds. [ev.rs](../src/bidding/ev.rs),
`ev_all`, tops up replay with range-only worlds. Mid-play
`sample_defender_remnants` in [sampler.rs](../src/bidding/sampler.rs) can drop
soft auction readings while preserving hard play constraints.

Reading tightness or inconsistency can therefore change both information and
the mixture of fallback worlds. The current returned worlds do not expose the
mixture to the scorer. No particular A/B reversal has been attributed to this.
Starvation is measured only on the relabel path
([logit-calibration.md](ai-bidder/logit-calibration.md): 0.3–0.8% of decisions
under self-play, ~16.5% under the teacher walk, with short draws dropped rather
than topped up) and by `probe-replay-yield`; no A/B reports it.

## 9 SD objectives calibration and scenario bounds

**Partly resolved 2026-10-16 (`36d01481`).** `single_dummy_playout_with` and
`DeclarerObjective::Make(need)` add an opt-in safety-play declarer (most
sampled worlds that make, ties by mean tricks); the default stays
mean-tricks, pinned by a golden test. A same-layout, same-strain unit test (a
four-card notrump ending, ♠AQ finesse against the ace, worlds solved by DDS)
shows mean-tricks and 7NT finessing where 6NT takes the safety play. On
`probe-slam-battery --make` the objective does not fix the recorded detours:
one-way onside 80→70% and double-hook 60→85% at 16 worlds, 85→85% and
85→80% at 64. The detours track world count, not the objective; the residual
64-world leak is unexplained (strategy fusion is a guess). measurement.md now
calls plain/PD and the SD pair per-arm scenario checks, not bounds on a paired
delta, with the imperfect-doubler counterexample. **Open:** sensitivity to
objective, λ and shave on an actual divergent population; the aligned-cell λ
fit; the opening lead still maximizes mean tricks.

The original finding:
[single_dummy.rs](../src/single_dummy.rs), `declarer_choice`, maximizes mean
tricks; the playout interface has strain but no contract level. Mean tricks
and making a contract are different objectives. As an illustrative example,
12 certain tricks make six, while 80% of 13 and 20% of 10 average 12.4 tricks
but fail six 20% of the time. The
[slam battery](../examples/probe-slam-battery/main.rs) already records
mean-max detours losing DD-fair slam cases; it attributes them to a small world
sample, so whether a making-probability objective would fix them is untested.

The population-transfer section of
[measurement.md](measurement.md#known-biases) records a 28-percentage-point
6m alignment gap against Pavlicek. The same section reports that fitting the
better-aligned 6M/6NT cells gives lambda(6) = 0.510 rather than the shipped
0.664 in [common::SD_BLEND_LAMBDA](../examples/common/mod.rs). That figure is
quoted, not re-derived: the per-cell rates it needs are not published (the
unweighted mean of the two cell fits is 0.5055), it comes from a later run than
the shipped fit, that run's level-4 lambda also drifted (0.644 against 0.539),
and the doc already flags its holdout as owed. A fresh pons holdout
matching the same imported target tests simulation stability; it does not
establish matched human outcomes or the conditional treatment effect.

Plain/PD are scenario sensitivity checks, not guaranteed bounds on a paired
treatment effect. An illustrative same-board NV case has ON at 4♥ down one
and OFF at 3NT down one: both plain scores are -50 and both PD scores -100,
so both reported deltas are zero. An imperfect doubler who doubles only ON
produces a -50-point, -2-IMP delta, outside those endpoints. Bounds for
individual arms do not imply bounds from subtracting like endpoints.

## 10 Dataset identity and label transplantation

**Partly resolved 2026-10-16 (`82d4a60d`).** The trainer's validation cut now
moves forward to the next provable board start (an opening row, features
20/41, with a dealer hand unlike the previous opener's so `--replay` tables
stay together). The new tail is a subset of the old one; `--row-split`
reproduces old manifests, and weights sidecars record `val_split`.
`splice-labels.py` and `features-v8.md` now say that neither the positional
path (one-hot/DD/tags) nor the key (summary context, no cell) proves the
auction prefix. `splice-labels.py --validate` counts unmatched, ambiguous,
shared and identity-mismatched transplants, identity rebuilt from each chunk.
**New finding:** the key omits the cell, and the uniform shards' rotation went
from six cells to three, yet 84–85% of rows key-matched, so probably a third
or more of the uniform rows took a label from another cell (it matters only
where the M32 cut relabelled the row). **Measured 2026-10-18:** `--validate`
on the 20 M32 shards finds axis/enriched clean and every uniform mismatch a
cell. 918,237 uniform rows (40.9% of their transplants) took a label from a
Dutch cell, but only 58,602 of the 424,070 transplanted relabels (13.8%) did;
824 of those also came from another auction prefix. No ambiguous, shared, tag
or table mismatches occurred
([features-v8.md §2](ai-bidder/features-v8.md#2-labels-without-a-relabel--the-transplant)).
The splice now refuses those pairs (jdh8, 2026-10-18; `--any-identity`
reproduces the old splice byte for byte). **Open:** a stored per-row identity
sibling for future dumps.

The original finding:

[trainer/src/data.rs](../trainer/src/data.rs), `load_mixture`, divides each
stem at `rows - round(rows * val_frac)`. It stores no board boundary for the
split, while [dump-teacher](../examples/dump-teacher/main.rs) emits multiple
auction/cell rows per board. The board-disjoint guarantee is therefore too
strong. In a contiguous, non-overlapping stem this can leak the boundary board;
it is not evidence of widespread leakage or invalid fresh-deal A/Bs.
[logit-calibration.md](ai-bidder/logit-calibration.md) already records the
contiguous-tail split and treats held-out CE as diagnostic. (The
"board-disjoint" comments in `trainer/` are gone.)

[splice-labels.py](../scripts/splice-labels.py) keys by hand/context summaries,
vulnerability, one-hot and DD. The context in
[features.rs](../src/bidding/features.rs), `push_context`, does not encode
the full auction. Its positional fast path checks labels/DD/tags without
full row identity: it compares only the one-hot+DD block and the tags, with the
chunk windows and the total row count asserted. Thus
[features-v8.md](ai-bidder/features-v8.md)'s "the row's auction is identical
(that is what the key asserts)" is not established; its `--diff` evidence on one
chunk (labels moved 0) is a partial check. The keyed path drops duplicate old
keys. This does not prove the approximately 424k transplanted labels are wrong
or explain v8's loss.

- Store a stable deal ID, dealer, vulnerability, full auction prefix and
  cell per row in future dumps, so a splice can be checked against it.

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
policy until that proposed separation is explicitly adopted (jdh8,
2026-10-16: the no-reuse rule stands until then).

DD caching already exists in [bba-decompose](../examples/bba-decompose/main.rs)
and [anchor.sh](../scripts/anchor.sh). General
[common::solve_divergent](../examples/common/mod.rs) does not reuse that cache
and requests all strains. A cache or selected-position solve is a benchmark
candidate, not a demonstrated speedup.

**`ab-dump-sd` profiled 2026-10-19** (sd-lead, 16 worlds, the
`competitive-4cell-nsew` default/acctoff ns pair: 1,228,800 boards, 6,847
fired, 13,694 lead questions; `JOBS`/`DDSS_THREADS` 96 under a load of ~510;
temporary phase timers, report byte-identical to the archived one):

| Phase | Wall | Share |
| --- | ---: | ---: |
| Load both dumps | 9.3 s | 1.4% |
| Build readers + leader readings | 1.4 s | 0.2% |
| Rejection sampling (serial, one thread) | 257 s | 39.7% |
| DD solves (232,798 trick-one `Legal` boards, 96 threads) | 379 s | 58.6% |
| Total | 650 s | |

DD is the larger phase, but sampling is ~40% of wall clock and runs on one
thread while the solver's pool waits. The two phases alternate per 4,096-question
chunk inside `single_dummy_leads_with_stats`. Most of the sampling time goes to
starved questions: a question that cannot fill its 16 worlds draws the whole
16 × 4,096 = 65,536-draw budget before the fallback tier fills the rest. The
2,839 such questions (both arms) drew 186M of the 266M draws (≈70%) and
yielded ≈10.4k of the 219k sampled worlds (≈4.8%). One draw costs ≈1.1 µs:
dealing 39 cards plus `within_ranges`.

Levers, ranked by how much each preserves the KR1 proof:

1. **Overlap sampling with solving**, byte-identical. A sampler thread owns
   the RNG and draws in the same order, while the main thread (where the
   solver must stay) solves sub-batches as they arrive. Wall time becomes
   about `max(sample, solve)`, ≈ −40% at this load. Every caller of
   `single_dummy_leads` benefits: ab-dump-sd and the twelve other SD harnesses
   and probes. Cost: a `Send` bound on the RNG.
   **Shipped 2026-10-19.** On the same pair, run back to back with
   `JOBS`/`DDSS_THREADS` 96, both reports are byte-identical to the archived
   one. Wall time went from 690 s (HEAD, load ~690) to 460 s (−33%, load
   ~600). Load was lower for the second run, so the size of the cut is
   approximate; the identity is exact.
2. **Parallel sampling with per-question seeds** drawn serially from the RNG.
   This is faster still on a free box, where DD speeds up and serial
   sampling would dominate. It changes the RNG stream, so every SD number
   moves within noise and archived reports stop reproducing.
   **Parked 2026-10-19 on `park/sd-parallel-sampling`** (a sampler pool as
   wide as the DD pool). At the 96-thread cap it is 17% slower: 554 s twice
   against lever 1's 481 s and 466 s, run in ABBA order. Lever 1 is already
   DD-bound there, and the extra sampler threads take cores from DD. The
   stream change is noise: plain SD +0.0018 against the archived +0.0016,
   SD-PD +0.0000 against −0.0002 (each ±0.0007), same verdicts. Flip plan:
   re-measure on a free box (`JOBS`/`DDSS_THREADS` 640).
3. **Early exit on starved questions** (a dry limit like the replay
   sampler's). This cuts ≈70% of the draws but changes the stream and,
   slightly, the mix of fallback worlds. It is a scorer change and needs its
   own argument.
   **Parked 2026-10-19 on `park/sd-infeasible-skip`** in its lossless form.
   A replay of every lead question's accept positions on the same pair
   priced the rules. Of the questions that run out of draws, 1,728 still
   accept a few worlds, so a dry limit of 4,096 cuts 78% of draws but loses
   13.7% of the accepted worlds. Stopping after 16,384 draws with no accept
   cuts 30% and loses 0.84%. The parked form skips only readings
   `Inferences::may_complete` proves empty (per-seat length and HCP bounds),
   which loses nothing: it caught 448 of the 1,101 questions that never
   accept and cut 11% of draws. Timed against lever 1: 477 s against 474 s,
   no gain, because lever 1 is DD-bound at the cap. Same flip plan as
   lever 2, ideally stacked on it.

The `solve_divergent` cache above prices `ab-dump-diff`, not this tool. Here
DD is not marginal, but its 17 solves per question are distinct worlds, so a
cache has nothing to reuse.

- Define development reuse, data separation and final confirmation policy;
  record deal identities so overlap can be handled correctly.
- Benchmark cache reuse and reached-position solves against full tables.
- Count contract/declarer divergence for DD and auction/reading divergence
  for information-sensitive SD work; zero contract divergence does not mean
  zero potential disclosure effect.

## Convention comparisons and global retraining

The [1NT response ledger](one-notrump-constructive.md) and
[7ae1ecb](https://github.com/jdh8/pons/commit/7ae1ecb) support the implemented
`1NT - 3♦` five-five-major package against its then-current routing. The
200k-board, opponents-silenced A/B also authored the package's continuations
and tuned its points floor (`8..` against `7..` and `9..`) on the same
comparison. It does not
establish the best possible meaning for that bid slot.

- Compare complete candidate structures, including the hands displaced by
  each meaning, using a common raw-hand filter covering their trigger union.
- Rank by population-weighted total gain; a higher per-divergent mean at
  equal total gain does not establish a better global threshold.
- Freeze structure and thresholds before independent confirmation, and
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
| Flat-eight invite | [PR #4](https://github.com/jdh8/pons/pull/4) and [1NT ledger](one-notrump-constructive.md): DD's pass edge softened under SD-PD and flipped toward inviting under plain SD | Preserve the scorer and population with the claim |
| SD current-trick sequence collapse | [f6a7a7ae](https://github.com/jdh8/pons/commit/f6a7a7ae) repaired the playout; the refit is in [measurement.md](measurement.md#known-biases) | Pre-fix playout results need their corruption warning |
| v5 factory siblings | [bde861ef](https://github.com/jdh8/pons/commit/bde861ef) synchronized the paths; [card-manifold.md](ai-bidder/card-manifold.md) records parity | Check system identity across declared and undeclared paths |
| Multi-double corpus reading | [bank ledger](pdd-bank-ledger.md) records the 2026-08-27 generator repair | Older declared-Multi rows can still carry the stale reading |
| Two PD definitions | [42bd3282](https://github.com/jdh8/pons/commit/42bd3282) unified the scorer on 2026-09-27 | Label pre/post-scorer series; do not attribute that break to bidding |

These are historical defects or revised interpretations, not a list of
unfixed runtime bugs. Preserve the original evidence and the repair together
when reopening a lane.
