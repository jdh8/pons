#!/bin/sh
# Historical Phase 5 flip gate. It now compares aliases after the v6 flip;
# reproduce the measured arm from the pre-flip commit recorded in the handoff.
R=${1:?usage: ab-v6-floor.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== v6 honest-net gate start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (american-v6 vs american)"
for vul in none both; do
    arm american "$vul" --our-floor american
    arm american-v6 "$vul" --our-floor american-v6
    diffpair american-v6 american "$vul"
done
log "=== v6 honest-net gate done"
