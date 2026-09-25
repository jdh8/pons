#!/bin/sh
# ab-3nt-unusual-veto.sh — the unusual-4NT arm of the 3NT rail
# (`InstinctProfile::their_3nt_unusual_veto`) against plain american, vs BBA.
#
#   plain   american with the arm off (--no-ns-3nt-unusual-veto)   (control)
#   on      american as shipped since 2026-09-25: our silent side's 4NT over
#           their 3NT needs two five-card suits
#
# Found by the 2026-09-20 shipping-arm decompose (c3bb94a7): the v6 floor
# bids 4NT over their 3NT with junk (85 boards, -939 plain / -1,194 PD IMPs
# over 409.6k, BBA passed every one); `1♣ - 1♠ - 2NT - 3NT` is its biggest
# lane.  Only fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-3nt-unusual-veto.sh ab-results/3nt-unusual-veto \
#       >ab-results/3nt-unusual-veto.log 2>&1 &
R=${1:?usage: ab-3nt-unusual-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 3nt unusual veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american --no-ns-3nt-unusual-veto
    arm on    "$vul" --our-floor american
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== 3nt unusual veto done"
