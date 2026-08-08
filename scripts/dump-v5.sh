#!/bin/sh
# dump-v5.sh — the compact-config (features v5) mixture corpus.
#
# 22.pdd rows 3,250,000..4,200,000, registered in docs/pdd-bank-ledger.md
# (2026-08-08) before this run.  Three slices, twenty shards:
#
#   uniform-0..7   8 x 31,250 deals, rows 3.25M..3.5M — the six DEFAULT_CELLS
#                  rotating, the v4-shaped bulk (gate-1 comparability)
#   enriched-0..3  4 x 125,000 drawn, rows 3.5M..4.0M — `--enrich 28:9
#                  --replay`, both all-American kickback cells (the slot that
#                  taught v4's slot 77; unchanged so the pair-flip bar carries)
#   axis-XXXX      8 x 20,000 deals, rows 4.0M..4.16M — one top-8 knob axis
#                  each (probe ranking, card-manifold.md §Axis selection),
#                  `--replay` over [base/base, flip/base]: the mixed table
#                  emits both asymmetric views, so ours- and theirs-side dims
#                  both train from two auctions per deal
#
# Shards are independent processes (EPBot is single-threaded); all twenty run
# concurrently under the caller's scheduling.  Build ONCE before launching —
# never rebuild while shards are in flight.
#
#   cargo build --release --features serde --example dump-teacher
#   setsid nohup scripts/idle-run.sh scripts/dump-v5.sh \
#       >target/corpus-v5.log 2>&1 &
set -eu
cd "$(dirname "$0")/.."

BANK=/nfs2/jdh8/pons/22.pdd
OUT=target/corpus-v5
BIN=target/release/examples/dump-teacher
COMMON="--deals $BANK --teacher bba --configured --feature-version 5"
mkdir -p "$OUT"

# shellcheck disable=SC2086
shard() { # shard NAME SKIP BOARDS SEED [extra flags...]
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
# Top-8 axes by probe frequency; hex = dump-teacher's axis-bit table.
i=0
for bit in 0004 1000 2000 0002 4000 0800 8000 0020; do
    shard "axis-$bit" $((4000000 + i * 20000)) 20000 $((300 + i)) \
        --replay --cell "a-off/a-off" --cell "a-off+$bit/a-off"
    i=$((i + 1))
done
wait
echo "dump-v5: all shards done"
