#!/bin/sh
# ab-landy-rungs.sh — N1h and N1i of the competitive-1NT campaign
# (docs/one-notrump-competitive.md §N1h/N1i): the Landy counter's minor rungs
# re-priced, two ways, against one shared baseline.
#
#   JOBS=12 setsid nohup scripts/idle-run.sh scripts/ab-landy-rungs.sh \
#       ab-results/landy-low >ab-results/landy-low.log 2>&1 < /dev/null &
#
# Arms, all with `--their-2c-landy true --filter-1nt` (a true declaration of
# the reference opponent — BBA's 2/1 card overcalls 1NT with Multi-Landy —
# plus the enriched-probing raw-hand gate, applied to every arm so they stay
# seed-aligned):
#
#   low-off  the shipped stack: cue points(10..), 3m points(8..=9),
#            2♦ points(..=9), 2NT points(2..=9)
#   low-on   N1h: cue points(9..), 3m points(7..=8)
#   hcp-on   N1i: cue hcp(9..), 3m hcp(7..=8), 2♦ and 2NT hcp(..=6)
#
# **Both measured 2026-08-15 and REFUTED; the lane is closed.**  N1h landed on
# `plain wash | PD loss` (vul PD −0.00081 ±0.00074), N1i on no CI-clear cell
# with all eight leaning negative, pooled over three seeds at 230.4k bd/vul.
# The durable finding is that `cue ← X` measured negative in both arms against
# N1d's +2.0…+5.1 for the same migration the other way: the cue floor is
# settled.  This script is kept so the arms can be regenerated, not because
# either knob is a candidate.
#
# The isolation gate leaks here by construction (10-13% foreign) — both arms
# edit a cue *constraint*, which is what reopened the mirror leak for N1d/N1f.
# Read the `--gate-opener ours` split rather than a pass/fail.
#
# Resumable: an existing arm dir or diff file is skipped, and SEED_BASE
# persists in $R/landy-low.seed.  Iron rule: do NOT rebuild binaries while
# this runs.
R=${1:?usage: ab-landy-rungs.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for landy-low)
log "=== landy-rungs SEED_BASE=$SEED_BASE sha=$SHA"

for v in none both; do
    arm low-off "$v" --their-2c-landy true --filter-1nt
    arm low-on "$v" --their-2c-landy true --defense-2c-landy-low-minors --filter-1nt
    arm hcp-on "$v" --their-2c-landy true --defense-2c-landy-hcp-rungs --filter-1nt
    for on in low-on hcp-on; do
        diffpair "$on" low-off "$v"
        sddiff "$on" low-off "$v"
    done
done

log "landy-rungs done"
