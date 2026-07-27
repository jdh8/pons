#!/bin/sh
# eval-shape-ab.sh — the shape-reading evaluator's 2×2: `set_eval_shape` and
# `set_sum_closure`, crossed.
#
# The v4 twin (`evaluator_v4_dnf`, features_eval_v4) reads each hidden seat as a
# distribution over the 560-shape lattice — E[len], sd[len], log-mass — instead
# of four suit-length {min, max} pairs.  It is NLL-par with v3 by construction
# (+0.00004 against a matched control, seed spread 0.0006), so this is not an
# accuracy change and must not be sold as one.  What it buys is invariance: a
# hull is not a function of the reading, so `set_sum_closure` — which provably
# rejects no hand — displaces the endpoint columns 4.19σ at 81% of nodes, and
# every reading-fidelity chop has had to buy an evaluator retrain before it
# could be judged on merit.
#
# The crossing is what makes that measurable.  Pre-measured at 3000 boards,
# vul none, the closure's *bidding* footprint splits cleanly:
#
#   no net at all (--no-ns-bilans)  37.13%   samplers + authored gates
#   v3 endpoint net live            39.00%   the net adds +1.87 pts
#   v4 shape net live               37.10%   the net adds -0.03 pts
#
# So ~37 points of the closure's footprint is real information change that
# should move boards, and the ~1.9 points on top of it — under endpoints only —
# is the feature perturbation this campaign exists to delete.
#
# Four arms per vul; the two diffs that matter are the closure pair under each
# encoding, and their difference is the result:
#
#   base          shipped default             (v3 hulls, closure off)
#   base-closure  --ns-sum-closure            (v3 hulls, closure on)  <- contaminated
#   shape         --ns-eval-shape             (v4 shape, closure off)
#   shape-closure --ns-eval-shape --ns-sum-closure                    <- clean
#
# `shape vs base` falls out of the same deals for free: that is A/B-1, the
# encoding swap on its own.  It is a NON-INFERIORITY check and nothing more —
# it moves 1.6% of boards with a par-NLL net behind them, so the changed
# decisions should be a coin flip; at 204.8k bd/arm/vul the interval is ±0.004
# IMPs against an effect the NLL scaling puts near 0.0005.  Read it as "did the
# re-parameterization cost anything", never as a win.
#
# RESULT (2026-07-27, sha 580655f, SEED_BASE 1785154805, 204.8k bd/arm/vul):
#
#   closure cost, pooled     plain              PD
#     under endpoints        -0.1766            -0.3445
#     under shape            -0.1588            -0.3168
#     difference            +0.0177 +-0.0138   +0.0278 +-0.0164
#
#   A/B-1, pooled           -0.0037 +-0.0028   -0.0034 +-0.0030
#
# The mechanism is confirmed -- one sign in all four cells, both pooled
# intervals excluding zero -- and it closed the roadmap instead of opening it:
# the closure costs -0.18, so recovering a tenth of it leaves it dead on merit,
# and no chop on the ledger sits inside the +-0.02 the encoding can move.
# Measure the remaining hull-tightening chops directly on v3.  A/B-1 came back a
# small REAL loss rather than a wash, so `set_eval_shape` stays off.
#
#   setsid nohup scripts/idle-run.sh scripts/eval-shape-ab.sh \
#       ab-results/eval-shape >ab-results/eval-shape.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: eval-shape-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== eval-shape start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (shape twin × sum closure)"
for vul in none both; do
    arm base "$vul"
    arm base-closure "$vul" --ns-sum-closure
    arm shape "$vul" --ns-eval-shape
    arm shape-closure "$vul" --ns-eval-shape --ns-sum-closure

    # The prize: what the closure costs under each encoding.  A wash-or-win on
    # the second where the first loses is what unblocks the closure roadmap.
    diffpair base-closure base "$vul"
    diffpair shape-closure shape "$vul"
    # A/B-1, free from the same deals: the encoding swap alone.
    diffpair shape base "$vul"
done
log "=== eval-shape done"
