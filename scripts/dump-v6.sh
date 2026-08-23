#!/bin/sh
# Phase 5 policy corpus: the v5 compact-config mixture regenerated through
# features_v6's live authored reading.  Training overlap is deliberate: the
# extractor is the treatment, so identical deals/cells make the held-out gate
# comparable.  Registered in docs/pdd-bank-ledger.md before the run.
set -eu
cd "$(dirname "$0")/.."

BANK=/nfs2/jdh8/pons/22.pdd
OUT=${DUMP_OUT:-target/corpus-v6}
BIN=target/release/examples/dump-teacher
COMMON="--deals $BANK --teacher bba --configured --feature-version 6"
[ "${DUMP_VS_BBA:-false}" = true ] && COMMON="$COMMON --vs-bba"
mkdir -p "$OUT"

# shellcheck disable=SC2086
shard() {
    name=$1 skip=$2 boards=$3 seed=$4
    shift 4
    [ -s "$OUT/$name.json" ] && { echo "skip $name (exists)"; return 0; }
    $BIN $COMMON --skip "$skip" --boards "$boards" --seed "$seed" \
        --out "$OUT/$name" "$@" >"$OUT/$name.log" 2>&1 &
}

for i in 0 1 2 3 4 5 6 7; do
    shard "uniform-$i" $((3250000 + i * 31250)) 31250 $((100 + i))
done
for i in 0 1 2 3; do
    shard "enriched-$i" $((3500000 + i * 125000)) 125000 $((200 + i)) \
        --enrich 28:9 --replay --cell a-on/a-on --cell a-off/a-off
done
i=0
for bit in 0004 1000 2000 0002 4000 0800 8000 0020; do
    shard "axis-$bit" $((4000000 + i * 20000)) 20000 $((300 + i)) \
        --replay --cell "a-off/a-off" --cell "a-off+$bit/a-off"
    i=$((i + 1))
done
wait
echo "dump-v6: all shards done"
