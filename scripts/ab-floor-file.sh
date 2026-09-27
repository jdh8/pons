#!/bin/sh
# ab-floor-file.sh — a file-loaded floor candidate against the shipped v6 floor,
# vs BBA (docs/ai-bidder/floor-sweep.md).
#
#   plain   american as shipped (the M32 v6 artifact, seed 1)        (control)
#   file    american-file: the same book, rails and regime input, the net a
#           logit mean over the v6 blobs in PONS_FLOOR_WEIGHTS (K = 1 is one
#           file-loaded net)
#
# One binary serves the whole sweep: candidates are files, never builds.
#
#   PONS_FLOOR_WEIGHTS=/mnt/ssd-data/jdh8/pons-sweep/seed2.f32 BOARDS=204800 \
#       setsid nohup scripts/idle-run.sh scripts/ab-floor-file.sh \
#       ab-results/sweep-seed2 >ab-results/sweep-seed2.log 2>&1 &
R=${1:?usage: ab-floor-file.sh RESULTS_DIR}
: "${PONS_FLOOR_WEIGHTS:?set PONS_FLOOR_WEIGHTS=a.f32,b.f32,...}"
export PONS_FLOOR_WEIGHTS
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
# Pin the blobs like ab-lib pins the shard count: a resume with other weights
# would diff a file arm drawn from two different nets.
W=$(echo "$PONS_FLOOR_WEIGHTS" | tr , '\n' | xargs sha256sum)
[ -s "$R/weights" ] || echo "$W" >"$R/weights"
[ "$(cat "$R/weights")" = "$W" ] || { echo "ab-floor-file: $R was generated with other blobs (see $R/weights)" >&2; exit 1; }

log "=== floor-file start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
log "PONS_FLOOR_WEIGHTS=$PONS_FLOOR_WEIGHTS"
echo "$W" | while read -r line; do log "blob $line"; done
for vul in none both; do
    arm plain "$vul" --our-floor american
    arm file  "$vul" --our-floor american-file
    diffpair file plain "$vul"
    sddiff file plain "$vul"
done
log "=== floor-file done"
