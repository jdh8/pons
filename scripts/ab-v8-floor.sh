#!/bin/sh
# ab-v8-floor.sh — the v8 learned floor (features_v8: v6 + the artificial
# block, M32 labels spliced, no relabel) against the shipped v6 floor, vs BBA.
#
#   plain   american as shipped (the M32 v6 artifact)            (control)
#   v8      american-v8: the same book, the same rails, the v8 net
#
# The retrain that floor-rail-campaign.md stop criterion 5 waits for: the
# input-side fix for the phantom-strain blindness every rail patched on the
# output side.  On a win, flip `american()` to v8 and re-run each shipped
# rail's runner with OUR_FLOOR=american-v8 to re-arbitrate it.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-v8-floor.sh ab-results/v8-floor \
#       >ab-results/v8-floor.log 2>&1 &
R=${1:?usage: ab-v8-floor.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== v8 floor start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american
    arm v8    "$vul" --our-floor american-v8
    diffpair v8 plain "$vul"
    sddiff v8 plain "$vul"
done
log "=== v8 floor done"
