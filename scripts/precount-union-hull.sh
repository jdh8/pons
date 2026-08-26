#!/bin/sh
# precount-union-hull.sh — bid-only pre-count for the `union_hull` reading knob.
#
# `docs/pdi.md` follow-on 1: `Inferences::assemble` never recomputed `players`
# from `unions`, so a post-walk union reached only the nets' feature block.
# `--ns-union-hull` closes that.  Before any double-dummy time, two counts:
#
#   reach   — `probe-union-hull` replays the OFF arm's own auctions and asks
#             both pinned partnerships for the call at every one of our turns.
#             A decision-level flip count: strictly more sensitive than the
#             contract diff, and it needs no ON arm.
#   moved   — the classic pre-count: both arms bid, final contracts compared,
#             no solver (docs/measurement.md item 6).
#
# SEED_BASE is pinned to the PDI pre-count's so the two are comparable board for
# board.
#
#   setsid nohup scripts/idle-run.sh scripts/precount-union-hull.sh \
#       /mnt/hdd-data/jdh8/pons-ab-results/union-hull-precount \
#       >/mnt/hdd-data/jdh8/pons-ab-results/union-hull-precount.log 2>&1 &
R=${1:?usage: precount-union-hull.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-union-hull'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=1787700673

log "=== union-hull pre-count start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on "$vul" --ns-union-hull
done
log "=== generation done"

for vul in none both; do
    out="$R/reach-$vul.txt"
    [ -s "$out" ] || target/release/examples/probe-union-hull "$R/off-$vul" --show 20 >"$out" 2>&1
    log "reach $vul: $(sed -n 's/^call flips *: //p' "$out")"
done

python3 "$(dirname "$0")/precount-contract-diff.py" "$R" | tee -a "$R/log"
log "=== union-hull pre-count done"
