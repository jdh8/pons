#!/bin/sh
# N4 screen: the v6 twin trained and served on BBA's disclosed readings versus
# the shipped v6 floor on the same filtered deals.  A floor swap also moves
# defensive lanes, so the package isolation gate does not apply; attribute the
# direct `1NT (2♦)` subset from the paired divergence records before reading the
# whole filtered-arm headline.
set -eu
ROOT_R=${1:?usage: ab-v6-their-reading.sh RESULTS_DIR}
R=$ROOT_R
. "$(dirname "$0")/ab-lib.sh"

for seed_set in 1 2; do
    R="$ROOT_R/seed-$seed_set"
    mkdir -p "$R"
    SEED_BASE=$(seed_for v6-their-reading)
    log "=== v6-their-reading seed=$seed_set SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"
    for vul in none both; do
        arm base "$vul" --their-2d-multi true --filter-1nt
        arm their "$vul" --our-floor american-v6-their --their-2d-multi true \
            --ns-their-multi-advance-read --ns-their-multi-double-read --filter-1nt
        diffpair their base "$vul"
    done
done

log "v6-their-reading done"
