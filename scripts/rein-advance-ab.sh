#!/bin/sh
# rein-advance-ab.sh — measure reining in a minimum takeout doubler that
# over-raises partner's *forced* advance of our double into a doubled game
# (`set_rein_advance_raise`, default on), and re-open the free-1NT floor now
# that the pass-line no longer overbids.  Two questions:
#
#   Q1  Does the rein help?                           rein vs norein
#       We stop reaching doubled 3-4 level contracts a minimum doubler had no
#       business declaring (board 1: 16 combined HCP -> 4♦X).  This is a "stop
#       doing a bad thing" fix — fewer doubled minuses is a real plain-DD gain,
#       not obstruction the wall hides — so plain DD is the arbiter, with PD and
#       sd-lead along for the decision table.
#   Q2  With the overbidding reined, does floor 6 still beat floor 8?
#       rein-floor8 vs rein — the de-confound re-run.  Earlier floor 6 won
#       largely because the pass-line (floor 8) overbid; if that was the whole
#       story, floor 8 should now wash or win.  The floor axis is mixed (a light
#       1NT has lead/obstruction value DD prices at zero), so sd-lead is arbiter.
#
# One SEED_BASE for the experiment (paired diffs need identical deals), arms
# strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/rein-advance-ab.sh \
#       ab-results/rein-advance >ab-results/rein-advance.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/rein-advance.seed.  Do NOT rebuild the binaries
# while this runs.
R=${1:?usage: rein-advance-ab.sh RESULTS_DIR}
SHOW=3
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

exp=rein-advance
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-rein"        "$vul"                              # rein on (shipped candidate)
    arm "$exp-norein"      "$vul" --no-ns-rein-advance-raise   # rein off (old baseline)
    arm "$exp-rein-floor8" "$vul" --ns-free-1nt-floor 8        # rein on, floor 8 (re-sweep)

    diffpair "$exp-rein"        "$exp-norein" "$vul"  # Q1: what the rein buys
    diffpair "$exp-rein-floor8" "$exp-rein"   "$vul"  # Q2: floor 6->8 with the rein in

    sddiff   "$exp-rein"        "$exp-norein" "$vul"                          # Q1 arbiter cross-check
    sddiff   "$exp-rein-floor8" "$exp-rein"   "$vul" --on-ns-free-1nt-floor 8 # Q2 arbiter
done

log "campaign done"
