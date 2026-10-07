#!/bin/sh
# ab-heart-rebid-keycard.sh — responder asks keycards for hearts over
# `1♥ - 1♠ - 2♥` (2+ hearts, 16+) and `1♥ - 1♠ - 3♥` (2+ hearts, 14+)
# (`rebid.heart_rebid_keycard`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-heart-rebid-keycard.sh ab-results/heart-rebid-keycard \
#       >ab-results/heart-rebid-keycard.log 2>&1 < /dev/null &
#
# What is being tested.  Off, both nodes top out at `4♥` / `3NT`: a 16–22
# count with a heart fit signs off in game.  At anchor `25aea82c` our `4♥`
# there with BBA in slam is 146 boards, −1,045 plain / −1,040 PD per 409,600
# (BBA's slam in hearts on 122).  On: the `4NT` ask and its RKCB tree, on the
# two spade-raise nodes' thresholds.
#
#   off  --no-ns-heart-rebid-keycard
#   on   the shipped default
#
# sha 5d662b20-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791323769, heart-rebid-keycard)    plain +0.0019 / +0.0022, PD +0.0020 / +0.0023, 79 / 82 fired
#   seed 2 (1791324314, heart-rebid-keycard-2)  plain +0.0014 / +0.0017, PD +0.0014 / +0.0018, 68 / 70 fired
#
# Pooled plain +0.0017 / +0.0020, PD +0.0017 / +0.0020, every cell of both
# seeds outside its CI.  Seed 1 split (plain): the 4NT asks 104 boards +504,
# opener passing the now-capped `4♥` (was the floor's blind `6♥`) 57 boards
# +353.  **Shipped default-on 2026-10-07.**  Both seeds were measured with the
# knob default off and `on` armed by the retired `--ns-heart-rebid-keycard`.
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-heart-rebid-keycard.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-rebid-keycard)

log "=== heart-rebid-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-heart-rebid-keycard
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== heart-rebid-keycard A/B done"
