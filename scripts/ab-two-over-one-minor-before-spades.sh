#!/bin/sh
# ab-two-over-one-minor-before-spades.sh — over `1♥`, a game force with four
# spades and a longer minor bids the minor first
# (`response.two_over_one_minor_before_spades`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-minor-before-spades.sh ab-results/minor-before-spades \
#       >ab-results/minor-before-spades.log 2>&1 < /dev/null &
#
# What is being tested.  `1♠` (weight 1.7) outbids every 2/1, so 4=1=3=5 with
# 16 HCP responds `1♥ - 1♠`.  BBA bids the minor first and shows the spades
# next round: anchor `46d0dc14`, 419 rows, −535 plain / −474 PD per 409,600
# boards.  On: `1♠` yields to the 2/1 on exactly four spades, `points(13..)`
# and five or more clubs (or five diamonds and at most three clubs); responder
# bids a natural `2♠` after `1♥ - 2m - 2♦/2♥`; opener answers `3♥` on six
# hearts or `3NT` without four spades, and the floor places the spade fit.
#
#   off  --no-ns-two-over-one-minor-before-spades (1♠ first)
#   on   the shipped default
#
# sha 68ecd94a-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791196825, minor-before-spades)    plain +0.0002 / +0.0004, PD +0.0002 / +0.0003,  92 /  95 fired
#   seed 2 (1791197349, minor-before-spades-2)  plain +0.0003 / +0.0005, PD +0.0003 / +0.0005, 121 / 123 fired
#   pooled                                      plain +0.00025 / +0.00046, PD +0.00022 / +0.00042
#
# All eight cells positive, each inside its CI (±0.0007–0.0009); +0.4 to +0.9
# IMPs per fired.  Wash/wash, **shipped default-on 2026-10-05** on naturalness.
# (Measured with the knob default off and `on` armed by the retired
# `--ns-two-over-one-minor-before-spades`.)  Worst board left: after `1♥ - 2♦
# - 3♥` (opener's solid-six jump) responder has no `3♠` and raises to `4♥` on
# two, losing a 4-4 spade fit.  Each new seed wants a fresh results dir.
R=${1:?usage: ab-two-over-one-minor-before-spades.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for minor-before-spades)

log "=== minor-before-spades A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-two-over-one-minor-before-spades
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== minor-before-spades A/B done"
