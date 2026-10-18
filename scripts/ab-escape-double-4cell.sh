#!/bin/sh
# ab-escape-double-4cell.sh — isolate the doubles of their escape from our
# doubled 1NT, the part of the runout the four-cell confirmation blamed.
#
# scripts/ab-nt-doubling-4cell.sh (2026-10-18) read `one_nt_runout` a
# plain-DD loss at both colour cells (ns −0.0004 ±0.0002, ew −0.0008
# ±0.0002).  Counted over every divergent board, the escapes themselves won
# (ns +539, ew +352 IMPs); the loss was opener leaving in partner's double
# of their escape — after our business redouble (ns −1 290, ew −1 082), and
# after an unredoubled `1NT (X)`, where the double was almost never the
# authored stack double but the floor's takeout, left in anyway (ns −508,
# ew −1 282).  Opener's leave-in now follows the knob that authored the
# double, so each arm below removes one double *and* its leave-in.
#
# Arms, per cell, identical deals (`--filter-1nt --wide-1nt`):
#   default   american as shipped
#   nostack   --no-ns-penalize-escape-stack   no stack X, no leave-in without XX
#   novalues  --no-ns-penalize-escape-values  no values X after `1NT (X) XX`
#   runoff    --no-ns-one-nt-runout           at none/both only: completes the
#             runout's own four-cell record (ns/ew are in nt-doubling-4cell)
#
# PRE-REGISTERED (2026-10-18, before any number is read).  VULS="none both
# ns ew", fresh seed, BOARDS=2400000 accepted per arm per cell, results in
# ab-results/escape-double-4cell, density from the default arm.  Both knobs
# add penalty doubles, so plain DD arbitrates and PD is the double-blind
# column (measurement.md's domain addendum).
#   - A knob's default (on) stands in a cell whose default-vs-off plain DD
#     reads non-inferior or better.
#   - A knob whose default-vs-off plain DD reads loss in all four cells is
#     proposed to jdh8 for default-off — flagged, not applied.
#   - Any other reading is flagged for jdh8's decision, not applied.
#   - runoff at none/both is recorded beside the ns/ew trade-off; it decides
#     nothing here.
# No isolation gate: the runout anchors on our 1NT overcall as well as our
# opening (`one_nt_anchor`, `(1♦) 1NT (X)`).  sd-lead is a tie-break only.
#
#   JOBS=96 BOARDS=2400000 VULS="none both ns ew" DDSS_THREADS=96 \
#       BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       scripts/idle-run.sh scripts/ab-escape-double-4cell.sh ab-results/escape-double-4cell
#
# Ran 2026-10-18 on the 72f838f1 binaries.  The stack double has been default
# off since: `bba-gen` now spells it `--ns-penalize-escape-stack` (and
# `ab-dump-sd --on-ns-penalize-escape-stack`), so the `nostack` lines below
# are the pre-flip spelling and only resume under SKIP_BUILD=1 with those
# binaries.
#
# Resumable; SEED_BASE persists in $R/escape-double.seed.  Iron rule: do NOT
# rebuild binaries while this runs.
R=${1:?usage: ab-escape-double-4cell.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for escape-double)
log "=== escape-double SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm default  "$v" --filter-1nt --wide-1nt
    arm nostack  "$v" --filter-1nt --wide-1nt --no-ns-penalize-escape-stack
    arm novalues "$v" --filter-1nt --wide-1nt --no-ns-penalize-escape-values
    DENSITY=$(density_of "$R/default-$v")
    diffpair default nostack "$v"
    diffpair default novalues "$v"
    sddiff default nostack  "$v" --off-no-ns-penalize-escape-stack
    sddiff default novalues "$v" --off-no-ns-penalize-escape-values
    case $v in
        none | both)
            arm runoff "$v" --filter-1nt --wide-1nt --no-ns-one-nt-runout
            diffpair default runoff "$v"
            sddiff default runoff "$v" --off-no-ns-one-nt-runout
            ;;
    esac
done
log "escape-double done"
