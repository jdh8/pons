#!/bin/sh
# ab-minor-jump-notrump.sh — opener jumps to `3NT` over `1m - 1x` on a six-card
# minor and 18+ HCP (`rebid.minor_jump_notrump`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-minor-jump-notrump.sh ab-results/minor-jump-nt \
#       >ab-results/minor-jump-nt.log 2>&1 < /dev/null &
#
# What is being tested.  Off, every 16+ six-card minor jumps to `3m`, which
# has no ceiling, and responder's floor passes it on 5–7 HCP with game on
# (opener mostly 18–21).  BBA's `1♦ - 1♠ - 3♦` is 15–18, its `3NT` 18–21 on
# six diamonds.  The anchor at `46d0dc14` prices our `3m` vs its `3NT` at 667
# rows, −1,148 plain / −979 PD per 409,600 boards (`1♦ - 1♠` 191 rows,
# −360 / −296; `1♦ - 1♥` 185; `1♣ - 1♠` 141; `1♣ - 1♥` 120; `1♣ - 1♦` 30).
# On: 18+ HCP rebids `3NT` (read 18–21, six of the minor), `3m` keeps 16–17.
#
#   off  --no-ns-minor-jump-notrump
#   on   the shipped default
#
# sha 19d377f0-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791228890, minor-jump-nt)    plain +0.0013 / +0.0013, PD +0.0013 / +0.0013, 227 / 258 fired
#   seed 2 (1791229495, minor-jump-nt-2)  plain +0.0009 / +0.0009, PD +0.0005 / +0.0006, 241 / 255 fired
#
# Pooled plain +0.0011 / +0.0011, PD +0.0009 / +0.0009, every pooled cell
# outside its CI.  **Shipped default-on 2026-10-06.**  Both seeds were
# measured with the knob default off and `on` armed by the retired
# `--ns-minor-jump-notrump`.  Each new seed wants a fresh results dir.
R=${1:?usage: ab-minor-jump-notrump.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for minor-jump-nt)

log "=== minor-jump-nt A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-minor-jump-notrump
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== minor-jump-nt A/B done"
