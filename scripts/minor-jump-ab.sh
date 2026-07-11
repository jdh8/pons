#!/bin/sh
# minor-jump-ab.sh — price the advancer's INVITATIONAL MINOR JUMP on the rich
# advance of partner's takeout double ((1t)-X-(P)-3m).  The minor jump is now the
# default; this A/B isolates it against the floor by turning it OFF in one arm:
#   base      — minor jump OFF (--no-ns-advance-minor-jump); the rung falls to
#               the floor
#   minorjump — default system: 3m = 5+ one-suiter, 10-12, denying a 4-card unbid
#               major (ranks below the notrump ladder), plus the doubler's
#               stopper-ask cue continuation
# The minor jump is the residual for a no-stopper shapely invitational minor that
# would otherwise have to cue; a 4-card unbid major still cues, a stopper still
# bids notrump, and 13+ minors still cue/3NT.  SHIPPED default-on 2026-07-11: two
# seeds SIG+ in all four cells (plain >= PD -> constructive, not a doubling
# artifact); the doubler's Western-cue stopper-ask right-sides the 3NT.
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
    arm base      "$vul" --no-ns-advance-minor-jump
    arm minorjump "$vul"
    diffpair minorjump base "$vul"
done
log "=== minor-jump A/B done"
