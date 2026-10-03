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
#   off  the floor (default)
#   p15  pass on ≤15 HCP, `3NT` on balanced 17+; the rest falls to the floor
R=${1:?usage: ab-preemptive-minor-raise.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for preemptive-minor)

log "=== preemptive-minor A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm p15 "$vul" --ns-preemptive-minor-raise-pass 15
    gatepair p15 off "$vul"
    diffpair p15 off "$vul"
done
log "=== preemptive-minor A/B done"
