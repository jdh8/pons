#!/usr/bin/env bash
#
# relabel-worker.sh — one box's share of the M-series relabel
# (docs/ai-bidder/logit-calibration.md §6; fleet in docs/shared-machine-data-gen.md).
#
# Walks the v6 corpus recipe of scripts/dump-v6.sh in CHUNK-deal windows and
# runs `dump-teacher --relabel` on every chunk this box owns: chunk g (a global
# counter over shards in recipe order) belongs to the box whose OFFSETS
# contain g mod STRIDE.  A chunk whose sidecar already records >= LAYOUTS is
# skipped; one recorded short is extended in place (no solve repeated); one
# found under EXTRA_OUT (another box's tree, read-only here) is always
# skipped.  Outputs are tmp+rename inside the binary, so a killed chunk leaves
# nothing behind and is simply redone.
#
# Environment (systemd reads it from ~/.config/pons/relabel-<run>.env):
#   OUT        chunk root, written as $OUT/<shard>/chunk-<c>.{f32,tags,json,ret}
#   BANK       the deal bank (default /nfs2/jdh8/pons/22.pdd; copy it locally on a box without NFS)
#   STRIDE     number of residues in the partition (sum of host weights); default 1
#   OFFSETS    space-separated residues this box owns; default "0"
#   LAYOUTS    layouts per decision = 2M of the deepest cut wanted; default 64
#   CHUNK      deals per chunk; default 5000 (~3 box-hours on 32 cores at 64 layouts)
#   REVERSE    1 = walk the chunk list backwards (the mop-up pass); default 0
#   EXTRA_OUT  colon-separated other roots the existence gate also consults
#   DUMP_COMMON  override the recipe's common flags (default: the v6 recipe's)
#   RECIPE_FILE  override the shard recipe with a file of `name skip boards seed [flags]` lines
#
# SIGHUP drains: the current chunk finishes, then the script exits 0.
# SIGTERM stops now (the unit SIGKILLs the binary; tmp+rename keeps the tree clean).
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=${OUT:?set OUT to the chunk root}
BANK=${BANK:-/nfs2/jdh8/pons/22.pdd}
STRIDE=${STRIDE:-1}
OFFSETS=${OFFSETS:-0}
LAYOUTS=${LAYOUTS:-64}
CHUNK=${CHUNK:-5000}
REVERSE=${REVERSE:-0}
EXTRA_OUT=${EXTRA_OUT:-}
BIN=target/release/examples/dump-teacher
COMMON=${DUMP_COMMON:-"--deals $BANK --teacher bba --configured --feature-version 6"}

drain=0
trap 'drain=1' HUP

# The recipe of scripts/dump-v6.sh: name skip boards seed [extra flags].
# RECIPE_FILE replaces it with a file of such lines (a dry run on a small bank).
recipe() {
	if [[ -n ${RECIPE_FILE:-} ]]; then
		cat "$RECIPE_FILE"
		return
	fi
	for i in 0 1 2 3 4 5 6 7; do
		echo "uniform-$i $((3250000 + i * 31250)) 31250 $((100 + i))"
	done
	for i in 0 1 2 3; do
		echo "enriched-$i $((3500000 + i * 125000)) 125000 $((200 + i)) --enrich 28:9 --replay --cell a-on/a-on --cell a-off/a-off"
	done
	i=0
	for bit in 0004 1000 2000 0002 4000 0800 8000 0020; do
		echo "axis-$bit $((4000000 + i * 20000)) 20000 $((300 + i)) --replay --cell a-off/a-off --cell a-off+$bit/a-off"
		i=$((i + 1))
	done
}

# Every chunk of every shard: g shard c skip boards seed extra...
chunks() {
	local g=0 name skip boards seed extra n c s b
	while read -r name skip boards seed extra; do
		n=$(((boards + CHUNK - 1) / CHUNK))
		for ((c = 0; c < n; c++)); do
			s=$((skip + c * CHUNK))
			b=$((boards - c * CHUNK))
			((b > CHUNK)) && b=$CHUNK
			echo "$g $name $c $s $b $seed $extra"
			g=$((g + 1))
		done
	done < <(recipe)
}

# Layouts a finished chunk records (0 when none): "<root> <layouts>"
recorded() {
	local root json
	for root in "$OUT" ${EXTRA_OUT//:/ }; do
		json=$root/$1/chunk-$2.json
		[[ -r $json ]] || continue
		echo "$root $(sed -n 's/^ *"layouts": \([0-9]*\),*$/\1/p' "$json" | head -1)"
		return
	done
	echo "- 0"
}

list=$(chunks)
[[ $REVERSE = 1 ]] && list=$(tac <<<"$list")
while read -r g name c skip boards seed extra; do
	case " $OFFSETS " in *" $((g % STRIDE)) "*) ;; *) continue ;; esac
	read -r root have < <(recorded "$name" "$c")
	if ((have >= LAYOUTS)); then
		continue
	elif [[ $root != "$OUT" && $root != - ]]; then
		echo "chunk $g $name/$c: $root has $have < $LAYOUTS layouts; its owner extends it — skipped"
		continue
	fi
	mkdir -p "$OUT/$name"
	echo "chunk $g $name/$c: skip $skip boards $boards seed $seed → $LAYOUTS layouts (have $have)"
	# shellcheck disable=SC2086
	$BIN $COMMON --skip "$skip" --boards "$boards" --seed "$seed" \
		--relabel --layouts "$LAYOUTS" --out "$OUT/$name/chunk-$c" $extra
	if ((drain)); then
		echo "relabel-worker: drained after chunk $g"
		exit 0
	fi
done <<<"$list"
echo "relabel-worker: every owned chunk is at $LAYOUTS layouts"
