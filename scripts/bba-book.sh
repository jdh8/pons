#!/usr/bin/env bash
#
# bba-book.sh — walk EPBot's rule book across every core.
#
# EPBot is single-threaded and thread-unsafe, so the walk parallelizes by
# PROCESS: one shard per subtree.  A first "frontier" pass walks the top of the
# tree (auctions shorter than $FRONTIER) in one process and writes the key of
# every node it declined to expand; each of those keys then becomes one process
# that walks its own subtree to the campaign's ceilings.  The shards are disjoint
# by construction — a node belongs to exactly one prefix — so the dump is the
# concatenation of `root.jsonl` and every `NNNNN.jsonl`.
#
#   scripts/bba-book.sh RUN_DIR [probe-bba-book flags...]
#   cargo run --release --features serde --example probe-bba-book -- --stats RUN_DIR
#   cargo run --release --features serde --example probe-bba-book -- --render RUN_DIR --prefix "1♠"
#
# RESUMABLE.  A shard writes `NNNNN.jsonl.part` and renames it on success, so a
# re-run skips every shard that finished and redoes only the ones that did not.
# Deleting `frontier.txt` and `root.jsonl` restarts the whole walk.
#
# On a shared box wrap the WHOLE script in scripts/idle-run.sh, and never
# rebuild the binary while it runs (docs/shared-machine-data-gen.md).
#
#   RUN=ab-results/bba-book/$(date +%F)-$(git rev-parse --short HEAD)
#   scripts/idle-run.sh scripts/bba-book.sh "$RUN" --corpus "$RUN/../corpus"
#
# Environment: FRONTIER (default 3) how deep the single-process pass goes,
# JOBS (default nproc) concurrent shards, SKIP_BUILD=1 to use the binary as is
# — which is what you want if a run is already in flight.
set -euo pipefail

if [[ $# -lt 1 || ${1:-} == "-h" || ${1:-} == "--help" ]]; then
	sed -n '2,/^set -euo/p' "$0" | sed 's/^# \{0,1\}//;/^set -euo/d'
	exit 2
fi

run=$1
shift
flags=("$@")

root=$(cd "$(dirname "$0")/.." && pwd)
run=$(cd "$(dirname "$run")" && pwd)/$(basename "$run")
# The default `--card cards/American.bbsa` and `--lib vendor/bba/...` are
# repo-relative, so every shard has to start here.
cd "$root"
bin="$root/target/release/examples/probe-bba-book"
frontier=${FRONTIER:-3}
jobs=${JOBS:-$(nproc)}

if [[ ${SKIP_BUILD:-0} != 1 ]]; then
	cargo build --release --features serde --example probe-bba-book
elif [[ ! -x $bin ]]; then
	echo "SKIP_BUILD=1 but the binary is missing: $bin" >&2
	exit 1
fi

mkdir -p "$run"
# Pin the binary INTO the run directory and use that copy.  Two reasons: a
# rebuild mid-run can no longer swap the walker under the shards (the iron rule
# of docs/shared-machine-data-gen.md, enforced instead of remembered), and the
# dump ships with the exact program that produced it.
cp "$bin" "$run/probe-bba-book"
bin="$run/probe-bba-book"

# Provenance beside the dump: which engine, which card, which flags.  A book
# walk is only reproducible if the card is, and the card is a build input.
{
	echo "sha      $(git -C "$root" rev-parse HEAD)"
	echo "dirty    $(git -C "$root" status --porcelain | wc -l) file(s)"
	echo "date     $(date -Is)"
	echo "frontier $frontier"
	echo "jobs     $jobs"
	echo "flags    ${flags[*]-}"
} >"$run/RUN"

# 1. The top of the tree, in one process, plus the shard list.
if [[ ! -f $run/root.jsonl ]]; then
	"$bin" --frontier "$frontier" --frontier-out "$run/frontier.txt" \
		--output "$run/root.jsonl.part" "${flags[@]-}"
	mv "$run/root.jsonl.part" "$run/root.jsonl"
fi

# 2. One process per frontier key, `jobs` at a time.
total=$(wc -l <"$run/frontier.txt")
echo "bba-book: $total shard(s), $jobs at a time" >&2
index=0
while IFS= read -r key; do
	index=$((index + 1))
	out=$(printf '%s/%05d.jsonl' "$run" "$index")
	[[ -f $out ]] && continue
	# `|| true`: `wait -n` reports the finished job's status, and under `set -e`
	# one failed shard would otherwise tear down the whole pool.  A shard that
	# fails simply never gets renamed, and the re-run picks it up.
	while (($(jobs -rp | wc -l) >= jobs)); do wait -n || true; done
	# `--prefix=` rather than `--prefix `: a third- or fourth-seat key starts
	# with a pass (`- - 1♠`), which a space-separated value would hand to the
	# argument parser as a flag.
	{
		"$bin" --prefix="$key" --output "$out.part" "${flags[@]-}" 2>"$out.log" &&
			mv "$out.part" "$out"
	} &
done <"$run/frontier.txt"
wait || true

done_count=$(find "$run" -name '[0-9]*.jsonl' | wc -l)
echo "bba-book: $done_count/$total shard(s) complete in $run" >&2
[[ $done_count -eq $total ]] || {
	echo "bba-book: re-run the same command to retry the rest" >&2
	exit 1
}
