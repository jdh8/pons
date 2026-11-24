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
#        (`--no-ns-…` since the READ of 2026-11-25)
#   on   notrump.rebid_checkback_shape_keycard on: support_points 13+ on the
#        5-3 fit, 12+ after the forcing `3M` is raised (the default since
#        that READ)
#
# Not in this arm: the 4-4 fit in the other major (`3m - 3oM - 4oM`, two rows;
# `4NT` there is the quantitative raise), the doubled checkback (owed since
# 2026-09-30), the `1♥ - 1♠ - 2NT` lane (no checkback).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-24 (878fa705, SEED_BASE 1795473254996592931, 1,638,400 boards
# per cell, vs BBA): WIN 4/4 on plain, PD, plain SD and SD-PD; the isolation
# gate FAILED in every cell on 1-2 boards.
#
#   cell  fired  plain  PD     IMPs/board (plain, 95% CI)  SD plain / SD-PD   foreign
#   none  439    +1128  +1075  +0.0007 ± 0.0002            +1498 / +1455      1
#   ns    478    +1438  +1351  +0.0009 ± 0.0003            +2115 / +2062      1
#   ew    432    +1130  +1070  +0.0007 ± 0.0002            +1685 / +1643      2
#   both  472    +1432  +1345  +0.0009 ± 0.0003            +2053 / +1994      2
#
# +2.6 (we are non-vul) to +3.0 (we are vul) IMPs per fired board, 1 in
# ~3,600 boards.  One seed, four vulnerabilities.  The foreign boards are
# THEIR `1♣ - 1♠ - 2NT - 3♠ - 4♠ - 4NT - 5♣`: with the knob on we double the
# `5♣` answer and they stop in `5♠` instead of `6♠`.  Traced 2026-11-24, a
# harness leak, not an engine one: `common::european_minors_opponents` builds
# the book we read EPBot's alerted calls off from a copy of the arm's own
# agreements, so the knob re-read THEIR alerted `4NT` on support points.  The
# copy now resets this knob (the third such reset there); 6 boards of 1,821
# cannot carry the verdict.  A one-cell gate re-check on the same seed is
# `ab-results/checkback-gate-recheck`.  The knob is still default off: the
# flip awaits jdh8.
#
# READ 2026-11-25, the gate re-check (948751b0, the same SEED_BASE, VULS=ew,
# 1,638,400 boards, vs BBA): isolation gate PASSED, 0 foreign of 430
# divergent boards (was 2), so the reset holds.  WIN on all four scorers:
#
#   cell  fired  plain  PD     IMPs/board (plain, 95% CI)  SD plain / SD-PD   foreign
#   ew    430    +1117  +1064  +0.0007 ± 0.0002            +1568 / +1536      0
#
# The first read's ew cell was +1130 plain on 432 fired, so the leak carried
# nothing.  One cell re-run, not four: none, ns and both keep the first
# read's numbers.
# Shipped default-on on this READ (jdh8, 2026-11-25); the arms below now turn
# the base off instead of the candidate on.
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
    arm base "$vul" --no-ns-rebid-checkback-shape-keycard
    arm on "$vul"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-rebid-checkback-shape-keycard
done
log "=== rebid-checkback-shape-keycard A/B done"
