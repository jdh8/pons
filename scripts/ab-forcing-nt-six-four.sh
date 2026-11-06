#!/bin/sh
# ab-forcing-nt-six-four.sh — opener's `1♠ - 1NT - 2♥` on 6♠4♥, vs BBA, four
# cells, plain DD + perfect defense from one solve, plus SD-PD.  Arms are
# sequential and share one persistent fresh seed.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-forcing-nt-six-four.sh ab-results/forcing-nt-six-four \
#       >ab-results/forcing-nt-six-four.log 2>&1 < /dev/null &
#
# Why.  Node #3 of the 2026-11-06 re-anchor's Constructive / book / round-2
# bucket (docs/bba-gap-campaign.md § "Current ranking (`07864122`,
# 2026-11-06)") is opener's six-card `2♠` rebid.  Its coherent slice is 6♠
# with four-plus hearts over `1♠ - 1NT`, where BBA rebids `2♥`: 200 boards,
# −377 plain / −257 PD.  Over every board BBA rebids `2♥` on each ≤14-HCP
# 6♠4♥ (298/298) and 6♠5♥ (83/83); we rebid `2♠` on all of them.
#
#   on   rebid.forcing_nt_six_four_hearts (default on since 2026-11-06): 6+♠ 4+♥ that would rebid `2♠`
#        bids `2♥` (weight 1.01 over the six-card `2♠`'s 1.00)
#
# READ 2026-11-06 (vs BBA, arm − base, IMPs/board ±95%, none / ns / ew / both):
#   r1 24e2ace3, SEED 1793928790672211553, 204,800/cell: bare rebid, every
#      cell non-inferior and negative (−0.14 to −0.54 /fired plain).  Trace:
#      opener passed `2♥ - 2NT` on six spades, the floor bid `4♠` over
#      `2♥ - 2♠` on 14 HCP.  Both rows authored (rebids/jump_shifts.rs).
#   r2 SEED 1793929160112958448, 204,800/cell: +0.0002 +0.0004 +0.0002
#      +0.0003 plain, all non-inferior.
#   r3 pre-declared, SEED 1793929388758490793, 819,200/cell, read alone:
#      plain +0.0002±1 +0.0003±2 +0.0002±1 +0.0003±2 (e-4), win 4/4; PD
#      +0.0002 +0.0003 +0.0002 +0.0002, win at ew, non-inferior elsewhere;
#      fired 235 / 280 / 236 / 276.  Shipped default-on; `base` is now
#      `--no-ns-forcing-nt-six-four-hearts`.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/forcing-nt-six-four.seed.  Iron rule:
# do NOT rebuild binaries while this runs.
R=${1:?usage: ab-forcing-nt-six-four.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for forcing-nt-six-four)

log "=== forcing-nt-six-four A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-forcing-nt-six-four-hearts
    arm on "$vul"
    gatepair on base "$vul"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-forcing-nt-six-four-hearts
done
log "=== forcing-nt-six-four A/B done"
