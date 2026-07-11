#!/bin/sh
# minor-jump-ab.sh — price the advancer's INVITATIONAL MINOR JUMP on the rich
# advance of partner's takeout double ((1t)-X-(P)-3m).  The rich book is now the
# default, so both arms carry it; this isolates just the new minor jump:
#   base      — default system (rich advance, no minor jump)
#   minorjump — base + --ns-advance-minor-jump (3m = 5+ one-suiter, 10-12,
#               denying a 4-card unbid major; ranks below the notrump ladder)
# The minor jump is the residual for a no-stopper shapely invitational minor that
# would otherwise have to cue; a 4-card unbid major still cues, a stopper still
# bids notrump, and 13+ minors still cue/3NT.  It reopens a rung the original
# rich design excluded (the old broad high-weighted minor jump was a DD leak) —
# so measure whether the narrow, sub-notrump, major-denying version leaks too.
# Both scorers, both vuls, one shared SEED_BASE, arms strictly sequential.
# Modeled on scripts/rich-advance-ab.sh; do NOT touch the codebase while it runs.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/minor-jump-ab.sh ab-results/minor-jump \
#       >ab-results/minor-jump.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: minor-jump-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== minor-jump A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base      "$vul"
    arm minorjump "$vul" --ns-advance-minor-jump
    diffpair minorjump base "$vul"
done
log "=== minor-jump A/B done"
