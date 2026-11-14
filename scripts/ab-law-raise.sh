#!/bin/sh
# ab-law-raise.sh — the Law-raise rail (`InstinctProfile::advance_law_raise`)
# against plain american, vs BBA.
#
#   plain   american with the rail off (--ns-law-raise off)         (control)
#   law1    --ns-law-raise 1: the raise must sit one level inside the Law
#   law0    --ns-law-raise 0: the raise may reach the Law level
#
# With four or more trumps for partner's overcall (they opened, we have only
# passed, partner bid one suit read as five or more) the learned floor passes
# where both references raise.  Census 2026-11-14 on the boards where the
# reference acts and we pass: 572 boards, -972 +-301 plain / -502 +-337 PD
# over 409.6k vs BBA (b3ffe8ee); 50 boards, -130 +-91 / -60 +-107 over 20k vs
# BEN (f43436de).  By slack vs BBA: at the Law level 330 boards, -464 / -68;
# one inside it 229, -477 / -429.  The census cannot see the boards where both
# pass and the rail now raises alone; this run prices them.
#
# Only fires on boards they open, so the gate reads `theirs`.  A competitive
# treatment, hence four vulnerability cells.  Part of the raise's worth is
# obstruction that DD underprices (CLAUDE.md, iron rules): a plain-DD loss
# here is not a refutation without the single-dummy pair.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-law-raise.sh ab-results/law-raise \
#       >ab-results/law-raise.log 2>&1 &
R=${1:?usage: ab-law-raise.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== law raise start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul vuls=$VULS"
for vul in $VULS; do
    arm plain "$vul" --our-floor american --ns-law-raise off
    arm law1  "$vul" --our-floor american --ns-law-raise 1
    arm law0  "$vul" --our-floor american --ns-law-raise 0
    for on in law1 law0; do
        gatepair "$on" plain "$vul" theirs
        diffpair "$on" plain "$vul"
        sddiff "$on" plain "$vul"
    done
    diffpair law0 law1 "$vul"
done
log "=== law raise done"
