#!/bin/sh
# five-card-major-takeout-ab.sh — A/B for the opt-in unbid-five-card-major
# takeout-double suppress (`set_suppress_5card_major_takeout`). One two-arm
# experiment: default (doubles with an unbid 5-card major) vs
# --ns-suppress-5card-major-takeout (overcalls the major instead), both
# vulnerabilities, plain + PD + sd-lead (a competitive range, so sd is the
# arbiter), arms strictly sequential, one shared SEED_BASE (paired diffs need
# identical deals). Modeled on scripts/flat-4333-takeout-ab.sh; do NOT touch the
# codebase while it runs (bba-gen-parallel re-invokes cargo build; it must stay a
# no-op).
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/five-card-major-takeout-ab.sh ab-results/five-card-major-takeout \
#       >ab-results/five-card-major-takeout.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: five-card-major-takeout-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Opt-in (default off): the `overcall` arm turns the knob on (overcall the unbid
# five-card major); the `keep` arm is the default system (doubles). diffpair /
# sddiff report overcall − keep, so positive = the knob gains vs BBA.
log "=== five-card-major-takeout A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm overcall "$vul" --ns-suppress-5card-major-takeout
    arm keep     "$vul"
    diffpair overcall keep "$vul"
    sddiff   overcall keep "$vul"
done
log "=== five-card-major-takeout A/B done"
