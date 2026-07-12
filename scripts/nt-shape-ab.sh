#!/bin/sh
# nt-shape-ab.sh — A/B for the 1NT opening SHAPE policy (NotrumpShape /
# `--nt-shape`, docs/bidding-options.md A1), CONTESTED vs the BBA reference
# opponent. Routed through bba-gen (not the self-play ab-nt-shape-contested
# runner) so plain + PD + sd-lead all come from the same shards — and a wider
# 1NT is a concealment / space-stealing idea, exactly what sd-lead prices and
# plain-DD self-play undercounts. Two diff pairs per vul: wide vs classic (the
# shipped 5422-minor widening) and wide6322 vs wide (marginal value of the
# experimental 6322 superset). Both vulnerabilities, arms strictly sequential,
# one shared SEED_BASE. Modeled on scripts/rule-of-20-ab.sh; do NOT touch the
# codebase while it runs.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/nt-shape-ab.sh ab-results/nt-shape \
#       >ab-results/nt-shape.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: nt-shape-ab.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# wide is the shipped default; classic is the pre-redesign baseline, wide6322 the
# experimental superset. + diff favors the first (redesign) arm.
log "=== nt-shape A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm classic  "$vul" --nt-shape classic
    arm wide     "$vul" --nt-shape wide
    arm wide6322 "$vul" --nt-shape wide6322
    diffpair wide     classic "$vul"   # shipped widening
    diffpair wide6322 wide    "$vul"   # marginal 6322
    sddiff   wide     classic "$vul"
    sddiff   wide6322 wide    "$vul"
done
log "=== nt-shape A/B done"
