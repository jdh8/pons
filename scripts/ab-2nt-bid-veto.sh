#!/bin/sh
# ab-2nt-bid-veto.sh — the bidding sibling of the 2NT-double rail
# (`InstinctProfile::their_2nt_bid_veto`) against plain american, vs BBA.
#
#   plain   american with the rail off (--no-ns-2nt-bid-veto)       (control)
#   on      american as shipped since 2026-09-26: our silent side's bids over
#           their 2NT opening auction are masked on 7 HCP or fewer
#
# Found by the floor-rail series' R6 re-census (anchor 7e0bc648): over
# `2NT - 3♣ - 3♦` the v6 floor bids 3♥, over `2NT - 3NT` 4♣, on 0-7 HCP
# (201 boards, -976 plain / -1,250 PD IMPs over 409.6k; BBA passed 201 of
# 213).  Only fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2nt-bid-veto.sh ab-results/2nt-bid-veto \
#       >ab-results/2nt-bid-veto.log 2>&1 &
R=${1:?usage: ab-2nt-bid-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 2nt bid veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american --no-ns-2nt-bid-veto
    arm on    "$vul" --our-floor american
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== 2nt bid veto done"
