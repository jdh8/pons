#!/bin/sh
# ab-landy-counter.sh — N1 of the competitive-1NT campaign
# (docs/one-notrump-competitive.md): our counter-defense when their 2♣ overcall
# of our 1NT is Landy (both majors) instead of natural clubs.
#
# The census picked this package: the (2♣) bucket is the anchor's largest 1NT
# interference loss and the only one negative on BOTH scorers
# (plain −0.74/bd pooled, PD −0.70 NV).  Mechanism traced there — systems-on
# keeps Stayman (useless against a hand holding both majors) and turns 2♦/2♥
# into Jacoby transfers *into* their suits.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/ab-landy-counter.sh \
#       ab-results/landy-counter >ab-results/landy-counter.log 2>&1 < /dev/null &
#
# BBA's 2/1 card overcalls 1NT with Multi-Landy, whose 2♣ *is* Landy, so the
# reference opponent bids the trigger unprompted — no --their-conv needed.
#
# `--filter-1nt` is the enriched-probing gate (balanced 15-17 somewhere, a raw
# hand test applied BEFORE any bidding) and rides BOTH arms, so the arms deal the
# same board set and stay seed-aligned for the paired diff.  Headline is then
# IMPs per *accepted* deal; multiply by the trigger density for a per-board
# figure and scale the CI the same way (docs/measurement.md).
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE persists
# in $R/landy.seed.  Iron rule: do NOT rebuild binaries while this runs.
R=${1:?usage: ab-landy-counter.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy)
log "=== landy-counter SEED_BASE=$SEED_BASE sha=$SHA"

for v in none both; do
    arm landy-on  "$v" --defense-2c-landy --filter-1nt
    arm landy-off "$v"                    --filter-1nt
    diffpair landy-on landy-off "$v"        # ship gate: plain + PD in one solve
    # The counter is a constructive/defensive contract choice, not obstruction,
    # so plain+PD decide.  sd is read only as a tie-breaker if they disagree.
    sddiff landy-on landy-off "$v"
done

log "landy-counter done"
