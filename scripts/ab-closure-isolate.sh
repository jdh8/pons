#!/bin/sh
# ab-closure-isolate.sh — why did chop C1 lose?
#
# `scripts/ab-closure.sh` measured the `Σ len = 13` closure at −0.037/−0.046
# plain and −0.067/−0.075 PD (NV/vul, 204,800 bd/arm/vul, SEED 1785011107).
# The closure is exact and membership-inert — pinned — so the sampler cannot be
# the cause.  Two mechanisms remain:
#
#   (a) the bilans evaluator net reads hulls it was never fit on (chop F1), or
#   (b) authored gates read the new `.min`/`.max` (fit_sum_game, keycard_trump,
#       support_floor) and their thresholds were tuned without the closure.
#
# This pair takes the net out of BOTH arms.  The delta that survives is (b);
# the delta that vanishes was (a).
#
#   nb-off   --no-ns-bilans
#   nb-sum   --no-ns-bilans --ns-sum-closure
#
#   setsid nohup scripts/ab-closure-isolate.sh ab-results/closure-isolate \
#       >ab-results/closure-isolate.log 2>&1 &
R=${1:?usage: ab-closure-isolate.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== closure isolate start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm nb-off "$vul" --no-ns-bilans
    arm nb-sum "$vul" --no-ns-bilans --ns-sum-closure
    diffpair nb-sum nb-off "$vul"
done
log "=== closure isolate done"
