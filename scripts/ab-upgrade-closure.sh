#!/bin/sh
# ab-upgrade-closure.sh — DNF chop C2, re-measured after the ceilings shipped.
#
# `Envelope::narrow_to_upgrade` closes `hcp` against `points` through the shape
# upgrade: balanced shapes never upgrade, so a box whose own lengths force
# balanced reads `points == hcp` instead of carrying the scale's global 2-HCP
# slack at each end.  Exact — it drops no hand the box claims.
#
# C2 measured bidding-inert (0/3000 boards) in 2026-07 for one reason: the
# forward `Points`/`Hcp::project` wrote FLOORS only, so `points.max <- hcp.max +
# ceiling` had no ceiling to bite.  `reading.strength_ceilings` shipped
# default-on 2026-08-16 and killed that premise; the pre-check at 20,480 boards
# now fires 18 (0.09%), all of them slam decisions — the closure lowers
# `points.max` and `combined_points` stops reaching the slam gate.
#
# Two arms per vul, identical deals:
#   off   the shipped default
#   upg   --ns-upgrade-closure
#
# Historical Phase-3 gate: this run predates the honest-reading v6 retrain.
#
# PRE-REGISTERED (jdh8, before the numbers): C2 is a soundness/information
# correction, so it takes the reading-drift bar — **non-loss ships it**
# default-on (plain wash-or-win AND PD non-loss, both vuls).  A plain win with
# a PD loss is a doubling artifact and does not ship.  Any loss gets its worst
# divergent boards traced before a conclusion.
#
#   setsid nohup scripts/idle-run.sh scripts/ab-upgrade-closure.sh \
#       ab-results/c2-upgrade-closure >ab-results/c2-upgrade-closure.log 2>&1 </dev/null &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-upgrade-closure.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== closure C2 start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm upg "$vul" --ns-upgrade-closure
    diffpair upg off "$vul"
done
log "=== closure C2 done"
