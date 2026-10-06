#!/bin/sh
# ab-heart-rebid-invite-accept.sh — opener accepts the `3♥` invite over
# `1♥ - 1♠ - 2♥` on the floor's fit-sum gate (`rebid.heart_rebid_invite_accept`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-heart-rebid-invite-accept.sh ab-results/heart-rebid-accept \
#       >ab-results/heart-rebid-accept.log 2>&1 < /dev/null &
#
# What is being tested.  Off, opener accepts responder's `3♥` (2+ hearts,
# 10–12) on 14+ points whatever its length, so a six-card 13 passes and the
# vulnerable game is left in `3♥`.  At anchor `25aea82c` our `3♥` vs BBA's `4♥`
# at `1♥ - 1♠ - 2♥ -` is 97 rows, −364 plain / −306 PD per 409,600 boards
# (both-vul −10 a board, none-vul +5).  On: 13+ points, or 12+ with seven
# hearts (own + 10 + trumps ≥ 31).
#
#   off  --no-ns-heart-rebid-invite-accept
#   on   the shipped default
#
# sha 74854683-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791317756, heart-rebid-accept)    plain +0.0005 / +0.0010, PD +0.0004 / +0.0007, 44 / 45 fired
#   seed 2 (1791318376, heart-rebid-accept-2)  plain +0.0004 / +0.0009, PD +0.0003 / +0.0006, 43 / 45 fired
#
# Pooled plain +0.0005 / +0.0010, PD +0.0004 / +0.0007, every pooled cell
# outside its CI.  **Shipped default-on 2026-10-07.**  Both seeds were
# measured with the knob default off and `on` armed by the retired
# `--ns-heart-rebid-invite-accept`.  Each new seed wants a fresh results dir.
R=${1:?usage: ab-heart-rebid-invite-accept.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-rebid-accept)

log "=== heart-rebid-accept A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-heart-rebid-invite-accept
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== heart-rebid-accept A/B done"
