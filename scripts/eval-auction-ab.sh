#!/bin/sh
# eval-auction-ab.sh — evaluator v3 (calls-tail) A/B: `set_eval_auction` on vs
# the shipped default.  The 2026-07-27 NLL ablation priced the last-4-call
# identities at 0.038 NLL over the hull-only vector (bare calls; the full
# tags+alerts block adds only 0.004 more and is not served); the v3 twin
# (`evaluator_v3_dnf`, features_eval_v3, trained on the same --dnf regime the
# default serves) turns that into a bidding arm.  Only the bilans game/slam
# gates consume trick estimates, so this is a decision A/B on those gates, not
# a reading change — divergences are the net re-pricing game/slam boundaries
# with the raw calls in view.
#
# SHIPPED default-on 2026-07-27 (`win | win`, SEED_BASE 1785138816): plain DD
# +0.0180 ± 0.0042 (none) / +0.0284 ± 0.0056 (both), PD +0.0222 / +0.0360,
# fired 1.3-1.6% at +1.3 to +2.3 IMPs/fired.  Arms are expressed off the new
# default:
#   on   the shipped default    (evaluator_v3_dnf, hulls + last 4 calls)
#   off  --no-ns-eval-auction   (evaluator_v2_dnf, hull-only input)
#
#   setsid nohup scripts/idle-run.sh scripts/eval-auction-ab.sh \
#       ab-results/eval-auction >ab-results/eval-auction.log 2>&1 &
#
# Resumable; SEED_BASE persists in $R/seed (a NEW dir -> a new seed).
R=${1:?usage: eval-auction-ab.sh RESULTS_DIR}
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== eval-auction start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul (v3 calls-tail evaluator vs hull-only)"
for vul in none both; do
    arm on "$vul"
    arm off "$vul" --no-ns-eval-auction
    diffpair on off "$vul"
done
log "=== eval-auction done"
