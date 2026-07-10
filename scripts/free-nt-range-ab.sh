#!/bin/sh
# free-nt-range-ab.sh — optimize responder's `1X (1Y) 1NT` range, decoupled
# from the shared free-bid suit floor. Two axes:
#   1. capability add — the natural invitational 2NT *jump* over a 1-level
#      overcall (11-12 + stopper; --ns-free-2nt-jump), the invite the ordinary
#      2NT rule (a 2-level overcall's min-level) leaves stranded.
#   2. the isolated 1NT floor (--ns-free-1nt-floor {6,7,8}), re-tested decoupled
#      from the suit floor whose 6->8 raise the campaign already refuted.
#
# Classify first (docs/convention-tuning.md): the 2NT jump is a constructive
# capability-add — trust plain DD. The floor axis is mixed (a light 1NT has
# obstruction/lead value DD prices at zero) — sd-lead is the arbiter, so we
# disclose the ON knobs to the blind leader. One SEED_BASE for the experiment
# (paired diffs need identical deals), arms strictly sequential, both scorers,
# both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/free-nt-range-ab.sh \
#       ab-results/free-nt-range >ab-results/free-nt-range.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/free-nt-range.seed. Do NOT rebuild the binaries
# while this runs.
#
# HISTORICAL RECORD (run at sha 4b7c984): the 2NT jump shipped default-on and
# unconditional, so `--ns-free-2nt-jump` was retired — this script no longer
# reproduces against HEAD (there is no flag to turn the jump off). Kept for the
# SEED_BASE and the arm layout; see memory project_free-1nt-range.
R=${1:?usage: free-nt-range-ab.sh RESULTS_DIR}
SHOW=3
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

exp=free-nt-range
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"                                          # 6:10, no jump (baseline)
    arm "$exp-j"   "$vul" --ns-free-2nt-jump                       # capability add: +2NT 11-12
    arm "$exp-j7"  "$vul" --ns-free-2nt-jump --ns-free-1nt-floor 7 # isolated floor 7
    arm "$exp-j8"  "$vul" --ns-free-2nt-jump --ns-free-1nt-floor 8 # isolated floor 8

    diffpair "$exp-j"  "$exp-off" "$vul"  # what the 2NT jump alone buys
    diffpair "$exp-j7" "$exp-j"   "$vul"  # floor 6->7 on top of the jump
    diffpair "$exp-j8" "$exp-j"   "$vul"  # floor 6->8 on top of the jump
    diffpair "$exp-j8" "$exp-off" "$vul"  # net package vs baseline (ship gate)

    sddiff "$exp-j"  "$exp-off" "$vul" --on-ns-free-2nt-jump
    sddiff "$exp-j8" "$exp-off" "$vul" --on-ns-free-2nt-jump --on-ns-free-1nt-floor 8
done

log "campaign done"
