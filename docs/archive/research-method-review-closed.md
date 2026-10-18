# Bridge research methods: closed entries

Moved here verbatim on 2026-10-16 from
[research-method-review.md](../research-method-review.md), which keeps the
open corrections and the decisions still owed. Only links were rebased; the
section numbers are the live review's. Every entry below is done: a measured
result, a recorded trade-off or an explicit decision. The doc that owns each
conclusion has it too; this file keeps the review's own record of how it was
reached.

## Closed issues

| Priority | Issue | Completion landed in |
| --- | --- | --- |
| P1 | [#6 Deal overlap between fresh-seed runs](https://github.com/jdh8/pons/issues/6) | `measurement.md` and affected replication ledgers |
| P1 | [#7 Non-inferiority margin instead of wash as no loss](https://github.com/jdh8/pons/issues/7) | `measurement.md`, the ship rules and affected verdicts — margin and verdicts landed 2026-10-14; [inventory](#issue-7-non-inferiority-inventory) decided and re-measured; both losses inside δ repaired and proven non-inferior 2026-10-15 (XYZ's `2♥`, the reverse's HCP floor); reporter coverage and the density / small-sample guards landed 2026-10-15 (the two modeled SAT diagnostics labelled exempt, and the resume guard fingerprinting the binary an arm runs, 2026-10-16); the four status-unknown rows with an off arm re-measured 2026-10-15 (`competitive_4333` and `takeout_support` proven, `penalty_no_pull` inert and removed, the diamond-transfer contest a loss inside δ, its repair `diamond_transfer_responder_captain` shipped default-on 2026-10-15 on a three-arm A/B); `PointCount` proven win \| win 2026-10-15; the RKCB queen ask superseded (no off arm) |
| P1 | [#8 Direct A/B versus reference-relative IMP gain](https://github.com/jdh8/pons/issues/8) | scorer reports, `measurement.md` and anchor ledgers |
| P2 | [#9 Vulnerability cells on one seed are one deal set](https://github.com/jdh8/pons/issues/9) | `measurement.md` checklist 5 (2026-10-14) and the [cell-count audit](#issue-9-cell-counts-cited-as-replication) (2026-10-16) |

## Issue #7: non-inferiority inventory

Defaults that shipped on a plain-DD wash before the margin, re-read on
2026-10-14 against δ = 0.002 IMPs/board
([measurement.md](../measurement.md#the-decision-table)). The worst
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
| `direct_weak_jump_overcall` (majors) | +0.0002 ±0.0028 (**−0.0026**) / +0.0031 ±0.0035 (−0.0004) | +0.0026 ±0.0032 / +0.0051 ±0.0040 — straddles too, so row "wash \| PD win" never applied | 1.16% / 1.11% | [ben-gap-campaign.md](../ben-gap-campaign.md) | Trade-off: a preempt's obstruction is DD-invisible, so DD understates it |
| Direct-seat table as exact per-overcall rows (knobless) | +0.0005 ±0.0029 (−0.0024) / −0.0009 ±0.0037 (**−0.0046**) | +0.0035 ±0.0035 / +0.0008 ±0.0043 | not quoted | CHANGELOG 2026-08-06 | Trade-off: KR2 structural conversion |
| Phase 3 substitute-don't-intersect (knobless) | seed 1 −0.0008 ±0.0023 / −0.0013 ±0.0027 (**−0.0040**); seed 2 +0.0013 ±0.0022 / −0.0002 ±0.0027 (−0.0029) | both seeds straddle | 0.57–0.68% | [authored-reading-handoff.md](../authored-reading-handoff.md) | Trade-off: KR2 reading architecture; the measured code no longer exists to re-run |
| Reading-drift batch, round 3 (seven repairs + rail) | +0.0007 ±0.0050 (**−0.0043**) / +0.0049 ±0.0058 (−0.0009) | −0.0010 ±0.0058 / +0.0067 ±0.0067 | 2.91% / 2.58% | [reading-drift-handoff.md](../reading-drift-handoff.md) | Trade-off: correctness repairs |
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
([measurement.md](../measurement.md) checklist 5), so the question is whether
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
| Phase 5 v6 flip, American ([authored-reading-handoff.md](../authored-reading-handoff.md)) | one, fresh deals | "won all four cells", no ± printed | **win \| win**, recovered from dl02 `ab-results/v6-honest-american`: plain +0.0297 ±0.0103 / +0.0374 ±0.0125, PD +0.0164 ±0.0120 / +0.0171 ±0.0143; now in the handoff |
| `set_nt_splinter` with `nt_splinter_answer` | 1785227147, self-play, 5M bd/vul | "win in all four cells", no CI | **Non-inferior** by the δ / 24 bound: 176 of 5M boards diverge (0.0035%). A win is not shown; relabelled in [bba-1nt-splinter.md](../ai-bidder/bba-1nt-splinter.md) and [one-notrump-constructive.md](../one-notrump-constructive.md) |
| A maximum 1NT opener stretches a weak Lebensohl sign-off to game (0.10.0, knobless, [lebensohl.rs](../../src/bidding/american/competition/lebensohl.rs)) | one, 500k, `lebensohl-ab` | "positive in every cell", +0.0010 to +0.0012, no CI; the cells are two Lebensohl styles × two vulnerabilities on one deal set | **Re-measured 2026-10-16: plain DD win \| PD loss**, the doubling-artifact row. Knob `competition.lebensohl_signoff_raise`, `scripts/ab-lebensohl-signoff-raise.sh`, `SEED_BASE` 1792096531615205258, 384k `--filter-lebensohl-raise` boards/arm/vul (density 0.004719), gate 0 foreign. On − off, IMPs/fired, none/both (1159/1049 fired): plain +0.31/+1.60, PD −0.71/+0.04, sd-lead +0.96/+2.69, SD-PD +0.12/+1.41. Losses sit on three trumps (17 HCP) and on 16 HCP with four trumps; every raising opener in the slice was balanced, so none reached the bar by an upgrade. **Tightened the same day**: a fresh seed (1792098317406025283, `scripts/ab-lebensohl-signoff-target.sh`) read `24` over `23` white plain +0.18 non-inferior \| PD +1.33 win per fired, so white ships `24`: no three-trump raise on a balanced opener, where points are HCP. Red keeps `23`, the [recorded trade-off](#red-lebensohl-target-a-recorded-trade-off) below. Both seeds measured only balanced openers: see [the filter scope](#filter-scope-the-default-1nt-is-wider-than-the-scan). Default `Some((24, 23))` |
| `multi_minor_slam_try` at `15` rather than `13` ([minor-transfer-slam.md](../minor-transfer-slam.md)) | 1787642695 (round 1 fired 19 boards) | `15` over `13` rests on a "cleaner win on all four of its cells"; head-to-head t +0.70 / +0.43, round 1 the other sign | **Measured 2026-10-16 and reversed:** an enriched head-to-head on two fresh seeds reads `13` over `15` win \| win in all eight cells (Round 3 in the linked doc); the default is now `13` |

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

The standing rule for the strict-scan verdicts and the runner-by-runner
classification are in
[Filter scope: the rule](#filter-scope-the-rule-and-the-runner-classification).

**Rejections resting on the count**, where a default left off needs no
proof: `advance_pass_yield_major` (seed 1785321152, per-fired figures only),
Phase 4 follow-up 2 Variant A (1786978290, raw IMPs, later seeds not run),
the splinter floor at 8 (totals only) and the Rubens revert (plain
straddles, PD carries it).

**Pooled across the two vulnerabilities of one seed:** C1's encoding
difference in [dnf-migration.md](../dnf-migration.md) and A/B-1 in
`scripts/ab-eval-shape.sh` ("both pooled intervals exclude zero") count each
deal twice, so those intervals are too narrow. The closure verdict rests on
its −0.18 cost and `set_eval_shape` stays off, so nothing changes; the doc
now says so.

## Filter scope: the rule and the runner classification

Moved here on 2026-10-18; decided 2026-10-16, nothing owed.

The `bba-gen` scans `--filter-1nt`, `--filter-landy` and `--filter-preempt`
take `is_1nt_opener`'s candidate: 15–17 HCP, balanced, at most one
doubleton. The default `NotrumpShape::Wide6322` also opens 5m422 and 6m322,
so every verdict those scans produced since Wide6322 shipped on 2026-07-12
omits the semi-balanced 1NT openers. The scans keep the strict candidate by
default, so their persisted seeds still reproduce; new runs pass
`--wide-1nt` (jdh8, 2026-10-16), which switches all three to the wide
candidate (15–17 HCP, no suit shorter than two, a superset of the default).
The wide re-runs of the Lebensohl and Multi-slam verdicts are
[archived](#filter-scope-the-default-1nt-is-wider-than-the-scan).

**Decided 2026-10-16 (jdh8): no blanket re-run, and no coverage threshold.**
14.4% of the default's 1NT openers are 5m422 or 6m322, so a preflight
would report about that share missed for nearly every lane after our 1NT,
and a 5% trigger would fire on all of them. The rule, in
[measurement.md](../measurement.md#the-decision-table), turns on what the
treatment touches: opener's call means re-measure with `--wide-1nt` before
the verdict is relied on, responder's call keeps a "balanced openers only"
label, and our defense to their 1NT is covered (BBA's 1NT is 98.0%
strict-balanced and 2.0% 5m422 on 4,000 `--isolate-defense` boards, seed
1792160000). Fifty runners use a strict scan. Eight now pass `--wide-1nt`:
the four natural-overcall runs, and the four §6 confirmations
(`2d-multi-doubler`, `landy-splinter-tails`, `landy-wilkosz` at their
asymmetric cells; `landy-strength-residue` for
`defense_2c_landy_strength_nv_invite` at `ns` and `both`). The none/both
cells those four originally shipped on were strict, so those cells fall
under the same rule. The other 42 runners, classified from their headers:

| Class | Runners (`scripts/ab-*.sh` unless named) | Status |
| --- | --- | --- |
| Opener's call, or the arm also authors opener's continuation | `2d-double`, `2d-multi`, `2d-multi-balance`, `2d-multi-doubler-nt`, `2d-multi-doubler-min-nt`, `2d-multi-escape`, `2d-multi-kk`, `2d-multi-px`, `2d-multi-residue`, `landy-counter`, `landy-confirm`, `landy-bba`, `landy-doubler-game`, `landy-minor-slam`, `landy-opener`, `landy-splinter-stopper`, `landy-strength`, `landy-texas`, `nt-high-overcall`, `a3`, `sd-pd-queue.sh` (runout leg) | Balanced openers only; re-measure with `--wide-1nt` before relying on the verdict |
| Responder's, the doubler's or a reading only | `landy-read`, `landy-rungs`, `landy-doubler-rebids`, `landy-doubler-flip`, `landy-lia-doubler`, `landy-lia3`, `landy-nt-remeasure`, `landy-notrump-shape`, `landy-nt-vs-x`, `landy-major-jam`, `landy-splinter-rebids` | Balanced openers only; the label stands |
| Their 1NT (BBA opens) | `nt-natural-overcall`, `one-nt-defense-range`, `stayman-defense`, `a5` | Covered to within 2% |
| No live verdict | `landy-lia`, `landy-lia-repair`, `landy-lia2` (superseded); `2d-multi-slam` (superseded by the wide `2d-multi-slam-floor`, which uses `--filter-1nt` only in its comments); `v6-their-reading` (screen of a twin never shipped) | None owed |

The classification reads each runner's header for the seats its arms
author; a whole-table arm that also authors opener's answers counts as
opener's. `ab-lebensohl`, `ab-one-nt-runout` and `ab-uvu` keep their own
strict `is_1nt_opener` and fall under the same rule (`a3` drives the first
two).

## 4 Disclosure in SD dump rescoring: the fix and the settled re-measures

**Cause confirmed and the dump harness fixed 2026-10-16 (`7c60e309`).**
`ab-dump-sd` read every seat through the plain partnership, which decodes the
opponents' calls only when alerted. Our pair sits North–South, so when we
declared, the East–West leader saw only our alerted calls: West read the
unalerted `(1NT) 2♥` as `points 8..37` under both 9–14 and 20–37. A
North–South leader was unaffected after 2026-08-16. The alerted negative
double reaches both leaders today; its July inertness probably came from the
unpinned decode contexts fixed 2026-08-09 (inferred, not verified).
`common::DumpReader` (now `TableReader`) reads East–West with our partnership declared as its
opponent, so every North–South call decodes off its authoring rule. The band
now moves 195 of 300 diagnostic leads and 48 trick counts (legacy: 0 and 0);
the default system is byte-identical (`smoke-default`, 20,000 auctions).
Reports print `leader reading: disclosed|legacy`, and
`--legacy-leader-reading` reproduces old outputs; regression tests in
`examples/ab-dump-sd/tests.rs`.

**Self-play harnesses fixed (`430fac29`).** `ab-nt-defense-matrix`,
`ab-point-count`, `ab-dnf-sd-lead`, `ab-meckstroth-2nt`, `ab-notrump-minors`,
`ab-forcing-nt-two-suiter`, `ab-fuzzy-strength`, `ab-size-ask-*`,
`ab-nt-splinter` and `ab-one-nt-runout`, plus the SD-declarer rows of `ab-kickback`, `ab-minor-keycard`, `ab-slam-entry` and
`probe-sd-calibration` read through `common::TableReader::self_play`: each
pair reads with the other pair's partnership declared as its opponent. Six
harnesses that read every seat through one fixed book now read each table
with its own pairs' books. `--legacy-leader-reading` reproduces old outputs
byte for byte; the default system is byte-identical. The SD-blend λ
(`common::SD_BLEND_LAMBDA`) was fitted with `probe-sd-calibration` on the old
reader; a refit may be owed.

**Re-measured 2026-10-16 on the disclosed reader**, fresh deals, four
vulnerability cells, no default changed (details in the owning docs):

| Verdict | Runner, seed | Decision row now | Original rationale | Decision owed (jdh8) |
| --- | --- | --- | --- | --- |
| Natural overcall of 1NT: HCP floor 8 + authored advance | `ab-nt-natural-overcall-disclosed.sh`, 1792107984640740881 | **loss \| win**: plain DD loses at `ew` (−0.0035 ±0.0021 per accepted board, about −0.0009 per deal), PD and SD-PD win every cell | SD-PD half stands; plain DD fails in `ew`, never measured in August. Bucketed: the advance carries the whole gain, the floor's boards lose plain in all four cells | **Bisect run** (`ab-nt-overcall-floor-bisect.sh`, 1792119766568383243): floor 8 − floor 0 with the advance on loses on every scorer in every cell (plain −0.0071 to −0.0076 ±0.0020 per accepted board). The [sweep](../defensive-overcalls.md#every-live-floor-against-floor-0-the-sweep-2026-10-16--floor-0-ships) (1792143998575635198) confirms it against floors 7, 8, 9. **Decided 2026-10-16: floor 0, advance kept** |
| The 8:14 overcall band | `ab-nt-overcall-band-disclosed.sh`, 1792110704895967184 | 9:14 and 10:14 lose plain DD in three and four cells | "NV holds" stands, firmer (SD-PD now a clear loss for tighter bands); "vul leaning tighten" does not | Keep 8:14 (proposed) |
| Modern negative doubles + forcing free bids | `ab-modern-negx-disclosed.sh`, 1792113860641334667 | **win \| loss**: plain wins every cell; PD loses at `ew` and `both` (lower bounds −0.0234 / −0.0231) | The PD loss follows *their* vulnerability, not ours, so July's "thin vul games of ours" named the wrong side; SD-PD now non-inferior at `both`, inconclusive at `ew` | Keep Modern as a recorded trade-off (proposed). Also: the knob changes how we read *their* negative double (13% of divergent boards opened by them, so the isolation gate cannot pass) |

**Weak-two HCP gauge, re-measured 2026-10-16** (`ab-weak-two-hcp-disclosed.sh`,
pre-registered in `fded92a7`, SEED_BASE 1792122226364649837, 512,000 boards
per cell, self-play): `--weak-two-hcp 5:10` against the shipped
`points(5..=10)` reads plain DD **win** in all four cells (+0.0045 ±0.0025
none, +0.0056 ±0.0028 ns) and PD **inconclusive** in all four; SD-PD win at
`ns`, non-inferior at `none`, inconclusive at `ew`/`both`, a loss nowhere.
That is the pre-registered *insufficient data* branch: the knob stays off as
a recorded trade-off, and July's "over-disclose to the blind lead" rationale
is unproven, not confirmed. The legacy-reader control matches plain DD and
PD to the IMP, so the comparison is valid. A larger pre-registered run on a
fresh seed would settle PD.

The original finding, for the record:

**P1 — recorded harness defect.**
[measurement.md](../measurement.md#scorers-srcscoringrs) records that changing the ON overcall
reading from 9–14 to an absurd 20–37 reproduces the same IMPs, as does dropping
the modern-negative-double flag. The intended disclosure manipulation is inert.
[ab-dump-sd](../../examples/ab-dump-sd/main.rs), `lead_inputs` and the ON/OFF
partnership construction, are the relevant paths.

The leader's sampler is fed by the scored arm's own system (`lead_inputs`
calls `partnership.infer` on the ON or OFF partnership), so the manipulation
reaches the reader. The probable cause, from reading the code and not yet
tested: when the leader reads our side's calls,
[projection.rs](../../src/bidding/inference/projection.rs) decodes them through our
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

### The two verdicts decided 2026-10-16

Both decided by jdh8 on 2026-10-16 as proposed: Modern kept as a recorded
trade-off, `8:14` kept.

| Verdict | Runner, seed | Decision row now | Original rationale | Decision (jdh8, 2026-10-16) |
| --- | --- | --- | --- | --- |
| The 8:14 overcall band | `ab-nt-overcall-band-disclosed.sh`, 1792110704895967184 | 9:14 and 10:14 lose plain DD in three and four cells | "NV holds" stands, firmer (SD-PD now a clear loss for tighter bands); "vul leaning tighten" does not | Keep 8:14 |
| Modern negative doubles + forcing free bids | `ab-modern-negx-disclosed.sh`, 1792113860641334667 | **win \| loss**: plain wins every cell; PD loses at `ew` and `both` (lower bounds −0.0234 / −0.0231) | The PD loss follows *their* vulnerability, not ours, so July's "thin vul games of ours" named the wrong side; SD-PD now non-inferior at `both`, inconclusive at `ew` | Keep Modern as a recorded trade-off. Also: the knob changes how we read *their* negative double (13% of divergent boards opened by them, so the isolation gate cannot pass) |

## 5 Filter coverage: the original finding

**P1 — confirmed coverage limitation and overbroad interpretation.**
[measurement.md](../measurement.md#the-decision-table) says filters change the
per-board denominator but not the divergent set. In general the filtered
quantity is `E[delta | divergence, filter]`, not `E[delta | divergence]`.
These agree only under an appropriate coverage or distribution argument.

[bba-gen](../../examples/bba-gen/main.rs), `is_1nt_opener`, admits 15–17 HCP,
no singleton or void, and at most one doubleton. The
[Landy shape runner](../../scripts/ab-landy-notrump-shape.sh) explicitly records
that the wide-shape 1NT slice is invisible to its arms and that this bounds the
claim. Reporting per-divergent IMPs does not recover that excluded population.

## 6 Asymmetric vulnerability: the evidence and the four-cell runs

**P1 — coverage risk with an observed sign reversal.**
[anchor.sh](../../scripts/anchor.sh) and 163 of the 169 `scripts/ab-*.sh` runners
loop only none/both; `measurement.md` asks for a none/both split.
Competitive, sacrifice and doubling decisions also depend on favourable and
unfavourable vulnerability. The repo already has a decisive example in
[ab-landy-nt-vs-x.sh](../../scripts/ab-landy-nt-vs-x.sh):

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

**Run 2026-10-16** (pre-registered in `e75cb386`, 4,608,000 accepted boards
per arm per cell, `--filter-landy --wide-1nt` or `--filter-1nt --wide-1nt`,
gate 0 foreign everywhere), IMPs per accepted board, plain \| PD:

| Knob | Cell | Plain DD | PD | SD-PD | Reading |
| --- | --- | --- | --- | --- | --- |
| `landy_wilkosz` | ns | +0.0003 ±0.0001 win | +0.0006 ±0.0001 win | win | stands |
| `defense_2c_landy_strength_nv_invite` | ns | **−0.0008 ±0.0003 loss** | −0.0001 ±0.0003 non-inferior | −0.0004 ±0.0003 loss | trade-off, see below |
| | both (re-run) | +0.0009 ±0.0003 win | +0.0016 ±0.0003 win | win | stands |
| `landy_splinter_tails` | ns | +0.0001 ±0.0001 win | +0.0000 ±0.0001 non-inferior | non-inferior | stands |
| | ew | +0.0071 ±0.0003 win | +0.0007 ±0.0002 win | non-inferior | stands |
| `multi_doubler_major` | ns | +0.0007 ±0.0001 win | +0.0002 ±0.0001 win | win | stands |
| | ew | +0.0009 ±0.0001 win | −0.0000 ±0.0001 non-inferior | win | stands |

The strength invite (an 8–9 HCP hand keeps the values double when we are
vulnerable) loses plain DD at `ns`, −0.42 IMPs per fired board over 8,710
boards, about −7×10⁻⁶ per deal: far inside δ, but a loss. It differs from
off only when we are vulnerable, so `both` is its only other live cell, and
that cell shipped it on an older seed with the strict candidate.
**Re-run `both`, decided by jdh8** (rule pre-registered in the runner
before reading: on iff `both` reads a plain DD win with PD not a loss;
`ab-results/landy-strength-residue-both`, seed 1792152287380614337, sha
`bf359622`, 4,608,000 accepted boards per arm, density 0.009021, gate 0
foreign of 8,599 divergent): plain +0.0009 ±0.0003 **win** (+0.46 per
fired board), PD +0.0016 ±0.0003 win, SD-PD +0.0013 ±0.0003 win (plain SD
−0.0001 ±0.0003 non-inferior). **The knob stays on**, a recorded trade-off:
it wins at `both` and loses plain DD at `ns`. The two cells differ only in
*their* colour, which the gate does not read; a `both`-only gate
(`vulnerable() & they-vulnerable`) is the untested refinement it suggests.
Caveat on the SD column: the off arm's sampler fell back on 44.8% of its
worlds (on: 1.0%), so the SD numbers price the off arm's reading coarsely.

**The confirm queue, 2026-10-16** (`scripts/confirm-queue-2026-10-16.sh`
at `a9362f96`, each rule pre-registered in its runner's header before any
number was read, fresh seed per step, `ab-results/<step>`, gate 0 foreign
in every gated cell; the competitive runner has no gate step). IMPs per
board (per accepted board on the filtered runs), plain \| PD, 95% CI:

| Knob | Step, seed | Cell | Fired | Plain DD | PD | sd-lead plain / SD-PD | Reading |
| --- | --- | --- | ---: | --- | --- | --- | --- |
| `their_2nt_double_veto` | `2nt-double-veto-nsew`, 1792162558292829023, 409,728 bd | ns | 1,526 | +0.0161 ±0.0016 win | +0.0001 ±0.0011 non-inferior | win / non-inferior | stands |
| | | ew | 1,584 | +0.0196 ±0.0019 win | +0.0009 ±0.0014 non-inferior | win / inconclusive | stands |
| `their_3nt_pull_veto` | `3nt-pull-veto-nsew`, 1792163023089299798, 409,728 bd | ns | 188 | +0.0036 ±0.0007 win | +0.0068 ±0.0010 win | win / win | stands |
| | | ew | 269 | **−0.0010 ±0.0007 loss** | +0.0087 ±0.0010 win | loss / win | flagged (a) |
| `competitive_accountant` | `competitive-4cell-nsew`, 1792163301883580764, 1,228,800 bd, unfiltered | ns | 2,208 | +0.0022 ±0.0007 win | −0.0003 ±0.0006 non-inferior | win / non-inferior | stands |
| | | ew | 5,138 | +0.0112 ±0.0010 win | −0.0005 ±0.0009 non-inferior | win / non-inferior | stands |
| `competitive_pass_demotion` | same | ns | 2,052 | +0.0020 ±0.0006 win | −0.0007 ±0.0006 loss | win / non-inferior | stands (PD double-blind) |
| | | ew | 4,839 | +0.0106 ±0.0010 win | −0.0020 ±0.0008 loss | win / non-inferior | stands (PD double-blind) |
| `competitive_push_natural_only` | same | ns | 3,411 | −0.0000 ±0.0008 non-inferior | +0.0010 ±0.0008 win | non-inferior / non-inferior | stands |
| | | ew | 4,844 | **−0.0016 ±0.0011 loss** | +0.0028 ±0.0011 win | loss / win | flagged (b) |
| `landy_opener_px` + `landy_opener_rungs` | `landy-opener-nsew`, 1792167042453704682, 4,608,000 bd, density 0.009017 | ns | 34,263 | +0.0072 ±0.0006 win | −0.0018 ±0.0006 loss | win / win | stands (PD blind to `px`; SD-PD tie-break wins) |
| | | ew | 74,766 | **−0.0090 ±0.0010 loss** | **−0.0236 ±0.0010 loss** | loss / loss | flagged (c) |
| `landy_doubler_px` (today's whole ladder vs the floor) | `landy-doubler-px-nsew`, 1792177143207773380, 4,608,000 bd, density 0.009015 | ns | 25,036 | +0.0068 ±0.0005 win | +0.0005 ±0.0001 win | non-inferior / non-inferior | stands |
| | | ew | 37,086 | +0.0272 ±0.0007 win | +0.0125 ±0.0004 win | win / win | stands |
| `nt_high_overcall_x_leave_in` | `nt-high-overcall-nsew` (Round 9), 1792184733335833470, 2,400,000 bd, density 0.007748 | ns | 4,945 | +0.0004 ±0.0005 non-inferior | +0.0035 ±0.0005 win | **loss / loss** | flagged (d) |
| | | ew | 5,474 | +0.0127 ±0.0006 win | +0.0151 ±0.0006 win | win / win | stands |
| `nt_high_overcall_x_major_at_four` (N3-fit, fresh seed) | `nt-high-overcall-n3fit` (Round 10), 1792187440017068589, 2,400,000 bd, density 0.00775 | none | 2,581 | +0.0008 ±0.0003 win | +0.0012 ±0.0003 win | non-inferior / non-inferior | replicated |
| | | both | 3,427 | +0.0018 ±0.0004 win | +0.0028 ±0.0005 win | non-inferior / win | replicated |

The four flagged readings, with a proposed reversible default for each (no
default changed; each is jdh8's decision):

- **(a) The 3NT pull veto at `ew`** (we are favourable). The veto removes
  our calls, so OFF is the arm that bids more, and from OFF's side the row
  is `win | loss`, the decision table's doubling artifact. Four of the five
  worst boards for the veto are OFF's pull of their `3NT` to `4♥` going
  down undoubled (`3NT 4♥ - - -`): plain DD credits the sacrifice because
  BBA did not double it. PD +13.3 IMPs per fired board. *Proposed:* keep
  on as a recorded trade-off, plain lower bound −0.0017.
  **Decided 2026-10-17 (jdh8):** kept on as a recorded trade-off.
- **(b) The natural-only push at `ew`.** A knob that doubles less, so plain
  DD arbitrates and the lower bound −0.0027 is past −δ. The worst boards
  are the exploit its ship section names: OFF's double of a cue bid or a
  keycard answer (`4NT - 5♦ X`, `5♣ - 5♦ X`) that makes BBA run from a
  making slam. It shipped on the BEN Tier-F guard for exactly that reason.
  *Proposed:* keep on, and run the BEN guard at `ew` (the 2026-10-12 guard
  read `none`/`both` only); flip off if BEN also reads a plain loss there.
  **Decided 2026-10-17 (jdh8):** kept on; the BEN guard at `ew` is owed.
  **Guard read 2026-10-18:** inconclusive (plain −0.0006 ±0.0023), so it
  stays on as the recorded trade-off ([below](#the-ben-guard-at-ew-and-the-live-record-at-closure-2026-10-18)).
- **(c) The Landy opener package at `ew`** (they are vulnerable, we are
  not): a loss on every scorer, −0.56 plain per fired board.
  `ab-dump-bucket --by node` puts every top bucket on the notrump rungs
  against a pass: `2NT vs -` −1.5…−3.9 plain per fired, `3NT vs -`
  −1.1…−4.2; the penalty `X` is not among them, and the remaining buckets
  net about +90k plain (inferred from the totals). At `ns` the same `3NT`
  rung is positive (+0.7…+2.6). When they are vulnerable, defending their
  undoubled `2M` beats our non-vulnerable notrump. *Proposed:* author a
  colour gate that drops both rungs when they are vulnerable and we are
  not; it is byte-identical at the other three cells by construction, so
  one `ew` A/B (the gated arm against today's default) decides it. Keep
  the package on until then: `px` alone was refused in 2026-09, and off
  gives back the `ns`/`none`/`both` gain.
  **Shipped 2026-10-17** (jdh8 approved the gate): `landy_opener_rungs_unless_favourable`,
  `ab-landy-opener.sh GATE=1` at `9de51c9f`, seed 1792221503542310373,
  4,608,000 accepted boards at `ew`, density 0.009021, preflight 0 divergent
  at none/ns/both (96,000 boards each), gate 0 foreign of 58,430 divergent:
  plain +0.0318 ±0.0009, PD +0.0520 ±0.0009, sd-plain +0.0117 ±0.0009,
  SD-PD +0.0316 ±0.0008, all wins.
- **(d) The N3 leave-in at `ns`** (we are unfavourable). By the runner's
  letter it stands (plain DD non-inferior). By measurement.md's addendum,
  which names this lane as its exemplar (plain arbitrates, sd-lead
  tie-breaks), it reads a loss: −2.56 plain / −1.27 PD per fired board on
  sd-lead. That is the v1 leave-in's sign flip again, at the cell the
  original none/both ship could not see. *Proposed:* author a colour gate
  (no leave-in when we are vulnerable and they are not), byte-identical
  elsewhere, and A/B it at `ns` alone. Keep on until then.
  **Shipped 2026-10-17** under Round 11's pre-registered rule (plain DD
  non-inferior or better, sd-lead plain not a loss; PD double-blind):
  `nt_high_overcall_x_leave_in_unless_unfavourable`, `ab-nt-high-overcall.sh
  ROUND=11` at `e092aa82`, seed 1792236493237230273, 2,400,000 boards at
  `ns`, density 0.00776, preflight 0 divergent at none/ew/both (96,000 boards
  each), gate 0 foreign of 4,837 divergent: plain −0.0002 ±0.0005
  (non-inferior), PD −0.0031 ±0.0005, sd-plain +0.0055 ±0.0005, SD-PD
  +0.0030 ±0.0005. The first build (`9de51c9f`) failed its preflight, 6–11
  divergent per 96,000 at the untouched cells, and was rebuilt on
  `except_at`.

### The BEN guard at `ew` and the live record at closure *(2026-10-18)*

**(b) closed 2026-10-18 by its pre-registered rule.** The BEN Tier-F guard
for `competitive_push_natural_only` at `ew` (`ab-ben-competitive-demotion.sh`
`VULS=ew ARMS=natural` at `451ce675`, `SEED_BASE` 1792163301883580764, 80
servers, 256,000 boards per arm, 838 fired) reads natural − control plain
−0.0006 ±0.0023 (inconclusive), PD +0.0016 ±0.0022 (non-inferior), sd-lead
plain −0.0007 ±0.0025 / SD-PD +0.0011 ±0.0024. That is the rule's third
branch: the knob stays on as the recorded trade-off decided 2026-10-17. The
result and its worst boards are in
[competitive-accountant.md](../ai-bidder/competitive-accountant.md).

The live §6 as it stood when it closed, moved here verbatim (links rebased):

**P1, tooling resolved and confirmations run 2026-10-16; all four decisions taken 2026-10-17; one BEN guard owed.** Competitive,
sacrifice and doubling decisions depend on favourable and unfavourable
vulnerability, and `ab-landy-nt-vs-x.sh` once flipped sign between `ns` and
`ew` ([evidence](#6-asymmetric-vulnerability-the-evidence-and-the-four-cell-runs)).
Done 2026-10-16: `ab-lib.sh` takes `VULS` (default `none both`) and the 40
runners with a plain `for v in none both` loop read it; measurement.md
checklist 8 now asks for all four cells on competitive, sacrifice and
doubling treatments and on any knob that reads `vulnerable()`, and for a
verdict to name its cells. First use: the colour-split
`lebensohl_signoff_raise` holds at `ns` and `ew` too
([its rows](#filter-scope-the-default-1nt-is-wider-than-the-scan)).

Confirmed at their asymmetric cells: `landy_notrump_no_major_favourable`,
`lebensohl_signoff_raise`, `landy_doubler_game` (`ew`), and, on 2026-10-16,
`landy_wilkosz`, `landy_splinter_tails` and `multi_doubler_major`.
`defense_2c_landy_strength_nv_invite` stays on as a recorded trade-off: it
wins every scorer at `both` and loses plain DD at `ns` (−0.0008 ±0.0003 per
accepted board). The two cells differ only in *their* colour, which the gate
does not read; a `both`-only gate is the untested refinement. Numbers in the
[archive](#6-asymmetric-vulnerability-the-evidence-and-the-four-cell-runs).

The confirm queue (`scripts/confirm-queue-2026-10-16.sh`, 2026-10-16,
[numbers](#6-asymmetric-vulnerability-the-evidence-and-the-four-cell-runs))
confirmed `their_2nt_double_veto`, `competitive_accountant`,
`competitive_pass_demotion` and `landy_doubler_px` at `ns` and `ew`, and
replicated N3-fit (`nt_high_overcall_x_major_at_four`) on a fresh seed.
The Landy opener package's `ew` loss is repaired: the colour gate
`landy_opener_rungs_unless_favourable` shipped default-on 2026-10-17, plain
+0.0318 ±0.0009 at `ew` over the ungated package. The N3 leave-in's `ns`
sd-lead loss is repaired the same way: `nt_high_overcall_x_leave_in_unless_unfavourable`
shipped default-on 2026-10-17, plain non-inferior (−0.0002 ±0.0005) and
sd-plain +0.0055 ±0.0005 at `ns` over the ungated leave-in.

- **Decided 2026-10-17 (jdh8): both stay on.**
  `their_3nt_pull_veto` is a recorded trade-off at `ew` (plain −0.0010
  ±0.0007, PD +0.0087): the off arm's gain is pulls of their `3NT` that BBA
  failed to double. `competitive_push_natural_only` stays on at `ew` (plain
  −0.0016 ±0.0011, PD +0.0028) pending its BEN guard.
- **Owed: the BEN Tier-F guard for `competitive_push_natural_only` at `ew`**
  (same `SEED_BASE` 1792163301883580764 as the four-cell run). Turn the knob
  off if BEN also reads a plain DD loss there. Launched 2026-10-17 at 256,000
  boards per arm; the three-way rule is pre-registered in
  [competitive-accountant.md](../ai-bidder/competitive-accountant.md).

### The last four doubling knobs *(2026-10-18)*

**Closed 2026-10-18: every knob decided, the three loss-side flips committed**
(`2720675b` trap-pass colour gate on, `eb394474` stack double off, `9a9e91a0`
values double off). Each decision is also recorded where the knob lives:
[bidding-options.md](../bidding-options.md) A3 for the runout and the escape
doubles, the `trap_pass_unless_unfavourable` doc comment in
`src/bidding/agreements.rs`, and
[evaluator-net.md](../ai-bidder/evaluator-net.md#against-the-shipped-floor) for
the accountant floor. The live §6 as it stood when it closed, moved here
verbatim (links rebased):

**P1, closed 2026-10-18.** `ab-lib.sh` takes `VULS`, the 40 runners with a
plain `for v in none both` loop read it, and measurement.md checklist 8 asks
for all four cells on competitive, sacrifice and doubling treatments and on
any knob that reads `vulnerable()`. Every knob the four-cell confirmations
flagged is decided. The Landy opener's notrump rungs and the N3 leave-in got
colour gates that shipped. `defense_2c_landy_strength_nv_invite`,
`their_3nt_pull_veto` and `competitive_push_natural_only` stay on as
recorded trade-offs. The last of these closed on its BEN Tier-F guard at
`ew`, plain −0.0006 ±0.0023 (inconclusive)
([competitive-accountant.md](../ai-bidder/competitive-accountant.md)). The
evidence, the runs and the record as it stood when it closed are
[archived](#6-asymmetric-vulnerability-the-evidence-and-the-four-cell-runs).

Since 2026-10-18 every runner that sources ab-lib loops over `$VULS`,
including the four `ab-ben-*` runners (`ab-landy-nt-vs-x.sh` deliberately
spells out its four cells).

The seven scripts that do not source ab-lib
(`ab-two-over-one{,-nofit-floor,-gate-sd,-gate-rescale}.sh`,
`sd-pd-{queue,nt-defense,readjudicate}.sh`) and the data-gen
`cachalot-xfix-gen.sh` read `${VULS:-none both}` too, defaults unchanged.

**Open: the last four doubling knobs (read 2026-10-18).** `penalty_pass`,
`trap_pass`, the doubled-1NT runout and `accountant_floor` had been measured
only by seat-swap harnesses (`ab-nt-defense-matrix`, `ab-lebensohl`,
`ab-one-nt-runout`, `ab-accountant-floor`). These put the treated pair N/S at
one table and E/W at the other under one absolute vulnerability, so `ns` is
unfavourable at one table and favourable at the other. Every board nets the
two, and these harnesses cannot measure a colour cell, although they accept
`--vulnerability ns`. `bba-gen` keeps us N/S against a fixed reference, so the
confirmation ran there: `scripts/ab-nt-doubling-4cell.sh` (SEED_BASE
1792323411437067982, `--filter-1nt --wide-1nt`, 2,400,000 boards per arm per
cell, density 0.2525) and `scripts/ab-accountant-floor-4cell.sh` (1,228,800
unfiltered). Rules were pre-registered in each header. All four knobs add
doubles or conversions, so plain DD arbitrates and PD is the double-blind
column. Figures are default − off, in IMPs per (filtered) board:

| Knob | Cell | Fired | Plain DD | PD | Decision (jdh8, 2026-10-18) |
| --- | --- | --- | --- | --- | --- |
| `accountant_floor` | `ns` | 20,013 | +0.0546 ±0.0024 | +0.0462 ±0.0026 | stands |
| `accountant_floor` | `ew` | 16,500 | +0.0347 ±0.0018 | +0.0315 ±0.0019 | stands |
| `penalty_pass` | both | 0 | — | — | inert vs BBA (its `2♣` is Landy), recorded as inert, not as confirmed |
| `trap_pass` | `ew` | 509 | +0.0008 ±0.0002 | +0.0013 ±0.0002 | stands |
| `trap_pass` | `ns` | 333 | −0.0005 ±0.0001 | −0.0001 ±0.0001 | colour gate built, confirmed, default on |
| `one_nt_runout` | `ns` | 1,093 | −0.0004 ±0.0002 | +0.0020 ±0.0003 | recorded trade-off, escape doubles isolated |
| `one_nt_runout` | `ew` | 1,101 | −0.0008 ±0.0002 | +0.0017 ±0.0002 | recorded trade-off, escape doubles isolated |

sd-lead agrees in sign on every row (trap pass `ns` sd-plain −0.0007, SD-PD
−0.0004; runout sd-plain −0.0010 / −0.0014 with SD-PD +0.0013 / +0.0010).

*Trap pass.* Split by lane over every divergent board (`ab-dump-bucket --by
lane`): the `(2M) X - -` conversion reads −415 IMPs at `ns` and +2,155 at
`ew`. Converting partner's takeout double of a white partscore collects
+300/+500, less than our red game. The `1NT (2x)` lane loses plain at both
cells (−696 / −223), on only 120–160 boards, so it is a lead, not a verdict.
`competition.trap_pass_unless_unfavourable` (built on the new
`constraint::waived_at`) waives the trap at unfavourable only. This run chose
it, so `scripts/ab-trap-pass-colour.sh` confirmed it on a fresh seed
(1792337712783260436, `ns`, 2,400,000 boards, 320 fired): gate − none, plain
DD +0.0005 ±0.0001, PD +0.0003 ±0.0001, sd-plain +0.0007, SD-PD +0.0005, all
wins, and 0 divergent boards at `none`, `both` and `ew` (96,000 each). Default
on since 2026-10-18.

*Runout: the recorded trade-off.* Accepted loss: plain DD lower bounds
−0.0006 (`ns`) and −0.0010 (`ew`) per filtered board; `none` and `both`
were not measured against BBA. Counted over every divergent board, the
two-level escapes win plain DD (`ns` +539, `ew` +352 IMPs). The loss is
opener leaving in our later double of their escape:

| Our double of their escape | `ns` | `ew` |
| --- | --- | --- |
| after `1NT (X) XX`: values double, opener sits | 334 bds, −1,290 | 319 bds, −1,082 |
| after `1NT (X) -`: the floor's takeout double, opener sits | 375 bds, −508 | 437 bds, −1,282 |
| after `1NT (X) -`: the authored stack double, opener sits | 3 bds, +31 | 4 bds, +46 |

**Discrepancy, flagged.** The leave-in (`leave_in_escape_penalty`) sits for any
double after the escape. In the unredoubled lane the book authors only the
stack double, which almost never fires, so opener is leaving in the floor's
takeout doubles as penalty: a call whose meaning and reading disagree. Since
2026-10-18 the leave-in follows the knob that authored the double. The default
is byte-identical (re-dealt 25,000 boards at `ns` and `ew`), so it still
sits. `scripts/ab-escape-double-4cell.sh` (seed 1792339065938270446,
2,400,000 boards per arm per cell) measured each knob, with its leave-in, at
all four cells, default − off:

| Knob | Cell | Fired | Plain DD | PD | sd-plain | SD-PD |
| --- | --- | --- | --- | --- | --- | --- |
| `penalize_escape_stack` | `none` | 463 | −0.0006 ±0.0001 | +0.0009 ±0.0001 | −0.0008 | +0.0006 |
| `penalize_escape_stack` | `both` | 382 | −0.0004 ±0.0002 | +0.0011 ±0.0002 | −0.0008 | +0.0008 |
| `penalize_escape_stack` | `ns` | 396 | −0.0003 ±0.0001 | +0.0011 ±0.0002 | −0.0006 | +0.0008 |
| `penalize_escape_stack` | `ew` | 470 | −0.0007 ±0.0002 | +0.0008 ±0.0002 | −0.0010 | +0.0007 |
| `penalize_escape_values` | all four | 0 | — | — | — | — |
| `one_nt_runout` | `none` | 1,074 | −0.0010 ±0.0002 | +0.0014 ±0.0002 | −0.0015 | +0.0008 |
| `one_nt_runout` | `both` | 1,099 | −0.0007 ±0.0003 | +0.0021 ±0.0003 | −0.0014 | +0.0014 |

*Stack: default off (jdh8, 2026-10-18).* A plain-DD loss in every cell, PD
the double-blind column; off is a plain win and PD non-inferior (worst lower
bound −0.0013 > −δ). The worst boards are all `1NT X - 2x - 2y X - - -`
against our 3NT or 4M: opener sitting for the floor's takeout double.

*Values: not measured.* The leave-in is `stack || (XX && values)` and the
stack rule fires after the redouble too, so with the stack on the values knob
was shadowed: the floor doubled anyway and opener still sat. With the stack
off it is live: `scripts/ab-escape-values-lane.sh` measured it against
`nostack` on the same deals (on = `nostack`, off = both knobs off):

| Cell | Fired | Plain DD | PD |
| --- | --- | --- | --- |
| `none` | 302 | −0.0006 ±0.0001 | −0.0000 ±0.0001 |
| `both` | 324 | −0.0006 ±0.0002 | +0.0002 ±0.0001 |
| `ns` | 320 | −0.0006 ±0.0001 | +0.0000 ±0.0001 |
| `ew` | 304 | −0.0005 ±0.0001 | +0.0001 ±0.0001 |

A plain-DD loss in every cell, PD a wash; the worst boards are again opener
sitting for the double (`1NT X XX 2♥ - 2♠ X - - -`) against our 3NT.
*Values: default off (jdh8, 2026-10-18).* Off is a plain win and PD
non-inferior.

*Runout at `none`/`both`.* The same trade-off as `ns`/`ew`: plain DD loses
in all four cells. Kept on (jdh8, 2026-10-18).



## 7 Auction conditioning: the documentation pass

Documentation done 2026-10-16: every current doc and doc comment that called
replay "the self-play posterior", "worlds the bidder reproduces" or
"auction-consistent worlds" now reads relaxed authored-rule conditioning
(within `MARGIN` of the best legal logit, abstaining off-book), or range-only
conditioning for the range path. exact-posterior.md states the `1M - 2M -`
range failure as the non-default path's and flags the counted-reading
re-run's 91.2% deal overlap. A stale "default off" for `set_rule_accept` in
sampled-projection.md was corrected (default on since `74d783d`).

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
carry an `.inputs` stamp and are rescored when it changes. **BEN and the
remaining runners (`bf8fe233`):** `ben-gen` writes atomically and answers
`--verify`/`--provenance`; `ben-gen-parallel.sh` waits on every worker,
refuses a fleet whose size differs from `SHARDS`, clears stale shards and
verifies; `ab-lib.sh` takes `GENERATOR=ben-gen`, so the `ab-ben-*`,
binary-arm, weak-two-balancing (now honouring `JOBS`) and split-by-opening
runners resume on the manifest and shard check; `ab-reading-knobs` keeps its
layout with the same checks. `load_dump` rejects shard dirs whose `gen_args`
differ beyond seed/output/count/port/start-board/note (none of 2,723 scanned
here and on dl02 did). Results dirs without `arm.meta` now refuse to resume
without `FORCE_REUSE=1`.

**Closed 2026-10-18: the BEN servers.** The open gap was that the servers
stayed outside the fingerprint (the runners' `--note` hashed the conf on disk,
not what the fleet loaded, and nothing covered BEN's commit, its patches or
its interpreter), that `load_dump` ignored `--note` so a dir mixing two
runtimes loaded, and that the tier was a label only. Now
`scripts/ben-servers.sh start` writes a deterministic
`$BEN_DIR/run/servers.meta` once every port answers (BEN's describe and
commit, a sha256 of `git diff --binary HEAD` and of each untracked file, the
conf, the interpreter and its package set, the server flags, tier and fleet
size), adopts an already-running port only when that manifest matches, and
`stop` removes it. `ab-lib.sh` cksums it into every ben-gen arm's fingerprint
(no manifest, no arm) and copies it beside the arm; the four `ab-ben-*`
runners pass `--note "$(ben_note)"`, the manifest's sha256;
`ben-gen-parallel.sh` refuses a missing manifest, a `-t` other than its tier,
or a running fleet of another size. `shard_config` keeps `--note` as
configuration less its `assembled-from=` tokens: of the 46 noted shard dirs
here and on dl02, the only one that now refuses is
`ben-anchor/2026-09-13-daa8bf4a/both`, whose two recovered shards ran the
PASS-level patch and six did not (recorded in
[ben-gap-campaign.md](../ben-gap-campaign.md), superseded by the 2026-10-11
anchor, dl02 only). Covered by `scripts/test-arm-fingerprint.sh` (86 cases)
and `examples/common/tests/dumps.rs`. Residual: the manifest attests the BEN
tree at launch, so editing `~/ben` while the fleet runs is invisible to it.

The original finding, for the record:
[ab-lib.sh](../../scripts/ab-lib.sh), `arm`, considers an arm complete by shard
count; `diffpair` skips when two report files are nonempty. A guard pins
JOBS and BOARDS per results directory, but reusing it after changing flags or
HEAD can still reuse an older run. `bba-gen` writes each shard straight to its
final path, so a crashed worker leaves a truncated shard that the count accepts,
and `bba-gen-parallel.sh` ends with a bare `wait` that discards exit codes.
[common::load_dump](../../examples/common/mod.rs) appends later shards without
checking their labels, vulnerability or configuration. Dumps carry seed/args
but not a full immutable code, weights and scorer identity.

- Save a manifest with source SHA, dirty diff identity, weights hash,
  generator/scorer/solver versions, configuration and expected shard IDs/counts.
- Resume only on matching manifests; validate every shard and report.
- Collect each worker's exit status and verify expected board counts, not
  just matching lengths between two incomplete arms.
