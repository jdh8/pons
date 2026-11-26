#!/bin/sh
# ab-longer-hearts-first.sh — the heart half of the longer suit first
# (`opening.longer_hearts_first`): 5♠ with 6+♥ opens `1♥`, vs BBA, four
# cells, plain DD + perfect defense from one solve.  Arms are sequential and
# share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh longer-hearts-first \
#       scripts/ab-longer-hearts-first.sh ab-results/longer-hearts-first
#
# Why.  `opening.longer_suit_first` lost 4/4 on 2026-11-25
# (ab-longer-suit-first.sh): its 6m5M `1m` half lost at the opening and the
# widened `1m` reading drifted the contested floor on 28k boards, while the
# 6♥5♠ `1♥` read +454 plain / +514 PD on 728 boards (none cell).  That arm
# could not price the heart half apart from its own drift; this one leaves
# the `1m` rules and reading alone.  Anchor 4592d0a3: `1♠` where BBA opens
# `1♥` is 371 rows per 409,600 boards, -533 plain.
#
#   base --no-ns-longer-hearts-first: 5♠6♥ opens `1♠`
#   on   the default since 2026-11-27: `1♥` on 6+♥ with exactly five spades
#        (12+ points, 10+ HCP)
#
# Not in this arm: any continuation.  Opener's rebids are the book's natural
# ones (`1♥ - 1NT - 2♥`; `2♠` is a reverse).  Expect ~730 boards fired a cell
# (0.04%): at +0.0003 IMPs/board the win needs a CI near +-0.0002.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# MEASURED 2026-11-27, a WIN, shipped default-on (sha e46af7be; seed
# 1795705454956034453; 1,638,400 bd/cell; isolation gate passed in every
# cell).  IMPs none / ns / ew / both, CI +-0.0002..0.0003 per board:
#
#   plain  +376 / +366 / +507 / +432  (win / non-inferior / win / non-inferior)
#   PD     +471 / +512 / +603 / +531  (win 4/4)
#   fired  717 / 775 / 718 / 762 (0.04-0.05%), +0.47..+0.71 plain per fired
#
# Shippable per the table (plain non-inferior worst cell, PD win).  Since the
# flip the `on` arm is the default and `base` passes
# `--no-ns-longer-hearts-first`.
#
# Resumable; SEED_BASE persists in $R/longer-hearts-first.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-longer-hearts-first.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for longer-hearts-first)

log "=== longer-hearts-first A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-longer-hearts-first
    arm on "$vul"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
done
log "=== longer-hearts-first A/B done"
