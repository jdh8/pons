#!/usr/bin/env bash
#
# ben-gen-parallel.sh — one ben-gen shard per running BEN server instance.
#
#   SEED_BASE=... scripts/ben-gen-parallel.sh OUTDIR PER_SHARD [ben-gen args...]
#
# Ports are discovered from the servers ben-servers.sh started; shard i talks
# to port i and is seeded SEED_BASE+i, so an experiment's arms share deals by
# exporting one SEED_BASE across invocations (the seed-hygiene invariant).
# Pass the tier the servers run (-t f|s) and the vul arm (-v none|both) after
# PER_SHARD. The clients are IO-bound; the servers (already idle-class) are
# the real load. Builds ben-gen up front — never rebuild while a run is live.
#
set -euo pipefail

outdir="${1:?usage: ben-gen-parallel.sh OUTDIR PER_SHARD [ben-gen args...]}"
per="${2:?usage: ben-gen-parallel.sh OUTDIR PER_SHARD [ben-gen args...]}"
shift 2
SEED_BASE="${SEED_BASE:-$(date +%s)}"
echo "ben-gen-parallel: SEED_BASE=$SEED_BASE outdir=$outdir per-shard=$per args: $*"

cd "$(dirname "$(readlink -f "$0")")/.."
cargo build --release --features serde --example ben-gen
mkdir -p "$outdir"

# `|| true` absorbs the pipeline's pipefail status when no server matches, so
# the [ -n ] guard below gets to print its message instead of a silent exit 1.
ports=$(pgrep -u "$USER" -af 'gameapi\.py' | grep -o -- '--port [0-9]*' | awk '{print $2}' | sort -n || true)
[ -n "$ports" ] || {
	echo "no BEN servers running — scripts/ben-servers.sh start N [f|s]" >&2
	exit 1
}

i=0
for port in $ports; do
	target/release/examples/ben-gen --count "$per" --seed "$((SEED_BASE + i))" \
		--port "$port" -o "$outdir/shard-$i.json" "$@" &
	i=$((i + 1))
done
wait
echo "ben-gen-parallel: $i shards × $per boards done in $outdir"
