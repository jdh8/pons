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
#   base   the book today
#   misfit rebid.heart_rebid_misfit_pass: `2NT` over `2♥` needs 12 HCP
#   raise  rebid.minor_rebid_four_card_raise: `3m` on four-card support
#   spade  rebid.spade_jump_rebid_invite: `3♠` on six spades and 10-12 HCP
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
    arm base "$vul"
    arm misfit "$vul" --ns-heart-rebid-misfit-pass
    gatepair misfit base "$vul" || log "isolation gate misfit vs base ($vul) FAILED, scoring anyway"
    diffpair misfit base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.misfit.vs.base.$vul.plain.txt")
    log "pre-count misfit vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff misfit base "$vul" --on-ns-heart-rebid-misfit-pass
    arm raise "$vul" --ns-minor-rebid-four-card-raise
    gatepair raise base "$vul" || log "isolation gate raise vs base ($vul) FAILED, scoring anyway"
    diffpair raise base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.raise.vs.base.$vul.plain.txt")
    log "pre-count raise vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff raise base "$vul" --on-ns-minor-rebid-four-card-raise
    arm spade "$vul" --ns-spade-jump-rebid-invite
    gatepair spade base "$vul" || log "isolation gate spade vs base ($vul) FAILED, scoring anyway"
    diffpair spade base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.spade.vs.base.$vul.plain.txt")
    log "pre-count spade vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff spade base "$vul" --on-ns-spade-jump-rebid-invite
done
log "=== heart-spade-tails A/B done"
