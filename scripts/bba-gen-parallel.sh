#!/bin/sh
# bba-gen-parallel: fill a bba-gen board dump using every core.
#
# EPBot is single-threaded and thread-unsafe, so we parallelize across PROCESSES,
# not threads: one shard per worker with a distinct --seed (bba-gen is
# deterministic in its seed, so distinct seeds give disjoint, mergeable shards).
# `bba-score` reads the shards back into one match — their boards concatenate.
#
#   scripts/bba-gen-parallel.sh OUTDIR PER_SHARD_COUNT [extra bba-gen flags...]
#   cargo run --release --features serde --example bba-score -- OUTDIR/shard-*.json --score pd
#
# Total boards = PER_SHARD_COUNT * nproc; use a multiple of 4 to keep dealer
# balance. Each worker is one thread, so nproc workers fill the box exactly (not
# oversubscribed). On a shared machine wrap the WHOLE script in scripts/idle-run.sh.
set -eu

[ $# -ge 2 ] || { echo "usage: $0 OUTDIR PER_SHARD_COUNT [bba-gen flags...]" >&2; exit 2; }
outdir=$1
count=$2
shift 2

n=$(nproc)
bin="$(cd "$(dirname "$0")/.." && pwd)/target/release/examples/bba-gen"

cargo build --release --features serde --example bba-gen
mkdir -p "$outdir"
for i in $(seq 0 $((n - 1))); do
    "$bin" --count "$count" --seed "$i" --output "$outdir/shard-$i.json" "$@" &
done
wait
echo "bba-gen-parallel: wrote $n shards of $count boards to $outdir" >&2
