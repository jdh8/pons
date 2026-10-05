#!/bin/sh
# ab-unbalanced-1nt-rebid.sh — a minimum 5m-4♥ rebids `1NT` over `1m - 1♠`
# (`rebid.unbalanced_1nt_rebid`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-unbalanced-1nt-rebid.sh ab-results/unbalanced-1nt \
#       >ab-results/unbalanced-1nt.log 2>&1 < /dev/null &
#
# What is being tested.  Off, `1♦ - 1♠ - 2♦` and `1♣ - 1♠ - 2♣` promise five,
# and responder's floor never shows four hearts over them.  BBA's `2m` promises
# six; its `1NT` is 11–16 on 0–3 spades.  The anchor at `46d0dc14` prices our
# `2♦` vs its `1NT` at 760 rows (−817 plain / −1,097 PD) and `2♣` vs `1NT` at
# 995 (−453 / −540) per 409,600 boards, every row a 5m-4♥ opener.  On: that
# minimum rebids `1NT`, and over XYZ's `2♦ - 2NT` responder shows four hearts
# with `3♥`.
#
#   off  --no-ns-unbalanced-1nt-rebid
#   on   the shipped default
#
# sha 79edf194-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791213731, unbalanced-1nt)    plain +0.0005 / +0.0004, PD +0.0001 / +0.0001, 448 / 486 fired
#   seed 2 (1791214349, unbalanced-1nt-2)  plain +0.0008 / +0.0006, PD +0.0005 / +0.0002, 463 / 491 fired
#
# Seed 1's `3♥` rung had no spade cap (a seven-spade hand bid `3♥` and lost
# its `4♠`); seed 2 is the capped build.  All eight cells positive, all
# inside their CIs: a wash with a positive lean.  **Shipped default-on
# 2026-10-05** on naturalness, jdh8's call: it is BBA's treatment.  Both seeds
# were measured with the knob default off and `on` armed by the retired
# `--ns-unbalanced-1nt-rebid`.  Each new seed wants a fresh
# results dir.
R=${1:?usage: ab-unbalanced-1nt-rebid.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for unbalanced-1nt)

log "=== unbalanced-1nt A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-unbalanced-1nt-rebid
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== unbalanced-1nt A/B done"
