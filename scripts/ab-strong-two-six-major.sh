#!/bin/sh
# ab-strong-two-six-major.sh — open the strong `2♣` on 20+ HCP with a six-card
# major (`opening.strong_two_six_major`), vs BBA, four cells, plain DD +
# perfect defense from one solve.  Arms are sequential and share one
# persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh strong-two-six-major \
#       scripts/ab-strong-two-six-major.sh ab-results/strong-two-six-major
#
# Why.  On the BBA anchor a0977ce9, cut by auction (scripts/cut-auction.py),
# our `1M` where BBA opens `2♣` is 954 rows per 409,600 boards, -1,429 plain
# / -1,141 PD.  By HCP and the two longest suits, the one dense cell is 20-21
# HCP with a six- or seven-card major and no second suit (6-3, 7-2, 7-3): 366
# rows, -661 plain.  With a second suit of four or more the same counts are a wash
# (135 rows, +3), and 17-19 HCP is scattered freaks.  The worst boards are
# `1M` passed out and `1M - 1NT - 3M` under BBA's slam.  A first-call pair,
# so the figure prices BBA's continuations with its call.
#
#   base the book: `2♣` on 22+ points
#   on   --ns-strong-two-six-major 20: also on 20+ HCP with 6+ in a major
#
# Not in this arm: any continuation.  The strong-`2♣` lane's answers still
# count opener for 22, so a loss may be responder's slam drives on a 20;
# bucket by responder's first rebid before reading it as the opening.  The
# arm also takes the 6-4 and 6-5 hands the census reads as a wash.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# MEASURED 2026-11-25, a WASH (non-inferior 4/4 on both scorers, no win),
# stays off (sha f786856c, the frozen 45b3f565 tree; seed 1795618433883867323;
# 1,638,400 bd/cell).  IMPs/board none / ns / ew / both, CI +-0.0004..0.0005:
#
#   plain  -0.0001 / +0.0002 / -0.0001 / +0.0002
#   PD     -0.0003 / -0.0001 / -0.0002 / -0.0001
#   fired  1,398 / 1,447 / 1,402 / 1,440 (0.09%)
#
# The isolation gate FAILED in every cell: 119 / 85 / 107 / 72 divergent boards
# opened by THEM (their `2♣`; our defensive call moves), though the mirror pins
# this knob off (`common::mirror_agreements`).  Split by opener
# (`ab-dump-diff --jsonl`), plain / PD IMPs:
#   ours    -206 / -474, +161 / -293, -154 / -378, +238 / -190
#   theirs    +2 /   +2, +138 / +119,  +11 /   -6,  +37 /  +58
# The foreign boards are worth nothing, so the verdict stands; the leak is
# flagged (same pattern as two-over-one-hearts r3) and not root-caused.
#
# Resumable; SEED_BASE persists in $R/strong-two-six-major.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-strong-two-six-major.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-two-six-major)

log "=== strong-two-six-major A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-strong-two-six-major 20
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
done
log "=== strong-two-six-major A/B done"
