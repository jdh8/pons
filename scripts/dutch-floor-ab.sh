#!/bin/sh
# dutch-floor-ab.sh — A/B A of the WJ-floor campaign: does `dutch()` gain from
# the BBA-distilled floor the way `american()` did (+0.11/+0.25 IMPs/bd when it
# was promoted 2026-07-19)?  `dutch()` never got that swap — it ran
# `with_instinct_floor` everywhere until now.
#
# Both arms are the SAME authored Dutch books (`bare_dutch`); the only
# difference is the floor that catches off-book auctions:
#   dutch-instinct  deterministic `instinct()` ladder  (the old default)
#   dutch           `NeuralFloorBba`                   (the treatment)
#
# Floor swap → it moves both constructive and contested calls, so plain DD is
# the primary and pd guards the doubling tail; read the decision table in
# docs/measurement.md.  This is the prerequisite for A/B B (the WJ net over 1♦):
# B's non-1♦ subtrees *are* this arm's treatment, so B is uninterpretable first.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/dutch-floor-ab.sh ab-results/dutch-floor \
#       >ab-results/dutch-floor.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: dutch-floor-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== dutch floor swap start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (dutch vs dutch-instinct)"
for vul in none both; do
    arm dutch-instinct "$vul" --our-floor dutch-instinct
    arm dutch          "$vul" --our-floor dutch
    diffpair dutch dutch-instinct "$vul"
done
log "=== dutch floor swap done"
