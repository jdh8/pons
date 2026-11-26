#!/bin/sh
# ab-single-raise-gap.sh — three trumps on at most 9 HCP raise `1M` to `2M`
# whatever the support points (`response.single_raise_gap`), vs BBA, four
# cells, plain DD + perfect defense from one solve.  Arms are sequential and
# share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh single-raise-gap \
#       scripts/ab-single-raise-gap.sh ab-results/single-raise-gap
#
# Why.  The single raise is 6-9 support points; the three-card limit raise
# through the forcing `1NT` is 10-12 HCP (rebids/forcing_notrump.rs).  Three
# trumps on 9 HCP with a doubleton (10+ support points) fits neither, bids
# `1NT` and then only takes a preference.  Anchor 4592d0a3, lead 5: our `1NT`
# where BBA bids `2♠` over `1♠` is 325 rows per 409,600 boards, -462 plain /
# -532 PD; the 9-HCP hands are 147 rows, -298, spread over every
# continuation.  The heart sibling reads +46 on 146 rows, so this is a soft
# lead: split the result by major before reading it.
#
#   base the book
#   on   --ns-single-raise-gap: `2M` on exactly three trumps, 10+ support
#        points and at most 9 HCP
#
# With it: responder's game-try acceptance (raises/game_try.rs) reads 8+
# support points instead of 8-9, so the new hands accept.  Opener's tries
# keep their 16-18 / 19+ gates; if this loses, look there first.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/single-raise-gap.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-single-raise-gap.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for single-raise-gap)

log "=== single-raise-gap A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-single-raise-gap
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
done
log "=== single-raise-gap A/B done"
