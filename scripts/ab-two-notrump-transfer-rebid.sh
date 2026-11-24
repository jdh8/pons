#!/bin/sh
# ab-two-notrump-transfer-rebid.sh — responder's rebid over the plain `3M` completion of a `2NT` transfer is authored,
# vs BBA, four cells, plain DD + perfect defense from one solve, plus SD-PD.
# Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh two-notrump-transfer-rebid \
#       scripts/ab-two-notrump-transfer-rebid.sh ab-results/two-notrump-transfer-rebid
#
# Why.  The one losing bucket of the super-accept A/B
# (scripts/ab-two-notrump-super-accept.sh, round 3): "3M completion, slam",
# -117 to -155 plain IMPs per cell on 137-150 boards.  Traced 2026-11-24: the
# seat `2NT - 3♦ - 3♥ -` has no book node, so the floor bids, and its slam
# milestones there are the evaluator net alone (accountant floor, no collar).
# With the `4M` jump beside it the `3M` reads 2-3 trumps, not 2-5, and the
# net tips about a point earlier: `3NT -> 6NT` on five-card 10-counts (76
# boards, 46% make, 30.2 combined HCP) and `4♥ -> 6♥` on six trumps and 3-9
# HCP (24 boards, 25% make).  The seat overbids in the super-accept's off arm
# too (5♥ and 10 HCP: `6NT` on 45 of 110), so the table is measured against
# the floor, not against the old reading.
#
#   base notrump.two_notrump_transfer_rebid off: the floor bids the seat
#   on   notrump.two_notrump_transfer_rebid on.  Round 2 (the code today):
#        five trumps bid `3NT` 4-9, quantitative `4NT` 10, `6NT` 11-13 (the
#        direct table's bands one point lower; 14+ stays with the floor); six
#        bid `4M`, or `6M` on 11+ support points; opener passes the `4M` and
#        answers the `4NT` as it answers the direct one.  The 22-24 `2♣`
#        lanes share it, two points lower.  Round 1 sat on the direct bands
#        (`3NT` 5-10, `4NT` 11, `6NT` 12-16) and left opener's seat over `4M`
#        to the floor; see its READ below.
#
# Not in this arm: a keycard ask for the six-card hand (`4NT` is the
# quantitative raise; opener cannot tell them apart), opener's choice over
# `3NT`, second suits, the interfered tails (all natural, left to the floor),
# and the `2NT` opening's reading floor (`hcp 0..=21`, flagged).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-24 (24eab2cb, SEED_BASE 1795486471760789996, 1,638,400 boards
# per cell, vs BBA): LOSS 4/4 on plain, PD, plain SD and SD-PD.  Not dead:
# the bands sit a point too high and one continuation is unauthored.
#
#   cell  fired  plain  PD    IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   none  532    -713   -529  -0.0004 ± 0.0003            -1385 / -1262
#   ns    594    -1184  -863  -0.0007 ± 0.0003            -2415 / -2209
#   ew    529    -679   -495  -0.0004 ± 0.0003            -1559 / -1430
#   both  588    -1154  -837  -0.0007 ± 0.0003            -2268 / -2044
#
# Isolation gate passed in every cell.  SD fell back on 13-14% of the on
# arm's boards against 4% of the base arm's, so SD is not arm-symmetric.
# Trace (`ab-dump-diff --jsonl`, none + both pooled, 1,120 boards, -1,867
# plain), by the first differing call and that hand's HCP:
#
#   floor -> table              hand                  n    plain  /board
#   6NT -> 4NT (resp)           11, five trumps       142  -484   -3.41
#   6NT -> 3NT (resp)           10, five              129  -307   -2.38
#   6NT -> 3NT (resp)            9, five               53  +148   +2.79
#   7NT -> 6NT (resp)           14-16, five            46  -468   -10.2
#   7NT -> 6NT (resp)           12-13, five            41  +197   +4.80
#   3NT -> pass of 3M (resp)     4, five              295  -211   -0.72
#   pass -> 6♥ over 4♥ (opener)  20-21                 22  -218   -9.91
#   7♠ -> 6♠ (resp)             six or more            40  -207   -5.18
#
# So the floor's slams are right one point lower than the direct table's
# bands (the fifth trump is worth the point): `6NT` from 11, an invite on 10,
# a grand from 14, pass of `3M` only below 4.  Opener's `6♥` over the `4♥`
# sign-off is the floor bidding an unauthored seat.  The five losing rows
# those fixes address (all but the six-card `7♠ -> 6♠`) are -1,688 of the
# -1,867.  Each row is conditional on the floor's own choice (the 14-16 row
# is only the hands where it picked `7NT`), so round 2 hands 14+ back to the
# floor instead of bidding a blanket grand.  Round 2 = those bands plus
# opener's pass over `4M`, results in ab-results/two-notrump-transfer-rebid-r2
# (a fresh SEED_BASE); the knob stays default off.
#
# Resumable; SEED_BASE persists in $R/two-notrump-transfer-rebid.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-notrump-transfer-rebid.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-notrump-transfer-rebid)

log "=== two-notrump-transfer-rebid A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-two-notrump-transfer-rebid
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --on-ns-two-notrump-transfer-rebid
done
log "=== two-notrump-transfer-rebid A/B done"
