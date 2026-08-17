#!/bin/sh
# ab-pass-reading.sh — the remaining pass-reading census work item, priced vs BBA.
#
#   base       shipped defaults
#   probed     --ns-probe 100000      Partnership::probe behavioral boxes, traffic-
#              keyed (the floor's passes — `1NT -`, `2♣/2NT -`, fourth seat — read
#              their measured bands; the only reader that can)
#
# Expectations, stated before the run (docs/ai-bidder/sampled-projection.md):
# the probed arm's boxes cover ~58% of decision traffic including territory no
# symbolic reading reaches.
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
    arm probed "$vul" --ns-probe 100000
    diffpair probed base "$vul"
done
log "=== pass-reading done"
