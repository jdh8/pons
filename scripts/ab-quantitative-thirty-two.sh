#!/bin/sh
# ab-quantitative-thirty-two.sh — the quantitative notrump lanes bid `6NT`
# on a certain 32 HCP, vs BBA, four cells, plain DD + perfect defense from
# one solve, plus SD-PD.  Arms are sequential and share one persistent fresh
# seed.
#
#   BOARDS=1638400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-quantitative-thirty-two.sh ab-results/quantitative-thirty-two \
#       >ab-results/quantitative-thirty-two.log 2>&1 < /dev/null &
#
# Why.  The invite bands are two points wide and opener accepts only on a
# maximum, so 32 and 33 combined stop in `4NT`.  On the BBA anchor 0772f1c4
# (docs/bba-gap-campaign.md, round-2 cut, lead 2) our `4NT` where BBA bids
# `6NT` is 58 rows (29 deals) per 409,600 boards, -408 IMPs plain / -399 PD:
# every loser has opener passing the invite on 15-16 (accept gate 17 facing
# 16-17; 21 facing 11-12 over `2NT`).  Lead 3's `4NT` half is the other
# end: the 12-count invite over the 18-19 `2NT` rebid, raised on 19 to a
# 31-point `6NT`, 22 rows -166.  A balanced 18-20 with no major bids `3NT`
# over `1NT` today (round 1, not in that cut).
#
#   base two-point invites, maximum-only accepts
#        (`--no-ns-quantitative-thirty-two`; the default when this ran, the
#        flags flipped with the default 2026-11-21)
#   on   notrump.quantitative_thirty_two:
#          1NT, direct and after Stayman   4NT 16, 6NT 17+ (direct 17-20), accept 16+
#          1NT, five-card major (transfer) 4NT 16, 6NT 17+, accept 16+
#          2NT, direct and after 3C - 3D   4NT 11, 6NT 12+, accept 21
#          2C - 2x - 2NT (shifted)         4NT 9, 6NT 10+, accept 23+
#          the 18-19 2NT rebid             4NT 13, 6NT 14+, accept 19
#
# Not in this arm: the two checkback ladders over the `2NT` rebid (still
# 12-13), a 15-count invite (BBA's band), a `5NT` pick-a-slam on the
# five-card major, the interfered tails (the floor's, as before).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-21 (6602b8ee-dirty, SEED_BASE 1795249359375984672, 1,638,400
# boards per cell, vs BBA): WIN 4/4, plain, PD and SD-PD all agree.
#
#   cell  fired  plain   PD      IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   none  827    +4082   +4040   +0.0025 ± 0.0003            +4516 / +4529
#   ns    865    +4893   +4865   +0.0030 ± 0.0004            +5230 / +5229
#   ew    821    +4071   +4029   +0.0025 ± 0.0003            +4238 / +4223
#   both  847    +4873   +4835   +0.0030 ± 0.0004            +5226 / +5217
#
# About +4.9 (we are non-vul) to +5.8 (we are vul) IMPs per fired board.
# Isolation gate passed in every cell (all divergent boards opened by us,
# game reached in both arms on every one).  By lane, the four cells summed
# (3,360 fired, +17,919 plain / +17,773 PD; `ab-dump-diff --jsonl`):
#
#   lane                                   fired  plain   per fired
#   1NT - 6NT where base bid 3NT (18-20)   508    +5620   +11.1
#   1NT - 6NT where base bid 4NT (17)      466    +2666   +5.7
#   1NT - 4NT, opener accepts on 16        289    +1581   +5.5
#   1NT, Stayman lanes (6NT or accept)     441    +2829   +6.4
#   1NT, transfer lanes (6NT, 6M, accept)  401    +3511   +8.8
#   2NT and 2C - 2D - 2NT lanes            272    +1796   +6.6
#   18-19 2NT rebid, 12 bids 3NT           983    -84     -0.1
#
# The rebid row is a wash on its own (PD +10), as docs/notrump-slam.md's
# coin flip at 31 says; it ships with the knob as measured.  The two
# checkback ladders stay 12-13.
#
# Resumable; SEED_BASE persists in $R/quantitative-thirty-two.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-quantitative-thirty-two.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for quantitative-thirty-two)

log "=== quantitative-thirty-two A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-quantitative-thirty-two
    arm on "$vul"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-quantitative-thirty-two
done
log "=== quantitative-thirty-two A/B done"
