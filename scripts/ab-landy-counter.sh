#!/bin/sh
# ab-landy-counter.sh — N1 / N1b / N1c of the competitive-1NT campaign
# (docs/one-notrump-competitive.md): our counter-defense when their 2♣ overcall
# of our 1NT is Landy (both majors) instead of natural clubs.
#
# The census picked this package: the (2♣) bucket is the anchor's largest 1NT
# interference loss and the only one negative on BOTH scorers
# (plain −0.74/bd pooled, PD −0.70 NV).  Mechanism traced there — systems-on
# keeps Stayman (useless against a hand holding both majors) and turns 2♦/2♥
# into Jacoby transfers *into* their suits.
#
#   JOBS=24 setsid nohup scripts/idle-run.sh scripts/ab-landy-counter.sh \
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
    # 2/1 reference, so an off arm must force the natural read explicitly.
    #
    # Round 5 (N1d/e/f, 2026-08-14): the cue repairs, stacked in evidence
    # order on top of N1c.  Earlier rounds' arms (landy-cues, landy-off) and
    # their diffs live in ab-results/landy-counter*/ and ab-results/landy-n1c*/;
    # this round regenerates its control at HEAD (a shared seed pairs DEALS,
    # not code).
    # Since the 2026-08-14 stack ship the engine default IS landy-f, so the
    # other arms are spelled by switching pieces OFF (the flags are
    # Option<bool>: `--defense-2c-landy-<knob> false`).
    arm landy-xfer "$v" --their-2c-landy true --defense-2c-landy-cue-floor false \
        --defense-2c-landy-fit-answers false --defense-2c-landy-competition false --filter-1nt
    arm landy-d "$v" --their-2c-landy true \
        --defense-2c-landy-fit-answers false --defense-2c-landy-competition false --filter-1nt
    arm landy-e "$v" --their-2c-landy true --defense-2c-landy-competition false --filter-1nt
    arm landy-f "$v" --their-2c-landy true --filter-1nt
    arm landy-on "$v" --their-2c-landy true --defense-2c-landy-transfer false \
        --defense-2c-landy-cue-floor false --defense-2c-landy-fit-answers false \
        --defense-2c-landy-competition false --filter-1nt
    # Three increments and the ship comparison: d↔xfer is the cue floor alone
    # (N1d), e↔d the doubleton-notrump answers on top (N1e), f↔e the
    # interfered tails on top (N1f) — expected CI-wide on its own, the hole
    # fired on 47/460k boards — and f↔on is what decides shipping: the full
    # stack against the shipped counter (docs/one-notrump-competitive.md).
    diffpair landy-d landy-xfer "$v"
    diffpair landy-e landy-d "$v"
    diffpair landy-f landy-e "$v"
    diffpair landy-f landy-on "$v"
    # The counter is a constructive/defensive contract choice, not obstruction,
    # so plain+PD decide.  sd is read only as a tie-breaker if they disagree.
    sddiff landy-d landy-xfer "$v"
    sddiff landy-e landy-d "$v"
    sddiff landy-f landy-e "$v"
    sddiff landy-f landy-on "$v"
done

log "landy-counter done"
