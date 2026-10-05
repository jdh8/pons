#!/bin/sh
# ab-forcing-notrump-heart-raise.sh — responder raises opener's `2♥` after
# `1♠ - 1NT` (`rebid.forcing_notrump_heart_raise`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-forcing-notrump-heart-raise.sh ab-results/fnt-heart-raise \
#       >ab-results/fnt-heart-raise.log 2>&1 < /dev/null &
#
# What is being tested.  After `1♠ - 1NT - 2♥` responder has no heart raise:
# 10–12 with four hearts bids the `2NT` invite (11–12) or passes, and a weak
# hand with two spades and four hearts gives false preference to `2♠`.  BBA
# raises to `3♥` on 9–12 with 4+ hearts (`4♥` on shape): anchor `46d0dc14`, our
# `2NT` vs its `4♥` is 31 rows, −256 plain / −268 PD per 409,600 boards.
#
#   off  --no-ns-forcing-notrump-heart-raise (no heart raise)
#   on   the shipped default
#
# Each new seed wants a fresh results dir.  Sha 9576e040-dirty, 204,800
# boards/arm/vul, isolation gate passed.  IMPs/board none / both:
#
#   seed 1 (1791221520, fnt-heart-raise)    plain +0.0012 / +0.0019, PD +0.0013 / +0.0020, 126 / 131 fired
#   seed 2 (1791222320, fnt-heart-raise-2)  plain +0.0012 / +0.0018, PD +0.0016 / +0.0022, 112 / 132 fired
#   pooled                                  plain +0.0012 / +0.0018, PD +0.0014 / +0.0021
#
# +2.0 to +3.4 IMPs per fired; all eight cells positive outside their CIs.
# **Shipped default-on 2026-10-06.**  (Measured with the knob default off and
# `on` armed by the retired `--ns-forcing-notrump-heart-raise`.)
R=${1:?usage: ab-forcing-notrump-heart-raise.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for fnt-heart-raise)

log "=== fnt-heart-raise A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-forcing-notrump-heart-raise
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== fnt-heart-raise A/B done"
