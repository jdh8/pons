#!/bin/sh
# ab-preemptive-minor-raise.sh — opener's rebid over our weak `1m - 3m`
# (`response.preemptive_minor_raise_pass`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-preemptive-minor-raise.sh ab-results/preemptive-minor \
#       >ab-results/preemptive-minor.log 2>&1 < /dev/null &
#
# What is being tested.  Without a node the instinct floor's raise rung bids
# `4m` on any 13+ hand over the preemptive `3m` (5+ support, ≤9 support
# points).  Anchor `46d0dc14` vs BBA, per 409,600 boards: `1♣ - 3♣ - 4♣`
# where BBA passes or bids `3NT` 155 rows (−658 plain / −886 PD), `1♦ - 3♦ -
# 4♦` 136 rows (−278 / −454).  BBA passes on 11–16, `3NT` 17–21 balanced.
#
#   off  the floor
#   p15  pass on ≤15 HCP, `3NT` on balanced 17+; the rest falls to the floor
#   p16  the same with the pass to 16 (BBA's ceiling)
#
# Seed 1 (1791011690, sha 46d0dc14-dirty = 61857a27), 204,800 boards/arm/vul,
# isolation gate passed.  IMPs/board none / both:
#
#   plain +0.0016 ±0.0004 / +0.0019 ±0.0006, 139 / 137 fired
#   PD    +0.0022 ±0.0006 / +0.0031 ±0.0008, +3.3 / +4.6 per fired
#
# Seed 2 (1791018922, sha 396497cb, `ab-results/preemptive-minor-2`), p15:
#
#   plain +0.0016 ±0.0005 / +0.0020 ±0.0006, 136 / 141 fired
#   PD    +0.0021 ±0.0006 / +0.0029 ±0.0008
#
# p16 vs off, two seeds pooled (409,600 boards/vul), 286 / 293 fired:
#
#   plain +0.0016 / +0.0020,  PD +0.0022 / +0.0030
#
# p16 vs p15: 11 / 15 fired over the two seeds, plain +6 / +13 IMPs, PD +10 /
# +21, positive in all eight cells — every one a 16-count passing `3m`.
# **Shipped default `Some(16)` 2026-10-03.**  A new seed wants a fresh
# results dir (`seed_for` reuses the one it finds).
R=${1:?usage: ab-preemptive-minor-raise.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for preemptive-minor)

log "=== preemptive-minor A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --ns-preemptive-minor-raise-pass off
    arm p15 "$vul" --ns-preemptive-minor-raise-pass 15
    gatepair p15 off "$vul"
    diffpair p15 off "$vul"
    arm p16 "$vul" --ns-preemptive-minor-raise-pass 16
    gatepair p16 off "$vul"
    diffpair p16 off "$vul"
    diffpair p16 p15 "$vul"
done
log "=== preemptive-minor A/B done"
