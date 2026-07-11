#!/bin/sh
# longest-advance-ab.sh — price advancing partner's takeout double of a
# one-of-a-suit opening ([1t, X, P]) with the LONGEST suit (higher-ranking on a
# tie) instead of the highest-ranking eligible suit.  The flat advance_double
# scores every eligible 4+ suit alike, so the classifier's argmax bids the
# highest-ranking one regardless of length — holding five clubs and four spades
# it advances 1♠, not 2♣.  --ns-longest-advance grades the natural rung by length
# so the longer suit wins, equal-length ties breaking to the higher suit (it also
# governs the rich advance's weak natural and forced-suit picks, a no-op here
# since rich is off in both arms).  Two arms per vul:
#   base    — default system (flat advance_double, highest-ranking)
#   longest — base + --ns-longest-advance
# diff longest vs base prices the change.  Both scorers, arms strictly sequential,
# one shared SEED_BASE (paired diffs need identical deals).  Modeled on
# scripts/advance-double-ab.sh; do NOT touch the codebase while it runs.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/longest-advance-ab.sh ab-results/longest-advance \
#       >ab-results/longest-advance.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: longest-advance-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== longest-advance A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base    "$vul"
    arm longest "$vul" --ns-longest-advance
    diffpair longest base "$vul"
done
log "=== longest-advance A/B done"
