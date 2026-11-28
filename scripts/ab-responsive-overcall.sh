#!/bin/sh
# ab-responsive-overcall.sh — advancer's responsive double after partner's
# overcall and their raise (`defense.responsive_overcall_enabled`), vs BBA,
# four cells, plain DD + perfect defense from one solve.  Arms are sequential
# and share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh responsive-overcall \
#       scripts/ab-responsive-overcall.sh ab-results/responsive-overcall
#
# Why.  The knob's only record is self-play PD against the floor
# (`ab-responsive --conv overcall`, 400k filtered x 2): NV +0.928
# IMPs/divergent, vul -0.178, about 1% divergent (docs/bidding-options.md).
# That is one scorer, two cells and no reference opponent.  Bridge World
# Standard 2017 plays the double (VI.C, docs/bws-2017.md §2).  This is its
# first run against BBA and its first plain-DD number.
#
#   base the book: `(1t) overcall (2t)` / `(3t)` is the floor's
#   ro   --ns-responsive-overcall: `X` there shows the two suits unbid by
#        opener and partner, 4+ each, 8+ points (defense/responsive.rs); a
#        hand that does not qualify still falls to the floor.  Since round 2
#        overcaller's answer is authored, redoubled or not: four cards in one
#        of doubler's suits bid it (a major fit jumps on 13-15 where the two
#        level was open and bids game on 16+), anything else rebids the
#        overcalled suit at the cheapest level, and there is no penalty pass
#
# Not in this arm: doubler's rebid over the answer (the floor's), a notrump
# or three-card answer, and any disclosure: EPBot has no row for it (its
# `Responsive double` is the takeout-double form, its `Snapdragon Double` is
# over a new suit), so BBA meets the double undescribed.  The declared EPBot
# book keeps the shipped defense, so the arm cannot re-read their calls.
#
# The knob moves only auctions they open: the gate is `theirs`.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard (against BBA a PD loss is recorded, not a veto); delta = 0.002
# IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
# Iron rule: do NOT rebuild binaries while this runs.
#
# MEASURED 2026-11-28, a loss as built, stays off (SEED_BASE
# 1795828526733790909, 1,638,400 boards/arm/vul, 0.26-0.28% fired, gate
# passed).  Plain DD none / ns / ew / both: -0.0011 ±0.0005 / -0.0027 ±0.0006
# / +0.0000 ±0.0006 / -0.0021 ±0.0006 IMPs/board (-0.40 / -1.05 / +0.01 /
# -0.77 per fired): a loss in three cells.  PD -0.0039 ±0.0006 / -0.0064
# ±0.0007 / -0.0029 ±0.0007 / -0.0055 ±0.0007: a loss in all four.
#
# Traced (none / ns): the loss is overcaller's answer, which is the floor's.
# It jumps to game in its own suit on 1,316 / 1,219 of the 4,452 / 4,282
# fires for -2,732 / -3,808 plain IMPs, more than the cell's whole loss
# (-1,795 / -4,492), and passes the double for penalty on 309 / 284 for
# -579 / -542.  It bids one of doubler's suits below game on 2 / 1 boards;
# game in doubler's suit earns +1,253 / +1,127 on 491 / 491.  The double
# itself is too wide: a doubler with at most one card in partner's suit
# loses -1.2 / -2.2 a board, one with three is flat.  Over the raise to
# three the double wins plain (+235 / +179 on 460 / 401).  Not dead: owed an
# authored answer before a re-measure.
#
# ROUND 2, queued 2026-11-28 with the answer authored:
#
#   BOARDS=1638400 scripts/queue-ab.sh responsive-overcall-answer \
#       scripts/ab-responsive-overcall.sh ab-results/responsive-overcall-answer
#
# Read it by overcaller's answer and by doubler's length in partner's suit
# before the headline: the first says whether doubler's rebid needs a table,
# the second whether the double should deny three-card support.
R=${1:?usage: ab-responsive-overcall.sh RESULTS_DIR}
SHOW=${SHOW:-50}
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== responsive-overcall A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm ro "$vul" --ns-responsive-overcall
    gatepair ro base "$vul" theirs || log "isolation gate ro vs base ($vul) FAILED, scoring anyway"
    diffpair ro base "$vul"
done
log "=== responsive-overcall A/B done"
