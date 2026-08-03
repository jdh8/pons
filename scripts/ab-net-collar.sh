#!/bin/sh
# ab-net-collar.sh — collar the bilans net instead of letting it replace the
# authored point arithmetic at the game and slam milestones.
#
# The shipped `set_bilans_floor` wiring masks the authored gate off and hands
# the net the whole criterion — an unbounded reach below the point threshold
# *and* an unbounded veto over hands the point sums accept.  `set_net_collar`
# gives the arithmetic the criterion back and lets the net rule on it in one
# direction only, chosen by the decision's own IMP economics (`break_even`):
# accelerate at game (reach at most 2 points below the threshold), veto at slam.
# Games never break even above even money, slams never below — see
# `break_even_keys_the_collar_direction`.
#
# Two arms per vul, identical deals:
#   off      the shipped default (net replaces the arithmetic)
#   collar   --ns-net-collar
#
# NOT an arm: `--no-ns-bilans`.  This is a treatment of the net's *licence*, not
# of its presence, so both arms keep the net on.
#
# Read the verdict off docs/measurement.md's decision table, then read the
# per-site attribution: the candidate touches nine milestones with two different
# shapes, and 5m is under explicit doubt (it shares 4M's `combined_points(25)`
# while needing eleven tricks), so a pooled number can hide four winning game
# sites carrying a losing 5m.  `--show 40` is deliberately wide for that — group
# the worst boards by the bid that first diverged before concluding anything.
#
#   setsid nohup scripts/ab-net-collar.sh ab-results/net-collar \
#       >ab-results/net-collar.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: ab-net-collar.sh RESULTS_DIR}
SHOW=${SHOW:-40}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== net collar start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm collar "$vul" --ns-net-collar
    diffpair collar off "$vul"
done
log "=== net collar done"
