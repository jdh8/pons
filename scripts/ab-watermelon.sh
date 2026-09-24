#!/bin/sh
# ab-watermelon.sh — one Watermelon Dutch Doubleton knob on american, against
# plain american on the same deals.  Two arms, both `--our-floor american`
# against the BBA reference opponent:
#
#   plain   american as shipped                                  (control)
#   on      + FLAG — one of
#             --ns-5542      opening.five_five_four_two
#             --ns-wide-1c   opening.wide_one_club   (1NT/2♦! relay tails unauthored)
#             --ns-odwrotka  rebid.odwrotka          (opener after a step is floor-only)
#
# NO_GATE=1 skips the isolation gate, for a knob that moves the floor's regime
# input or the declared card (`--ns-wide-1c`: features.rs slot 0 + card system
# 2), where divergence on boards they open is part of the treatment and the
# whole-arm diff is the headline.
#
# Spec: https://jdh8.github.io/watermelon-dutch/.  One knob per results dir,
# runs sequential; a new dir gets a new seed.  sddiff prices what an opening
# structure tells the opening leader, which plain DD cannot see.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-watermelon.sh ab-results/wm-5542 --ns-5542 \
#       >ab-results/wm-5542.log 2>&1 &
R=${1:?usage: ab-watermelon.sh RESULTS_DIR FLAG}
FLAG=${2:?usage: ab-watermelon.sh RESULTS_DIR FLAG}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== watermelon $FLAG start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american
    arm on    "$vul" --our-floor american "$FLAG"
    [ -n "${NO_GATE:-}" ] || gatepair on plain "$vul"
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== watermelon $FLAG done"
