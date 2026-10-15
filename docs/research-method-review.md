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

| Priority | Issue | Completion landed in |
| --- | --- | --- |
| P1 | [#6 Deal overlap between fresh-seed runs](https://github.com/jdh8/pons/issues/6) | `measurement.md` and affected replication ledgers |
| P1 | [#7 Non-inferiority margin instead of wash as no loss](https://github.com/jdh8/pons/issues/7) | `measurement.md`, the ship rules and affected verdicts — margin and verdicts landed 2026-10-14; [inventory](#issue-7-non-inferiority-inventory) decided and re-measured; both losses inside δ repaired and proven non-inferior 2026-10-15 (XYZ's `2♥`, the reverse's HCP floor); reporter coverage and the density / small-sample guards landed 2026-10-15 (the two modeled SAT diagnostics labelled exempt, and the resume guard fingerprinting the binary an arm runs, 2026-10-16); the four status-unknown rows with an off arm re-measured 2026-10-15 (`competitive_4333` and `takeout_support` proven, `penalty_no_pull` inert and removed, the diamond-transfer contest a loss inside δ, its repair `diamond_transfer_responder_captain` shipped default-on 2026-10-15 on a three-arm A/B); `PointCount` proven win \| win 2026-10-15; the RKCB queen ask superseded (no off arm) |
| P1 | [#8 Direct A/B versus reference-relative IMP gain](https://github.com/jdh8/pons/issues/8) | scorer reports, `measurement.md` and anchor ledgers |
| P2 | [#9 Vulnerability cells on one seed are one deal set](https://github.com/jdh8/pons/issues/9) | `measurement.md` checklist 5 (2026-10-14) and the [cell-count audit](#issue-9-cell-counts-cited-as-replication) (2026-10-16) |

All four are closed. The entries below preserve the remaining review
findings. Delete an item once it is done, recording the measured result or the
explicit decision in the doc that owns the conclusion; git keeps the history.
Keep previously published outputs identifiable when an instrument or protocol
changes.

## Issue #7: non-inferiority inventory

Defaults that shipped on a plain-DD wash before the margin, re-read on
2026-10-14 against δ = 0.002 IMPs/board
([measurement.md](measurement.md#the-decision-table)). The worst
vulnerability cell decides. Two exact shortcuts: an enriched row whose lower
bound clears −δ per filtered board clears it at any density (the margin only
widens, `δ / density`); and a change that moves fewer than δ / 24 of boards
(0.0083%) cannot lose δ even if every moved board loses the 24-IMP maximum.
Figures are the ledgers' own; this pass re-measured nothing and reverted
nothing.

**Inconclusive: a plain-DD loss beyond δ is not ruled out.** Each needs
jdh8's decision: re-measure on fresh deals with a pre-declared board count,
or record the trade-off below. The proposed default in the last column keeps
the default as is.

| Change | Plain DD none / both (lower bound) | PD | Fired | Source | Proposed |
| --- | --- | --- | --- | --- | --- |
| `direct_weak_jump_overcall` (majors) | +0.0002 ±0.0028 (**−0.0026**) / +0.0031 ±0.0035 (−0.0004) | +0.0026 ±0.0032 / +0.0051 ±0.0040 — straddles too, so row "wash \| PD win" never applied | 1.16% / 1.11% | [ben-gap-campaign.md](ben-gap-campaign.md) | Trade-off: a preempt's obstruction is DD-invisible, so DD understates it |
| Direct-seat table as exact per-overcall rows (knobless) | +0.0005 ±0.0029 (−0.0024) / −0.0009 ±0.0037 (**−0.0046**) | +0.0035 ±0.0035 / +0.0008 ±0.0043 | not quoted | CHANGELOG 2026-08-06 | Trade-off: KR2 structural conversion |
| Phase 3 substitute-don't-intersect (knobless) | seed 1 −0.0008 ±0.0023 / −0.0013 ±0.0027 (**−0.0040**); seed 2 +0.0013 ±0.0022 / −0.0002 ±0.0027 (−0.0029) | both seeds straddle | 0.57–0.68% | [authored-reading-handoff.md](authored-reading-handoff.md) | Trade-off: KR2 reading architecture; the measured code no longer exists to re-run |
| Reading-drift batch, round 3 (seven repairs + rail) | +0.0007 ±0.0050 (**−0.0043**) / +0.0049 ±0.0058 (−0.0009) | −0.0010 ±0.0058 / +0.0067 ±0.0067 | 2.91% / 2.58% | [reading-drift-handoff.md](reading-drift-handoff.md) | Trade-off: correctness repairs |
| Raise readers off the point-count axis (fix) | +0.0034 ±0.0088 (**−0.0054**), 6,400 boards | +0.0033 ±0.0088 | 27 / 12,800 tables | CHANGELOG 2026-07-30 | Trade-off: correctness; the sample was too small to say anything |

**Non-inferior under δ** (no action): `DOUBLE_PUSH_CEILING`, O3
`direct_minor_weak_jump_overcall`, `response.major_raise_slam_try`, the
`limit_raise_acceptance` and `fourth_suit_forcing` fresh re-runs, `reading.completion_alerts`, the
fresh-sweep reading repairs (lower bounds −0.0016 / −0.0019, the closest),
Part A of the cramped doubled answer, the 5-2 trump repair, the keycard
trump-proof rail, the cue-blocked face + notrump dichotomy, the `lane_suits`
polish, `reading.bid_exclusion`, `strength_ceilings` (legacy arm; that
code state no longer exists), `modern_double_both_majors`,
`doubled_notrump_max_length`, `strong_two_loose_positive` (alone, −0.0001),
`nt_structure_side_gate`, `respect_keycard_signoff`,
`minor_jump_notrump_slam_try`, and the one-level opening HCP floor. From
per-seed `diff.*.txt` files in `pons-ab-results` (the CHANGELOG gives no
per-cell ±): `unbalanced_1nt_rebid`, `two_over_one_reverse_extras`,
`two_over_one_side_suit_first` and `two_over_one_minor_before_spades`.
Shipped as wins with a plain lower bound of exactly 0, so non-inferior, not
wins; each still ships on its PD win: `drury_splinters`,
`strong_two_positive_notrump`. Enriched, cleared per filtered board: the Landy
counter and `their_landy_reading`, the N1j ladder and weak-2♦ cap, N1q `strx`,
the N4 v7 Multi counter, `their_multi_reading`, `multi_weak_escape`,
Kokish–Kraft's none cell, N3's `at_least_as_long`, `transfer_longer_major`,
and `weak_two_longest_first` (0.029% density gives a margin of 6.9 per
accepted deal, against a lower bound of −0.011). The `(1NT) 2♦` mirror M1+M2
clears too: its seed-1 none-vul lower bound is −0.0023 per filtered board, and
`--filter-1nt` accepts about 19% of deals (Monte Carlo, 200k deals, on
`is_1nt_opener`), so it is −0.0004 per board, inside the margin below which
an 87% acceptance rate would have been needed. Below the δ / 24 bound: `upgrade_closure`, the cue / table-alert /
pass readings, DOPI/ROPI/DEPO, `set_advance_2nt_continuation`,
`set_floor_rkcb`, `weak_new_suit_rebid`, `keycard_answer_gates` (0
divergent), the walk's shape floor, the Michaels complements, the
`may_have_made` reading fix, and `defense_2c_landy_competition` (17 / 11 boards
per 230.4k filtered).

**Status unknown: no plain-DD CI was recorded.** Searched on 2026-10-14
for surviving diffs and dumps in `pons-ab-results`, whose oldest entry is
2026-07-06. Six rows were answered by diff files already on disk, pooled the
way the ledgers pool them; all six are non-inferior and move out of this list:

| Change | Plain DD none / both (lower bound) | PD | Results dir |
| --- | --- | --- | --- |
| `set_length_soundness` | +0.0010 ±0.0010 (0.0000) / +0.0006 ±0.0014 (−0.0008) | +0.0022 ±0.0011 / +0.0023 ±0.0016 | `reading-knobs/2026-07-17/scores` (BEN guard: −0.0001 ±0.0026 in the both cell straddles −δ) |
| `set_major_support_double` | −0.0004 ±0.0007 (−0.0011) / +0.0004 ±0.0009 (−0.0005) | +0.0009 ±0.0009 / +0.0016 ±0.0011 | `competitive-book/p3c-support-x-*` |
| `nt_overcall_systems_on` (major split) | per split board +0.0013 ±0.0038 (−0.0025) / +0.0044 ±0.0052 (−0.0008); at 30% density −0.0008 per board | −0.0003 ±0.0047 / +0.0018 ±0.0062 | `nt-overcall-systems-on` |
| `passed_hand_overcall` | +0.0008 ±0.0009 / +0.0011 ±0.0015 (−0.0004), 76,800 boards | +0.0011 ±0.0017 (both) | `a5/passed-hand-overcall-*` |
| `forcing_ceiling_read` | +0.0001 ±0.0001 both cells, three seeds pooled (614k boards) | +0.0003 ±0.0001 / ±0.0002 | `p0b-forcing-ceiling{,-s2,-s3}` |
| Unauthored-`2NT` advance blanket | −0.0002 ±0.0003 (−0.0005) / −0.0002 ±0.0004 (−0.0006), three seeds | −0.0002 ±0.0003 / −0.0003 ±0.0004 | `defenders-nt-blanket-{2,3,4}`; listed here by mistake, its handoff doc already had the CIs |

`penalize_escape_stack` is below the δ / 24 bound: 498 divergent of 5M
filtered boards, at the ~19% `--filter-1nt` density about 0.0019% of drawn
boards.

Still unknown; no dumps survive, so only a fresh A/B can settle these (all five resolved 2026-10-15, below):
`PointScale::PointCount` (self-play, "plain wash" with no numbers), the
linearised `upgrade` (a retrain A/B; no plain number at all),
`set_competitive_4333` (no plain CI, and the both-vul cell was never
measured), `penalty_no_pull` (none cell; no PD either) and
`set_takeout_support` (none cell). Two are close but not proven. The RKCB
queen ask was measured on 10M boards per cell in self-play; its PD intervals
were about ±0.0002, so a plain lower bound near −δ is implausible.
`set_competition_over_diamond_transfer` fired on about 400 of 1M filtered
boards, about 0.0076% of drawn boards against the 0.0083% bound, but the
count is rounded and the density is estimated.

**Status-unknown re-measures (run 2026-10-15, sha `d3ef9787`-dirty with
`tree.patch`, 96 × 12,800 = 1,228,800 boards per arm and cell vs BBA,
`ab-results/issue7-unknown/<experiment>/`, each with its own `seed`).** Each
diff is shipped − the alternative the row's own A/B shipped over; cells are
none / both. Deals disjoint from every earlier #7 run (above).

| Change vs alternative | Plain DD | PD | SD plain / SD-PD | Fired | Verdict |
| --- | --- | --- | --- | --- | --- |
| `competitive_4333` `suppress` vs `allow` | +9 / −1 IMPs in all | +29 / +21 IMPs | −33 / −6, −7 / +10 IMPs | 9 / 7 boards | **non-inferior**: below the δ / 24 bound |
| `penalty_no_pull` vs `--ns-allow-pull` | 0 | 0 | 0 | **0 boards** | **inert**: the latch's pull rules live in `instinct()` and never moved a call under the shipped floor. Knob removed 2026-10-15 (jdh8), the no-pull gate hard-wired; `smoke-default` byte-identical (20,000 auctions, seed 1) |
| `takeout_support` strict vs off | +0.0003 ±0.0011 / +0.0004 ±0.0014 | **+0.0035 ±0.0013 / +0.0061 ±0.0017** | −0.0006 ±0.0012 / **−0.0020 ±0.0015** (both); SD-PD +0.0011 ±0.0013 / **+0.0018 ±0.0016** | 1.3% | **non-inferior \| win**: proven. Plain SD loses both-vul beyond δ while SD-PD wins: the gate trades undoubled partscores for fewer doubled minuses |
| `competition_over_diamond_transfer` vs off | **−0.0001 ±0.0001 / −0.0002 ±0.0001** | **−0.0005 ±0.0002 / −0.0005 ±0.0002** | −0.0001 / **−0.0002 ±0.0001**; SD-PD **−0.0004 / −0.0005** | 94 / 63 boards | **loss \| loss** inside δ, −1.3 to −9.3 IMPs per fired board |

The diamond-transfer loss is opener's reply to an overcall above `3♣`
(`diamond_overcalled_high`), not the doubled branch. Every one of the
five worst PD boards per cell is `1NT - 2NT (3♥/3♠)` and then either opener's `3NT` (17+ HCP
and a stopper, "to play" opposite a transfer that may hold nothing) or
opener's penalty `X` followed by responder's `3NT`. That answer is
unauthored, so the floor pulls a weak diamond hand to notrump. Off, we pass
and they play `3M` undoubled. Repairs to measure, each reversible: drop the
`3NT` rung; author responder's answer to the `X` (pass, `4♦` with six);
or switch only the high-overcall rows off. The default stays on until one
is measured: jdh8's call.

**Decided 2026-10-15 (jdh8, grill rounds 1–2).** Three dispositions:

- *Diamond transfer*: repair, not a trade-off. Built behind the sub-knob
  `competition.diamond_transfer_responder_captain` (`bba-gen
  --ns-diamond-transfer-captain` then, `--no-ns-diamond-transfer-captain`
  now): opener's `3NT` rung is gone over
  `1NT - 2NT (3♦/3♥/3♠)` (opener doubles with four of their suit or passes,
  responder captain) and responder's answer to the penalty `X` is authored
  (pass; `4♦` with six diamonds and at most one card in their suit).
  Responder after opener's pass stays floor-owned, as today. Default
  byte-identical (`smoke-default`, 20,000 auctions, seed 1). **A/B owed**,
  three arms on one seed at the declared 1,228,800 boards/arm/vul: shipped,
  captain, package off, diffs captain − shipped and captain − off, plain DD +
  PD + SD; the knob flips on if captain is non-inferior | win over shipped
  and not behind off, and the package turns off if off beats both.

  *First run* (`ab-results/issue7-unknown/diamond-captain/`, `df306e32`-dirty,
  SEED_BASE 1792065277114931076): captain − shipped **win \| win** in both
  cells (plain +0.0001 / +0.0002, PD +0.0003 / +0.0002, ±0.0001), but
  captain − off plain non-inferior, PD **loss** inside δ (−146 / −166 IMPs,
  −0.0001 ±0.0001). Traced: after the authored pull `1NT - 2NT (3♥/3♠) X -
  4♦ -`, opener's call was the floor's, and it was `4♠` on **all 44** such
  auctions over both cells; responder passed it or ran to `5♦`. Repaired:
  opener passes the pull (`diamond_pull_accepted`); `smoke-default`
  byte-identical (20,000 auctions, seed 1).

  *Re-run* (`ab-results/issue7-unknown-r2/diamond-captain/`, fresh SEED_BASE
  1792067229587040987, same size; deals disjoint from the first run):

  | Diff (none / both) | Plain DD | PD | SD plain / SD-PD | Fired | Verdict |
  | --- | --- | --- | --- | --- | --- |
  | captain − shipped | +0.0001 / +0.0001 | **+0.0003 / +0.0003** | +23 / +32 IMPs, non-inferior; SD-PD **+0.0002 / +0.0002** | 65 / 43 boards | **win \| win** (+166 / +168 plain, +404 / +326 PD IMPs) |
  | captain − off | +0.0001 / +0.0000 | −0.0000 / −0.0001 | +87 / −23 IMPs; SD-PD −30 / −87 IMPs, non-inferior | 67 / 44 boards | **non-inferior \| non-inferior** (+102 / +51 plain, −45 / −72 PD IMPs) |

  All intervals ±0.0001. Opener now passes the pull on every board (26 / 21
  auctions). Captain's worst boards against off are the `(3♣)` lane, which
  the knob does not touch. **Shipped default-on 2026-10-15**; the package
  stays on, since off beats neither. The worst cell's lower bound per board is
  above −δ on both scorers for both diffs, so this is a proof, not a
  trade-off.
- *`PointCount` and the linearised `upgrade`*: one fresh A/B vs
  `rule-of-n-floored`, declared 5,120,000 boards/arm/vul (a 2,000-board
  pilot moved our auction on ~14.5% of boards at σ ≈ 2.3 IMPs/board), plain
  DD + PD, no SD. Run 2026-10-15 at `b9fd92dd`-dirty (the runner's size
  line only, `tree.patch`), `ab-results/issue7-unknown/point-scale/`,
  SEED_BASE 1792050558052897282, 96 × 53,336 boards per arm and cell.
  Shipped − `rule-of-n-floored`:

  | Cell | Plain DD | PD | Fired |
  | --- | --- | --- | --- |
  | none | +0.0509 ±0.0020 | +0.0495 ±0.0022 | 15.1% |
  | both | +0.0474 ±0.0025 | +0.0441 ±0.0027 | 14.3% |

  **Win \| win** in both cells, +0.31 to +0.34 IMPs per fired board: the
  `PointCount` default and its linearised `upgrade` are proven, and no
  trade-off is recorded. Deals drawn from a fresh SEED_BASE, disjoint from
  every other #7 run (below).
- *RKCB queen ask*: **superseded**. The ask is structural now (the knob is
  deleted and BBA always relays), so no off arm exists to measure; the
  surviving evidence is 10M self-play boards per cell with PD intervals of
  about ±0.0002, under which a plain lower bound at −δ is implausible. No
  statistical proof is claimed.

Three more sat outside their own row and were re-measured (below).
`longer_major_response` shipped on the naturalness tiebreak with a PD *loss*
(−0.12 / −0.22 per divergent), and that row asks for a push on both
scorers. `forcing_nt_two_suiter`'s SD-PD none cell is CI-clear negative
(−0.0011 ±0.0005). `forcing_nt_jump_shifts` was called "win | win" on
totals, while its SD-PD washes.

**Decided 2026-10-14 (jdh8).** Re-measure K=8, `competitive_push_natural_only`
and the three flagged status-unknown rows (`longer_major_response`,
`forcing_nt_two_suiter`, `forcing_nt_jump_shifts`) on fresh deals vs BBA, at
sizes declared before the run in `scripts/ab-non-inferiority.sh` (K=8
1,638,400 boards/arm/vul, the rest 1,228,800); the results land in
`ab-results/non-inferiority/<experiment>/`. Re-score whatever dumps survive for
the other status-unknown rows (done 2026-10-14, above); a row with no dump
stays unknown. Record the
five rows below as trade-offs.

**Fresh re-measures (run 2026-10-14, sha `0a69df03`-dirty with
`tree.patch` saved per experiment).** Each diff is shipped − alternative, so
positive favours the shipped default. Cells are none / both; ± is the 95%
half-width; the single-dummy columns (16 worlds) give the worse cell.

| Change | Plain DD | PD | SD plain / SD-PD (worst) | Fired | Verdict |
| --- | --- | --- | --- | --- | --- |
| K=8 floor vs K=4 | +0.0020 ±0.0019 / +0.0046 ±0.0023 | +0.0058 ±0.0021 / +0.0076 ±0.0025 | not run | 4.3% / 3.7% | **win \| win**: proven, moves out of Inconclusive |
| `competitive_push_natural_only` vs off | −0.0003 ±0.0009 / −0.0003 ±0.0011 (−0.0014) | +0.0014 ±0.0009 / +0.0027 ±0.0011 | −0.0006 ±0.0010 / +0.0003 ±0.0010 | 0.31% / 0.37% | **non-inferior \| win**: shippable default-on, now proven |
| `forcing_nt_jump_shifts` vs Meckstroth | −0.0001 ±0.0004 / −0.0003 ±0.0005 (−0.0008) | +0.0000 ±0.0005 / −0.0001 ±0.0006 | −0.0004 ±0.0005 / −0.0003 ±0.0005 | 0.10% / 0.11% | **non-inferior \| non-inferior**: the earlier "win" does not replicate; it stays on the naturalness tiebreak (natural jump shifts over a convention) |
| `longer_major_response` vs off | +0.0000 ±0.0006 / −0.0005 ±0.0008 (−0.0013) | −0.0003 ±0.0007 / **−0.0010 ±0.0009** | −0.0005 ±0.0008 / **−0.0011 ±0.0009** (both) | 0.41% | **non-inferior \| loss**: no table row; both-vul PD and SD-PD are CI-clear negative, inside δ |
| `forcing_nt_two_suiter` vs off | **−0.0002 ±0.0002 / −0.0003 ±0.0003** | **−0.0007 ±0.0002 / −0.0009 ±0.0003** | +0.0000 ±0.0002 / **−0.0003 ±0.0002** (none) | 0.04% / 0.05% | **loss \| loss**: every DD cell, −0.5 to −1.9 IMPs per fired board; plain SD alone is non-inferior |

**`forcing_nt_two_suiter` trace (2026-10-15).** All 1,117 divergent boards
of both cells, bucketed by opener's shape and responder's answer to
`1♥ - 1NT - 2♠` (plain / PD IMPs; total −641 / −1,889):

| Bucket | Boards | Plain | PD |
| --- | --- | --- | --- |
| Responder `4♠` with three spades and a doubleton heart | 359 | −733 | −1,142 |
| Responder `3♥` preference with a doubleton (off stops in `2♥`) | 161 | −331 | −494 |
| Opener with 6+ hearts (any answer) | 150 | −451 | −669 |
| Responder `4♥` with three hearts | 58 | +285 | +251 |
| Responder `3NT` | 314 | +274 | −74 |
| Opener 16–17 HCP | 406 | +127 | −164 |
| Opener 13–14 HCP (15–17 *points* by length) | 330 | −539 | −985 |

The leak is the answer table, not the invitation. `4♠` (weight 1.3, exactly
three spades) outranks the 5-2 heart fit, which `4♥` (3+ hearts) never takes,
and off reaches `4♥` through `2♥ - 3♥ - 4♥` or stops in `2♥`. Opener's
6+ heart hands reverse too and land in the 4-3 or in `3NT` (−140 plain on the
`3NT` with a doubleton). The reverse also forces a minimum responder to
`3♥`. The 16–17 HCP hands and the three-heart `4♥` already gain. Repairs to
measure, each reversible: let `4♥` take 2+ hearts above `4♠`; cap the
reverse at five hearts; raise the floor to 15 HCP. The default stays on
until one is measured: jdh8's call. (Settled 2026-10-15: the five-heart
cap and the 15 HCP floor both won and shipped; the reverse is now
non-inferior vs off, below.)

**`longer_major_response`** has no row in the decision table: plain is
non-inferior, PD and SD-PD lose in the both cell (lower bounds −0.0019 and
−0.0020). It shipped on naturalness with this same PD sign. Proposed
reversible default: keep it on as a recorded trade-off on naturalness, PD
loss −0.0010 accepted, until the repair below is measured. (Settled
2026-10-15: non-inferior on every scorer on top of the repair, below.)

Its trace (2026-10-15, all 10,155 divergent boards, −541 plain / −1,637 PD)
puts the loss in the continuation, not the response. On 3,039 boards the
hearts-first arm plays a heart fit that the `1♠` response never finds
(−5,016 plain / −4,842 PD); the other 7,116 gain (+4,475 / +3,205). The
leading sequences are `1♦ - 1♠ - 1NT - 2♣` (437 boards, −1,153) and
`1♦ - 1♠ - 1NT - 2♠` (336, −943). In XYZ the weak `2♠` (1.1) outranks the
5♠4♥ `2♥` (1.05), and the invitational `2♠` (1.2) outranks the invitational
`2♥` (1.1), so a 5-4 or 5-5 responder never shows hearts. Opener's answer to
the weak `2♥` was not authored either. The repair is
`rebid.xyz_second_major_first` (then default off, shipped default-on below): `2♥` with four hearts and at
most five spades outranks `2♠` in both rounds, and opener corrects the weak
`2♥` to `2♠` only with three spades and a doubleton heart. A smaller leak
stays with the floor: responder passes `1m - 1♠ - 2m` where hearts-first
reached a heart contract (173 boards, −870 plain).

**Repair A/Bs (launched 2026-10-14 17:14Z, `ab-results/issue7-repairs/`).**
`reverse-heart-game` (`rebid.forcing_nt_reverse_heart_game`) and
`xyz-second-major-first`, each repair − shipped vs BBA, 1,228,800 boards per
arm and cell, declared before the run.

| Repair | Plain DD none / both | PD none / both | SD plain / SD-PD both | Fired | Verdict |
| --- | --- | --- | --- | --- | --- |
| `xyz_second_major_first` | +0.0010 ±0.0002 / +0.0014 ±0.0002 | +0.0014 ±0.0002 / +0.0018 ±0.0003 | +0.0011 ±0.0002 / +0.0015 ±0.0003 | 0.06% / 0.07% | **win \| win** on every cell and scorer, +1.6 to +2.6 IMPs per fired board: **shipped default-on** |
| `forcing_nt_reverse_heart_game` | −0.0001 ±0.0001 / −0.0001 ±0.0002 | −0.0001 ±0.0002 / −0.0002 ±0.0002 | **−0.0002 ±0.0002 / −0.0003 ±0.0003** | 0.02% / 0.03% | **loss** (SD both-vul); stays opt-in |

The XYZ gain is natural: the invitational `2♥` reaches `4♥` where `2♠`
reached `3NT` (405 both-vul boards, +739 plain), and the weak `2♥` partscore
beats `2♠` (262, +668); its worst boards are opener passing the weak `2♥`
with a maximum. The heart-game repair failed because the trace priced the
`4♠` bucket against the auctions without the reverse, not against `4♥`; the
two-suiter loss stays open, its next levers the 6+ heart reverse (−451) and
the 13–14 HCP hands (−539). `longer_major_response` should be re-measured on
top of the XYZ repair before its trade-off is recorded.

**`longer_major_response` on top of the XYZ repair (run 2026-10-15, sha
`2dbbc57d`, `SEED_BASE=1792007254549759702`, 96 × 12,800 = 1,228,800
boards per arm and cell, `ab-results/issue7-remeasure/longer-major-response/`).**
Shipped − off, as before:

| Cell | Plain DD | PD | SD plain | SD-PD | Fired |
| --- | --- | --- | --- | --- | --- |
| none | +0.0009 ±0.0006 (**win**) | +0.0005 ±0.0007 | +0.0005 ±0.0007 | +0.0001 ±0.0007 | 0.41% |
| both | +0.0003 ±0.0008 | +0.0000 ±0.0009 | −0.0003 ±0.0009 | −0.0006 ±0.0009 | 0.40% |

**Non-inferior \| non-inferior**, SD-PD non-inferior (worst lower bound
−0.0015): the both-vul PD and SD-PD losses (−0.0010, −0.0011) were the XYZ
leak and are gone with `xyz_second_major_first` on. The default is proven
and no trade-off is recorded.

**Reverse levers (run 2026-10-15, sha `2dbbc57d`-dirty with `tree.patch`,
`SEED_BASE=1792009682846689673`, 1,228,800 boards per arm and cell,
`ab-results/issue7-remeasure/reverse-levers/`).** The two remaining
two-suiter buckets, as default-off knobs on opener's `1♥ - 1NT - 2♠`:
`rebid.forcing_nt_reverse_five_hearts` (exactly five hearts, so the 6-4
rebids `2♥`) and `rebid.forcing_nt_reverse_hcp_floor` (15+ HCP as well as
15–17 points). Five arms on one seed: the old reverse, no reverse (`off`),
each lever and both. A 6-4 with 15+ HCP counts 16+ points and takes the
`3♥` jump, so the floor already catches almost every 6-4 reverse; `both`
and `floor` differ on 9 boards (+8 / +22 plain, +14 / +38 PD, both − floor).

| Diff | Plain DD none / both | PD none / both | SD-PD none / both | Fired |
| --- | --- | --- | --- | --- |
| old − off (replication) | **−0.0003 ±0.0002** / −0.0003 ±0.0003 | **−0.0007 ±0.0003 / −0.0009 ±0.0003** | **−0.0003 ±0.0002** / −0.0002 ±0.0003 | 0.04% |
| five − old | +0.0002 ±0.0001 / +0.0003 ±0.0001 | +0.0004 ±0.0001 / +0.0005 ±0.0002 | +0.0003 ±0.0001 / +0.0003 ±0.0001 | 0.01% |
| floor − old | +0.0003 ±0.0001 / +0.0004 ±0.0002 | +0.0006 ±0.0002 / +0.0007 ±0.0002 | +0.0003 ±0.0002 / +0.0004 ±0.0002 | 0.02% |
| both − old | +0.0003 ±0.0001 / +0.0004 ±0.0002 | +0.0006 ±0.0002 / +0.0007 ±0.0002 | +0.0003 ±0.0002 / +0.0004 ±0.0002 | 0.02% |
| five − off | −0.0000 ±0.0002 / +0.0001 ±0.0003 | **−0.0004 ±0.0002 / −0.0004 ±0.0003** | +0.0000 ±0.0002 / +0.0002 ±0.0003 | 0.04% |
| floor − off | +0.0000 ±0.0002 / +0.0001 ±0.0002 | −0.0002 ±0.0002 / −0.0002 ±0.0003 | +0.0001 ±0.0002 / +0.0003 ±0.0003 | 0.03% |
| both − off | +0.0000 ±0.0002 / +0.0002 ±0.0002 | −0.0002 ±0.0002 / −0.0001 ±0.0003 | +0.0001 ±0.0002 / +0.0003 ±0.0003 | 0.03% |

The old reverse replicates its loss. Each lever **wins** against it on every
cell and scorer (both: +2.2 to +4.4 IMPs per fired board), so both shipped
default-on. With them the reverse is **non-inferior \| non-inferior** vs no
reverse (worst lower bounds −0.0004 PD none, −0.0002 plain none) and wins
SD-PD both-vul, so `forcing_nt_two_suiter` stays on, now proven. The five-heart
cap alone still loses PD to no reverse; the HCP floor carries the repair.

**Deal independence (#6 gate, checked 2026-10-15).** `deal-overlap` over the
`shipped-none` arm of every #7 run — the five first-round re-measures
(`ab-results/non-inferiority/`, run on dl02 and copied here), the two repairs
(`issue7-repairs/`), `longer-major-response` and `reverse-levers`
(`issue7-remeasure/`) and the first status-unknown run
(`issue7-unknown/competitive-4333`): **0 shared deals in all 45 pairs**.
Positive control: two arms of one experiment share all 1,228,800. So the
reverse-levers `old − off` replication of the first-round two-suiter loss and the
`longer_major_response` re-measure stand on deals disjoint from the runs they
confirm or supersede. Re-checked after the status-unknown rows: adding
`takeout-support`, `diamond-transfer`, `point-scale` and both diamond-captain
runs gives 15 `shipped-none` arms, **0 shared deals in all 105 pairs**; the
positive control (the r2 run's `shipped` and `captain` arms) shares all
1,228,800.

**Recorded trade-offs.** Each default is kept without a proof; the loss it
accepts is its worst plain-DD lower bound per board.

| Change | Basis | Loss accepted |
| --- | --- | --- |
| `direct_weak_jump_overcall` (majors) | Obstruction: a preempt's cost to the opponents is invisible to DD, so DD understates it | −0.0026 (none) |
| Direct-seat table as exact per-overcall rows | KR2: the structural conversion to rows | −0.0046 (both) |
| Phase 3 substitute-don't-intersect | KR2: the reading architecture; the measured code no longer exists to re-run | −0.0040 (seed 1, both) |
| Reading-drift batch, round 3 | Correctness: seven reading repairs plus a rail | −0.0043 (none) |
| Raise readers off the point-count axis | Correctness; 6,400 boards could not resolve anything | −0.0054 |

## Issue #9: cell counts cited as replication

Audited 2026-10-16: every verdict that cites a cell count or a sign pattern
across cells ("4/4 cells", "all four cells", "every cell", "eight of eight")
in the CHANGELOG, `docs/` (archive included), the `scripts/ab-*.sh` headers
and the knob doc comments in `src/`, which repeat CHANGELOG verdicts. About
470 mentions. The cells of one seed are one deal set
([measurement.md](measurement.md) checklist 5), so the question is whether
the per-cell CIs carry the decision, whatever the count says. This pass
re-measured nothing and reverted nothing.

- **Two or more seeds** (about 100 verdicts): out of scope. `bba-gen` seeds
  each shard from its own stream, and every recorded pair of bases is
  hundreds of seconds apart against 32 shards, so its seeds are separate
  deal sets. The `seeded_deals` window overlap is #6's.
- **One seed, the CIs carry the decision** (about 70): wording only. Where
  the CHANGELOG omits the ±, the owning doc or script header has it (the
  Landy doubler ladder, `quantitative_six_notrump`, Landy Texas, the two
  Multi doubler notrump rungs). Four Landy A/Bs share seed 1789977169
  (`splinter-rebids`, `splinter-tails`, `wilkosz`, `nt-vs-x`); each clears
  its own CIs and none is cited as corroborating another.
- **One seed, the count is the evidence** (about 20 verdicts), below.

**Already re-read by #7**, no further action: `passed_hand_overcall`,
`strength_ceilings`'s legacy arm, `set_advance_2nt_continuation`, N1q
`strx`, Kokish–Kraft's none cell, N3's `at_least_as_long`, the cue-blocked
face + notrump dichotomy, the 5-2 trump repair, Part A of the cramped
doubled answer, DOPI/ROPI/DEPO and `nt_overcall_systems_on`; the reading-drift
round 3 batch and `direct_weak_jump_overcall` are #7's recorded trade-offs.

**Superseded**, the measured code no longer ships: the v5 floor swap ("3 of 4
cells", seed 1786137947) and the M32 v6 promotion (American plain DD vs BBA
−0.0108 ±0.0116 / −0.0041 ±0.0140, inconclusive), both under the K = 8
floor, whose K = 4 step beat M32 by 4–7σ; and the Wide-vs-Classic
`NotrumpShape` step, under the two-seed Wide6322 default.

**Live, not covered by #7:**

| Verdict | Seed | What the count stood in for | Status |
| --- | --- | --- | --- |
| Phase 5 v6 flip, American ([authored-reading-handoff.md](authored-reading-handoff.md)) | one, fresh deals | "won all four cells", no ± printed | **win \| win**, recovered from dl02 `ab-results/v6-honest-american`: plain +0.0297 ±0.0103 / +0.0374 ±0.0125, PD +0.0164 ±0.0120 / +0.0171 ±0.0143; now in the handoff |
| `set_nt_splinter` with `nt_splinter_answer` | 1785227147, self-play, 5M bd/vul | "win in all four cells", no CI | **Non-inferior** by the δ / 24 bound: 176 of 5M boards diverge (0.0035%). A win is not shown; relabelled in [bba-1nt-splinter.md](ai-bidder/bba-1nt-splinter.md) and [one-notrump-constructive.md](one-notrump-constructive.md) |
| A maximum 1NT opener stretches a weak Lebensohl sign-off to game (0.10.0, knobless, [lebensohl.rs](../src/bidding/american/competition/lebensohl.rs)) | one, 500k, `lebensohl-ab` | "positive in every cell", +0.0010 to +0.0012, no CI; the cells are two Lebensohl styles × two vulnerabilities on one deal set | **Re-measured 2026-10-16: plain DD win \| PD loss**, the doubling-artifact row. Knob `competition.lebensohl_signoff_raise`, `scripts/ab-lebensohl-signoff-raise.sh`, `SEED_BASE` 1792096531615205258, 384k `--filter-lebensohl-raise` boards/arm/vul (density 0.004719), gate 0 foreign. On − off, IMPs/fired, none/both (1159/1049 fired): plain +0.31/+1.60, PD −0.71/+0.04, sd-lead +0.96/+2.69, SD-PD +0.12/+1.41. Losses sit on three trumps (17 HCP) and on 16 HCP with four trumps; every raising opener in the slice was balanced, so none reached the bar by an upgrade. **Tightened the same day**: a fresh seed (1792098317406025283, `scripts/ab-lebensohl-signoff-target.sh`) read `24` over `23` white plain +0.18 non-inferior \| PD +1.33 win per fired, so white ships `24`: no three-trump raise on a balanced opener, where points are HCP. Red keeps `23`, the [recorded trade-off](#red-lebensohl-target-a-recorded-trade-off) below. Both seeds measured only balanced openers: see [the filter scope](#filter-scope-the-default-1nt-is-wider-than-the-scan). Default `Some((24, 23))` |
| `multi_minor_slam_try` at `15` rather than `13` ([minor-transfer-slam.md](minor-transfer-slam.md)) | 1787642695 (round 1 fired 19 boards) | `15` over `13` rests on a "cleaner win on all four of its cells"; head-to-head t +0.70 / +0.43, round 1 the other sign | **Measured 2026-10-16 and reversed:** an enriched head-to-head on two fresh seeds reads `13` over `15` win \| win in all eight cells (Round 3 in the linked doc); the default is now `13` |

#### Red Lebensohl target, a recorded trade-off

Red keeps `23` without a proof of either kind. Rescored 2026-10-16 with
`--density` 0.004717 on seed 1792098317406025283 (0.004719 on
1792096531615205258), red cell, IMPs per accepted board ±95% CI:

| `23` against | plain DD | PD | sd-lead | SD-PD |
| --- | --- | --- | --- | --- |
| `24` | +0.0018 ±0.0011, win | −0.0017 ±0.0013, loss | +0.0039 ±0.0012 | +0.0010 ±0.0013 |
| off | +0.0040 ±0.0013, win | −0.0006 ±0.0015, non-inferior | +0.0068 ±0.0014 | +0.0030 ±0.0016 |
| off, first seed | +0.0044 ±0.0013, win | +0.0001 ±0.0014, non-inferior | | |
| `24`, wide seed | +0.0015 ±0.0010, win | −0.0017 ±0.0012, loss | +0.0036 ±0.0011 | +0.0010 ±0.0012 |
| off, wide seed | +0.0034 ±0.0013, win | −0.0008 ±0.0015, non-inferior | +0.0062 ±0.0013 | +0.0026 ±0.0015 |

The wide seed is the confirmation in the next section (density 0.005522).
Every row is the decision table's doubling-artifact row, and `24` over `23`
is the converse loss \| win, PD's synthetic-X row, so neither target has a
shippable case over the other. The basis for keeping the incumbent: a
vulnerable game breaks even at about 37.5% at IMPs against 45% white, so
red stretches a point lighter on bridge grounds, and the single-dummy
scorer prefers `23` (sd-lead a win over `24`, SD-PD a win over off). The
accepted loss is PD's lower bound: −0.0030 per accepted board against `24`
and −0.0021 against off, that is −1.4×10⁻⁵ and −1.0×10⁻⁵ IMPs per random
deal, both far inside δ. Those per-deal figures hold only for the balanced
openers the scan admitted; see the next section. Different seeds share no
deals: `deal-overlap` read `0 shared (0.00% of 384000, 0.00% of 384000):
ab-results/lebensohl-signoff-raise/raise-none ∩
ab-results/lebensohl-signoff-target/t23-none`.

#### Filter scope: the default 1NT is wider than the scan

Both new filters, `--filter-lebensohl-raise` and `--filter-multi-slam`,
took `is_1nt_opener`'s candidate: 15–17 HCP, balanced, at most one
doubleton. The default `NotrumpShape::Wide6322` also opens 5m422 and 6m322,
and those shapes upgrade a point, so they are exactly where a 17-HCP opener
still raises three trumps at `24`. The four seeds above never saw one: in
the strict slice every raising opener was balanced. Both filters now take
`is_wide_1nt_opener` (15–17 HCP, no suit shorter than two, a superset of
the default), and each runner was re-run on a fresh seed with its pass
conditions written into the header first.

| Run | Seed, density | Diff | Plain DD /accepted | PD /accepted | Fired | Pre-registered condition |
| --- | --- | --- | --- | --- | --- | --- |
| Lebensohl, white | 1792101907466221783, 0.005522 | `24` − `23` | +0.0007 ±0.0008, non-inferior | +0.0031 ±0.0009, win | 881 | met: `24` stands |
| | | `24` − off | +0.0010 ±0.0005, win | +0.0003 ±0.0006, non-inferior | 356 | met |
| Lebensohl, red | | `23` − off | +0.0034 ±0.0013, win | −0.0008 ±0.0015, non-inferior | 1088 | met: the trade-off stands |
| Lebensohl, ew (we white) | 1792104633669095399, 0.005478 | `24` − `23` | −0.0001 ±0.0008, non-inferior | +0.0022 ±0.0009, win | 893 | met: `24` stands |
| | | `24` − off | +0.0009 ±0.0005, win | +0.0003 ±0.0006, non-inferior | 344 | met |
| Lebensohl, ns (we red) | | `23` − off | +0.0035 ±0.0013, win | −0.0011 ±0.0015, non-inferior | 1102 | met: the trade-off stands |
| Multi slam, none | 1792102740877734887, 0.001824 | `13` − `15` | +0.0018 ±0.0012, win | +0.0023 ±0.0013, win | 968 | met: `13` stands |
| Multi slam, both | | `13` − `15` | +0.0017 ±0.0011, win | +0.0020 ±0.0012, win | 494 | met |

The ns/ew rows are §6's four-cell check on a third Lebensohl seed (`ns`: we
red against white; `ew`: we white against red). Gate 0 foreign in all
fourteen pairs. On the wide Lebensohl seed, 163 of the
white `23` arm's 1,236 raises came from 5m422/6m322 openers (13%), and
`24` still raises ten of them on three trumps (17 HCP, 18 points). Different
seeds share no deals: `deal-overlap` read 0 shared for the wide `t23-none`
and the ns/ew `t23-ns` against every other Lebensohl seed, for the wide `s13-none` against both
earlier Multi seeds, and for the first two Multi seeds against each other.

The older scans, `--filter-1nt`, `--filter-landy` and `--filter-preempt`
(about 45 runners), keep the strict candidate, so their persisted seeds still
reproduce, and every verdict they produced since Wide6322 shipped on
2026-07-12 omits the semi-balanced 1NT openers. New runs pass `--wide-1nt`
(jdh8, 2026-10-16), which switches all three to the wide candidate.

**Rejections resting on the count**, where a default left off needs no
proof: `advance_pass_yield_major` (seed 1785321152, per-fired figures only),
Phase 4 follow-up 2 Variant A (1786978290, raw IMPs, later seeds not run),
the splinter floor at 8 (totals only) and the Rubens revert (plain
straddles, PD carries it).

**Pooled across the two vulnerabilities of one seed:** C1's encoding
difference in [dnf-migration.md](dnf-migration.md) and A/B-1 in
`scripts/ab-eval-shape.sh` ("both pooled intervals exclude zero") count each
deal twice, so those intervals are too narrow. The closure verdict rests on
its −0.18 cost and `set_eval_shape` stays off, so nothing changes; the doc
now says so.

## 4 Disclosure in SD dump rescoring

**Cause confirmed and the dump harness fixed 2026-10-16 (`7c60e309`).**
`ab-dump-sd` read every seat through the plain partnership, which decodes the
opponents' calls only when alerted. Our pair sits North–South, so when we
declared, the East–West leader saw only our alerted calls: West read the
unalerted `(1NT) 2♥` as `points 8..37` under both 9–14 and 20–37. A
North–South leader was unaffected after 2026-08-16. The alerted negative
double reaches both leaders today; its July inertness probably came from the
unpinned decode contexts fixed 2026-08-09 (inferred, not verified).
`common::DumpReader` reads East–West with our partnership declared as its
opponent, so every North–South call decodes off its authoring rule. The band
now moves 195 of 300 diagnostic leads and 48 trick counts (legacy: 0 and 0);
the default system is byte-identical (`smoke-default`, 20,000 auctions).
Reports print `leader reading: disclosed|legacy`, and
`--legacy-leader-reading` reproduces old outputs; regression tests in
`examples/ab-dump-sd/tests.rs`.

**Open.** (1) The self-play SD harnesses read the leader the same way:
`ab-nt-defense-matrix`, `ab-point-count`, `ab-dnf-sd-lead`,
`ab-meckstroth-2nt`, `ab-notrump-minors`, `ab-forcing-nt-two-suiter`,
`ab-fuzzy-strength`, `ab-size-ask-*`, `ab-nt-splinter`, `ab-one-nt-runout`.
(2) Four verdicts rest on SD pricing the disclosure of an unalerted call of
ours, and need a re-measure: the natural overcall HCP floor of 8 with its
advance (`defensive-overcalls.md`, CHANGELOG 2026-08-23), the 8:14 overcall
band (`archive/point-count-threshold-campaign.md`, `convention-tuning.md`),
Modern negative doubles' sd overrule of the vul-PD −0.026
(`competitive-book.md`), and the weak-two HCP knobs (self-play
`ab-point-count`). Thirteen more cite the sd row but stand on DD or on an
alerted call (`nt_overcall_systems_on`'s major cells the strongest).

The original finding, for the record:

**P1 — recorded harness defect.**
[measurement.md](measurement.md#scorers-srcscoringrs) records that changing the ON overcall
reading from 9–14 to an absurd 20–37 reproduces the same IMPs, as does dropping
the modern-negative-double flag. The intended disclosure manipulation is inert.
[ab-dump-sd](../examples/ab-dump-sd/main.rs), `lead_inputs` and the ON/OFF
partnership construction, are the relevant paths.

The leader's sampler is fed by the scored arm's own system (`lead_inputs`
calls `partnership.infer` on the ON or OFF partnership), so the manipulation
reaches the reader. The probable cause, from reading the code and not yet
tested: when the leader reads our side's calls,
[projection.rs](../src/bidding/inference/projection.rs) decodes them through our
book only under `table_alerts` or declared opponents, and with undeclared
opponents (as in `ab-dump-sd`) only at `ReadingScope::Alerted`. Unalerted
natural calls keep the natural walk. The overcall band only sets
`points(lo..=hi)` on the unalerted natural overcall rules in
`american/defense/nt_defense.rs`, so the leader never sees it. Whether the
modern negative double follows the same path is unchecked.

Existing results describe routing and auctions under that reader.
`measurement.md` already withdraws the claim that a dump rescore prices
disclosure or lead-direction value. What it lacks is the list of verdicts
whose mechanism depended on that pricing (concealment, lead direction,
disclosure cost); those need a treatment-by-treatment qualification.

- Confirm the alerted-only decode as the cause with a diagnostic lead case.
- Make an absurd-band manipulation change the sampled model and demonstrate
  an effect on a diagnostic lead case.
- Identify affected concealment, lead-direction and disclosure-cost verdicts;
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
that the wide-shape 1NT slice is invisible to its arms and that this bounds the
claim. Reporting per-divergent IMPs does not recover that excluded population.

Done 2026-10-16: `measurement.md` now states the coverage condition, and
`bba-gen --wide-1nt` (always on for the two #9 filters) draws the candidate
from every 15–17 HCP hand with no suit shorter than two, a superset of the
default opener; see [the filter scope](#filter-scope-the-default-1nt-is-wider-than-the-scan).
The verdicts drawn on the strict candidate since 2026-07-12 are not re-run.

- Run an unfiltered bidding-only preflight and count divergences outside
  the raw-hand filter before interpreting an enriched result globally.
- For a trigger that is not a 1NT opening, use the union of candidate
  triggers or separate strata, and report the population weights and
  conditional effects together.

The point estimate in
[probe-weak-two-major](../examples/probe-weak-two-major/main.rs) is internally
consistent: its mean is over boards reaching the face and its density is
reach/draws, so their product is total swing/draws. Its scaled CI treats the
estimated density as fixed; rate uncertainty is a smaller follow-up. Do not
describe this as a demonstrated normalization bug. Its stage-1 `accepts()`
raw-hand predicate has the same coverage limit as the bba-gen filters, and
stopping at `--count` gives `boards/draws` a small stopping-rule bias.

## 6 Asymmetric vulnerability in competitive experiments

**P1 — coverage risk with an observed sign reversal.**
[anchor.sh](../scripts/anchor.sh) and 163 of the 169 `scripts/ab-*.sh` runners
loop only none/both; `measurement.md` asks for a none/both split.
Competitive, sacrifice and doubling decisions also depend on favourable and
unfavourable vulnerability. The repo already has a decisive example in
[ab-landy-nt-vs-x.sh](../scripts/ab-landy-nt-vs-x.sh):

| Vulnerability | Plain DD IMPs/board | Plain SD IMPs/board |
| --- | ---: | ---: |
| EW vulnerable | +0.0147 | +0.0042 |
| NS vulnerable | -0.0107 | -0.0206 |
| Neither | | -0.0108 |
| Both | | -0.0054 |

The favourable gate (`landy_notrump_no_major_favourable`, 2026-09-22) shipped
after this four-cell comparison; the script is the only runner that loops all
four cells. This is evidence
for extending that practice, not a claim that all existing runners lack it.

Done 2026-10-16: `ab-lib.sh` takes `VULS` (default `none both`) and the 40
runners with a plain `for v in none both` loop read it; measurement.md
checklist 8 now asks for all four cells on competitive, sacrifice and
doubling treatments and on any knob that reads `vulnerable()`, and for a
verdict to name its cells. First use: the colour-split
`lebensohl_signoff_raise` holds at `ns` and `ew` too
([its rows](#filter-scope-the-default-1nt-is-wider-than-the-scan)).

- **Owed:** re-confirming the competitive defaults shipped on none/both
  only. No list is drawn up yet; the runners whose own loops differ (about
  130, including those that never source ab-lib) need hand edits first.

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

Documentation done 2026-10-16: every current doc and doc comment that called
replay "the self-play posterior", "worlds the bidder reproduces" or
"auction-consistent worlds" now reads relaxed authored-rule conditioning
(within `MARGIN` of the best legal logit, abstaining off-book), or range-only
conditioning for the range path. exact-posterior.md states the `1M - 2M -`
range failure as the non-default path's and flags the counted-reading
re-run's 91.2% deal overlap. A stale "default off" for `set_rule_accept` in
sampled-projection.md was corrected (default on since `74d783d`).

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

- Report draws, acceptance, fallback counts and per-board fallback rates.
- On important divergent sets compare a common fallback policy or an
  insufficient-sample/no-signal policy, and record sensitivity.

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

- Add same-layout, same-strain cases with different contract objectives to
  distinguish safety play from mean-trick maximization.
- Report sensitivity to the objective, lambda and analytic shave on the
  actual divergent population; validate the aligned-cell fit before using it.
- Qualify point-estimate and truth-bracket language. A larger sample
  reduces sampling error, not these modelling assumptions.

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
where the M32 cut relabelled the row). **Open:** run `--validate` on the 20
M32 shards (the data is not on this box); a stored per-row identity sibling
for future dumps.

The original finding:

[trainer/src/data.rs](../trainer/src/data.rs), `load_mixture`, divides each
stem at `rows - round(rows * val_frac)`. It stores no board boundary for the
split, while [dump-teacher](../examples/dump-teacher/main.rs) emits multiple
auction/cell rows per board. The board-disjoint guarantee is therefore too
strong. In a contiguous, non-overlapping stem this can leak the boundary board;
it is not evidence of widespread leakage or invalid fresh-deal A/Bs.
[logit-calibration.md](ai-bidder/logit-calibration.md) already records the
contiguous-tail split and treats held-out CE as diagnostic; the claims still to
correct are the "board-disjoint" comments in `trainer/src/data.rs` and
`trainer/src/main.rs`.

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

- Preserve a stable deal ID, dealer, vulnerability, full auction prefix and
  cell; group every row of a board into one training/validation partition.
- Validate both positional and keyed splices against that identity.
- Audit existing splices and report unmatched, ambiguous or changed rows
  before deciding which labels need recomputation.

## 11 Experiment identity and resumable runs

**Resolved 2026-10-16 for `bba-gen` and `ab-lib.sh`.** Shards are written to
a temp file and renamed after an fsync; `bba-gen --verify` checks a shard's
board count and seed; `bba-gen-parallel.sh` waits on every worker by pid,
exits non-zero on any failure and verifies exactly `shard-0..N-1`; `ab-lib.sh`
and `anchor.sh` resume on that check. `arm.meta` is a manifest: the runtime
fingerprint plus shards, per-shard and `SEED_BASE`, compared, with source
SHA, dirty-diff hash, JOBS and BOARDS recorded but not compared (the binary's
bytes are the code identity, so a doc-only commit does not wedge a resume;
comparing the SHA is a one-line change if wanted). Gate, diff and SD reports
carry an `.inputs` stamp and are rescored when it changes. **Open:**
`ben-gen-parallel.sh` (bare `wait`) and `ben-gen` (non-atomic writes); the
runners that shadow `arm`/`diffpair`/`sddiff` or keep their own resume
(`ab-ben-*`, `ab-diamond-floor`, `ab-face-corroborate`, `ab-reading-drift`,
`ab-walk-shape`, `ab-weak-two-balancing`, `ab-reading-knobs`,
`ab-nt-overcall-gladiator`, `ab-nt-overcall-systems-on`); `common::load_dump`
does not compare `gen_args` across shards.

The original finding, for the record:
[ab-lib.sh](../scripts/ab-lib.sh), `arm`, considers an arm complete by shard
count; `diffpair` skips when two report files are nonempty. A guard pins
JOBS and BOARDS per results directory, but reusing it after changing flags or
HEAD can still reuse an older run. `bba-gen` writes each shard straight to its
final path, so a crashed worker leaves a truncated shard that the count accepts,
and `bba-gen-parallel.sh` ends with a bare `wait` that discards exit codes.
[common::load_dump](../examples/common/mod.rs) appends later shards without
checking their labels, vulnerability or configuration. Dumps carry seed/args
but not a full immutable code, weights and scorer identity.

- Save a manifest with source SHA, dirty diff identity, weights hash,
  generator/scorer/solver versions, configuration and expected shard IDs/counts.
- Resume only on matching manifests; validate every shard and report.
- Collect each worker's exit status and verify expected board counts, not
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
