#!/bin/sh
# ab-fourth-suit-keycard.sh — responder's `4NT` keycard ask over opener's
# answer to the fourth suit in `1♥ - 1♠ - 2♣ - 2♦` (16+ points; spades over
# the `2♠` delayed raise with 5+ spades, hearts over every other answer with
# 3+ hearts) (`rebid.fourth_suit_keycard`).
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-fourth-suit-keycard.sh ab-results/fourth-suit-keycard \
#       >ab-results/fourth-suit-keycard.log 2>&1 < /dev/null &
#
# What is being tested.  Off, responder's placement tops out at game.  At
# anchor `25aea82c` (shipping arm) our game against BBA's slam or grand in
# the lane is 104 boards, −909 plain per 409,600 (hearts 14 deals, spades 8,
# clubs 17 — clubs out of this ask's reach).
#
#   off  the default
#   on   --ns-fourth-suit-keycard
#
# sha 50914634-dirty, 204,800 boards/arm/vul, isolation gate passed.
# IMPs/board none / both:
#
#   seed 1 (1791372729, fourth-suit-keycard-v2)    plain +0.0001 / +0.0001, PD +0.0001 / +0.0001, 14 / 14 fired
#   seed 2 (1791373228, fourth-suit-keycard-v2-2)  plain +0.0002 / +0.0003, PD +0.0002 / +0.0003, 13 / 14 fired
#
# Every cell positive, every cell inside its CI: a wash; a convention on a
# wash stays opt-in.  The losers are 28–31 HCP slams on four or five
# keycards failing DD.  A first cut without the two-keycard gate
# (fourth-suit-keycard{,-2}: 1791371551 plain +0.0004 / +0.0004, PD the
# same; 1791372096 +0.0001 / +0.0001) bid 6♥ off two when a one-keycard
# asker heard 5♠.
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-fourth-suit-keycard.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for fourth-suit-keycard)

log "=== fourth-suit-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on "$vul" --ns-fourth-suit-keycard
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== fourth-suit-keycard A/B done"
