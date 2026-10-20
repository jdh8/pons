#!/bin/sh
# ab-xx-dont-pdp.sh — the PD-P column for a finished ab-xx-dont-major-first.sh
# run: every pair that runner diffs, rescored with prefix-honoring perfect
# defense (ab-dump-diff --score pdp).  Run it after the A/B is done, with the
# A/B's own JOBS and BOARDS (ab-lib checks them against $R):
#
#   JOBS=96 BOARDS=786432 DDSS_THREADS=96 \
#       scripts/ab-xx-dont-pdp.sh ab-results/xx-dont-major-first-2x
#
# PRE-REGISTERED 2026-10-21 (jdh8-approved, before any number of the 2× run
# existed): in this lane PD-P replaces PD in the decision table — plain DD
# and PD-P, each scorer's verdict its worst cell, δ = 0.002 / density.  PD
# stays in the report as a description.  Why: the lane's premise is a
# business XX of our penalty X, a call both arms saw; `ns_score_bid` erases it
# (a making 1NTxx scores as undoubled 1NT), so it prices a board in which the
# lane never happened.  PD-P keeps the double-dummy axe on everything after
# the split (a failing runout is scored at least doubled), so it stays harsh
# on running.  docs/defensive-overcalls.md § DONT runout, Loose ends 1–2.
R=${1:?usage: ab-xx-dont-pdp.sh RESULTS_DIR}
VULS=${VULS:-none ns ew both}
. "$(dirname "$0")/ab-lib.sh"

for v in $VULS; do
    DENSITY=$(density_of "$R/off-$v")
    for c in longer tied doubler dont; do
        pdpdiff "$c" off "$v"
    done
    for pair in "longer dont" "tied dont" "tied longer" "doubler longer" "doubler tied"; do
        # shellcheck disable=SC2086
        set -- $pair
        pdpdiff "$1" "$2" "$v"
    done
done
log "xx-dont-pdp done"
