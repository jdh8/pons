#!/bin/sh
# ab-nt-structure-opening-gate.sh — confine the notrump-structure relay
# blanket to a 1NT opening (`ReadingProfile::nt_structure_opening_gate`,
# defect 1 of `nt_structure_artificial`, docs/ai-bidder/bba-1nt-minors.md).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-structure-opening-gate.sh ab-results/nt-structure-gate \
#       >ab-results/nt-structure-gate.log 2>&1 < /dev/null &
#
# One seed per results dir; run a second dir for the second seed.
#
# What is being tested.  The blanket suppresses every call after responder's
# first `2♠`/`3♣`/`2NT` as a Puppet relay without checking the opening was
# 1NT, so `1♠ - 2♠` reads with no spade length, and `1♠ - 2♠ - 3♥`,
# `1♠ (X) 2♠` and the calls after Jacoby `2NT` read as nothing.  Traced
# 2026-10-02 from docs/exact-posterior.md §6.  `smoke-default --count 40000
# --seed 1` with the gate on moves 682 of 40000 self-play boards (1.7%):
# `1♠ - 2♠` 175, `1M - 2NT` 120, contested `2♠` raises ~140, `3♣` after a
# minor or contested opening, Ogust.  No 1NT auction among them.
#
#   gate  `--ns-nt-structure-opening-gate`: the blanket fires over 1NT only
#   off   the shipped default
#
# No opener isolation gate: the reading fix applies to both sides' auctions
# (our defenders read BBA's `1♠ - 2♠` too), so a divergent board opened by
# either side is in scope.  probe-divergence still reports the split.
#
# Gate, fixed before the run: the knob ships default-on iff, pooled over two
# seeds, plain DD is a win at both vulnerabilities, or plain a wash and PD a
# win (docs/measurement.md decision table).  A loss is traced before any
# verdict; a loss that traces to the floor having trained on the blanketed
# reading stays opt-in as retrain-gated.
R=${1:?usage: ab-nt-structure-opening-gate.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for nt-structure-gate)

log "=== nt-structure-gate A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm gate "$vul" --ns-nt-structure-opening-gate
    out="$R/divergence.gate.vs.off.$vul.txt"
    [ -s "$out" ] || "$PROBE" "$R/gate-$vul" "$R/off-$vul" >"$out"
    diffpair gate off "$vul"
done
log "=== nt-structure-gate A/B done"
