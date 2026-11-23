#!/bin/sh
# ab-rebid-checkback-shape-keycard.sh — the `1m - 1M - 2NT` checkback's keycard gates count support points, not HCP,
# vs BBA, four cells, plain DD + perfect defense from one solve, plus SD-PD.
# Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh rebid-checkback-shape-keycard \
#       scripts/ab-rebid-checkback-shape-keycard.sh ab-results/rebid-checkback-shape-keycard
#
# Why.  On the BBA anchor a0977ce9 (docs/bba-gap-campaign.md, round-2 cut,
# lead 3b) responder signs off in `4♠` after `1♦ - 1♠ - 2NT - 3♣ - 3♠` or
# `... 3♠ - 4♠` where BBA reaches slam: 37 rows per 409,600 boards, -269
# plain.  The hands are shapely (`Q9753.AQT4.KQ5.8`, `QT9752.AQJ87..J2`): the
# two keycard gates read raw HCP (14+ on the 5-3 fit, 12+ on the 6-2) though
# the fit is known.  The `.pdd` census (examples/probe-drury-slam.rs
# --checkback, ~100M deals; opener balanced 18-19) prices the ask against
# `4M` by responder's support points, IMPs per case, not vul:
#
#   responder sp   11      12      13      14      15
#   5-3 fit        -3.41   -0.64   +2.39   +5.13   +7.22
#   6-2 or better  -1.25   +1.48   +4.18   +6.32   +7.95
#
#   base notrump.rebid_checkback_shape_keycard off: hcp(14..) / hcp(12..)
#   on   notrump.rebid_checkback_shape_keycard on: support_points 13+ on the
#        5-3 fit, 12+ after the forcing `3M` is raised
#
# Not in this arm: the 4-4 fit in the other major (`3m - 3oM - 4oM`, two rows;
# `4NT` there is the quantitative raise), the doubled checkback (owed since
# 2026-09-30), the `1♥ - 1♠ - 2NT` lane (no checkback).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/rebid-checkback-shape-keycard.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-rebid-checkback-shape-keycard.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for rebid-checkback-shape-keycard)

log "=== rebid-checkback-shape-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-rebid-checkback-shape-keycard
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --on-ns-rebid-checkback-shape-keycard
done
log "=== rebid-checkback-shape-keycard A/B done"
