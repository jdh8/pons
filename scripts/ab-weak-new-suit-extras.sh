#!/bin/sh
# ab-weak-new-suit-extras.sh — opener's strong hands over the weak new suit
# over their double, `1o (X) 2y -` (`competition.weak_new_suit_extras`, the
# follow-up to `weak_new_suit_rebid`; no 2/1 after their X).
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-weak-new-suit-extras.sh ab-results/weak-new-suit-extras \
#       >ab-results/weak-new-suit-extras.log 2>&1 < /dev/null &
#
# What is being tested.  Off, opener raises `2y` on 15+ with four and
# otherwise passes — 163 passes on 16+ HCP in the 819,200 boards of
# `ab-results/weak-new-suit-rebid` (66 with a six-card suit, 26 balanced
# 18–19, 21 with three-card support), e.g. `2♥` passed on
# AKQJ642.—.43.AQ52.  On: `4M` on a six-card major and 18+ points, `3o` on a
# six-card suit and 16+, `3y` on three-card support and 17+, `2NT` on a
# balanced 18–19; responder accepts on 8+, else passes.
#
#   off  --no-ns-weak-new-suit-extras
#   on   the default since 2026-10-04
#
# Hypothesis (pre-run): ~0.02% fired, +2…3 IMPs/fired — about +0.0005
# IMPs/board, at the edge of one seed's CI.
#
# SHIPPED default-on 2026-10-04 (SEED_BASE 1791050732, 819,200 boards/arm/
# vul, `ab-results/weak-new-suit-extras`): plain +0.0002 ±0.0001 none /
# +0.0003 ±0.0002 both, PD +0.0003 ±0.0002 both cells; +2.5…+3.0 IMPs/fired,
# 80/93 fired, 32 games reached only on.  Run with the knob as an opt-in
# `--ns-weak-new-suit-extras` on arm; the flags below are the post-flip
# spelling.
R=${1:?usage: ab-weak-new-suit-extras.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for weak-new-suit-extras)

log "=== weak-new-suit-extras A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-weak-new-suit-extras
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== weak-new-suit-extras A/B done"
