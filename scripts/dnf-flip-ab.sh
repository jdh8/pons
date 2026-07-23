#!/bin/sh
# dnf-flip-ab.sh — DNF chop F: the `set_dnf_reading` flip candidate vs the
# shipped default, as a paired BBA diffpair match (docs/dnf-migration.md).
#
# Three arms per vul, identical deals:
#   off   the shipped default (both knobs off)
#   dnf   --ns-dnf                       (the flip candidate)
#   both  --ns-dnf --ns-gauge-membership (the gauge ride-along)
# Verdicts: dnf-vs-off decides the flip; both-vs-dnf decides whether the
# membership teeth ride along.  The sd-lead axis is already covered by the
# in-process knob matrix (examples/ab-dnf-sd-lead, recorded in the ledger).
#
# The knobs only change OUR side's reading (hull tightening -> net features +
# PartnerShownLen gates); BBA and both scorers are knob-free, so the arms
# differ purely in our bidding.  Read plain DD as primary with pd guarding the
# doubling tail (docs/measurement.md).
#
#   setsid nohup scripts/idle-run.sh scripts/dnf-flip-ab.sh \
#       ab-results/dnf-flip >ab-results/dnf-flip.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: dnf-flip-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== dnf flip start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (chop F: --ns-dnf vs default, gauge ride-along)"
for vul in none both; do
    arm off  "$vul"
    arm dnf  "$vul" --ns-dnf
    arm both "$vul" --ns-dnf --ns-gauge-membership
    diffpair dnf off "$vul"
    diffpair both dnf "$vul"
done
log "=== dnf flip done"
