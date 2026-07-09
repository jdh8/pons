#!/bin/sh
# nt-overcall-no-major-ab.sh — A/B for the opt-in tight 2-level minor
# overcall (`set_nt_overcall_no_major`, docs/bba-gap-campaign.md
# def-r1 overcall lever). OFF arm = shipped default (5-card major overcalls 1NT);
# ON arm = --ns-nt-overcall-no-major (bar 5-card majors so the suit is
# overcalled naturally to find the fit). Both vulnerabilities, THREE scorers (plain + pd via
# ab-dump-diff, sd-lead via ab-dump-sd — the trustworthy scorer for a
# competitive range), arms strictly sequential, one shared SEED_BASE. Do NOT
# touch the codebase while it runs (bba-gen-parallel re-invokes cargo build;
# must stay a no-op).
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/nt-overcall-no-major-ab.sh ab-results/nt-overcall-no-major \
#       >ab-results/nt-overcall-no-major.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: nt-overcall-no-major-ab.sh RESULTS_DIR}
SHOW=4
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== nt-overcall-no-major A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-nt-overcall-no-major
    diffpair on off "$vul"
    sddiff on off "$vul"
done
log "=== nt-overcall-no-major A/B done"
