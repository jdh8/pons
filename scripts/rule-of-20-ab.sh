#!/bin/sh
# rule-of-20-ab.sh — A/B for the opt-in Rule-of-20 light openings
# (`set_rule_of_20`, docs/bba-gap-campaign.md bucket #2). One two-arm experiment:
# off (default = pass sound 10-11 counts) vs --ns-rule-of-20, both vulnerabilities,
# both scorers, arms strictly sequential, one shared SEED_BASE (paired diffs need
# identical deals). Modeled on scripts/competitive-rebid-ab.sh; do NOT touch the
# codebase while it runs (bba-gen-parallel re-invokes cargo build; must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/rule-of-20-ab.sh ab-results/rule-of-20 \
#       >ab-results/rule-of-20.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: rule-of-20-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Opt-in default-off: the OFF arm is the current default system, ON adds Rule of 20.
log "=== rule-of-20 A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-rule-of-20
    diffpair on off "$vul"
done
log "=== rule-of-20 A/B done"
