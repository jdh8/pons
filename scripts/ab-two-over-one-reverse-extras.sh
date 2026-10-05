#!/bin/sh
# ab-two-over-one-reverse-extras.sh — after a 2/1, opener's reverse promises
# extras (`rebid.two_over_one_reverse_extras`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-reverse-extras.sh ab-results/reverse-extras \
#       >ab-results/reverse-extras.log 2>&1 < /dev/null &
#
# What is being tested.  A new suit after a 2/1 is shape only, so `1♥ - 2m -
# 2♠` is any opening with four spades.  BBA and BEN both gate it: 15–20 in
# BBA's book, and none of their own reverses (183 by BBA in 409,600 boards, 80
# by BEN) is below 15 HCP; their minimum 4♠5♥ rebids `2NT` 61% of the time.
# On: `1♥ - 2m - 2♠` and `1♦ - 2♣ - 2M` need `points(15..)`; after `1♥ - 2m`
# the minimum with four spades rebids `2NT` and responder shows four spades
# over it with a natural `3♠`.
#
#   off  --no-ns-two-over-one-reverse-extras (the reverse on any strength)
#   on   the shipped default
#
# sha 04b2efd3-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both (the pooled row and the lanes are seeds 1–2):
#
#   seed 1 (1791202644, reverse-extras)    plain +0.0002 / +0.0002, PD +0.0002 / +0.0002, 106 / 109 fired
#   seed 2 (1791203151, reverse-extras-2)  plain −0.0003 / −0.0004, PD −0.0004 / −0.0004, 120 / 121 fired
#   pooled                                 plain −0.00006 / −0.00010, PD −0.00008 / −0.00014
#
#   seed 3 (1791205422, reverse-extras-3)  plain +0.0002 / +0.0003, PD +0.0002 / +0.0003, 113 / 116 fired
#
# The seeds disagree in sign and every cell is inside or at its CI
# (±0.0004–0.0006): a null (three seeds sum to +15 / +14 plain IMPs, 0 / +2
# PD, on 614,400 boards).  **Shipped default-on 2026-10-05** on naturalness,
# jdh8's call: it is what the other natural bidders play.  Seeds 1–2 were
# measured with the knob default off and `on` armed by the retired
# `--ns-two-over-one-reverse-extras`; seed 3 is the shipped build, against a
# default that also plays `rebid.two_over_one_side_suit_first`.  By lane, pooled plain
# IMPs none / both (boards): `1♥ - 2♣` `2♠ → 2NT` +13 / +17 (72 / 73), `1♥ -
# 2♦` `2♠ → 2NT` −34 / −40 (22), `1♦ - 2♣` `2M → 2NT` −53 / −66 (109 / 111),
# and the boards where only the sharper reading moved a later call +55 / +52
# (21 / 22).  The losing lanes are mostly the same contract; what moves them
# is three or four slam swings (off reaches a making `6♠` through `2♠ - 3♠`,
# on stops in `4♠` after `2NT - 3♠`), not an unauthored continuation.  Each
# new seed wants a fresh results dir.
R=${1:?usage: ab-two-over-one-reverse-extras.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for reverse-extras)

log "=== reverse-extras A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-two-over-one-reverse-extras
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== reverse-extras A/B done"
