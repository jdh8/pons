#!/bin/sh
# balanced-takeout-ab.sh — attribute the 4-4-3-2 takeout suppression by what the
# opponents OPENED (real auction context, not inferred). 5332 is already shipped
# (no 4-card major, X can't find a fit). The anchor split (opener = the takeout-
# short suit) put the 4-4-3-2 loss over MAJOR openings (-3.2 to -3.8 IMPs/div,
# even holding the one unbid 4-card major) vs a mild loss over MINOR openings
# (-1.39). This measures each with the true opener. Three arms per vul:
#   base — default system (5332 suppressed, 4432 doubled)
#   maj  — base + --ns-suppress-4432-vs-major (pass 4432 over a major opening)
#   min  — base + --ns-suppress-4432-vs-minor (pass 4432 over a minor opening)
# diff maj vs base and min vs base price each opener slice. Both scorers, arms
# strictly sequential, one shared
# SEED_BASE (paired diffs need identical deals). Modeled on
# scripts/flat-4333-takeout-ab.sh; do NOT touch the codebase while it runs
# (bba-gen-parallel re-invokes cargo build; it must stay a no-op).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/balanced-takeout-ab.sh ab-results/balanced-takeout-final \
#       >ab-results/balanced-takeout-final.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: balanced-takeout-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== balanced-takeout final A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base "$vul"
    arm maj  "$vul" --ns-suppress-4432-vs-major
    arm min  "$vul" --ns-suppress-4432-vs-minor
    diffpair maj base "$vul"
    diffpair min base "$vul"
done
log "=== balanced-takeout final A/B done"
