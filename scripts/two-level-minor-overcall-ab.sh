#!/bin/sh
# two-level-minor-overcall-ab.sh — A/B for the opt-in tight 2-level minor
# overcall (`set_two_level_minor_overcall_tight`, docs/bba-gap-campaign.md
# def-r1 overcall lever). OFF arm = shipped default (2♣/2♦ overcall at 11+);
# ON arm = --ns-two-level-minor-overcall-tight (demand 15+, strand the losing
# 11–14 minimums into Pass). Both vulnerabilities, THREE scorers (plain + pd via
# ab-dump-diff, sd-lead via ab-dump-sd — the trustworthy scorer for a
# competitive range), arms strictly sequential, one shared SEED_BASE. Do NOT
# touch the codebase while it runs (bba-gen-parallel re-invokes cargo build;
# must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/two-level-minor-overcall-ab.sh ab-results/two-level-minor-overcall \
#       >ab-results/two-level-minor-overcall.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: two-level-minor-overcall-ab.sh RESULTS_DIR}
SHOW=4
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== two-level-minor-overcall A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-two-level-minor-overcall-tight
    diffpair on off "$vul"
    sddiff on off "$vul"
done
log "=== two-level-minor-overcall A/B done"
