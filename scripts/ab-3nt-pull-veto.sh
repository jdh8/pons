#!/bin/sh
# ab-3nt-pull-veto.sh — the floor's 3NT-pull rail
# (`InstinctProfile::their_3nt_pull_veto`) against plain american, vs BBA.
#
#   plain   american with the rail off (--no-ns-3nt-pull-veto)    (control)
#   on      american as shipped since 2026-09-25: our silent side never pulls
#           their 3NT to a suit of five cards or fewer
#
# Found by the wide-1♣ attribution (2026-09-24): the default v6 floor bids four
# of their suit over their 3NT with junk, worth up to ~+0.017 IMPs/board.  The
# rail only fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-3nt-pull-veto.sh ab-results/3nt-pull-veto \
#       >ab-results/3nt-pull-veto.log 2>&1 &
R=${1:?usage: ab-3nt-pull-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 3nt pull veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american --no-ns-3nt-pull-veto
    arm on    "$vul" --our-floor american
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== 3nt pull veto done"
