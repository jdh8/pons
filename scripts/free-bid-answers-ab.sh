#!/bin/sh
# free-bid-answers-ab.sh — Fix-1 v2: complete the free-bid convention. The
# free-bid-quality A/B's worst vulnerable-PD boards were opener PASSING a
# game-going free bid out (unauthored continuation, the Gladiator/Rubens
# precedent). `answer_free_bid` now forces opener's answer at both levels
# (raise / cheapest NT with stopper / natural second suit / rebid catch-all,
# no Pass). Re-measure the family representative: modern (implies free bids +
# answers) vs off. The refuted quality gate stays opt-in and out of the arms.
#
#   setsid nohup scripts/idle-run.sh scripts/free-bid-answers-ab.sh \
#       ab-results/free-bid-answers >ab-results/free-bid-answers.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/free-bid-answers.seed. Do NOT rebuild
# the binaries while this runs.
R=${1:?usage: free-bid-answers-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"

exp=free-bid-answers
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-modern" "$vul" --ns-negative-double-shape modern
    diffpair "$exp-modern" "$exp-off" "$vul" # ship gate: modern-complete vs baseline
done

log "campaign done"
