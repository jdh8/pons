#!/bin/sh
# ab-longer-suit-first.sh — open the longer suit on a five-card major under a
# longer suit (`opening.longer_suit_first`), vs BBA, four cells, plain DD +
# perfect defense from one solve.  Arms are sequential and share one
# persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh longer-suit-first \
#       scripts/ab-longer-suit-first.sh ab-results/longer-suit-first
#
# Why.  On the BBA anchor a0977ce9, cut by auction (scripts/cut-auction.py),
# our `1M` where BBA opens a longer suit is 1,787 rows per 409,600 boards,
# -2,878 plain / -2,406 PD: `1♥` vs `1♣` 391 rows -743, `1♠` vs `1♣` 317
# -727, `1♠` vs `1♥` 371 -557, `1♥` vs `1♦` 379 -448, `1♠` vs `1♦` 329 -403.
# Every row is 5M with a 6+ minor, or 5♠ with 6+♥.  It is a first-call pair,
# so the figure prices BBA's continuations with its call.
#
#   base the book: a five-card major opens `1M`, 5♠6♥ opens `1♠`
#   on   --ns-longer-suit-first: `1m` on 6+m with exactly five in a major,
#        `1♥` on 6+♥ with exactly five spades (12+ points, 10+ HCP)
#
# Not in this arm: any continuation.  Opener's rebids are the book's natural
# ones (probed 2026-11-25: `1♣ - 1♦ - 1♥`; `1♣ - 1♠ - 2♣` on a minimum,
# `2♥` / `3♥` on 17+ / 18+; `1♥ - 1NT - 2♥`), and the net floor passes
# `1♣ - 1NT` on a minimum 6♣5♥.  If this loses, bucket by opener's rebid
# before reading it as the opening.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# MEASURED 2026-11-25, a LOSS in all four cells, stays off (sha 8b04e984, the
# frozen 45b3f565 tree; seed 1795603545036763043; 1,638,400 bd/cell; isolation
# gate passed, 0 foreign).  IMPs/board none / ns / ew / both:
#
#   plain  -0.0060 / -0.0085 / -0.0059 / -0.0083  (CI +-0.0013..0.0015)
#   PD     -0.0135 / -0.0163 / -0.0141 / -0.0156
#   fired  33,148 / 31,343 / 31,262 / 29,598 (1.8-2.0%)
#
# Traced, none cell (`ab-dump-diff --jsonl`, plain / PD IMPs):
#   opening changed           3,726 bd   -4,303 / -3,838
#     6m5M opening `1m`       2,998 bd   -4,757 / -4,352, two thirds of it
#                                         contested (partner's major is lost
#                                         when they interfere)
#     6♥5♠ opening `1♥`         728 bd     +454 /   +514
#   same opening, contested  28,273 bd   -5,049 / -17,395  reading drift: the
#                                         widened `1m` reading moves the learned
#                                         contested floor on hands that are not
#                                         6-5 at all (docs/reading-drift-handoff.md)
#   same opening, uncontested 1,149 bd     -465 /   -915
# Opener's rebid is not the culprit: the minor half loses at the opening.  The
# credible follow-up is the heart half alone (6♥5♠ opens `1♥`, the `1m` reading
# untouched), which this arm cannot price apart from its own drift.
#
# Resumable; SEED_BASE persists in $R/longer-suit-first.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-longer-suit-first.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for longer-suit-first)

log "=== longer-suit-first A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-longer-suit-first
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
done
log "=== longer-suit-first A/B done"
