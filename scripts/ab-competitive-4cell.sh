#!/bin/sh
# ab-competitive-4cell.sh — the competitive accountant's three default-on
# knobs, confirmed at the asymmetric cells (docs/research-method-review.md § 6).
#
# The gate prices our vulnerability against theirs, yet every ship read only
# `none` and `both`.  Four arms per cell, identical deals, every comparison the
# shipped default against one knob off with the others at default (the knobs
# nest: the demotion is meaningless with the accountant off, the natural-only
# push inert with the demotion off):
#   default   american as shipped
#   acctoff   --no-ns-competitive-accountant           (ships 2026-08)
#   pdoff     --no-ns-competitive-pass-demotion        (kept 2026-10-12)
#   control   --no-ns-competitive-push-natural-only    (ships 2026-10-12)
#
# FOUR-CELL CONFIRMATION (pre-registered 2026-10-16, before any number is
# read).  VULS="ns ew", fresh seed, unfiltered, BOARDS=1228800 (the 2026-10-14
# non-inferiority re-measure's size), results in
# ab-results/competitive-4cell-nsew.  The knobs change how often we double, so
# plain DD is the arbiter and PD a double-blind column (measurement.md's
# domain addendum; competitive-accountant.md "Verdict").  The default stands
# in a cell whose plain DD reads non-inferior or better; any other reading is
# flagged for a decision, not applied.
#
#   VULS="ns ew" JOBS=96 BOARDS=1228800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-competitive-4cell.sh ab-results/competitive-4cell-nsew \
#       >ab-results/competitive-4cell-nsew.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-competitive-4cell.sh RESULTS_DIR}
SHOW=${SHOW:-20}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== competitive 4cell start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for v in $VULS; do
    arm default "$v"
    arm acctoff "$v" --no-ns-competitive-accountant
    arm pdoff "$v" --no-ns-competitive-pass-demotion
    arm control "$v" --no-ns-competitive-push-natural-only
    for off in acctoff pdoff control; do
        diffpair default "$off" "$v"
        # The gate publishes nothing (projection-invisible), so the blind
        # leader needs no knob disclosure.
        sddiff default "$off" "$v"
    done
done
log "=== competitive 4cell done"
