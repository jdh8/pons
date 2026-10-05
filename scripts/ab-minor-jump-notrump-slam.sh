#!/bin/sh
# ab-minor-jump-notrump-slam.sh — responder's `4m` slam try over `1m - 1x -
# 3NT` (`rebid.minor_jump_notrump_slam_try`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-minor-jump-notrump-slam.sh ab-results/minor-jump-nt-slam \
#       >ab-results/minor-jump-nt-slam.log 2>&1 < /dev/null &
#
# What is being tested.  Over opener's `3NT` (six of the minor, 18–21) the
# floor bids `6NT` on 13+ and passes the rest, so a 9–12 count with a fit
# plays `3NT` when the minor slam is on.  BBA raises to `4m` as a slam try;
# the anchor at `46d0dc14` prices the lane at 119 rows where it reaches slam
# and we stop in game.  On: `4m` on 2+ support, floor..=12 HCP, no six-card
# suit of responder's own; opener `4NT` RKCB on 20+, else `5m`.
#
#   off  --ns-minor-jump-notrump-slam-try off
#   t8   --ns-minor-jump-notrump-slam-try 8   (BBA's floor)
#   t9   the shipped default
#
# sha ceb1331c-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791231712, minor-jump-nt-slam)    t9  plain +0.0003 / +0.0007, PD +0.0003 / +0.0007, 45 / 52 fired
#                                              t11 plain +0.0001 / +0.0003, PD +0.0001 / +0.0003, 10 / 12 fired
#   seed 2 (1791232692, minor-jump-nt-slam-2)  t9  plain +0.0002 / +0.0003, PD +0.0002 / +0.0003, 70 / 74 fired
#                                              t8  plain +0.0001 / +0.0002, PD +0.0001 / +0.0002, 88 / 95 fired
#
# t9 pooled plain +0.0002 / +0.0005, PD +0.0002 / +0.0005: every cell
# positive, both-vul outside its CI, none-vul a positive wash.  **Shipped
# default-on `Some(9)` 2026-10-06.**  Both seeds ran with the knob default
# off, so `off` was the bare default then.  Seed 1 ran t9 and t11; t11 was
# dropped for t8 from seed 2 on.  Worst t9 boards: opener's 18–19 sign-off
# `5♦` one down where `3NT` made.
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-minor-jump-notrump-slam.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for minor-jump-nt-slam)

log "=== minor-jump-nt-slam A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --ns-minor-jump-notrump-slam-try off
    arm t8 "$vul" --ns-minor-jump-notrump-slam-try 8
    arm t9 "$vul"
    for a in t8 t9; do
        gatepair "$a" off "$vul"
        diffpair "$a" off "$vul"
    done
done
log "=== minor-jump-nt-slam A/B done"
