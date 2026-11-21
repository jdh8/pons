#!/bin/sh
# ab-one-club-one-diamond-two-notrump.sh — opener's balanced 18–19 `2NT`
# ahead of `1M` after `1♣ - 1♦`, vs BBA, four cells, plain DD + perfect
# defense from one solve, plus SD-PD.  Arms are sequential and share one
# persistent fresh seed.
#
#   BOARDS=1638400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-one-club-one-diamond-two-notrump.sh ab-results/one-club-one-diamond-two-notrump \
#       >ab-results/one-club-one-diamond-two-notrump.log 2>&1 < /dev/null &
#
# Why.  Our `1♦` response to `1♣` is `4+ ♦, 6+ points, ≤3 ♥, ≤3 ♠`
# (`response.up_the_line`), so opener's `1M` there can never find a 4-4 fit.
# `1M` (weight 1.30) still outranks `2NT` (1.10), so a balanced 18–19 with a
# four-card major rebids `1M` and responder passes it on a minimum.  On the
# BBA anchor `0772f1c4` (shipping arm, 409,600 boards × 2 cells) that is 288
# rows (149 deals), −618 plain / −517 PD: responder passes `1M` on 150 rows
# with BBA in `3NT` on 130 (docs/bba-gap-campaign.md, "Shipping round-2, cut
# by auction", lead 1).
#
#   on   rebid.one_club_one_diamond_two_notrump_first: `2NT` at 1.40,
#        above `1M`, below the `3♦` raise (1.50)
#   base `1M` first (`--no-ns-one-club-one-diamond-two-notrump-first`; the
#        default when this ran, the flags flipped with the default
#        2026-11-21)
#
# Sized off the anchor: ~150 deals per 409,600, so ~600 fired per 1,638,400
# per cell; at the anchor's −2.1 IMPs per row, all of it coming back is about
# +1,250 IMPs per cell (+0.0008 IMPs/board).  Unfiltered: no filter reaches
# this prefix, and the fire rate does not need one.  The continuations are
# the existing `1♣ - 1♦ - 2NT -` table (`3NT` 6+, `4NT` 12–13, `6NT` 14+);
# its interfered tails are the floor's, as before the change.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-21 (92f53b5e-dirty, SEED_BASE 1795177221863966046, 1,638,400
# boards per cell, vs BBA): WIN 4/4, plain, PD and SD-PD all agree.
#
#   cell  fired  plain   PD      IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   none  746    +1434   +1196   +0.0009 ± 0.0002            +2133 / +2005
#   ns    815    +2629   +2232   +0.0016 ± 0.0003            +3517 / +3266
#   ew    742    +1416   +1172   +0.0009 ± 0.0002            +2036 / +1892
#   both  796    +2568   +2179   +0.0016 ± 0.0003            +3495 / +3244
#
# About +1.9 (non-vul) to +3.2 (vul) IMPs per fired board.  Isolation gate
# passed in every cell (0 boards opened by BBA).  The worst boards are the
# quantitative tail: `2NT - 4NT` passed where the baseline's `1M` route
# reached a making `6NT`, and `2NT - 4NT - 6NT` down where it stopped in
# `3NT`; untraced, as the net is a win in every cell.  Shipped default-on.
#
# Resumable; SEED_BASE persists in $R/one-club-one-diamond-two-notrump.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-one-club-one-diamond-two-notrump.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for one-club-one-diamond-two-notrump)

log "=== one-club-one-diamond-two-notrump A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-one-club-one-diamond-two-notrump-first
    arm on "$vul"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-one-club-one-diamond-two-notrump-first
done
log "=== one-club-one-diamond-two-notrump A/B done"
