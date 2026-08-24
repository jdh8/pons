#!/bin/sh
# ab-dutch-multi.sh — Dutch Phase 3's Multi slice, both variants, one run.
#
# Three arms, all `--our-floor dutch` against the same BBA reference opponent on
# the same deals, so each comparison is paired and the only moving part is the
# 2-level opening table:
#
#   plain      Dutch with american's three natural weak twos   (control)
#   multi      + `--ns-multi-2d --no-ns-multi-2d-champion` — BBA's book, verbatim
#   champion   + `--ns-multi-2d`             — the polish.club structure (default)
#
# Two verdicts come out of it:
#
#   E1  multi vs plain      — does the Multi slice ship at all?
#   E2  champion vs multi   — and if so, which variant?  The champion default
#                             flips only on a **win**; a wash keeps BBA-verbatim,
#                             whose row alignment with the WJ teacher net and
#                             whose `.bbsa` expressibility are the tiebreakers.
#
# **Preemptive family**, so read it the way docs/measurement.md says to: plain DD
# under-credits obstruction and concealment, pd guards the doubling tail, and
# `sddiff` (16-world single-dummy leads) is what prices the information the
# Multi hides — the whole point of the call is that the opponents do not know
# which major it is.  A plain-DD loss with a pd/sd win is the *expected* shape
# here, not a red flag; trace the worst divergent boards before calling any
# reading final.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-dutch-multi.sh ab-results/dutch-multi \
#       >ab-results/dutch-multi.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir → a new seed).  A
# confirmatory re-measure of either verdict wants its own results dir so it gets
# its own seed.
R=${1:?usage: ab-dutch-multi.sh RESULTS_DIR}
# `sddiff` needs the single-dummy harness, and ab-lib.sh builds every binary
# once up front — nothing may rebuild while the run is in flight.
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== dutch multi start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm plain    "$vul" --our-floor dutch
    arm multi    "$vul" --our-floor dutch --ns-multi-2d --no-ns-multi-2d-champion
    arm champion "$vul" --our-floor dutch --ns-multi-2d

    for pair in multi:plain champion:multi champion:plain; do
        on=${pair%%:*}
        off=${pair#*:}
        diffpair "$on" "$off" "$vul"
        sddiff "$on" "$off" "$vul"
    done
done
log "=== dutch multi done"
