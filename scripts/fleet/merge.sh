#!/usr/bin/env bash
#
# merge.sh — concatenate collected shards into one dump the trainer reads as-is.
#
# Validates that every shard sidecar agrees on the invariants that make rows
# mergeable (same feature/layout/system/SHA), that seeds are distinct (no
# double-counted rows), then `cat`s the .f32 / .tags in seed order and writes a
# summed .json.  The result is byte-for-byte the same format as one big dump.
#
# Usage:
#   scripts/fleet/merge.sh DIR [OUT_STEM]
#     DIR        directory of shard-*.{f32,tags,json} (run.sh's --out)
#     OUT_STEM   output path stem (default DIR/merged) → .f32 .tags .json
#
set -euo pipefail

IN="${1:?usage: merge.sh DIR [OUT_STEM]}"
STEM="${2:-$IN/merged}"

shopt -s nullglob
JSONS=("$IN"/shard-*.json)
shopt -u nullglob
((${#JSONS[@]})) || { echo "merge: no shard-*.json in $IN" >&2; exit 1; }
# Seed order (shard-<seed>.json), so .f32/.tags concat order is deterministic.
mapfile -t JSONS < <(printf '%s\n' "${JSONS[@]}" | sort -V)

# Invariants that MUST match for rows to be poolable.  Any mismatch = corruption.
PINS='[.feature_version,.features_len,.softmax_len,.row_len,.row_bytes,.dtype,.layout,.system,.git_sha,(.search//null)]'
ref="$(jq -c "$PINS" "${JSONS[0]}")"
for j in "${JSONS[@]}"; do
	cur="$(jq -c "$PINS" "$j")"
	[ "$cur" = "$ref" ] || { echo "merge: sidecar mismatch in $j" >&2; echo "  ref $ref" >&2; echo "  got $cur" >&2; exit 1; }
done

# Distinct seeds — duplicates would silently double-count rows.
nseed="${#JSONS[@]}"
nuniq="$(jq -r '.seed' "${JSONS[@]}" | sort -un | wc -l)"
[ "$nseed" = "$nuniq" ] || { echo "merge: duplicate seeds across shards — would double-count rows" >&2; exit 1; }

# Concatenate the payloads in the same (seed) order.
STEMS=("${JSONS[@]%.json}")
cat "${STEMS[@]/%/.f32}" >"$STEM.f32"
cat "${STEMS[@]/%/.tags}" >"$STEM.tags"

# Summed sidecar: shared pins through, counts added, seeds listed.  Per-shard
# .json files stay for full diagnostics (the divergence grid etc.).
jq -s '{
	feature_version: .[0].feature_version, features_len: .[0].features_len,
	softmax_len: .[0].softmax_len, row_len: .[0].row_len, row_bytes: .[0].row_bytes,
	dtype: .[0].dtype, layout: .[0].layout, tags: .[0].tags,
	system: .[0].system, search: (.[0].search // null), git_sha: .[0].git_sha,
	shards: length, seeds: [.[].seed],
	boards: ([.[].boards] | add), rows: ([.[].rows] | add),
	offbook_rows: ([.[].offbook_rows // 0] | add),
	contested_rows: ([.[].contested_rows // 0] | add),
	note: "merged by scripts/fleet/merge.sh; per-shard sidecars retain diagnostics"
}' "${JSONS[@]}" >"$STEM.json"

# Cross-check sizes against the row count — catches a truncated/partial shard.
rows="$(jq .rows "$STEM.json")"
rb="$(jq .row_bytes "$STEM.json")"
f32sz="$(stat -c%s "$STEM.f32")"
tagsz="$(stat -c%s "$STEM.tags")"
[ "$f32sz" = "$((rows * rb))" ] || { echo "merge: .f32 size $f32sz != rows*row_bytes $((rows * rb)) — truncated shard?" >&2; exit 1; }
[ "$tagsz" = "$rows" ] || { echo "merge: .tags size $tagsz != rows $rows — truncated shard?" >&2; exit 1; }

echo "merge: ${#JSONS[@]} shards → $STEM.{f32,tags,json}  ($rows rows, $((f32sz / 1024 / 1024)) MB)"
