#!/bin/sh
# ab-game-pull-veto.sh — the game sibling of the 3NT-pull rail
# (`InstinctProfile::their_game_pull_veto`) against plain american, vs BBA.
#
#   plain   american with the rail off (--no-ns-game-pull-veto)     (control)
#   on      american as shipped since 2026-09-26: our silent side's suit pull
#           of their game (4♥/4♠/5♣/5♦) into a suit of four cards or fewer
#           is masked
#
# Found by the R4 trace of the 2026-09-20 shipping-arm decompose (c3bb94a7):
# over their Smolen `1NT - 2♣ - 2♦ - 3♠ - 4♥` the v6 floor bids 4♠ on junk
# (48 boards over every game, -197 plain / -577 PD IMPs over 409.6k, BBA
# passed 48 of 50).  Only fires on boards they open, so the gate reads `theirs`.
#
#   BOARDS=204800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-game-pull-veto.sh ab-results/game-pull-veto \
#       >ab-results/game-pull-veto.log 2>&1 &
R=${1:?usage: ab-game-pull-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== game pull veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain "$vul" --our-floor american --no-ns-game-pull-veto
    arm on    "$vul" --our-floor american
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== game pull veto done"
