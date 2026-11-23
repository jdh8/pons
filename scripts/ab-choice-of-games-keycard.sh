#!/bin/sh
# ab-choice-of-games-keycard.sh — opener asks keycards over the choice-of-games `1M - 3NT` on 18+ support points,
# vs BBA, four cells, plain DD + perfect defense from one solve, plus SD-PD.
# Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh choice-of-games-keycard \
#       scripts/ab-choice-of-games-keycard.sh ab-results/choice-of-games-keycard
#
# Why.  On the BBA anchor a0977ce9 (docs/bba-gap-campaign.md, round-2 cut,
# lead 6) `1♠ - 3NT -` is 38 rows per 409,600 boards where we sign off (`4♠`
# or a pass) and BBA cue-bids: -216 plain / -216 PD, BBA in `6♠`.  Opener has
# no slam move at all: the table is `4M` unbalanced, else pass.  The `.pdd`
# census (examples/probe-drury-slam.rs --choice-of-games, ~100M deals) prices
# the ask (slam on at most one keycard missing, else `5M`) against `4M`
# opposite a (4333) 12-15 with 3-4 trumps, IMPs per case (not vul / vul):
#
#   opener sp   16      17      18      19      20      21
#   not vul     -3.10   -0.65   +2.15   +4.92   +7.09   +8.67
#   vul         -3.73   -0.80   +2.53   +5.80   +8.38   +10.24
#
# At 18 every shape is positive (balanced +1.36, a singleton +1.95 not vul);
# at 17 only the no-singleton unbalanced hands are (+0.64).
#
#   base response.choice_of_games_keycard None (the book today)
#   on   response.choice_of_games_keycard Some(18): `4NT` RKCB on 18+ support
#        points, the 1430 ladder and the answerer's pass below it
#
# Not in this arm: a cue-bidding route (BBA's `4♣` / `4♦`), the `4NT (X)` and
# overcalled tails (the shared RKCB gap, park/rkcb-doubled-answer), and any
# move on the 17s.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/choice-of-games-keycard.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-choice-of-games-keycard.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for choice-of-games-keycard)

log "=== choice-of-games-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-choice-of-games-keycard 18
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --on-ns-choice-of-games-keycard 18
done
log "=== choice-of-games-keycard A/B done"
