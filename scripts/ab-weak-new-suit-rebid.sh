#!/bin/sh
# ab-weak-new-suit-rebid.sh — responder's second turn after the weak new
# suit over their double, `1o (X) 2y - 3y -`
# (`competition.weak_new_suit_rebid`, docs/next-steps.md, the rebase-
# rejection entry's flagged `1♠ (X) 2♣` question).
#
#   BOARDS=409600 setsid nohup scripts/idle-run.sh \
#       scripts/ab-weak-new-suit-rebid.sh ab-results/weak-new-suit-rebid \
#       >ab-results/weak-new-suit-rebid.log 2>&1 < /dev/null &
#
# What is being tested.  Strong hands with no fit redouble, so the 2-level
# new suit over their X is weak (6–9, five-plus) and not a 2/1 (jdh8's
# ruling, 2026-10-04).  Off, responder's second turn replays the uncontested
# 2/1 tree through the systems-on rebase: most weak hands are rejected and
# pass by accident, and `1♥ (X) 2♣ - 3♣ -` offers an illegal `3♣`.  On, an
# authored table: `4♥` on 8+ points, `3NT` on 8+ HCP over a minor, else
# pass.
#
#   off  --no-ns-weak-new-suit-rebid
#   on   the default since 2026-10-04
#
# Hypothesis (pre-run): rare — a few hundred per 409,600 — and a small win
# on the maxima that now bid game opposite a 15+ raise.
#
# SHIPPED default-on 2026-10-04 on the naturalness tiebreak (SEED_BASE
# 1791047769, 409,600 boards/arm/vul, `ab-results/weak-new-suit-rebid`):
# 0 divergent boards in either cell — our side reached `1o (X) 2y - 3y -`
# 5–6 times per vul, always on a minimum both arms pass.  Run with the knob
# as an opt-in `--ns-weak-new-suit-rebid` on arm; the flags below are the
# post-flip spelling.
R=${1:?usage: ab-weak-new-suit-rebid.sh RESULTS_DIR}
BUILD_EXTRA='--example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for weak-new-suit-rebid)

log "=== weak-new-suit-rebid A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul" --no-ns-weak-new-suit-rebid
    arm on "$vul"
    gatepair on off "$vul"
    diffpair on off "$vul"
done
log "=== weak-new-suit-rebid A/B done"
