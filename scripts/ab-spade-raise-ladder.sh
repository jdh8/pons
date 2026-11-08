#!/bin/sh
# ab-spade-raise-ladder.sh — opener's spade raise ladder after `1x - 1♠`
# (*Open work* item 9 of docs/bba-gap-campaign.md; 170 boards, −434 plain on
# the `6141cdb2` anchor).
#
# Round 1 (2026-11-08, seed 1794069695886318620) ran one arm with both
# knobs: none cell plain −0.0017 ±0.0015 (loss, 95 fired), PD −0.0020; the
# both cell stopped at the isolation gate (1 foreign board: our defender's
# call over their `1♣ - 1♠ - 4♠ - 4NT - 5♣`).  Round 2 splits it:
#
#   ranges   (--ns-spade-raise-ladder)   opener's `2♠` 12–14, `3♠` 15–16, `4♠` 17+
#   answers  (--ns-spade-raise-answers)  responder's `3♠` (`4♠` on 7+, RKCB on
#            14+) and `4♠` (RKCB on 12+) nodes after `1♣`/`1♦`/`1♥`, both tails
#   full     both knobs
#
# The ranges knob also moves how we read their raise ladder, so its gate may
# show foreign boards; the runner logs a failed gate and goes on to score, and
# the verdict must name the foreign count.
#
# Round 2 read (seed 1794073969209614479, 0 foreign in every gate): ranges
# non-inferior 2/2, plain +0.0007 / +0.0013, PD +0.0008 / +0.0011 → shipped
# default-on 2026-11-08; answers −0.0009 / −0.0013 (inconclusive); full
# −0.0003 / +0.0001.  Since the flip, `base` is the shipped ranges; the old
# ranges are --no-ns-spade-raise-ladder.  Re-running needs the arms below
# re-pointed (base = --no-ns-spade-raise-ladder).
#
# Round 3 (2026-11-08) re-measures both knobs under `--filter-spade-raise`
# (density ~9.5%, 100% coverage of round 2's divergent boards), 4x round 2's
# boards, to resolve what round 2 could only bound:
#
#   base     the shipped ranges
#   old      --no-ns-spade-raise-ladder (the 12–15 / 16–18 / 19+ ladder)
#   answers  --ns-spade-raise-answers on top of the shipped ranges
#
# Read base vs old (does the shipped ladder win?) and answers vs base.  The
# verdict is per *accepted* board; DENSITY rescales it to all boards, so
# δ = 0.002 holds per board of the whole draw.
#
# Arm 2 (opener splinters, sub-arms 2a/2b on singleton honours) is measured
# on top of the shipped ranges under the same filter.
#
# Round 3 read (2026-11-08, 4eac08d6, seed 1794082373901736538, density
# 0.094228, IMPs per accepted board, gates 0 foreign except 1 in
# answers/both):
#   base vs old     plain +0.0028 ±0.0024 / +0.0067 ±0.0032 (none / both),
#                   PD +0.0010 / +0.0039 → the shipped ladder WINS 2/2
#   answers vs base plain −0.0053 ±0.0018 / −0.0056 ±0.0023, PD −0.0058 /
#                   −0.0063 → LOSS 2/2, stays opt-in.  Traced: passing 9–11
#                   HCP over the 17+ `4♠` where the floor bids `6♠` (−1032 on
#                   474 boards, none), RKCB then stopping short of the floor's
#                   `7♠` (−487 / 99), `4♠` over `3♠` on 11–13 HCP where the
#                   floor bids `6♠` (−331 / 102); `4♠` on 7+ over `3♠` wins
#                   (+98 / 336).  The asks (12+ / 14+ points) are too high.
#
# Round 4 (2026-11-08) sweeps the two asks (item 9 (c)), re-bucketed in
# *points* from round 3's per-board trace: over the 17+ `4♠`, passing lost
# at 10 / 11 (−114 / −914 IMPs on 173 / 328 boards) and won at 8 / 9; over
# the 15–16 `3♠`, `4♠` lost at 11 / 12 / 13 (−34 / −110 / −198 on 11 / 26 /
# 64).  The nodes are disjoint, so each ask is swept alone with the other at
# its default, every arm `--ns-spade-raise-answers` on the shipped ranges,
# diffed vs base (answers off):
#
#   base       the shipped ranges, answers off
#   three-N    --ns-spade-raise-three-ask N, N in 11 12 13 (RKCB over `3♠`)
#   four-N     --ns-spade-raise-four-ask N,  N in 10 11    (RKCB over `4♠`)
#
# Ship a value only when non-inferior 2/2 plain (δ = 0.002 per board of the
# whole draw).  DIFF_JSONL=1 keeps the per-board trace for the next
# re-bucketing.
#
# Round 4 read (2026-11-08, bca5f355-dirty, seed 1794087241043204556,
# density 0.094602, plain IMPs per accepted board none / both, PD agreeing,
# gates 1 / 4 foreign in none / both):
#   three-11  −0.0150 ±0.0023 / −0.0198 ±0.0029    four-10  −0.0096 / −0.0125
#   three-12  −0.0083 ±0.0021 / −0.0107 ±0.0026    four-11  −0.0050 / −0.0062
#   three-13  −0.0066 ±0.0019 / −0.0086 ±0.0024
# LOSS 12/12, monotone in the ask: more RKCB, more loss.  The trace splits
# the loss between the non-RKCB answers (`4♠` / pass where the floor jumps
# to slam, −1816 on 1080 boards in three-13 none) and the RKCB auctions we
# add (−655 on 538), so the asks are not the lever; nothing ships, defaults
# stay 14 / 12.  Item 9 (c) is closed.
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-spade-raise-ladder.sh ab-results/spade-raise-ladder-r4 \
#       >ab-results/spade-raise-ladder-r4.log 2>&1 < /dev/null &
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-spade-raise-ladder.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for spade-raise-ladder)
F=--filter-spade-raise

log "=== spade-raise-ladder A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
A=--ns-spade-raise-answers
for vul in $VULS; do
    arm base "$vul" $F
    for n in 11 12 13; do arm three-$n "$vul" $F $A --ns-spade-raise-three-ask $n; done
    for n in 10 11; do arm four-$n "$vul" $F $A --ns-spade-raise-four-ask $n; done
    DENSITY=$(density_of "$R/base-$vul")
    log "density ($vul) = $DENSITY"
    for a in three-11 three-12 three-13 four-10 four-11; do
        gatepair $a base "$vul" || log "isolation gate $a vs base ($vul) FAILED, scoring anyway"
        diffpair $a base "$vul"
    done
done
log "=== spade-raise-ladder A/B done"
