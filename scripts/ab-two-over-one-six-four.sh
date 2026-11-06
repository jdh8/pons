#!/bin/sh
# ab-two-over-one-six-four.sh — opener's `1♠ - 2m - 2♥` on 6♠4♥, vs BBA, four
# cells, plain DD + perfect defense from one solve, plus SD-PD.  Arms are
# sequential and share one persistent fresh seed.
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-six-four.sh ab-results/two-over-one-six-four \
#       >ab-results/two-over-one-six-four.log 2>&1 < /dev/null &
#
# Why.  Node #3 of the 2026-11-06 re-anchor (docs/bba-gap-campaign.md §
# "Current ranking (`07864122`, 2026-11-06)") is opener's six-card `2♠`.  Its
# forcing-1NT slice shipped as `rebid.forcing_nt_six_four_hearts` (a159a431);
# the 2/1 slice, 6♠4-5♥ over `1♠ - 2♣/2♦` where BBA rebids `2♥`, is 74
# boards, −212 plain / −232 PD.
#
#   on   rebid.two_over_one_six_four_hearts: a 6+♠ 4+♥ that would rebid `2♠`
#        bids `2♥` (weight 1.41 over the six-card `2♠`'s 1.40), and over
#        `2♥ - 3NT` a <15-point six spades bids `4♠`
#
# Pre-run self-play probe (uncontested, none vul, 2M deals, seed 17): 417
# boards change, +472 IMPs on − off; the `2♥ - 3NT` node was −174 on 113
# until opener's `4♠` was authored (opener passed 3NT on six spades).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/two-over-one-six-four.seed.  Iron rule:
# do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-over-one-six-four.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-over-one-six-four)

log "=== two-over-one-six-four A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-two-over-one-six-four-hearts
    gatepair on base "$vul"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --on-ns-two-over-one-six-four-hearts
done
log "=== two-over-one-six-four A/B done"
