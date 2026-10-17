#!/bin/sh
# ab-system-variants.sh — price the whole gadget layer (docs/system-variants.md §5).
#
# Three arms per vul, identical deals, all vs BBA on its 2/1 card:
#   advanced       the shipped default (`Agreements::advanced()` ≡ `default()`)
#   basic          `--ns-preset basic`: the beginner card, every gadget at its
#                  natural authored state, same v6 net floor
#   basic-instinct `--ns-preset basic --our-floor american-instinct`: the same
#                  book on the deterministic floor (the §3 ablation)
#
# `basic vs advanced` is the first measurement of the gadget layer as a sum —
# every knob was measured against the rest of the layer, never together — and
# the number a beginner is told when the web preset is explained.
# `basic-instinct vs basic` answers whether the net floor, trained with the
# gadgets on, still plays the natural book honestly.
# Expected: basic loses; the headline is *how much*, read on both scorers.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-system-variants.sh ab-results/system-variants \
#       >ab-results/system-variants.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-system-variants.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== system variants start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm advanced "$vul"
    arm basic "$vul" --ns-preset basic
    arm basic-instinct "$vul" --ns-preset basic --our-floor american-instinct
    diffpair basic advanced "$vul"
    diffpair basic-instinct basic "$vul"
done
log "=== system variants done"
