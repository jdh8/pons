#!/bin/sh
# ab-forcing-notrump-doubleton-raise.sh — responder's `3M` limit raise over
# opener's six-card `2M` rebid accepts a doubleton
# (`rebid.forcing_notrump_doubleton_raise`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-forcing-notrump-doubleton-raise.sh ab-results/fnt-doubleton-raise \
#       >ab-results/fnt-doubleton-raise.log 2>&1 < /dev/null &
#
# What is being tested.  After `1M - 1NT - 2M` the raise wants three cards, so
# 10–12 with a doubleton bids the `2NT` invite (11–12) or passes, and opener's
# `3NT` acceptance plays the 6-2 fit in notrump.  BBA raises to `3♠` on 10–12
# with 2–3 spades (or `4♠` on 11–12): anchor `46d0dc14`, our `2NT` vs its `4♠`
# over `1♠ - 1NT - 2♠` is 136 rows, −576 plain / −564 PD per 409,600 boards.
#
#   off  --no-ns-forcing-notrump-doubleton-raise (three-card raise)
#   on   the shipped default
#
# Each new seed wants a fresh results dir.  Sha 1cdea547-dirty, 204,800
# boards/arm/vul, isolation gate passed.  IMPs/board none / both:
#
#   seed 1 (1791218827, fnt-doubleton-raise)    plain +0.0019 / +0.0034, PD +0.0020 / +0.0034, 254 / 286 fired
#   seed 2 (1791219358, fnt-doubleton-raise-2)  plain +0.0022 / +0.0034, PD +0.0022 / +0.0033, 245 / 280 fired
#   pooled                                      plain +0.0020 / +0.0034, PD +0.0021 / +0.0033
#
# +1.5 to +2.5 IMPs per fired; all eight cells positive outside their CIs.
# **Shipped default-on 2026-10-06.**  (Measured with the knob default off and
# `on` armed by the retired `--ns-forcing-notrump-doubleton-raise`.)
R=${1:?usage: ab-forcing-notrump-doubleton-raise.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for fnt-doubleton-raise)

log "=== fnt-doubleton-raise A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-forcing-notrump-doubleton-raise
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== fnt-doubleton-raise A/B done"
