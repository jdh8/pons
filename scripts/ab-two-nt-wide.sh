#!/bin/sh
# ab-two-nt-wide.sh — DNF chop G0: the wide-minor 2NT opening shape
# (`set_two_notrump_wide`) vs the shipped `balanced()` default, as a paired BBA
# diffpair match (docs/dnf-migration.md).
#
# Two arms per vul, identical deals:
#   off   the shipped default (balanced-only 2NT)
#   wide  --ns-two-nt-wide  (the G0 treatment: {M 2..=4, m 2..=6},
#                            drops 5M(332), adds 5m422/6m322)
# Verdict: wide-vs-off, plain DD as primary with pd guarding the doubling tail
# (docs/measurement.md).
#
# The knob changes only OUR side's 2NT opening shape + its inference reading;
# BBA and both scorers are knob-free, so the arms differ purely in our bidding.
#
# sd-lead is deliberately NOT scored here: ab-dump-sd would disclose the
# DEFAULT (balanced) reading to the blind leader while the wide arm actually
# holds a 6-card minor — a disclosure mismatch that flatters concealment
# (docs/measurement.md).  An in-process knob-matrix sd runner (cf.
# ab-dnf-sd-lead) is the honest way to add the sd axis; parked.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-two-nt-wide.sh \
#       ab-results/two-nt-wide >ab-results/two-nt-wide.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-two-nt-wide.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== two-nt-wide start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (chop G0: --ns-two-nt-wide vs default)"
for vul in none both; do
    arm off  "$vul"
    arm wide "$vul" --ns-two-nt-wide
    diffpair wide off "$vul"
done
log "=== two-nt-wide done"
