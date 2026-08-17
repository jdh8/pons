#!/bin/sh
# Historical Phase 5 flip gate. It now compares aliases after the v6 flip;
# reproduce the measured arm from the pre-flip commit recorded in the handoff.
R=${1:?usage: ab-v6-floor.sh RESULTS_DIR american|dutch}
SYSTEM=${2:?usage: ab-v6-floor.sh RESULTS_DIR american|dutch}
[ "$SYSTEM" = american ] || [ "$SYSTEM" = dutch ] || { echo "system must be american|dutch" >&2; exit 2; }
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== v6 honest-net gate start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul ($SYSTEM-v6 vs $SYSTEM)"
for vul in none both; do
    arm "$SYSTEM" "$vul" --our-floor "$SYSTEM"
    arm "$SYSTEM-v6" "$vul" --our-floor "$SYSTEM-v6"
    diffpair "$SYSTEM-v6" "$SYSTEM" "$vul"
done
log "=== v6 honest-net gate done"
