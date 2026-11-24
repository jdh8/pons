#!/bin/sh
# ab-heart-spade-tails.sh — three natural fixes to responder's rebid in `1♥ - 1♠ - 2x`, one arm each,
# vs BBA, four cells, plain DD + perfect defense from one solve, plus SD-PD.
# Arms are sequential and share one persistent fresh seed.
#
#   BOARDS=1638400 scripts/queue-ab.sh heart-spade-tails \
#       scripts/ab-heart-spade-tails.sh ab-results/heart-spade-tails
#
# Why.  On the BBA anchor a0977ce9 (docs/bba-gap-campaign.md, round-2 cut,
# lead 7) responder's `2NT` on 10-12 over `1♥ - 1♠ - 2x` is 490 rows, -494
# plain / -781 PD, diffuse.  Three slices of it, per 409,600 boards:
#
#   misfit  over `2♥` the `2NT` is a singleton or void (two hearts raise).
#           Opener declines with `3♥` on 112 of 149 rows: -242 / -394, where
#           BBA passed `2♥`.  By HCP: 9 -93, 10 -69, 11 -60, 12 -3 plain.
#   raise   over `2♦`, four-card support bids `2NT` where BBA raises to `3♦`
#           (our `3m` needs five): 60 rows, -148 / -202.
#   spade   six or more spades bid `2NT` where BBA bids `3♠` or `4♠`:
#           68 rows, -146 / -137.
#
#   base   the book before this READ (all three knobs off: `--no-ns-…`)
#   misfit rebid.heart_rebid_misfit_pass: `2NT` over `2♥` needs 12 HCP
#   raise  rebid.minor_rebid_four_card_raise: `3m` on four-card support
#   spade  rebid.spade_jump_rebid_invite: `3♠` on six spades and 10-12 HCP
#   (all three the default since this READ; each arm turns the other two off)
#          (opener raises on three, or two and 13+; `3NT` on a short spade and
#          15+ HCP), `4♠` on seven
#
# Each arm is read against base on its own; they touch different hands, so a
# sum is the package.  Not in any arm: the `2NT` rows where BBA bids `3NT`
# (52 rows, -111 / -62: an invite-floor question), `2♠` over the `2♥` rebid.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-25 (ffb2017f, SEED_BASE 1795499692299080341, 1,638,400 boards
# per cell, vs BBA): all three arms pass 4/4 on plain, PD, plain SD and SD-PD;
# isolation gate PASSED in all twelve cells.
#
#   arm    cell  fired  plain  PD     IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   misfit none  833    +1074  +1948  +0.0007 ± 0.0001            +567 / +1255
#   misfit ns    916    +1352  +2725  +0.0008 ± 0.0002            +453 / +1538
#   misfit ew    834    +1100  +1979  +0.0007 ± 0.0001            +596 / +1289
#   misfit both  912    +1375  +2758  +0.0008 ± 0.0002            +465 / +1579
#   raise  none  425    +140   +197   +0.0001 ± 0.0001 (non-inf)  +151 / +176
#   raise  ns    464    +306   +365   +0.0002 ± 0.0001            +425 / +464
#   raise  ew    424    +147   +204   +0.0001 ± 0.0001 (non-inf)  +139 / +155
#   raise  both  463    +306   +365   +0.0002 ± 0.0001            +429 / +470
#   spade  none  298    +605   +696   +0.0004 ± 0.0001            +560 / +622
#   spade  ns    322    +856   +979   +0.0005 ± 0.0002            +768 / +838
#   spade  ew    298    +605   +696   +0.0004 ± 0.0001            +560 / +622
#   spade  both  318    +825   +949   +0.0005 ± 0.0001            +742 / +817
#
# Package (sum, the arms own disjoint nodes): about +0.0012 / +0.0015 IMPs/board
# plain (we are non-vul / vul).  misfit wins on every scorer, and PD is about
# twice plain: passing `2♥` saves the doubled `3♥` / `3NT`.  raise is a win
# vul and non-inferior non-vul (lower bound 0.0000, point estimate positive).
# spade is +2.0 to +2.7 IMPs per fired board.  The cells share their deals (one
# seed), so 4/4 is one deal sample at four vulnerabilities; spade's none and ew
# cells are byte-equal because this uncontested node never sees EW's vul.
# Residue: 8 of each cell's 50 worst misfit boards (−57 / −85 IMPs) are 6+♠
# misfits that now pass `2♥`.  Responder's table over `1♥ - 1♠ - 2♥` has no
# spade call at all, which is the `2♠` gap named above.
# All three shipped default-on on this READ (jdh8, 2026-11-25).
#
# Resumable; SEED_BASE persists in $R/heart-spade-tails.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-heart-spade-tails.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for heart-spade-tails)

log "=== heart-spade-tails A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --no-ns-heart-rebid-misfit-pass --no-ns-minor-rebid-four-card-raise --no-ns-spade-jump-rebid-invite
    arm misfit "$vul" --no-ns-minor-rebid-four-card-raise --no-ns-spade-jump-rebid-invite
    gatepair misfit base "$vul" || log "isolation gate misfit vs base ($vul) FAILED, scoring anyway"
    diffpair misfit base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.misfit.vs.base.$vul.plain.txt")
    log "pre-count misfit vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff misfit base "$vul" --on-no-ns-minor-rebid-four-card-raise --on-no-ns-spade-jump-rebid-invite --off-no-ns-heart-rebid-misfit-pass --off-no-ns-minor-rebid-four-card-raise --off-no-ns-spade-jump-rebid-invite
    arm raise "$vul" --no-ns-heart-rebid-misfit-pass --no-ns-spade-jump-rebid-invite
    gatepair raise base "$vul" || log "isolation gate raise vs base ($vul) FAILED, scoring anyway"
    diffpair raise base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.raise.vs.base.$vul.plain.txt")
    log "pre-count raise vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff raise base "$vul" --on-no-ns-heart-rebid-misfit-pass --on-no-ns-spade-jump-rebid-invite --off-no-ns-heart-rebid-misfit-pass --off-no-ns-minor-rebid-four-card-raise --off-no-ns-spade-jump-rebid-invite
    arm spade "$vul" --no-ns-heart-rebid-misfit-pass --no-ns-minor-rebid-four-card-raise
    gatepair spade base "$vul" || log "isolation gate spade vs base ($vul) FAILED, scoring anyway"
    diffpair spade base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.spade.vs.base.$vul.plain.txt")
    log "pre-count spade vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff spade base "$vul" --on-no-ns-heart-rebid-misfit-pass --on-no-ns-minor-rebid-four-card-raise --off-no-ns-heart-rebid-misfit-pass --off-no-ns-minor-rebid-four-card-raise --off-no-ns-spade-jump-rebid-invite
done
log "=== heart-spade-tails A/B done"
