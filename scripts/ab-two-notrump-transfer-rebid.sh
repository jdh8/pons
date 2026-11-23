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
#   on   notrump.two_notrump_transfer_rebid on: five trumps bid `3NT` 5-10,
#        quantitative `4NT` 11, `6NT` 12-16 (the direct table's bands; 17+
#        stays with the floor); six bid `4M`, or `6M` on 11+ support points;
#        opener answers the `4NT` as it answers the direct one.  The 22-24
#        `2♣` lanes share it, two points lower.
#
# Not in this arm: a keycard ask for the six-card hand (`4NT` is the
# quantitative raise; opener cannot tell them apart), opener's choice over
# `3NT`, second suits, the interfered tails (all natural, left to the floor),
# and the `2NT` opening's reading floor (`hcp 0..=21`, flagged).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
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
