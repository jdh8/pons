#!/bin/sh
# advance-2nt-ab.sh — price the doubler's ACCEPT/DECLINE of the advancer's
# invitational 2NT on the rich advance of a takeout double
# ((1t)-X-(P)-2NT-(P)-?).  The 2NT invite is default-on, but its continuation
# falls to the instinct floor, which treats 2NT as non-forcing and PASSES even
# game-going hands — this A/B measures authoring the natural accept/decline:
#   base    — 2NT continuation left to the floor (--no-ns-advance-2nt-continuation)
#   twont   — default system: authored accept/decline (Pass = decline, 3NT = accept
#             to play, new 5-card major = game-forcing; advancer places game)
# SHIPPED default-on 2026-07-11: wash-positive all four cells (NV/vul × plain/PD),
# fires ~6/409.6k so it can't move the aggregate but never loses — fixing the
# floor-passes-a-game bug in the default-on rich advance earns the flip.
# Both scorers, both vuls, one shared SEED_BASE, arms strictly sequential.
# Modeled on scripts/minor-jump-ab.sh; do NOT touch the codebase while it runs.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/advance-2nt-ab.sh ab-results/advance-2nt \
#       >ab-results/advance-2nt.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: advance-2nt-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== advance-2nt A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm base  "$vul" --no-ns-advance-2nt-continuation
    arm twont "$vul"
    diffpair twont base "$vul"
done
log "=== advance-2nt A/B done"
