#!/bin/sh
# dutch-ab.sh — Phase 2.1 ship gate for the Dutch champion candidate: does the
# wide-1♣ system (dutch(), --our-floor dutch) reach better contracts than
# american() against the BBA reference opponent, on identical deals?
#
# Both arms use identical bba-gen flags except --our-floor, so the paired
# ab-dump-diff isolates the Dutch bidding diff (openings + 1♣ responses + the
# 1♦-relay opener rebids) against one common opponent model. Constructive
# change → plain DD is the honest primary metric; pd guards the doubling tail.
# (No sddiff: Phase 2.1 has no obstructive/preemptive calls to price blind.)
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/dutch-ab.sh ab-results/dutch-phase2.1 \
#       >ab-results/dutch-phase2.1.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).
R=${1:?usage: dutch-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== dutch Phase 2.1 start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (dutch vs american)"
for vul in none both; do
    arm american "$vul"
    arm dutch    "$vul" --our-floor dutch
    diffpair dutch american "$vul"
done
log "=== dutch Phase 2.1 done"
