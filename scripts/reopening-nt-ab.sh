#!/bin/sh
# reopening-nt-ab.sh — measure opener's authored balanced-18-19 notrump actions
# in a `1X (1Y) …` auction the instinct floor otherwise passed out (reopening
# 1NT, 3NT over responder's free 1NT, responder's raise), and re-sweep the
# free-1NT floor now that both continuations are sound.  Two questions:
#
#   Q1  Does the reopening-notrump package help?   base vs noreopen
#       A constructive capability-add (reaching a game/partscore the lone
#       takeout double missed) — trust plain DD; PD along for the decision table.
#   Q2  With the continuations authored, does floor 6 still beat floor 8?
#       floor8 vs base — the de-confound.  The floor axis is mixed (a light 1NT
#       has obstruction/lead value DD prices at zero), so sd-lead is the arbiter.
#
# One SEED_BASE for the experiment (paired diffs need identical deals), arms
# strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/reopening-nt-ab.sh \
#       ab-results/reopening-nt >ab-results/reopening-nt.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/reopening-nt.seed.  Do NOT rebuild the binaries
# while this runs.
R=${1:?usage: reopening-nt-ab.sh RESULTS_DIR}
SHOW=3
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

exp=reopening-nt
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-base"     "$vul"                              # floor 6, reopening on (shipped)
    arm "$exp-noreopen" "$vul" --no-ns-reopening-notrump    # reopening OFF (baseline)
    arm "$exp-floor8"   "$vul" --ns-free-1nt-floor 8        # floor 8, reopening on (re-sweep)

    diffpair "$exp-base"   "$exp-noreopen" "$vul"  # Q1: what the reopening package buys
    diffpair "$exp-floor8" "$exp-base"     "$vul"  # Q2: floor 6->8 with sound continuations

    sddiff   "$exp-floor8" "$exp-base"     "$vul" --on-ns-free-1nt-floor 8  # Q2 arbiter
done

log "campaign done"
