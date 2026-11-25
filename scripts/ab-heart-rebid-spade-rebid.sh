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
#   base   the book before this READ (the knob off: `--no-ns-…`)
#   spade  rebid.heart_rebid_spade_rebid (the default since this READ), all
#          on at most one heart:
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
# READ 2026-11-25 (35b2a50b, the queue snapshot of 229f4b99; SEED_BASE
# 1795562137919430161, 1,638,400 boards per cell, vs BBA): win 4/4 on plain,
# PD, plain SD and SD-PD; isolation gate PASSED in all four cells.
#
#   cell  fired  plain  PD     IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   none  461    +729   +832   +0.0004 ± 0.0001            +664 / +757
#   ns    503    +951   +1063  +0.0006 ± 0.0002            +748 / +805
#   ew    455    +725   +834   +0.0004 ± 0.0001            +687 / +781
#   both  501    +946   +1057  +0.0006 ± 0.0002            +895 / +999
#
# By responder's hand (none / both; boards, plain, PD), every bucket positive
# on plain:
#
#   6♠ <=9    2♠   111 +122 +188 / 126 +139 +215   (sized a wash)
#   7♠ <=9    2♠    33  +71  +81 /  36  +78  +95
#   6♠ 10-11  3♠   101  +54  +23 / 106 +137  +79   (sized +193 / +270)
#   6♠ 12     3♠    33  +81 +107 /  35 +106 +138
#   6♠ 13+    4♠   110 +130 +134 / 117 +109 +111
#   7♠ 10-12  4♠    39 +150 +164 /  41 +218 +240
#   7♠ 13+    4♠    32 +121 +135 /  38 +159 +179
#
# The win beats the sizing (+550 / +700) because `2♠` on six is not a wash and
# seven spades do better than priced.  The weak slice is the `3♠` invite on
# 10-11 where base passed `2♥`: 79 boards +16 plain / -40 PD non-vul, 83
# boards +87 / 0 vul (the rest of that row replaces `3NT`).  Two boards per
# cell diverge outside responder's seat and net 0: opener's answer to an
# unchanged `3NT` (`4♥` for `6♥` on eight hearts), whose reading loses the
# six-spade hands the new calls outrank.  The SD arms sample the new calls
# thinly: 10% of ON worlds fall back (22% of boards hold one) against 0.5% OFF.
# The cells share their deals (one seed), so 4/4 is one deal sample at four
# vulnerabilities.  Shipped default-on on this READ (jdh8, 2026-11-25).
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
    arm base "$vul" --no-ns-heart-rebid-spade-rebid
    arm spade "$vul"
    gatepair spade base "$vul" || log "isolation gate spade vs base ($vul) FAILED, scoring anyway"
    diffpair spade base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.spade.vs.base.$vul.plain.txt")
    log "pre-count spade vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff spade base "$vul" --off-no-ns-heart-rebid-spade-rebid
done
log "=== heart-rebid-spade-rebid A/B done"
