#!/bin/sh
# ab-closure.sh — DNF chop C1: close each read box over `Σ len = 13`.
#
# `Envelope::sum_feasible` only *tests* the 13-card sum; nothing narrows with
# it, so a both-majors reading claims `{♠ 5..13, ♥ 5..13, ♦ 0..13, ♣ 0..13}`
# when eight of the thirteen cards are already spoken for.  The closure is
# exact and membership-inert (every real hand satisfies the sum), so the
# sampler cannot move — only `EnvelopeUnion::hull` (tighter) and the `subset_of` dedup
# (fewer terms).  Smoke-measured 8.0% of boards diverge.
#
# Two arms per vul, identical deals:
#   off   the shipped default (no closure)
#   sum   --ns-sum-closure
#
# C2 (`--ns-upgrade-closure`) is NOT an arm: measured 0/3000 boards divergent,
# because nothing consumes `Envelope::strength.hcp` at default settings
# (`gauge_membership` off, and the feature/evaluator nets read lengths +
# `points` only).  See docs/dnf-migration.md.
#
#   setsid nohup scripts/ab-closure.sh ab-results/closure \
#       >ab-results/closure.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-closure.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== closure C1 start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm sum "$vul" --ns-sum-closure
    diffpair sum off "$vul"
done
log "=== closure C1 done"
