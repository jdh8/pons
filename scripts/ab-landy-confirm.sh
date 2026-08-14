#!/bin/sh
# ab-landy-confirm.sh — second-seed confirmation of the Landy ship pair
# (landy-f, the full N1c+N1d/e/f stack, against landy-on, the shipped base
# counter).  The landy-n1c-confirm precedent: a fresh SEED_BASE, the two arms
# only, pooled with the first seed before any default flips
# (docs/one-notrump-competitive.md — "confirm a single-seed negative [or
# borderline] before designing against it").
#
#   PER_SHARD=7200 setsid nohup scripts/idle-run.sh scripts/ab-landy-confirm.sh \
#       ab-results/landy-n1def-confirm >ab-results/landy-n1def-confirm.log 2>&1 &
R=${1:?usage: ab-landy-confirm.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-confirm)
log "=== landy-confirm SEED_BASE=$SEED_BASE sha=$SHA"

for v in none both; do
    # Post-flip spellings (2026-08-14 stack ship): landy-f IS the engine
    # default; landy-on switches the stack off to recover the base counter.
    arm landy-f "$v" --their-2c-landy true --filter-1nt
    arm landy-on "$v" --their-2c-landy true --defense-2c-landy-transfer false \
        --defense-2c-landy-cue-floor false --defense-2c-landy-fit-answers false \
        --defense-2c-landy-competition false --filter-1nt
    diffpair landy-f landy-on "$v"
    sddiff landy-f landy-on "$v"
done

log "landy-confirm done"
