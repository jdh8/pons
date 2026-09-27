#!/bin/sh
# ab-opener-new-suit.sh — opener's minimum new-suit rebids, vs BBA.
#
#   plain   american before this change                           (control)
#   d2c     --ns-one-diamond-two-clubs: 1♦ - 1M - 2♣ on 4+♣, ≤5♦
#   tcm     the three-card minor, shipped default-on 2026-09-27 (measured
#           as --ns-forcing-nt-three-card-minor; the other arms now pass the
#           off-switch to reproduce the run): 1M - 1NT - 2m on a 5332's
#           three-card minor, read as 3+
#
# Found by the BEN Tier-S Constructive/book/round-2 trace (2026-09-27): both
# references bid the new suit where we rebid our own (the "rebid vs new
# lower suit" cell, 170 bd −17.5 PD/1000 vs BEN, 3,290 bd −10.7 vs BBA).
#
#   BOARDS=409600 setsid nohup scripts/idle-run.sh \
#       scripts/ab-opener-new-suit.sh ab-results/opener-new-suit \
#       >ab-results/opener-new-suit.log 2>&1 &
R=${1:?usage: ab-opener-new-suit.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== opener new suit start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american --no-ns-forcing-nt-three-card-minor
    arm d2c   "$vul" --our-floor american --no-ns-forcing-nt-three-card-minor --ns-one-diamond-two-clubs
    arm tcm   "$vul" --our-floor american
    for on in d2c tcm; do
        # tcm's reading also reads BBA's (undeclared, so ours) `1M - 1NT - 2m`
        # as 3+ — true of BBA, which rebids three-card minors — so its gate is
        # a report, not a stop (the ab-watermelon.sh NO_GATE precedent).
        if [ "$on" = tcm ]; then
            gatepair "$on" plain "$vul" || log "gate: tcm also reads their 2m (see gate file)"
        else
            gatepair "$on" plain "$vul"
        fi
        diffpair "$on" plain "$vul"
        sddiff "$on" plain "$vul"
    done
done
log "=== opener new suit done"
