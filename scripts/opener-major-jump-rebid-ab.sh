#!/bin/sh
# opener-major-jump-rebid-ab.sh — A/B for the opt-in major jump-rebid rung
# (`set_opener_major_jump_rebid`, docs/bba-gap-campaign.md bucket #3
# residual: the 6+ ♥ / 6+ ♠ underbids in Constructive/book/round-2). One
# two-arm experiment: on (shipped default = opener jumps 3M on a six-card major
# with 16+ over 1♥-1♠ / 1M-1NT, plus responder's continuation) vs off
# (--no-ns-opener-major-jump-rebid = the minimum 2M rebid with no upper bound).
# The minor extras ladder stays ON in both arms, so this isolates the major
# increment. Both
# vulnerabilities, both scorers, arms strictly sequential, one shared SEED_BASE
# (paired diffs need identical deals). Modeled on
# scripts/opener-extras-ladder-ab.sh; do NOT touch the codebase while it runs
# (bba-gen-parallel re-invokes cargo build; must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/opener-major-jump-rebid-ab.sh ab-results/opener-major-jump-rebid \
#       >ab-results/opener-major-jump-rebid.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: opener-major-jump-rebid-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Shipped default-on: ON arm is the current default system, OFF disables the rung.
log "=== opener-major-jump-rebid A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-opener-major-jump-rebid
    arm on  "$vul"
    diffpair on off "$vul"
done
log "=== opener-major-jump-rebid A/B done"
