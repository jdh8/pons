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
# current UNIX time in nanoseconds, so invocations within one second can poll
# fresh hands instead of replaying the old fixed 0..nproc deals (those got
# oversampled across experiments — a
# stale, non-representative slice).  For a MULTI-ARM A/B the arms must share deals
# so a paired ab-dump-diff is valid, so set it ONCE and reuse it across the arms:
#
#   export SEED_BASE=$(date +%s%N)   # one base for the whole experiment
#   for arm in base +feat …; do scripts/bba-gen-parallel.sh out/$arm 6400 $flags; done
#   # NEXT experiment: a NEW base. Nanosecond resolution avoids the old
#   # same-second collision. Bases must differ by at least the shard count;
#   # run deal-overlap before citing independent replication (#6).
#
# The chosen SEED_BASE is echoed; record it (with the git SHA) to reproduce a run.
# See docs/shared-machine-data-gen.md ("Seed hygiene").
#
# COMPLETENESS.  A run succeeds only if every worker exits 0 AND the result
# verifies: exactly shards 0..N-1, each parsing as a dump of PER_SHARD_COUNT
# boards dealt from seed SEED_BASE+i.  Anything else exits non-zero.  bba-gen
# writes a hidden temp file and renames it into place, so a killed worker
# leaves a hole (no shard-i.json), never a truncated shard.  A run first
# deletes OUTDIR's old shard-*.json, so a failed worker cannot leave an earlier
# run's shard in its slot, nor an earlier, larger JOBS strays past N-1.
#
#   SEED_BASE=… JOBS=N scripts/bba-gen-parallel.sh --verify OUTDIR PER_SHARD_COUNT
#
# runs only that check (no build, no bidding) — ab-lib.sh's resume test.  The
# check runs this checkout's target/release/examples/bba-gen even when BBA_GEN
# names another generator: `--verify` reads the shared dump format and loads
# no EPBot, so it needs neither the wrapper nor the newer glibc.
set -eu

verify=0
if [ "${1:-}" = --verify ]; then verify=1; shift; fi
[ $# -ge 2 ] || { echo "usage: $0 [--verify] OUTDIR PER_SHARD_COUNT [bba-gen flags...]" >&2; exit 2; }
outdir=$1
count=$2
shift 2

n=${JOBS:-$(nproc)}   # cap worker processes on a shared box; defaults to all cores
checker=$(cd "$(dirname "$0")/.." && pwd)/target/release/examples/bba-gen
# BBA_GEN overrides the binary — an A/B whose off arm is a different build (a
# pre-fix worktree) rather than a flag; see scripts/ab-rkcb-asker.sh.  ab-lib.sh
# resolves it the same way to fingerprint arms (arm_fingerprint); keep in step.
bin="${BBA_GEN:-$checker}"

# verify_shards — report every problem on stderr, then fail if there was one.
verify_shards() {
    bad=0
    i=0
    while [ "$i" -lt "$n" ]; do
        f=$outdir/shard-$i.json
        if [ ! -f "$f" ]; then
            echo "bba-gen-parallel: missing $f" >&2
            bad=$((bad + 1))
        elif ! "$checker" --verify "$f" --count "$count" --seed "$((seed_base + i))" >/dev/null; then
            bad=$((bad + 1))
        fi
        i=$((i + 1))
    done
    have=$(find "$outdir" -maxdepth 1 -name 'shard-*.json' 2>/dev/null | wc -l | tr -d ' ')
    if [ "$have" != "$n" ]; then
        echo "bba-gen-parallel: $outdir holds $have shard-*.json, expected $n (shard-0..$((n - 1)))" >&2
        bad=$((bad + 1))
    fi
    [ "$bad" = 0 ] || {
        echo "bba-gen-parallel: $outdir failed verification ($bad problem(s); SEED_BASE=$seed_base, $n shards x $count boards)" >&2
        return 1
    }
}

if [ "$verify" = 1 ]; then
    seed_base=${SEED_BASE:?--verify needs the SEED_BASE the shards were dealt from}
    [ -x "$checker" ] || { echo "bba-gen-parallel: --verify needs $checker" >&2; exit 1; }
    verify_shards
    exit
fi

seed_base=${SEED_BASE:-$(date +%s%N)}
if [ "${SKIP_BUILD:-0}" != 1 ]; then
    cargo build --release --features "${FEATURES:-serde}" --example bba-gen
else
    for b in "$bin" "$checker"; do
        [ -x "$b" ] || { echo "SKIP_BUILD=1 but bba-gen binary is missing: $b" >&2; exit 1; }
    done
fi
mkdir -p "$outdir"
find "$outdir" -maxdepth 1 \( -name 'shard-*.json' -o -name '.shard-*.tmp' \) -exec rm -f {} +

pids=
for i in $(seq 0 $((n - 1))); do
    "$bin" --count "$count" --seed "$((seed_base + i))" --output "$outdir/shard-$i.json" "$@" &
    pids="$pids $!"
done
# Wait on each worker by pid: a bare `wait` returns 0 even when one died.
failed=0
i=0
for pid in $pids; do
    if ! wait "$pid"; then
        echo "bba-gen-parallel: worker $i (seed $((seed_base + i))) failed" >&2
        failed=$((failed + 1))
    fi
    i=$((i + 1))
done
if [ "$failed" != 0 ]; then
    echo "bba-gen-parallel: $failed of $n workers failed; $outdir is incomplete (SEED_BASE=$seed_base)" >&2
    exit 1
fi
verify_shards
echo "bba-gen-parallel: SEED_BASE=$seed_base, wrote $n shards of $count boards to $outdir (verified)" >&2
