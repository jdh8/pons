#!/usr/bin/env bash
#
# reading-knobs-ab.sh — the pass-reading ship-gate A/B (docs/ben-gap-campaign.md,
# dual-reference rule): primary fresh-seed vs BEN Tier F, guard same-seed vs
# BBA, arms strictly sequential, scored plain + PD at the end.
#
#   setsid nohup scripts/idle-run.sh scripts/reading-knobs-ab.sh \
#       >ab-results/reading-knobs/run.log 2>&1 &
#
# Arms: off (baseline) vs --ns-pass-reading (the ship candidate exactly as it
# would ship: own-side passes only, table-alert reading still off).  BEN cells
# run FIRST — the servers live on deleted inodes (~/ben was removed) and
# cannot be restarted; BBA needs nothing external.  One SEED_BASE for the
# whole experiment, recorded in note.txt with the git SHA.  ben-gen/bba-gen
# binaries are prebuilt; the parallel scripts' cargo build must stay a no-op —
# do not touch the codebase while this runs.
set -euo pipefail
cd "$(dirname "$0")/.."

EXP=ab-results/reading-knobs/2026-07-17
mkdir -p "$EXP/scores"
SHA=$(git rev-parse --short HEAD)
if [ ! -s "$EXP/seed" ]; then date +%s >"$EXP/seed"; fi
SEED_BASE=$(cat "$EXP/seed")
export SEED_BASE
PER=${PER:-6400}

log() { echo "$(date -u +%FT%TZ) $*" >&2; }
log "=== reading-knobs A/B start: sha=$SHA SEED_BASE=$SEED_BASE per-shard=$PER"
echo "sha=$SHA SEED_BASE=$SEED_BASE per-shard=$PER arms=off,pass" >"$EXP/note.txt"

# A shard that panicked leaves a missing/empty file; flag it loudly but keep
# the chain alive — score with matching shard subsets manually if this fires.
check() {
	local dir=$1 want=$2 ok=0
	for f in "$dir"/shard-*.json; do [ -s "$f" ] && ok=$((ok + 1)); done
	[ "$ok" -eq "$want" ] || log "!!! $dir has $ok/$want live shards — pair shards manually when scoring"
}

# Phase 1 — primary vs BEN Tier F (8 servers on 8085-8092, fragile: run first)
for arm in off pass; do
	flags=()
	[ "$arm" = pass ] && flags=(--ns-pass-reading)
	for vul in none both; do
		dir="$EXP/ben-$arm/$vul"
		[ -d "$dir" ] && { log "skip $dir (exists)"; continue; }
		log "generate $dir"
		scripts/ben-gen-parallel.sh "$dir" "$PER" -v "$vul" -t f "${flags[@]+"${flags[@]}"}"
		check "$dir" 8
	done
done

# Phase 2 — guard vs BBA (same SEED_BASE; anchor-style cells)
for arm in off pass; do
	flags=()
	[ "$arm" = pass ] && flags=(--ns-pass-reading)
	for vul in none both; do
		dir="$EXP/bba-$arm/$vul"
		[ -d "$dir" ] && { log "skip $dir (exists)"; continue; }
		log "generate $dir"
		scripts/bba-gen-parallel.sh "$dir" "$PER" -v "$vul" "${flags[@]+"${flags[@]}"}"
		check "$dir" "$(nproc)"
	done
done

# Phase 3 — scoring: per-arm pooled IMPs/board + paired on-vs-off diffs,
# both brackets (plain DD + PD), per vulnerability cell.
for ref in ben bba; do
	for arm in off pass; do
		for vul in none both; do
			for score in plain pd; do
				out="$EXP/scores/$ref-$arm-$vul-$score.txt"
				[ -s "$out" ] && continue
				log "score $out"
				target/release/examples/bba-score "$EXP/$ref-$arm/$vul"/shard-*.json \
					--score "$score" >"$out" 2>&1 || log "!!! scoring failed: $out"
			done
		done
	done
	for vul in none both; do
		for score in plain pd; do
			out="$EXP/scores/diff-$ref-$vul-$score.txt"
			[ -s "$out" ] && continue
			log "diff $out"
			target/release/examples/ab-dump-diff "$EXP/$ref-pass/$vul" "$EXP/$ref-off/$vul" \
				--score "$score" >"$out" 2>&1 || log "!!! diff failed: $out"
		done
	done
done

log "=== reading-knobs A/B done: $EXP/scores/"
