#!/bin/sh
# ab-trap-pass-colour.sh — confirm the trap pass's colour gate
# (`competition.trap_pass_unless_unfavourable`) on fresh deals.
#
# The four-cell confirmation (scripts/ab-nt-doubling-4cell.sh, 2026-10-18)
# read the trap pass a plain-DD loss at `ns` (−0.0005 ±0.0001) and a win at
# `ew` (+0.0008 ±0.0002); the `(2M) X` advance lane carried the sign flip.
# That run chose the gate, so this one confirms it on a new SEED_BASE.
#
# Arms, per cell, identical deals (`--filter-1nt --wide-1nt`, the
# population the gate was chosen on):
#   default     american as shipped (the gate default-on since 2026-10-18)
#   notrapgate  --no-ns-trap-pass-unless-unfavourable
# The 2026-10-18 run predates the flip: its dirs hold `default` (no gate) and
# `trapgate` (--ns-trap-pass-unless-unfavourable) arms, read trapgate − default.
#
# PRE-REGISTERED (2026-10-18, before any number is read).  Two invocations
# of this script, each with its own results dir and seed:
#   1. VULS=ns BOARDS=2400000 → ab-results/trap-pass-colour.  The gate
#      removes conversions to a doubled contract, so plain DD arbitrates and
#      PD is the double-blind column (measurement.md's domain addendum).
#      The gate ships default-on iff ns plain DD (gate − no gate) reads win
#      or non-inferior.  It did (2026-10-18, +0.0005 ±0.0001).  Any other reading is flagged for jdh8.
#   2. VULS="none both ew" BOARDS=96000 → ab-results/trap-pass-colour-inert.
#      `waived_at` matches `WE` exactly, so all three cells must read
#      0 divergent boards.  Any divergence blocks the ship and is traced.
# No isolation gate: the trap shapes boards we open (`1NT (2x)`) and boards
# they open (`(2M) X - ?`).  sd-lead is reported as a tie-break only.
#
#   JOBS=96 BOARDS=2400000 VULS=ns DDSS_THREADS=96 \
#       BBA_GEN=$HOME/.local/opt/glibc-2.36/bba-gen \
#       scripts/idle-run.sh scripts/ab-trap-pass-colour.sh ab-results/trap-pass-colour
#
# Resumable; SEED_BASE persists in $R/trap-pass-colour.seed.  Iron rule: do
# NOT rebuild binaries while this runs.
R=${1:?usage: ab-trap-pass-colour.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for trap-pass-colour)
log "=== trap-pass-colour SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

for v in $VULS; do
    arm default  "$v" --filter-1nt --wide-1nt
    arm notrapgate "$v" --filter-1nt --wide-1nt --no-ns-trap-pass-unless-unfavourable
    DENSITY=$(density_of "$R/default-$v")
    diffpair default notrapgate "$v"
    sddiff default notrapgate "$v" --off-no-ns-trap-pass-unless-unfavourable
done
log "trap-pass-colour done"
