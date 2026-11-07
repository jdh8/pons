#!/bin/sh
# ab-spade-raise-ladder.sh — opener's spade raise ladder after `1x - 1♠`
# (*Open work* item 9 of docs/bba-gap-campaign.md; 170 boards, −434 plain on
# the `6141cdb2` anchor).
#
# Round 1 (2026-11-08, seed 1794069695886318620) ran one arm with both
# knobs: none cell plain −0.0017 ±0.0015 (loss, 95 fired), PD −0.0020; the
# both cell stopped at the isolation gate (1 foreign board: our defender's
# call over their `1♣ - 1♠ - 4♠ - 4NT - 5♣`).  Round 2 splits it:
#
#   ranges   (--ns-spade-raise-ladder)   opener's `2♠` 12–14, `3♠` 15–16, `4♠` 17+
#   answers  (--ns-spade-raise-answers)  responder's `3♠` (`4♠` on 7+, RKCB on
#            14+) and `4♠` (RKCB on 12+) nodes after `1♣`/`1♦`/`1♥`, both tails
#   full     both knobs
#
# The ranges knob also moves how we read their raise ladder, so its gate may
# show foreign boards; the runner logs a failed gate and goes on to score, and
# the verdict must name the foreign count.
#
# Round 2 read (seed 1794073969209614479, 0 foreign in every gate): ranges
# non-inferior 2/2, plain +0.0007 / +0.0013, PD +0.0008 / +0.0011 → shipped
# default-on 2026-11-08; answers −0.0009 / −0.0013 (inconclusive); full
# −0.0003 / +0.0001.  Since the flip, `base` is the shipped ranges; the old
# ranges are --no-ns-spade-raise-ladder.  Re-running needs the arms below
# re-pointed (base = --no-ns-spade-raise-ladder).
#
# Arm 2 (opener splinters, sub-arms 2a/2b on singleton honours) is measured
# on top of this one only if it ships.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-spade-raise-ladder.sh ab-results/spade-raise-ladder-r2 \
#       >ab-results/spade-raise-ladder-r2.log 2>&1 < /dev/null &
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-spade-raise-ladder.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for spade-raise-ladder)

log "=== spade-raise-ladder A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul"
    arm ranges "$vul" --ns-spade-raise-ladder
    arm answers "$vul" --ns-spade-raise-answers
    arm full "$vul" --ns-spade-raise-ladder --ns-spade-raise-answers
    for on in ranges answers full; do
        gatepair "$on" base "$vul" || log "isolation gate $on vs base ($vul) FAILED, scoring anyway"
        diffpair "$on" base "$vul"
    done
done
log "=== spade-raise-ladder A/B done"
