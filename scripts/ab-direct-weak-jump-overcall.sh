#!/bin/sh
# ab-direct-weak-jump-overcall.sh — paired BBA validation of the default-off
# direct weak jump overcall. OFF is the shipped simple 1M overcall; ON bids 2M
# with exactly six cards, 8+ points, and at most 11 HCP. Both vulnerabilities,
# plain DD + perfect defense from one solve, plus plain SD and SD-PD. Arms are
# sequential and share one persistent fresh seed.
#
#   PER_SHARD=6400 setsid nohup scripts/idle-run.sh \
#       scripts/ab-direct-weak-jump-overcall.sh \
#       ab-results/direct-weak-jump-overcall/<date>-<sha> \
#       >ab-results/direct-weak-jump-overcall.log 2>&1 &
R=${1:?usage: ab-direct-weak-jump-overcall.sh RESULTS_DIR}
SHOW=50
BUILD_EXTRA='--example ab-dump-sd'
. "$(dirname "$0")/ab-lib.sh"
SEED_BASE=$(seed_for)

log "=== direct-weak-jump-overcall A/B start, sha=$SHA, SEED_BASE=$SEED_BASE, ${SHARDS}x${PER_SHARD} bd/arm/vul"
for vul in none both; do
    arm off "$vul"
    arm on  "$vul" --ns-direct-weak-jump-overcall
    diffpair on off "$vul"
    sddiff on off "$vul" --on-ns-direct-weak-jump-overcall
done
log "=== direct-weak-jump-overcall A/B done"
