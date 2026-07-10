#!/bin/sh
# free-bid-quality-ab.sh — P3b″: the suit-quality gate the P3b′ floor sweep
# asked for (set_free_bid_quality / --ns-free-bid-quality). The free-bid
# family's vulnerable leak is plain-DD-visible and strength-independent; the
# gate demands two of the top three honors for a vulnerable 1-level free bid
# and drops the vulnerable free 1NT. Measured on Modern (the family
# representative and the fallback@1/@2 waterfall's biggest cell — BBA doubles
# 70% of it): arms off / modern / modern+quality.
#
# One SEED_BASE for the experiment (paired diffs need identical deals), arms
# strictly sequential, both scorers, both vulnerabilities.
#
#   setsid nohup scripts/idle-run.sh scripts/free-bid-quality-ab.sh \
#       ab-results/free-bid-quality >ab-results/free-bid-quality.log 2>&1 &
#
# Resumable: an arm dir / diff file that already exists is skipped; the
# SEED_BASE persists in $R/free-bid-quality.seed. Do NOT rebuild the binaries
# while this runs.
R=${1:?usage: free-bid-quality-ab.sh RESULTS_DIR}
SHOW=3
. "$(dirname "$0")/ab-lib.sh"

exp=free-bid-quality
SEED_BASE=$(seed_for "$exp")
log "=== $exp SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD boards/arm/vul"

for vul in none both; do
    arm "$exp-off" "$vul"
    arm "$exp-modern" "$vul" --ns-negative-double-shape modern
    arm "$exp-modernq" "$vul" --ns-negative-double-shape modern --ns-free-bid-quality
    diffpair "$exp-modern" "$exp-off" "$vul"     # replicate P3d (anchor)
    diffpair "$exp-modernq" "$exp-off" "$vul"    # ship gate
    diffpair "$exp-modernq" "$exp-modern" "$vul" # what the gate buys
done

log "campaign done"
