#!/bin/sh
# ab-two-over-one-reverse-quantitative.sh — responder's quantitative bands
# lowered after opener's reverse in a 2/1, vs BBA, four cells, plain DD +
# perfect defense from one solve, plus SD-PD.  Arms are sequential and share
# one persistent fresh seed.
#
#   BOARDS=6553600 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-reverse-quantitative.sh ab-results/two-over-one-reverse-quantitative \
#       >ab-results/two-over-one-reverse-quantitative.log 2>&1 < /dev/null &
#
# Why.  `rebid.two_over_one_quantitative` keeps responder's bands flat
# (`3NT` 13–17, `4NT` 18–20, `6NT` 21+) whatever opener rebid.  Under
# `two_over_one_reverse_extras` a reverse (`1♥ - 2m - 2♠`, `1♦ - 2♣ - 2M`)
# shows `points(15..)`: 14+ HCP on 95% of the 1,359 such boards per
# 3,276,800 in ab-results/two-over-one-opener-invite (on-none), where a
# 16–17 count bid `3NT` and opener passed it below 16.
#
#   on   rebid.two_over_one_reverse_quantitative (default on since
#        2026-11-12): after a reverse `3NT` is 13–15, `4NT` exactly 16,
#        `6NT` 17+ (31 combined facing 14); opener raises the `4NT` on 15+
#   base the flat bands (the default until 2026-11-12)
#
# Sized by rewriting that run's auctions and pricing them with
# `probe-divergence --imps` (one table, 63 moved boards of 180): "`4NT`
# 16–17, accept on 15+" +107 / +125 IMPs plain none/both (PD +100 / +116);
# "accept on 14+" +148 / +171 (PD +133 / +156).  The authored bands sit
# between.  Both tables: about +250 IMPs per 3,276,800 boards per cell,
# +0.00008 IMPs/board on ~126 fired, so 6,553,600 boards is the smallest
# run likely to clear zero.  The quantitative `4NT` met interference on 0 of
# 2,765 boards there: its interfered tails stay unauthored.
#
# READ 2026-11-12 (vs BBA, on − base, none / ns / ew / both):
#   r1 4eb9267b + dirty, SEED 1794389024466134412, 6,553,600/cell: plain
#      +277 / +328 / +255 / +328 IMPs (+0.00004 to +0.00005 IMPs/board; the
#      reporter prints the CI as ±0.0000 and its lower bound is above 0),
#      win 4/4; PD +272 / +319 / +250 / +319, win 4/4; SD plain +513 / +674
#      / +407 / +592, SD-PD +510 / +675 / +408 / +587 (+0.0001), win 4/4;
#      fired 135 / 140 / 132 / 136 (+1.93 to +2.41 IMPs/fired plain), SD
#      fired 243–256; isolation gate passed every cell.  Shipped default-on;
#      `base` is now `--no-ns-two-over-one-reverse-quantitative`.  That
#      run's arms were `base` bare and `on
#      --ns-two-over-one-reverse-quantitative`.  The four cells score the
#      same deals, so 4/4 is one sample, not four.  The sizing above
#      doubled for "both tables", but pons sits NS only: per 6,553,600 it
#      predicted ~126 fired and +214 / +250 IMPs none/both, which is what
#      the run found.  The ON arm's SD sampler fell back on 38–41% of
#      worlds vs 25–26% OFF (the draw budget, diagnosed in
#      ab-two-over-one-quantitative.sh).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/two-over-one-reverse-quantitative.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-over-one-reverse-quantitative.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-over-one-reverse-quantitative)

log "=== two-over-one-reverse-quantitative A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-two-over-one-reverse-quantitative
    arm on "$vul"
    gatepair on base "$vul"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-two-over-one-reverse-quantitative
done
log "=== two-over-one-reverse-quantitative A/B done"
