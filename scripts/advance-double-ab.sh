#!/bin/sh
# advance-double-ab.sh — price the RICH advance of partner's takeout double of a
# one-of-a-suit opening ([1t, X, P] → advancer acts).  The flat floor gives the
# advancer only a cheapest natural suit, a 3NT, and a penalty pass — the whole
# 10+ invitational-or-better band collapses into "bid your cheapest suit," with
# no way to invite or force.  --ns-rich-advance adds the cue-asks-for-a-major
# Stayman-ask, a 1NT/2NT/3NT ladder, weak shapely game jumps, and a forced
# 3-card response when broke.  Two arms per vul:
#   base — default system (flat advance_double)
#   rich — base + --ns-rich-advance
# diff rich vs base prices the change.  Both scorers, arms strictly sequential,
# one shared SEED_BASE (paired diffs need identical deals).  Modeled on
# scripts/balanced-takeout-ab.sh; do NOT touch the codebase while it runs.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/advance-double-ab.sh ab-results/advance-double \
#       >ab-results/advance-double.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: advance-double-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== advance-double A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base  "$vul"
    arm rich  "$vul" --ns-rich-advance
    arm rubens "$vul" --ns-rich-advance --ns-advance-rubens
    diffpair rich   base "$vul"   # baseline (known wash)
    diffpair rubens rich "$vul"   # the Rubens increment (right-siding is DD-invisible)
done
log "=== advance-double A/B done"
