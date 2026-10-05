#!/bin/sh
# ab-two-over-one-side-suit-first.sh — after a 2/1, opener's six-card major
# with extras shows a four-card side suit before jumping to `3M`
# (`rebid.two_over_one_side_suit_first`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-side-suit-first.sh ab-results/side-suit-first \
#       >ab-results/side-suit-first.log 2>&1 < /dev/null &
#
# What is being tested.  After `1M - 2x`, six of the major and `points(15..)`
# jumps to `3M` (weight 1.7) whatever else the hand holds, and responder's
# table over the jump has only `4M` and `3NT`: `1♥ - 2m - 3♥ - 4♥` buries the
# 4-4 spade fit `response.two_over_one_minor_before_spades` left for round two
# (that A/B's worst board, −14).  On: the same hand with a four-card suit
# outside the two bid shows it at the jump's weight (`2♠`, or a three-level
# new suit below responder's); the jump is left one-suited.
#
#   off  --no-ns-two-over-one-side-suit-first (the jump whatever the side suit)
#   on   the shipped default
#
# sha 04b2efd3-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both (the pooled row and the lanes are seeds 1–2):
#
#   seed 1 (1791200237, side-suit-first)    plain +0.0003 / +0.0004, PD +0.0002 / +0.0004, 102 / 106 fired
#   seed 2 (1791200766, side-suit-first-2)  plain +0.0004 / +0.0006, PD +0.0004 / +0.0006,  97 / 100 fired
#   pooled                                  plain +0.00034 / +0.00053, PD +0.00029 / +0.00049
#
#   seed 3 (1791204922, side-suit-first-3)  plain +0.0001 / −0.0001, PD +0.0001 / −0.0002,  98 / 104 fired
#
# Seeds 1–2: all eight cells positive, each inside its CI (±0.0009–0.0011);
# +0.6 to +1.1 IMPs per fired.  Wash/wash, **shipped default-on 2026-10-05**
# on naturalness (BBA and BEN both bid `2♠` on 4♠6♥ with 15+, never the
# jump).  Seeds 1–2 were measured with the knob default off and `on` armed by
# the retired `--ns-two-over-one-side-suit-first`; seed 3 is the shipped
# build, against a default that also plays
# `rebid.two_over_one_reverse_extras`, and confirms the wash.  By
# lane, pooled plain IMPs none / both (boards): `2♥` over `1♠` +102 / +151
# (46 / 48), a two-level minor +40 / +60 (52 / 53), a three-level minor +37 /
# +63 (50 / 51), later-round reading drift +4 / +5 (26 / 28) — and `2♠` over
# `1♥`, the lane that prompted it, −42 / −61 (25 / 26), negative on both
# seeds.  When responder raises `2♠` (6 boards, −44 both) opener's third call
# is `4NT` every time: `second_suit::opener_third_agree` asks on
# `points(15..)`, which this hand holds by construction.  Each new seed wants
# a fresh results dir.
R=${1:?usage: ab-two-over-one-side-suit-first.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for side-suit-first)

log "=== side-suit-first A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-two-over-one-side-suit-first
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== side-suit-first A/B done"
