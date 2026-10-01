#!/bin/sh
# ab-major-raise-slam-try.sh — opener's slam try over `1M - 2M` through a
# long-suit game try (`response.major_raise_slam_try`, docs/next-steps.md).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-major-raise-slam-try.sh ab-results/major-raise-slam-try \
#       >ab-results/major-raise-slam-try.log 2>&1 < /dev/null &
#
# What is being tested.  The book asks `4NT` over a single raise on 22+
# support points.  `probe-seam-lookahead --trace 4NT` (600,000 self-play
# deals, seed 1790874659, 378 such hands) shows where that ends: `5M` going
# down about one time in five, or `6M` making 58%.  Through a long-suit try
# the same hands play `4M`, or `6M` only opposite an accept, making 72%.
# With the knob on in that probe, the 340 hands holding a four-card side suit
# gain +0.98 plain / +1.28 PD (spades, 236) and +0.64 / +0.75 (hearts, 104)
# IMPs each — about +0.0005 / +0.0006 IMPs/board, self-play.
#
#   try   the default since 2026-10-02: a 22+ opener with a four-card side
#         suit makes the long-suit try there; game over a decline (the
#         existing 18+ rule), the floor's slam judgement over an accept.  A
#         one-suited 22+ hand still asks.
#   off   `--no-ns-major-raise-slam-try`: `4NT` on any 22+
#
# Gate, fixed before the run: the knob ships default-on iff, pooled over the
# seeds, plain DD is a win at both vulnerabilities, or plain a wash and PD a
# win (docs/measurement.md decision table), with every isolation gate passed.
#
# SHIPPED default-on 2026-10-02 (plain wash, PD win).  Three seeds
# (1790883190, 1790883714, 1790884244; `major-raise-slam-try{,-r2,-r3}`),
# 204,800 boards/arm/vul each, every isolation gate passed, run while the
# knob was still opt-in (the on arm was `--ns-major-raise-slam-try`, the off
# arm the bare default).  IMPs/board none / both:
#
#   seed 1   plain +0.0006 ±0.0007 / +0.0007 ±0.0008, 57 / 65 fired
#            PD    +0.0006 ±0.0007 / +0.0007 ±0.0009
#   seed 2   plain +0.0002 ±0.0007 / +0.0002 ±0.0008, 60 / 67 fired
#            PD    +0.0003 ±0.0007 / +0.0002 ±0.0009
#   seed 3   plain +0.0004 ±0.0007 / +0.0005 ±0.0009, 64 / 66 fired
#            PD    +0.0004 ±0.0007 / +0.0006 ±0.0009
#   pooled   plain +0.00038 ±0.00040 / +0.00044 ±0.00049  (614,400 boards/vul)
#            PD    +0.00044 ±0.00041 / +0.00052 ±0.00050  (12/12 cells > 0)
#
# Where it comes from (pooled, 181 / 198 fired): the declined try, 78 / 88
# boards at +2.69 / +2.57 plain and +3.03 / +2.95 PD each — the off arm is
# at the five level or in a slam opposite a minimum.  After an accept opener
# passes on 77 / 90 boards and that is a wash against the ask (+0.10 / +0.41
# plain).  They act over the try on 17 / 10 boards, +0.29 / −1.80 plain.
R=${1:?usage: ab-major-raise-slam-try.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for major-raise-slam-try)

log "=== major-raise-slam-try A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-major-raise-slam-try
    arm try "$vul"
    gatepair try off "$vul"
    diffpair try off "$vul"
done
log "=== major-raise-slam-try A/B done"
