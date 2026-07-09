#!/bin/sh
# opener-extras-ladder-ab.sh — A/B for the opt-in opener strength-showing rebid
# ladder (`set_opener_extras_ladder`, docs/bba-gap-campaign.md bucket #3:
# Constructive/book/round-2). One two-arm experiment: on (shipped default = the
# jump-rebid / reverse / jump-shift ladder after a minor opening and a one-level
# response) vs off (--no-ns-opener-extras-ladder, the minimum natural 2m/2M rebid
# with no upper bound), both vulnerabilities, both scorers, arms strictly sequential, one
# shared SEED_BASE (paired diffs need identical deals). Modeled on
# scripts/rule-of-20-ab.sh; do NOT touch the codebase while it runs
# (bba-gen-parallel re-invokes cargo build; must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/opener-extras-ladder-ab.sh ab-results/opener-extras-ladder \
#       >ab-results/opener-extras-ladder.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: opener-extras-ladder-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shipped default-on: ON arm is the current default system, OFF disables the ladder.
log "=== opener-extras-ladder A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-opener-extras-ladder
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== opener-extras-ladder A/B done"
