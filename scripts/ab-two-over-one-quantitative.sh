#!/bin/sh
# ab-two-over-one-quantitative.sh — responder's quantitative `4NT` after a
# no-fit 2/1, vs BBA, four cells, plain DD + perfect defense from one solve,
# plus SD-PD.  Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-quantitative.sh ab-results/two-over-one-quantitative \
#       >ab-results/two-over-one-quantitative.log 2>&1 < /dev/null &
#
# Why.  Round-2 is shipping #2 of the 2026-11-11 re-anchor (docs/bba-gap-campaign.md
# § "Current ranking (`7f3d57bd`, 2026-11-11)"); its largest unmeasured node
# is responder's 2/1 game-force `3NT` 13+ HCP: 600 rows, −802 plain / −796 PD
# per 409,600 boards, all of it on 18+ HCP (134 rows, −920 plain), where BBA
# bids a second suit or `4NT` and reaches the slam.
#
#   on   rebid.two_over_one_quantitative (default on since 2026-11-11): `3NT` keeps 13–17 HCP,
#        `4NT` (natural, quantitative) takes 18–20, `6NT` 21+; opener answers
#        `4NT` with `6M` on six of the major and 13+, `6NT` on 13+, else pass
#   base `3NT` on 13+, uncapped (the default until 2026-11-11)
#
# READ 2026-11-11 (vs BBA, on − base, IMPs/board ±95%, none / ns / ew / both):
#   r1 929dcc74 + dirty, SEED 1794339913594442018, 819,200/cell: plain
#      +0.0010 +0.0012 +0.0009 +0.0012 (±0.0005), win 4/4; PD +0.0010
#      +0.0012 +0.0009 +0.0012, win 4/4; SD plain +0.0012 +0.0017 +0.0014
#      +0.0014, SD-PD +0.0012 +0.0017 +0.0013 +0.0014 (±0.0005), win 4/4;
#      fired 471 / 493 / 463 / 483 (+1.7 to +2.0 IMPs/fired plain);
#      isolation gate passed every cell.  Shipped default-on; `base` is now
#      `--no-ns-two-over-one-quantitative`.  That run's arms were `base`
#      bare and `on --ns-two-over-one-quantitative`.

# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/two-over-one-quantitative.seed.  Iron rule:
# do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-over-one-quantitative.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-over-one-quantitative)

log "=== two-over-one-quantitative A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-two-over-one-quantitative
    arm on "$vul"
    gatepair on base "$vul"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-two-over-one-quantitative
done
log "=== two-over-one-quantitative A/B done"
