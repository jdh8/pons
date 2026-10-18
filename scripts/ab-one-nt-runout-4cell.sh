#!/bin/sh
# ab-one-nt-runout-4cell.sh — re-measure the doubled-1NT runout
# (`one_nt_runout`) against the default that no longer doubles their escape.
#
# The runout was kept on 2026-10-18 as a recorded trade-off: plain DD lost in
# all four cells (scripts/ab-nt-doubling-4cell.sh at ns/ew,
# scripts/ab-escape-double-4cell.sh at none/both), PD won.  Counted over every
# divergent board the escapes themselves won plain (ns +539, ew +352 IMPs);
# the loss was opener leaving in our double of their escape.  Both of those
# doubles have since gone default-off (`penalize_escape_stack`, eb394474;
# `penalize_escape_values`, 9a9e91a0), so the trade-off was measured on a
# default that no longer exists.  This run asks whether the runout alone,
# without the leave-ins, is a plain-DD win.
#
# Arms, per cell, identical deals (`--filter-1nt --wide-1nt`):
#   default   american as shipped (stack and values doubles off)
#   runoff    --no-ns-one-nt-runout
#
# PRE-REGISTERED (2026-10-19, before any number is read).  VULS="none both
# ns ew", fresh seed, BOARDS=2400000 accepted per arm per cell, results in
# ab-results/runout-4cell, density from the default arm.  The runout adds a
# business redouble and penalty conversions, so plain DD arbitrates and PD is
# the double-blind column (measurement.md's domain addendum).
#   - Plain DD non-inferior or better (lower bound above −δ) in all four
#     cells → the trade-off is retired: the runout stands as confirmed, and
#     the archive's "recorded trade-off" rows are superseded.
#   - Plain DD loss in all four cells → the trade-off stands; trace the worst
#     divergent boards by lane (`ab-dump-bucket --by lane`) and report to
#     jdh8.  Nothing is flipped.
#   - Any other reading is flagged for jdh8's decision, not applied.
# No isolation gate: the runout anchors on our 1NT overcall as well as our
# opening (`one_nt_anchor`, `(1♦) 1NT (X)`).  sd-lead is a tie-break only.
#
#   JOBS=96 BOARDS=2400000 VULS="none both ns ew" DDSS_THREADS=96 \
#       BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       scripts/idle-run.sh scripts/ab-one-nt-runout-4cell.sh ab-results/runout-4cell
#
# Read 2026-10-19 (verdict in docs/archive/research-method-review-closed.md
# §6): plain DD, PD and sd-lead win in all four cells; the trade-off is retired.
#
# Resumable; SEED_BASE persists in $R/runout.seed.  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-one-nt-runout-4cell.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for runout)
log "=== runout-4cell SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm default "$v" --filter-1nt --wide-1nt
    arm runoff  "$v" --filter-1nt --wide-1nt --no-ns-one-nt-runout
    DENSITY=$(density_of "$R/default-$v")
    diffpair default runoff "$v"
    sddiff default runoff "$v" --off-no-ns-one-nt-runout
done
log "runout-4cell done"
