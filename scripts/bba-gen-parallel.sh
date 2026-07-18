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
#
# SEED HYGIENE.  Shard i uses seed (SEED_BASE + i).  SEED_BASE defaults to the
# current UNIX time, so every invocation polls FRESH hands instead of replaying
# the old fixed 0..nproc deals (those got oversampled across experiments — a
# stale, non-representative slice).  For a MULTI-ARM A/B the arms must share deals
# so a paired ab-dump-diff is valid, so set it ONCE and reuse it across the arms:
#
#   export SEED_BASE=$(date +%s)           # one base for the whole experiment
#   for arm in base +feat …; do scripts/bba-gen-parallel.sh out/$arm 6400 $flags; done
#   # NEXT experiment: a NEW base (a fresh `date +%s`) → fresh hands.
#
# The chosen SEED_BASE is echoed; record it (with the git SHA) to reproduce a run.
# See docs/shared-machine-data-gen.md ("Seed hygiene").
set -eu

[ $# -ge 2 ] || { echo "usage: $0 OUTDIR PER_SHARD_COUNT [bba-gen flags...]" >&2; exit 2; }
outdir=$1
count=$2
shift 2

n=${JOBS:-$(nproc)}   # cap worker processes on a shared box; defaults to all cores
seed_base=${SEED_BASE:-$(date +%s)}
bin="$(cd "$(dirname "$0")/.." && pwd)/target/release/examples/bba-gen"

cargo build --release --features "${FEATURES:-serde}" --example bba-gen
mkdir -p "$outdir"
for i in $(seq 0 $((n - 1))); do
    "$bin" --count "$count" --seed "$((seed_base + i))" --output "$outdir/shard-$i.json" "$@" &
done
wait
echo "bba-gen-parallel: SEED_BASE=$seed_base, wrote $n shards of $count boards to $outdir" >&2
