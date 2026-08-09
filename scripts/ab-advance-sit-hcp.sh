#!/bin/sh
# ab-advance-sit-hcp.sh — sweep the advancer's 4-card penalty-pass quality
# gate over partner's takeout double (`(1t) X -`). The shipped gate is two
# of the top three honors; the arms swap it for a per-suit HCP floor
# (defense.advance_sit_hcp_gate).  The gates nest, {6+} ⊂ {top2} ⊂ {5+}:
#   off  — default system (top_honors(t, 2..); the shipped behavior)
#   hcp5 — --ns-advance-sit-hcp 5: admits exactly AJxx, removes nothing
#   hcp6 — --ns-advance-sit-hcp 6: drops exactly bare KQxx, keeps KQJx
# Both prior A/Bs on this band (ab-results/advance-penalty-pass/,
# ab-results/advance-pass-yield/) were refuted *narrowings* with the honor
# gate held fixed — the gate itself was never varied.  DD overprices the sit
# (the yield's sd bracket), so plain DD flatters hcp5 (a widening) and
# undersells hcp6 (a narrowing): a plain-DD win for hcp5 needs the sd bracket
# before shipping, a plain-DD loss for hcp6 needs it before burying.
# Exposure is thin (the yield fired ~2/6400 bd); read the per-fired delta,
# not just the per-board wash.  Two vuls, one shared SEED_BASE, arms strictly
# sequential; modeled on scripts/ab-advance-pass-yield.sh; do NOT touch the
# codebase while it runs.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-advance-sit-hcp.sh ab-results/advance-sit-hcp \
#       >ab-results/advance-sit-hcp.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: ab-advance-sit-hcp.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== advance-sit-hcp sweep start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm hcp5 "$vul" --ns-advance-sit-hcp 5
    arm hcp6 "$vul" --ns-advance-sit-hcp 6
    diffpair hcp5 off "$vul"
    diffpair hcp6 off "$vul"
    sddiff hcp5 off "$vul"
    sddiff hcp6 off "$vul"
done
log "=== advance-sit-hcp sweep done"
