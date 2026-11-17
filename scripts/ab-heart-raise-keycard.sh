#!/bin/sh
# ab-heart-raise-keycard.sh — hearts agreed at `1♠ - 2♥ - 3♥`: responder's
# keycard ask or `4♥`, vs BBA, four cells, plain DD + perfect defense from one
# solve.  Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=3276800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-heart-raise-keycard.sh ab-results/heart-raise-keycard \
#       >ab-results/heart-raise-keycard.log 2>&1 < /dev/null &
#
# Why.  At `1♠ - 2♥ - 3♥ -` responder's table has no heart call: `3♠` on
# three spades, else the no-fit notrump ladder.  SIZED 2026-11-17 on the
# `ab-results/two-over-one-opener-invite` on-arm (our table A against BBA's
# table B on the same cards, `ab-dump-diff`, 3,276,800 boards per cell): 640 /
# 673 / 652 / 674 boards reach the node (none / ns / ew / both), -1025 / -1153
# / -1170 / -1307 plain IMPs (-1.60 to -1.94 per board reached, PD the same),
# -0.0003 to -0.0004 IMPs/board.  By responder's call over the four cells:
# `3NT` 638 fired -1973 (97 grands where BBA bids six, -1043; 231 slams where
# BBA bids game, -701), `3♠` 424 -999, the floor's `4♣` 712 -911, `4♥` 143
# -353, `4♦` 205 +73.
#
#   base  game_force.heart_raise_keycard = None (the default)
#   k16   Some(16): `4NT` RKCB on 16+ points, else `4♥`; opener asks over the
#         sign-off on 17+
#   k18   Some(18)
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# VERDICT 2026-11-17 (sha 1fc09115-dirty, SEED_BASE 1794846341798324340,
# 3,276,800 bd/cell; the four cells are the same deals, one sample):
#   k16  +79 ±194 / +108 ±244 / +89 ±193 / +120 ±242 plain IMPs (none / ns /
#        ew / both), 153 / 171 / 151 / 166 fired, PD within 3 IMPs:
#        non-inferior, no win.
#   k18  -154 ±192 / -219 ±246 / -144 ±191 / -168 ±241: non-inferior against
#        δ, negative every cell.
# Stays opt-in, default None.  The loss bucket is `3NT` -> `4♥` passed out
# (k16 none: -143 on 31): opener's ask over the sign-off (17+) is too high.
# CORRECTION to the sizing above: it counted BBA's own East-West auctions
# (the "floor's `4♣`" is BBA's cue bid).  Our side reaches the node on 207
# boards per cell; base passes the forcing raise on 9 of them.
#
# Resumable; SEED_BASE persists in $R/heart-raise-keycard.seed.  Iron rule:
# do NOT rebuild binaries while this runs.
R=${1:?usage: ab-heart-raise-keycard.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-raise-keycard)

log "=== heart-raise-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    for k in 16 18; do
        arm "k$k" "$vul" --ns-heart-raise-keycard "$k"
        gatepair "k$k" base "$vul"
        diffpair "k$k" base "$vul"
    done
done
log "=== heart-raise-keycard A/B done"
