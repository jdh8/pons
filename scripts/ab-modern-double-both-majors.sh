#!/bin/sh
# ab-modern-double-both-majors.sh — opener's answer to the both-majors Modern
# double `1♣ (1♦) X -`, and the doubler's second turn over it
# (`competition.modern_double_both_majors`, docs/next-steps.md, the sibling
# of the shipped `1m (1M) X -` table).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-modern-double-both-majors.sh ab-results/modern-double-both-majors \
#       >ab-results/modern-double-both-majors.log 2>&1 < /dev/null &
#
# What is being tested.  Off, the floor answers and reads the double as game
# values, as in the sibling: `4♠` on KJ75.Q43.43.AJ62, `4♥` on a 4-4
# minimum, `3NT` on a 13-count.  On, the sibling's table (`2M`/`3M`/`4M` by
# strength, spades on a tie; the cheaper major on three; `1NT` 12–14 / `2NT`
# 18–19 with a ♦ stopper; `2♣` on five) plus the doubler's second turn over
# `1M`/`2M`/`1NT`/`2♣` — the net floor raised `2♠` to `3♠` on 7 HCP with no
# Pass in its top five, and bid notrump with no ♦ stopper.
#
#   off  --no-ns-modern-double-both-majors (the floor)
#   on   the default since 2026-10-03
#
# Hypothesis (pre-run): the same disease as the sibling at a rarer node
# (BEN's census: `1NT` 20%, `1♥` 16%, `2♣` 15%, `2♥`/`2♠` 12% each) — a
# plain win near +0.001 and a PD win, a few hundred fired per 204,800.
#
# SHIPPED default-on 2026-10-03 (SEED_BASE 1791035800, 204,800
# boards/arm/vul, `ab-results/modern-double-both-majors`), IMPs/board none /
# both: plain +0.0002 ±0.0004 / +0.0002 ±0.0005 (wash), PD +0.0007 ±0.0005 /
# +0.0008 ±0.0007 (win) — the decision table's shippable wash/win row.  Rarer
# than hoped: 56 / 56 fired, every one a board we opened.  The plain wash is
# the floor's game overbids sometimes making (41 boards reach game only off).
# Run with the knob as an opt-in `--ns-modern-double-both-majors` on arm; the
# flags below are the post-flip spelling.
R=${1:?usage: ab-modern-double-both-majors.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for modern-double-both-majors)

log "=== modern-double-both-majors A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-modern-double-both-majors
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== modern-double-both-majors A/B done"
