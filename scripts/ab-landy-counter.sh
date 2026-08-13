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
    # `--their-2c-landy` is a declaration override: bba-gen now DERIVES the
    # Landy read from the opponents' declaration and defaults it ON vs the
    # 2/1 reference, so the off arm must force the natural read explicitly.
    arm landy-cues "$v" --their-2c-landy true  --defense-2c-landy-cues --filter-1nt
    arm landy-on   "$v" --their-2c-landy true                          --filter-1nt
    arm landy-off  "$v" --their-2c-landy false                         --filter-1nt
    # Three arms, two paired diffs: on↔off prices the base counter (N1), and
    # cues↔on prices the GF-minor-cue overlay (N1b) alone — the falsifiable
    # delta of the 1♣ (2♣) analogy (docs/one-notrump-competitive.md).
    diffpair landy-on   landy-off "$v"      # ship gate: plain + PD in one solve
    diffpair landy-cues landy-on  "$v"
    # The counter is a constructive/defensive contract choice, not obstruction,
    # so plain+PD decide.  sd is read only as a tie-breaker if they disagree.
    sddiff landy-on   landy-off "$v"
    sddiff landy-cues landy-on  "$v"
done

log "landy-counter done"
