#!/bin/sh
# ab-nt-structure-side-gate.sh — confine the notrump-structure relay blanket
# to the 1NT side's own calls (`ReadingProfile::nt_structure_side_gate`,
# defect 2 of `nt_structure_artificial`, docs/ai-bidder/bba-1nt-minors.md).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-nt-structure-side-gate.sh ab-results/nt-structure-side \
#       >ab-results/nt-structure-side.log 2>&1 < /dev/null &
#
# One seed per results dir; run a second dir for the second seed.
#
# What is being tested.  The blanket has no `is_opening_side` gate, so after
# `1NT - 2♠`/`2NT`/`3♣` (Puppet / minor transfers) a defender's overcall or
# competitive raise reads as a relay — as nothing.  Defect 1 (the 1NT-opening
# gate, ab-nt-structure-opening-gate.sh) shipped default-on first; over that
# default `smoke-default --count 40000 --seed 1` with this gate on moves 66 of
# 40000 self-play boards (0.17%), every one a 1NT auction with a defender's
# call after the relay.
#
#   side  the default since 2026-10-02: the blanket covers the 1NT side only
#   off   `--no-ns-nt-structure-side-gate`: the pre-fix blanket
#
# No opener isolation gate: the reading applies to both sides' auctions (our
# side reads BBA's defenders over our 1NT, and our defenders' own calls over
# BBA's 1NT - 2♠/2NT/3♣ are read by our partner), so either opener is in scope.
#
# Gate, fixed before the run: the knob ships default-on iff, pooled over two
# seeds, plain DD is a win at both vulnerabilities, or plain a wash and PD a
# win (docs/measurement.md decision table).  A loss is traced before any
# verdict; a loss that traces to the floor having trained on the blanketed
# reading stays opt-in as retrain-gated.
#
# WASH 2026-10-02, SHIPPED default-on on the naturalness tiebreak (jdh8:
# a natural overcall should read as natural).  Run while the knob was opt-in
# (sha 9529016f + the knob; the side arm was `--ns-nt-structure-side-gate`,
# now `--no-ns-nt-structure-side-gate` marks the off arm).
# IMPs/board none / both, fired = boards that diverged:
#
#   seed 1790927570  plain -0.0001 ±0.0005 / +0.0000 ±0.0005, 58 / 40 fired
#                    PD    -0.0001 ±0.0006 / +0.0000 ±0.0006
#   seed 1790928070  plain -0.0002 ±0.0005 / -0.0001 ±0.0005, 49 / 38 fired
#                    PD    -0.0001 ±0.0005 / +0.0001 ±0.0006
#   pooled IMPs      plain -65 / -18, PD -46 / +23 over 409,600 boards/vul
#                    (-0.61 / -0.23 plain, -0.43 / +0.29 PD per fired board)
#
# 0.02-0.03% fired against BBA, a tenth of self-play's 0.17%.  71% of the
# divergent boards were opened by BBA (our defence over their 1NT - 2♠ /
# 2NT / 3♣); the first differing call is ours every time, 57% a pass where
# the off arm bid -- partner's overcall now reads as a suit, and the floor,
# trained under the blanket, stops competing.
R=${1:?usage: ab-nt-structure-side-gate.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for nt-structure-side)

log "=== nt-structure-side A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-nt-structure-side-gate
    arm side "$vul"
    out="$R/divergence.side.vs.off.$vul.txt"
    [ -s "$out" ] || "$PROBE" "$R/side-$vul" "$R/off-$vul" >"$out"
    diffpair side off "$vul"
done
log "=== nt-structure-side A/B done"
