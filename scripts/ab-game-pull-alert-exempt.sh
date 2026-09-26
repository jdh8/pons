#!/bin/sh
# ab-game-pull-alert-exempt.sh — the game-pull rail's code exemption
# (`InstinctProfile::their_game_pull_alert_exempt`) against plain american,
# vs BBA.
#
#   plain   american as shipped: the game-pull rail masks every silent-side
#           suit pull of their game into a suit of four cards or fewer (control)
#   on      the rail stands aside when their game is a code — alerted and
#           promising fewer than three cards in its strain (a keycard reply,
#           a cue); completions still masked (--ns-game-pull-alert-exempt)
#
# The shipped rail's worst A/B boards (SEED_BASE 1790405692) were their
# artificial 5♣/5♦ where our masked junk sacrifice had happened to stop their
# slam.  Only fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-game-pull-alert-exempt.sh ab-results/game-pull-alert-exempt \
#       >ab-results/game-pull-alert-exempt.log 2>&1 &
R=${1:?usage: ab-game-pull-alert-exempt.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== game pull alert exempt start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american
    arm on    "$vul" --our-floor american --ns-game-pull-alert-exempt
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== game pull alert exempt done"
