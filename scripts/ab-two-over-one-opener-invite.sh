#!/bin/sh
# ab-two-over-one-opener-invite.sh — opener's quantitative raise of
# responder's capped `3NT` after a no-fit 2/1, vs BBA, four cells, plain DD +
# perfect defense from one solve, plus SD-PD.  Arms are sequential and share
# one persistent fresh seed.
#
#   BOARDS=819200 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-opener-invite.sh ab-results/two-over-one-opener-invite \
#       >ab-results/two-over-one-opener-invite.log 2>&1 < /dev/null &
#
# Why.  `rebid.two_over_one_quantitative` (shipped 2026-11-11) caps
# responder's `3NT` at 17 HCP, so the floor no longer raises it to `6NT`.
# In that ship's run (ab-results/two-over-one-quantitative, priced with
# `probe-divergence --imps`) "opener passed `3NT` where the baseline bid
# `6NT`" was 64–66 boards per cell, −154 / −205 / −154 / −192 IMPs plain
# none/ns/ew/both: combined 29–30 HCP gained by passing, 31+ lost.
#
#   on   rebid.two_over_one_opener_invite (default on since 2026-11-11):
#        over the capped `3NT` opener bids `6NT` on 18+ HCP and a natural
#        `4NT` on 16–17; responder raises `4NT` to `6NT` on 15+ (31
#        combined), else passes
#   base opener's call over `3NT` is the floor's (the default until
#        2026-11-11)
#
# Expect a small effect: the priced bucket is about −0.0002 IMPs/board, so
# 819,200 boards (±0.0005 last run) may read inconclusive; widen BOARDS
# before calling it.
#
# READ 2026-11-11 (vs BBA, on − base, IMPs/board ±95%, none / ns / ew / both):
#   r1 6056ef59 + dirty, SEED 1794350445707357507, 3,276,800/cell: plain
#      +0.0003 +0.0003 +0.0003 +0.0003 (±0.0002; +1019 / +1117 / +1016 /
#      +1146 IMPs), win 4/4; PD +0.0003 every cell (+897 / +970 / +894 /
#      +1003), win 4/4; SD plain +0.0004 +0.0005 +0.0004 +0.0004, SD-PD
#      +0.0003 +0.0004 +0.0003 +0.0003 (±0.0002), win 4/4; fired 1281 /
#      1334 / 1270 / 1316 (+0.80 to +0.87 IMPs/fired plain).  Shipped
#      default-on; `base` is now `--no-ns-two-over-one-opener-invite`.
#      That run's arms were `base` bare and `on
#      --ns-two-over-one-opener-invite`.  The ON arm's SD sampler fell
#      back on 25% of worlds vs 4% OFF.
#
# CENSUS 2026-11-12 (r1's divergent boards priced by opener's rebid, no new
# A/B; plain IMPs none / both per 3,276,800 boards): no-fit +746 / +854,
# jump `3M` +19 / +19, raises +265 / +294 on 232-243 boards.  The only
# losing raise prefix is `1♠ - 2♥ - 3♥`: 7 boards, -61 / -72.  Excluding
# every raise prefix gives up a win; excluding the heart raise alone is 2
# boards per million, unmeasurable.  Raise-prefix flag closed, as shipped.
# Lead beside it, unsized: at `1♠ - 2♥ - 3♥ -` (656 boards, none) responder
# bids `3NT` on 208 with a nine-card heart fit (`responder_rebid` has no rule
# for the raised suit) and we end in `6♥` on ~120 of those where BBA stops in
# game on ~55; 8 boards pass `3♥` in a game force.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Resumable; SEED_BASE persists in $R/two-over-one-opener-invite.seed.  Iron
# rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-over-one-opener-invite.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-over-one-opener-invite)

log "=== two-over-one-opener-invite A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-two-over-one-opener-invite
    arm on "$vul"
    gatepair on base "$vul"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-two-over-one-opener-invite
done
log "=== two-over-one-opener-invite A/B done"
