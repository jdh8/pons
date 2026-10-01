#!/bin/sh
# ab-strong-two-grand.sh — the grand rung below a natural positive to our
# strong `2♣` (`rebid.strong_two_grand`, docs/next-steps.md item 2, the strong
# `2♣` lane).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-strong-two-grand.sh ab-results/strong-two-grand \
#       >ab-results/strong-two-grand.log 2>&1 < /dev/null &
#
# What is being tested.  Re-cutting the three `strong-two-loose` seeds by
# final contract (our `both` arm against BBA's table, the boards we open `2♣`
# and hear a positive), the lane still trails BBA by −2,316 plain / −2,828 PD
# per 1,228,800 boards, and missed grands are 235 rows, −2,022 / −2,010 of
# it.  Two holes:
#
#   - the book's king ask wants all three side kings (`2♣ - 2♠ - 4NT - 5♣ -
#     5NT - 6♦ - 6♠`) and has no ask at all over `5♦` or `5♥`;
#   - over a minor positive the keycard ask is the floor's, whose minor
#     ladder bids seven only on 37 combined points (`2♣ - 3♦ - 4NT - 5♣ -
#     6♦`).
#
# `probe-strong-two-grand` (8M uncontested self-play deals, 27,427 in the
# lane) prices the bar: with all five keycards and the trump queen, seven of
# a major makes double-dummy on 43% / 69% / 86% of the boards with one / two
# / three side kings, seven of a minor on 30% / 64% / 84%, against a
# break-even near 56–58%.  The same probe run with the knob on reads +0.0012
# / +0.0014 IMPs/board (none / both), 3,004 divergent, self-play.
#
#   grand  the default since 2026-10-01: every keycard ask below a positive
#          bids seven on two of the three side kings — at once with two of
#          the asker's own, else through `5NT` — and the same ladder answers
#          the floor's `4NT` over a minor positive (three kings wanted when
#          the asker has only three trumps)
#   off    `--no-ns-strong-two-grand`: the classic king ask, the floor's
#          minor ladder
#
# SHIPPED default-on 2026-10-01.  Three seeds (1790866672, 1790867173,
# 1790867674; `strong-two-grand{,-r2,-r3}`), 204,800 boards/arm/vul each,
# every isolation gate passed, run while the knob was still opt-in (the on
# arm was `--ns-strong-two-grand`, the off arm the bare default).  IMPs/board
# none / both:
#
#   seed 1   plain +0.0010 ±0.0007 / +0.0011 ±0.0009, 40 / 43 fired
#            PD    +0.0010 ±0.0007 / +0.0010 ±0.0009
#   seed 2   plain +0.0006 ±0.0009 / +0.0007 ±0.0011, 58 / 62 fired
#            PD    +0.0005 ±0.0009 / +0.0007 ±0.0011
#   seed 3   plain +0.0012 ±0.0008 / +0.0016 ±0.0010, 53 / 56 fired
#            PD    +0.0012 ±0.0008 / +0.0016 ±0.0010
#   pooled   plain +0.0009 ±0.0005 / +0.0012 ±0.0006   (614,400 boards/vul)
#            PD    +0.0009 ±0.0005 / +0.0011 ±0.0006   (12/12 cells > 0)
#
# 142 / 152 new grands, 71% making double-dummy: majors 115 / 122 boards,
# +476 / +638 plain IMPs; minors 27 / 30 boards, +97 / +71.  Grand shave
# (docs/measurement.md, 3–10% of the DD-making grands failing): plain
# +0.0008…+0.0005 / +0.0010…+0.0006.
R=${1:?usage: ab-strong-two-grand.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-two-grand)

log "=== strong-two-grand A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off   "$vul" --no-ns-strong-two-grand
    arm grand "$vul"
    gatepair grand off "$vul"
    diffpair grand off "$vul"
done
log "=== strong-two-grand A/B done"
