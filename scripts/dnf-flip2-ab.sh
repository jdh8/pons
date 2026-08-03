#!/bin/sh
# dnf-flip2-ab.sh — DNF chop F2b: the `set_dnf_reading` flip re-measured with
# the knob-matched evaluator (`evaluator_v2_dnf`, trained on knob-on prefixed
# readings).  F1 traced the original flip's loss to the bilans evaluator
# reading knob-tightened ranges it was never fit on (docs/dnf-migration.md);
# the dnf arm now serves the retrained net automatically via `dnf_reading()`.
#
# Two arms per vul, identical deals (the F gauge-membership arm is dropped:
# 0 fired in 409,600).  Since the F2b flip shipped default-on, the arms are
# expressed off the new default:
#   off   --no-ns-dnf  (the legacy hull reading, pre-flip default)
#   dnf   the shipped default (union-of-boxes + knob-matched evaluator)
#
#   setsid nohup scripts/idle-run.sh scripts/dnf-flip2-ab.sh \
#       ab-results/dnf-flip2 >ab-results/dnf-flip2.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: dnf-flip2-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== dnf flip2 start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (chop F2b: DNF-on + evaluator_v2_dnf vs --no-ns-dnf)"
for vul in none both; do
    arm off "$vul" --no-ns-dnf
    arm dnf "$vul"
    diffpair dnf off "$vul"
done
log "=== dnf flip2 done"
