#!/bin/sh
# flat-4333-takeout-ab.sh — A/B for the opt-in flat-4333 takeout-double suppress
# (`set_suppress_flat_4333_takeout`). One two-arm experiment: default (doubles a
# 12–14 flat 4333) vs --ns-suppress-flat-4333-takeout (routes it to Pass), both
# vulnerabilities, both scorers, arms strictly sequential, one shared SEED_BASE
# (paired diffs need identical deals). Modeled on scripts/competitive-rebid-ab.sh;
# do NOT touch the codebase while it runs (bba-gen-parallel re-invokes cargo
# build; it must stay a no-op).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/flat-4333-takeout-ab.sh ab-results/flat-4333-takeout \
#       >ab-results/flat-4333-takeout.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: flat-4333-takeout-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shipped default-on: the ON arm is the default system (suppresses), the OFF arm
# re-enables the flat-4333 takeout double.
log "=== flat-4333-takeout A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm on  "$vul"
    arm off "$vul" --no-ns-suppress-flat-4333-takeout
    diffpair on off "$vul"
done
log "=== flat-4333-takeout A/B done"
