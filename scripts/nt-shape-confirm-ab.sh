#!/bin/sh
# nt-shape-confirm-ab.sh — CONFIRMING second seed for the wide6322-vs-wide
# comparison only (docs/bidding-options.md A1, NotrumpShape). The first seed
# (1783843252, scripts/nt-shape-ab.sh) had wide6322 soft-positive in all 6
# cells, contradicting its archived "net-neutral/rejected" verdict — one seed
# is not a ship. This runs a fresh, independent seed at the same scale and asks
# the single question: does wide6322 beat wide again? Only the two arms that
# matter (wide, wide6322); the classic baseline is irrelevant here and skipped.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/nt-shape-confirm-ab.sh ab-results/nt-shape-confirm \
#       >ab-results/nt-shape-confirm.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: nt-shape-confirm-ab.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== nt-shape CONFIRM start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (wide6322 vs wide)"
for vul in none both; do
    arm wide     "$vul" --nt-shape wide
    arm wide6322 "$vul" --nt-shape wide6322
    diffpair wide6322 wide "$vul"
    sddiff   wide6322 wide "$vul"
done
log "=== nt-shape CONFIRM done"
