#!/bin/sh
# ab-rkcb-five-clubs-four.sh — an asker with none or one keycard reads a `5♣`
# answer as four in every book RKCB lane, vs BBA, four cells, plain DD +
# perfect defense from one solve, plus SD-PD.  Arms are sequential and share
# one persistent fresh seed.
#
#   BOARDS=1638400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-rkcb-five-clubs-four.sh ab-results/rkcb-five-clubs-four \
#       >ab-results/rkcb-five-clubs-four.log 2>&1 < /dev/null &
#
# Why.  The super-accept A/B (scripts/ab-two-notrump-super-accept.sh, round 3)
# shipped this reading in the one lane facing a 20+ hand, where the plain
# table had signed off in `5M` facing four keycards (11 boards per 1.6M,
# -100 plain IMPs in round 2).  The floor's own decode (`resolve_total`)
# already reads every lane this way; the book's asker tables do not.
#
#   base the book's asker takes `5♣` for one unless it holds three or four
#   on   decision.reading.rkcb_five_clubs_four: none of our own takes the
#        queen relay where there is room (else six), one bids six, two still
#        signs off; minors too (`6m` on none or one, no relay)
#
# Not in this arm: a grand try on one keycard of our own, the `5♦` answer
# (already read high on one or two), the doubled-answer tails
# (`park/rkcb-doubled-answer`).
#
# Verdict per docs/measurement.md's decision table: plain DD is the verdict,
# PD the guard; delta = 0.002 IMPs/board.  Read each cell's `fired` count first.
#
# READ 2026-11-23 (SEED_BASE 1795382556049855042, 1,638,400 bd/arm/cell,
# sha 54f59270-dirty, isolation gate passed 4/4): NOT A WIN.  Plain DD is
# negative in every cell; PD and SD-PD lose where NS is vulnerable.
#   cell  fired  plain IMPs/bd (95% CI)      PD IMPs/bd      SD plain / SD-PD
#   none   112   -0.0001 ±0.0001 non-inf     -0.0001 non-inf -0.0000 / -0.0001
#   ns     119   -0.0001 ±0.0001 non-inf     -0.0002 loss    -0.0001 / -0.0002 loss
#   ew     108   -0.0000 ±0.0001 non-inf     -0.0001 non-inf -0.0000 / -0.0001
#   both   118   -0.0001 ±0.0001 non-inf     -0.0002 loss    -0.0001 / -0.0002 loss
# Plain IMPs per cell: -90 / -170 / -66 / -167 (none/ns/ew/both).
# Split by base's continuation after `5♣ - 5M` (ab-dump-diff --show 200):
#   partner bids 7 over the sign-off  24-26 bd  -114 / -146 / -114 / -146
#   sign-off passed out               69-78 bd   -60 / -122 /  -36 / -119
#   other (partner raises to 6, 6NT)  15 bd      +84 /  +98 /  +84 /  +98
# The premise fails.  Outside the lane facing 20+, an asker with none or one
# keycard still meets partners holding one (splinters, Jacoby 2NT): about
# 50 boards per cell where six fails, -2 or -11 each, against 20 where
# partner held four and passed 5M.  And when partner does hold four, base's
# sign-off already works: partner bids the grand, which makes 18-20 times in
# 24-26.  Bidding six takes that chance away.  The super-accept lane, where
# one or two keycards in total is impossible, keeps its reading
# unconditionally.  No credible flip: jdh8 dropped the knob uncommitted
# (2026-11-23), so this runner is a record and its `--ns-rkcb-five-clubs-four`
# flags no longer exist.
#
# Resumable; SEED_BASE persists in $R/rkcb-five-clubs-four.seed.
# Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-rkcb-five-clubs-four.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for rkcb-five-clubs-four)

log "=== rkcb-five-clubs-four A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm base "$vul"
    arm on "$vul" --ns-rkcb-five-clubs-four
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
    fired=$(sed -n 's/.*: \([0-9]*\) fired (.*/\1/p' "$R/diff.on.vs.base.$vul.plain.txt")
    log "pre-count on vs base $vul: ${fired:-0} contract-divergent boards$([ "${fired:-0}" -lt 50 ] && echo ' (< 50): INSUFFICIENT DATA for this cell')"
    sddiff on base "$vul" --on-ns-rkcb-five-clubs-four
done
log "=== rkcb-five-clubs-four A/B done"
