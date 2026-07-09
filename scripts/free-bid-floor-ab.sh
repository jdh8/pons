#!/bin/sh
# free-bid-floor-ab.sh — sweep the 1-level free-bid floor (set_free_bid_floor /
# --ns-free-bid-floor) to trim the vulnerable-PD leak the whole free-bid family
# (free bids, Modern, Cachalot, Sputnik) inherits. Prior campaign: free6 and
# Modern6 both lose vs off on the vul cells to the 6-count 1-level/1NT floor.
#
# Sweep {6,7,8} on the plain free-bid arm to find the floor, then confirm the
# winner on Modern (the family representative — Cachalot ≈ Modern ≈ Sputnik).
# One SEED_BASE for the experiment (paired diffs need identical deals), arms
# strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/free-bid-floor-ab.sh \
#       ab-results/free-bid-floor >ab-results/free-bid-floor.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/free-bid-floor.seed. Do NOT rebuild the binaries
# while this runs.
R=${1:?usage: free-bid-floor-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"

exp=free-bid-floor
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-free6" "$vul" --ns-free-bids --ns-free-bid-floor 6
    arm "$exp-free7" "$vul" --ns-free-bids --ns-free-bid-floor 7
    arm "$exp-free8" "$vul" --ns-free-bids --ns-free-bid-floor 8
    arm "$exp-modern6" "$vul" --ns-negative-double-shape modern --ns-free-bid-floor 6
    arm "$exp-modern8" "$vul" --ns-negative-double-shape modern --ns-free-bid-floor 8
    diffpair "$exp-free6" "$exp-off" "$vul"       # reproduce the leak (anchor)
    diffpair "$exp-free7" "$exp-off" "$vul"       # floor 7 vs baseline
    diffpair "$exp-free8" "$exp-off" "$vul"       # floor 8 vs baseline (ship gate)
    diffpair "$exp-free8" "$exp-free6" "$vul"     # mechanism: what the bump buys
    diffpair "$exp-modern8" "$exp-off" "$vul"     # family ship gate
    diffpair "$exp-modern8" "$exp-modern6" "$vul" # mechanism on Modern
done

log "campaign done"
