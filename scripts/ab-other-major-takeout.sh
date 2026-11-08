#!/bin/sh
# ab-other-major-takeout.sh — responder's double of the other-major overcall
# as a takeout for the minors (`competition.other_major_takeout_double`,
# lane (a) of docs/bba-gap-campaign.md, anchor ce114214).
#
#   base  the shipped double: 4+ of THEIR suit, 8+ HCP
#   on    --ns-other-major-takeout-double: 3+♣ 3+♦, ≤2 opener's suit, ≤4 of
#         theirs, 7+ over (1♠), 10+ at the two level; opener's answers, the
#         doubler's second turn and the contested tails authored
#
# Filtered: the double fires on 0.18% of boards.  --filter-other-major-double
# has density ~9.7% and keeps ~89% of the boards where the base double fires
# and ~92% of the boards where the new shape is reachable (measured on the
# anchor's deals).  The verdict is per accepted board; DENSITY rescales it
# to the whole draw, so δ = 0.002 holds per board of the whole draw.
#
# Round 1 (2026-11-09, ad2242e8-dirty, seed 1794167493548294797, density
# 0.0974, IMPs per accepted board, none / both): plain +0.0229 ±0.0023 /
# +0.0255 ±0.0030, PD +0.0264 / +0.0267, win 2/2.  Gate FAILED: 1042 / 946
# divergent boards opened by them, because `european_minors_opponents` copied
# the knob into the declared book and so re-read BBA's own `1M (OM) X` (foreign
# plain +804 / +215, PD −76 / −1009).  Owned alone: plain +8559 / +10239,
# PD +10886 / +11935.  Round 2 pins the knob off there and draws a fresh seed.
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-other-major-takeout.sh ab-results/other-major-takeout-r2 \
#       >ab-results/other-major-takeout-r2.log 2>&1 < /dev/null &
#
# Round 2 read (2026-11-09, ad2242e8-dirty, seed 1794169626132547350, density
# 0.0974, IMPs per accepted board, gates 0 foreign in both cells): plain
# +0.0219 ±0.0022 / +0.0241 ±0.0029, PD +0.0284 ±0.0027 / +0.0284 ±0.0035,
# 5,766 / 5,892 fired → WIN 2/2, shipped default-on.  Re-running needs the
# arms re-pointed: base is now --no-ns-other-major-takeout-double and `on` is
# the default.
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-other-major-takeout.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for other-major-takeout)
F=--filter-other-major-double

log "=== other-major-takeout A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" $F
    arm on "$vul" $F --ns-other-major-takeout-double
    DENSITY=$(density_of "$R/base-$vul")
    log "density ($vul) = $DENSITY"
    gatepair on base "$vul" || log "isolation gate on vs base ($vul) FAILED, scoring anyway"
    diffpair on base "$vul"
done
log "=== other-major-takeout A/B done"
