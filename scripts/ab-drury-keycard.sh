#!/bin/sh
# ab-drury-keycard.sh — opener's direct `4NT` over Reverse Drury needs 20
# support points, not 19, vs BBA, four cells, plain DD + perfect defense from
# one solve, plus SD-PD.  Arms are sequential and share one persistent fresh
# seed.
#
#   BOARDS=1638400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-drury-keycard.sh ab-results/drury-keycard \
#       >ab-results/drury-keycard.log 2>&1 < /dev/null &
#
# Why.  On the BBA anchor a0977ce9 (docs/bba-gap-campaign.md, round-2 cut,
# lead 5) opener's `- - 1M - 2♣ - 4NT` on a hand without a singleton or void
# is 100 rows per 409,600 boards, -199 plain / -206 PD; with shortness it is
# +98.  A short hand on 16-20 splinters first, so the 19+ `4NT` sees shortness
# only from 21.  The `.pdd` census (examples/probe-drury-slam.rs, ~100M deals)
# prices the ask (slam on one keycard missing, `5M` on two) against `4M`
# opposite a 9-11 passed hand with 3+ trumps, IMPs per case, not vul:
#
#   opener sp   balanced  no singleton  singleton  void
#   19          -1.90     -0.53         -1.59      -0.71
#   20          +1.28     +2.65         +0.91      +0.91
#   21          +4.58     +5.83         +3.59      +2.98
#
# so 19 with no shortness is the one losing cell the ladder still asks on.
#
#   base response.drury_keycard 19 (the ladder before this run)
#   on   response.drury_keycard 20: a 19 without shortness bids `4M`
#        (the default since this READ; both arms now pass the flag)
#
# Not in this arm: the splinter lane's `4NT` over the interest step (19+,
# a different population: responder has denied a wasted honor), the
# post-answer slam decision (the census: slam on one missing beats needing
# all five from 20 up), the grand-slam rung.
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-23 (0cbc0dfc-dirty, SEED_BASE 1795428182203345817, 1,638,400
# boards per cell, vs BBA): WIN 4/4 on plain and PD; SD disagrees in sign but
# stays non-inferior.
#
#   cell  fired  plain  PD    IMPs/board (plain, 95% CI)  SD plain / SD-PD
#   none  152    +252   +289  +0.0002 ± 0.0001            -116 / -80
#   ns    187    +431   +503  +0.0003 ± 0.0002            -117 / -58
#   ew    148    +242   +277  +0.0001 ± 0.0001            -81 / -47
#   both  181    +457   +529  +0.0003 ± 0.0002            -105 / -48
#
# About +1.6 (we are non-vul) to +2.5 (we are vul) IMPs per fired board, 1 in
# ~10,000 boards.  Isolation gate passed in every cell.  The four cells share
# their deals (one seed), so 4/4 is one deal sample at four vulnerabilities.
# The SD column is negative in every cell (-0.5 to -0.8 IMPs/fired plain):
# on a sampled lead the 19-count slams make more often than DD allows.  SD
# sampling fell back on 6-8% of the base arm's boards against 1% of the on
# arm's, so the SD comparison is not arm-symmetric.
#
# Resumable; SEED_BASE persists in $R/drury-keycard.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-drury-keycard.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for drury-keycard)

log "=== drury-keycard A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul" --ns-drury-keycard 19
    arm on "$vul" --ns-drury-keycard 20
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --on-ns-drury-keycard 20 --off-ns-drury-keycard 19
done
log "=== drury-keycard A/B done"
