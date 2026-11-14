#!/bin/sh
# ab-2nt-short-bid-veto.sh — the short-suit sibling of the 2NT-bid rail
# (`InstinctProfile::their_2nt_short_bid_veto`) against plain american, vs BBA.
#
#   plain   american with the rail off (--no-ns-2nt-short-bid-veto)  (control)
#   on      american as shipped since 2026-11-15: our silent side's suit bids
#           on four cards or fewer over their 2NT opening auction are masked
#
# Found by the R9 trace (2026-11-15, docs/floor-rail-campaign.md): at
# `(2NT) - (3♣)` the v6 floor bids 3♥ on a short suit (26 of 27 such bids
# over 204,800 boards hold fewer than five hearts) and the Law-raise rail
# raises the phantom to 4♥ doubled (12 boards, -159 +-91 plain over the four
# cells of ab-law-raise.sh).  About 27 boards per 204,800 fire, so expect
# non-inferiority, not a win (measured 2026-11-15, before the default flip, with
# the arms spelled `plain` = default and `on` = --ns-2nt-short-bid-veto:
# non-inferior 4/4 plain and PD, a win at ew).  Only fires on boards they open, so the gate
# reads `theirs`; a competitive treatment, hence four vulnerability cells.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-2nt-short-bid-veto.sh ab-results/2nt-short-bid-veto \
#       >ab-results/2nt-short-bid-veto.log 2>&1 &
R=${1:?usage: ab-2nt-short-bid-veto.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== 2nt short bid veto start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm plain "$vul" --our-floor american --no-ns-2nt-short-bid-veto
    arm on    "$vul" --our-floor american
    gatepair on plain "$vul" theirs
    diffpair on plain "$vul"
    sddiff on plain "$vul"
done
log "=== 2nt short bid veto done"
