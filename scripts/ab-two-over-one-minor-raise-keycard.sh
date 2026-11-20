#!/bin/sh
# ab-two-over-one-minor-raise-keycard.sh — responder's `4NT` as RKCB for its
# minor after opener raises it in a 2/1, vs BBA, four cells, plain DD +
# perfect defense from one solve, plus SD-PD.  Arms are sequential and share
# one persistent fresh seed.
#
#   BOARDS=3276800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-two-over-one-minor-raise-keycard.sh ab-results/two-over-one-minor-raise-keycard \
#       >ab-results/two-over-one-minor-raise-keycard.log 2>&1 < /dev/null &
#
# Why.  `rebid.two_over_one_quantitative` gives every no-fit 2/1 node the
# bands `3NT` 13–17, `4NT` 18–20, `6NT` 21+, and opener's raise of
# responder's minor (`1M - 2m - 3m`, `1♦ - 2♣ - 3♣`) took them too: an 18–20
# count bids a natural `4NT` on a nine-card fit, opener passes it on 12 or
# bids `6NT`.  On the BBA anchor `b13dd9fe` (shipping arm, 819,200 boards)
# the `4NT` 18–20 node is 141 rows, −385 plain / −390 PD; the prefixes
# `1♠ - 2♣ - 3♣`, `1♦ - 2♣ - 3♣`, `1♥ - 2♦ - 3♦` are 25 of them and −284 /
# −296.  BBA bids `4m`, cues, keycards and plays `6m` (or `7NT`).
#
#   on   rebid.two_over_one_minor_raise_keycard: `3NT` keeps 13–17, every
#        18+ bids `4NT` RKCB for the minor, answered by the book's ladder
#   base the quantitative bands (`--no-ns-two-over-one-minor-raise-keycard`;
#        the default when this ran, the flags flipped with the default
#        2026-11-20)
#
# Sized off the anchor: ~25 fired per 819,200, so ~100 per 3,276,800 per
# cell; at the anchor's −11 IMPs per fired board, recovering half is about
# +500 IMPs per cell (+0.00015 IMPs/board).  The `4NT` ask's interfered
# tails are the RKCB ladder's own (unauthored, as every RKCB lane).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; δ = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# Amendment 2026-11-19 15:50Z, before any score was read: the first launch
# died at the none cell's isolation gate, 1 of 272 divergent boards opened by
# them (`1♥ - 2♣ - 3♣ - 4NT - 5♥`: BBA's own RKCB; the on arm's mirror read of
# their `4NT` is the faithful one and we stop doubling the `5♥` answer).  As in
# ab-heart-raise-splinter.sh the gate logs and the cell is scored anyway; at
# read time report the sides apart (`ab-dump-bucket --by side`).
#
# READ 2026-11-20 (58c9f4a3-dirty, SEED_BASE 1795086497417332106, 3,276,800
# boards per cell, vs BBA): WIN 4/4, plain, PD and SD-PD all agree.
#
#   cell  fired  plain   PD      IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   none  272    +755    +760    +0.0002 ± 0.0001            +856  / +846
#   ns    296    +1047   +1064   +0.0003 ± 0.0001            +1104 / +1113
#   ew    261    +737    +745    +0.0002 ± 0.0001            +785  / +787
#   both  280    +990    +1007   +0.0003 ± 0.0001            +1061 / +1063
#
# About +2.8 (non-vul) to +3.5 (vul) IMPs per fired board.  Foreign boards:
# 1 each in none and ns (the same deal, their `1♥ - 2♣ - 3♣ - 4NT - 5♥`), 0 in
# ew and both.  `--by side`, plain / PD: we open none +757 / +765, ns +1049 /
# +1069; they open −2 / −5 in each (we no longer double their `5♥` answer).
# The worst boards are `6m` where the baseline's `6NT` made (a ruff or a trump
# loser), and `6m` bid where the baseline passed `4NT`; untraced, as the net
# is a win in every cell.  Shipped default-on.
#
# Resumable; SEED_BASE persists in $R/two-over-one-minor-raise-keycard.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-two-over-one-minor-raise-keycard.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for two-over-one-minor-raise-keycard)

log "=== two-over-one-minor-raise-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-two-over-one-minor-raise-keycard
    arm on "$vul"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED (their 4NT mirror-read), scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --off-no-ns-two-over-one-minor-raise-keycard
done
log "=== two-over-one-minor-raise-keycard A/B done"
