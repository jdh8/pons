#!/bin/sh
# rich-advance-ab.sh — price making the RICH advance of partner's takeout double
# ([1t, X, P]) the default, chasing the buried-major loss that made flat-book
# longest-first a wash.  The flat advance_double gives the advancer only a
# cheapest natural suit; longest-first there buries a biddable 4-card major under
# a longer minor (5♦4♠ → 1♦, not 1♠) with nothing to rescue it.  The rich book
# adds the missing constructive rung — a 4-card major with 8-10 jumps to 2M
# (defense.rs advance_double_rich) — so the weak hands bid their longest suit
# while the invitational ones show the major.  Three arms per vul, one shared
# SEED_BASE (paired diffs need identical deals), arms strictly sequential:
#   flat     — flat advance_double, highest-ranking (--no-ns-rich-advance
#              --no-ns-longest-advance; the old default, now opt-out)
#   rich     — rich structure, longest-first off (--no-ns-longest-advance)
#   richlong — default system: rich + longest-first (the shipped whole)
# Diffs decompose the change: rich-vs-flat = the structure alone; richlong-vs-flat
# = the full "rich as default" proposition; richlong-vs-rich = the marginal value
# of longest-first once the jump-rescue exists.  Both scorers.  Rubens (the
# right-siding transfer layer, DD-blind) is deliberately left off.  Modeled on
# scripts/longest-advance-ab.sh; do NOT touch the codebase while it runs.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/rich-advance-ab.sh ab-results/rich-advance \
#       >ab-results/rich-advance.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: rich-advance-ab.sh RESULTS_DIR}
PER_SHARD=${PER_SHARD:-12800}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== rich-advance A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm flat     "$vul" --no-ns-rich-advance --no-ns-longest-advance
    arm rich     "$vul" --no-ns-longest-advance
    arm richlong "$vul"
    diffpair rich     flat "$vul"
    diffpair richlong flat "$vul"
    diffpair richlong rich "$vul"
done
log "=== rich-advance A/B done"
