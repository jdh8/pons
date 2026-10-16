#!/bin/sh
# ab-modern-negx-disclosed.sh — re-measure the shipped Modern negative double
# with forcing free bids and opener's answers (`competition.
# negative_double_shape = Modern`) against the pre-Modern `both-majors`
# double under the fixed SD leader (docs/research-method-review.md § 4;
# docs/competitive-book.md ledger row P3b″′; the original runner is
# scripts/ab-free-bid-answers.sh).
#
#   JOBS=96 BOARDS=393216 BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       DDSS_THREADS=96 RAYON_NUM_THREADS=96 VULS="none ns ew both" \
#       setsid nohup scripts/idle-run.sh scripts/ab-modern-negx-disclosed.sh \
#       ab-results/modern-negx-disclosed \
#       >ab-results/modern-negx-disclosed.log 2>&1 < /dev/null &
#
# Why.  Modern shipped (2026-07) on plain DD +0.0213 NV / +0.0074 vul (CI>0)
# over a vul-PD −0.0256 (CI<0), the PD loss "overruled" by a
# disclosure-corrected sd-lead row.  The 2026-07-25 SD-PD rescore already
# flipped vul (−0.0075 ±0.0057), but every SD number so far came from the
# pre-7c60e309 leader, which read only our *alerted* calls when we declared:
# the forcing free bids and opener's answers are unalerted and were never
# disclosed.  Fresh deals, current code, the disclosed reader.
#
# Arms (one SEED_BASE, random deals — the lane fires on ~1–2% of boards):
#   shipped  bare (Modern + forcing free bids + answers, the default)
#   bothmaj  `--ns-negative-double-shape both-majors` (no free bids)
# ON = shipped, OFF = bothmaj; the leader reads OFF with
# `--off-ns-negative-double-shape both-majors`.
#
# PRE-REGISTERED 2026-10-16, before any number is read:
#   * Sizes: BOARDS=393216 boards/arm/vul (96 shards × 4096), four cells
#     VULS="none ns ew both" (we sit N/S: `ns` = we vul, `ew` = we NV).  One
#     seed; no extension.  Unfiltered, so no density.
#   * Gate: `probe-divergence --gate-opener ours`, 0 foreign in every cell.
#   * Decision: the decision-table row from the worst cell of plain DD and of
#     PD.  Modern stands on `non-inferior|win` or better.  A `win|loss` (or
#     plain win, PD inconclusive) is the doubling-artifact row: the default is
#     then not supported by the table and is flagged for jdh8 as a recorded
#     trade-off — proposed reversible default: keep Modern (both-majors loses
#     plain DD), with the PD loss it accepts named.  A plain-DD loss in any
#     cell is flagged with the same default and the cell named.
#   * The July rationale ("the vul-PD loss is the perfect-defense doubling
#     artifact, overruled by the sd arbiter") stands iff the SD-PD Verdict is
#     `non-inferior` or `win` in every cell where PD is `loss` or
#     `inconclusive`.  Otherwise the 07-25 rejection is confirmed under the
#     disclosed reader.
#
# AMENDED 2026-10-16 after the first cell's gate FAILED, before any score was
# read: 1635 of 12,361 divergent boards at `none` are opened by THEM, every
# shown one `1m (1M) X` with OUR advance differing (`2♥` vs Pass).  The shape
# knob also sets how we read *their* negative double, so it is a whole-system
# setting, not an own-auction package, and the gate cannot read 0 foreign.
# The original ship (ab-free-bid-answers.sh) had no gate and measured the
# whole system, which is also what the default deploys.  So: the gate is
# logged, not fatal; the verdict reads the full population as pre-registered
# above; and `ab-dump-bucket --by side` (who opened × who differed first)
# reports the owned and foreign slices beside it.  The reading leak itself
# is flagged for jdh8.
#
# VERDICT (2026-10-16, code 32dc6fd4 + this commit's ab-dump-sd flag, SEED_BASE
# 1792113860641334667, 393,216 boards/arm/cell, unfiltered, leader reading
# disclosed; gate 1524–1635 foreign of ~12,300 divergent per cell, see the
# amendment).  shipped − bothmaj, IMPs/board ±95% CI (~3.1% fired):
#           plain DD             PD                    sd-lead          SD-PD
#   none  +0.0313 ±0.0029 win  +0.0001 ±0.0033 inc   +0.0419 ±0.0030  +0.0153 ±0.0033 win
#   ns    +0.0349 ±0.0035 win  +0.0018 ±0.0040 inc   +0.0492 ±0.0036  +0.0210 ±0.0040 win
#   ew    +0.0202 ±0.0033 win  −0.0194 ±0.0040 loss  +0.0322 ±0.0034  −0.0032 ±0.0039 inc
#   both  +0.0231 ±0.0036 win  −0.0188 ±0.0043 loss  +0.0398 ±0.0038  +0.0028 ±0.0042 NI
# `--by side` (plain / PD IMPs): we open — none +10281/−868, ns +11678/+224,
#   ew +6195/−8086, both +7389/−7528; they open (the reading leak) — none
#   +2030/+894, ns +2039/+469, ew +1733/+475, both +1711/+129.
# Decision row (worst cell): plain `win` | PD `loss` — the doubling-artifact
# row, below ship.  The PD loss sits where THEY are vulnerable (ew, both),
# not where we are (ns is inconclusive, +0.0018): it tracks the value of
# PD's synthetic double of their failing contracts, which the both-majors
# arm defends more often, not "thin vul games" of ours.  July rationale
# (SD-PD covers every PD loss/inconclusive cell): holds at none, ns and both
# (both's 07-25 SD-PD sign flip, −0.0075, is now +0.0028 NI); fails at ew
# (SD-PD inconclusive).  FLAGGED for jdh8, nothing changed.  Proposed
# reversible default: keep Modern as a recorded trade-off (both-majors loses
# plain DD in every cell; the accepted loss is PD's ew/both lower bounds,
# −0.0234 / −0.0231 IMPs/board).  Also flagged: the shape knob changes our
# reading of THEIR negative double (13% of divergent boards; positive on
# both scorers in every cell).
#
# Resumable; SEED_BASE persists in $R/modern-negx-disclosed.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-modern-negx-disclosed.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for modern-negx-disclosed)
log "=== modern-negx-disclosed SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul vuls=$VULS"

for v in $VULS; do
    arm shipped "$v"
    arm bothmaj "$v" --ns-negative-double-shape both-majors
    gatepair shipped bothmaj "$v" ours || log "gate FAILED $v (foreign boards: see the amendment above)"
    diffpair shipped bothmaj "$v"
    sddiff shipped bothmaj "$v" --off-ns-negative-double-shape both-majors
done

log "modern-negx-disclosed done"
