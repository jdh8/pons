#!/bin/sh
# cachalot-xfix-gen.sh — Cachalot arms for the contested-X isolation A/B.
# Both at sha HEAD+fix, seed 1783679026: ON = authored contested-X answer
# (default), OFF = --no-ns-cachalot-contested-x (the old floored continuation).
# ON-vs-OFF isolates exactly the fix. Bidding only; diffs run separately.
set -eu
cd "$(dirname "$0")/.."
R=ab-results/cachalot-xfix
SEED=1783679026
PER=6400
mkdir -p "$R"
for vul in both none; do
    on="$R/cachalot-xfix-$vul"
    off="$R/cachalot-off-$vul"
    if [ ! -d "$on" ]; then
        echo "generate $on (ON, SEED_BASE=$SEED, vul=$vul)"
        SEED_BASE=$SEED scripts/bba-gen-parallel.sh "$on" "$PER" \
            -v "$vul" --ns-negative-double-shape cachalot
    else
        echo "skip $on (exists)"
    fi
    if [ ! -d "$off" ]; then
        echo "generate $off (OFF, SEED_BASE=$SEED, vul=$vul)"
        SEED_BASE=$SEED scripts/bba-gen-parallel.sh "$off" "$PER" \
            -v "$vul" --ns-negative-double-shape cachalot --no-ns-cachalot-contested-x
    else
        echo "skip $off (exists)"
    fi
done
echo "cachalot-xfix gen done"
