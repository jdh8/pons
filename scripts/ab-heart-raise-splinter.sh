#!/bin/sh
# ab-heart-raise-splinter.sh — opener's splinter raise over `1m - 1♥`
# (`3♠` / `4m`), the heart mirror of `ab-spade-raise-splinter.sh` (*Open
# work* item 9 (b) of docs/bba-gap-campaign.md), on top of the shipped
# 15 / 17 heart ladder.
#
#   base   --ns-heart-raise-splinter off: a blind `4♥` on 17+
#   any    --ns-heart-raise-splinter 4: splinter on any singleton or a void,
#          the spade lane's gate and grand rung
#
# No small (BBA's `1`) arm: the spade lane's any beat small 4/4 head-to-head.
# Read any vs base per accepted board; DENSITY rescales δ = 0.002 to the
# whole draw.
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-heart-raise-splinter.sh ab-results/heart-raise-splinter-r1 \
#       >ab-results/heart-raise-splinter-r1.log 2>&1 < /dev/null &
#
# Round 1 (2026-11-11, seed 1794327526422590351, 409,600 boards/cell, density
# 0.0835): WIN 4/4. any plain +0.0038 ±0.0014 / +0.0054 ±0.0017 (none / both),
# PD +0.0038 / +0.0053. Isolation gate 11 / 13 foreign (their splinter read as
# one), −27 / −72 IMPs. Shipped default-on at 4; see CHANGELOG.
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-heart-raise-splinter.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-raise-splinter)
F=--filter-heart-raise

log "=== heart-raise-splinter A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" $F --ns-heart-raise-splinter off
    arm any "$vul" $F --ns-heart-raise-splinter 4
    DENSITY=$(density_of "$R/base-$vul")
    log "density ($vul) = $DENSITY"
    gatepair any base "$vul" || log "isolation gate any vs base ($vul) FAILED, scoring anyway"
    diffpair any base "$vul"
done
log "=== heart-raise-splinter A/B done"
