#!/bin/sh
# ab-pass-reading.sh — the two census work items, priced vs BBA.
#
# Both knobs land default-off (2026-07-30); this run decides whether either
# earns a default flip pre-retrain, and calibrates the retrain case:
#
#   base       shipped defaults
#   exclusion  --ns-pass-exclusion    passes exclude declined heavier siblings
#              (the weak-two defensive passes read <=16 points instead of ⊤)
#   probed     --ns-probe 100000      Stance::probe behavioral boxes, traffic-
#              keyed (the floor's passes — `1NT -`, `2♣/2NT -`, fourth seat — read
#              their measured bands; the only reader that can)
#
# Expectations, stated before the run (docs/ai-bidder/sampled-projection.md):
# the exclusion band equals the REFUTED weak_two_pass_gate (C1 encoding loss
# pre-retrain), so loss|wash on plain is the C1 signature, not a soundness
# failure — disposition is stay-off + queue for the feature retrain.  The
# probed arm is the new information: its boxes cover ~58% of decision traffic
# including territory no symbolic reading reaches.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-pass-reading.sh \
#       ab-results/pass-reading >ab-results/pass-reading.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-pass-reading.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== pass-reading start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base "$vul"
    arm exclusion "$vul" --ns-pass-exclusion
    arm probed "$vul" --ns-probe 100000
    diffpair exclusion base "$vul"
    diffpair probed base "$vul"
done
log "=== pass-reading done"
