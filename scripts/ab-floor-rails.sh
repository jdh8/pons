#!/bin/sh
# ab-floor-rails.sh — re-arbitrate the five shipped floor rails against a
# file-loaded floor (docs/archive/floor-sweep.md rule 6), as one lumped arm.
#
#   file    american-file, rails on       (the candidate as it would ship)
#   norail  american-file, all five rails off
#
# Run it in an ab-floor-file.sh results dir: the `file` arms are already there
# on the same SEED_BASE and blobs, so only `norail` is generated.  A positive
# `file vs norail` means the rails still earn on this net; split per rail only
# if it reads a wash or a loss.
#
#   PONS_FLOOR_WEIGHTS=a.f32,b.f32,... BOARDS=204800 setsid nohup \
#       scripts/idle-run.sh scripts/ab-floor-rails.sh ab-results/sweep-k4 \
#       >ab-results/sweep-k4-rails.log 2>&1 &
R=${1:?usage: ab-floor-rails.sh RESULTS_DIR}
: "${PONS_FLOOR_WEIGHTS:?set PONS_FLOOR_WEIGHTS=a.f32,b.f32,...}"
export PONS_FLOOR_WEIGHTS
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)
W=$(echo "$PONS_FLOOR_WEIGHTS" | tr , '\n' | xargs sha256sum)
[ -s "$R/weights" ] || echo "$W" >"$R/weights"
[ "$(cat "$R/weights")" = "$W" ] || { echo "ab-floor-rails: $R was generated with other blobs (see $R/weights)" >&2; exit 1; }

log "=== floor-rails start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm file   "$vul" --our-floor american-file
    arm norail "$vul" --our-floor american-file \
        --no-ns-3nt-pull-veto --no-ns-2nt-double-veto --no-ns-3nt-unusual-veto \
        --no-ns-game-pull-veto --no-ns-2nt-bid-veto
    diffpair file norail "$vul"
    sddiff file norail "$vul"
done
log "=== floor-rails done"
