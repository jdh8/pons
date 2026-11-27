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
#        hand that does not qualify still falls to the floor
#
# Not in this arm: overcaller's answer to the double (the floor's, reading
# the alerted two-suiter), and any disclosure: EPBot has no row for it (its
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
