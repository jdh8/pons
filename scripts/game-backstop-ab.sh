#!/bin/sh
# game-backstop-ab.sh — A/B for *deleting* the 2/1 game backstop
# (`set_game_backstop`). The backstop is three crude rules (4♥/4♠/3NT) answering
# every game-forcing continuation the three authored rounds miss; it was written
# against the old deterministic floor, and the floor became the BBA-distilled net
# on 2026-07-19. One two-arm experiment: keep (shipped default) vs drop
# (--no-ns-game-backstop, those nodes fall through to the floor), both
# vulnerabilities, both scorers, arms strictly sequential, one shared SEED_BASE
# (paired diffs need identical deals). Modeled on
# scripts/opener-extras-ladder-ab.sh; do NOT touch the codebase while it runs
# (bba-gen-parallel re-invokes cargo build; must stay a no-op).
#
# Self-play pre-check (silenced opponents, 200k×2, seed 1784479600) says the
# deletion WINS both scorers both vuls, firing 1.15%: plain +0.0257/+0.0306,
# PD +0.0315/+0.0370 IMPs/board NV/vul. This leg is the real-routing verdict.
#
#   PER_SHARD=12800 setsid nohup scripts/idle-run.sh \
#       scripts/game-backstop-ab.sh ab-results/game-backstop \
#       >ab-results/game-backstop.log 2>&1 &
#
# Resumable: an existing arm dir or a non-empty diff file is skipped; the
# SEED_BASE persists in $R/seed so a restart stays seed-aligned.
R=${1:?usage: game-backstop-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

# Three arms. KEEP is the pre-2026-07-20 system (node registered, floor unaware
# of the force); DROP deletes the node only; FORCE is the shipped default —
# node deleted *and* the floor told the 2/1 forces game, restoring by rule the
# invariant the node held by omission. Diffed against KEEP so a positive delta
# reads "the change gains"; force-vs-drop prices the forcing rail on its own.
#
# Flag polarity follows the *shipped* defaults, so FORCE passes nothing. The
# recorded run predates the default flip and used the old spellings
# (--no-ns-game-backstop / --ns-two-over-one-force); a re-run wants a fresh dir.
log "=== game-backstop A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm keep  "$vul" --ns-game-backstop --no-ns-two-over-one-force
    arm drop  "$vul" --no-ns-two-over-one-force
    arm force "$vul"
    diffpair drop  keep "$vul"
    diffpair force keep "$vul"
    diffpair force drop "$vul"
done
log "=== game-backstop A/B done"
