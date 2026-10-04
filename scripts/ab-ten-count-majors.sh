#!/bin/sh
# ab-ten-count-majors.sh — open `1M` on a 10-HCP 5-4-3-1 with no wasted short
# honour in first/second seat (`opening.ten_count_majors`), alone and with
# `opening.eleven_count_majors` (the two together copy BBA's light majors).
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-ten-count-majors.sh ab-results/ten-count-majors \
#       >ab-results/ten-count-majors.log 2>&1 < /dev/null &
#
#   off  the default: `points(12..=21) & hcp(10..)`
#   ten  --ns-ten-count-majors
#   bba  --ns-ten-count-majors --ns-eleven-count-majors
#
# Hypothesis (pre-run): `ten` fires ~0.5% of boards.  The 10-count anchor
# boards where BBA opened and we passed split plain −0.66 / PD +0.4..0.65 per
# board, the eleven-count signature, so expect plain ≥ 0 and PD < 0; the
# 5-4-3-1 has a ruffing value and a second suit, the best case for a light 1M.
#
# MEASURED 2026-10-05, stays off (SEED_BASE 1791135744, 204,800 boards/arm/
# vul).  ten (0.35% fired): plain +0.0014 ±0.0014 none / +0.0018 ±0.0019 both,
# PD −0.0019 ±0.0016 / −0.0030 ±0.0021.  bba (1.26%): plain +0.0047 ±0.0025 /
# +0.0029 ±0.0033, PD −0.0076 ±0.0030 / −0.0134 ±0.0039.
R=${1:?usage: ab-ten-count-majors.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for ten-count-majors)

log "=== ten-count-majors A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm ten "$vul" --ns-ten-count-majors
    arm bba "$vul" --ns-ten-count-majors --ns-eleven-count-majors
    diffpair ten off "$vul"
    diffpair bba off "$vul"
done
log "=== ten-count-majors A/B done"
