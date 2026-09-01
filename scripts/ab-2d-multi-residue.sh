#!/bin/sh
# ab-2d-multi-residue.sh — N4 residue: disclosed-Multi reader and 3♠ ask.
#
#   JOBS=24 BOARDS=460800 setsid nohup scripts/idle-run.sh \
#       scripts/ab-2d-multi-residue.sh ab-results/2d-multi-residue \
#       >ab-results/2d-multi-residue.log 2>&1 < /dev/null &
#
# Three independent seed sets, four aligned arms, both vulnerabilities:
#   base    shipped N4 v7
#   reader  exact 6+♥ | 6+♠ opponent-reading only
#   search  3♠ stopper ask, responder fit-search continuation
#   place   3♠ stopper ask, opener places immediately
#
# Every pair is gated through `probe-divergence --gate-opener ours` before it
# is scored.  The direct search-vs-place pair decides between the two stopper
# continuations if both pass the repository decision table.  Plain and PD are
# co-arbiters; sd is the tie-breaker.  All arms explicitly pin the other N4
# residue switch off, so each effect is independently measurable.
#
# Resumable: each seed set owns its seed, arms, probes, and diffs.  Iron rule:
# do NOT rebuild binaries while this runs.
ROOT_R=${1:?usage: ab-2d-multi-residue.sh RESULTS_DIR}
R=$ROOT_R
BUILD_EXTRA='--example ab-dump-sd --example probe-divergence'
. "$(dirname "$0")/ab-lib.sh"
PROBE=target/release/examples/probe-divergence

for seed_set in 1 2 3; do
    R="$ROOT_R/seed-$seed_set"
    mkdir -p "$R"
    SEED_BASE=$(seed_for multi-residue)
    log "=== 2d-multi-residue seed=$seed_set SEED_BASE=$SEED_BASE sha=$SHA shards=$SHARDS x $PER_SHARD bd/arm/vul"

    for v in none both; do
        arm base "$v" --their-2d-multi true --ns-their-multi-read false \
            --ns-multi-stopper-ask off --filter-1nt
        arm reader "$v" --their-2d-multi true --ns-their-multi-read true \
            --ns-multi-stopper-ask off --filter-1nt
        arm search "$v" --their-2d-multi true --ns-their-multi-read false \
            --ns-multi-stopper-ask search --filter-1nt
        arm place "$v" --their-2d-multi true --ns-their-multi-read false \
            --ns-multi-stopper-ask place --filter-1nt

        for pair in reader:base search:base place:base search:place; do
            on=${pair%%:*}
            off=${pair#*:}
            gatepair "$on" "$off" "$v"
            diffpair "$on" "$off" "$v"
            sddiff "$on" "$off" "$v"
        done
    done
done

log "2d-multi-residue done"
