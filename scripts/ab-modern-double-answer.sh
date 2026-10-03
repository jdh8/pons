#!/bin/sh
# ab-modern-double-answer.sh — opener's answer to the Modern negative double
# of a one-level major overcall (`competition.modern_double_answer`,
# docs/next-steps.md, the `1m (1♥) X -` lane).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-modern-double-answer.sh ab-results/modern-double-answer \
#       >ab-results/modern-double-answer.log 2>&1 < /dev/null &
#
# What is being tested.  Off, the instinct floor answers and reads the double
# as game values: `4♠` on any fit from 11 points, `3NT` on 13 with a stopper,
# a penalty pass with four of their suit.  Anchor `46d0dc14` vs BBA, per
# 409,600 boards: 507 rows, −1,323 plain / −2,726 PD.  On, the table BBA and
# BEN agree on: `2M` 12–14 / `3M` 15–16 / `4M` 17+ with four, the major on
# three, `1NT` 12–14 / `2NT` 18–19 with a stopper, a long minor, no pass.
#
#   off  --no-ns-modern-double-answer (the floor)
#   on   the default since 2026-10-03
#
# Hypothesis (pre-run): the doc's ceiling, about +0.003 plain / +0.007 PD per
# board, at roughly 1,000 fired per 204,800 (both siblings) — one seed should
# resolve the PD half.
#
# SHIPPED default-on 2026-10-03 (SEED_BASE 1791031962,
# 204,800 boards/arm/vul, `ab-results/modern-double-answer`), IMPs/board
# none / both: plain +0.0019 ±0.0012 / +0.0023 ±0.0016, PD +0.0059 ±0.0015 /
# +0.0065 ±0.0021; 592 / 608 fired, every one a board we opened.  Run with
# the knob as an opt-in `--ns-modern-double-answer` on arm; the flags below
# are the post-flip spelling.
R=${1:?usage: ab-modern-double-answer.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for modern-double-answer)

log "=== modern-double-answer A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-modern-double-answer
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== modern-double-answer A/B done"
