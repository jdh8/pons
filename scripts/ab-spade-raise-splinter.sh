#!/bin/sh
# ab-spade-raise-splinter.sh — opener's splinter raise over `1x - 1♠` (*Open
# work* item 9 (b) of docs/bba-gap-campaign.md), on top of the shipped ladder.
#
#   base   --ns-spade-raise-splinter off: the shipped 12–14 / 15–16 / 17+
#          ladder, a blind `4♠` on 17+
#   small  --ns-spade-raise-splinter 1: splinter on a small singleton or a
#          void (BBA's rule); responder RKCB on points − ½ × waste ≥ 10
#   any    --ns-spade-raise-splinter 4: the same on any singleton (the
#          default since round 3)
#
# Read small and any vs base per accepted board; DENSITY rescales δ = 0.002
# to the whole draw.
#
# Round 1 (2026-11-10, seed 1794257911236651474, 409,600 boards/cell, density
# 0.094): LOSS 4/4. small plain −0.0015 ±0.0011 / −0.0021 ±0.0014 (none /
# both), any −0.0021 ±0.0014 / −0.0028 ±0.0017, PD agreeing. Responder's
# 4♠ sign-off holding the ace of the short suit (half-counted as waste) where
# the floor's 6♠ over a blind 4♠ makes.
# Round 2 (2026-11-10, seed 1794297366600873980, K/Q/J-only waste): NON-INFERIOR
# 8/8, no win. small plain +0.0005 ±0.0011 / ±0.0014, any +0.0005 ±0.0014 /
# ±0.0017, PD the same. Residue: RKCB 6♠ where the floor's 7♠ makes.
# Round 3 (2026-11-10, seed 1794301689386795430, the grand rung and the
# fit-aware gate; base ran with the flag absent, which was off then): WIN 8/8.
# small plain +0.0040 ±0.0012 / +0.0051 ±0.0015 (none / both), any +0.0057
# ±0.0016 / +0.0071 ±0.0020, PD within 0.0001; any vs small +0.0018 ±0.0010 /
# +0.0020 ±0.0012. Shipped default-on at 4; see CHANGELOG.
#
#   BOARDS=409600 DIFF_JSONL=1 setsid nohup scripts/idle-run.sh \
#       scripts/ab-spade-raise-splinter.sh ab-results/spade-raise-splinter-r1 \
#       >ab-results/spade-raise-splinter-r1.log 2>&1 < /dev/null &
#
# Each new seed wants a fresh results dir.
R=${1:?usage: ab-spade-raise-splinter.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for spade-raise-splinter)
F=--filter-spade-raise

log "=== spade-raise-splinter A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in $VULS; do
    arm base "$vul" $F --ns-spade-raise-splinter off
    arm small "$vul" $F --ns-spade-raise-splinter 1
    arm any "$vul" $F --ns-spade-raise-splinter 4
    DENSITY=$(density_of "$R/base-$vul")
    log "density ($vul) = $DENSITY"
    for a in small any; do
        gatepair $a base "$vul" || log "isolation gate $a vs base ($vul) FAILED, scoring anyway"
        diffpair $a base "$vul"
    done
done
log "=== spade-raise-splinter A/B done"
