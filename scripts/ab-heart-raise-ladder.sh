#!/bin/sh
# ab-heart-raise-ladder.sh — opener's heart raise ladder after `1m - 1♥`
# (*Open work* item 9 (e) of docs/bba-gap-campaign.md), the sibling of
# `rebid.spade_raise_ladder` (scripts/ab-spade-raise-ladder.sh round 3).
#
#   base    the old ladder (default until 2026-11-10): `2♥` 12–15, `3♥`
#           16–18, `4♥` 19+
#   ladder  --ns-heart-raise-invite 15 --ns-heart-raise-game 17 (the spade
#           ladder: 12–14 / 15–16 / 17+)
#   invite  --ns-heart-raise-invite 15 with the old game cut 19 (12–14 /
#           15–18 / 19+)
#
# Responder's nodes over the raise are the floor's in every arm.  The anchor
# at `50d9ee65` shows no loss to recover at opener's rebid (+273 plain on 305
# rows, −251 on 323 on the fresh seed); the spade census (5.4M raises) is the
# case for the move.
#
# Round 1 (2026-11-10, bcb1f572-dirty, seed 1794240504570765238, density
# 0.083713, 0 foreign, 3117 / 3434 fired; base vs ladder only): plain +0.0015
# ±0.0023 / +0.0006 ±0.0030 per accepted board (none / both), PD +0.0013 /
# +0.0002 → non-inferior 4/4, no win.  By first divergent call the two cuts
# pull apart: the 15-point `3♥` (was `2♥`) +762 / +1558 plain, +482 / +857 PD
# on 827 / 953 boards; the 17–18 `4♥` (was `3♥`) −316 / −487 plain, −730 /
# −1096 PD on 883 / 865, its worst boards the floor's `7♥` / `7NT` jumps over
# the `4♥`.  Round 2 adds the `invite` arm to measure the first cut alone.
#
# Round 2 read (2026-11-10, bcb1f572-dirty, seed 1794242921277593930, density
# 0.083766, 0 foreign in every gate, IMPs per accepted board none / both):
#   ladder  plain +0.0018 ±0.0023 / +0.0027 ±0.0030, PD +0.0016 / +0.0024
#           (3125 / 3450 fired) → non-inferior 4/4, no win, as round 1
#   invite  plain −0.0007 ±0.0017 / −0.0004 ±0.0022, PD +0.0000 / +0.0006
#           (1948 / 2212 fired) → non-inferior 4/4, a wash
# Round 1's split did not hold: with `3♥` 15–18 responder passes or bids only
# `4♥` where it bid game or asked over 16–18 (`3♥ -` `4♥` → pass −346 / −766
# plain, `4NT` → `4♥` −212 / −272), which returns the 15-point invite's gain
# (+412 / +1148).  The two cuts work only together.  Both knobs stay at their
# then-defaults (16 / 19); the default flip was jdh8's call.
#
# Default flipped to 15 / 17 on 2026-11-10 (jdh8, symmetry with spades, on
# the non-inferiority above); the arms below now pin every cut, so `base` is
# the old ladder and `ladder` the default.
#
# Read each arm vs base per *accepted* board under `--filter-heart-raise`;
# DENSITY rescales δ = 0.002 to the whole draw.  The knob also moves how we
# read their raise ladder, so the gate may show foreign boards; the runner
# logs a failed gate and goes on to score, and the verdict must name the
# foreign count.
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-heart-raise-ladder.sh ab-results/heart-raise-ladder-r2 \
#       >ab-results/heart-raise-ladder-r2.log 2>&1 < /dev/null &
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-heart-raise-ladder.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-raise-ladder)
F=--filter-heart-raise

log "=== heart-raise-ladder A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" $F --ns-heart-raise-invite 16 --ns-heart-raise-game 19
    arm invite "$vul" $F --ns-heart-raise-invite 15 --ns-heart-raise-game 19
    arm ladder "$vul" $F --ns-heart-raise-invite 15 --ns-heart-raise-game 17
    DENSITY=$(density_of "$R/base-$vul")
    log "density ($vul) = $DENSITY"
    for a in invite ladder; do
        gatepair $a base "$vul" || log "isolation gate $a vs base ($vul) FAILED, scoring anyway"
        diffpair $a base "$vul"
    done
done
log "=== heart-raise-ladder A/B done"
