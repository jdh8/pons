#!/bin/sh
# ab-strong-jump-shift.sh — the strong jump shift `1♥ - 2♠`
# (`response.heart_spade_strong_jump_shift`), vs BBA, four cells, plain DD +
# perfect defense from one solve.  Arms are sequential and share one
# persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh strong-jump-shift \
#       scripts/ab-strong-jump-shift.sh ab-results/strong-jump-shift
#
# Why.  Anchor 4592d0a3, lead 4: our `1♠` where BBA bids `2♠` over `1♥` is
# 303 rows per 409,600 boards, -442 plain / -456 PD.  Every row is 5+ spades,
# at most two hearts, 15+ HCP.  The 15-counts wash (118 rows, -19); 16-17
# lose (130 rows, -354), slams the `1♠` response signs off under at round 2.
#
#   base the book: `1♥ - 2♠` is the weak jump shift
#   f16  --ns-heart-spade-strong-jump-shift 16: 5+ spades, at most two
#        hearts, 16+ HCP, game-forcing, by an unpassed hand; opener rebids
#        `3♠` / `3♥` / `3m` / `2NT`, responder places or asks
#   f15  the same from 15, BBA's floor; `f15 vs f16` prices the 15-counts
#        (responses/strong_jump_shift.rs)
#
# The arm also gives up the unpassed weak `2♠` (+113 / +181 on the 2026-10-07
# cut): bucket the fired boards by which hand moved.  Not authored: opener
# over responder's `3NT`, and every contested tail but opener's `(X)`; those
# are the floor's, reading the jump shift as natural.  The card still tells
# BBA our two-level jump shifts are weak (EPBot has one row for all of them),
# so BBA defends `2♠` as a preempt: read PD with that in mind.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# MEASURED 2026-11-27, NON-INFERIOR, stays off pending jdh8 (sha e89f9301;
# seed 1795735355469146667; 1,638,400 bd/cell).  IMPs none / ns / ew / both:
#
#   f16 vs base  plain +106 / +168 / +111 / +127  (NI 4/4, CI +-0.0002..0.0003)
#                PD    +163 / +333 / +120 / +212  (NI 4/4)
#                fired 684 / 685 / 696 / 698 (0.04%)
#   f15 vs base  plain +361 / +505 / +360 / +403  (NI 4/4, +0.0002..0.0003)
#                PD    +430 / +693 / +380 / +507  (NI / win / NI / NI)
#                fired 882 / 891 / 894 / 901 (0.05%)
#   f15 vs f16   plain +231 / +301 / +225 / +242  (NI / win / NI / NI)
#                PD    +243 / +324 / +236 / +265  (win / win / NI / NI)
#
# The 15 floor beats 16 in every cell.  Isolation gate FAILED in every cell
# (32 / 22 / 49 / 40 boards they opened, the same leak as
# ab-single-raise-gap.sh); none cell f15, they open +60 plain, we open +301.
# Bucketed by node (f15, none cell): the lost weak `2♠` costs -137 plain on
# 226 boards; the strong `2♠` over `1♠` is -23 on 304 boards with opener the
# dealer but +440 on 309 with opener in second seat, an asymmetry not traced.
# Decision row: plain NI / PD NI, so naturalness breaks the tie (BWS 2017
# IV.G(f) and BBA both play it strong), but the card still declares weak jump
# shifts, so some of the PD gain may come from BBA defending a preempt.
#
# Resumable; SEED_BASE persists in $R/strong-jump-shift.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-strong-jump-shift.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for strong-jump-shift)

log "=== strong-jump-shift A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    for floor in 16 15; do
        arm "f$floor" "$vul" --ns-heart-spade-strong-jump-shift "$floor"
        gatepair "f$floor" base "$vul" || log "isolation gate f$floor vs base ($vul) FAILED, scoring anyway"
        diffpair "f$floor" base "$vul"
    done
    diffpair f15 f16 "$vul"
done
log "=== strong-jump-shift A/B done"
