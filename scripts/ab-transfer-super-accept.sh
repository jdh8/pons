#!/bin/sh
# ab-transfer-super-accept.sh — opener's jump super-accept of a Jacoby
# transfer over `1NT` (`notrump.transfer_super_accept`), vs BBA, four cells,
# plain DD + perfect defense from one solve.  Arms are sequential and share
# one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh transfer-super-accept \
#       scripts/ab-transfer-super-accept.sh ab-results/transfer-super-accept
#
# Why.  The knob's only record is a 640k-board DD wash leaning negative
# (-0.055 IMPs/fired) with a `stale-PD` figure (docs/bidding-options.md), from
# before the current floor and scorers.  Bridge World Standard 2017 presumes
# the superaccept (V.C(m)(i), docs/bws-2017.md §2), BBA's card has a row for
# it, and its `2NT` sibling won 4/4 on 2026-11-23 once responder's tail was
# authored.  This replaces the stale verdict.
#
#   base the book: opener completes `1NT - 2♦ - 2♥` / `1NT - 2♥ - 2♠`
#   sa   --ns-transfer-super-accept: opener jumps to `3M` on four-card
#        support and 17 HCP (notrump/transfers.rs); the card row
#        `Super acceptance after NT` moves with it, so BBA is told
#
# Not in this arm: an authored responder table over the jump (the instinct
# floor's, reading four trumps and 17), fit- or shortness-showing
# super-accepts, and the interfered tails, which live under
# `competition.competition_over_transfer` (off in both arms).  A loss here
# gets its worst boards traced by final level before it is called dead: the
# `2NT` sibling's first round lost on exactly that unauthored tail.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
# Iron rule: do NOT rebuild binaries while this runs.
#
# MEASURED 2026-11-28, stays off (SEED_BASE 1795815144234142806, 1,638,400
# boards/arm/vul, 0.04-0.05% fired).  Plain DD none / ns / ew / both:
# +0.0001 ±0.0002 / +0.0005 ±0.0003 / +0.0002 ±0.0002 / +0.0004 ±0.0003
# IMPs/board (+0.27 / +1.00 / +0.36 / +0.98 per fired): non-inferior, a win
# in both cells where we are vulnerable.  PD -0.0003 ±0.0002 / -0.0001
# ±0.0003 / -0.0003 ±0.0002 / -0.0001 ±0.0003: a loss when we are not
# vulnerable, recorded, not a veto against BBA.  A convention needs a plain
# win in every cell to become the default.  By contract class (none / both):
# the partscore the jump turns into a game earns +286 / +880 plain on 448 /
# 510 boards and is PD's loss (-250 / -11); every other class loses, the
# floor's table over `3M` among them (game -> slam -11 / -39 on 87 / 91,
# slam -> grand -21 / -32 on 14 / 16).
#
# The isolation gate reports 111-126 boards a cell opened by them.  They are
# ours: `(1x) 1NT - 2♦ - 3♥`, systems on over our `1NT` overcall, shares
# `complete_transfer`.  They score +99 / +237 plain (none / both).
R=${1:?usage: ab-transfer-super-accept.sh RESULTS_DIR}
SHOW=${SHOW:-50}
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== transfer-super-accept A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm sa "$vul" --ns-transfer-super-accept
    gatepair sa base "$vul" || log "isolation gate sa vs base ($vul) FAILED, scoring anyway"
    diffpair sa base "$vul"
done
log "=== transfer-super-accept A/B done"
