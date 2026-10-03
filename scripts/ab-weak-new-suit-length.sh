#!/bin/sh
# ab-weak-new-suit-length.sh — how long responder's weak new suit over their
# double is, `1o (X) 2y` (`competition.weak_new_suit_length`).
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-weak-new-suit-length.sh ab-results/weak-new-suit-length \
#       >ab-results/weak-new-suit-length.log 2>&1 < /dev/null &
#
# What is being tested.  We bid `2y` on any five-card suit and 6–9 points;
# BBA's book wants six (`probe-bba-book --prefix "1♠ (X)"`: `2♣(5-10 6+♣)`)
# and reaches the call about a tenth as often.  In the 2026-10-03 anchor
# (`ab-results/anchor/2026-10-03-46d0dc14`, 204,800 boards/vul) the cell
# "we bid 2y, BBA passes" is 265 boards: the 199 five-card hands cost
# −2.83 plain / −4.66 PD IMPs each, the 64 six-card hands *gain*
# +2.56 / +1.80.
#
#   five  --ns-weak-new-suit-length 5: `2y` on 5+ cards
#   six   the default since 2026-10-04: the five-card hands fall through to
#         the natural `1NT` (6–9 HCP) or pass
#
# Hypothesis (pre-run): ~0.05% fired, +2…4 IMPs/fired — about +0.001 plain,
# +0.002 PD IMPs/board.  The `1NT` leg is the doubt: the anchor's
# "we bid 1NT, BBA passes" cell is itself negative (−0.98 / −1.62 over 169),
# so read the on-arm `1NT` and `Pass` buckets separately.
#
# SHIPPED default 6 2026-10-04 (SEED_BASE 1791055789, 819,200 boards/arm/
# vul, `ab-results/weak-new-suit-length`): plain +0.0006 ±0.0003 none /
# +0.0008 ±0.0004 both, PD +0.0009 ±0.0004 / +0.0011 ±0.0005; 665/684 fired
# (0.08%), +0.7…+1.4 IMPs/fired.  By the six arm's call (`probe-divergence
# --imps`): `1NT` 558/583 boards, plain +0.69/+0.87, PD +0.95/+1.14 per
# board; pass 87/82 boards, plain +0.62/+2.21, PD +2.38/+3.90.  Run with the
# knob as an opt-in `--ns-weak-new-suit-length 6` on arm; the flags below
# are the post-flip spelling.
R=${1:?usage: ab-weak-new-suit-length.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for weak-new-suit-length)

log "=== weak-new-suit-length A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm five "$vul" --ns-weak-new-suit-length 5
    arm six "$vul"
    gatepair six five "$vul"
    diffpair six five "$vul"
done
log "=== weak-new-suit-length A/B done"
