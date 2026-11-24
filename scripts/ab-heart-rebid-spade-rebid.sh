#!/bin/sh
# ab-heart-rebid-spade-rebid.sh — responder's own spade suit over `1♥ - 1♠ - 2♥` on a misfit,
# vs BBA, four cells, plain DD + perfect defense from one solve, plus SD-PD.
# Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh heart-rebid-spade-rebid \
#       scripts/ab-heart-rebid-spade-rebid.sh ab-results/heart-rebid-spade-rebid
#
# Why.  The residue of ab-heart-spade-tails.sh's `misfit` arm: responder's
# table over `1♥ - 1♠ - 2♥` has no spade call, so a six-spade misfit passes
# `2♥` or bids notrump.  BBA's book there: `2♠` 6-10, `3♠` 11+ (forcing),
# `4♠` on seven, all on at most one heart.
#
#   base   the default book (knob off)
#   spade  rebid.heart_rebid_spade_rebid, all on at most one heart:
#          `2♠` 6+♠ and <=9 HCP, to play (opener passes);
#          `3♠` six spades and 10-12 HCP (opener raises on three, or two and
#          13+; `3NT` on a short spade and 15+ HCP; else passes);
#          `4♠` seven spades and 10+ HCP, or six and 13+.
#
# Sizing (2026-11-25, the shipped `misfit-none` arm, 1,638,400 boards, 437
# misfit boards with 6+♠; flat contracts priced by ab-dump-diff against the
# contract we reach today, plain DD, none / both):
#
#   6♠ <=9    97 bd  2♠ +9 / +6      (4♠ -237 / -333)
#   6♠ 10-11 101 bd  3♠ invite +193 / +270 (PD +173 / +235)
#   6♠ 12     38 bd  3♠ invite +50 / +42   (PD +74 / +72)
#   6♠ 13+   113 bd  4♠ +109 / +124  (the invite -84 / -212)
#   7♠ <=9    29 bd  2♠ +63 / +78
#   7♠ 10-12  24 bd  4♠ +95 / +117
#   7♠ 13+    35 bd  4♠ +43 / +49
#
# The invite priced opener's "doubleton and 13+ points" as 12+ HCP.  With two
# or more hearts every spade contract lost to the heart raise, so the raise
# keeps its rank.  Expect about +550 / +700 IMPs per cell on ~440 fired.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/heart-rebid-spade-rebid.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-heart-rebid-spade-rebid.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-rebid-spade-rebid)

log "=== heart-rebid-spade-rebid A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm spade "$vul" --ns-heart-rebid-spade-rebid
    gatepair spade base "$vul" || log "isolation gate spade vs base ($vul) FAILED, scoring anyway"
    diffpair spade base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.spade.vs.base.$vul.plain.txt")
    log "pre-count spade vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff spade base "$vul" --on-ns-heart-rebid-spade-rebid
done
log "=== heart-rebid-spade-rebid A/B done"
