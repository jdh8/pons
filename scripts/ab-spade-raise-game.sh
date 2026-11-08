#!/bin/sh
# ab-spade-raise-game.sh — responder's game over `1x - 1♠ - 2♠` (*Open work*
# item 9 (d) of docs/bba-gap-campaign.md).
#
#   base  the shipped 12–14 `2♠`: book 4♠ on 12+ after `1♥`, the floor after `1m`
#   game  --ns-spade-raise-game: 4♠ over `1♥ - 1♠ - 2♠` on 11+, or 10+ with a
#         fifth spade
#
# Round 1 (2026-11-09, fa13a265-dirty, seed 1794155273629583038) had a wider
# arm: 11+ white, 10+ red, or 10+ with a fifth spade, in the `1♥` book and in
# a partial table over `1m - 1♠ - 2♠` claiming 10–12.  Whole arm plain
# −0.0005 ±0.0008 / −0.0001 ±0.0013 (none / both, non-inferior), PD −0.0029 /
# −0.0070 (loss).  Split by lane, plain / PD IMPs on 409,600 accepted boards:
#   `1m`  −946 / −1702, −1829 / −3846 (`4♠` vs the floor's `3♠ - - -`)
#   `1♥`  +750 / +530,  +1789 / +975
# Vulnerable 10s with four spades in `1♥` read +47 plain, −354 PD.  So round 2
# drops the `1m` table and the vulnerable split; a selected arm, hence a
# fresh seed.
#
# Round 2 read (2026-11-09, fa13a265-dirty, seed 1794157576022267568, density
# 0.094595, 0 foreign in both gates), per accepted board, none / both:
#   plain +0.0017 ±0.0005 / +0.0046 ±0.0009 (win 2/2, 382 / 472 fired)
#   PD    +0.0012 ±0.0006 / +0.0035 ±0.0010 (win 2/2)
# → shipped default-on 2026-11-09.  Since the flip `base` needs
# --no-ns-spade-raise-game and `game` no flag; re-point the arms before
# re-running.
#
# Read game vs base per accepted board; DENSITY rescales δ = 0.002 to the
# whole draw.
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-spade-raise-game.sh ab-results/spade-raise-game-r2 \
#       >ab-results/spade-raise-game-r2.log 2>&1 < /dev/null &
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-spade-raise-game.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for spade-raise-game)
F=--filter-spade-raise

log "=== spade-raise-game A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" $F
    arm game "$vul" $F --ns-spade-raise-game
    DENSITY=$(density_of "$R/base-$vul")
    log "density ($vul) = $DENSITY"
    gatepair game base "$vul" || log "isolation gate game vs base ($vul) FAILED, scoring anyway"
    diffpair game base "$vul"
done
log "=== spade-raise-game A/B done"
