#!/bin/sh
# ab-two-over-one-hearts-first.sh — the major 2/1's suit order
# (`response.two_over_one_hearts_first`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-hearts-first.sh ab-results/two-over-one-hearts \
#       >ab-results/two-over-one-hearts.log 2>&1 < /dev/null &
#
# What is being tested.  The 2/1s race on weight, clubs before diamonds before
# hearts, so `1♠ - 2♣` is bid on 2=5=2=4 and `1♠ - 2♦` on 1=6=4=2.  BBA bids
# the longest suit first, a four-card tie up the line, a five-card tie the
# higher.  Anchor `46d0dc14` vs BBA, per 409,600 boards: the suit-choice call
# pairs over `1♥` / `1♠` (ours and BBA's both a 2/1 or `1♠`) 1,738 rows, −3,212
# plain / −2,742 PD; `2♣`/`2♦` where BBA bids `2♥` 767 rows, −1,568 / −1,294.
#
#   off  the weight ladder
#   on   --ns-two-over-one-hearts-first
#
# First build, BBA's whole order over both majors (seed 1791188672, sha
# 67fcc74c-dirty, 204,800 boards/arm/vul, `ab-results/two-over-one-longest`):
# a wash, plain +0.0002 / −0.0008, PD +0.0003 / −0.0008 IMPs/board (none /
# both, CI ±0.0015–0.0019), 321 / 335 fired.  Split by the changed 2/1, plain
# pooled over both vuls: `2♥` for a minor over `1♠` +229 IMPs on 244 boards;
# `2♦` for `2♣` (5-5 minors, longer diamonds) −393 on 210, its slams in `6NT`
# / `7NT` where `2♣` found the diamond fit; the rest is reading drift on ~100
# boards whose 2/1 did not change.  This arm keeps the heart half only.
#
# MEASURED 2026-10-05, a wash, stays off (sha 67fcc74c-dirty, 204,800
# boards/arm/vul, isolation gate passed).  IMPs/board none / both:
#
#   seed 1 (1791189650)  plain +0.0001 / +0.0002, PD +0.0002 / +0.0002, 125 / 129 fired
#   seed 2 (1791190176)  plain +0.0001 / −0.0002, PD +0.0001 / −0.0002, 148 / 152 fired
#   pooled               plain +0.0001 / −0.00002, PD +0.00015 / +0.00001
#
# Seed 2 wants `ab-results/two-over-one-hearts-2` (`seed_for` reuses the seed
# it finds in a results dir).
#
# ROUND 3, 2026-11-25, none cell only (sha b212eea9, the frozen 45b3f565 tree;
# seed 1795637731189018879; 1,638,400 bd/arm).  The isolation gate FAILED on
# none (76 of 1,068 divergent boards opened by them, their `1♠ - 2x` auctions,
# though the mirror pins the knob off) and `gatepair` is fatal here, so the
# run stopped before scoring and before ns / ew / both.  Scored afterwards by
# hand: plain +0.0000 (+75 IMPs), PD +0.0000 (+59), CI +-0.0003, 1,068
# fired; non-inferior both scorers.  By opener: ours 992 bd +73 / +74,
# theirs 76 bd +2 / -15.  A wash a third time, on one cell.
#
# ROUND 3 COMPLETED 2026-11-27 (pueue 38, restarted with `gatepair`
# non-fatal; none reused, ns / ew / both on the same seed).  A WASH 4/4,
# stays off.  IMPs none / ns / ew / both, CI +-0.0003..0.0004 per board:
#
#   plain  +75 / +204 / +154 / +131  (non-inferior 4/4, +0.0000..+0.0001)
#   PD     +59 / +232 / +168 / +139  (non-inferior 4/4)
#   fired  1,068 / 1,073 / 1,020 / 1,047 (0.06-0.07%)
#
# The gate failed in every cell: 76 / 39 / 28 / 31 boards opened by THEM.
# Split by opener (`ab-dump-diff --jsonl`), plain / PD:
#   ours    +73 / +74, +177 / +197, +203 / +218, +180 / +193
#   theirs   +2 / -15,  +27 /  +35,  -49 /  -50,  -49 /  -54
# Root cause: `common::european_minors_opponents` builds the declared EPBot
# book from the ARM's agreements, so the on arm reads EPBot's `1♠ - 2♥` /
# `2♦` under this knob and our defensive calls move (the mirror pin does not
# apply: a declared opponent bypasses the mirror).  Same class as lead 3b's
# leak; pinned off there 2026-11-27, which closes it.  Worth under 0.00004
# IMPs/board a cell, so the verdict stands.
#
# Lead 3 CLOSED 2026-11-27 (jdh8): three washes, worth ~0; stays opt-in.
R=${1:?usage: ab-two-over-one-hearts-first.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-over-one-hearts)

log "=== two-over-one-hearts A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm off "$vul"
    arm on "$vul" --ns-two-over-one-hearts-first
    gatepair on off "$vul" || log "isolation gate on vs off ($vul) FAILED, scoring anyway"
    diffpair on off "$vul"
done
log "=== two-over-one-hearts A/B done"
