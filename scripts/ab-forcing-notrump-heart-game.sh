#!/bin/sh
# ab-forcing-notrump-heart-game.sh — responder's game over `1♠ - 1NT - 2♥`
# (`rebid.forcing_notrump_heart_game`; lane (b) of docs/bba-gap-campaign.md).
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-forcing-notrump-heart-game.sh ab-results/fnt-heart-game \
#       >ab-results/fnt-heart-game.log 2>&1 < /dev/null &
#
# What is being tested.  Responder invites with `3♥` on four hearts and 10–12
# and opener accepts on 14+ points.  BBA bids game on those hands and invites
# on 8–9: anchor `ce114214`, our `3♥` vs its `4♥` −403 plain / −372 PD per
# 409,600 boards, 84% of the passed invites making game.
#
#   base  --no-ns-forcing-notrump-heart-game: `3♥` on 10–12, pass below
#   game  the shipped default: `4♥` on 10–12, `3♥` on 8–9 with four hearts,
#         opener passes the game
#
# Round 1 (2026-11-09, 9f8eb0d1-dirty, seed 1794213973288604725, 409,600
# boards per cell, unfiltered, 0 foreign in both gates) had a wider arm that
# also bid `4♠` on three spades and 10–12.  Whole arm plain +0.0008 ±0.0004 /
# +0.0019 ±0.0006, PD +0.0005 / +0.0013 (none / both, win 4/4).  By first
# divergent call, plain / PD IMPs:
#   `3♥` vs pass  +248 / +200, +480 / +378  (138 / 155 boards)
#   `4♥` vs `3♥`  +107 / +54,  +264 / +174  (81 / 86)
#   `4♠` vs `3♠`  −24 / −69,   +40 / −40    (41 / 49)
# So round 2 drops the `4♠` half; a selected arm, hence a fresh seed.
#
# Round 2 read (2026-11-09, 9f8eb0d1-dirty, seed 1794216015315708893, 0
# foreign in both gates), per board, none / both:
#   plain +0.0010 ±0.0004 / +0.0021 ±0.0006 (win 2/2, 222 / 247 fired)
#   PD    +0.0008 ±0.0004 / +0.0017 ±0.0006 (win 2/2)
#   `3♥` vs pass  +288 / +248, +573 / +498  (137 / 151 boards)
#   `4♥` vs `3♥`  +125 / +78,  +305 / +213  (85 / 96)
# → shipped default-on 2026-11-09.  (Measured with the knob default off and
# `game` armed by the retired `--ns-forcing-notrump-heart-game`.)
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-forcing-notrump-heart-game.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for fnt-heart-game)

log "=== fnt-heart-game A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" --no-ns-forcing-notrump-heart-game
    arm game "$vul"
    gatepair game base "$vul" || log "isolation gate game vs base ($vul) FAILED, scoring anyway"
    diffpair game base "$vul"
done
log "=== fnt-heart-game A/B done"
