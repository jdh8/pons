#!/bin/sh
# ab-exclusion-retrain.sh — pass-exclusion re-measured behind its evaluator twin.
#
# The 2026-07-30 pre-retrain run (ab-results/pass-reading) measured exclusion
# at plain −0.006 / PD −0.003, 0.68% fired — the pre-registered C1 signature:
# the shipped v3 twin was fit on readings without the exclusion caps, so the
# knob-on arm feeds it out-of-distribution inputs.  The probe gate
# (probe-closure-features --pass-exclusion, seed 1785351807) confirmed the
# retrain is earnable — 10.8% cross-arm membership rejections and real moment
# movement, the anti-C1 picture.  This run re-prices the knob with the
# exclusion twin (evaluator_v3_exclusion, corpus --dnf --pass-exclusion)
# serving the ON arm; knob-off serving is structurally unchanged.
#
# Decision table (docs/measurement.md): plain wash + PD win or better flips
# set_pass_exclusion_reading default-on; a plain loss again closes the thread,
# knob stays opt-in.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-exclusion-retrain.sh \
#       ab-results/exclusion-retrain >ab-results/exclusion-retrain.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-exclusion-retrain.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== exclusion-retrain start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base "$vul"
    arm exclusion "$vul" --ns-pass-exclusion
    diffpair exclusion base "$vul"
done
log "=== exclusion-retrain done"
