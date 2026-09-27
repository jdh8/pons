#!/bin/sh
# ab-stayman-defense.sh — the stale-pop re-measure of `set_stayman_defense`
# (docs/bidding-options.md A5): our opt-in defense to `(1NT) - (2♣)` vs BBA,
# on vs off, both vuls, plain + PD + sd-lead.  The ON arm's sd read discloses
# the defense, so the blind leader reads `X` as lead-directing clubs.
#
#   JOBS=24 BOARDS=409600 setsid nohup scripts/idle-run.sh scripts/ab-stayman-defense.sh \
#       ab-results/stayman-defense >ab-results/stayman-defense.log 2>&1 < /dev/null &
#
# Resumable (ab-lib.sh); SEED_BASE persists in $R/seed.  Do NOT rebuild while it runs.
R=${1:?usage: ab-stayman-defense.sh RESULTS_DIR}
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"

SEED_BASE=$(seed_for)
common="--isolate-defense --filter-1nt"
log "=== stayman-defense SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS per-shard=$PER_SHARD"
for v in none both; do
    # shellcheck disable=SC2086  # deliberate word-split of the flag list
    arm on  "$v" --ns-defense-to-their-stayman $common
    # shellcheck disable=SC2086
    arm off "$v" $common
    diffpair on off "$v"
    sddiff on off "$v" --on-ns-stayman-defense
done
log "stayman-defense A/B done"
