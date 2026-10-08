#!/bin/sh
# ab-heart-spade-jump-shift.sh — opener's natural `3♣` / `3♦` jump shift over
# `1♥ - 1♠` (five hearts, 4+ minor, 18+ points, game-forcing) and responder's
# authored answers (`rebid.heart_spade_jump_shift`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-heart-spade-jump-shift.sh ab-results/heart-spade-jump-shift \
#       >ab-results/heart-spade-jump-shift.log 2>&1 < /dev/null &
#
# What is being tested.  Off, `1♥ - 1♠ - 2m` is 12–19+, so an 18+ two-suiter
# can be passed in `2m` or answer the fourth suit `3NT` and be passed there
# (the recurring loser of `rebid.diamond_rebid_fourth_suit`).  On, the jump
# shift caps `2m` at 17 and responder's table (`4NT` heart keycards, `4♥`,
# the `4m` slam raise that opener answers with minor keycards, `3♠`, `5m`,
# `3NT`) never passes it.
#
#   off  --no-ns-heart-spade-jump-shift
#   on   the default
#
# v1 (5+ hearts), seed 1791442363, sha d41a598e-dirty, 204,800 boards/arm/vul,
# isolation gate passed, 178 / 187 fired: plain +0.0002 / +0.0005, PD
# −0.0002 / +0.0000 (none / both), a wash.  Split (plain): the hands that
# rebid `2m` off +97 / +158 on 143 / 150 boards; the six-heart hands it took
# from the `3♥` jump rebid −66 / −62 on 35 / 37 → v2 caps hearts at five.
#
# v2 (exactly five hearts), sha d41a598e-dirty, 204,800 boards/arm/vul,
# isolation gate passed.  IMPs/board none / both:
#
#   seed 1 (1791443084, heart-spade-jump-shift-v2)    plain +0.0011 / +0.0017, PD +0.0009 / +0.0015, 133 / 133 fired
#   seed 2 (1791443608, heart-spade-jump-shift-v2-2)  plain +0.0005 / +0.0008, PD +0.0002 / +0.0005, 127 / 131 fired
#
# Pooled plain +0.0008 / +0.0013, PD +0.0005 / +0.0010, eight of eight cells
# positive, every seed-1 cell outside its CI.  **Shipped default-on
# 2026-10-08.**
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-heart-spade-jump-shift.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-spade-jump-shift)

log "=== heart-spade-jump-shift A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-heart-spade-jump-shift
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== heart-spade-jump-shift A/B done"
