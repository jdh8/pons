#!/bin/sh
# ab-accountant-floor-4cell.sh — `decision.instinct.accountant_floor` confirmed
# at the asymmetric cells (docs/research-method-review.md § 6 "Open"; the
# 1NT-lane siblings are scripts/ab-nt-doubling-4cell.sh, whose header says
# why this runs on bba-gen rather than the seat-swap `ab-accountant-floor`).
#
# The floor prices each converted game/slam gate as P(tricks) against the
# break-even for *our* vulnerability, so `ns` (we are red against white) and
# `ew` (white against red) are the cells its 2026-07-21 none/both ship never
# saw.
#   acdefault  american as shipped
#   acoff      --no-ns-accountant  the authored point-sum gates
#
# PRE-REGISTERED (2026-10-18, before any number is read).  VULS="ns ew",
# fresh seed, BOARDS=1228800 unfiltered per arm per cell (the competitive
# 4-cell size), results in ab-results/accountant-floor-4cell, no gate (both
# sides open).  Game and slam bidding, not doubling, so both scorers count:
# the default stands in a cell whose plain DD and PD both read non-inferior
# or better; any other reading is flagged for jdh8's decision, not applied.
# sd-lead (plain / SD-PD) is reported as a tie-break only.
#
#   JOBS=96 BOARDS=1228800 VULS="ns ew" DDSS_THREADS=96 \
#       BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen setsid nohup \
#       scripts/idle-run.sh scripts/ab-accountant-floor-4cell.sh \
#       ab-results/accountant-floor-4cell \
#       >ab-results/accountant-floor-4cell.log 2>&1 </dev/null &
#
# Resumable; SEED_BASE persists in $R/accountant-floor.seed.  Iron rule: do
# NOT rebuild binaries while this runs.
R=${1:?usage: ab-accountant-floor-4cell.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for accountant-floor)
log "=== accountant-floor SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm acdefault "$v"
    arm acoff     "$v" --no-ns-accountant
    diffpair acdefault acoff "$v"
    # The gates publish nothing (projection-invisible), so the blind leader
    # needs no knob disclosure.
    sddiff acdefault acoff "$v"
done

log "accountant-floor done"
