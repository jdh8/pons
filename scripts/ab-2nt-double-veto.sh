#!/bin/sh
# ab-2nt-double-veto.sh — the floor's 2NT-double rail
# (`InstinctProfile::their_2nt_double_veto`) against plain american, vs BBA.
#
#   plain   american with the rail off (--no-ns-2nt-double-veto)   (control)
#   on      american as shipped since 2026-09-25: our silent side never
#           doubles over their 2NT opening auction (2NT - 3♣ / 3♦ / 3♥ / 3NT)
#
# Found by the 2026-09-20 shipping-arm decompose (c3bb94a7): the v6 floor
# doubles 2NT - 3NT and 2NT - 3♣ with 4–9 HCP junk (≈−9.7k plain / −12.2k PD
# IMPs over 409.6k boards, BBA passed every one).  Only fires on boards they
# open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2nt-double-veto.sh ab-results/2nt-double-veto \
#       >ab-results/2nt-double-veto.log 2>&1 &
R=${1:?usage: ab-2nt-double-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 2nt double veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american --no-ns-2nt-double-veto
    arm on    "$vul" --our-floor american
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== 2nt double veto done"
