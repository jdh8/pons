#!/bin/sh
# ab-eleven-count-majors.sh — open `1M` on every 11-HCP five-card major in
# first/second seat (`opening.eleven_count_majors`).
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-eleven-count-majors.sh ab-results/eleven-count-majors \
#       >ab-results/eleven-count-majors.log 2>&1 < /dev/null &
#
#   on   --ns-eleven-count-majors: the flat 5-3-3-2 and wasted-doubleton
#        11-counts open too
#   off  the default: `points(12..=21) & hcp(10..)`, so an 11-count opens
#        only when its shape upgrades it
#
# Hypothesis (pre-run): ~1% of boards fired.  A constructive contract-boundary
# change, so plain DD can see it; the doubt is responder's game gates, tuned
# on a 12-point minimum, overreaching opposite the flat 11.  Read the on-arm
# buckets by responder's call before blaming the idea.
#
# No isolation gate: it classifies by who opened in the *off* arm, and the
# off arm passes these hands, so BBA opens ~70% of the divergent boards by
# construction.  The real check is probe-divergence's "who bid differently
# first", which read ours 1872/1872 at none.
#
# MEASURED 2026-10-04, stays off (SEED_BASE 1791130311, 204,800 boards/arm/
# vul, 0.91% fired): plain +0.0020 ±0.0022 none / −0.0015 ±0.0028 both, PD
# −0.0071 ±0.0026 / −0.0120 ±0.0034.  The PD loss is our own partscores
# (−1071/−2179); slams we reach and the off arm does not cost −334 plain
# pooled and are decided by the evaluator net, not the point gates.
R=${1:?usage: ab-eleven-count-majors.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for eleven-count-majors)

log "=== eleven-count-majors A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on "$vul" --ns-eleven-count-majors
    diffpair on off "$vul"
done
log "=== eleven-count-majors A/B done"
