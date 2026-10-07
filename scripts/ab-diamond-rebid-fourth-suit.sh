#!/bin/sh
# ab-diamond-rebid-fourth-suit.sh — responder's `3♣` fourth-suit game force
# over `1♥ - 1♠ - 2♦` (every 13+ HCP hand; opener answers 3♠/3♥/3NT/3♦,
# responder places or asks keycards for hearts) with the `4♥` fast-arrival
# raise beside it (three hearts, 13–15) (`rebid.diamond_rebid_fourth_suit`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-diamond-rebid-fourth-suit.sh ab-results/diamond-rebid-fourth-suit \
#       >ab-results/diamond-rebid-fourth-suit.log 2>&1 < /dev/null &
#
# What is being tested.  Off, the node's only game rung is `3NT`, so every
# 13+ hand bids it whatever the fit.  At anchor `25aea82c` three hearts and
# 13+ alone are 88 boards, −561 plain per 409,600 (BBA's slam in hearts on 34).
#
#   off  --no-ns-diamond-rebid-fourth-suit
#   on   the default
#
# A first cut (`3♣` as a bare heart slam try, 3+ hearts 16+, opener asking
# on 14+) read plain +0.0006 / +0.0007, PD +0.0006 / +0.0008 pooled over
# seeds 1791352448 / 1791353601, the `4♥` raise carrying it; superseded by
# this fourth-suit tree before shipping.
#
# sha 682305a4-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791356713, diamond-rebid-fourth-suit)    plain +0.0006 / +0.0008, PD +0.0007 / +0.0008, 93 / 96 fired
#   seed 2 (1791357256, diamond-rebid-fourth-suit-2)  plain +0.0008 / +0.0009, PD +0.0009 / +0.0010, 103 / 106 fired
#
# Pooled plain +0.0007 / +0.0009, PD +0.0008 / +0.0009, eight of eight cells
# positive, every seed-2 cell outside its CI.  Split (plain, none): the `4♥`
# raise 26 / 25 boards +67 / +59, the floor's keycard ask over it 8 / 8
# boards +46 / +46; the `3♣` path's slams 14 / 17 boards +25 / +14, its game
# boards 45 / 53 −5 / +47; `3♣ - 3NT` passed 26 / 23 boards −22 / −7 (opener's
# unjumped 19-counts).  **Shipped default-on 2026-10-07.**
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-diamond-rebid-fourth-suit.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for diamond-rebid-fourth-suit)

log "=== diamond-rebid-fourth-suit A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-diamond-rebid-fourth-suit
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== diamond-rebid-fourth-suit A/B done"
