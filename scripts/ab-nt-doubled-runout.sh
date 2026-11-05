#!/bin/sh
# ab-nt-doubled-runout.sh — the natural runout after BBA doubles our 1NT
# overcall (`DefenseKnobs::nt_overcall_doubled_runout`, built and shipped
# default-on 2026-11-05: won 4/4 cells plain and PD), vs BBA, four cells, plain DD + perfect defense from one solve,
# plus SD-PD.  Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-doubled-runout.sh ab-results/nt-doubled-runout \
#       >ab-results/nt-doubled-runout.log 2>&1 < /dev/null &
#
# Why.  `(1t) 1NT (X)` has no book node: the systems-on graft authors only
# uncontested keys, so the net floors the whole lane, and the BBA-relabel
# K = 8 bids a phantom 3♦ there on five-card heart hands
# (docs/reseed-gap-handoff.md §5 "Owed from the ship").  The package mirrors
# the instinct floor's runout of our doubled 1NT opening: five-plus escapes
# and the 4-4 minor bust under 8 HCP, XX on 7+, else pass; the overcaller
# passes the escape, sits for their double of it, and runs to its own
# five-plus suit in the balancing seat.  Nodes after XX and their bids over
# an escape stay with the floor.  Design: docs/defensive-overcalls.md § "The
# natural runout after they double our 1NT overcall".
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  The trigger is rare (they open, we
# overcall 1NT, they double): read each cell's `fired` count first — a cell
# under 50 contract-divergent boards is INSUFFICIENT DATA, and if every cell
# is, re-run at BOARDS=409600 with the same SEED_BASE in a fresh dir.  The
# arm's escapes and redouble change the reading, so sddiff passes the on flag.
# Nothing is changed by this runner's verdict; the default is jdh8's call.
#
# READ 2026-11-05 (SEED_BASE 1793866199902140248, on − off, IMPs/board ±95%):
#   plain  none +0.0012±0.0005  ns +0.0017±0.0007  ew +0.0018±0.0006  both +0.0024±0.0008
#   PD     none +0.0011±0.0005  ns +0.0015±0.0006  ew +0.0018±0.0006  both +0.0023±0.0008
#   SD-PD  +0.0009 / +0.0011 / +0.0015 / +0.0019; fired 64 / 67 / 74 / 76 of
#   204,800; isolation gate 0 foreign in every cell.  Win 4/4 → shipped
#   default-on; the `off` arm now passes `--no-ns-nt-doubled-runout`.
#
# Resumable; SEED_BASE persists in $R/nt-doubled-runout.seed.  Iron rule: do
# NOT rebuild binaries while this runs.
R=${1:?usage: ab-nt-doubled-runout.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for nt-doubled-runout)

log "=== nt-doubled-runout A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm off "$vul" --no-ns-nt-doubled-runout
    arm on "$vul"
    gatepair on off "$vul" theirs
    diffpair on off "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.off.$vul.plain.txt")
    log "pre-count on vs off $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on off "$vul" --off-no-ns-nt-doubled-runout
done
log "=== nt-doubled-runout A/B done"
