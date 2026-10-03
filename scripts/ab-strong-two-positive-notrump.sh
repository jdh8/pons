#!/bin/sh
# ab-strong-two-positive-notrump.sh — the notrump count over the balanced
# `2NT` positive to our strong `2♣` (`rebid.strong_two_positive_notrump`,
# docs/next-steps.md item 2, the strong `2♣` lane).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-strong-two-positive-notrump.sh ab-results/strong-two-count \
#       >ab-results/strong-two-count.log 2>&1 < /dev/null &
#
# What is being tested.  Over `2♣ - 2NT` a hand with no five-card major is
# the floor's, and the floor's `6NT` there is the evaluator net's: it is bid
# on as little as 22 HCP opposite a positive that since 2026-10-01 starts at
# 7.  `probe-strong-two-grand` (8M uncontested self-play deals, seed 20261003,
# 6,314 boards in the lane): the direct `2♣ - 2NT - 6NT` is 2,860 boards and
# makes double-dummy on 58%; with opener at most 24 it makes on 19% / 36% /
# 57% / 77% opposite 7 / 8 / 9 / 10.
#
#   count  the default since 2026-10-03: opener bids `3NT` on up to 24 HCP,
#          `6NT` on 25–29, `7NT` on 30+; responder raises the `3NT` to six on
#          9+ and seven on 14+, the `6NT` to seven on 11+
#   off    `--no-ns-strong-two-positive-notrump`: the floor
#
# SHIPPED default-on 2026-10-03.  Three seeds (1791004115, 1791004618,
# 1791005119; `strong-two-count{,-r2,-r3}`), 204,800 boards/arm/vul each,
# every isolation gate passed, run while the knob was still opt-in (the on
# arm was `--ns-strong-two-positive-notrump`, the off arm the bare default).
# IMPs/board none / both:
#
#   seed 1   plain +0.0005 ±0.0008 / +0.0006 ±0.0010, 57 / 60 fired
#            PD    +0.0006 ±0.0008 / +0.0007 ±0.0010
#   seed 2   plain +0.0002 ±0.0007 / −0.0000 ±0.0008, 40 / 45 fired
#            PD    +0.0002 ±0.0007 / +0.0001 ±0.0009
#   seed 3   plain +0.0008 ±0.0008 / +0.0009 ±0.0010, 58 / 59 fired
#            PD    +0.0009 ±0.0008 / +0.0010 ±0.0010
#   pooled   plain +0.0005 ±0.0004 / +0.0005 ±0.0005   (614,400 boards/vul)
#            PD    +0.0006 ±0.0004 / +0.0006 ±0.0006   (11/12 cells > 0)
#
# The same-seed self-play pair (`probe-strong-two-grand`, 8M deals, with and
# without `--no-notrump-count`): 3,387 divergent boards, +0.00055 ±0.00013 /
# +0.00067 ±0.00015 IMPs/board double-dummy; `6NT` → `3NT` is 1,435 of them
# for +4,200 IMPs, the new sevens 46 for +306, the floor's sevens now stopped
# in six 80 for −78.
R=${1:?usage: ab-strong-two-positive-notrump.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-two-count)

log "=== strong-two-count A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off   "$vul" --no-ns-strong-two-positive-notrump
    arm count "$vul"
    gatepair count off "$vul"
    diffpair count off "$vul"
done
log "=== strong-two-count A/B done"
