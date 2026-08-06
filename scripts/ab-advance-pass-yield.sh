#!/bin/sh
# ab-advance-pass-yield.sh — price the weak advancer's pass-yield to a 4-card
# major over partner's takeout double (`(1t) X -`). The cap A/B
# (ab-results/advance-penalty-pass/) refuted strength-restricting the sit
# outright (−2 IMPs/fired, both scorers): the wide sit is a measured edge over
# BBA's own 8.6% convert rate.  This is the surviving sliver of that idea:
# ONLY below the cue band (hcp ≤ 9), a sit holding a 4+ unbid major bids the
# longest-first ladder instead of converting — strong sits stand.  Two arms
# per vul (penalty conversions are vul-sensitive; the cap run was NV-only),
# one shared SEED_BASE, arms strictly sequential:
#   off   — default system (yield off; the shipped behavior)
#   yield — --ns-advance-pass-yield (set_advance_pass_yield_major)
# Both scorers + sd-lead.  Exposure is thin (~2 fired/6400 bd NV — the yield
# needs a clean `(1t) X -` with a weak sit holding a 4+ major), so read the
# per-fired delta, not just the per-board wash.  Modeled on
# scripts/ab-rich-advance.sh; do NOT touch the codebase while it runs.
#
#   setsid nohup scripts/idle-run.sh \
#       scripts/ab-advance-pass-yield.sh ab-results/advance-pass-yield \
#       >ab-results/advance-pass-yield.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: ab-advance-pass-yield.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== advance-pass-yield A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm yield "$vul" --ns-advance-pass-yield
    diffpair yield off "$vul"
    sddiff yield off "$vul"
done
log "=== advance-pass-yield A/B done"
