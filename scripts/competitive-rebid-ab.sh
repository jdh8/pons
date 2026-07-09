#!/bin/sh
# competitive-rebid-ab.sh — A/B for the opt-in competitive long-suit rebid floor
# (`set_competitive_rebid`, docs/competitive-book.md P5). One two-arm experiment:
# off (default) vs --ns-competitive-rebid, both vulnerabilities, both scorers,
# arms strictly sequential, one shared SEED_BASE (paired diffs need identical
# deals). Modeled on scripts/competitive-book-ab.sh; do NOT touch the codebase
# while it runs (bba-gen-parallel re-invokes cargo build; it must stay a no-op).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/competitive-rebid-ab.sh ab-results/competitive-rebid \
#       >ab-results/competitive-rebid.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: competitive-rebid-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shipped default-on: the OFF arm disables it, the ON arm is the default system.
log "=== competitive-rebid A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-competitive-rebid
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== competitive-rebid A/B done"
