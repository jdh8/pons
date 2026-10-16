#!/usr/bin/env bash
#
# ben-gen-parallel.sh — one ben-gen shard per running BEN server instance.
#
#   SEED_BASE=... [SHARDS=N] scripts/ben-gen-parallel.sh OUTDIR PER_SHARD [ben-gen args...]
#
# Ports are discovered from the servers ben-servers.sh started; shard i talks
# to port i and is seeded SEED_BASE+i, so an experiment's arms share deals by
# exporting one SEED_BASE across invocations (the seed-hygiene invariant).
# Pass the tier the servers run (-t f|s) and the vul arm (-v none|both) after
# PER_SHARD. The clients are IO-bound; the servers (already idle-class) are
# the real load. Builds ben-gen up front — never rebuild while a run is live.
#
# SHARDS, when set, is the shard count the caller expects: a run refuses to
# start unless exactly that many servers are up, so a fleet that grew or
# shrank between two arms cannot silently change the deal set (JOBS is not
# read here; the server count is the parallelism).
#
# COMPLETENESS.  A run succeeds only if every worker exits 0 AND the result
# verifies: exactly shards 0..N-1, each parsing as a dump of PER_SHARD boards
# dealt from seed SEED_BASE+i.  Anything else exits non-zero.  ben-gen writes
# a hidden temp file and renames it into place, so a worker that aborts (BEN
# unreachable, a desync) leaves a hole, never a truncated shard.  A run first
# deletes OUTDIR's old shard-*.json, so a failed worker cannot leave an
# earlier run's shard in its slot, nor a larger earlier fleet strays past N-1.
#
#   SEED_BASE=… SHARDS=N scripts/ben-gen-parallel.sh --verify OUTDIR PER_SHARD
#
# runs only that check (no build, no servers) — ab-lib.sh's resume test for a
# GENERATOR=ben-gen runner.  Mirrors bba-gen-parallel.sh; keep the two in step.
#
set -euo pipefail

verify=0
if [ "${1:-}" = --verify ]; then verify=1; shift; fi
usage="usage: ben-gen-parallel.sh [--verify] OUTDIR PER_SHARD [ben-gen args...]"
outdir="${1:?$usage}"
per="${2:?$usage}"
shift 2

cd "$(dirname "$(readlink -f "$0")")/.."
bin=target/release/examples/ben-gen

# verify_shards N — report every problem on stderr, then fail if there was one.
verify_shards() {
	local n=$1 bad=0 i have
	for ((i = 0; i < n; i++)); do
		local f=$outdir/shard-$i.json
		if [ ! -f "$f" ]; then
			echo "ben-gen-parallel: missing $f" >&2
			bad=$((bad + 1))
		elif ! "$bin" --verify "$f" --count "$per" --seed "$((SEED_BASE + i))" >/dev/null; then
			bad=$((bad + 1))
		fi
	done
	have=$(find "$outdir" -maxdepth 1 -name 'shard-*.json' 2>/dev/null | wc -l | tr -d ' ')
	if [ "$have" != "$n" ]; then
		echo "ben-gen-parallel: $outdir holds $have shard-*.json, expected $n (shard-0..$((n - 1)))" >&2
		bad=$((bad + 1))
	fi
	[ "$bad" = 0 ] || {
		echo "ben-gen-parallel: $outdir failed verification ($bad problem(s); SEED_BASE=$SEED_BASE, $n shards x $per boards)" >&2
		return 1
	}
}

if [ "$verify" = 1 ]; then
	: "${SEED_BASE:?--verify needs the SEED_BASE the shards were dealt from}"
	: "${SHARDS:?--verify needs SHARDS, the shard count the arm was generated with}"
	[ -x "$bin" ] || { echo "ben-gen-parallel: --verify needs $bin" >&2; exit 1; }
	verify_shards "$SHARDS"
	exit
fi

SEED_BASE="${SEED_BASE:-$(date +%s%N)}"
echo "ben-gen-parallel: SEED_BASE=$SEED_BASE outdir=$outdir per-shard=$per args: $*"

if [ "${SKIP_BUILD:-0}" != 1 ]; then
	cargo build --release --features serde --example ben-gen
elif [ ! -x "$bin" ]; then
	echo "SKIP_BUILD=1 but $bin is missing" >&2
	exit 1
fi

# `|| true` absorbs the pipeline's pipefail status when no server matches, so
# the [ -n ] guard below gets to print its message instead of a silent exit 1.
# `sort -nu`: idle-run.sh no longer execs, so its wrapper AND the python it
# forked both carry `gameapi.py --port N` — without the dedupe every port is
# discovered twice and the run launches two clients per server, interleaving
# their Tier-S sampling on one server RNG.
ports=$(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -nu || true)
[ -n "$ports" ] || {
	echo "no BEN servers running — scripts/ben-servers.sh start N [f|s]" >&2
	exit 1
}
n=$(printf '%s\n' "$ports" | wc -l | tr -d ' ')
if [ -n "${SHARDS:-}" ] && [ "$n" != "$SHARDS" ]; then
	echo "ben-gen-parallel: $n BEN servers running, expected SHARDS=$SHARDS; a fleet change breaks shard pairing" >&2
	exit 1
fi

mkdir -p "$outdir"
find "$outdir" -maxdepth 1 \( -name 'shard-*.json' -o -name '.shard-*.tmp' \) -exec rm -f {} +

pids=()
i=0
for port in $ports; do
	"$bin" --count "$per" --seed "$((SEED_BASE + i))" \
		--port "$port" -o "$outdir/shard-$i.json" "$@" &
	pids+=("$!")
	i=$((i + 1))
done
# Wait on each worker by pid: a bare `wait` returns 0 even when one died.
failed=0
for i in "${!pids[@]}"; do
	if ! wait "${pids[$i]}"; then
		echo "ben-gen-parallel: worker $i (seed $((SEED_BASE + i))) failed" >&2
		failed=$((failed + 1))
	fi
done
if [ "$failed" != 0 ]; then
	echo "ben-gen-parallel: $failed of $n workers failed; $outdir is incomplete (SEED_BASE=$SEED_BASE)" >&2
	exit 1
fi
verify_shards "$n"
echo "ben-gen-parallel: $n shards × $per boards done in $outdir (verified)"
