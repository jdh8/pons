#!/bin/sh
# ab-2nt-unusual-veto.sh — the unusual-4NT rail over their 2NT
# (`InstinctProfile::their_2nt_unusual_veto`) against plain american, vs BBA.
#
#   plain   american as shipped                                      (control)
#   on      american with our silent side's 4NT over their 2NT requiring two
#           five-card suits (--ns-2nt-unusual-veto)
#
# Found by the R4 trace of the 2026-09-20 shipping-arm decompose (c3bb94a7):
# the v6 floor jumps to 4NT over their `1♠ - 2NT` / `2M - 2NT` on a long minor
# (123 boards, -446 plain / -498 PD IMPs over 409.6k; BBA passed 97, its 21
# 3♣ lost too).  Only fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2nt-unusual-veto.sh ab-results/2nt-unusual-veto \
#       >ab-results/2nt-unusual-veto.log 2>&1 &
R=${1:?usage: ab-2nt-unusual-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 2nt unusual veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american
    arm on    "$vul" --our-floor american --ns-2nt-unusual-veto
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== 2nt unusual veto done"
