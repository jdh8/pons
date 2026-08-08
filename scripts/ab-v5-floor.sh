#!/bin/sh
# ab-v5-floor.sh — the compact-config retrain's shipping gate: `american-v5`
# (the 144-input net whose regime input is both sides' `Agreements`) vs the
# shipped `american` (the 368-input v4 net fed whole convention cards), both
# at default knobs, both against BBA.  docs/ai-bidder/card-manifold.md §"The
# retrain": v5 must not regress at defaults before any per-axis knob A/B can
# mean anything.
#
# Floor swap → constructive and contested calls both move, so plain DD is the
# primary and pd guards the doubling tail; read docs/measurement.md's table.
#
# RAN 2026-08-08 (seed 1786137947): plain +0.0353/+0.0262 (none/both), PD
# +0.0039/−0.0009 wash — user shipped the swap, so `american` now IS the v5
# floor and `american-v5` is an alias.  A re-run of this script therefore
# compares identical arms; to reproduce the measured pair, put the pre-swap
# v4 wiring back behind a name first (american_with_config).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-v5-floor.sh ab-results/v5-floor \
#       >ab-results/v5-floor.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: ab-v5-floor.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== v5 floor gate start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (american-v5 vs american)"
for vul in none both; do
    arm american    "$vul" --our-floor american
    arm american-v5 "$vul" --our-floor american-v5
    diffpair american-v5 american "$vul"
done
log "=== v5 floor gate done"
