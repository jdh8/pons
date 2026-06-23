#!/bin/sh
# gib-scavenge: grow a GIB double-dummy database from idle CPU.
#
# Loops the one-shot `gib generate`, one disjoint random-seeded shard per pass,
# and PAUSES while the target filesystem is low on space — so a forgotten
# scavenger can't fill a shared disk. Shards are named by seed (reproducible)
# and merge with `cat`. Driven by scripts/gib-scavenge.service (SCHED_IDLE).
# Keep it a SINGLE instance: one shard already saturates every core.
#
# Knobs (env): GIB_OUT (dir), GIB_MIN_FREE_KIB (pause threshold), GIB_COUNT.
set -eu

OUT="${GIB_OUT:-$HOME/gib-shards}"
MIN_KIB="${GIB_MIN_FREE_KIB:-20971520}"          # pause below ~20 GiB free
COUNT="${GIB_COUNT:-100000}"
BIN="$(cd "$(dirname "$0")/.." && pwd)/target/release/examples/gib"

mkdir -p "$OUT"
while true; do
    # df failure -> empty -> 0 -> treated as low (fail-safe: don't write).
    avail=$(df --output=avail -k "$OUT" | tail -n1 | tr -dc 0-9)
    if [ "${avail:-0}" -lt "$MIN_KIB" ]; then
        echo "gib-scavenge: $(( ${avail:-0} / 1048576 )) GiB free below threshold, pausing 10m"
        sleep 600
        continue
    fi
    seed=$(od -An -tu8 -N8 /dev/urandom | tr -d ' ')
    "$BIN" generate --count "$COUNT" --seed "$seed" --out "$OUT/shard-$seed.txt"
done
